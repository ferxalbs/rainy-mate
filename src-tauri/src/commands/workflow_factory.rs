use crate::ai::specs::manifest::{AgentSpec, RuntimeMode};
use crate::ai::specs::skills::AgentSkills;
use crate::ai::specs::soul::AgentSoul;
use crate::services::agent_library::{AgentLibraryEntry, AgentLibraryService};
use crate::services::workflow_recorder::{
    RecordedWorkflow, WorkflowRecordedStep, WorkflowRecorderService,
};
use std::sync::Arc;
use tauri::{command, State};

const MIN_USEFUL_STEPS_FOR_GENERATION: usize = 3;
const MIN_TOOL_CALLS_FOR_GENERATION: usize = 1;

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
    pub recording_metrics: ForgeRecordingMetrics,
    pub generated_spec: AgentSpec,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeRecordingMetrics {
    pub total_steps: usize,
    pub useful_steps: usize,
    pub tool_calls_count: usize,
    pub decision_points_count: usize,
    pub errors_count: usize,
    pub retries_count: usize,
    pub ready_to_generate: bool,
    pub missing_requirements: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeValidationResult {
    pub passed: bool,
    pub coverage_score: u8,
    pub determinism_score: u8,
    pub safety_score: u8,
    pub total_score: u8,
    pub reasons: Vec<String>,
    pub checked_at_ms: i64,
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

    let metrics = derive_recording_metrics(&recording);
    if !metrics.ready_to_generate {
        return Err(format!(
            "Forge gate not satisfied: {}",
            metrics.missing_requirements.join("; ")
        ));
    }

    let generated_spec = build_generated_spec(&recording, resolved_agent_name);

    Ok(ForgeGenerateResponse {
        recording,
        recording_metrics: metrics,
        generated_spec,
    })
}

#[command]
pub async fn save_generated_agent(
    library: State<'_, Arc<AgentLibraryService>>,
    spec: AgentSpec,
) -> Result<AgentLibraryEntry, String> {
    ensure_spec_is_save_ready(&spec)?;
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

#[command]
pub async fn validate_generated_agent(
    recorder: State<'_, Arc<WorkflowRecorderService>>,
    spec: AgentSpec,
    recording_id: Option<String>,
) -> Result<ForgeValidationResult, String> {
    let recording = if let Some(id) = recording_id {
        recorder.get_recording(&id).await
    } else {
        None
    };

    Ok(run_forge_validation(&spec, recording.as_ref()))
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

fn is_valid_tool_name(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-')
}

fn derive_allowed_tools(recording: &RecordedWorkflow) -> Vec<String> {
    let mut allowed_tools: Vec<String> = recording
        .steps
        .iter()
        .filter(|step| step.kind == "tool_call")
        .map(|step| step.label.trim().to_lowercase())
        .filter(|value| is_valid_tool_name(value))
        .collect();
    allowed_tools.sort();
    allowed_tools.dedup();
    allowed_tools
}

fn build_structured_playbook(recording: &RecordedWorkflow) -> String {
    let goal = recording
        .steps
        .iter()
        .find(|step| step.kind == "user_instruction")
        .map(|step| step.label.clone())
        .unwrap_or_else(|| "Execute the workflow exactly as recorded.".to_string());

    let preconditions = recording
        .steps
        .iter()
        .filter(|step| step.kind == "status")
        .take(3)
        .enumerate()
        .map(|(index, step)| format!("{}. {}", index + 1, step.label))
        .collect::<Vec<_>>();

    let ordered_steps = recording
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.kind.as_str(),
                "tool_call" | "tool_result" | "decision" | "retry"
            )
        })
        .take(14)
        .enumerate()
        .map(|(index, step)| format!("{}. [{}] {}", index + 1, step.kind, step.label))
        .collect::<Vec<_>>();

    let decision_rules = recording
        .steps
        .iter()
        .filter(|step| step.kind == "decision")
        .take(6)
        .enumerate()
        .map(|(index, step)| format!("{}. {}", index + 1, step.label))
        .collect::<Vec<_>>();

    let fallbacks = recording
        .steps
        .iter()
        .filter(|step| step.kind == "error" || step.kind == "retry")
        .take(6)
        .enumerate()
        .map(|(index, step)| format!("{}. [{}] {}", index + 1, step.kind, step.label))
        .collect::<Vec<_>>();

    format!(
        "# Goal\n{}\n\n# Preconditions\n{}\n\n# Ordered Steps\n{}\n\n# Decision Rules\n{}\n\n# Fallbacks\n{}\n\n# Success Criteria\n- Complete the workflow with no unresolved errors.\n- Verify each critical tool result before final response.\n- Return explicit output + verification summary.",
        goal,
        if preconditions.is_empty() {
            "1. Validate workspace context and permissions.".to_string()
        } else {
            preconditions.join("\n")
        },
        if ordered_steps.is_empty() {
            "1. Execute the core workflow in stable sequential order.".to_string()
        } else {
            ordered_steps.join("\n")
        },
        if decision_rules.is_empty() {
            "1. If results are ambiguous, re-check using a read-only verification step.".to_string()
        } else {
            decision_rules.join("\n")
        },
        if fallbacks.is_empty() {
            "1. On failure, retry once with safer parameters and explain why.".to_string()
        } else {
            fallbacks.join("\n")
        },
    )
}

