use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ──────────────────────────────────────────────────────────────────────────
// Desktop Node (Nerve Center)
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DesktopNodeStatus {
    Online,
    Busy,
    Offline,
}

// @RESERVED - Will be used when listing connected nodes in UI
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNode {
    pub id: String,
    pub workspace_id: String,
    pub hostname: String,
    pub platform: String, // "darwin" | "win32" | "linux"
    pub skills_manifest: Vec<SkillManifest>,
    pub status: DesktopNodeStatus,
    pub last_heartbeat: i64,
    pub paired_at: i64,
    #[serde(default)]
    pub runtime_stats: RuntimeStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub methods: Vec<SkillMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMethod {
    pub name: String,
    pub description: String,
    pub airlock_level: AirlockLevel,
    pub parameters: HashMap<String, ParameterSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterSchema {
    #[serde(rename = "type")]
    pub param_type: String, // "string" | "number" | ...
    pub required: Option<bool>,
    pub description: Option<String>,
}

// ──────────────────────────────────────────────────────────────────────────
// The Airlock (Security Firewall)
// ──────────────────────────────────────────────────────────────────────────

use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, PartialOrd, Default)]
#[repr(u8)]
pub enum AirlockLevel {
    #[default]
    Safe = 0,
    Sensitive = 1,
    Dangerous = 2,
}

