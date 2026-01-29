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
                } else if let Some(state) = map.get("state") {
                    // Deep nested Inngest state
                    // state -> _results -> output
                    if let Some(results) = state.get("_results") {
                        if let Some(output) = results.get("output") {
                            if let Some(output_arr) = output.as_array() {
                                // Try to find the last assistant message that is NOT just a tool call
                                // Or just dump the last message's content
                                if let Some(last) = output_arr.last() {
                                    if let Some(c) = last.get("content") {
                                        if let Some(s) = c.as_str() {
                                            s.to_string()
                                        } else {
                                            c.to_string()
                                        }
                                    } else {
                                        output.to_string()
                                    }
                                } else {
                                    output.to_string()
                                }
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
