use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
// Assumption: We need to store ATM Config somewhere.
// For now, I'll pass it or use a global state if available.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ATMClientState {
    pub base_url: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ATMClient {
    http: Client,
    state: Arc<Mutex<ATMClientState>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BootstrapResponse {
    pub success: bool,
    pub workspace: WorkspaceAuth,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceAuth {
    pub id: String,
    pub name: String,
    #[serde(rename = "apiKey")]
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateAgentParams {
    pub name: String,
    pub type_: String, // "default"
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PairingCodeResponse {
    pub code: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: i64,
}

impl ATMClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            http: Client::new(),
            state: Arc::new(Mutex::new(ATMClientState { base_url, api_key })),
        }
    }

    pub async fn set_credentials(&self, api_key: String) {
        let mut state = self.state.lock().await;
        state.api_key = Some(api_key);
    }

    pub async fn generate_pairing_code(&self) -> Result<PairingCodeResponse, String> {
        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;

        let url = format!("{}/admin/pairing", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Generate Pairing Code failed: {}", err_text));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn bootstrap(
        &self,
        master_key: String,
        user_api_key: String,
        name: String,
    ) -> Result<WorkspaceAuth, String> {
        let state = self.state.lock().await;
        let url = format!("{}/bootstrap", state.base_url);

        let res = self
            .http
            .post(&url)
            .json(&serde_json::json!({
                "masterKey": master_key,
                "apiKey": user_api_key,
                "name": name,
                "ownerId": "rainy-mate-desktop"
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Bootstrap failed: {} - {}", status, err_text));
        }

        let body: BootstrapResponse = res.json().await.map_err(|e| e.to_string())?;

        if body.success {
            Ok(body.workspace)
        } else {
            Err("Bootstrap failed".to_string())
        }
    }

    pub async fn create_agent(
        &self,
        params: CreateAgentParams,
    ) -> Result<serde_json::Value, String> {
        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;

        let url = format!("{}/admin/agents", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Create Agent failed: {}", err_text));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn list_agents(&self) -> Result<serde_json::Value, String> {
        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;

        let url = format!("{}/admin/agents", state.base_url);

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("List Agents failed: {}", err_text));
        }

        res.json().await.map_err(|e| e.to_string())
    }
}
