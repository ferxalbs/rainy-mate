mod models;

pub use models::{
    ExternalAgentAuditEvent, ExternalAgentAuditEventType, ExternalAgentSession,
    ExternalAgentSessionStatus, ExternalRuntimeAvailability, ExternalRuntimeKind,
    NewExternalAgentSession,
};

use crate::models::neural::AirlockLevel;
use crate::services::audit_emitter::{AuditEmitter, FleetAuditEvent};
use crate::services::chat_artifacts::{artifact_from_path, push_unique_artifact};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::{oneshot, Mutex, Notify, RwLock};
use uuid::Uuid;

const MAX_SESSION_OUTPUT_BYTES: usize = 128 * 1024;
const MAX_AUDIT_EVENTS: usize = 200;

#[async_trait]
trait ExternalAgentCommandFactory: Send + Sync {
    async fn build_command(
        &self,
        runtime_kind: ExternalRuntimeKind,
        workspace_path: &Path,
        prompt: &str,
    ) -> Result<Command, String>;
}

#[derive(Default)]
struct RealExternalAgentCommandFactory;

#[async_trait]
impl ExternalAgentCommandFactory for RealExternalAgentCommandFactory {
    async fn build_command(
        &self,
        runtime_kind: ExternalRuntimeKind,
        workspace_path: &Path,
        prompt: &str,
    ) -> Result<Command, String> {
        let workspace_str = workspace_path
            .to_str()
            .ok_or_else(|| "Workspace path must be valid UTF-8".to_string())?;
        let mut command = match runtime_kind {
            ExternalRuntimeKind::Codex => {
                let mut cmd = Command::new("codex");
                cmd.args([
                    "exec",
                    "--full-auto",
                    "--skip-git-repo-check",
                    "--cd",
                    workspace_str,
                    "--json",
                    prompt,
                ]);
                cmd
            }
            ExternalRuntimeKind::Claude => {
                let mut cmd = Command::new("claude");
                cmd.args([
                    "--print",
                    "--verbose",
                    "--output-format",
                    "stream-json",
                    "--include-partial-messages",
                    "--permission-mode",
                    "acceptEdits",
                    "--add-dir",
                    workspace_str,
                ]);
                cmd
            }
        };

        command.current_dir(workspace_path);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        Ok(command)
    }
}

struct ExternalAgentSessionHandle {
    session: RwLock<ExternalAgentSession>,
    cancel_tx: Mutex<Option<oneshot::Sender<()>>>,
    completion: Notify,
}

impl ExternalAgentSessionHandle {
    fn new(session: ExternalAgentSession) -> Self {
        Self {
            session: RwLock::new(session),
            cancel_tx: Mutex::new(None),
            completion: Notify::new(),
        }
    }

    async fn snapshot(&self) -> ExternalAgentSession {
        self.session.read().await.clone()
    }
}

#[derive(Clone)]
pub struct ExternalAgentRuntime {
    sessions: Arc<RwLock<HashMap<String, Arc<ExternalAgentSessionHandle>>>>,
    command_factory: Arc<dyn ExternalAgentCommandFactory>,
    audit_emitter: Arc<RwLock<Option<Arc<AuditEmitter>>>>,
    binary_locator: Arc<dyn Fn(&str) -> Option<String> + Send + Sync>,
}

