with open('src-tauri/src/ai/keychain.rs', 'r') as f:
    c = f.read()
# we need to NOT prefix account with underscore but simply allow unused
c = c.replace('let _account', 'let account')
c = c.replace('let account = format!("api_key_{}", provider);', '#[allow(unused_variables)]\n        let account = format!("api_key_{}", provider);')
with open('src-tauri/src/ai/keychain.rs', 'w') as f:
    f.write(c)

with open('src-tauri/src/services/quick_delegate_modal.rs', 'r') as f:
    c = f.read()
c = c.replace('use crate::services::{FileManager, MacOSNativeNotificationBridge, MacOSQuickDelegateBridge, SettingsManager};', '#[allow(unused_imports)]\nuse crate::services::{FileManager, MacOSNativeNotificationBridge, MacOSQuickDelegateBridge, SettingsManager};')
c = c.replace('pub struct QuickDelegateModalService', '#[allow(dead_code)]\npub struct QuickDelegateModalService')
c = c.replace('impl QuickDelegateModalService {', '#[allow(dead_code)]\nimpl QuickDelegateModalService {')
c = c.replace('async fn execute_native_modal_prompt', '#[allow(dead_code)]\nasync fn execute_native_modal_prompt')
c = c.replace('fn emit_finish_notification', '#[allow(dead_code)]\nfn emit_finish_notification')
c = c.replace('fn truncate_message', '#[allow(dead_code)]\nfn truncate_message')
c = c.replace('fn build_summary', '#[allow(dead_code)]\nfn build_summary')
with open('src-tauri/src/services/quick_delegate_modal.rs', 'w') as f:
    f.write(c)

with open('src-tauri/src/services/mod.rs', 'r') as f:
    c = f.read()
c = c.replace('pub use macos_quick_delegate::MacOSQuickDelegateBridge;', '#[allow(unused_imports)]\npub use macos_quick_delegate::MacOSQuickDelegateBridge;')
with open('src-tauri/src/services/mod.rs', 'w') as f:
    f.write(c)

with open('src-tauri/src/lib.rs', 'r') as f:
    c = f.read()
c = c.replace('tauri_app.run(|app_handle, event| {', 'tauri_app.run(|#[allow(unused_variables)] app_handle, #[allow(unused_variables)] event| {')
c = c.replace('use tauri::Manager;', '#[allow(unused_imports)]\nuse tauri::Manager;')
with open('src-tauri/src/lib.rs', 'w') as f:
    f.write(c)