fn derive_recording_metrics(recording: &RecordedWorkflow) -> ForgeRecordingMetrics {
    let total_steps = recording.steps.len();
    let useful_steps = recording
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.kind.as_str(),
                "user_instruction"
                    | "agent_run"
                    | "agent_run_requested"
                    | "tool_call"
                    | "tool_result"
                    | "decision"
                    | "error"
                    | "retry"
            )
        })
        .count();

    let tool_calls_count = recording
        .steps
        .iter()
        .filter(|step| step.kind == "tool_call")
        .count();
    let decision_points_count = recording
        .steps
        .iter()
        .filter(|step| step.kind == "decision")
        .count();
    let errors_count = recording
        .steps
        .iter()
        .filter(|step| step.kind == "error")
        .count();
    let retries_count = recording
        .steps
        .iter()
        .filter(|step| step.kind == "retry")
        .count();

    let mut missing_requirements = Vec::new();
    if useful_steps < MIN_USEFUL_STEPS_FOR_GENERATION {
        missing_requirements.push(format!(
            "Need at least {} useful steps (currently {})",
            MIN_USEFUL_STEPS_FOR_GENERATION, useful_steps
        ));
    }
    if tool_calls_count < MIN_TOOL_CALLS_FOR_GENERATION {
        missing_requirements.push(format!(
            "Need at least {} tool call (currently {})",
            MIN_TOOL_CALLS_FOR_GENERATION, tool_calls_count
        ));
    }

    ForgeRecordingMetrics {
        total_steps,
        useful_steps,
        tool_calls_count,
        decision_points_count,
        errors_count,
        retries_count,
        ready_to_generate: missing_requirements.is_empty(),
        missing_requirements,
    }
}

fn run_forge_validation(
    spec: &AgentSpec,
    recording: Option<&RecordedWorkflow>,
) -> ForgeValidationResult {
    let mut reasons = Vec::new();
    let mut coverage_score: i32 = 30;
    let mut determinism_score: i32 = 35;
    let mut safety_score: i32 = 35;

    let soul_content_lower = spec.soul.soul_content.to_lowercase();
    let has_structured_sections = soul_content_lower.contains("# goal")
        && soul_content_lower.contains("# ordered steps")
        && soul_content_lower.contains("# decision rules")
        && soul_content_lower.contains("# fallbacks")
        && soul_content_lower.contains("# success criteria");

    if has_structured_sections {
        coverage_score += 20;
        determinism_score += 15;
    } else {
        reasons.push("Missing structured playbook sections in soul content".to_string());
    }

    if spec.airlock.tool_policy.mode == "allowlist" && !spec.airlock.tool_policy.allow.is_empty() {
        coverage_score += 20;
        safety_score += 25;
    } else {
        reasons.push(
            "Tool policy is not strict enough (allowlist with observed tools required)".to_string(),
        );
    }

    if let Some(workflow) = recording {
        let metrics = derive_recording_metrics(workflow);
        if metrics.ready_to_generate {
            coverage_score += 20;
            determinism_score += 15;
        } else {
            reasons.extend(metrics.missing_requirements);
        }

        let recorded_tools = derive_allowed_tools(workflow);
        if !recorded_tools.is_empty() {
            let missing_tools = recorded_tools
                .iter()
                .filter(|tool| !spec.airlock.tool_policy.allow.contains(tool))
                .cloned()
                .collect::<Vec<_>>();
            if missing_tools.is_empty() {
                safety_score += 15;
            } else {
                reasons.push(format!(
                    "Spec allowlist is missing recorded tools: {}",
                    missing_tools.join(", ")
                ));
            }
        }
    } else {
        reasons.push("No recording context supplied for validation".to_string());
    }

    let coverage_score = coverage_score.clamp(0, 100) as u8;
    let determinism_score = determinism_score.clamp(0, 100) as u8;
    let safety_score = safety_score.clamp(0, 100) as u8;
    let total_score =
        (((coverage_score as u32) + (determinism_score as u32) + (safety_score as u32)) / 3) as u8;
    let passed = total_score >= 75 && reasons.is_empty();

    ForgeValidationResult {
        passed,
        coverage_score,
        determinism_score,
        safety_score,
        total_score,
        reasons: if reasons.is_empty() {
            vec!["Validation passed".to_string()]
        } else {
            reasons
        },
        checked_at_ms: now_ms(),
    }
}

