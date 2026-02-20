// Rainy Cowork - User Settings Service
// Manages user preferences including AI model selection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Available AI model for selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    pub id: String,
    pub name: String,
    pub description: String,
    pub thinking_level: String,
    pub is_premium: bool,
    pub is_available: bool,
    pub provider: String,
}

/// User settings persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct UserSettings {
    pub selected_model: String,
    pub theme: String,
    pub notifications_enabled: bool,
    pub profile: UserProfile,
    pub auto_reconnect_cloud: bool,
    pub tool_policy_version_floor: HashMap<String, u64>,
    pub embedder_provider: String,
    pub embedder_model: String,
}

/// User profile metadata for desktop personalization and cloud identity sync
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct UserProfile {
    pub display_name: String,
    pub email: String,
    pub organization: String,
    pub role: String,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            selected_model: "gemini-3-flash-high".to_string(),
            theme: "system".to_string(),
            notifications_enabled: true,
            profile: UserProfile::default(),
            auto_reconnect_cloud: true,
            tool_policy_version_floor: HashMap::new(),
            embedder_provider: "gemini".to_string(),
            embedder_model: "gemini-embedding-001".to_string(),
        }
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            display_name: "Rainy User".to_string(),
            email: "".to_string(),
            organization: "".to_string(),
            role: "Builder".to_string(),
        }
    }
}

/// Settings manager for persistence and retrieval
pub struct SettingsManager {
    settings_path: PathBuf,
    settings: UserSettings,
}

impl SettingsManager {
    pub fn new() -> Self {
        let settings_path = Self::get_settings_path();
        let settings = Self::load_from_disk(&settings_path);

        Self {
            settings_path,
            settings,
        }
    }

    fn get_settings_path() -> PathBuf {
        let app_data = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.enosislabs.rainy-cowork");

        // Ensure directory exists
        fs::create_dir_all(&app_data).ok();

        app_data.join("settings.json")
    }

