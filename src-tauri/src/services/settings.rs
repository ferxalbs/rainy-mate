// Rainy Cowork - User Settings Service
// Manages user preferences including AI model selection

use crate::ai::model_catalog::{
    ensure_supported_model_slug, find_catalog_model, ModelProvider,
};
use crate::ai::provider::AIProviderManager;
use crate::models::neural::ToolAccessPolicy;
use crate::services::mcp_service::{McpPermissionMode, PersistedMcpServerConfig};
use rainy_sdk::models::{CapabilityFlag, ModelCatalogItem};
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
    #[serde(default)]
    pub workspace_tool_access_policies: HashMap<String, WorkspaceToolPolicyState>,
    pub embedder_provider: String,
    pub embedder_model: String,
    #[serde(default)]
    pub mcp_permission_mode: McpPermissionMode,
    #[serde(default)]
    pub mcp_servers: Vec<PersistedMcpServerConfig>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceToolPolicyState {
    pub tool_access_policy: ToolAccessPolicy,
    pub tool_access_policy_version: u64,
    pub tool_access_policy_hash: String,
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
            workspace_tool_access_policies: HashMap::new(),
            embedder_provider: "gemini".to_string(),
            embedder_model: crate::services::memory_vault::types::EMBEDDING_MODEL.to_string(),
            mcp_permission_mode: McpPermissionMode::Ask,
            mcp_servers: Vec::new(),
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
    fn capability_flag_enabled(flag: Option<&CapabilityFlag>) -> bool {
        matches!(flag, Some(CapabilityFlag::Bool(true)))
    }

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
                            "gemini-embedding-2-preview" | "gemini-embedding-001" => {}
                            "text-embedding-004"
                            | "embedding-001"
                            | "embedding-gecko-001"
                            | "gemini-embedding-exp"
                            | "gemini-embedding-exp-03-07" => {
                                settings.embedder_model = "gemini-embedding-001".to_string();
                            }
                            _ => {
                                settings.embedder_model =
                                    crate::services::memory_vault::types::EMBEDDING_MODEL
                                        .to_string();
                            }
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

    pub fn get_workspace_tool_policy_state(
        &self,
        workspace_id: &str,
    ) -> Option<WorkspaceToolPolicyState> {
        self.settings
            .workspace_tool_access_policies
            .get(workspace_id)
            .cloned()
    }

    pub fn get_workspace_tool_policy(&self, workspace_id: &str) -> Option<ToolAccessPolicy> {
        self.get_workspace_tool_policy_state(workspace_id)
            .map(|state| state.tool_access_policy)
    }

    pub fn set_workspace_tool_policy_state(
        &mut self,
        workspace_id: &str,
        state: WorkspaceToolPolicyState,
    ) -> Result<(), String> {
        self.settings
            .workspace_tool_access_policies
            .insert(workspace_id.to_string(), state);
        self.save_to_disk()
    }

    pub fn get_mcp_permission_mode(&self) -> McpPermissionMode {
        self.settings.mcp_permission_mode.clone()
    }

    pub fn set_mcp_permission_mode(&mut self, mode: McpPermissionMode) -> Result<(), String> {
        self.settings.mcp_permission_mode = mode;
        self.save_to_disk()
    }

    pub fn get_mcp_servers(&mut self) -> Vec<PersistedMcpServerConfig> {
        self.settings.mcp_servers.clone()
    }

    pub fn upsert_mcp_server(&mut self, config: PersistedMcpServerConfig) -> Result<(), String> {
        if let Some(existing) = self
            .settings
            .mcp_servers
            .iter_mut()
            .find(|s| s.name.eq_ignore_ascii_case(&config.name))
        {
            *existing = config;
        } else {
            self.settings.mcp_servers.push(config);
        }
        self.save_to_disk()
    }

    pub fn remove_mcp_server(&mut self, name: &str) -> Result<(), String> {
        let before = self.settings.mcp_servers.len();
        self.settings
            .mcp_servers
            .retain(|s| !s.name.eq_ignore_ascii_case(name));
        if self.settings.mcp_servers.len() == before {
            return Err(format!("MCP server '{}' not found", name));
        }
        self.save_to_disk()
    }

    fn dynamic_model_option(slug: &str) -> ModelOption {
        if let Some(entry) = find_catalog_model(slug, ModelProvider::RainyApi) {
            return ModelOption {
                id: entry.slug.to_string(),
                name: entry.name.to_string(),
                description: entry.description.to_string(),
                thinking_level: entry.thinking_level.unwrap_or("n/a").to_string(),
                is_premium: true,
                is_available: true,
                provider: "Rainy API".to_string(),
            };
        }

        ModelOption {
            id: slug.to_string(),
            name: slug.to_string(),
            description: "Discovered dynamically from Rainy API v3.".to_string(),
            thinking_level: "n/a".to_string(),
            is_premium: true,
            is_available: true,
            provider: "Rainy API".to_string(),
        }
    }

    fn dynamic_model_option_from_catalog(item: &ModelCatalogItem) -> ModelOption {
        if let Some(entry) = find_catalog_model(&item.id, ModelProvider::RainyApi) {
            return ModelOption {
                id: entry.slug.to_string(),
                name: entry.name.to_string(),
                description: entry.description.to_string(),
                thinking_level: entry.thinking_level.unwrap_or("n/a").to_string(),
                is_premium: true,
                is_available: true,
                provider: "Rainy API".to_string(),
            };
        }

        let caps = item.rainy_capabilities.as_ref();
        let thinking_level =
            if Self::capability_flag_enabled(caps.and_then(|caps| caps.reasoning.as_ref())) {
                "dynamic"
            } else {
                "n/a"
            };

        ModelOption {
            id: item.id.clone(),
            name: item.name.clone().unwrap_or_else(|| item.id.clone()),
            description: "Discovered dynamically from Rainy API v3.".to_string(),
            thinking_level: thinking_level.to_string(),
            is_premium: true,
            is_available: true,
            provider: "Rainy API".to_string(),
        }
    }

    /// Get available models
    pub async fn get_available_models(
        provider_manager: Option<&AIProviderManager>,
    ) -> Vec<ModelOption> {
        let mut models = Vec::new();

        if let Some(provider_manager) = provider_manager {
            if let Ok(catalog) = provider_manager.get_models_catalog("rainy_api").await {
                models.extend(catalog.iter().map(Self::dynamic_model_option_from_catalog));
            } else if let Ok(dynamic_models) = provider_manager.get_models("rainy_api").await {
                models.extend(dynamic_models.iter().map(|slug| Self::dynamic_model_option(slug)));
            }
        }

        models.sort_by(|a, b| a.name.cmp(&b.name));
        models.dedup_by(|a, b| a.id == b.id);
        models
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}
