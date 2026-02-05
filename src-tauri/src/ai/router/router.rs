// Intelligent Router
// Orchestrates load balancing, cost optimization, capability matching, and fallback

use crate::ai::provider_trait::ProviderWithStats;
use crate::ai::provider_types::{
    AIError, ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    ProviderId, ProviderResult, StreamingCallback,
};
use crate::ai::router::fallback_chain::FallbackStrategy;
use crate::ai::router::load_balancer::LoadBalancingStrategy;
use crate::ai::router::{
    CapabilityMatcher, CircuitBreaker, CostOptimizer, FallbackChain, LoadBalancer,
};
use std::sync::Arc;

/// Router configuration
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Load balancing strategy
    pub load_balancing_strategy: LoadBalancingStrategy,
    /// Fallback strategy
    pub fallback_strategy: FallbackStrategy,
    /// Enable cost optimization
    pub enable_cost_optimization: bool,
    /// Enable capability matching
    pub enable_capability_matching: bool,
    /// Maximum retry attempts
    pub max_retries: usize,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            load_balancing_strategy: LoadBalancingStrategy::RoundRobin,
            fallback_strategy: FallbackStrategy::SkipUnhealthy,
            enable_cost_optimization: true,
            enable_capability_matching: true,
            max_retries: 3,
        }
    }
}

/// Intelligent router for provider selection
#[derive(Debug)]
pub struct IntelligentRouter {
    /// Load balancer
    load_balancer: LoadBalancer,
    /// Cost optimizer
    cost_optimizer: CostOptimizer,
    /// Capability matcher
    capability_matcher: CapabilityMatcher,
    /// Fallback chain
    fallback_chain: FallbackChain,
    /// Circuit breakers for each provider
    circuit_breakers: std::collections::HashMap<ProviderId, CircuitBreaker>,
    /// Configuration
    config: RouterConfig,
}

impl IntelligentRouter {
    /// Create a new intelligent router
    pub fn new(config: RouterConfig) -> Self {
        Self {
            load_balancer: LoadBalancer::default(),
            cost_optimizer: CostOptimizer::default(),
            capability_matcher: CapabilityMatcher::default(),
            fallback_chain: FallbackChain::default(),
            circuit_breakers: std::collections::HashMap::new(),
            config,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(RouterConfig::default())
    }

    /// Add a provider to the router
    pub fn add_provider(&mut self, provider: Arc<ProviderWithStats>) {
        let provider_id = provider.provider().id().clone();

        // Add to all components
        self.load_balancer.add_provider(provider.clone());
        self.cost_optimizer.add_provider(provider.clone());
        self.capability_matcher.add_provider(provider.clone());
        self.fallback_chain.add_provider(provider.clone());

        // Create circuit breaker for provider
        self.circuit_breakers
            .insert(provider_id.clone(), CircuitBreaker::default());
    }

    /// Remove a provider from the router
    pub fn remove_provider(&mut self, provider_id: &ProviderId) {
        self.load_balancer.remove_provider(provider_id);
        self.cost_optimizer.remove_provider(provider_id);
        self.capability_matcher.remove_provider(provider_id);
        self.fallback_chain.remove_provider(provider_id);
        self.circuit_breakers.remove(provider_id);
    }

    /// Get a provider by ID
    #[allow(dead_code)]
    pub fn get_provider(&self, provider_id: &ProviderId) -> Option<Arc<ProviderWithStats>> {
        self.load_balancer
            .providers()
            .iter()
            .find(|p| p.provider().id() == provider_id)
            .cloned()
    }

    /// Get all providers
    pub fn get_all_providers(&self) -> Vec<Arc<ProviderWithStats>> {
        self.load_balancer.providers().to_vec()
    }

    /// Complete a chat request with intelligent routing
    pub async fn complete(
        &self,
        request: ChatCompletionRequest,
    ) -> ProviderResult<ChatCompletionResponse> {
        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            // Select provider based on strategy
            let provider = self.select_provider(&request).await;

            if let Some(provider) = provider {
                let provider_id = provider.provider().id().clone();

                // Check circuit breaker
                let circuit_breaker = self.circuit_breakers.get(&provider_id);
                if let Some(cb) = circuit_breaker {
                    if !cb.allow_request().await {
                        tracing::warn!(
                            "Circuit breaker open for provider {}, skipping",
                            provider_id
                        );
                        continue;
                    }
                }

                // Execute request
                let result = provider.provider().complete(request.clone()).await;

                match result {
                    Ok(response) => {
                        // Record success
                        if let Some(cb) = self.circuit_breakers.get(&provider_id) {
                            cb.record_success().await;
                        }

                        return Ok(response);
                    }
                    Err(e) => {
                        // Record failure
                        if let Some(cb) = self.circuit_breakers.get(&provider_id) {
                            cb.record_failure().await;
                        }

                        last_error = Some(e.clone());
                        tracing::warn!(
                            "Provider {} failed on attempt {}: {}",
                            provider_id,
                            attempt + 1,
                            e
                        );
                    }
                }
            } else {
                return Err(AIError::Internal("No providers available".to_string()));
            }
        }

        // All attempts failed
        Err(last_error
            .unwrap_or_else(|| AIError::Internal("All provider attempts failed".to_string())))
    }

