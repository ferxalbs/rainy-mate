// Rainy Cowork - Router Commands (PHASE 3)
// Tauri commands for IntelligentRouter with advanced routing and streaming

use crate::ai::router::fallback_chain::FallbackStrategy;
use crate::ai::router::load_balancer::LoadBalancingStrategy;
use crate::ai::router::router::{RouterConfig, RouterStats};
use crate::ai::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, EmbeddingRequest,
    EmbeddingResponse, IntelligentRouter, ProviderId, StreamingChunk,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{ipc::Channel, State};
use tokio::sync::RwLock;

/// State wrapper for IntelligentRouter
pub struct IntelligentRouterState(pub Arc<RwLock<IntelligentRouter>>);

/// Router configuration DTO for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfigDto {
    pub load_balancing_strategy: String,
    pub fallback_strategy: String,
    pub cost_optimization_enabled: bool,
    pub capability_matching_enabled: bool,
    pub max_retries: usize,
}

impl From<&RouterConfig> for RouterConfigDto {
    fn from(config: &RouterConfig) -> Self {
        Self {
            load_balancing_strategy: format!("{:?}", config.load_balancing_strategy),
            fallback_strategy: format!("{:?}", config.fallback_strategy),
            cost_optimization_enabled: config.enable_cost_optimization,
            capability_matching_enabled: config.enable_capability_matching,
            max_retries: config.max_retries,
        }
    }
}

/// Router statistics DTO for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterStatsDto {
    pub total_providers: usize,
    pub healthy_providers: usize,
    pub circuit_breakers_open: usize,
}

impl From<RouterStats> for RouterStatsDto {
    fn from(stats: RouterStats) -> Self {
        Self {
            total_providers: stats.total_providers,
            healthy_providers: stats.healthy_providers,
            circuit_breakers_open: stats.circuit_breakers_open,
        }
    }
}

/// Chat request for intelligent routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedChatRequest {
    pub messages: Vec<ChatMessageDto>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub preferred_provider: Option<String>,
}

/// Chat message DTO for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
}

