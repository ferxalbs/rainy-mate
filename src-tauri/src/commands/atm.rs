use crate::services::atm_client::{ATMClient, CreateAgentParams};
use tauri::{command, State};

#[command]
pub async fn bootstrap_atm(
    client: State<'_, ATMClient>,
    neural: State<'_, crate::commands::neural::NeuralServiceState>,
    master_key: String,
    user_api_key: String,
    name: String,
) -> Result<crate::services::atm_client::WorkspaceAuth, String> {
    let auth = client
        .bootstrap(master_key.clone(), user_api_key.clone(), name)
        .await?;
    // Automatically set credentials in client
    client.set_credentials(auth.api_key.clone()).await;
    // Keep desktop node bridge aligned with workspace and keys for auto-connect.
    neural.0.set_workspace_id(auth.id.clone()).await;
    neural
        .0
        .set_credentials(master_key, user_api_key)
        .await
        .map_err(|e| format!("Failed to set neural credentials: {}", e))?;
    Ok(auth)
}

#[command]
pub async fn create_atm_agent(
    client: State<'_, ATMClient>,
    name: String,
    agent_type: String,
    config: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let params = CreateAgentParams {
        name,
        type_: agent_type,
        config,
    };
    client.create_agent(params).await
}

#[command]
pub async fn list_atm_agents(client: State<'_, ATMClient>) -> Result<serde_json::Value, String> {
    client.list_agents().await
}

#[command]
pub async fn list_atm_commands(
    client: State<'_, ATMClient>,
    limit: Option<usize>,
    status: Option<String>,
) -> Result<Vec<crate::services::atm_client::ATMCommandSummary>, String> {
    client.list_commands(limit, status).await
}

#[command]
pub async fn get_atm_command_details(
    client: State<'_, ATMClient>,
    command_id: String,
    progress_limit: Option<usize>,
) -> Result<crate::services::atm_client::CommandDetailsResponse, String> {
    client.get_command_details(command_id, progress_limit).await
}

#[command]
pub async fn get_atm_command_progress(
    client: State<'_, ATMClient>,
    command_id: String,
    since: Option<i64>,
    limit: Option<usize>,
) -> Result<crate::services::atm_client::CommandProgressResponse, String> {
    client.get_command_progress(command_id, since, limit).await
}

#[command]
pub async fn get_atm_command_metrics(
    client: State<'_, ATMClient>,
    command_id: String,
) -> Result<crate::services::atm_client::CommandMetricsResponse, String> {
    client.get_command_metrics(command_id).await
}

#[command]
pub async fn get_atm_workspace_command_metrics(
    client: State<'_, ATMClient>,
    window_ms: Option<i64>,
    limit: Option<usize>,
) -> Result<crate::services::atm_client::WorkspaceCommandMetricsResponse, String> {
    client.get_workspace_command_metrics(window_ms, limit).await
}

#[command]
pub async fn generate_pairing_code(
    client: State<'_, ATMClient>,
) -> Result<crate::services::atm_client::PairingCodeResponse, String> {
    client.generate_pairing_code().await
}

#[command]
pub async fn set_atm_credentials(
    client: State<'_, ATMClient>,
    api_key: String,
) -> Result<(), String> {
    client.set_credentials(api_key).await;
    Ok(())
}

#[command]
pub async fn has_atm_credentials(client: State<'_, ATMClient>) -> Result<bool, String> {
    Ok(client.has_credentials().await)
}

#[command]
pub async fn ensure_atm_credentials_loaded(client: State<'_, ATMClient>) -> Result<bool, String> {
    client.ensure_credentials_loaded().await
}

#[command]
pub async fn reset_neural_workspace(
    client: State<'_, ATMClient>,
    neural: State<'_, crate::commands::neural::NeuralServiceState>,
    master_key: String,
    user_api_key: String,
) -> Result<(), String> {
    // 1. Delete workspace on server
    client.reset_workspace(master_key, user_api_key).await?;

    // 2. Clear local credentials
    neural.0.clear_credentials().await?;
    client.clear_credentials().await?;

    Ok(())
}
