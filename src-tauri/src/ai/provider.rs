// Rainy Cowork - AI Provider Trait and Manager
// Unified abstraction layer using rainy-sdk for premium features

use crate::ai::provider_trait::AIProvider;
use crate::ai::provider_types::{ProviderConfig, ProviderId};
use crate::ai::providers::moonshot::MoonshotProvider;
use crate::ai::{gemini::GeminiProvider, keychain::KeychainManager};
use crate::models::{AIProviderConfig, ProviderType};
use futures::StreamExt;
use rainy_sdk::{
    models::{ThinkingConfig, ThinkingLevel},
    ChatCompletionRequest, ChatMessage, RainyClient,
};
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

/// Manager for AI providers - uses rainy-sdk for paid plans, Gemini for free tier
pub struct AIProviderManager {
    keychain: KeychainManager,
    gemini: GeminiProvider,

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
            client_pool: Arc::new(RwLock::new(HashMap::new())),
            http_client,
        }
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

    pub async fn list_providers(&self) -> Vec<AIProviderConfig> {
        let mut providers = vec![];

        // Always show Gemini (free tier fallback)
        providers.push(AIProviderConfig {
            provider: ProviderType::Gemini,
            name: "Google Gemini".to_string(),
            model: "gemini-2.5-flash".to_string(),
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
                model: "gemini-2.5-flash".to_string(),
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
        S: Fn(crate::ai::provider_types::StreamingChunk) + Send + Sync + 'static,
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
