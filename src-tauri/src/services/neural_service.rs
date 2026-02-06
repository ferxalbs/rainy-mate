use crate::models::neural::{CommandResult, DesktopNodeStatus, QueuedCommand, SkillManifest};
use crate::services::security::NodeAuthenticator;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct NeuralService {
    http: Client,
    base_url: String,
    metadata: Arc<Mutex<NodeMetadata>>,
    authenticator: NodeAuthenticator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub node_id: Option<String>,
    pub workspace_id: String,
    pub hostname: String,
    pub platform: String,
    pub platform_key: Option<String>,
    pub user_api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterResponse {
    success: bool,
    #[serde(rename = "nodeId")]
    node_id: String,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HeartbeatResponse {
    success: bool,
    #[serde(rename = "pendingCommands")]
    pending_commands: Vec<QueuedCommand>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CommandsResponse {
    commands: Vec<QueuedCommand>,
}

impl NeuralService {
    pub fn new(base_url: String, workspace_id: String, authenticator: NodeAuthenticator) -> Self {
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "unknown-host".to_string());

        let platform = std::env::consts::OS.to_string();

        Self {
            http: Client::new(),
            base_url,
            metadata: Arc::new(Mutex::new(NodeMetadata {
                node_id: None,
                workspace_id,
                hostname,
                platform,
                platform_key: None,
                user_api_key: None,
            })),
            authenticator,
        }
    }

    pub async fn set_workspace_id(&self, workspace_id: String) {
        let mut metadata = self.metadata.lock().await;
        metadata.workspace_id = workspace_id;
        // Reset node_id to force re-registration with new workspace
        metadata.node_id = None;
    }

    /// Set authentication credentials (Platform Key + User API Key)
    /// Keys are also persisted to macOS Keychain for session persistence
    pub async fn set_credentials(
        &self,
        platform_key: String,
        user_api_key: String,
    ) -> Result<(), String> {
        // Store in memory
        let mut metadata = self.metadata.lock().await;
        metadata.platform_key = Some(platform_key.clone());
        metadata.user_api_key = Some(user_api_key.clone());
        metadata.node_id = None; // Force re-registration with new credentials

        // Persist to Keychain
        let keychain = crate::ai::keychain::KeychainManager::new();
        keychain.store_key("neural_platform_key", &platform_key)?;
        keychain.store_key("neural_user_api_key", &user_api_key)?;

        Ok(())
    }

    /// Clear authentication credentials (Logout/Reset)
    pub async fn clear_credentials(&self) -> Result<(), String> {
        // 1. Clear In-Memory State
        let mut metadata = self.metadata.lock().await;
        metadata.platform_key = None;
        metadata.user_api_key = None;
        metadata.node_id = None;

        // 2. Remove from Keychain
        let keychain = crate::ai::keychain::KeychainManager::new();
        // Ignore errors if keys don't exist
        let _ = keychain.delete_key("neural_platform_key");
        let _ = keychain.delete_key("neural_user_api_key");

        Ok(())
    }

    /// Load credentials from Keychain (call on app startup)
    pub async fn load_credentials_from_keychain(&self) -> Result<bool, String> {
        let keychain = crate::ai::keychain::KeychainManager::new();

        let platform_key = keychain.get_key("neural_platform_key")?;
        let user_api_key = keychain.get_key("neural_user_api_key")?;

        if let (Some(pk), Some(uk)) = (platform_key, user_api_key) {
            let mut metadata = self.metadata.lock().await;
            metadata.platform_key = Some(pk);
            metadata.user_api_key = Some(uk);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if credentials are configured
    pub async fn has_credentials(&self) -> bool {
        let metadata = self.metadata.lock().await;
        metadata.platform_key.is_some() && metadata.user_api_key.is_some()
    }

    /// Check if node is registered (has a node_id from the server)
    pub async fn is_registered(&self) -> bool {
        let metadata = self.metadata.lock().await;
        metadata.node_id.is_some()
    }

    /// Retrieve credentials (for session persistence in UI)
    pub async fn get_credentials(&self) -> Option<(String, String)> {
        let metadata = self.metadata.lock().await;
        if let (Some(pk), Some(uk)) = (&metadata.platform_key, &metadata.user_api_key) {
            Some((pk.clone(), uk.clone()))
        } else {
            None
        }
    }

    /// Registers this Desktop Node with the Cloud Cortex
    pub async fn register(
        &self,
        skills: Vec<SkillManifest>,
        allowed_paths: Vec<String>,
    ) -> Result<String, String> {
        let mut metadata = self.metadata.lock().await;

        // If already registered, return existing ID (or maybe re-register?)
        if let Some(id) = &metadata.node_id {
            return Ok(id.clone());
        }

        // Check for credentials
        let platform_key = metadata
            .platform_key
            .as_ref()
            .ok_or("Not authenticated: Platform Key required")?
            .clone();

        let url = format!("{}/v1/nodes/register", self.base_url);

        let fingerprint = self
            .authenticator
            .get_device_fingerprint()
            .await
            .unwrap_or_else(|_| "unknown".to_string());

        let body = serde_json::json!({
            "workspaceId": metadata.workspace_id,
            "hostname": metadata.hostname,
            "platform": metadata.platform,
            "skills": skills,
            "allowedPaths": allowed_paths,
            "fingerprint": fingerprint
        });

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", platform_key))
            .header("X-Device-Fingerprint", fingerprint)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Registration failed: {} - {}", status, err_text));
        }

        let data: RegisterResponse = res.json().await.map_err(|e| e.to_string())?;

        if data.success {
            metadata.node_id = Some(data.node_id.clone());
            Ok(data.node_id)
        } else {
            Err(data.message)
        }
    }

    /// Sends a heartbeat and checks for pending commands
    pub async fn heartbeat(&self, status: DesktopNodeStatus) -> Result<Vec<QueuedCommand>, String> {
        let metadata = self.metadata.lock().await;
        let node_id = metadata.node_id.as_ref().ok_or("Node not registered")?;
        let platform_key = metadata.platform_key.as_ref().ok_or("Not authenticated")?;

        let url = format!("{}/v1/nodes/{}/heartbeat", self.base_url, node_id);

        let body = serde_json::json!({
            "status": status // Serializes based on enum config (lowercase)
        });

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", platform_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(format!("Heartbeat failed: {}", res.status()));
        }

        let data: HeartbeatResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(data.pending_commands)
    }

    /// Polls specifically for commands
    pub async fn poll_commands(&self) -> Result<Vec<QueuedCommand>, String> {
        let metadata = self.metadata.lock().await;
        let node_id = metadata.node_id.as_ref().ok_or("Node not registered")?;

        let url = format!("{}/v1/nodes/{}/commands", self.base_url, node_id);

        let res = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(format!("Poll commands failed: {}", res.status()));
        }

        let data: CommandsResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(data.commands)
    }

    /// Mark a command as started
    pub async fn start_command(&self, command_id: &str) -> Result<(), String> {
        let metadata = self.metadata.lock().await;
        let node_id = metadata.node_id.as_ref().ok_or("Node not registered")?;
        let platform_key = metadata.platform_key.as_ref().ok_or("Not authenticated")?;

        let url = format!(
            "{}/v1/nodes/{}/commands/{}/start",
            self.base_url, node_id, command_id
        );

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", platform_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(format!("Start command failed: {}", res.status()));
        }

        Ok(())
    }

    /// Report command completion
    pub async fn complete_command(
        &self,
        command_id: &str,
        result: CommandResult,
    ) -> Result<(), String> {
        let metadata = self.metadata.lock().await;
        let node_id = metadata.node_id.as_ref().ok_or("Node not registered")?;
        let platform_key = metadata.platform_key.as_ref().ok_or("Not authenticated")?;

        let url = format!(
            "{}/v1/nodes/{}/commands/{}/complete",
            self.base_url, node_id, command_id
        );

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", platform_key))
            .json(&result)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(format!("Complete command failed: {}", res.status()));
        }

        Ok(())
    }
}
