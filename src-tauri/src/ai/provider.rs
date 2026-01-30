// Rainy Cowork - AI Provider Trait and Manager
// Unified abstraction layer using rainy-sdk for premium features

use crate::ai::{gemini::GeminiProvider, keychain::KeychainManager};
use crate::models::{AIProviderConfig, ProviderType};
use futures::StreamExt;
use rainy_sdk::{ChatCompletionRequest, ChatMessage, CoworkCapabilities, CoworkPlan, RainyClient};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

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

/// Pooled RainyClient with connection reuse
#[derive(Clone)]
struct PooledClient {
    client: Arc<RainyClient>,
    created_at: Instant,
}

/// Manager for AI providers - uses rainy-sdk for paid plans, Gemini for free tier
pub struct AIProviderManager {
    keychain: KeychainManager,
    gemini: GeminiProvider,
    /// Cached capabilities from last check
    cached_caps: Arc<RwLock<Option<CachedCapabilities>>>,
    /// Connection pool for RainyClient instances
    client_pool: Arc<RwLock<HashMap<String, PooledClient>>>,
    /// HTTP client with optimized settings
    #[allow(dead_code)]
    http_client: reqwest::Client,
}

impl AIProviderManager {
    pub fn new() -> Self {
        // Create optimized HTTP client with timeouts and connection pooling
        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_nodelay(true)
            .http2_keep_alive_interval(Duration::from_secs(30))
            .http2_keep_alive_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            keychain: KeychainManager::new(),
            gemini: GeminiProvider::new(),
            cached_caps: Arc::new(RwLock::new(None)),
            client_pool: Arc::new(RwLock::new(HashMap::new())),
            http_client,
        }
    }

    /// Get Rainy SDK client for Cowork if API key is configured (with connection pooling)
    async fn get_cowork_client(&self) -> Option<Arc<RainyClient>> {
        // Try multiple key sources for cowork access:
        // 1. Dedicated cowork_api key
        // 2. General rainy_api key (might have cowork subscription)
        // 3. Any other available key that might work
        let api_key = self
            .keychain
            .get_key("cowork_api")
            .ok()
            .flatten()
            .or_else(|| self.keychain.get_key("rainy_api").ok().flatten());

        if let Some(key) = api_key {
            tracing::info!(
                "Using key for cowork client: cowork_api={}, rainy_api={}",
                self.keychain
                    .get_key("cowork_api")
                    .is_ok_and(|k| k.is_some()),
                self.keychain
                    .get_key("rainy_api")
                    .is_ok_and(|k| k.is_some())
            );
            return self.get_or_create_client("cowork_api", &key).await;
        }

        tracing::warn!("No suitable API key found for cowork client");
        None
    }

    /// Get or create a pooled RainyClient instance
    async fn get_or_create_client(
        &self,
        provider_key: &str,
        api_key: &str,
    ) -> Option<Arc<RainyClient>> {
        let mut pool = self.client_pool.write().await;

        // Check if we have a valid cached client (less than 5 minutes old)
        if let Some(pooled) = pool.get(provider_key) {
            if pooled.created_at.elapsed() < Duration::from_secs(300) {
                return Some(pooled.client.clone());
            }
        }

        // Create new client
        let client = RainyClient::with_api_key(api_key).ok()?;
        let client = Arc::new(client);

        // Cache the client
        pool.insert(
            provider_key.to_string(),
            PooledClient {
                client: client.clone(),
                created_at: Instant::now(),
            },
        );

        Some(client)
    }

    /// Get cowork capabilities (with optimized caching)
    pub async fn get_capabilities(&self) -> CoworkCapabilities {
        println!("🔍 AIProviderManager::get_capabilities() called");

        // Check cache (valid for 5 minutes)
        {
            let cached = self.cached_caps.read().await;
            if let Some(cached) = cached.as_ref() {
                if cached.fetched_at.elapsed().as_secs() < 300 {
                    println!(
                        "📋 Using cached capabilities: plan={}, models={}, can_make_request={}",
                        cached.capabilities.profile.plan.name,
                        cached.capabilities.models.len(),
                        cached.capabilities.can_make_request()
                    );
                    return cached.capabilities.clone();
                }
            }
        }

        // Check for API keys
        let has_cowork_key = self.keychain.get_key("cowork_api").ok().flatten().is_some();
        let has_rainy_key = self.keychain.get_key("rainy_api").ok().flatten().is_some();
        println!(
            "🔑 API Keys available: cowork_api={}, rainy_api={}",
            has_cowork_key, has_rainy_key
        );

        // Fetch from SDK
        if let Some(client) = self.get_cowork_client().await {
            println!("🌐 Cowork client created, fetching capabilities...");
            match client.get_cowork_capabilities().await {
                Ok(mut caps) => {
                    println!(
                        "✅ Raw capabilities from API: plan={}, paid={}, models={}, limit={}/{}",
                        caps.profile.plan.name,
                        caps.profile.plan.is_paid(),
                        caps.models.len(),
                        caps.profile.usage.used,
                        caps.profile.usage.limit
                    );

                    // Fix: Ensure free plan has correct limits and models until server deployment propagates
                    if caps.profile.plan.id == "free" {
                        if caps.profile.usage.limit == 0 {
                            caps.profile.usage.limit = 30;
                            println!("🔧 Applied free plan limit fix: 0 → 30");
                        }
                        // Ensure free users get basic Cowork models
                        if caps.models.is_empty() {
                            caps.models = vec![
                                "gemini-2.5-flash-lite".to_string(),
                                "gemini-flash-lite-latest".to_string(),
                                "llama-3.1-8b-instant".to_string(),
                            ];
                            println!(
                                "🔧 Applied free plan models fix: added {} models",
                                caps.models.len()
                            );
                        }
                    }

                    let final_can_make_request = caps.can_make_request();
                    println!(
                        "🎯 Final capabilities: plan={}, models={}, can_make_request={}",
                        caps.profile.plan.name,
                        caps.models.len(),
                        final_can_make_request
                    );

                    let mut cached = self.cached_caps.write().await;
                    *cached = Some(CachedCapabilities {
                        capabilities: caps.clone(),
                        fetched_at: Instant::now(),
                    });

                    return caps;
                }
                Err(e) => {
                    println!("❌ Failed to fetch cowork capabilities: {}", e);
                }
            }
        } else {
            println!("❌ No cowork client available (no API keys)");
        }

        // Fallback to free plan
        println!("🔄 Using free plan fallback");
        let fallback = CoworkCapabilities::free();
        println!(
            "📋 Fallback capabilities: models={}, can_make_request={}",
            fallback.models.len(),
            fallback.can_make_request()
        );
        fallback
    }

    /// Get cowork models directly from API (efficient with connection reuse)
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
    pub async fn has_paid_plan(&self) -> bool {
        self.get_capabilities().await.profile.plan.is_paid()
    }

    /// Get current plan
    #[allow(dead_code)]
    pub async fn get_plan(&self) -> CoworkPlan {
        self.get_capabilities().await.profile.plan
    }

    /// List available providers based on plan
    pub async fn list_providers(&self) -> Vec<AIProviderConfig> {
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
                model: "gemini-2.5-flash".to_string(),
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
    /// Only returns models that actually exist in the Rainy SDK
    pub async fn get_models(&self, provider: &str) -> Result<Vec<String>, String> {
        match provider {
            "rainy_api" => {
                // Get models from API key if available, otherwise return default SDK models
                // Note: OpenAI and Anthropic models are not currently available via Rainy API
                if let Ok(api_key) = self.keychain.get_key("rainy_api") {
                    if let Some(key) = api_key {
                        if let Ok(client) = RainyClient::with_api_key(&key) {
                            // SDK call may fail (e.g., API format mismatch) - fall through to hardcoded list
                            if let Ok(available) = client.list_available_models().await {
                                // Flatten all models from all providers
                                let mut all_models: Vec<String> = Vec::new();
                                for (_provider_name, models) in available.providers {
                                    all_models.extend(models);
                                }

                                if !all_models.is_empty() {
                                    return Ok(all_models);
                                }
                            }
                        }
                    }
                }

                // Fallback: Return models that are confirmed to exist in the SDK
                // These are the actual models available via the Rainy API
                Ok(vec![
                    // Gemini 3 Series - Advanced reasoning
                    "gemini-3-pro-preview".to_string(),
                    "gemini-3-flash-preview".to_string(),
                    "gemini-3-pro-image-preview".to_string(),
                    // Gemini 2.5 Series - Stable production models
                    "gemini-2.5-pro".to_string(),
                    "gemini-2.5-flash".to_string(),
                    "gemini-2.5-flash-lite".to_string(),
                    // Groq Models - High-speed inference
                    "llama-3.1-8b-instant".to_string(),
                    "llama-3.3-70b-versatile".to_string(),
                    "moonshotai/kimi-k2-instruct-0905".to_string(),
                    // Cerebras Models
                    "cerebras/llama3.1-8b".to_string(),
                    // Enosis Labs Models
                    "astronomer-2-pro".to_string(),
                    "astronomer-2".to_string(),
                    "astronomer-1-5".to_string(),
                    "astronomer-1-max".to_string(),
                    "astronomer-1".to_string(),
                ])
            }
            "cowork_api" => {
                let caps = self.get_capabilities().await;
                Ok(caps.models)
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
    ///
    /// # Arguments
    /// * `provider` - The provider type to use
    /// * `model` - The model name to use
    /// * `prompt` - The prompt text
    /// * `on_progress` - Callback for progress updates (percentage, message)
    /// * `on_token` - Optional callback for streaming tokens (called for each token chunk)
    pub async fn execute_prompt<F, S>(
        &self,
        provider: &ProviderType,
        model: &str,
        prompt: &str,
        on_progress: F,
        on_token: Option<S>,
    ) -> Result<String, String>
    where
        F: Fn(u8, Option<String>) + Send + Sync + 'static,
        S: Fn(String) + Send + Sync + 'static,
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
                let client = self
                    .get_or_create_client("rainy_api", &api_key)
                    .await
                    .ok_or("Failed to create Rainy API client")?;

                match on_token {
                    Some(token_callback) => {
                        // STREAMING PATH
                        on_progress(30, Some("Starting stream...".to_string()));

                        let request =
                            ChatCompletionRequest::new(model, vec![ChatMessage::user(prompt)])
                                .with_stream(true);

                        let mut stream = client
                            .create_chat_completion_stream(request)
                            .await
                            .map_err(|e| e.to_string())?;

                        let mut full_response = String::new();

                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    if let Some(choice) = chunk.choices.first() {
                                        if let Some(content) = &choice.delta.content {
                                            token_callback(content.clone());
                                            full_response.push_str(content);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Stream error: {}", e);
                                    if full_response.is_empty() {
                                        return Err(e.to_string());
                                    }
                                    // Otherwise, return partial response
                                    break;
                                }
                            }
                        }

                        on_progress(100, Some("Complete".to_string()));
                        Ok(full_response)
                    }
                    None => {
                        // EXISTING BLOCKING PATH
                        let result = client
                            .simple_chat(model, prompt)
                            .await
                            .map_err(|e| e.to_string())?;

                        on_progress(100, Some("Complete".to_string()));
                        Ok(result)
                    }
                }
            }
            ProviderType::CoworkApi => {
                println!(
                    "🚀 execute_prompt: Starting Cowork API execution for model '{}'",
                    model
                );

                // Cowork API (supports free tier)
                let caps = self.get_capabilities().await;
                println!(
                    "📊 Capabilities check: plan={}, models_count={}, can_make_request={}",
                    caps.profile.plan.name,
                    caps.models.len(),
                    caps.can_make_request()
                );

                if !caps.can_use_model(model) {
                    println!(
                        "❌ Model '{}' not available. Available models: {:?}",
                        model, caps.models
                    );
                    return Err(format!(
                        "Model {} not available on {} plan",
                        model, caps.profile.plan.name
                    ));
                }
                println!("✅ Model '{}' is available", model);

                if !caps.can_make_request() {
                    println!(
                        "❌ Cannot make request: used={}/{}, upgrade_msg={:?}",
                        caps.profile.usage.used, caps.profile.usage.limit, caps.upgrade_message
                    );
                    if let Some(msg) = &caps.upgrade_message {
                        return Err(msg.clone());
                    }
                    return Err("Usage limit reached. Upgrade for more access.".to_string());
                }
                println!(
                    "✅ Can make request: used={}/{}",
                    caps.profile.usage.used, caps.profile.usage.limit
                );

                // Try cowork_api key first, fallback to rainy_api key (same SDK supports both)
                let api_key = self
                    .keychain
                    .get_key("cowork_api")
                    .ok()
                    .or_else(|| self.keychain.get_key("rainy_api").ok())
                    .ok_or_else(|| "No Cowork API key found".to_string())?;

                println!("🔑 Using API key for Cowork execution");

                on_progress(10, Some("Connecting to Cowork API...".to_string()));
                let client = self
                    .get_or_create_client("cowork_api", api_key.as_ref().unwrap())
                    .await
                    .ok_or("Failed to create Cowork API client")?;
                println!("🔗 Cowork API client created successfully");

                match on_token {
                    Some(token_callback) => {
                        // STREAMING PATH
                        on_progress(30, Some("Starting stream...".to_string()));
                        println!(
                            "📤 Sending streaming chat request: model='{}', prompt_length={}",
                            model,
                            prompt.len()
                        );

                        let request =
                            ChatCompletionRequest::new(model, vec![ChatMessage::user(prompt)])
                                .with_stream(true);

                        let mut stream = client
                            .create_chat_completion_stream(request)
                            .await
                            .map_err(|e| {
                                println!("❌ Stream request failed: {}", e);
                                e.to_string()
                            })?;

                        let mut full_response = String::new();

                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    if let Some(choice) = chunk.choices.first() {
                                        if let Some(content) = &choice.delta.content {
                                            token_callback(content.clone());
                                            full_response.push_str(content);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Stream error: {}", e);
                                    println!("⚠️ Stream error: {}", e);
                                    if full_response.is_empty() {
                                        return Err(e.to_string());
                                    }
                                    // Otherwise, return partial response
                                    break;
                                }
                            }
                        }

                        println!(
                            "✅ Streaming chat request successful, response_length={}",
                            full_response.len()
                        );
                        on_progress(100, Some("Complete".to_string()));
                        Ok(full_response)
                    }
                    None => {
                        // EXISTING BLOCKING PATH
                        on_progress(30, Some("Sending request...".to_string()));
                        println!(
                            "📤 Sending chat request: model='{}', prompt_length={}",
                            model,
                            prompt.len()
                        );
                        let result = client.simple_chat(model, prompt).await.map_err(|e| {
                            println!("❌ Chat request failed: {}", e);
                            e.to_string()
                        })?;

                        println!(
                            "✅ Chat request successful, response_length={}",
                            result.len()
                        );
                        on_progress(100, Some("Complete".to_string()));
                        Ok(result)
                    }
                }
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
    pub async fn can_use_feature(&self, feature: &str) -> bool {
        self.get_capabilities().await.can_use_feature(feature)
    }

    /// Invalidate cached capabilities (call after API key changes)
    pub async fn invalidate_cache(&self) {
        let mut cached = self.cached_caps.write().await;
        *cached = None;

        // Also clear client pool to force reconnection with new keys
        let mut pool = self.client_pool.write().await;
        pool.clear();
    }

    /// Batch multiple API calls for better performance
    /// Reserved for future batch validation feature
    #[allow(dead_code)]
    pub async fn batch_validate_keys(
        &self,
        providers: Vec<(String, String)>,
    ) -> Vec<(String, Result<bool, String>)> {
        let futures = providers.into_iter().map(|(provider, key)| async move {
            let result = self.validate_api_key(&provider, &key).await;
            (provider, result)
        });

        futures::future::join_all(futures).await
    }
}

impl Default for AIProviderManager {
    fn default() -> Self {
        Self::new()
    }
}
