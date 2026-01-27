// Message Bus Module
//
// This module implements the MessageBus for inter-agent communication.
// It provides a simple in-memory message queue system that allows agents
// to send messages to each other asynchronously.
//
// ## Features
//
// - Point-to-point messaging: Send messages to specific agents
// - Broadcast messaging: Send messages to all agents except the sender
// - Thread-safe: Uses Arc<RwLock> for concurrent access
// - Simple API: Easy to use for agent communication
//
// ## Usage
//
// ```rust
// use rainy_cowork_lib::agents::{MessageBus, AgentMessage};
//
// let message_bus = Arc::new(MessageBus::new());
//
// // Send a message to a specific agent
// message_bus.send(
//     "director-1".to_string(),
//     "researcher-1".to_string(),
//     AgentMessage::TaskAssign { task_id: "task-1".to_string(), task }
// ).await?;
//
// // Receive messages
// let messages = message_bus.receive("researcher-1").await;
//
// // Broadcast to all agents
// message_bus.broadcast(
//     "director-1".to_string(),
//     AgentMessage::TaskResult { task_id: "task-1".to_string(), result }
// ).await;
// ```

use crate::agents::agent_trait::AgentError;
use crate::agents::types::AgentMessage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Message bus for inter-agent communication
///
/// Provides a simple in-memory message queue system that allows agents
/// to send messages to each other asynchronously.
///
/// # Thread Safety
///
/// The MessageBus is thread-safe and can be shared across multiple agents
/// using Arc<RwLock> for concurrent access.
pub struct MessageBus {
    /// Message queues for each agent
    /// Maps agent_id to a vector of pending messages
    queues: Arc<RwLock<HashMap<String, Vec<AgentMessage>>>>,
}

