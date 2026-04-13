use crate::ai::provider_trait::{AIProvider, AIProviderFactory};
use crate::ai::provider_types::{
    AIError, ChatCompletionRequest, ChatCompletionResponse, ContentPart, EmbeddingRequest,
    EmbeddingResponse, FunctionCall, MessageContent, ProviderCapabilities, ProviderConfig,
    ProviderEventCallback, ProviderHealth, ProviderId, ProviderResult, ProviderStreamEvent,
    ProviderStreamUsage, ProviderToolCallDelta, ProviderToolLifecycleEvent,
    ProviderToolLifecycleState, ProviderType, StreamingCallback, ToolCall,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use rainy_sdk::models::{
    build_reasoning_config, CapabilityFlag, ChatStreamEvent, FunctionDefinition, ModelCatalogItem,
    OpenAIChatCompletionRequest, OpenAIChatMessage, OpenAIContentPart, OpenAIFunctionCall,
    OpenAIImageUrl, OpenAIMessageContent, OpenAIMessageRole, OpenAIToolCall, ReasoningMode,
    ReasoningPreference, ResponsesApiResponse, ResponsesRequest, ResponsesUsage, ThinkingConfig,
    ThinkingLevel, Tool, ToolChoice, ToolFunction, ToolType,
};
use rainy_sdk::RainyClient;
use serde_json::{json, Value};
use std::collections::HashMap;
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
    fn map_stream_usage(
        model: &str,
        usage: Option<rainy_sdk::models::Usage>,
    ) -> Option<ProviderStreamUsage> {
        usage.map(|usage| ProviderStreamUsage {
            model: Some(model.to_string()),
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        })
    }

    fn map_stream_tool_delta(tool_call: &rainy_sdk::models::ToolCall) -> ProviderToolCallDelta {
        ProviderToolCallDelta {
            index: tool_call.index,
            id: tool_call.id.clone(),
            r#type: tool_call.r#type.clone(),
            name: tool_call
                .function
                .as_ref()
                .and_then(|function| function.name.clone()),
            arguments: tool_call
                .function
                .as_ref()
                .and_then(|function| function.arguments.clone()),
        }
    }

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
        let resolved_effort = if effort_lower == "enabled" {
            "medium"
        } else {
            effort
        };

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
            if let Some(effort) = request
                .reasoning_effort
                .as_deref()
                .filter(|s| !s.is_empty())
            {
                thinking_config = Self::thinking_config_from_catalog(&model_id, effort, catalog);
            }
        }

        let is_anthropic =
            crate::ai::model_catalog::normalize_model_slug(&model_id).starts_with("anthropic/");

        let mut sdk_request = OpenAIChatCompletionRequest::new(
            model_id,
            request.messages.iter().map(Self::map_message).collect(),
        );

        if let Some(config) = thinking_config {
            if is_anthropic {
                // Anthropic extended thinking uses `{"thinking":{"type":"enabled","budget_tokens":N}}`
                // (not `thinking_config`). Map the budget value from ThinkingConfig.
                let budget = config.thinking_budget.unwrap_or(8192);
                sdk_request = sdk_request.with_anthropic_thinking(budget);
            } else {
                sdk_request = sdk_request.with_thinking_config(config);
            }
        }
        if let Some(max_tokens) = request.max_tokens {
            sdk_request = sdk_request.with_max_tokens(max_tokens);
        }
        if let Some(max_completion_tokens) = request.max_completion_tokens {
            sdk_request = sdk_request.with_max_completion_tokens(max_completion_tokens);
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
        if let Some(stream_options) = request.stream_options.clone() {
            sdk_request = sdk_request.with_stream_options(stream_options);
        }
        if let Some(tools) = Self::map_tools(request.tools.as_deref()) {
            sdk_request = sdk_request.with_tools(tools);
        }
        if let Some(tool_choice) = Self::map_tool_choice(request.tool_choice.as_ref()) {
            sdk_request = sdk_request.with_tool_choice(tool_choice);
        }
        if let Some(parallel_tool_calls) = request.parallel_tool_calls {
            sdk_request.parallel_tool_calls = Some(parallel_tool_calls);
        }
        if let Some(seed) = request.seed {
            sdk_request.seed = Some(seed);
        }
        if let Some(prompt_cache_key) = request.prompt_cache_key.clone() {
            sdk_request.prompt_cache_key = Some(prompt_cache_key);
        }
        if let Some(provider) = request.provider.clone() {
            sdk_request = sdk_request.with_provider(provider);
        }
        if let Some(provider_options) = request.provider_options.clone() {
            sdk_request.provider_options = Some(provider_options);
        }
        if let Some(prompt_cache_retention) = request.prompt_cache_retention.clone() {
            sdk_request.prompt_cache_retention = Some(prompt_cache_retention);
        }
        if let Some(reasoning) = request.reasoning.clone() {
            sdk_request = sdk_request.with_reasoning(reasoning);
        }
        if let Some(include_reasoning) = request.include_reasoning {
            sdk_request = sdk_request.with_include_reasoning(include_reasoning);
        }
        if let Some(metadata) = request.metadata.clone() {
            sdk_request = sdk_request.with_metadata(metadata);
        }
        if let Some(service_tier) = request.service_tier.clone() {
            sdk_request = sdk_request.with_service_tier(service_tier);
        }
        if let Some(store) = request.store {
            sdk_request.store = Some(store);
        }
        if let Some(safety_identifier) = request.safety_identifier.clone() {
            sdk_request.safety_identifier = Some(safety_identifier);
        }
        if let Some(modalities) = request.modalities.clone() {
            sdk_request.modalities = Some(modalities);
        }
        if let Some(audio) = request.audio.clone() {
            sdk_request.audio = Some(audio);
        }
        if let Some(prediction) = request.prediction.clone() {
            sdk_request.prediction = Some(prediction);
        }
        if let Some(verbosity) = request.verbosity.clone() {
            sdk_request.verbosity = Some(verbosity);
        }
        if let Some(web_search_options) = request.web_search_options.clone() {
            sdk_request.web_search_options = Some(web_search_options);
        }
        if let Some(functions) = request.functions.clone() {
            sdk_request.functions = Some(functions);
        }
        if let Some(function_call) = request.function_call.clone() {
            sdk_request.function_call = Some(function_call);
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
        sdk_request.max_output_tokens = request.max_completion_tokens.or(request.max_tokens);
        sdk_request.tool_choice = Self::map_responses_tool_choice(request.tool_choice.as_ref());
        sdk_request.user = Some("rainy-mate".to_string());
        sdk_request.prompt_cache_key = request.prompt_cache_key.clone();
        sdk_request.reasoning = request.reasoning.clone();
        sdk_request.include_reasoning = request.include_reasoning;
        sdk_request.parallel_tool_calls = request.parallel_tool_calls;
        sdk_request.metadata = request.metadata.clone();
        sdk_request.service_tier = request.service_tier.clone();
        sdk_request.store = request.store;
        sdk_request.safety_identifier = request.safety_identifier.clone();
        sdk_request.provider_options = request.provider_options.clone();
        sdk_request.prompt_cache_retention = request.prompt_cache_retention.clone();
        sdk_request.text = request.text.clone();
        sdk_request.instructions = request.instructions.clone();
        sdk_request.include = request.include.clone();
        sdk_request.previous_response_id = request.previous_response_id.clone();
        sdk_request.conversation = request.conversation.clone();
        sdk_request.prompt = request.prompt.clone();
        sdk_request.background = request.background;
        sdk_request.context_management = request.context_management.clone();
        sdk_request.truncation = request.truncation.clone();

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
            if sdk_request.reasoning.is_none()
                && !matches!(effort.to_lowercase().as_str(), "none" | "disabled")
            {
                sdk_request.reasoning = Some(json!({ "effort": effort }));
            }
        }

        sdk_request
    }

    fn response_event_type(value: &Value) -> Option<&str> {
        value.get("type").and_then(Value::as_str)
    }

    fn response_usage(value: &Value) -> Option<ResponsesUsage> {
        value
            .pointer("/response/usage")
            .cloned()
            .or_else(|| value.get("usage").cloned())
            .and_then(|usage| serde_json::from_value::<ResponsesUsage>(usage).ok())
    }

    fn map_responses_usage(
        model: &str,
        usage: Option<ResponsesUsage>,
    ) -> Option<ProviderStreamUsage> {
        usage.map(|usage| {
            let prompt_tokens = usage.input_tokens.unwrap_or(0)
                + usage.cache_creation_input_tokens.unwrap_or(0)
                + usage.cache_read_input_tokens.unwrap_or(0);
            let completion_tokens = usage.output_tokens.unwrap_or(0);
            ProviderStreamUsage {
                model: Some(model.to_string()),
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            }
        })
    }

    fn response_text_delta(value: &Value) -> Option<(bool, String)> {
        let event_type = Self::response_event_type(value)?;
        let text = value
            .get("delta")
            .and_then(Value::as_str)
            .or_else(|| value.get("text").and_then(Value::as_str))
            .map(ToString::to_string)?;

        if text.is_empty() {
            return None;
        }

        if event_type.contains("output_text") {
            return Some((false, text));
        }

        if event_type.contains("reasoning") {
            return Some((true, text));
        }

        None
    }

    fn response_tool_delta_from_item(item: &Value) -> Option<ProviderToolCallDelta> {
        let item_type = item.get("type").and_then(Value::as_str)?;
        if item_type != "function_call" {
            return None;
        }

        let index = item
            .get("output_index")
            .and_then(Value::as_u64)
            .or_else(|| item.get("index").and_then(Value::as_u64))
            .unwrap_or(0) as u32;

        Some(ProviderToolCallDelta {
            index,
            id: item
                .get("call_id")
                .and_then(Value::as_str)
                .or_else(|| item.get("id").and_then(Value::as_str))
                .map(ToString::to_string),
            r#type: Some("function".to_string()),
            name: item
                .get("name")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            arguments: item
                .get("arguments")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .or_else(|| item.get("arguments").map(Value::to_string)),
        })
    }

    fn response_tool_delta(value: &Value) -> Option<ProviderToolCallDelta> {
        let event_type = Self::response_event_type(value)?;
        if event_type.contains("function_call_arguments") {
            return Some(ProviderToolCallDelta {
                index: value
                    .get("output_index")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                id: value
                    .get("call_id")
                    .and_then(Value::as_str)
                    .or_else(|| value.get("item_id").and_then(Value::as_str))
                    .map(ToString::to_string),
                r#type: Some("function".to_string()),
                name: value
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                arguments: value
                    .get("delta")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .or_else(|| {
                        value
                            .get("arguments")
                            .and_then(Value::as_str)
                            .map(ToString::to_string)
                    }),
            });
        }

        value
            .get("item")
            .and_then(Self::response_tool_delta_from_item)
            .or_else(|| {
                value
                    .get("output_item")
                    .and_then(Self::response_tool_delta_from_item)
            })
    }

    fn completed_response_from_event(value: Value) -> Option<ResponsesApiResponse> {
        if let Some(response) = value.get("response").cloned() {
            return serde_json::from_value::<ResponsesApiResponse>(response).ok();
        }

        if value.get("output_text").is_some() || value.get("output").is_some() {
            return serde_json::from_value::<ResponsesApiResponse>(value).ok();
        }

        None
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
                    airlock_level: None,
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
                    tool_call_streaming: function_calling,
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
                tool_call_streaming: true,
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
                    airlock_level: None,
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

    async fn complete_responses_event_stream(
        &self,
        request: &ChatCompletionRequest,
        callback: ProviderEventCallback,
    ) -> ProviderResult<()> {
        let api_request = Self::build_responses_request(request).with_stream(true);
        let mut stream = self
            .client
            .create_response_stream(api_request)
            .await
            .map_err(|e| {
                AIError::APIError(format!(
                    "Rainy responses stream failed for model '{}': {}",
                    request.model, e
                ))
            })?;

        let mut latest_tool_calls: HashMap<u32, ProviderToolCallDelta> = HashMap::new();
        let mut announced_tool_calls: HashMap<u32, ProviderToolCallDelta> = HashMap::new();
        let mut assistant_text = String::new();
        let mut last_finish_reason: Option<String> = None;

        while let Some(event_result) = stream.next().await {
            let raw = event_result.map_err(|e| {
                AIError::APIError(format!(
                    "Rainy responses stream error for model '{}': {}",
                    request.model, e
                ))
            })?;

            if let Some((is_reasoning, delta)) = Self::response_text_delta(&raw) {
                if is_reasoning {
                    callback(ProviderStreamEvent::ThoughtDelta(delta));
                } else {
                    assistant_text.push_str(&delta);
                    callback(ProviderStreamEvent::TextDelta(delta));
                }
            }

            if let Some(mapped) = Self::response_tool_delta(&raw) {
                let state = if announced_tool_calls.contains_key(&mapped.index) {
                    ProviderToolLifecycleState::ArgumentsDelta
                } else {
                    ProviderToolLifecycleState::Announced
                };
                latest_tool_calls.insert(mapped.index, mapped.clone());
                announced_tool_calls
                    .entry(mapped.index)
                    .or_insert_with(|| mapped.clone());
                callback(ProviderStreamEvent::ToolCallDelta(
                    ProviderToolLifecycleEvent {
                        state,
                        tool_call: mapped,
                    },
                ));
            }

            if let Some(usage) =
                Self::map_responses_usage(&request.model, Self::response_usage(&raw))
            {
                callback(ProviderStreamEvent::Usage(usage));
            }

            if matches!(
                Self::response_event_type(&raw),
                Some("response.completed" | "response.failed" | "response.incomplete")
            ) {
                if let Some(response) = Self::completed_response_from_event(raw.clone()) {
                    if let Some(final_text) = response.output_text.clone() {
                        let remaining = final_text
                            .strip_prefix(&assistant_text)
                            .unwrap_or(final_text.as_str());
                        if !remaining.is_empty() {
                            assistant_text.push_str(remaining);
                            callback(ProviderStreamEvent::TextDelta(remaining.to_string()));
                        }
                    }

                    if let Some(usage) =
                        Self::map_responses_usage(&request.model, response.usage.clone())
                    {
                        callback(ProviderStreamEvent::Usage(usage));
                    }

                    if let Some(output) = response.output.clone() {
                        if let Some(tool_calls) = Self::extract_tool_calls_from_output(&output) {
                            for (index, tool_call) in tool_calls.into_iter().enumerate() {
                                callback(ProviderStreamEvent::ToolCallDelta(
                                    ProviderToolLifecycleEvent {
                                        state: ProviderToolLifecycleState::Ready,
                                        tool_call: ProviderToolCallDelta {
                                            index: index as u32,
                                            id: Some(tool_call.id),
                                            r#type: Some(tool_call.r#type),
                                            name: Some(tool_call.function.name),
                                            arguments: Some(tool_call.function.arguments),
                                        },
                                    },
                                ));
                            }
                        }
                    } else if !latest_tool_calls.is_empty() {
                        for tool_call in latest_tool_calls.values() {
                            callback(ProviderStreamEvent::ToolCallDelta(
                                ProviderToolLifecycleEvent {
                                    state: ProviderToolLifecycleState::Ready,
                                    tool_call: tool_call.clone(),
                                },
                            ));
                        }
                    }

                    last_finish_reason = response.status.clone().or_else(|| {
                        if latest_tool_calls.is_empty() {
                            Some("stop".to_string())
                        } else {
                            Some("tool_calls".to_string())
                        }
                    });
                }
            }

            callback(ProviderStreamEvent::Raw(raw));
        }

        callback(ProviderStreamEvent::Completed {
            finish_reason: last_finish_reason,
        });
        Ok(())
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

    async fn complete_event_stream(
        &self,
        request: ChatCompletionRequest,
        callback: ProviderEventCallback,
    ) -> ProviderResult<()> {
        match Self::resolve_transport_for_request(&request) {
            RainyTransport::ChatCompletions => {
                let catalog = self.cached_catalog.read().await;
                let api_request = Self::build_openai_request(&request, &catalog).with_stream(true);

                let mut stream = self
                    .client
                    .create_openai_chat_completion_stream_events(api_request)
                    .await
                    .map_err(|e| {
                        AIError::APIError(format!(
                            "Rainy chat.completions stream failed for model '{}': {}",
                            request.model, e
                        ))
                    })?;

                let mut latest_tool_calls: HashMap<u32, ProviderToolCallDelta> = HashMap::new();
                let mut announced_tool_calls: HashMap<u32, ProviderToolCallDelta> = HashMap::new();
                let mut last_finish_reason: Option<String> = None;

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(ChatStreamEvent::Chunk(chunk)) => {
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(content) = choice
                                    .delta
                                    .content
                                    .as_ref()
                                    .filter(|value| !value.is_empty())
                                {
                                    callback(ProviderStreamEvent::TextDelta(content.clone()));
                                }

                                if let Some(thought) = choice
                                    .delta
                                    .thought
                                    .as_ref()
                                    .filter(|value| !value.is_empty())
                                {
                                    callback(ProviderStreamEvent::ThoughtDelta(thought.clone()));
                                }

                                if let Some(tool_calls) = choice.delta.tool_calls.as_ref() {
                                    for tool_call in tool_calls {
                                        let mapped = Self::map_stream_tool_delta(tool_call);
                                        let state = if announced_tool_calls
                                            .contains_key(&tool_call.index)
                                        {
                                            ProviderToolLifecycleState::ArgumentsDelta
                                        } else {
                                            ProviderToolLifecycleState::Announced
                                        };
                                        latest_tool_calls.insert(tool_call.index, mapped.clone());
                                        announced_tool_calls
                                            .entry(tool_call.index)
                                            .or_insert_with(|| mapped.clone());
                                        callback(ProviderStreamEvent::ToolCallDelta(
                                            ProviderToolLifecycleEvent {
                                                state,
                                                tool_call: mapped,
                                            },
                                        ));
                                    }
                                }

                                if let Some(usage) =
                                    Self::map_stream_usage(&chunk.model, chunk.usage.clone())
                                {
                                    callback(ProviderStreamEvent::Usage(usage));
                                }

                                if let Some(finish_reason) = choice.finish_reason.clone() {
                                    last_finish_reason = Some(finish_reason.clone());
                                    if finish_reason == "tool_calls" {
                                        for tool_call in latest_tool_calls.values() {
                                            callback(ProviderStreamEvent::ToolCallDelta(
                                                ProviderToolLifecycleEvent {
                                                    state: ProviderToolLifecycleState::Ready,
                                                    tool_call: tool_call.clone(),
                                                },
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        Ok(ChatStreamEvent::Billing(billing)) => {
                            if let Some(usage) = billing.usage {
                                callback(ProviderStreamEvent::Usage(ProviderStreamUsage {
                                    model: Some(request.model.clone()),
                                    prompt_tokens: usage.prompt_tokens.unwrap_or(0),
                                    completion_tokens: usage.completion_tokens.unwrap_or(0),
                                    total_tokens: usage.prompt_tokens.unwrap_or(0)
                                        + usage.completion_tokens.unwrap_or(0),
                                }));
                            }
                        }
                        Ok(ChatStreamEvent::Raw(raw)) => {
                            callback(ProviderStreamEvent::Raw(raw));
                        }
                        Err(e) => {
                            return Err(AIError::APIError(format!(
                                "Rainy chat.completions stream error for model '{}': {}",
                                request.model, e
                            )));
                        }
                    }
                }

                callback(ProviderStreamEvent::Completed {
                    finish_reason: last_finish_reason,
                });
                Ok(())
            }
            RainyTransport::Responses => {
                self.complete_responses_event_stream(&request, callback)
                    .await
            }
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
                let api_request = Self::build_openai_request(&request, &catalog).with_stream(true);

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
                let streaming_callback = Arc::clone(&callback);
                let adapter: ProviderEventCallback = Arc::new(move |event| match event {
                    ProviderStreamEvent::TextDelta(content) => {
                        if !content.is_empty() {
                            streaming_callback(crate::ai::provider_types::StreamingChunk {
                                content,
                                thought: None,
                                is_final: false,
                                finish_reason: None,
                            });
                        }
                    }
                    ProviderStreamEvent::ThoughtDelta(thought) => {
                        if !thought.is_empty() {
                            streaming_callback(crate::ai::provider_types::StreamingChunk {
                                content: String::new(),
                                thought: Some(thought),
                                is_final: false,
                                finish_reason: None,
                            });
                        }
                    }
                    ProviderStreamEvent::Completed { finish_reason } => {
                        streaming_callback(crate::ai::provider_types::StreamingChunk {
                            content: String::new(),
                            thought: None,
                            is_final: true,
                            finish_reason,
                        });
                    }
                    ProviderStreamEvent::ToolCallDelta(_)
                    | ProviderStreamEvent::Usage(_)
                    | ProviderStreamEvent::Raw(_) => {}
                });

                self.complete_responses_event_stream(&request, adapter)
                    .await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_text_delta_maps_output_and_reasoning_events() {
        let output = json!({
            "type": "response.output_text.delta",
            "delta": "hello"
        });
        let reasoning = json!({
            "type": "response.reasoning_text.delta",
            "delta": "thinking"
        });

        assert_eq!(
            RainySDKProvider::response_text_delta(&output),
            Some((false, "hello".to_string()))
        );
        assert_eq!(
            RainySDKProvider::response_text_delta(&reasoning),
            Some((true, "thinking".to_string()))
        );
    }

    #[test]
    fn response_tool_delta_maps_argument_deltas_and_output_items() {
        let arguments_delta = json!({
            "type": "response.function_call_arguments.delta",
            "output_index": 2,
            "call_id": "call_123",
            "name": "list_files",
            "delta": "{\"path\":\"src\"}"
        });
        let output_item = json!({
            "type": "response.output_item.added",
            "item": {
                "type": "function_call",
                "output_index": 3,
                "call_id": "call_456",
                "name": "read_file",
                "arguments": "{\"path\":\"Cargo.toml\"}"
            }
        });

        let mapped_delta = RainySDKProvider::response_tool_delta(&arguments_delta)
            .expect("argument delta should map");
        assert_eq!(mapped_delta.index, 2);
        assert_eq!(mapped_delta.id.as_deref(), Some("call_123"));
        assert_eq!(mapped_delta.name.as_deref(), Some("list_files"));

        let mapped_item =
            RainySDKProvider::response_tool_delta(&output_item).expect("output item should map");
        assert_eq!(mapped_item.index, 3);
        assert_eq!(mapped_item.id.as_deref(), Some("call_456"));
        assert_eq!(mapped_item.name.as_deref(), Some("read_file"));
    }

    #[test]
    fn completed_response_from_event_reads_nested_response_payload() {
        let event = json!({
            "type": "response.completed",
            "response": {
                "id": "resp_1",
                "model": "openai/gpt-5",
                "status": "completed",
                "output_text": "done",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 4
                }
            }
        });

        let response = RainySDKProvider::completed_response_from_event(event)
            .expect("completed response should deserialize");
        assert_eq!(response.output_text.as_deref(), Some("done"));
        assert_eq!(response.status.as_deref(), Some("completed"));
        assert_eq!(
            response.usage.and_then(|usage| usage.input_tokens),
            Some(10)
        );
    }
}
