use super::security::AgentSignature;
use super::skills::AgentSkills;
use super::soul::AgentSoul;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub id: String,
    pub version: String, // "2.0.0"

    pub soul: AgentSoul,
    pub skills: AgentSkills,

    #[serde(default)]
    pub memory_config: MemoryConfig,

    #[serde(default)]
    pub connectors: ConnectorsConfig,

    // Security layer - REQUIRED for v2
    pub signature: Option<AgentSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub strategy: String, // "vector", "simple_buffer", "hybrid"

    #[serde(default)]
    pub retrieval: RetrievalConfig,

    #[serde(default)]
    pub persistence: PersistenceConfig,

    #[serde(default)]
    pub knowledge: KnowledgeConfig,

    // Backward compat: accept flat fields from old specs on disk
    #[serde(default)]
    pub retention_days: Option<u32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    pub retention_days: u32,
    pub max_tokens: u32,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            retention_days: 30,
            max_tokens: 32000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    pub cross_session: bool,
    pub per_connector_isolation: bool,
    pub session_scope: String, // "per_user" | "per_channel" | "global"
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            cross_session: true,
            per_connector_isolation: false,
            session_scope: "global".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeConfig {
    pub enabled: bool,
    #[serde(default)]
    pub indexed_files: Vec<KnowledgeFile>,
}

impl Default for KnowledgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            indexed_files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeFile {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub indexed_at: u64,
    pub chunk_count: u32,
}

impl MemoryConfig {
    /// Get effective retention_days (nested takes priority over flat legacy)
    pub fn effective_retention_days(&self) -> u32 {
        self.retention_days.unwrap_or(self.retrieval.retention_days)
    }

    /// Get effective max_tokens (nested takes priority over flat legacy)
    pub fn effective_max_tokens(&self) -> u32 {
        self.max_tokens.unwrap_or(self.retrieval.max_tokens)
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            strategy: "hybrid".to_string(),
            retrieval: RetrievalConfig::default(),
            persistence: PersistenceConfig::default(),
            knowledge: KnowledgeConfig::default(),
            retention_days: None,
            max_tokens: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectorsConfig {
    pub telegram_enabled: bool,
    pub telegram_channel_id: Option<String>,
    pub auto_reply: bool,
}
