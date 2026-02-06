use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: u64,
}

pub struct McpClient {
    process: Arc<Mutex<Option<Child>>>,
    request_id: Arc<Mutex<u64>>,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            request_id: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn start(&self) -> Result<(), String> {
        let process = Command::new("npx")
            .args(&["-y", "chrome-devtools-mcp@latest"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn MCP process: {}", e))?;

        let mut guard = self.process.lock().await;
        *guard = Some(process);

        println!("[McpClient] Started Chrome DevTools MCP");
        Ok(())
    }

    pub async fn navigate(&self, url: &str) -> Result<Value, String> {
        self.call_method("Page.navigate", serde_json::json!({ "url": url }))
            .await
    }

    pub async fn click(&self, selector: &str) -> Result<Value, String> {
        // Implementation depends on the specific MCP tool definition
        // Assuming generic CDP or tool wrapper
        self.call_method(
            "Runtime.evaluate",
            serde_json::json!({
                "expression": format!("document.querySelector('{}').click()", selector)
            }),
        )
        .await
    }

    async fn call_method(&self, method: &str, params: Value) -> Result<Value, String> {
        let mut guard = self.process.lock().await;

        if guard.is_none() {
            // Auto-start if not running
            drop(guard);
            self.start().await?;
            guard = self.process.lock().await;
        }

        let child = guard.as_mut().ok_or("MCP process not running")?;

        let stdin = child.stdin.as_mut().ok_or("Failed to access stdin")?;
        // We can't easily read stdout here without a background loop reading lines
        // For a full implementation, we need a separate reader task processing responses
        // This is a simplified version for the prototype concept

        let mut id_guard = self.request_id.lock().await;
        *id_guard += 1;
        let id = *id_guard;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id,
        };

        let json = serde_json::to_string(&request).unwrap();
        stdin
            .write_all(format!("{}\n", json).as_bytes())
            .await
            .map_err(|e| format!("Failed to write to MCP stdin: {}", e))?;

        // In a real implementation, we would need to wait for the response matching ID
        // For now, we return a placeholder or need to implement the full async reader loop

        Ok(serde_json::json!({ "status": "sent", "id": id }))
    }
}
