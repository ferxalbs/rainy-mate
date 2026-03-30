// Rainy MaTE - Workspace Commands
// Tauri commands for advanced workspace management

use crate::services::{
    EffectiveLocalAgentPolicy, LocalAgentSecurityService, MateLaunchpadService,
    PermissionOverride, SettingsManager, Workspace, WorkspaceLaunchpadSummary, WorkspaceManager,
    WorkspacePermissions,
};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

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
    workspace_manager
        .load_workspace(&id)
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
    workspace_manager
        .list_workspaces()
        .map_err(|e| e.to_string())
}

/// Delete a workspace by ID
#[tauri::command]
pub async fn delete_workspace(
    id: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<(), String> {
    workspace_manager
        .delete_workspace(&id)
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
    let mut workspace = workspace_manager
        .load_workspace(&workspace_id)
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
    let mut workspace = workspace_manager
        .load_workspace(&workspace_id)
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
    let workspace = workspace_manager
        .load_workspace(&workspace_id)
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
    let workspace = workspace_manager
        .load_workspace(&workspace_id)
        .map_err(|e| format!("Failed to load workspace: {}", e))?;

    Ok(workspace_manager.get_effective_permissions(&workspace, &path))
}

#[tauri::command]
pub async fn get_effective_local_agent_policy(
    workspace_id: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<EffectiveLocalAgentPolicy, String> {
    let settings = settings.lock().await;
    Ok(LocalAgentSecurityService::resolve(
        &workspace_manager.inner().clone(),
        &settings,
        &workspace_id,
        None,
    ))
}

/// Get all available workspace templates
#[tauri::command]
pub async fn get_workspace_templates(
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<Vec<crate::services::WorkspaceTemplate>, String> {
    workspace_manager.get_templates().map_err(|e| e.to_string())
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
    workspace_manager
        .get_analytics(&workspace_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mate_pack_definitions(
) -> Result<Vec<crate::services::MatePackDefinition>, String> {
    Ok(MateLaunchpadService::pack_definitions())
}

#[tauri::command]
pub async fn list_first_run_scenarios(
) -> Result<Vec<crate::services::FirstRunScenarioDefinition>, String> {
    Ok(MateLaunchpadService::first_run_scenarios())
}

#[tauri::command]
pub async fn get_workspace_launchpad(
    workspace_path: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<WorkspaceLaunchpadSummary, String> {
    let workspace = workspace_manager
        .ensure_workspace_for_path(&workspace_path)
        .map_err(|e| e.to_string())?;
    Ok(MateLaunchpadService::get_workspace_summary(&workspace))
}

#[tauri::command]
pub async fn update_workspace_launch_config(
    workspace_path: String,
    trust_preset: String,
    enabled_pack_ids: Vec<String>,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<WorkspaceLaunchpadSummary, String> {
    let workspace = workspace_manager
        .ensure_workspace_for_path(&workspace_path)
        .map_err(|e| e.to_string())?;
    MateLaunchpadService::update_workspace_launch_config(
        workspace_manager.inner(),
        &workspace.id,
        &trust_preset,
        &enabled_pack_ids,
    )
}

#[tauri::command]
pub async fn build_workspace_first_run_prompt(
    workspace_path: String,
    scenario_id: String,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<String, String> {
    let workspace = workspace_manager
        .ensure_workspace_for_path(&workspace_path)
        .map_err(|e| e.to_string())?;
    MateLaunchpadService::build_first_run_prompt(&workspace, &scenario_id)
}

#[tauri::command]
pub async fn record_workspace_launch_result(
    workspace_path: String,
    scenario_id: String,
    chat_id: Option<String>,
    success: bool,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
) -> Result<WorkspaceLaunchpadSummary, String> {
    let workspace = workspace_manager
        .ensure_workspace_for_path(&workspace_path)
        .map_err(|e| e.to_string())?;
    MateLaunchpadService::record_workspace_launch(
        workspace_manager.inner(),
        &workspace.id,
        &scenario_id,
        chat_id.as_deref(),
        success,
    )
}
