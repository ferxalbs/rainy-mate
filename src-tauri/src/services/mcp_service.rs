use crate::ai::provider_types::Tool;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransportConfig,
    pub timeout_secs: u64,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpTransportConfig {
    Stdio { command: String, args: Vec<String> },
    Sse { url: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
}

pub enum McpTransportHandle {
    Stdio { _child: tokio::sync::Mutex<Child> },
    Sse { _client: Client, _url: String },
}

pub struct McpConnection {
    pub config: McpServerConfig,
    pub _tools: Vec<Tool>,
    pub original_names: HashMap<String, String>,
    pub _handle: McpTransportHandle,
    next_id: std::sync::atomic::AtomicU64,
}

impl McpConnection {
    pub async fn connect(config: McpServerConfig) -> Result<Self, String> {
        let handle = match &config.transport {
            McpTransportConfig::Stdio { command, args } => {
                let mut cmd = tokio::process::Command::new(command);
                cmd.args(args);
                cmd.stdin(Stdio::piped());
                cmd.stdout(Stdio::piped());

                if let Some(env) = &config.env {
                    cmd.envs(env);
                }

                let child = cmd
                    .spawn()
                    .map_err(|e| format!("Failed to spawn MCP stdio transport: {}", e))?;

                McpTransportHandle::Stdio {
                    _child: Mutex::new(child),
                }
            }
            McpTransportConfig::Sse { url } => {
                let client = Client::new();
                McpTransportHandle::Sse {
                    _client: client,
                    _url: url.clone(),
                }
            }
        };

        let mut connection = Self {
            config,
            _tools: Vec::new(),
            original_names: HashMap::new(),
            _handle: handle,
            next_id: std::sync::atomic::AtomicU64::new(1),
        };

        connection.initialize().await?;
        connection.discover_tools().await?;

        Ok(connection)
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Implement initialize handshake
        // @TODO MCP protocol implementation is planned for next iteration
        let _req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self
                .next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05", // Example version
                "capabilities": {},
                "clientInfo": {
                    "name": "Rainy MaTE",
                    "version": "0.5.94"
                }
            })),
        };

        // TODO: Send request and await response.
        Ok(())
    }

    async fn discover_tools(&mut self) -> Result<(), String> {
        // Implement tools/list
        // Parse the ToolDefinitions out, namespace them mcp_{server}_{tool}, and store in `self.tools`
        // Store the original mapping in `self.original_names`
        Ok(())
    }

    pub async fn call_tool(&self, name: &str, input: serde_json::Value) -> Result<String, String> {
        let original_name = self
            .original_names
            .get(name)
            .ok_or_else(|| format!("Unknown MCP tool: {}", name))?;

        // @TODO Wire up request sending when completing MCP loop
        let _req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self
                .next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": original_name,
                "arguments": input
            })),
        };

        // TODO: Send request and await response.
        Ok("Tool execution not yet fully wired".to_string())
    }
}

pub struct McpService {
    connections: Arc<Mutex<Vec<McpConnection>>>,
    tool_cache: Arc<Mutex<Vec<Tool>>>,
}

impl McpService {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(Vec::new())),
            tool_cache: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn get_tools(&self) -> Vec<Tool> {
        self.tool_cache.lock().await.clone()
    }

    pub fn is_mcp_tool(name: &str) -> bool {
        name.starts_with("mcp_")
    }

    pub fn extract_mcp_server(name: &str) -> Option<String> {
        if !Self::is_mcp_tool(name) {
            return None;
        }
        let parts: Vec<&str> = name.splitn(3, '_').collect();
        if parts.len() >= 2 {
            Some(parts[1].to_string())
        } else {
            None
        }
    }

    pub async fn connect_server(&self, config: McpServerConfig) -> Result<(), String> {
        let conn = McpConnection::connect(config).await?;
        self.connections.lock().await.push(conn);
        Ok(())
    }

    pub async fn call_mcp_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        input: serde_json::Value,
    ) -> Result<String, String> {
        let conns = self.connections.lock().await;
        if let Some(conn) = conns.iter().find(|c| c.name() == server_name) {
            conn.call_tool(tool_name, input).await
        } else {
            Err(format!(
                "MCP Server '{}' not found or not connected",
                server_name
            ))
        }
    }
}

#[tauri::command]
pub async fn connect_mcp_server(
    state: tauri::State<'_, Arc<McpService>>,
    config: McpServerConfig,
) -> Result<(), String> {
    state.connect_server(config).await
}
