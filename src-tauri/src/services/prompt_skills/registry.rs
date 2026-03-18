use crate::ai::specs::{PromptSkillBinding, PromptSkillKind, PromptSkillScope};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromptSkillSourceKind {
    Direct,
    PluginManifest,
    InstructionFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredPromptSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub body_markdown: String,
    pub source_path: String,
    pub scope: PromptSkillScope,
    pub kind: PromptSkillKind,
    pub source_kind: PromptSkillSourceKind,
    pub source_hash: String,
    pub discovered_at: i64,
    pub valid: bool,
    pub parse_error: Option<String>,
    pub scripts: Vec<String>,
    pub references: Vec<String>,
    pub all_agents_enabled: bool,
}

impl DiscoveredPromptSkill {
    pub fn to_binding(&self) -> PromptSkillBinding {
        PromptSkillBinding {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            content: self.body_markdown.clone(),
            source_path: self.source_path.clone(),
            scope: self.scope.clone(),
            kind: self.kind.clone(),
            source_hash: self.source_hash.clone(),
            enabled: true,
            last_synced_at: now_ts(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PromptSkillRegistryEntry {
    pub source_path: String,
    #[serde(default)]
    pub all_agents_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PromptSkillRegistryFile {
    #[serde(default)]
    entries: Vec<PromptSkillRegistryEntry>,
}

pub struct PromptSkillRegistry {
    path: PathBuf,
}

impl PromptSkillRegistry {
    pub fn global(app_data_dir: &Path) -> Result<Self, String> {
        let root = app_data_dir.join("prompt_skills");
        fs::create_dir_all(&root).map_err(|e| format!("Failed to create prompt skill dir: {}", e))?;
        Ok(Self {
            path: root.join("registry.json"),
        })
    }

    pub fn project(workspace_path: &Path) -> Result<Self, String> {
        let root = workspace_path.join(".rainy-mate").join("registry");
        fs::create_dir_all(&root).map_err(|e| format!("Failed to create project prompt skill dir: {}", e))?;
        Ok(Self {
            path: root.join("prompt-skills.json"),
        })
    }

    fn load(&self) -> Result<PromptSkillRegistryFile, String> {
        if !self.path.exists() {
            return Ok(PromptSkillRegistryFile::default());
        }
        let raw = fs::read_to_string(&self.path)
            .map_err(|e| format!("Failed to read prompt skill registry: {}", e))?;
        serde_json::from_str(&raw).map_err(|e| format!("Invalid prompt skill registry json: {}", e))
    }

    fn save(&self, file: &PromptSkillRegistryFile) -> Result<(), String> {
        let raw = serde_json::to_string_pretty(file)
            .map_err(|e| format!("Failed to serialize prompt skill registry: {}", e))?;
        fs::write(&self.path, raw).map_err(|e| format!("Failed to write prompt skill registry: {}", e))
    }

    pub fn get_entries(&self) -> Result<HashMap<String, PromptSkillRegistryEntry>, String> {
        let file = self.load()?;
        Ok(file
            .entries
            .into_iter()
            .map(|entry| (entry.source_path.clone(), entry))
            .collect())
    }

    pub fn set_all_agents_enabled(
        &self,
        source_path: &str,
        enabled: bool,
    ) -> Result<(), String> {
        let mut file = self.load()?;
        if let Some(entry) = file.entries.iter_mut().find(|entry| entry.source_path == source_path) {
            entry.all_agents_enabled = enabled;
        } else {
            file.entries.push(PromptSkillRegistryEntry {
                source_path: source_path.to_string(),
                all_agents_enabled: enabled,
            });
        }
        file.entries.sort_by(|a, b| a.source_path.cmp(&b.source_path));
        self.save(&file)
    }
}

pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}
