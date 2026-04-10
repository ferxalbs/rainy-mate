#[cfg(target_os = "macos")]
use crate::commands::airlock::AirlockServiceState;
#[cfg(target_os = "macos")]
use libloading::Library;
#[cfg(target_os = "macos")]
use std::ffi::{c_char, CStr, CString};
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};
#[cfg(target_os = "macos")]
use tauri::{AppHandle, Emitter, Manager};
#[cfg(target_os = "macos")]
use tokio::sync::mpsc;

#[cfg(target_os = "macos")]
static ACTION_TX: OnceLock<mpsc::UnboundedSender<(String, Option<String>)>> = OnceLock::new();
#[cfg(target_os = "macos")]
static BRIDGE_INITIALIZED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "macos")]
static NOTIFICATION_BRIDGE: OnceLock<Result<NotificationBridgeSymbols, String>> = OnceLock::new();

#[cfg(target_os = "macos")]
struct NotificationBridgeSymbols {
    _library: Library,
    initialize: unsafe extern "C" fn(extern "C" fn(*const c_char, *const c_char)),
    runtime_supported: unsafe extern "C" fn() -> i32,
    request_authorization: unsafe extern "C" fn() -> i32,
    authorization_status: unsafe extern "C" fn() -> i32,
    send: unsafe extern "C" fn(*const c_char, *const c_char, *const c_char, *const c_char) -> i32,
    activate_app: unsafe extern "C" fn(),
}

#[cfg(target_os = "macos")]
fn notification_bridge_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(macos_dir) = exe.parent() {
            paths.push(macos_dir.join("libRainyNativeNotifications.dylib"));
            if let Some(contents_dir) = macos_dir.parent() {
                paths.push(
                    contents_dir
                        .join("Frameworks")
                        .join("libRainyNativeNotifications.dylib"),
                );
            }
        }
    }
    paths
}

#[cfg(target_os = "macos")]
fn notification_bridge() -> Result<&'static NotificationBridgeSymbols, String> {
    let result = NOTIFICATION_BRIDGE.get_or_init(|| unsafe {
        let mut tried = Vec::new();
        for path in notification_bridge_candidates() {
            tried.push(path.display().to_string());
            if !path.exists() {
                continue;
            }

            let library = match Library::new(&path) {
                Ok(library) => library,
                Err(error) => {
                    return Err(format!(
                        "Failed to load libRainyNativeNotifications.dylib from {}: {}",
                        path.display(),
                        error
                    ));
                }
            };

            let initialize = *library
                .get::<unsafe extern "C" fn(extern "C" fn(*const c_char, *const c_char))>(
                    b"rainy_notification_bridge_initialize\0",
                )
                .map_err(|e| e.to_string())?;
            let runtime_supported = *library
                .get::<unsafe extern "C" fn() -> i32>(
                    b"rainy_notification_bridge_runtime_supported\0",
                )
                .map_err(|e| e.to_string())?;
            let request_authorization = *library
                .get::<unsafe extern "C" fn() -> i32>(
                    b"rainy_notification_bridge_request_authorization\0",
                )
                .map_err(|e| e.to_string())?;
            let authorization_status = *library
                .get::<unsafe extern "C" fn() -> i32>(
                    b"rainy_notification_bridge_authorization_status\0",
                )
                .map_err(|e| e.to_string())?;
            let send = *library
                .get::<unsafe extern "C" fn(
                    *const c_char,
                    *const c_char,
                    *const c_char,
                    *const c_char,
                ) -> i32>(b"rainy_notification_bridge_send\0")
                .map_err(|e| e.to_string())?;
            let activate_app = *library
                .get::<unsafe extern "C" fn()>(b"rainy_notification_bridge_activate_app\0")
                .map_err(|e| e.to_string())?;

            return Ok(NotificationBridgeSymbols {
                _library: library,
                initialize,
                runtime_supported,
                request_authorization,
                authorization_status,
                send,
                activate_app,
            });
        }

        Err(format!(
            "libRainyNativeNotifications.dylib not found. Tried: {}",
            tried.join(", ")
        ))
    });

    result.as_ref().map_err(|error| error.clone())
}

