use crate::ai::agent::events::AgentEvent;
use serde::Serialize;
use std::time::{Duration, Instant};

const FRONTEND_STREAM_FLUSH_MS: u64 = 80;
const FRONTEND_THOUGHT_FLUSH_MS: u64 = 120;
const FRONTEND_STATUS_DEBOUNCE_MS: u64 = 150;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendAgentEvent {
    pub run_id: String,
    pub timestamp_ms: i64,
    #[serde(flatten)]
    pub payload: AgentEvent,
}

/// Controls how agent runtime events are projected to the frontend.
/// `Modern` preserves the provider/runtime stream as directly as possible.
/// `AuditLegacy` keeps the older batched/debounced transport for comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontendProjectionMode {
    Modern,
    AuditLegacy,
}

pub struct FrontendEventProjector {
    mode: FrontendProjectionMode,
    buffered_stream: String,
    buffered_reasoning: String,
    last_stream_emit: Option<Instant>,
    last_thought_emit: Option<Instant>,
    last_status_emit: Option<Instant>,
    last_status_text: Option<String>,
}

impl FrontendEventProjector {
    pub fn new(mode: FrontendProjectionMode) -> Self {
        Self {
            mode,
            buffered_stream: String::new(),
            buffered_reasoning: String::new(),
            last_stream_emit: None,
            last_thought_emit: None,
            last_status_emit: None,
            last_status_text: None,
        }
    }

    pub fn project(&mut self, event: &AgentEvent) -> Vec<AgentEvent> {
        if self.mode == FrontendProjectionMode::Modern {
            return Self::project_modern(event);
        }

        let now = Instant::now();
        match event {
            AgentEvent::StreamChunk(chunk) => {
                self.buffered_stream.push_str(chunk);
                if self.should_emit_stream(now) {
                    self.flush_stream(now)
                } else {
                    Vec::new()
                }
            }
            AgentEvent::Thought(_) => {
                let mut projected = self.flush_stream(now);
                if self.should_emit_thought(now) {
                    self.last_thought_emit = Some(now);
                    projected.push(event.clone());
                }
                projected
            }
            AgentEvent::Reasoning(delta) => {
                self.buffered_reasoning.push_str(delta);
                if self.should_emit_thought(now) {
                    let mut projected = self.flush_stream(now);
                    if let Some(reasoning) = self.flush_reasoning(now) {
                        projected.push(reasoning);
                    }
                    projected
                } else {
                    Vec::new()
                }
            }
            AgentEvent::Status(text) => {
                let mut projected = self.flush_stream(now);
                if let Some(reasoning) = self.flush_reasoning(now) {
                    projected.push(reasoning);
                }
                if self.should_emit_status(text, now) {
                    self.last_status_emit = Some(now);
                    self.last_status_text = Some(text.clone());
                    projected.push(event.clone());
                }
                projected
            }
            _ => {
                let mut projected = self.flush_stream(now);
                if let Some(reasoning) = self.flush_reasoning(now) {
                    projected.push(reasoning);
                }
                projected.push(event.clone());
                projected
            }
        }
    }

    pub fn flush_pending(&mut self) -> Vec<AgentEvent> {
        if self.mode == FrontendProjectionMode::Modern {
            return Vec::new();
        }

        let now = Instant::now();
        let mut projected = self.flush_stream(now);
        if let Some(reasoning) = self.flush_reasoning(now) {
            projected.push(reasoning);
        }
        projected
    }

    fn flush_stream(&mut self, now: Instant) -> Vec<AgentEvent> {
        if self.buffered_stream.is_empty() {
            return Vec::new();
        }

        self.last_stream_emit = Some(now);
        vec![AgentEvent::StreamChunk(std::mem::take(
            &mut self.buffered_stream,
        ))]
    }

    fn flush_reasoning(&mut self, now: Instant) -> Option<AgentEvent> {
        if self.buffered_reasoning.is_empty() {
            return None;
        }

        self.last_thought_emit = Some(now);
        Some(AgentEvent::Reasoning(std::mem::take(
            &mut self.buffered_reasoning,
        )))
    }

    fn should_emit_stream(&self, now: Instant) -> bool {
        self.last_stream_emit.is_none_or(|last| {
            now.duration_since(last) >= Duration::from_millis(FRONTEND_STREAM_FLUSH_MS)
        })
    }

    fn should_emit_thought(&self, now: Instant) -> bool {
        self.last_thought_emit.is_none_or(|last| {
            now.duration_since(last) >= Duration::from_millis(FRONTEND_THOUGHT_FLUSH_MS)
        })
    }

    fn should_emit_status(&self, text: &str, now: Instant) -> bool {
        if Self::is_priority_status(text) {
            return true;
        }

        let duplicate = self.last_status_text.as_deref() == Some(text);
        let recent = self.last_status_emit.is_some_and(|last| {
            now.duration_since(last) < Duration::from_millis(FRONTEND_STATUS_DEBOUNCE_MS)
        });

        !(duplicate && recent)
    }

