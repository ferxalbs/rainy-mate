// Gemini BYOK Provider Adapter for IntelligentRouter
//
// Wraps the Google Generative Language API to implement the full AIProvider
// trait so it can be added to the IntelligentRouter. Supports multi-turn
// conversations (chat history), non-streaming completions (for tool-calling
// turns), and SSE streaming for plain-text turns.

use crate::ai::provider_trait::{AIProvider, AIProviderFactory};
use crate::ai::provider_types::{
    AIError, ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    MessageContent, ProviderCapabilities, ProviderConfig, ProviderHealth, ProviderId,
    ProviderResult, ProviderType, StreamingCallback, StreamingChunk, TokenUsage,
};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

// ─── Gemini API types ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_config: Option<GeminiToolConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiSystemInstruction {
    parts: Vec<GeminiTextPart>,
}

/// A text-only part used for system instructions (always text).
#[derive(Debug, Serialize, Deserialize)]
struct GeminiTextPart {
    text: String,
}

/// A part in a message — can be text or a function call from the model.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text {
        text: String,
    },
    FunctionCall {
        function_call: GeminiFunctionCall,
        #[serde(skip_serializing_if = "Option::is_none")]
        thought_signature: Option<String>,
        #[serde(
            rename = "thoughtSignature",
            skip_serializing_if = "Option::is_none",
            default
        )]
        thought_signature_camel: Option<String>,
    },
    FunctionResponse {
        function_response: GeminiFunctionResponse,
    },
    // Catch-all for unknown part types (e.g. inlineData) — skip text extraction.
    Unknown(serde_json::Value),
}

impl GeminiPart {
    fn as_text(&self) -> Option<&str> {
        if let GeminiPart::Text { text } = self {
            Some(text.as_str())
        } else {
            None
        }
    }
}

/// Function call issued by the model (inside a GeminiPart).
#[derive(Debug, Serialize, Deserialize)]
struct GeminiFunctionCall {
    name: String,
    args: serde_json::Value,
}

/// Function response sent back to Gemini after executing a tool.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFunctionResponse {
    name: String,
    response: serde_json::Value,
}

