// Rainy SDK Provider
// Wrapper around rainy-sdk v0.6.1 for the new provider abstraction layer

use crate::ai::provider_trait::{AIProvider, AIProviderFactory};
use crate::ai::provider_types::{
    AIError, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, EmbeddingRequest,
    EmbeddingResponse, ProviderCapabilities, ProviderConfig, ProviderHealth, ProviderId,
    ProviderResult, ProviderType, StreamingCallback,
};
use async_trait::async_trait;
use futures::StreamExt;
use rainy_sdk::RainyClient;
use std::sync::Arc;

/// Rainy SDK provider
pub struct RainySDKProvider {
    /// Provider configuration
    config: ProviderConfig,
    /// Rainy client
    client: RainyClient,
    /// Cached capabilities
    cached_capabilities: tokio::sync::RwLock<Option<ProviderCapabilities>>,
}

impl RainySDKProvider {
    /// Create a new Rainy SDK provider
    pub fn new(config: ProviderConfig) -> ProviderResult<Self> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| AIError::Authentication("API key is required".to_string()))?;

        let client = RainyClient::with_api_key(api_key).map_err(|e| {
            AIError::Authentication(format!("Failed to create Rainy client: {}", e))
        })?;

        Ok(Self {
            config,
            client,
            cached_capabilities: tokio::sync::RwLock::new(None),
        })
    }

    /// Map virtual model IDs to real model IDs and thinking config
    fn map_model_id(model_id: &str) -> (String, Option<rainy_sdk::models::ThinkingConfig>) {
        use rainy_sdk::models::{ThinkingConfig, ThinkingLevel};

        let mut thinking_config = ThinkingConfig::default();
        thinking_config.include_thoughts = Some(true); // Always include thoughts for thinking models

        // Strip any prefix (e.g., "rainy:", "cowork:") from model_id
        // The Rainy API expects clean model IDs without provider prefixes
        let clean_id = if let Some(colon_pos) = model_id.find(':') {
            &model_id[colon_pos + 1..]
        } else {
            model_id
        };

        match clean_id {
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

            // Gemini 3 Pro mappings
            "gemini-3-pro-low" => {
                thinking_config.thinking_level = Some(ThinkingLevel::Low);
                ("gemini-3-pro-preview".to_string(), Some(thinking_config))
            }
            "gemini-3-pro-high" => {
                thinking_config.thinking_level = Some(ThinkingLevel::High);
                ("gemini-3-pro-preview".to_string(), Some(thinking_config))
            }

            // Pass through others
            _ => (clean_id.to_string(), None),
        }
    }
}

#[async_trait]
impl AIProvider for RainySDKProvider {
    fn id(&self) -> &ProviderId {
        &self.config.id
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::RainySDK
    }

    async fn capabilities(&self) -> ProviderResult<ProviderCapabilities> {
        // Check cache first
        {
            let cache = self.cached_capabilities.read().await;
            if let Some(caps) = cache.as_ref() {
                return Ok(caps.clone());
            }
        }

        let capabilities = ProviderCapabilities {
            chat_completions: true,
            embeddings: true,
            streaming: true,
            function_calling: true,
            vision: true,
            web_search: true,
            max_context_tokens: 128000,
            max_output_tokens: 8192,
            models: vec![
                "gemini-2.5-pro".to_string(),
                "gemini-2.0-flash".to_string(),
                "gemini-1.5-flash".to_string(),
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "claude-3.5-sonnet".to_string(),
                "grok-2".to_string(),
            ],
        };

        // Cache for 5 minutes
        let mut cache = self.cached_capabilities.write().await;
        *cache = Some(capabilities.clone());

        Ok(capabilities)
    }

    async fn health_check(&self) -> ProviderResult<ProviderHealth> {
        // Try a simple chat completion to check health
        let request = ChatCompletionRequest {
            messages: vec![ChatMessage::user("Hello")],
            model: self.config.model.clone(),
            max_tokens: Some(10),
            ..Default::default()
        };

        match self.complete(request).await {
            Ok(_) => Ok(ProviderHealth::Healthy),
            Err(e) => {
                // Check if it's a rate limit error (degraded)
                if matches!(e, AIError::RateLimit(_)) {
                    Ok(ProviderHealth::Degraded)
                } else {
                    Ok(ProviderHealth::Unhealthy)
                }
            }
        }
    }

