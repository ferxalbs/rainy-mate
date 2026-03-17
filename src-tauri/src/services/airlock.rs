//! Airlock Security Service
//!
//! The Airlock is a security firewall that controls command execution
//! based on risk levels. Commands from the Cloud Cortex must pass through
//! the Airlock before being executed on the Desktop.
//!
//! ## Security Levels
//! - **Level 0 (Safe)**: Read-only operations - auto-approved
//! - **Level 1 (Sensitive)**: Write operations - requires notification
//! - **Level 2 (Dangerous)**: Execution operations - requires explicit approval

use crate::models::neural::{AirlockLevel, QueuedCommand};
use crate::services::ThirdPartySkillRegistry;
use crate::services::tool_policy::get_tool_policy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{oneshot, Mutex};

// Used when emitting approval request events to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub command_id: String,
    pub intent: String,
    pub tool_name: Option<String>,
    pub payload_summary: String,
    pub airlock_level: AirlockLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    pub timestamp: i64,
}

/// Result of an approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalResult {
    Approved,
    Rejected,
    Timeout,
}

struct PendingApproval {
    request: ApprovalRequest,
    responder: oneshot::Sender<ApprovalResult>,
}

// Used during command execution
#[derive(Clone)]
pub struct AirlockService {
    app: AppHandle,
    pending_approvals: Arc<Mutex<HashMap<String, PendingApproval>>>,
    headless_mode: Arc<AtomicBool>,
}

impl std::fmt::Debug for AirlockService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AirlockService")
            .field("headless_mode", &self.headless_mode)
            .finish()
    }
}

impl AirlockService {
    fn default_approval_timeout_secs(effective_level: AirlockLevel) -> u64 {
        if effective_level == AirlockLevel::Dangerous {
            30
        } else {
            10
        }
    }

    fn resolve_approval_timeout_secs(
        command: &QueuedCommand,
        effective_level: AirlockLevel,
    ) -> Option<u64> {
        match command.approval_timeout_secs {
            Some(0) => None,
            Some(value) => Some(value),
            None => Some(Self::default_approval_timeout_secs(effective_level)),
        }
    }

    fn summarize_payload(command: &QueuedCommand) -> String {
        let params = command.payload.params.as_ref();
        let path = params
            .and_then(|value| value.get("path"))
            .and_then(|value| value.as_str());
        let url = params
            .and_then(|value| value.get("url"))
            .and_then(|value| value.as_str());
        let query = params
            .and_then(|value| value.get("query"))
            .and_then(|value| value.as_str());

        if let Some(path) = path {
            return format!("path={}", path);
        }
        if let Some(url) = url {
            return format!("url={}", url);
        }
        if let Some(query) = query {
            return format!("query={}", query);
        }
        if let Some(content) = command.payload.content.as_deref() {
            let compact = content.chars().take(120).collect::<String>();
            if !compact.is_empty() {
                return compact;
            }
        }

        serde_json::to_string(&command.payload.params).unwrap_or_default()
    }

    fn is_agent_run_bootstrap(command: &QueuedCommand) -> bool {
        if command.intent == "agent.run" {
            return true;
        }
        matches!(
            (
                command.payload.skill.as_deref(),
                command.payload.method.as_deref()
            ),
            (Some("agent"), Some("run"))
        )
    }

    fn infer_tool_name(command: &QueuedCommand) -> Option<String> {
        if Self::is_agent_run_bootstrap(command) {
            return Some("agent.run".to_string());
        }
        if let Some(method) = command.payload.method.as_ref() {
            return Some(method.clone());
        }
        command
            .intent
            .rsplit('.')
            .next()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    }

    fn is_mcp_tool(command: &QueuedCommand) -> bool {
        command
            .payload
            .method
            .as_ref()
            .map(|method| crate::services::mcp_service::McpService::is_mcp_tool(method))
            .unwrap_or(false)
    }

