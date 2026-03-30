use serde::{Deserialize, Serialize};

use crate::services::{Workspace, WorkspaceManager};

const REPO_GUARDIAN_TOOLS: &[&str] = &[
    "read_file",
    "read_many_files",
    "list_files",
    "search_files",
    "git_status",
    "git_diff",
    "git_log",
    "git_show",
    "git_branch_list",
    "web_search",
    "read_web_page",
    "write_file",
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
    pub first_run_completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub first_run_scenario_id: Option<String>,
    pub launch_count: u64,
    pub successful_launch_count: u64,
    pub last_launch_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_launch_chat_id: Option<String>,
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
                tool_ids: REPO_GUARDIAN_TOOLS.iter().map(|tool| tool.to_string()).collect(),
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
                tool_ids: WORKSPACE_FORGER_TOOLS.iter().map(|tool| tool.to_string()).collect(),
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
                tool_ids: INCIDENT_SCRIBE_TOOLS.iter().map(|tool| tool.to_string()).collect(),
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
                tool_ids: KNOWLEDGE_WEAVER_TOOLS
                    .iter()
                    .map(|tool| tool.to_string())
                    .collect(),
            },
        ]
    }

    pub fn first_run_scenarios() -> Vec<FirstRunScenarioDefinition> {
        vec![
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
            last_launch_at: workspace.launchpad.last_launch_at.map(|value| value.to_rfc3339()),
            last_launch_chat_id: workspace.launchpad.last_launch_chat_id.clone(),
            capability_summary: capability_summary(workspace, trust),
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

    pub fn record_workspace_launch(
        workspace_manager: &WorkspaceManager,
        workspace_id: &str,
        scenario_id: &str,
        chat_id: Option<&str>,
        success: bool,
    ) -> Result<WorkspaceLaunchpadSummary, String> {
        let mut workspace = workspace_manager
            .load_workspace(workspace_id)
            .map_err(|e| e.to_string())?;
        let now = chrono::Utc::now();
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
        workspace_manager
            .save_workspace(&workspace, crate::services::ConfigFormat::Json)
            .map_err(|e| e.to_string())?;
        Ok(Self::get_workspace_summary(&workspace))
    }

    pub fn build_first_run_prompt(workspace: &Workspace, scenario_id: &str) -> Result<String, String> {
        let allowed_paths = if workspace.allowed_paths.is_empty() {
            "No explicit allowed paths were configured yet.".to_string()
        } else {
            workspace.allowed_paths.join(", ")
        };

        let trust_preset = normalize_trust_preset(&workspace.launchpad.trust_preset);
        let enabled_packs = sanitize_pack_ids(&workspace.launchpad.enabled_pack_ids);
        let pack_text = if enabled_packs.is_empty() {
            "No packs are enabled yet.".to_string()
        } else {
            enabled_packs.join(", ")
        };

        let scenario_prompt = match scenario_id {
            "codebase_copilot" => "Analyze this workspace like a senior engineer inheriting a production codebase. Read the repo shape, recent changelog context, and workspace memory overlay. Produce: 1) the most important current system truths, 2) the top product or technical risks, and 3) the next concrete engineering actions. Update WORKSTATE if useful.",
            "file_organizer" => "Inspect this workspace as an operations-focused file organizer. Identify clutter, stale artifacts, naming inconsistencies, and obvious archival candidates. Produce a concrete cleanup plan first; if safe, perform bounded organization work and explain every action. Update WORKSTATE with the resulting state.",
            "docs_builder" => "Read the current workspace, changelog context, and any durable memory files. Produce a polished project brief suitable for a technical founder or investor. Generate a native document artifact if the workspace contents support it, and update workspace memory with the decisive summary.",
            _ => return Err(format!("Unknown first-run scenario '{}'", scenario_id)),
        };

        Ok(format!(
            "You are running a guided MaTE launch scenario.\n\nWorkspace: {}\nTrust preset: {}\nEnabled packs: {}\nAllowed paths: {}\n\nConstraints:\n- Treat this as a production-grade workspace.\n- Prefer stable, auditable actions.\n- Use the current workspace files and memory overlay as the source of truth.\n- If you generate a document or archive, keep it inside the active workspace.\n\nTask:\n{}",
            workspace.name, trust_preset, pack_text, allowed_paths, scenario_prompt
        ))
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

fn capability_summary(workspace: &Workspace, trust_preset: &str) -> WorkspaceCapabilitySummary {
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

    let effective_tool_policy_mode = match trust_preset {
        "conservative" => "allowlist",
        "elevated" => "all",
        _ => "balanced_allowlist",
    }
    .to_string();

    WorkspaceCapabilitySummary {
        label: match trust_preset {
            "conservative" => "Conservative".to_string(),
            "elevated" => "Elevated".to_string(),
            _ => "Balanced".to_string(),
        },
        effective_tool_policy_mode,
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
    }
}
