// Status Monitoring Module
//
// This module provides status monitoring and statistics functionality for AgentRegistry.
// It tracks agent status, provides statistics, and monitors agent health.
//
// ## Responsibilities
//
// - Monitor agent status (idle, busy, error)
// - Calculate registry statistics
// - Filter agents by status
// - Provide status queries

use crate::agents::agent_trait::Agent;
use crate::agents::types::{AgentInfo, AgentStatus};
use dashmap::DashMap;
use std::sync::Arc;

/// Status monitoring functionality for AgentRegistry
pub struct StatusMonitor {
    /// Map of agent ID to agent instance (shared reference)
    agents: Arc<DashMap<String, Arc<dyn Agent>>>,
}

impl StatusMonitor {
    /// Create a new StatusMonitor
    ///
    /// # Arguments
    ///
    /// * `agents` - Map of agent ID to agent instance
    ///
    /// # Returns
    ///
    /// A new StatusMonitor instance
    pub fn new(agents: Arc<DashMap<String, Arc<dyn Agent>>>) -> Self {
        Self { agents }
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
    pub fn get_agent_status(&self, agent_id: &str) -> Option<AgentStatus> {
        self.agents
            .get(agent_id)
            .map(|agent| agent.value().info().status)
    }

    /// Get all busy agents
    ///
    /// # Returns
    ///
    /// Vector of agent information for busy agents
    pub fn get_busy_agents(&self) -> Vec<AgentInfo> {
        self.agents
            .iter()
            .filter(|entry| matches!(entry.value().info().status, AgentStatus::Busy))
            .map(|entry| entry.value().info())
            .collect()
    }

    /// Get all idle agents
    ///
    /// # Returns
    ///
    /// Vector of agent information for idle agents
    pub fn get_idle_agents(&self) -> Vec<AgentInfo> {
        self.agents
            .iter()
            .filter(|entry| matches!(entry.value().info().status, AgentStatus::Idle))
            .map(|entry| entry.value().info())
            .collect()
    }

    /// Get all agents in error state
    ///
    /// # Returns
    ///
    /// Vector of agent information for agents in error state
    pub fn get_error_agents(&self) -> Vec<AgentInfo> {
        self.agents
            .iter()
            .filter(|entry| matches!(entry.value().info().status, AgentStatus::Error(_)))
            .map(|entry| entry.value().info())
            .collect()
    }

    /// Get all registered agents
    ///
    /// # Returns
    ///
    /// Vector of agent information for all registered agents
    pub fn list_agents(&self) -> Vec<AgentInfo> {
        self.agents
            .iter()
            .map(|entry| entry.value().info())
            .collect()
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
        self.agents.get(agent_id).map(|a| a.clone())
    }

    /// Get total number of agents
    ///
    /// # Returns
    ///
    /// Total number of registered agents
    pub fn total_agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Get number of idle agents
    ///
    /// # Returns
    ///
    /// Number of idle agents
    pub fn idle_agent_count(&self) -> usize {
        self.get_idle_agents().len()
    }

    /// Get number of busy agents
    ///
    /// # Returns
    ///
    /// Number of busy agents
    pub fn busy_agent_count(&self) -> usize {
        self.get_busy_agents().len()
    }

    /// Get number of agents in error state
    ///
    /// # Returns
    ///
    /// Number of agents in error state
    pub fn error_agent_count(&self) -> usize {
        self.get_error_agents().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::base_agent::BaseAgent;
    use crate::agents::agent_trait::AgentConfig;
    use crate::agents::types::AgentType;
    use crate::ai::provider::AIProviderManager;
    use crate::agents::message_bus::MessageBus;

    #[tokio::test]
    async fn test_get_agent_status() {
        let agents: Arc<DashMap<String, Arc<dyn Agent>>> = Arc::new(DashMap::new());
        let ai_provider = Arc::new(AIProviderManager::new());
        let message_bus = Arc::new(MessageBus::new());

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let agent: Arc<dyn Agent> = Arc::new(BaseAgent::new(config.clone(), ai_provider, message_bus));
        agents.insert(config.agent_id.clone(), agent);

        let monitor = StatusMonitor::new(agents);

        let status = monitor.get_agent_status("test-agent");
        assert!(status.is_some());
        assert!(matches!(status.unwrap(), AgentStatus::Idle));

        let not_found = monitor.get_agent_status("non-existent");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_get_idle_agents() {
        let agents: Arc<DashMap<String, Arc<dyn Agent>>> = Arc::new(DashMap::new());
        let ai_provider = Arc::new(AIProviderManager::new());
        let message_bus = Arc::new(MessageBus::new());

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let agent: Arc<dyn Agent> = Arc::new(BaseAgent::new(config.clone(), ai_provider, message_bus));
        agents.insert(config.agent_id.clone(), agent);

        let monitor = StatusMonitor::new(agents);

        let idle_agents = monitor.get_idle_agents();
        assert_eq!(idle_agents.len(), 1);
        assert_eq!(idle_agents[0].id, "test-agent");
    }

    #[tokio::test]
    async fn test_get_busy_agents() {
        let agents: Arc<DashMap<String, Arc<dyn Agent>>> = Arc::new(DashMap::new());
        let ai_provider = Arc::new(AIProviderManager::new());
        let message_bus = Arc::new(MessageBus::new());

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let agent: Arc<dyn Agent> = Arc::new(BaseAgent::new(config.clone(), ai_provider, message_bus));
        agents.insert(config.agent_id.clone(), agent);

        let monitor = StatusMonitor::new(agents);

        let busy_agents = monitor.get_busy_agents();
        assert_eq!(busy_agents.len(), 0);
    }

    #[tokio::test]
    async fn test_list_agents() {
        let agents: Arc<DashMap<String, Arc<dyn Agent>>> = Arc::new(DashMap::new());
        let ai_provider = Arc::new(AIProviderManager::new());
        let message_bus = Arc::new(MessageBus::new());

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

        let agent1: Arc<dyn Agent> = Arc::new(BaseAgent::new(
            config1.clone(),
            ai_provider.clone(),
            message_bus.clone(),
        ));
        let agent2: Arc<dyn Agent> = Arc::new(BaseAgent::new(config2.clone(), ai_provider, message_bus));

        agents.insert(config1.agent_id.clone(), agent1);
        agents.insert(config2.agent_id.clone(), agent2);

        let monitor = StatusMonitor::new(agents);

        let all_agents = monitor.list_agents();
        assert_eq!(all_agents.len(), 2);
    }

    #[tokio::test]
    async fn test_agent_counts() {
        let agents: Arc<DashMap<String, Arc<dyn Agent>>> = Arc::new(DashMap::new());
        let ai_provider = Arc::new(AIProviderManager::new());
        let message_bus = Arc::new(MessageBus::new());

        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let agent: Arc<dyn Agent> = Arc::new(BaseAgent::new(config.clone(), ai_provider, message_bus));
        agents.insert(config.agent_id.clone(), agent);

        let monitor = StatusMonitor::new(agents);

        assert_eq!(monitor.total_agent_count(), 1);
        assert_eq!(monitor.idle_agent_count(), 1);
        assert_eq!(monitor.busy_agent_count(), 0);
        assert_eq!(monitor.error_agent_count(), 0);
    }
}
