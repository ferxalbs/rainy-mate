use crate::ai::specs::manifest::{AgentSpec, RuntimeMode};
use crate::ai::specs::soul::AgentSoul;
use crate::ai::specs::skills::AgentSkills;
use crate::services::agent_library::{AgentLibraryEntry, AgentLibraryService};
use crate::services::workflow_recorder::{RecordedWorkflow, WorkflowRecordedStep, WorkflowRecorderService};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{command, State};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordWorkflowStepInput {
    pub kind: String,
    pub label: String,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartWorkflowRecordingInput {
    pub title: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateAgentFromRecordingInput {
    pub recording_id: String,
    pub agent_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeGenerateResponse {
    pub recording: RecordedWorkflow,
    pub generated_spec: AgentSpec,
}

#[command]
pub async fn start_workflow_recording(
    recorder: State<'_, Arc<WorkflowRecorderService>>,
    input: Option<StartWorkflowRecordingInput>,
) -> Result<RecordedWorkflow, String> {
    recorder
        .start_recording(input.and_then(|value| value.title))
        .await
}

#[command]
pub async fn record_workflow_step(
    recorder: State<'_, Arc<WorkflowRecorderService>>,
    input: RecordWorkflowStepInput,
) -> Result<WorkflowRecordedStep, String> {
    recorder
        .record_step(input.kind, input.label, input.payload)
        .await
}

#[command]
pub async fn stop_workflow_recording(
    recorder: State<'_, Arc<WorkflowRecorderService>>,
) -> Result<RecordedWorkflow, String> {
    recorder.stop_recording().await
}

#[command]
pub async fn get_workflow_recording(
    recorder: State<'_, Arc<WorkflowRecorderService>>,
    recording_id: String,
) -> Result<Option<RecordedWorkflow>, String> {
    Ok(recorder.get_recording(&recording_id).await)
}

#[command]
pub async fn get_active_workflow_recording(
    recorder: State<'_, Arc<WorkflowRecorderService>>,
) -> Result<Option<RecordedWorkflow>, String> {
    Ok(recorder.active_recording().await)
}

#[command]
pub async fn generate_agent_spec_from_recording(
    recorder: State<'_, Arc<WorkflowRecorderService>>,
    input: GenerateAgentFromRecordingInput,
) -> Result<ForgeGenerateResponse, String> {
    let recording = recorder
        .get_recording(&input.recording_id)
        .await
        .ok_or_else(|| format!("Recording '{}' not found", input.recording_id))?;

    let resolved_agent_name = input
        .agent_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| recording.title.clone());

    let generated_spec = build_generated_spec(&recording, resolved_agent_name);

    Ok(ForgeGenerateResponse {
        recording,
        generated_spec,
    })
}

#[command]
pub async fn save_generated_agent(
    library: State<'_, Arc<AgentLibraryService>>,
    spec: AgentSpec,
) -> Result<AgentLibraryEntry, String> {
    library.save_spec(&spec)
}

#[command]
pub async fn list_generated_agents(
    library: State<'_, Arc<AgentLibraryService>>,
) -> Result<Vec<AgentLibraryEntry>, String> {
    library.list_specs()
}

#[command]
pub async fn load_generated_agent(
    library: State<'_, Arc<AgentLibraryService>>,
    agent_id: String,
) -> Result<AgentSpec, String> {
    library.load_spec(&agent_id)
}

fn build_summary(recording: &RecordedWorkflow) -> String {
    if recording.steps.is_empty() {
        return "No explicit steps were recorded.".to_string();
    }

    recording
        .steps
        .iter()
        .take(12)
        .enumerate()
        .map(|(index, step)| format!("{}. [{}] {}", index + 1, step.kind, step.label))
        .collect::<Vec<_>>()
        .join("\n")
}

fn derive_allowed_tools(recording: &RecordedWorkflow) -> Vec<String> {
    let mut allowed_tools: Vec<String> = recording
        .steps
        .iter()
        .filter(|step| step.kind == "tool_call")
        .map(|step| step.label.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    allowed_tools.sort();
    allowed_tools.dedup();
    allowed_tools
}

fn build_generated_spec(recording: &RecordedWorkflow, agent_name: String) -> AgentSpec {
    let allowed_tools = derive_allowed_tools(recording);
    let summary = build_summary(recording);
    AgentSpec {
        id: format!("agent_{}", uuid::Uuid::new_v4()),
        version: "3.0.0".to_string(),
        soul: AgentSoul {
            name: agent_name,
            description: "Generated from recorded workflow".to_string(),
            soul_content: format!(
                "# Mission\nExecute the recorded workflow reliably.\n\n# Recorded Summary\n{}\n\n# Reliability Rules\n- Follow the step order where possible.\n- Verify each tool result before claiming success.\n- Report blocks/errors explicitly.",
                summary
            ),
            ..Default::default()
        },
        skills: AgentSkills {
            capabilities: vec![],
            tools: HashMap::new(),
        },
        airlock: crate::ai::specs::manifest::AirlockConfig {
            tool_policy: crate::ai::specs::manifest::AirlockToolPolicy {
                mode: if allowed_tools.is_empty() {
                    "all".to_string()
                } else {
                    "allowlist".to_string()
                },
                allow: allowed_tools,
                deny: Vec::new(),
            },
            ..Default::default()
        },
        memory_config: Default::default(),
        connectors: Default::default(),
        runtime: crate::ai::specs::manifest::RuntimeConfig {
            mode: RuntimeMode::Single,
            ..Default::default()
        },
        signature: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_generated_spec, build_summary, derive_allowed_tools};
    use crate::services::workflow_recorder::{RecordedWorkflow, WorkflowRecordedStep};

    fn sample_recording(steps: Vec<WorkflowRecordedStep>) -> RecordedWorkflow {
        RecordedWorkflow {
            id: "rec_1".to_string(),
            title: "Demo".to_string(),
            started_at_ms: 1,
            stopped_at_ms: Some(2),
            step_count: steps.len(),
            steps,
        }
    }

    #[test]
    fn derive_allowed_tools_is_deduped_and_sorted() {
        let recording = sample_recording(vec![
            WorkflowRecordedStep {
                id: "s1".to_string(),
                kind: "tool_call".to_string(),
                label: "write_file".to_string(),
                payload: None,
                timestamp_ms: 1,
            },
            WorkflowRecordedStep {
                id: "s2".to_string(),
                kind: "tool_call".to_string(),
                label: "read_file".to_string(),
                payload: None,
                timestamp_ms: 2,
            },
            WorkflowRecordedStep {
                id: "s3".to_string(),
                kind: "tool_call".to_string(),
                label: "write_file".to_string(),
                payload: None,
                timestamp_ms: 3,
            },
            WorkflowRecordedStep {
                id: "s4".to_string(),
                kind: "status".to_string(),
                label: "ignored".to_string(),
                payload: None,
                timestamp_ms: 4,
            },
        ]);

        assert_eq!(
            derive_allowed_tools(&recording),
            vec!["read_file".to_string(), "write_file".to_string()]
        );
    }

    #[test]
    fn build_summary_limits_to_twelve_steps() {
        let steps = (0..20)
            .map(|i| WorkflowRecordedStep {
                id: format!("s{}", i),
                kind: "tool_call".to_string(),
                label: format!("step_{}", i),
                payload: None,
                timestamp_ms: i,
            })
            .collect::<Vec<_>>();
        let recording = sample_recording(steps);

        let summary = build_summary(&recording);
        let lines = summary.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 12);
        assert!(lines[0].contains("step_0"));
        assert!(lines[11].contains("step_11"));
    }

    #[test]
    fn generated_spec_uses_all_mode_for_empty_tool_calls() {
        let recording = sample_recording(vec![WorkflowRecordedStep {
            id: "s1".to_string(),
            kind: "user_instruction".to_string(),
            label: "do something".to_string(),
            payload: None,
            timestamp_ms: 1,
        }]);
        let spec = build_generated_spec(&recording, "Forge Agent".to_string());
        assert_eq!(spec.airlock.tool_policy.mode, "all");
        assert!(spec.airlock.tool_policy.allow.is_empty());
    }
}
