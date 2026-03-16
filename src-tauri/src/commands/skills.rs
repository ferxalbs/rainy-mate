use crate::models::neural::{
    AirlockLevel, CommandPriority, CommandResult, CommandStatus, QueuedCommand, RainyPayload,
};
use crate::services::{
    skill_installer::{verify_downloaded_bundle_signature, write_temp_downloaded_skill},
    SkillExecutor, SkillInstaller,
};
use crate::services::ThirdPartySkillRegistry;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State};
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
        approved_by: Some("user".to_string()),
        result: None,
        created_at: Some(chrono::Utc::now().timestamp()),
        started_at: Some(chrono::Utc::now().timestamp()),
        completed_at: None,
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

#[tauri::command]
pub async fn list_installed_skills() -> Result<Vec<crate::services::third_party_skill_registry::InstalledThirdPartySkill>, String> {
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

#[tauri::command]
pub async fn remove_installed_skill(
    req: RemoveInstalledSkillRequest,
) -> Result<(), String> {
    let registry = ThirdPartySkillRegistry::new()?;
    let installed = registry
        .list_skills()?
        .into_iter()
        .find(|skill| skill.id == req.skill_id && skill.version == req.version)
        .ok_or_else(|| format!("Skill {}@{} not found", req.skill_id, req.version))?;
    registry.remove(&req.skill_id, &req.version)
        .and_then(|_| {
            let binary_path = std::path::Path::new(&installed.binary_path);
            let install_dir = binary_path
                .parent()
                .ok_or_else(|| "Installed skill binary path has no parent directory".to_string())?;
            std::fs::remove_dir_all(install_dir)
                .map_err(|e| format!("Failed to remove installed skill directory: {}", e))
        })
}

#[tauri::command]
pub async fn install_local_skill(req: SkillInstallRequest) -> Result<crate::services::third_party_skill_registry::InstalledThirdPartySkill, String> {
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
        return Err(format!("ATM public key fetch failed ({}): {}", status, body));
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

    let temp_dir = write_temp_downloaded_skill(&bundle.skill_id, &bundle.manifest_toml, &wasm_bytes)?;
    let installer = SkillInstaller::new()?;
    installer.install_from_downloaded_bundle(&temp_dir, Some(&public_key.public_key_hex))
}
