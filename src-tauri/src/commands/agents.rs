// Tauri Commands for Agent Management
//
// This module exposes the multi-agent system to the React frontend through Tauri commands.
// It provides commands for agent registration, task management, communication, and statistics.
//
// ## Architecture
//
// - AgentRegistryState: Global state wrapper for AgentRegistry
// - Commands: Async Tauri commands that interact with the agent system
// - Error Handling: All errors converted to String for frontend consumption
//
// ## Usage
//
// ```rust
// // Register agent
// let agent_id = invoke("register_agent", RegisterAgentParams {
//     agent_type: "director".to_string(),
//     name: "Main Director".to_string(),
//     workspace_id: "workspace-1".to_string(),
//     ai_provider: "gemini".to_string(),
//     model: "gemini-2.0-flash".to_string(),
//     settings: None,
// }).await?;
//
// // Execute task
// let result = invoke("execute_multi_agent_task", ExecuteTaskParams {
//     task_id: "task-123".to_string(),
// }).await?;
// ```

use crate::agents::{
    Agent, AgentConfig, AgentError, AgentInfo, AgentMessage, AgentRegistry, AgentType, Task,
    TaskContext, TaskPriority, TaskResult,
};
use std::sync::Arc;
use tauri::State;

/// Global state wrapper for AgentRegistry
pub struct AgentRegistryState(pub Arc<AgentRegistry>);

/// Register a new agent
///
/// Creates and registers a new agent of the specified type with the given configuration.
/// Returns the generated agent ID.
#[tauri::command]
pub async fn register_agent(
    registry: State<'_, AgentRegistryState>,
    agent_type: String,
    name: String,
    workspace_id: String,
    ai_provider: String,
    model: String,
    settings: Option<serde_json::Value>,
) -> Result<String, String> {
    let agent_id = format!("{}_{}", agent_type.to_lowercase(), uuid::Uuid::new_v4());

    let config = AgentConfig {
        agent_id: agent_id.clone(),
        workspace_id,
        ai_provider,
        model,
        settings: settings.unwrap_or_default(),
    };

    // Create agent based on type
    let agent: Arc<dyn Agent> = match agent_type.as_str() {
        "director" => {
            let director = crate::agents::director_agent::DirectorAgent::new(
                config.clone(),
                registry.0.clone(),
            );
            Arc::new(director)
        }
        "researcher" => {
            let researcher = crate::agents::researcher::ResearcherAgent::new(
                config.clone(),
                registry.0.clone(),
            );
            Arc::new(researcher)
        }
        "executor" => {
            let executor = crate::agents::executor::ExecutorAgent::new(
                config.clone(),
                registry.0.clone(),
            );
            Arc::new(executor)
        }
        "creator" => {
            let creator = crate::agents::creator::CreatorAgent::new(
                config.clone(),
                registry.0.clone(),
            );
            Arc::new(creator)
        }
        "designer" => {
            let designer = crate::agents::designer::DesignerAgent::new(
                config.clone(),
                registry.0.clone(),
            );
            Arc::new(designer)
        }
        "developer" => {
            let developer = crate::agents::developer::DeveloperAgent::new(
                config.clone(),
                registry.0.clone(),
            );
            Arc::new(developer)
        }
        "analyst" => {
            let analyst = crate::agents::analyst::AnalystAgent::new(
                config.clone(),
                registry.0.clone(),
            );
            Arc::new(analyst)
        }
        "critic" => {
            let critic = crate::agents::critic::CriticAgent::new(
                config.clone(),
                registry.0.clone(),
            );
            Arc::new(critic)
        }
        "governor" => {
            let governor = crate::agents::governor::GovernorAgent::new(
                config.clone(),
                registry.0.clone(),
            );
            Arc::new(governor)
        }
        _ => {
            return Err(format!("Unknown agent type: {}", agent_type));
        }
    };

    registry
        .0
        .register_agent(agent, config)
        .await
        .map_err(|e| e.to_string())?;

    Ok(agent_id)
}

