// Task Management Module
//
// This module provides task assignment and management functionality for AgentRegistry.
// It handles task assignment to agents, task cancellation, and task tracking.
//
// ## Responsibilities
//
// - Assign tasks to appropriate agents based on capabilities
// - Cancel tasks and clean up assignments
// - Track task-to-agent mappings
// - Find best agent for a given task

use crate::agents::agent_trait::{Agent, AgentError};
use crate::agents::types::{AgentStatus, Task};
use dashmap::DashMap;
use std::sync::Arc;

/// Task management functionality for AgentRegistry
pub struct TaskManager {
    /// Map of task ID to agent ID (task assignments)
    pub(crate) task_assignments: DashMap<String, String>,
    /// Map of agent ID to agent instance (shared reference)
    agents: Arc<DashMap<String, Arc<dyn Agent>>>,
}

impl TaskManager {
    /// Create a new TaskManager
    ///
    /// # Arguments
    ///
    /// * `agents` - Map of agent ID to agent instance
    ///
    /// # Returns
    ///
    /// A new TaskManager instance
    pub fn new(agents: Arc<DashMap<String, Arc<dyn Agent>>>) -> Self {
        Self {
            task_assignments: DashMap::new(),
            agents,
        }
    }

    /// Assign task to appropriate agent
    ///
    /// # Arguments
    ///
    /// * `task` - The task to assign
    ///
    /// # Returns
    ///
    /// ID of the agent assigned to the task
    pub async fn assign_task(&self, task: Task) -> Result<String, AgentError> {
        // Find best agent for task
        let agent_id = self.find_best_agent(&task)?;

        // Get agent
        let agent = self
            .agents
            .get(&agent_id)
            .ok_or_else(|| AgentError::TaskExecutionFailed(format!("Agent {} not found", agent_id)))?;

        // Update agent status
        agent.update_status(AgentStatus::Busy).await;
        agent.set_current_task(Some(task.id.clone())).await;

        // Track assignment
        self.task_assignments.insert(task.id.clone(), agent_id.clone());

        Ok(agent_id)
    }

    /// Find best agent for task based on capabilities
    ///
    /// # Arguments
    ///
    /// * `task` - The task to find an agent for
    ///
    /// # Returns
    ///
    /// ID of the best agent for the task
    fn find_best_agent(&self, task: &Task) -> Result<String, AgentError> {
        for entry in self.agents.iter() {
            let agent = entry.value();
            if agent.can_handle(task) {
                // Check if agent is idle
                let info = agent.info();
                if matches!(info.status, AgentStatus::Idle) {
                    return Ok(info.id);
                }
            }
        }

        Err(AgentError::TaskExecutionFailed(
            "No available agent can handle this task".to_string(),
        ))
    }

    /// Get agent assigned to a task
    ///
    /// # Arguments
    ///
    /// * `task_id` - ID of the task
    ///
    /// # Returns
    ///
    /// Option containing the agent ID if found
    pub fn get_task_agent(&self, task_id: &str) -> Option<String> {
        self.task_assignments
            .get(task_id)
            .map(|entry| entry.value().clone())
    }

    /// Cancel a task
    ///
    /// # Arguments
    ///
    /// * `task_id` - ID of the task to cancel
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), AgentError> {
        if let Some((_, agent_id)) = self.task_assignments.remove(task_id) {
            if let Some(agent) = self.agents.get(&agent_id) {
                agent.update_status(AgentStatus::Idle).await;
                agent.set_current_task(None).await;
            }
            Ok(())
        } else {
            Err(AgentError::TaskExecutionFailed(format!(
                "Task {} not found",
                task_id
            )))
        }
    }

    /// Remove task assignment (called when task completes)
    ///
    /// # Arguments
    ///
    /// * `task_id` - ID of the task to remove
    pub fn remove_assignment(&self, task_id: &str) {
        self.task_assignments.remove(task_id);
    }

    /// Get number of active tasks
    ///
    /// # Returns
    ///
    /// Number of currently active tasks
    pub fn active_task_count(&self) -> usize {
        self.task_assignments.len()
    }

    /// Get all task assignments
    ///
    /// # Returns
    ///
    /// Vector of (task_id, agent_id) tuples
    pub fn get_all_assignments(&self) -> Vec<(String, String)> {
        self.task_assignments
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::base_agent::BaseAgent;
    use crate::agents::types::{TaskContext, TaskPriority};
    use crate::ai::provider::AIProviderManager;
    use crate::agents::message_bus::MessageBus;

    #[tokio::test]
    async fn test_task_assignment() {
        let agents: Arc<DashMap<String, Arc<dyn Agent>>> = Arc::new(DashMap::new());
        let ai_provider = Arc::new(AIProviderManager::new());
        let message_bus = Arc::new(MessageBus::new());

        let config = crate::agents::agent_trait::AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let agent: Arc<dyn Agent> = Arc::new(BaseAgent::new(config.clone(), ai_provider, message_bus));
        agents.insert(config.agent_id.clone(), agent);

        let task_manager = TaskManager::new(agents);

        let task = Task {
            id: "task-1".to_string(),
            description: "Test task".to_string(),
            priority: TaskPriority::High,
            dependencies: vec![],
            context: TaskContext {
                workspace_id: "workspace-1".to_string(),
                user_instruction: "Test".to_string(),
                relevant_files: vec![],
                memory_context: vec![],
            },
        };

        let agent_id = task_manager.assign_task(task).await;
        assert!(agent_id.is_ok());
    }

    #[tokio::test]
    async fn test_get_task_agent() {
        let agents: Arc<DashMap<String, Arc<dyn Agent>>> = Arc::new(DashMap::new());
        let task_manager = TaskManager::new(agents);

        task_manager
            .task_assignments
            .insert("task-1".to_string(), "agent-1".to_string());

        let agent_id = task_manager.get_task_agent("task-1");
        assert_eq!(agent_id, Some("agent-1".to_string()));

        let not_found = task_manager.get_task_agent("non-existent");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let agents: Arc<DashMap<String, Arc<dyn Agent>>> = Arc::new(DashMap::new());
        let ai_provider = Arc::new(AIProviderManager::new());
        let message_bus = Arc::new(MessageBus::new());

        let config = crate::agents::agent_trait::AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let agent: Arc<dyn Agent> = Arc::new(BaseAgent::new(config.clone(), ai_provider, message_bus));
        agents.insert(config.agent_id.clone(), agent);

        let task_manager = TaskManager::new(agents);

        // Manually add a task assignment
        task_manager
            .task_assignments
            .insert("task-1".to_string(), "test-agent".to_string());

        let result = task_manager.cancel_task("task-1").await;
        assert!(result.is_ok());

        let not_found = task_manager.cancel_task("non-existent").await;
        assert!(not_found.is_err());
    }

    #[tokio::test]
    async fn test_active_task_count() {
        let agents: Arc<DashMap<String, Arc<dyn Agent>>> = Arc::new(DashMap::new());
        let task_manager = TaskManager::new(agents);

        assert_eq!(task_manager.active_task_count(), 0);

        task_manager
            .task_assignments
            .insert("task-1".to_string(), "agent-1".to_string());
        task_manager
            .task_assignments
            .insert("task-2".to_string(), "agent-2".to_string());

        assert_eq!(task_manager.active_task_count(), 2);
    }
}