    /// Complete a chat request with streaming
    pub async fn complete_stream(
        &self,
        request: ChatCompletionRequest,
        callback: StreamingCallback,
    ) -> ProviderResult<()> {
        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            // Select provider based on strategy
            let provider = self.select_provider(&request).await;

            if let Some(provider) = provider {
                let provider_id = provider.provider().id().clone();

                // Check circuit breaker
                let circuit_breaker = self.circuit_breakers.get(&provider_id);
                if let Some(cb) = circuit_breaker {
                    if !cb.allow_request().await {
                        tracing::warn!(
                            "Circuit breaker open for provider {}, skipping",
                            provider_id
                        );
                        continue;
                    }
                }

                // Execute request
                let result = provider
                    .provider()
                    .complete_stream(request.clone(), Arc::clone(&callback))
                    .await;

                match result {
                    Ok(()) => {
                        // Record success
                        if let Some(cb) = self.circuit_breakers.get(&provider_id) {
                            cb.record_success().await;
                        }

                        return Ok(());
                    }
                    Err(e) => {
                        // Record failure
                        if let Some(cb) = self.circuit_breakers.get(&provider_id) {
                            cb.record_failure().await;
                        }

                        last_error = Some(e.clone());
                        tracing::warn!(
                            "Provider {} failed on attempt {}: {}",
                            provider_id,
                            attempt + 1,
                            e
                        );
                    }
                }
            } else {
                return Err(AIError::Internal("No providers available".to_string()));
            }
        }

