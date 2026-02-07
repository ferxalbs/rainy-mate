use crate::services::ATMClient;
use tauri::State;

#[tauri::command]
pub async fn deploy_agent(
    client: State<'_, ATMClient>,
    spec: crate::ai::specs::AgentSpec,
) -> Result<serde_json::Value, String> {
    client.deploy_agent(spec).await
}
