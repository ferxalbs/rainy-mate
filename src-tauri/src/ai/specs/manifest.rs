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
    pub retention_days: u32,
    pub max_tokens: u32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            strategy: "hybrid".to_string(),
            retention_days: 30,
            max_tokens: 32000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectorsConfig {
    pub telegram_enabled: bool,
    pub telegram_channel_id: Option<String>,
    pub auto_reply: bool,
}
