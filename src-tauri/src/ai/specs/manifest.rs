use super::security::AgentSignature;
use super::skills::AgentSkills;
use super::soul::AgentSoul;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub id: String,
    pub version: String, // "3.0.0"

    pub soul: AgentSoul,
    pub skills: AgentSkills,

    #[serde(default)]
    pub airlock: AirlockConfig,

    #[serde(default)]
    pub memory_config: MemoryConfig,

    #[serde(default)]
    pub connectors: ConnectorsConfig,

    #[serde(default)]
    pub runtime: RuntimeConfig,

    // v3 runtime fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(default, rename = "maxTokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    // Security layer
    pub signature: Option<AgentSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMode {
    #[default]
    Single,
    Supervisor,
    HierarchicalSupervisor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfig {
    #[serde(default)]
    pub mode: RuntimeMode,
    #[serde(default = "default_max_specialists")]
    pub max_specialists: u8,
    #[serde(default = "default_verification_required")]
    pub verification_required: bool,
    #[serde(default)]
    pub delegation: DelegationConfig,
    #[serde(default)]
    pub language_policy: LanguagePolicyConfig,
}

fn default_max_specialists() -> u8 {
    3
}

fn default_verification_required() -> bool {
    true
}

fn default_max_depth() -> u8 {
    2
}

fn default_max_threads() -> u8 {
    4
}

fn default_job_max_runtime_seconds() -> u32 {
    900
}

fn default_internal_coordination_language() -> String {
    "english".to_string()
}

fn default_final_response_language_mode() -> String {
    "user".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationConfig {
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
    #[serde(default = "default_max_threads")]
    pub max_threads: u8,
    #[serde(default = "default_max_specialists")]
    pub max_parallel_subagents: u8,
    #[serde(default = "default_job_max_runtime_seconds")]
    pub job_max_runtime_seconds: u32,
    #[serde(default)]
    pub final_synthesis_required: bool,
}

impl Default for DelegationConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            max_threads: default_max_threads(),
            max_parallel_subagents: default_max_specialists(),
            job_max_runtime_seconds: default_job_max_runtime_seconds(),
            final_synthesis_required: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePolicyConfig {
    #[serde(default = "default_internal_coordination_language")]
    pub internal_coordination_language: String,
    #[serde(default = "default_final_response_language_mode")]
    pub final_response_language_mode: String,
}

impl Default for LanguagePolicyConfig {
    fn default() -> Self {
        Self {
            internal_coordination_language: default_internal_coordination_language(),
            final_response_language_mode: default_final_response_language_mode(),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            mode: RuntimeMode::Single,
            max_specialists: default_max_specialists(),
            verification_required: default_verification_required(),
            delegation: DelegationConfig::default(),
            language_policy: LanguagePolicyConfig::default(),
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Airlock — tool permissions, scopes, and rate limits
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirlockConfig {
    #[serde(default)]
    pub tool_policy: AirlockToolPolicy,

    #[serde(default)]
    pub tool_levels: HashMap<String, u8>,

    #[serde(default)]
    pub scopes: AirlockScopes,

    #[serde(default)]
    pub rate_limits: AirlockRateLimits,
}

impl Default for AirlockConfig {
    fn default() -> Self {
        Self {
            tool_policy: AirlockToolPolicy::default(),
            tool_levels: HashMap::new(),
            scopes: AirlockScopes::default(),
            rate_limits: AirlockRateLimits::default(),
        }
    }
}

impl AirlockConfig {
    /// Check whether a tool is permitted by this airlock's policy.
    /// Used by both ThinkStep (tool advertising) and generate_system_prompt (dynamic capability display)
    /// so the LLM _sees_ exactly the tools it will _get_.
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        let policy = &self.tool_policy;
        if policy.deny.iter().any(|item| item == tool_name) {
            return false;
        }
        if policy.mode == "allowlist" {
            return policy.allow.iter().any(|item| item == tool_name);
        }
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirlockToolPolicy {
    #[serde(default = "default_policy_mode")]
    pub mode: String, // "all" | "allowlist"
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
}

fn default_policy_mode() -> String {
    "all".to_string()
}

impl Default for AirlockToolPolicy {
    fn default() -> Self {
        Self {
            mode: "all".to_string(),
            allow: Vec::new(),
            deny: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirlockScopes {
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub blocked_paths: Vec<String>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub blocked_domains: Vec<String>,
}

impl Default for AirlockScopes {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            blocked_paths: Vec::new(),
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AirlockRateLimits {
    #[serde(default)]
    pub max_requests_per_minute: u32,
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
