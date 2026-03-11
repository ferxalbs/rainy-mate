use crate::services::memory_vault::profiles::FALLBACK_EMBEDDING_PROFILE;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum EmbeddingTaskType {
    RetrievalDocument,
    RetrievalQuery,
}

impl EmbeddingTaskType {
    fn as_api_value(self) -> &'static str {
        match self {
            Self::RetrievalDocument => "RETRIEVAL_DOCUMENT",
            Self::RetrievalQuery => "RETRIEVAL_QUERY",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiEmbeddingRequest {
    model: String,
    content: GeminiContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_dimensionality: Option<u32>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiBatchEmbeddingRequest {
    requests: Vec<GeminiEmbeddingRequest>,
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

    pub async fn embed_text_with_task(
        &self,
        text: &str,
        task_type: EmbeddingTaskType,
    ) -> Result<Vec<f32>, String> {
        if self.api_key.is_empty() {
            return Err(format!(
                "Missing embedding API key for provider: {}",
                self.provider
            ));
        }

        match self.embed_gemini(text, &self.model, task_type).await {
            Ok(v) => Ok(v),
            Err(primary_error) => {
                if self.model == FALLBACK_EMBEDDING_PROFILE.model {
                    return Err(primary_error);
                }

                self.embed_gemini(text, FALLBACK_EMBEDDING_PROFILE.model, task_type)
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

    pub async fn embed_text_for_model_with_task_strict(
        &self,
        text: &str,
        model: &str,
        task_type: EmbeddingTaskType,
    ) -> Result<Vec<f32>, String> {
        if self.api_key.is_empty() {
            return Err(format!(
                "Missing embedding API key for provider: {}",
                self.provider
            ));
        }
        self.embed_gemini(text, model, task_type).await
    }

    pub async fn embed_texts_for_model_with_task(
        &self,
        texts: &[String],
        model: &str,
        task_type: EmbeddingTaskType,
    ) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        if self.api_key.is_empty() {
            return Err(format!(
                "Missing embedding API key for provider: {}",
                self.provider
            ));
        }
        self.embed_gemini_batch(texts, model, task_type).await
    }

    async fn embed_gemini(
        &self,
        text: &str,
        model: &str,
        task_type: EmbeddingTaskType,
    ) -> Result<Vec<f32>, String> {
        let req_body = GeminiEmbeddingRequest {
            model: format!("models/{}", model),
            content: GeminiContent {
                parts: vec![GeminiPart {
                    text: text.to_string(),
                }],
            },
            task_type: Some(task_type.as_api_value().to_string()),
            output_dimensionality: Some(crate::services::memory_vault::types::EMBEDDING_DIM as u32),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent",
            model
        );

        let res = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
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

    async fn embed_gemini_batch(
        &self,
        texts: &[String],
        model: &str,
        task_type: EmbeddingTaskType,
    ) -> Result<Vec<Vec<f32>>, String> {
        let requests = texts
            .iter()
            .map(|text| GeminiEmbeddingRequest {
                model: format!("models/{}", model),
                content: GeminiContent {
                    parts: vec![GeminiPart {
                        text: text.to_string(),
                    }],
                },
                task_type: Some(task_type.as_api_value().to_string()),
                output_dimensionality: Some(crate::services::memory_vault::types::EMBEDDING_DIM as u32),
            })
            .collect::<Vec<_>>();

        let req_body = GeminiBatchEmbeddingRequest { requests };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:batchEmbedContents",
            model
        );

        let res = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&req_body)
            .send()
            .await
            .map_err(|e| format!("Gemini batch embedding request failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let text_err = res.text().await.unwrap_or_default();
            return Err(format!(
                "Gemini batch embedding API error: {} - {}",
                status, text_err
            ));
        }

        let value: Value = res
            .json()
            .await
            .map_err(|e| format!("Parsing Gemini batch embedding response failed: {}", e))?;

        let mut embeddings = Vec::new();
        if let Some(items) = value.get("embeddings").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(vals) = parse_embedding_values(item) {
                    embeddings.push(vals);
                }
            }
        } else if let Some(items) = value.get("responses").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(vals) = item.get("embedding").and_then(parse_embedding_values) {
                    embeddings.push(vals);
                }
            }
        }

        if embeddings.len() != texts.len() {
            return Err(format!(
                "Gemini batch embedding size mismatch: expected {}, got {}",
                texts.len(),
                embeddings.len()
            ));
        }

        Ok(embeddings)
    }
}

fn parse_embedding_values(value: &Value) -> Option<Vec<f32>> {
    let values = if let Some(v) = value.get("values") {
        v
    } else if let Some(v) = value.get("embedding").and_then(|emb| emb.get("values")) {
        v
    } else {
        return None;
    };

    let arr = values.as_array()?;
    let mut out = Vec::with_capacity(arr.len());
    for n in arr {
        out.push(n.as_f64()? as f32);
    }
    Some(out)
}