impl ExternalAgentRuntime {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            command_factory: Arc::new(RealExternalAgentCommandFactory),
            audit_emitter: Arc::new(RwLock::new(None)),
            binary_locator: Arc::new(Self::find_binary_in_path),
        }
    }

    #[cfg(test)]
    fn new_with_factory(command_factory: Arc<dyn ExternalAgentCommandFactory>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            command_factory,
            audit_emitter: Arc::new(RwLock::new(None)),
            binary_locator: Arc::new(|binary_name| Some(format!("/mock/bin/{}", binary_name))),
        }
    }

    pub async fn set_audit_emitter(&self, audit_emitter: Arc<AuditEmitter>) {
        let mut guard = self.audit_emitter.write().await;
        *guard = Some(audit_emitter);
    }

    pub async fn create_session(
        &self,
        request: NewExternalAgentSession,
    ) -> Result<ExternalAgentSession, String> {
        let availability = self.runtime_availability(request.runtime_kind).await;
        if !availability.installed {
            return Err(availability.status_message);
        }
        let workspace_path = Self::normalize_workspace_path(&request.workspace_path)?;
        if !workspace_path.exists() {
            return Err(format!(
                "Workspace path '{}' does not exist",
                workspace_path.display()
            ));
        }
        if !workspace_path.is_dir() {
            return Err(format!(
                "Workspace path '{}' is not a directory",
                workspace_path.display()
            ));
        }

        let session = ExternalAgentSession {
            session_id: Uuid::new_v4().to_string(),
            runtime_kind: request.runtime_kind,
            workspace_path: workspace_path.to_string_lossy().to_string(),
            task_summary: request.task_summary.trim().to_string(),
            launch_command_preview: None,
            status: ExternalAgentSessionStatus::Pending,
            created_at: chrono::Utc::now().timestamp_millis(),
            created_at_iso: None,
            started_at: None,
            started_at_iso: None,
            finished_at: None,
            finished_at_iso: None,
            duration_ms: None,
            last_message: None,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            error: None,
            touched_paths: Vec::new(),
            artifacts: Vec::new(),
            audit_events: Vec::new(),
        };
        let mut session = session;
        Self::refresh_session_timestamps(&mut session);

        let handle = Arc::new(ExternalAgentSessionHandle::new(session.clone()));
        self.sessions
            .write()
            .await
            .insert(session.session_id.clone(), handle);
        self.record_event(
            session.session_id.as_str(),
            ExternalAgentAuditEventType::SessionCreated,
            format!("Created {:?} external worker session", session.runtime_kind),
        )
        .await;
        Ok(session)
    }

    pub async fn list_runtime_availability(&self) -> Vec<ExternalRuntimeAvailability> {
        vec![
            self.runtime_availability(ExternalRuntimeKind::Codex).await,
            self.runtime_availability(ExternalRuntimeKind::Claude).await,
        ]
    }

    pub async fn runtime_availability(
        &self,
        runtime_kind: ExternalRuntimeKind,
    ) -> ExternalRuntimeAvailability {
        let binary_name = runtime_kind.as_str().to_string();
        let binary_path = (self.binary_locator)(runtime_kind.as_str());
        let install_hint = Self::install_hint(runtime_kind).to_string();
        let installed = binary_path.is_some();
        let status_message = if let Some(path) = binary_path.as_ref() {
            format!(
                "{} CLI detected at '{}'.",
                display_runtime_label(runtime_kind),
                path
            )
        } else {
            format!(
                "{} CLI is not installed or not available on PATH. Install it first. {}",
                display_runtime_label(runtime_kind),
                install_hint
            )
        };

        ExternalRuntimeAvailability {
            runtime_kind,
            installed,
            binary_name,
            binary_path,
            install_hint,
            status_message,
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Result<ExternalAgentSession, String> {
        let handle = self
            .sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("Unknown external agent session '{}'", session_id))?;
        Ok(handle.snapshot().await)
    }

    pub async fn list_sessions(
        &self,
        workspace_path: Option<&str>,
    ) -> Result<Vec<ExternalAgentSession>, String> {
        let workspace_path = match workspace_path {
            Some(path) => Some(Self::normalize_workspace_path(path)?),
            None => None,
        };
        let handles: Vec<_> = self.sessions.read().await.values().cloned().collect();
        let mut sessions = Vec::with_capacity(handles.len());
        for handle in handles {
            let session = handle.snapshot().await;
            if let Some(ref workspace_path) = workspace_path {
                if Path::new(&session.workspace_path) != workspace_path {
                    continue;
                }
            }
            sessions.push(session);
        }
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(sessions)
    }

    pub async fn send_message(
        &self,
        session_id: &str,
        message: String,
    ) -> Result<ExternalAgentSession, String> {
        let handle = self.lookup_handle(session_id).await?;
        {
            let session = handle.session.read().await;
            if session.status != ExternalAgentSessionStatus::Pending {
                return Err(format!(
                    "Session '{}' is not pending and cannot accept a new message",
                    session_id
                ));
            }
        }

        let prompt = {
            let mut session = handle.session.write().await;
            let normalized_message = message.trim().to_string();
            if normalized_message.is_empty() {
                return Err("External agent message cannot be empty".to_string());
            }
            session.last_message = Some(normalized_message.clone());
            session.status = ExternalAgentSessionStatus::Running;
            session.started_at = Some(chrono::Utc::now().timestamp_millis());
            session.finished_at = None;
            session.exit_code = None;
            session.error = None;
            session.launch_command_preview = Some(Self::command_preview(
                session.runtime_kind,
                Path::new(&session.workspace_path),
            ));
            Self::refresh_session_timestamps(&mut session);
            Self::build_prompt(&session.task_summary, &normalized_message)
        };
        self.record_event(
            session_id,
            ExternalAgentAuditEventType::SessionStarted,
            "External worker execution started".to_string(),
        )
        .await;

        let snapshot = handle.snapshot().await;
        let runtime_kind = snapshot.runtime_kind;
        let mut command = self
            .command_factory
            .build_command(runtime_kind, Path::new(&snapshot.workspace_path), &prompt)
            .await?;
        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to start {:?} session: {}", runtime_kind, e))?;
        if matches!(runtime_kind, ExternalRuntimeKind::Claude) {
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(prompt.as_bytes())
                    .await
                    .map_err(|e| format!("Failed to send prompt to Claude session stdin: {}", e))?;
                stdin
                    .write_all(b"\n")
                    .await
                    .map_err(|e| format!("Failed to finalize Claude session stdin: {}", e))?;
                stdin
                    .shutdown()
                    .await
                    .map_err(|e| format!("Failed to close Claude session stdin: {}", e))?;
            }
        }
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        {
            let mut tx_guard = handle.cancel_tx.lock().await;
            *tx_guard = Some(cancel_tx);
        }

        let runtime = self.clone();
        let session_id_owned = session_id.to_string();
        let handle_for_task = handle.clone();
        tokio::spawn(async move {
            runtime
                .drive_child(
                    session_id_owned,
                    handle_for_task,
                    child,
                    stdout,
                    stderr,
                    cancel_rx,
                )
                .await;
        });

        Ok(handle.snapshot().await)
    }

    pub async fn cancel_session(&self, session_id: &str) -> Result<ExternalAgentSession, String> {
        let handle = self.lookup_handle(session_id).await?;
        let status = { handle.session.read().await.status };

        match status {
            ExternalAgentSessionStatus::Pending => {
                let mut session = handle.session.write().await;
                session.status = ExternalAgentSessionStatus::Cancelled;
                session.finished_at = Some(chrono::Utc::now().timestamp_millis());
                session.error = Some("Cancelled before execution".to_string());
                Self::refresh_session_timestamps(&mut session);
                drop(session);
                self.record_event(
                    session_id,
                    ExternalAgentAuditEventType::SessionCancelled,
                    "Cancelled before execution".to_string(),
                )
                .await;
                handle.completion.notify_waiters();
                Ok(handle.snapshot().await)
            }
            ExternalAgentSessionStatus::Running => {
                let sent = {
                    let mut cancel_guard = handle.cancel_tx.lock().await;
                    cancel_guard.take()
                };
                if let Some(cancel_tx) = sent {
                    let _ = cancel_tx.send(());
                }
                let _ = self.wait_for_session(session_id, Some(5_000)).await;
                self.get_session(session_id).await
            }
            _ => self.get_session(session_id).await,
        }
    }

    pub async fn wait_for_session(
        &self,
        session_id: &str,
        timeout_ms: Option<u64>,
    ) -> Result<ExternalAgentSession, String> {
        let handle = self.lookup_handle(session_id).await?;
        if Self::is_terminal_status(handle.session.read().await.status) {
            return Ok(handle.snapshot().await);
        }

        let notified = handle.completion.notified();
        if let Some(timeout_ms) = timeout_ms {
            tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), notified)
                .await
                .map_err(|_| format!("Timed out waiting for external session '{}'", session_id))?;
        } else {
            notified.await;
        }

        Ok(handle.snapshot().await)
    }

    async fn lookup_handle(
        &self,
        session_id: &str,
    ) -> Result<Arc<ExternalAgentSessionHandle>, String> {
        self.sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("Unknown external agent session '{}'", session_id))
    }

    async fn drive_child(
        &self,
        session_id: String,
        handle: Arc<ExternalAgentSessionHandle>,
        mut child: tokio::process::Child,
        stdout: Option<tokio::process::ChildStdout>,
        stderr: Option<tokio::process::ChildStderr>,
        mut cancel_rx: oneshot::Receiver<()>,
    ) {
        let stdout_task = stdout.map(|stream| {
            let handle = handle.clone();
            let runtime = self.clone();
            let stream_session_id = session_id.clone();
            tokio::spawn(async move {
                runtime
                    .capture_stream(stream_session_id, stream, handle, false)
                    .await;
            })
        });
        let stderr_task = stderr.map(|stream| {
            let handle = handle.clone();
            let runtime = self.clone();
            let stream_session_id = session_id.clone();
            tokio::spawn(async move {
                runtime
                    .capture_stream(stream_session_id, stream, handle, true)
                    .await;
            })
        });

        let cancellation_requested = tokio::select! {
            status = child.wait() => {
                match status {
                    Ok(status) => {
                        let mut session = handle.session.write().await;
                        session.exit_code = status.code();
                        session.finished_at = Some(chrono::Utc::now().timestamp_millis());
                        if status.success() {
                            session.status = ExternalAgentSessionStatus::Completed;
                        } else {
                            session.status = ExternalAgentSessionStatus::Failed;
                            session.error = Some(format!(
                                "External runtime exited with code {:?}",
                                status.code()
                            ));
                        }
                        Self::refresh_session_timestamps(&mut session);
                    }
                    Err(error) => {
                        let mut session = handle.session.write().await;
                        session.finished_at = Some(chrono::Utc::now().timestamp_millis());
                        session.status = ExternalAgentSessionStatus::Failed;
                        session.error = Some(format!("Failed to await child process: {}", error));
                        Self::refresh_session_timestamps(&mut session);
                    }
                }
                let snapshot = handle.snapshot().await;
                let event_type = if snapshot.status == ExternalAgentSessionStatus::Completed {
                    ExternalAgentAuditEventType::SessionCompleted
                } else {
                    ExternalAgentAuditEventType::SessionFailed
                };
                let message = if snapshot.status == ExternalAgentSessionStatus::Completed {
                    "External worker completed".to_string()
                } else {
                    snapshot
                        .error
                        .clone()
                        .unwrap_or_else(|| "External worker failed".to_string())
                };
                self.record_event(&session_id, event_type, message).await;
                false
            }
            _ = &mut cancel_rx => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                let mut session = handle.session.write().await;
                session.finished_at = Some(chrono::Utc::now().timestamp_millis());
                session.status = ExternalAgentSessionStatus::Cancelled;
                session.error = Some("Cancelled by MaTE operator".to_string());
                session.exit_code = None;
                Self::refresh_session_timestamps(&mut session);
                drop(session);
                self.record_event(
                    &session_id,
                    ExternalAgentAuditEventType::SessionCancelled,
                    "Cancelled by MaTE operator".to_string(),
                )
                .await;
                true
            }
        };

        if let Some(task) = stdout_task {
            let _ = task.await;
        }
        if let Some(task) = stderr_task {
            let _ = task.await;
        }

        {
            let mut cancel_guard = handle.cancel_tx.lock().await;
            *cancel_guard = None;
        }
        if !cancellation_requested {
            let mut session = handle.session.write().await;
            if session.finished_at.is_none() {
                session.finished_at = Some(chrono::Utc::now().timestamp_millis());
                Self::refresh_session_timestamps(&mut session);
            }
        }
        handle.completion.notify_waiters();
        tracing::info!(
            "External agent session {} reached terminal state",
            session_id
        );
    }

    async fn capture_stream<R>(
        &self,
        session_id: String,
        mut reader: R,
        handle: Arc<ExternalAgentSessionHandle>,
        is_stderr: bool,
    ) where
        R: AsyncRead + Unpin + Send + 'static,
    {
        let mut buffer = [0u8; 2048];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break,
                Ok(count) => {
                    let chunk = String::from_utf8_lossy(&buffer[..count]).to_string();
                    let mut session = handle.session.write().await;
                    let target = if is_stderr {
                        &mut session.stderr
                    } else {
                        &mut session.stdout
                    };
                    Self::append_output(target, &chunk);
                    let full_output = target.clone();
                    let workspace_path = session.workspace_path.clone();
                    let mut new_touched_paths = Vec::new();
                    let mut new_artifacts = Vec::new();

                    for path in Self::extract_paths_from_text(&full_output) {
                        let normalized_path = if Path::new(&path).exists() {
                            PathBuf::from(&path)
                                .canonicalize()
                                .unwrap_or_else(|_| PathBuf::from(&path))
                                .to_string_lossy()
                                .to_string()
                        } else {
                            path.clone()
                        };
                        if !Path::new(&normalized_path).starts_with(&workspace_path) {
                            continue;
                        }
                        if !session
                            .touched_paths
                            .iter()
                            .any(|existing| existing == &normalized_path)
                        {
                            session.touched_paths.push(normalized_path.clone());
                            new_touched_paths.push(normalized_path.clone());
                        }
                        if Path::new(&normalized_path).exists() {
                            if let Some(artifact) = artifact_from_path(
                                &normalized_path,
                                if is_stderr {
                                    "external_agent_stderr"
                                } else {
                                    "external_agent_stdout"
                                },
                            ) {
                                let before = session.artifacts.len();
                                push_unique_artifact(&mut session.artifacts, artifact.clone());
                                if session.artifacts.len() > before {
                                    new_artifacts.push(artifact);
                                }
                            }
                        }
                    }
                    drop(session);
                    let event_type = if is_stderr {
                        ExternalAgentAuditEventType::StderrChunk
                    } else {
                        ExternalAgentAuditEventType::StdoutChunk
                    };
                    self.record_event(&session_id, event_type, truncate_event_message(&chunk))
                        .await;
                    for path in new_touched_paths {
                        self.record_event(
                            &session_id,
                            ExternalAgentAuditEventType::FileTouched,
                            path,
                        )
                        .await;
                    }
                    for artifact in new_artifacts {
                        self.record_event(
                            &session_id,
                            ExternalAgentAuditEventType::ArtifactEmitted,
                            artifact.path,
                        )
                        .await;
                    }
                }
                Err(error) => {
                    let mut session = handle.session.write().await;
                    let target = if is_stderr {
                        &mut session.stderr
                    } else {
                        &mut session.stdout
                    };
                    Self::append_output(target, &format!("\n[stream read error: {}]", error));
                    break;
                }
            }
        }
    }

    fn append_output(target: &mut String, chunk: &str) {
        target.push_str(chunk);
        if target.len() > MAX_SESSION_OUTPUT_BYTES {
            let excess = target.len() - MAX_SESSION_OUTPUT_BYTES;
            target.drain(..excess);
        }
    }

    fn build_prompt(task_summary: &str, message: &str) -> String {
        format!(
            "You are an external coding worker operating under MaTE governance.\n\
             Stay inside the provided workspace. When you create, edit, move, or inspect files, mention the affected absolute paths in your output.\n\
             If you generate a final deliverable, report its absolute path explicitly.\n\nTask summary:\n{}\n\nOperator message:\n{}\n",
            task_summary.trim(),
            message.trim()
        )
    }

    fn command_preview(runtime_kind: ExternalRuntimeKind, workspace_path: &Path) -> String {
        let workspace = workspace_path.to_string_lossy();
        match runtime_kind {
            ExternalRuntimeKind::Codex => format!(
                "codex exec --full-auto --skip-git-repo-check --cd {} --json <prompt>",
                shell_escape_preview(&workspace)
            ),
            ExternalRuntimeKind::Claude => format!(
                "claude --print --verbose --output-format stream-json --include-partial-messages --permission-mode acceptEdits --add-dir {} <prompt>",
                shell_escape_preview(&workspace)
            ),
        }
    }

    fn normalize_workspace_path(path: &str) -> Result<PathBuf, String> {
        let input = PathBuf::from(path);
        if !input.is_absolute() {
            return Err(format!("Workspace path '{}' must be absolute", path));
        }
        Ok(input
            .canonicalize()
            .map_err(|e| format!("Cannot canonicalize workspace path '{}': {}", path, e))?)
    }

    fn refresh_session_timestamps(session: &mut ExternalAgentSession) {
        session.created_at_iso = Some(timestamp_to_iso(session.created_at));
        session.started_at_iso = session.started_at.map(timestamp_to_iso);
        session.finished_at_iso = session.finished_at.map(timestamp_to_iso);
        session.duration_ms = match (session.started_at, session.finished_at) {
            (Some(started_at), Some(finished_at)) => Some((finished_at - started_at).max(0)),
            _ => None,
        };
    }

    fn is_terminal_status(status: ExternalAgentSessionStatus) -> bool {
        matches!(
            status,
            ExternalAgentSessionStatus::Completed
                | ExternalAgentSessionStatus::Failed
                | ExternalAgentSessionStatus::Cancelled
        )
    }

    fn find_binary_in_path(binary_name: &str) -> Option<String> {
        let path_var = env::var_os("PATH")?;
        let candidate_names: Vec<String> = if cfg!(windows) {
            vec![
                binary_name.to_string(),
                format!("{}.exe", binary_name),
                format!("{}.cmd", binary_name),
                format!("{}.bat", binary_name),
            ]
        } else {
            vec![binary_name.to_string()]
        };

        env::split_paths(&path_var).find_map(|dir| {
            for candidate_name in &candidate_names {
                let candidate = dir.join(candidate_name);
                if candidate.is_file() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
            None
        })
    }

    fn install_hint(runtime_kind: ExternalRuntimeKind) -> &'static str {
        match runtime_kind {
            ExternalRuntimeKind::Codex => {
                "Example: install the OpenAI Codex CLI and verify `codex --help` works in your terminal."
            }
            ExternalRuntimeKind::Claude => {
                "Example: install Claude Code and verify `claude --help` works in your terminal."
            }
        }
    }

    async fn record_event(
        &self,
        session_id: &str,
        event_type: ExternalAgentAuditEventType,
        message: String,
    ) {
        let handle = match self.sessions.read().await.get(session_id).cloned() {
            Some(handle) => handle,
            None => return,
        };
        let timestamp = chrono::Utc::now().timestamp_millis();
        let snapshot = {
            let mut session = handle.session.write().await;
            session.audit_events.push(ExternalAgentAuditEvent {
                event_type: event_type.clone(),
                message: message.clone(),
                timestamp,
            });
            if session.audit_events.len() > MAX_AUDIT_EVENTS {
                let excess = session.audit_events.len() - MAX_AUDIT_EVENTS;
                session.audit_events.drain(..excess);
            }
            session.clone()
        };

        let audit_emitter = { self.audit_emitter.read().await.clone() };
        if let Some(audit_emitter) = audit_emitter {
            audit_emitter
                .enqueue(FleetAuditEvent {
                    action_type: format!("{:?}", event_type).to_ascii_lowercase(),
                    outcome: snapshot_status_label(snapshot.status).to_string(),
                    agent_id: Some(snapshot.session_id),
                    tool_name: Some(format!("{:?}", snapshot.runtime_kind).to_ascii_lowercase()),
                    airlock_level: Some(AirlockLevel::Dangerous as u8),
                    payload_json: Some(
                        serde_json::json!({
                            "message": message,
                            "workspacePath": snapshot.workspace_path,
                            "touchedPaths": snapshot.touched_paths,
                            "artifactCount": snapshot.artifacts.len(),
                        })
                        .to_string(),
                    ),
                })
                .await;
        }
    }

    fn extract_paths_from_text(text: &str) -> Vec<String> {
        let mut values = Vec::new();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            Self::collect_paths_from_json(&json, &mut values);
        }
        let path_pattern =
            Regex::new(r#"(/[^ \n\r\t"'<>()\[\]{}:,;]+)"#).expect("valid path regex");
        for captures in path_pattern.captures_iter(text) {
            if let Some(m) = captures.get(1) {
                values.push(m.as_str().trim_end_matches('.').to_string());
            }
        }
        values.sort();
        values.dedup();
        values
    }

    fn collect_paths_from_json(value: &serde_json::Value, output: &mut Vec<String>) {
        match value {
            serde_json::Value::String(text) => {
                if text.starts_with('/') {
                    output.push(text.to_string());
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    Self::collect_paths_from_json(value, output);
                }
            }
            serde_json::Value::Object(map) => {
                for (key, value) in map {
                    let normalized = key.to_ascii_lowercase();
                    if normalized.contains("path")
                        || normalized.contains("file")
                        || normalized.contains("directory")
                        || normalized.contains("artifact")
                        || value.is_array()
                        || value.is_object()
                    {
                        Self::collect_paths_from_json(value, output);
                    }
                }
            }
            _ => {}
        }
    }
}

