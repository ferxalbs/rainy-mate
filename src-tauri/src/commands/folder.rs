// Rainy Cowork - Folder Commands
// Tauri commands for user folder management

use crate::models::folder::{FolderAccess, UserFolder};
use crate::services::FolderManager;
use tauri::State;

/// Add a new user folder
#[tauri::command]
pub async fn add_user_folder(
    path: String,
    name: String,
    folder_manager: State<'_, FolderManager>,
) -> Result<UserFolder, String> {
    folder_manager
        .add_folder(path, name, FolderAccess::FullAccess)
        .await
}

/// List all user folders
#[tauri::command]
pub async fn list_user_folders(
    folder_manager: State<'_, FolderManager>,
) -> Result<Vec<UserFolder>, String> {
    Ok(folder_manager.list_folders().await)
}

/// Remove a user folder by ID
#[tauri::command]
pub async fn remove_user_folder(
    id: String,
    folder_manager: State<'_, FolderManager>,
) -> Result<(), String> {
    folder_manager.remove_folder(&id).await
}
