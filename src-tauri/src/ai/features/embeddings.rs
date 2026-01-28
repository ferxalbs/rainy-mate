// Rainy Cowork - Embeddings Feature (PHASE 3)
// Embedding generation using rainy-sdk

use crate::ai::provider_types::{AIError, ProviderResult, EmbeddingRequest, EmbeddingResponse, TokenUsage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Embedding service
pub struct EmbeddingService {
    /// Rainy client
    client: Arc<rainy_sdk::RainyClient>,
}

impl EmbeddingService {
    /// Create a new embedding service
    pub fn new(client: Arc<rainy_sdk::RainyClient>) -> Self {
        Self { client }
    }

    /// Generate embeddings for a single text
    pub async fn embed(&self, request: EmbeddingRequest) -> ProviderResult<EmbeddingResponse> {
        // Note: rainy-sdk doesn't currently support embeddings
        // This is a placeholder for future implementation
        Err(AIError::UnsupportedCapability(
            "Embeddings not yet supported in rainy-sdk. This feature will be available in a future version.".to_string()
        ))
    }

    /// Generate embeddings for multiple texts (batch)
    pub async fn embed_batch(&self, requests: Vec<EmbeddingRequest>) -> ProviderResult<Vec<EmbeddingResponse>> {
        // Note: rainy-sdk doesn't currently support embeddings
        // This is a placeholder for future implementation
        Err(AIError::UnsupportedCapability(
            "Embeddings not yet supported in rainy-sdk. This feature will be available in a future version.".to_string()
        ))
    }

    /// Calculate cosine similarity between two embeddings
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return 0.0;
        }

        (dot_product / (magnitude_a * magnitude_b)) as f64
    }

    /// Calculate Euclidean distance between two embeddings
    pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() {
            return f64::MAX;
        }

        a.iter().zip(b.iter())
            .map(|(x, y)| (x - y).powi(2) as f64)
            .sum::<f64>()
            .sqrt()
    }
}

/// Embedding batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingBatchRequest {
    pub inputs: Vec<String>,
    pub model: String,
}

/// Embedding batch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingBatchResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub model: String,
    pub usage: TokenUsage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];

        let similarity = EmbeddingService::cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];

        let similarity = EmbeddingService::cosine_similarity(&a, &b);
        assert!((similarity - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![3.0, 4.0, 0.0];

        let distance = EmbeddingService::euclidean_distance(&a, &b);
        assert!((distance - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_euclidean_distance_same() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];

        let distance = EmbeddingService::euclidean_distance(&a, &b);
        assert!((distance - 0.0).abs() < 0.001);
    }
}
