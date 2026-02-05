use crate::models::neural::{
    AirlockLevel, CommandPriority, CommandResult, CommandStatus, QueuedCommand, RainyPayload,
};
use crate::services::SkillExecutor;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn execute_skill(
    skill_executor: State<'_, Arc<SkillExecutor>>,
    workspace_id: String,
    skill: String,
    method: String,
    params: serde_json::Value,
) -> Result<CommandResult, String> {
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
            allowed_paths: vec![], // Local execution relies on workspace config, not cloud overrides
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