#[cfg(target_os = "macos")]
extern "C" fn notification_action_callback(action: *const c_char, command_id: *const c_char) {
    let Some(tx) = ACTION_TX.get() else {
        return;
    };

    let action = unsafe {
        if action.is_null() {
            None
        } else {
            CStr::from_ptr(action)
                .to_str()
                .ok()
                .map(|value| value.to_string())
        }
    };

    let command_id = unsafe {
        if command_id.is_null() {
            None
        } else {
            CStr::from_ptr(command_id)
                .to_str()
                .ok()
                .map(|value| value.to_string())
        }
    };

    if let Some(action) = action {
        let _ = tx.send((action, command_id));
    }
}

#[cfg(target_os = "macos")]
fn c_string(input: &str) -> Result<CString, String> {
    CString::new(input).map_err(|_| "Notification payload contains interior null byte".to_string())
}

#[cfg(target_os = "macos")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NotificationFocusPayload {
    kind: String,
    workspace_id: String,
    chat_id: String,
}

#[cfg(target_os = "macos")]
pub struct MacOSNativeNotificationBridge;

#[cfg(target_os = "macos")]
impl MacOSNativeNotificationBridge {
    pub fn is_runtime_supported() -> bool {
        match notification_bridge() {
            Ok(bridge) => unsafe { (bridge.runtime_supported)() == 1 },
            Err(error) => {
                tracing::warn!("{}", error);
                false
            }
        }
    }