fn ensure_spec_is_save_ready(spec: &AgentSpec) -> Result<(), String> {
    if spec.id.trim().is_empty() {
        return Err("Generated agent is missing a valid id".to_string());
    }
    if spec.soul.name.trim().is_empty() {
        return Err("Generated agent is missing a valid name".to_string());
    }
    if spec.soul.soul_content.trim().is_empty() {
        return Err("Generated agent is missing a valid soul content".to_string());
    }
    if spec.airlock.tool_policy.mode.trim() != "allowlist" {
        return Err("Generated agent must use allowlist tool policy".to_string());
    }
    if spec.airlock.tool_policy.allow.is_empty() {
        return Err("Generated agent must include at least one allowed tool".to_string());
    }
    Ok(())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn build_generated_spec(recording: &RecordedWorkflow, agent_name: String) -> AgentSpec {
    let allowed_tools = derive_allowed_tools(recording);
    let summary = build_summary(recording);
    let structured = build_structured_playbook(recording);

    AgentSpec {
        id: format!("agent_{}", uuid::Uuid::new_v4()),
        version: "3.0.0".to_string(),
        soul: AgentSoul {
            name: agent_name,
            description: "Specialist generated from recorded workflow".to_string(),
            soul_content: format!(
                "# Mission\nExecute this specialized workflow reliably.\n\n# Specialist Positioning\nThis agent is a task specialist generated by Forge. It complements the base agent and should be used for this specific operational workflow.\n\n# Recorded Summary\n{}\n\n{}\n\n# Reliability Rules\n- Never invent tool outputs.\n- If a required tool result is missing, stop and request clarification.\n- Prefer deterministic execution over creative variation.",
                summary, structured
            ),
            ..Default::default()
        },
        skills: AgentSkills::default(),
        airlock: crate::ai::specs::manifest::AirlockConfig {
            tool_policy: crate::ai::specs::manifest::AirlockToolPolicy {
                mode: "allowlist".to_string(),
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
        model: None,
        temperature: None,
        max_tokens: None,
        provider: None,
        signature: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_generated_spec, build_summary, derive_allowed_tools, derive_recording_metrics,
        run_forge_validation,
    };
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
    fn generated_spec_uses_allowlist_mode() {
        let recording = sample_recording(vec![WorkflowRecordedStep {
            id: "s1".to_string(),
            kind: "user_instruction".to_string(),
            label: "do something".to_string(),
            payload: None,
            timestamp_ms: 1,
        }]);
        let spec = build_generated_spec(&recording, "Forge Agent".to_string());
        assert_eq!(spec.airlock.tool_policy.mode, "allowlist");
        assert!(spec.airlock.tool_policy.allow.is_empty());
    }

    #[test]
    fn recording_metrics_gate_requires_steps_and_tool_calls() {
        let empty = sample_recording(vec![]);
        let metrics = derive_recording_metrics(&empty);
        assert!(!metrics.ready_to_generate);
        assert!(!metrics.missing_requirements.is_empty());

        let valid = sample_recording(vec![
            WorkflowRecordedStep {
                id: "s1".to_string(),
                kind: "user_instruction".to_string(),
                label: "task".to_string(),
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
                kind: "tool_result".to_string(),
                label: "read_file".to_string(),
                payload: None,
                timestamp_ms: 3,
            },
        ]);
        let valid_metrics = derive_recording_metrics(&valid);
        assert!(valid_metrics.ready_to_generate);
    }

    #[test]
    fn forge_validation_passes_for_structured_specialist() {
        let recording = sample_recording(vec![
            WorkflowRecordedStep {
                id: "s1".to_string(),
                kind: "user_instruction".to_string(),
                label: "Generate report".to_string(),
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
                kind: "tool_result".to_string(),
                label: "read_file".to_string(),
                payload: None,
                timestamp_ms: 3,
            },
        ]);

        let spec = build_generated_spec(&recording, "Forge Agent".to_string());
        let result = run_forge_validation(&spec, Some(&recording));
        assert!(result.passed);
        assert!(result.total_score >= 75);
    }

    #[test]
    fn derive_allowed_tools_filters_invalid_labels() {
        let recording = sample_recording(vec![
            WorkflowRecordedStep {
                id: "s1".to_string(),
                kind: "tool_call".to_string(),
                label: "read_file".to_string(),
                payload: None,
                timestamp_ms: 1,
            },
            WorkflowRecordedStep {
                id: "s2".to_string(),
                kind: "tool_call".to_string(),
                label: "bad tool name".to_string(),
                payload: None,
                timestamp_ms: 2,
            },
            WorkflowRecordedStep {
                id: "s3".to_string(),
                kind: "tool_call".to_string(),
                label: "../escape".to_string(),
                payload: None,
                timestamp_ms: 3,
            },
        ]);

        let tools = derive_allowed_tools(&recording);
        assert_eq!(tools, vec!["read_file".to_string()]);
    }
}