// ─── Gemini Tool declaration types ──────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiTool {
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiThinkingConfig {
    thinking_level: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_config: Option<GeminiThinkingConfig>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiToolConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    function_calling_config: Option<GeminiFunctionCallingConfig>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFunctionCallingConfig {
    mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_function_names: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: GeminiContent,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsage {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    total_token_count: Option<u32>,
}

// SSE streaming chunk from Gemini
#[derive(Debug, Deserialize)]
struct GeminiStreamChunk {
    candidates: Option<Vec<GeminiCandidate>>,
}

/// Map our internal model slug to the actual Gemini API model ID.
fn resolve_model_id(model: &str) -> String {
    let normalized = crate::ai::model_catalog::normalize_model_slug(model);
    match normalized {
        "gemini-3-flash-minimal" | "gemini-3-flash-high" | "gemini-3-flash-preview" => {
            "gemini-3-flash-preview".to_string()
        }
        "gemini-3.1-flash-lite-preview" => "gemini-3.1-flash-lite-preview".to_string(),
        // Already a full API id or unknown — pass through unchanged
        other => other.to_string(),
    }
}

/// Extract thinking level from user-provided model slug if available
fn extract_thinking_level(model: &str) -> Option<String> {
    let normalized = crate::ai::model_catalog::normalize_model_slug(model);
    if normalized.contains("-minimal") {
        Some("minimal".to_string())
    } else if normalized.contains("-low") {
        Some("low".to_string())
    } else if normalized.contains("-medium") {
        Some("medium".to_string())
    } else if normalized.contains("-high") {
        Some("high".to_string())
    } else {
        None
    }
}

/// Convert our ChatMessage list to Gemini's `contents` array.
/// System messages are extracted separately into `system_instruction`.
fn build_gemini_request_parts(
    messages: &[crate::ai::provider_types::ChatMessage],
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    thinking_level: Option<String>,
) -> (
    Option<GeminiSystemInstruction>,
    Vec<GeminiContent>,
    Option<GeminiGenerationConfig>,
) {
    let mut system_text_parts: Vec<GeminiTextPart> = Vec::new();
    let mut contents: Vec<GeminiContent> = Vec::new();
    let mut tool_name_by_id: HashMap<String, String> = HashMap::new();

    for msg in messages {
        let text = msg.content.text();
        match msg.role.as_str() {
            "system" => {
                system_text_parts.push(GeminiTextPart { text });
            }
            "user" => {
                contents.push(GeminiContent {
                    role: "user".to_string(),
                    parts: vec![GeminiPart::Text { text }],
                });
            }
            "assistant" => {
                let mut parts: Vec<GeminiPart> = Vec::new();

                if !text.is_empty() {
                    parts.push(GeminiPart::Text { text });
                }

                if let Some(calls) = msg.tool_calls.as_ref() {
                    for call in calls {
                        let args =
                            serde_json::from_str::<serde_json::Value>(&call.function.arguments)
                                .unwrap_or_else(|_| {
                                    serde_json::json!({
                                        "raw_arguments": call.function.arguments
                                    })
                                });
                        tool_name_by_id.insert(call.id.clone(), call.function.name.clone());
                        let thought_signature = call
                            .extra_content
                            .as_ref()
                            .and_then(|extra| extra.get("google"))
                            .and_then(|google| google.get("thought_signature"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        parts.push(GeminiPart::FunctionCall {
                            function_call: GeminiFunctionCall {
                                name: call.function.name.clone(),
                                args,
                            },
                            thought_signature: thought_signature.clone(),
                            thought_signature_camel: thought_signature,
                        });
                    }
                }

                if !parts.is_empty() {
                    contents.push(GeminiContent {
                        role: "model".to_string(),
                        parts,
                    });
                }
            }
            "tool" => {
                let tool_name = msg
                    .tool_call_id
                    .as_ref()
                    .and_then(|id| tool_name_by_id.get(id))
                    .cloned()
                    .or_else(|| msg.name.clone())
                    .unwrap_or_else(|| "tool_result".to_string());

                let tool_payload = if text.is_empty() {
                    serde_json::json!({})
                } else {
                    serde_json::from_str::<serde_json::Value>(&text).unwrap_or_else(|_| {
                        serde_json::json!({
                            "content": text
                        })
                    })
                };

                contents.push(GeminiContent {
                    role: "user".to_string(),
                    parts: vec![GeminiPart::FunctionResponse {
                        function_response: GeminiFunctionResponse {
                            name: tool_name,
                            response: serde_json::json!({
                                "result": tool_payload
                            }),
                        },
                    }],
                });
            }
            // tool / other roles — append as user turn so the conversation stays coherent.
            _ => {
                contents.push(GeminiContent {
                    role: "user".to_string(),
                    parts: vec![GeminiPart::Text {
                        text: format!("[tool result]\n{}", text),
                    }],
                });
            }
        }
    }

    let system_instruction = if system_text_parts.is_empty() {
        None
    } else {
        Some(GeminiSystemInstruction {
            parts: system_text_parts,
        })
    };

    let generation_config =
        if temperature.is_some() || max_tokens.is_some() || thinking_level.is_some() {
            Some(GeminiGenerationConfig {
                temperature,
                max_output_tokens: max_tokens,
                thinking_config: thinking_level.map(|lvl| GeminiThinkingConfig {
                    thinking_level: lvl,
                }),
            })
        } else {
            None
        };

    (system_instruction, contents, generation_config)
}

fn build_function_calling_config(
    tools: Option<&[crate::ai::provider_types::Tool]>,
    tool_choice: Option<&crate::ai::provider_types::ToolChoice>,
) -> Option<GeminiFunctionCallingConfig> {
    let has_tools = tools.map(|t| !t.is_empty()).unwrap_or(false);
    if !has_tools {
        return None;
    }

    match tool_choice {
        Some(crate::ai::provider_types::ToolChoice::None) => Some(GeminiFunctionCallingConfig {
            mode: "NONE".to_string(),
            allowed_function_names: None,
        }),
        Some(crate::ai::provider_types::ToolChoice::Tool(tool)) => {
            Some(GeminiFunctionCallingConfig {
                mode: "ANY".to_string(),
                allowed_function_names: Some(vec![tool.function.name.clone()]),
            })
        }
        Some(crate::ai::provider_types::ToolChoice::Auto) => Some(GeminiFunctionCallingConfig {
            // Agent runtime requires deterministic, tool-first behavior to avoid fabricated text.
            mode: "ANY".to_string(),
            allowed_function_names: None,
        }),
        None => Some(GeminiFunctionCallingConfig {
            mode: "AUTO".to_string(),
            allowed_function_names: None,
        }),
    }
}

fn build_tool_config(
    tools: Option<&[crate::ai::provider_types::Tool]>,
    tool_choice: Option<&crate::ai::provider_types::ToolChoice>,
) -> Option<GeminiToolConfig> {
    let function_calling_config = build_function_calling_config(tools, tool_choice);
    if function_calling_config.is_none() {
        return None;
    }
    Some(GeminiToolConfig {
        function_calling_config,
    })
}

fn normalize_gemini_type(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "string" => Some("STRING"),
        "integer" => Some("INTEGER"),
        "number" => Some("NUMBER"),
        "boolean" => Some("BOOLEAN"),
        "array" => Some("ARRAY"),
        "object" => Some("OBJECT"),
        _ => None,
    }
}

fn resolve_local_refs(
    value: &mut serde_json::Value,
    definitions: Option<&serde_json::Map<String, serde_json::Value>>,
) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(reference) = map.get("$ref").and_then(|value| value.as_str()) {
                if let Some(def_name) = reference.strip_prefix("#/definitions/") {
                    if let Some(definitions) = definitions {
                        if let Some(resolved) = definitions.get(def_name) {
                            let mut cloned = resolved.clone();
                            resolve_local_refs(&mut cloned, Some(definitions));
                            *value = cloned;
                            return;
                        }
                    }
                }
            }

            for nested in map.values_mut() {
                resolve_local_refs(nested, definitions);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                resolve_local_refs(item, definitions);
            }
        }
        _ => {}
    }
}

