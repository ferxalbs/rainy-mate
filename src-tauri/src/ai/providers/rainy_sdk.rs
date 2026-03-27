use crate::ai::provider_trait::{AIProvider, AIProviderFactory};
use crate::ai::provider_types::{
    AIError, ChatCompletionRequest, ChatCompletionResponse, ContentPart, EmbeddingRequest,
    EmbeddingResponse, FunctionCall, MessageContent, ProviderCapabilities, ProviderConfig,
    ProviderHealth, ProviderId, ProviderResult, ProviderType, StreamingCallback, ToolCall,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use rainy_sdk::models::{
    build_reasoning_config, CapabilityFlag, FunctionDefinition, ModelCatalogItem,
    OpenAIChatCompletionRequest, OpenAIChatMessage, OpenAIContentPart, OpenAIFunctionCall,
    OpenAIImageUrl, OpenAIMessageContent, OpenAIMessageRole, OpenAIToolCall, ReasoningMode,
    ReasoningPreference, ResponsesApiResponse, ResponsesRequest, ThinkingConfig, ThinkingLevel,
    Tool, ToolChoice, ToolFunction, ToolType,
};
use rainy_sdk::RainyClient;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RainyTransport {
    ChatCompletions,
    Responses,
}

pub struct RainySDKProvider {
    config: ProviderConfig,
    client: RainyClient,
    cached_capabilities: tokio::sync::RwLock<Option<ProviderCapabilities>>,
    /// Catalog items cached from last successful get_models_catalog call.
    /// Used by resolve_reasoning_from_catalog to derive ThinkingConfig from v2 caps.
    cached_catalog: tokio::sync::RwLock<Vec<ModelCatalogItem>>,
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
            cached_catalog: tokio::sync::RwLock::new(Vec::new()),
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

    /// Derive a ThinkingConfig from v2 catalog capabilities given a base model ID and
    /// reasoning_effort value. Used when the model ID is a plain base slug (not a virtual
    /// thinking-level slug) and the caller provided an explicit reasoning_effort.
    fn thinking_config_from_catalog(
        model_id: &str,
        effort: &str,
        catalog: &[ModelCatalogItem],
    ) -> Option<ThinkingConfig> {
        let effort_lower = effort.to_lowercase();
        // "none" / "disabled" sentinel — caller wants thinking off; return no config.
        if matches!(effort_lower.as_str(), "none" | "disabled") {
            return None;
        }

        let clean_id = crate::ai::model_catalog::normalize_model_slug(model_id);
        let item = catalog
            .iter()
            .find(|item| crate::ai::model_catalog::normalize_model_slug(&item.id) == clean_id)?;
        let v2 = item.rainy_capabilities_v2.as_ref()?;
        let controls = v2.reasoning.controls.as_ref()?;
        let profiles = &v2.reasoning.profiles;

        // Determine which reasoning mode the model uses.
        // When explicit level/effort arrays are absent, fall back to profile parameter_path.
        let has_thinking_level_array = controls
            .thinking_level
            .as_ref()
            .is_some_and(|v| !v.is_empty());
        let has_effort_array = controls.reasoning_effort == Some(true)
            || controls.effort.as_ref().is_some_and(|v| !v.is_empty());
        let has_budget = controls.thinking_budget.is_some();
        let has_toggle = controls.reasoning_toggle == Some(true);

        let has_level_profile = profiles
            .iter()
            .any(|p| p.parameter_path == "thinking_config.thinking_level");
        let has_budget_profile = profiles.iter().any(|p| {
            p.parameter_path == "thinking.budget_tokens"
                || p.parameter_path == "thinking_config.thinking_budget"
        });
        let has_effort_profile = profiles
            .iter()
            .any(|p| p.parameter_path == "reasoning.effort");

        if has_effort_array || has_effort_profile {
            // Effort-mode models (gpt-5, o-series) use the Responses API path.
            return None;
        }

        // Budget mode — map effort string to token budget.
        if has_budget || has_budget_profile {
            let budget_tokens: i32 = match effort_lower.as_str() {
                "low" => 1024,
                "high" => 32768,
                "enabled" | "medium" | _ => 8192,
            };
            let mut config = ThinkingConfig::default();
            config.include_thoughts = Some(true);
            config.thinking_budget = Some(budget_tokens);
            return Some(config);
        }

        // Thinking-level mode — explicit array or profile hint.
        if !has_thinking_level_array && !has_level_profile && !has_toggle {
            return None;
        }

        // "enabled" with a toggle-only model — resolve to a sensible default level.
        let resolved_effort = if effort_lower == "enabled" { "medium" } else { effort };

        let mode = ReasoningMode::ThinkingLevel;
        let preference = ReasoningPreference {
            mode,
            value: Some(resolved_effort.to_string()),
            budget: None,
        };

        let payload = build_reasoning_config(item, &preference)?;
        let tc = payload.get("thinking_config")?;
        let level_str = tc.get("thinking_level")?.as_str()?;

        let thinking_level = match level_str.to_lowercase().as_str() {
            "minimal" => ThinkingLevel::Minimal,
            "low" => ThinkingLevel::Low,
            "medium" => ThinkingLevel::Medium,
            "high" => ThinkingLevel::High,
            _ => return None,
        };

        let mut config = ThinkingConfig::default();
        config.include_thoughts = Some(true);
        config.thinking_level = Some(thinking_level);
        Some(config)
    }

    fn resolve_transport(model_id: &str) -> RainyTransport {
        let normalized = crate::ai::model_catalog::normalize_model_slug(model_id);
        if normalized.starts_with("gpt-5")
            || normalized == "o3"
            || normalized.starts_with("o4")
            || normalized.starts_with("openai/gpt-5")
            || normalized.starts_with("openai/o3")
            || normalized.starts_with("openai/o4")
        {
            RainyTransport::Responses
        } else {
            RainyTransport::ChatCompletions
        }
    }

    fn resolve_transport_for_request(request: &ChatCompletionRequest) -> RainyTransport {
        let preferred = Self::resolve_transport(&request.model);
        let has_tools = request
            .tools
            .as_ref()
            .map(|tools| !tools.is_empty())
            .unwrap_or(false);

        if preferred == RainyTransport::Responses && has_tools {
            return RainyTransport::ChatCompletions;
        }

        preferred
    }

    fn map_content(content: &MessageContent) -> OpenAIMessageContent {
        match content {
            MessageContent::Text(text) => OpenAIMessageContent::Text(text.clone()),
            MessageContent::Parts(parts) => OpenAIMessageContent::Parts(
                parts
                    .iter()
                    .map(|part| match part {
                        ContentPart::Text { text } => {
                            OpenAIContentPart::Text { text: text.clone() }
                        }
                        ContentPart::ImageUrl { image_url } => OpenAIContentPart::ImageUrl {
                            image_url: OpenAIImageUrl {
                                url: image_url.url.clone(),
                                detail: image_url.detail.clone(),
                            },
                        },
                    })
                    .collect(),
            ),
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
                (MessageContent::Text(text), Some(tool_calls))
                    if text.is_empty() && !tool_calls.is_empty() =>
                {
                    None
                }
                (content, _) => Some(Self::map_content(content)),
            },
            name: message.name.clone(),
            tool_calls: message.tool_calls.as_ref().map(|calls| {
                calls
                    .iter()
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

    fn map_tools(tools: Option<&[crate::ai::provider_types::Tool]>) -> Option<Vec<Tool>> {
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
            // rainy-sdk models ToolChoice as an untagged enum with unit variants for
            // None/Auto, which serializes to JSON null instead of "none"/"auto".
            // Omit the field entirely and let the API default to automatic selection.
            Some(crate::ai::provider_types::ToolChoice::None) => None,
            Some(crate::ai::provider_types::ToolChoice::Auto) => None,
            Some(crate::ai::provider_types::ToolChoice::Tool(tool)) => Some(ToolChoice::Tool {
                r#type: ToolType::Function,
                function: ToolFunction {
                    name: tool.function.name.clone(),
                },
            }),
            None => None,
        }
    }

    fn build_openai_request(
        request: &ChatCompletionRequest,
        catalog: &[ModelCatalogItem],
    ) -> OpenAIChatCompletionRequest {
        let (model_id, mut thinking_config) = Self::map_model_id(&request.model);

        // When model is a plain base slug (no thinking level encoded), try to derive
        // ThinkingConfig from v2 catalog capabilities using the reasoning_effort field.
        if thinking_config.is_none() {
            if let Some(effort) = request.reasoning_effort.as_deref().filter(|s| !s.is_empty()) {
                thinking_config =
                    Self::thinking_config_from_catalog(&model_id, effort, catalog);
            }
        }

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

    fn map_response_content_parts(parts: &[ContentPart]) -> Value {
        Value::Array(
            parts
                .iter()
                .map(|part| match part {
                    ContentPart::Text { text } => json!({
                        "type": "input_text",
                        "text": text,
                    }),
                    ContentPart::ImageUrl { image_url } => {
                        let mut item = json!({
                            "type": "input_image",
                            "image_url": image_url.url,
                        });
                        if let Some(detail) = image_url.detail.as_ref() {
                            item["detail"] = Value::String(detail.clone());
                        }
                        item
                    }
                })
                .collect(),
        )
    }

    fn parse_json_or_string(raw: &str) -> Value {
        serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
    }

    fn map_responses_input_items(request: &ChatCompletionRequest) -> Vec<Value> {
        let mut items = Vec::new();

        for message in &request.messages {
            if let Some(tool_calls) = message.tool_calls.as_ref() {
                if !message.text().trim().is_empty() {
                    items.push(json!({
                        "role": "assistant",
                        "content": message.text(),
                    }));
                }

                for call in tool_calls {
                    items.push(json!({
                        "type": "function_call",
                        "call_id": call.id,
                        "name": call.function.name,
                        "arguments": call.function.arguments,
                    }));
                }
                continue;
            }

            if message.role == "tool" {
                items.push(json!({
                    "type": "function_call_output",
                    "call_id": message.tool_call_id.clone().unwrap_or_default(),
                    "output": Self::parse_json_or_string(&message.text()),
                }));
                continue;
            }

            let content = match &message.content {
                MessageContent::Text(text) => Value::String(text.clone()),
                MessageContent::Parts(parts) => Self::map_response_content_parts(parts),
            };

            items.push(json!({
                "role": message.role,
                "content": content,
            }));
        }

        items
    }

    fn map_responses_tool_choice(
        tool_choice: Option<&crate::ai::provider_types::ToolChoice>,
    ) -> Option<Value> {
        match tool_choice {
            Some(crate::ai::provider_types::ToolChoice::None) => Some(json!("none")),
            Some(crate::ai::provider_types::ToolChoice::Auto) => Some(json!("auto")),
            Some(crate::ai::provider_types::ToolChoice::Tool(tool)) => Some(json!({
                "type": "function",
                "name": tool.function.name,
            })),
            None => None,
        }
    }

    fn build_responses_request(request: &ChatCompletionRequest) -> ResponsesRequest {
        let (model_id, _) = Self::map_model_id(&request.model);
        let mut sdk_request = ResponsesRequest::new(
            model_id.clone(),
            Value::Array(Self::map_responses_input_items(request)),
        );

        sdk_request.stream = Some(request.stream);
        sdk_request.temperature = request.temperature;
        sdk_request.top_p = request.top_p;
        sdk_request.max_output_tokens = request.max_tokens;
        sdk_request.tool_choice = Self::map_responses_tool_choice(request.tool_choice.as_ref());
        sdk_request.user = Some("rainy-mate".to_string());

        if let Some(tools) = request.tools.as_ref() {
            let mapped_tools = tools
                .iter()
                .map(|tool| {
                    json!({
                        "type": "function",
                        "name": tool.function.name,
                        "description": tool.function.description,
                        "parameters": tool.function.parameters,
                    })
                })
                .collect();
            sdk_request.tools = Some(mapped_tools);
        }

        if model_id.starts_with("gpt-5") || model_id.starts_with("o") {
            let effort = request
                .reasoning_effort
                .as_deref()
                .filter(|v| !v.is_empty())
                .unwrap_or("medium");
            // "none"/"disabled" means the user wants to turn off reasoning entirely.
            if !matches!(effort.to_lowercase().as_str(), "none" | "disabled") {
                sdk_request.reasoning = Some(json!({ "effort": effort }));
            }
        }

        sdk_request
    }

    fn extract_text_from_output(output: &[Value]) -> Option<String> {
        let mut fragments = Vec::new();

        for item in output {
            if item.get("type").and_then(Value::as_str) == Some("message") {
                if let Some(content) = item.get("content").and_then(Value::as_array) {
                    for part in content {
                        if let Some(text) = part.get("text").and_then(Value::as_str) {
                            fragments.push(text.to_string());
                        } else if let Some(text) = part
                            .get("content")
                            .and_then(|content| content.get(0))
                            .and_then(|part| part.get("text"))
                            .and_then(Value::as_str)
                        {
                            fragments.push(text.to_string());
                        }
                    }
                }
            }
        }

        let text = fragments.join("");
        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    }

    fn extract_tool_calls_from_output(output: &[Value]) -> Option<Vec<ToolCall>> {
        let calls: Vec<ToolCall> = output
            .iter()
            .filter_map(|item| {
                let item_type = item.get("type").and_then(Value::as_str)?;
                if item_type != "function_call" {
                    return None;
                }

                let name = item.get("name").and_then(Value::as_str)?.to_string();
                let arguments = item
                    .get("arguments")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .or_else(|| item.get("arguments").map(Value::to_string))
                    .unwrap_or_else(|| "{}".to_string());
                let id = item
                    .get("call_id")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("id").and_then(Value::as_str))
                    .unwrap_or("call_missing_id")
                    .to_string();

                Some(ToolCall {
                    id,
                    r#type: "function".to_string(),
                    extra_content: Some(item.clone()),
                    function: FunctionCall { name, arguments },
                })
            })
            .collect();

        if calls.is_empty() {
            None
        } else {
            Some(calls)
        }
    }

    fn map_responses_response(
        transport: RainyTransport,
        response: ResponsesApiResponse,
    ) -> ProviderResult<ChatCompletionResponse> {
        let output = response.output.unwrap_or_default();
        let content = response
            .output_text
            .clone()
            .or_else(|| Self::extract_text_from_output(&output));
        let tool_calls = Self::extract_tool_calls_from_output(&output);
        let usage = response.usage.unwrap_or_default();
        let finish_reason = response
            .extra
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                if tool_calls.is_some() {
                    "tool_calls"
                } else {
                    "stop"
                }
            })
            .to_string();

        Ok(ChatCompletionResponse {
            content,
            tool_calls,
            model: response.model.unwrap_or_default(),
            usage: crate::ai::provider_types::TokenUsage {
                prompt_tokens: usage.input_tokens.unwrap_or(0),
                completion_tokens: usage.output_tokens.unwrap_or(0),
                total_tokens: usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0),
            },
            finish_reason,
            provider_metadata: Some(json!({
                "transport": match transport {
                    RainyTransport::ChatCompletions => "chat.completions",
                    RainyTransport::Responses => "responses",
                },
                "response_id": response.id,
                "object": response.object,
                "output": output,
            })),
        })
    }

    async fn fetch_capabilities(&self) -> ProviderCapabilities {
        match self.client.get_models_catalog().await {
            Ok(models) if !models.is_empty() => {
                // Cache the catalog for reasoning config resolution
                {
                    let mut cache = self.cached_catalog.write().await;
                    *cache = models.clone();
                }

                let mut model_ids: Vec<String> =
                    models.iter().map(|item| item.id.clone()).collect();
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
                    supports(
                        item.rainy_capabilities
                            .as_ref()
                            .and_then(|caps| caps.tools.as_ref()),
                    )
                });
                let vision = models.iter().any(|item| {
                    supports(
                        item.rainy_capabilities
                            .as_ref()
                            .and_then(|caps| caps.image_input.as_ref()),
                    )
                });

                ProviderCapabilities {
                    chat_completions: true,
                    embeddings: false,
                    streaming: true,
                    function_calling,
                    vision,
                    web_search: true,
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
                    "o3".to_string(),
                    "o4-mini".to_string(),
                    "claude-sonnet-4".to_string(),
                    "claude-opus-4-1".to_string(),
                ],
            },
        }
    }

    async fn complete_chat(
        &self,
        request: ChatCompletionRequest,
    ) -> ProviderResult<ChatCompletionResponse> {
        let catalog = self.cached_catalog.read().await;
        let api_request = Self::build_openai_request(&request, &catalog);

        let response = self
            .client
            .create_openai_chat_completion(api_request)
            .await
            .map_err(|e| {
                AIError::APIError(format!(
                    "Rainy chat.completions request failed for model '{}': {}",
                    request.model, e
                ))
            })?;

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
            calls
                .into_iter()
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
            provider_metadata: Some(json!({
                "transport": "chat.completions",
            })),
        })
    }

    async fn complete_responses(
        &self,
        request: ChatCompletionRequest,
    ) -> ProviderResult<ChatCompletionResponse> {
        let api_request = Self::build_responses_request(&request);
        let (response, metadata) = self
            .client
            .create_response(api_request)
            .await
            .map_err(|e| {
                AIError::APIError(format!(
                    "Rainy responses request failed for model '{}': {}",
                    request.model, e
                ))
            })?;

        let mut mapped = Self::map_responses_response(RainyTransport::Responses, response)?;
        if let Some(existing) = mapped.provider_metadata.as_mut() {
            existing["response_mode"] = metadata
                .response_mode
                .map(Value::String)
                .unwrap_or(Value::Null);
            existing["billing_plan"] = metadata
                .billing_plan
                .map(Value::String)
                .unwrap_or(Value::Null);
        }
        Ok(mapped)
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
        match Self::resolve_transport_for_request(&request) {
            RainyTransport::ChatCompletions => self.complete_chat(request).await,
            RainyTransport::Responses => self.complete_responses(request).await,
        }
    }

    async fn complete_stream(
        &self,
        request: ChatCompletionRequest,
        callback: StreamingCallback,
    ) -> ProviderResult<()> {
        match Self::resolve_transport_for_request(&request) {
            RainyTransport::ChatCompletions => {
                let catalog = self.cached_catalog.read().await;
                let api_request =
                    Self::build_openai_request(&request, &catalog).with_stream(true);

                let mut stream = self
                    .client
                    .create_openai_chat_completion_stream(api_request)
                    .await
                    .map_err(|e| {
                        AIError::APIError(format!(
                            "Rainy chat.completions stream failed for model '{}': {}",
                            request.model, e
                        ))
                    })?;

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
                            return Err(AIError::APIError(format!(
                                "Rainy chat.completions stream error for model '{}': {}",
                                request.model, e
                            )));
                        }
                    }
                }

                Ok(())
            }
            RainyTransport::Responses => {
                let response = self.complete_responses(request).await?;
                callback(crate::ai::provider_types::StreamingChunk {
                    content: response.content.unwrap_or_default(),
                    thought: None,
                    is_final: true,
                    finish_reason: Some(response.finish_reason),
                });
                Ok(())
            }
        }
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
