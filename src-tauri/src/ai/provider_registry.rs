// Provider Registry
// Manages registration and retrieval of AI providers

use crate::ai::provider_types::{
    ProviderId, ProviderType, ProviderConfig, ProviderCapabilities, ProviderHealth,
    ChatCompletionRequest, ChatCompletionResponse,
    EmbeddingRequest, EmbeddingResponse,
    StreamingChunk, StreamingCallback,
    ProviderResult, AIError,
};
use crate::ai::provider_trait::{AIProvider, ProviderWithStats};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Provider registry
pub struct ProviderRegistry {
    /// Registered providers
    providers: Arc<DashMap<ProviderId, ProviderWithStats>>,
    /// Default provider ID
    default_provider: Arc<RwLock<Option<ProviderId>>>,
}

impl ProviderRegistry {
    /// Create a new provider registry
    pub fn new() -> Self {
        Self {
            providers: Arc::new(DashMap::new()),
            default_provider: Arc::new(RwLock::new(None)),
        }
    }

    /// Register a provider
    pub fn register(&self, provider: Arc<dyn AIProvider>) -> ProviderResult<()> {
        let id = provider.id().clone();
        let provider_with_stats = ProviderWithStats::new(provider);
        self.providers.insert(id, provider_with_stats);
        Ok(())
    }

    /// Unregister a provider
    pub fn unregister(&self, id: &ProviderId) -> ProviderResult<()> {
        self.providers.remove(id)
            .ok_or_else(|| AIError::ProviderNotFound(id.to_string()))?;
        Ok(())
    }

    /// Get a provider by ID
    pub fn get(&self, id: &ProviderId) -> ProviderResult<Arc<ProviderWithStats>> {
        self.providers.get(id)
            .map(|p| Arc::new(p.clone()))
            .ok_or_else(|| AIError::ProviderNotFound(id.to_string()))
    }

    /// Get all providers
    pub fn get_all(&self) -> Vec<Arc<ProviderWithStats>> {
        self.providers.iter()
            .map(|p| Arc::new(p.clone()))
            .collect()
    }

    /// Get providers by type
    pub fn get_by_type(&self, provider_type: ProviderType) -> Vec<Arc<ProviderWithStats>> {
        self.providers.iter()
            .filter(|p| p.provider().provider_type() == provider_type)
            .map(|p| Arc::new(p.clone()))
            .collect()
    }

    /// Get healthy providers
    pub async fn get_healthy(&self) -> ProviderResult<Vec<Arc<ProviderWithStats>>> {
        let mut healthy_providers = Vec::new();
        for provider in self.providers.iter() {
            let health = provider.provider().health_check().await?;
            if health == ProviderHealth::Healthy {
                healthy_providers.push(Arc::new(provider.clone()));
            }
        }
        Ok(healthy_providers)
    }

    /// Set the default provider
    pub async fn set_default(&self, id: &ProviderId) -> ProviderResult<()> {
        if !self.providers.contains_key(id) {
            return Err(AIError::ProviderNotFound(id.to_string()));
        }
        *self.default_provider.write().await = Some(id.clone());
        Ok(())
    }

    /// Get the default provider
    pub async fn get_default(&self) -> ProviderResult<Arc<ProviderWithStats>> {
        let default_id = self.default_provider.read().await;
        match default_id.as_ref() {
            Some(id) => self.get(id),
            None => {
                // If no default is set, return the first available provider
                self.providers.iter()
                    .next()
                    .map(|p| Arc::new(p.clone()))
                    .ok_or_else(|| AIError::Internal("No providers registered".to_string()))
            }
        }
    }

    /// Get provider capabilities
    pub async fn get_capabilities(&self, id: &ProviderId) -> ProviderResult<ProviderCapabilities> {
        let provider = self.get(id)?;
        provider.provider().capabilities().await
    }

    /// Check provider health
    pub async fn check_health(&self, id: &ProviderId) -> ProviderResult<ProviderHealth> {
        let provider = self.get(id)?;
        provider.provider().health_check().await
    }

    /// Complete a chat request with a specific provider
    pub async fn complete(
        &self,
        id: &ProviderId,
        request: ChatCompletionRequest,
    ) -> ProviderResult<ChatCompletionResponse> {
        let provider = self.get(id)?;
        let start = std::time::Instant::now();
        let result = provider.provider().complete(request).await;
        let latency = start.elapsed().as_millis() as u64;

        // Update stats
        let mut provider_mut = self.providers.get_mut(id).unwrap();
        let tokens = result.as_ref()
            .map(|r| r.usage.total_tokens as u64)
            .unwrap_or(0);
        provider_mut.update_stats(result.is_ok(), latency, tokens);

        result
    }

    /// Complete a chat request with streaming
    pub async fn complete_stream(
        &self,
        id: &ProviderId,
        request: ChatCompletionRequest,
        callback: StreamingCallback,
    ) -> ProviderResult<()> {
        let provider = self.get(id)?;
        let start = std::time::Instant::now();
        let result = provider.provider().complete_stream(request, callback).await;
        let latency = start.elapsed().as_millis() as u64;

        // Update stats
        let mut provider_mut = self.providers.get_mut(id).unwrap();
        provider_mut.update_stats(result.is_ok(), latency, 0);

        result
    }

    /// Generate embeddings
    pub async fn embed(
        &self,
        id: &ProviderId,
        request: EmbeddingRequest,
    ) -> ProviderResult<EmbeddingResponse> {
        let provider = self.get(id)?;
        let start = std::time::Instant::now();
        let result = provider.provider().embed(request).await;
        let latency = start.elapsed().as_millis() as u64;

        // Update stats
        let mut provider_mut = self.providers.get_mut(id).unwrap();
        let tokens = result.as_ref()
            .map(|r| r.usage.total_tokens as u64)
            .unwrap_or(0);
        provider_mut.update_stats(result.is_ok(), latency, tokens);

        result
    }

    /// Get provider statistics
    pub fn get_stats(&self, id: &ProviderId) -> ProviderResult<crate::ai::provider_trait::ProviderStats> {
        let provider = self.providers.get(id)
            .ok_or_else(|| AIError::ProviderNotFound(id.to_string()))?;
        Ok(provider.stats().clone())
    }

    /// Get all provider statistics
    pub fn get_all_stats(&self) -> Vec<(ProviderId, crate::ai::provider_trait::ProviderStats)> {
        self.providers.iter()
            .map(|p| (p.key().clone(), p.value().stats().clone()))
            .collect()
    }

    /// Clear all providers
    pub fn clear(&self) {
        self.providers.clear();
    }

    /// Get the number of registered providers
    pub fn count(&self) -> usize {
        self.providers.len()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ProviderRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_default() {
        let registry = ProviderRegistry::default();
        assert_eq!(registry.count(), 0);
    }
}
