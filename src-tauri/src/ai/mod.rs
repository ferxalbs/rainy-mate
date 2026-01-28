// Rainy Cowork - AI Module
// AI provider abstraction using rainy-sdk for premium features

pub mod gemini;
pub mod keychain;
pub mod provider;

// PHASE 3: AI Provider Integration
pub mod provider_types;
pub mod provider_trait;
pub mod provider_registry;
pub mod providers;
pub mod router;
pub mod features;

// Legacy exports (deprecated)
pub use provider::AIProviderManager;

// PHASE 3 exports
pub use provider_types::{
    ProviderId, ProviderType, ProviderCapabilities, ProviderHealth,
    ProviderConfig, ChatMessage, ChatCompletionRequest, ChatCompletionResponse,
    TokenUsage, EmbeddingRequest, EmbeddingResponse, StreamingChunk,
    AIError, ProviderResult, StreamingCallback,
};
pub use provider_trait::{AIProvider, AIProviderFactory, ProviderWithStats, ProviderStats};
pub use provider_registry::ProviderRegistry;
pub use providers::{
    RainySDKProvider, RainySDKProviderFactory,
    OpenAIProvider, OpenAIProviderFactory,
    AnthropicProvider, AnthropicProviderFactory,
    XAIProvider, XAIProviderFactory,
};
pub use router::{
    IntelligentRouter, LoadBalancer, CostOptimizer, CapabilityMatcher,
    FallbackChain, CircuitBreaker, CircuitState,
};
pub use features::{
    EmbeddingService, StreamingService, WebSearchService, UsageAnalytics,
    EmbeddingBatchRequest, EmbeddingBatchResponse,
    StreamingRequest, StreamingResponse,
    SearchResults, SearchResult,
    ProviderUsage, TotalUsage, UsageStatistics, UsageReport,
};
