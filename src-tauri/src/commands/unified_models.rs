// Unified Model Management Commands
// Provides a unified interface for managing models from all providers

use crate::ai::mode_selector::{ModeSelector, TaskComplexity, UseCase};
use crate::ai::provider::AIProviderManager;
use crate::ai::provider_types::StreamingChunk;
use crate::models::ProviderType;
// RainyClient import removed - using static model lists instead of API calls
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Unified model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedModel {
    /// Unique model identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Provider source
    pub provider: String,
    /// Model capabilities
    pub capabilities: ModelCapabilities,
    /// Whether the model is enabled by user
    pub enabled: bool,
    /// Processing mode (FastChat, DeepProcessing, etc.)
    pub processing_mode: String,
}

/// Model capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Supports chat completions
    pub chat: bool,
    /// Supports streaming
    pub streaming: bool,
    /// Supports function calling
    pub function_calling: bool,
    /// Supports vision/image analysis
    pub vision: bool,
    /// Supports web search
    pub web_search: bool,
    /// Maximum context tokens
    pub max_context: usize,
    /// Maximum output tokens
    pub max_output: usize,
}

/// User model preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserModelPreferences {
    /// Disabled model IDs
    pub disabled_models: Vec<String>,
    /// Default model for fast chat
    pub default_fast_model: Option<String>,
    /// Default model for deep processing
    pub default_deep_model: Option<String>,
}

impl Default for UserModelPreferences {
    fn default() -> Self {
        Self {
            disabled_models: vec![],
            default_fast_model: Some("gemini-2.0-flash".to_string()),
            default_deep_model: Some("gemini-2.5-pro".to_string()),
        }
    }
}

/// Get all available models from all providers
#[tauri::command]
pub async fn get_unified_models(
    app: AppHandle,
    provider_manager: tauri::State<'_, Arc<AIProviderManager>>,
) -> Result<Vec<UnifiedModel>, String> {
    let mut models = Vec::new();

    // Get models from Rainy SDK
    if let Ok(rainy_models) = get_rainy_sdk_models(&app).await {
        models.extend(rainy_models);
    }

    // Get models from other providers via AIProviderManager
    let other_models = get_provider_manager_models(&provider_manager).await?;

    // Add other models, avoiding duplicates
    for model in other_models {
        if !models.iter().any(|m| m.id == model.id) {
            models.push(model);
        }
    }

    // Apply user preferences (filter disabled models)
    let preferences = load_user_preferences(&app).await;
    models.retain(|m| !preferences.disabled_models.contains(&m.id));

    Ok(models)
}

/// Get models from Rainy SDK
/// - Cowork: fetches from API (dynamic based on plan)
/// - Rainy API: static list (all available models)
async fn get_rainy_sdk_models(_app: &AppHandle) -> Result<Vec<UnifiedModel>, String> {
    use crate::ai::keychain::KeychainManager;

    let mut models = Vec::new();
    let keychain = KeychainManager::new();

    // Check if user has Rainy API key
    let has_rainy = keychain
        .get_key("rainy_api")
        .map(|k| k.is_some())
        .unwrap_or(false);

    // Static Rainy API model list (all available models from rainy-sdk)
    // This includes Gemini 3 with thinking level variants
    if has_rainy {
        let rainy_api_models = [
            // OpenAI models
            "gpt-4o",
            "gpt-5",
            "gpt-5-pro",
            "o3",
            "o4-mini",
            // Anthropic models
            "claude-sonnet-4",
            "claude-opus-4-1",
            // Google Gemini 2.5 models
            "gemini-2.5-pro",
            "gemini-2.5-flash",
            "gemini-2.5-flash-lite",
            // Google Gemini 3 Flash with thinking levels
            "gemini-3-flash-minimal",
            "gemini-3-flash-low",
            "gemini-3-flash-medium",
            "gemini-3-flash-high",
            "gemini-3-flash-preview",
            // Google Gemini 3 Pro with thinking levels
            "gemini-3-pro-low",
            "gemini-3-pro-high",
            "gemini-3-pro-preview",
            // Gemini 3 Image
            "gemini-3-pro-image-preview",
            // Groq models
            "llama-3.1-8b-instant",
            "llama-3.3-70b-versatile",
            "moonshotai/kimi-k2-instruct-0905",
            // Cerebras models
            "cerebras/llama3.1-8b",
            // Enosis Labs models
            "astronomer-1",
            "astronomer-1-max",
            "astronomer-1.5",
            "astronomer-2",
            "astronomer-2-pro",
        ];

        for model_name in &rainy_api_models {
            // Avoid duplicates with Cowork models
            let model_id = format!("rainy:{}", model_name);
            if !models.iter().any(|m: &UnifiedModel| m.id == model_id) {
                models.push(UnifiedModel {
                    id: model_id,
                    name: model_name.to_string(),
                    provider: "Rainy API".to_string(),
                    capabilities: get_default_capabilities(),
                    enabled: true,
                    processing_mode: "rainy_api".to_string(),
                });
            }
        }
    }

    Ok(models)
}

