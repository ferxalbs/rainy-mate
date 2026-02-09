// Agents Module - Multi-Agent System for Rainy MaTE
//
// DEPRECATED: This module is being replaced by the Native Agent Runtime (src/ai/agent/runtime.rs)
// and AgentSpec V2 system. Do not add new features here.
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
pub mod analyst;
pub mod base_agent;
pub mod creator;
pub mod critic;
pub mod designer;
pub mod developer;
pub mod director_agent;
#[cfg(test)]
mod director_agent_tests;
pub mod executor;
pub mod governor;

pub mod registry;
pub mod researcher;
pub mod status_monitoring;
pub mod task_management;
pub mod types;

// Re-export commonly used types for convenience
pub use agent_trait::{Agent, AgentConfig, AgentError};
pub use base_agent::BaseAgent;
pub use registry::{AgentRegistry, RegistryStatistics};
pub use types::{
    AgentInfo, AgentMessage, AgentStatus, AgentType, MemoryEntry, Task, TaskContext, TaskPriority,
    TaskResult,
};

// PHASE 2 specialized agents - available but not re-exported to avoid dead code
// Use directly when needed: agents::analyst::AnalystAgent, etc.
