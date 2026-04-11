use crate::models::neural::{
    AirlockLevel, CommandPriority, CommandResult, CommandStatus, QueuedCommand, RainyPayload,
};
use crate::services::ThirdPartySkillRegistry;
use crate::services::{
    skill_installer::{verify_downloaded_bundle_signature, write_temp_downloaded_skill},
    PromptSkillDiscoveryService, PromptSkillRegistry, SkillExecutor, SkillInstaller,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

/// Execute a skill directly from the frontend (local Deep Mode execution).
/// This bypasses the cloud command queue and executes file operations locally.
///
/// # Parameters
/// - `workspace_id`: The workspace ID (used for logging/tracking)
/// - `skill`: The skill name (e.g., "filesystem")
/// - `method`: The method name (e.g., "write_file", "read_file")
/// - `params`: JSON parameters for the method
/// - `workspace_path`: The actual filesystem path where files should be created (e.g., "/Users/fer/Projects/myproject")
#[tauri::command]
pub async fn execute_skill(
    skill_executor: State<'_, Arc<SkillExecutor>>,
    workspace_id: String,
    skill: String,
    method: String,
    params: serde_json::Value,
    workspace_path: Option<String>,
) -> Result<CommandResult, String> {
    // For local execution, use the workspace_path as the allowed path
    // This enables relative path resolution in SkillExecutor
    let allowed_paths = match workspace_path {
        Some(path) => vec![path],
        None => vec![], // Will fall back to workspace config lookup
    };

    // Construct a pseudo-command to reuse the existing SkillExecutor logic
    let command = QueuedCommand {
        id: Uuid::new_v4().to_string(),
        workspace_id: Some(workspace_id),
        desktop_node_id: Some("desktop-local".to_string()),
        intent: format!("{}.{}", skill, method),
        payload: RainyPayload {
            skill: Some(skill),
            method: Some(method),
            params: Some(params),
            content: None,
            allowed_paths, // Pass the workspace path for local path resolution
            blocked_paths: vec![],
            allowed_domains: vec![],
            blocked_domains: vec![],
            tool_access_policy: None,
            tool_access_policy_version: None,
            tool_access_policy_hash: None,
            ..Default::default()
        },
        priority: CommandPriority::Normal,
        status: CommandStatus::Pending,
        airlock_level: AirlockLevel::Safe, // Assumed safe since triggered by user via UI
        approval_timeout_secs: None,
        approved_by: Some("user".to_string()),
        result: None,
        created_at: Some(chrono::Utc::now().timestamp()),
        started_at: Some(chrono::Utc::now().timestamp()),
        completed_at: None,
        schema_version: None,
    };

    // Execute directly via the shared SkillExecutor
    let result = skill_executor.execute(&command).await;

    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInstallRequest {
    pub source_dir: String,
    #[serde(default)]
    pub allow_unsigned_dev: bool,
    #[serde(default)]
    pub platform_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSkillInstallRequest {
    pub base_url: String,
    pub skill_id: String,
    pub platform_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetInstalledSkillEnabledRequest {
    pub skill_id: String,
    pub version: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveInstalledSkillRequest {
    pub skill_id: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteSkillBundleResponse {
    pub skill_id: String,
    pub manifest_toml: String,
    pub wasm_base64: String,
    pub package_signature: String,
    pub signature_algorithm: String,
    pub key_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillPublicKeyResponse {
    pub algorithm: String,
    pub public_key_hex: String,
    pub key_id: String,
}

fn app_data_dir(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))
}

#[tauri::command]
pub async fn list_installed_skills(
) -> Result<Vec<crate::services::third_party_skill_registry::InstalledThirdPartySkill>, String> {
    let registry = ThirdPartySkillRegistry::new()?;
    registry.list_skills()
}

#[tauri::command]
pub async fn set_installed_skill_enabled(
    req: SetInstalledSkillEnabledRequest,
) -> Result<(), String> {
    let registry = ThirdPartySkillRegistry::new()?;
    registry.set_enabled(&req.skill_id, &req.version, req.enabled)
}

// PERF: Replacing blocking std::fs operations with non-blocking tokio::fs in the async thread pool
// to prevent executor thread starvation.
#[tauri::command]
pub async fn remove_installed_skill(req: RemoveInstalledSkillRequest) -> Result<(), String> {
    let registry = ThirdPartySkillRegistry::new()?;
    let installed = registry
        .list_skills()?
        .into_iter()
        .find(|skill| skill.id == req.skill_id && skill.version == req.version)
        .ok_or_else(|| format!("Skill {}@{} not found", req.skill_id, req.version))?;
    registry.remove(&req.skill_id, &req.version)?;
    let binary_path = std::path::Path::new(&installed.binary_path);
    let install_dir = binary_path
        .parent()
        .ok_or_else(|| "Installed skill binary path has no parent directory".to_string())?;
    tokio::fs::remove_dir_all(install_dir)
        .await
        .map_err(|e| format!("Failed to remove installed skill directory: {}", e))
}

#[tauri::command]
pub async fn install_local_skill(
    req: SkillInstallRequest,
) -> Result<crate::services::third_party_skill_registry::InstalledThirdPartySkill, String> {
    let installer = SkillInstaller::new()?;
    installer.install_from_directory(
        std::path::Path::new(&req.source_dir),
        req.platform_key.as_deref(),
        req.allow_unsigned_dev,
    )
}

#[tauri::command]
pub async fn install_skill_from_atm(
    _app_handle: AppHandle,
    req: RemoteSkillInstallRequest,
) -> Result<crate::services::third_party_skill_registry::InstalledThirdPartySkill, String> {
    let public_key_url = format!(
        "{}/v1/skills/public-key",
        req.base_url.trim_end_matches('/')
    );
    let url = format!(
        "{}/v1/skills/{}/download",
        req.base_url.trim_end_matches('/'),
        req.skill_id
    );
    let client = reqwest::Client::new();
    let key_response = client
        .get(&public_key_url)
        .header("Authorization", format!("Bearer {}", req.platform_key))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch ATM public key: {}", e))?;

    if !key_response.status().is_success() {
        let status = key_response.status();
        let body = key_response.text().await.unwrap_or_default();
        return Err(format!(
            "ATM public key fetch failed ({}): {}",
            status, body
        ));
    }

    let public_key: SkillPublicKeyResponse = key_response
        .json()
        .await
        .map_err(|e| format!("Invalid ATM public key response: {}", e))?;
    if public_key.algorithm != "ed25519" {
        return Err(format!(
            "Unsupported ATM public key algorithm: {}",
            public_key.algorithm
        ));
    }

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", req.platform_key))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch skill from ATM: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("ATM skill download failed ({}): {}", status, body));
    }

    let bundle: RemoteSkillBundleResponse = response
        .json()
        .await
        .map_err(|e| format!("Invalid ATM skill response: {}", e))?;
    let wasm_bytes = base64::engine::general_purpose::STANDARD
        .decode(bundle.wasm_base64.as_bytes())
        .map_err(|e| format!("Invalid wasm base64 payload: {}", e))?;
    if bundle.signature_algorithm != "ed25519" {
        return Err(format!(
            "Unsupported ATM bundle signature algorithm: {}",
            bundle.signature_algorithm
        ));
    }
    if let Some(bundle_key_id) = &bundle.key_id {
        if *bundle_key_id != public_key.key_id {
            return Err(format!(
                "ATM bundle key mismatch: bundle={} public-key={}",
                bundle_key_id, public_key.key_id
            ));
        }
    }
    if !verify_downloaded_bundle_signature(
        &bundle.manifest_toml,
        &wasm_bytes,
        &bundle.package_signature,
        &public_key.public_key_hex,
    ) {
        return Err("ATM skill bundle signature verification failed".to_string());
    }

    let temp_dir =
        write_temp_downloaded_skill(&bundle.skill_id, &bundle.manifest_toml, &wasm_bytes)?;
    let installer = SkillInstaller::new()?;
    installer.install_from_downloaded_bundle(&temp_dir, Some(&public_key.public_key_hex))
}

#[tauri::command]
pub async fn list_prompt_skills(
    app_handle: AppHandle,
    workspace_path: Option<String>,
) -> Result<Vec<crate::services::DiscoveredPromptSkill>, String> {
    let service = PromptSkillDiscoveryService::new(app_data_dir(&app_handle)?);
    let workspace_path = workspace_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(std::path::Path::new);
    service.discover(workspace_path)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPromptSkillAllAgentsEnabledRequest {
    pub source_path: String,
    pub enabled: bool,
    #[serde(default)]
    pub workspace_path: Option<String>,
}

#[tauri::command]
pub async fn set_prompt_skill_all_agents_enabled(
    app_handle: AppHandle,
    req: SetPromptSkillAllAgentsEnabledRequest,
) -> Result<(), String> {
    let workspace_path = req
        .workspace_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(std::path::Path::new);
    let registry = if let Some(workspace_path) = workspace_path {
        let source_path = std::path::Path::new(&req.source_path);
        if source_path.starts_with(workspace_path) {
            PromptSkillRegistry::project(workspace_path)?
        } else {
            PromptSkillRegistry::global(&app_data_dir(&app_handle)?)?
        }
    } else {
        PromptSkillRegistry::global(&app_data_dir(&app_handle)?)?
    };
    registry.set_all_agents_enabled(&req.source_path, req.enabled)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshPromptSkillSnapshotRequest {
    pub source_path: String,
    #[serde(default)]
    pub workspace_path: Option<String>,
}

#[tauri::command]
pub async fn refresh_prompt_skill_snapshot(
    app_handle: AppHandle,
    req: RefreshPromptSkillSnapshotRequest,
) -> Result<crate::services::PromptSkillBinding, String> {
    let service = PromptSkillDiscoveryService::new(app_data_dir(&app_handle)?);
    let workspace_path = req
        .workspace_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(std::path::Path::new);
    service.refresh_binding(workspace_path, std::path::Path::new(&req.source_path))
}

// ===== Plan Execution Commands =====

/// A tool call parsed from agent response content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedToolCall {
    pub skill: String,
    pub method: String,
    pub params: serde_json::Value,
}

/// Parse filesystem tool calls from agent response content.
/// Extracts write_file / append_file / read_file / list_files / search_files
/// call syntax that the agent may include in its responses.
#[tauri::command]
pub async fn parse_tool_calls(content: String) -> Result<Vec<ParsedToolCall>, String> {
    Ok(parse_tool_calls_from_content(&content))
}

/// Internal helper — parse tool calls without async overhead, reusable from
/// `execute_plan_from_content`.
pub fn parse_tool_calls_from_content(content: &str) -> Vec<ParsedToolCall> {
    use regex::Regex;
    let mut calls: Vec<ParsedToolCall> = Vec::new();

    let patterns: &[(&str, &str, &str)] = &[
        // (method, skill, regex)
        (
            "write_file",
            "filesystem",
            r#"write_file\s*\(\s*["']([^"']+)["']\s*,\s*["']?([^)]*?)["']?\s*\)"#,
        ),
        (
            "append_file",
            "filesystem",
            r#"append_file\s*\(\s*["']([^"']+)["']\s*,\s*["']?([^)]*?)["']?\s*\)"#,
        ),
        (
            "read_file",
            "filesystem",
            r#"read_file\s*\(\s*["']([^"']+)["']\s*\)"#,
        ),
        (
            "list_files",
            "filesystem",
            r#"list_files\s*\(\s*["']([^"']+)["']\s*\)"#,
        ),
    ];

    for &(method, skill, pattern) in patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(content) {
                let params = if method == "write_file" || method == "append_file" {
                    serde_json::json!({ "path": cap[1].to_string(), "content": cap[2].to_string() })
                } else {
                    serde_json::json!({ "path": cap[1].to_string() })
                };
                calls.push(ParsedToolCall {
                    skill: skill.to_string(),
                    method: method.to_string(),
                    params,
                });
            }
        }
    }

    // search_files("query", optional "path")
    if let Ok(re) =
        Regex::new(r#"search_files\s*\(\s*["']([^"']+)["']\s*(?:,\s*["']([^"']+)["'])?\s*\)"#)
    {
        for cap in re.captures_iter(content) {
            let path = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            calls.push(ParsedToolCall {
                skill: "filesystem".to_string(),
                method: "search_files".to_string(),
                params: serde_json::json!({ "query": cap[1].to_string(), "path": path }),
            });
        }
    }

    calls
}

/// Result of executing a plan extracted from agent response content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutePlanResult {
    pub success: bool,
    pub summary: String,
    pub executed_count: usize,
    pub error: Option<String>,
}

/// Parse and execute all filesystem tool calls found in agent response content.
/// Returns a structured result with per-step details — keeps orchestration logic in Rust.
#[tauri::command]
pub async fn execute_plan_from_content(
    skill_executor: State<'_, Arc<SkillExecutor>>,
    workspace_id: String,
    content: String,
    workspace_path: Option<String>,
) -> Result<ExecutePlanResult, String> {
    let calls = parse_tool_calls_from_content(&content);

    if calls.is_empty() {
        return Ok(ExecutePlanResult {
            success: false,
            summary: String::new(),
            executed_count: 0,
            error: Some("No executable operations found in the plan.".to_string()),
        });
    }

    let allowed_paths = workspace_path.map(|p| vec![p]).unwrap_or_default();

    let mut completed: Vec<String> = Vec::new();

    for call in &calls {
        let command = QueuedCommand {
            id: Uuid::new_v4().to_string(),
            workspace_id: Some(workspace_id.clone()),
            desktop_node_id: Some("desktop-local".to_string()),
            intent: format!("{}.{}", call.skill, call.method),
            payload: RainyPayload {
                skill: Some(call.skill.clone()),
                method: Some(call.method.clone()),
                params: Some(call.params.clone()),
                content: None,
                allowed_paths: allowed_paths.clone(),
                blocked_paths: vec![],
                allowed_domains: vec![],
                blocked_domains: vec![],
                tool_access_policy: None,
                tool_access_policy_version: None,
                tool_access_policy_hash: None,
                ..Default::default()
            },
            priority: CommandPriority::Normal,
            status: CommandStatus::Pending,
            airlock_level: AirlockLevel::Safe,
            approval_timeout_secs: None,
            approved_by: Some("user".to_string()),
            result: None,
            created_at: Some(chrono::Utc::now().timestamp()),
            started_at: Some(chrono::Utc::now().timestamp()),
            completed_at: None,
            schema_version: None,
        };

        let result = skill_executor.execute(&command).await;
        let path = call
            .params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if !result.success {
            return Ok(ExecutePlanResult {
                success: false,
                summary: completed.join("\n"),
                executed_count: completed.len(),
                error: Some(format!(
                    "Failed: {}(\"{}\"): {}",
                    call.method,
                    path,
                    result.error.as_deref().unwrap_or("unknown error")
                )),
            });
        }

        completed.push(format!("✅ {}(\"{}\")", call.method, path));
    }

    Ok(ExecutePlanResult {
        success: true,
        summary: format!("**Execution Complete**\n\n{}", completed.join("\n")),
        executed_count: completed.len(),
        error: None,
    })
}