    fn is_priority_status(text: &str) -> bool {
        if text.starts_with("RAG_TELEMETRY:") || text.starts_with("CONTEXT_COMPACTION:") {
            return true;
        }

        let lower = text.to_lowercase();
        lower.contains("approval")
            || lower.contains("failed")
            || lower.contains("error")
            || lower.contains("cancel")
            || lower.contains("terminated")
    }

    fn project_modern(event: &AgentEvent) -> Vec<AgentEvent> {
        match event {
            AgentEvent::Status(text) => Self::translate_status(text)
                .map(|translated| vec![translated])
                .unwrap_or_else(|| vec![event.clone()]),
            _ => vec![event.clone()],
        }
    }

    fn translate_status(text: &str) -> Option<AgentEvent> {
        if let Some(raw) = text.strip_prefix("RAG_TELEMETRY:") {
            let parsed = serde_json::from_str::<serde_json::Value>(raw).ok()?;
            return Some(AgentEvent::RagTelemetry(
                crate::ai::agent::events::RagTelemetryPayload {
                    history_source: parsed
                        .get("history_source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("persisted_long_chat")
                        .to_string(),
                    retrieval_mode: parsed
                        .get("retrieval_mode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unavailable")
                        .to_string(),
                    embedding_profile: parsed
                        .get("embedding_profile")
                        .and_then(|v| v.as_str())
                        .unwrap_or(crate::services::memory_vault::types::EMBEDDING_MODEL)
                        .to_string(),
                },
            ));
        }

        if let Some(raw) = text.strip_prefix("CONTEXT_COMPACTION:") {
            let parsed = serde_json::from_str::<serde_json::Value>(raw).ok()?;
            let trigger_tokens = parsed
                .get("trigger_tokens")
                .and_then(|v| v.as_u64())
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or_default();
            return Some(AgentEvent::ContextCompaction(
                crate::ai::agent::events::ContextCompactionPayload {
                    applied: parsed
                        .get("applied")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    trigger_tokens,
                    source_estimated_tokens: parsed
                        .get("source_estimated_tokens")
                        .and_then(|v| v.as_u64())
                        .and_then(|value| u32::try_from(value).ok()),
                    source_message_count: parsed
                        .get("source_message_count")
                        .and_then(|v| v.as_u64())
                        .and_then(|value| usize::try_from(value).ok()),
                    kept_recent_count: parsed
                        .get("kept_recent_count")
                        .and_then(|v| v.as_u64())
                        .and_then(|value| usize::try_from(value).ok()),
                    compression_model: parsed
                        .get("compression_model")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    best_practice: parsed
                        .get("best_practice")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                },
            ));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::{FrontendEventProjector, FrontendProjectionMode};
    use crate::ai::agent::events::AgentEvent;

    #[test]
    fn batches_stream_chunks_until_boundary() {
        let mut projector = FrontendEventProjector::new(FrontendProjectionMode::AuditLegacy);

        assert!(matches!(
            projector.project(&AgentEvent::StreamChunk("hello".to_string())).as_slice(),
            [AgentEvent::StreamChunk(chunk)] if chunk == "hello"
        ));
        assert!(projector
            .project(&AgentEvent::StreamChunk(" world".to_string()))
            .is_empty());
        assert!(matches!(
            projector.project(&AgentEvent::Status("checkpoint".to_string())).as_slice(),
            [AgentEvent::StreamChunk(chunk), AgentEvent::Status(status)]
                if chunk == " world" && status == "checkpoint"
        ));
    }

    #[test]
    fn debounces_duplicate_statuses() {
        let mut projector = FrontendEventProjector::new(FrontendProjectionMode::AuditLegacy);
        let first = projector.project(&AgentEvent::Status("Planning".to_string()));
        let second = projector.project(&AgentEvent::Status("Planning".to_string()));

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    #[test]
    fn modern_mode_translates_telemetry_statuses() {
        let mut projector = FrontendEventProjector::new(FrontendProjectionMode::Modern);
        let events = projector.project(&AgentEvent::Status(
            "RAG_TELEMETRY:{\"history_source\":\"persisted_long_chat\",\"retrieval_mode\":\"ann\",\"embedding_profile\":\"gemini-embedding-001\"}".to_string(),
        ));

        assert!(matches!(
            events.as_slice(),
            [AgentEvent::RagTelemetry(payload)]
                if payload.retrieval_mode == "ann"
                    && payload.embedding_profile == "gemini-embedding-001"
        ));
    }

    #[test]
    fn modern_mode_keeps_stream_chunks_unbuffered() {
        let mut projector = FrontendEventProjector::new(FrontendProjectionMode::Modern);
        let first = projector.project(&AgentEvent::StreamChunk("hello".to_string()));
        let second = projector.project(&AgentEvent::StreamChunk(" world".to_string()));

        assert!(matches!(
            first.as_slice(),
            [AgentEvent::StreamChunk(chunk)] if chunk == "hello"
        ));
        assert!(matches!(
            second.as_slice(),
            [AgentEvent::StreamChunk(chunk)] if chunk == " world"
        ));
    }
}
