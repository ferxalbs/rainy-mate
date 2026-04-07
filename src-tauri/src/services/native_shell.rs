use crate::ai::agent::manager::{AgentManager, ChatSessionDto};
use crate::services::quick_delegate_modal::{focus_main_window, run_native_delegate_prompt};
use crate::services::{
    MacOSNativeShellBridge, QuickDelegateModalService,
    session_coordinator::SessionCoordinator,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

const MAX_RECENT_CHATS: usize = 5;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeShellStatus {
    pub available: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeShellRecentChat {
    pub chat_id: String,
    pub workspace_id: String,
    pub title: String,
    pub updated_at: String,
    pub message_count: i64,
    pub is_active: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeShellSnapshot {
    pub available: bool,
    pub workspace_name: Option<String>,
    pub workspace_path: Option<String>,
    pub pending_approval_count: usize,
    pub active_session_count: usize,
    pub quick_delegate_busy: bool,
    pub recent_chats: Vec<NativeShellRecentChat>,
}

#[derive(Clone)]
pub struct NativeShellService {
    app_handle: AppHandle,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeShellResumePayload {
    workspace_id: String,
    chat_id: String,
}

impl NativeShellService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn status(&self) -> NativeShellStatus {
        NativeShellStatus {
            available: MacOSNativeShellBridge::is_runtime_supported(),
        }
    }

    pub async fn refresh(
        &self,
        workspace_name: Option<String>,
        workspace_path: Option<String>,
        pending_approval_count: usize,
    ) -> Result<NativeShellSnapshot, String> {
        let quick_delegate = self.app_handle.state::<Arc<QuickDelegateModalService>>();
        let session_coordinator = self.app_handle.state::<Arc<SessionCoordinator>>();
        let agent_manager = self.app_handle.state::<AgentManager>();

        let active_sessions = session_coordinator.list_active();
        let active_chat_ids: std::collections::HashSet<String> = active_sessions
            .iter()
            .map(|session| session.chat_id.clone())
            .collect();

        let recent_chats = if let Some(workspace_id) = workspace_path.as_deref() {
            agent_manager
                .list_chat_sessions(workspace_id)
                .await
                .map_err(|e| e.to_string())?
                .into_iter()
                .take(MAX_RECENT_CHATS)
                .map(|chat| build_recent_chat(chat, &active_chat_ids))
                .collect()
        } else {
            Vec::new()
        };

        let snapshot = NativeShellSnapshot {
            available: MacOSNativeShellBridge::is_runtime_supported(),
            workspace_name,
            workspace_path,
            pending_approval_count,
            active_session_count: active_sessions.len(),
            quick_delegate_busy: quick_delegate.status().busy,
            recent_chats,
        };

        MacOSNativeShellBridge::update_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    pub async fn handle_bridge_action(&self, action: String, payload: Option<String>) {
        match action.as_str() {
            "hotkey" | "show_palette" => {
                let _ = MacOSNativeShellBridge::show_palette();
            }
            "open_main" => {
                let _ = focus_main_window(&self.app_handle);
            }
            "open_quick_delegate" => {
                let quick_delegate = self.app_handle.state::<Arc<QuickDelegateModalService>>();
                let _ = quick_delegate.open();
            }
            "review_approvals" => {
                let _ = focus_main_window(&self.app_handle);
                let _ = self.app_handle.emit("native-shell:review_approvals", ());
            }
            "resume_chat" => {
                if let Some(payload) = payload.as_deref() {
                    if let Ok(parsed) = serde_json::from_str::<NativeShellResumePayload>(payload) {
                        let _ = focus_main_window(&self.app_handle);
                        let _ = self.app_handle.emit(
                            "agent:notification_clicked",
                            serde_json::json!({
                                "workspaceId": parsed.workspace_id,
                                "chatId": parsed.chat_id,
                            }),
                        );
                    }
                }
            }
            "submit_prompt" => {
                let prompt = payload.unwrap_or_default();
                if prompt.trim().is_empty() {
                    return;
                }

                let quick_delegate = self.app_handle.state::<Arc<QuickDelegateModalService>>();
                if quick_delegate.begin_run().is_err() {
                    return;
                }

                let app_handle = self.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let result = run_native_delegate_prompt(app_handle.clone(), prompt).await;
                    if let Err(error) = result {
                        tracing::warn!("Native shell prompt failed: {}", error);
                    }
                    let quick_delegate = app_handle.state::<Arc<QuickDelegateModalService>>();
                    quick_delegate.finish_run();
                });
            }
            _ => {}
        }
    }
}

fn build_recent_chat(
    chat: ChatSessionDto,
    active_chat_ids: &std::collections::HashSet<String>,
) -> NativeShellRecentChat {
    NativeShellRecentChat {
        chat_id: chat.id.clone(),
        workspace_id: chat.workspace_id,
        title: chat
            .title
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "Untitled chat".to_string()),
        updated_at: chat.last_message_at.unwrap_or(chat.updated_at),
        message_count: chat.message_count,
        is_active: active_chat_ids.contains(&chat.id),
    }
}
