use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
struct OpenAIEmbeddingRequest {
    input: String,
    model: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}
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
            "oai" | "openai" => "openai".to_string(),
            other => other.to_string(),
        };

        let default_model = if normalized_provider == "gemini" {
            "gemini-embedding-001".to_string()
        } else {
            "text-embedding-3-small".to_string()
        };

        let selected_model = model.unwrap_or(default_model);
        let normalized_model = if normalized_provider == "gemini" {
            match selected_model.as_str() {
                "text-embedding-004"
                | "embedding-001"
                | "embedding-gecko-001"
                | "gemini-embedding-exp"
                | "gemini-embedding-exp-03-07" => "gemini-embedding-001".to_string(),
                other => other.to_string(),
            }
        } else {
            selected_model
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

        if self.provider.to_lowercase() == "gemini" {
            self.embed_gemini(text).await
        } else {
            self.embed_openai(text).await
        }
    }

    async fn embed_gemini(&self, text: &str) -> Result<Vec<f32>, String> {
        let req_body = GeminiEmbeddingRequest {
            model: format!("models/{}", self.model),
            content: GeminiContent {
                parts: vec![GeminiPart {
                    text: text.to_string(),
                }],
            },
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
            self.model, self.api_key
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

    async fn embed_openai(&self, text: &str) -> Result<Vec<f32>, String> {
        let req_body = OpenAIEmbeddingRequest {
            input: text.to_string(),
            model: self.model.clone(),
        };

        let res = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&req_body)
            .send()
            .await
            .map_err(|e| format!("Embedding request failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let text_err = res.text().await.unwrap_or_default();
            return Err(format!("Embedding API error: {} - {}", status, text_err));
        }

        let mut parsed: OpenAIEmbeddingResponse = res
            .json()
            .await
            .map_err(|e| format!("Parsing embedding response failed: {}", e))?;

        parsed
            .data
            .pop()
            .map(|data| data.embedding)
            .ok_or_else(|| "No embedding returned".to_string())
    }
}
