// Unified error types for the Agent subsystem.
// Provides structured error handling across memory, workflow, provider, and tool operations.

use std::fmt;

/// Unified error type for agent operations
#[derive(Debug)]
pub enum AgentError {
    /// Memory operations failed (SQLite, retrieval, persistence)
    Memory(String),
    /// Provider/LLM call failed
    Provider(String),
    /// Tool/skill execution failed
    Tool { tool_name: String, message: String },
    /// Workflow reached step limit or invalid transition
    Workflow(String),
    /// Timeout during an operation
    Timeout { operation: String, duration_ms: u64 },
    /// Airlock denied a tool invocation
    AirlockDenied { tool: String, level: String },
    /// Serialization/deserialization failure
    Serialization(String),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Memory(msg) => write!(f, "[Memory] {}", msg),
            Self::Provider(msg) => write!(f, "[Provider] {}", msg),
            Self::Tool { tool_name, message } => {
                write!(f, "[Tool:{}] {}", tool_name, message)
            }
            Self::Workflow(msg) => write!(f, "[Workflow] {}", msg),
            Self::Timeout {
                operation,
                duration_ms,
            } => write!(f, "[Timeout] {} after {}ms", operation, duration_ms),
            Self::AirlockDenied { tool, level } => {
                write!(f, "[Airlock] {} denied at level {}", tool, level)
            }
            Self::Serialization(msg) => write!(f, "[Serde] {}", msg),
        }
    }
}

impl std::error::Error for AgentError {}

impl From<sqlx::Error> for AgentError {
    fn from(e: sqlx::Error) -> Self {
        Self::Memory(e.to_string())
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

impl From<reqwest::Error> for AgentError {
    fn from(e: reqwest::Error) -> Self {
        Self::Provider(e.to_string())
    }
}

/// Convert AgentError to the String type used by existing workflow methods.
/// This allows gradual migration — new code can use `AgentError`, old code still gets `String`.
impl From<AgentError> for String {
    fn from(e: AgentError) -> Self {
        e.to_string()
    }
}

/// Classify whether an error is retryable (transient) or permanent.
impl AgentError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Provider(_) | Self::Timeout { .. } | Self::Memory(_)
        )
    }
}
