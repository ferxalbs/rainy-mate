use super::protocol::{SpecialistRole, SpecialistStatus, SupervisorPlan};
use crate::ai::provider_types::{
    ProviderStreamUsage, ProviderToolLifecycleState, ToolCall,
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistEventPayload {
    pub run_id: String,
    pub agent_id: String,
    pub role: SpecialistRole,
    pub status: SpecialistStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u8>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_like_used: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistCompletedPayload {
    pub run_id: String,
    pub agent_id: String,
    pub role: SpecialistRole,
    pub summary: String,
    pub response_preview: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u8>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub tool_count: u32,
    pub write_like_used: bool,
    pub started_at_ms: i64,
    pub finished_at_ms: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistFailedPayload {
    pub run_id: String,
    pub agent_id: String,
    pub role: SpecialistRole,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u8>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_like_used: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupervisorSummaryPayload {
    pub run_id: String,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamToolCallPayload {
    pub state: ProviderToolLifecycleState,
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AgentEvent {
    Status(String),
    Thought(String),
    StreamChunk(String),
    StreamToolCall(StreamToolCallPayload),
    Usage(ProviderStreamUsage),
    ToolCall(ToolCall),
    ToolResult {
        id: String,
        result: String,
    },
    // @RESERVED: runtime error path not yet emitted but match arms handle it
    #[allow(dead_code)]
    Error(String),
    MemoryStored(String),
    SupervisorPlanCreated(SupervisorPlan),
    SpecialistSpawned(SpecialistEventPayload),
    SpecialistStatusChanged(SpecialistEventPayload),
    SpecialistCompleted(SpecialistCompletedPayload),
    SpecialistFailed(SpecialistFailedPayload),
    SupervisorSummary(SupervisorSummaryPayload),
}
