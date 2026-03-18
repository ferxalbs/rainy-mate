use crate::ai::specs::{PromptSkillBinding, PromptSkillScope};
use super::parser::parse_prompt_skill;
use super::registry::{
    now_ts, DiscoveredPromptSkill, PromptSkillRegistry, PromptSkillSourceKind,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

const ROOT_FILES: &[&str] = &["SKILL.md"];
const PROJECT_DIRS: &[&str] = &[
    "skills",
    "skills/.curated",
    "skills/.experimental",
    "skills/.system",
    ".agents/skills",
    ".claude/skills",
    ".codex/skills",
    ".cursor/skills",
    ".continue/skills",
    ".augment/skills",
    ".kilocode/skills",
    ".kiro/skills",
    ".qwen/skills",
    ".roo/skills",
    ".goose/skills",
    ".rainy-mate/skills",
];
const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
];

pub struct PromptSkillDiscoveryService {
    app_data_dir: PathBuf,
}

impl PromptSkillDiscoveryService {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }

    pub fn discover(&self, workspace_path: Option<&Path>) -> Result<Vec<DiscoveredPromptSkill>, String> {
        let global_entries = PromptSkillRegistry::global(&self.app_data_dir)?.get_entries()?;
        let project_entries = if let Some(workspace_path) = workspace_path {
            PromptSkillRegistry::project(workspace_path)?.get_entries()?
        } else {
            BTreeMap::new().into_iter().collect()
        };

        let mut seen = HashSet::new();
        let mut skills = Vec::new();

        if let Some(workspace_path) = workspace_path {
            for path in collect_workspace_candidates(workspace_path)? {
                if let Some(skill) = self.load_candidate(
                    &path,
                    workspace_scope(&path, workspace_path),
                    PromptSkillSourceKind::Direct,
                    &project_entries,
                    &mut seen,
                ) {
                    skills.push(skill);
                }
            }

            for path in collect_plugin_declared_candidates(workspace_path)? {
                if let Some(skill) = self.load_candidate(
                    &path,
                    workspace_scope(&path, workspace_path),
                    PromptSkillSourceKind::PluginManifest,
                    &project_entries,
                    &mut seen,
                ) {
                    skills.push(skill);
                }
            }
        }

        for path in collect_global_candidates(&self.app_data_dir)? {
            if let Some(skill) = self.load_candidate(
                &path,
                global_scope(&path, &self.app_data_dir),
                PromptSkillSourceKind::Direct,
                &global_entries,
                &mut seen,
            ) {
                skills.push(skill);
            }
        }

        skills.sort_by(|a, b| a.name.cmp(&b.name).then(a.source_path.cmp(&b.source_path)));
        Ok(skills)
    }

    pub fn refresh_binding(
        &self,
        workspace_path: Option<&Path>,
        source_path: &Path,
    ) -> Result<PromptSkillBinding, String> {
        let scope = if let Some(workspace_path) = workspace_path {
            if source_path.starts_with(workspace_path.join(".rainy-mate")) {
                PromptSkillScope::MateManaged
            } else if source_path.starts_with(workspace_path) {
                PromptSkillScope::Project
            } else {
                global_scope(source_path, &self.app_data_dir)
            }
        } else {
            global_scope(source_path, &self.app_data_dir)
        };

        let parsed = parse_prompt_skill(source_path)?;
        Ok(DiscoveredPromptSkill {
            id: parsed.id,
            name: parsed.name,
            description: parsed.description,
            body_markdown: parsed.body_markdown,
            source_path: parsed.source_path.to_string_lossy().to_string(),
            scope,
            source_kind: PromptSkillSourceKind::Direct,
            source_hash: parsed.source_hash,
            discovered_at: now_ts(),
            valid: true,
            parse_error: None,
            scripts: parsed.scripts,
            references: parsed.references,
            all_agents_enabled: false,
        }
        .to_binding())
    }

    fn load_candidate(
        &self,
        path: &Path,
        scope: PromptSkillScope,
        source_kind: PromptSkillSourceKind,
        registry_entries: &std::collections::HashMap<String, super::registry::PromptSkillRegistryEntry>,
        seen: &mut HashSet<String>,
    ) -> Option<DiscoveredPromptSkill> {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let key = canonical.to_string_lossy().to_string();
        if !seen.insert(key.clone()) {
            return None;
        }

        match parse_prompt_skill(&canonical) {
            Ok(parsed) => Some(DiscoveredPromptSkill {
                id: parsed.id,
                name: parsed.name,
                description: parsed.description,
                body_markdown: parsed.body_markdown,
                source_path: parsed.source_path.to_string_lossy().to_string(),
                scope,
                source_kind,
                source_hash: parsed.source_hash,
                discovered_at: now_ts(),
                valid: true,
                parse_error: None,
                scripts: parsed.scripts,
                references: parsed.references,
                all_agents_enabled: registry_entries
                    .get(&key)
                    .map(|entry| entry.all_agents_enabled)
                    .unwrap_or(false),
            }),
            Err(error) => Some(DiscoveredPromptSkill {
                id: format!("invalid-{}", short_hash(&key)),
                name: canonical
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Invalid Skill")
                    .to_string(),
                description: String::new(),
                body_markdown: String::new(),
                source_path: key.clone(),
                scope,
                source_kind,
                source_hash: short_hash(&key),
                discovered_at: now_ts(),
                valid: false,
                parse_error: Some(error),
                scripts: Vec::new(),
                references: Vec::new(),
                all_agents_enabled: false,
            }),
        }
    }
}