/// Recursively clean JSON Schema to match Gemini's strict OpenAPI 3.0 subset requirements.
fn clean_schema_for_gemini(value: &mut serde_json::Value) {
    if let serde_json::Value::Object(map) = value {
        // Remove unsupported JSON Schema features
        map.remove("$schema");
        map.remove("definitions");
        map.remove("$ref");
        map.remove("default");
        map.remove("additionalProperties");
        map.remove("oneOf");
        map.remove("anyOf");
        map.remove("allOf");
        map.remove("not");
        map.remove("nullable");

        // Ensure type is a single uppercase string (Gemini's protobuf requirement)
        if let Some(type_val) = map.get_mut("type") {
            if let serde_json::Value::Array(arr) = type_val {
                // If it's something like ["string", "null"], use the first supported type.
                if let Some(supported) = arr.iter().find_map(|entry| match entry {
                    serde_json::Value::String(s) => normalize_gemini_type(s),
                    _ => None,
                }) {
                    *type_val = serde_json::Value::String(supported.to_string());
                }
            }
            if let serde_json::Value::String(s) = type_val {
                if let Some(normalized) = normalize_gemini_type(s) {
                    *type_val = serde_json::Value::String(normalized.to_string());
                } else {
                    map.remove("type");
                }
            }
        }

        // Recursively clean nested properties and items
        if let Some(properties) = map.get_mut("properties") {
            if let serde_json::Value::Object(props) = properties {
                for (_, prop_schema) in props.iter_mut() {
                    clean_schema_for_gemini(prop_schema);
                }
            }
        }
        if let Some(items) = map.get_mut("items") {
            clean_schema_for_gemini(items);
        }

        // Keep `required` entries aligned with declared properties.
        let property_names: Vec<String> = map
            .get("properties")
            .and_then(|v| v.as_object())
            .map(|props| props.keys().cloned().collect())
            .unwrap_or_default();
        if let Some(required) = map.get_mut("required") {
            if let serde_json::Value::Array(entries) = required {
                entries.retain(|entry| {
                    entry
                        .as_str()
                        .map(|name| property_names.iter().any(|p| p == name))
                        .unwrap_or(false)
                });
            }
        }

        if map.contains_key("properties") && !map.contains_key("type") {
            map.insert(
                "type".to_string(),
                serde_json::Value::String("OBJECT".to_string()),
            );
        } else if map.contains_key("items") && !map.contains_key("type") {
            map.insert(
                "type".to_string(),
                serde_json::Value::String("ARRAY".to_string()),
            );
        }
    }
}

