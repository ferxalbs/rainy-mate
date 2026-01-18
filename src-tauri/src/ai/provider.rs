// Rainy Cowork - AI Provider Trait and Manager
// Unified abstraction layer using rainy-sdk for premium features

use crate::ai::{gemini::GeminiProvider, keychain::KeychainManager};
use crate::models::{AIProviderConfig, ProviderType};
use rainy_sdk::{CoworkCapabilities, CoworkTier, RainyClient};

/// Error type for AI operations
#[derive(Debug, thiserror::Error)]
pub enum AIError {
    #[error("API request failed: {0}")]
    RequestFailed(String),
    #[error("Invalid API key")]
    InvalidApiKey,
    #[error("Rate limited")]
    RateLimited,
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Provider not available: {0}")]
    ProviderNotAvailable(String),
    #[error("Premium feature required: {0}")]
    PremiumRequired(String),
}

/// Cached cowork capabilities
#[derive(Debug, Clone)]
pub struct CachedCapabilities {
    pub capabilities: CoworkCapabilities,
    pub fetched_at: std::time::Instant,
}

/// Manager for AI providers - uses rainy-sdk for premium, Gemini for free tier
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

    /// Get Rainy SDK client if API key is configured
    async fn get_rainy_client(&self) -> Option<RainyClient> {
        let api_key = self.keychain.get_key("rainy_api").ok()??;
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
        if let Some(client) = self.get_rainy_client().await {
            if let Ok(caps) = client.get_cowork_capabilities().await {
                self.cached_caps = Some(CachedCapabilities {
                    capabilities: caps.clone(),
                    fetched_at: std::time::Instant::now(),
                });
                return caps;
            }
        }

        // Fallback to free tier
        CoworkCapabilities::free()
    }

    /// Check if user has premium access
    pub async fn is_premium(&mut self) -> bool {
        self.get_capabilities().await.tier.is_premium()
    }

    /// Get current tier
    pub async fn get_tier(&mut self) -> CoworkTier {
        self.get_capabilities().await.tier
    }

    /// List available providers based on tier
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

        // Show Rainy API if premium
        if caps.tier.is_premium() {
            providers.insert(
                0,
                AIProviderConfig {
                    provider: ProviderType::RainyApi,
                    name: format!("Rainy API ({})", caps.tier_name),
                    model: "gpt-4o".to_string(),
                    is_available: true,
                    requires_api_key: true,
                },
            );
        }

        providers
    }

    /// Get available models based on tier
    pub async fn get_models(&mut self, provider: &str) -> Result<Vec<String>, String> {
        match provider {
            "rainy_api" => {
                let caps = self.get_capabilities().await;
                if caps.tier.is_premium() {
                    Ok(caps.models)
                } else {
                    Err("Premium subscription required for Rainy API models".to_string())
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
                // Validate by trying to create client and check capabilities
                match RainyClient::with_api_key(api_key) {
                    Ok(client) => match client.get_cowork_capabilities().await {
                        Ok(caps) => Ok(caps.is_valid),
                        Err(_) => Ok(false),
                    },
                    Err(_) => Ok(false),
                }
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
                // Use rainy-sdk for premium access
                let caps = self.get_capabilities().await;
                if !caps.tier.is_premium() {
                    return Err(
                        "Premium subscription required. Please add a Rainy API key.".to_string()
                    );
                }

                if !caps.can_use_model(model) {
                    return Err(format!(
                        "Model {} not available for {} tier",
                        model, caps.tier_name
                    ));
                }

                on_progress(10, Some("Connecting to Rainy API...".to_string()));

                let client = self
                    .get_rainy_client()
                    .await
                    .ok_or("No Rainy API key configured")?;

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

    /// Check if a feature is available based on tier
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
