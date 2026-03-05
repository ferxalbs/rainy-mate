// Rainy Cowork - AI Provider Trait and Manager
// Unified abstraction layer using rainy-sdk for premium features

use crate::ai::provider_trait::AIProvider;
use crate::ai::provider_types::{ProviderConfig, ProviderId};
use crate::ai::providers::moonshot::MoonshotProvider;
use crate::ai::{gemini::GeminiProvider, keychain::KeychainManager};
use crate::models::{AIProviderConfig, ProviderType};
use futures::StreamExt;
use rainy_sdk::{
    models::{ModelCatalogItem, ThinkingConfig, ThinkingLevel},
    ChatCompletionRequest, ChatMessage, RainyClient, DEFAULT_BASE_URL,
};
use serde::Deserialize;
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

/// Pooled RainyClient with connection reuse
#[derive(Clone)]
struct PooledClient {
    client: Arc<RainyClient>,
    created_at: Instant,
}

#[derive(Clone)]
struct CachedModelCatalog {
    models: Vec<ModelCatalogItem>,
    fetched_at: Instant,
}

/// Manager for AI providers - uses rainy-sdk for paid plans, Gemini for free tier
pub struct AIProviderManager {
    keychain: KeychainManager,
    gemini: GeminiProvider,

    /// Connection pool for RainyClient instances
    client_pool: Arc<RwLock<HashMap<String, PooledClient>>>,
    /// Cached Rainy model catalogs keyed by provider.
    model_catalog_cache: Arc<RwLock<HashMap<String, CachedModelCatalog>>>,
    /// HTTP client with optimized settings
    #[allow(dead_code)]
    http_client: reqwest::Client,
}

