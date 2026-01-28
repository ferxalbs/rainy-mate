// Fallback Chain
// Provides automatic fallback to alternative providers

use crate::ai::provider_trait::ProviderWithStats;
use crate::ai::provider_types::{ProviderId, ProviderHealth};
use std::sync::Arc;

/// Fallback strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Try providers in order
    Sequential,
    /// Try all providers in parallel
    Parallel,
    /// Try providers in order, but skip unhealthy ones
    SkipUnhealthy,
}

/// Fallback chain configuration
#[derive(Debug, Clone)]
pub struct FallbackChainConfig {
    /// Fallback strategy
    pub strategy: FallbackStrategy,
    /// Maximum number of fallback attempts
    pub max_attempts: usize,
    /// Timeout for each attempt (in seconds)
    pub attempt_timeout: u64,
}

impl Default for FallbackChainConfig {
    fn default() -> Self {
        Self {
            strategy: FallbackStrategy::SkipUnhealthy,
            max_attempts: 3,
            attempt_timeout: 30,
        }
    }
}

/// Fallback chain for provider resilience
pub struct FallbackChain {
    /// Provider chain (in priority order)
    chain: Vec<Arc<ProviderWithStats>>,
    /// Configuration
    config: FallbackChainConfig,
}

impl FallbackChain {
    /// Create a new fallback chain
    pub fn new(config: FallbackChainConfig) -> Self {
        Self {
            chain: Vec::new(),
            config,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(FallbackChainConfig::default())
    }

    /// Add a provider to the chain
    pub fn add_provider(&mut self, provider: Arc<ProviderWithStats>) {
        self.chain.push(provider);
    }

    /// Add a provider at a specific position
    pub fn add_provider_at(&mut self, index: usize, provider: Arc<ProviderWithStats>) {
        if index <= self.chain.len() {
            self.chain.insert(index, provider);
        }
    }

    /// Remove a provider from the chain
    pub fn remove_provider(&mut self, provider_id: &ProviderId) {
        self.chain.retain(|p| p.provider().id() != provider_id);
    }

    /// Get provider at index
    pub fn get_provider(&self, index: usize) -> Option<&Arc<ProviderWithStats>> {
        self.chain.get(index)
    }

    /// Get the first provider in the chain
    pub fn first_provider(&self) -> Option<&Arc<ProviderWithStats>> {
        self.chain.first()
    }

    /// Get the last provider in the chain
    pub fn last_provider(&self) -> Option<&Arc<ProviderWithStats>> {
        self.chain.last()
    }

    /// Get all providers in the chain
    pub fn providers(&self) -> &[Arc<ProviderWithStats>] {
        &self.chain
    }

    /// Get the number of providers in the chain
    pub fn len(&self) -> usize {
        self.chain.len()
    }

    /// Check if chain is empty
    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }

    /// Get configuration
    pub fn config(&self) -> &FallbackChainConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: FallbackChainConfig) {
        self.config = config;
    }

    /// Get next provider to try (based on strategy)
    pub async fn get_next_provider(
        &self,
        last_attempted: Option<&ProviderId>,
    ) -> Option<Arc<ProviderWithStats>> {
        match self.config.strategy {
            FallbackStrategy::Sequential => self.get_next_sequential(last_attempted),
            FallbackStrategy::Parallel => self.get_next_parallel(),
            FallbackStrategy::SkipUnhealthy => self.get_next_skip_unhealthy(last_attempted).await,
        }
    }

    /// Get next provider sequentially
    fn get_next_sequential(&self, last_attempted: Option<&ProviderId>) -> Option<Arc<ProviderWithStats>> {
        if let Some(last_id) = last_attempted {
            // Find the index of the last attempted provider
            if let Some(index) = self.chain.iter().position(|p| p.provider().id() == last_id) {
                // Return the next provider
                if index + 1 < self.chain.len() {
                    return Some(self.chain[index + 1].clone());
                }
            }
        }

        // Return the first provider if no last attempted or no next
        self.chain.first().cloned()
    }

    /// Get next provider (parallel strategy - just return first)
    fn get_next_parallel(&self) -> Option<Arc<ProviderWithStats>> {
        self.chain.first().cloned()
    }

    /// Get next provider skipping unhealthy ones
    async fn get_next_skip_unhealthy(&self, last_attempted: Option<&ProviderId>) -> Option<Arc<ProviderWithStats>> {
        let start_index = if let Some(last_id) = last_attempted {
            self.chain.iter().position(|p| p.provider().id() == last_id)
                .map(|i| i + 1)
                .unwrap_or(0)
        } else {
            0
        };

        // Find the next healthy provider
        for i in start_index..self.chain.len() {
            let provider = &self.chain[i];
            let health = provider.provider().health_check().await;
            if health.is_ok() && health.unwrap() == ProviderHealth::Healthy {
                return Some(provider.clone());
            }
        }

        // Wrap around to the beginning if needed
        for i in 0..start_index {
            let provider = &self.chain[i];
            let health = provider.provider().health_check().await;
            if health.is_ok() && health.unwrap() == ProviderHealth::Healthy {
                return Some(provider.clone());
            }
        }

        // No healthy provider found
        None
    }

    /// Check if a provider is in the chain
    pub fn contains(&self, provider_id: &ProviderId) -> bool {
        self.chain.iter().any(|p| p.provider().id() == provider_id)
    }

    /// Get the index of a provider in the chain
    pub fn index_of(&self, provider_id: &ProviderId) -> Option<usize> {
        self.chain.iter().position(|p| p.provider().id() == provider_id)
    }

    /// Clear the chain
    pub fn clear(&mut self) {
        self.chain.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_chain_creation() {
        let chain = FallbackChain::default();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_fallback_chain_strategy() {
        let config = FallbackChainConfig {
            strategy: FallbackStrategy::Parallel,
            ..Default::default()
        };
        let chain = FallbackChain::new(config);
        assert_eq!(chain.config().strategy, FallbackStrategy::Parallel);
    }
}
