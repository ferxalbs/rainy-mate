// Rainy Cowork - Settings Commands
// Tauri commands for user settings and model selection

use crate::ai::keychain::KeychainManager;
use crate::ai::provider::AIProviderManager;
use crate::commands::airlock::AirlockServiceState;
use crate::services::settings::{ModelOption, SettingsManager, UserProfile, UserSettings};
use crate::services::MacOSNativeNotificationBridge;
use crate::services::WorkspaceManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationStatus {
    pub enabled: bool,
    pub platform: String,
    pub native_runtime_supported: bool,
    pub permission: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessCredential {
    pub provider: String,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemReadiness {
    pub platform: String,
    pub notifications_enabled: bool,
    pub native_notification_runtime_supported: bool,
    pub notification_permission: String,
    pub workspace_count: usize,
    pub has_workspace: bool,
    pub pending_airlock_approvals: usize,
    pub pending_airlock_messages: u64,
    pub credentials: Vec<ReadinessCredential>,
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNotificationRequest {
    pub title: String,
    pub body: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
}

/// Get all user settings
#[tauri::command]
pub async fn get_user_settings(
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<UserSettings, String> {
    let settings = settings.lock().await;
    Ok(settings.get_settings().clone())
}

/// Get currently selected model
#[tauri::command]
pub async fn get_selected_model(
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<String, String> {
    let settings = settings.lock().await;
    Ok(settings.get_selected_model().to_string())
}

/// Set selected model
#[tauri::command]
pub async fn set_selected_model(
    model: String,
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<(), String> {
    let mut settings = settings.lock().await;
    settings.set_selected_model(model)
}

/// Set theme
#[tauri::command]
pub async fn set_theme(
    theme: String,
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<(), String> {
    let mut settings = settings.lock().await;
    settings.set_theme(theme)
}

/// Set notifications enabled
#[tauri::command]
pub async fn set_notifications(
    enabled: bool,
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<(), String> {
    let mut settings = settings.lock().await;
    settings.set_notifications(enabled)
}

#[tauri::command]
pub async fn get_notification_status(
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<NotificationStatus, String> {
    let settings = settings.lock().await;
    let native_status = MacOSNativeNotificationBridge::authorization_status();
    Ok(NotificationStatus {
        enabled: settings.get_settings().notifications_enabled,
        platform: std::env::consts::OS.to_string(),
        native_runtime_supported: cfg!(target_os = "macos")
            && MacOSNativeNotificationBridge::is_runtime_supported(),
        permission: match native_status {
            1 => "granted",
            -1 => "denied",
            -2 => "unsupported",
            _ => "unknown",
        }
        .to_string(),
    })
}

#[tauri::command]
pub async fn request_notification_permission(
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<bool, String> {
    let settings = settings.lock().await;
    if !settings.get_settings().notifications_enabled {
        return Err("Notifications are disabled in settings".to_string());
    }

    if !MacOSNativeNotificationBridge::is_runtime_supported() {
        return Err(
            "Native macOS notifications are unavailable because Rainy MaTE is not running from a bundled app"
                .to_string(),
        );
    }

    Ok(MacOSNativeNotificationBridge::request_authorization())
}

#[tauri::command]
pub async fn send_test_notification(
    #[allow(unused_variables)] app: AppHandle,
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<(), String> {
    let settings = settings.lock().await;
    if !settings.get_settings().notifications_enabled {
        return Err("Notifications are disabled in settings".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        if !MacOSNativeNotificationBridge::is_runtime_supported() {
            return Err(
                "Native macOS notifications are unavailable because Rainy MaTE is not running from a bundled app"
                    .to_string(),
            );
        }

        if !MacOSNativeNotificationBridge::request_authorization() {
            return Err("macOS notification permission was denied".to_string());
        }
        return MacOSNativeNotificationBridge::send_test_notification(
            "Rainy MaTE",
            "Test notification from the native macOS notification bridge.",
        );
    }

    #[cfg(not(target_os = "macos"))]
    {
        app.emit(
            "desktop:notification",
            DesktopNotificationRequest {
                title: "Rainy MaTE".to_string(),
                body: "Test notification from the local desktop runtime.".to_string(),
                kind: "test".to_string(),
                command_id: None,
            },
        )
        .map_err(|e| format!("Failed to emit test notification: {}", e))
    }
}

#[tauri::command]
pub async fn focus_airlock_request(
    app: AppHandle,
    command_id: Option<String>,
) -> Result<(), String> {
    MacOSNativeNotificationBridge::activate_app();

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();

    app.emit("airlock:notification_clicked", command_id)
        .map_err(|e| format!("Failed to emit notification activation event: {}", e))
}

#[tauri::command]
pub async fn get_system_readiness(
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
    workspace_manager: State<'_, Arc<WorkspaceManager>>,
    airlock_state: State<'_, AirlockServiceState>,
) -> Result<SystemReadiness, String> {
    let settings = settings.lock().await;
    let native_status = MacOSNativeNotificationBridge::authorization_status();
    let notification_permission = match native_status {
        1 => "granted",
        -1 => "denied",
        -2 => "unsupported",
        _ => "unknown",
    }
    .to_string();

    let workspace_count = workspace_manager
        .list_workspaces()
        .map(|items| items.len())
        .unwrap_or(0);

    let keychain = KeychainManager::new();
    let providers = [
        "rainy_api",
        "gemini",
        "openai",
        "anthropic",
        "xai",
        "openrouter",
    ];
    let credentials = providers
        .iter()
        .map(|provider| ReadinessCredential {
            provider: (*provider).to_string(),
            configured: keychain
                .get_key(provider)
                .ok()
                .and_then(|value| value)
                .is_some(),
        })
        .collect::<Vec<_>>();

    let (pending_airlock_approvals, pending_airlock_messages) = {
        let guard = airlock_state.0.lock().await;
        if let Some(airlock) = guard.as_ref() {
            let pending_approvals = airlock.get_pending_approvals().await.len();
            let pending_messages = airlock.count_pending_messages().await.unwrap_or(0);
            (pending_approvals, pending_messages)
        } else {
            (0, 0)
        }
    };

    Ok(SystemReadiness {
        platform: std::env::consts::OS.to_string(),
        notifications_enabled: settings.get_settings().notifications_enabled,
        native_notification_runtime_supported: cfg!(target_os = "macos")
            && MacOSNativeNotificationBridge::is_runtime_supported(),
        notification_permission,
        workspace_count,
        has_workspace: workspace_count > 0,
        pending_airlock_approvals,
        pending_airlock_messages,
        credentials,
    })
}

/// Get user profile
#[tauri::command]
pub async fn get_user_profile(
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<UserProfile, String> {
    let settings = settings.lock().await;
    Ok(settings.get_profile().clone())
}

/// Set user profile
#[tauri::command]
pub async fn set_user_profile(
    profile: UserProfile,
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<(), String> {
    let mut settings = settings.lock().await;
    settings.set_profile(profile)
}

/// Get embedder provider
#[tauri::command]
pub async fn get_embedder_provider(
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<String, String> {
    let settings = settings.lock().await;
    Ok(settings.get_embedder_provider().to_string())
}

/// Set embedder provider
#[tauri::command]
pub async fn set_embedder_provider(
    provider: String,
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<(), String> {
    let mut settings = settings.lock().await;
    settings.set_embedder_provider(provider)
}

/// Get embedder model
#[tauri::command]
pub async fn get_embedder_model(
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<String, String> {
    let settings = settings.lock().await;
    Ok(settings.get_embedder_model().to_string())
}

/// Set embedder model
#[tauri::command]
pub async fn set_embedder_model(
    model: String,
    settings: State<'_, Arc<Mutex<SettingsManager>>>,
) -> Result<(), String> {
    let mut settings = settings.lock().await;
    settings.set_embedder_model(model)
}

/// Get available models based on user's plan
#[tauri::command]
pub async fn get_available_models(
    provider_manager: State<'_, Arc<AIProviderManager>>,
) -> Result<Vec<ModelOption>, String> {
    Ok(SettingsManager::get_available_models(Some(provider_manager.inner().as_ref())).await)
}