impl MessageBus {
    /// Create a new MessageBus
    ///
    /// # Returns
    ///
    /// A new MessageBus instance with empty message queues
    pub fn new() -> Self {
        Self {
            queues: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Send a message to a specific agent
    ///
    /// # Arguments
    ///
    /// * `from` - ID of the sender agent
    /// * `to` - ID of the recipient agent
    /// * `message` - The message to send
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn send(
        &self,
        from: String,
        to: String,
        message: AgentMessage,
    ) -> Result<(), AgentError> {
        let mut queues = self.queues.write().await;
        queues.entry(to).or_insert_with(Vec::new).push(message);
        Ok(())
    }

    /// Receive all pending messages for an agent
    ///
    /// This method removes and returns all pending messages for the specified
    /// agent. After calling this method, the agent's message queue will be empty.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent to receive messages for
    ///
    /// # Returns
    ///
    /// Vector of pending messages (empty if no messages are pending)
    pub async fn receive(&self, agent_id: &str) -> Vec<AgentMessage> {
        let mut queues = self.queues.write().await;
        queues.remove(agent_id).unwrap_or_default()
    }

    /// Broadcast a message to all agents except the sender
    ///
    /// This method sends the same message to all registered agents except
    /// the sender. This is useful for announcements and notifications.
    ///
    /// # Arguments
    ///
    /// * `from` - ID of the sender agent (will not receive the message)
    /// * `message` - The message to broadcast
    pub async fn broadcast(&self, from: String, message: AgentMessage) {
        let mut queues = self.queues.write().await;
        for (agent_id, queue) in queues.iter_mut() {
            if agent_id != &from {
                queue.push(message.clone());
            }
        }
    }

    /// Get the number of pending messages for an agent
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent to check
    ///
    /// # Returns
    ///
    /// Number of pending messages
    pub async fn pending_count(&self, agent_id: &str) -> usize {
        let queues = self.queues.read().await;
        queues.get(agent_id).map(|q| q.len()).unwrap_or(0)
    }

    /// Check if an agent has any pending messages
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent to check
    ///
    /// # Returns
    ///
    /// true if there are pending messages, false otherwise
    pub async fn has_pending(&self, agent_id: &str) -> bool {
        self.pending_count(agent_id).await > 0
    }

    /// Clear all messages for an agent
    ///
    /// # Arguments
    ///
    /// * `agent_id` - ID of the agent to clear messages for
    pub async fn clear(&self, agent_id: &str) {
        let mut queues = self.queues.write().await;
        queues.remove(agent_id);
    }

    /// Clear all messages for all agents
    ///
    /// This removes all message queues from the bus.
    pub async fn clear_all(&self) {
        let mut queues = self.queues.write().await;
        queues.clear();
    }

    /// Get the number of agents with pending messages
    ///
    /// # Returns
    ///
    /// Number of agents that have at least one pending message
    pub async fn active_agents_count(&self) -> usize {
        let queues = self.queues.read().await;
        queues.values().filter(|q| !q.is_empty()).count()
    }

    /// Get total number of pending messages across all agents
    ///
    /// # Returns
    ///
    /// Total number of pending messages
    pub async fn total_pending_count(&self) -> usize {
        let queues = self.queues.read().await;
        queues.values().map(|q| q.len()).sum()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::types::{Task, TaskContext, TaskPriority, TaskResult};

    #[tokio::test]
    async fn test_send_and_receive() {
        let message_bus = MessageBus::new();

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

        let message = AgentMessage::TaskAssign {
            task_id: "task-1".to_string(),
            task,
        };

        message_bus
            .send("agent-1".to_string(), "agent-2".to_string(), message)
            .await
            .unwrap();

        let messages = message_bus.receive("agent-2").await;
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_broadcast() {
        let message_bus = MessageBus::new();

        let result = TaskResult {
            success: true,
            output: "Test output".to_string(),
            errors: vec![],
            metadata: serde_json::json!({}),
        };

        let message = AgentMessage::TaskResult {
            task_id: "task-1".to_string(),
            result,
        };

        // Register some agents by sending messages to them
        message_bus
            .send("agent-1".to_string(), "agent-2".to_string(), AgentMessage::QueryMemory {
                query: "test".to_string(),
            })
            .await
            .unwrap();
        message_bus
            .send("agent-1".to_string(), "agent-3".to_string(), AgentMessage::QueryMemory {
                query: "test".to_string(),
            })
            .await
            .unwrap();

        // Clear the messages
        message_bus.receive("agent-2").await;
        message_bus.receive("agent-3").await;

        // Broadcast from agent-1
        message_bus
            .broadcast("agent-1".to_string(), message)
            .await;

        // agent-2 and agent-3 should receive the broadcast
        let messages_2 = message_bus.receive("agent-2").await;
        let messages_3 = message_bus.receive("agent-3").await;

        assert_eq!(messages_2.len(), 1);
        assert_eq!(messages_3.len(), 1);
    }

    #[tokio::test]
    async fn test_pending_count() {
        let message_bus = MessageBus::new();

        assert_eq!(message_bus.pending_count("agent-1").await, 0);

        message_bus
            .send("agent-1".to_string(), "agent-1".to_string(), AgentMessage::QueryMemory {
                query: "test".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(message_bus.pending_count("agent-1").await, 1);
    }

    #[tokio::test]
    async fn test_has_pending() {
        let message_bus = MessageBus::new();

        assert!(!message_bus.has_pending("agent-1").await);

        message_bus
            .send("agent-1".to_string(), "agent-1".to_string(), AgentMessage::QueryMemory {
                query: "test".to_string(),
            })
            .await
            .unwrap();

        assert!(message_bus.has_pending("agent-1").await);
    }

    #[tokio::test]
    async fn test_clear() {
        let message_bus = MessageBus::new();

        message_bus
            .send("agent-1".to_string(), "agent-1".to_string(), AgentMessage::QueryMemory {
                query: "test".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(message_bus.pending_count("agent-1").await, 1);

        message_bus.clear("agent-1").await;

        assert_eq!(message_bus.pending_count("agent-1").await, 0);
    }

    #[tokio::test]
    async fn test_clear_all() {
        let message_bus = MessageBus::new();

        message_bus
            .send("agent-1".to_string(), "agent-1".to_string(), AgentMessage::QueryMemory {
                query: "test".to_string(),
            })
            .await
            .unwrap();
        message_bus
            .send("agent-1".to_string(), "agent-2".to_string(), AgentMessage::QueryMemory {
                query: "test".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(message_bus.total_pending_count().await, 2);

        message_bus.clear_all().await;

        assert_eq!(message_bus.total_pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_active_agents_count() {
        let message_bus = MessageBus::new();

        assert_eq!(message_bus.active_agents_count().await, 0);

        message_bus
            .send("agent-1".to_string(), "agent-1".to_string(), AgentMessage::QueryMemory {
                query: "test".to_string(),
            })
            .await
            .unwrap();
        message_bus
            .send("agent-1".to_string(), "agent-2".to_string(), AgentMessage::QueryMemory {
                query: "test".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(message_bus.active_agents_count().await, 2);
    }
}
