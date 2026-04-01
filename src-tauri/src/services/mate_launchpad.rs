use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::neural::{AirlockLevel, ToolAccessPolicy};
use crate::services::{get_tool_policy, LocalAgentSecurityService, Workspace, WorkspaceManager};

const MAX_RECENT_RUNS: usize = 8;

const EXECUTE_RISK_TOOLS: &[&str] = &[
    "execute_command",
    "browse_url",
    "open_new_tab",
    "click_element",
    "type_text",
    "go_back",
    "submit_form",
    "http_post_json",
];

const DELETE_RISK_TOOLS: &[&str] = &["delete_file", "move_file"];

const REPO_GUARDIAN_TOOLS: &[&str] = &[
    "read_file",
    "read_many_files",
    "read_file_chunk",
    "list_files",
    "list_files_detailed",
    "search_files",
    "file_exists",
    "get_file_info",
    "git_status",
    "git_diff",
    "git_log",
    "git_show",
    "git_branch_list",
    "web_search",
    "read_web_page",
    "write_file",
    "append_file",
];

const WORKSPACE_FORGER_TOOLS: &[&str] = &[
    "list_files",
    "list_files_detailed",
    "search_files",
    "mkdir",
    "write_file",
    "append_file",
    "move_file",
    "delete_file",
    "archive_create",
];

const INCIDENT_SCRIBE_TOOLS: &[&str] = &[
    "read_file",
    "read_file_chunk",
    "search_files",
    "read_many_files",
    "web_search",
    "read_web_page",
    "write_file",
    "append_file",
    "pdf_create",
    "docx_create",
];

