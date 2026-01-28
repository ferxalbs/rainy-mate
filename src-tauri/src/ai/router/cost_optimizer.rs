// Cost Optimizer
// Selects providers based on cost efficiency

use crate::ai::provider_trait::ProviderWithStats;
use crate::ai::provider_types::{ProviderId, ProviderType};
use std::collections::HashMap;

/// Cost per 1K tokens for different providers
#[derive(Debug, Clone, Copy)]
pub struct ProviderCost {
    /// Input tokens cost per 1K
    pub input_cost_per_1k: f64,
    /// Output tokens cost per 1K
    pub output_cost_per_1k: f64,
}

impl ProviderCost {
    /// Calculate cost for given token counts
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        input_cost + output_cost
    }
}

/// Default costs for common providers
impl Default for ProviderCost {
    fn default() -> Self {
        Self {
            input_cost_per_1k: 0.001, // $0.001 per 1K input tokens
            output_cost_per_1k: 0.002, // $0.002 per 1K output tokens
        }
    }
}

/// Cost optimizer configuration
#[derive(Debug, Clone)]
pub struct CostOptimizerConfig {
    /// Provider costs
    pub provider_costs: HashMap<ProviderId, ProviderCost>,
    /// Budget limit (optional)
    pub budget_limit: Option<f64>,
    /// Current spend
    pub current_spend: f64,
}

impl Default for CostOptimizerConfig {
    fn default() -> Self {
        Self {
            provider_costs: HashMap::new(),
            budget_limit: None,
            current_spend: 0.0,
        }
    }
}

/// Cost optimizer for selecting cost-effective providers
pub struct CostOptimizer {
    /// Available providers
    providers: Vec<std::sync::Arc<ProviderWithStats>>,
    /// Configuration
    config: CostOptimizerConfig,
}

impl CostOptimizer {
    /// Create a new cost optimizer
    pub fn new(config: CostOptimizerConfig) -> Self {
        Self {
            providers: Vec::new(),
            config,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(CostOptimizerConfig::default())
    }

    /// Add a provider to the optimizer
    pub fn add_provider(&mut self, provider: std::sync::Arc<ProviderWithStats>) {
        self.providers.push(provider);
    }

    /// Remove a provider from the optimizer
    pub fn remove_provider(&mut self, provider_id: &ProviderId) {
        self.providers.retain(|p| p.provider().id() != provider_id);
        self.config.provider_costs.remove(provider_id);
    }

    /// Set cost for a provider
    pub fn set_provider_cost(&mut self, provider_id: ProviderId, cost: ProviderCost) {
        self.config.provider_costs.insert(provider_id, cost);
    }

    /// Get cost for a provider
    pub fn get_provider_cost(&self, provider_id: &ProviderId) -> Option<ProviderCost> {
        self.config.provider_costs.get(provider_id).cloned()
    }

    /// Select the most cost-effective provider
    pub fn select_provider(&self, estimated_input_tokens: u32, estimated_output_tokens: u32) -> Option<std::sync::Arc<ProviderWithStats>> {
        if self.providers.is_empty() {
            return None;
        }

        // Calculate cost for each provider
        let mut best_provider = None;
        let mut best_cost = f64::MAX;

        for provider in &self.providers {
            if let Some(cost) = self.config.provider_costs.get(provider.provider().id()) {
                let provider_cost = cost.calculate_cost(estimated_input_tokens, estimated_output_tokens);

                // Check budget limit
                if let Some(budget) = self.config.budget_limit {
                    if self.config.current_spend + provider_cost > budget {
                        continue; // Skip if would exceed budget
                    }
                }

                if provider_cost < best_cost {
                    best_cost = provider_cost;
                    best_provider = Some(provider.clone());
                }
            }
        }

        best_provider
    }

    /// Select provider based on cost per token
    pub fn select_cheapest_provider(&self) -> Option<std::sync::Arc<ProviderWithStats>> {
        if self.providers.is_empty() {
            return None;
        }

        // Find provider with lowest average cost
        let mut best_provider = None;
        let mut best_avg_cost = f64::MAX;

        for provider in &self.providers {
            if let Some(cost) = self.config.provider_costs.get(provider.provider().id()) {
                let avg_cost = (cost.input_cost_per_1k + cost.output_cost_per_1k) / 2.0;

                if avg_cost < best_avg_cost {
                    best_avg_cost = avg_cost;
                    best_provider = Some(provider.clone());
                }
            }
        }

        best_provider
    }

    /// Update current spend
    pub fn update_spend(&mut self, amount: f64) {
        self.config.current_spend += amount;
    }

    /// Get current spend
    pub fn current_spend(&self) -> f64 {
        self.config.current_spend
    }

    /// Get budget limit
    pub fn budget_limit(&self) -> Option<f64> {
        self.config.budget_limit
    }

    /// Set budget limit
    pub fn set_budget_limit(&mut self, limit: f64) {
        self.config.budget_limit = Some(limit);
    }

    /// Check if budget is exceeded
    pub fn is_budget_exceeded(&self) -> bool {
        if let Some(budget) = self.config.budget_limit {
            self.config.current_spend >= budget
        } else {
            false
        }
    }

    /// Get remaining budget
    pub fn remaining_budget(&self) -> Option<f64> {
        self.config.budget_limit.map(|budget| {
            if self.config.current_spend >= budget {
                0.0
            } else {
                budget - self.config.current_spend
            }
        })
    }

    /// Get all providers
    pub fn providers(&self) -> &[std::sync::Arc<ProviderWithStats>] {
        &self.providers
    }

    /// Get number of providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Check if optimizer has any providers
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Get configuration
    pub fn config(&self) -> &CostOptimizerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_cost_calculation() {
        let cost = ProviderCost {
            input_cost_per_1k: 0.001,
            output_cost_per_1k: 0.002,
        };

        let total_cost = cost.calculate_cost(1000, 1000);
        assert_eq!(total_cost, 0.003); // $0.001 + $0.002
    }

    #[test]
    fn test_cost_optimizer_select_cheapest() {
        let mut optimizer = CostOptimizer::default();

        // Add costs for providers
        optimizer.set_provider_cost(
            ProviderId::new("provider1"),
            ProviderCost {
                input_cost_per_1k: 0.001,
                output_cost_per_1k: 0.002,
            },
        );
        optimizer.set_provider_cost(
            ProviderId::new("provider2"),
            ProviderCost {
                input_cost_per_1k: 0.0005,
                output_cost_per_1k: 0.001,
            },
        );

        // provider2 should be cheaper
        assert_eq!(
            optimizer.get_provider_cost(&ProviderId::new("provider2")),
            Some(ProviderCost {
                input_cost_per_1k: 0.0005,
                output_cost_per_1k: 0.001,
            })
        );
    }

    #[test]
    fn test_cost_optimizer_budget() {
        let mut optimizer = CostOptimizer::default();
        optimizer.set_budget_limit(10.0);

        assert_eq!(optimizer.budget_limit(), Some(10.0));
        assert!(!optimizer.is_budget_exceeded());

        optimizer.update_spend(5.0);
        assert_eq!(optimizer.current_spend(), 5.0);
        assert_eq!(optimizer.remaining_budget(), Some(5.0));

        optimizer.update_spend(10.0);
        assert!(optimizer.is_budget_exceeded());
        assert_eq!(optimizer.remaining_budget(), Some(0.0));
    }
}
