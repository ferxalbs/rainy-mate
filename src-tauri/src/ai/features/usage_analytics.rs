// Rainy Cowork - Usage Analytics Feature (PHASE 3)
// Usage tracking and analytics for AI providers

use crate::ai::provider_types::{ProviderId, TokenUsage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Usage analytics service
pub struct UsageAnalytics {
    /// Usage data per provider
    usage_data: Arc<RwLock<HashMap<ProviderId, ProviderUsage>>>,
}

impl UsageAnalytics {
    /// Create a new usage analytics service
    pub fn new() -> Self {
        Self {
            usage_data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record usage for a provider
    pub async fn record_usage(&self, provider_id: &ProviderId, usage: TokenUsage) {
        let mut data = self.usage_data.write().await;
        let provider_usage = data.entry(provider_id.clone()).or_insert_with(ProviderUsage::default);

        provider_usage.total_requests += 1;
        provider_usage.total_tokens += usage.total_tokens as u64;
        provider_usage.prompt_tokens += usage.prompt_tokens as u64;
        provider_usage.completion_tokens += usage.completion_tokens as u64;
        provider_usage.last_used = Some(chrono::Utc::now());
    }

    /// Get usage for a specific provider
    pub async fn get_usage(&self, provider_id: &ProviderId) -> Option<ProviderUsage> {
        let data = self.usage_data.read().await;
        data.get(provider_id).cloned()
    }

    /// Get all usage data
    pub async fn get_all_usage(&self) -> HashMap<ProviderId, ProviderUsage> {
        let data = self.usage_data.read().await;
        data.clone()
    }

    /// Get total usage across all providers
    pub async fn get_total_usage(&self) -> TotalUsage {
        let data = self.usage_data.read().await;
        let mut total = TotalUsage::default();

        for usage in data.values() {
            total.total_requests += usage.total_requests;
            total.total_tokens += usage.total_tokens;
            total.prompt_tokens += usage.prompt_tokens;
            total.completion_tokens += usage.completion_tokens;
        }

        total
    }

    /// Reset usage for a provider
    pub async fn reset_usage(&self, provider_id: &ProviderId) {
        let mut data = self.usage_data.write().await;
        data.insert(provider_id.clone(), ProviderUsage::default());
    }

    /// Reset all usage data
    pub async fn reset_all_usage(&self) {
        let mut data = self.usage_data.write().await;
        data.clear();
    }

    /// Get usage statistics
    pub async fn get_statistics(&self) -> UsageStatistics {
        let data = self.usage_data.read().await;
        let mut stats = UsageStatistics::default();

        for (provider_id, usage) in data.iter() {
            stats.provider_count += 1;
            stats.total_requests += usage.total_requests;
            stats.total_tokens += usage.total_tokens;

            if usage.total_requests > stats.max_requests {
                stats.max_requests = usage.total_requests;
                stats.most_used_provider = Some(provider_id.clone());
            }

            if usage.total_tokens > stats.max_tokens {
                stats.max_tokens = usage.total_tokens;
                stats.highest_token_provider = Some(provider_id.clone());
            }
        }

        stats
    }
}

impl Default for UsageAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

/// Provider usage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    /// Total requests made
    pub total_requests: u64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Prompt tokens used
    pub prompt_tokens: u64,
    /// Completion tokens used
    pub completion_tokens: u64,
    /// Last used timestamp
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for ProviderUsage {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_tokens: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            last_used: None,
        }
    }
}

/// Total usage across all providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalUsage {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

impl Default for TotalUsage {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_tokens: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
        }
    }
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStatistics {
    /// Number of providers with usage data
    pub provider_count: usize,
    /// Total requests across all providers
    pub total_requests: u64,
    /// Total tokens across all providers
    pub total_tokens: u64,
    /// Maximum requests by a single provider
    pub max_requests: u64,
    /// Most used provider
    pub most_used_provider: Option<ProviderId>,
    /// Maximum tokens by a single provider
    pub max_tokens: u64,
    /// Provider with highest token usage
    pub highest_token_provider: Option<ProviderId>,
}

impl Default for UsageStatistics {
    fn default() -> Self {
        Self {
            provider_count: 0,
            total_requests: 0,
            total_tokens: 0,
            max_requests: 0,
            most_used_provider: None,
            max_tokens: 0,
            highest_token_provider: None,
        }
    }
}

/// Usage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    pub statistics: UsageStatistics,
    pub total_usage: TotalUsage,
    pub provider_usage: HashMap<String, ProviderUsage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_usage_default() {
        let usage = ProviderUsage::default();
        assert_eq!(usage.total_requests, 0);
        assert_eq!(usage.total_tokens, 0);
        assert!(usage.last_used.is_none());
    }

    #[test]
    fn test_total_usage_default() {
        let total = TotalUsage::default();
        assert_eq!(total.total_requests, 0);
        assert_eq!(total.total_tokens, 0);
    }

    #[test]
    fn test_usage_statistics_default() {
        let stats = UsageStatistics::default();
        assert_eq!(stats.provider_count, 0);
        assert_eq!(stats.total_requests, 0);
        assert!(stats.most_used_provider.is_none());
    }

    #[tokio::test]
    async fn test_record_usage() {
        let analytics = UsageAnalytics::new();
        let provider_id = ProviderId::new("test-provider");

        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };

        analytics.record_usage(&provider_id, usage).await;

        let recorded = analytics.get_usage(&provider_id).await;
        assert!(recorded.is_some());
        assert_eq!(recorded.unwrap().total_requests, 1);
        assert_eq!(recorded.unwrap().total_tokens, 150);
    }

    #[tokio::test]
    async fn test_reset_usage() {
        let analytics = UsageAnalytics::new();
        let provider_id = ProviderId::new("test-provider");

        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };

        analytics.record_usage(&provider_id, usage).await;
        analytics.reset_usage(&provider_id).await;

        let recorded = analytics.get_usage(&provider_id).await;
        assert!(recorded.is_some());
        assert_eq!(recorded.unwrap().total_requests, 0);
        assert_eq!(recorded.unwrap().total_tokens, 0);
    }
}
