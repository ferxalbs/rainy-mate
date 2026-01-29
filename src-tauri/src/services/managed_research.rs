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
                if let Some(c) = map.get("content").and_then(|v| v.as_str()) {
                    c.to_string()
                } else if let Some(o) = map.get("output").and_then(|v| v.as_str()) {
                    o.to_string()
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
