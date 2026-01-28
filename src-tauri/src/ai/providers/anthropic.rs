// Anthropic Provider
// Direct integration with Anthropic API for Claude 3.5/4, Opus, Sonnet, Haiku models

use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use crate::ai::provider_types::{
    ProviderId, ProviderType, ProviderConfig, ProviderCapabilities, ProviderHealth,
    ChatCompletionRequest, ChatCompletionResponse,
    EmbeddingRequest, EmbeddingResponse,
    StreamingChunk, StreamingCallback,
    ProviderResult, AIError, ChatMessage, TokenUsage,
};
use crate::ai::provider_trait::{AIProvider, AIProviderFactory};
use serde::{Deserialize, Serialize};

/// Anthropic API base URL
const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";
/// Anthropic API version
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// Anthropic provider
pub struct AnthropicProvider {
    /// Provider configuration
    config: ProviderConfig,
    /// HTTP client
    client: reqwest::Client,
    /// API key
    api_key: String,
    /// Base URL (can be customized)
    base_url: String,
}

/// Anthropic message format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic API request body
#[derive(Debug, Serialize)]
struct AnthropicChatRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
}

/// Anthropic API response
#[derive(Debug, Deserialize)]
struct AnthropicChatResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContentBlock>,
    model: String,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

/// Anthropic content block
#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

/// Anthropic token usage
#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic streaming event
#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(flatten)]
    data: serde_json::Value,
}

/// Anthropic error response
#[derive(Debug, Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    error_type: String,
    error: AnthropicErrorDetail,
}

/// Anthropic error detail
#[derive(Debug, Deserialize)]
struct AnthropicErrorDetail {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

/// Content block delta for streaming
#[derive(Debug, Deserialize)]
struct ContentBlockDelta {
    #[serde(rename = "type")]
    delta_type: String,
    text: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(config: ProviderConfig) -> ProviderResult<Self> {
        let api_key = config.api_key.clone()
            .ok_or_else(|| AIError::Authentication("API key is required".to_string()))?;

        let base_url = config.base_url.clone()
            .unwrap_or_else(|| ANTHROPIC_API_BASE.to_string());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .map_err(|e| AIError::Configuration(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            api_key,
            base_url,
        })
    }

