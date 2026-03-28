set -e

echo "Fixing serial macros..."
sed -i 's/^\s*#\[serial\]/    #[serial_test::serial]/g' src-tauri/src/commands/memory.rs src-tauri/src/services/memory_vault/repository.rs src-tauri/src/ai/agent/workflow.rs src-tauri/src/ai/agent/verification_test.rs
sed -i 's/use serial_test::serial;//g' src-tauri/src/commands/memory.rs src-tauri/src/services/memory_vault/repository.rs src-tauri/src/ai/agent/workflow.rs src-tauri/src/ai/agent/verification_test.rs

echo "Fixing unused variables and imports in src-tauri/src/lib.rs..."
sed -i 's/tauri_app.run(|app_handle, event| {/tauri_app.run(|_app_handle, _event| {/g' src-tauri/src/lib.rs
sed -i '/use tauri::Manager;/d' src-tauri/src/lib.rs

echo "Fixing unused variables in src-tauri/src/ai/keychain.rs..."
sed -i 's/let account = format!("api_key_{}", provider);/let _account = format!("api_key_{}", provider);/g' src-tauri/src/ai/keychain.rs

echo "Fixing dead code in src-tauri/src/commands/agent.rs..."
sed -i 's/    NativeModal,/    #[allow(dead_code)]\n    NativeModal,/g' src-tauri/src/commands/agent.rs

echo "Fixing dead code in src-tauri/src/services/macos_native_notifications.rs..."
sed -i 's/impl MacOSNativeNotificationBridge {/impl MacOSNativeNotificationBridge {\n    #![allow(dead_code)]/g' src-tauri/src/services/macos_native_notifications.rs

echo "Fixing dead code in src-tauri/src/services/macos_quick_delegate.rs..."
sed -i 's/impl MacOSQuickDelegateBridge {/impl MacOSQuickDelegateBridge {\n    #![allow(dead_code)]/g' src-tauri/src/services/macos_quick_delegate.rs

echo "Fixing dead code and unused imports in src-tauri/src/services/quick_delegate_modal.rs..."
sed -i 's/pub struct QuickDelegateModalService {/#[allow(dead_code)]\npub struct QuickDelegateModalService {/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/impl QuickDelegateModalService {/impl QuickDelegateModalService {\n    #![allow(dead_code)]/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/fn execute_native_modal_prompt/#[allow(dead_code)]\nfn execute_native_modal_prompt/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/fn emit_finish_notification/#[allow(dead_code)]\nfn emit_finish_notification/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/fn truncate_message/#[allow(dead_code)]\nfn truncate_message/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/fn build_summary/#[allow(dead_code)]\nfn build_summary/g' src-tauri/src/services/quick_delegate_modal.rs
sed -i 's/use crate::services::{FileManager, MacOSNativeNotificationBridge, MacOSQuickDelegateBridge, SettingsManager};/use crate::services::{FileManager, SettingsManager};/g' src-tauri/src/services/quick_delegate_modal.rs

echo "Fixing ci.yml..."
# We will use python to reliably edit ci.yml
