mod args;
mod browser;
mod filesystem;
mod registry;
mod shell;
mod web;

use crate::models::neural::{CommandResult, QueuedCommand, ToolAccessPolicy};
use crate::services::browser_controller::BrowserController;
use crate::services::settings::SettingsManager;
use crate::services::workspace::WorkspaceManager;
use crate::services::ManagedResearchService;
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use std::sync::Arc;

const MAX_TOOL_OUTPUT_BYTES: usize = 48 * 1024;

fn truncate_output(input: &str) -> String {
    if input.len() <= MAX_TOOL_OUTPUT_BYTES {
        return input.to_string();
    }
    let mut cut = 0usize;
    for (idx, _) in input.char_indices() {
        if idx <= MAX_TOOL_OUTPUT_BYTES {
            cut = idx;
        } else {
            break;
        }
    }
    let mut out = input[..cut].to_string();
    out.push_str("\n\n[TRUNCATED: output exceeded tool limit]");
    out
}

pub struct SkillExecutor {
    workspace_manager: Arc<WorkspaceManager>,
    managed_research: Arc<ManagedResearchService>,
    browser: Arc<BrowserController>,
}

impl SkillExecutor {
    fn is_allowed_shell_command(command: &str) -> bool {
        matches!(
            command,
            "npm" | "pnpm" | "bun" | "cargo" | "git" | "ls" | "grep" | "echo" | "cat"
        )
    }

    fn is_tool_allowed(method: &str, policy: Option<&ToolAccessPolicy>) -> bool {
        let Some(policy) = policy else {
            return true;
        };

        if !policy.enabled {
            return false;
        }

        if policy.deny.iter().any(|tool| tool == method) {
            return false;
        }

        if policy.mode == "allowlist" {
            return policy.allow.iter().any(|tool| tool == method);
        }

        true
    }

