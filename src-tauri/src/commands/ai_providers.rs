// Rainy Cowork - AI Provider Commands (PHASE 3)
// Tauri commands for AI provider management using the new provider registry

use crate::ai::provider_trait::AIProviderFactory;
use crate::ai::providers::{
    AnthropicProviderFactory, OpenAIProviderFactory, RainySDKProviderFactory, XAIProviderFactory,
};
use crate::ai::{
    AIProvider, ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    ProviderCapabilities, ProviderConfig, ProviderHealth, ProviderId, ProviderRegistry,
    ProviderType,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// Provider information DTO for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub provider_type: String,
    pub model: String,
    pub enabled: bool,
    pub priority: u32,
    pub health: String,
    pub capabilities: ProviderCapabilities,
}

/// Provider statistics DTO for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatsDto {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub total_tokens: u64,
    pub last_request: Option<String>,
}

/// Provider registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterProviderRequest {
    pub id: String,
    pub provider_type: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub enabled: bool,
    pub priority: u32,
    pub rate_limit: Option<u32>,
    pub timeout: u64,
}

/// Chat completion request from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequestDto {
    pub provider_id: Option<String>,
    pub messages: Vec<ChatMessageDto>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub stream: bool,
}

/// Chat message DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
}

impl From<ChatMessageDto> for crate::ai::ChatMessage {
    fn from(dto: ChatMessageDto) -> Self {
        crate::ai::ChatMessage {
            role: dto.role,
            content: dto.content.into(),
            name: dto.name,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

/// Embedding request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequestDto {
    pub provider_id: Option<String>,
    pub input: String,
    pub model: Option<String>,
}

/// State wrapper for ProviderRegistry
pub struct ProviderRegistryState(pub Arc<ProviderRegistry>);

/// List all registered providers
#[tauri::command]
pub async fn list_all_providers(
    registry: State<'_, ProviderRegistryState>,
) -> Result<Vec<ProviderInfo>, String> {
    let providers = registry.0.get_all();
    let mut provider_infos = Vec::new();

    for provider in providers {
        let provider_ref = provider.provider();
        let id = provider_ref.id().as_str().to_string();
        let provider_type = provider_ref.provider_type().to_string();
        let model = provider_ref.default_model().to_string();
        let config = provider_ref.config();
        let capabilities = provider_ref
            .capabilities()
            .await
            .map_err(|e| format!("Failed to get capabilities: {}", e))?;
        let health = provider_ref
            .health_check()
            .await
            .map_err(|e| format!("Failed to check health: {}", e))?;

        provider_infos.push(ProviderInfo {
            id,
            provider_type,
            model,
            enabled: config.enabled,
            priority: config.priority,
            health: format!("{:?}", health),
            capabilities,
        });
    }

    Ok(provider_infos)
}

/// Get detailed information about a specific provider
#[tauri::command]
pub async fn get_provider_info(
    id: String,
    registry: State<'_, ProviderRegistryState>,
) -> Result<ProviderInfo, String> {
    let provider_id = ProviderId::new(id);
    let provider = registry.0.get(&provider_id).map_err(|e| e.to_string())?;
    let provider_ref = provider.provider();

    let capabilities = provider_ref
        .capabilities()
        .await
        .map_err(|e| format!("Failed to get capabilities: {}", e))?;
    let health = provider_ref
        .health_check()
        .await
        .map_err(|e| format!("Failed to check health: {}", e))?;

    Ok(ProviderInfo {
        id: provider_ref.id().as_str().to_string(),
        provider_type: provider_ref.provider_type().to_string(),
        model: provider_ref.default_model().to_string(),
        enabled: provider_ref.config().enabled,
        priority: provider_ref.config().priority,
        health: format!("{:?}", health),
        capabilities,
    })
}

/// Register a new provider
#[tauri::command]
pub async fn register_provider(
    request: RegisterProviderRequest,
    registry: State<'_, ProviderRegistryState>,
) -> Result<String, String> {
    let provider_type = match request.provider_type.as_str() {
        "openai" => ProviderType::OpenAI,
        "anthropic" => ProviderType::Anthropic,
        "google" => ProviderType::Google,
        "xai" => ProviderType::XAI,
        "local" => ProviderType::Local,
        "custom" => ProviderType::Custom,
        "rainy-sdk" => ProviderType::RainySDK,
        _ => return Err(format!("Unknown provider type: {}", request.provider_type)),
    };

    let config = ProviderConfig {
        id: ProviderId::new(&request.id),
        provider_type,
        api_key: request.api_key,
        base_url: request.base_url,
        model: request.model,
        params: std::collections::HashMap::new(),
        enabled: request.enabled,
        priority: request.priority,
        rate_limit: request.rate_limit,
        timeout: request.timeout,
    };

    // Create provider based on type
    let provider: Arc<dyn AIProvider> = match provider_type {
        ProviderType::RainySDK => {
            <RainySDKProviderFactory as AIProviderFactory>::validate_config(&config)
                .map_err(|e| format!("Invalid config: {}", e))?;
            <RainySDKProviderFactory as AIProviderFactory>::create(config)
                .await
                .map_err(|e| format!("Failed to create provider: {}", e))?
        }
        ProviderType::OpenAI => {
            <OpenAIProviderFactory as AIProviderFactory>::validate_config(&config)
                .map_err(|e| format!("Invalid config: {}", e))?;
            <OpenAIProviderFactory as AIProviderFactory>::create(config)
                .await
                .map_err(|e| format!("Failed to create provider: {}", e))?
        }
        ProviderType::Anthropic => {
            <AnthropicProviderFactory as AIProviderFactory>::validate_config(&config)
                .map_err(|e| format!("Invalid config: {}", e))?;
            <AnthropicProviderFactory as AIProviderFactory>::create(config)
                .await
                .map_err(|e| format!("Failed to create provider: {}", e))?
        }
        ProviderType::XAI => {
            <XAIProviderFactory as AIProviderFactory>::validate_config(&config)
                .map_err(|e| format!("Invalid config: {}", e))?;
            <XAIProviderFactory as AIProviderFactory>::create(config)
                .await
                .map_err(|e| format!("Failed to create provider: {}", e))?
        }

        _ => {
            return Err(format!(
                "Provider type {:?} is not yet implemented.",
                provider_type
            ));
        }
    };

    // Register provider
    registry
        .0
        .register(provider)
        .map_err(|e| format!("Failed to register provider: {}", e))?;

    Ok(request.id)
}

/// Unregister a provider
#[tauri::command]
pub async fn unregister_provider(
    id: String,
    registry: State<'_, ProviderRegistryState>,
) -> Result<(), String> {
    let provider_id = ProviderId::new(id);
    registry
        .0
        .unregister(&provider_id)
        .map_err(|e| e.to_string())
}

/// Set the default provider
#[tauri::command]
pub async fn set_default_provider(
    id: String,
    registry: State<'_, ProviderRegistryState>,
) -> Result<(), String> {
    let provider_id = ProviderId::new(id);
    registry
        .0
        .set_default(&provider_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get the default provider
#[tauri::command]
pub async fn get_default_provider(
    registry: State<'_, ProviderRegistryState>,
) -> Result<ProviderInfo, String> {
    let provider = registry.0.get_default().await.map_err(|e| e.to_string())?;
    let provider_ref = provider.provider();

    let capabilities = provider_ref
        .capabilities()
        .await
        .map_err(|e| format!("Failed to get capabilities: {}", e))?;
    let health = provider_ref
        .health_check()
        .await
        .map_err(|e| format!("Failed to check health: {}", e))?;

    Ok(ProviderInfo {
        id: provider_ref.id().as_str().to_string(),
        provider_type: provider_ref.provider_type().to_string(),
        model: provider_ref.default_model().to_string(),
        enabled: provider_ref.config().enabled,
        priority: provider_ref.config().priority,
        health: format!("{:?}", health),
        capabilities,
    })
}

/// Get provider statistics
#[tauri::command]
pub async fn get_provider_stats(
    id: String,
    registry: State<'_, ProviderRegistryState>,
) -> Result<ProviderStatsDto, String> {
    let provider_id = ProviderId::new(id);
    let stats = registry
        .0
        .get_stats(&provider_id)
        .map_err(|e| e.to_string())?;

    Ok(ProviderStatsDto {
        total_requests: stats.total_requests,
        successful_requests: stats.successful_requests,
        failed_requests: stats.failed_requests,
        avg_latency_ms: stats.avg_latency_ms,
        total_tokens: stats.total_tokens,
        last_request: stats.last_request.map(|dt| dt.to_rfc3339()),
    })
}

/// Get all provider statistics
#[tauri::command]
pub async fn get_all_provider_stats(
    registry: State<'_, ProviderRegistryState>,
) -> Result<Vec<(String, ProviderStatsDto)>, String> {
    let stats = registry.0.get_all_stats();
    let mut result = Vec::new();

    for (id, stats) in stats {
        result.push((
            id.as_str().to_string(),
            ProviderStatsDto {
                total_requests: stats.total_requests,
                successful_requests: stats.successful_requests,
                failed_requests: stats.failed_requests,
                avg_latency_ms: stats.avg_latency_ms,
                total_tokens: stats.total_tokens,
                last_request: stats.last_request.map(|dt| dt.to_rfc3339()),
            },
        ));
    }

    Ok(result)
}

/// Test provider connection
#[tauri::command]
pub async fn test_provider_connection(
    id: String,
    registry: State<'_, ProviderRegistryState>,
) -> Result<ProviderHealth, String> {
    let provider_id = ProviderId::new(id);
    let health = registry
        .0
        .check_health(&provider_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(health)
}

/// Get provider capabilities
#[tauri::command]
pub async fn get_provider_capabilities(
    id: String,
    registry: State<'_, ProviderRegistryState>,
) -> Result<ProviderCapabilities, String> {
    let provider_id = ProviderId::new(id);
    let capabilities = registry
        .0
        .get_capabilities(&provider_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(capabilities)
}

/// Complete a chat request
#[tauri::command]
pub async fn complete_chat(
    request: ChatCompletionRequestDto,
    registry: State<'_, ProviderRegistryState>,
) -> Result<ChatCompletionResponse, String> {
    // Determine provider ID
    let provider_id = if let Some(id) = request.provider_id {
        ProviderId::new(id)
    } else {
        // Use default provider
        let default_provider = registry
            .0
            .get_default()
            .await
            .map_err(|e| format!("No default provider: {}", e))?;
        default_provider.provider().id().clone()
    };

    // Convert messages
    let messages: Vec<crate::ai::ChatMessage> =
        request.messages.into_iter().map(|m| m.into()).collect();

    // Determine model
    let model = request.model.unwrap_or_else(|| {
        // Get default model from provider
        let provider = registry.0.get(&provider_id).unwrap();
        provider.provider().default_model().to_string()
    });

    // Create completion request
    let completion_request = ChatCompletionRequest {
        messages,
        model,
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        top_p: request.top_p,
        frequency_penalty: request.frequency_penalty,
        presence_penalty: request.presence_penalty,
        stop: request.stop,
        stream: request.stream,
        tools: None,
        tool_choice: None,
        json_mode: false,
    };

    // Execute completion
    let response = registry
        .0
        .complete(&provider_id, completion_request)
        .await
        .map_err(|e| e.to_string())?;

    Ok(response)
}

/// Generate embeddings
#[tauri::command]
pub async fn generate_embeddings(
    request: EmbeddingRequestDto,
    registry: State<'_, ProviderRegistryState>,
) -> Result<EmbeddingResponse, String> {
    // Determine provider ID
    let provider_id = if let Some(id) = request.provider_id {
        ProviderId::new(id)
    } else {
        // Use default provider
        let default_provider = registry
            .0
            .get_default()
            .await
            .map_err(|e| format!("No default provider: {}", e))?;
        default_provider.provider().id().clone()
    };

    // Determine model
    let model = request.model.unwrap_or_else(|| {
        // Get default model from provider
        let provider = registry.0.get(&provider_id).unwrap();
        provider.provider().default_model().to_string()
    });

    // Create embedding request
    let embedding_request = EmbeddingRequest {
        input: request.input,
        model,
    };

    // Execute embedding
    let response = registry
        .0
        .embed(&provider_id, embedding_request)
        .await
        .map_err(|e| e.to_string())?;

    Ok(response)
}

/// Get available models for a provider
#[tauri::command]
pub async fn get_provider_available_models(
    id: String,
    registry: State<'_, ProviderRegistryState>,
) -> Result<Vec<String>, String> {
    let provider_id = ProviderId::new(id);
    let provider = registry.0.get(&provider_id).map_err(|e| e.to_string())?;
    let models = provider
        .provider()
        .available_models()
        .await
        .map_err(|e| e.to_string())?;
    Ok(models)
}

/// Clear all providers
#[tauri::command]
pub async fn clear_providers(registry: State<'_, ProviderRegistryState>) -> Result<(), String> {
    registry.0.clear();
    Ok(())
}

/// Get provider count
#[tauri::command]
pub async fn get_provider_count(
    registry: State<'_, ProviderRegistryState>,
) -> Result<usize, String> {
    Ok(registry.0.count())
}
