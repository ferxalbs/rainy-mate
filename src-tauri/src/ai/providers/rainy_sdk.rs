use crate::ai::provider_trait::{AIProvider, AIProviderFactory};
use crate::ai::provider_types::{
    AIError, ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    FunctionCall, ProviderCapabilities, ProviderConfig, ProviderHealth, ProviderId, ProviderResult,
    ProviderType, StreamingCallback, ToolCall,
};
use async_trait::async_trait;
use futures::StreamExt;
use rainy_sdk::models::{
    CapabilityFlag, FunctionDefinition, OpenAIChatCompletionRequest, OpenAIChatMessage,
    OpenAIContentPart, OpenAIFunctionCall, OpenAIMessageContent, OpenAIMessageRole,
    OpenAIToolCall, OpenAIImageUrl, ThinkingConfig, ThinkingLevel, Tool, ToolChoice, ToolFunction,
    ToolType,
};
use rainy_sdk::RainyClient;
use std::sync::Arc;

pub struct RainySDKProvider {
    config: ProviderConfig,
    client: RainyClient,
    cached_capabilities: tokio::sync::RwLock<Option<ProviderCapabilities>>,
}

impl RainySDKProvider {
    pub fn new(config: ProviderConfig) -> ProviderResult<Self> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| AIError::Authentication("API key is required".to_string()))?;

        let mut auth_config = rainy_sdk::AuthConfig::new(api_key.clone());

        if let Some(base_url) = config
            .base_url
            .clone()
            .or_else(|| std::env::var("RAINY_API_BASE_URL").ok())
        {
            auth_config = auth_config.with_base_url(base_url);
        }

        auth_config = auth_config.with_timeout(config.timeout).with_retry(true);

        let client = RainyClient::with_config(auth_config).map_err(|e| {
            AIError::Authentication(format!("Failed to create Rainy client: {}", e))
        })?;

        Ok(Self {
            config,
            client,
            cached_capabilities: tokio::sync::RwLock::new(None),
        })
    }

    fn map_model_id(model_id: &str) -> (String, Option<ThinkingConfig>) {
        let mut thinking_config = ThinkingConfig::default();
        thinking_config.include_thoughts = Some(true);

        let clean_id = crate::ai::model_catalog::normalize_model_slug(model_id);

        match clean_id {
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
            "gemini-3-pro-low" => {
                thinking_config.thinking_level = Some(ThinkingLevel::Low);
                ("gemini-3-pro-preview".to_string(), Some(thinking_config))
            }
            "gemini-3-pro-high" => {
                thinking_config.thinking_level = Some(ThinkingLevel::High);
                ("gemini-3-pro-preview".to_string(), Some(thinking_config))
            }
            _ => (clean_id.to_string(), None),
        }
    }

    fn map_content(content: &crate::ai::provider_types::MessageContent) -> OpenAIMessageContent {
        match content {
            crate::ai::provider_types::MessageContent::Text(text) => {
                OpenAIMessageContent::Text(text.clone())
            }
            crate::ai::provider_types::MessageContent::Parts(parts) => {
                OpenAIMessageContent::Parts(
                    parts.iter()
                        .map(|part| match part {
                            crate::ai::provider_types::ContentPart::Text { text } => {
                                OpenAIContentPart::Text { text: text.clone() }
                            }
                            crate::ai::provider_types::ContentPart::ImageUrl { image_url } => {
                                OpenAIContentPart::ImageUrl {
                                    image_url: OpenAIImageUrl {
                                        url: image_url.url.clone(),
                                        detail: image_url.detail.clone(),
                                    },
                                }
                            }
                        })
                        .collect(),
                )
            }
        }
    }

    fn map_message(message: &crate::ai::provider_types::ChatMessage) -> OpenAIChatMessage {
        let role = match message.role.as_str() {
            "system" => OpenAIMessageRole::System,
            "assistant" => OpenAIMessageRole::Assistant,
            "tool" => OpenAIMessageRole::Tool,
            _ => OpenAIMessageRole::User,
        };

        OpenAIChatMessage {
            role,
            content: match (&message.content, &message.tool_calls) {
                (crate::ai::provider_types::MessageContent::Text(text), Some(tool_calls))
                    if text.is_empty() && !tool_calls.is_empty() =>
                {
                    None
                }
                (content, _) => Some(Self::map_content(content)),
            },
            name: message.name.clone(),
            tool_calls: message.tool_calls.as_ref().map(|calls| {
                calls.iter()
                    .map(|call| OpenAIToolCall {
                        id: call.id.clone(),
                        r#type: call.r#type.clone(),
                        extra_content: call.extra_content.clone(),
                        function: OpenAIFunctionCall {
                            name: call.function.name.clone(),
                            arguments: call.function.arguments.clone(),
                        },
                    })
                    .collect()
            }),
            tool_call_id: message.tool_call_id.clone(),
        }
    }

    fn map_tools(
        tools: Option<&[crate::ai::provider_types::Tool]>,
    ) -> Option<Vec<Tool>> {
        tools.map(|tools| {
            tools
                .iter()
                .map(|tool| Tool {
                    r#type: ToolType::Function,
                    function: FunctionDefinition {
                        name: tool.function.name.clone(),
                        description: Some(tool.function.description.clone()),
                        parameters: Some(tool.function.parameters.clone()),
                    },
                })
                .collect()
        })
    }

    fn map_tool_choice(
        tool_choice: Option<&crate::ai::provider_types::ToolChoice>,
    ) -> Option<ToolChoice> {
        match tool_choice {
            Some(crate::ai::provider_types::ToolChoice::None) => Some(ToolChoice::None),
            Some(crate::ai::provider_types::ToolChoice::Auto) => Some(ToolChoice::Auto),
            Some(crate::ai::provider_types::ToolChoice::Tool(tool)) => Some(ToolChoice::Tool {
                r#type: ToolType::Function,
                function: ToolFunction {
                    name: tool.function.name.clone(),
                },
            }),
            None => None,
        }
    }

    fn build_openai_request(request: &ChatCompletionRequest) -> OpenAIChatCompletionRequest {
        let (model_id, thinking_config) = Self::map_model_id(&request.model);

        let mut sdk_request = OpenAIChatCompletionRequest::new(
            model_id,
            request.messages.iter().map(Self::map_message).collect(),
        );

        if let Some(config) = thinking_config {
            sdk_request = sdk_request.with_thinking_config(config);
        }
        if let Some(max_tokens) = request.max_tokens {
            sdk_request = sdk_request.with_max_tokens(max_tokens);
        }
        if let Some(temperature) = request.temperature {
            sdk_request = sdk_request.with_temperature(temperature);
        }
        if let Some(top_p) = request.top_p {
            sdk_request = sdk_request.with_top_p(top_p);
        }
        if let Some(frequency_penalty) = request.frequency_penalty {
            sdk_request = sdk_request.with_frequency_penalty(frequency_penalty);
        }
        if let Some(presence_penalty) = request.presence_penalty {
            sdk_request = sdk_request.with_presence_penalty(presence_penalty);
        }
        if let Some(stop) = request.stop.clone() {
            sdk_request = sdk_request.with_stop(stop);
        }
        if let Some(tools) = Self::map_tools(request.tools.as_deref()) {
            sdk_request = sdk_request.with_tools(tools);
        }
        if let Some(tool_choice) = Self::map_tool_choice(request.tool_choice.as_ref()) {
            sdk_request = sdk_request.with_tool_choice(tool_choice);
        }

        sdk_request
    }

    async fn fetch_capabilities(&self) -> ProviderCapabilities {
        match self.client.get_models_catalog().await {
            Ok(models) if !models.is_empty() => {
                let mut model_ids: Vec<String> = models.iter().map(|item| item.id.clone()).collect();
                model_ids.sort();
                model_ids.dedup();

                let max_context_tokens = models
                    .iter()
                    .filter_map(|item| item.context_length)
                    .max()
                    .unwrap_or(128_000);

                let supports = |flag: Option<&CapabilityFlag>| {
                    matches!(flag, Some(CapabilityFlag::Bool(true)))
                };

                let function_calling = models.iter().any(|item| {
                    supports(item.rainy_capabilities.as_ref().and_then(|caps| caps.tools.as_ref()))
                });
                let vision = models.iter().any(|item| {
                    supports(
                        item.rainy_capabilities
                            .as_ref()
                            .and_then(|caps| caps.image_input.as_ref()),
                    )
                });
                let web_search = true;

                ProviderCapabilities {
                    chat_completions: true,
                    embeddings: false,
                    streaming: true,
                    function_calling,
                    vision,
                    web_search,
                    max_context_tokens,
                    max_output_tokens: 65_536,
                    models: model_ids,
                }
            }
            _ => ProviderCapabilities {
                chat_completions: true,
                embeddings: false,
                streaming: true,
                function_calling: true,
                vision: true,
                web_search: true,
                max_context_tokens: 128_000,
                max_output_tokens: 65_536,
                models: vec![
                    "gemini-3-flash-preview".to_string(),
                    "gemini-3.1-flash-lite-preview".to_string(),
                    "gemini-3-flash-minimal".to_string(),
                    "gemini-3-flash-low".to_string(),
                    "gemini-3-flash-medium".to_string(),
                    "gemini-3-flash-high".to_string(),
                    "gemini-3-pro-preview".to_string(),
                    "gemini-3-pro-low".to_string(),
                    "gemini-3-pro-high".to_string(),
                    "gpt-4o".to_string(),
                    "gpt-5".to_string(),
                    "gpt-5-pro".to_string(),
                    "claude-sonnet-4".to_string(),
                    "claude-opus-4-1".to_string(),
                ],
            },
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
        {
            let cache = self.cached_capabilities.read().await;
            if let Some(caps) = cache.as_ref() {
                return Ok(caps.clone());
            }
        }

        let capabilities = self.fetch_capabilities().await;

        let mut cache = self.cached_capabilities.write().await;
        *cache = Some(capabilities.clone());
        Ok(capabilities)
    }

    async fn health_check(&self) -> ProviderResult<ProviderHealth> {
        let request = ChatCompletionRequest {
            messages: vec![crate::ai::provider_types::ChatMessage::user("Hello")],
            model: self.config.model.clone(),
            max_tokens: Some(10),
            ..Default::default()
        };

        match self.complete(request).await {
            Ok(_) => Ok(ProviderHealth::Healthy),
            Err(e) => {
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
        let api_request = Self::build_openai_request(&request);

        let response = self
            .client
            .create_openai_chat_completion(api_request)
            .await
            .map_err(|e| AIError::APIError(format!("Rainy API request failed: {}", e)))?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AIError::APIError("No response choices".to_string()))?;

        let content = choice.message.content.and_then(|content| match content {
            OpenAIMessageContent::Text(text) => Some(text),
            OpenAIMessageContent::Parts(parts) => {
                let text = parts
                    .into_iter()
                    .filter_map(|part| match part {
                        OpenAIContentPart::Text { text } => Some(text),
                        OpenAIContentPart::ImageUrl { .. } => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
        });

        let tool_calls = choice.message.tool_calls.map(|calls| {
            calls.into_iter()
                .map(|call| ToolCall {
                    id: call.id,
                    r#type: call.r#type,
                    extra_content: call.extra_content,
                    function: FunctionCall {
                        name: call.function.name,
                        arguments: call.function.arguments,
                    },
                })
                .collect()
        });

        Ok(ChatCompletionResponse {
            content,
            tool_calls,
            model: response.model,
            usage: {
                let (prompt, completion, total) = match response.usage.as_ref() {
                    Some(usage) => (
                        usage.prompt_tokens,
                        usage.completion_tokens,
                        usage.total_tokens,
                    ),
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
        let api_request = Self::build_openai_request(&request).with_stream(true);

        let mut stream = self
            .client
            .create_openai_chat_completion_stream(api_request)
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

                        callback(crate::ai::provider_types::StreamingChunk {
                            content,
                            thought,
                            is_final,
                            finish_reason,
                        });
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
