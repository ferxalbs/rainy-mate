use crate::services::memory_vault::profiles::FALLBACK_EMBEDDING_PROFILE;
use reqwest::Client;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize)]
struct GeminiEmbeddingRequest {
    model: String,
    content: GeminiContent,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiEmbeddingResponse {
    embedding: GeminiEmbeddingData,
}

#[derive(Debug, Deserialize)]
struct GeminiEmbeddingData {
    values: Vec<f32>,
}

#[derive(Debug)]
pub struct EmbedderService {
    client: Client,
    provider: String,
    api_key: String,
    model: String,
}

impl EmbedderService {
    pub fn new(provider: String, api_key: String, model: Option<String>) -> Self {
        let normalized_provider = match provider.trim().to_lowercase().as_str() {
            "g" | "google" | "gemini" => "gemini".to_string(),
            // Step 3 HIVE MIND SEED production path is Gemini-only for memory embeddings.
            _ => "gemini".to_string(),
        };

        let default_model = crate::services::memory_vault::types::EMBEDDING_MODEL.to_string();

        let selected_model = model.unwrap_or(default_model);
        let normalized_model = match selected_model.trim() {
            "gemini-embedding-2-preview" | "gemini-embedding-001" => selected_model,
            "text-embedding-004"
            | "embedding-001"
            | "embedding-gecko-001"
            | "gemini-embedding-exp"
            | "gemini-embedding-exp-03-07" => "gemini-embedding-001".to_string(),
            _ => crate::services::memory_vault::types::EMBEDDING_MODEL.to_string(),
        };

        Self {
            client: Client::new(),
            provider: normalized_provider,
            api_key,
            model: normalized_model,
        }
    }

    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>, String> {
        if self.api_key.is_empty() {
            return Err(format!(
                "Missing embedding API key for provider: {}",
                self.provider
            ));
        }

        match self.embed_gemini(text, &self.model).await {
            Ok(v) => Ok(v),
            Err(primary_error) => {
                if self.model == FALLBACK_EMBEDDING_PROFILE.model {
                    return Err(primary_error);
                }

                self.embed_gemini(text, FALLBACK_EMBEDDING_PROFILE.model)
                    .await
                    .map_err(|fallback_error| {
                        format!(
                            "Embedding failed for '{}' and fallback '{}': {} | {}",
                            self.model, FALLBACK_EMBEDDING_PROFILE.model, primary_error, fallback_error
                        )
                    })
            }
        }
    }

    pub async fn embed_text_for_model(&self, text: &str, model: &str) -> Result<Vec<f32>, String> {
        if self.api_key.is_empty() {
            return Err(format!(
                "Missing embedding API key for provider: {}",
                self.provider
            ));
        }
        self.embed_gemini(text, model).await
    }

    pub fn provider_name(&self) -> &str {
        &self.provider
    }

    async fn embed_gemini(&self, text: &str, model: &str) -> Result<Vec<f32>, String> {
        let req_body = GeminiEmbeddingRequest {
            model: format!("models/{}", model),
            content: GeminiContent {
                parts: vec![GeminiPart {
                    text: text.to_string(),
                }],
            },
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
            model, self.api_key
        );

        let res = self
            .client
            .post(&url)
            .json(&req_body)
            .send()
            .await
            .map_err(|e| format!("Gemini embedding request failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let text_err = res.text().await.unwrap_or_default();
            return Err(format!(
                "Gemini embedding API error: {} - {}",
                status, text_err
            ));
        }

        let parsed: GeminiEmbeddingResponse = res
            .json()
            .await
            .map_err(|e| format!("Parsing Gemini embedding response failed: {}", e))?;

        Ok(parsed.embedding.values)
    }

}
