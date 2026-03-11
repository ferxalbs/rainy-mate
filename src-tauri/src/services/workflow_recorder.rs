use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use uuid::Uuid;

const MAX_STEPS_PER_RECORDING: usize = 400;
const MAX_HISTORY_RECORDINGS: usize = 50;
const MAX_TITLE_CHARS: usize = 120;
const MAX_KIND_CHARS: usize = 64;
const MAX_LABEL_CHARS: usize = 512;
const MAX_PAYLOAD_BYTES: usize = 24 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRecordedStep {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub payload: Option<serde_json::Value>,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedWorkflow {
    pub id: String,
    pub title: String,
    pub started_at_ms: i64,
    pub stopped_at_ms: Option<i64>,
    pub step_count: usize,
    pub steps: Vec<WorkflowRecordedStep>,
}

#[derive(Default)]
struct RecorderState {
    active: Option<RecordedWorkflow>,
    history: HashMap<String, RecordedWorkflow>,
}

#[derive(Clone, Default)]
pub struct WorkflowRecorderService {
    state: Arc<Mutex<RecorderState>>,
}

impl WorkflowRecorderService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start_recording(&self, title: Option<String>) -> Result<RecordedWorkflow, String> {
        let mut lock = self.state.lock().await;
        if lock.active.is_some() {
            return Err("A recording is already active".to_string());
        }

        let normalized_title = title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(truncate_to_char_boundary)
            .unwrap_or_else(|| "New Recorded Workflow".to_string());

        let now = now_ms();
        let workflow = RecordedWorkflow {
            id: format!("rec_{}", Uuid::new_v4()),
            title: truncate_with_limit(normalized_title, MAX_TITLE_CHARS),
            started_at_ms: now,
            stopped_at_ms: None,
            step_count: 0,
            steps: Vec::new(),
        };

        lock.active = Some(workflow.clone());
        Ok(workflow)
    }

    pub async fn record_step(
        &self,
        kind: String,
        label: String,
        payload: Option<serde_json::Value>,
    ) -> Result<WorkflowRecordedStep, String> {
        let mut lock = self.state.lock().await;
        let active = lock
            .active
            .as_mut()
            .ok_or_else(|| "No active recording session".to_string())?;
        if active.steps.len() >= MAX_STEPS_PER_RECORDING {
            return Err(format!(
                "Recording step limit reached (max {})",
                MAX_STEPS_PER_RECORDING
            ));
        }

        let normalized_kind = kind.trim().to_string();
        if normalized_kind.is_empty() {
            return Err("Recording step kind is required".to_string());
        }
        if !is_allowed_kind(&normalized_kind) {
            return Err(format!("Unsupported recording step kind '{}'", normalized_kind));
        }
        let normalized_label = label.trim().to_string();
        if normalized_label.is_empty() {
            return Err("Recording step label is required".to_string());
        }
        let serialized_payload_bytes = payload
            .as_ref()
            .map(|value| value.to_string().len())
            .unwrap_or(0);
        if serialized_payload_bytes > MAX_PAYLOAD_BYTES {
            return Err(format!(
                "Recording step payload too large ({} bytes, max {})",
                serialized_payload_bytes, MAX_PAYLOAD_BYTES
            ));
        }

        let step = WorkflowRecordedStep {
            id: format!("step_{}", Uuid::new_v4()),
            kind: truncate_with_limit(normalized_kind, MAX_KIND_CHARS),
            label: truncate_with_limit(normalized_label, MAX_LABEL_CHARS),
            payload,
            timestamp_ms: now_ms(),
        };

        active.steps.push(step.clone());
        active.step_count = active.steps.len();
        Ok(step)
    }

    pub async fn stop_recording(&self) -> Result<RecordedWorkflow, String> {
        let mut lock = self.state.lock().await;
        let mut active = lock
            .active
            .take()
            .ok_or_else(|| "No active recording session".to_string())?;

        active.stopped_at_ms = Some(now_ms());
        active.step_count = active.steps.len();

        lock.history.insert(active.id.clone(), active.clone());
        trim_history_if_needed(&mut lock.history);
        Ok(active)
    }

    pub async fn get_recording(&self, id: &str) -> Option<RecordedWorkflow> {
        let lock = self.state.lock().await;
        if let Some(active) = lock.active.as_ref() {
            if active.id == id {
                return Some(active.clone());
            }
        }
        lock.history.get(id).cloned()
    }

    pub async fn active_recording(&self) -> Option<RecordedWorkflow> {
        let lock = self.state.lock().await;
        lock.active.clone()
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn trim_history_if_needed(history: &mut HashMap<String, RecordedWorkflow>) {
    if history.len() <= MAX_HISTORY_RECORDINGS {
        return;
    }
    if let Some(oldest_id) = history
        .iter()
        .min_by_key(|(_, workflow)| workflow.started_at_ms)
        .map(|(id, _)| id.clone())
    {
        history.remove(&oldest_id);
    }
}

fn truncate_with_limit(value: String, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn truncate_to_char_boundary(value: &str) -> String {
    value.to_string()
}

fn is_allowed_kind(kind: &str) -> bool {
    matches!(
        kind,
        "user_instruction"
            | "agent_run"
            | "tool_call"
            | "tool_result"
            | "status"
            | "agent_generated"
            | "agent_run_requested"
    )
}

#[cfg(test)]
mod tests {
    use super::WorkflowRecorderService;

    #[tokio::test]
    async fn start_record_stop_roundtrip() {
        let service = WorkflowRecorderService::new();

        let started = service
            .start_recording(Some("Smoke".to_string()))
            .await
            .expect("start recording should work");

        service
            .record_step(
                "tool_call".to_string(),
                "read_file".to_string(),
                Some(serde_json::json!({"path":"README.md"})),
            )
            .await
            .expect("record step should work");

        let stopped = service
            .stop_recording()
            .await
            .expect("stop recording should work");

        assert_eq!(started.id, stopped.id);
        assert_eq!(stopped.step_count, 1);
        assert!(stopped.stopped_at_ms.is_some());

        let fetched = service
            .get_recording(&stopped.id)
            .await
            .expect("recording should be persisted");
        assert_eq!(fetched.step_count, 1);
    }

    #[tokio::test]
    async fn rejects_invalid_step_kind() {
        let service = WorkflowRecorderService::new();
        service
            .start_recording(Some("Smoke".to_string()))
            .await
            .expect("start recording should work");

        let err = service
            .record_step("unknown".to_string(), "bad".to_string(), None)
            .await
            .expect_err("invalid kind should fail");
        assert!(err.contains("Unsupported recording step kind"));
    }
}
