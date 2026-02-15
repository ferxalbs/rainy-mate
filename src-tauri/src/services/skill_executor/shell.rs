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

                self.execute_command(&args.command, args.args, args.timeout_ms, &root_path)
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
            "git_show" => {
                let args: GitShowArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_git_show(workspace_id, args, allowed_paths, blocked_paths)
                    .await
            }
            "git_branch_list" => {
                let args: GitBranchListArgs = match serde_json::from_value(params.clone()) {
                    Ok(a) => a,
                    Err(e) => return self.error(&format!("Invalid parameters: {}", e)),
                };
                self.handle_git_branch_list(workspace_id, args, allowed_paths, blocked_paths)
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
        self.execute_command("git", git_args, None, &cwd).await
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
        self.execute_command("git", git_args, None, &cwd).await
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
        self.execute_command("git", git_args, None, &cwd).await
    }

    async fn handle_git_show(
        &self,
        workspace_id: String,
        args: GitShowArgs,
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

        let target = args.target.unwrap_or_else(|| "HEAD".to_string());
        let max_lines = args.max_lines.unwrap_or(300).clamp(20, 2000);
        let git_args = vec![
            "--no-pager".to_string(),
            "show".to_string(),
            "--stat".to_string(),
            "--patch".to_string(),
            "--format=fuller".to_string(),
            "-n".to_string(),
            "1".to_string(),
            target,
        ];
        let result = self.execute_command("git", git_args, None, &cwd).await;
        if !result.success {
            return result;
        }

        let output = result.output.unwrap_or_default();
        let mut line_count = 0u32;
        let mut clipped = String::new();
        for line in output.lines() {
            if line_count >= max_lines {
                clipped.push_str("\n[TRUNCATED: git_show exceeded max_lines]");
                break;
            }
            clipped.push_str(line);
            clipped.push('\n');
            line_count += 1;
        }

        CommandResult {
            success: true,
            output: Some(clipped.trim_end().to_string()),
            error: None,
            exit_code: Some(0),
        }
    }

    async fn handle_git_branch_list(
        &self,
        workspace_id: String,
        args: GitBranchListArgs,
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

        let include_remote = args.include_remote.unwrap_or(true);
        let mut git_args = vec![
            "branch".to_string(),
            "--list".to_string(),
            "--verbose".to_string(),
        ];
        if include_remote {
            git_args.push("--all".to_string());
        }

        self.execute_command("git", git_args, None, &cwd).await
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
        timeout_ms: Option<u64>,
        cwd: &PathBuf,
    ) -> CommandResult {
        if !Self::is_allowed_shell_command(command) {
            return self.error(&format!("Command '{}' is not allowed", command));
        }

        let timeout = timeout_ms.unwrap_or(120_000).clamp(500, 600_000);
        let command_future = tokio::process::Command::new(command)
            .args(&args)
            .current_dir(Path::new(cwd))
            .kill_on_drop(true)
            .output();
        let output = tokio::time::timeout(
            tokio::time::Duration::from_millis(timeout),
            command_future,
        )
        .await;

        match output {
            Ok(Ok(out)) => {
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
            Ok(Err(e)) => self.error(&format!("Failed to execute command: {}", e)),
            Err(_) => self.error(&format!(
                "Command timed out after {}ms: {} {}",
                timeout,
                command,
                args.join(" ")
            )),
        }
    }
}
