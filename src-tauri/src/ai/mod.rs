// Rainy Cowork - AI Module
#![allow(unused_imports)]
// AI provider abstraction using rainy-sdk for premium features

pub mod gemini;
pub mod keychain;
pub mod provider;

// PHASE 3: AI Provider Integration
pub mod agent;
pub mod features;
pub mod provider_registry;
pub mod provider_trait;
pub mod provider_types;
pub mod providers;
pub mod router;

// PHASE 4: Unified Model System
pub mod mode_selector;
pub mod unified_model_registry;

// Legacy exports (deprecated)
pub use provider::AIProviderManager;

// PHASE 3 exports - only what's actively used
pub use provider_registry::ProviderRegistry;
pub use provider_trait::{AIProvider, AIProviderFactory};
pub use provider_types::{
    AIError, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, EmbeddingRequest,
    EmbeddingResponse, ProviderCapabilities, ProviderConfig, ProviderHealth, ProviderId,
    ProviderResult, ProviderType, StreamingCallback, StreamingChunk, TokenUsage,
};
pub use router::IntelligentRouter;

// PHASE 4 exports
pub use mode_selector::{ModeSelector, ProcessingMode, TaskComplexity, UseCase};
pub use unified_model_registry::{
    ModelCapabilities, ModelCapability, ModelContext, ProviderSource, UnifiedModel,
    UnifiedModelRegistry, UserModelPreferences,
};
