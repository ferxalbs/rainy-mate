// Rainy Cowork - Task Commands
// Tauri commands for task management

use crate::models::{ProviderType, Task, TaskEvent};
use crate::services::TaskManager;
use tauri::{ipc::Channel, State};

/// Create a new task with workspace validation
#[tauri::command]
pub async fn create_task(
    description: String,
    provider: ProviderType,
    model: String,
    workspace_path: Option<String>,
    task_manager: State<'_, TaskManager>,
) -> Result<Task, String> {
    let mut task = Task::new(description, provider, model);
    task.workspace_path = workspace_path;

    task_manager.add_task_validated(task.clone()).await?;

    Ok(task)
}

/// Set workspace context for task manager
#[tauri::command]
pub async fn set_task_manager_workspace(
    workspace_id: String,
    workspace_manager: State<'_, crate::services::WorkspaceManager>,
    task_manager: State<'_, TaskManager>,
) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&workspace_id)
        .map_err(|e| format!("Invalid workspace ID: {}", e))?;

    let workspace = workspace_manager
        .load_workspace(&uuid)
        .map_err(|e| format!("Failed to load workspace: {}", e))?;

    task_manager.set_workspace(workspace).await;
    tracing::info!("Workspace context set for task manager: {}", workspace_id);
    Ok(())
}

/// Execute a task with progress reporting
#[tauri::command]
pub async fn execute_task(
    task_id: String,
    on_event: Channel<TaskEvent>,
    task_manager: State<'_, TaskManager>,
) -> Result<(), String> {
    task_manager.execute_task(&task_id, on_event).await
}

/// Pause a running task
#[tauri::command]
pub async fn pause_task(
    task_id: String,
    task_manager: State<'_, TaskManager>,
) -> Result<(), String> {
    task_manager.pause_task(&task_id).await
}

/// Resume a paused task
#[tauri::command]
pub async fn resume_task(
    task_id: String,
    task_manager: State<'_, TaskManager>,
) -> Result<(), String> {
    task_manager.resume_task(&task_id).await
}

/// Cancel a task
#[tauri::command]
pub async fn cancel_task(
    task_id: String,
    task_manager: State<'_, TaskManager>,
) -> Result<(), String> {
    task_manager.cancel_task(&task_id).await
}

/// Get current task status
#[tauri::command]
pub async fn get_task(
    task_id: String,
    task_manager: State<'_, TaskManager>,
) -> Result<Option<Task>, String> {
    Ok(task_manager.get_task(&task_id).await)
}

/// List all tasks
#[tauri::command]
pub async fn list_tasks(task_manager: State<'_, TaskManager>) -> Result<Vec<Task>, String> {
    Ok(task_manager.list_tasks().await)
}
