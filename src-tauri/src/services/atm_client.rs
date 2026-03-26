use crate::ai::keychain::KeychainManager;
use crate::services::atm_auth::{
    clear_owner_auth_bundle, load_owner_auth_bundle, ATMOwnerAuthBundle,
};
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
    pub platform_key: Option<String>,
    pub user_api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ATMClient {
    http: Client,
    state: Arc<Mutex<ATMClientState>>,
}

const ATM_ADMIN_KEYCHAIN_ID: &str = "atm_admin_key";

#[derive(Debug, Clone)]
pub struct ATMServiceStatus {
    pub ready: bool,
    pub code: Option<String>,
    pub message: String,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AtmAgentSummary {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub agent_type: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub logical_spec_id: Option<String>,
    pub is_duplicate_logical_spec: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AtmAgentListResponse {
    pub agents: Vec<AtmAgentSummary>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AtmWorkspaceModel {
    pub id: String,
    pub name: String,
    pub provider: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AtmWorkspaceModelCatalogResponse {
    pub models: Vec<AtmWorkspaceModel>,
    pub cached: bool,
    pub fetched_at: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PairingCodeResponse {
    pub code: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: i64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ATMCommandSummary {
    pub id: String,
    pub intent: String,
    pub status: String,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub desktop_node_id: Option<String>,
    pub timings: Option<ATMCommandTimings>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ATMCommandTimings {
    pub queue_delay_ms: Option<i64>,
    pub run_duration_ms: Option<i64>,
    pub total_duration_ms: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListCommandsResponse {
    pub commands: Vec<ATMCommandSummary>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ATMCommandProgressEvent {
    pub id: String,
    pub level: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ATMCommandDetails {
    pub id: String,
    pub intent: String,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub desktop_node_id: Option<String>,
    pub timings: Option<ATMCommandTimings>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandDetailsResponse {
    pub command: ATMCommandDetails,
    pub progress: Vec<ATMCommandProgressEvent>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommandProgressResponse {
    pub command_id: String,
    pub progress: Vec<ATMCommandProgressEvent>,
    pub next_since: i64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ATMCommandProgressMetrics {
    pub total_events: i64,
    pub first_event_at: Option<i64>,
    pub last_event_at: Option<i64>,
    pub by_level: std::collections::HashMap<String, i64>,
    pub dropped_events_total: i64,
    pub suppressed_events_total: i64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommandMetricsResponse {
    pub command_id: String,
    pub status: String,
    pub timings: ATMCommandTimings,
    pub progress: ATMCommandProgressMetrics,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCommandMetricAverages {
    pub queue_delay_ms: Option<i64>,
    pub run_duration_ms: Option<i64>,
    pub total_duration_ms: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCommandMetricsResponse {
    pub workspace_id: String,
    pub window_ms: i64,
    pub since: i64,
    pub sampled_commands: i64,
    pub status_counts: std::collections::HashMap<String, i64>,
    pub failure_buckets: std::collections::HashMap<String, i64>,
    pub averages: WorkspaceCommandMetricAverages,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EndpointLatencyMetrics {
    pub avg_total_ms: Option<i64>,
    pub p95_total_ms: Option<i64>,
    pub avg_run_ms: Option<i64>,
    pub p95_run_ms: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EndpointSignalMetrics {
    pub warn_events: Option<i64>,
    pub error_events: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EndpointMetricsItem {
    pub key: String,
    pub label: String,
    pub requests: i64,
    pub rate_per_second: Option<f64>,
    pub success_rate: Option<f64>,
    pub error_rate: Option<f64>,
    pub latency: EndpointLatencyMetrics,
    pub signals: Option<EndpointSignalMetrics>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EndpointMetricsResponse {
    pub workspace_id: String,
    pub window_ms: i64,
    pub since: i64,
    pub endpoints: Vec<EndpointMetricsItem>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetricsAlert {
    pub id: String,
    pub source: String,
    pub key: String,
    pub severity: String,
    pub reason: String,
    pub status: String,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
    pub acked_at: Option<i64>,
    pub acked_by: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MetricsAlertsResponse {
    pub alerts: Vec<MetricsAlert>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AlertSyncItem {
    pub source: String,
    pub key: String,
    pub severity: String,
    pub reason: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AlertSyncResponse {
    pub success: bool,
    pub upserts: i64,
    pub resolved: i64,
    pub open_count: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSloConfig {
    pub endpoint_error_rate_warn: f64,
    pub endpoint_error_rate_critical: f64,
    pub endpoint_p95_warn_ms: f64,
    pub endpoint_p95_critical_ms: f64,
    pub endpoint_slo_error_rate_target: f64,
    pub endpoint_slo_p95_target_ms: f64,
    pub endpoint_regression_error_rate_factor: f64,
    pub endpoint_regression_error_rate_delta: f64,
    pub endpoint_regression_p95_factor: f64,
    pub endpoint_regression_p95_delta_ms: f64,
    pub failure_timeout_warn: f64,
    pub failure_timeout_critical: f64,
    pub failure_runtime_warn: f64,
    pub failure_runtime_critical: f64,
    pub failure_transport_warn: f64,
    pub failure_transport_critical: f64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSloResponse {
    pub metrics_slo: MetricsSloConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetricsAlertRetentionConfig {
    pub days: i64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetricsAlertRetentionResponse {
    pub metrics_alert_retention: MetricsAlertRetentionConfig,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetricsAlertCleanupResponse {
    pub success: bool,
    pub deleted: i64,
    pub retention_days: i64,
    pub cutoff_ts: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AdminPermissions {
    pub can_edit_slo: bool,
    pub can_ack_alerts: bool,
    pub can_edit_alert_retention: bool,
    pub can_run_alert_cleanup: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AdminPermissionsResponse {
    pub permissions: AdminPermissions,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolAccessPolicy {
    pub enabled: bool,
    pub mode: String,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolAccessPolicyState {
    pub tool_access_policy: ToolAccessPolicy,
    pub tool_access_policy_version: u64,
    pub tool_access_policy_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ToolAccessPolicyResponse {
    pub tool_access_policy: ToolAccessPolicy,
    #[serde(default)]
    pub tool_access_policy_version: u64,
    #[serde(default)]
    pub tool_access_policy_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FleetStatusResponse {
    pub workspace_id: String,
    #[serde(default)]
    pub current_airlock_policy: Option<serde_json::Value>,
    pub nodes: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FleetDispatchResponse {
    pub success: bool,
    pub dispatch: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FleetRetireNodeResponse {
    pub success: bool,
    pub node_id: String,
    pub retired_at: i64,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSharedAgentSummary {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub status: String,
    pub logical_spec_id: Option<String>,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSharedAgentsResponse {
    pub workspace_id: String,
    pub agents: Vec<WorkspaceSharedAgentSummary>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSharedAgentSpecResponse {
    pub workspace_id: String,
    pub agent_id: String,
    pub name: String,
    pub logical_spec_id: Option<String>,
    pub updated_at: i64,
    pub spec: crate::ai::specs::AgentSpec,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceAgentSummary {
    pub id: String,
    pub source_agent_id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub installs: i64,
    pub author_label: String,
    pub visibility: String,
    pub status: String,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceAgentsResponse {
    pub workspace_id: String,
    pub agents: Vec<MarketplaceAgentSummary>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PublishMarketplaceAgentRequest {
    pub source_agent_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author_label: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PublishMarketplaceAgentResponse {
    pub action: String,
    pub id: String,
    pub source_agent_id: String,
    pub name: String,
    pub visibility: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceAgentSpecResponse {
    pub workspace_id: String,
    pub marketplace_id: String,
    pub source_agent_id: String,
    pub name: String,
    pub installs: i64,
    pub updated_at: i64,
    pub spec: crate::ai::specs::AgentSpec,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AdminPolicyAuditEvent {
    pub id: String,
    pub actor: String,
    pub event_type: String,
    pub previous: Option<serde_json::Value>,
    pub next: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AdminPolicyAuditResponse {
    pub events: Vec<AdminPolicyAuditEvent>,
}

impl ATMClient {
    fn parse_service_status(status: reqwest::StatusCode, body: &str) -> ATMServiceStatus {
        let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
        let code = parsed
            .as_ref()
            .and_then(|value| value.get("code"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());
        let message = parsed
            .as_ref()
            .and_then(|value| value.get("error").or_else(|| value.get("message")))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| body.trim().to_string());

        ATMServiceStatus {
            ready: status.is_success(),
            code,
            message: if message.is_empty() {
                format!("HTTP {}", status)
            } else {
                message
            },
        }
    }

    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            http: Client::new(),
            state: Arc::new(Mutex::new(ATMClientState {
                base_url,
                api_key,
                platform_key: None,
                user_api_key: None,
            })),
        }
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

    pub async fn set_owner_auth_context(
        &self,
        platform_key: String,
        user_api_key: String,
        workspace_id: String,
    ) -> Result<(), String> {
        {
            let mut state = self.state.lock().await;
            state.platform_key = Some(platform_key.clone());
            state.user_api_key = Some(user_api_key.clone());
        }

        let bundle = ATMOwnerAuthBundle {
            platform_key,
            user_api_key,
            workspace_id,
        };

        crate::services::atm_auth::save_owner_auth_bundle(&bundle)
    }

    pub async fn load_credentials_from_keychain(&self) -> Result<bool, String> {
        let keychain = KeychainManager::new();
        let stored = keychain.get_key(ATM_ADMIN_KEYCHAIN_ID)?;
        if let Some(api_key) = stored {
            self.set_credentials(api_key).await;

            if let Some(bundle) = load_owner_auth_bundle()? {
                let mut state = self.state.lock().await;
                state.platform_key = Some(bundle.platform_key);
                state.user_api_key = Some(bundle.user_api_key);
            } else if let Ok(Some(pk)) = keychain.get_key("neural_platform_key") {
                let mut state = self.state.lock().await;
                state.platform_key = Some(pk);
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn clear_credentials(&self) -> Result<(), String> {
        let mut state = self.state.lock().await;
        state.api_key = None;
        state.platform_key = None;
        state.user_api_key = None;

        let keychain = KeychainManager::new();
        let _ = keychain.delete_key(ATM_ADMIN_KEYCHAIN_ID);
        let _ = clear_owner_auth_bundle();
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

    pub async fn get_service_status(&self) -> Result<ATMServiceStatus, String> {
        let state = self.state.lock().await;
        let url = format!("{}/health/ready", state.base_url);
        drop(state);

        let res = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Rainy-ATM readiness check failed: {}", e))?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Ok(Self::parse_service_status(status, &body))
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

        let mut req = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key));

        if let Some(pk) = state.platform_key.as_ref() {
            req = req.header("x-rainy-platform-key", pk);
        }
        if let Some(user_api_key) = state.user_api_key.as_ref() {
            req = req.header("x-rainy-api-key", user_api_key);
        }

        let res = req.json(&params).send().await.map_err(|e| e.to_string())?;

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

        let mut req = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key));

        if let Some(pk) = state.platform_key.as_ref() {
            req = req.header("x-rainy-platform-key", pk);
        }
        if let Some(user_api_key) = state.user_api_key.as_ref() {
            req = req.header("x-rainy-api-key", user_api_key);
        }

        let res = req.send().await.map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("List Agents failed: {}", err_text));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn list_agent_summaries(&self) -> Result<Vec<AtmAgentSummary>, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/agents", state.base_url);

        let mut req = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key));

        if let Some(pk) = state.platform_key.as_ref() {
            req = req.header("x-rainy-platform-key", pk);
        }
        if let Some(user_api_key) = state.user_api_key.as_ref() {
            req = req.header("x-rainy-api-key", user_api_key);
        }

        let res = req.send().await.map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("List Agents failed: {}", err_text));
        }

        let response: AtmAgentListResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(response.agents)
    }

    pub async fn list_workspace_models(&self) -> Result<Vec<AtmWorkspaceModel>, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let platform_key = state.platform_key.clone().ok_or("Missing platform key")?;
        let user_api_key = state.user_api_key.clone().ok_or("Missing owner API key")?;
        let url = format!("{}/admin/models/catalog", state.base_url);

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("x-rainy-platform-key", platform_key)
            .header("x-rainy-api-key", user_api_key)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Workspace model catalog lookup failed: {} - {}",
                status, err_text
            ));
        }

        let response: AtmWorkspaceModelCatalogResponse =
            res.json().await.map_err(|e| e.to_string())?;
        Ok(response.models)
    }

    pub async fn verify_authenticated_connection(&self) -> Result<(), String> {
        let service_status = self.get_service_status().await?;
        if !service_status.ready {
            let code = service_status.code.unwrap_or_else(|| "UNKNOWN".to_string());
            return Err(format!(
                "Rainy-ATM auth check failed: 503 Service Unavailable - {{\"error\":\"{}\",\"code\":\"{}\"}}",
                service_status.message, code
            ));
        }

        if !self.ensure_credentials_loaded().await? {
            return Err("Not authenticated with Rainy-ATM".to_string());
        }

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/workspace", state.base_url);

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Rainy-ATM connection error: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Rainy-ATM auth check failed: {} - {}",
                status, err_text
            ));
        }

        Ok(())
    }

    pub async fn list_commands(
        &self,
        limit: Option<usize>,
        status: Option<String>,
    ) -> Result<Vec<ATMCommandSummary>, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let mut url = format!(
            "{}/admin/commands?limit={}",
            state.base_url,
            limit.unwrap_or(50)
        );
        if let Some(s) = status {
            if !s.trim().is_empty() {
                url.push_str("&status=");
                url.push_str(s.trim());
            }
        }

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("List commands failed: {} - {}", status, err_text));
        }

        let body: ListCommandsResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(body.commands)
    }

    pub async fn get_command_details(
        &self,
        command_id: String,
        progress_limit: Option<usize>,
    ) -> Result<CommandDetailsResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!(
            "{}/admin/commands/{}?progressLimit={}",
            state.base_url,
            command_id,
            progress_limit.unwrap_or(200)
        );

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get command details failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_command_progress(
        &self,
        command_id: String,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<CommandProgressResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!(
            "{}/admin/commands/{}/progress?since={}&limit={}",
            state.base_url,
            command_id,
            since.unwrap_or(0),
            limit.unwrap_or(200)
        );

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get command progress failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_command_metrics(
        &self,
        command_id: String,
    ) -> Result<CommandMetricsResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/commands/{}/metrics", state.base_url, command_id);

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get command metrics failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_workspace_command_metrics(
        &self,
        window_ms: Option<i64>,
        limit: Option<usize>,
    ) -> Result<WorkspaceCommandMetricsResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!(
            "{}/admin/metrics/commands?windowMs={}&limit={}",
            state.base_url,
            window_ms.unwrap_or(24 * 60 * 60 * 1000),
            limit.unwrap_or(500)
        );

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get workspace command metrics failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_endpoint_metrics(
        &self,
        window_ms: Option<i64>,
        limit: Option<usize>,
    ) -> Result<EndpointMetricsResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!(
            "{}/admin/metrics/endpoints?windowMs={}&limit={}",
            state.base_url,
            window_ms.unwrap_or(60 * 60 * 1000),
            limit.unwrap_or(2000)
        );

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get endpoint metrics failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn sync_metrics_alerts(
        &self,
        alerts: Vec<AlertSyncItem>,
    ) -> Result<AlertSyncResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/metrics/alerts/sync", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({ "alerts": alerts }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Sync metrics alerts failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn list_metrics_alerts(
        &self,
        status: Option<String>,
        limit: Option<usize>,
    ) -> Result<Vec<MetricsAlert>, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let mut url = format!(
            "{}/admin/metrics/alerts?limit={}",
            state.base_url,
            limit.unwrap_or(100)
        );
        if let Some(s) = status {
            if !s.trim().is_empty() {
                url.push_str("&status=");
                url.push_str(s.trim());
            }
        }

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "List metrics alerts failed: {} - {}",
                status, err_text
            ));
        }

        let body: MetricsAlertsResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(body.alerts)
    }

    pub async fn acknowledge_metrics_alert(
        &self,
        alert_id: String,
        acked_by: Option<String>,
    ) -> Result<serde_json::Value, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/metrics/alerts/{}/ack", state.base_url, alert_id);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "ackedBy": acked_by.unwrap_or_else(|| "desktop-admin".to_string())
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Acknowledge metrics alert failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_metrics_slo(&self) -> Result<MetricsSloConfig, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/metrics/slo", state.base_url);

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!("Get metrics SLO failed: {} - {}", status, err_text));
        }

        let body: MetricsSloResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(body.metrics_slo)
    }

    pub async fn update_metrics_slo(
        &self,
        metrics_slo: MetricsSloConfig,
    ) -> Result<MetricsSloConfig, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/metrics/slo", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({ "metricsSlo": metrics_slo }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Update metrics SLO failed: {} - {}",
                status, err_text
            ));
        }

        let body: MetricsSloResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(body.metrics_slo)
    }

    pub async fn get_metrics_alert_retention(&self) -> Result<MetricsAlertRetentionConfig, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/metrics/alerts/retention", state.base_url);

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get metrics alert retention failed: {} - {}",
                status, err_text
            ));
        }

        let body: MetricsAlertRetentionResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(body.metrics_alert_retention)
    }

    pub async fn update_metrics_alert_retention(
        &self,
        metrics_alert_retention: MetricsAlertRetentionConfig,
    ) -> Result<MetricsAlertRetentionConfig, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/metrics/alerts/retention", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({ "metricsAlertRetention": metrics_alert_retention }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Update metrics alert retention failed: {} - {}",
                status, err_text
            ));
        }

        let body: MetricsAlertRetentionResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(body.metrics_alert_retention)
    }

    pub async fn cleanup_metrics_alerts(&self) -> Result<MetricsAlertCleanupResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/metrics/alerts/cleanup", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Cleanup metrics alerts failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_admin_permissions(&self) -> Result<AdminPermissions, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/permissions", state.base_url);

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get admin permissions failed: {} - {}",
                status, err_text
            ));
        }

        let body: AdminPermissionsResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(body.permissions)
    }

    pub async fn update_admin_permissions(
        &self,
        permissions: AdminPermissions,
        platform_key: String,
        user_api_key: String,
    ) -> Result<AdminPermissions, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/permissions", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("x-rainy-platform-key", platform_key)
            .header("x-rainy-api-key", user_api_key)
            .json(&serde_json::json!({ "permissions": permissions }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Update admin permissions failed: {} - {}",
                status, err_text
            ));
        }

        let body: AdminPermissionsResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(body.permissions)
    }

    pub async fn list_admin_policy_audit(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<AdminPolicyAuditEvent>, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!(
            "{}/admin/permissions/audit?limit={}",
            state.base_url,
            limit.unwrap_or(50)
        );

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "List admin policy audit failed: {} - {}",
                status, err_text
            ));
        }

        let body: AdminPolicyAuditResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(body.events)
    }

    pub async fn get_tool_access_policy(&self) -> Result<ToolAccessPolicyState, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/tools/policy", state.base_url);

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get tool access policy failed: {} - {}",
                status, err_text
            ));
        }

        let body: ToolAccessPolicyResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(ToolAccessPolicyState {
            tool_access_policy: body.tool_access_policy,
            tool_access_policy_version: body.tool_access_policy_version,
            tool_access_policy_hash: body.tool_access_policy_hash,
        })
    }

    pub async fn update_tool_access_policy(
        &self,
        tool_access_policy: ToolAccessPolicy,
        platform_key: String,
        user_api_key: String,
    ) -> Result<ToolAccessPolicyState, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/tools/policy", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("x-rainy-platform-key", platform_key)
            .header("x-rainy-api-key", user_api_key)
            .json(&serde_json::json!({ "toolAccessPolicy": tool_access_policy }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Update tool access policy failed: {} - {}",
                status, err_text
            ));
        }

        let body: ToolAccessPolicyResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(ToolAccessPolicyState {
            tool_access_policy: body.tool_access_policy,
            tool_access_policy_version: body.tool_access_policy_version,
            tool_access_policy_hash: body.tool_access_policy_hash,
        })
    }

    pub async fn get_fleet_status(&self) -> Result<FleetStatusResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/admin/fleet/status", state.base_url);

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get fleet status failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn push_fleet_policy(
        &self,
        tool_access_policy: ToolAccessPolicy,
    ) -> Result<serde_json::Value, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let platform_key = state.platform_key.clone().ok_or("Missing platform key")?;
        let user_api_key = state.user_api_key.clone().ok_or("Missing owner API key")?;
        let url = format!("{}/admin/fleet/policy", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("x-rainy-platform-key", platform_key)
            .header("x-rainy-api-key", user_api_key)
            .json(&serde_json::json!({ "toolAccessPolicy": tool_access_policy }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Push fleet policy failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn trigger_fleet_kill_switch(&self) -> Result<FleetDispatchResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let platform_key = state.platform_key.clone().ok_or("Missing platform key")?;
        let user_api_key = state.user_api_key.clone().ok_or("Missing owner API key")?;
        let url = format!("{}/admin/fleet/kill", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("x-rainy-platform-key", platform_key)
            .header("x-rainy-api-key", user_api_key)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Fleet kill switch failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn retire_fleet_node(
        &self,
        node_id: String,
        reason: Option<String>,
    ) -> Result<FleetRetireNodeResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let platform_key = state.platform_key.clone().ok_or("Missing platform key")?;
        let user_api_key = state.user_api_key.clone().ok_or("Missing owner API key")?;
        let url = format!("{}/admin/fleet/nodes/{}/retire", state.base_url, node_id);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("x-rainy-platform-key", platform_key)
            .header("x-rainy-api-key", user_api_key)
            .json(&serde_json::json!({
                "reason": reason.unwrap_or_else(|| "retired_from_desktop".to_string())
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Retire fleet node failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn list_workspace_shared_agents(
        &self,
        limit: Option<usize>,
    ) -> Result<WorkspaceSharedAgentsResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!(
            "{}/v1/workspace/agents?limit={}",
            state.base_url,
            limit.unwrap_or(100)
        );

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "List workspace shared agents failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_workspace_shared_agent_spec(
        &self,
        agent_id: String,
    ) -> Result<WorkspaceSharedAgentSpecResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!("{}/v1/workspace/agents/{}/spec", state.base_url, agent_id);

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get workspace shared agent spec failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn list_marketplace_agents(
        &self,
        limit: Option<usize>,
    ) -> Result<MarketplaceAgentsResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!(
            "{}/v1/marketplace/agents?limit={}",
            state.base_url,
            limit.unwrap_or(100)
        );

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "List marketplace agents failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn publish_marketplace_agent(
        &self,
        payload: PublishMarketplaceAgentRequest,
    ) -> Result<PublishMarketplaceAgentResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let platform_key = state.platform_key.clone().ok_or("Missing platform key")?;
        let user_api_key = state.user_api_key.clone().ok_or("Missing owner API key")?;
        let url = format!("{}/v1/marketplace/agents", state.base_url);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("x-rainy-platform-key", platform_key)
            .header("x-rainy-api-key", user_api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Publish marketplace agent failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_marketplace_agent_spec(
        &self,
        marketplace_id: String,
    ) -> Result<MarketplaceAgentSpecResponse, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let url = format!(
            "{}/v1/marketplace/agents/{}/spec",
            state.base_url, marketplace_id
        );

        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Get marketplace agent spec failed: {} - {}",
                status, err_text
            ));
        }

        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn send_fleet_audit_events(
        &self,
        node_id: String,
        events: Vec<crate::services::audit_emitter::FleetAuditEvent>,
    ) -> Result<usize, String> {
        self.verify_authenticated_connection().await?;

        let state = self.state.lock().await;
        let api_key = state.api_key.as_ref().ok_or("Not authenticated")?;
        let platform_key = state.platform_key.clone().ok_or("Missing platform key")?;
        let url = format!("{}/v1/nodes/{}/audit/events", state.base_url, node_id);

        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", platform_key))
            .header("x-workspace-api-key", api_key)
            .json(&serde_json::json!({ "events": events }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Fleet audit flush failed: {} - {}",
                status, err_text
            ));
        }

        let value: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        Ok(value.get("written").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
    }

    /// Deploys an AgentSpec to the Cloud.
    ///
    /// Cloud ATM applies/rotates its own HMAC signature for stored specs.
    /// We intentionally strip any local signature payload before upload.
    pub async fn deploy_agent(
        &self,
        mut spec: crate::ai::specs::AgentSpec,
    ) -> Result<serde_json::Value, String> {
        self.verify_authenticated_connection().await?;

        // Signature shape for local specs differs from cloud HMAC signature.
        // Clearing it avoids false validation failures on /admin/agents.
        spec.signature = None;

        // Wrap in CreateAgentParams
        let config_json = serde_json::to_value(&spec).map_err(|e| e.to_string())?;

        let params = CreateAgentParams {
            name: spec.soul.name.clone(),
            type_: "v3_spec".to_string(),
            config: config_json,
        };

        // Upload
        self.create_agent(params).await
    }
}
