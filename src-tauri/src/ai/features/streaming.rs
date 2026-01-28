// Rainy Cowork - Streaming Feature (PHASE 3)
// Streaming chat completions using rainy-sdk

use crate::ai::provider_types::{AIError, ProviderResult, StreamingChunk, StreamingCallback};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Streaming service
pub struct StreamingService {
    /// Rainy client
    client: Arc<rainy_sdk::RainyClient>,
}

impl StreamingService {
    /// Create a new streaming service
    pub fn new(client: Arc<rainy_sdk::RainyClient>) -> Self {
        Self { client }
    }

    /// Stream chat completion
    pub async fn stream_chat(
        &self,
        model: &str,
        messages: &[crate::ai::ChatMessage],
        callback: StreamingCallback,
    ) -> ProviderResult<()> {
        // Note: rainy-sdk doesn't currently support streaming
        // This is a placeholder for future implementation
        Err(AIError::UnsupportedCapability(
            "Streaming not yet supported in rainy-sdk. This feature will be available in a future version.".to_string()
        ))
    }

    /// Stream chat completion with options
    pub async fn stream_chat_with_options(
        &self,
        model: &str,
        messages: &[crate::ai::ChatMessage],
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        callback: StreamingCallback,
    ) -> ProviderResult<()> {
        // Note: rainy-sdk doesn't currently support streaming
        // This is a placeholder for future implementation
        Err(AIError::UnsupportedCapability(
            "Streaming not yet supported in rainy-sdk. This feature will be available in a future version.".to_string()
        ))
    }

    /// Convert streaming chunks to complete text
    pub fn chunks_to_text(chunks: &[StreamingChunk]) -> String {
        chunks.iter()
            .filter(|c| !c.is_final)
            .map(|c| c.content.clone())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get final chunk from streaming response
    pub fn get_final_chunk(chunks: &[StreamingChunk]) -> Option<&StreamingChunk> {
        chunks.iter().find(|c| c.is_final)
    }
}

/// Streaming request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingRequest {
    pub model: String,
    pub messages: Vec<crate::ai::ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
}

/// Streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResponse {
    pub chunks: Vec<StreamingChunk>,
    pub complete_text: String,
    pub finish_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunks_to_text() {
        let chunks = vec![
            StreamingChunk {
                content: "Hello".to_string(),
                is_final: false,
                finish_reason: None,
            },
            StreamingChunk {
                content: " world".to_string(),
                is_final: false,
                finish_reason: None,
            },
            StreamingChunk {
                content: "!".to_string(),
                is_final: true,
                finish_reason: Some("stop".to_string()),
            },
        ];

        let text = StreamingService::chunks_to_text(&chunks);
        assert_eq!(text, "Hello world!");
    }

    #[test]
    fn test_get_final_chunk() {
        let chunks = vec![
            StreamingChunk {
                content: "Hello".to_string(),
                is_final: false,
                finish_reason: None,
            },
            StreamingChunk {
                content: " world".to_string(),
                is_final: true,
                finish_reason: Some("stop".to_string()),
            },
        ];

        let final_chunk = StreamingService::get_final_chunk(&chunks);
        assert!(final_chunk.is_some());
        assert_eq!(final_chunk.unwrap().content, " world");
    }

    #[test]
    fn test_get_final_chunk_none() {
        let chunks = vec![
            StreamingChunk {
                content: "Hello".to_string(),
                is_final: false,
                finish_reason: None,
            },
            StreamingChunk {
                content: " world".to_string(),
                is_final: false,
                finish_reason: None,
            },
        ];

        let final_chunk = StreamingService::get_final_chunk(&chunks);
        assert!(final_chunk.is_none());
    }
}
