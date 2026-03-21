use crate::ai::model_catalog::{
    all_catalog_models, ensure_supported_model_slug, find_catalog_model, normalize_model_slug,
    CatalogModel,
    ModelProvider,
};
use crate::ai::mode_selector::{ModeSelector, TaskComplexity, UseCase};
use crate::ai::provider::AIProviderManager;
use crate::ai::provider_types::StreamingChunk;
use crate::models::ProviderType;
use rainy_sdk::models::{CapabilityFlag, ModelCatalogItem};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedModel {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub capabilities: ModelCapabilities,
    pub enabled: bool,
    pub processing_mode: String,
    pub reasoning_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub chat: bool,
    pub streaming: bool,
    pub function_calling: bool,
    pub vision: bool,
    pub web_search: bool,
    pub reasoning: bool,
    pub max_context: usize,
    pub max_output: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserModelPreferences {
    pub disabled_models: Vec<String>,
    pub default_fast_model: Option<String>,
    pub default_deep_model: Option<String>,
}

impl Default for UserModelPreferences {
    fn default() -> Self {
        Self {
            disabled_models: vec![],
            default_fast_model: Some("rainy:gemini-3-flash-preview".to_string()),
            default_deep_model: Some("rainy:gemini-3-pro-preview".to_string()),
        }
    }
}

fn to_unified_model(entry: &CatalogModel) -> UnifiedModel {
    let (provider_prefix, provider_name, processing_mode) = match entry.provider {
        ModelProvider::RainyApi => ("rainy", "Rainy API", "rainy_api"),
        ModelProvider::GeminiByok => ("gemini", "Google Gemini", "direct"),
    };

    UnifiedModel {
        id: format!("{}:{}", provider_prefix, entry.slug),
        name: entry.name.to_string(),
        provider: provider_name.to_string(),
        capabilities: ModelCapabilities {
            chat: true,
            streaming: entry.streaming,
            function_calling: entry.function_calling,
            vision: entry.vision,
            web_search: entry.web_search,
            reasoning: entry.thinking_level.is_some(),
            max_context: entry.max_context,
            max_output: entry.max_output,
        },
        enabled: true,
        processing_mode: processing_mode.to_string(),
        reasoning_level: entry.thinking_level.map(ToString::to_string),
    }
}

fn slug_to_title(slug: &str) -> String {
    slug.split(['/', '-', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn capability_flag_enabled(flag: Option<&CapabilityFlag>) -> bool {
    matches!(flag, Some(CapabilityFlag::Bool(true)))
}

fn dynamic_rainy_model_from_catalog(item: &ModelCatalogItem) -> UnifiedModel {
    if let Some(entry) = find_catalog_model(&item.id, ModelProvider::RainyApi) {
        return to_unified_model(&entry);
    }

    let caps = item.rainy_capabilities.as_ref();

    UnifiedModel {
        id: format!("rainy:{}", item.id),
        name: item
            .name
            .clone()
            .unwrap_or_else(|| slug_to_title(&item.id)),
        provider: "Rainy API".to_string(),
        capabilities: ModelCapabilities {
            chat: true,
            streaming: true,
            function_calling: capability_flag_enabled(caps.and_then(|caps| caps.tools.as_ref())),
            vision: capability_flag_enabled(caps.and_then(|caps| caps.image_input.as_ref())),
            web_search: true,
            reasoning: capability_flag_enabled(caps.and_then(|caps| caps.reasoning.as_ref())),
            max_context: item.context_length.unwrap_or(128_000) as usize,
            max_output: 65_536,
        },
        enabled: true,
        processing_mode: "rainy_api".to_string(),
        reasoning_level: if capability_flag_enabled(caps.and_then(|caps| caps.reasoning.as_ref())) {
            Some("dynamic".to_string())
        } else {
            None
        },
    }
}

fn dynamic_rainy_model(slug: &str) -> UnifiedModel {
    if let Some(entry) = find_catalog_model(slug, ModelProvider::RainyApi) {
        return to_unified_model(&entry);
    }

    UnifiedModel {
        id: format!("rainy:{slug}"),
        name: slug_to_title(slug),
        provider: "Rainy API".to_string(),
        capabilities: ModelCapabilities {
            chat: true,
            streaming: true,
            function_calling: true,
            vision: false,
            web_search: true,
            reasoning: false,
            max_context: 128_000,
            max_output: 65_536,
        },
        enabled: true,
        processing_mode: "rainy_api".to_string(),
        reasoning_level: None,
    }
}

#[tauri::command]
pub async fn get_unified_models(
    app: AppHandle,
    provider_manager: tauri::State<'_, Arc<AIProviderManager>>,
) -> Result<Vec<UnifiedModel>, String> {
    let has_gemini_key = provider_manager.has_api_key("gemini").await.unwrap_or(false);

    let mut models: Vec<UnifiedModel> = all_catalog_models()
        .into_iter()
        .filter(|entry| matches!(entry.provider, ModelProvider::GeminiByok) && has_gemini_key)
        .map(|entry| to_unified_model(&entry))
        .collect();

    if let Ok(catalog) = provider_manager.get_models_catalog("rainy_api").await {
        models.extend(catalog.iter().map(dynamic_rainy_model_from_catalog));
    } else if let Ok(dynamic_models) = provider_manager.get_models("rainy_api").await {
        models.extend(dynamic_models.iter().map(|slug| dynamic_rainy_model(slug)));
    }

    let preferences = load_user_preferences(&app).await;
    models.retain(|m| !preferences.disabled_models.contains(&m.id));
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models.dedup_by(|a, b| a.id == b.id);

    Ok(models)
}

async fn load_user_preferences(app: &AppHandle) -> UserModelPreferences {
    let preferences_path = app
        .path()
        .app_data_dir()
        .unwrap()
        .join("model_preferences.json");

    if let Ok(content) = tokio::fs::read_to_string(&preferences_path).await {
        if let Ok(preferences) = serde_json::from_str::<UserModelPreferences>(&content) {
            return preferences;
        }
    }

    UserModelPreferences::default()
}

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

    tokio::fs::write(&preferences_path, content)
        .await
        .map_err(|e| format!("Failed to write preferences: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn toggle_model(app: AppHandle, model_id: String, enabled: bool) -> Result<(), String> {
    let mut preferences = load_user_preferences(&app).await;

    if enabled {
        preferences.disabled_models.retain(|id| id != &model_id);
    } else if !preferences.disabled_models.contains(&model_id) {
        preferences.disabled_models.push(model_id);
    }

    save_user_preferences(&app, &preferences).await
}

#[tauri::command]
pub async fn set_default_fast_model(app: AppHandle, model_id: String) -> Result<(), String> {
    ensure_supported_model_slug(normalize_model_slug(&model_id))?;
    let mut preferences = load_user_preferences(&app).await;
    preferences.default_fast_model = Some(model_id);
    save_user_preferences(&app, &preferences).await
}

#[tauri::command]
pub async fn set_default_deep_model(app: AppHandle, model_id: String) -> Result<(), String> {
    ensure_supported_model_slug(normalize_model_slug(&model_id))?;
    let mut preferences = load_user_preferences(&app).await;
    preferences.default_deep_model = Some(model_id);
    save_user_preferences(&app, &preferences).await
}

#[tauri::command]
pub async fn get_user_preferences(app: AppHandle) -> Result<UserModelPreferences, String> {
    Ok(load_user_preferences(&app).await)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

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

#[tauri::command]
pub async fn send_unified_message(
    _app: AppHandle,
    provider_manager: tauri::State<'_, Arc<AIProviderManager>>,
    model_id: String,
    messages: Vec<ChatMessage>,
    use_case: String,
) -> Result<String, String> {
    let parts: Vec<&str> = model_id.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid model ID format".to_string());
    }

    let provider_name = parts[0];
    let model_name = parts[1];

    ensure_supported_model_slug(model_name)?;

    let api_key = {
        use crate::ai::keychain::KeychainManager;
        let keychain = KeychainManager::new();
        keychain
            .get_key("rainy_api")
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "No API key found".to_string())?
    };

    let use_case_enum = match use_case.as_str() {
        "chat" => UseCase::QuickQuestion,
        "code" => UseCase::CodeReview,
        "analysis" => UseCase::FileOperation,
        "research" => UseCase::WebResearch,
        "streaming" => UseCase::StreamingResponse,
        _ => UseCase::QuickQuestion,
    };

    let _processing_mode = ModeSelector::select_mode(&api_key, use_case_enum, TaskComplexity::Low);

    let prompt = messages_to_prompt(&messages);

    let provider_type = match provider_name {
        "rainy_api" | "rainy" => ProviderType::RainyApi,
        "gemini" => ProviderType::Gemini,
        _ => ProviderType::RainyApi,
    };

    provider_manager
        .execute_prompt(
            &provider_type,
            model_name,
            &prompt,
            |_progress, _message| {},
            None::<fn(StreamingChunk)>,
        )
        .await
        .map_err(|e| e.to_string())
}

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
        let provider_manager = app.state::<Arc<AIProviderManager>>();
        let models = get_unified_models(app.clone(), provider_manager).await?;
        if let Some(model) = models.iter().find(|m| m.id == model_id) {
            return Ok(model.clone());
        }
    }

    let provider_manager = app.state::<Arc<AIProviderManager>>();
    let models = get_unified_models(app.clone(), provider_manager).await?;
    models
        .into_iter()
        .next()
        .ok_or_else(|| "No available models found".to_string())
}

#[derive(Clone, Serialize)]
pub struct StreamEvent {
    pub event: String,
    pub data: String,
}

#[tauri::command]
pub async fn unified_chat_stream(
    provider_manager: tauri::State<'_, Arc<AIProviderManager>>,
    message: String,
    model_id: String,
    on_event: tauri::ipc::Channel<StreamEvent>,
) -> Result<(), String> {
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
    ensure_supported_model_slug(model_name)?;

    let provider_type = match provider_name {
        "rainy_api" | "rainy" => ProviderType::RainyApi,
        "gemini" => ProviderType::Gemini,
        _ => ProviderType::RainyApi,
    };

    let channel = on_event.clone();

    let result = provider_manager
        .execute_prompt(
            &provider_type,
            model_name,
            &message,
            |_progress, _msg| {},
            Some(move |chunk: crate::ai::provider_types::StreamingChunk| {
                if !chunk.content.is_empty() {
                    let _ = channel.send(StreamEvent {
                        event: "token".to_string(),
                        data: chunk.content,
                    });
                }
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
        let model_id = "rainy:gemini-3-flash-preview";
        let parts: Vec<&str> = model_id.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "rainy");
        assert_eq!(parts[1], "gemini-3-flash-preview");
    }
}
