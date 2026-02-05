// Rainy Cowork - File Commands
// Tauri commands for file system operations with versioning

use crate::models::{FileChange, Workspace};
use crate::services::FileManager;
use std::sync::Arc;
use tauri::State;

/// Select a workspace folder using native dialog
#[tauri::command]
pub async fn select_workspace() -> Result<Option<Workspace>, String> {
    // This will be called from the frontend using the dialog plugin
    // The actual dialog is handled by the frontend
    Ok(None)
}

/// Set the active workspace
#[tauri::command]
pub async fn set_workspace(
    path: String,
    name: String,
    file_manager: State<'_, Arc<FileManager>>,
) -> Result<Workspace, String> {
    file_manager.set_workspace(path, name).await
}

/// Get current workspace
#[tauri::command]
pub async fn get_workspace(
    file_manager: State<'_, Arc<FileManager>>,
) -> Result<Option<Workspace>, String> {
    Ok(file_manager.get_workspace().await)
}

/// List contents of a directory
#[tauri::command]
pub async fn list_directory(
    path: String,
    file_manager: State<'_, Arc<FileManager>>,
) -> Result<Vec<FileEntry>, String> {
    file_manager.list_directory(&path).await
}

/// Read file content
#[tauri::command]
pub async fn read_file(
    path: String,
    file_manager: State<'_, Arc<FileManager>>,
) -> Result<String, String> {
    file_manager.read_file(&path).await
}

/// Write file with automatic versioning
#[tauri::command]
pub async fn write_file(
    path: String,
    content: String,
    task_id: Option<String>,
    file_manager: State<'_, Arc<FileManager>>,
) -> Result<FileChange, String> {
    file_manager.write_file(&path, &content, task_id).await
}

/// Append content to file with automatic versioning
#[tauri::command]
pub async fn append_file(
    path: String,
    content: String,
    task_id: Option<String>,
    file_manager: State<'_, Arc<FileManager>>,
) -> Result<FileChange, String> {
    file_manager.append_file(&path, &content, task_id).await
}

/// Create a version snapshot before modification
#[tauri::command]
pub async fn create_snapshot(
    path: String,
    task_id: String,
    file_manager: State<'_, Arc<FileManager>>,
) -> Result<String, String> {
    file_manager.create_snapshot(&path, &task_id).await
}

/// Rollback a file to a previous version
#[tauri::command]
pub async fn rollback_file(
    version_id: String,
    file_manager: State<'_, Arc<FileManager>>,
) -> Result<(), String> {
    file_manager.rollback(&version_id).await
}

/// List file changes for a task
#[tauri::command]
pub async fn list_file_changes(
    task_id: Option<String>,
    file_manager: State<'_, Arc<FileManager>>,
) -> Result<Vec<FileChange>, String> {
    file_manager.list_changes(task_id).await
}

/// File entry for directory listing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub modified: Option<String>,
}