// ──────────────────────────────────────────────────────────────────────────
// RainyRPC Protocol (Cloud <-> Desktop)
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RainyIntent {
    Chat,
    Execute,
    Steer,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CommandPriority {
    High,
    #[default]
    Normal,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CommandStatus {
    #[default]
    Pending,
    Approved,
    Running,
    Completed,
    Failed,
    Rejected,
}

// @RESERVED - Full RainyRPC message format for Pub/Sub integration
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RainyMessage {
    pub id: String,
    pub timestamp: i64,
    pub intent: RainyIntent,
    pub context: RainyContext,
    pub payload: RainyPayload,
    pub signature: String,
}

// @RESERVED - Context for RainyMessage, used in Pub/Sub integration
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RainyContext {
    pub user_id: String,
    pub workspace_id: String,
    pub session_id: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RainyPayload {
    pub skill: Option<String>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
    pub content: Option<String>,
    /// Allowed paths for this command (from workspace config)
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    /// Blocked paths for this command (from agent Airlock scopes)
    #[serde(default)]
    pub blocked_paths: Vec<String>,
    /// Allowlist of domains for web/browser operations (empty = no allowlist)
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Denylist of domains for web/browser operations
    #[serde(default)]
    pub blocked_domains: Vec<String>,
    /// Optional workspace tool access policy (deny-first) pushed by Cloud.
    #[serde(default)]
    pub tool_access_policy: Option<ToolAccessPolicy>,
    /// Optional monotonically increasing workspace tool policy version.
    #[serde(default)]
    pub tool_access_policy_version: Option<u64>,
    /// Optional SHA-256 hash of canonicalized tool policy.
    #[serde(default)]
    pub tool_access_policy_hash: Option<String>,
    /// Connector that originated the command (e.g. "telegram", "discord", "whatsapp").
    #[serde(default)]
    pub connector_id: Option<String>,
    /// End-user identifier from the connector (peer phone, user ID, etc.).
    #[serde(default)]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAccessPolicy {
    pub enabled: bool,
    pub mode: String,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedCommand {
    pub id: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub desktop_node_id: Option<String>,
    /// The intent format is "skill.method" e.g. "filesystem.list_files"
    pub intent: String,
    pub payload: RainyPayload,
    #[serde(default)]
    pub priority: CommandPriority,
    #[serde(default)]
    pub status: CommandStatus,
    #[serde(default)]
    pub airlock_level: AirlockLevel,
    /// Approval timeout override in seconds.
    /// `None` = use Airlock defaults, `Some(0)` = wait indefinitely.
    #[serde(default)]
    pub approval_timeout_secs: Option<u64>,
    #[serde(default)]
    pub approved_by: Option<String>,
    #[serde(default)]
    pub result: Option<CommandResult>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub started_at: Option<i64>,
    #[serde(default)]
    pub completed_at: Option<i64>,
    /// Schema version for ATM <-> Desktop contract validation.
    /// Absent on older payloads; bump when the envelope shape changes in a breaking way.
    #[serde(default)]
    pub schema_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStats {
    pub active_supervisor_runs: usize,
    pub active_specialists: usize,
    #[serde(default)]
    pub supervisors: Vec<SupervisorRunStatus>,
    #[serde(default)]
    pub tool_usage_by_role: ToolUsageByRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupervisorRunStatus {
    pub run_id: String,
    pub status: String,
    pub specialist_count: usize,
    #[serde(default)]
    pub completed_specialists: usize,
    #[serde(default)]
    pub failed_specialists: usize,
    #[serde(default)]
    pub specialists: Vec<SpecialistRuntimeStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolUsageByRole {
    pub research: u64,
    pub executor: u64,
    pub verifier: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistRuntimeStatus {
    pub agent_id: String,
    pub role: String,
    pub status: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub active_tool: Option<String>,
    #[serde(default)]
    pub started_at_ms: Option<i64>,
    #[serde(default)]
    pub finished_at_ms: Option<i64>,
    #[serde(default)]
    pub tool_count: u32,
    #[serde(default)]
    pub write_like_used: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Canonical ATM <-> Desktop contract fixture (inlined — no external file dependency).
    // Keep in sync with the fixture object in rainy-atm/src/integrations/atm-contract.test.ts.
    // Any change that breaks these tests means the contract has changed and both sides must be updated.
    const FIXTURE: &str = r#"{
        "id": "cmd-contract-001",
        "workspaceId": "ws-contract",
        "desktopNodeId": "node-contract",
        "intent": "agent.run",
        "payload": {
            "skill": "agent",
            "method": "run",
            "params": { "attachments": [] },
            "content": "List files in the workspace",
            "allowedPaths": ["/workspace"],
            "blockedPaths": ["/etc"],
            "allowedDomains": ["example.com"],
            "blockedDomains": [],
            "toolAccessPolicy": null,
            "toolAccessPolicyVersion": null,
            "toolAccessPolicyHash": null,
            "connectorId": "telegram",
            "userId": "user-contract"
        },
        "priority": "normal",
        "status": "pending",
        "airlockLevel": 0,
        "approvalTimeoutSecs": null,
        "approvedBy": null,
        "result": null,
        "createdAt": 1743000000000,
        "startedAt": null,
        "completedAt": null,
        "schemaVersion": "1"
    }"#;

    #[test]
    fn atm_queued_command_fixture_deserializes_without_field_loss() {
        let cmd: QueuedCommand =
            serde_json::from_str(FIXTURE).expect("fixture must deserialize into QueuedCommand");

        assert_eq!(cmd.id, "cmd-contract-001");
        assert_eq!(cmd.workspace_id.as_deref(), Some("ws-contract"));
        assert_eq!(cmd.desktop_node_id.as_deref(), Some("node-contract"));
        assert_eq!(cmd.intent, "agent.run");
        assert!(matches!(cmd.priority, CommandPriority::Normal));
        assert!(matches!(cmd.status, CommandStatus::Pending));
        assert!(matches!(cmd.airlock_level, AirlockLevel::Safe));
        assert_eq!(cmd.created_at, Some(1743000000000));
        assert_eq!(cmd.schema_version.as_deref(), Some("1"));
    }

    #[test]
    fn atm_queued_command_fixture_payload_fields_preserved() {
        let cmd: QueuedCommand =
            serde_json::from_str(FIXTURE).expect("fixture must deserialize into QueuedCommand");

        assert_eq!(cmd.payload.skill.as_deref(), Some("agent"));
        assert_eq!(cmd.payload.method.as_deref(), Some("run"));
        assert_eq!(
            cmd.payload.content.as_deref(),
            Some("List files in the workspace")
        );
        assert_eq!(cmd.payload.allowed_paths, vec!["/workspace"]);
        assert_eq!(cmd.payload.blocked_paths, vec!["/etc"]);
        assert_eq!(cmd.payload.allowed_domains, vec!["example.com"]);
        assert!(cmd.payload.blocked_domains.is_empty());
        assert_eq!(cmd.payload.connector_id.as_deref(), Some("telegram"));
        assert_eq!(cmd.payload.user_id.as_deref(), Some("user-contract"));
    }

    #[test]
    fn atm_queued_command_fixture_round_trips_without_field_loss() {
        let cmd: QueuedCommand =
            serde_json::from_str(FIXTURE).expect("fixture must deserialize");
        let re_serialized =
            serde_json::to_value(&cmd).expect("re-serialize must succeed");
        let fixture_value: serde_json::Value =
            serde_json::from_str(FIXTURE).expect("fixture must be valid JSON");

        // All non-null fields in the fixture must survive the round-trip.
        // Null fields in the fixture may serialize as absent (serde skip_serializing_if).
        for (key, fixture_val) in fixture_value.as_object().unwrap() {
            if fixture_val.is_null() {
                continue; // null → Option::None → may be omitted in output
            }
            assert_eq!(
                re_serialized.get(key),
                Some(fixture_val),
                "field '{key}' was lost or changed during round-trip"
            );
        }
    }

    #[test]
    fn desktop_node_status_accepts_runtime_values() {
        let online: DesktopNodeStatus =
            serde_json::from_str("\"online\"").expect("online should deserialize");
        let busy: DesktopNodeStatus =
            serde_json::from_str("\"busy\"").expect("busy should deserialize");
        let offline: DesktopNodeStatus =
            serde_json::from_str("\"offline\"").expect("offline should deserialize");

        assert!(matches!(online, DesktopNodeStatus::Online));
        assert!(matches!(busy, DesktopNodeStatus::Busy));
        assert!(matches!(offline, DesktopNodeStatus::Offline));
    }

    #[test]
    fn desktop_node_status_rejects_ui_status_values() {
        assert!(serde_json::from_str::<DesktopNodeStatus>("\"connected\"").is_err());
        assert!(serde_json::from_str::<DesktopNodeStatus>("\"pending-pairing\"").is_err());
        assert!(serde_json::from_str::<DesktopNodeStatus>("\"error\"").is_err());
    }
}
