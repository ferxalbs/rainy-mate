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
    task.workspace_path = workspace_path.clone();

    // If no workspace context is set but we have a path, create an ad-hoc workspace context
    if task_manager.get_workspace().await.is_none() {
        if let Some(path) = &workspace_path {
            let name = std::path::Path::new(path)
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("Ad-hoc Workspace"))
                .to_string_lossy()
                .to_string();

            let workspace = crate::services::workspace::Workspace {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                allowed_paths: vec![path.clone()],
                permissions: crate::services::workspace::WorkspacePermissions {
                    can_read: true,
                    can_write: true,
                    can_execute: true,
                    can_delete: true,
                    can_create_agents: true,
                },
                permission_overrides: vec![],
                agents: vec![],
                memory: crate::services::workspace::WorkspaceMemory {
                    max_size: 1000000,
                    current_size: 0,
                    retention_policy: "30d".to_string(),
                },
                settings: crate::services::workspace::WorkspaceSettings {
                    theme: "system".to_string(),
                    language: "en".to_string(),
                    auto_save: true,
                    notifications_enabled: true,
                },
            };

            task_manager.set_workspace(workspace).await;
            tracing::info!(
                "Auto-initialized ad-hoc workspace context for path: {}",
                path
            );
        }
    }

    task_manager.add_task_validated(task.clone()).await?;

    Ok(task)
}

/// Set workspace context for task manager
#[tauri::command]
pub async fn set_task_manager_workspace(
    workspace_id: String,
    folder_manager: State<'_, crate::services::FolderManager>,
    task_manager: State<'_, TaskManager>,
) -> Result<(), String> {
    let folder = folder_manager
        .get_folder(&workspace_id)
        .await
        .ok_or_else(|| format!("Folder not found: {}", workspace_id))?;

    // Convert folder to workspace for task manager
    let workspace = crate::services::workspace::Workspace {
        id: folder.id.clone(),
        name: folder.name,
        allowed_paths: vec![folder.path.clone()],
        permissions: crate::services::workspace::WorkspacePermissions {
            can_read: true,
            can_write: true,
            can_execute: true,
            can_delete: true,
            can_create_agents: true,
        },
        permission_overrides: vec![],
        agents: vec![],
        memory: crate::services::workspace::WorkspaceMemory {
            max_size: 1000000, // 1MB default
            current_size: 0,
            retention_policy: "30d".to_string(),
        },
        settings: crate::services::workspace::WorkspaceSettings {
            theme: "dark".to_string(),
            language: "en".to_string(),
            auto_save: true,
            notifications_enabled: true,
        },
    };

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

/// Save task queue state to disk
#[tauri::command]
pub async fn save_task_queue_state(
    path: String,
    task_manager: State<'_, crate::services::TaskManager>,
) -> Result<(), String> {
    task_manager
        .save_state(&std::path::PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())
}

/// Load task queue state from disk
#[tauri::command]
pub async fn load_task_queue_state(
    path: String,
    task_manager: State<'_, crate::services::TaskManager>,
) -> Result<(), String> {
    task_manager
        .load_state(&std::path::PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())
}

/// Start background task processing
#[tauri::command]
pub async fn start_background_task_processing(
    task_manager: State<'_, crate::services::TaskManager>,
) -> Result<(), String> {
    task_manager.start_background_processing().await;
    Ok(())
}
