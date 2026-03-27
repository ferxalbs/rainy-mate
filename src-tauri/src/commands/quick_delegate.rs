use crate::services::{QuickDelegateModalService, QuickDelegateStatus};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn open_quick_delegate_modal(
    quick_delegate: State<'_, Arc<QuickDelegateModalService>>,
) -> Result<(), String> {
    quick_delegate.open()
}

#[tauri::command]
pub async fn get_quick_delegate_status(
    quick_delegate: State<'_, Arc<QuickDelegateModalService>>,
) -> Result<QuickDelegateStatus, String> {
    Ok(quick_delegate.status())
}