/// Convert our internal Tool list to Gemini's functionDeclarations format.
fn build_gemini_tools(
    tools: &[crate::ai::provider_types::Tool],
) -> ProviderResult<Option<Vec<GeminiTool>>> {
    if tools.is_empty() {
        return Ok(None);
    }
    let mut declarations = Vec::with_capacity(tools.len());

    for tool in tools {
        let mut parameters = tool.function.parameters.clone();
        let definitions = parameters
            .as_object()
            .and_then(|root| root.get("definitions"))
            .and_then(|value| value.as_object())
            .cloned();
        resolve_local_refs(&mut parameters, definitions.as_ref());
        clean_schema_for_gemini(&mut parameters);
        let Some(root) = parameters.as_object_mut() else {
            return Err(AIError::InvalidRequest(format!(
                "Gemini schema conversion failed for '{}': root parameters must be an object",
                tool.function.name
            )));
        };

        root.insert(
            "type".to_string(),
            serde_json::Value::String("OBJECT".to_string()),
        );
        if !root.contains_key("properties") {
            root.insert("properties".to_string(), serde_json::json!({}));
        }

        declarations.push(GeminiFunctionDeclaration {
            name: tool.function.name.clone(),
            description: tool.function.description.clone(),
            parameters,
        });
    }

    Ok(Some(vec![GeminiTool {
        function_declarations: declarations,
    }]))
}

// ─── Adapter struct ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{build_gemini_tools, clean_schema_for_gemini, resolve_local_refs};
    use crate::ai::provider_types::{FunctionDefinition, Tool};

    #[test]
    fn clean_schema_removes_definitions_and_refs_after_resolution() {
        let mut schema = serde_json::json!({
            "type": "object",
            "definitions": {
                "PathEntry": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                }
            },
            "properties": {
                "entries": {
                    "type": "array",
                    "items": { "$ref": "#/definitions/PathEntry" }
                }
            }
        });

        let definitions = schema
            .as_object()
            .and_then(|root| root.get("definitions"))
            .and_then(|value| value.as_object())
            .cloned();
        resolve_local_refs(&mut schema, definitions.as_ref());
        clean_schema_for_gemini(&mut schema);

        let schema_text = schema.to_string();
        assert!(!schema_text.contains("definitions"));
        assert!(!schema_text.contains("$ref"));
        assert!(schema_text.contains("\"ARRAY\""));
        assert!(schema_text.contains("\"OBJECT\""));
    }

    #[test]
    fn build_gemini_tools_accepts_ref_based_schema() {
        let tool = Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "test_tool".to_string(),
                description: "Test tool".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "definitions": {
                        "Item": {
                            "type": "object",
                            "properties": {
                                "value": { "type": "string" }
                            }
                        }
                    },
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": { "$ref": "#/definitions/Item" }
                        }
                    }
                }),
            },
        };

        let built = build_gemini_tools(&[tool]).expect("schema should sanitize");
        let json = serde_json::to_string(&built).expect("serialize");
        assert!(!json.contains("definitions"));
        assert!(!json.contains("$ref"));
    }
}

pub struct GeminiProviderAdapter {
    config: ProviderConfig,
    api_key: String,
    client: reqwest::Client,
}

impl GeminiProviderAdapter {
    pub fn new(config: ProviderConfig) -> ProviderResult<Self> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| AIError::Authentication("Gemini API key is required".to_string()))?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .map_err(|e| AIError::Configuration(format!("HTTP client error: {}", e)))?;

        Ok(Self {
            config,
            api_key,
            client,
        })
    }

    fn model_url(&self, model: &str, endpoint: &str) -> String {
        let api_model = resolve_model_id(model);
        format!(
            "{}/models/{}:{}?key={}",
            GEMINI_API_BASE, api_model, endpoint, self.api_key
        )
    }
}

