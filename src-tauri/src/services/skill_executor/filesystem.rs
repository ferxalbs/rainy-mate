use super::args::*;
use super::SkillExecutor;
use crate::models::neural::CommandResult;
use base64::prelude::*;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

impl SkillExecutor {
    pub(super) fn normalize_absolute_path(path: &Path) -> Result<PathBuf, String> {
        if !path.is_absolute() {
            return Err(format!("Path '{}' must be absolute", path.display()));
        }

        let mut normalized = PathBuf::from("/");
        for component in path.components() {
            match component {
                Component::RootDir => {}
                Component::CurDir => {}
                Component::Normal(part) => normalized.push(part),
                Component::ParentDir => {
                    if !normalized.pop() {
                        return Err(format!("Invalid path '{}'", path.display()));
                    }
                }
                Component::Prefix(_) => {
                    return Err(format!("Unsupported path prefix for '{}'", path.display()));
                }
            }
        }

        Ok(normalized)
    }

    pub(super) fn is_path_blocked(
        normalized_target: &Path,
        blocked_paths: &[String],
        allowed_roots: &[String],
    ) -> bool {
        for blocked in blocked_paths {
            let blocked_path = Path::new(blocked);
            if blocked_path.is_absolute() {
                if let Ok(normalized_blocked) = Self::normalize_absolute_path(blocked_path) {
                    if normalized_target.starts_with(&normalized_blocked) {
                        return true;
                    }
                }
                continue;
            }

            for root in allowed_roots {
                if let Ok(normalized_root) = Self::normalize_absolute_path(Path::new(root)) {
                    let candidate = normalized_root.join(blocked_path);
                    if let Ok(normalized_candidate) = Self::normalize_absolute_path(&candidate) {
                        if normalized_target.starts_with(&normalized_candidate) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Resolve a path within the workspace. First tries to load local workspace,
    /// falls back to using allowed_paths from the command payload (Cloud-provided).
    pub(super) async fn resolve_path(
        &self,
        workspace_id: String,
        path_str: &str,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> Result<PathBuf, String> {
        let path_buf = PathBuf::from(path_str);

        if path_buf.is_absolute() {
            let normalized_target = Self::normalize_absolute_path(&path_buf)?;

            let workspace_allowed = match self.workspace_manager.load_workspace(&workspace_id) {
                Ok(ws) => ws.allowed_paths,
                Err(_) => {
                    if !allowed_paths.is_empty() {
                        allowed_paths.to_vec()
                    } else {
                        return Err(
                            "No allowed paths configured for this workspace. Configure allowed paths before filesystem operations."
                                .to_string(),
                        );
                    }
                }
            };

            if !workspace_allowed.is_empty() {
                let is_allowed = workspace_allowed.iter().any(|allowed| {
                    Self::normalize_absolute_path(Path::new(allowed))
                        .map(|p| normalized_target.starts_with(p))
                        .unwrap_or(false)
                });

                if !is_allowed {
                    return Err(format!(
                        "Path '{}' is outside allowed workspace paths",
                        path_str
                    ));
                }
            }

            if Self::is_path_blocked(&normalized_target, blocked_paths, &workspace_allowed) {
                return Err(format!("Path '{}' is blocked by Airlock scopes", path_str));
            }

            return Ok(normalized_target);
        }

        let workspace_allowed_paths = match self.workspace_manager.load_workspace(&workspace_id) {
            Ok(ws) => ws.allowed_paths,
            Err(_) => {
                if allowed_paths.is_empty() {
                    return Err(
                        "No workspace context found. Please provide an absolute path (e.g. /Users/name/Projects) to start."
                            .to_string(),
                    );
                }
                allowed_paths.to_vec()
            }
        };

        if workspace_allowed_paths.is_empty() {
            return Err("Workspace has no allowed paths".to_string());
        }

        let root_str = &workspace_allowed_paths[0];
        let root = PathBuf::from(root_str);
        let target_path = root.join(path_str);

        let normalized_target = Self::normalize_absolute_path(&target_path)?;
        let is_allowed = workspace_allowed_paths.iter().any(|allowed| {
            Self::normalize_absolute_path(Path::new(allowed))
                .map(|p| normalized_target.starts_with(p))
                .unwrap_or(false)
        });

        if !is_allowed {
            return Err(format!(
                "Path '{}' is outside allowed workspace paths",
                path_str
            ));
        }

        if Self::is_path_blocked(&normalized_target, blocked_paths, &workspace_allowed_paths) {
            return Err(format!("Path '{}' is blocked by Airlock scopes", path_str));
        }

        Ok(normalized_target)
    }

    pub(super) async fn execute_filesystem(
        &self,
        workspace_id: String,
        method: &str,
        params: &Option<Value>,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let params = match params {
            Some(p) => p,
            None => return self.error("Missing parameters"),
        };

        match method {
            "read_file" => {
                self.handle_read_file(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "read_many_files" => {
                self.handle_read_many_files(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "list_files" => {
                self.handle_list_files(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "list_files_detailed" => {
                self.handle_list_files_detailed(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "file_exists" => {
                self.handle_file_exists(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "get_file_info" => {
                self.handle_get_file_info(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "search_files" => {
                self.handle_search_files(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "read_file_chunk" => {
                self.handle_read_file_chunk(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "write_file" => {
                self.handle_write_file(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "append_file" => {
                self.handle_append_file(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "mkdir" => {
                self.handle_make_dir(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "delete_file" => {
                self.handle_delete_file(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "move_file" => {
                self.handle_move_file(workspace_id, params, allowed_paths, blocked_paths)
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

    async fn handle_read_file(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: ReadFileArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

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

    async fn handle_read_many_files(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: ReadManyFilesArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        if args.paths.is_empty() {
            return self.error("paths cannot be empty");
        }

        let max_files = 20usize;
        let max_file_bytes = 128 * 1024usize;
        let mut results = Vec::new();

        for path_str in args.paths.into_iter().take(max_files) {
            let resolved = match self
                .resolve_path(
                    workspace_id.clone(),
                    &path_str,
                    allowed_paths,
                    blocked_paths,
                )
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    results.push(serde_json::json!({
                        "path": path_str,
                        "ok": false,
                        "error": e,
                    }));
                    continue;
                }
            };

            match fs::read(&resolved).await {
                Ok(bytes) => {
                    let truncated = bytes.len() > max_file_bytes;
                    let clipped = if truncated {
                        &bytes[..max_file_bytes]
                    } else {
                        bytes.as_slice()
                    };
                    let content = String::from_utf8_lossy(clipped).to_string();
                    results.push(serde_json::json!({
                        "path": resolved.to_string_lossy(),
                        "ok": true,
                        "bytes": bytes.len(),
                        "truncated": truncated,
                        "content": content,
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "path": resolved.to_string_lossy(),
                        "ok": false,
                        "error": format!("Failed to read file: {}", e),
                    }));
                }
            }
        }

        CommandResult {
            success: true,
            output: Some(serde_json::json!({ "files": results }).to_string()),
            error: None,
            exit_code: Some(0),
        }
    }

    async fn handle_list_files(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let path_str = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let path = match self
            .resolve_path(workspace_id, path_str, allowed_paths, blocked_paths)
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

    async fn handle_list_files_detailed(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: ListFilesDetailedArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path_str = args.path.as_deref().unwrap_or(".");
        let include_hidden = args.include_hidden.unwrap_or(false);
        let limit = args.limit.unwrap_or(200).clamp(1, 2000);

        let path = match self
            .resolve_path(workspace_id, path_str, allowed_paths, blocked_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        let mut entries = Vec::new();
        match fs::read_dir(&path).await {
            Ok(mut dir) => {
                while let Ok(Some(entry)) = dir.next_entry().await {
                    if entries.len() >= limit {
                        break;
                    }

                    let name = entry.file_name().to_string_lossy().to_string();
                    if !include_hidden && name.starts_with('.') {
                        continue;
                    }

                    let metadata = match entry.metadata().await {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let modified = metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs());

                    entries.push(serde_json::json!({
                        "name": name,
                        "path": entry.path().to_string_lossy().to_string(),
                        "is_dir": metadata.is_dir(),
                        "is_file": metadata.is_file(),
                        "size_bytes": metadata.len(),
                        "readonly": metadata.permissions().readonly(),
                        "modified_unix": modified,
                    }));
                }

                let output = serde_json::json!({
                    "directory": path.to_string_lossy(),
                    "count": entries.len(),
                    "limit": limit,
                    "include_hidden": include_hidden,
                    "entries": entries,
                });
                CommandResult {
                    success: true,
                    output: Some(output.to_string()),
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
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: SearchFilesArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path_str = args.path.as_deref().unwrap_or(".");
        let search_content = args.search_content.unwrap_or(true);
        let case_sensitive = args.case_sensitive.unwrap_or(false);
        let max_files = args.max_files.unwrap_or(2000).clamp(100, 20_000);

        let root_path = match self
            .resolve_path(workspace_id, path_str, allowed_paths, blocked_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        let regex = match regex::RegexBuilder::new(&args.query)
            .case_insensitive(!case_sensitive)
            .build()
        {
            Ok(r) => r,
            Err(e) => return self.error(&format!("Invalid regex query: {}", e)),
        };

        let mut results = Vec::new();
        let mut queue = vec![root_path];
        let mut scanned_files = 0;

        while let Some(current_dir) = queue.pop() {
            let mut entries = match fs::read_dir(&current_dir).await {
                Ok(read_dir) => read_dir,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();

                scanned_files += 1;
                if scanned_files > max_files {
                    break;
                }

                if path.is_dir() {
                    if !file_name.starts_with('.')
                        && file_name != "node_modules"
                        && file_name != "target"
                    {
                        queue.push(path);
                    }
                } else {
                    let mut matches = false;

                    if regex.is_match(&file_name) {
                        matches = true;
                    } else if search_content {
                        let can_scan_by_extension = path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| {
                                matches!(
                                    ext,
                                    "txt"
                                        | "md"
                                        | "rs"
                                        | "ts"
                                        | "tsx"
                                        | "js"
                                        | "jsx"
                                        | "json"
                                        | "toml"
                                        | "yml"
                                        | "yaml"
                                        | "css"
                                        | "html"
                                        | "lock"
                                        | "cfg"
                                        | "conf"
                                        | "ini"
                                        | "env"
                                        | "sh"
                                        | "py"
                                        | "go"
                                        | "java"
                                        | "c"
                                        | "cpp"
                                        | "h"
                                        | "hpp"
                                        | "sql"
                                        | "graphql"
                                )
                            })
                            .unwrap_or_else(|| {
                                // Allow extensionless text-like files (Dockerfile, Makefile, etc.)
                                path.file_name()
                                    .and_then(|n| n.to_str())
                                    .map(|name| {
                                        matches!(
                                            name,
                                            "Dockerfile"
                                                | "Makefile"
                                                | "justfile"
                                                | "Procfile"
                                                | ".env"
                                                | ".env.local"
                                                | ".env.example"
                                        )
                                    })
                                    .unwrap_or(false)
                            });

                        if can_scan_by_extension {
                            if let Ok(content) = fs::read_to_string(&path).await {
                                if regex.is_match(&content) {
                                    matches = true;
                                }
                            }
                        }
                    }

                    if matches {
                        results.push(path.to_string_lossy().to_string());
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

    async fn handle_file_exists(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: FileExistsArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        match fs::metadata(&path).await {
            Ok(meta) => {
                let output = serde_json::json!({
                    "path": path.to_string_lossy(),
                    "exists": true,
                    "is_file": meta.is_file(),
                    "is_dir": meta.is_dir(),
                });
                CommandResult {
                    success: true,
                    output: Some(output.to_string()),
                    error: None,
                    exit_code: Some(0),
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let output = serde_json::json!({
                    "path": path.to_string_lossy(),
                    "exists": false,
                    "is_file": false,
                    "is_dir": false,
                });
                CommandResult {
                    success: true,
                    output: Some(output.to_string()),
                    error: None,
                    exit_code: Some(0),
                }
            }
            Err(e) => self.error(&format!("Failed to inspect path: {}", e)),
        }
    }

    async fn handle_get_file_info(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: FileInfoArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        let metadata = match fs::metadata(&path).await {
            Ok(m) => m,
            Err(e) => return self.error(&format!("Failed to read file metadata: {}", e)),
        };

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let created = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        let output = serde_json::json!({
            "path": path.to_string_lossy(),
            "exists": true,
            "is_file": metadata.is_file(),
            "is_dir": metadata.is_dir(),
            "size_bytes": metadata.len(),
            "readonly": metadata.permissions().readonly(),
            "modified_unix": modified,
            "created_unix": created,
        });

        CommandResult {
            success: true,
            output: Some(output.to_string()),
            error: None,
            exit_code: Some(0),
        }
    }

    async fn handle_read_file_chunk(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: ReadFileChunkArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        let offset = args.offset.unwrap_or(0);
        let requested_len = args.length.unwrap_or(8192).clamp(1, 65536);

        let metadata = match fs::metadata(&path).await {
            Ok(m) => m,
            Err(e) => return self.error(&format!("Failed to read file metadata: {}", e)),
        };
        if metadata.is_dir() {
            return self.error("Path is a directory; expected a text file");
        }

        let file_size = metadata.len();
        if offset > file_size {
            return self.error("Offset is beyond end of file");
        }

        let mut file = match fs::File::open(&path).await {
            Ok(f) => f,
            Err(e) => return self.error(&format!("Failed to open file: {}", e)),
        };

        if let Err(e) = file.seek(std::io::SeekFrom::Start(offset)).await {
            return self.error(&format!("Failed to seek file: {}", e));
        }

        let mut buffer = vec![0u8; requested_len];
        let read = match file.read(&mut buffer).await {
            Ok(n) => n,
            Err(e) => return self.error(&format!("Failed to read file chunk: {}", e)),
        };
        buffer.truncate(read);

        let text = String::from_utf8_lossy(&buffer).to_string();
        let next_offset = offset.saturating_add(read as u64);
        let eof = next_offset >= file_size;

        let output = serde_json::json!({
            "path": path.to_string_lossy(),
            "offset": offset,
            "requested_length": requested_len,
            "bytes_read": read,
            "next_offset": next_offset,
            "file_size": file_size,
            "eof": eof,
            "content": text,
        });

        CommandResult {
            success: true,
            output: Some(output.to_string()),
            error: None,
            exit_code: Some(0),
        }
    }

    async fn handle_write_file(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: WriteFileArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

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
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: WriteFileArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths, blocked_paths)
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

    async fn handle_make_dir(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: MakeDirArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        match fs::create_dir_all(&path).await {
            Ok(_) => CommandResult {
                success: true,
                output: Some(format!("Successfully created directory {}", args.path)),
                error: None,
                exit_code: Some(0),
            },
            Err(e) => self.error(&format!("Failed to create directory: {}", e)),
        }
    }

    async fn handle_delete_file(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: DeleteFileArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        if path.is_dir() {
            match fs::remove_dir_all(&path).await {
                Ok(_) => CommandResult {
                    success: true,
                    output: Some(format!("Successfully deleted directory {}", args.path)),
                    error: None,
                    exit_code: Some(0),
                },
                Err(e) => self.error(&format!("Failed to delete directory: {}", e)),
            }
        } else {
            match fs::remove_file(&path).await {
                Ok(_) => CommandResult {
                    success: true,
                    output: Some(format!("Successfully deleted file {}", args.path)),
                    error: None,
                    exit_code: Some(0),
                },
                Err(e) => self.error(&format!("Failed to delete file: {}", e)),
            }
        }
    }

    async fn handle_move_file(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: MoveFileArgs = match serde_json::from_value(params.clone()) {
            Ok(a) => a,
            Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
        };

        let source = match self
            .resolve_path(
                workspace_id.clone(),
                &args.source,
                allowed_paths,
                blocked_paths,
            )
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        let destination = match self
            .resolve_path(workspace_id, &args.destination, allowed_paths, blocked_paths)
            .await
        {
            Ok(p) => p,
            Err(e) => return self.error(&e),
        };

        if let Some(parent) = destination.parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return self.error(&format!(
                    "Failed to create parent directories for destination: {}",
                    e
                ));
            }
        }

        match fs::rename(&source, &destination).await {
            Ok(_) => CommandResult {
                success: true,
                output: Some(format!(
                    "Successfully moved {} to {}",
                    args.source, args.destination
                )),
                error: None,
                exit_code: Some(0),
            },
            Err(e) => self.error(&format!("Failed to move file: {}", e)),
        }
    }
}
