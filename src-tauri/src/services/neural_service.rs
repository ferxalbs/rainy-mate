use crate::ai::agent::runtime_registry::RuntimeRegistry;
use crate::models::neural::{
    CommandResult, DesktopNodeStatus, QueuedCommand, RuntimeStats, SkillManifest,
};
use crate::services::atm_auth::{
    clear_owner_auth_bundle, load_owner_auth_bundle, save_owner_auth_bundle, ATMOwnerAuthBundle,
};
use crate::services::manifest_signing::sign_skills_manifest;
use crate::services::security::NodeAuthenticator;
use crate::services::tool_manifest::build_skill_manifest_from_runtime;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Clone)]
pub struct NeuralService {
    http: Client,
    base_url: String,
    metadata: Arc<Mutex<NodeMetadata>>,
    register_lock: Arc<Mutex<()>>,
    authenticator: NodeAuthenticator,
    manifest_state: Arc<Mutex<ManifestState>>,
    runtime_registry: Option<Arc<RuntimeRegistry>>,
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

#[derive(Debug, Serialize, Deserialize)]
struct AuthContextResponse {
    success: bool,
    #[serde(rename = "workspaceId")]
    workspace_id: String,
    #[serde(rename = "workspaceName")]
    workspace_name: String,
}

#[derive(Debug, Default)]
struct ManifestState {
    last_hash: Option<String>,
    dirty: bool,
}

impl NeuralService {
    fn summarize_http_error_body(raw: &str) -> String {
        let compact = raw
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        if compact.is_empty() {
            return "empty error body".to_string();
        }
        if compact.contains("<html") || compact.contains("<!doctype") {
            return "temporary upstream HTML error response".to_string();
        }
        const MAX_LEN: usize = 220;
        if compact.len() > MAX_LEN {
            return format!("{}...", &compact[..MAX_LEN]);
        }
        compact
    }

    async fn clear_node_id(&self) {
        let mut metadata = self.metadata.lock().await;
        metadata.node_id = None;
    }

    async fn reset_node_on_status(&self, status: reqwest::StatusCode) {
        if matches!(
            status,
            reqwest::StatusCode::UNAUTHORIZED
                | reqwest::StatusCode::NOT_FOUND
                | reqwest::StatusCode::CONFLICT
        ) {
            self.clear_node_id().await;
        }
    }

    async fn get_auth_context(&self) -> Result<(String, String), String> {
        let metadata = self.metadata.lock().await;
        let node_id = metadata
            .node_id
            .clone()
            .ok_or("Node not registered".to_string())?;
        let platform_key = metadata
            .platform_key
            .clone()
            .ok_or("Not authenticated".to_string())?;
        Ok((node_id, platform_key))
    }

    async fn get_node_id(&self) -> Result<String, String> {
        let metadata = self.metadata.lock().await;
        metadata
            .node_id
            .clone()
            .ok_or("Node not registered".to_string())
    }

    pub fn new(
        base_url: String,
        workspace_id: String,
        authenticator: NodeAuthenticator,
        runtime_registry: Option<Arc<RuntimeRegistry>>,
    ) -> Self {
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
            register_lock: Arc::new(Mutex::new(())),
            authenticator,
            manifest_state: Arc::new(Mutex::new(ManifestState::default())),
            runtime_registry,
        }
    }

    pub async fn set_workspace_id(&self, workspace_id: String) {
        // Clone values needed for I/O, then drop the lock so keychain operations
        // do not block other metadata reads (heartbeat, registration, etc.)
        let (platform_key, user_api_key) = {
            let mut metadata = self.metadata.lock().await;
            metadata.workspace_id = workspace_id.clone();
            // Reset node_id to force re-registration with new workspace
            metadata.node_id = None;
            (metadata.platform_key.clone(), metadata.user_api_key.clone())
        };

        // Keychain I/O outside the lock
        let keychain = crate::ai::keychain::KeychainManager::new();
        if let Err(e) = keychain.store_key("neural_workspace_id", &workspace_id) {
            eprintln!("Failed to persist neural workspace id: {}", e);
        }

        if let (Some(platform_key), Some(user_api_key)) = (platform_key, user_api_key) {
            let bundle = ATMOwnerAuthBundle {
                platform_key,
                user_api_key,
                workspace_id: workspace_id.clone(),
            };
            if let Err(e) = save_owner_auth_bundle(&bundle) {
                eprintln!("Failed to persist ATM owner auth bundle: {}", e);
            }
        }
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

        let workspace_id = metadata.workspace_id.clone();

        // Persist to Keychain
        let keychain = crate::ai::keychain::KeychainManager::new();
        keychain.store_key("neural_platform_key", &platform_key)?;
        keychain.store_key("neural_user_api_key", &user_api_key)?;
        save_owner_auth_bundle(&ATMOwnerAuthBundle {
            platform_key,
            user_api_key,
            workspace_id,
        })?;

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
        let _ = keychain.delete_key("neural_workspace_id");
        let _ = clear_owner_auth_bundle();

        Ok(())
    }

