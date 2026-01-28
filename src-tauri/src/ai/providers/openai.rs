// OpenAI Provider
// Direct integration with OpenAI API for GPT-4, GPT-4o, o1 models

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

/// OpenAI API base URL
const OPENAI_API_BASE: &str = "https://api.openai.com/v1";

/// OpenAI provider
pub struct OpenAIProvider {
    /// Provider configuration
    config: ProviderConfig,
    /// HTTP client
    client: reqwest::Client,
    /// API key
    api_key: String,
    /// Base URL (can be customized for OpenAI-compatible endpoints)
    base_url: String,
}

/// OpenAI API request body
#[derive(Debug, Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    stream: bool,
}

/// OpenAI message format
#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

/// OpenAI API response
#[derive(Debug, Deserialize)]
struct OpenAIChatResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

/// OpenAI choice
#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: u32,
    message: OpenAIMessage,
    finish_reason: String,
}

/// OpenAI token usage
#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// OpenAI embedding request
#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: String,
}

/// OpenAI embedding response
#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    object: String,
    data: Vec<OpenAIEmbeddingData>,
    model: String,
    usage: OpenAIUsage,
}

/// OpenAI embedding data
#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    object: String,
    embedding: Vec<f32>,
    index: u32,
}

/// OpenAI streaming chunk
#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIStreamChoice>,
}

/// OpenAI streaming choice
#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    index: u32,
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

/// OpenAI delta message
#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

/// OpenAI error response
#[derive(Debug, Deserialize)]
struct OpenAIError {
    error: OpenAIErrorDetail,
}

