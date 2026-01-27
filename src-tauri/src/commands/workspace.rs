// Rainy Cowork - Workspace Commands
// Tauri commands for advanced workspace management

use crate::services::{PermissionOverride, Workspace, WorkspaceManager, WorkspacePermissions};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

/// Create a new workspace
#[tauri::command]
pub async fn create_workspace(
    name: String,
    allowed_paths: Vec<String>,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<Workspace, String> {
    workspace_manager
        .create_workspace(name, allowed_paths)
        .map_err(|e| e.to_string())
}

/// Load a workspace by ID
#[tauri::command]
pub async fn load_workspace(
    id: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<Workspace, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    workspace_manager
        .load_workspace(&uuid)
        .map_err(|e| e.to_string())
}

/// Save a workspace
#[tauri::command]
pub async fn save_workspace(
    workspace: Workspace,
    format: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<(), String> {
    let config_format = match format.as_str() {
        "json" => crate::services::ConfigFormat::Json,
        "toml" => crate::services::ConfigFormat::Toml,
        _ => return Err("Invalid format. Use 'json' or 'toml'".to_string()),
    };

    workspace_manager
        .save_workspace(&workspace, config_format)
        .map_err(|e| e.to_string())
}

/// List all workspace IDs
#[tauri::command]
pub async fn list_workspaces(
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<Vec<String>, String> {
    let ids = workspace_manager
        .list_workspaces()
        .map_err(|e| e.to_string())?;

    Ok(ids.iter().map(|id| id.to_string()).collect())
}

/// Delete a workspace by ID
#[tauri::command]
pub async fn delete_workspace(
    id: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    workspace_manager
        .delete_workspace(&uuid)
        .map_err(|e| e.to_string())
}

/// Add a permission override for a specific path
#[tauri::command]
pub async fn add_permission_override(
    workspace_id: String,
    path: String,
    permissions: WorkspacePermissions,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let mut workspace = workspace_manager
        .load_workspace(&uuid)
        .map_err(|e| format!("Failed to load workspace: {}", e))?;

    workspace_manager
        .add_permission_override(&mut workspace, path, permissions)
        .map_err(|e| e.to_string())?;

    // Save updated workspace
    workspace_manager
        .save_workspace(&workspace, crate::services::ConfigFormat::Json)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Remove a permission override for a specific path
#[tauri::command]
pub async fn remove_permission_override(
    workspace_id: String,
    path: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let mut workspace = workspace_manager
        .load_workspace(&uuid)
        .map_err(|e| format!("Failed to load workspace: {}", e))?;

    workspace_manager
        .remove_permission_override(&mut workspace, &path)
        .map_err(|e| e.to_string())?;

    // Save updated workspace
    workspace_manager
        .save_workspace(&workspace, crate::services::ConfigFormat::Json)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get all permission overrides for a workspace
#[tauri::command]
pub async fn get_permission_overrides(
    workspace_id: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<Vec<PermissionOverride>, String> {
    let uuid = Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let workspace = workspace_manager
        .load_workspace(&uuid)
        .map_err(|e| format!("Failed to load workspace: {}", e))?;

    Ok(workspace_manager.get_permission_overrides(&workspace))
}

/// Get effective permissions for a specific path
#[tauri::command]
pub async fn get_effective_permissions(
    workspace_id: String,
    path: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<WorkspacePermissions, String> {
    let uuid = Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let workspace = workspace_manager
        .load_workspace(&uuid)
        .map_err(|e| format!("Failed to load workspace: {}", e))?;

    Ok(workspace_manager.get_effective_permissions(&workspace, &path))
}

/// Get all available workspace templates
#[tauri::command]
pub async fn get_workspace_templates(
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<Vec<crate::services::WorkspaceTemplate>, String> {
    workspace_manager
        .get_templates()
        .map_err(|e| e.to_string())
}

/// Create a workspace from a template
#[tauri::command]
pub async fn create_workspace_from_template(
    template_id: String,
    name: String,
    custom_paths: Option<Vec<String>>,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<Workspace, String> {
    workspace_manager
        .create_from_template(&template_id, name, custom_paths)
        .map_err(|e| e.to_string())
}

/// Save a custom workspace template
#[tauri::command]
pub async fn save_workspace_template(
    template: crate::services::WorkspaceTemplate,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<(), String> {
    workspace_manager
        .save_template(&template)
        .map_err(|e| e.to_string())
}

/// Delete a workspace template
#[tauri::command]
pub async fn delete_workspace_template(
    template_id: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<(), String> {
    workspace_manager
        .delete_template(&template_id)
        .map_err(|e| e.to_string())
}

/// Get analytics for a workspace
#[tauri::command]
pub async fn get_workspace_analytics(
    workspace_id: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<crate::services::WorkspaceAnalytics, String> {
    let uuid = Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid UUID: {}", e))?;
    workspace_manager
        .get_analytics(&uuid)
        .map_err(|e| e.to_string())
}