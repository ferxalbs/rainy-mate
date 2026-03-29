// Rainy MaTE - AI Provider Commands
// Tauri commands for AI provider management with rainy-sdk integration

use crate::ai::AIProviderManager;
use crate::models::AIProviderConfig;

use std::sync::Arc;
use tauri::State;

/// List available AI providers
#[tauri::command]
pub async fn list_providers(
    provider_manager: State<'_, Arc<AIProviderManager>>,
) -> Result<Vec<AIProviderConfig>, String> {
    Ok(provider_manager.list_providers().await)
}

/// Validate an API key for a provider
#[tauri::command]
pub async fn validate_api_key(
    provider: String,
    api_key: String,
    provider_manager: State<'_, Arc<AIProviderManager>>,
) -> Result<bool, String> {
    provider_manager.validate_api_key(&provider, &api_key).await
}

/// Store API key securely in macOS Keychain
#[tauri::command]
pub async fn store_api_key(
    provider: String,
    api_key: String,
    provider_manager: State<'_, Arc<AIProviderManager>>,
) -> Result<(), String> {
    let result = provider_manager.store_api_key(&provider, &api_key).await;
    // Invalidate cache after storing new key
    provider_manager.invalidate_cache().await;
    result
}

/// Get stored API key from macOS Keychain
#[tauri::command]
pub async fn get_api_key(
    provider: String,
    provider_manager: State<'_, Arc<AIProviderManager>>,
) -> Result<Option<String>, String> {
    provider_manager.get_api_key(&provider).await
}

/// Delete stored API key from macOS Keychain
#[tauri::command]
pub async fn delete_api_key(
    provider: String,
    provider_manager: State<'_, Arc<AIProviderManager>>,
) -> Result<(), String> {
    let result = provider_manager.delete_api_key(&provider).await;
    // Invalidate cache after deleting key
    provider_manager.invalidate_cache().await;
    result
}

/// Get available models for a provider
#[tauri::command]
pub async fn get_provider_models(
    provider: String,
    provider_manager: State<'_, Arc<AIProviderManager>>,
) -> Result<Vec<String>, String> {
    provider_manager.get_models(&provider).await
}

/// Check if API key exists for a provider
#[tauri::command]
pub async fn has_api_key(
    provider: String,
    provider_manager: State<'_, Arc<AIProviderManager>>,
) -> Result<bool, String> {
    provider_manager.has_api_key(&provider).await
}