const KNOWLEDGE_WEAVER_TOOLS: &[&str] = &[
    "read_file",
    "read_many_files",
    "read_file_chunk",
    "list_files",
    "search_files",
    "ingest_document",
    "save_memory",
    "recall_memory",
    "pdf_create",
    "docx_create",
    "excel_write",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatePackDefinition {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub recommended_for: String,
    pub expected_outputs: Vec<String>,
    pub default_trust_preset: String,
    pub tool_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstRunScenarioDefinition {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub recommended_pack_ids: Vec<String>,
    pub suggested_outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct WorkspacePlannedActionSummary {
    pub create_or_update: Vec<String>,
    pub move_or_delete: Vec<String>,
    pub external_actions: Vec<String>,
    pub memory_actions: Vec<String>,
}

impl Default for WorkspacePlannedActionSummary {
    fn default() -> Self {
        Self {
            create_or_update: Vec::new(),
            move_or_delete: Vec::new(),
            external_actions: Vec::new(),
            memory_actions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLaunchPreflight {
    pub scenario_id: String,
    pub scenario_title: String,
    pub trust_preset: String,
    pub enabled_pack_ids: Vec<String>,
    pub enabled_pack_titles: Vec<String>,
    pub approved_tool_ids: Vec<String>,
    pub touched_paths: Vec<String>,
    pub intent_summary: String,
    pub planned_actions: WorkspacePlannedActionSummary,
    pub expected_outputs: Vec<String>,
    pub effective_tool_policy_mode: String,
    pub highest_airlock_level: u8,
    pub requires_explicit_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct WorkspaceLaunchRunRecord {
    pub request_id: String,
    pub scenario_id: String,
    pub scenario_title: String,
    pub trust_preset: String,
    pub enabled_pack_ids: Vec<String>,
    pub approved_tool_ids: Vec<String>,
    pub touched_paths: Vec<String>,
    pub intent_summary: String,
    pub planned_actions: WorkspacePlannedActionSummary,
    pub actual_tool_ids: Vec<String>,
    pub actual_touched_paths: Vec<String>,
    pub produced_artifact_paths: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub effective_tool_policy_mode: String,
    pub highest_airlock_level: u8,
    pub requires_explicit_approval: bool,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub chat_id: Option<String>,
    pub success: Option<bool>,
}

impl Default for WorkspaceLaunchRunRecord {
    fn default() -> Self {
        Self {
            request_id: String::new(),
            scenario_id: String::new(),
            scenario_title: String::new(),
            trust_preset: "balanced".to_string(),
            enabled_pack_ids: Vec::new(),
            approved_tool_ids: Vec::new(),
            touched_paths: Vec::new(),
            intent_summary: String::new(),
            planned_actions: WorkspacePlannedActionSummary::default(),
            actual_tool_ids: Vec::new(),
            actual_touched_paths: Vec::new(),
            produced_artifact_paths: Vec::new(),
            expected_outputs: Vec::new(),
            effective_tool_policy_mode: String::new(),
            highest_airlock_level: 0,
            requires_explicit_approval: false,
            status: "prepared".to_string(),
            created_at: String::new(),
            completed_at: None,
            chat_id: None,
            success: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePreparedLaunch {
    pub request_id: String,
    pub prompt: String,
    pub preflight: WorkspaceLaunchPreflight,
    pub launchpad: WorkspaceLaunchpadSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLaunchpadSummary {
    pub workspace_id: String,
    pub workspace_name: String,
    pub trust_preset: String,
    pub enabled_pack_ids: Vec<String>,
    pub first_run_completed_at: Option<String>,
    pub first_run_scenario_id: Option<String>,
    pub launch_count: u64,
    pub successful_launch_count: u64,
    pub last_launch_at: Option<String>,
    pub last_launch_chat_id: Option<String>,
    pub capability_summary: WorkspaceCapabilitySummary,
    pub recent_runs: Vec<WorkspaceLaunchRunRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCapabilitySummary {
    pub label: String,
    pub effective_tool_policy_mode: String,
    pub allowed_paths_count: usize,
    pub permissions: WorkspaceCapabilityPermissions,
    pub enabled_capabilities: Vec<String>,
    pub cautions: Vec<String>,
    pub enforced_pack_ids: Vec<String>,
    pub active_tool_ids: Vec<String>,
    pub requires_explicit_approval: bool,
    pub highest_airlock_level: u8,
    pub suggested_outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCapabilityPermissions {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_delete: bool,
    pub can_create_agents: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceLaunchSettings {
    pub trust_preset: String,
    pub enabled_pack_ids: Vec<String>,
    pub first_run_completed_at: Option<DateTime<Utc>>,
    pub first_run_scenario_id: Option<String>,
    pub launch_count: u64,
    pub successful_launch_count: u64,
    pub last_launch_at: Option<DateTime<Utc>>,
    pub last_launch_chat_id: Option<String>,
    pub recent_runs: Vec<WorkspaceLaunchRunRecord>,
}

impl Default for WorkspaceLaunchSettings {
    fn default() -> Self {
        Self {
            trust_preset: "balanced".to_string(),
            enabled_pack_ids: vec![
                "repo_guardian".to_string(),
                "workspace_forger".to_string(),
                "knowledge_weaver".to_string(),
            ],
            first_run_completed_at: None,
            first_run_scenario_id: None,
            launch_count: 0,
            successful_launch_count: 0,
            last_launch_at: None,
            last_launch_chat_id: None,
            recent_runs: Vec::new(),
        }
    }
}

pub struct MateLaunchpadService;

impl MateLaunchpadService {
    pub fn pack_definitions() -> Vec<MatePackDefinition> {
        vec![
            MatePackDefinition {
                id: "repo_guardian".to_string(),
                title: "Repo Guardian".to_string(),
                summary: "Reviews repos, diffs, issues, and changelogs to surface risky changes and draft decisive next actions.".to_string(),
                recommended_for: "Active codebases and technical due diligence".to_string(),
                expected_outputs: vec![
                    "Review notes".to_string(),
                    "Risk summary".to_string(),
                    "Patch suggestions".to_string(),
                ],
                default_trust_preset: "balanced".to_string(),
                tool_ids: REPO_GUARDIAN_TOOLS.iter().map(|tool| (*tool).to_string()).collect(),
            },
            MatePackDefinition {
                id: "workspace_forger".to_string(),
                title: "Workspace Forger".to_string(),
                summary: "Organizes a working directory, archives stale artifacts, and keeps workspace state current without leaving cleanup to the user.".to_string(),
                recommended_for: "Messy projects, downloads, and working trees".to_string(),
                expected_outputs: vec![
                    "Organized files".to_string(),
                    "Updated WORKSTATE.md".to_string(),
                    "Archive bundle".to_string(),
                ],
                default_trust_preset: "balanced".to_string(),
                tool_ids: WORKSPACE_FORGER_TOOLS.iter().map(|tool| (*tool).to_string()).collect(),
            },
            MatePackDefinition {
                id: "incident_scribe".to_string(),
                title: "Incident Scribe".to_string(),
                summary: "Turns logs and operational evidence into a current incident narrative with concrete anomalies and next checks.".to_string(),
                recommended_for: "Operational debugging and production support".to_string(),
                expected_outputs: vec![
                    "Incident diary".to_string(),
                    "Anomaly summary".to_string(),
                    "Follow-up checklist".to_string(),
                ],
                default_trust_preset: "conservative".to_string(),
                tool_ids: INCIDENT_SCRIBE_TOOLS.iter().map(|tool| (*tool).to_string()).collect(),
            },
            MatePackDefinition {
                id: "knowledge_weaver".to_string(),
                title: "Knowledge Weaver".to_string(),
                summary: "Builds durable project memory from docs, changelogs, ADRs, and reports so MaTE can answer from current workspace truth.".to_string(),
                recommended_for: "Codebases, product docs, and research workspaces".to_string(),
                expected_outputs: vec![
                    "Ingested knowledge".to_string(),
                    "Generated docs".to_string(),
                    "Workspace memory refresh".to_string(),
                ],
                default_trust_preset: "conservative".to_string(),
                tool_ids: KNOWLEDGE_WEAVER_TOOLS.iter().map(|tool| (*tool).to_string()).collect(),
            },
        ]
    }

    pub fn first_run_scenarios() -> Vec<FirstRunScenarioDefinition> {
        vec![
            FirstRunScenarioDefinition {
                id: "release_readiness".to_string(),
                title: "Release Readiness".to_string(),
                summary: "Audit the repo like a launch owner, surface the real release blockers, and leave behind a decisive ship/no-ship brief.".to_string(),
                recommended_pack_ids: vec![
                    "repo_guardian".to_string(),
                    "knowledge_weaver".to_string(),
                ],
                suggested_outputs: vec![
                    "Release brief".to_string(),
                    "Risk register".to_string(),
                    "Ship checklist".to_string(),
                ],
            },
            FirstRunScenarioDefinition {
                id: "codebase_copilot".to_string(),
                title: "Codebase Copilot".to_string(),
                summary: "Audit the current repo, update workspace memory, and produce a decisive engineering brief for what matters next.".to_string(),
                recommended_pack_ids: vec![
                    "repo_guardian".to_string(),
                    "knowledge_weaver".to_string(),
                ],
                suggested_outputs: vec![
                    "Engineering brief".to_string(),
                    "Risk map".to_string(),
                    "Updated workspace memory".to_string(),
                ],
            },
            FirstRunScenarioDefinition {
                id: "file_organizer".to_string(),
                title: "File Organizer".to_string(),
                summary: "Inspect the active workspace, identify clutter, and propose or execute a cleaner structure with an auditable result.".to_string(),
                recommended_pack_ids: vec!["workspace_forger".to_string()],
                suggested_outputs: vec![
                    "Cleanup plan".to_string(),
                    "Renaming suggestions".to_string(),
                    "Archive bundle".to_string(),
                ],
            },
            FirstRunScenarioDefinition {
                id: "docs_builder".to_string(),
                title: "Docs Builder".to_string(),
                summary: "Ingest the current workspace context and generate a polished summary document that a teammate or investor can understand fast.".to_string(),
                recommended_pack_ids: vec!["knowledge_weaver".to_string()],
                suggested_outputs: vec![
                    "Project brief".to_string(),
                    "PDF or DOCX artifact".to_string(),
                    "Updated knowledge state".to_string(),
                ],
            },
        ]
    }

    pub fn get_workspace_summary(workspace: &Workspace) -> WorkspaceLaunchpadSummary {
        let trust = normalize_trust_preset(&workspace.launchpad.trust_preset);
        WorkspaceLaunchpadSummary {
            workspace_id: workspace.id.clone(),
            workspace_name: workspace.name.clone(),
            trust_preset: trust.to_string(),
            enabled_pack_ids: sanitize_pack_ids(&workspace.launchpad.enabled_pack_ids),
            first_run_completed_at: workspace
                .launchpad
                .first_run_completed_at
                .map(|value| value.to_rfc3339()),
            first_run_scenario_id: workspace.launchpad.first_run_scenario_id.clone(),
            launch_count: workspace.launchpad.launch_count,
            successful_launch_count: workspace.launchpad.successful_launch_count,
            last_launch_at: workspace
                .launchpad
                .last_launch_at
                .map(|value| value.to_rfc3339()),
            last_launch_chat_id: workspace.launchpad.last_launch_chat_id.clone(),
            capability_summary: capability_summary(workspace, trust),
            recent_runs: workspace.launchpad.recent_runs.clone(),
        }
    }

    pub fn update_workspace_launch_config(
        workspace_manager: &WorkspaceManager,
        workspace_id: &str,
        trust_preset: &str,
        enabled_pack_ids: &[String],
    ) -> Result<WorkspaceLaunchpadSummary, String> {
        let mut workspace = workspace_manager
            .load_workspace(workspace_id)
            .map_err(|e| e.to_string())?;
        workspace.launchpad.trust_preset = normalize_trust_preset(trust_preset).to_string();
        workspace.launchpad.enabled_pack_ids = sanitize_pack_ids(enabled_pack_ids);
        workspace_manager
            .save_workspace(&workspace, crate::services::ConfigFormat::Json)
            .map_err(|e| e.to_string())?;
        Ok(Self::get_workspace_summary(&workspace))
    }

    pub fn prepare_workspace_launch(
        workspace_manager: &WorkspaceManager,
        workspace_id: &str,
        scenario_id: &str,
    ) -> Result<WorkspacePreparedLaunch, String> {
        let mut workspace = workspace_manager
            .load_workspace(workspace_id)
            .map_err(|e| e.to_string())?;
        let preflight = build_launch_preflight(&workspace, scenario_id)?;
        let request_id = format!("launch_{}", uuid::Uuid::new_v4());
        let created_at = Utc::now();
        let prompt = build_first_run_prompt(&workspace, scenario_id, &preflight)?;

        workspace.launchpad.recent_runs.insert(
            0,
            WorkspaceLaunchRunRecord {
                request_id: request_id.clone(),
                scenario_id: preflight.scenario_id.clone(),
                scenario_title: preflight.scenario_title.clone(),
                trust_preset: preflight.trust_preset.clone(),
                enabled_pack_ids: preflight.enabled_pack_ids.clone(),
                approved_tool_ids: preflight.approved_tool_ids.clone(),
                touched_paths: preflight.touched_paths.clone(),
                intent_summary: preflight.intent_summary.clone(),
                planned_actions: preflight.planned_actions.clone(),
                actual_tool_ids: Vec::new(),
                actual_touched_paths: Vec::new(),
                produced_artifact_paths: Vec::new(),
                expected_outputs: preflight.expected_outputs.clone(),
                effective_tool_policy_mode: preflight.effective_tool_policy_mode.clone(),
                highest_airlock_level: preflight.highest_airlock_level,
                requires_explicit_approval: preflight.requires_explicit_approval,
                status: "prepared".to_string(),
                created_at: created_at.to_rfc3339(),
                completed_at: None,
                chat_id: None,
                success: None,
            },
        );
        trim_recent_runs(&mut workspace.launchpad.recent_runs);
        workspace_manager
            .save_workspace(&workspace, crate::services::ConfigFormat::Json)
            .map_err(|e| e.to_string())?;

        Ok(WorkspacePreparedLaunch {
            request_id,
            prompt,
            preflight,
            launchpad: Self::get_workspace_summary(&workspace),
        })
    }

    pub fn record_workspace_launch(
        workspace_manager: &WorkspaceManager,
        workspace_id: &str,
        request_id: &str,
        scenario_id: &str,
        chat_id: Option<&str>,
        success: bool,
        actual_tool_ids: &[String],
        actual_touched_paths: &[String],
        produced_artifact_paths: &[String],
    ) -> Result<WorkspaceLaunchpadSummary, String> {
        let mut workspace = workspace_manager
            .load_workspace(workspace_id)
            .map_err(|e| e.to_string())?;
        let now = Utc::now();
        workspace.launchpad.launch_count = workspace.launchpad.launch_count.saturating_add(1);
        if success {
            workspace.launchpad.successful_launch_count = workspace
                .launchpad
                .successful_launch_count
                .saturating_add(1);
            workspace.launchpad.first_run_completed_at = Some(now);
            workspace.launchpad.first_run_scenario_id = Some(scenario_id.to_string());
        }
        workspace.launchpad.last_launch_at = Some(now);
        workspace.launchpad.last_launch_chat_id = chat_id.map(|value| value.to_string());

        if let Some(run) = workspace
            .launchpad
            .recent_runs
            .iter_mut()
            .find(|record| record.request_id == request_id)
        {
            run.status = if success {
                "completed".to_string()
            } else {
                "failed".to_string()
            };
            run.completed_at = Some(now.to_rfc3339());
            run.chat_id = chat_id.map(|value| value.to_string());
            run.success = Some(success);
            run.actual_tool_ids = actual_tool_ids.to_vec();
            run.actual_touched_paths = actual_touched_paths.to_vec();
            run.produced_artifact_paths = produced_artifact_paths.to_vec();
        }

        workspace_manager
            .save_workspace(&workspace, crate::services::ConfigFormat::Json)
            .map_err(|e| e.to_string())?;
        Ok(Self::get_workspace_summary(&workspace))
    }

    pub fn build_first_run_prompt(
        workspace: &Workspace,
        scenario_id: &str,
    ) -> Result<String, String> {
        let preflight = build_launch_preflight(workspace, scenario_id)?;
        build_first_run_prompt(workspace, scenario_id, &preflight)
    }

    pub fn constrain_tool_policy_for_workspace(
        workspace: &Workspace,
        base: ToolAccessPolicy,
    ) -> (ToolAccessPolicy, String) {
        let enabled_pack_ids = sanitize_pack_ids(&workspace.launchpad.enabled_pack_ids);
        if enabled_pack_ids.is_empty() {
            return (base, String::new());
        }

        let active_tool_ids = effective_launch_tool_ids(
            workspace,
            &enabled_pack_ids,
            normalize_trust_preset(&workspace.launchpad.trust_preset),
        );
        if active_tool_ids.is_empty() {
            return (base, String::new());
        }

        let allow = if base.mode == "allowlist" && !base.allow.is_empty() {
            active_tool_ids
                .into_iter()
                .filter(|tool| base.allow.iter().any(|allowed| allowed == tool))
                .collect::<Vec<_>>()
        } else {
            active_tool_ids
        };

        (
            ToolAccessPolicy {
                enabled: base.enabled,
                mode: "allowlist".to_string(),
                allow,
                deny: base.deny,
            },
            "+launchpad".to_string(),
        )
    }
}

fn normalize_trust_preset(value: &str) -> &'static str {
    match value {
        "conservative" => "conservative",
        "elevated" => "elevated",
        _ => "balanced",
    }
}

fn sanitize_pack_ids(ids: &[String]) -> Vec<String> {
    let mut values: Vec<String> = ids
        .iter()
        .filter(|value| {
            matches!(
                value.as_str(),
                "repo_guardian" | "workspace_forger" | "incident_scribe" | "knowledge_weaver"
            )
        })
        .cloned()
        .collect();
    values.sort();
    values.dedup();
    values
}

fn pack_by_id(pack_id: &str) -> Option<MatePackDefinition> {
    MateLaunchpadService::pack_definitions()
        .into_iter()
        .find(|pack| pack.id == pack_id)
}

fn scenario_by_id(scenario_id: &str) -> Option<FirstRunScenarioDefinition> {
    MateLaunchpadService::first_run_scenarios()
        .into_iter()
        .find(|scenario| scenario.id == scenario_id)
}

fn effective_pack_ids_for_scenario(
    workspace: &Workspace,
    scenario_id: &str,
) -> Result<Vec<String>, String> {
    let scenario = scenario_by_id(scenario_id)
        .ok_or_else(|| format!("Unknown first-run scenario '{}'", scenario_id))?;
    let enabled = sanitize_pack_ids(&workspace.launchpad.enabled_pack_ids);
    let mut effective = enabled
        .iter()
        .filter(|pack_id| {
            scenario
                .recommended_pack_ids
                .iter()
                .any(|value| value == *pack_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    if effective.is_empty() {
        effective = scenario.recommended_pack_ids;
    }
    effective.sort();
    effective.dedup();
    Ok(effective)
}

fn active_tool_ids(enabled_pack_ids: &[String], trust_preset: &str) -> Vec<String> {
    let base_tools = enabled_pack_ids
        .iter()
        .filter_map(|pack_id| pack_by_id(pack_id))
        .flat_map(|pack| pack.tool_ids)
        .collect::<BTreeSet<_>>();

    base_tools
        .into_iter()
        .filter(|tool| match trust_preset {
            "conservative" => {
                !EXECUTE_RISK_TOOLS.iter().any(|value| value == tool)
                    && !DELETE_RISK_TOOLS.iter().any(|value| value == tool)
            }
            "balanced" => !EXECUTE_RISK_TOOLS.iter().any(|value| value == tool),
            _ => true,
        })
        .collect()
}

fn effective_launch_tool_ids(
    workspace: &Workspace,
    enabled_pack_ids: &[String],
    trust_preset: &str,
) -> Vec<String> {
    let candidate_tool_ids = active_tool_ids(enabled_pack_ids, trust_preset);
    let workspace_policy =
        LocalAgentSecurityService::tool_policy_from_permissions(&workspace.permissions);

    candidate_tool_ids
        .into_iter()
        .filter(|tool| {
            if !workspace_policy.enabled {
                return false;
            }
            if workspace_policy.deny.iter().any(|denied| denied == tool) {
                return false;
            }
            if workspace_policy.mode == "allowlist" {
                return workspace_policy.allow.iter().any(|allowed| allowed == tool);
            }
            true
        })
        .collect()
}

fn expected_outputs_for_pack_ids(pack_ids: &[String]) -> Vec<String> {
    let mut outputs = BTreeSet::new();
    for pack_id in pack_ids {
        if let Some(pack) = pack_by_id(pack_id) {
            outputs.extend(pack.expected_outputs);
        }
    }
    outputs.into_iter().collect()
}

fn highest_airlock_level(tool_ids: &[String]) -> AirlockLevel {
    tool_ids
        .iter()
        .filter_map(|tool| get_tool_policy(tool))
        .map(|policy| policy.airlock_level)
        .max_by_key(|level| *level as u8)
        .unwrap_or(AirlockLevel::Safe)
}

fn build_launch_preflight(
    workspace: &Workspace,
    scenario_id: &str,
) -> Result<WorkspaceLaunchPreflight, String> {
    let scenario = scenario_by_id(scenario_id)
        .ok_or_else(|| format!("Unknown first-run scenario '{}'", scenario_id))?;
    let trust_preset = normalize_trust_preset(&workspace.launchpad.trust_preset);
    let enabled_pack_ids = effective_pack_ids_for_scenario(workspace, scenario_id)?;
    let enabled_pack_titles = enabled_pack_ids
        .iter()
        .filter_map(|pack_id| pack_by_id(pack_id).map(|pack| pack.title))
        .collect::<Vec<_>>();
    let approved_tool_ids = effective_launch_tool_ids(workspace, &enabled_pack_ids, trust_preset);
    let planned_actions = summarize_planned_actions(&approved_tool_ids);
    let mut expected_outputs = expected_outputs_for_pack_ids(&enabled_pack_ids);
    expected_outputs.extend(scenario.suggested_outputs.clone());
    expected_outputs.sort();
    expected_outputs.dedup();
    let highest_level = highest_airlock_level(&approved_tool_ids);

    Ok(WorkspaceLaunchPreflight {
        scenario_id: scenario.id.clone(),
        scenario_title: scenario.title,
        trust_preset: trust_preset.to_string(),
        enabled_pack_ids,
        enabled_pack_titles,
        approved_tool_ids,
        touched_paths: workspace.allowed_paths.clone(),
        intent_summary: scenario_intent_summary(&scenario.id),
        planned_actions,
        expected_outputs,
        effective_tool_policy_mode: trust_mode_label(trust_preset).to_string(),
        highest_airlock_level: highest_level as u8,
        requires_explicit_approval: highest_level >= AirlockLevel::Dangerous,
    })
}

fn build_first_run_prompt(
    workspace: &Workspace,
    scenario_id: &str,
    preflight: &WorkspaceLaunchPreflight,
) -> Result<String, String> {
    let allowed_paths = if workspace.allowed_paths.is_empty() {
        "No explicit allowed paths were configured yet.".to_string()
    } else {
        workspace.allowed_paths.join(", ")
    };

    let pack_text = if preflight.enabled_pack_titles.is_empty() {
        "No packs are enabled yet.".to_string()
    } else {
        preflight.enabled_pack_titles.join(", ")
    };

    let scenario_prompt = match scenario_id {
        "release_readiness" => "Review this repository like the final launch owner for a serious production release. Inspect the current codebase shape, recent changelog truth, and workspace memory overlay. Produce: 1) the release-critical system truths, 2) the highest-risk blockers or regressions, 3) a ship/no-ship recommendation with explicit reasons, and 4) a concrete checklist of what must happen next. Update WORKSTATE with the decisive release state.",
        "codebase_copilot" => "Analyze this workspace like a senior engineer inheriting a production codebase. Read the repo shape, recent changelog context, and workspace memory overlay. Produce: 1) the most important current system truths, 2) the top product or technical risks, and 3) the next concrete engineering actions. Update WORKSTATE if useful.",
        "file_organizer" => "Inspect this workspace as an operations-focused file organizer. Identify clutter, stale artifacts, naming inconsistencies, and obvious archival candidates. Produce a concrete cleanup plan first; if safe, perform bounded organization work and explain every action. Update WORKSTATE with the resulting state.",
        "docs_builder" => "Read the current workspace, changelog context, and any durable memory files. Produce a polished project brief suitable for a technical founder or investor. Generate a native document artifact if the workspace contents support it, and update workspace memory with the decisive summary.",
        _ => return Err(format!("Unknown first-run scenario '{}'", scenario_id)),
    };

    let expected_outputs = if preflight.expected_outputs.is_empty() {
        "No explicit output contract was configured.".to_string()
    } else {
        preflight.expected_outputs.join(", ")
    };

    Ok(format!(
        "You are running a guided MaTE launch scenario.\n\nWorkspace: {}\nTrust preset: {}\nEnabled packs: {}\nAllowed paths: {}\nExecution contract: {}\nIntent summary: {}\nPlanned action groups:\n- Create or update: {}\n- Move or delete: {}\n- External actions: {}\n- Memory actions: {}\nExpected outputs: {}\nHighest Airlock level in scope: L{}\n\nConstraints:\n- Treat this as a production-grade workspace.\n- Prefer stable, auditable actions.\n- Use the current workspace files and memory overlay as the source of truth.\n- Stay inside the approved tool set for this launch.\n- Show a concrete plan before the first mutating action.\n- If you generate a document or archive, keep it inside the active workspace.\n\nTask:\n{}",
            workspace.name,
            preflight.trust_preset,
            pack_text,
            allowed_paths,
            preflight.effective_tool_policy_mode,
            preflight.intent_summary,
            join_or_none(&preflight.planned_actions.create_or_update),
            join_or_none(&preflight.planned_actions.move_or_delete),
            join_or_none(&preflight.planned_actions.external_actions),
            join_or_none(&preflight.planned_actions.memory_actions),
            expected_outputs,
            preflight.highest_airlock_level,
            scenario_prompt
        ))
}

fn scenario_intent_summary(scenario_id: &str) -> String {
    match scenario_id {
        "release_readiness" => {
            "Bound the run to repo-safe release analysis and a decisive ship-readiness brief."
                .to_string()
        }
        "codebase_copilot" => {
            "Audit the current codebase and leave behind a grounded engineering brief."
                .to_string()
        }
        "file_organizer" => {
            "Inspect the workspace, propose cleanup first, then perform only bounded organization work."
                .to_string()
        }
        "docs_builder" => {
            "Turn the current workspace into a polished summary with durable memory updates."
                .to_string()
        }
        _ => "Run a governed workspace scenario inside the approved tool scope.".to_string(),
    }
}

fn summarize_planned_actions(tool_ids: &[String]) -> WorkspacePlannedActionSummary {
    let mut create_or_update = Vec::new();
    let mut move_or_delete = Vec::new();
    let mut external_actions = Vec::new();
    let mut memory_actions = Vec::new();

    for tool in tool_ids {
        match tool.as_str() {
            "write_file" | "append_file" | "mkdir" | "pdf_create" | "excel_write"
            | "docx_create" | "archive_create" => create_or_update.push(tool.clone()),
            "move_file" | "delete_file" => move_or_delete.push(tool.clone()),
            "execute_command" | "browse_url" | "open_new_tab" | "click_element" | "type_text"
            | "go_back" | "submit_form" | "http_post_json" => external_actions.push(tool.clone()),
            "save_memory" | "recall_memory" | "ingest_document" => {
                memory_actions.push(tool.clone())
            }
            _ => {}
        }
    }

    WorkspacePlannedActionSummary {
        create_or_update,
        move_or_delete,
        external_actions,
        memory_actions,
    }
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn trust_mode_label(trust_preset: &str) -> &'static str {
    match trust_preset {
        "conservative" => "pack_allowlist_safe",
        "elevated" => "pack_allowlist_elevated",
        _ => "pack_allowlist_balanced",
    }
}

fn trim_recent_runs(recent_runs: &mut Vec<WorkspaceLaunchRunRecord>) {
    recent_runs.truncate(MAX_RECENT_RUNS);
}

fn capability_summary(workspace: &Workspace, trust_preset: &str) -> WorkspaceCapabilitySummary {
    let enabled_pack_ids = sanitize_pack_ids(&workspace.launchpad.enabled_pack_ids);
    let active_tool_ids = effective_launch_tool_ids(workspace, &enabled_pack_ids, trust_preset);
    let highest_level = highest_airlock_level(&active_tool_ids);
    let mut enabled_capabilities = vec!["Workspace memory overlay".to_string()];
    let mut cautions = Vec::new();

    if workspace.permissions.can_read {
        enabled_capabilities.push("Read files and inspect project state".to_string());
    }
    if workspace.permissions.can_write {
        enabled_capabilities.push("Write files and generate artifacts".to_string());
    } else {
        cautions.push("File generation is limited because write access is disabled.".to_string());
    }
    if workspace.permissions.can_execute {
        enabled_capabilities.push("Execute workspace commands when approved".to_string());
    } else {
        cautions.push("Shell execution is disabled in this workspace.".to_string());
    }
    if workspace.permissions.can_delete {
        enabled_capabilities.push("Prune or archive stale files".to_string());
    } else {
        cautions.push("Delete-style cleanup is disabled in this workspace.".to_string());
    }
    if workspace.allowed_paths.is_empty() {
        cautions.push("No allowed paths are configured yet.".to_string());
    }
    if highest_level >= AirlockLevel::Dangerous {
        cautions
            .push("Some selected pack actions will require explicit Airlock approval.".to_string());
    }

    WorkspaceCapabilitySummary {
        label: match trust_preset {
            "conservative" => "Conservative".to_string(),
            "elevated" => "Elevated".to_string(),
            _ => "Balanced".to_string(),
        },
        effective_tool_policy_mode: trust_mode_label(trust_preset).to_string(),
        allowed_paths_count: workspace.allowed_paths.len(),
        permissions: WorkspaceCapabilityPermissions {
            can_read: workspace.permissions.can_read,
            can_write: workspace.permissions.can_write,
            can_execute: workspace.permissions.can_execute,
            can_delete: workspace.permissions.can_delete,
            can_create_agents: workspace.permissions.can_create_agents,
        },
        enabled_capabilities,
        cautions,
        enforced_pack_ids: enabled_pack_ids.clone(),
        active_tool_ids,
        requires_explicit_approval: highest_level >= AirlockLevel::Dangerous,
        highest_airlock_level: highest_level as u8,
        suggested_outputs: expected_outputs_for_pack_ids(&enabled_pack_ids),
    }
}

#[cfg(test)]
mod tests {
    use super::{scenario_intent_summary, summarize_planned_actions};

    #[test]
    fn release_readiness_has_explicit_launch_intent() {
        let summary = scenario_intent_summary("release_readiness");
        assert!(summary.contains("ship-readiness"));
    }

    #[test]
    fn planned_actions_group_tools_by_risk_and_type() {
        let summary = summarize_planned_actions(&[
            "write_file".to_string(),
            "delete_file".to_string(),
            "execute_command".to_string(),
            "save_memory".to_string(),
        ]);

        assert_eq!(summary.create_or_update, vec!["write_file".to_string()]);
        assert_eq!(summary.move_or_delete, vec!["delete_file".to_string()]);
        assert_eq!(
            summary.external_actions,
            vec!["execute_command".to_string()]
        );
        assert_eq!(summary.memory_actions, vec!["save_memory".to_string()]);
    }
}