    fn effective_airlock_level(command: &QueuedCommand) -> AirlockLevel {
        if Self::is_agent_run_bootstrap(command) {
            return AirlockLevel::Safe;
        }
        let declared = command.airlock_level;
        let policy_level = Self::infer_tool_name(command)
            .and_then(|tool| {
                get_tool_policy(&tool)
                    .map(|policy| policy.airlock_level)
                    .or_else(|| Self::third_party_tool_level(&tool))
            })
            .unwrap_or(AirlockLevel::Dangerous);

        if policy_level > declared {
            policy_level
        } else {
            declared
        }
    }

    fn insert_pending_approval(
        pending: &mut HashMap<String, PendingApproval>,
        request: ApprovalRequest,
        responder: oneshot::Sender<ApprovalResult>,
    ) {
        pending.insert(
            request.command_id.clone(),
            PendingApproval { request, responder },
        );
    }

    fn remove_pending_approval(
        pending: &mut HashMap<String, PendingApproval>,
        command_id: &str,
    ) -> Option<PendingApproval> {
        pending.remove(command_id)
    }

    fn list_pending_approvals(pending: &HashMap<String, PendingApproval>) -> Vec<ApprovalRequest> {
        let mut approvals: Vec<ApprovalRequest> = pending
            .values()
            .map(|entry| entry.request.clone())
            .collect();
        approvals.sort_by_key(|request| request.timestamp);
        approvals
    }

    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
            headless_mode: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_headless_mode(&self, enabled: bool) {
        self.headless_mode.store(enabled, Ordering::Relaxed);
        tracing::info!("Airlock: Headless mode set to {}", enabled);
    }

    pub fn is_headless_mode(&self) -> bool {
        self.headless_mode.load(Ordering::Relaxed)
    }

    // Called during command execution flow
    pub async fn check_permission(&self, command: &QueuedCommand) -> Result<bool, String> {
        if Self::is_agent_run_bootstrap(command) {
            tracing::debug!(
                "Airlock: Auto-approved agent.run bootstrap command {}",
                command.id
            );
            return Ok(true);
        }

        // MCP tools use a dedicated global ask/no-ask gate in McpService.
        // They intentionally bypass 3-level Airlock classification.
        if Self::is_mcp_tool(command) {
            tracing::debug!("Airlock: Bypassing MCP tool {}; delegated to MCP gate", command.id);
            return Ok(true);
        }

        let inferred_tool = Self::infer_tool_name(command);
        let has_policy = inferred_tool
            .as_ref()
            .and_then(|tool| get_tool_policy(tool).map(|_| ()).or_else(|| Self::third_party_tool_level(tool).map(|_| ())))
            .is_some();
        if !has_policy {
            tracing::warn!(
                "Airlock: Denying command {} because tool policy is missing (tool={:?})",
                command.id,
                inferred_tool
            );
            return Ok(false);
        }

        let effective_level = Self::effective_airlock_level(command);
        if effective_level != command.airlock_level {
            tracing::warn!(
                "Airlock: Escalating command {} level from {:?} to {:?} based on tool policy",
                command.id,
                command.airlock_level,
                effective_level
            );
        }

        match effective_level {
            AirlockLevel::Safe => {
                // Level 0: Auto-approve read-only operations
                tracing::debug!("Airlock: Auto-approved SAFE command {}", command.id);
                Ok(true)
            }
            AirlockLevel::Sensitive => {
                // Level 1: Write operations
                if self.is_headless_mode() {
                    tracing::info!(
                        "Airlock: Auto-approved SENSITIVE command {} (Headless Mode)",
                        command.id
                    );
                    Ok(true)
                } else {
                    tracing::info!(
                        "Airlock: SENSITIVE command {} requires notification",
                        command.id
                    );
                    self.request_approval(command, effective_level, false).await
                }
            }
            AirlockLevel::Dangerous => {
                // Level 2: Execution operations
                // Dangerous commands always require explicit approval.
                // Headless mode never bypasses this gate.
                if self.is_headless_mode() {
                    tracing::warn!(
                        "Airlock: DANGEROUS command {} still requires explicit approval (headless mode disabled for this level)",
                        command.id
                    );
                }
                tracing::warn!(
                    "Airlock: DANGEROUS command {} requires explicit approval",
                    command.id
                );
                self.request_approval(command, effective_level, false).await
            }
        }
    }