/// Get models from AI Provider Manager
async fn get_provider_manager_models(
    provider_manager: &Arc<AIProviderManager>,
) -> Result<Vec<UnifiedModel>, String> {
    let mut models = Vec::new();

    // Get available provider configs
    let providers = provider_manager.list_providers().await;

    for config in providers {
        // Map provider type to internal string and display attributes
        // We include RainyApi and CoworkApi here as a fallback in case SDK fails
        // but get_unified_models will deduplicate if SDK succeeds
        let (provider_str_opt, prefix, provider_display) = match config.provider {
            ProviderType::Gemini => (Some("gemini"), "gemini", "Google Gemini"),
            ProviderType::RainyApi => (Some("rainy_api"), "rainy", "Rainy API"),
            #[allow(unreachable_patterns)]
            _ => (None, "", ""),
        };

        if let Some(provider_str) = provider_str_opt {
            // Get models for this provider
            if let Ok(provider_models) = provider_manager.get_models(provider_str).await {
                for model_name in provider_models {
                    // Determine processing mode based on provider type
                    let processing_mode = match config.provider {
                        ProviderType::RainyApi => "rainy_api".to_string(),
                        _ => "direct".to_string(),
                    };

                    let model = UnifiedModel {
                        // Use consistent ID prefixing (cowork:, rainy:, gemini:)
                        // to match SDK implementation
                        id: format!("{}:{}", prefix, model_name),
                        name: model_name.clone(),
                        provider: provider_display.to_string(),
                        capabilities: get_default_capabilities(),
                        enabled: true,
                        processing_mode,
                    };
                    models.push(model);
                }
            }
        }
    }

    Ok(models)
}

/// Get default model capabilities
fn get_default_capabilities() -> ModelCapabilities {
    ModelCapabilities {
        chat: true,
        streaming: true,
        function_calling: true,
        vision: true,
        web_search: true,
        max_context: 128000,
        max_output: 8192,
    }
}

/// Get Rainy API key from keychain
async fn get_rainy_api_key(_app: &AppHandle) -> Result<String, String> {
    use crate::ai::keychain::KeychainManager;

    // Use KeychainManager directly or via app state if registered
    // Here assuming concise instantiation as in other files
    let keychain = KeychainManager::new();

    // Try cowork key first, then rainy key

    if let Ok(Some(key)) = keychain.get_key("rainy_api") {
        return Ok(key);
    }

    Err("No API key found".to_string())
}

/// Load user preferences from storage
async fn load_user_preferences(app: &AppHandle) -> UserModelPreferences {
    let preferences_path = app
        .path()
        .app_data_dir()
        .unwrap()
        .join("model_preferences.json");

    if let Ok(content) = std::fs::read_to_string(&preferences_path) {
        if let Ok(preferences) = serde_json::from_str::<UserModelPreferences>(&content) {
            return preferences;
        }
    }

    UserModelPreferences::default()
}

