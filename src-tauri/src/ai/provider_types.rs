// Provider Types for AI Provider Abstraction Layer
// Defines shared types used across all provider implementations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Unique identifier for a provider
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(pub String);

impl ProviderId {
    /// Create a new provider ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the provider ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Provider type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderType {
    /// OpenAI provider
    OpenAI,
    /// Anthropic provider
    Anthropic,
    /// Google provider
    Google,
    /// xAI provider
    XAI,
    /// Local provider (Ollama)
    Local,
    /// Custom provider
    Custom,
    /// Rainy SDK provider
    RainySDK,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Google => write!(f, "google"),
            ProviderType::XAI => write!(f, "xai"),
            ProviderType::Local => write!(f, "local"),
            ProviderType::Custom => write!(f, "custom"),
            ProviderType::RainySDK => write!(f, "rainy-sdk"),
        }
    }
}

/// Provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Whether the provider supports chat completions
    pub chat_completions: bool,
    /// Whether the provider supports embeddings
    pub embeddings: bool,
    /// Whether the provider supports streaming
    pub streaming: bool,
    /// Whether the provider supports function calling
    pub function_calling: bool,
    /// Whether the provider supports vision/image analysis
    pub vision: bool,
    /// Whether the provider supports web search
    pub web_search: bool,
    /// Maximum context window in tokens
    pub max_context_tokens: u32,
    /// Maximum output tokens
    pub max_output_tokens: u32,
    /// Supported models
    pub models: Vec<String>,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            chat_completions: true,
            embeddings: false,
            streaming: false,
            function_calling: false,
            vision: false,
            web_search: false,
            max_context_tokens: 4096,
            max_output_tokens: 2048,
            models: vec![],
        }
    }
}

/// Provider health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderHealth {
    /// Provider is healthy and operational
    Healthy,
    /// Provider is degraded (slow but working)
    Degraded,
    /// Provider is unhealthy (errors or timeouts)
    Unhealthy,
    /// Provider status is unknown
    Unknown,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider ID
    pub id: ProviderId,
    /// Provider type
    pub provider_type: ProviderType,
    /// API key (encrypted in storage)
    pub api_key: Option<String>,
    /// Base URL for API requests
    pub base_url: Option<String>,
    /// Model to use
    pub model: String,
    /// Additional parameters
    pub params: HashMap<String, serde_json::Value>,
    /// Whether the provider is enabled
    pub enabled: bool,
    /// Priority for routing (lower = higher priority)
    pub priority: u32,
    /// Maximum requests per minute
    pub rate_limit: Option<u32>,
    /// Timeout in seconds
    pub timeout: u64,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            id: ProviderId::new("default"),
            provider_type: ProviderType::Custom,
            api_key: None,
            base_url: None,
            model: "default".to_string(),
            params: HashMap::new(),
            enabled: true,
            priority: 100,
            rate_limit: None,
            timeout: 30,
        }
    }
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message role (system, user, assistant)
    pub role: String,
    /// Message content
    pub content: String,
    /// Optional name for the message
    pub name: Option<String>,
}

impl ChatMessage {
    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            name: None,
        }
    }

    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            name: None,
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            name: None,
        }
    }
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// Messages to send
    pub messages: Vec<ChatMessage>,
    /// Model to use
    pub model: String,
    /// Temperature (0.0 to 2.0)
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Top P (0.0 to 1.0)
    pub top_p: Option<f32>,
    /// Frequency penalty (-2.0 to 2.0)
    pub frequency_penalty: Option<f32>,
    /// Presence penalty (-2.0 to 2.0)
    pub presence_penalty: Option<f32>,
    /// Stop sequences
    pub stop: Option<Vec<String>>,
    /// Whether to stream the response
    pub stream: bool,
}

impl Default for ChatCompletionRequest {
    fn default() -> Self {
        Self {
            messages: vec![],
            model: "default".to_string(),
            temperature: Some(0.7),
            max_tokens: None,
            top_p: Some(1.0),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop: None,
            stream: false,
        }
    }
}

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// Generated content
    pub content: String,
    /// Model used
    pub model: String,
    /// Tokens used
    pub usage: TokenUsage,
    /// Finish reason
    pub finish_reason: String,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Prompt tokens
    pub prompt_tokens: u32,
    /// Completion tokens
    pub completion_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Create new token usage
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

/// Embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    /// Text to embed
    pub input: String,
    /// Model to use
    pub model: String,
}

/// Embedding response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Embedding vector
    pub embedding: Vec<f32>,
    /// Model used
    pub model: String,
    /// Tokens used
    pub usage: TokenUsage,
}

/// Streaming chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChunk {
    /// Content chunk
    pub content: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Finish reason (if final)
    pub finish_reason: Option<String>,
}

/// Provider error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIError {
    /// Authentication error
    Authentication(String),
    /// Rate limit exceeded
    RateLimit(String),
    /// Invalid request
    InvalidRequest(String),
    /// API error
    APIError(String),
    /// Network error
    NetworkError(String),
    /// Timeout
    Timeout(String),
    /// Provider not found
    ProviderNotFound(String),
    /// Model not found
    ModelNotFound(String),
    /// Unsupported capability
    UnsupportedCapability(String),
    /// Configuration error
    Configuration(String),
    /// Internal error
    Internal(String),
}

impl std::fmt::Display for AIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AIError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            AIError::RateLimit(msg) => write!(f, "Rate limit exceeded: {}", msg),
            AIError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            AIError::APIError(msg) => write!(f, "API error: {}", msg),
            AIError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            AIError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            AIError::ProviderNotFound(msg) => write!(f, "Provider not found: {}", msg),
            AIError::ModelNotFound(msg) => write!(f, "Model not found: {}", msg),
            AIError::UnsupportedCapability(msg) => write!(f, "Unsupported capability: {}", msg),
            AIError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            AIError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AIError {}

/// Provider result type
pub type ProviderResult<T> = Result<T, AIError>;

/// Streaming callback type
pub type StreamingCallback = Arc<dyn Fn(StreamingChunk) + Send + Sync>;
