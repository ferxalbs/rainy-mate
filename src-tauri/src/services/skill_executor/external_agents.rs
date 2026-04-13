use super::args::*;
use super::SkillExecutor;
use crate::models::neural::CommandResult;
use crate::services::{ExternalRuntimeKind, NewExternalAgentSession};
use serde_json::Value;
use std::path::Path;

impl SkillExecutor {
    pub(super) async fn execute_external_agents(
        &self,
        workspace_id: String,
        method: &str,
        params: &Option<Value>,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let params = match params {
            Some(params) => params,
            None => return self.error("Missing parameters"),
        };

        match method {
            "spawn_external_agent_session" => {
                self.handle_spawn_external_agent_session(
                    workspace_id,
                    params,
                    allowed_paths,
                    blocked_paths,
                )
                .await
            }
            "send_external_agent_message" => self.handle_send_external_agent_message(params).await,
            "wait_external_agent_session" => self.handle_wait_external_agent_session(params).await,
            "list_external_agent_sessions" => {
                self.handle_list_external_agent_sessions(
                    workspace_id,
                    params,
                    allowed_paths,
                    blocked_paths,
                )
                .await
            }
            "cancel_external_agent_session" => {
                self.handle_cancel_external_agent_session(params).await
            }
            _ => CommandResult {
                success: false,
                output: None,
                error: Some(format!("Unknown external agent method: {}", method)),
                exit_code: Some(1),
            },
        }
    }

    async fn handle_spawn_external_agent_session(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: SpawnExternalAgentSessionArgs = match serde_json::from_value(params.clone()) {
            Ok(args) => args,
            Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
        };
        let runtime_kind = match ExternalRuntimeKind::from_str(&args.runtime) {
            Some(kind) => kind,
            None => return self.error(&format!("Unknown external runtime '{}'", args.runtime)),
        };
        let workspace_path = match args.workspace_path.as_deref() {
            Some(path) => match self
                .resolve_path(workspace_id, path, allowed_paths, blocked_paths)
                .await
            {
                Ok(path) => path,
                Err(error) => return self.error(&error),
            },
            None => match self
                .resolve_path(workspace_id, ".", allowed_paths, blocked_paths)
                .await
            {
                Ok(path) => path,
                Err(error) => return self.error(&error),
            },
        };
        let workspace_root = if workspace_path.is_dir() {
            workspace_path
        } else {
            match workspace_path.parent() {
                Some(parent) => parent.to_path_buf(),
                None => return self.error("Cannot determine workspace root for external session"),
            }
        };

        let runtime = {
            let guard = self.external_agent_runtime.read().await;
            guard.clone()
        };
        let runtime = match runtime {
            Some(runtime) => runtime,
            None => return self.error("External agent runtime not initialized"),
        };

        match runtime
            .create_session(NewExternalAgentSession {
                runtime_kind,
                workspace_path: workspace_root.to_string_lossy().to_string(),
                task_summary: args.task_summary,
            })
            .await
        {
            Ok(session) => CommandResult {
                success: true,
                output: Some(serde_json::to_string(&session).unwrap_or_default()),
                error: None,
                exit_code: Some(0),
            },
            Err(error) => self.error(&error),
        }
    }

    async fn handle_send_external_agent_message(&self, params: &Value) -> CommandResult {
        let args: SendExternalAgentMessageArgs = match serde_json::from_value(params.clone()) {
            Ok(args) => args,
            Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
        };
        let runtime = {
            let guard = self.external_agent_runtime.read().await;
            guard.clone()
        };
        let runtime = match runtime {
            Some(runtime) => runtime,
            None => return self.error("External agent runtime not initialized"),
        };
        match runtime.send_message(&args.session_id, args.message).await {
            Ok(session) => CommandResult {
                success: true,
                output: Some(serde_json::to_string(&session).unwrap_or_default()),
                error: None,
                exit_code: Some(0),
            },
            Err(error) => self.error(&error),
        }
    }

    async fn handle_wait_external_agent_session(&self, params: &Value) -> CommandResult {
        let args: WaitExternalAgentSessionArgs = match serde_json::from_value(params.clone()) {
            Ok(args) => args,
            Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
        };
        let runtime = {
            let guard = self.external_agent_runtime.read().await;
            guard.clone()
        };
        let runtime = match runtime {
            Some(runtime) => runtime,
            None => return self.error("External agent runtime not initialized"),
        };
        match runtime
            .wait_for_session(&args.session_id, args.timeout_ms)
            .await
        {
            Ok(session) => CommandResult {
                success: true,
                output: Some(serde_json::to_string(&session).unwrap_or_default()),
                error: None,
                exit_code: Some(0),
            },
            Err(error) => self.error(&error),
        }
    }

    async fn handle_list_external_agent_sessions(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: ListExternalAgentSessionsArgs =
            serde_json::from_value(params.clone()).unwrap_or(ListExternalAgentSessionsArgs {
                workspace_path: None,
            });
        let workspace_filter = match args.workspace_path.as_deref() {
            Some(path) => match self
                .resolve_path(workspace_id, path, allowed_paths, blocked_paths)
                .await
            {
                Ok(path) => Some(path),
                Err(error) => return self.error(&error),
            },
            None => None,
        };

        let runtime = {
            let guard = self.external_agent_runtime.read().await;
            guard.clone()
        };
        let runtime = match runtime {
            Some(runtime) => runtime,
            None => return self.error("External agent runtime not initialized"),
        };

        let workspace_filter = workspace_filter.as_ref().map(|path| {
            if path.is_dir() {
                path.to_string_lossy().to_string()
            } else {
                Path::new(path)
                    .parent()
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string()
            }
        });

        match runtime.list_sessions(workspace_filter.as_deref()).await {
            Ok(sessions) => CommandResult {
                success: true,
                output: Some(serde_json::to_string(&sessions).unwrap_or_default()),
                error: None,
                exit_code: Some(0),
            },
            Err(error) => self.error(&error),
        }
    }

    async fn handle_cancel_external_agent_session(&self, params: &Value) -> CommandResult {
        let args: CancelExternalAgentSessionArgs = match serde_json::from_value(params.clone()) {
            Ok(args) => args,
            Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
        };
        let runtime = {
            let guard = self.external_agent_runtime.read().await;
            guard.clone()
        };
        let runtime = match runtime {
            Some(runtime) => runtime,
            None => return self.error("External agent runtime not initialized"),
        };
        match runtime.cancel_session(&args.session_id).await {
            Ok(session) => CommandResult {
                success: true,
                output: Some(serde_json::to_string(&session).unwrap_or_default()),
                error: None,
                exit_code: Some(0),
            },
            Err(error) => self.error(&error),
        }
    }
}