/// Save user preferences to storage
async fn save_user_preferences(
    app: &AppHandle,
    preferences: &UserModelPreferences,
) -> Result<(), String> {
    let preferences_path = app
        .path()
        .app_data_dir()
        .unwrap()
        .join("model_preferences.json");

    let content = serde_json::to_string_pretty(preferences)
        .map_err(|e| format!("Failed to serialize preferences: {}", e))?;

    std::fs::write(&preferences_path, content)
        .map_err(|e| format!("Failed to write preferences: {}", e))?;

    Ok(())
}

/// Toggle model enabled/disabled state
#[tauri::command]
pub async fn toggle_model(app: AppHandle, model_id: String, enabled: bool) -> Result<(), String> {
    let mut preferences = load_user_preferences(&app).await;

    if enabled {
        preferences.disabled_models.retain(|id| id != &model_id);
    } else {
        if !preferences.disabled_models.contains(&model_id) {
            preferences.disabled_models.push(model_id);
        }
    }

    save_user_preferences(&app, &preferences).await
}

/// Set default model for fast chat
#[tauri::command]
pub async fn set_default_fast_model(app: AppHandle, model_id: String) -> Result<(), String> {
    let mut preferences = load_user_preferences(&app).await;
    preferences.default_fast_model = Some(model_id);
    save_user_preferences(&app, &preferences).await
}

/// Set default model for deep processing
#[tauri::command]
pub async fn set_default_deep_model(app: AppHandle, model_id: String) -> Result<(), String> {
    let mut preferences = load_user_preferences(&app).await;
    preferences.default_deep_model = Some(model_id);
    save_user_preferences(&app, &preferences).await
}

/// Get user model preferences
#[tauri::command]
pub async fn get_user_preferences(app: AppHandle) -> Result<UserModelPreferences, String> {
    Ok(load_user_preferences(&app).await)
}

/// Send a message using the unified model system
#[tauri::command]
pub async fn send_unified_message(
    app: AppHandle,
    provider_manager: tauri::State<'_, Arc<AIProviderManager>>,
    model_id: String,
    messages: Vec<ChatMessage>,
    use_case: String,
) -> Result<String, String> {
    // Parse model ID to get provider and model name
    let parts: Vec<&str> = model_id.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid model ID format".to_string());
    }

    // parts[0] is provider_str (e.g., "openai", "rainy_api"), parts[1] is model_name
    let provider_name = parts[0];
    let model_name = parts[1];

    // Get Rainy API key
    let api_key = get_rainy_api_key(&app).await?;

    // Determine processing mode
    let use_case_enum = match use_case.as_str() {
        "chat" => UseCase::QuickQuestion,
        "code" => UseCase::CodeReview,
        "analysis" => UseCase::FileOperation, // Mapping closest match
        "research" => UseCase::WebResearch,
        "streaming" => UseCase::StreamingResponse,
        _ => UseCase::QuickQuestion,
    };

    let _processing_mode = ModeSelector::select_mode(&api_key, use_case_enum, TaskComplexity::Low);

    // Convert messages to prompt
    let prompt = messages_to_prompt(&messages);

    // Execute based on provider and mode

    // Map string provider name back to ProviderType for manager
    let provider_type = match provider_name {
        "rainy_api" | "rainy" => ProviderType::RainyApi,
        "gemini" => ProviderType::Gemini,
        _ => ProviderType::RainyApi,
    };

    let result = provider_manager
        .execute_prompt(
            &provider_type,
            model_name,
            &prompt,
            |progress, message| {
                // Progress callback
                println!("Progress: {}% - {:?}", progress, message);
            },
            None::<fn(StreamingChunk)>,
        ) // No streaming for non-stream calls
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