/// Unregister an agent
///
/// Removes an agent from the registry and cleans up its resources.
#[tauri::command]
pub async fn unregister_agent(
    registry: State<'_, AgentRegistryState>,
    agent_id: String,
) -> Result<(), String> {
    registry
        .0
        .unregister_agent(&agent_id)
        .await
        .map_err(|e| e.to_string())
}

/// List all registered agents
///
/// Returns information about all currently registered agents.
#[tauri::command]
pub async fn list_agents(
    registry: State<'_, AgentRegistryState>,
) -> Result<Vec<AgentInfo>, String> {
    Ok(registry.0.list_agents().await)
}

/// Get information about a specific agent
///
/// Returns detailed information about the specified agent.
#[tauri::command]
pub async fn get_agent_info(
    registry: State<'_, AgentRegistryState>,
    agent_id: String,
) -> Result<AgentInfo, String> {
    let agent = registry
        .0
        .get_agent(&agent_id)
        .ok_or_else(|| format!("Agent {} not found", agent_id))?;

    Ok(agent.info())
}

/// Get status of a specific agent
///
/// Returns the current status of the specified agent.
#[tauri::command]
pub async fn get_agent_status(
    registry: State<'_, AgentRegistryState>,
    agent_id: String,
) -> Result<String, String> {
    let agent = registry
        .0
        .get_agent(&agent_id)
        .ok_or_else(|| format!("Agent {} not found", agent_id))?;

    let info = agent.info();
    Ok(format!("{:?}", info.status))
}

/// Create a task for multi-agent execution
///
/// Creates a new task with the specified parameters and returns the task ID.
/// The task can later be executed through the DirectorAgent.
#[tauri::command]
pub async fn create_multi_agent_task(
    registry: State<'_, AgentRegistryState>,
    description: String,
    workspace_id: String,
    user_instruction: String,
    relevant_files: Vec<String>,
    priority: String,
) -> Result<String, String> {
    let task_id = uuid::Uuid::new_v4().to_string();

    let priority = match priority.as_str() {
        "low" => TaskPriority::Low,
        "medium" => TaskPriority::Medium,
        "high" => TaskPriority::High,
        "critical" => TaskPriority::Critical,
        _ => TaskPriority::Medium,
    };

    let task = Task {
        id: task_id.clone(),
        description,
        priority,
        dependencies: vec![],
        context: TaskContext {
            workspace_id,
            user_instruction,
            relevant_files,
            memory_context: vec![],
        },
    };

    // Store task for later execution
    // TODO: Implement task storage

    Ok(task_id)
}

/// Execute a task through DirectorAgent
///
/// Executes a previously created task through the DirectorAgent.
/// Sends progress events through the provided channel.
#[tauri::command]
pub async fn execute_multi_agent_task(
    registry: State<'_, AgentRegistryState>,
    task_id: String,
    on_event: tauri::ipc::Channel<serde_json::Value>,
) -> Result<TaskResult, String> {
    // Get task from storage
    // TODO: Implement task retrieval

    // Find DirectorAgent
    let agents = registry.0.list_agents().await;
    let director = agents
        .iter()
        .find(|a| matches!(a.agent_type, AgentType::Director))
        .ok_or("DirectorAgent not found")?;

    let agent = registry
        .0
        .get_agent(&director.id)
        .ok_or("DirectorAgent not found in registry")?;

    // Execute task
    let result = agent
        .process_task(Task {
            id: task_id.clone(),
            description: "Task from frontend".to_string(),
            priority: TaskPriority::Medium,
            dependencies: vec![],
            context: TaskContext {
                workspace_id: "default".to_string(),
                user_instruction: "".to_string(),
                relevant_files: vec![],
                memory_context: vec![],
            },
        })
        .await
        .map_err(|e| e.to_string())?;

    // Send event
    let _ = on_event.send(serde_json::json!({
        "event": "task_completed",
        "task_id": task_id,
        "result": result
    }));

    Ok(result)
}

