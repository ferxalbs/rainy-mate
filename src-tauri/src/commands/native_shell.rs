use crate::commands::airlock::AirlockServiceState;
use crate::services::{FileManager, NativeShellService, NativeShellSnapshot, NativeShellStatus};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_native_shell_status(
    native_shell: State<'_, Arc<NativeShellService>>,
) -> Result<NativeShellStatus, String> {
    Ok(native_shell.status())
}

#[tauri::command]
pub async fn refresh_native_shell(
    native_shell: State<'_, Arc<NativeShellService>>,
    file_manager: State<'_, Arc<FileManager>>,
    airlock_state: State<'_, AirlockServiceState>,
) -> Result<NativeShellSnapshot, String> {
    let workspace = file_manager.get_workspace().await;
    let pending_approval_count = {
        let guard = airlock_state.0.lock().await;
        if let Some(airlock) = guard.as_ref() {
            airlock.get_pending_approvals().await.len()
        } else {
            0
        }
    };

    native_shell
        .refresh(
            workspace.as_ref().map(|workspace| workspace.name.clone()),
            workspace.as_ref().map(|workspace| workspace.path.clone()),
            pending_approval_count,
        )
        .await
}

#[tauri::command]
pub async fn show_native_shell_palette(
    native_shell: State<'_, Arc<NativeShellService>>,
) -> Result<(), String> {
    if !native_shell.status().available {
        return Err("Native shell is unavailable in the current runtime".to_string());
    }

    crate::services::MacOSNativeShellBridge::show_palette()
}
