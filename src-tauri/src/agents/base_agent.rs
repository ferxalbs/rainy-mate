// Base Agent Module
//
// This module provides the BaseAgent struct, which implements common functionality
// that all specialized agents can inherit from. It provides:
//
// - Agent information management
// - Status tracking
// - Task assignment and tracking
// - Message sending and receiving
// - AI provider integration
//
// Specialized agents (Director, Researcher, Executor, etc.) should wrap BaseAgent
// and implement the Agent trait with their specific logic.
//
// ## Usage
//
// ```rust
// use rainy_cowork_lib::agents::{BaseAgent, AgentConfig, AgentType, AgentStatus};
//
// let config = AgentConfig {
//     agent_id: "researcher-1".to_string(),
//     workspace_id: "workspace-1".to_string(),
//     ai_provider: "gemini".to_string(),
//     model: "gemini-2.0-flash".to_string(),
//     settings: serde_json::json!({}),
// };
//
// let base_agent = BaseAgent::new(config, ai_provider, message_bus);
// base_agent.update_status(AgentStatus::Busy).await;
// ```

use crate::agents::agent_trait::{Agent, AgentConfig, AgentError};

use crate::agents::types::{AgentInfo, AgentMessage, AgentStatus, AgentType, Task, TaskResult};
use crate::ai::provider::AIProviderManager;
use crate::ai::provider_types::StreamingChunk;
use crate::models::ProviderType;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Base agent providing common functionality for all specialized agents
///
/// This struct implements shared agent functionality including:
/// - Agent information management
/// - Status tracking
/// - Message communication
/// - AI provider integration
///
/// Specialized agents should wrap this struct and implement the Agent trait.
pub struct BaseAgent {
    /// Agent information (thread-safe)
    info: Arc<RwLock<AgentInfo>>,
    /// Agent configuration
    config: AgentConfig,
    /// AI provider manager
    ai_provider: Arc<AIProviderManager>,
}

impl BaseAgent {
    /// Create a new BaseAgent
    ///
    /// # Arguments
    ///
    /// * `config` - Agent configuration
    /// * `ai_provider` - AI provider manager
    /// * `message_bus` - Message bus for communication
    ///
    /// # Returns
    ///
    /// A new BaseAgent instance
    pub fn new(
        config: AgentConfig,
        ai_provider: Arc<AIProviderManager>,
        _message_bus: Arc<()>, // Removed MessageBus type
    ) -> Self {
        let info = AgentInfo {
            id: config.agent_id.clone(),
            name: config.agent_id.clone(),
            agent_type: AgentType::Director, // Will be overridden by specialized agents
            status: AgentStatus::Idle,
            current_task: None,
        };

        Self {
            info: Arc::new(RwLock::new(info)),
            config,
            ai_provider,
        }
    }

    /// Update the agent's status
    pub async fn update_status(&self, status: AgentStatus) {
        let mut info = self.info.write().await;
        info.status = status;
    }

    /// Set the current task being processed
    pub async fn set_current_task(&self, task_id: Option<String>) {
        let mut info = self.info.write().await;
        info.current_task = task_id;
    }

    /// Query the AI provider with a prompt
    pub async fn query_ai(&self, prompt: &str) -> Result<String, AgentError> {
        // Map provider name to ProviderType
        let provider_type = match self.config.ai_provider.as_str() {
            "rainy_api" => ProviderType::RainyApi,
            "gemini" => ProviderType::Gemini,
            _ => {
                return Err(AgentError::InvalidConfig(format!(
                    "Unknown provider: {}",
                    self.config.ai_provider
                )))
            }
        };

        // Execute prompt using AI provider manager
        let response = self
            .ai_provider
            .execute_prompt(
                &provider_type,
                &self.config.model,
                prompt,
                |_, _| {},
                None::<fn(StreamingChunk)>,
            )
            .await
            .map_err(|e| AgentError::TaskExecutionFailed(e.to_string()))?;

        Ok(response)
    }

    /// Get the agent's configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
}

#[async_trait::async_trait]
impl Agent for BaseAgent {
    fn info(&self) -> AgentInfo {
        // synchronous clone
        AgentInfo {
            id: self.config.agent_id.clone(),
            name: self.config.agent_id.clone(),
            agent_type: AgentType::Director,
            status: AgentStatus::Idle,
            current_task: None,
        }
    }

    async fn process_task(&self, task: Task) -> Result<TaskResult, AgentError> {
        // Base implementation - specialized agents should override this
        let prompt = format!(
            "Task: {}\n\nContext: {}\n\nPlease complete this task.",
            task.description, task.context.user_instruction
        );

        let response = self.query_ai(&prompt).await?;

        Ok(TaskResult {
            success: true,
            output: response,
            errors: vec![],
            metadata: serde_json::json!({
                "task_id": task.id,
                "agent_id": self.config.agent_id,
            }),
        })
    }

    async fn handle_message(&self, message: AgentMessage) -> Result<(), AgentError> {
        match message {
            AgentMessage::TaskAssign { task, .. } => {
                self.process_task(task).await?;
            }
            _ => {}
        }
        Ok(())
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "task_processing".to_string(),
            "message_handling".to_string(),
            "ai_query".to_string(),
        ]
    }

    fn can_handle(&self, _task: &Task) -> bool {
        true
    }

    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError> {
        self.config = config;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        Ok(())
    }

    async fn update_status(&self, status: AgentStatus) {
        let mut info = self.info.write().await;
        info.status = status;
    }

    async fn set_current_task(&self, task_id: Option<String>) {
        let mut info = self.info.write().await;
        info.current_task = task_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_base_agent_creation() {
        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let ai_provider = Arc::new(AIProviderManager::new());

        let agent = BaseAgent::new(config, ai_provider, Arc::new(()));
        assert_eq!(agent.config().agent_id, "test-agent");
    }

    #[tokio::test]
    async fn test_status_update() {
        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let ai_provider = Arc::new(AIProviderManager::new());

        let agent = BaseAgent::new(config, ai_provider, Arc::new(()));
        agent.update_status(AgentStatus::Busy).await;

        let info = agent.info();
        assert!(matches!(info.status, AgentStatus::Busy));
    }

    #[tokio::test]
    async fn test_current_task() {
        let config = AgentConfig {
            agent_id: "test-agent".to_string(),
            workspace_id: "workspace-1".to_string(),
            ai_provider: "gemini".to_string(),
            model: "gemini-2.0-flash".to_string(),
            settings: serde_json::json!({}),
        };

        let ai_provider = Arc::new(AIProviderManager::new());

        let agent = BaseAgent::new(config, ai_provider, Arc::new(()));
        agent.set_current_task(Some("task-1".to_string())).await;

        let info = agent.info();
        assert_eq!(info.current_task, Some("task-1".to_string()));
    }
}
