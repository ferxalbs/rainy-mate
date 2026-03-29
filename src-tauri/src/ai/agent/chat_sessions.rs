// Chat Session Tauri Command Wrappers
// Thin Tauri command layer extracted from manager.rs to keep module size bounded.
// All real logic lives in AgentManager methods.
use crate::ai::agent::manager::{
    AgentEntity, AgentManager, ChatCompactionStateDto, ChatHistoryWindowDto,
    ChatRuntimeTelemetryDto, DEFAULT_LONG_CHAT_SCOPE_ID,
};
use crate::ai::specs::manifest::AgentSpec;
use crate::services::chat_artifacts::ChatArtifact;
use tauri::State;

const DEFAULT_WORKSPACE_ID: &str = "default";

#[tauri::command]
pub async fn save_agent_to_db(
    state: State<'_, AgentManager>,
    id: String,
    name: String,
    description: Option<String>,
    soul: Option<String>,
) -> Result<String, String> {
    use crate::ai::specs::skills::AgentSkills;
    use crate::ai::specs::soul::AgentSoul;

    let spec = AgentSpec {
        id,
        version: "3.0.0".to_string(),
        soul: AgentSoul {
            name,
            description: description.unwrap_or_default(),
            soul_content: soul.unwrap_or_default(),
            ..Default::default()
        },
        skills: AgentSkills::default(),
        airlock: Default::default(),
        memory_config: Default::default(),
        connectors: Default::default(),
        runtime: Default::default(),
        model: None,
        temperature: None,
        max_tokens: None,
        provider: None,
        signature: None,
    };

    state.create_agent(&spec).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_agents_from_db(
    state: State<'_, AgentManager>,
) -> Result<Vec<AgentEntity>, String> {
    state.list_agents().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_chat_message(
    state: State<'_, AgentManager>,
    chat_id: String,
    role: String,
    content: String,
    artifacts: Option<Vec<ChatArtifact>>,
) -> Result<String, String> {
    state
        .save_message_with_artifacts(&chat_id, &role, &content, artifacts.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_chat_history(
    state: State<'_, AgentManager>,
    chat_id: String,
) -> Result<Vec<(String, String, String)>, String> {
    state.get_history(&chat_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_chat_history(
    state: State<'_, AgentManager>,
    chat_id: String,
) -> Result<(), String> {
    state
        .clear_history(&chat_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn compact_session_cmd(
    state: State<'_, AgentManager>,
    chat_id: String,
    summary_content: String,
    keep_recent_count: usize,
) -> Result<(), String> {
    state
        .compact_session(&chat_id, &summary_content, keep_recent_count)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_chat_compaction_state(
    state: State<'_, AgentManager>,
    chat_scope_id: String,
) -> Result<Option<ChatCompactionStateDto>, String> {
    state
        .get_latest_chat_compaction(&chat_scope_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_chat_runtime_telemetry(
    state: State<'_, AgentManager>,
    chat_scope_id: String,
) -> Result<Option<ChatRuntimeTelemetryDto>, String> {
    state
        .get_chat_runtime_telemetry(&chat_scope_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_default_chat_scope() -> Result<String, String> {
    Ok(DEFAULT_LONG_CHAT_SCOPE_ID.to_string())
}

#[tauri::command]
pub async fn get_or_create_workspace_chat(
    state: State<'_, AgentManager>,
    workspace_id: String,
) -> Result<String, String> {
    let ws = if workspace_id.trim().is_empty() {
        DEFAULT_WORKSPACE_ID.to_string()
    } else {
        workspace_id
    };

    // Try to find the latest chat for this workspace
    if let Some(chat) = state
        .get_latest_workspace_chat(&ws)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(chat.id);
    }

    // No existing chat — create one
    let chat = state
        .create_chat_session(&ws)
        .await
        .map_err(|e| e.to_string())?;
    Ok(chat.id)
}

#[tauri::command]
pub async fn get_chat_history_window(
    state: State<'_, AgentManager>,
    chat_scope_id: String,
    cursor_rowid: Option<i64>,
    limit: Option<usize>,
) -> Result<ChatHistoryWindowDto, String> {
    state
        .get_history_window(&chat_scope_id, cursor_rowid, limit.unwrap_or(100))
        .await
        .map_err(|e| e.to_string())
}
