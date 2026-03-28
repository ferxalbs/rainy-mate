import re
with open('src-tauri/src/lib.rs', 'r') as f:
    content = f.read()

# Only remove unused QuickDelegateModal imports, DO NOT REMOVE tauri::Manager
content = content.replace("use crate::services::{FileManager, MacOSNativeNotificationBridge, MacOSQuickDelegateBridge, SettingsManager};", "use crate::services::{FileManager, SettingsManager};")

# Instead of blindly changing the run closure, let's look at it
old_run = """    tauri_app.run(|app_handle, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Ready = event {"""
new_run = """    tauri_app.run(|#[allow(unused_variables)] app_handle, #[allow(unused_variables)] event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Ready = event {"""

content = content.replace(old_run, new_run)

with open('src-tauri/src/lib.rs', 'w') as f:
    f.write(content)
