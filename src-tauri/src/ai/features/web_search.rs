// Rainy Cowork - Web Search Feature (PHASE 3)
// Web search integration using rainy-sdk

use crate::ai::provider_types::{AIError, ProviderResult};
use rainy_sdk::{SearchOptions, SearchResult as SdkSearchResult, SearchResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Web search service
pub struct WebSearchService {
    /// Rainy client
    client: Arc<rainy_sdk::RainyClient>,
}

impl WebSearchService {
    /// Create a new web search service
    pub fn new(client: Arc<rainy_sdk::RainyClient>) -> Self {
        Self { client }
    }

    /// Perform web search
    pub async fn search(&self, query: &str, options: Option<SearchOptions>) -> ProviderResult<SearchResults> {
        let response = self.client.web_search(query, options).await
            .map_err(|e| AIError::APIError(format!("Web search failed: {}", e)))?;

        Ok(SearchResults {
            query: query.to_string(),
            results: response.results.into_iter().map(|r| SearchResult {
                title: r.title,
                url: r.url,
                content: r.content,
                score: r.score,
            }).collect(),
            answer: response.answer,
        })
    }

    /// Get search results only (no AI answer)
    pub async fn search_results_only(&self, query: &str, max_results: u32) -> ProviderResult<Vec<SearchResult>> {
        let options = SearchOptions::advanced()
            .with_max_results(max_results);

        let response = self.client.web_search(query, Some(options)).await
            .map_err(|e| AIError::APIError(format!("Web search failed: {}", e)))?;

        Ok(response.results.into_iter().map(|r| SearchResult {
            title: r.title,
            url: r.url,
            content: r.content,
            score: r.score,
        }).collect())
    }
}

/// Search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub answer: Option<String>,
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub content: String,
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            title: "Test Title".to_string(),
            url: "https://example.com".to_string(),
            content: "Test content".to_string(),
            score: 0.95,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test Title"));
        assert!(json.contains("https://example.com"));
    }

    #[test]
    fn test_search_results_serialization() {
        let results = SearchResults {
            query: "test query".to_string(),
            results: vec![
                SearchResult {
                    title: "Result 1".to_string(),
                    url: "https://example1.com".to_string(),
                    content: "Content 1".to_string(),
                    score: 0.9,
                },
            ],
            answer: Some("AI answer".to_string()),
        };

        let json = serde_json::to_string(&results).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("AI answer"));
    }
}
