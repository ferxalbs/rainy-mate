// Agent Registry Module
//
// This module provides the AgentRegistry service for managing all agents in the multi-agent system.
// It handles agent registration, lifecycle management, task assignment, and coordination.
//
// ## Responsibilities
//
// - Register and manage agents with unique IDs
// - Track agent status and lifecycle
// - Assign tasks to appropriate agents based on capabilities
// - Coordinate between agents for parallel execution
// - Provide statistics and monitoring
//
// ## Usage
//
// ```rust
// use rainy_cowork_lib::agents::{AgentRegistry, AgentConfig, Task};
//
// let registry = AgentRegistry::new(ai_provider);
// registry.register_agent(agent, config).await?;
// let agent_id = registry.assign_task(task).await?;
// ```

use crate::agents::agent_trait::{Agent, AgentConfig, AgentError};
use crate::agents::message_bus::MessageBus;
use crate::agents::status_monitoring::StatusMonitor;
use crate::agents::task_management::TaskManager;
use crate::agents::types::{AgentInfo, AgentMessage, Task};
use crate::ai::provider::AIProviderManager;
use dashmap::DashMap;
use std::sync::Arc;

/// Statistics about the agent registry
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistryStatistics {
    /// Total number of registered agents
    pub total_agents: usize,
    /// Number of idle agents
    pub idle_agents: usize,
    /// Number of busy agents
    pub busy_agents: usize,
    /// Number of agents in error state
    pub error_agents: usize,
    /// Number of currently active tasks
    pub active_tasks: usize,
}

/// Registry for managing all agents in the multi-agent system
///
/// This service provides centralized management of agents including:
/// - Registration and lifecycle management
/// - Task assignment based on capabilities
/// - Agent coordination and communication
/// - Status monitoring and statistics
pub struct AgentRegistry {
    /// Map of agent ID to agent instance
    agents: Arc<DashMap<String, Arc<dyn Agent>>>,
    /// Map of agent ID to agent configuration
    agent_configs: DashMap<String, AgentConfig>,
    /// Task manager for task assignment and tracking
    task_manager: TaskManager,
    /// Status monitor for agent status tracking
    status_monitor: StatusMonitor,
    /// Message bus for inter-agent communication
    message_bus: Arc<MessageBus>,
    /// AI provider manager for agent operations
    ai_provider: Arc<AIProviderManager>,
}

impl AgentRegistry {
    /// Create a new AgentRegistry
    ///
    /// # Arguments
    ///
    /// * `ai_provider` - AI provider manager for agent operations
    ///
    /// # Returns
    ///
    /// A new AgentRegistry instance
    pub fn new(ai_provider: Arc<AIProviderManager>) -> Self {
        let message_bus = Arc::new(MessageBus::new());
        let agents = Arc::new(DashMap::new());

        Self {
            agents: agents.clone(),
            agent_configs: DashMap::new(),
            task_manager: TaskManager::new(agents.clone()),
            status_monitor: StatusMonitor::new(agents),
            message_bus,
            ai_provider,
        }
    }

    /// Register a new agent
    ///
    /// # Arguments
    ///
    /// * `agent` - The agent to register
    /// * `config` - Configuration for the agent
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn register_agent(
        &self,
        agent: Arc<dyn Agent>,
        config: AgentConfig,
    ) -> Result<(), AgentError> {
        let agent_id = config.agent_id.clone();

        // Check if agent already exists
        if self.agents.contains_key(&agent_id) {
            return Err(AgentError::InvalidConfig(format!(
                "Agent {} already registered",
                agent_id
            )));
        }

        // Store agent and config
        self.agents.insert(agent_id.clone(), agent.clone());
        self.agent_configs.insert(agent_id.clone(), config);

        Ok(())
    }

    /// Unregister an agent
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent to unregister
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn unregister_agent(&self, agent_id: &str) -> Result<(), AgentError> {
        // Remove agent and config
        self.agents.remove(agent_id);
        self.agent_configs.remove(agent_id);

        // Clear task assignments for this agent
        let assignments_to_cancel: Vec<String> = self
            .task_manager
            .get_all_assignments()
            .iter()
            .filter(|(_, agent_id_ref)| agent_id_ref == agent_id)
            .map(|(task_id, _)| task_id.clone())
            .collect();

        for task_id in assignments_to_cancel {
            let _ = self.task_manager.cancel_task(&task_id).await;
        }

        Ok(())
    }

    /// Get agent by ID
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent to retrieve
    ///
    /// # Returns
    ///
    /// Option containing the agent if found
    pub fn get_agent(&self, agent_id: &str) -> Option<Arc<dyn Agent>> {
        self.status_monitor.get_agent(agent_id)
    }

    /// List all registered agents
    ///
    /// # Returns
    ///
    /// Vector of agent information for all registered agents
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        self.status_monitor.list_agents()
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
        // Assign task using task manager
        let agent_id = self.task_manager.assign_task(task.clone()).await?;

