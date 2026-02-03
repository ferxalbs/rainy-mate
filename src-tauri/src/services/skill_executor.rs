use crate::models::neural::{CommandResult, QueuedCommand};
use crate::services::workspace::WorkspaceManager;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

#[derive(Clone)]
pub struct SkillExecutor {
    workspace_manager: Arc<WorkspaceManager>,
}

impl SkillExecutor {
    pub fn new(workspace_manager: Arc<WorkspaceManager>) -> Self {
        Self { workspace_manager }
    }

    pub async fn execute(&self, command: &QueuedCommand) -> CommandResult {
        // Parse payload
        let payload = &command.payload;
        let skill = payload.skill.as_deref().unwrap_or("unknown");
        let method = payload.method.as_deref().unwrap_or("unknown");
        let workspace_id_str = &command.workspace_id;

        let workspace_id = match Uuid::parse_str(workspace_id_str) {
            Ok(uuid) => uuid,
            Err(_) => return self.error("Invalid workspace ID format"),
        };

        match skill {
            "filesystem" => {
                self.execute_filesystem(workspace_id, method, &payload.params)
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

    async fn execute_filesystem(
        &self,
        workspace_id: Uuid,
        method: &str,
        params: &Option<Value>,
    ) -> CommandResult {
        let params = match params {
            Some(p) => p,
            None => return self.error("Missing parameters"),
        };

        match method {
            "read_file" => self.handle_read_file(workspace_id, params).await,
            "list_files" => self.handle_list_files(workspace_id, params).await,
            "search_files" => self.handle_search_files(workspace_id, params).await,
            "write_file" => self.handle_write_file(workspace_id, params).await,
            _ => CommandResult {
                success: false,
                output: None,
                error: Some(format!("Unknown filesystem method: {}", method)),
                exit_code: Some(1),
            },
        }
    }

    async fn resolve_path(&self, workspace_id: Uuid, path_str: &str) -> Result<PathBuf, String> {
        let workspace = self
            .workspace_manager
            .load_workspace(&workspace_id)
            .map_err(|e| format!("Failed to load workspace: {}", e))?;

        // Assume first allowed path is the root request is relative to
        // If allowed_paths is empty, access is denied
        if workspace.allowed_paths.is_empty() {
            return Err("Workspace has no allowed paths".to_string());
        }

        let root_str = &workspace.allowed_paths[0];
        let root = PathBuf::from(root_str);

        // Prevent absolute paths escaping valid roots if user provided absolute path
        // But for "relative path" tool, we join with root.
        let target_path = root.join(path_str);

        // Use WorkspaceManager's generic validation which checks against all allowed paths
        // But first we must produce a path string to check
        let target_path_str = target_path.to_string_lossy().to_string();

        if let Err(e) = self
            .workspace_manager
            .validate_path(&workspace, &target_path_str)
        {
            return Err(e.to_string());
        }

        Ok(target_path)
    }

    async fn handle_read_file(&self, workspace_id: Uuid, params: &Value) -> CommandResult {
        let path_str = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return self.error("Missing path parameter"),
        };

        let path = match self.resolve_path(workspace_id, path_str).await {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        match fs::read_to_string(path).await {
            Ok(content) => CommandResult {
                success: true,
                output: Some(content), // Return raw content
                error: None,
                exit_code: Some(0),
            },
            Err(e) => self.error(&format!("Failed to read file: {}", e)),
        }
    }

    async fn handle_list_files(&self, workspace_id: Uuid, params: &Value) -> CommandResult {
        let path_str = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let path = match self.resolve_path(workspace_id, path_str).await {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        let mut entries = Vec::new();
        match fs::read_dir(path).await {
            Ok(mut dir) => {
                while let Ok(Some(entry)) = dir.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let ft = entry.file_type().await.unwrap();
                    let kind = if ft.is_dir() { "directory" } else { "file" };
                    entries.push(serde_json::json!({
                        "name": name,
                        "type": kind
                    }));
                }

                CommandResult {
                    success: true,
                    output: Some(serde_json::to_string(&entries).unwrap()),
                    error: None,
                    exit_code: Some(0),
                }
            }
            Err(e) => self.error(&format!("Failed to list files: {}", e)),
        }
    }

    async fn handle_search_files(&self, _workspace_id: Uuid, _params: &Value) -> CommandResult {
        // Placeholder
        self.error("search_files not implemented yet")
    }

    async fn handle_write_file(&self, workspace_id: Uuid, params: &Value) -> CommandResult {
        let path_str = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return self.error("Missing path parameter"),
        };
        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return self.error("Missing content parameter"),
        };

        let path = match self.resolve_path(workspace_id, path_str).await {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        // Ensure parent dir exists
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return self.error(&format!("Failed to create parent directories: {}", e));
            }
        }

        match fs::write(path, content).await {
            Ok(_) => CommandResult {
                success: true,
                output: Some("File written successfully".to_string()),
                error: None,
                exit_code: Some(0),
            },
            Err(e) => self.error(&format!("Failed to write file: {}", e)),
        }
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
mod tests {
    // Tests require setting up WorkspaceManager with real filesystem.
    // See integration tests directory for full test coverage.
    // @TODO: Refactor to use a WorkspaceProvider trait for better testability.
}
