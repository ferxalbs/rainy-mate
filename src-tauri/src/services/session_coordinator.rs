//! SessionCoordinator — unified session lifecycle for local and remote agent runs.
//!
//! Ensures both Telegram-originated and local chat sessions:
//! - are persisted in SQLite
//! - emit `agent://event` to the frontend
//! - appear in `list_active_sessions`

use crate::ai::agent::events::AgentEvent;
use crate::ai::agent::manager::AgentManager;
use crate::commands::agent_frontend_events::FrontendAgentEvent;
use chrono::Utc;
use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;

pub enum SessionSource {
    Local,
    NativeModal,
    Remote {
        connector_id: String,
        #[allow(dead_code)]
        session_peer: String,
    },
}

struct ActiveSession {
    source: SessionSource,
    started_at: Instant,
    run_id: String,
    workspace_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSessionInfo {
    pub chat_id: String,
    pub run_id: String,
    pub source: String,
    pub connector_id: Option<String>,
    pub elapsed_secs: u64,
}

pub struct SessionCoordinator {
    agent_manager: Arc<AgentManager>,
    app_handle: tauri::AppHandle,
    active_sessions: DashMap<String, ActiveSession>,
}

impl SessionCoordinator {
    pub fn new(agent_manager: Arc<AgentManager>, app_handle: tauri::AppHandle) -> Self {
        Self {
            agent_manager,
            app_handle,
            active_sessions: DashMap::new(),
        }
    }

    /// Create/reuse chat session, save user message, emit `session://started`.
    /// Returns `(chat_id, run_id)`.
    pub async fn start_remote_session(
        &self,
        chat_id: Option<String>,
        workspace_id: &str,
        prompt: &str,
        connector_id: &str,
        session_peer: &str,
        command_id: Option<String>,
    ) -> Result<(String, String), String> {
        let chat_id = chat_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let run_id = command_id
            .as_deref()
            .map(|id| format!("remote-{}", id))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        self.agent_manager
            .ensure_chat_session_with_source(
                &chat_id,
                workspace_id,
                "remote",
                Some(connector_id),
                Some(session_peer),
            )
            .await
            .map_err(|e| format!("Failed to ensure chat session: {}", e))?;

        self.agent_manager
            .save_message(&chat_id, "user", prompt)
            .await
            .map_err(|e| format!("Failed to save user message: {}", e))?;

        // Guard against duplicate starts (e.g. rapid retries from the same Telegram message)
        if self.active_sessions.contains_key(&chat_id) {
            tracing::warn!(
                "[SessionCoordinator] Session {} already active — ignoring duplicate start",
                chat_id
            );
        } else {
            self.active_sessions.insert(
                chat_id.clone(),
                ActiveSession {
                    source: SessionSource::Remote {
                        connector_id: connector_id.to_string(),
                        session_peer: session_peer.to_string(),
                    },
                    started_at: Instant::now(),
                    run_id: run_id.clone(),
                    workspace_id: workspace_id.to_string(),
                },
            );
        }

        let _ = self.app_handle.emit(
            "session://started",
            serde_json::json!({
                "chatId": chat_id,
                "runId": run_id,
                "workspaceId": workspace_id,
                "source": "remote",
                "connectorId": connector_id,
            }),
        );

        Ok((chat_id, run_id))
    }

    /// Create/reuse chat session for a native quick-delegate run, save user message,
    /// register it as active, and emit `session://started`.
    pub async fn start_native_modal_session(
        &self,
        chat_id: Option<String>,
        run_id: Option<String>,
        workspace_id: &str,
        prompt: &str,
    ) -> Result<(String, String), String> {
        let chat_id = chat_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let run_id = run_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        self.agent_manager
            .ensure_chat_session_with_source(&chat_id, workspace_id, "native_modal", None, None)
            .await
            .map_err(|e| format!("Failed to ensure native modal chat session: {}", e))?;

        self.agent_manager
            .save_message(&chat_id, "user", prompt)
            .await
            .map_err(|e| format!("Failed to save native modal user message: {}", e))?;

        if self.active_sessions.contains_key(&chat_id) {
            tracing::warn!(
                "[SessionCoordinator] Native modal session {} already active — ignoring duplicate start",
                chat_id
            );
        } else {
            self.active_sessions.insert(
                chat_id.clone(),
                ActiveSession {
                    source: SessionSource::NativeModal,
                    started_at: Instant::now(),
                    run_id: run_id.clone(),
                    workspace_id: workspace_id.to_string(),
                },
            );
        }

        let _ = self.app_handle.emit(
            "session://started",
            serde_json::json!({
                "chatId": chat_id,
                "runId": run_id,
                "workspaceId": workspace_id,
                "source": "native_modal",
                "connectorId": serde_json::Value::Null,
            }),
        );

        Ok((chat_id, run_id))
    }

