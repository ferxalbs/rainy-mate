use crate::models::neural::{DesktopNodeStatus, QueuedCommand, SkillManifest};
use crate::services::NeuralService;
use tauri::{command, State};

pub struct NeuralServiceState(pub NeuralService);

#[command]
pub async fn set_neural_workspace_id(
    state: State<'_, NeuralServiceState>,
    workspace_id: String,
) -> Result<(), String> {
    state.0.set_workspace_id(workspace_id).await;
    Ok(())
}

#[command]
pub async fn register_node(
    state: State<'_, NeuralServiceState>,
    skills: Vec<SkillManifest>,
    allowed_paths: Vec<String>,
) -> Result<String, String> {
    state.0.register(skills, allowed_paths).await
}

#[command]
pub async fn send_heartbeat(
    state: State<'_, NeuralServiceState>,
    status: DesktopNodeStatus,
) -> Result<Vec<QueuedCommand>, String> {
    state.0.heartbeat(status).await
}

#[command]
pub async fn poll_commands(
    state: State<'_, NeuralServiceState>,
) -> Result<Vec<QueuedCommand>, String> {
    state.0.poll_commands().await
}

#[command]
pub async fn start_command_execution(
    state: State<'_, NeuralServiceState>,
    command_id: String,
) -> Result<(), String> {
    state.0.start_command(&command_id).await
}

#[command]
pub async fn complete_command_execution(
    state: State<'_, NeuralServiceState>,
    command_id: String,
    result: crate::models::neural::CommandResult,
) -> Result<(), String> {
    state.0.complete_command(&command_id, result).await
}

#[command]
pub async fn set_neural_credentials(
    state: State<'_, NeuralServiceState>,
    platform_key: String,
    user_api_key: String,
) -> Result<(), String> {
    state.0.set_credentials(platform_key, user_api_key).await
}

#[command]
pub async fn clear_neural_credentials(state: State<'_, NeuralServiceState>) -> Result<(), String> {
    state.0.clear_credentials().await
}

#[command]
pub async fn load_neural_credentials(state: State<'_, NeuralServiceState>) -> Result<bool, String> {
    state.0.load_credentials_from_keychain().await
}

#[command]
pub async fn has_neural_credentials(state: State<'_, NeuralServiceState>) -> Result<bool, String> {
    Ok(state.0.has_credentials().await)
}

#[command]
pub async fn get_neural_credentials_values(
    state: State<'_, NeuralServiceState>,
) -> Result<Option<(String, String)>, String> {
    Ok(state.0.get_credentials().await)
}