fn truncate_event_message(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.chars().count() <= 240 {
        return trimmed.to_string();
    }
    format!("{}...", trimmed.chars().take(240).collect::<String>())
}

fn snapshot_status_label(status: ExternalAgentSessionStatus) -> &'static str {
    match status {
        ExternalAgentSessionStatus::Pending => "pending",
        ExternalAgentSessionStatus::Running => "running",
        ExternalAgentSessionStatus::Completed => "completed",
        ExternalAgentSessionStatus::Failed => "failed",
        ExternalAgentSessionStatus::Cancelled => "cancelled",
    }
}

fn display_runtime_label(runtime_kind: ExternalRuntimeKind) -> &'static str {
    match runtime_kind {
        ExternalRuntimeKind::Codex => "Codex",
        ExternalRuntimeKind::Claude => "Claude Code",
    }
}

fn timestamp_to_iso(timestamp_ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(timestamp_ms)
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| timestamp_ms.to_string())
}

fn shell_escape_preview(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockCommandFactory {
        launch_count: AtomicUsize,
    }

    #[async_trait]
    impl ExternalAgentCommandFactory for MockCommandFactory {
        async fn build_command(
            &self,
            runtime_kind: ExternalRuntimeKind,
            workspace_path: &Path,
            prompt: &str,
        ) -> Result<Command, String> {
            self.launch_count.fetch_add(1, Ordering::Relaxed);
            let mut cmd = Command::new("/bin/sh");
            let escaped_prompt = prompt.replace('\'', "'\"'\"'");
            let mode = match runtime_kind {
                ExternalRuntimeKind::Codex => "codex",
                ExternalRuntimeKind::Claude => "claude",
            };
            let embedded_path = Regex::new(r#"(/[^ \n\r\t"'<>()\[\]{}:,;]+)"#)
                .expect("valid regex")
                .captures(prompt)
                .and_then(|captures| captures.get(1))
                .map(|m| m.as_str().to_string());
            let script = if prompt.contains("sleep-then-cancel") {
                format!("printf '{}:{}\\n'; sleep 30", mode, escaped_prompt)
            } else if prompt.contains("force-fail") {
                format!("printf '{}:{}\\n' >&2; exit 7", mode, escaped_prompt)
            } else if let Some(path) = embedded_path {
                format!(
                    "printf '{{\"path\":\"{}\"}}\\n'; printf 'artifact:{}\\n' >&2",
                    path, path
                )
            } else {
                format!(
                    "printf '{}:{}\\n'; printf 'warn:{}\\n' >&2",
                    mode, escaped_prompt, mode
                )
            };
            cmd.arg("-lc").arg(script);
            cmd.current_dir(workspace_path);
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
            Ok(cmd)
        }
    }

    fn make_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn should_run_real_smoke() -> bool {
        matches!(
            std::env::var("RUN_REAL_EXTERNAL_AGENT_SMOKE")
                .ok()
                .as_deref(),
            Some("1") | Some("true") | Some("TRUE")
        )
    }

    #[tokio::test]
    async fn create_and_complete_external_session() {
        let runtime = ExternalAgentRuntime::new_with_factory(Arc::new(MockCommandFactory {
            launch_count: AtomicUsize::new(0),
        }));
        let dir = make_temp_dir();
        let session = runtime
            .create_session(NewExternalAgentSession {
                runtime_kind: ExternalRuntimeKind::Codex,
                workspace_path: dir.path().to_string_lossy().to_string(),
                task_summary: "Inspect the project".to_string(),
            })
            .await
            .expect("create session");
        assert_eq!(session.status, ExternalAgentSessionStatus::Pending);

        runtime
            .send_message(&session.session_id, "Summarize the workspace".to_string())
            .await
            .expect("start session");
        let completed = runtime
            .wait_for_session(&session.session_id, Some(5_000))
            .await
            .expect("wait session");

        assert_eq!(completed.status, ExternalAgentSessionStatus::Completed);
        assert!(completed.stdout.contains("Summarize the workspace"));
        assert!(completed.stderr.contains("warn:codex"));
        let expected_preview = format!(
            "codex exec --full-auto --skip-git-repo-check --cd {} --json <prompt>",
            dir.path()
                .canonicalize()
                .expect("canonical workspace path")
                .to_string_lossy()
        );
        assert_eq!(
            completed.launch_command_preview.as_deref(),
            Some(expected_preview.as_str())
        );
        assert!(completed.audit_events.iter().any(|event| {
            matches!(
                event.event_type,
                ExternalAgentAuditEventType::SessionCompleted
            )
        }));
    }

    #[tokio::test]
    async fn cancel_running_external_session() {
        let runtime = ExternalAgentRuntime::new_with_factory(Arc::new(MockCommandFactory {
            launch_count: AtomicUsize::new(0),
        }));
        let dir = make_temp_dir();
        let session = runtime
            .create_session(NewExternalAgentSession {
                runtime_kind: ExternalRuntimeKind::Claude,
                workspace_path: dir.path().to_string_lossy().to_string(),
                task_summary: "Long run".to_string(),
            })
            .await
            .expect("create session");

        runtime
            .send_message(&session.session_id, "sleep-then-cancel".to_string())
            .await
            .expect("start session");
        let cancelled = runtime
            .cancel_session(&session.session_id)
            .await
            .expect("cancel session");

        assert_eq!(cancelled.status, ExternalAgentSessionStatus::Cancelled);
        assert!(cancelled.error.unwrap_or_default().contains("Cancelled"));
    }

    #[tokio::test]
    async fn rejects_relative_workspace_paths() {
        let runtime = ExternalAgentRuntime::new_with_factory(Arc::new(MockCommandFactory {
            launch_count: AtomicUsize::new(0),
        }));
        let result = runtime
            .create_session(NewExternalAgentSession {
                runtime_kind: ExternalRuntimeKind::Codex,
                workspace_path: "relative/path".to_string(),
                task_summary: "Bad".to_string(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn cancel_pending_session_without_launching_process() {
        let factory = Arc::new(MockCommandFactory {
            launch_count: AtomicUsize::new(0),
        });
        let runtime = ExternalAgentRuntime::new_with_factory(factory.clone());
        let dir = make_temp_dir();
        let session = runtime
            .create_session(NewExternalAgentSession {
                runtime_kind: ExternalRuntimeKind::Codex,
                workspace_path: dir.path().to_string_lossy().to_string(),
                task_summary: "Cancel before run".to_string(),
            })
            .await
            .expect("create session");

        let cancelled = runtime
            .cancel_session(&session.session_id)
            .await
            .expect("cancel pending session");

        assert_eq!(cancelled.status, ExternalAgentSessionStatus::Cancelled);
        assert_eq!(factory.launch_count.load(Ordering::Relaxed), 0);
        assert!(cancelled.audit_events.iter().any(|event| matches!(
            event.event_type,
            ExternalAgentAuditEventType::SessionCancelled
        )));
    }

    #[tokio::test]
    async fn wait_for_running_session_can_timeout() {
        let runtime = ExternalAgentRuntime::new_with_factory(Arc::new(MockCommandFactory {
            launch_count: AtomicUsize::new(0),
        }));
        let dir = make_temp_dir();
        let session = runtime
            .create_session(NewExternalAgentSession {
                runtime_kind: ExternalRuntimeKind::Claude,
                workspace_path: dir.path().to_string_lossy().to_string(),
                task_summary: "Timeout".to_string(),
            })
            .await
            .expect("create session");

        runtime
            .send_message(&session.session_id, "sleep-then-cancel".to_string())
            .await
            .expect("start session");
        let result = runtime
            .wait_for_session(&session.session_id, Some(10))
            .await;

        assert!(result.is_err());
        let snapshot = runtime
            .get_session(&session.session_id)
            .await
            .expect("get session");
        assert_eq!(snapshot.status, ExternalAgentSessionStatus::Running);
        let _ = runtime.cancel_session(&session.session_id).await;
    }

    #[tokio::test]
    async fn rejects_second_message_after_session_started() {
        let runtime = ExternalAgentRuntime::new_with_factory(Arc::new(MockCommandFactory {
            launch_count: AtomicUsize::new(0),
        }));
        let dir = make_temp_dir();
        let session = runtime
            .create_session(NewExternalAgentSession {
                runtime_kind: ExternalRuntimeKind::Codex,
                workspace_path: dir.path().to_string_lossy().to_string(),
                task_summary: "One shot".to_string(),
            })
            .await
            .expect("create session");

        runtime
            .send_message(&session.session_id, "first message".to_string())
            .await
            .expect("start session");
        let second = runtime
            .send_message(&session.session_id, "second message".to_string())
            .await;

        assert!(second.is_err());
    }

    #[tokio::test]
    async fn extracts_touched_paths_and_artifacts_from_worker_output() {
        let runtime = ExternalAgentRuntime::new_with_factory(Arc::new(MockCommandFactory {
            launch_count: AtomicUsize::new(0),
        }));
        let dir = make_temp_dir();
        let artifact_path = dir.path().join("brief.pdf");
        std::fs::write(&artifact_path, b"fake-pdf").expect("write artifact");
        let session = runtime
            .create_session(NewExternalAgentSession {
                runtime_kind: ExternalRuntimeKind::Codex,
                workspace_path: dir.path().to_string_lossy().to_string(),
                task_summary: "Artifact run".to_string(),
            })
            .await
            .expect("create session");

        runtime
            .send_message(
                &session.session_id,
                format!("mention {}", artifact_path.to_string_lossy()),
            )
            .await
            .expect("start session");
        let completed = runtime
            .wait_for_session(&session.session_id, Some(5_000))
            .await
            .expect("wait session");
        let canonical_artifact_path = artifact_path
            .canonicalize()
            .expect("canonical artifact path")
            .to_string_lossy()
            .to_string();

        assert!(completed
            .touched_paths
            .iter()
            .any(|path| path == &canonical_artifact_path));
        assert!(completed
            .artifacts
            .iter()
            .any(|artifact| artifact.path == canonical_artifact_path));
    }

    #[tokio::test]
    #[ignore = "dev-only smoke test against real Codex CLI"]
    async fn real_codex_cli_smoke() {
        if !should_run_real_smoke() {
            return;
        }

        let runtime = ExternalAgentRuntime::new();
        let dir = make_temp_dir();
        let session = runtime
            .create_session(NewExternalAgentSession {
                runtime_kind: ExternalRuntimeKind::Codex,
                workspace_path: dir.path().to_string_lossy().to_string(),
                task_summary: "Codex real smoke".to_string(),
            })
            .await
            .expect("create real codex session");

        runtime
            .send_message(
                &session.session_id,
                format!(
                    "Reply with exactly OK and mention {} in the output.",
                    dir.path().to_string_lossy()
                ),
            )
            .await
            .expect("start real codex session");
        let completed = runtime
            .wait_for_session(&session.session_id, Some(120_000))
            .await
            .expect("wait real codex session");

        assert_eq!(completed.status, ExternalAgentSessionStatus::Completed);
        assert!(completed.stdout.contains("OK"));
    }

    #[tokio::test]
    #[ignore = "dev-only smoke test against real Claude CLI"]
    async fn real_claude_cli_smoke() {
        if !should_run_real_smoke() {
            return;
        }

        let runtime = ExternalAgentRuntime::new();
        let dir = make_temp_dir();
        let session = runtime
            .create_session(NewExternalAgentSession {
                runtime_kind: ExternalRuntimeKind::Claude,
                workspace_path: dir.path().to_string_lossy().to_string(),
                task_summary: "Claude real smoke".to_string(),
            })
            .await
            .expect("create real claude session");

        runtime
            .send_message(
                &session.session_id,
                format!(
                    "Reply with exactly OK and mention {} in the output.",
                    dir.path().to_string_lossy()
                ),
            )
            .await
            .expect("start real claude session");
        let completed = runtime
            .wait_for_session(&session.session_id, Some(120_000))
            .await
            .expect("wait real claude session");

        assert_eq!(completed.status, ExternalAgentSessionStatus::Completed);
        assert!(completed.stdout.contains("OK"));
    }
}
