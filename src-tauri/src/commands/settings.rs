// Rainy Cowork - Settings Commands
// Tauri commands for user settings and model selection

use crate::ai::provider::AIProviderManager;
use crate::services::settings::{ModelOption, SettingsManager, UserProfile, UserSettings};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

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

/// Get available models based on user's plan
#[tauri::command]
pub async fn get_available_models(
    _ai_provider: State<'_, Arc<AIProviderManager>>,
) -> Result<Vec<ModelOption>, String> {
    Ok(SettingsManager::get_available_models())
}