    async fn complete(
        &self,
        request: ChatCompletionRequest,
    ) -> ProviderResult<ChatCompletionResponse> {
        let (model_id, thinking_config) = Self::map_model_id(&request.model);

        // Convert strict ChatMessage to rainy-sdk ChatMessage
        let messages = request
            .messages
            .iter()
            .map(|msg| match msg.role.as_str() {
                "system" => rainy_sdk::models::ChatMessage::system(&msg.content),
                "user" => rainy_sdk::models::ChatMessage::user(&msg.content),
                "assistant" => rainy_sdk::models::ChatMessage::assistant(&msg.content),
                _ => rainy_sdk::models::ChatMessage::user(&msg.content),
            })
            .collect();

        // Build request
        let mut sdk_request =
            rainy_sdk::models::ChatCompletionRequest::new(model_id.clone(), messages);

        if let Some(config) = thinking_config {
            sdk_request = sdk_request.with_thinking_config(config);
        }

        if let Some(max_tokens) = request.max_tokens {
            sdk_request = sdk_request.with_max_tokens(max_tokens);
        }

        if let Some(temperature) = request.temperature {
            sdk_request = sdk_request.with_temperature(temperature);
        }

        let (response, _) = self
            .client
            .chat_completion(sdk_request)
            .await
            .map_err(|e| AIError::APIError(format!("Chat completion failed: {}", e)))?;

        // Extract content from first choice
        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(ChatCompletionResponse {
            content: Some(content),
            tool_calls: None,
            model: request.model.clone(),
            usage: {
                let (prompt, completion, total) = match response.usage.as_ref() {
                    Some(u) => (u.prompt_tokens, u.completion_tokens, u.total_tokens),
                    None => (0, 0, 0),
                };
                crate::ai::provider_types::TokenUsage {
                    prompt_tokens: prompt,
                    completion_tokens: completion,
                    total_tokens: total,
                }
            },
            finish_reason: "stop".to_string(),
        })
    }

    async fn complete_stream(
        &self,
        request: ChatCompletionRequest,
        callback: StreamingCallback,
    ) -> ProviderResult<()> {
        let (model_id, thinking_config) = Self::map_model_id(&request.model);

        // Convert strict ChatMessage to rainy-sdk ChatMessage
        let messages = request
            .messages
            .iter()
            .map(|msg| match msg.role.as_str() {
                "system" => rainy_sdk::models::ChatMessage::system(&msg.content),
                "user" => rainy_sdk::models::ChatMessage::user(&msg.content),
                "assistant" => rainy_sdk::models::ChatMessage::assistant(&msg.content),
                _ => rainy_sdk::models::ChatMessage::user(&msg.content),
            })
            .collect();

        // Build request
        let mut sdk_request =
            rainy_sdk::models::ChatCompletionRequest::new(model_id.clone(), messages)
                .with_stream(true);

        if let Some(config) = thinking_config {
            sdk_request = sdk_request.with_thinking_config(config);
        }

        if let Some(max_tokens) = request.max_tokens {
            sdk_request = sdk_request.with_max_tokens(max_tokens);
        }

        if let Some(temperature) = request.temperature {
            sdk_request = sdk_request.with_temperature(temperature);
        }

        let mut stream = self
            .client
            .chat_completion_stream(sdk_request)
            .await
            .map_err(|e| AIError::APIError(format!("Stream initialization failed: {}", e)))?;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Some(choice) = chunk.choices.first() {
                        let content = choice.delta.content.clone().unwrap_or_default();
                        let thought = choice.delta.thought.clone();
                        let finish_reason = choice.finish_reason.clone();
                        let is_final = finish_reason.is_some() || chunk.choices.is_empty();

                        let streaming_chunk = crate::ai::provider_types::StreamingChunk {
                            content,
                            thought,
                            is_final,
                            finish_reason,
                        };

                        callback(streaming_chunk);
                    }
                }
                Err(e) => {
                    return Err(AIError::APIError(format!("Stream error: {}", e)));
                }
            }
        }

        Ok(())
    }

    async fn embed(&self, _request: EmbeddingRequest) -> ProviderResult<EmbeddingResponse> {
        // Embeddings not yet supported in rainy-sdk
        Err(AIError::UnsupportedCapability(
            "Embeddings not yet supported in rainy-sdk".to_string(),
        ))
    }

    fn default_model(&self) -> &str {
        &self.config.model
    }

    async fn available_models(&self) -> ProviderResult<Vec<String>> {
        let capabilities = self.capabilities().await?;
        Ok(capabilities.models)
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }
}

/// Rainy SDK provider factory
pub struct RainySDKProviderFactory;

#[async_trait]
impl AIProviderFactory for RainySDKProviderFactory {
    async fn create(config: ProviderConfig) -> ProviderResult<Arc<dyn AIProvider>> {
        Self::validate_config(&config)?;
        Ok(Arc::new(RainySDKProvider::new(config)?))
    }

    fn validate_config(config: &ProviderConfig) -> ProviderResult<()> {
        if config.api_key.is_none() {
            return Err(AIError::Authentication("API key is required".to_string()));
        }

        if config.model.is_empty() {
            return Err(AIError::InvalidRequest("Model is required".to_string()));
        }

        Ok(())
    }
}