    pub fn initialize(app: AppHandle, airlock_state: AirlockServiceState) {
        if !Self::is_runtime_supported() {
            tracing::warn!(
                "macOS native notification bridge disabled because the process is not running from a supported app bundle"
            );
            return;
        }

        if BRIDGE_INITIALIZED.load(Ordering::SeqCst) {
            tracing::debug!("macOS native notification bridge already initialized");
            return;
        }

        let (tx, mut rx) = mpsc::unbounded_channel::<(String, Option<String>)>();
        if ACTION_TX.set(tx).is_err() {
            tracing::warn!("macOS native notification bridge channel already registered");
            return;
        }

        let Ok(bridge) = notification_bridge() else {
            tracing::warn!("macOS native notification bridge dylib unavailable");
            return;
        };

        unsafe { (bridge.initialize)(notification_action_callback) };

        BRIDGE_INITIALIZED.store(true, Ordering::SeqCst);

        tracing::info!("macOS native notification bridge initialized");

        tauri::async_runtime::spawn(async move {
            while let Some((action, command_id)) = rx.recv().await {
                let _ = app.emit(
                    "airlock:notification_action",
                    serde_json::json!({
                        "action": action,
                        "commandId": command_id,
                    }),
                );

                if matches!(action.as_str(), "open" | "approve" | "reject") {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                    if let Some(payload) = parse_focus_payload(command_id.as_deref()) {
                        let _ = app.emit(
                            "agent:notification_clicked",
                            serde_json::json!({
                                "workspaceId": payload.workspace_id,
                                "chatId": payload.chat_id,
                            }),
                        );
                    } else {
                        let _ = app.emit("airlock:notification_clicked", command_id.clone());
                    }
                }

                if let Some(command_id) = command_id.as_deref() {
                    let guard = airlock_state.0.lock().await;
                    if let Some(airlock) = guard.as_ref() {
                        match action.as_str() {
                            "approve" => {
                                let _ = airlock.respond_to_approval(command_id, true).await;
                            }
                            "reject" => {
                                let _ = airlock.respond_to_approval(command_id, false).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
    }

    pub fn authorization_status() -> i32 {
        if !Self::is_runtime_supported() {
            return -2;
        }
        notification_bridge()
            .map(|bridge| unsafe { (bridge.authorization_status)() })
            .unwrap_or(-2)
    }

    pub fn request_authorization() -> bool {
        if !Self::is_runtime_supported() {
            return false;
        }
        notification_bridge()
            .map(|bridge| unsafe { (bridge.request_authorization)() == 1 })
            .unwrap_or(false)
    }

    pub fn send_airlock_notification(
        title: &str,
        body: &str,
        command_id: Option<&str>,
    ) -> Result<(), String> {
        Self::send(title, body, command_id, Some("RAINY_AIRLOCK_CATEGORY"))
    }

    pub fn send_test_notification(title: &str, body: &str) -> Result<(), String> {
        Self::send(title, body, None, None)
    }

    pub fn send_agent_notification(
        title: &str,
        body: &str,
        workspace_id: Option<&str>,
        chat_id: Option<&str>,
    ) -> Result<(), String> {
        let payload = match (workspace_id, chat_id) {
            (Some(workspace_id), Some(chat_id)) => Some(
                serde_json::json!({
                    "kind": "chat_focus",
                    "workspaceId": workspace_id,
                    "chatId": chat_id,
                })
                .to_string(),
            ),
            _ => None,
        };

        Self::send(title, body, payload.as_deref(), Some("RAINY_AGENT_CATEGORY"))
    }

    pub fn activate_app() {
        if !Self::is_runtime_supported() {
            return;
        }
        if let Ok(bridge) = notification_bridge() {
            unsafe { (bridge.activate_app)() };
        }
    }

    fn send(
        title: &str,
        body: &str,
        command_id: Option<&str>,
        category: Option<&str>,
    ) -> Result<(), String> {
        if !Self::is_runtime_supported() {
            return Err(
                "Native macOS notifications are unavailable because Rainy MaTE is not running from a bundled app"
                    .to_string(),
            );
        }

        let title = c_string(title)?;
        let body = c_string(body)?;
        let command = match command_id {
            Some(value) => Some(c_string(value)?),
            None => None,
        };
        let category = match category {
            Some(value) => Some(c_string(value)?),
            None => None,
        };

        let bridge = notification_bridge()?;
        let ok = unsafe {
            (bridge.send)(
                title.as_ptr(),
                body.as_ptr(),
                command
                    .as_ref()
                    .map_or(std::ptr::null(), |value| value.as_ptr()),
                category
                    .as_ref()
                    .map_or(std::ptr::null(), |value| value.as_ptr()),
            )
        };

        if ok == 1 {
            Ok(())
        } else {
            Err("Native notification bridge failed to queue notification".to_string())
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub struct MacOSNativeNotificationBridge;

#[cfg(not(target_os = "macos"))]
impl MacOSNativeNotificationBridge {
    pub fn is_runtime_supported() -> bool {
        false
    }

    pub fn initialize(
        _app: tauri::AppHandle,
        _airlock_state: crate::commands::airlock::AirlockServiceState,
    ) {
    }

    pub fn authorization_status() -> i32 {
        1
    }

    pub fn request_authorization() -> bool {
        true
    }

    pub fn send_airlock_notification(
        _title: &str,
        _body: &str,
        _command_id: Option<&str>,
    ) -> Result<(), String> {
        Ok(())
    }

    pub fn send_test_notification(_title: &str, _body: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn send_agent_notification(
        _title: &str,
        _body: &str,
        _workspace_id: Option<&str>,
        _chat_id: Option<&str>,
    ) -> Result<(), String> {
        Ok(())
    }

    pub fn activate_app() {}
}

#[cfg(target_os = "macos")]
fn parse_focus_payload(command_id: Option<&str>) -> Option<NotificationFocusPayload> {
    let raw = command_id?;
    let payload: NotificationFocusPayload = serde_json::from_str(raw).ok()?;
    if payload.kind == "chat_focus" {
        Some(payload)
    } else {
        None
    }
}
