// Agent Trait Module
//
// This module defines the core Agent trait that all specialized agents must implement.
// The trait provides a common interface for agent operations and ensures consistency
// across the multi-agent system.
//
// ## Agent Trait
//
// The Agent trait defines the following methods:
// - info(): Get agent metadata
// - process_task(): Execute a task and return the result
// - handle_message(): Process incoming messages from other agents
// - capabilities(): List the agent's capabilities
// - can_handle(): Check if the agent can handle a specific task
// - initialize(): Set up the agent with configuration
// - shutdown(): Clean up agent resources
//
// ## AgentError
//
// Comprehensive error types for agent operations:
// - TaskExecutionFailed: Task processing errors
// - MessageHandlingFailed: Message processing errors
// - NotInitialized: Agent used before initialization
// - AgentBusy: Agent is currently busy
// - InvalidConfig: Configuration errors
// - Io: I/O operation errors
// - Serialization: JSON serialization errors

use crate::agents::types::{AgentInfo, AgentMessage, AgentStatus, Task, TaskResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique identifier for the agent
    pub agent_id: String,
    /// ID of the workspace this agent belongs to
    pub workspace_id: String,
    /// AI provider to use (e.g., "gemini", "openai")
    pub ai_provider: String,
    /// Model to use for AI operations
    pub model: String,
    /// Additional agent-specific settings
    pub settings: serde_json::Value,
}

/// Errors that can occur during agent operations
#[derive(Debug, Error)]
pub enum AgentError {
    /// Task execution failed
    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),

    /// Message handling failed
    #[error("Message handling failed: {0}")]
    MessageHandlingFailed(String),

    /// Agent not initialized
    #[error("Agent not initialized")]
    NotInitialized,

    /// Agent is busy with another task
    #[error("Agent busy: {0}")]
    AgentBusy(String),

    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// I/O operation error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// AI provider error
    #[error("AI provider error: {0}")]
    AIProvider(String),

    /// Memory operation error
    #[error("Memory operation error: {0}")]
    Memory(String),

    /// Approval required but not granted
    #[error("Approval denied: {0}")]
    ApprovalDenied(String),
}

/// Core trait that all agents must implement
///
/// This trait defines the interface for agent operations in the multi-agent system.
/// All specialized agents (Director, Researcher, Executor, etc.) must implement this trait.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get agent information
    ///
    /// Returns metadata about the agent including ID, name, type, and status.
    fn info(&self) -> AgentInfo;

    /// Process a task and return result
    ///
    /// Executes the given task and returns the result. This is the main method
    /// through which agents perform work.
    ///
    /// # Arguments
    ///
    /// * `task` - The task to process
    ///
    /// # Returns
    ///
    /// A Result containing the TaskResult on success, or an AgentError on failure.
    async fn process_task(&self, task: Task) -> Result<TaskResult, AgentError>;

    /// Handle incoming messages from other agents
    ///
    /// Processes messages sent by other agents through the message bus.
    /// This enables inter-agent communication and coordination.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to handle
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure of message handling.
    async fn handle_message(&self, message: AgentMessage) -> Result<(), AgentError>;

    /// Get agent capabilities
    ///
    /// Returns a list of strings describing what this agent can do.
    /// This is used by the Director to determine which agent should handle
    /// a given task.
    ///
    /// # Returns
    ///
    /// A vector of capability descriptions.
    fn capabilities(&self) -> Vec<String>;

    /// Check if agent can handle a specific task type
    ///
    /// Determines whether this agent is suitable for processing the given task.
    /// The Director uses this to route tasks to appropriate agents.
    ///
    /// # Arguments
    ///
    /// * `task` - The task to evaluate
    ///
    /// # Returns
    ///
    /// true if the agent can handle the task, false otherwise.
    fn can_handle(&self, task: &Task) -> bool;

    /// Initialize agent with configuration
    ///
    /// Sets up the agent with the provided configuration. This must be called
    /// before any other agent methods.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the agent
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure of initialization.
    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError>;

    /// Cleanup agent resources
    ///
    /// Performs cleanup operations when the agent is being shut down.
    /// This should release any held resources and save state if necessary.
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure of shutdown.
    async fn shutdown(&mut self) -> Result<(), AgentError>;

    /// Update the agent's status
    ///
    /// # Arguments
    ///
    /// * `status` - New status to set
    async fn update_status(&self, status: AgentStatus);

    /// Set the current task being processed
    ///
    /// # Arguments
    ///
    /// * `task_id` - ID of the task, or None if no task
    async fn set_current_task(&self, task_id: Option<String>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_serialization() {
        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({"timeout": 30}),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AgentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.agent_id, deserialized.agent_id);
    }

    #[test]
    fn test_agent_error_display() {
        let error = AgentError::TaskExecutionFailed("Test error".to_string());
        assert_eq!(error.to_string(), "Task execution failed: Test error");

        let error = AgentError::NotInitialized;
        assert_eq!(error.to_string(), "Agent not initialized");
    }

    #[test]
    fn test_agent_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let agent_error: AgentError = io_error.into();
        assert!(matches!(agent_error, AgentError::Io(_)));
    }

    #[test]
    fn test_agent_error_from_serialization() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let agent_error: AgentError = json_error.into();
        assert!(matches!(agent_error, AgentError::Serialization(_)));
    }
}
