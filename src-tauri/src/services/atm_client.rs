use crate::ai::keychain::KeychainManager;
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

const ATM_ADMIN_KEYCHAIN_ID: &str = "atm_admin_key";

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

    pub async fn get_state(&self) -> ATMClientState {
        self.state.lock().await.clone()
    }

    pub async fn set_credentials(&self, api_key: String) {
        let mut state = self.state.lock().await;
        state.api_key = Some(api_key);

        // Best-effort persistence to keychain for session continuity
        let keychain = KeychainManager::new();
        if let Some(key) = state.api_key.as_ref() {
            if let Err(e) = keychain.store_key(ATM_ADMIN_KEYCHAIN_ID, key) {
                eprintln!("[ATMClient] Failed to persist admin key: {}", e);
            }
        }
    }

    pub async fn load_credentials_from_keychain(&self) -> Result<bool, String> {
        let keychain = KeychainManager::new();
        let stored = keychain.get_key(ATM_ADMIN_KEYCHAIN_ID)?;
        if let Some(api_key) = stored {
            self.set_credentials(api_key).await;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn clear_credentials(&self) -> Result<(), String> {
        let mut state = self.state.lock().await;
        state.api_key = None;

        let keychain = KeychainManager::new();
        let _ = keychain.delete_key(ATM_ADMIN_KEYCHAIN_ID);
        Ok(())
    }

    pub async fn ensure_credentials_loaded(&self) -> Result<bool, String> {
        {
            let state = self.state.lock().await;
            if state.api_key.is_some() {
                return Ok(true);
            }
        }

        self.load_credentials_from_keychain().await
    }

    pub async fn has_credentials(&self) -> bool {
        self.ensure_credentials_loaded().await.unwrap_or(false)
    }

    pub async fn generate_pairing_code(&self) -> Result<PairingCodeResponse, String> {
        if !self.ensure_credentials_loaded().await? {
            return Err("Not authenticated".to_string());
        }

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

    /// Delete workspace from the server (reset)
    pub async fn reset_workspace(
        &self,
        master_key: String,
        user_api_key: String,
    ) -> Result<(), String> {
        let state = self.state.lock().await;
        let url = format!("{}/bootstrap", state.base_url);

        let res = self
            .http
            .delete(&url)
            .json(&serde_json::json!({
                "masterKey": master_key,
                "apiKey": user_api_key
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Reset failed: {} - {}", status, err_text));
        }

        Ok(())
    }

    pub async fn create_agent(
        &self,
        params: CreateAgentParams,
    ) -> Result<serde_json::Value, String> {
        if !self.ensure_credentials_loaded().await? {
            return Err("Not authenticated".to_string());
        }

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
        if !self.ensure_credentials_loaded().await? {
            return Err("Not authenticated".to_string());
        }

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

    /// Deploys an AgentSpec v2 to the Cloud, signing it first.
    pub async fn deploy_agent(
        &self,
        mut spec: crate::ai::specs::AgentSpec,
    ) -> Result<serde_json::Value, String> {
        use crate::ai::features::security_service::SecurityService;

        // 1. Sign the agent package
        let security = SecurityService::new();

        // Hash capabilities for integrity
        let skills_json = serde_json::to_value(&spec.skills).map_err(|e| e.to_string())?;
        let cap_hash = SecurityService::hash_capabilities(&skills_json);

        // Create content to sign (Soul + Skills Hash + Version)
        let signable_content = format!("{}:{}:{}", spec.soul.name, cap_hash, spec.version);
        let signature_str = security
            .sign_content(&signable_content)
            .map_err(|e| e.to_string())?;
        let pub_key = security
            .get_public_key_string()
            .map_err(|e| e.to_string())?;

        // Attach signature
        spec.signature = Some(crate::ai::specs::AgentSignature {
            signature: signature_str,
            signer_id: pub_key, // Using public key as ID for now
            capabilities_hash: cap_hash,
            origin_device_id: "desktop-local".to_string(), // TODO: Get real device ID
            signed_at: chrono::Utc::now().timestamp(),
        });

        // 2. Wrap in CreateAgentParams
        let config_json = serde_json::to_value(&spec).map_err(|e| e.to_string())?;

        let params = CreateAgentParams {
            name: spec.soul.name.clone(),
            type_: "v2_spec".to_string(),
            config: config_json,
        };

        // 3. Upload
        self.create_agent(params).await
    }
}
