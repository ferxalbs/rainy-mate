// Rainy Cowork - AI Features Module (PHASE 3)
// Enhanced features for AI providers

pub mod embeddings;
pub mod streaming;
pub mod usage_analytics;
pub mod web_search;

pub use embeddings::{EmbeddingService, EmbeddingBatchRequest, EmbeddingBatchResponse};
pub use streaming::{StreamingService, StreamingRequest, StreamingResponse};
pub use usage_analytics::{UsageAnalytics, ProviderUsage, TotalUsage, UsageStatistics, UsageReport};
pub use web_search::{WebSearchService, SearchResults, SearchResult};
