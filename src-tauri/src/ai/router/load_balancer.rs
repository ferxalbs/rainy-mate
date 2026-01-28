// Load Balancer
// Distributes requests across multiple providers

use crate::ai::provider_trait::ProviderWithStats;
use crate::ai::provider_types::ProviderId;
use rand::Rng;
use std::sync::Arc;

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancingStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// Random selection
    Random,
}

/// Load balancer configuration
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// Load balancing strategy
    pub strategy: LoadBalancingStrategy,
    /// Provider weights (for weighted strategies)
    pub weights: std::collections::HashMap<ProviderId, u32>,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::RoundRobin,
            weights: std::collections::HashMap::new(),
        }
    }
}

/// Load balancer for distributing requests
pub struct LoadBalancer {
    /// Available providers
    providers: Vec<Arc<ProviderWithStats>>,
    /// Current round-robin index
    round_robin_index: std::sync::atomic::AtomicUsize,
    /// Configuration
    config: LoadBalancerConfig,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(config: LoadBalancerConfig) -> Self {
        Self {
            providers: Vec::new(),
            round_robin_index: std::sync::atomic::AtomicUsize::new(0),
            config,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(LoadBalancerConfig::default())
    }

    /// Add a provider to the load balancer
    pub fn add_provider(&mut self, provider: Arc<ProviderWithStats>) {
        self.providers.push(provider);
    }

    /// Remove a provider from the load balancer
    pub fn remove_provider(&mut self, provider_id: &ProviderId) {
        self.providers.retain(|p| p.provider().id() != provider_id);
    }

    /// Get all providers
    pub fn providers(&self) -> &[Arc<ProviderWithStats>] {
        &self.providers
    }

    /// Select a provider based on the configured strategy
    pub fn select_provider(&self) -> Option<Arc<ProviderWithStats>> {
        if self.providers.is_empty() {
            return None;
        }

        match self.config.strategy {
            LoadBalancingStrategy::RoundRobin => self.select_round_robin(),
            LoadBalancingStrategy::LeastConnections => self.select_least_connections(),
            LoadBalancingStrategy::WeightedRoundRobin => self.select_weighted_round_robin(),
            LoadBalancingStrategy::Random => self.select_random(),
        }
    }

    /// Round-robin selection
    fn select_round_robin(&self) -> Option<Arc<ProviderWithStats>> {
        let index = self.round_robin_index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            % self.providers.len();
        self.providers.get(index).cloned()
    }

    /// Least connections selection
    fn select_least_connections(&self) -> Option<Arc<ProviderWithStats>> {
        self.providers
            .iter()
            .min_by_key(|p| p.stats().total_requests)
            .cloned()
    }

    /// Weighted round-robin selection
    fn select_weighted_round_robin(&self) -> Option<Arc<ProviderWithStats>> {
        if self.config.weights.is_empty() {
            return self.select_round_robin();
        }

        // Calculate total weight
        let total_weight: u32 = self.providers
            .iter()
            .filter_map(|p| self.config.weights.get(p.provider().id()))
            .sum();

        if total_weight == 0 {
            return self.select_round_robin();
        }

        // Select random value
        let mut random_value = rand::random::<u32>() % total_weight;
        let mut current_weight = 0u32;

        // Find provider based on weight
        for provider in &self.providers {
            if let Some(&weight) = self.config.weights.get(provider.provider().id()) {
                current_weight += weight;
                if random_value < current_weight {
                    return Some(provider.clone());
                }
            }
        }

        // Fallback to round-robin
        self.select_round_robin()
    }

    /// Random selection
    fn select_random(&self) -> Option<Arc<ProviderWithStats>> {
        if self.providers.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..self.providers.len());
        self.providers.get(index).cloned()
    }

    /// Get the number of providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Check if load balancer has any providers
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Get configuration
    pub fn config(&self) -> &LoadBalancerConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: LoadBalancerConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_balancer_creation() {
        let lb = LoadBalancer::default();
        assert_eq!(lb.provider_count(), 0);
        assert!(lb.is_empty());
    }

    #[test]
    fn test_load_balancer_strategy() {
        let config = LoadBalancerConfig {
            strategy: LoadBalancingStrategy::Random,
            ..Default::default()
        };
        let lb = LoadBalancer::new(config);
        assert_eq!(lb.config().strategy, LoadBalancingStrategy::Random);
    }
}