    fn hash_tool_policy(policy: &ToolAccessPolicy) -> String {
        let mut allow = policy.allow.clone();
        allow.sort();
        let mut deny = policy.deny.clone();
        deny.sort();
        let canonical = serde_json::json!({
            "enabled": policy.enabled,
            "mode": policy.mode,
            "allow": allow,
            "deny": deny,
        })
        .to_string();
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn new(
        workspace_manager: Arc<WorkspaceManager>,
        managed_research: Arc<ManagedResearchService>,
        browser: Arc<BrowserController>,
    ) -> Self {
        Self {
            workspace_manager,
            managed_research,
            browser,
        }
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        let provider_manager = Arc::new(crate::ai::provider::AIProviderManager::new());
        let research = Arc::new(ManagedResearchService::new(provider_manager));
        let browser = Arc::new(BrowserController::new());
        let wm = Arc::new(WorkspaceManager::new().unwrap_or_else(|_| {
            panic!("Failed to create mock WorkspaceManager for test");
        }));

        Self {
            workspace_manager: wm,
            managed_research: research,
            browser,
        }
    }

    pub async fn execute(&self, command: &QueuedCommand) -> CommandResult {
        let payload = &command.payload;
        let skill = payload.skill.as_deref().unwrap_or("unknown");
        let method = payload.method.as_deref().unwrap_or("unknown");
        let tool_policy = payload.tool_access_policy.as_ref();

        let workspace_id = match &command.workspace_id {
            Some(id) => id.clone(),
            None => return self.error("Missing workspace ID in command"),
        };

        let allowed_paths = &payload.allowed_paths;
        let blocked_paths = &payload.blocked_paths;
        let allowed_domains = &payload.allowed_domains;
        let blocked_domains = &payload.blocked_domains;
        if let (Some(policy), Some(expected_hash)) =
            (tool_policy, payload.tool_access_policy_hash.as_deref())
        {
            let actual_hash = Self::hash_tool_policy(policy);
            if actual_hash != expected_hash {
                return self.error("Tool policy hash mismatch; rejecting command");
            }
        }
        if let Some(version) = payload.tool_access_policy_version {
            let mut settings = SettingsManager::new();
            let last_seen = settings.get_tool_policy_floor(&workspace_id);
            if version < last_seen {
                return self.error(&format!(
                    "Stale tool policy version {} (latest seen {})",
                    version, last_seen
                ));
            }
            if version > last_seen {
                if let Err(e) = settings.set_tool_policy_floor(&workspace_id, version) {
                    return self.error(&format!(
                        "Failed to persist tool policy version floor: {}",
                        e
                    ));
                }
            }
        }
        if !Self::is_tool_allowed(method, tool_policy) {
            return self.error(&format!(
                "Tool '{}' is blocked by workspace tool policy",
                method
            ));
        }

        match skill {
            "filesystem" => {
                self.execute_filesystem(
                    workspace_id,
                    method,
                    &payload.params,
                    allowed_paths,
                    blocked_paths,
                )
                .await
            }
            "shell" => {
                self.execute_shell(
                    workspace_id,
                    method,
                    &payload.params,
                    allowed_paths,
                    blocked_paths,
                )
                .await
            }
            "web" => {
                self.execute_web(method, &payload.params, allowed_domains, blocked_domains)
                    .await
            }
            "browser" => {
                self.execute_browser(method, &payload.params, allowed_domains, blocked_domains)
                    .await
            }
            _ => CommandResult {
                success: false,
                output: None,
                error: Some(format!("Unknown skill: {}", skill)),
                exit_code: Some(1),
            },
        }
    }

    fn validate_http_url(url: &str) -> Result<reqwest::Url, String> {
        let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;

        match parsed.scheme() {
            "http" | "https" => {}
            _ => return Err("Only http:// and https:// URLs are allowed".to_string()),
        }

        let host = parsed
            .host_str()
            .ok_or_else(|| "URL must include a valid host".to_string())?
            .to_ascii_lowercase();

        if host == "localhost" || host.ends_with(".localhost") {
            return Err("localhost URLs are blocked".to_string());
        }

        if let Ok(ip) = host.parse::<IpAddr>() {
            match ip {
                IpAddr::V4(v4) => {
                    if v4.is_loopback() || v4.is_private() || v4.is_link_local() {
                        return Err("Private or loopback IPs are blocked".to_string());
                    }
                }
                IpAddr::V6(v6) => {
                    if v6.is_loopback() || v6.is_unique_local() || v6.is_unspecified() {
                        return Err("Private or loopback IPs are blocked".to_string());
                    }
                }
            }
        }

        Ok(parsed)
    }

    fn domain_rule_matches(host: &str, rule: &str) -> bool {
        let normalized_host = host.trim().trim_end_matches('.').to_ascii_lowercase();
        let normalized_rule = rule.trim().trim_end_matches('.').to_ascii_lowercase();
        if normalized_rule.is_empty() {
            return false;
        }
        if normalized_rule == "*" {
            return true;
        }
        if let Some(root) = normalized_rule.strip_prefix("*.") {
            return normalized_host == root || normalized_host.ends_with(&format!(".{}", root));
        }
        normalized_host == normalized_rule
            || normalized_host.ends_with(&format!(".{}", normalized_rule))
    }

    fn enforce_domain_scope(
        url: &str,
        allowed_domains: &[String],
        blocked_domains: &[String],
    ) -> Result<(), String> {
        let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| "URL must include a valid host".to_string())?
            .to_ascii_lowercase();

        if blocked_domains
            .iter()
            .any(|rule| Self::domain_rule_matches(&host, rule))
        {
            return Err(format!("Domain '{}' is blocked by Airlock scopes", host));
        }

        if !allowed_domains.is_empty()
            && !allowed_domains
                .iter()
                .any(|rule| Self::domain_rule_matches(&host, rule))
        {
            return Err(format!(
                "Domain '{}' is not in Airlock allowed_domains",
                host
            ));
        }

        Ok(())
    }

    fn error(&self, msg: &str) -> CommandResult {
        CommandResult {
            success: false,
            output: None,
            error: Some(msg.to_string()),
            exit_code: Some(1),
        }
    }
}

#[cfg(test)]
mod policy_tests {
    use super::SkillExecutor;
    use crate::models::neural::ToolAccessPolicy;

