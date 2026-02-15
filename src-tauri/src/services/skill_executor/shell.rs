use super::args::*;
use super::{truncate_output, SkillExecutor};
use crate::models::neural::CommandResult;
use serde_json::Value;
use std::path::{Path, PathBuf};

impl SkillExecutor {
    pub(super) async fn execute_shell(
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
            "execute_command" => {
                let args: ExecuteCommandArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };

                let root_path = match self
                    .resolve_path(workspace_id, ".", allowed_paths, blocked_paths)
                    .await
                {
                    Ok(p) => p,
                    Err(e) => return self.error(&e),
                };

                self.execute_command(&args.command, args.args, &root_path)
                    .await
            }
            "git_status" => {
                let args: GitStatusArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_git_status(workspace_id, args, allowed_paths, blocked_paths)
                    .await
            }
            "git_diff" => {
                let args: GitDiffArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_git_diff(workspace_id, args, allowed_paths, blocked_paths)
                    .await
            }
            "git_log" => {
                let args: GitLogArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_git_log(workspace_id, args, allowed_paths, blocked_paths)
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

    async fn handle_git_status(
        &self,
        workspace_id: String,
        args: GitStatusArgs,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let cwd = match self
            .resolve_git_working_dir(workspace_id, args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(path) => path,
            Err(e) => return self.error(&e),
        };

        let mut git_args = vec!["status".to_string()];
        if args.short.unwrap_or(true) {
            git_args.push("--short".to_string());
        }
        self.execute_command("git", git_args, &cwd).await
    }

    async fn handle_git_diff(
        &self,
        workspace_id: String,
        args: GitDiffArgs,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let cwd = match self
            .resolve_git_working_dir(workspace_id, args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(path) => path,
            Err(e) => return self.error(&e),
        };

        let mut git_args = vec!["diff".to_string()];
        if args.staged.unwrap_or(false) {
            git_args.push("--staged".to_string());
        }
        self.execute_command("git", git_args, &cwd).await
    }

    async fn handle_git_log(
        &self,
        workspace_id: String,
        args: GitLogArgs,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let cwd = match self
            .resolve_git_working_dir(workspace_id, args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(path) => path,
            Err(e) => return self.error(&e),
        };

        let max_count = args.max_count.unwrap_or(20).clamp(1, 100);
        let git_args = vec![
            "--no-pager".to_string(),
            "log".to_string(),
            "--oneline".to_string(),
            format!("-{}", max_count),
        ];
        self.execute_command("git", git_args, &cwd).await
    }

    pub(super) async fn resolve_git_working_dir(
        &self,
        workspace_id: String,
        path: Option<String>,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> Result<PathBuf, String> {
        let target = path.unwrap_or_else(|| ".".to_string());
        let resolved = self
            .resolve_path(workspace_id, &target, allowed_paths, blocked_paths)
            .await?;
        if resolved.is_dir() {
            Ok(resolved)
        } else {
            resolved
                .parent()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| "Cannot determine parent directory for git path".to_string())
        }
    }

    /// Execute a shell command
    pub(super) async fn execute_command(
        &self,
        command: &str,
        args: Vec<String>,
        cwd: &PathBuf,
    ) -> CommandResult {
        if !Self::is_allowed_shell_command(command) {
            return self.error(&format!("Command '{}' is not allowed", command));
        }

        let output = tokio::process::Command::new(command)
            .args(&args)
            .current_dir(Path::new(cwd))
            .output()
            .await;

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let exit_code = out.status.code().unwrap_or(1);
                let combined = format!("{}\n{}", stdout, stderr).trim().to_string();

                CommandResult {
                    success: out.status.success(),
                    output: Some(truncate_output(&combined)),
                    error: if !out.status.success() {
                        Some(truncate_output(&stderr))
                    } else {
                        None
                    },
                    exit_code: Some(exit_code),
                }
            }
            Err(e) => self.error(&format!("Failed to execute command: {}", e)),
        }
    }
}