    /// Load credentials from Keychain (call on app startup)
    pub async fn load_credentials_from_keychain(&self) -> Result<bool, String> {
        let keychain = crate::ai::keychain::KeychainManager::new();

        if let Some(bundle) = load_owner_auth_bundle()? {
            let mut metadata = self.metadata.lock().await;
            metadata.platform_key = Some(bundle.platform_key);
            metadata.user_api_key = Some(bundle.user_api_key);
            if !bundle.workspace_id.trim().is_empty() {
                metadata.workspace_id = bundle.workspace_id;
            }
            Ok(true)
        } else {
            let platform_key = keychain.get_key("neural_platform_key")?;
            let user_api_key = keychain.get_key("neural_user_api_key")?;
            let workspace_id = keychain.get_key("neural_workspace_id")?;

            if let (Some(pk), Some(uk)) = (platform_key, user_api_key) {
                let mut metadata = self.metadata.lock().await;
                metadata.platform_key = Some(pk.clone());
                metadata.user_api_key = Some(uk.clone());
                if let Some(ws) = workspace_id {
                    metadata.workspace_id = ws.clone();
                }

                let bundle = ATMOwnerAuthBundle {
                    platform_key: pk,
                    user_api_key: uk,
                    workspace_id: metadata.workspace_id.clone(),
                };
                if let Err(e) = save_owner_auth_bundle(&bundle) {
                    eprintln!("Failed to migrate legacy neural auth bundle: {}", e);
                }
                Ok(true)
            } else {
                Ok(false)
            }
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

    /// Retrieve the persisted workspace ID (for frontend to skip localStorage).
    pub async fn get_workspace_id(&self) -> Option<String> {
        let metadata = self.metadata.lock().await;
        let id = metadata.workspace_id.trim().to_string();
        if id.is_empty() || id == "pending-pairing" {
            None
        } else {
            Some(id)
        }
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

    pub async fn can_attempt_registration(&self) -> bool {
        let metadata = self.metadata.lock().await;
        metadata.platform_key.is_some()
            && metadata.user_api_key.is_some()
            && !metadata.workspace_id.trim().is_empty()
            && metadata.workspace_id != "pending-pairing"
    }

    pub async fn sync_workspace_id_with_auth_context(&self) -> Result<Option<String>, String> {
        let (platform_key, local_workspace_id) = {
            let metadata = self.metadata.lock().await;
            (
                metadata
                    .platform_key
                    .clone()
                    .ok_or("Not authenticated: Platform Key required".to_string())?,
                metadata.workspace_id.clone(),
            )
        };

        let url = format!("{}/v1/nodes/auth-context", self.base_url);
        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", platform_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Auth context sync failed: {} - {}",
                status, err_text
            ));
        }

        let data: AuthContextResponse = res.json().await.map_err(|e| e.to_string())?;
        if !data.success {
            return Err("Auth context sync failed: success=false".to_string());
        }

        if data.workspace_id != local_workspace_id {
            eprintln!(
                "[NeuralService] Auth-context workspace mismatch. local={} server={}. Updating local workspace and clearing node_id.",
                local_workspace_id, data.workspace_id
            );
            // set_workspace_id already clears node_id — no need to call clear_node_id again
            self.set_workspace_id(data.workspace_id.clone()).await;
            return Ok(Some(data.workspace_id));
        }

        Ok(None)
    }