/// Chat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Convert chat messages to a single prompt
fn messages_to_prompt(messages: &[ChatMessage]) -> String {
    messages
        .iter()
        .map(|msg| {
            let role = match msg.role.as_str() {
                "system" => "System",
                "user" => "User",
                "assistant" => "Assistant",
                _ => "User",
            };
            format!("{}: {}", role, msg.content)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Get recommended model for a specific use case
#[tauri::command]
pub async fn get_recommended_model(
    app: AppHandle,
    use_case: String,
) -> Result<UnifiedModel, String> {
    let preferences = load_user_preferences(&app).await;

    let model_id = match use_case.as_str() {
        "chat" | "fast" => preferences.default_fast_model,
        "analysis" | "research" | "deep" => preferences.default_deep_model,
        _ => preferences.default_fast_model,
    };

    if let Some(model_id) = model_id {
        // Get all models and find the requested one
        let provider_manager = app.state::<Arc<AIProviderManager>>();
        let models = get_unified_models(app.clone(), provider_manager).await?;

        if let Some(model) = models.iter().find(|m| m.id == model_id) {
            return Ok(model.clone());
        }
    }

    // Fallback to default
    Ok(UnifiedModel {
        id: "rainy:gemini-2.5-flash".to_string(),
        name: "Gemini 2.5 Flash".to_string(),
        provider: "rainy".to_string(),
        capabilities: get_default_capabilities(),
        enabled: true,
        processing_mode: "rainy_api".to_string(),
    })
}

/// Event for streaming responses
#[derive(Clone, Serialize)]
pub struct StreamEvent {
    pub event: String,
    pub data: String,
}

/// Stream chat response (simulated for now)
#[tauri::command]
pub async fn unified_chat_stream(
    provider_manager: tauri::State<'_, Arc<AIProviderManager>>,
    message: String,
    model_id: String,
    on_event: tauri::ipc::Channel<StreamEvent>,
) -> Result<(), String> {
    // Parse model ID to get provider and model name
    let parts: Vec<&str> = model_id.split(':').collect();
    if parts.len() != 2 {
        let _ = on_event.send(StreamEvent {
            event: "error".to_string(),
            data: "Invalid model ID format".to_string(),
        });
        return Err("Invalid model ID format".to_string());
    }

    let provider_name = parts[0];
    let model_name = parts[1];

    // Determine processing mode (simplified for stream)
    let _processing_mode = ModeSelector::select_mode(
        &"dummy_key", // We'll validate later
        UseCase::StreamingResponse,
        TaskComplexity::Low,
    );

    // Map string provider name to ProviderType
    let provider_type = match provider_name {
        "rainy_api" | "rainy" => ProviderType::RainyApi,
        "gemini" => ProviderType::Gemini,
        _ => ProviderType::RainyApi,
    };

    // Execute prompt with real streaming
    let prompt = message; // In real chat, we'd process history
    let channel = on_event.clone();

    let result = provider_manager
        .execute_prompt(
            &provider_type,
            model_name,
            &prompt,
            |_progress, _msg| {
                // Progress updates are optional for streaming
            },
            Some(move |chunk: crate::ai::provider_types::StreamingChunk| {
                // Emit content token if present
                if !chunk.content.is_empty() {
                    let _ = channel.send(StreamEvent {
                        event: "token".to_string(),
                        data: chunk.content,
                    });
                }

                // Emit thinking chunk if present
                if let Some(thought) = chunk.thought {
                    let _ = channel.send(StreamEvent {
                        event: "thinking".to_string(),
                        data: thought,
                    });
                }
            }),
        )
        .await;

    match result {
        Ok(_response) => {
            // Stream complete - send done event
            let _ = on_event.send(StreamEvent {
                event: "done".to_string(),
                data: "".to_string(),
            });
            Ok(())
        }
        Err(e) => {
            let _ = on_event.send(StreamEvent {
                event: "error".to_string(),
                data: e.to_string(),
            });
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_messages_to_prompt() {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are helpful".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
        ];

        let prompt = messages_to_prompt(&messages);
        assert!(prompt.contains("System: You are helpful"));
        assert!(prompt.contains("User: Hello"));
    }

    #[test]
    fn test_model_id_parsing() {
        let model_id = "openai:gpt-4o";
        let parts: Vec<&str> = model_id.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "openai");
        assert_eq!(parts[1], "gpt-4o");
    }
}
