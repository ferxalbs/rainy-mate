// Capability Matcher
// Matches requests to providers based on required capabilities

use crate::ai::provider_trait::ProviderWithStats;
use crate::ai::provider_types::{ProviderId, ProviderCapabilities};
use std::collections::HashSet;

/// Required capabilities for a request
#[derive(Debug, Clone, Default)]
pub struct RequiredCapabilities {
    /// Chat completions required
    pub chat_completions: bool,
    /// Embeddings required
    pub embeddings: bool,
    /// Streaming required
    pub streaming: bool,
    /// Function calling required
    pub function_calling: bool,
    /// Vision/image analysis required
    pub vision: bool,
    /// Web search required
    pub web_search: bool,
    /// Minimum context window required
    pub min_context_tokens: Option<u32>,
    /// Minimum output tokens required
    pub min_output_tokens: Option<u32>,
    /// Required models (if any)
    pub required_models: Option<HashSet<String>>,
}

impl RequiredCapabilities {
    /// Create new required capabilities
    pub fn new() -> Self {
        Self::default()
    }

    /// Require chat completions
    pub fn require_chat_completions(mut self) -> Self {
        self.chat_completions = true;
        self
    }

    /// Require embeddings
    pub fn require_embeddings(mut self) -> Self {
        self.embeddings = true;
        self
    }

    /// Require streaming
    pub fn require_streaming(mut self) -> Self {
        self.streaming = true;
        self
    }

    /// Require function calling
    pub fn require_function_calling(mut self) -> Self {
        self.function_calling = true;
        self
    }

    /// Require vision
    pub fn require_vision(mut self) -> Self {
        self.vision = true;
        self
    }

    /// Require web search
    pub fn require_web_search(mut self) -> Self {
        self.web_search = true;
        self
    }

    /// Set minimum context tokens
    pub fn min_context_tokens(mut self, tokens: u32) -> Self {
        self.min_context_tokens = Some(tokens);
        self
    }

    /// Set minimum output tokens
    pub fn min_output_tokens(mut self, tokens: u32) -> Self {
        self.min_output_tokens = Some(tokens);
        self
    }

    /// Require specific models
    pub fn require_models(mut self, models: Vec<String>) -> Self {
        self.required_models = Some(models.into_iter().collect());
        self
    }

    /// Check if provider matches required capabilities
    pub fn matches(&self, provider_caps: &ProviderCapabilities) -> bool {
        // Check basic capabilities
        if self.chat_completions && !provider_caps.chat_completions {
            return false;
        }

        if self.embeddings && !provider_caps.embeddings {
            return false;
        }

        if self.streaming && !provider_caps.streaming {
            return false;
        }

        if self.function_calling && !provider_caps.function_calling {
            return false;
        }

        if self.vision && !provider_caps.vision {
            return false;
        }

        if self.web_search && !provider_caps.web_search {
            return false;
        }

        // Check context window
        if let Some(min_context) = self.min_context_tokens {
            if provider_caps.max_context_tokens < min_context {
                return false;
            }
        }

        // Check output tokens
        if let Some(min_output) = self.min_output_tokens {
            if provider_caps.max_output_tokens < min_output {
                return false;
            }
        }

        // Check required models
        if let Some(required_models) = &self.required_models {
            if !required_models.is_empty() {
                let has_required_model = provider_caps.models.iter()
                    .any(|model| required_models.contains(model));
                if !has_required_model {
                    return false;
                }
            }
        }

        true
    }
}

/// Capability matcher configuration
#[derive(Debug, Clone, Default)]
pub struct CapabilityMatcherConfig {
    /// Prefer providers with more capabilities
    pub prefer_more_capable: bool,
    /// Score weights for matching
    pub weights: CapabilityWeights,
}

/// Weights for capability scoring
#[derive(Debug, Clone, Copy)]
pub struct CapabilityWeights {
    /// Weight for chat completions
    pub chat_completions: f32,
    /// Weight for embeddings
    pub embeddings: f32,
    /// Weight for streaming
    pub streaming: f32,
    /// Weight for function calling
    pub function_calling: f32,
    /// Weight for vision
    pub vision: f32,
    /// Weight for web search
    pub web_search: f32,
    /// Weight for context window
    pub context_window: f32,
    /// Weight for output tokens
    pub output_tokens: f32,
}

impl Default for CapabilityWeights {
    fn default() -> Self {
        Self {
            chat_completions: 1.0,
            embeddings: 1.0,
            streaming: 1.0,
            function_calling: 1.0,
            vision: 1.0,
            web_search: 1.0,
            context_window: 0.001, // Lower weight for large numbers
            output_tokens: 0.001, // Lower weight for large numbers
        }
    }
}

/// Capability matcher for selecting providers based on capabilities
pub struct CapabilityMatcher {
    /// Available providers
    providers: Vec<std::sync::Arc<ProviderWithStats>>,
    /// Configuration
    config: CapabilityMatcherConfig,
}

