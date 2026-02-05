// xAI Provider for Grok Models
// Direct integration with xAI's Grok API using OpenAI-compatible endpoints

use crate::ai::{
    AIError, AIProvider, AIProviderFactory, ChatCompletionRequest, ChatCompletionResponse,
    ChatMessage, EmbeddingRequest, EmbeddingResponse, ProviderCapabilities, ProviderConfig,
    ProviderHealth, ProviderId, ProviderResult, StreamingCallback, StreamingChunk, TokenUsage,
};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// xAI Provider implementation for Grok models
#[derive(Clone, Debug)]
pub struct XAIProvider {
    /// HTTP client for API requests
    client: Arc<Client>,
    /// API key for authentication
    api_key: Arc<str>,
    /// Base URL for API requests
    base_url: Arc<str>,
    /// Provider configuration
    config: ProviderConfig,
}

impl XAIProvider {
    /// Create a new xAI provider instance
    pub fn new(client: Client, config: ProviderConfig) -> Self {
        let api_key = config.api_key.clone().unwrap_or_default();
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.x.ai/v1".to_string());

        Self {
            client: Arc::new(client),
            api_key: Arc::from(api_key),
            base_url: Arc::from(base_url),
            config,
        }
    }

    /// Get the base URL for API requests
    fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the API key
    fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Create the HTTP client with proper headers
    fn create_client() -> Client {
        Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap_or_default()
    }
}

#[async_trait]
impl AIProvider for XAIProvider {
    /// Get the provider ID
    fn id(&self) -> &ProviderId {
        &self.config.id
    }

    /// Get the provider type
    fn provider_type(&self) -> crate::ai::ProviderType {
        self.config.provider_type
    }

    /// Get the default model
    fn default_model(&self) -> &str {
        &self.config.model
    }

    /// Get available models
    async fn available_models(&self) -> ProviderResult<Vec<String>> {
        Ok(vec![
            // Grok 4.1 Family (2M context)
            "grok-4-1-fast-reasoning".to_string(),
            "grok-4-1-fast-non-reasoning".to_string(),
            // Grok 4 Family (256K context)
            "grok-4-fast-reasoning".to_string(),
            "grok-4-fast-non-reasoning".to_string(),
            "grok-4".to_string(),
            // Grok 3 Family (200K context)
            "grok-3".to_string(),
            "grok-3-fast".to_string(),
            "grok-3-mini".to_string(),
            "grok-3-mini-reasoning".to_string(),
            // Grok Code Family (256K context)
            "grok-code-fast-1".to_string(),
            // Grok 2 Family (128K context)
            "grok-2".to_string(),
            "grok-2-vision".to_string(),
            "grok-2-image-1212".to_string(),
        ])
    }

    /// Get provider capabilities
    async fn capabilities(&self) -> ProviderResult<ProviderCapabilities> {
        Ok(ProviderCapabilities {
            chat_completions: true,
            embeddings: false,
            streaming: true,
            function_calling: true,
            vision: true,
            web_search: true,
            max_context_tokens: 2_000_000, // grok-4-1 family has 2M context
            max_output_tokens: 16384,
            models: vec![
                // Grok 4.1 Family (2M context)
                "grok-4-1-fast-reasoning".to_string(),
                "grok-4-1-fast-non-reasoning".to_string(),
                // Grok 4 Family (256K context)
                "grok-4-fast-reasoning".to_string(),
                "grok-4-fast-non-reasoning".to_string(),
                "grok-4".to_string(),
                // Grok 3 Family (200K context)
                "grok-3".to_string(),
                "grok-3-fast".to_string(),
                "grok-3-mini".to_string(),
                "grok-3-mini-reasoning".to_string(),
                // Grok Code Family (256K context)
                "grok-code-fast-1".to_string(),
                // Grok 2 Family (128K context)
                "grok-2".to_string(),
                "grok-2-vision".to_string(),
                "grok-2-image-1212".to_string(),
            ],
        })
    }

