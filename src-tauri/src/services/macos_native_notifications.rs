#[cfg(target_os = "macos")]
use crate::commands::airlock::AirlockServiceState;
#[cfg(target_os = "macos")]
use std::ffi::{c_char, CStr, CString};
#[cfg(target_os = "macos")]
use std::sync::{atomic::{AtomicBool, Ordering}, OnceLock};
#[cfg(target_os = "macos")]
use tauri::{AppHandle, Emitter, Manager};
#[cfg(target_os = "macos")]
use tokio::sync::mpsc;

#[cfg(target_os = "macos")]
static ACTION_TX: OnceLock<mpsc::UnboundedSender<(String, Option<String>)>> = OnceLock::new();
#[cfg(target_os = "macos")]
static BRIDGE_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "macos")]
#[link(name = "RainyNativeNotifications", kind = "dylib")]
unsafe extern "C" {
    fn rainy_notification_bridge_initialize(
        callback: extern "C" fn(*const c_char, *const c_char),
    );
    fn rainy_notification_bridge_runtime_supported() -> i32;
    fn rainy_notification_bridge_request_authorization() -> i32;
    fn rainy_notification_bridge_authorization_status() -> i32;
    fn rainy_notification_bridge_send(
        title: *const c_char,
        body: *const c_char,
        command_id: *const c_char,
        category_id: *const c_char,
    ) -> i32;
    fn rainy_notification_bridge_activate_app();
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
            CStr::from_ptr(action).to_str().ok().map(|value| value.to_string())
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
pub struct MacOSNativeNotificationBridge;

#[cfg(target_os = "macos")]
impl MacOSNativeNotificationBridge {
    pub fn is_runtime_supported() -> bool {
        unsafe { rainy_notification_bridge_runtime_supported() == 1 }
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

        unsafe {
            rainy_notification_bridge_initialize(notification_action_callback);
        }

        BRIDGE_INITIALIZED.store(true, Ordering::SeqCst);

        tracing::info!("macOS native notification bridge initialized");

        tauri::async_runtime::spawn(async move {
            while let Some((action, command_id)) = rx.recv().await {
                let _ = app.emit("airlock:notification_action", serde_json::json!({
                    "action": action,
                    "commandId": command_id,
                }));

                if matches!(action.as_str(), "open" | "approve" | "reject") {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit("airlock:notification_clicked", command_id.clone());
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
        unsafe { rainy_notification_bridge_authorization_status() }
    }

    pub fn request_authorization() -> bool {
        if !Self::is_runtime_supported() {
            return false;
        }
        unsafe { rainy_notification_bridge_request_authorization() == 1 }
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

    pub fn activate_app() {
        if !Self::is_runtime_supported() {
            return;
        }
        unsafe {
            rainy_notification_bridge_activate_app();
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

        let ok = unsafe {
            rainy_notification_bridge_send(
                title.as_ptr(),
                body.as_ptr(),
                command.as_ref().map_or(std::ptr::null(), |value| value.as_ptr()),
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

    pub fn activate_app() {}
}
