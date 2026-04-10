#[cfg(target_os = "macos")]
use crate::services::{NativeShellService, NativeShellSnapshot};
#[cfg(target_os = "macos")]
use libloading::Library;
#[cfg(target_os = "macos")]
use std::ffi::{c_char, CStr, CString};
#[cfg(target_os = "macos")]
use std::path::PathBuf;
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
static NATIVE_SHELL_BRIDGE: OnceLock<Result<NativeShellBridgeSymbols, String>> = OnceLock::new();

#[cfg(target_os = "macos")]
struct NativeShellBridgeSymbols {
    _library: Library,
    initialize: unsafe extern "C" fn(extern "C" fn(*const c_char, *const c_char)),
    runtime_supported: unsafe extern "C" fn() -> i32,
    show_palette: unsafe extern "C" fn() -> i32,
    update_snapshot: unsafe extern "C" fn(*const c_char) -> i32,
}

#[cfg(target_os = "macos")]
fn native_shell_bridge_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(macos_dir) = exe.parent() {
            paths.push(macos_dir.join("libRainyNativeShell.dylib"));
            if let Some(contents_dir) = macos_dir.parent() {
                paths.push(
                    contents_dir
                        .join("Frameworks")
                        .join("libRainyNativeShell.dylib"),
                );
            }
        }
    }
    paths
}

#[cfg(target_os = "macos")]
fn native_shell_bridge() -> Result<&'static NativeShellBridgeSymbols, String> {
    let result = NATIVE_SHELL_BRIDGE.get_or_init(|| unsafe {
        let mut tried = Vec::new();
        for path in native_shell_bridge_candidates() {
            tried.push(path.display().to_string());
            if !path.exists() {
                continue;
            }

            let library = match Library::new(&path) {
                Ok(library) => library,
                Err(error) => {
                    return Err(format!(
                        "Failed to load libRainyNativeShell.dylib from {}: {}",
                        path.display(),
                        error
                    ));
                }
            };

            let initialize = *library
                .get::<unsafe extern "C" fn(extern "C" fn(*const c_char, *const c_char))>(
                    b"rainy_native_shell_bridge_initialize\0",
                )
                .map_err(|e| e.to_string())?;
            let runtime_supported = *library
                .get::<unsafe extern "C" fn() -> i32>(
                    b"rainy_native_shell_bridge_runtime_supported\0",
                )
                .map_err(|e| e.to_string())?;
            let show_palette = *library
                .get::<unsafe extern "C" fn() -> i32>(
                    b"rainy_native_shell_bridge_show_palette\0",
                )
                .map_err(|e| e.to_string())?;
            let update_snapshot = *library
                .get::<unsafe extern "C" fn(*const c_char) -> i32>(
                    b"rainy_native_shell_bridge_update_snapshot\0",
                )
                .map_err(|e| e.to_string())?;

            return Ok(NativeShellBridgeSymbols {
                _library: library,
                initialize,
                runtime_supported,
                show_palette,
                update_snapshot,
            });
        }

        Err(format!(
            "libRainyNativeShell.dylib not found. Tried: {}",
            tried.join(", ")
        ))
    });

    result.as_ref().map_err(|error| error.clone())
}

#[cfg(target_os = "macos")]
extern "C" fn native_shell_callback(action: *const c_char, payload: *const c_char) {
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
    CString::new(input)
        .map_err(|_| "Native shell payload contains interior null byte".to_string())
}

#[cfg(target_os = "macos")]
pub struct MacOSNativeShellBridge;

#[cfg(target_os = "macos")]
impl MacOSNativeShellBridge {
    pub fn is_runtime_supported() -> bool {
        match native_shell_bridge() {
            Ok(bridge) => unsafe { (bridge.runtime_supported)() == 1 },
            Err(error) => {
                tracing::warn!("{}", error);
                false
            }
        }
    }

    pub fn initialize(_app: AppHandle, native_shell: Arc<NativeShellService>) {
        if !Self::is_runtime_supported() {
            tracing::warn!(
                "macOS native shell bridge disabled because AppKit runtime is unavailable"
            );
            return;
        }

        if BRIDGE_INITIALIZED.load(Ordering::SeqCst) {
            tracing::debug!("macOS native shell bridge already initialized");
            return;
        }

        let (tx, mut rx) = mpsc::unbounded_channel::<(String, Option<String>)>();
        if ACTION_TX.set(tx).is_err() {
            tracing::warn!("macOS native shell bridge channel already registered");
            return;
        }

        let Ok(bridge) = native_shell_bridge() else {
            tracing::warn!("macOS native shell bridge dylib unavailable");
            return;
        };

        unsafe { (bridge.initialize)(native_shell_callback) };

        BRIDGE_INITIALIZED.store(true, Ordering::SeqCst);
        tracing::info!("macOS native shell bridge initialized");

        tauri::async_runtime::spawn(async move {
            while let Some((action, payload)) = rx.recv().await {
                native_shell.handle_bridge_action(action, payload).await;
            }
        });
    }

    pub fn show_palette() -> Result<(), String> {
        if !Self::is_runtime_supported() {
            return Err("Native shell is unavailable in the current runtime".to_string());
        }

        let bridge = native_shell_bridge()?;
        let ok = unsafe { (bridge.show_palette)() };
        if ok == 1 {
            Ok(())
        } else {
            Err("Native shell bridge failed to show the palette".to_string())
        }
    }

    pub fn update_snapshot(snapshot: &NativeShellSnapshot) -> Result<(), String> {
        if !Self::is_runtime_supported() {
            return Ok(());
        }

        let payload =
            serde_json::to_string(snapshot).map_err(|e| format!("Invalid shell snapshot: {}", e))?;
        let payload = c_string(&payload)?;
        let bridge = native_shell_bridge()?;
        let ok = unsafe { (bridge.update_snapshot)(payload.as_ptr()) };
        if ok == 1 {
            Ok(())
        } else {
            Err("Native shell bridge failed to update its snapshot".to_string())
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub struct MacOSNativeShellBridge;

#[cfg(not(target_os = "macos"))]
impl MacOSNativeShellBridge {
    #[allow(dead_code)]
    pub fn is_runtime_supported() -> bool {
        false
    }

    #[allow(dead_code)]
    pub fn initialize(
        _app: tauri::AppHandle,
        _native_shell: std::sync::Arc<crate::services::NativeShellService>,
    ) {
    }

    #[allow(dead_code)]
    pub fn show_palette() -> Result<(), String> {
        Err("Native shell is only available on macOS".to_string())
    }

    #[allow(dead_code)]
    pub fn update_snapshot(
        _snapshot: &crate::services::NativeShellSnapshot,
    ) -> Result<(), String> {
        Ok(())
    }
}