    /// Check provider health
    async fn health_check(&self) -> ProviderResult<ProviderHealth> {
        let request = self
            .client
            .get(&format!("{}/models", self.base_url()))
            .header("Authorization", format!("Bearer {}", self.api_key()));

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(ProviderHealth::Healthy)
                } else {
                    Ok(ProviderHealth::Unhealthy)
                }
            }
            Err(_) => Ok(ProviderHealth::Unhealthy),
        }
    }

    /// Execute a chat completion request
    async fn complete(
        &self,
        request: ChatCompletionRequest,
    ) -> ProviderResult<ChatCompletionResponse> {
        let request_body = XAIChatRequest::from(request.clone());
        let request_builder = self
            .client
            .post(&format!("{}/chat/completions", self.base_url()))
            .header("Authorization", format!("Bearer {}", self.api_key()))
            .header("Content-Type", "application/json")
            .json(&request_body);

        match self.execute_request(request_builder).await {
            Ok(response) => Ok(response.to_completion_response()),
            Err(e) => Err(e),
        }
    }

    /// Execute a streaming chat completion request
    async fn complete_stream(
        &self,
        request: ChatCompletionRequest,
        callback: StreamingCallback,
    ) -> ProviderResult<()> {
        let request_body = XAIChatRequest::from(request.clone());
        let request_builder = self
            .client
            .post(&format!("{}/chat/completions", self.base_url()))
            .header("Authorization", format!("Bearer {}", self.api_key()))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request_body);

        self.execute_stream_request(request_builder, callback).await
    }

    /// Generate embeddings
    async fn embed(&self, _request: EmbeddingRequest) -> ProviderResult<EmbeddingResponse> {
        Err(AIError::UnsupportedCapability(
            "xAI does not support embeddings".to_string(),
        ))
    }

    /// Get the provider configuration
    fn config(&self) -> &ProviderConfig {
        &self.config
    }
}

impl XAIProvider {
    /// Execute a regular HTTP request
    async fn execute_request(&self, builder: RequestBuilder) -> Result<XAIChatResponse, AIError> {
        builder
            .send()
            .await
            .map_err(|e| AIError::NetworkError(e.to_string()))?
            .json()
            .await
            .map_err(|e| AIError::APIError(format!("Failed to parse xAI API response: {}", e)))
    }

    /// Execute a streaming request
    async fn execute_stream_request(
        &self,
        builder: RequestBuilder,
        callback: StreamingCallback,
    ) -> Result<(), AIError> {
        let response = builder
            .send()
            .await
            .map_err(|e| AIError::NetworkError(e.to_string()))?;

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AIError::NetworkError(e.to_string()))?;

            let text = String::from_utf8_lossy(&chunk);
            let lines: Vec<&str> = text.split('\n').filter(|l| !l.is_empty()).collect();

            for line in lines {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        return Ok(());
                    }

