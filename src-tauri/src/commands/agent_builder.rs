use crate::ai::specs::AgentSpec;
use crate::services::{ATMClient, PromptSkillDiscoveryService};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

fn specs_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    Ok(app_dir.join("agent_specs"))
}

fn materialize_prompt_skills(
    app_handle: &AppHandle,
    workspace_path: Option<&str>,
    mut spec: AgentSpec,
) -> Result<AgentSpec, String> {
    let Some(workspace_path) = workspace_path.filter(|value| !value.trim().is_empty()) else {
        return Ok(spec);
    };

    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    let service = PromptSkillDiscoveryService::new(app_dir);
    let discovered = service.discover(Some(std::path::Path::new(workspace_path)))?;

    let mut merged: BTreeMap<String, crate::ai::specs::PromptSkillBinding> = spec
        .skills
        .prompt_skills
        .into_iter()
        .filter(|binding| binding.enabled)
        .map(|binding| (binding.source_path.clone(), binding))
        .collect();

    for skill in discovered.into_iter().filter(|skill| skill.valid && skill.all_agents_enabled) {
        merged
            .entry(skill.source_path.clone())
            .or_insert_with(|| skill.to_binding());
    }

    spec.skills.prompt_skills = merged.into_values().collect();
    spec.skills
        .prompt_skills
        .sort_by(|a, b| a.name.cmp(&b.name).then(a.source_path.cmp(&b.source_path)));
    Ok(spec)
}

#[tauri::command]
pub async fn save_agent_spec(
    app_handle: AppHandle,
    spec: AgentSpec,
    workspace_path: Option<String>,
) -> Result<String, String> {
    if spec.id.trim().is_empty() {
        return Err("Agent spec id is required".to_string());
    }

    if spec.soul.name.trim().is_empty() {
        return Err("Agent name is required".to_string());
    }

    let spec = materialize_prompt_skills(&app_handle, workspace_path.as_deref(), spec)?;
    let dir = specs_dir(&app_handle)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create spec dir: {}", e))?;

    let path = dir.join(format!("{}.json", spec.id));
    let body = serde_json::to_string_pretty(&spec)
        .map_err(|e| format!("Failed to serialize agent spec: {}", e))?;
    std::fs::write(&path, body).map_err(|e| format!("Failed to write spec file: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn load_agent_spec(app_handle: AppHandle, id: String) -> Result<AgentSpec, String> {
    let dir = specs_dir(&app_handle)?;
    let path = dir.join(format!("{}.json", id));
    let body = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "Failed to read agent spec {}: {}",
            path.to_string_lossy(),
            e
        )
    })?;
    serde_json::from_str(&body).map_err(|e| format!("Invalid agent spec json: {}", e))
}

#[tauri::command]
pub async fn list_agent_specs(app_handle: AppHandle) -> Result<Vec<AgentSpec>, String> {
    let dir = specs_dir(&app_handle)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut specs = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("Failed to list specs: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read spec entry: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let body = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.to_string_lossy(), e))?;
        let spec: AgentSpec = serde_json::from_str(&body)
            .map_err(|e| format!("Invalid json in {}: {}", path.to_string_lossy(), e))?;
        specs.push(spec);
    }

    specs.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(specs)
}

#[tauri::command]
pub async fn deploy_agent_spec(
    app_handle: AppHandle,
    client: State<'_, ATMClient>,
    spec: AgentSpec,
    workspace_path: Option<String>,
) -> Result<serde_json::Value, String> {
    let materialized = materialize_prompt_skills(&app_handle, workspace_path.as_deref(), spec)?;
    save_agent_spec(app_handle, materialized.clone(), workspace_path).await?;
    client.deploy_agent(materialized).await
}
