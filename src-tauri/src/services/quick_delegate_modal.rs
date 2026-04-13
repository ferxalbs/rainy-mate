use crate::commands::agent::{run_agent_workflow_internal, WorkflowInvocationSource};
use crate::commands::settings::DesktopNotificationRequest;
use crate::services::{
    FileManager, MacOSNativeNotificationBridge, MacOSQuickDelegateBridge, SettingsManager,
};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickDelegateStatus {
    pub available: bool,
    pub busy: bool,
}

pub struct QuickDelegateModalService {
    app_handle: AppHandle,
    busy: AtomicBool,
}

impl QuickDelegateModalService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            busy: AtomicBool::new(false),
        }
    }

    pub fn status(&self) -> QuickDelegateStatus {
        QuickDelegateStatus {
            available: MacOSQuickDelegateBridge::is_runtime_supported(),
            busy: self.busy.load(Ordering::SeqCst),
        }
    }

    pub fn open(&self) -> Result<(), String> {
        MacOSQuickDelegateBridge::show(Some("idle"), None)
    }

    pub fn focus_main_window(&self) -> Result<(), String> {
        focus_main_window(&self.app_handle)
    }

    pub fn begin_run(&self) -> Result<(), String> {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| "A quick delegation is already running.".to_string())
    }

    pub fn finish_run(&self) {
        self.busy.store(false, Ordering::SeqCst);
    }

    pub async fn handle_bridge_action(&self, action: String, payload: Option<String>) {
        match action.as_str() {
            "hotkey" => {
                let _ = self.open();
            }
            "open_main" => {
                let _ = self.focus_main_window();
            }
            "cancel" => {
                let _ = MacOSQuickDelegateBridge::hide();
            }
            "submit" => {
                let prompt = payload.unwrap_or_default();
                if prompt.trim().is_empty() {
                    let _ = MacOSQuickDelegateBridge::set_state(
                        "error",
                        Some("Type a delegation request first."),
                    );
                    return;
                }

                if let Err(error) = self.begin_run() {
                    let _ = MacOSQuickDelegateBridge::set_state("error", Some(&error));
                    return;
                }

                let app_handle = self.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = MacOSQuickDelegateBridge::set_state(
                        "running",
                        Some("Delegating to Rainy..."),
                    );
                    let _ = MacOSQuickDelegateBridge::hide();

                    let result =
                        run_native_delegate_prompt(app_handle.clone(), prompt.clone()).await;
                    if let Err(error) = result {
                        let _ = MacOSQuickDelegateBridge::show(
                            Some("error"),
                            Some(&truncate_message(&error, 140)),
                        );
                    }

                    let quick_delegate = app_handle.state::<Arc<QuickDelegateModalService>>();
                    quick_delegate.finish_run();
                });
            }
            _ => {}
        }
    }
}

pub fn focus_main_window(app_handle: &AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

pub async fn run_native_delegate_prompt(
    app_handle: AppHandle,
    prompt: String,
) -> Result<(), String> {
    let file_manager = app_handle.state::<Arc<FileManager>>();
    let workspace_id = file_manager
        .get_workspace()
        .await
        .map(|workspace| workspace.path)
        .unwrap_or_else(|| "default".to_string());

    let model_id = {
        let settings = app_handle.state::<Arc<Mutex<SettingsManager>>>();
        let settings = settings.lock().await;
        settings.get_selected_model().to_string()
    };

    let chat_id = uuid::Uuid::new_v4().to_string();
    let run_id = format!("native-modal-{}", uuid::Uuid::new_v4());

    let result = run_agent_workflow_internal(
        app_handle.clone(),
        prompt.clone(),
        model_id,
        workspace_id.clone(),
        None,
        Some(chat_id.clone()),
        Some(run_id.clone()),
        None,
        None,
        WorkflowInvocationSource::NativeModal,
    )
    .await;

    match result {
        Ok(response) => {
            emit_finish_notification(
                &app_handle,
                "Rainy MaTE",
                &format!(
                    "Quick delegation finished: {}",
                    build_summary(&response.response)
                ),
                "agent_finish",
                Some(workspace_id),
                Some(chat_id),
            )?;
            Ok(())
        }
        Err(error) => {
            emit_finish_notification(
                &app_handle,
                "Rainy MaTE",
                &format!("Quick delegation failed: {}", truncate_message(&error, 140)),
                "agent_failure",
                None,
                None,
            )?;
            Err(error)
        }
    }
}

fn emit_finish_notification(
    app_handle: &AppHandle,
    title: &str,
    body: &str,
    kind: &str,
    workspace_id: Option<String>,
    chat_id: Option<String>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        match MacOSNativeNotificationBridge::send_agent_notification(
            title,
            body,
            workspace_id.as_deref(),
            chat_id.as_deref(),
        ) {
            Ok(()) => return Ok(()),
            Err(error) => {
                tracing::warn!("Quick delegate native notification unavailable: {}", error);
            }
        }
    }

    app_handle
        .emit(
            "desktop:notification",
            DesktopNotificationRequest {
                title: title.to_string(),
                body: body.to_string(),
                kind: kind.to_string(),
                command_id: None,
                workspace_id,
                chat_id,
            },
        )
        .map_err(|e| format!("Failed to emit finish notification: {}", e))
}

fn truncate_message(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let mut out = String::new();
    for ch in trimmed.chars().take(max_chars.saturating_sub(1)) {
        out.push(ch);
    }
    out.push_str("...");
    out
}

fn build_summary(response: &str) -> String {
    let compact = response.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_message(&compact, 90)
}
