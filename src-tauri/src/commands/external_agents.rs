use crate::services::{
    ExternalAgentRuntime, ExternalAgentSession, ExternalRuntimeAvailability, ExternalRuntimeKind,
    NewExternalAgentSession,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn create_external_agent_session(
    runtime_kind: String,
    workspace_path: String,
    task_summary: String,
    external_runtime: State<'_, Arc<ExternalAgentRuntime>>,
) -> Result<ExternalAgentSession, String> {
    let runtime_kind = ExternalRuntimeKind::from_str(&runtime_kind)
        .ok_or_else(|| format!("Unknown external runtime '{}'", runtime_kind))?;
    external_runtime
        .create_session(NewExternalAgentSession {
            runtime_kind,
            workspace_path,
            task_summary,
        })
        .await
}

#[tauri::command]
pub async fn send_external_agent_input(
    session_id: String,
    message: String,
    external_runtime: State<'_, Arc<ExternalAgentRuntime>>,
) -> Result<ExternalAgentSession, String> {
    external_runtime.send_message(&session_id, message).await
}

#[tauri::command]
pub async fn get_external_agent_session(
    session_id: String,
    external_runtime: State<'_, Arc<ExternalAgentRuntime>>,
) -> Result<ExternalAgentSession, String> {
    external_runtime.get_session(&session_id).await
}

#[tauri::command]
pub async fn list_external_agent_sessions(
    workspace_path: Option<String>,
    external_runtime: State<'_, Arc<ExternalAgentRuntime>>,
) -> Result<Vec<ExternalAgentSession>, String> {
    external_runtime.list_sessions(workspace_path.as_deref()).await
}

#[tauri::command]
pub async fn wait_external_agent_session(
    session_id: String,
    timeout_ms: Option<u64>,
    external_runtime: State<'_, Arc<ExternalAgentRuntime>>,
) -> Result<ExternalAgentSession, String> {
    external_runtime
        .wait_for_session(&session_id, timeout_ms)
        .await
}

#[tauri::command]
pub async fn cancel_external_agent_session(
    session_id: String,
    external_runtime: State<'_, Arc<ExternalAgentRuntime>>,
) -> Result<ExternalAgentSession, String> {
    external_runtime.cancel_session(&session_id).await
}

#[tauri::command]
pub async fn get_external_agent_runtime_availability(
    external_runtime: State<'_, Arc<ExternalAgentRuntime>>,
) -> Result<Vec<ExternalRuntimeAvailability>, String> {
    Ok(external_runtime.list_runtime_availability().await)
}