        // Get agent
        let agent = self
            .get_agent(&agent_id)
            .ok_or_else(|| AgentError::TaskExecutionFailed(format!("Agent {} not found", agent_id)))?;

        // Execute task asynchronously
        let agent_clone = agent.clone();
        let task_id = task.id.clone();
        let registry = self.clone();

        tokio::spawn(async move {
            let result = agent_clone.process_task(task).await;

            // Update agent status
            agent_clone
                .update_status(crate::agents::types::AgentStatus::Idle)
                .await;
            agent_clone.set_current_task(None).await;

            // Remove assignment
            registry.task_manager.remove_assignment(&task_id);

            // Handle result
            match result {
                Ok(_task_result) => {
                    // Send result to Director or other agents
                    // TODO: Implement result handling
                    println!("Task {} completed successfully", task_id);
                }
                Err(e) => {
                    // Handle error
                    agent_clone
                        .update_status(crate::agents::types::AgentStatus::Error(
                            e.to_string(),
                        ))
                        .await;
                    eprintln!("Task {} failed: {}", task_id, e);
                }
            }
        });

        Ok(agent_id)
    }

    /// Get agent status
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent
    ///
    /// # Returns
    ///
    /// Option containing the agent status if found
    pub fn get_agent_status(&self, agent_id: &str) -> Option<crate::agents::types::AgentStatus> {
        self.status_monitor.get_agent_status(agent_id)
    }

    /// Get all busy agents
    ///
    /// # Returns
    ///
    /// Vector of agent information for busy agents
    pub async fn get_busy_agents(&self) -> Vec<AgentInfo> {
        self.status_monitor.get_busy_agents()
    }

    /// Get all idle agents
    ///
    /// # Returns
    ///
    /// Vector of agent information for idle agents
    pub async fn get_idle_agents(&self) -> Vec<AgentInfo> {
        self.status_monitor.get_idle_agents()
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
        self.task_manager.get_task_agent(task_id)
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
        self.task_manager.cancel_task(task_id).await
    }

    /// Coordinate multiple agents for a task
    ///
    /// # Arguments
    ///
    /// * `task` - The task to coordinate
    ///
    /// # Returns
    ///
    /// Vector of agent IDs participating in the task
    pub async fn coordinate_agents(&self, task: Task) -> Result<Vec<String>, AgentError> {
        let mut participating_agents = Vec::new();

        // Find all agents that can handle this task
        for entry in self.agents.iter() {
            let agent = entry.value();
            if agent.can_handle(&task) {
                let info = agent.info();
                if matches!(info.status, crate::agents::types::AgentStatus::Idle) {
                    participating_agents.push(info.id.clone());
                }
            }
        }

        if participating_agents.is_empty() {
            return Err(AgentError::TaskExecutionFailed(
                "No available agents for coordination".to_string(),
            ));
        }

        // Assign task to the first available agent
        let _primary_agent = &participating_agents[0];
        self.assign_task(task).await?;

        Ok(participating_agents)
    }

    /// Broadcast a message to all agents
    ///
    /// # Arguments
    ///
    /// * `message` - The message to broadcast
    pub async fn broadcast_message(&self, message: AgentMessage) {
        for entry in self.agents.iter() {
            let agent = entry.value();
            let info = agent.info();
            if let Err(e) = agent.handle_message(message.clone()).await {
                eprintln!("Failed to send message to agent {}: {}", info.id, e);
            }
        }
    }

    /// Get registry statistics
    ///
    /// # Returns
    ///
    /// Statistics about the registry
    pub fn get_statistics(&self) -> RegistryStatistics {
        RegistryStatistics {
            total_agents: self.status_monitor.total_agent_count(),
            idle_agents: self.status_monitor.idle_agent_count(),
            busy_agents: self.status_monitor.busy_agent_count(),
            error_agents: self.status_monitor.error_agent_count(),
            active_tasks: self.task_manager.active_task_count(),
        }
    }

    /// Get message bus reference
    ///
    /// # Returns
    ///
    /// Reference to the message bus
    pub fn message_bus(&self) -> Arc<MessageBus> {
        self.message_bus.clone()
    }

    /// Get AI provider reference
    ///
    /// # Returns
    ///
    /// Reference to the AI provider manager
    pub fn ai_provider(&self) -> Arc<AIProviderManager> {
        self.ai_provider.clone()
    }
}

