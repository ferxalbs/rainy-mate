// Rainy Cowork - Google Gemini Provider (GenAI SDK)
// Updated for Gemini 3 models with thinking level support

use crate::ai::provider::AIError;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Gemini API base URL (v1beta for latest features)
const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Thinking levels for Gemini 3 models
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
}

impl Default for ThinkingLevel {
    fn default() -> Self {
        ThinkingLevel::High
    }
}

/// Gemini provider - for users with their own Google API key (BYOK)
///
/// **Free Tier Models (Gemini BYOK)**:
/// - `gemini-3-flash-minimal` - Fast responses with minimal thinking
/// - `gemini-3-flash-high` - Deep reasoning for complex tasks
/// - `gemini-2.5-flash-lite` - Lightweight, fast responses
///
/// All other models require Rainy API subscription.
pub struct GeminiProvider {
    client: Client,
}

impl GeminiProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Get available Gemini model IDs for free tier (BYOK)
    ///
    /// Only 3 models are available for free users with their own Gemini API key:
    /// - Gemini 3 Flash with Minimal thinking (fast)
    /// - Gemini 3 Flash with High thinking (accurate)
    /// - Gemini 2.5 Flash Lite (lightweight)
    ///
    /// All other models (GPT-4o, GPT-5, Claude, Gemini Pro, etc.) require Rainy API.
    pub fn available_models(&self) -> Vec<String> {
        vec![
            "gemini-3-flash-minimal".to_string(),
            "gemini-3-flash-high".to_string(),
            "gemini-2.5-flash-lite".to_string(),
        ]
    }

    /// Map user-friendly model names to actual API model IDs
    fn get_api_model_id(&self, model: &str) -> &'static str {
        match model {
            "gemini-3-flash-minimal" => "gemini-3-flash-preview",
            "gemini-3-flash-high" => "gemini-3-flash-preview",
            "gemini-2.5-flash-lite" => "gemini-2.5-flash-lite",
            // Unknown models - use high-quality default
            _ => "gemini-3-flash-preview",
        }
    }

    /// Get thinking configuration based on model variant
    fn get_thinking_config(&self, model: &str) -> Option<ThinkingConfig> {
        match model {
            "gemini-3-flash-minimal" => Some(ThinkingConfig {
                thinking_level: Some(ThinkingLevel::Minimal),
                thinking_budget: None,
            }),
            "gemini-3-flash-high" => Some(ThinkingConfig {
                thinking_level: Some(ThinkingLevel::High),
                thinking_budget: None,
            }),
            "gemini-2.5-flash-lite" => Some(ThinkingConfig {
                thinking_level: None,
                thinking_budget: Some(0), // Disable thinking for lite
            }),
            _ => None,
        }
    }

    /// Validate API key against Gemini API
    pub async fn validate_key(&self, api_key: &str) -> Result<bool, AIError> {
        let response = self
            .client
            .get(format!("{}/models?key={}", GEMINI_API_BASE_URL, api_key))
            .send()
            .await
            .map_err(|e| AIError::RequestFailed(e.to_string()))?;

        Ok(response.status().is_success())
    }

    /// Complete a prompt with API key and progress callback
    pub async fn complete_with_api_key<F>(
        &self,
        model: &str,
        prompt: &str,
        api_key: &str,
        on_progress: F,
    ) -> Result<String, AIError>
    where
        F: Fn(u8, Option<String>) + Send + Sync + 'static,
    {
        on_progress(10, Some("Preparing Gemini request...".to_string()));

        // Get actual API model ID and thinking config for the variant
        let api_model_id = self.get_api_model_id(model);
        let thinking_config = self.get_thinking_config(model);

        let generation_config = thinking_config.map(|tc| GenerationConfig {
            thinking_config: Some(tc),
        });

        let request_body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
            generation_config,
        };

        on_progress(30, Some(format!("Sending to {}...", model)));

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            GEMINI_API_BASE_URL, api_model_id, api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AIError::RequestFailed(e.to_string()))?;

        if response.status() == 401 || response.status() == 403 {
            return Err(AIError::InvalidApiKey);
        }

        if response.status() == 429 {
            return Err(AIError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AIError::RequestFailed(error_text));
        }

        on_progress(70, Some("Processing Gemini response...".to_string()));

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| AIError::RequestFailed(format!("Failed to parse response: {}", e)))?;

        on_progress(90, Some("Extracting content...".to_string()));

        // Extract text content from response
        let content = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        on_progress(100, Some("Complete".to_string()));

        Ok(content)
    }
}

impl Default for GeminiProvider {
    fn default() -> Self {
        Self::new()
    }
}

// GenAI SDK request/response structures

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_config: Option<ThinkingConfig>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_level: Option<ThinkingLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_budget: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}