                    // Parse SSE data
                    if let Ok(chunk) = serde_json::from_str::<XAIStreamingChunk>(data) {
                        if let Some(delta) =
                            chunk.choices.first().and_then(|c| c.delta.content.clone())
                        {
                            let _ = callback(StreamingChunk {
                                content: delta,
                                thought: None,
                                is_final: false,
                                finish_reason: chunk
                                    .choices
                                    .first()
                                    .and_then(|c| c.finish_reason.clone()),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// xAI Chat Request structure (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XAIChatRequest {
    pub model: String,
    pub messages: Vec<XAIChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    pub stream: bool,
}

impl From<ChatCompletionRequest> for XAIChatRequest {
    fn from(req: ChatCompletionRequest) -> Self {
        Self {
            model: req.model.clone(),
            messages: req.messages.into_iter().map(|m| m.into()).collect(),
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            top_p: req.top_p,
            frequency_penalty: req.frequency_penalty,
            presence_penalty: req.presence_penalty,
            stop: req.stop,
            stream: false,
        }
    }
}

/// xAI Chat Message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XAIChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl From<ChatMessage> for XAIChatMessage {
    fn from(msg: ChatMessage) -> Self {
        Self {
            role: msg.role,
            content: msg.content,
            name: msg.name,
        }
    }
}

/// xAI Chat Response structure (OpenAI-compatible)
#[derive(Debug, Clone, Deserialize)]
pub struct XAIChatResponse {
    pub model: String,
    pub choices: Vec<XAIChoice>,
    pub usage: Option<XAITokenUsage>,
}

impl XAIChatResponse {
    /// Convert to standard ChatCompletionResponse
    pub fn to_completion_response(&self) -> ChatCompletionResponse {
        let content = self
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let usage = self
            .usage
            .as_ref()
            .map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            })
            .unwrap_or_else(|| TokenUsage::new(0, 0));

        let finish_reason = self
            .choices
            .first()
            .and_then(|c| c.finish_reason.clone())
            .unwrap_or_else(|| "stop".to_string());

        ChatCompletionResponse {
            content: Some(content),
            tool_calls: None,
            model: self.model.clone(),
            usage,
            finish_reason,
        }
    }
}

/// xAI Choice structure
#[derive(Debug, Clone, Deserialize)]
pub struct XAIChoice {
    pub message: XAIChatMessage,
    pub finish_reason: Option<String>,
}

/// xAI Token Usage structure
#[derive(Debug, Clone, Deserialize)]
pub struct XAITokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// xAI Logprobs structure

/// xAI Logprob Content structure

/// xAI Streaming Chunk structure
#[derive(Debug, Clone, Deserialize)]
pub struct XAIStreamingChunk {
    pub choices: Vec<XAIStreamingChoice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct XAIStreamingChoice {
    pub delta: XAIDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct XAIDelta {
    pub content: Option<String>,
}

/// xAI Provider Factory
#[derive(Debug, Default, Clone, Copy)]
pub struct XAIProviderFactory;

#[async_trait]
impl AIProviderFactory for XAIProviderFactory {
    /// Validate provider configuration
    fn validate_config(config: &ProviderConfig) -> ProviderResult<()> {
        if config.api_key.is_none() {
            return Err(AIError::Configuration(
                "xAI API key is required".to_string(),
            ));
        }
        Ok(())
    }

    /// Create a new provider instance
    async fn create(config: ProviderConfig) -> ProviderResult<Arc<dyn AIProvider>> {
        Self::validate_config(&config)?;
        let client = XAIProvider::create_client();
        Ok(Arc::new(XAIProvider::new(client, config)) as Arc<dyn AIProvider>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xai_chat_request_from_completion_request() {
        let request = ChatCompletionRequest {
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are a helpful assistant.".to_string(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hello!".to_string(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            model: "grok-3".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: false,
            tools: None,
            tool_choice: None,
            json_mode: false,
        };

        let xai_request = XAIChatRequest::from(request);

        assert_eq!(xai_request.model, "grok-3");
        assert_eq!(xai_request.messages.len(), 2);
        assert_eq!(xai_request.messages[0].role, "system");
        assert_eq!(xai_request.messages[1].role, "user");
        assert_eq!(xai_request.temperature, Some(0.7));
        assert_eq!(xai_request.max_tokens, Some(100));
        assert!(!xai_request.stream);
    }

    #[test]
    fn test_xai_factory_validate_config() {
        // Valid config
        let valid_config = ProviderConfig {
            id: ProviderId::new("xai-valid"),
            provider_type: crate::ai::ProviderType::XAI,
            api_key: Some("test-key".to_string()),
            base_url: None,
            model: "grok-3".to_string(),
            params: std::collections::HashMap::new(),
            enabled: true,
            priority: 5,
            rate_limit: None,
            timeout: 300,
        };
        assert!(XAIProviderFactory::validate_config(&valid_config).is_ok());

        // Invalid config (no API key)
        let invalid_config = ProviderConfig {
            id: ProviderId::new("xai-invalid"),
            provider_type: crate::ai::ProviderType::XAI,
            api_key: None,
            base_url: None,
            model: "grok-3".to_string(),
            params: std::collections::HashMap::new(),
            enabled: true,
            priority: 5,
            rate_limit: None,
            timeout: 300,
        };
        assert!(XAIProviderFactory::validate_config(&invalid_config).is_err());
    }
}