impl From<ChatMessageDto> for ChatMessage {
    fn from(dto: ChatMessageDto) -> Self {
        ChatMessage {
            role: dto.role,
            content: dto.content.into(),
            name: dto.name,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

/// Streaming event for Tauri channel
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum StreamingEvent {
    #[serde(rename_all = "camelCase")]
    Started { model: String, provider_id: String },
    #[serde(rename_all = "camelCase")]
    Chunk { content: String, is_final: bool },
    #[serde(rename_all = "camelCase")]
    Finished {
        finish_reason: String,
        total_chunks: usize,
    },
    #[serde(rename_all = "camelCase")]
    Error { message: String },
}

/// Embedding request for intelligent routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedEmbeddingRequest {
    pub input: String,
    pub model: Option<String>,
    pub preferred_provider: Option<String>,
}

/// Get router configuration
#[tauri::command]
pub async fn get_router_config(
    router: State<'_, IntelligentRouterState>,
) -> Result<RouterConfigDto, String> {
    let router = router.0.read().await;
    let config = router.config();
    Ok(RouterConfigDto::from(config))
}

/// Update router configuration
#[tauri::command]
pub async fn update_router_config(
    load_balancing_strategy: Option<String>,
    fallback_strategy: Option<String>,
    cost_optimization_enabled: Option<bool>,
    capability_matching_enabled: Option<bool>,
    max_retries: Option<usize>,
    router: State<'_, IntelligentRouterState>,
) -> Result<RouterConfigDto, String> {
    let mut router = router.0.write().await;
    let mut config = router.config().clone();

    if let Some(strategy) = load_balancing_strategy {
        config.load_balancing_strategy = match strategy.to_lowercase().as_str() {
            "roundrobin" | "round_robin" => LoadBalancingStrategy::RoundRobin,
            "leastconnections" | "least_connections" => LoadBalancingStrategy::LeastConnections,
            "weightedroundrobin" | "weighted_round_robin" | "weighted" => {
                LoadBalancingStrategy::WeightedRoundRobin
            }
            "random" => LoadBalancingStrategy::Random,
            _ => config.load_balancing_strategy,
        };
    }

    if let Some(strategy) = fallback_strategy {
        config.fallback_strategy = match strategy.to_lowercase().as_str() {
            "sequential" => FallbackStrategy::Sequential,
            "parallel" => FallbackStrategy::Parallel,
            "skipunhealthy" | "skip_unhealthy" => FallbackStrategy::SkipUnhealthy,
            _ => config.fallback_strategy,
        };
    }

    if let Some(enabled) = cost_optimization_enabled {
        config.enable_cost_optimization = enabled;
    }

    if let Some(enabled) = capability_matching_enabled {
        config.enable_capability_matching = enabled;
    }

    if let Some(retries) = max_retries {
        config.max_retries = retries;
    }

    router.set_config(config.clone());
    Ok(RouterConfigDto::from(&config))
}

/// Get router statistics
#[tauri::command]
pub async fn get_router_stats(
    router: State<'_, IntelligentRouterState>,
) -> Result<RouterStatsDto, String> {
    let router = router.0.read().await;
    let stats = router.get_stats();
    Ok(RouterStatsDto::from(stats))
}

/// Complete a chat using intelligent routing
#[tauri::command]
pub async fn complete_with_routing(
    request: RoutedChatRequest,
    router: State<'_, IntelligentRouterState>,
) -> Result<ChatCompletionResponse, String> {
    let router = router.0.read().await;

    // Convert messages
    let messages: Vec<ChatMessage> = request.messages.into_iter().map(|m| m.into()).collect();

    // Build request
    let chat_request = ChatCompletionRequest {
        messages,
        model: request.model.unwrap_or_else(|| "default".to_string()),
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        top_p: request.top_p,
        frequency_penalty: request.frequency_penalty,
        presence_penalty: request.presence_penalty,
        stop: request.stop,
        stream: false,
        tools: None,
        tool_choice: None,
        json_mode: false,
    };

    // Execute with intelligent routing
    let response = router
        .complete(chat_request)
        .await
        .map_err(|e| e.to_string())?;

    Ok(response)
}

/// Stream a chat using intelligent routing with Tauri channels
#[tauri::command]
pub async fn stream_with_routing(
    request: RoutedChatRequest,
    on_event: Channel<StreamingEvent>,
    router: State<'_, IntelligentRouterState>,
) -> Result<(), String> {
    let router = router.0.read().await;

    // Convert messages
    let messages: Vec<ChatMessage> = request.messages.into_iter().map(|m| m.into()).collect();

    let model = request
        .model
        .clone()
        .unwrap_or_else(|| "default".to_string());

    // Build request
    let chat_request = ChatCompletionRequest {
        messages,
        model: model.clone(),
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        top_p: request.top_p,
        frequency_penalty: request.frequency_penalty,
        presence_penalty: request.presence_penalty,
        stop: request.stop,
        stream: true,
        tools: None,
        tool_choice: None,
        json_mode: false,
    };

    // Send started event (we'll get the actual provider from the router)
    let _ = on_event.send(StreamingEvent::Started {
        model: model.clone(),
        provider_id: "intelligent_router".to_string(),
    });

    // Create channel for callback
    let channel = on_event.clone();

    // Create callback for streaming chunks
    let callback = Arc::new(move |chunk: StreamingChunk| {
        let event = StreamingEvent::Chunk {
            content: chunk.content,
            is_final: chunk.is_final,
        };
        let _ = channel.send(event);
    });

    // Execute with intelligent routing and streaming
    match router.complete_stream(chat_request, callback).await {
        Ok(()) => {
            // Send finished event
            // Note: chunk_count tracking not implemented yet
            let _ = on_event.send(StreamingEvent::Finished {
                finish_reason: "stop".to_string(),
                total_chunks: 0,
            });
            Ok(())
        }
        Err(e) => {
            // Send error event
            let _ = on_event.send(StreamingEvent::Error {
                message: e.to_string(),
            });
            Err(e.to_string())
        }
    }
}

/// Generate embeddings using intelligent routing
#[tauri::command]
pub async fn embed_with_routing(
    request: RoutedEmbeddingRequest,
    router: State<'_, IntelligentRouterState>,
) -> Result<EmbeddingResponse, String> {
    let router = router.0.read().await;

    // Build request
    let embedding_request = EmbeddingRequest {
        input: request.input,
        model: request.model.unwrap_or_else(|| "default".to_string()),
    };

    // Execute with intelligent routing
    let response = router
        .embed(embedding_request)
        .await
        .map_err(|e| e.to_string())?;

    Ok(response)
}

/// Add a provider to the router
#[tauri::command]
pub async fn add_provider_to_router(
    provider_id: String,
    router: State<'_, IntelligentRouterState>,
    registry: State<'_, crate::commands::ai_providers::ProviderRegistryState>,
) -> Result<(), String> {
    let provider = registry
        .0
        .get(&ProviderId::new(&provider_id))
        .map_err(|e| e.to_string())?;

    let mut router = router.0.write().await;
    router.add_provider(provider);

    Ok(())
}

/// Remove a provider from the router
#[tauri::command]
pub async fn remove_provider_from_router(
    provider_id: String,
    router: State<'_, IntelligentRouterState>,
) -> Result<(), String> {
    let mut router = router.0.write().await;
    router.remove_provider(&ProviderId::new(&provider_id));
    Ok(())
}

/// Get all providers in the router
#[tauri::command]
pub async fn get_router_providers(
    router: State<'_, IntelligentRouterState>,
) -> Result<Vec<String>, String> {
    let router = router.0.read().await;
    let providers = router.get_all_providers();

    Ok(providers
        .iter()
        .map(|p| p.provider().id().as_str().to_string())
        .collect())
}

/// Check if the router has any providers
#[tauri::command]
pub async fn router_has_providers(
    router: State<'_, IntelligentRouterState>,
) -> Result<bool, String> {
    let router = router.0.read().await;
    Ok(!router.get_all_providers().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_config_dto_conversion() {
        let config = RouterConfig::default();
        let dto: RouterConfigDto = (&config).into();

        assert!(dto.cost_optimization_enabled);
        assert!(dto.capability_matching_enabled);
        assert_eq!(dto.max_retries, 3);
    }

    #[test]
    fn test_streaming_event_serialization() {
        let event = StreamingEvent::Chunk {
            content: "Hello".to_string(),
            is_final: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("chunk"));
        assert!(json.contains("Hello"));
    }
}