// Implement Clone for AgentRegistry
impl Clone for AgentRegistry {
    fn clone(&self) -> Self {
        Self {
            agents: self.agents.clone(),
            agent_configs: self.agent_configs.clone(),
            task_manager: TaskManager::new(self.agents.clone()),
            status_monitor: StatusMonitor::new(self.agents.clone()),
            message_bus: self.message_bus.clone(),
            ai_provider: self.ai_provider.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::base_agent::BaseAgent;
    use crate::agents::types::{AgentType, TaskContext, TaskPriority};

    #[tokio::test]
    async fn test_registry_creation() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        let stats = registry.get_statistics();
        assert_eq!(stats.total_agents, 0);
        assert_eq!(stats.active_tasks, 0);
    }

    #[tokio::test]
    async fn test_register_agent() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let message_bus = registry.message_bus();
        let agent = Arc::new(BaseAgent::new(config.clone(), registry.ai_provider(), message_bus));

        let result = registry.register_agent(agent, config).await;
        assert!(result.is_ok());

        let stats = registry.get_statistics();
        assert_eq!(stats.total_agents, 1);
    }

    #[tokio::test]
    async fn test_duplicate_agent_registration() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let message_bus = registry.message_bus();
        let agent1 = Arc::new(BaseAgent::new(
            config.clone(),
            registry.ai_provider(),
            message_bus.clone(),
        ));
        let agent2 = Arc::new(BaseAgent::new(config.clone(), registry.ai_provider(), message_bus));

        let result1 = registry.register_agent(agent1, config.clone()).await;
        assert!(result1.is_ok());

        let result2 = registry.register_agent(agent2, config).await;
        assert!(result2.is_err());
    }

    #[tokio::test]
    async fn test_get_agent() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let message_bus = registry.message_bus();
        let agent = Arc::new(BaseAgent::new(config.clone(), registry.ai_provider(), message_bus));

        registry.register_agent(agent, config).await.unwrap();

        let retrieved = registry.get_agent("test-agent");
        assert!(retrieved.is_some());

        let not_found = registry.get_agent("non-existent");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_agents() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        let config1 = AgentConfig {
            agent_id: "agent-1".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let config2 = AgentConfig {
            agent_id: "agent-2".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let message_bus = registry.message_bus();
        let agent1 = Arc::new(BaseAgent::new(
            config1.clone(),
            registry.ai_provider(),
            message_bus.clone(),
        ));
        let agent2 = Arc::new(BaseAgent::new(config2.clone(), registry.ai_provider(), message_bus));

        registry.register_agent(agent1, config1).await.unwrap();
        registry.register_agent(agent2, config2).await.unwrap();

        let agents = registry.list_agents().await;
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let message_bus = registry.message_bus();
        let agent = Arc::new(BaseAgent::new(config.clone(), registry.ai_provider(), message_bus));

        registry.register_agent(agent, config).await.unwrap();

        let stats = registry.get_statistics();
        assert_eq!(stats.total_agents, 1);
        assert_eq!(stats.idle_agents, 1);
        assert_eq!(stats.busy_agents, 0);
        assert_eq!(stats.error_agents, 0);
    }

    #[tokio::test]
    async fn test_get_idle_agents() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let message_bus = registry.message_bus();
        let agent = Arc::new(BaseAgent::new(config.clone(), registry.ai_provider(), message_bus));

        registry.register_agent(agent, config).await.unwrap();

        let idle_agents = registry.get_idle_agents().await;
        assert_eq!(idle_agents.len(), 1);
    }

    #[tokio::test]
    async fn test_get_busy_agents() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let message_bus = registry.message_bus();
        let agent = Arc::new(BaseAgent::new(config.clone(), registry.ai_provider(), message_bus));

        registry.register_agent(agent, config).await.unwrap();

        let busy_agents = registry.get_busy_agents().await;
        assert_eq!(busy_agents.len(), 0);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let message_bus = registry.message_bus();
        let agent = Arc::new(BaseAgent::new(config.clone(), registry.ai_provider(), message_bus));

        registry.register_agent(agent, config).await.unwrap();

        // Manually add a task assignment
        registry
            .task_manager
            .task_assignments
            .insert("task-1".to_string(), "test-agent".to_string());

        let result = registry.cancel_task("task-1").await;
        assert!(result.is_ok());

        let not_found = registry.cancel_task("non-existent").await;
        assert!(not_found.is_err());
    }

    #[tokio::test]
    async fn test_get_task_agent() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        // Manually add a task assignment
        registry
            .task_manager
            .task_assignments
            .insert("task-1".to_string(), "test-agent".to_string());

        let agent_id = registry.get_task_agent("task-1");
        assert_eq!(agent_id, Some("test-agent".to_string()));

        let not_found = registry.get_task_agent("non-existent");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_registry_clone() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = AgentRegistry::new(ai_provider);

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let message_bus = registry.message_bus();
        let agent = Arc::new(BaseAgent::new(config.clone(), registry.ai_provider(), message_bus));

        registry.register_agent(agent, config).await.unwrap();

        let cloned = registry.clone();
        let stats = cloned.get_statistics();
        assert_eq!(stats.total_agents, 1);
    }
}
