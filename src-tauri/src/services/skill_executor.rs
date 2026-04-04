mod args;
mod browser;
mod documents;
mod evm;
mod filesystem;
mod registry;
mod scheduler;
mod shell;
mod web;

use crate::models::neural::{CommandResult, QueuedCommand, ToolAccessPolicy};
use crate::services::beam_rpc::BeamRpcService;
use crate::services::browser_controller::BrowserController;
use crate::services::settings::SettingsManager;
use crate::services::third_party_skill_registry::{
    InstalledThirdPartySkill, ThirdPartySkillRegistry,
};
use crate::services::wasm_sandbox::{WasmExecutionRequest, WasmSandboxService};
use crate::services::workspace::WorkspaceManager;
use crate::services::ManagedResearchService;
use crate::services::MemoryManager;
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    memory_manager: Arc<RwLock<Option<Arc<MemoryManager>>>>,
    scheduler: Arc<RwLock<Option<Arc<crate::services::persistent_scheduler::PersistentScheduler>>>>,
    third_party_registry: Arc<ThirdPartySkillRegistry>,
    wasm_sandbox: Arc<WasmSandboxService>,
    mcp_service: Arc<crate::services::mcp_service::McpService>,
    /// Beam RPC + Secure Local Signing Bridge — injected during setup
    beam_rpc: Arc<RwLock<Option<Arc<BeamRpcService>>>>,
}

