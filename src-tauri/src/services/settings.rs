// Rainy Cowork - User Settings Service
// Manages user preferences including AI model selection

use crate::ai::model_catalog::{
    all_catalog_models, ensure_supported_model_slug, ModelProvider,
};
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
            selected_model: "gemini-3-flash-preview".to_string(),
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
        ensure_supported_model_slug(&model)?;
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
        all_catalog_models()
            .into_iter()
            .filter(|entry| matches!(entry.provider, ModelProvider::RainyApi))
            .map(|entry| ModelOption {
                id: entry.slug.to_string(),
                name: entry.name.to_string(),
                description: entry.description.to_string(),
                thinking_level: entry.thinking_level.unwrap_or("n/a").to_string(),
                is_premium: true,
                is_available: true,
                provider: "Rainy API".to_string(),
            })
            .collect()
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}
