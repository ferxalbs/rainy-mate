// Rainy Cowork - AI Provider Trait and Manager
// Unified abstraction layer using rainy-sdk for premium features

use crate::ai::{gemini::GeminiProvider, keychain::KeychainManager};
use crate::models::{AIProviderConfig, ProviderType};
use rainy_sdk::{CoworkCapabilities, CoworkPlan, RainyClient};

/// Error type for AI operations
#[derive(Debug, thiserror::Error)]
pub enum AIError {
    #[error("API request failed: {0}")]
    RequestFailed(String),
    #[error("Invalid API key")]
    InvalidApiKey,
    #[error("Rate limited")]
    RateLimited,
    #[error("Provider not available: {0}")]
    ProviderNotAvailable(String),
}

/// Cached cowork capabilities
#[derive(Debug, Clone)]
pub struct CachedCapabilities {
    pub capabilities: CoworkCapabilities,
    pub fetched_at: std::time::Instant,
}

/// Manager for AI providers - uses rainy-sdk for paid plans, Gemini for free tier
pub struct AIProviderManager {
    keychain: KeychainManager,
    gemini: GeminiProvider,
    /// Cached capabilities from last check
    cached_caps: Option<CachedCapabilities>,
}

impl AIProviderManager {
    pub fn new() -> Self {
        Self {
            keychain: KeychainManager::new(),
            gemini: GeminiProvider::new(),
            cached_caps: None,
        }
    }

    /// Get Rainy SDK client for Cowork if API key is configured
    async fn get_cowork_client(&self) -> Option<RainyClient> {
        let api_key = self.keychain.get_key("cowork_api").ok()??;
        RainyClient::with_api_key(&api_key).ok()
    }

    /// Get cowork capabilities (with caching)
    pub async fn get_capabilities(&mut self) -> CoworkCapabilities {
        // Check cache (valid for 5 minutes)
        if let Some(cached) = &self.cached_caps {
            if cached.fetched_at.elapsed().as_secs() < 300 {
                return cached.capabilities.clone();
            }
        }

        // Fetch from SDK
        if let Some(client) = self.get_cowork_client().await {
            if let Ok(caps) = client.get_cowork_capabilities().await {
                self.cached_caps = Some(CachedCapabilities {
                    capabilities: caps.clone(),
                    fetched_at: std::time::Instant::now(),
                });
                return caps;
            }
        }

        // Fallback to free plan
        CoworkCapabilities::free()
    }

    /// Get cowork models directly from API (efficient)
    pub async fn get_cowork_models_from_api(
        &self,
    ) -> Result<rainy_sdk::cowork::CoworkModelsResponse, String> {
        if let Some(client) = self.get_cowork_client().await {
            client.get_cowork_models().await.map_err(|e| e.to_string())
        } else {
            Err("No Cowork API key configured".to_string())
        }
    }

    /// Check if user has a paid plan
    #[allow(dead_code)]
    pub async fn has_paid_plan(&mut self) -> bool {
        self.get_capabilities().await.profile.plan.is_paid()
    }

    /// Get current plan
    #[allow(dead_code)]
    pub async fn get_plan(&mut self) -> CoworkPlan {
        self.get_capabilities().await.profile.plan
    }

    /// List available providers based on plan
    pub async fn list_providers(&mut self) -> Vec<AIProviderConfig> {
        let caps = self.get_capabilities().await;
        let mut providers = vec![];

        // Always show Gemini (free tier fallback)
        providers.push(AIProviderConfig {
            provider: ProviderType::Gemini,
            name: "Google Gemini".to_string(),
            model: "gemini-2.5-flash".to_string(),
            is_available: true,
            requires_api_key: true,
        });

        // Show Rainy API (Pay-As-You-Go)
        // Always available as option, user provides key
        providers.insert(
            0,
            AIProviderConfig {
                provider: ProviderType::RainyApi,
                name: "Rainy API".to_string(),
                model: "gpt-4o".to_string(),
                is_available: true,
                requires_api_key: true,
            },
        );

        // Show Cowork Subscription if valid
        if caps.profile.plan.is_paid() {
            providers.insert(
                1,
                AIProviderConfig {
                    provider: ProviderType::CoworkApi,
                    name: format!("Cowork ({})", caps.profile.plan.name),
                    model: "gemini-3-pro-preview".to_string(),
                    is_available: true,
                    requires_api_key: true,
                },
            );
        } else {
            providers.insert(
                1,
                AIProviderConfig {
                    provider: ProviderType::CoworkApi,
                    name: "Cowork Subscription".to_string(),
                    model: "gemini-3-pro-preview".to_string(),
                    is_available: true,
                    requires_api_key: true,
                },
            );
        }

        providers
    }

