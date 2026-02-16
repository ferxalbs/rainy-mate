use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

// @RESERVED for Generic LLM Service Layer
#[allow(dead_code)]
pub struct LLMClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ChatCompletionChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ChunkChoice {
    delta: Delta,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Delta {
    content: Option<String>,
    // Moonshot/Kimi "Thinking" might appear in content or specific field?
    // Standard OpenAI usually puts it in content for reasoning models unless specified.
    // For now, we assume standard content streaming.
}

impl LLMClient {
    pub fn new(api_key: String) -> Self {
        // Default to Moonshot API for Kimi
        Self {
            client: Client::new(),
            api_key,
            model: "moonshot-v1-128k".to_string(), // Kimi 128k model
            base_url: "https://api.moonshot.cn/v1".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    #[allow(dead_code)]
    pub async fn stream_completion(
        &self,
        messages: Vec<Message>,
        callback: Arc<Mutex<dyn FnMut(String) + Send + Sync>>,
    ) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.base_url);
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            temperature: Some(0.3), // Lower temp for reasoning
        };

        let mut stream = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .bytes_stream();

        let mut full_response = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    let s = String::from_utf8_lossy(&bytes);
                    if s.starts_with("data: [DONE]") {
                        break;
                    }
                    if s.starts_with("data: ") {
                        let data = s.trim_start_matches("data: ");
                        if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    full_response.push_str(content);
                                    let mut cb = callback.lock().await;
                                    (cb)(content.clone());
                                }
                            }
                        }
                    }
                }
                Err(e) => error!("Stream error: {}", e),
            }
        }

        Ok(full_response)
    }
}