    fn third_party_tool_level(tool: &str) -> Option<AirlockLevel> {
        ThirdPartySkillRegistry::new()
            .ok()
            .and_then(|registry| registry.find_method_airlock_level(tool).ok().flatten())
    }

    // Internal implementation for user approval flow
    async fn request_approval(
        &self,
        command: &QueuedCommand,
        effective_level: AirlockLevel,
        allow_on_timeout: bool,
    ) -> Result<bool, String> {
        let timeout_secs = Self::resolve_approval_timeout_secs(command, effective_level);
        let now = chrono::Utc::now().timestamp_millis();
        let request = ApprovalRequest {
            command_id: command.id.clone(),
            intent: command.intent.clone(),
            tool_name: Self::infer_tool_name(command),
            payload_summary: Self::summarize_payload(command),
            airlock_level: effective_level,
            timeout_secs,
            expires_at: timeout_secs.map(|value| now + (value as i64 * 1000)),
            timestamp: now,
        };

        let (tx, rx) = oneshot::channel::<ApprovalResult>();

        // Store the pending request and sender so frontend can restore state after reload.
        {
            let mut pending = self.pending_approvals.lock().await;
            Self::insert_pending_approval(&mut pending, request.clone(), tx);
        };

        // Emit event to frontend
        self.app
            .emit("airlock:approval_required", &request)
            .map_err(|e| format!("Failed to emit approval event: {}", e))?;

        let result = if let Some(timeout_secs) = timeout_secs {
            match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await {
                Ok(Ok(result)) => result,
                Ok(Err(_)) | Err(_) => ApprovalResult::Timeout,
            }
        } else {
            match rx.await {
                Ok(result) => result,
                Err(_) => ApprovalResult::Rejected,
            }
        };

        // Clean up pending approval
        {
            let mut pending = self.pending_approvals.lock().await;
            Self::remove_pending_approval(&mut pending, &command.id);
        }

        match result {
            ApprovalResult::Approved => {
                tracing::info!("Airlock: Command {} APPROVED by user", command.id);
                // Notify frontend to clear the request from UI
                let _ = self.app.emit("airlock:approval_resolved", &command.id);
                Ok(true)
            }
            ApprovalResult::Rejected => {
                tracing::info!("Airlock: Command {} REJECTED by user", command.id);
                // Notify frontend to clear the request from UI
                let _ = self.app.emit("airlock:approval_resolved", &command.id);
                Ok(false)
            }
            ApprovalResult::Timeout => {
                // Timeout or channel closed
                // Notify frontend to clear the request from UI
                let _ = self.app.emit("airlock:approval_resolved", &command.id);

                if allow_on_timeout {
                    tracing::warn!(
                        "Airlock: Command {} timed out, allowing due to explicit policy override",
                        command.id
                    );
                    return Ok(true);
                }

                tracing::warn!(
                    "Airlock: Command {} timed out, denying by default",
                    command.id
                );
                Ok(false)
            }
        }
    }

    /// Respond to an approval request (called from frontend via Tauri command)
    pub async fn respond_to_approval(
        &self,
        command_id: &str,
        approved: bool,
    ) -> Result<(), String> {
        let mut pending = self.pending_approvals.lock().await;

        if let Some(entry) = Self::remove_pending_approval(&mut pending, command_id) {
            let result = if approved {
                ApprovalResult::Approved
            } else {
                ApprovalResult::Rejected
            };

            entry
                .responder
                .send(result)
                .map_err(|_| "Channel closed".to_string())?;
            Ok(())
        } else {
            Err(format!("No pending approval for command {}", command_id))
        }
    }