    /// Registers this Desktop Node with the Cloud Cortex
    pub async fn register(
        &self,
        skills: Vec<SkillManifest>,
        allowed_paths: Vec<String>,
    ) -> Result<String, String> {
        let _register_guard = self.register_lock.lock().await;

        let (existing_node_id, platform_key, workspace_id, hostname, platform) = {
            let metadata = self.metadata.lock().await;
            (
                metadata.node_id.clone(),
                metadata
                    .platform_key
                    .clone()
                    .ok_or("Not authenticated: Platform Key required".to_string())?,
                metadata.workspace_id.clone(),
                metadata.hostname.clone(),
                metadata.platform.clone(),
            )
        };

        // If already registered, return existing ID.
        if let Some(id) = existing_node_id {
            return Ok(id);
        }

        let url = format!("{}/v1/nodes/register", self.base_url);

        let fingerprint = self
            .authenticator
            .get_device_fingerprint()
            .await
            .unwrap_or_else(|_| "unknown".to_string());

        let body = serde_json::json!({
            "workspaceId": workspace_id.clone(),
            "hostname": hostname,
            "platform": platform,
            "skills": skills,
            "allowedPaths": allowed_paths,
            "fingerprint": fingerprint
        });

        let skills_signature = sign_skills_manifest(&skills, &platform_key);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", platform_key))
            .header("X-Device-Fingerprint", fingerprint)
            .header("x-skills-signature", &skills_signature)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Registration failed: {} - {} (workspace_id={})",
                status, err_text, workspace_id
            ));
        }

        let data: RegisterResponse = res.json().await.map_err(|e| e.to_string())?;

        if data.success {
            let mut metadata = self.metadata.lock().await;
            metadata.node_id = Some(data.node_id.clone());
            let mut manifest_state = self.manifest_state.lock().await;
            manifest_state.last_hash = Some(skills_signature);
            manifest_state.dirty = false;
            Ok(data.node_id)
        } else {
            Err(data.message)
        }
    }

    /// Sends a heartbeat and checks for pending commands
    pub async fn heartbeat(&self, status: DesktopNodeStatus) -> Result<Vec<QueuedCommand>, String> {
        let (node_id, platform_key) = self.get_auth_context().await?;

        let url = format!("{}/v1/nodes/{}/heartbeat", self.base_url, node_id);

        let manifests = build_skill_manifest_from_runtime()?;
        let manifest_hash = sign_skills_manifest(&manifests, &platform_key);
        let include_manifest = {
            let state = self.manifest_state.lock().await;
            state.dirty || state.last_hash.as_deref() != Some(manifest_hash.as_str())
        };

        let runtime_stats: RuntimeStats = if let Some(registry) = self.runtime_registry.as_ref() {
            let snapshot = registry.snapshot().await;
            RuntimeStats {
                active_supervisor_runs: snapshot.active_supervisor_runs,
                active_specialists: snapshot.active_specialists,
                supervisors: snapshot
                    .supervisors
                    .into_iter()
                    .map(|run| crate::models::neural::SupervisorRunStatus {
                        run_id: run.run_id,
                        status: run.status,
                        specialist_count: run.specialist_count,
                        completed_specialists: run.completed_specialists,
                        failed_specialists: run.failed_specialists,
                        specialists: run
                            .specialists
                            .into_iter()
                            .map(
                                |specialist| crate::models::neural::SpecialistRuntimeStatus {
                                    agent_id: specialist.agent_id,
                                    role: specialist.role.as_str().to_string(),
                                    status: format!("{:?}", specialist.status).to_ascii_lowercase(),
                                    depends_on: specialist.depends_on,
                                    detail: specialist.detail,
                                    active_tool: specialist.active_tool,
                                    started_at_ms: specialist.started_at_ms,
                                    finished_at_ms: specialist.finished_at_ms,
                                    tool_count: specialist.tool_count,
                                    write_like_used: specialist.write_like_used,
                                },
                            )
                            .collect(),
                    })
                    .collect(),
                tool_usage_by_role: crate::models::neural::ToolUsageByRole {
                    research: snapshot.tool_usage_by_role.research,
                    executor: snapshot.tool_usage_by_role.executor,
                    verifier: snapshot.tool_usage_by_role.verifier,
                },
            }
        } else {
            RuntimeStats::default()
        };

        let body = if include_manifest {
            serde_json::json!({
                "status": status,
                "skills": manifests,
                "runtimeStats": runtime_stats,
            })
        } else {
            serde_json::json!({
                "status": status,
                "runtimeStats": runtime_stats,
            })
        };

        let mut request = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", platform_key));
        if include_manifest {
            request = request.header("x-skills-signature", &manifest_hash);
        }
        let res = request
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            self.reset_node_on_status(status).await;
            let err_text = res.text().await.unwrap_or_default();
            let summary = Self::summarize_http_error_body(&err_text);
            return Err(format!("Heartbeat failed: {} - {}", status, summary));
        }

        if include_manifest {
            let mut state = self.manifest_state.lock().await;
            state.last_hash = Some(manifest_hash);
            state.dirty = false;
        }

        let data: HeartbeatResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(data.pending_commands)
    }

    /// Gracefully marks the node offline on the server.
    pub async fn disconnect(&self) -> Result<(), String> {
        let (node_id, platform_key) = match self.get_auth_context().await {
            Ok(ctx) => ctx,
            Err(_) => return Ok(()),
        };

        let url = format!("{}/v1/nodes/{}/disconnect", self.base_url, node_id);
        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", platform_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            if matches!(
                status,
                reqwest::StatusCode::UNAUTHORIZED
                    | reqwest::StatusCode::NOT_FOUND
                    | reqwest::StatusCode::CONFLICT
            ) {
                self.clear_node_id().await;
                return Ok(());
            }
            return Err(format!("Disconnect failed: {}", status));
        }

        self.clear_node_id().await;

        Ok(())
    }

    /// Polls specifically for commands
    pub async fn poll_commands(&self) -> Result<Vec<QueuedCommand>, String> {
        let node_id = self.get_node_id().await?;

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
        let (node_id, platform_key) = self.get_auth_context().await?;

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
            let status = res.status();
            self.reset_node_on_status(status).await;
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Start command failed: {} - {} (command_id={})",
                status, err_text, command_id
            ));
        }

        Ok(())
    }

    /// Report command completion
    pub async fn complete_command(
        &self,
        command_id: &str,
        result: CommandResult,
    ) -> Result<(), String> {
        let (node_id, platform_key) = self.get_auth_context().await?;

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
            let status = res.status();
            self.reset_node_on_status(status).await;
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Complete command failed: {} - {} (command_id={})",
                status, err_text, command_id
            ));
        }

        Ok(())
    }

    /// Stream incremental command progress to Cloud.
    pub async fn report_command_progress(
        &self,
        command_id: &str,
        level: &str,
        message: &str,
        data: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let (node_id, platform_key) = self.get_auth_context().await?;

        let url = format!(
            "{}/v1/nodes/{}/commands/{}/progress",
            self.base_url, node_id, command_id
        );

        let body = serde_json::json!({
            "level": level,
            "message": message,
            "data": data,
        });

        const MAX_ATTEMPTS: usize = 3;
        let mut attempt = 0usize;
        loop {
            attempt += 1;
            let res = self
                .http
                .post(&url)
                .header("Authorization", format!("Bearer {}", platform_key))
                .json(&body)
                .send()
                .await;

            match res {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) => {
                    let status = response.status();
                    if status == reqwest::StatusCode::UNAUTHORIZED
                        || status == reqwest::StatusCode::NOT_FOUND
                        || status == reqwest::StatusCode::CONFLICT
                    {
                        self.reset_node_on_status(status).await;
                    }
                    let retryable = status.is_server_error();
                    if !retryable || attempt >= MAX_ATTEMPTS {
                        let err_text = response.text().await.unwrap_or_default();
                        return Err(format!(
                            "Report progress failed: {} - {} (command_id={})",
                            status, err_text, command_id
                        ));
                    }
                }
                Err(err) => {
                    if attempt >= MAX_ATTEMPTS {
                        return Err(err.to_string());
                    }
                }
            }

            // Exponential backoff (100ms, 200ms, then fail).
            sleep(Duration::from_millis(100 * (1 << (attempt - 1)) as u64)).await;
        }
    }
}
