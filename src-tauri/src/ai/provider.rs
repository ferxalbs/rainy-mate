// Rainy Cowork - AI Provider Trait and Manager
// Abstraction layer for multiple AI providers

use crate::ai::{gemini::GeminiProvider, keychain::KeychainManager, rainy_api::RainyApiProvider};
use crate::models::{AIProviderConfig, ProviderType};

/// Error type for AI operations
#[derive(Debug, thiserror::Error)]
pub enum AIError {
    #[error("API request failed: {0}")]
    RequestFailed(String),
    #[error("Invalid API key")]
    InvalidApiKey,
    #[error("Rate limited")]
    RateLimited,
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Provider not available: {0}")]
    ProviderNotAvailable(String),
}

/// Manager for AI providers
pub struct AIProviderManager {
    keychain: KeychainManager,
    rainy_api: RainyApiProvider,
    gemini: GeminiProvider,
}

impl AIProviderManager {
    pub fn new() -> Self {
        Self {
            keychain: KeychainManager::new(),
            rainy_api: RainyApiProvider::new(),
            gemini: GeminiProvider::new(),
        }
    }

    /// List available providers
    pub async fn list_providers(&self) -> Vec<AIProviderConfig> {
        vec![
            AIProviderConfig {
                provider: ProviderType::RainyApi,
                name: "Rainy API".to_string(),
                model: "gpt-4o".to_string(),
                is_available: true,
                requires_api_key: true,
            },
            AIProviderConfig {
                provider: ProviderType::Gemini,
                name: "Google Gemini".to_string(),
                model: "gemini-3-pro-preview".to_string(),
                is_available: true,
                requires_api_key: true,
            },
        ]
    }

    /// Validate an API key for a provider
    pub async fn validate_api_key(&self, provider: &str, api_key: &str) -> Result<bool, String> {
        match provider {
            "rainy_api" => self
                .rainy_api
                .validate_key(api_key)
                .await
                .map_err(|e| e.to_string()),
            "gemini" => self
                .gemini
                .validate_key(api_key)
                .await
                .map_err(|e| e.to_string()),
            _ => Err(AIError::ProviderNotAvailable(provider.to_string()).to_string()),
        }
    }

    /// Store API key in macOS Keychain
    pub async fn store_api_key(&self, provider: &str, api_key: &str) -> Result<(), String> {
        self.keychain.store_key(provider, api_key)
    }

    /// Get API key from macOS Keychain
    pub async fn get_api_key(&self, provider: &str) -> Result<Option<String>, String> {
        self.keychain.get_key(provider)
    }

    /// Delete API key from macOS Keychain
    pub async fn delete_api_key(&self, provider: &str) -> Result<(), String> {
        self.keychain.delete_key(provider)
    }

    /// Check if API key exists for a provider
    pub async fn has_api_key(&self, provider: &str) -> Result<bool, String> {
        Ok(self.keychain.has_key(provider))
    }

    /// Get available models for a provider
    pub async fn get_models(&self, provider: &str) -> Result<Vec<String>, String> {
        match provider {
            "rainy_api" => Ok(self.rainy_api.available_models()),
            "gemini" => Ok(self.gemini.available_models()),
            _ => Err(AIError::ProviderNotAvailable(provider.to_string()).to_string()),
        }
    }

    /// Check if model is valid for provider
    fn is_valid_model(&self, provider: &ProviderType, model: &str) -> bool {
        let models = match provider {
            ProviderType::RainyApi => self.rainy_api.available_models(),
            ProviderType::Gemini => self.gemini.available_models(),
        };
        models.iter().any(|m| m == model)
    }

    /// Execute a prompt using the specified provider
    pub async fn execute_prompt<F>(
        &self,
        provider: &ProviderType,
        model: &str,
        prompt: &str,
        on_progress: F,
    ) -> Result<String, String>
    where
        F: Fn(u8, Option<String>) + Send + Sync + 'static,
    {
        // Validate model exists for provider
        if !self.is_valid_model(provider, model) {
            return Err(
                AIError::ModelNotFound(format!("{} not found for {:?}", model, provider))
                    .to_string(),
            );
        }

        let provider_name = match provider {
            ProviderType::RainyApi => "rainy_api",
            ProviderType::Gemini => "gemini",
        };

        // Get API key from keychain
        let api_key = self.get_api_key(provider_name).await?.ok_or_else(|| {
            AIError::ProviderNotAvailable(format!("No API key configured for {}", provider_name))
                .to_string()
        })?;

        match provider {
            ProviderType::RainyApi => self
                .rainy_api
                .complete_with_api_key(model, prompt, &api_key, on_progress)
                .await
                .map_err(|e| e.to_string()),
            ProviderType::Gemini => self
                .gemini
                .complete_with_api_key(model, prompt, &api_key, on_progress)
                .await
                .map_err(|e| e.to_string()),
        }
    }
}

impl Default for AIProviderManager {
    fn default() -> Self {
        Self::new()
    }
}
