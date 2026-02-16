// Moonshot AI Provider (Kimi)
// Integration for Kimi models (moonshot-v1-*, kimi-k2.5) via OpenAI-compatible API

use crate::ai::provider_trait::{AIProvider, AIProviderFactory};
use crate::ai::provider_types::MessageContent;
use crate::ai::provider_types::{
    AIError, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ProviderCapabilities,
    ProviderConfig, ProviderHealth, ProviderId, ProviderResult, ProviderType, StreamingCallback,
    StreamingChunk, TokenUsage,
};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Moonshot API base URL
const MOONSHOT_API_BASE: &str = "https://api.moonshot.cn/v1";

/// Moonshot provider
pub struct MoonshotProvider {
    /// Provider configuration
    config: ProviderConfig,
    /// HTTP client
    client: reqwest::Client,
    /// API key
    api_key: String,
    /// Base URL
    base_url: String,
}

/// OpenAI-compatible API request body
#[derive(Debug, Serialize)]
struct MoonshotChatRequest {
    model: String,
    messages: Vec<MoonshotMessage>,
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

/// Moonshot message format
#[derive(Debug, Serialize, Deserialize)]
struct MoonshotMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<MessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

/// Moonshot API response
#[derive(Debug, Deserialize)]
struct MoonshotChatResponse {
    model: String,
    choices: Vec<MoonshotChoice>,
    usage: MoonshotUsage,
}

/// Moonshot choice
#[derive(Debug, Deserialize)]
struct MoonshotChoice {
    message: MoonshotMessage,
    finish_reason: String,
}

/// Moonshot token usage
#[derive(Debug, Deserialize)]
struct MoonshotUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// Moonshot streaming chunk
#[derive(Debug, Deserialize)]
struct MoonshotStreamChunk {
    choices: Vec<MoonshotStreamChoice>,
}

/// Moonshot streaming choice
#[derive(Debug, Deserialize)]
struct MoonshotStreamChoice {
    delta: MoonshotDelta,
    finish_reason: Option<String>,
}

/// Moonshot delta message
#[derive(Debug, Deserialize)]
struct MoonshotDelta {
    #[serde(default)]
    content: Option<String>,
}

/// Moonshot error response
#[derive(Debug, Deserialize)]
struct MoonshotError {
    error: MoonshotErrorDetail,
}

/// Moonshot error detail
#[derive(Debug, Deserialize)]
struct MoonshotErrorDetail {
    message: String,
}

impl MoonshotProvider {
    /// Create a new Moonshot provider
    pub fn new(config: ProviderConfig) -> ProviderResult<Self> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| AIError::Authentication("API key is required".to_string()))?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| MOONSHOT_API_BASE.to_string());

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

    /// Convert chat messages to Moonshot format
    fn convert_messages(messages: &[ChatMessage]) -> Vec<MoonshotMessage> {
        messages
            .iter()
            .map(|msg| MoonshotMessage {
                role: msg.role.clone(),
                content: Some(msg.content.clone()),
                name: msg.name.clone(),
            })
            .collect()
    }

    /// Map Moonshot error to AIError
    fn map_error(status: reqwest::StatusCode, error: MoonshotError) -> AIError {
        match status {
            reqwest::StatusCode::UNAUTHORIZED => AIError::Authentication(error.error.message),
            reqwest::StatusCode::TOO_MANY_REQUESTS => AIError::RateLimit(error.error.message),
            reqwest::StatusCode::BAD_REQUEST => AIError::InvalidRequest(error.error.message),
            _ => AIError::APIError(format!("Moonshot API error: {}", error.error.message)),
        }
    }

    /// Get available models
    fn available_models() -> Vec<String> {
        vec![
            "moonshot-v1-8k".to_string(),
            "moonshot-v1-32k".to_string(),
            "moonshot-v1-128k".to_string(),
            "kimi-k2.5".to_string(),
        ]
    }
}

#[async_trait]
impl AIProvider for MoonshotProvider {
    fn id(&self) -> &ProviderId {
        &self.config.id
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Moonshot
    }

    async fn capabilities(&self) -> ProviderResult<ProviderCapabilities> {
        Ok(ProviderCapabilities {
            chat_completions: true,
            embeddings: false, // Moonshot doesn't provide embeddings yet
            streaming: true,
            function_calling: false, // Check if supported
            vision: false, // Kimi supports vision but requires multipart/form-data with file upload first (handled separately if needed)
            web_search: true, // Kimi has built-in search
            max_context_tokens: 128000,
            max_output_tokens: 4096,
            models: Self::available_models(),
        })
    }

    async fn health_check(&self) -> ProviderResult<ProviderHealth> {
        // Try a simple models list request to check health
        let response = self
            .client
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

    async fn complete(
        &self,
        request: ChatCompletionRequest,
    ) -> ProviderResult<ChatCompletionResponse> {
        let moonshot_request = MoonshotChatRequest {
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

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&moonshot_request)
            .send()
            .await
            .map_err(|e| AIError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let error: MoonshotError = response
                .json()
                .await
                .map_err(|e| AIError::APIError(format!("Failed to parse error: {}", e)))?;
            return Err(Self::map_error(status, error));
        }

        let chat_response: MoonshotChatResponse = response
            .json()
            .await
            .map_err(|e| AIError::APIError(format!("Failed to parse response: {}", e)))?;

        let choice = chat_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AIError::APIError("No response choices".to_string()))?;

        Ok(ChatCompletionResponse {
            content: choice
                .message
                .content
                .as_ref()
                .map(|c: &MessageContent| c.text()),
            tool_calls: None,
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
        let moonshot_request = MoonshotChatRequest {
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

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&moonshot_request)
            .send()
            .await
            .map_err(|e| AIError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let error: MoonshotError = response
                .json()
                .await
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
                            thought: None,
                            is_final: true,
                            finish_reason: Some("stop".to_string()),
                        });
                        return Ok(());
                    }

                    if let Ok(chunk_data) = serde_json::from_str::<MoonshotStreamChunk>(data) {
                        if let Some(choice) = chunk_data.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                callback(StreamingChunk {
                                    content: content.clone(),
                                    thought: None, // Kimi puts thought in content usually? Or maybe separate field?
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

    async fn embed(
        &self,
        _request: crate::ai::provider_types::EmbeddingRequest,
    ) -> ProviderResult<crate::ai::provider_types::EmbeddingResponse> {
        Err(AIError::UnsupportedCapability(
            "Embeddings not supported by Moonshot provider yet".to_string(),
        ))
    }

    fn default_model(&self) -> &str {
        if self.config.model == "default" {
            "moonshot-v1-128k"
        } else {
            &self.config.model
        }
    }

    async fn available_models(&self) -> ProviderResult<Vec<String>> {
        Ok(Self::available_models())
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }
}

/// Moonshot provider factory
// @RESERVED for Phase 3 Provider Registry
#[allow(dead_code)]
pub struct MoonshotProviderFactory;

#[async_trait]
impl AIProviderFactory for MoonshotProviderFactory {
    async fn create(config: ProviderConfig) -> ProviderResult<Arc<dyn AIProvider>> {
        Self::validate_config(&config)?;
        Ok(Arc::new(MoonshotProvider::new(config)?))
    }

    fn validate_config(config: &ProviderConfig) -> ProviderResult<()> {
        if config.api_key.is_none() {
            return Err(AIError::Authentication("API key is required".to_string()));
        }
        Ok(())
    }
}
