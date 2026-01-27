// Agents Module - Multi-Agent System for Rainy MaTE
//
// This module implements the foundational agent infrastructure for the multi-agent system.
// It provides the core traits, types, and base implementations that all specialized agents
// will build upon.
//
// ## Architecture
//
// The agent system consists of:
// - **Agent Trait**: The core interface that all agents must implement
// - **BaseAgent**: A common implementation providing shared functionality
// - **MessageBus**: Inter-agent communication infrastructure
// - **Agent Types**: Core data structures for agents, tasks, and messages
//
// ## Agent Types
//
// - Director: Orchestrates and coordinates all other agents
// - Researcher: Conducts research and gathers information
// - Executor: Executes tasks and operations
// - Creator: Creates content and artifacts
// - Designer: Designs UI/UX and visual elements
// - Developer: Writes and maintains code
// - Analyst: Analyzes data and provides insights
// - Critic: Evaluates quality and provides feedback
// - Governor: Enforces security and compliance policies
//
// ## Usage
//
// ```rust
// use rainy_cowork_lib::agents::{Agent, BaseAgent, AgentConfig, MessageBus};
//
// let message_bus = Arc::new(MessageBus::new());
// let config = AgentConfig {
//     agent_id: "director-1".to_string(),
//     workspace_id: "workspace-1".to_string(),
//     ai_provider: "gemini".to_string(),
//     model: "gemini-2.0-flash".to_string(),
//     settings: serde_json::json!({}),
// };
//
// let agent = BaseAgent::new(config, ai_provider, message_bus);
// agent.initialize(config.clone()).await?;
// ```

pub mod agent_trait;
pub mod base_agent;
pub mod message_bus;
pub mod registry;
pub mod status_monitoring;
pub mod task_management;
pub mod types;

// Re-export commonly used types for convenience
pub use agent_trait::{Agent, AgentConfig, AgentError};
pub use base_agent::BaseAgent;
pub use message_bus::MessageBus;
pub use registry::{AgentRegistry, RegistryStatistics};
pub use status_monitoring::StatusMonitor;
pub use task_management::TaskManager;
pub use types::{
    AgentInfo, AgentMessage, AgentStatus, AgentType, MemoryEntry, Task, TaskContext,
    TaskPriority, TaskResult,
};