/// OpenAI error detail
#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    code: Option<String>,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(config: ProviderConfig) -> ProviderResult<Self> {
        let api_key = config.api_key.clone()
            .ok_or_else(|| AIError::Authentication("API key is required".to_string()))?;

        let base_url = config.base_url.clone()
            .unwrap_or_else(|| OPENAI_API_BASE.to_string());

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

    /// Convert chat messages to OpenAI format
    fn convert_messages(messages: &[ChatMessage]) -> Vec<OpenAIMessage> {
        messages.iter()
            .map(|msg| OpenAIMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
                name: msg.name.clone(),
            })
            .collect()
    }

    /// Map OpenAI error to AIError
    fn map_error(status: reqwest::StatusCode, error: OpenAIError) -> AIError {
        match status {
            reqwest::StatusCode::UNAUTHORIZED => AIError::Authentication(error.error.message),
            reqwest::StatusCode::TOO_MANY_REQUESTS => AIError::RateLimit(error.error.message),
            reqwest::StatusCode::BAD_REQUEST => AIError::InvalidRequest(error.error.message),
            _ => AIError::APIError(format!("OpenAI API error: {}", error.error.message)),
        }
    }

    /// Get available models based on capabilities
    fn available_models() -> Vec<String> {
        vec![
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-4".to_string(),
            "o1-preview".to_string(),
            "o1-mini".to_string(),
            "text-embedding-3-small".to_string(),
            "text-embedding-3-large".to_string(),
        ]
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn id(&self) -> &ProviderId {
        &self.config.id
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAI
    }

    async fn capabilities(&self) -> ProviderResult<ProviderCapabilities> {
        Ok(ProviderCapabilities {
            chat_completions: true,
            embeddings: true,
            streaming: true,
            function_calling: true,
            vision: true, // GPT-4o supports vision
            web_search: false, // Not directly supported
            max_context_tokens: 128000, // GPT-4o context window
            max_output_tokens: 4096,
            models: Self::available_models(),
        })
    }

    async fn health_check(&self) -> ProviderResult<ProviderHealth> {
        // Try a simple models list request to check health
        let response = self.client
            .get(format!("{}/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(ProviderHealth::Healthy)
                } else if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    Ok(ProviderHealth::Degraded)
                } else {
                    Ok(ProviderHealth::Unhealthy)
                }
            }
            Err(_) => Ok(ProviderHealth::Unhealthy),
        }
    }

    async fn complete(&self, request: ChatCompletionRequest) -> ProviderResult<ChatCompletionResponse> {
        let openai_request = OpenAIChatRequest {
            model: request.model.clone(),
            messages: Self::convert_messages(&request.messages),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop,
            stream: false,
        };

        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| AIError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let error: OpenAIError = response.json().await
                .map_err(|e| AIError::APIError(format!("Failed to parse error: {}", e)))?;
            return Err(Self::map_error(status, error));
        }

        let chat_response: OpenAIChatResponse = response.json().await
            .map_err(|e| AIError::APIError(format!("Failed to parse response: {}", e)))?;

        let choice = chat_response.choices.into_iter()
            .next()
            .ok_or_else(|| AIError::APIError("No response choices".to_string()))?;

        Ok(ChatCompletionResponse {
            content: choice.message.content,
            model: chat_response.model,
            usage: TokenUsage {
                prompt_tokens: chat_response.usage.prompt_tokens,
                completion_tokens: chat_response.usage.completion_tokens,
                total_tokens: chat_response.usage.total_tokens,
            },
            finish_reason: choice.finish_reason,
        })
    }

    async fn complete_stream(
        &self,
        request: ChatCompletionRequest,
        callback: StreamingCallback,
    ) -> ProviderResult<()> {
        let openai_request = OpenAIChatRequest {
            model: request.model.clone(),
            messages: Self::convert_messages(&request.messages),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop,
            stream: true,
        };

        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| AIError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let error: OpenAIError = response.json().await
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

                    if data == "[DONE]" {
                        callback(StreamingChunk {
                            content: String::new(),
                            is_final: true,
                            finish_reason: Some("stop".to_string()),
                        });
                        return Ok(());
                    }

                    if let Ok(chunk_data) = serde_json::from_str::<OpenAIStreamChunk>(data) {
                        if let Some(choice) = chunk_data.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                callback(StreamingChunk {
                                    content: content.clone(),
                                    is_final: choice.finish_reason.is_some(),
                                    finish_reason: choice.finish_reason.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn embed(&self, request: EmbeddingRequest) -> ProviderResult<EmbeddingResponse> {
        let embedding_request = OpenAIEmbeddingRequest {
            model: request.model.clone(),
            input: request.input.clone(),
        };

        let response = self.client
            .post(format!("{}/embeddings", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&embedding_request)
            .send()
            .await
            .map_err(|e| AIError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let error: OpenAIError = response.json().await
                .map_err(|e| AIError::APIError(format!("Failed to parse error: {}", e)))?;
            return Err(Self::map_error(status, error));
        }

        let embedding_response: OpenAIEmbeddingResponse = response.json().await
            .map_err(|e| AIError::APIError(format!("Failed to parse response: {}", e)))?;

        let data = embedding_response.data.into_iter()
            .next()
            .ok_or_else(|| AIError::APIError("No embedding data".to_string()))?;

        Ok(EmbeddingResponse {
            embedding: data.embedding,
            model: embedding_response.model,
            usage: TokenUsage {
                prompt_tokens: embedding_response.usage.prompt_tokens,
                completion_tokens: 0,
                total_tokens: embedding_response.usage.total_tokens,
            },
        })
    }

    fn supports_capability(&self, capability: &str) -> bool {
        match capability {
            "chat_completions" | "streaming" | "embeddings" | "function_calling" | "vision" => true,
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

/// OpenAI provider factory
pub struct OpenAIProviderFactory;

#[async_trait]
impl AIProviderFactory for OpenAIProviderFactory {
    async fn create(config: ProviderConfig) -> ProviderResult<Arc<dyn AIProvider>> {
        Self::validate_config(&config)?;
        Ok(Arc::new(OpenAIProvider::new(config)?))
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
    fn test_convert_messages() {
        let messages = vec![
            ChatMessage::system("You are a helpful assistant"),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
        ];

        let openai_messages = OpenAIProvider::convert_messages(&messages);
        assert_eq!(openai_messages.len(), 3);
        assert_eq!(openai_messages[0].role, "system");
        assert_eq!(openai_messages[1].role, "user");
        assert_eq!(openai_messages[2].role, "assistant");
    }

    #[test]
    fn test_available_models() {
        let models = OpenAIProvider::available_models();
        assert!(models.contains(&"gpt-4o".to_string()));
        assert!(models.contains(&"gpt-4o-mini".to_string()));
        assert!(models.contains(&"text-embedding-3-small".to_string()));
    }

    #[test]
    fn test_map_error() {
        let openai_error = OpenAIError {
            error: OpenAIErrorDetail {
                message: "Invalid API key".to_string(),
                error_type: "authentication".to_string(),
                code: None,
            },
        };

        let error = OpenAIProvider::map_error(reqwest::StatusCode::UNAUTHORIZED, openai_error);
        assert!(matches!(error, AIError::Authentication(_)));
    }
}