    fn load_from_disk(path: &PathBuf) -> UserSettings {
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(path) {
                if let Ok(mut settings) = serde_json::from_str::<UserSettings>(&contents) {
                    if settings.embedder_provider == "gemini" {
                        match settings.embedder_model.as_str() {
                            "text-embedding-004"
                            | "embedding-001"
                            | "embedding-gecko-001"
                            | "gemini-embedding-exp"
                            | "gemini-embedding-exp-03-07" => {
                                settings.embedder_model = "gemini-embedding-001".to_string();
                            }
                            _ => {}
                        }
                    }
                    return settings;
                }
            }
        }
        UserSettings::default()
    }

    fn save_to_disk(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(&self.settings_path, json).map_err(|e| format!("Failed to save settings: {}", e))
    }

    /// Get current user settings
    pub fn get_settings(&self) -> &UserSettings {
        &self.settings
    }

    /// Get selected model
    pub fn get_selected_model(&self) -> &str {
        &self.settings.selected_model
    }

    /// Set selected model and persist
    pub fn set_selected_model(&mut self, model: String) -> Result<(), String> {
        self.settings.selected_model = model;
        self.save_to_disk()
    }

    /// Set theme and persist
    pub fn set_theme(&mut self, theme: String) -> Result<(), String> {
        self.settings.theme = theme;
        self.save_to_disk()
    }

    /// Set notifications and persist
    pub fn set_notifications(&mut self, enabled: bool) -> Result<(), String> {
        self.settings.notifications_enabled = enabled;
        self.save_to_disk()
    }

    /// Get user profile
    pub fn get_profile(&self) -> &UserProfile {
        &self.settings.profile
    }

    /// Set user profile and persist
    pub fn set_profile(&mut self, profile: UserProfile) -> Result<(), String> {
        self.settings.profile = profile;
        self.save_to_disk()
    }

    /// Get embedder provider
    pub fn get_embedder_provider(&self) -> &str {
        &self.settings.embedder_provider
    }

    /// Set embedder provider and persist
    pub fn set_embedder_provider(&mut self, provider: String) -> Result<(), String> {
        self.settings.embedder_provider = provider;
        self.save_to_disk()
    }

    /// Get embedder model
    pub fn get_embedder_model(&self) -> &str {
        &self.settings.embedder_model
    }

    /// Set embedder model and persist
    pub fn set_embedder_model(&mut self, model: String) -> Result<(), String> {
        self.settings.embedder_model = model;
        self.save_to_disk()
    }

    /// Get the persisted minimum accepted tool policy version for a workspace.
    pub fn get_tool_policy_floor(&self, workspace_id: &str) -> u64 {
        self.settings
            .tool_policy_version_floor
            .get(workspace_id)
            .copied()
            .unwrap_or(0)
    }

    /// Persist the minimum accepted tool policy version for a workspace.
    pub fn set_tool_policy_floor(
        &mut self,
        workspace_id: &str,
        version: u64,
    ) -> Result<(), String> {
        self.settings
            .tool_policy_version_floor
            .insert(workspace_id.to_string(), version);
        self.save_to_disk()
    }

    /// Get available models
    pub fn get_available_models() -> Vec<ModelOption> {
        let mut models = vec![];

        // Rainy API models (static list from rainy-sdk - pay-as-you-go)
        // Note: Gemini BYOK is handled separately when user selects gemini: prefix
        // These are ALL available models via Rainy API

        // OpenAI models
        for (id, name, desc) in [
            ("gpt-4o", "GPT-4o", "OpenAI's flagship multimodal model"),
            ("gpt-5", "GPT-5", "OpenAI's most advanced reasoning model"),
            (
                "gpt-5-pro",
                "GPT-5 Pro",
                "Maximum capability for complex tasks",
            ),
            ("o3", "O3", "OpenAI's reasoning-focused model"),
            ("o4-mini", "O4 Mini", "Fast and efficient reasoning"),
        ] {
            models.push(ModelOption {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
                thinking_level: "n/a".to_string(),
                is_premium: true,
                is_available: true,
                provider: "Rainy API".to_string(),
            });
        }

        // Anthropic models
        for (id, name, desc) in [
            (
                "claude-sonnet-4",
                "Claude Sonnet 4",
                "Anthropic's balanced model",
            ),
            (
                "claude-opus-4-1",
                "Claude Opus 4.1",
                "Anthropic's most capable model",
            ),
        ] {
            models.push(ModelOption {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
                thinking_level: "n/a".to_string(),
                is_premium: true,
                is_available: true,
                provider: "Rainy API".to_string(),
            });
        }

        // Google Gemini 2.5 models
        for (id, name, desc) in [
            (
                "gemini-2.5-pro",
                "Gemini 2.5 Pro",
                "Google's most capable model",
            ),
            (
                "gemini-2.5-flash",
                "Gemini 2.5 Flash",
                "Fast multimodal responses",
            ),
            (
                "gemini-2.5-flash-lite",
                "Gemini 2.5 Flash Lite",
                "Lightweight cost-effective AI",
            ),
        ] {
            models.push(ModelOption {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
                thinking_level: "n/a".to_string(),
                is_premium: true,
                is_available: true,
                provider: "Rainy API".to_string(),
            });
        }

        // Google Gemini 3 Flash models with thinking levels (minimal, low, medium, high)
        for (id, name, desc, thinking) in [
            (
                "gemini-3-flash-minimal",
                "Gemini 3 Flash (Minimal)",
                "Fastest responses with minimal thinking",
                "minimal",
            ),
            (
                "gemini-3-flash-low",
                "Gemini 3 Flash (Low)",
                "Fast responses with light thinking",
                "low",
            ),
            (
                "gemini-3-flash-medium",
                "Gemini 3 Flash (Medium)",
                "Balanced speed and reasoning",
                "medium",
            ),
            (
                "gemini-3-flash-high",
                "Gemini 3 Flash (High)",
                "Deep reasoning for complex tasks",
                "high",
            ),
            // Also add the base preview model
            (
                "gemini-3-flash-preview",
                "Gemini 3 Flash",
                "Fast thinking with reasoning",
                "medium",
            ),
        ] {
            models.push(ModelOption {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
                thinking_level: thinking.to_string(),
                is_premium: true,
                is_available: true,
                provider: "Rainy API".to_string(),
            });
        }

        // Google Gemini 3 Pro models with thinking levels (low, high)
        for (id, name, desc, thinking) in [
            (
                "gemini-3-pro-low",
                "Gemini 3 Pro (Low)",
                "Advanced reasoning with faster responses",
                "low",
            ),
            (
                "gemini-3-pro-high",
                "Gemini 3 Pro (High)",
                "Maximum reasoning capabilities",
                "high",
            ),
            // Also add the base preview model
            (
                "gemini-3-pro-preview",
                "Gemini 3 Pro",
                "Advanced reasoning model",
                "medium",
            ),
        ] {
            models.push(ModelOption {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
                thinking_level: thinking.to_string(),
                is_premium: true,
                is_available: true,
                provider: "Rainy API".to_string(),
            });
        }

        // Other Gemini 3 models
        models.push(ModelOption {
            id: "gemini-3-pro-image-preview".to_string(),
            name: "Gemini 3 Pro Image".to_string(),
            description: "Multimodal reasoning".to_string(),
            thinking_level: "n/a".to_string(),
            is_premium: true,
            is_available: true,
            provider: "Rainy API".to_string(),
        });

        // Groq models
        for (id, name, desc) in [
            (
                "llama-3.1-8b-instant",
                "Llama 3.1 8B Instant",
                "Meta's fast open-source model",
            ),
            (
                "llama-3.3-70b-versatile",
                "Llama 3.3 70B Versatile",
                "Meta's versatile model",
            ),
            (
                "moonshotai/kimi-k2-instruct-0905",
                "Kimi K2",
                "Moonshot AI's instruction model",
            ),
        ] {
            models.push(ModelOption {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
                thinking_level: "n/a".to_string(),
                is_premium: true,
                is_available: true,
                provider: "Rainy API".to_string(),
            });
        }

        // Cerebras models
        models.push(ModelOption {
            id: "cerebras/llama3.1-8b".to_string(),
            name: "Cerebras Llama 3.1 8B".to_string(),
            description: "Cerebras-accelerated Llama".to_string(),
            thinking_level: "n/a".to_string(),
            is_premium: true,
            is_available: true,
            provider: "Rainy API".to_string(),
        });

        // Enosis Labs models
        for (id, name, desc) in [
            ("astronomer-1", "Astronomer 1", "Enosis Labs base model"),
            (
                "astronomer-1-max",
                "Astronomer 1 Max",
                "Enhanced Astronomer model",
            ),
            ("astronomer-1.5", "Astronomer 1.5", "Improved Astronomer"),
            ("astronomer-2", "Astronomer 2", "Next-gen Astronomer"),
            (
                "astronomer-2-pro",
                "Astronomer 2 Pro",
                "Enosis Labs flagship model",
            ),
        ] {
            models.push(ModelOption {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
                thinking_level: "n/a".to_string(),
                is_premium: true,
                is_available: true,
                provider: "Rainy API".to_string(),
            });
        }

        models
    }

    /// Reserved for future Rainy API model info display
    #[allow(dead_code)]
    fn get_model_info(model_id: &str) -> (String, String) {
        match model_id {
            "gpt-4o" => (
                "GPT-4o".to_string(),
                "OpenAI's flagship multimodal model".to_string(),
            ),
            "gpt-5" => (
                "GPT-5".to_string(),
                "OpenAI's most advanced reasoning model".to_string(),
            ),
            "gpt-5-pro" => (
                "GPT-5 Pro".to_string(),
                "Maximum capability for complex tasks".to_string(),
            ),
            "o3" => (
                "O3".to_string(),
                "OpenAI's reasoning-focused model".to_string(),
            ),
            "o4-mini" => (
                "O4 Mini".to_string(),
                "Fast and efficient reasoning".to_string(),
            ),
            "claude-sonnet-4" => (
                "Claude Sonnet 4".to_string(),
                "Anthropic's balanced model".to_string(),
            ),
            "claude-opus-4-1" => (
                "Claude Opus 4.1".to_string(),
                "Anthropic's most capable model".to_string(),
            ),
            "gemini-2.5-pro" => (
                "Gemini 2.5 Pro".to_string(),
                "Google's most capable model".to_string(),
            ),
            "gemini-2.5-flash" => (
                "Gemini 2.5 Flash".to_string(),
                "Fast multimodal responses".to_string(),
            ),
            _ => (model_id.to_string(), "Premium AI model".to_string()),
        }
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}
