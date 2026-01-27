// Agent Types Module
//
// This module defines the core data structures used throughout the agent system.
// All types are serializable with serde for easy persistence and communication.
//
// ## Types
//
// - AgentType: Enumeration of all agent types in the system
// - AgentStatus: Current status of an agent
// - AgentInfo: Metadata about an agent
// - AgentMessage: Messages sent between agents
// - Task: A unit of work assigned to an agent
// - TaskResult: Result of task execution
// - TaskContext: Contextual information for a task
// - MemoryEntry: A piece of information stored in agent memory
// - TaskPriority: Priority level for tasks

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Types of agents in the multi-agent system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    /// Orchestrates and coordinates all other agents
    Director,
    /// Conducts research and gathers information
    Researcher,
    /// Executes tasks and operations
    Executor,
    /// Creates content and artifacts
    Creator,
    /// Designs UI/UX and visual elements
    Designer,
    /// Writes and maintains code
    Developer,
    /// Analyzes data and provides insights
    Analyst,
    /// Evaluates quality and provides feedback
    Critic,
    /// Enforces security and compliance policies
    Governor,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Director => write!(f, "Director"),
            AgentType::Researcher => write!(f, "Researcher"),
            AgentType::Executor => write!(f, "Executor"),
            AgentType::Creator => write!(f, "Creator"),
            AgentType::Designer => write!(f, "Designer"),
            AgentType::Developer => write!(f, "Developer"),
            AgentType::Analyst => write!(f, "Analyst"),
            AgentType::Critic => write!(f, "Critic"),
            AgentType::Governor => write!(f, "Governor"),
        }
    }
}

/// Current status of an agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is idle and available for tasks
    Idle,
    /// Agent is currently processing a task
    Busy,
    /// Agent encountered an error
    Error(String),
}

/// Metadata about an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique identifier for the agent
    pub id: String,
    /// Human-readable name of the agent
    pub name: String,
    /// Type of the agent
    pub agent_type: AgentType,
    /// Current status of the agent
    pub status: AgentStatus,
    /// ID of the task currently being processed, if any
    pub current_task: Option<String>,
}

/// Priority level for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

/// A unit of work assigned to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for the task
    pub id: String,
    /// Description of what needs to be done
    pub description: String,
    /// Priority level of the task
    pub priority: TaskPriority,
    /// IDs of tasks that must complete before this one
    pub dependencies: Vec<String>,
    /// Contextual information for the task
    pub context: TaskContext,
}

/// Contextual information for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    /// ID of the workspace this task belongs to
    pub workspace_id: String,
    /// Original user instruction
    pub user_instruction: String,
    /// Files relevant to this task
    pub relevant_files: Vec<String>,
    /// Memory entries relevant to this task
    pub memory_context: Vec<MemoryEntry>,
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Whether the task completed successfully
    pub success: bool,
    /// Output produced by the task
    pub output: String,
    /// Any errors encountered during execution
    pub errors: Vec<String>,
    /// Additional metadata about the result
    pub metadata: serde_json::Value,
}

/// A piece of information stored in agent memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier for the memory entry
    pub id: String,
    /// Content of the memory entry
    pub content: String,
    /// Optional embedding vector for semantic search
    pub embedding: Option<Vec<f32>>,
    /// When this entry was created
    pub timestamp: DateTime<Utc>,
    /// Tags for categorization and retrieval
    pub tags: Vec<String>,
}

/// Messages sent between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessage {
    /// Assign a task to an agent
    TaskAssign {
        task_id: String,
        task: Task,
    },
    /// Return the result of a completed task
    TaskResult {
        task_id: String,
        result: TaskResult,
    },
    /// Query agent memory for information
    QueryMemory {
        query: String,
    },
    /// Response to a memory query
    MemoryResponse {
        results: Vec<MemoryEntry>,
    },
    /// Request approval for an operation
    RequestApproval {
        operation: String,
    },
    /// Response to an approval request
    ApprovalResponse {
        approved: bool,
        reason: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_display() {
        assert_eq!(AgentType::Director.to_string(), "Director");
        assert_eq!(AgentType::Researcher.to_string(), "Researcher");
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Medium);
        assert!(TaskPriority::Medium > TaskPriority::Low);
    }

    #[test]
    fn test_serialization() {
        let task = Task {
            id: "task-1".to_string(),
            description: "Test task".to_string(),
            priority: TaskPriority::High,
            dependencies: vec![],
            context: TaskContext {
                workspace_id: "ws-1".to_string(),
                user_instruction: "Test".to_string(),
                relevant_files: vec![],
                memory_context: vec![],
            },
        };

        let json = serde_json::to_string(&task).unwrap();
        let deserialized: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(task.id, deserialized.id);
    }
}