    /// Emit `agent://event` to frontend. Safe to call from sync closures.
    pub fn emit_agent_event(&self, run_id: &str, event: AgentEvent) {
        let _ = self.app_handle.emit(
            "agent://event",
            FrontendAgentEvent {
                run_id: run_id.to_string(),
                timestamp_ms: Utc::now().timestamp_millis(),
                payload: event,
            },
        );
    }

    /// Save assistant response, generate fallback title, emit `session://finished`.
    pub async fn finish_remote_session(
        &self,
        chat_id: &str,
        response: &str,
        prompt: &str,
    ) -> Result<(), String> {
        self.agent_manager
            .save_message(chat_id, "assistant", response)
            .await
            .map_err(|e| format!("Failed to save assistant message: {}", e))?;

        // Set a fallback title if none exists
        if let Ok(Some(session)) = self.agent_manager.get_chat_session(chat_id).await {
            let needs_title = session
                .title
                .as_deref()
                .map(is_placeholder_title)
                .unwrap_or(true);
            if needs_title {
                let title = build_fallback_title(prompt);
                let _ = self
                    .agent_manager
                    .update_chat_title(chat_id, Some(&title))
                    .await;
            }
        }

        let (run_id, workspace_id) = self
            .active_sessions
            .remove(chat_id)
            .map(|(_, s)| (s.run_id, s.workspace_id))
            .unwrap_or_default();

        let _ = self.app_handle.emit(
            "session://finished",
            serde_json::json!({
                "chatId": chat_id,
                "runId": run_id,
                "workspaceId": workspace_id,
            }),
        );

        Ok(())
    }

    /// Save the assistant response for a native modal session, derive a fallback
    /// title when needed, and emit `session://finished`.
    pub async fn finish_native_modal_session(
        &self,
        chat_id: &str,
        response: &str,
        prompt: &str,
    ) -> Result<(), String> {
        self.finish_remote_session(chat_id, response, prompt).await
    }

    /// Register a local session so it appears in `list_active_sessions`.
    pub fn register_local(&self, chat_id: String, run_id: String, workspace_id: String) {
        self.active_sessions.insert(
            chat_id,
            ActiveSession {
                source: SessionSource::Local,
                started_at: Instant::now(),
                run_id,
                workspace_id,
            },
        );
    }

    /// Unregister a session (local or remote) and emit `session://finished` so the
    /// frontend clears its active-run indicator. Use this on error paths where
    /// `finish_remote_session` is not called.
    pub fn abort_session(&self, chat_id: &str) {
        let (run_id, workspace_id) = self
            .active_sessions
            .remove(chat_id)
            .map(|(_, s)| (s.run_id, s.workspace_id))
            .unwrap_or_default();
        let _ = self.app_handle.emit(
            "session://finished",
            serde_json::json!({
                "chatId": chat_id,
                "runId": run_id,
                "workspaceId": workspace_id,
            }),
        );
    }

    /// Unregister a session without emitting an event. Use only when the
    /// session was never exposed to the frontend (e.g., failed before start).
    pub fn unregister(&self, chat_id: &str) {
        self.active_sessions.remove(chat_id);
    }

    /// List all currently active sessions for the frontend.
    pub fn list_active(&self) -> Vec<ActiveSessionInfo> {
        self.active_sessions
            .iter()
            .map(|entry| {
                let (source, connector_id) = match &entry.value().source {
                    SessionSource::Local => ("local".to_string(), None),
                    SessionSource::NativeModal => ("native_modal".to_string(), None),
                    SessionSource::Remote { connector_id, .. } => {
                        ("remote".to_string(), Some(connector_id.clone()))
                    }
                };
                ActiveSessionInfo {
                    chat_id: entry.key().clone(),
                    run_id: entry.value().run_id.clone(),
                    source,
                    connector_id,
                    elapsed_secs: entry.value().started_at.elapsed().as_secs(),
                }
            })
            .collect()
    }
}

fn is_placeholder_title(value: &str) -> bool {
    let n = value.trim().to_lowercase();
    n.is_empty() || n == "new thread" || n == "new chat" || n.starts_with("workspace session:")
}

fn build_fallback_title(seed: &str) -> String {
    let compact = seed
        .split_whitespace()
        .filter(|p| !p.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = compact
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '`' | '.' | ':' | ';' | ',' | '-'))
        .trim();
    if trimmed.is_empty() {
        return "Remote session".to_string();
    }
    trimmed.chars().take(72).collect()
}