    #[test]
    fn tool_policy_is_deny_first() {
        let policy = ToolAccessPolicy {
            enabled: true,
            mode: "allowlist".to_string(),
            allow: vec!["read_file".to_string(), "write_file".to_string()],
            deny: vec!["write_file".to_string()],
        };

        assert!(SkillExecutor::is_tool_allowed("read_file", Some(&policy)));
        assert!(!SkillExecutor::is_tool_allowed("write_file", Some(&policy)));
    }

    #[test]
    fn tool_policy_disabled_blocks_all() {
        let policy = ToolAccessPolicy {
            enabled: false,
            mode: "all".to_string(),
            allow: vec![],
            deny: vec![],
        };

        assert!(!SkillExecutor::is_tool_allowed("read_file", Some(&policy)));
        assert!(!SkillExecutor::is_tool_allowed("execute_command", Some(&policy)));
    }

    #[test]
    fn tool_policy_hash_is_stable_for_same_semantics() {
        let policy_a = ToolAccessPolicy {
            enabled: true,
            mode: "allowlist".to_string(),
            allow: vec!["write_file".to_string(), "read_file".to_string()],
            deny: vec!["execute_command".to_string()],
        };
        let policy_b = ToolAccessPolicy {
            enabled: true,
            mode: "allowlist".to_string(),
            allow: vec!["read_file".to_string(), "write_file".to_string()],
            deny: vec!["execute_command".to_string()],
        };

        assert_eq!(
            SkillExecutor::hash_tool_policy(&policy_a),
            SkillExecutor::hash_tool_policy(&policy_b),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::SkillExecutor;
    use std::path::{Path, PathBuf};

    #[test]
    fn shell_allowlist_matches_agents_policy() {
        for cmd in ["npm", "pnpm", "bun", "cargo", "git", "ls", "grep", "echo", "cat"] {
            assert!(SkillExecutor::is_allowed_shell_command(cmd));
        }
    }

    #[test]
    fn shell_allowlist_blocks_dangerous_commands() {
        for cmd in ["rm", "curl", "wget", "kill", "mv", "cp", "node"] {
            assert!(!SkillExecutor::is_allowed_shell_command(cmd));
        }
    }

    #[test]
    fn normalize_absolute_path_collapses_dot_segments() {
        let normalized = SkillExecutor::normalize_absolute_path(Path::new(
            "/Users/fer/Projects/../Projects/rainy-cowork/./src-tauri",
        ))
        .expect("expected valid absolute path");

        assert_eq!(
            normalized.to_string_lossy(),
            "/Users/fer/Projects/rainy-cowork/src-tauri"
        );
    }

    #[test]
    fn normalize_absolute_path_rejects_relative_paths() {
        let err =
            SkillExecutor::normalize_absolute_path(Path::new("relative/path/to/file")).unwrap_err();
        assert!(err.contains("must be absolute"));
    }

    #[test]
    fn domain_scope_matches_exact_and_wildcard_rules() {
        assert!(SkillExecutor::domain_rule_matches(
            "api.example.com",
            "example.com"
        ));
        assert!(SkillExecutor::domain_rule_matches(
            "api.example.com",
            "*.example.com"
        ));
        assert!(!SkillExecutor::domain_rule_matches("evil.com", "example.com"));
    }

    #[test]
    fn domain_scope_enforces_blocked_before_allowed() {
        let err = SkillExecutor::enforce_domain_scope(
            "https://api.example.com/v1",
            &["example.com".to_string()],
            &["api.example.com".to_string()],
        )
        .unwrap_err();

        assert!(err.contains("blocked"));
    }

    #[test]
    fn blocked_paths_reject_relative_and_absolute_matches() {
        let target =
            PathBuf::from("/Users/fer/Projects/rainy-cowork/src-tauri/src/services/skill_executor.rs");
        let allowed = vec!["/Users/fer/Projects/rainy-cowork".to_string()];
        let blocked_relative = vec!["src-tauri/src/services".to_string()];
        let blocked_absolute = vec!["/Users/fer/Projects/rainy-cowork/src-tauri".to_string()];

        assert!(SkillExecutor::is_path_blocked(
            &target,
            &blocked_relative,
            &allowed
        ));
        assert!(SkillExecutor::is_path_blocked(
            &target,
            &blocked_absolute,
            &allowed
        ));
    }
}