    /// Get all pending approval requests
    pub async fn get_pending_approvals(&self) -> Vec<ApprovalRequest> {
        let pending = self.pending_approvals.lock().await;
        Self::list_pending_approvals(&pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::neural::{
        CommandPriority, CommandStatus, QueuedCommand, RainyPayload, ToolAccessPolicy,
    };
    use rand::{rngs::StdRng, Rng, SeedableRng};

    fn make_request(command_id: &str, timestamp: i64) -> ApprovalRequest {
        ApprovalRequest {
            command_id: command_id.to_string(),
            intent: "filesystem.write_file".to_string(),
            tool_name: Some("write_file".to_string()),
            payload_summary: "{\"path\":\"/tmp/x\"}".to_string(),
            airlock_level: AirlockLevel::Sensitive,
            timeout_secs: Some(10),
            expires_at: Some(timestamp + 10_000),
            timestamp,
        }
    }

    #[tokio::test]
    async fn pending_approvals_are_sorted_by_timestamp() {
        let mut pending: HashMap<String, PendingApproval> = HashMap::new();
        let (tx_old, _rx_old) = oneshot::channel::<ApprovalResult>();
        let (tx_new, _rx_new) = oneshot::channel::<ApprovalResult>();

        AirlockService::insert_pending_approval(&mut pending, make_request("b", 20), tx_new);
        AirlockService::insert_pending_approval(&mut pending, make_request("a", 10), tx_old);

        let listed = AirlockService::list_pending_approvals(&pending);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].command_id, "a");
        assert_eq!(listed[1].command_id, "b");
    }

    #[tokio::test]
    async fn remove_pending_approval_returns_responder_and_cleans_store() {
        let mut pending: HashMap<String, PendingApproval> = HashMap::new();
        let (tx, rx) = oneshot::channel::<ApprovalResult>();
        AirlockService::insert_pending_approval(&mut pending, make_request("cmd-1", 1), tx);

        let entry = AirlockService::remove_pending_approval(&mut pending, "cmd-1")
            .expect("pending approval should exist");
        assert!(pending.is_empty());

        entry
            .responder
            .send(ApprovalResult::Approved)
            .expect("responder send should succeed");
        let result = rx.await.expect("receiver should get approval result");
        assert!(matches!(result, ApprovalResult::Approved));
    }

    #[test]
    fn effective_airlock_level_escalates_when_declared_is_lower_than_policy() {
        let command = QueuedCommand {
            id: "cmd-1".to_string(),
            workspace_id: Some("ws-1".to_string()),
            desktop_node_id: Some("node-1".to_string()),
            intent: "shell.execute_command".to_string(),
            payload: RainyPayload {
                skill: Some("shell".to_string()),
                method: Some("execute_command".to_string()),
                params: None,
                content: None,
                allowed_paths: vec![],
                blocked_paths: vec![],
                allowed_domains: vec![],
                blocked_domains: vec![],
                tool_access_policy: Some(ToolAccessPolicy {
                    enabled: true,
                    mode: "all".to_string(),
                    allow: vec![],
                    deny: vec![],
                }),
                tool_access_policy_version: None,
                tool_access_policy_hash: None,
                ..Default::default()
            },
            priority: CommandPriority::Normal,
            status: CommandStatus::Pending,
            airlock_level: AirlockLevel::Safe,
            approval_timeout_secs: None,
            approved_by: None,
            result: None,
            created_at: None,
            started_at: None,
            completed_at: None,
        };

        let level = AirlockService::effective_airlock_level(&command);
        assert_eq!(level, AirlockLevel::Dangerous);
    }

    fn level_from_u8(value: u8) -> AirlockLevel {
        match value {
            0 => AirlockLevel::Safe,
            1 => AirlockLevel::Sensitive,
            _ => AirlockLevel::Dangerous,
        }
    }

    fn make_command_with_tool(method: &str, declared: AirlockLevel) -> QueuedCommand {
        QueuedCommand {
            id: "cmd-prop".to_string(),
            workspace_id: Some("ws-1".to_string()),
            desktop_node_id: Some("node-1".to_string()),
            intent: format!("tool.{}", method),
            payload: RainyPayload {
                skill: Some("tool".to_string()),
                method: Some(method.to_string()),
                params: None,
                content: None,
                allowed_paths: vec![],
                blocked_paths: vec![],
                allowed_domains: vec![],
                blocked_domains: vec![],
                tool_access_policy: Some(ToolAccessPolicy {
                    enabled: true,
                    mode: "all".to_string(),
                    allow: vec![],
                    deny: vec![],
                }),
                tool_access_policy_version: None,
                tool_access_policy_hash: None,
                ..Default::default()
            },
            priority: CommandPriority::Normal,
            status: CommandStatus::Pending,
            airlock_level: declared,
            approval_timeout_secs: None,
            approved_by: None,
            result: None,
            created_at: None,
            started_at: None,
            completed_at: None,
        }
    }

