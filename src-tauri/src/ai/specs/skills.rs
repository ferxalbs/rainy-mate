use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromptSkillScope {
    Project,
    Global,
    MateManaged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromptSkillKind {
    PromptSkill,
    WorkspaceInstruction,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptSkillBinding {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub source_path: String,
    pub scope: PromptSkillScope,
    #[serde(default)]
    pub kind: PromptSkillKind,
    pub source_hash: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub last_synced_at: i64,
}

impl Default for PromptSkillScope {
    fn default() -> Self {
        Self::Project
    }
}

impl Default for PromptSkillKind {
    fn default() -> Self {
        Self::PromptSkill
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentSkills {
    // v3 fields
    #[serde(default)]
    pub workflows: Vec<SkillWorkflow>,
    #[serde(default)]
    pub tool_preferences: Vec<ToolPreference>,
    #[serde(default)]
    pub behaviors: Vec<SkillBehavior>,
    #[serde(default)]
    pub prompt_skills: Vec<PromptSkillBinding>,

    // v2 fields kept for backward-compat deserialization of old on-disk specs
    // skipped on serialization when empty so new specs don't emit them
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<Capability>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tools: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillWorkflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger: String,
    pub steps: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolPreference {
    pub tool_name: String,
    pub priority: String, // "prefer" | "avoid" | "never"
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillBehavior {
    pub id: String,
    pub name: String,
    pub instruction: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String, // e.g., "filesystem", "browser"
    pub description: String,
    pub scopes: Vec<String>, // e.g., "/Users/fer/Documents", "*.google.com"
    pub permissions: Vec<Permission>,
}
