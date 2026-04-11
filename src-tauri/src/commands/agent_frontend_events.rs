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

#[derive(Default)]
pub struct FrontendEventProjector {
    buffered_stream: String,
    buffered_reasoning: String,
    last_stream_emit: Option<Instant>,
    last_thought_emit: Option<Instant>,
    last_status_emit: Option<Instant>,
    last_status_text: Option<String>,
}

impl FrontendEventProjector {
    pub fn project(&mut self, event: &AgentEvent) -> Vec<AgentEvent> {
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
}

#[cfg(test)]
mod tests {
    use super::FrontendEventProjector;
    use crate::ai::agent::events::AgentEvent;

    #[test]
    fn batches_stream_chunks_until_boundary() {
        let mut projector = FrontendEventProjector::default();

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
        let mut projector = FrontendEventProjector::default();
        let first = projector.project(&AgentEvent::Status("Planning".to_string()));
        let second = projector.project(&AgentEvent::Status("Planning".to_string()));

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }
}
