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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RainyPayload {
    pub skill: Option<String>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
    pub content: Option<String>,
    /// Allowed paths for this command (from workspace config)
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    /// Optional workspace tool access policy (deny-first) pushed by Cloud.
    #[serde(default)]
    pub tool_access_policy: Option<ToolAccessPolicy>,
    /// Optional monotonically increasing workspace tool policy version.
    #[serde(default)]
    pub tool_access_policy_version: Option<u64>,
    /// Optional SHA-256 hash of canonicalized tool policy.
    #[serde(default)]
    pub tool_access_policy_hash: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::DesktopNodeStatus;

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