impl SkillExecutor {
    pub fn get_registered_tool_definitions() -> Vec<crate::ai::provider_types::Tool> {
        registry::registered_tool_definitions()
    }

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
        mcp_service: Arc<crate::services::mcp_service::McpService>,
    ) -> Self {
        let third_party_registry =
            Arc::new(ThirdPartySkillRegistry::new().expect("Failed to init third-party registry"));
        Self {
            workspace_manager,
            managed_research,
            browser,
            memory_manager: Arc::new(RwLock::new(None)),
            scheduler: Arc::new(RwLock::new(None)),
            third_party_registry,
            wasm_sandbox: Arc::new(WasmSandboxService::new()),
            mcp_service,
            beam_rpc: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_beam_rpc(&self, svc: Arc<BeamRpcService>) {
        let mut lock = self.beam_rpc.write().await;
        *lock = Some(svc);
    }

    pub async fn set_memory_manager(&self, mm: Arc<MemoryManager>) {
        let mut lock = self.memory_manager.write().await;
        *lock = Some(mm);
    }

    pub async fn set_scheduler(
        &self,
        scheduler: Arc<crate::services::persistent_scheduler::PersistentScheduler>,
    ) {
        let mut lock = self.scheduler.write().await;
        *lock = Some(scheduler);
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        let provider_manager = Arc::new(crate::ai::provider::AIProviderManager::new(
            crate::services::KeychainAccessService::new(),
        ));
        let research = Arc::new(ManagedResearchService::new(provider_manager));
        let browser = Arc::new(BrowserController::new());
        let wm = Arc::new(WorkspaceManager::new().unwrap_or_else(|_| {
            panic!("Failed to create mock WorkspaceManager for test");
        }));

        Self {
            workspace_manager: wm,
            managed_research: research,
            browser,
            memory_manager: Arc::new(RwLock::new(None)),
            scheduler: Arc::new(RwLock::new(None)),
            third_party_registry: Arc::new(
                ThirdPartySkillRegistry::new().expect("mock third-party registry"),
            ),
            wasm_sandbox: Arc::new(WasmSandboxService::new()),
            mcp_service: Arc::new(crate::services::mcp_service::McpService::new()),
            beam_rpc: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn execute(&self, command: &QueuedCommand) -> CommandResult {
        let payload = &command.payload;
        let skill = payload.skill.as_deref().unwrap_or("unknown");
        let method = payload.method.as_deref().unwrap_or("unknown");

        let workspace_id = match &command.workspace_id {
            Some(id) => id.clone(),
            None => return self.error("Missing workspace ID in command"),
        };
        let fallback_policy_state =
            if payload.tool_access_policy.is_none() || payload.allowed_paths.is_empty() {
                let settings = SettingsManager::new();
                let effective = crate::services::LocalAgentSecurityService::resolve(
                    &self.workspace_manager,
                    &settings,
                    &workspace_id,
                    None,
                );
                Some(effective)
            } else {
                None
            };
        let tool_policy = payload.tool_access_policy.as_ref().or(fallback_policy_state
            .as_ref()
            .map(|policy| &policy.tool_access_policy));

        let fallback_allowed_paths = fallback_policy_state
            .as_ref()
            .map(|policy| policy.allowed_paths.clone())
            .unwrap_or_default();
        let fallback_blocked_paths = fallback_policy_state
            .as_ref()
            .map(|policy| policy.blocked_paths.clone())
            .unwrap_or_default();
        let fallback_allowed_domains = fallback_policy_state
            .as_ref()
            .map(|policy| policy.allowed_domains.clone())
            .unwrap_or_default();
        let fallback_blocked_domains = fallback_policy_state
            .as_ref()
            .map(|policy| policy.blocked_domains.clone())
            .unwrap_or_default();
        let allowed_paths = if payload.allowed_paths.is_empty() {
            &fallback_allowed_paths
        } else {
            &payload.allowed_paths
        };
        let blocked_paths = if payload.blocked_paths.is_empty() {
            &fallback_blocked_paths
        } else {
            &payload.blocked_paths
        };
        let allowed_domains = if payload.allowed_domains.is_empty() {
            &fallback_allowed_domains
        } else {
            &payload.allowed_domains
        };
        let blocked_domains = if payload.blocked_domains.is_empty() {
            &fallback_blocked_domains
        } else {
            &payload.blocked_domains
        };
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

        if let Some(server_name) =
            crate::services::mcp_service::McpService::extract_mcp_server(method)
        {
            let params = payload
                .params
                .clone()
                .unwrap_or_else(|| serde_json::json!({}));
            let result = self
                .mcp_service
                .call_mcp_tool(&server_name, method, params)
                .await;
            return CommandResult {
                success: result.is_ok(),
                output: Some(result.unwrap_or_else(|e| format!("MCP Error: {}", e))),
                error: None,
                exit_code: Some(0), // we map all to output in rainy architecture generally unless explicitly errored
            };
        } else if crate::services::mcp_service::McpService::is_mcp_tool(method) {
            return CommandResult {
                success: false,
                output: Some(
                    "Invalid MCP tool name format. Expected mcp_{server}_{tool}".to_string(),
                ),
                error: None,
                exit_code: Some(1),
            };
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
            "memory" => {
                self.execute_memory(&workspace_id, method, &payload.params)
                    .await
            }
            "documents" => {
                self.execute_documents(
                    workspace_id,
                    method,
                    &payload.params,
                    allowed_paths,
                    blocked_paths,
                )
                .await
            }
            "workspace" => {
                self.execute_workspace_tools(workspace_id, method, &payload.params)
                    .await
            }
            "evm" => {
                self.execute_evm(workspace_id, method, &payload.params).await
            }
            _ => self
                .execute_third_party_skill(
                    command,
                    skill,
                    method,
                    &payload.params,
                    allowed_paths,
                    blocked_paths,
                    allowed_domains,
                    blocked_domains,
                )
                .await
                .unwrap_or_else(|| CommandResult {
                    success: false,
                    output: None,
                    error: Some(format!("Unknown skill: {}", skill)),
                    exit_code: Some(1),
                }),
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

    async fn execute_memory(
        &self,
        workspace_id: &str,
        method: &str,
        params: &Option<serde_json::Value>,
    ) -> CommandResult {
        let lock = self.memory_manager.read().await;
        let mm = match lock.as_ref() {
            Some(m) => m,
            None => return self.error("MemoryManager not initialized"),
        };

        let empty_params = serde_json::Value::Object(serde_json::Map::new());
        let params = params.as_ref().unwrap_or(&empty_params);

        // Explicit agent memory tools (save_memory / recall_memory) always use a
        // stable cross-chat namespace so facts persist across sessions and threads.
        // workspace-scoped search_memory continues to respect the originating scope.
        const USER_GLOBAL_NS: &str = "user:global";

        match method {
            "recall_memory" => {
                let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

                // Search user:global first, then current workspace, deduplicate by id.
                let mut combined = Vec::new();
                let mut seen_ids = std::collections::HashSet::new();

                for ns in &[USER_GLOBAL_NS, workspace_id] {
                    if let Ok(hits) = mm.search(ns, query, limit).await {
                        for entry in hits {
                            if seen_ids.insert(entry.id.clone()) {
                                combined.push(entry);
                            }
                        }
                    }
                }

                combined.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                combined.truncate(limit);

                CommandResult {
                    success: true,
                    output: Some(serde_json::to_string(&combined).unwrap_or_default()),
                    error: None,
                    exit_code: Some(0),
                }
            }
            "search_memory" => {
                let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

                match mm.search(workspace_id, query, limit).await {
                    Ok(results) => CommandResult {
                        success: true,
                        output: Some(serde_json::to_string(&results).unwrap_or_default()),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e.to_string()),
                }
            }
            "save_memory" => {
                let content = match params.get("content").and_then(|v| v.as_str()) {
                    Some(c) if !c.trim().is_empty() => c.to_string(),
                    _ => return self.error("save_memory requires a non-empty 'content' field"),
                };
                let tags: Vec<String> = params
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|t| t.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();

                let preview = content[..content.len().min(120)].to_string();
                // Always persist explicit user facts to the global namespace so
                // they are retrievable from any chat, thread, or workspace.
                match mm
                    .store_workspace_memory(
                        USER_GLOBAL_NS,
                        uuid::Uuid::new_v4().to_string(),
                        content,
                        "agent".to_string(),
                        tags,
                        std::collections::HashMap::new(),
                        chrono::Utc::now().timestamp(),
                        crate::services::memory_vault::MemorySensitivity::Internal,
                    )
                    .await
                {
                    Ok(()) => CommandResult {
                        success: true,
                        output: Some(format!("Memory saved: {}", preview)),
                        error: None,
                        exit_code: Some(0),
                    },
                    Err(e) => self.error(&e.to_string()),
                }
            }
            _ => self.error(&format!("Unknown memory method: {}", method)),
        }
    }

    async fn execute_third_party_skill(
        &self,
        command: &QueuedCommand,
        skill: &str,
        method: &str,
        params: &Option<serde_json::Value>,
        allowed_paths: &[String],
        blocked_paths: &[String],
        allowed_domains: &[String],
        blocked_domains: &[String],
    ) -> Option<CommandResult> {
        let resolved = match self.third_party_registry.resolve_method(skill, method) {
            Ok(Some(r)) => r,
            Ok(None) => return None,
            Err(e) => {
                return Some(
                    self.error(&format!("Failed to load third-party skill registry: {}", e)),
                )
            }
        };

        let (skill_def, method_def) = resolved;
        if command.airlock_level < method_def.airlock_level {
            return Some(self.error(&format!(
                "Command Airlock level {:?} is lower than third-party method '{}' required level {:?}",
                command.airlock_level, method_def.name, method_def.airlock_level
            )));
        }
        if let Err(e) = Self::validate_third_party_scopes(
            &skill_def,
            allowed_paths,
            blocked_paths,
            allowed_domains,
            blocked_domains,
        ) {
            return Some(self.error(&e));
        }
        let params_json = params
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "{}".to_string());

        let result = self
            .wasm_sandbox
            .execute(WasmExecutionRequest {
                skill: skill_def,
                method: method_def,
                params_json,
            })
            .await
            .into_command_result();
        Some(result)
    }

    fn validate_third_party_scopes(
        skill: &InstalledThirdPartySkill,
        allowed_paths: &[String],
        blocked_paths: &[String],
        allowed_domains: &[String],
        blocked_domains: &[String],
    ) -> Result<(), String> {
        for fs_perm in &skill.permissions.filesystem {
            let normalized_target = Self::normalize_absolute_path(Path::new(&fs_perm.host_path))
                .map_err(|e| format!("Invalid third-party skill filesystem permission: {}", e))?;

            if !allowed_paths.is_empty()
                && !allowed_paths.iter().any(|allowed| {
                    Self::normalize_absolute_path(Path::new(allowed))
                        .map(|root| normalized_target.starts_with(root))
                        .unwrap_or(false)
                })
            {
                return Err(format!(
                    "Third-party skill '{}' filesystem permission '{}' is outside command allowed paths",
                    skill.id, fs_perm.host_path
                ));
            }

            if Self::is_path_blocked(&normalized_target, blocked_paths, allowed_paths) {
                return Err(format!(
                    "Third-party skill '{}' filesystem permission '{}' is blocked by Airlock scopes",
                    skill.id, fs_perm.host_path
                ));
            }
        }

        for domain in &skill.permissions.network_domains {
            if blocked_domains
                .iter()
                .any(|rule| Self::domain_rule_matches(domain, rule))
            {
                return Err(format!(
                    "Third-party skill '{}' network domain '{}' is blocked by Airlock scopes",
                    skill.id, domain
                ));
            }
            if !allowed_domains.is_empty()
                && !allowed_domains
                    .iter()
                    .any(|rule| Self::domain_rule_matches(domain, rule))
            {
                return Err(format!(
                    "Third-party skill '{}' network domain '{}' is outside Airlock allowed_domains",
                    skill.id, domain
                ));
            }
        }

        Ok(())
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
        assert!(!SkillExecutor::is_tool_allowed(
            "execute_command",
            Some(&policy)
        ));
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
    use std::path::Path;

    #[test]
    fn shell_allowlist_matches_agents_policy() {
        for cmd in [
            "npm", "pnpm", "bun", "cargo", "git", "ls", "grep", "echo", "cat",
        ] {
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
        let temp_root = std::env::temp_dir().join("rainy-mate-normalize");
        let input = temp_root.join("../rainy-mate-normalize/./src-tauri");
        let normalized =
            SkillExecutor::normalize_absolute_path(&input).expect("expected valid absolute path");

        assert_eq!(
            normalized.to_string_lossy(),
            temp_root.join("src-tauri").to_string_lossy()
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
        assert!(!SkillExecutor::domain_rule_matches(
            "evil.com",
            "example.com"
        ));
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
        let project_root = std::env::temp_dir().join("rainy-mate-skill-executor");
        let target = project_root.join("src-tauri/src/services/skill_executor.rs");
        let allowed = vec![project_root.to_string_lossy().to_string()];
        let blocked_relative = vec!["src-tauri/src/services".to_string()];
        let blocked_absolute = vec![project_root.join("src-tauri").to_string_lossy().to_string()];

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
