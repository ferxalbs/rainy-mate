use crate::services::atm_client::{ATMClient, CreateAgentParams};
use tauri::{command, State};

#[command]
pub async fn bootstrap_atm(
    client: State<'_, ATMClient>,
    master_key: String,
    name: String,
) -> Result<crate::services::atm_client::WorkspaceAuth, String> {
    let auth = client.bootstrap(master_key, name).await?;
    // Automatically set credentials in client
    client.set_credentials(auth.api_key.clone()).await;
    Ok(auth)
}

#[command]
pub async fn create_atm_agent(
    client: State<'_, ATMClient>,
    name: String,
    type_: String,
    config: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let params = CreateAgentParams {
        name,
        type_,
        config,
    };
    client.create_agent(params).await
}

#[command]
pub async fn list_atm_agents(client: State<'_, ATMClient>) -> Result<serde_json::Value, String> {
    client.list_agents().await
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