    #[test]
    fn effective_level_never_downscopes_declared_for_known_tools_property_sweep() {
        let mut rng = StdRng::seed_from_u64(0xA11C0C);
        let tools = [
            "read_file",       // Safe
            "write_file",      // Sensitive
            "execute_command", // Dangerous
            "http_post_json",  // Dangerous
            "browse_url",      // Sensitive
        ];

        for _ in 0..512 {
            let declared = level_from_u8(rng.gen_range(0u8..=2u8));
            let tool = tools[rng.gen_range(0..tools.len())];
            let command = make_command_with_tool(tool, declared);
            let effective = AirlockService::effective_airlock_level(&command);
            assert!(
                effective >= declared,
                "effective={:?} declared={:?} tool={}",
                effective,
                declared,
                tool
            );
        }
    }

    #[test]
    fn unknown_tool_defaults_to_dangerous_property_sweep() {
        let mut rng = StdRng::seed_from_u64(0xD4E6E2);
        let alphabet = b"abcdefghijklmnopqrstuvwxyz0123456789_";

        for _ in 0..512 {
            let declared = level_from_u8(rng.gen_range(0u8..=2u8));
            let suffix_len = rng.gen_range(1usize..=12usize);
            let suffix: String = (0..suffix_len)
                .map(|_| {
                    let idx = rng.gen_range(0..alphabet.len());
                    alphabet[idx] as char
                })
                .collect();

            let method = format!("unknown_{}", suffix);
            let command = make_command_with_tool(&method, declared);
            let effective = AirlockService::effective_airlock_level(&command);
            assert_eq!(
                effective,
                AirlockLevel::Dangerous,
                "effective={:?} declared={:?} method={}",
                effective,
                declared,
                method
            );
        }
    }

    #[test]
    fn infer_tool_name_handles_malformed_intent() {
        let mut command = make_command_with_tool("read_file", AirlockLevel::Safe);
        command.payload.method = None;
        command.intent = "malformed-intent-without-dot".to_string();
        let inferred = AirlockService::infer_tool_name(&command);
        assert_eq!(inferred.as_deref(), Some("malformed-intent-without-dot"));
    }

    #[test]
    fn infer_tool_name_returns_none_when_payload_and_intent_are_empty() {
        let mut command = make_command_with_tool("read_file", AirlockLevel::Safe);
        command.payload.method = None;
        command.intent = String::new();
        let inferred = AirlockService::infer_tool_name(&command);
        assert!(inferred.is_none());
    }

    #[test]
    fn agent_run_bootstrap_is_detected() {
        let mut command = make_command_with_tool("read_file", AirlockLevel::Dangerous);
        command.intent = "agent.run".to_string();
        command.payload.skill = Some("agent".to_string());
        command.payload.method = Some("run".to_string());
        assert!(AirlockService::is_agent_run_bootstrap(&command));
        assert_eq!(
            AirlockService::effective_airlock_level(&command),
            AirlockLevel::Safe
        );
    }

    #[test]
    fn resolve_approval_timeout_uses_default_when_unspecified() {
        let command = make_command_with_tool("write_file", AirlockLevel::Sensitive);
        assert_eq!(
            AirlockService::resolve_approval_timeout_secs(&command, AirlockLevel::Sensitive),
            Some(10)
        );
    }

    #[test]
    fn resolve_approval_timeout_supports_durable_override() {
        let mut command = make_command_with_tool("execute_command", AirlockLevel::Dangerous);
        command.approval_timeout_secs = Some(0);
        assert_eq!(
            AirlockService::resolve_approval_timeout_secs(&command, AirlockLevel::Dangerous),
            None
        );
    }
}
