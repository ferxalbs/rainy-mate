// Rainy SDK Provider
// Wrapper around rainy-sdk v0.6.1 for the new provider abstraction layer

use async_trait::async_trait;
use std::sync::Arc;
use crate::ai::provider_types::{
    ProviderId, ProviderType, ProviderConfig, ProviderCapabilities, ProviderHealth,
    ChatCompletionRequest, ChatCompletionResponse,
    EmbeddingRequest, EmbeddingResponse,
    StreamingChunk, StreamingCallback,
    ProviderResult, AIError, ChatMessage,
};
use crate::ai::provider_trait::{AIProvider, AIProviderFactory};
use rainy_sdk::RainyClient;

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
        let api_key = config.api_key.as_ref()
            .ok_or_else(|| AIError::Authentication("API key is required".to_string()))?;

        let client = RainyClient::with_api_key(api_key)
            .map_err(|e| AIError::Authentication(format!("Failed to create Rainy client: {}", e)))?;

        Ok(Self {
            config,
            client,
            cached_capabilities: tokio::sync::RwLock::new(None),
        })
    }

    /// Get the rainy client
    pub fn client(&self) -> &RainyClient {
        &self.client
    }

    /// Convert chat messages to a single prompt string
    fn convert_messages_to_prompt(messages: &[ChatMessage]) -> String {
        messages.iter()
            .map(|msg| {
                let role = match msg.role.as_str() {
                    "system" => "System",
                    "user" => "User",
                    "assistant" => "Assistant",
                    _ => "User",
                };
                format!("{}: {}", role, msg.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Check if this is a Cowork API key
    fn is_cowork_key(api_key: &str) -> bool {
        api_key.starts_with("ra-cowork")
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

        // Fetch capabilities from rainy-sdk
        let is_cowork = self.config.api_key.as_ref()
            .map(|k| Self::is_cowork_key(k))
            .unwrap_or(false);

        let capabilities = if is_cowork {
            // Get Cowork capabilities
            let caps = self.client.get_cowork_capabilities().await
                .map_err(|e| AIError::APIError(format!("Failed to get Cowork capabilities: {}", e)))?;

            ProviderCapabilities {
                chat_completions: true,
                embeddings: true,
                streaming: false, // Not yet supported in rainy-sdk
                function_calling: true,
                vision: caps.features.image_analysis,
                web_search: caps.features.web_research,
                max_context_tokens: 128000, // Gemini 2.5 Pro
                max_output_tokens: 8192,
                models: caps.models,
            }
        } else {
            // Rainy API mode - use default capabilities
            ProviderCapabilities {
                chat_completions: true,
                embeddings: true,
                streaming: false, // Not yet supported in rainy-sdk
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
            }
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

    async fn complete(&self, request: ChatCompletionRequest) -> ProviderResult<ChatCompletionResponse> {
        let prompt = Self::convert_messages_to_prompt(&request.messages);

        let response = self.client.simple_chat(&request.model, &prompt).await
            .map_err(|e| AIError::APIError(format!("Chat completion failed: {}", e)))?;

        Ok(ChatCompletionResponse {
            content: response,
            model: request.model.clone(),
            usage: crate::ai::provider_types::TokenUsage {
                prompt_tokens: 0, // rainy-sdk doesn't provide token counts
                completion_tokens: 0,
                total_tokens: 0,
            },
            finish_reason: "stop".to_string(),
        })
    }

    async fn complete_stream(
        &self,
        _request: ChatCompletionRequest,
        _callback: StreamingCallback,
    ) -> ProviderResult<()> {
        // Streaming not yet supported in rainy-sdk
        Err(AIError::UnsupportedCapability("Streaming not yet supported in rainy-sdk".to_string()))
    }

    async fn embed(&self, request: EmbeddingRequest) -> ProviderResult<EmbeddingResponse> {
        // Embeddings not yet supported in rainy-sdk
        Err(AIError::UnsupportedCapability("Embeddings not yet supported in rainy-sdk".to_string()))
    }

    fn supports_capability(&self, capability: &str) -> bool {
        match capability {
            "chat_completions" | "function_calling" | "web_search" => true,
            "streaming" | "embeddings" => false, // Not yet supported
            _ => false,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cowork_key() {
        assert!(RainySDKProvider::is_cowork_key("ra-cowork12345678901234567890123456789012345678901234567890"));
        assert!(!RainySDKProvider::is_cowork_key("ra-1234567890"));
    }

    #[test]
    fn test_convert_messages_to_prompt() {
        let messages = vec![
            ChatMessage::system("You are a helpful assistant"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let prompt = RainySDKProvider::convert_messages_to_prompt(&messages);
        assert!(prompt.contains("System: You are a helpful assistant"));
        assert!(prompt.contains("User: Hello"));
        assert!(prompt.contains("Assistant: Hi there!"));
    }
}