impl CapabilityMatcher {
    /// Create a new capability matcher
    pub fn new(config: CapabilityMatcherConfig) -> Self {
        Self {
            providers: Vec::new(),
            config,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(CapabilityMatcherConfig::default())
    }

    /// Add a provider to the matcher
    pub fn add_provider(&mut self, provider: std::sync::Arc<ProviderWithStats>) {
        self.providers.push(provider);
    }

    /// Remove a provider from the matcher
    pub fn remove_provider(&mut self, provider_id: &ProviderId) {
        self.providers.retain(|p| p.provider().id() != provider_id);
    }

    /// Find providers that match required capabilities
    pub async fn find_matching_providers(
        &self,
        required: &RequiredCapabilities,
    ) -> Vec<std::sync::Arc<ProviderWithStats>> {
        let mut matching = Vec::new();

        for provider in &self.providers {
            let caps_result = provider.provider().capabilities().await;
            if let Ok(caps) = caps_result {
                if required.matches(&caps) {
                    matching.push(provider.clone());
                }
            }
        }

        matching
    }

    /// Select the best matching provider
    pub async fn select_best_provider(
        &self,
        required: &RequiredCapabilities,
    ) -> Option<std::sync::Arc<ProviderWithStats>> {
        let matching = self.find_matching_providers(required).await;

        if matching.is_empty() {
            return None;
        }

        if matching.len() == 1 {
            return Some(matching[0].clone());
        }

        // Score each provider
        let mut best_provider = None;
        let mut best_score = f32::MIN;

        for provider in &matching {
            let score = self.score_provider(provider, required).await;
            if score > best_score {
                best_score = score;
                best_provider = Some(provider.clone());
            }
        }

        best_provider
    }

    /// Score a provider based on capabilities
    async fn score_provider(
        &self,
        provider: &std::sync::Arc<ProviderWithStats>,
        required: &RequiredCapabilities,
    ) -> f32 {
        let caps_result = provider.provider().capabilities().await;
        let weights = &self.config.weights;

        let mut score = 0.0;

        // Handle Result type
        let caps = match caps_result {
            Ok(c) => c,
            Err(_) => return 0.0, // Return low score for providers with errors
        };

        // Score based on capabilities
        if caps.chat_completions {
            score += weights.chat_completions;
        }

        if caps.embeddings {
            score += weights.embeddings;
        }

        if caps.streaming {
            score += weights.streaming;
        }

        if caps.function_calling {
            score += weights.function_calling;
        }

        if caps.vision {
            score += weights.vision;
        }

        if caps.web_search {
            score += weights.web_search;
        }

        // Score based on context window
        score += (caps.max_context_tokens as f32) * weights.context_window;

        // Score based on output tokens
        score += (caps.max_output_tokens as f32) * weights.output_tokens;

        // Bonus for having more capabilities
        if self.config.prefer_more_capable {
            let capability_count = [
                caps.chat_completions,
                caps.embeddings,
                caps.streaming,
                caps.function_calling,
                caps.vision,
                caps.web_search,
            ]
            .iter()
            .filter(|&&c| c)
            .count() as f32;

            score += capability_count * 0.5;
        }

        score
    }

    /// Get all providers
    pub fn providers(&self) -> &[std::sync::Arc<ProviderWithStats>] {
        &self.providers
    }

    /// Get number of providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Check if matcher has any providers
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Get configuration
    pub fn config(&self) -> &CapabilityMatcherConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: CapabilityMatcherConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_capabilities_builder() {
        let caps = RequiredCapabilities::new()
            .require_chat_completions()
            .require_streaming()
            .min_context_tokens(8000);

        assert!(caps.chat_completions);
        assert!(caps.streaming);
        assert_eq!(caps.min_context_tokens, Some(8000));
    }

    #[test]
    fn test_required_capabilities_matches() {
        let required = RequiredCapabilities::new()
            .require_chat_completions()
            .require_streaming();

        let provider_caps = ProviderCapabilities {
            chat_completions: true,
            streaming: true,
            ..Default::default()
        };

        assert!(required.matches(&provider_caps));

        let provider_caps_no_streaming = ProviderCapabilities {
            chat_completions: true,
            streaming: false,
            ..Default::default()
        };

        assert!(!required.matches(&provider_caps_no_streaming));
    }

    #[test]
    fn test_required_capabilities_models() {
        let required = RequiredCapabilities::new()
            .require_models(vec!["gpt-4o".to_string(), "claude-3.5-sonnet".to_string()]);

        let provider_caps = ProviderCapabilities {
            models: vec!["gpt-4o".to_string(), "gemini-2.5-pro".to_string()],
            ..Default::default()
        };

        assert!(required.matches(&provider_caps));

        let provider_caps_no_match = ProviderCapabilities {
            models: vec!["gemini-2.5-pro".to_string()],
            ..Default::default()
        };

        assert!(!required.matches(&provider_caps_no_match));
    }
}