        // All attempts failed
        Err(last_error
            .unwrap_or_else(|| AIError::Internal("All provider attempts failed".to_string())))
    }

    /// Generate embeddings with intelligent routing
    pub async fn embed(&self, request: EmbeddingRequest) -> ProviderResult<EmbeddingResponse> {
        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            // Select provider based on strategy
            let provider = self.select_provider_for_embeddings(&request).await;

            if let Some(provider) = provider {
                let provider_id = provider.provider().id().clone();

                // Check circuit breaker
                let circuit_breaker = self.circuit_breakers.get(&provider_id);
                if let Some(cb) = circuit_breaker {
                    if !cb.allow_request().await {
                        tracing::warn!(
                            "Circuit breaker open for provider {}, skipping",
                            provider_id
                        );
                        continue;
                    }
                }

                // Execute request
                let result = provider.provider().embed(request.clone()).await;

                match result {
                    Ok(response) => {
                        // Record success
                        if let Some(cb) = self.circuit_breakers.get(&provider_id) {
                            cb.record_success().await;
                        }

                        return Ok(response);
                    }
                    Err(e) => {
                        // Record failure
                        if let Some(cb) = self.circuit_breakers.get(&provider_id) {
                            cb.record_failure().await;
                        }

                        last_error = Some(e.clone());
                        tracing::warn!(
                            "Provider {} failed on attempt {}: {}",
                            provider_id,
                            attempt + 1,
                            e
                        );
                    }
                }
            } else {
                return Err(AIError::Internal("No providers available".to_string()));
            }
        }

        // All attempts failed
        Err(last_error
            .unwrap_or_else(|| AIError::Internal("All provider attempts failed".to_string())))
    }

    /// Select a provider for a request
    async fn select_provider(
        &self,
        request: &ChatCompletionRequest,
    ) -> Option<Arc<ProviderWithStats>> {
        // Build required capabilities
        let mut required = crate::ai::router::capability_matcher::RequiredCapabilities::new()
            .require_chat_completions();
        if request.stream {
            required = required.require_streaming();
        }

        // If cost optimization is enabled, try cost optimizer first
        if self.config.enable_cost_optimization {
            let estimated_input = request
                .messages
                .iter()
                .map(|m| m.content.text().len() as u32 / 4) // Rough estimate: 4 chars per token
                .sum::<u32>();
            let estimated_output = request.max_tokens.unwrap_or(1000);

            if let Some(provider) = self
                .cost_optimizer
                .select_provider(estimated_input, estimated_output)
            {
                return Some(provider);
            }
        }

        // If capability matching is enabled, try capability matcher
        if self.config.enable_capability_matching {
            if let Some(provider) = self
                .capability_matcher
                .select_best_provider(&required)
                .await
            {
                return Some(provider);
            }
        }

        // Fallback to load balancer
        self.load_balancer.select_provider()
    }

    /// Select a provider for embeddings
    async fn select_provider_for_embeddings(
        &self,
        _request: &EmbeddingRequest,
    ) -> Option<Arc<ProviderWithStats>> {
        // Build required capabilities
        let required =
            crate::ai::router::capability_matcher::RequiredCapabilities::new().require_embeddings();

        // Try capability matcher first
        if self.config.enable_capability_matching {
            if let Some(provider) = self
                .capability_matcher
                .select_best_provider(&required)
                .await
            {
                return Some(provider);
            }
        }

        // Fallback to load balancer
        self.load_balancer.select_provider()
    }

    /// Get router statistics
    pub fn get_stats(&self) -> RouterStats {
        RouterStats {
            total_providers: self.load_balancer.provider_count(),
            healthy_providers: self.count_healthy_providers(),
            circuit_breakers_open: self.count_open_circuits(),
        }
    }

    /// Count healthy providers
    fn count_healthy_providers(&self) -> usize {
        self.load_balancer
            .providers()
            .iter()
            .filter(|p| {
                let provider_id = p.provider().id();
                if let Some(cb) = self.circuit_breakers.get(provider_id) {
                    // Use blocking call for stats
                    let state = futures::executor::block_on(async { cb.state().await });
                    state == crate::ai::router::circuit_breaker::CircuitState::Closed
                } else {
                    true
                }
            })
            .count()
    }

    /// Count open circuit breakers
    fn count_open_circuits(&self) -> usize {
        self.circuit_breakers
            .values()
            .filter(|cb| {
                let state = futures::executor::block_on(async { cb.state().await });
                state == crate::ai::router::circuit_breaker::CircuitState::Open
            })
            .count()
    }

    /// Get configuration
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: RouterConfig) {
        self.config = config;
    }
}

/// Router statistics
#[derive(Debug, Clone)]
pub struct RouterStats {
    /// Total number of providers
    pub total_providers: usize,
    /// Number of healthy providers
    pub healthy_providers: usize,
    /// Number of open circuit breakers
    pub circuit_breakers_open: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = IntelligentRouter::default();
        assert_eq!(router.get_stats().total_providers, 0);
    }

    #[test]
    fn test_router_config() {
        let config = RouterConfig {
            max_retries: 5,
            ..Default::default()
        };
        let router = IntelligentRouter::new(config);
        assert_eq!(router.config().max_retries, 5);
    }
}
