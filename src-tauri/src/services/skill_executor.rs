use crate::models::neural::{CommandResult, QueuedCommand};
use crate::services::workspace::WorkspaceManager;
use crate::services::{ManagedResearchService, WebResearchService};
use base64::prelude::*;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ReadFileArgs {
    /// The path to the file to read
    pub path: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct WriteFileArgs {
    /// The path where the file should be written
    pub path: String,
    /// The content to write to the file
    pub content: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ListFilesArgs {
    /// The directory path to list
    pub path: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct SearchFilesArgs {
    /// The regex query to search for
    pub query: String,
    /// The root path to start searching from
    pub path: Option<String>,
    /// Whether to search file content (true) or just filenames (false)
    pub search_content: Option<bool>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ExecuteCommandArgs {
    /// The command to execute (e.g., npm, cargo, git)
    pub command: String,
    /// Arguments for the command
    pub args: Vec<String>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct WebSearchArgs {
    /// The query to search for
    pub query: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ReadWebPageArgs {
    /// The URL to read
    pub url: String,
}

#[derive(Clone)]
pub struct SkillExecutor {
    workspace_manager: Arc<WorkspaceManager>,
    managed_research: Arc<ManagedResearchService>,
    web_research: Arc<WebResearchService>,
}

impl SkillExecutor {
    pub fn new(
        workspace_manager: Arc<WorkspaceManager>,
        managed_research: Arc<ManagedResearchService>,
        web_research: Arc<WebResearchService>,
    ) -> Self {
        Self {
            workspace_manager,
            managed_research,
            web_research,
        }
    }

    /// Get all available tools and their JSON schemas
    pub fn get_tool_definitions(&self) -> Vec<crate::ai::provider_types::Tool> {
        vec![
            crate::ai::provider_types::Tool {
                r#type: "function".to_string(),
                function: crate::ai::provider_types::FunctionDefinition {
                    name: "read_file".to_string(),
                    description: "Read the contents of a file".to_string(),
                    parameters: serde_json::to_value(schema_for!(ReadFileArgs)).unwrap(),
                },
            },
            crate::ai::provider_types::Tool {
                r#type: "function".to_string(),
                function: crate::ai::provider_types::FunctionDefinition {
                    name: "write_file".to_string(),
                    description: "Write content to a file".to_string(),
                    parameters: serde_json::to_value(schema_for!(WriteFileArgs)).unwrap(),
                },
            },
            crate::ai::provider_types::Tool {
                r#type: "function".to_string(),
                function: crate::ai::provider_types::FunctionDefinition {
                    name: "list_files".to_string(),
                    description: "List files in a directory".to_string(),
                    parameters: serde_json::to_value(schema_for!(ListFilesArgs)).unwrap(),
                },
            },
            crate::ai::provider_types::Tool {
                r#type: "function".to_string(),
                function: crate::ai::provider_types::FunctionDefinition {
                    name: "search_files".to_string(),
                    description: "Search for files by name or content using regex".to_string(),
                    parameters: serde_json::to_value(schema_for!(SearchFilesArgs)).unwrap(),
                },
            },
            crate::ai::provider_types::Tool {
                r#type: "function".to_string(),
                function: crate::ai::provider_types::FunctionDefinition {
                    name: "execute_command".to_string(),
                    description: "Execute a shell command (npm, cargo, git, ls, grep)".to_string(),
                    parameters: serde_json::to_value(schema_for!(ExecuteCommandArgs)).unwrap(),
                },
            },
            crate::ai::provider_types::Tool {
                r#type: "function".to_string(),
                function: crate::ai::provider_types::FunctionDefinition {
                    name: "web_search".to_string(),
                    description: "Search the web for information".to_string(),
                    parameters: serde_json::to_value(schema_for!(WebSearchArgs)).unwrap(),
                },
            },
            crate::ai::provider_types::Tool {
                r#type: "function".to_string(),
                function: crate::ai::provider_types::FunctionDefinition {
                    name: "read_web_page".to_string(),
                    description: "Read the content of a web page".to_string(),
                    parameters: serde_json::to_value(schema_for!(ReadWebPageArgs)).unwrap(),
                },
            },
        ]
    }

    pub async fn execute(&self, command: &QueuedCommand) -> CommandResult {
        // Parse payload
        let payload = &command.payload;
        let skill = payload.skill.as_deref().unwrap_or("unknown");
        let method = payload.method.as_deref().unwrap_or("unknown");

        let workspace_id = match &command.workspace_id {
            Some(id) => id.clone(),
            None => return self.error("Missing workspace ID in command"),
        };

        // Get allowed_paths from payload (Cloud sends these from workspace config)
        let allowed_paths = &payload.allowed_paths;

        match skill {
            "filesystem" => {
                self.execute_filesystem(workspace_id, method, &payload.params, allowed_paths)
                    .await
            }
            "shell" => {
                self.execute_shell(workspace_id, method, &payload.params, allowed_paths)
                    .await
            }
            "web" => {
                self.execute_web(workspace_id, method, &payload.params)
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
        workspace_id: String,
        method: &str,
        params: &Option<Value>,
        allowed_paths: &[String],
    ) -> CommandResult {
        let params = match params {
            Some(p) => p,
            None => return self.error("Missing parameters"),
        };

        match method {
            "read_file" => {
                self.handle_read_file(workspace_id, params, allowed_paths)
                    .await
            }
            "list_files" => {
                self.handle_list_files(workspace_id, params, allowed_paths)
                    .await
            }
            "search_files" => {
                self.handle_search_files(workspace_id, params, allowed_paths)
                    .await
            }
            "write_file" => {
                self.handle_write_file(workspace_id, params, allowed_paths)
                    .await
            }
            "append_file" => {
                self.handle_append_file(workspace_id, params, allowed_paths)
                    .await
            }
            _ => CommandResult {
                success: false,
                output: None,
                error: Some(format!("Unknown filesystem method: {}", method)),
                exit_code: Some(1),
            },
        }
    }

    async fn execute_shell(
        &self,
        workspace_id: String,
        method: &str,
        params: &Option<Value>,
        allowed_paths: &[String],
    ) -> CommandResult {
        let params = match params {
            Some(p) => p,
            None => return self.error("Missing parameters"),
        };

        match method {
            "execute_command" => {
                let args: ExecuteCommandArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };

                // For security, strict checking of args could be here, but we check command whitelist in impl

                // We need a CWD. Default to workspace root.
                // We reuse resolve_path to get the root.
                let root_path = match self.resolve_path(workspace_id, ".", allowed_paths).await {
                    Ok(p) => p,
                    Err(e) => return self.error(&e),
                };

                self.execute_command(&args.command, args.args, &root_path)
                    .await
            }
            _ => CommandResult {
                success: false,
                output: None,
                error: Some(format!("Unknown shell method: {}", method)),
                exit_code: Some(1),
            },
        }
    }

    async fn execute_web(
        &self,
        _workspace_id: String,
        method: &str,
        params: &Option<Value>,
    ) -> CommandResult {
        let params = match params {
            Some(p) => p,
            None => return self.error("Missing parameters"),
        };

        match method {
            "web_search" => {
                let args: WebSearchArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_web_search(&args.query).await
            }
            "read_web_page" => {
                let args: ReadWebPageArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_read_web_page(&args.url).await
            }
            _ => CommandResult {
                success: false,
                output: None,
                error: Some(format!("Unknown web method: {}", method)),
                exit_code: Some(1),
            },
        }
    }

    async fn handle_web_search(&self, query: &str) -> CommandResult {
        // Use ManagedResearchService which assumes "Phase 3" rainy-sdk integration
        // Note: perform_research uses API key from settings
        match self
            .managed_research
            .perform_research(query.to_string(), None)
            .await
        {
            Ok(result) => CommandResult {
                success: true,
                output: Some(format!(
                    "Research Result for '{}':\n{}",
                    query, result.content
                )),
                error: None,
                exit_code: Some(0),
            },
            Err(e) => self.error(&format!("Web search failed: {}", e)),
        }
    }

    async fn handle_read_web_page(&self, url: &str) -> CommandResult {
        match self.web_research.fetch_url(url).await {
            Ok(content) => CommandResult {
                success: true,
                output: Some(content.content_markdown), // Return markdown content
                error: None,
                exit_code: Some(0),
            },
            Err(e) => self.error(&format!("Failed to read web page: {:?}", e.to_string())),
        }
    }

    /// Execute a shell command
    async fn execute_command(
        &self,
        command: &str,
        args: Vec<String>,
        cwd: &PathBuf,
    ) -> CommandResult {
        // Whitelist safe commands
        let allowed_commands = vec!["npm", "pnpm", "cargo", "git", "ls", "grep", "echo", "cat"];
        if !allowed_commands.contains(&command) {
            return self.error(&format!("Command '{}' is not allowed", command));
        }

        let output = tokio::process::Command::new(command)
            .args(&args)
            .current_dir(cwd)
            .output()
            .await;

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let exit_code = out.status.code().unwrap_or(1);

                CommandResult {
                    success: out.status.success(),
                    output: Some(format!("{}\n{}", stdout, stderr).trim().to_string()),
                    error: if !out.status.success() {
                        Some(stderr)
                    } else {
                        None
                    },
                    exit_code: Some(exit_code),
                }
            }
            Err(e) => self.error(&format!("Failed to execute command: {}", e)),
        }
    }
    /// Resolve a path within the workspace. First tries to load local workspace,
    /// falls back to using allowed_paths from the command payload (Cloud-provided).
    async fn resolve_path(
        &self,
        workspace_id: String,
        path_str: &str,
        allowed_paths: &[String],
    ) -> Result<PathBuf, String> {
        let path_buf = PathBuf::from(path_str);

        // FAST PATH: If the path is absolute, validate it directly
        if path_buf.is_absolute() {
            // Try to load workspace to get allowed paths
            let workspace_allowed = match self.workspace_manager.load_workspace(&workspace_id) {
                Ok(ws) => ws.allowed_paths,
                Err(_) => {
                    // Use Cloud-provided allowed_paths
                    if !allowed_paths.is_empty() {
                        allowed_paths.to_vec()
                    } else {
                        // No restrictions, allow absolute path as-is (bootstrap mode)
                        return Ok(path_buf);
                    }
                }
            };

            // If we have allowed paths, validate the absolute path is within them
            if !workspace_allowed.is_empty() {
                let is_allowed = workspace_allowed
                    .iter()
                    .any(|allowed| path_str.starts_with(allowed));

                if !is_allowed {
                    return Err(format!(
                        "Path '{}' is outside allowed workspace paths",
                        path_str
                    ));
                }
            }

            return Ok(path_buf);
        }

        // RELATIVE PATH: Resolve against workspace root
        // Try to load workspace locally first
        let workspace_allowed_paths = match self.workspace_manager.load_workspace(&workspace_id) {
            Ok(ws) => ws.allowed_paths,
            Err(_) => {
                // Fallback to allowed_paths from command payload (Cloud-provided)
                if allowed_paths.is_empty() {
                    return Err(format!(
                        "No workspace context found. Please provide an absolute path (e.g. /Users/name/Projects) to start.",
                    ));
                }
                allowed_paths.to_vec()
            }
        };

        if workspace_allowed_paths.is_empty() {
            return Err("Workspace has no allowed paths".to_string());
        }

        let root_str = &workspace_allowed_paths[0];
        let root = PathBuf::from(root_str);

        // Build target path
        let target_path = root.join(path_str);

        // Validate path is within allowed paths
        let target_path_str = target_path.to_string_lossy().to_string();
        let is_allowed = workspace_allowed_paths
            .iter()
            .any(|allowed| target_path_str.starts_with(allowed));

        if !is_allowed {
            return Err(format!(
                "Path '{}' is outside allowed workspace paths",
                path_str
            ));
        }

        Ok(target_path)
    }

    async fn handle_read_file(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
    ) -> CommandResult {
        let args: ReadFileArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        // Determine mime type from extension
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let mime_type = match extension.as_str() {
            "png" => Some("image/png"),
            "jpg" | "jpeg" => Some("image/jpeg"),
            "webp" => Some("image/webp"),
            "gif" => Some("image/gif"),
            "pdf" => Some("application/pdf"),
            _ => None,
        };

        if let Some(mime) = mime_type {
            match fs::read(&path).await {
                Ok(bytes) => {
                    let b64_content = BASE64_STANDARD.encode(&bytes);
                    // Format as Data URI for easy consumption by agent/frontend
                    let output = format!("data:{};base64,{}", mime, b64_content);
                    CommandResult {
                        success: true,
                        output: Some(output),
                        error: None,
                        exit_code: Some(0),
                    }
                }
                Err(e) => self.error(&format!("Failed to read binary file: {}", e)),
            }
        } else {
            // Default to text read
            match fs::read_to_string(&path).await {
                Ok(content) => CommandResult {
                    success: true,
                    output: Some(content),
                    error: None,
                    exit_code: Some(0),
                },
                Err(e) => self.error(&format!("Failed to read file: {}", e)),
            }
        }
    }

    async fn handle_list_files(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
    ) -> CommandResult {
        // Handle optional path manually or via struct default?
        // Let's rely on struct default if possible, or manual parse
        // Params comes as Value, likely an object
        let path_str = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let path = match self
            .resolve_path(workspace_id, path_str, allowed_paths)
            .await
        {
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

    async fn handle_search_files(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
    ) -> CommandResult {
        let args: SearchFilesArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path_str = args.path.as_deref().unwrap_or(".");
        let search_content = args.search_content.unwrap_or(false);

        let root_path = match self
            .resolve_path(workspace_id, path_str, allowed_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        let regex = match regex::Regex::new(&args.query) {
            Ok(r) => r,
            Err(e) => return self.error(&format!("Invalid regex query: {}", e)),
        };

        let mut results = Vec::new();
        let mut queue = vec![root_path];

        // Breadth-first search with limit to avoid infinite loops or massive resource usage
        let max_files = 1000;
        let mut scanned_files = 0;

        while let Some(current_dir) = queue.pop() {
            let mut entries = match fs::read_dir(&current_dir).await {
                Ok(read_dir) => read_dir,
                Err(_) => continue, // Skip unreadable dirs
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();

                scanned_files += 1;
                if scanned_files > max_files {
                    break;
                }

                if path.is_dir() {
                    // Skip hidden directories like .git, node_modules
                    if !file_name.starts_with('.')
                        && file_name != "node_modules"
                        && file_name != "target"
                    {
                        queue.push(path);
                    }
                } else {
                    let mut matches = false;

                    // 1. Match filename
                    if regex.is_match(&file_name) {
                        matches = true;
                    }
                    // 2. Match content if requested
                    else if search_content {
                        // Only search text files - simple heuristic
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy();
                            if [
                                "md", "txt", "rs", "ts", "tsx", "js", "json", "toml", "yaml",
                                "yml", "css", "html",
                            ]
                            .contains(&ext_str.as_ref())
                            {
                                if let Ok(content) = fs::read_to_string(&path).await {
                                    if regex.is_match(&content) {
                                        matches = true;
                                    }
                                }
                            }
                        }
                    }

                    if matches {
                        results.push(serde_json::json!({
                            "name": file_name,
                            "path": path.to_string_lossy(),
                            "type": "file"
                        }));
                    }
                }
            }
            if scanned_files > max_files {
                break;
            }
        }

        CommandResult {
            success: true,
            output: Some(serde_json::to_string(&results).unwrap()),
            error: None,
            exit_code: Some(0),
        }
    }

    async fn handle_write_file(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
    ) -> CommandResult {
        let args: WriteFileArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        // Ensure parent dir exists
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return self.error(&format!("Failed to create parent directories: {}", e));
            }
        }

        match fs::write(path, &args.content).await {
            Ok(_) => CommandResult {
                success: true,
                output: Some("File written successfully".to_string()),
                error: None,
                exit_code: Some(0),
            },
            Err(e) => self.error(&format!("Failed to write file: {}", e)),
        }
    }

    async fn handle_append_file(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
    ) -> CommandResult {
        // Re-use WriteFileArgs since it has path + content
        let args: WriteFileArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        let file_res = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(path)
            .await;

        match file_res {
            Ok(mut file) => match file.write_all(args.content.as_bytes()).await {
                Ok(_) => CommandResult {
                    success: true,
                    output: Some("Content appended successfully".to_string()),
                    error: None,
                    exit_code: Some(0),
                },
                Err(e) => self.error(&format!("Failed to append content: {}", e)),
            },
            Err(e) => self.error(&format!("Failed to open file for appending: {}", e)),
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
    // Tests are fine to stay as is for now
}
