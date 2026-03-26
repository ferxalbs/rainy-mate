use crate::ai::specs::manifest::{AgentSpec, AirlockToolPolicy};
use crate::models::neural::ToolAccessPolicy;
use crate::services::settings::SettingsManager;
use crate::services::workspace::{WorkspaceManager, WorkspacePermissions};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

const READ_TOOLS: &[&str] = &[
    "read_file",
    "read_many_files",
    "read_file_chunk",
    "list_files",
    "list_files_detailed",
    "file_exists",
    "get_file_info",
    "search_files",
    "ingest_document",
    "git_status",
    "git_diff",
    "git_log",
    "git_show",
    "git_branch_list",
    "web_search",
    "read_web_page",
    "http_get_json",
    "http_get_text",
    "screenshot",
    "get_page_content",
    "get_page_snapshot",
    "extract_links",
    "wait_for_selector",
    "pdf_read",
    "excel_read",
    "recall_memory",
];

const WRITE_TOOLS: &[&str] = &[
    "write_file",
    "append_file",
    "mkdir",
    "pdf_create",
    "excel_write",
    "docx_create",
    "archive_create",
    "save_memory",
];

const DELETE_TOOLS: &[&str] = &["delete_file", "move_file"];

const EXECUTE_TOOLS: &[&str] = &[
    "execute_command",
    "browse_url",
    "open_new_tab",
    "click_element",
    "type_text",
    "go_back",
    "submit_form",
    "http_post_json",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveLocalAgentPolicy {
    pub workspace_id: String,
    pub allowed_paths: Vec<String>,
    pub blocked_paths: Vec<String>,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
    pub tool_access_policy: ToolAccessPolicy,
    pub tool_access_policy_source: String,
    pub notifications_enabled: bool,
    pub can_create_agents: bool,
}

pub struct LocalAgentSecurityService;

impl LocalAgentSecurityService {
    pub fn resolve(
        workspace_manager: &Arc<WorkspaceManager>,
        settings: &SettingsManager,
        workspace_id: &str,
        spec: Option<&AgentSpec>,
    ) -> EffectiveLocalAgentPolicy {
        let (workspace_allowed_paths, workspace_permissions, workspace_notifications) =
            match workspace_manager.load_workspace(workspace_id) {
                Ok(workspace) => (
                    workspace.allowed_paths,
                    workspace.permissions,
                    workspace.settings.notifications_enabled,
                ),
                Err(_) => (
                    Self::fallback_allowed_paths(workspace_id),
                    Self::fallback_permissions(),
                    settings.get_settings().notifications_enabled,
                ),
            };

        let mut allowed_paths = workspace_allowed_paths;
        let mut blocked_paths = Vec::new();
        let mut allowed_domains = Vec::new();
        let mut blocked_domains = Vec::new();

        if let Some(spec) = spec {
            allowed_paths =
                Self::merge_allowed_paths(&allowed_paths, &spec.airlock.scopes.allowed_paths);
            blocked_paths = Self::merge_unique(&blocked_paths, &spec.airlock.scopes.blocked_paths);
            allowed_domains =
                Self::merge_allowed_domains(&allowed_domains, &spec.airlock.scopes.allowed_domains);
            blocked_domains =
                Self::merge_unique(&blocked_domains, &spec.airlock.scopes.blocked_domains);
        }

        let (mut tool_access_policy, mut source) =
            if let Some(state) = settings.get_workspace_tool_policy_state(workspace_id) {
                (state.tool_access_policy, "settings".to_string())
            } else {
                (
                    Self::tool_policy_from_permissions(&workspace_permissions),
                    "workspace_permissions".to_string(),
                )
            };

        if let Some(spec) = spec {
            tool_access_policy =
                Self::merge_tool_policy(tool_access_policy, &spec.airlock.tool_policy);
            source.push_str("+spec");
        }

        EffectiveLocalAgentPolicy {
            workspace_id: workspace_id.to_string(),
            allowed_paths,
            blocked_paths,
            allowed_domains,
            blocked_domains,
            tool_access_policy,
            tool_access_policy_source: source,
            notifications_enabled: workspace_notifications
                && settings.get_settings().notifications_enabled,
            can_create_agents: workspace_permissions.can_create_agents,
        }
    }

    pub fn tool_policy_from_permissions(permissions: &WorkspacePermissions) -> ToolAccessPolicy {
        let mut deny = BTreeSet::new();

        if !permissions.can_read {
            deny.extend(READ_TOOLS.iter().map(|tool| (*tool).to_string()));
        }
        if !permissions.can_write {
            deny.extend(WRITE_TOOLS.iter().map(|tool| (*tool).to_string()));
        }
        if !permissions.can_delete {
            deny.extend(DELETE_TOOLS.iter().map(|tool| (*tool).to_string()));
        }
        if !permissions.can_execute {
            deny.extend(EXECUTE_TOOLS.iter().map(|tool| (*tool).to_string()));
        }

        ToolAccessPolicy {
            enabled: true,
            mode: "all".to_string(),
            allow: Vec::new(),
            deny: deny.into_iter().collect(),
        }
    }

    fn merge_tool_policy(
        base: ToolAccessPolicy,
        spec_policy: &AirlockToolPolicy,
    ) -> ToolAccessPolicy {
        let ToolAccessPolicy {
            enabled,
            mode: base_mode,
            allow: base_allow,
            deny: base_deny,
        } = base;
        let mut deny = BTreeSet::new();
        deny.extend(base_deny);
        deny.extend(spec_policy.deny.iter().cloned());

        let allow = if spec_policy.mode == "allowlist" {
            if base_mode == "allowlist" && !base_allow.is_empty() {
                base_allow
                    .into_iter()
                    .filter(|tool| spec_policy.allow.iter().any(|allowed| allowed == tool))
                    .collect()
            } else {
                spec_policy.allow.clone()
            }
        } else {
            base_allow
        };

        ToolAccessPolicy {
            enabled,
            mode: if spec_policy.mode == "allowlist" || base_mode == "allowlist" {
                "allowlist".to_string()
            } else {
                "all".to_string()
            },
            allow,
            deny: deny.into_iter().collect(),
        }
    }

    fn merge_allowed_paths(base: &[String], scoped: &[String]) -> Vec<String> {
        if scoped.is_empty() {
            return base.to_vec();
        }
        if base.is_empty() {
            return scoped.to_vec();
        }

        let mut merged = Vec::new();
        for base_path in base {
            let base_path_ref = Path::new(base_path);
            for scoped_path in scoped {
                let scoped_path_ref = Path::new(scoped_path);
                if base_path_ref.starts_with(scoped_path_ref) {
                    merged.push(base_path.clone());
                } else if scoped_path_ref.starts_with(base_path_ref) {
                    merged.push(scoped_path.clone());
                }
            }
        }

        Self::dedupe_preserve_order(merged)
    }

    fn merge_allowed_domains(base: &[String], scoped: &[String]) -> Vec<String> {
        if scoped.is_empty() {
            return base.to_vec();
        }
        if base.is_empty() {
            return scoped.to_vec();
        }

        let scoped_set = scoped
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        base.iter()
            .filter(|value| scoped_set.contains(&value.to_ascii_lowercase()))
            .cloned()
            .collect()
    }

    fn merge_unique(base: &[String], extra: &[String]) -> Vec<String> {
        let mut merged = Vec::with_capacity(base.len() + extra.len());
        merged.extend(base.iter().cloned());
        merged.extend(extra.iter().cloned());
        Self::dedupe_preserve_order(merged)
    }

    fn dedupe_preserve_order(values: Vec<String>) -> Vec<String> {
        let mut seen = BTreeSet::new();
        values
            .into_iter()
            .filter(|value| seen.insert(value.clone()))
            .collect()
    }

    fn fallback_allowed_paths(workspace_id: &str) -> Vec<String> {
        if Path::new(workspace_id).is_absolute() {
            vec![workspace_id.to_string()]
        } else {
            Vec::new()
        }
    }

    fn fallback_permissions() -> WorkspacePermissions {
        WorkspacePermissions {
            can_read: true,
            can_write: true,
            can_execute: false,
            can_delete: false,
            can_create_agents: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LocalAgentSecurityService;
    use crate::services::workspace::WorkspacePermissions;

    #[test]
    fn permissions_disable_mutating_tools() {
        let policy =
            LocalAgentSecurityService::tool_policy_from_permissions(&WorkspacePermissions {
                can_read: true,
                can_write: false,
                can_execute: false,
                can_delete: false,
                can_create_agents: true,
            });

        assert!(policy.deny.iter().any(|item| item == "write_file"));
        assert!(policy.deny.iter().any(|item| item == "execute_command"));
        assert!(policy.deny.iter().any(|item| item == "delete_file"));
        assert!(!policy.deny.iter().any(|item| item == "read_file"));
    }
}
