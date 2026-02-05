// Rainy SDK Provider
// Wrapper around rainy-sdk v0.6.1 for the new provider abstraction layer
// Enhanced with direct HTTP calls for tool calling support (SDK limitation bypass)

use crate::ai::provider_trait::{AIProvider, AIProviderFactory};
use crate::ai::provider_types::{
    AIError, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, EmbeddingRequest,
    EmbeddingResponse, FunctionCall, ProviderCapabilities, ProviderConfig, ProviderHealth,
    ProviderId, ProviderResult, ProviderType, StreamingCallback, Tool, ToolCall,
};
use async_trait::async_trait;
use futures::StreamExt;
use rainy_sdk::RainyClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Rainy API base URL for direct HTTP calls
const RAINY_API_BASE: &str = "https://api.rainy.dev";

// ============================================================================
// Internal types for OpenAI-compatible API with tool calling support
// These are used for direct HTTP calls that bypass SDK limitations
// ============================================================================

/// OpenAI-compatible request with tools support
#[derive(Debug, Serialize)]
struct RainyToolRequest {
    model: String,
    messages: Vec<RainyMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<RainyTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

/// OpenAI-compatible message format
#[derive(Debug, Serialize, Deserialize)]
struct RainyMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<RainyToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

/// OpenAI-compatible tool definition
#[derive(Debug, Serialize)]
struct RainyTool {
    r#type: String,
    function: RainyFunction,
}

/// OpenAI-compatible function definition
#[derive(Debug, Serialize)]
struct RainyFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// OpenAI-compatible tool call in response
#[derive(Debug, Serialize, Deserialize, Clone)]
struct RainyToolCall {
    id: String,
    r#type: String,
    function: RainyFunctionCall,
}

/// OpenAI-compatible function call details
#[derive(Debug, Serialize, Deserialize, Clone)]
struct RainyFunctionCall {
    name: String,
    arguments: String,
}

/// OpenAI-compatible response
#[derive(Debug, Deserialize)]
struct RainyResponse {
    model: String,
    choices: Vec<RainyChoice>,
    usage: Option<RainyUsage>,
}

/// OpenAI-compatible choice
#[derive(Debug, Deserialize)]
struct RainyChoice {
    message: RainyResponseMessage,
    finish_reason: String,
}

/// OpenAI-compatible response message (different from request message)
#[derive(Debug, Deserialize)]
struct RainyResponseMessage {
    #[allow(dead_code)]
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<RainyToolCall>>,
}

/// OpenAI-compatible usage
#[derive(Debug, Deserialize)]
struct RainyUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// OpenAI-compatible error
#[derive(Debug, Deserialize)]
struct RainyError {
    error: RainyErrorDetail,
}

/// OpenAI-compatible error detail
#[derive(Debug, Deserialize)]
struct RainyErrorDetail {
    message: String,
    #[serde(default)]
    r#type: String,
}

/// Rainy SDK provider
pub struct RainySDKProvider {
    /// Provider configuration
    config: ProviderConfig,
    /// Rainy client (for streaming and non-tool calls)
    client: RainyClient,
    /// HTTP client for direct API calls with tool support
    http_client: reqwest::Client,
    /// API key for direct calls
    api_key: String,
    /// Cached capabilities
    cached_capabilities: tokio::sync::RwLock<Option<ProviderCapabilities>>,
}

impl RainySDKProvider {
    /// Create a new Rainy SDK provider
    pub fn new(config: ProviderConfig) -> ProviderResult<Self> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| AIError::Authentication("API key is required".to_string()))?
            .clone();

        let client = RainyClient::with_api_key(&api_key).map_err(|e| {
            AIError::Authentication(format!("Failed to create Rainy client: {}", e))
        })?;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AIError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            http_client,
            api_key,
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
        let (model_id, _thinking_config) = Self::map_model_id(&request.model);

        // Convert messages to OpenAI-compatible format
        let messages: Vec<RainyMessage> = request
            .messages
            .iter()
            .map(|msg| RainyMessage {
                role: msg.role.clone(),
                content: if msg.content.is_empty() && msg.role == "assistant" {
                    // Assistant messages with tool_calls may have empty content
                    None
                } else {
                    Some(msg.content.clone())
                },
                name: msg.name.clone(),
                tool_calls: msg.tool_calls.as_ref().map(|calls| {
                    calls
                        .iter()
                        .map(|tc| RainyToolCall {
                            id: tc.id.clone(),
                            r#type: tc.r#type.clone(),
                            function: RainyFunctionCall {
                                name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                            },
                        })
                        .collect()
                }),
                tool_call_id: msg.tool_call_id.clone(),
            })
            .collect();

        // Convert tools to OpenAI-compatible format
        let tools: Option<Vec<RainyTool>> = request.tools.as_ref().map(|t| {
            t.iter()
                .map(|tool| RainyTool {
                    r#type: tool.r#type.clone(),
                    function: RainyFunction {
                        name: tool.function.name.clone(),
                        description: tool.function.description.clone(),
                        parameters: tool.function.parameters.clone(),
                    },
                })
                .collect()
        });

        // Build OpenAI-compatible request
        let api_request = RainyToolRequest {
            model: model_id,
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools,
            tool_choice: if request.tools.is_some() {
                Some("auto".to_string())
            } else {
                None
            },
        };

        // Make direct HTTP call to Rainy API
        let response = self
            .http_client
            .post(format!("{}/v1/chat/completions", RAINY_API_BASE))
            .bearer_auth(&self.api_key)
            .json(&api_request)
            .send()
            .await
            .map_err(|e| AIError::NetworkError(format!("Request failed: {}", e)))?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as structured error
            if let Ok(error) = serde_json::from_str::<RainyError>(&error_text) {
                return Err(AIError::APIError(format!(
                    "API error ({}): {}",
                    error.error.r#type, error.error.message
                )));
            }

            return Err(AIError::APIError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let api_response: RainyResponse = response
            .json()
            .await
            .map_err(|e| AIError::APIError(format!("Failed to parse response: {}", e)))?;

        // Extract choice
        let choice = api_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AIError::APIError("No response choices".to_string()))?;

        // Convert tool_calls from response to our format
        let tool_calls: Option<Vec<ToolCall>> = choice.message.tool_calls.map(|calls| {
            calls
                .into_iter()
                .map(|tc| ToolCall {
                    id: tc.id,
                    r#type: tc.r#type,
                    function: FunctionCall {
                        name: tc.function.name,
                        arguments: tc.function.arguments,
                    },
                })
                .collect()
        });

        // Log tool calls for debugging
        if let Some(ref calls) = tool_calls {
            tracing::info!(
                "[RainySDK] Received {} tool calls from LLM: {:?}",
                calls.len(),
                calls.iter().map(|c| &c.function.name).collect::<Vec<_>>()
            );
        }

        Ok(ChatCompletionResponse {
            content: choice.message.content,
            tool_calls,
            model: api_response.model,
            usage: {
                let (prompt, completion, total) = match api_response.usage.as_ref() {
                    Some(u) => (u.prompt_tokens, u.completion_tokens, u.total_tokens),
                    None => (0, 0, 0),
                };
                crate::ai::provider_types::TokenUsage {
                    prompt_tokens: prompt,
                    completion_tokens: completion,
                    total_tokens: total,
                }
            },
            finish_reason: choice.finish_reason,
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