fn collect_workspace_candidates(workspace_path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut candidates = Vec::new();
    for root_file in ROOT_FILES {
        let path = workspace_path.join(root_file);
        if path.exists() {
            candidates.push(workspace_path.to_path_buf());
        }
    }
    for dir in PROJECT_DIRS {
        let path = workspace_path.join(dir);
        candidates.extend(scan_skill_dirs(&path, 4)?);
    }
    if candidates.is_empty() {
        candidates.extend(scan_skill_dirs(workspace_path, 6)?);
    }
    Ok(candidates)
}

fn collect_global_candidates(app_data_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut candidates = Vec::new();
    if let Some(home_dir) = dirs::home_dir() {
        for dir in PROJECT_DIRS {
            candidates.extend(scan_skill_dirs(&home_dir.join(dir), 4)?);
        }
    }
    candidates.extend(scan_skill_dirs(&app_data_dir.join("prompt_skills").join("managed"), 4)?);
    Ok(candidates)
}

fn collect_plugin_declared_candidates(workspace_path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for rel in [
        ".claude-plugin/marketplace.json",
        ".claude-plugin/plugin.json",
    ] {
        let path = workspace_path.join(rel);
        if !path.exists() {
            continue;
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let value: Value = serde_json::from_str(&raw)
            .map_err(|e| format!("Invalid plugin manifest {}: {}", path.display(), e))?;
        collect_paths_from_json(&value, path.parent().unwrap_or(workspace_path), &mut out);
    }
    Ok(out)
}

fn collect_paths_from_json(value: &Value, base_dir: &Path, out: &mut Vec<PathBuf>) {
    match value {
        Value::String(raw) => {
            if raw.ends_with("SKILL.md") {
                let path = base_dir.join(raw);
                if let Some(parent) = path.parent() {
                    out.push(parent.to_path_buf());
                }
            } else if raw.contains("skills/") || raw.ends_with("/skills") || raw.ends_with("\\skills") {
                out.extend(scan_skill_dirs(&base_dir.join(raw), 4).unwrap_or_default());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_paths_from_json(item, base_dir, out);
            }
        }
        Value::Object(map) => {
            for value in map.values() {
                collect_paths_from_json(value, base_dir, out);
            }
        }
        _ => {}
    }
}

fn scan_skill_dirs(root: &Path, max_depth: usize) -> Result<Vec<PathBuf>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    if root.is_file() && root.file_name().and_then(|v| v.to_str()) == Some("SKILL.md") {
        if let Some(parent) = root.parent() {
            out.push(parent.to_path_buf());
        }
        return Ok(out);
    }
    for entry in WalkDir::new(root)
        .follow_links(true)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(keep_entry)
    {
        let entry = entry.map_err(|e| format!("Failed to walk {}: {}", root.display(), e))?;
        if entry.file_type().is_file() && entry.file_name().to_string_lossy() == "SKILL.md" {
            if let Some(parent) = entry.path().parent() {
                out.push(parent.to_path_buf());
            }
        }
    }
    Ok(out)
}

fn keep_entry(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    !IGNORED_DIR_NAMES.iter().any(|ignored| *ignored == name)
}

fn workspace_scope(path: &Path, workspace_path: &Path) -> PromptSkillScope {
    if path.starts_with(workspace_path.join(".rainy-mate")) {
        PromptSkillScope::MateManaged
    } else {
        PromptSkillScope::Project
    }
}

fn global_scope(path: &Path, app_data_dir: &Path) -> PromptSkillScope {
    if path.starts_with(app_data_dir.join("prompt_skills").join("managed")) {
        PromptSkillScope::MateManaged
    } else {
        PromptSkillScope::Global
    }
}

fn short_hash(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_project_skill() {
        let app_data = tempfile::tempdir().expect("app data");
        let workspace = tempfile::tempdir().expect("workspace");
        let skill_dir = workspace.path().join(".agents/skills/reviewer");
        std::fs::create_dir_all(&skill_dir).expect("mkdir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: Reviewer\ndescription: Reviews code\n---\nUse rg.",
        )
        .expect("write");

        let service = PromptSkillDiscoveryService::new(app_data.path().to_path_buf());
        let skills = service.discover(Some(workspace.path())).expect("discover");
        assert!(skills.iter().any(|skill| skill.name == "Reviewer"));
    }
}
