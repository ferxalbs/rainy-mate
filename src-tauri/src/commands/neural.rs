use crate::models::neural::{DesktopNodeStatus, QueuedCommand};
use crate::services::tool_manifest::build_skill_manifest_from_runtime;
use crate::services::NeuralService;
use tauri::{command, Manager, State};

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
    allowed_paths: Vec<String>,
) -> Result<String, String> {
    let skills = build_skill_manifest_from_runtime()?;
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

#[command]
pub async fn resume_neural_runtime(
    app_handle: tauri::AppHandle,
    command_poller: State<'_, std::sync::Arc<crate::services::CommandPoller>>,
) -> Result<(), String> {
    command_poller.start().await;

    if let Some(bridge) = app_handle.try_state::<crate::services::cloud_bridge::CloudBridge>() {
        let bridge_ref: &crate::services::cloud_bridge::CloudBridge = bridge.inner();
        bridge_ref.restart().await;
    }

    Ok(())
}

/// Classify a neural connection error string into a user-facing message.
/// Centralises error classification in Rust, keeping the frontend purely display-only.
#[command]
pub fn classify_neural_error(error: String) -> String {
    let lower = error.to_lowercase();

    if lower.contains("duplicate workspace mapping") {
        return "ATM has duplicate workspaces for this Platform Key. Reset/clean duplicate workspace records, then reconnect.".to_string();
    }

    if lower.contains("owner credentials mismatch")
        || lower.contains("invalid credentials")
        || lower.contains("validation failed")
    {
        return "Platform Key / Creator API Key are invalid for this ATM instance.".to_string();
    }

    if lower.contains("platformkey format")
        || lower.contains("apikey format")
        || lower.contains("rainy api key validation failed")
        || lower.contains("missing required checks")
    {
        // Pass the original error through — it already contains actionable detail.
        return error;
    }

    if lower.contains("db_not_ready")
        || lower.contains("node_register_transient")
        || lower.contains("service warming up")
    {
        return "Rainy ATM is still warming up after deploy. Wait a few seconds and retry."
            .to_string();
    }

    "Connection failed. Please check your credentials.".to_string()
}

/// Retrieve the stored neural workspace ID from the keychain.
/// Returns `None` if no workspace has been persisted yet.
#[command]
pub async fn get_neural_workspace_id(
    state: State<'_, NeuralServiceState>,
) -> Result<Option<String>, String> {
    Ok(state.0.get_workspace_id().await)
}
