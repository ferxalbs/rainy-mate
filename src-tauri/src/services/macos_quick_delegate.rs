#[cfg(target_os = "macos")]
use crate::services::QuickDelegateModalService;
#[cfg(target_os = "macos")]
use std::ffi::{c_char, CStr, CString};
#[cfg(target_os = "macos")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};
#[cfg(target_os = "macos")]
use tauri::AppHandle;
#[cfg(target_os = "macos")]
use tokio::sync::mpsc;

#[cfg(target_os = "macos")]
static ACTION_TX: OnceLock<mpsc::UnboundedSender<(String, Option<String>)>> = OnceLock::new();
#[cfg(target_os = "macos")]
static BRIDGE_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "macos")]
#[link(name = "RainyQuickDelegate", kind = "dylib")]
unsafe extern "C" {
    fn rainy_quick_delegate_bridge_initialize(
        callback: extern "C" fn(*const c_char, *const c_char),
    );
    fn rainy_quick_delegate_bridge_runtime_supported() -> i32;
    fn rainy_quick_delegate_bridge_show(state: *const c_char, message: *const c_char) -> i32;
    fn rainy_quick_delegate_bridge_hide() -> i32;
    fn rainy_quick_delegate_bridge_set_state(state: *const c_char, message: *const c_char) -> i32;
}

#[cfg(target_os = "macos")]
extern "C" fn quick_delegate_callback(action: *const c_char, payload: *const c_char) {
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

    let payload = unsafe {
        if payload.is_null() {
            None
        } else {
            CStr::from_ptr(payload)
                .to_str()
                .ok()
                .map(|value| value.to_string())
        }
    };

    if let Some(action) = action {
        let _ = tx.send((action, payload));
    }
}

#[cfg(target_os = "macos")]
fn c_string(input: &str) -> Result<CString, String> {
    CString::new(input).map_err(|_| "Quick delegate payload contains interior null byte".to_string())
}

#[cfg(target_os = "macos")]
pub struct MacOSQuickDelegateBridge;

#[cfg(target_os = "macos")]
impl MacOSQuickDelegateBridge {
    #![allow(dead_code)]
    #![allow(dead_code)]
    pub fn is_runtime_supported() -> bool {
        unsafe { rainy_quick_delegate_bridge_runtime_supported() == 1 }
    }

    pub fn initialize(_app: AppHandle, quick_delegate: Arc<QuickDelegateModalService>) {
        if !Self::is_runtime_supported() {
            tracing::warn!("macOS quick delegate bridge disabled because AppKit runtime is unavailable");
            return;
        }

        if BRIDGE_INITIALIZED.load(Ordering::SeqCst) {
            tracing::debug!("macOS quick delegate bridge already initialized");
            return;
        }

        let (tx, mut rx) = mpsc::unbounded_channel::<(String, Option<String>)>();
        if ACTION_TX.set(tx).is_err() {
            tracing::warn!("macOS quick delegate bridge channel already registered");
            return;
        }

        unsafe {
            rainy_quick_delegate_bridge_initialize(quick_delegate_callback);
        }

        BRIDGE_INITIALIZED.store(true, Ordering::SeqCst);
        tracing::info!("macOS quick delegate bridge initialized");

        tauri::async_runtime::spawn(async move {
            while let Some((action, payload)) = rx.recv().await {
                quick_delegate.handle_bridge_action(action, payload).await;
            }
        });
    }

    pub fn show(
        state: Option<&str>,
        message: Option<&str>,
    ) -> Result<(), String> {
        if !Self::is_runtime_supported() {
            return Err("Native macOS quick delegate is unavailable in the current runtime".to_string());
        }

        let state = c_string(state.unwrap_or("idle"))?;
        let message = c_string(message.unwrap_or(""))?;
        let ok = unsafe { rainy_quick_delegate_bridge_show(state.as_ptr(), message.as_ptr()) };
        if ok == 1 {
            Ok(())
        } else {
            Err("Native quick delegate bridge failed to show the modal".to_string())
        }
    }

    pub fn hide() -> Result<(), String> {
        if !Self::is_runtime_supported() {
            return Err("Native macOS quick delegate is unavailable in the current runtime".to_string());
        }

        let ok = unsafe { rainy_quick_delegate_bridge_hide() };
        if ok == 1 {
            Ok(())
        } else {
            Err("Native quick delegate bridge failed to hide the modal".to_string())
        }
    }

    pub fn set_state(
        state: &str,
        message: Option<&str>,
    ) -> Result<(), String> {
        if !Self::is_runtime_supported() {
            return Err("Native macOS quick delegate is unavailable in the current runtime".to_string());
        }

        let state = c_string(state)?;
        let message = c_string(message.unwrap_or(""))?;
        let ok = unsafe { rainy_quick_delegate_bridge_set_state(state.as_ptr(), message.as_ptr()) };
        if ok == 1 {
            Ok(())
        } else {
            Err("Native quick delegate bridge failed to update modal state".to_string())
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub struct MacOSQuickDelegateBridge;

#[cfg(not(target_os = "macos"))]
impl MacOSQuickDelegateBridge {
    #![allow(dead_code)]
    #![allow(dead_code)]
    pub fn is_runtime_supported() -> bool {
        false
    }

    pub fn initialize(
        _app: tauri::AppHandle,
        _quick_delegate: std::sync::Arc<crate::services::QuickDelegateModalService>,
    ) {
    }

    pub fn show(_state: Option<&str>, _message: Option<&str>) -> Result<(), String> {
        Err("Quick delegate is only available on macOS".to_string())
    }

    pub fn hide() -> Result<(), String> {
        Err("Quick delegate is only available on macOS".to_string())
    }

    pub fn set_state(_state: &str, _message: Option<&str>) -> Result<(), String> {
        Err("Quick delegate is only available on macOS".to_string())
    }
}
