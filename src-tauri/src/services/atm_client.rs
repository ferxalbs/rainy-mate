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
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            http: Client::new(),
            state: Arc::new(Mutex::new(ATMClientState { base_url, api_key })),
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

    pub async fn verify_authenticated_connection(&self) -> Result<(), String> {
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

    /// Deploys an AgentSpec v2 to the Cloud, signing it first.
    pub async fn deploy_agent(
        &self,
        mut spec: crate::ai::specs::AgentSpec,
    ) -> Result<serde_json::Value, String> {
        use crate::ai::features::security_service::SecurityService;

        self.verify_authenticated_connection().await?;

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
