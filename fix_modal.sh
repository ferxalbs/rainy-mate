git restore src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/use crate::services::{FileManager, MacOSNativeNotificationBridge, MacOSQuickDelegateBridge, SettingsManager};/#[allow(unused_imports)]\nuse crate::services::{FileManager, MacOSNativeNotificationBridge, MacOSQuickDelegateBridge, SettingsManager};/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/pub struct QuickDelegateModalService/#[allow(dead_code)]\npub struct QuickDelegateModalService/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/impl QuickDelegateModalService {/#[allow(dead_code)]\nimpl QuickDelegateModalService {/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/async fn execute_native_modal_prompt/#[allow(dead_code)]\nasync fn execute_native_modal_prompt/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/fn emit_finish_notification/#[allow(dead_code)]\nfn emit_finish_notification/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/fn truncate_message/#[allow(dead_code)]\nfn truncate_message/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/fn build_summary/#[allow(dead_code)]\nfn build_summary/g' src-tauri/src/services/quick_delegate_modal.rs