#[async_trait]
impl AIProvider for GeminiProviderAdapter {
    fn id(&self) -> &ProviderId {
        &self.config.id
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Google
    }

    async fn capabilities(&self) -> ProviderResult<ProviderCapabilities> {
        Ok(ProviderCapabilities {
            chat_completions: true,
            embeddings: false,
            streaming: true,
            function_calling: true,
            vision: false,
            web_search: false,
            max_context_tokens: 1_000_000,
            max_output_tokens: 8192,
            models: vec![
                "gemini-3-flash-preview".to_string(),
                "gemini-3.1-flash-lite-preview".to_string(),
            ],
        })
    }

    async fn health_check(&self) -> ProviderResult<ProviderHealth> {
        let url = format!("{}/models?key={}", GEMINI_API_BASE, self.api_key);
        match self.client.get(&url).send().await {
            Ok(r) if r.status().is_success() => Ok(ProviderHealth::Healthy),
            Ok(r) if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                Ok(ProviderHealth::Degraded)
            }
            _ => Ok(ProviderHealth::Unhealthy),
        }
    }

    async fn complete(
        &self,
        request: ChatCompletionRequest,
    ) -> ProviderResult<ChatCompletionResponse> {
        let thinking_level = self
            .config
            .params
            .get("thinkingLevel")
            .or_else(|| self.config.params.get("thinking_level"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| request.reasoning_effort.clone())
            .or_else(|| extract_thinking_level(&request.model));
        let tool_config = build_tool_config(request.tools.as_deref(), request.tool_choice.as_ref());

        let (system_instruction, contents, generation_config) = build_gemini_request_parts(
            &request.messages,
            request.temperature,
            request.max_tokens,
            thinking_level,
        );

        let gemini_tools = match request.tools.as_deref() {
            Some(tools) => build_gemini_tools(tools)?,
            None => None,
        };

        let body = GeminiRequest {
            contents,
            system_instruction,
            generation_config,
            tools: gemini_tools,
            tool_config,
        };

        let url = self.model_url(&request.model, "generateContent");

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AIError::NetworkError(format!("Gemini request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AIError::APIError(format!(
                "Gemini API error {}: {}",
                status, error_text
            )));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| AIError::APIError(format!("Failed to parse Gemini response: {}", e)))?;

        let candidate = gemini_response
            .candidates
            .into_iter()
            .next()
            .ok_or_else(|| AIError::APIError("No candidates in Gemini response".to_string()))?;

        let finish_reason = candidate
            .finish_reason
            .clone()
            .unwrap_or_else(|| "stop".to_string());
        let raw_parts_debug = serde_json::to_string(&candidate.content.parts).ok();

        // Separate text parts from functionCall parts.
        let mut text_parts: Vec<String> = Vec::new();
        let mut tool_calls: Vec<crate::ai::provider_types::ToolCall> = Vec::new();

        for part in candidate.content.parts {
            match part {
                GeminiPart::Text { text } => text_parts.push(text),
                GeminiPart::FunctionCall {
                    function_call,
                    thought_signature,
                    thought_signature_camel,
                } => {
                    let signature = thought_signature.or(thought_signature_camel);
                    tool_calls.push(crate::ai::provider_types::ToolCall {
                        id: uuid::Uuid::new_v4().to_string(),
                        r#type: "function".to_string(),
                        extra_content: signature.map(|sig| {
                            serde_json::json!({
                                "google": {
                                    "thought_signature": sig
                                }
                            })
                        }),
                        function: crate::ai::provider_types::FunctionCall {
                            name: function_call.name,
                            arguments: function_call.args.to_string(),
                        },
                        airlock_level: None,
                    });
                }
                GeminiPart::FunctionResponse { .. } => {}
                GeminiPart::Unknown(_) => {}
            }
        }

        let text = text_parts.join("");
        if text.trim().is_empty() && tool_calls.is_empty() {
            tracing::warn!(
                "[Gemini BYOK] Empty assistant response after completion. finish_reason={}, model={}, parts={}",
                finish_reason,
                request.model,
                raw_parts_debug.unwrap_or_else(|| "<unavailable>".to_string())
            );
        }

        let (prompt_tokens, completion_tokens, total_tokens) =
            if let Some(usage) = gemini_response.usage_metadata {
                (
                    usage.prompt_token_count.unwrap_or(0),
                    usage.candidates_token_count.unwrap_or(0),
                    usage.total_token_count.unwrap_or(0),
                )
            } else {
                (0, 0, 0)
            };

        Ok(ChatCompletionResponse {
            content: if text.is_empty() { None } else { Some(text) },
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            model: request.model,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
            },
            finish_reason,
            provider_metadata: None,
        })
    }

    async fn complete_stream(
        &self,
        request: ChatCompletionRequest,
        callback: StreamingCallback,
    ) -> ProviderResult<()> {
        let thinking_level = self
            .config
            .params
            .get("thinkingLevel")
            .or_else(|| self.config.params.get("thinking_level"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| request.reasoning_effort.clone())
            .or_else(|| extract_thinking_level(&request.model));
        let tool_config = build_tool_config(request.tools.as_deref(), request.tool_choice.as_ref());

        let (system_instruction, contents, generation_config) = build_gemini_request_parts(
            &request.messages,
            request.temperature,
            request.max_tokens,
            thinking_level,
        );

        let gemini_tools = match request.tools.as_deref() {
            Some(tools) => build_gemini_tools(tools)?,
            None => None,
        };

        let body = GeminiRequest {
            contents,
            system_instruction,
            generation_config,
            tools: gemini_tools,
            tool_config,
        };

        let url = self.model_url(&request.model, "streamGenerateContent?alt=sse");

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AIError::NetworkError(format!("Gemini stream request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AIError::APIError(format!(
                "Gemini stream error {}: {}",
                status, text
            )));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk =
                chunk_result.map_err(|e| AIError::NetworkError(format!("Stream error: {}", e)))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer.drain(..=newline_pos).collect::<String>();
                let line = line.trim();

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        callback(StreamingChunk {
                            content: String::new(),
                            thought: None,
                            is_final: true,
                            finish_reason: Some("stop".to_string()),
                        });
                        return Ok(());
                    }

                    if let Ok(parsed) = serde_json::from_str::<GeminiStreamChunk>(data) {
                        if let Some(candidates) = parsed.candidates {
                            for candidate in candidates {
                                // Only stream text parts — skip functionCall parts
                                // (function calls are resolved via the finalize complete() call).
                                let text = candidate
                                    .content
                                    .parts
                                    .iter()
                                    .filter_map(|p| p.as_text())
                                    .collect::<Vec<_>>()
                                    .join("");

                                if !text.is_empty() {
                                    let is_final = candidate
                                        .finish_reason
                                        .as_deref()
                                        .map(|r| r != "STOP" && r != "MAX_TOKENS")
                                        .unwrap_or(false);
                                    callback(StreamingChunk {
                                        content: text,
                                        thought: None,
                                        is_final,
                                        finish_reason: candidate.finish_reason,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn embed(&self, _request: EmbeddingRequest) -> ProviderResult<EmbeddingResponse> {
        Err(AIError::InvalidRequest(
            "Gemini BYOK provider does not support embeddings".to_string(),
        ))
    }

    fn default_model(&self) -> &str {
        &self.config.model
    }

    async fn available_models(&self) -> ProviderResult<Vec<String>> {
        Ok(vec![
            "gemini-3-flash-preview".to_string(),
            "gemini-3.1-flash-lite-preview".to_string(),
        ])
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }
}

pub struct GeminiProviderFactory;

#[async_trait]
impl AIProviderFactory for GeminiProviderFactory {
    async fn create(config: ProviderConfig) -> ProviderResult<Arc<dyn AIProvider>> {
        Self::validate_config(&config)?;
        Ok(Arc::new(GeminiProviderAdapter::new(config)?))
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
