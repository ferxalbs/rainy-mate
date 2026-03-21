use crate::services::mcp_service::{
    McpApprovalRequest, McpJsonImportResult, McpPermissionMode, McpRuntimeStatus,
    McpServerConfig, McpServerRuntimeStatus, PersistedMcpServerConfig,
};
use crate::services::McpService;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{command, State};

#[command]
pub async fn list_mcp_servers(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<Vec<PersistedMcpServerConfig>, String> {
    Ok(mcp_service.list_servers().await)
}

#[command]
pub async fn upsert_mcp_server(
    mcp_service: State<'_, Arc<McpService>>,
    config: PersistedMcpServerConfig,
) -> Result<(), String> {
    mcp_service.upsert_server(config).await
}

#[command]
pub async fn remove_mcp_server(
    mcp_service: State<'_, Arc<McpService>>,
    name: String,
) -> Result<(), String> {
    mcp_service.remove_server(&name).await
}

#[command]
pub async fn connect_mcp_saved_server(
    mcp_service: State<'_, Arc<McpService>>,
    name: String,
    env: Option<HashMap<String, String>>,
    headers: Option<HashMap<String, String>>,
) -> Result<(), String> {
    mcp_service.connect_saved_server(&name, env, headers).await
}

#[command]
pub async fn connect_mcp_server(
    mcp_service: State<'_, Arc<McpService>>,
    config: McpServerConfig,
) -> Result<(), String> {
    mcp_service.connect_server(config).await
}

#[command]
pub async fn disconnect_mcp_server(
    mcp_service: State<'_, Arc<McpService>>,
    name: String,
) -> Result<(), String> {
    mcp_service.disconnect_server(&name).await
}

#[command]
pub async fn refresh_mcp_server_tools(
    mcp_service: State<'_, Arc<McpService>>,
    name: String,
) -> Result<(), String> {
    mcp_service.refresh_server_tools(&name).await
}

#[command]
pub async fn list_mcp_runtime_servers(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<Vec<McpServerRuntimeStatus>, String> {
    Ok(mcp_service.list_runtime_statuses().await)
}

#[command]
pub async fn get_mcp_runtime_status(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<McpRuntimeStatus, String> {
    Ok(mcp_service.get_runtime_status().await)
}

#[command]
pub async fn get_mcp_permission_mode(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<McpPermissionMode, String> {
    Ok(mcp_service.get_permission_mode().await)
}

#[command]
pub async fn set_mcp_permission_mode(
    mcp_service: State<'_, Arc<McpService>>,
    mode: McpPermissionMode,
) -> Result<(), String> {
    mcp_service.set_permission_mode(mode).await
}

#[command]
pub async fn get_pending_mcp_approvals(
    mcp_service: State<'_, Arc<McpService>>,
) -> Result<Vec<McpApprovalRequest>, String> {
    Ok(mcp_service.get_pending_approvals().await)
}

#[command]
pub async fn respond_to_mcp_approval(
    mcp_service: State<'_, Arc<McpService>>,
    approval_id: String,
    approved: bool,
) -> Result<(), String> {
    mcp_service.respond_to_approval(&approval_id, approved).await
}

#[command]
pub async fn import_mcp_servers_from_json(
    mcp_service: State<'_, Arc<McpService>>,
    json_path: String,
    auto_connect: bool,
) -> Result<McpJsonImportResult, String> {
    mcp_service
        .import_servers_from_json(&json_path, auto_connect)
        .await
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpJsonConfigFile {
    pub path: String,
    pub content: String,
}

fn default_mcp_json_path() -> PathBuf {
    let base = dirs::config_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    crate::services::app_identity::resolve_child_file(base, "mcp/servers.json")
    .unwrap_or_else(|_| PathBuf::from(".").join("rainy-mate").join("mcp").join("servers.json"))
}

fn default_mcp_json_template() -> String {
    serde_json::json!({
        "mcpServers": {
            "filesystem": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/fer/Projects"],
                "timeoutSecs": 30
            },
            "localHttpExample": {
                "url": "http://127.0.0.1:8787/mcp",
                "timeoutSecs": 30,
                "headers": {
                    "Authorization": "Bearer REPLACE_ME"
                }
            }
        }
    })
    .to_string()
}

#[command]
pub async fn get_or_create_default_mcp_json_config() -> Result<McpJsonConfigFile, String> {
    let path = default_mcp_json_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create MCP config directory: {}", e))?;
    }

    if !path.exists() {
        let template = default_mcp_json_template();
        let pretty = serde_json::to_string_pretty(
            &serde_json::from_str::<serde_json::Value>(&template)
                .map_err(|e| format!("Failed to build MCP default JSON template: {}", e))?,
        )
        .map_err(|e| format!("Failed to pretty-print MCP JSON template: {}", e))?;
        std::fs::write(&path, pretty)
            .map_err(|e| format!("Failed to create MCP JSON config: {}", e))?;
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read MCP JSON config: {}", e))?;
    Ok(McpJsonConfigFile {
        path: path.to_string_lossy().to_string(),
        content,
    })
}

#[command]
pub async fn save_default_mcp_json_config(content: String) -> Result<McpJsonConfigFile, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))?;
    let pretty = serde_json::to_string_pretty(&parsed)
        .map_err(|e| format!("Failed to format JSON content: {}", e))?;
    let path = default_mcp_json_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create MCP config directory: {}", e))?;
    }
    std::fs::write(&path, &pretty).map_err(|e| format!("Failed to save MCP JSON config: {}", e))?;
    Ok(McpJsonConfigFile {
        path: path.to_string_lossy().to_string(),
        content: pretty,
    })
}

#[command]
pub async fn import_mcp_servers_from_default_json(
    mcp_service: State<'_, Arc<McpService>>,
    auto_connect: bool,
) -> Result<McpJsonImportResult, String> {
    let path = default_mcp_json_path();
    if !path.exists() {
        return Err("Default MCP JSON config does not exist yet".to_string());
    }
    mcp_service
        .import_servers_from_json(&path.to_string_lossy(), auto_connect)
        .await
}