    /// Get the HTTP client
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Convert chat messages to Anthropic format
    fn convert_messages(messages: &[ChatMessage]) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system_message = None;
        let mut anthropic_messages = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    system_message = Some(msg.content.clone());
                }
                "user" | "assistant" => {
                    anthropic_messages.push(AnthropicMessage {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                    });
                }
                _ => {
                    // Default to user for unknown roles
                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: msg.content.clone(),
                    });
                }
            }
        }

        (system_message, anthropic_messages)
    }

    /// Map Anthropic error to AIError
    fn map_error(status: reqwest::StatusCode, error: AnthropicError) -> AIError {
        match status {
            reqwest::StatusCode::UNAUTHORIZED => AIError::Authentication(error.error.message),
            reqwest::StatusCode::TOO_MANY_REQUESTS => AIError::RateLimit(error.error.message),
            reqwest::StatusCode::BAD_REQUEST => AIError::InvalidRequest(error.error.message),
            reqwest::StatusCode::SERVICE_UNAVAILABLE => AIError::APIError(format!("Service unavailable: {}", error.error.message)),
            _ => AIError::APIError(format!("Anthropic API error: {}", error.error.message)),
        }
    }

    /// Get available models based on capabilities
    fn available_models() -> Vec<String> {
        vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ]
    }

    /// Get default max tokens for model
    fn default_max_tokens(model: &str) -> u32 {
        if model.contains("opus") {
            4096
        } else {
            8192
        }
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    fn id(&self) -> &ProviderId {
        &self.config.id
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }

    async fn capabilities(&self) -> ProviderResult<ProviderCapabilities> {
        Ok(ProviderCapabilities {
            chat_completions: true,
            embeddings: false, // Anthropic doesn't provide embeddings
            streaming: true,
            function_calling: true,
            vision: true, // Claude 3.5 Sonnet supports vision
            web_search: false, // Not directly supported
            max_context_tokens: 200000, // Claude 3.5 Sonnet context window
            max_output_tokens: 8192,
            models: Self::available_models(),
        })
    }

    async fn health_check(&self) -> ProviderResult<ProviderHealth> {
        // Try a simple request to check health
        let request = ChatCompletionRequest {
            messages: vec![ChatMessage::user("Hi")],
            model: "claude-3-haiku-20240307".to_string(), // Use cheapest model for health check
            max_tokens: Some(10),
            ..Default::default()
        };

        match self.complete(request).await {
            Ok(_) => Ok(ProviderHealth::Healthy),
            Err(AIError::RateLimit(_)) => Ok(ProviderHealth::Degraded),
            Err(_) => Ok(ProviderHealth::Unhealthy),
        }
    }

    async fn complete(&self, request: ChatCompletionRequest) -> ProviderResult<ChatCompletionResponse> {
        let (system, messages) = Self::convert_messages(&request.messages);

        if messages.is_empty() {
            return Err(AIError::InvalidRequest("At least one message is required".to_string()));
        }

        let anthropic_request = AnthropicChatRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens.or_else(|| Some(Self::default_max_tokens(&request.model))),
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: request.stop,
            system,
            stream: false,
        };

        let response = self.client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("content-type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| AIError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let error: AnthropicError = response.json().await
                .map_err(|e| AIError::APIError(format!("Failed to parse error: {}", e)))?;
            return Err(Self::map_error(status, error));
        }

        let chat_response: AnthropicChatResponse = response.json().await
            .map_err(|e| AIError::APIError(format!("Failed to parse response: {}", e)))?;

        // Extract text content from content blocks
        let content = chat_response.content.iter()
            .filter_map(|block| block.text.clone())
            .collect::<Vec<_>>()
            .join("");

        if content.is_empty() {
            return Err(AIError::APIError("Empty response from Anthropic".to_string()));
        }

        Ok(ChatCompletionResponse {
            content,
            model: chat_response.model,
            usage: TokenUsage {
                prompt_tokens: chat_response.usage.input_tokens,
                completion_tokens: chat_response.usage.output_tokens,
                total_tokens: chat_response.usage.input_tokens + chat_response.usage.output_tokens,
            },
            finish_reason: chat_response.stop_reason.unwrap_or_else(|| "stop".to_string()),
        })
    }

    async fn complete_stream(
        &self,
        request: ChatCompletionRequest,
        callback: StreamingCallback,
    ) -> ProviderResult<()> {
        let (system, messages) = Self::convert_messages(&request.messages);

        if messages.is_empty() {
            return Err(AIError::InvalidRequest("At least one message is required".to_string()));
        }

        let anthropic_request = AnthropicChatRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens.or_else(|| Some(Self::default_max_tokens(&request.model))),
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: request.stop,
            system,
            stream: true,
        };

        let response = self.client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("content-type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| AIError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let error: AnthropicError = response.json().await
                .map_err(|e| AIError::APIError(format!("Failed to parse error: {}", e)))?;
            return Err(Self::map_error(status, error));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AIError::NetworkError(format!("Stream error: {}", e)))?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            // Process SSE lines
            while let Some(pos) = buffer.find('\n') {
                let line = buffer.drain(..=pos).collect::<String>();
                let line = line.trim();

                if line.starts_with("data: ") {
                    let data = &line[6..];

                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                        match event.event_type.as_str() {
                            "content_block_delta" => {
                                if let Ok(delta) = serde_json::from_value::<ContentBlockDelta>(event.data) {
                                    callback(StreamingChunk {
                                        content: delta.text,
                                        is_final: false,
                                        finish_reason: None,
                                    });
                                }
                            }
                            "message_stop" => {
                                callback(StreamingChunk {
                                    content: String::new(),
                                    is_final: true,
                                    finish_reason: Some("stop".to_string()),
                                });
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn embed(&self, _request: EmbeddingRequest) -> ProviderResult<EmbeddingResponse> {
        Err(AIError::UnsupportedCapability(
            "Anthropic does not provide embedding services. Use OpenAI or another provider for embeddings.".to_string()
        ))
    }

    fn supports_capability(&self, capability: &str) -> bool {
        match capability {
            "chat_completions" | "streaming" | "function_calling" | "vision" => true,
            "embeddings" => false,
            _ => false,
        }
    }

    fn default_model(&self) -> &str {
        &self.config.model
    }

    async fn available_models(&self) -> ProviderResult<Vec<String>> {
        Ok(Self::available_models())
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }
}

/// Anthropic provider factory
pub struct AnthropicProviderFactory;

#[async_trait]
impl AIProviderFactory for AnthropicProviderFactory {
    async fn create(config: ProviderConfig) -> ProviderResult<Arc<dyn AIProvider>> {
        Self::validate_config(&config)?;
        Ok(Arc::new(AnthropicProvider::new(config)?))
    }

    fn validate_config(config: &ProviderConfig) -> ProviderResult<()> {
        if config.api_key.is_none() {
            return Err(AIError::Authentication("API key is required".to_string()));
        }

        if config.model.is_empty() {
            return Err(AIError::InvalidRequest("Model is required".to_string()));
        }

        // Validate model is supported
        let valid_models = AnthropicProvider::available_models();
        if !valid_models.contains(&config.model) {
            // Allow custom model names that start with "claude-"
            if !config.model.starts_with("claude-") {
                return Err(AIError::InvalidRequest(
                    format!("Model '{}' is not supported. Use a Claude model.", config.model)
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_messages() {
        let messages = vec![
            ChatMessage::system("You are a helpful assistant"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let (system, anthropic_messages) = AnthropicProvider::convert_messages(&messages);
        assert_eq!(system, Some("You are a helpful assistant".to_string()));
        assert_eq!(anthropic_messages.len(), 2);
        assert_eq!(anthropic_messages[0].role, "user");
        assert_eq!(anthropic_messages[1].role, "assistant");
    }

    #[test]
    fn test_convert_messages_no_system() {
        let messages = vec![
            ChatMessage::user("Hello"),
        ];

        let (system, anthropic_messages) = AnthropicProvider::convert_messages(&messages);
        assert_eq!(system, None);
        assert_eq!(anthropic_messages.len(), 1);
    }

    #[test]
    fn test_available_models() {
        let models = AnthropicProvider::available_models();
        assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
        assert!(models.contains(&"claude-3-opus-20240229".to_string()));
        assert!(models.contains(&"claude-3-haiku-20240307".to_string()));
    }

    #[test]
    fn test_default_max_tokens() {
        assert_eq!(AnthropicProvider::default_max_tokens("claude-3-opus"), 4096);
        assert_eq!(AnthropicProvider::default_max_tokens("claude-3-5-sonnet"), 8192);
    }

    #[test]
    fn test_map_error() {
        let anthropic_error = AnthropicError {
            error_type: "error".to_string(),
            error: AnthropicErrorDetail {
                error_type: "authentication_error".to_string(),
                message: "Invalid API key".to_string(),
            },
        };

        let error = AnthropicProvider::map_error(reqwest::StatusCode::UNAUTHORIZED, anthropic_error);
        assert!(matches!(error, AIError::Authentication(_)));
    }
}