impl AIProviderManager {
    fn provider_aliases(provider: &str) -> &'static [&'static str] {
        match provider {
            "rainy_api" => &["rainy_api", "rainyapi", "cowork_api"],
            "gemini" => &["gemini", "gemini_byok"],
            _ => &[],
        }
    }

    fn provider_env_keys(provider: &str) -> &'static [&'static str] {
        match provider {
            "rainy_api" => &["RAINY_API_KEY", "COWORK_API_KEY"],
            "gemini" => &["GEMINI_API_KEY"],
            _ => &[],
        }
    }

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
            client_pool: Arc::new(RwLock::new(HashMap::new())),
            model_catalog_cache: Arc::new(RwLock::new(HashMap::new())),
            http_client,
        }
    }

    const MODEL_CACHE_TTL: Duration = Duration::from_secs(300);

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

    async fn invalidate_model_cache(&self, provider_key: &str) {
        let mut cache = self.model_catalog_cache.write().await;
        cache.remove(provider_key);
    }

    async fn resolve_api_key(&self, provider: &str) -> Result<Option<String>, String> {
        for alias in Self::provider_aliases(provider) {
            if let Some(key) = self.keychain.get_key(alias)? {
                return Ok(Some(key));
            }
        }

        for env_key in Self::provider_env_keys(provider) {
            if let Ok(value) = std::env::var(env_key) {
                if !value.trim().is_empty() {
                    return Ok(Some(value));
                }
            }
        }

        Ok(None)
    }

    fn rainy_base_url(&self) -> String {
        std::env::var("RAINY_API_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("COWORK_API_BASE_URL")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
    }

    async fn fetch_public_rainy_model_catalog(&self) -> Result<Vec<ModelCatalogItem>, String> {
        #[derive(Debug, Deserialize)]
        struct ModelListData<T> {
            data: Vec<T>,
        }

        #[derive(Debug, Deserialize)]
        struct Envelope<T> {
            data: ModelListData<T>,
        }

        #[derive(Debug, Deserialize)]
        struct PublicModelItem {
            id: String,
            #[allow(dead_code)]
            object: Option<String>,
            #[allow(dead_code)]
            created: Option<i64>,
            #[allow(dead_code)]
            owned_by: Option<String>,
            #[allow(dead_code)]
            root: Option<String>,
            #[allow(dead_code)]
            parent: Option<String>,
        }

        let base_url = self.rainy_base_url().trim_end_matches('/').to_string();
        let catalog_url = format!("{base_url}/api/v1/models/catalog");
        let public_models_url = format!("{base_url}/api/v1/models");

        let catalog_attempt = self
            .http_client
            .get(&catalog_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if catalog_attempt.status().is_success() {
            let envelope = catalog_attempt
                .json::<Envelope<ModelCatalogItem>>()
                .await
                .map_err(|e| e.to_string())?;
            if !envelope.data.data.is_empty() {
                return Ok(envelope.data.data);
            }
        } else {
            tracing::warn!(
                "[AIProviderManager] Public Rainy model catalog fetch failed: {}",
                catalog_attempt.status()
            );
        }

        let public_models_response = self
            .http_client
            .get(&public_models_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !public_models_response.status().is_success() {
            return Err(format!(
                "Public Rainy model list fetch failed with status {}",
                public_models_response.status()
            ));
        }

        let envelope = public_models_response
            .json::<Envelope<PublicModelItem>>()
            .await
            .map_err(|e| e.to_string())?;

        Ok(envelope
            .data
            .data
            .into_iter()
            .map(|item| ModelCatalogItem {
                id: item.id,
                ..Default::default()
            })
            .collect())
    }

    pub async fn get_models_catalog(&self, provider: &str) -> Result<Vec<ModelCatalogItem>, String> {
        match provider {
            "rainy_api" => {
                {
                    let cache = self.model_catalog_cache.read().await;
                    if let Some(entry) = cache.get(provider) {
                        if entry.fetched_at.elapsed() < Self::MODEL_CACHE_TTL {
                            return Ok(entry.models.clone());
                        }
                    }
                }

                let models = match self.fetch_public_rainy_model_catalog().await {
                    Ok(models) if !models.is_empty() => models,
                    Ok(_) | Err(_) => {
                        let api_key = self
                            .resolve_api_key("rainy_api")
                            .await?
                            .ok_or_else(|| "No public Rainy model catalog and no Rainy API key found".to_string())?;

                        let client = self
                            .get_or_create_client("rainy_api", &api_key)
                            .await
                            .ok_or_else(|| "Failed to create Rainy API client".to_string())?;

                        client
                            .get_models_catalog()
                            .await
                            .map_err(|e| e.to_string())?
                    }
                };

                let mut cache = self.model_catalog_cache.write().await;
                cache.insert(
                    provider.to_string(),
                    CachedModelCatalog {
                        models: models.clone(),
                        fetched_at: Instant::now(),
                    },
                );

                Ok(models)
            }
            _ => Err(AIError::ProviderNotAvailable(provider.to_string()).to_string()),
        }
    }

    pub async fn list_providers(&self) -> Vec<AIProviderConfig> {
        let mut providers = vec![];

        // Always show Gemini (free tier fallback)
        providers.push(AIProviderConfig {
            provider: ProviderType::Gemini,
            name: "Google Gemini".to_string(),
            model: "gemini-3-flash-preview".to_string(),
            is_available: true,
            requires_api_key: true,
        });

        // Show Moonshot AI (Kimi)
        providers.push(AIProviderConfig {
            provider: ProviderType::Moonshot,
            name: "Moonshot AI (Kimi)".to_string(),
            model: "kimi-k2.5".to_string(),
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
                model: "gemini-3-flash-preview".to_string(),
                is_available: true,
                requires_api_key: true,
            },
        );

        providers
    }

    /// Get available models based on plan
    /// Only returns models that actually exist in the Rainy SDK
    pub async fn get_models(&self, provider: &str) -> Result<Vec<String>, String> {
        match provider {
            "rainy_api" => {
                if let Ok(catalog) = self.get_models_catalog("rainy_api").await {
                    let mut models: Vec<String> = catalog.into_iter().map(|item| item.id).collect();
                    models.sort();
                    models.dedup();
                    if !models.is_empty() {
                        return Ok(models);
                    }
                }

                if let Ok(api_key) = self.keychain.get_key("rainy_api") {
                    if let Some(key) = api_key {
                        if let Ok(client) = RainyClient::with_api_key(&key) {
                            if let Ok(available) = client.list_available_models().await {
                                let mut all_models: Vec<String> =
                                    available.providers.into_values().flatten().collect();
                                all_models.sort();
                                all_models.dedup();
                                if !all_models.is_empty() {
                                    return Ok(all_models);
                                }
                            }
                        }
                    }
                }

                tracing::warn!(
                    "[AIProviderManager] Rainy model catalog unavailable and no dynamic model list could be fetched; returning empty model set instead of stale legacy defaults"
                );
                Ok(Vec::new())
            }

            "gemini" => Ok(self.gemini.available_models()),
            "moonshot" => Ok(vec![
                "moonshot-v1-8k".to_string(),
                "moonshot-v1-32k".to_string(),
                "moonshot-v1-128k".to_string(),
                "kimi-k2.5".to_string(),
            ]),
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

            "gemini" => self
                .gemini
                .validate_key(api_key)
                .await
                .map_err(|e| e.to_string()),

            "moonshot" => {
                // Validate Moonshot key by creating a temporary provider
                let config = ProviderConfig {
                    id: ProviderId::new("temp"),
                    provider_type: crate::ai::provider_types::ProviderType::Moonshot,
                    api_key: Some(api_key.to_string()),
                    model: "moonshot-v1-8k".to_string(),
                    ..Default::default()
                };

                let provider = MoonshotProvider::new(config).map_err(|e| e.to_string())?;

                match provider.health_check().await {
                    Ok(health) => match health {
                        crate::ai::provider_types::ProviderHealth::Healthy => Ok(true),
                        _ => Err("Moonshot API is not healthy".to_string()),
                    },
                    Err(e) => Err(e.to_string()),
                }
            }
            _ => Err(AIError::ProviderNotAvailable(provider.to_string()).to_string()),
        }
    }

    /// Store API key in macOS Keychain
    pub async fn store_api_key(&self, provider: &str, api_key: &str) -> Result<(), String> {
        self.keychain.store_key(provider, api_key)?;
        self.invalidate_model_cache(provider).await;
        Ok(())
    }

    /// Get API key from macOS Keychain
    pub async fn get_api_key(&self, provider: &str) -> Result<Option<String>, String> {
        self.resolve_api_key(provider).await
    }

    /// Delete API key from macOS Keychain
    pub async fn delete_api_key(&self, provider: &str) -> Result<(), String> {
        let aliases = Self::provider_aliases(provider);
        if aliases.is_empty() {
            self.keychain.delete_key(provider)?;
        } else {
            for alias in aliases {
                self.keychain.delete_key(alias)?;
            }
        }
        self.invalidate_model_cache(provider).await;
        Ok(())
    }

    /// Check if API key exists for a provider
    pub async fn has_api_key(&self, provider: &str) -> Result<bool, String> {
        Ok(self.resolve_api_key(provider).await?.is_some())
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
        S: Fn(crate::ai::provider_types::StreamingChunk) + Send + Sync + 'static,
    {
        crate::ai::model_catalog::ensure_supported_model_slug(model)?;
        match provider {
            ProviderType::RainyApi => {
                // Standard Rainy API (Pay-as-you-go)
                // Just needs a valid key, no plan checks
                let api_key = self
                    .resolve_api_key("rainy_api")
                    .await?
                    .ok_or("No Rainy API key found")?;

                on_progress(10, Some("Connecting to Rainy API...".to_string()));
                let client = self
                    .get_or_create_client("rainy_api", &api_key)
                    .await
                    .ok_or("Failed to create Rainy API client")?;

                let (real_model_id, thinking_config) = Self::map_model_id(model);
                match on_token {
                    Some(token_callback) => {
                        // STREAMING PATH
                        on_progress(30, Some("Starting stream...".to_string()));

                        let mut request = ChatCompletionRequest::new(
                            real_model_id.clone(),
                            vec![ChatMessage::user(prompt)],
                        )
                        .with_stream(true);

                        if let Some(config) = thinking_config.clone() {
                            request = request.with_thinking_config(config);
                        }

                        let mut stream = client
                            .create_chat_completion_stream(request)
                            .await
                            .map_err(|e| e.to_string())?;

                        let mut full_response = String::new();

                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    if let Some(choice) = chunk.choices.first() {
                                        let content =
                                            choice.delta.content.clone().unwrap_or_default();
                                        let thought = choice.delta.thought.clone();

                                        if !content.is_empty() || thought.is_some() {
                                            let chunk_data =
                                                crate::ai::provider_types::StreamingChunk {
                                                    content: content.clone(),
                                                    thought,
                                                    is_final: false,
                                                    finish_reason: None,
                                                };
                                            token_callback(chunk_data);
                                            full_response.push_str(&content);
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
                        let mut request = ChatCompletionRequest::new(
                            real_model_id,
                            vec![ChatMessage::user(prompt)],
                        );

                        if let Some(config) = thinking_config {
                            request = request.with_thinking_config(config);
                        }

                        let (response, _) = client
                            .chat_completion(request)
                            .await
                            .map_err(|e| e.to_string())?;

                        let result = response
                            .choices
                            .first()
                            .map(|c| c.message.content.clone())
                            .unwrap_or_default();

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

            ProviderType::Moonshot => {
                let api_key = self
                    .get_api_key("moonshot")
                    .await?
                    .ok_or_else(|| "No Moonshot API key configured".to_string())?;

                let config = ProviderConfig {
                    id: ProviderId::new("moonshot"),
                    provider_type: crate::ai::provider_types::ProviderType::Moonshot,
                    api_key: Some(api_key),
                    model: model.to_string(),
                    ..Default::default()
                };

                let provider = MoonshotProvider::new(config).map_err(|e| e.to_string())?;

                // Use streaming if callback provided, else blocking
                match on_token {
                    Some(token_callback) => {
                        on_progress(30, Some("Streaming response...".to_string()));

                        let request = crate::ai::provider_types::ChatCompletionRequest {
                            model: model.to_string(),
                            messages: vec![crate::ai::provider_types::ChatMessage::user(prompt)],
                            stream: true,
                            ..Default::default()
                        };

                        provider
                            .complete_stream(request, Arc::new(token_callback))
                            .await
                            .map(|_| String::new()) // Return empty string as token callback handled it
                            .map_err(|e| e.to_string())
                    }
                    None => {
                        on_progress(50, Some("Generating response...".to_string()));

                        let request = crate::ai::provider_types::ChatCompletionRequest {
                            model: model.to_string(),
                            messages: vec![crate::ai::provider_types::ChatMessage::user(prompt)],
                            stream: false,
                            ..Default::default()
                        };

                        let response = provider
                            .complete(request)
                            .await
                            .map_err(|e| e.to_string())?;

                        Ok(response.content.unwrap_or_default())
                    }
                }
            }
        }
    }

    /// Invalidate cached capabilities (call after API key changes)
    pub async fn invalidate_cache(&self) {
        let mut pool = self.client_pool.write().await;
        pool.clear();
        drop(pool);
        let mut catalog_cache = self.model_catalog_cache.write().await;
        catalog_cache.clear();
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

    /// Map virtual model IDs to real model IDs and thinking config
    fn map_model_id(model_id: &str) -> (String, Option<ThinkingConfig>) {
        let mut thinking_config = ThinkingConfig::default();
        thinking_config.include_thoughts = Some(true); // Always include thoughts for thinking models

        match model_id {
            // Gemini 3 Flash mappings
            "gemini-3-flash-minimal" => {
                thinking_config.thinking_level = Some(ThinkingLevel::Minimal);
                ("gemini-3-flash-preview".to_string(), Some(thinking_config))
            }
            "gemini-3-flash-low" => {
                thinking_config.thinking_level = Some(ThinkingLevel::Low);
                ("gemini-3-flash-preview".to_string(), Some(thinking_config))
            }
            "gemini-3-flash-medium" => {
                thinking_config.thinking_level = Some(ThinkingLevel::Medium);
                ("gemini-3-flash-preview".to_string(), Some(thinking_config))
            }
            "gemini-3-flash-high" => {
                thinking_config.thinking_level = Some(ThinkingLevel::High);
                ("gemini-3-flash-preview".to_string(), Some(thinking_config))
            }
            // Default for base model ID
            "gemini-3-flash-preview" => {
                thinking_config.thinking_level = Some(ThinkingLevel::Medium);
                ("gemini-3-flash-preview".to_string(), Some(thinking_config))
            }

            // Gemini 3 Pro mappings
            "gemini-3-pro-low" => {
                thinking_config.thinking_level = Some(ThinkingLevel::Low);
                ("gemini-3-pro-preview".to_string(), Some(thinking_config))
            }
            "gemini-3-pro-high" => {
                thinking_config.thinking_level = Some(ThinkingLevel::High);
                ("gemini-3-pro-preview".to_string(), Some(thinking_config))
            }
            // Default for base model ID
            "gemini-3-pro-preview" => {
                thinking_config.thinking_level = Some(ThinkingLevel::Medium);
                ("gemini-3-pro-preview".to_string(), Some(thinking_config))
            }

            // Pass through others
            _ => (model_id.to_string(), None),
        }
    }
}

impl Default for AIProviderManager {
    fn default() -> Self {
        Self::new()
    }
}