    /// Get available models based on plan
    pub async fn get_models(&mut self, provider: &str) -> Result<Vec<String>, String> {
        match provider {
            "rainy_api" => {
                // Standard Rainy API supports all models (subject to key permission/credits)
                Ok(vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "gpt-4-turbo".to_string(),
                    "claude-3.5-sonnet".to_string(),
                    "claude-3-opus".to_string(),
                ])
            }
            "cowork_api" => {
                let caps = self.get_capabilities().await;
                if caps.profile.plan.is_paid() {
                    Ok(caps.models)
                } else {
                    Err("Upgrade to a paid plan to access Cowork models".to_string())
                }
            }
            "gemini" => Ok(self.gemini.available_models()),
            _ => Err(AIError::ProviderNotAvailable(provider.to_string()).to_string()),
        }
    }

    /// Validate an API key for a provider
    pub async fn validate_api_key(&self, provider: &str, api_key: &str) -> Result<bool, String> {
        match provider {
            "rainy_api" => {
                // Validate generic Rainy API key
                let client = RainyClient::with_api_key(api_key).map_err(|e| e.to_string())?;
                client
                    .list_available_models()
                    .await
                    .map(|_| true)
                    .map_err(|e| e.to_string())
            }
            "cowork_api" => {
                // Validate Cowork key and profile
                let client = RainyClient::with_api_key(api_key).map_err(|e| e.to_string())?;
                client
                    .get_cowork_profile()
                    .await
                    .map(|_| true)
                    .map_err(|e| e.to_string())
            }
            "gemini" => self
                .gemini
                .validate_key(api_key)
                .await
                .map_err(|e| e.to_string()),
            _ => Err(AIError::ProviderNotAvailable(provider.to_string()).to_string()),
        }
    }

    /// Store API key in macOS Keychain
    pub async fn store_api_key(&self, provider: &str, api_key: &str) -> Result<(), String> {
        self.keychain.store_key(provider, api_key)
    }

    /// Get API key from macOS Keychain
    pub async fn get_api_key(&self, provider: &str) -> Result<Option<String>, String> {
        self.keychain.get_key(provider)
    }

    /// Delete API key from macOS Keychain
    pub async fn delete_api_key(&self, provider: &str) -> Result<(), String> {
        self.keychain.delete_key(provider)
    }

    /// Check if API key exists for a provider
    pub async fn has_api_key(&self, provider: &str) -> Result<bool, String> {
        Ok(self.keychain.has_key(provider))
    }

    /// Execute a prompt using the specified provider
    pub async fn execute_prompt<F>(
        &mut self,
        provider: &ProviderType,
        model: &str,
        prompt: &str,
        on_progress: F,
    ) -> Result<String, String>
    where
        F: Fn(u8, Option<String>) + Send + Sync + 'static,
    {
        match provider {
            ProviderType::RainyApi => {
                // Standard Rainy API (Pay-as-you-go)
                // Just needs a valid key, no plan checks
                let api_key = self
                    .keychain
                    .get_key("rainy_api")
                    .map_err(|e| e.to_string())?
                    .ok_or("No Rainy API key found")?;

                on_progress(10, Some("Connecting to Rainy API...".to_string()));
                let client = RainyClient::with_api_key(api_key).map_err(|e| e.to_string())?;

                on_progress(30, Some("Sending request...".to_string()));
                let result = client
                    .simple_chat(model, prompt)
                    .await
                    .map_err(|e| e.to_string())?;

                on_progress(100, Some("Complete".to_string()));
                Ok(result)
            }
            ProviderType::CoworkApi => {
                // Cowork Subscription (Credits)
                // Needs checks for capabilities/plan
                let caps = self.get_capabilities().await;
                if !caps.profile.plan.is_paid() {
                    return Err("Upgrade to a paid plan to use Cowork API.".to_string());
                }

                if !caps.can_use_model(model) {
                    return Err(format!(
                        "Model {} not available on {} plan",
                        model, caps.profile.plan.name
                    ));
                }

                if !caps.can_make_request() {
                    if let Some(msg) = &caps.upgrade_message {
                        return Err(msg.clone());
                    }
                    return Err("Usage limit reached. Upgrade for more access.".to_string());
                }

                // We need the key specifically for Cowork (might be different from rainy_api key)
                // But get_capabilities uses "rainy_api" key in get_rainy_client().
                // We should separate keys storage.

                let api_key = self
                    .keychain
                    .get_key("cowork_api")
                    .map_err(|e| e.to_string())?
                    .ok_or("No Cowork API key found")?;

                on_progress(10, Some("Connecting to Cowork API...".to_string()));
                let client = RainyClient::with_api_key(api_key).map_err(|e| e.to_string())?;

                on_progress(30, Some("Sending request...".to_string()));
                let result = client
                    .simple_chat(model, prompt)
                    .await
                    .map_err(|e| e.to_string())?;

                on_progress(100, Some("Complete".to_string()));
                Ok(result)
            }
            ProviderType::Gemini => {
                // Use direct Gemini (free tier)
                let api_key = self
                    .get_api_key("gemini")
                    .await?
                    .ok_or_else(|| "No Gemini API key configured".to_string())?;

                self.gemini
                    .complete_with_api_key(model, prompt, &api_key, on_progress)
                    .await
                    .map_err(|e| e.to_string())
            }
        }
    }

    /// Check if a feature is available based on plan
    pub async fn can_use_feature(&mut self, feature: &str) -> bool {
        self.get_capabilities().await.can_use_feature(feature)
    }

    /// Invalidate cached capabilities (call after API key changes)
    pub fn invalidate_cache(&mut self) {
        self.cached_caps = None;
    }
}

impl Default for AIProviderManager {
    fn default() -> Self {
        Self::new()
    }
}
