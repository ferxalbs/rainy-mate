use crate::models::neural::{
    AirlockLevel, CommandPriority, CommandResult, CommandStatus, QueuedCommand, RainyPayload,
};
use crate::services::SkillExecutor;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

/// Execute a skill directly from the frontend (local Deep Mode execution).
/// This bypasses the cloud command queue and executes file operations locally.
///
/// # Parameters
/// - `workspace_id`: The workspace ID (used for logging/tracking)
/// - `skill`: The skill name (e.g., "filesystem")
/// - `method`: The method name (e.g., "write_file", "read_file")
/// - `params`: JSON parameters for the method
/// - `workspace_path`: The actual filesystem path where files should be created (e.g., "/Users/fer/Projects/myproject")
#[tauri::command]
pub async fn execute_skill(
    skill_executor: State<'_, Arc<SkillExecutor>>,
    workspace_id: String,
    skill: String,
    method: String,
    params: serde_json::Value,
    workspace_path: Option<String>,
) -> Result<CommandResult, String> {
    // For local execution, use the workspace_path as the allowed path
    // This enables relative path resolution in SkillExecutor
    let allowed_paths = match workspace_path {
        Some(path) => vec![path],
        None => vec![], // Will fall back to workspace config lookup
    };

    // Construct a pseudo-command to reuse the existing SkillExecutor logic
    let command = QueuedCommand {
        id: Uuid::new_v4().to_string(),
        workspace_id: Some(workspace_id),
        desktop_node_id: Some("desktop-local".to_string()),
        intent: format!("{}.{}", skill, method),
        payload: RainyPayload {
            skill: Some(skill),
            method: Some(method),
            params: Some(params),
            content: None,
            allowed_paths, // Pass the workspace path for local path resolution
            tool_access_policy: None,
            tool_access_policy_version: None,
            tool_access_policy_hash: None,
        },
        priority: CommandPriority::Normal,
        status: CommandStatus::Pending,
        airlock_level: AirlockLevel::Safe, // Assumed safe since triggered by user via UI
        approved_by: Some("user".to_string()),
        result: None,
        created_at: Some(chrono::Utc::now().timestamp()),
        started_at: Some(chrono::Utc::now().timestamp()),
        completed_at: None,
    };

    // Execute directly via the shared SkillExecutor
    let result = skill_executor.execute(&command).await;

    Ok(result)
}
