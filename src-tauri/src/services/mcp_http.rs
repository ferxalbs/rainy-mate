use crate::services::mcp_service::{JsonRpcRequest, JsonRpcResponse, McpService};
use std::sync::Arc;

pub struct McpHttpProxy {
    mcp_service: Arc<McpService>,
}

impl McpHttpProxy {
    pub fn new(mcp_service: Arc<McpService>) -> Self {
        Self { mcp_service }
    }

    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "tools/call" => {
                // Parse tool parameters and dispatch to either SkillExecutor or McpService
                let params = request.params.unwrap_or(serde_json::json!({}));
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let _args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                if McpService::is_mcp_tool(name) {
                    // Route to external MCP connection
                    // Note: Here we'd actually look up the McpConnection and call it.
                    // For now, this is a skeleton.
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(serde_json::json!({
                            "content": [
                                { "type": "text", "text": format!("Dispatched {} via MCP proxy", name) }
                            ]
                        })),
                        error: None,
                    }
                } else {
                    // Option 2: Route to built-in SkillExecutor (or error if unsupported)
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(serde_json::json!({
                            "code": -32601,
                            "message": "Method not found or local tool execution via MCP Proxy is incomplete"
                        })),
                    }
                }
            }
            "tools/list" => {
                // Map local tools + external tools back out to the MCP format
                let mcp_tools = self.mcp_service.get_tools().await;

                let mut tools = Vec::new();
                for tool in mcp_tools {
                    tools.push(serde_json::json!({
                        "name": tool.function.name,
                        "description": tool.function.description,
                        "inputSchema": tool.function.parameters
                    }));
                }

                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::json!({ "tools": tools })),
                    error: None,
                }
            }
            "initialize" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "serverInfo": {
                        "name": "Rainy MaTE MCP Proxy",
                        "version": "0.5.94"
                    }
                })),
                error: None,
            },
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(serde_json::json!({
                    "code": -32601,
                    "message": "Method not found"
                })),
            },
        }
    }
}

#[tauri::command]
pub async fn handle_mcp_request(
    state: tauri::State<'_, Arc<McpHttpProxy>>,
    request: JsonRpcRequest,
) -> Result<JsonRpcResponse, String> {
    Ok(state.handle_request(request).await)
}
