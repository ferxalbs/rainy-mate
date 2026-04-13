use crate::ai::provider_types::{ProviderStreamUsage, ProviderToolLifecycleEvent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeContentStreamKind {
    AssistantText,
    ReasoningText,
    PlanText,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeContentDelta {
    pub stream_kind: RuntimeContentStreamKind,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum RuntimeStreamEvent {
    TurnStarted {
        tool_streaming: bool,
    },
    ContentDelta(RuntimeContentDelta),
    ToolCallLifecycle(ProviderToolLifecycleEvent),
    Usage(ProviderStreamUsage),
    Warning(String),
    TurnCompleted {
        #[serde(skip_serializing_if = "Option::is_none")]
        finish_reason: Option<String>,
    },
    Raw(serde_json::Value),
}

pub type RuntimeEventCallback = Arc<dyn Fn(RuntimeStreamEvent) + Send + Sync>;
