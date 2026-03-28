with open('src-tauri/src/services/quick_delegate_modal.rs', 'r') as f:
    lines = f.readlines()

out = []
for line in lines:
    if line.startswith('use crate::services::{FileManager, MacOSNativeNotificationBridge, MacOSQuickDelegateBridge, SettingsManager};'):
        out.append('#[allow(unused_imports)]\n')
        out.append(line)
    elif line.startswith('pub struct QuickDelegateModalService'):
        out.append('#[allow(dead_code)]\n')
        out.append(line)
    elif line.startswith('impl QuickDelegateModalService {'):
        out.append('#[allow(dead_code)]\n')
        out.append(line)
    elif line.startswith('async fn execute_native_modal_prompt'):
        out.append('#[allow(dead_code)]\n')
        out.append(line)
    elif line.startswith('fn emit_finish_notification'):
        out.append('#[allow(dead_code)]\n')
        out.append(line)
    elif line.startswith('fn truncate_message'):
        out.append('#[allow(dead_code)]\n')
        out.append(line)
    elif line.startswith('fn build_summary'):
        out.append('#[allow(dead_code)]\n')
        out.append(line)
    else:
        out.append(line)

with open('src-tauri/src/services/quick_delegate_modal.rs', 'w') as f:
    f.writelines(out)
