// Rainy Cowork - Web Research Feature (PHASE 3)
// Web search integration using rainy-sdk

use crate::ai::AIProviderManager;
use rainy_sdk::{RainyClient, ResearchConfig, ResearchResult};
use std::sync::Arc;

/// Managed research service that handles API keys internally
pub struct ManagedResearchService {
    /// Provider manager for API keys
    provider_manager: Arc<AIProviderManager>,
}

impl ManagedResearchService {
    /// Create a new managed research service
    pub fn new(provider_manager: Arc<AIProviderManager>) -> Self {
        Self { provider_manager }
    }

    /// Perform web research and return SDK result directly
    pub async fn perform_research(
        &self,
        topic: String,
        config: Option<ResearchConfig>,
    ) -> Result<ResearchResult, String> {
        // Retrieve API key
        let api_key = match self.provider_manager.get_api_key("rainy_api").await {
            Ok(Some(k)) => k,
            Ok(None) => {
                return Err(
                    "No API key configured. Please add your Rainy API key in settings.".to_string(),
                )
            }
            Err(e) => return Err(format!("Failed to retrieve API key: {}", e)),
        };

        let client = RainyClient::with_api_key(api_key).map_err(|e| e.to_string())?;

        let response = client
            .research(topic.clone(), config)
            .await
            .map_err(|e| e.to_string())?;

        if !response.success {
            let msg = response
                .message
                .unwrap_or_else(|| "Unknown research error".to_string());
            return Err(msg);
        }

        let result_value = response.result.unwrap_or(serde_json::Value::Null);

        let content = match result_value {
            serde_json::Value::String(s) => s,
            serde_json::Value::Object(map) => {
                // Check standard content fields
                if let Some(c) = map.get("content").and_then(|v| v.as_str()) {
                    c.to_string()
                } else if let Some(o) = map.get("output").and_then(|v| v.as_str()) {
                    o.to_string()
                } else if let Some(output_arr) = map.get("output").and_then(|v| v.as_array()) {
                    // Handle Inngest agent output array
                    // Check for last message content
                    if let Some(last_msg) = output_arr.last() {
                        if let Some(content) = last_msg.get("content") {
                            if let Some(s) = content.as_str() {
                                s.to_string()
                            } else {
                                content.to_string()
                            }
                        } else {
                            // Fallback to searching inside the array items for text
                            serde_json::Value::Array(output_arr.clone()).to_string()
                        }
                    } else {
                        "No content in output".to_string()
                    }
                } else if let Some(results) = map.get("_results") {
                    // Handle case where _results is at the top level (as seen in user screenshot)
                    if let Some(output) = results.get("output") {
                        if let Some(output_arr) = output.as_array() {
                            // Iterate backwards to find the last actual ASSISTANT message content
                            let last_text = output_arr.iter().rev().find_map(|msg| {
                                // 1. Check if role is 'assistant'
                                let is_assistant = msg
                                    .get("role")
                                    .and_then(|r| r.as_str())
                                    .map(|r| r == "assistant" || r == "model")
                                    .unwrap_or(false);

                                if !is_assistant {
                                    return None;
                                }

                                // 2. Check for text content
                                if let Some(content) = msg.get("content") {
                                    if let Some(s) = content.as_str() {
                                        if !s.trim().is_empty() {
                                            return Some(s.to_string());
                                        }
                                    }
                                }
                                None
                            });

                            last_text.unwrap_or_else(|| {
                                 if let Some(last) = output_arr.last() {
                                     let role = last.get("role").and_then(|r| r.as_str()).unwrap_or("unknown");
                                     if role == "tool_result" || role == "function" {
                                         "Research completed but no final summary was generated. The agent might have stopped early.".to_string()
                                     } else if role == "tool_call" || role == "assistant" {
                                         "Research agent is processing...".to_string()
                                     } else {
                                          format!("Status: {}", role)
                                     }
                                 } else {
                                     "No output content found.".to_string()
                                 }
                             })
                        } else {
                            output.to_string()
                        }
                    } else {
                        // Fallback if no output in _results
                        serde_json::Value::Object(map).to_string()
                    }
                } else if let Some(state) = map.get("state") {
                    // Deep nested Inngest state
                    // state -> _results -> output
                    if let Some(results) = state.get("_results") {
                        if let Some(output) = results.get("output") {
                            if let Some(output_arr) = output.as_array() {
                                // Iterate backwards to find the last actual ASISTANT message content
                                let last_text = output_arr.iter().rev().find_map(|msg| {
                                    // 1. Check if role is 'assistant' (to avoid tool_result, system, user)
                                    let is_assistant = msg
                                        .get("role")
                                        .and_then(|r| r.as_str())
                                        .map(|r| r == "assistant" || r == "model") // Handle both conventions
                                        .unwrap_or(false);

                                    if !is_assistant {
                                        return None;
                                    }

                                    // 2. Check for text content
                                    if let Some(content) = msg.get("content") {
                                        if let Some(s) = content.as_str() {
                                            if !s.trim().is_empty() {
                                                return Some(s.to_string());
                                            }
                                        }
                                    }
                                    None
                                });

                                last_text.unwrap_or_else(|| {
                                     // If we found no assistant text, check if the last message was a tool result (agent stopped early)
                                     if let Some(last) = output_arr.last() {
                                         let role = last.get("role").and_then(|r| r.as_str()).unwrap_or("unknown");
                                         if role == "tool_result" || role == "function" {
                                              // Provide a more user-friendly status if we are seeing raw tool results
                                             "Research completed but no final summary was generated. The agent might have hit a limit or stopped unexpectedly.".to_string()
                                         } else if role == "tool_call" || role == "assistant" { // assistant with no content = tool call usually
                                             "Research agent is processing...".to_string()
                                         } else {
                                             // Fallback for debugging, but try to be cleaner than raw JSON
                                              format!("Status: {}", role)
                                         }
                                     } else {
                                         "No output content found.".to_string()
                                     }
                                 })
                            } else {
                                output.to_string()
                            }
                        } else {
                            state.to_string()
                        }
                    } else {
                        state.to_string()
                    }
                } else {
                    serde_json::Value::Object(map).to_string()
                }
            }
            serde_json::Value::Null => String::new(),
            _ => result_value.to_string(),
        };
        let provider = response.provider.unwrap_or_else(|| "rainy".to_string());

        Ok(ResearchResult {
            topic: topic,
            content,
            sources: vec![], // Sources not available in current SDK sync response
            provider,
        })
    }
}
