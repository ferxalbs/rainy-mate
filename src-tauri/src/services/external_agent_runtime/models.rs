use crate::services::chat_artifacts::ChatArtifact;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExternalRuntimeKind {
    Codex,
    Claude,
}

impl ExternalRuntimeKind {
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalRuntimeAvailability {
    pub runtime_kind: ExternalRuntimeKind,
    pub installed: bool,
    pub binary_name: String,
    pub binary_path: Option<String>,
    pub install_hint: String,
    pub status_message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalAgentSessionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAgentAuditEventType {
    SessionCreated,
    SessionStarted,
    StdoutChunk,
    StderrChunk,
    FileTouched,
    ArtifactEmitted,
    SessionCompleted,
    SessionFailed,
    SessionCancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentAuditEvent {
    pub event_type: ExternalAgentAuditEventType,
    pub message: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAgentSession {
    pub session_id: String,
    pub runtime_kind: ExternalRuntimeKind,
    pub workspace_path: String,
    pub task_summary: String,
    pub launch_command_preview: Option<String>,
    pub status: ExternalAgentSessionStatus,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub last_message: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    #[serde(default)]
    pub touched_paths: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<ChatArtifact>,
    #[serde(default)]
    pub audit_events: Vec<ExternalAgentAuditEvent>,
}

#[derive(Debug, Clone)]
pub struct NewExternalAgentSession {
    pub runtime_kind: ExternalRuntimeKind,
    pub workspace_path: String,
    pub task_summary: String,
}
