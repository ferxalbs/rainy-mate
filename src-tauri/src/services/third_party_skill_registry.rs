use crate::ai::provider_types::{FunctionDefinition, Tool};
use crate::models::neural::{AirlockLevel, ParameterSchema, SkillManifest, SkillMethod};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPermissionFs {
    pub guest_path: String,
    pub host_path: String,
    pub mode: String, // read | read_write
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillPermissions {
    #[serde(default)]
    pub filesystem: Vec<SkillPermissionFs>,
    #[serde(default)]
    pub network_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledThirdPartyMethod {
    pub name: String,
    pub description: String,
    pub airlock_level: AirlockLevel,
    pub parameters: HashMap<String, ParameterSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledThirdPartySkill {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub runtime: String,
    pub binary_path: String,
    pub binary_sha256: String,
    pub enabled: bool,
    pub trust_state: String, // verified | unsigned_dev
    pub install_source: String, // atm | local_dev
    pub installed_at: i64,
    #[serde(default)]
    pub permissions: SkillPermissions,
    #[serde(default)]
    pub methods: Vec<InstalledThirdPartyMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RegistryFile {
    #[serde(default)]
    skills: Vec<InstalledThirdPartySkill>,
}

pub struct ThirdPartySkillRegistry {
    root_dir: PathBuf,
    index_path: PathBuf,
}

impl ThirdPartySkillRegistry {
    pub fn new() -> Result<Self, String> {
        let data_dir = crate::services::app_identity::resolve_child_dir(
            dirs::data_dir().ok_or_else(|| "Could not resolve data directory".to_string())?,
            "third_party_skills",
        )?;
        Self::new_with_root(data_dir)
    }

    pub(crate) fn new_with_root(root_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root_dir)
            .map_err(|e| format!("Failed to create third-party skill dir: {}", e))?;
        let index_path = root_dir.join("registry.json");
        Ok(Self { root_dir, index_path })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn load_index(&self) -> Result<RegistryFile, String> {
        if !self.index_path.exists() {
            return Ok(RegistryFile::default());
        }
        let body = fs::read_to_string(&self.index_path)
            .map_err(|e| format!("Failed to read registry index: {}", e))?;
        serde_json::from_str(&body).map_err(|e| format!("Invalid registry index json: {}", e))
    }

    fn save_index(&self, file: &RegistryFile) -> Result<(), String> {
        let body = serde_json::to_string_pretty(file)
            .map_err(|e| format!("Failed to serialize registry index: {}", e))?;
        fs::write(&self.index_path, body).map_err(|e| format!("Failed to write registry index: {}", e))
    }

    pub fn list_skills(&self) -> Result<Vec<InstalledThirdPartySkill>, String> {
        let mut skills = self.load_index()?.skills;
        skills.sort_by(|a, b| a.id.cmp(&b.id).then(a.version.cmp(&b.version)));
        Ok(skills)
    }

    pub fn upsert_skill(&self, skill: InstalledThirdPartySkill) -> Result<(), String> {
        let mut file = self.load_index()?;
        if file
            .skills
            .iter()
            .any(|s| s.methods.iter().any(|m| skill.methods.iter().any(|nm| nm.name == m.name)) && s.id != skill.id)
        {
            return Err("Method name collision with another installed third-party skill".to_string());
        }

        if let Some(existing) = file
            .skills
            .iter_mut()
            .find(|s| s.id == skill.id && s.version == skill.version)
        {
            *existing = skill;
        } else {
            file.skills.push(skill);
        }
        self.save_index(&file)
    }

    pub fn set_enabled(&self, skill_id: &str, version: &str, enabled: bool) -> Result<(), String> {
        let mut file = self.load_index()?;
        let Some(skill) = file
            .skills
            .iter_mut()
            .find(|s| s.id == skill_id && s.version == version) else {
            return Err(format!("Skill {}@{} not found", skill_id, version));
        };
        skill.enabled = enabled;
        self.save_index(&file)
    }

    pub fn remove(&self, skill_id: &str, version: &str) -> Result<(), String> {
        let mut file = self.load_index()?;
        let before = file.skills.len();
        file.skills.retain(|s| !(s.id == skill_id && s.version == version));
        if file.skills.len() == before {
            return Err(format!("Skill {}@{} not found", skill_id, version));
        }
        self.save_index(&file)
    }

    pub fn resolve_method(
        &self,
        skill_id: &str,
        method: &str,
    ) -> Result<Option<(InstalledThirdPartySkill, InstalledThirdPartyMethod)>, String> {
        let file = self.load_index()?;
        for skill in file.skills {
            if skill.id != skill_id || !skill.enabled {
                continue;
            }
            if let Some(method_def) = skill.methods.iter().find(|m| m.name == method).cloned() {
                return Ok(Some((skill, method_def)));
            }
        }
        Ok(None)
    }

    pub fn find_method_airlock_level(&self, method: &str) -> Result<Option<AirlockLevel>, String> {
        let file = self.load_index()?;
        let mut found: Option<AirlockLevel> = None;
        for skill in file.skills.into_iter().filter(|s| s.enabled) {
            if let Some(m) = skill.methods.iter().find(|m| m.name == method) {
                // Collisions are prevented at install time; keep fail-closed if somehow duplicated.
                if found.is_some() {
                    return Ok(Some(AirlockLevel::Dangerous));
                }
                found = Some(m.airlock_level);
            }
        }
        Ok(found)
    }

    pub fn dynamic_skill_manifests(&self) -> Result<Vec<SkillManifest>, String> {
        let mut manifests = Vec::new();
        for skill in self.list_skills()?.into_iter().filter(|s| s.enabled) {
            manifests.push(SkillManifest {
                name: skill.id,
                version: skill.version,
                methods: skill
                    .methods
                    .into_iter()
                    .map(|m| SkillMethod {
                        name: m.name,
                        description: m.description,
                        airlock_level: m.airlock_level,
                        parameters: m.parameters,
                    })
                    .collect(),
            });
        }
        Ok(manifests)
    }

    #[allow(dead_code)] // @RESERVED for provider-side dynamic tool exposure parity
    pub fn dynamic_tool_definitions(&self) -> Result<Vec<Tool>, String> {
        let mut tools = Vec::new();
        for skill in self.list_skills()?.into_iter().filter(|s| s.enabled) {
            for method in skill.methods {
                let mut required = Vec::new();
                let mut properties = serde_json::Map::new();
                for (name, schema) in method.parameters {
                    if schema.required.unwrap_or(false) {
                        required.push(name.clone());
                    }
                    properties.insert(
                        name,
                        serde_json::json!({
                            "type": schema.param_type,
                            "description": schema.description,
                        }),
                    );
                }
                let schema = serde_json::json!({
                    "type": "object",
                    "properties": properties,
                    "required": required,
                });
                tools.push(Tool {
                    r#type: "function".to_string(),
                    function: FunctionDefinition {
                        name: method.name,
                        description: format!("[{}] {}", skill.id, method.description),
                        parameters: schema,
                    },
                });
            }
        }
        Ok(tools)
    }

    pub fn now_ts() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::neural::AirlockLevel;

    fn method(name: &str, level: AirlockLevel) -> InstalledThirdPartyMethod {
        InstalledThirdPartyMethod {
            name: name.to_string(),
            description: "test".to_string(),
            airlock_level: level,
            parameters: HashMap::new(),
        }
    }

    fn skill(id: &str, version: &str, methods: Vec<InstalledThirdPartyMethod>) -> InstalledThirdPartySkill {
        InstalledThirdPartySkill {
            id: id.to_string(),
            name: id.to_string(),
            version: version.to_string(),
            author: "test".to_string(),
            description: String::new(),
            runtime: "wasi-core-v1".to_string(),
            binary_path: "/tmp/test.wasm".to_string(),
            binary_sha256: "deadbeef".to_string(),
            enabled: true,
            trust_state: "verified".to_string(),
            install_source: "atm".to_string(),
            installed_at: 0,
            permissions: SkillPermissions::default(),
            methods,
        }
    }

    #[test]
    fn registry_resolves_method_airlock_level() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = ThirdPartySkillRegistry::new_with_root(temp.path().to_path_buf()).expect("registry");
        registry
            .upsert_skill(skill("alpha", "1.0.0", vec![method("alpha_run", AirlockLevel::Sensitive)]))
            .expect("insert");

        let level = registry
            .find_method_airlock_level("alpha_run")
            .expect("lookup");
        assert_eq!(level, Some(AirlockLevel::Sensitive));
    }

    #[test]
    fn registry_rejects_method_name_collisions_across_skills() {
        let temp = tempfile::tempdir().expect("tempdir");
        let registry = ThirdPartySkillRegistry::new_with_root(temp.path().to_path_buf()).expect("registry");
        registry
            .upsert_skill(skill("alpha", "1.0.0", vec![method("shared_method", AirlockLevel::Safe)]))
            .expect("insert alpha");

        let err = registry
            .upsert_skill(skill("beta", "1.0.0", vec![method("shared_method", AirlockLevel::Dangerous)]))
            .expect_err("collision should be rejected");
        assert!(err.contains("Method name collision"));
    }
}
