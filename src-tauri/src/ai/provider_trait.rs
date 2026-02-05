// AI Provider Trait
// Defines the interface that all AI providers must implement

use crate::ai::provider_types::{
    ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    ProviderCapabilities, ProviderHealth, ProviderId, ProviderResult, ProviderType,
    StreamingCallback,
};
use async_trait::async_trait;
use std::sync::Arc;

/// AI Provider trait - all providers must implement this
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Get the provider ID
    fn id(&self) -> &ProviderId;

    /// Get the provider type
    fn provider_type(&self) -> ProviderType;

    /// Get the provider capabilities
    async fn capabilities(&self) -> ProviderResult<ProviderCapabilities>;

    /// Check provider health
    async fn health_check(&self) -> ProviderResult<ProviderHealth>;

    /// Complete a chat request
    async fn complete(
        &self,
        request: ChatCompletionRequest,
    ) -> ProviderResult<ChatCompletionResponse>;

    /// Complete a chat request with streaming
    async fn complete_stream(
        &self,
        request: ChatCompletionRequest,
        callback: StreamingCallback,
    ) -> ProviderResult<()>;

    /// Generate embeddings
    async fn embed(&self, request: EmbeddingRequest) -> ProviderResult<EmbeddingResponse>;

    /// Get the default model for this provider
    fn default_model(&self) -> &str;

    /// Get the list of available models
    async fn available_models(&self) -> ProviderResult<Vec<String>>;

    /// Get the provider configuration
    fn config(&self) -> &crate::ai::provider_types::ProviderConfig;
}

/// Helper trait for provider initialization
#[async_trait]
pub trait AIProviderFactory: Send + Sync {
    /// Create a new provider instance
    async fn create(
        config: crate::ai::provider_types::ProviderConfig,
    ) -> ProviderResult<Arc<dyn AIProvider>>;

    /// Validate the provider configuration
    fn validate_config(config: &crate::ai::provider_types::ProviderConfig) -> ProviderResult<()>;
}

/// Provider statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderStats {
    /// Total requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Last request timestamp
    pub last_request: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for ProviderStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_latency_ms: 0.0,
            total_tokens: 0,
            last_request: None,
        }
    }
}

/// Provider with statistics
#[derive(Clone)]
pub struct ProviderWithStats {
    /// The provider
    pub provider: Arc<dyn AIProvider>,
    /// Provider statistics
    pub stats: ProviderStats,
}

impl std::fmt::Debug for ProviderWithStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderWithStats")
            .field("provider_id", self.provider.id())
            .field("stats", &self.stats)
            .finish()
    }
}

impl ProviderWithStats {
    /// Create a new provider with stats
    pub fn new(provider: Arc<dyn AIProvider>) -> Self {
        Self {
            provider,
            stats: ProviderStats::default(),
        }
    }

    /// Update statistics after a request
    pub fn update_stats(&mut self, success: bool, latency_ms: u64, tokens: u64) {
        self.stats.total_requests += 1;
        if success {
            self.stats.successful_requests += 1;
        } else {
            self.stats.failed_requests += 1;
        }

        // Update average latency
        if self.stats.total_requests > 0 {
            let total_latency = self.stats.avg_latency_ms * (self.stats.total_requests - 1) as f64;
            self.stats.avg_latency_ms =
                (total_latency + latency_ms as f64) / self.stats.total_requests as f64;
        }

        self.stats.total_tokens += tokens;
        self.stats.last_request = Some(chrono::Utc::now());
    }

    /// Get the provider
    pub fn provider(&self) -> &dyn AIProvider {
        self.provider.as_ref()
    }

    /// Get the statistics
    pub fn stats(&self) -> &ProviderStats {
        &self.stats
    }
}