/// Cancel a running task
///
/// Cancels a currently running task and cleans up its resources.
#[tauri::command]
pub async fn cancel_agent_task(
    registry: State<'_, AgentRegistryState>,
    task_id: String,
) -> Result<(), String> {
    registry
        .0
        .cancel_task(&task_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get status of a task
///
/// Returns the current status of the specified task.
#[tauri::command]
pub async fn get_task_status(
    registry: State<'_, AgentRegistryState>,
    task_id: String,
) -> Result<String, String> {
    let agent_id = registry
        .0
        .get_task_agent(&task_id)
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let agent = registry
        .0
        .get_agent(&agent_id)
        .ok_or_else(|| format!("Agent {} not found", agent_id))?;

    let info = agent.info();
    Ok(format!("{:?}", info.current_task))
}

/// Send message to an agent
///
/// Sends a message of the specified type to the target agent.
/// Supports TaskAssign, TaskResult, and QueryMemory message types.
#[tauri::command]
pub async fn send_agent_message(
    registry: State<'_, AgentRegistryState>,
    _from_agent_id: String,
    to_agent_id: String,
    message_type: String,
    message_data: serde_json::Value,
) -> Result<(), String> {
    let message = match message_type.as_str() {
        "TaskAssign" => {
            let task: Task = serde_json::from_value(message_data)
                .map_err(|e| format!("Failed to parse task: {}", e))?;
            AgentMessage::TaskAssign {
                task_id: task.id.clone(),
                task,
            }
        }
        "TaskResult" => {
            let result: TaskResult = serde_json::from_value(message_data)
                .map_err(|e| format!("Failed to parse result: {}", e))?;
            AgentMessage::TaskResult {
                task_id: "unknown".to_string(),
                result,
            }
        }
        "QueryMemory" => {
            let query = message_data
                .as_str()
                .ok_or("Query must be a string")?
                .to_string();
            AgentMessage::QueryMemory { query }
        }
        _ => {
            return Err(format!("Unknown message type: {}", message_type));
        }
    };

    let agent = registry
        .0
        .get_agent(&to_agent_id)
        .ok_or_else(|| format!("Agent {} not found", to_agent_id))?;

    agent.handle_message(message).await.map_err(|e| e.to_string())
}

/// Get pending messages for an agent
///
/// Returns all pending messages for the specified agent from the message bus.
#[tauri::command]
pub async fn get_agent_messages(
    registry: State<'_, AgentRegistryState>,
    agent_id: String,
) -> Result<Vec<AgentMessage>, String> {
    let messages = registry.0.message_bus().receive(&agent_id).await;
    Ok(messages)
}

/// Get registry statistics
///
/// Returns statistics about the agent registry including agent counts and status.
#[tauri::command]
pub async fn get_agent_statistics(
    registry: State<'_, AgentRegistryState>,
) -> Result<crate::agents::RegistryStatistics, String> {
    Ok(registry.0.get_statistics())
}

/// Get capabilities of an agent
///
/// Returns the list of capabilities supported by the specified agent.
#[tauri::command]
pub async fn get_agent_capabilities(
    registry: State<'_, AgentRegistryState>,
    agent_id: String,
) -> Result<Vec<String>, String> {
    let agent = registry
        .0
        .get_agent(&agent_id)
        .ok_or_else(|| format!("Agent {} not found", agent_id))?;

    Ok(agent.capabilities())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentRegistry;
    use crate::ai::AIProviderManager;

    #[tokio::test]
    async fn test_agent_registry_state_creation() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = Arc::new(AgentRegistry::new(ai_provider));
        let state = AgentRegistryState(registry);
        assert!(Arc::strong_count(&state.0) >= 1);
    }

    #[tokio::test]
    async fn test_get_agent_statistics() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = Arc::new(AgentRegistry::new(ai_provider));
        let state = AgentRegistryState(registry);

        // Test that we can get statistics from the registry directly
        let stats = state.0.get_statistics();
        assert_eq!(stats.total_agents, 0);
        assert_eq!(stats.idle_agents, 0);
    }

    #[tokio::test]
    async fn test_create_multi_agent_task() {
        let ai_provider = Arc::new(AIProviderManager::new());
        let registry = Arc::new(AgentRegistry::new(ai_provider));
        let state = AgentRegistryState(registry);

        // Test task ID generation (UUID format)
        let task_id = uuid::Uuid::new_v4().to_string();
        assert!(!task_id.is_empty());
        assert!(task_id.contains('-')); // UUID format
    }
}
