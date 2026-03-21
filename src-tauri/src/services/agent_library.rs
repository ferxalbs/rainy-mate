use crate::ai::specs::manifest::AgentSpec;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLibraryEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub updated_at_ms: i64,
}

#[derive(Clone)]
pub struct AgentLibraryService {
    root: PathBuf,
}

impl AgentLibraryService {
    pub fn new_default() -> Result<Self, String> {
        let root = crate::services::app_identity::resolve_namespaced_child_dir(
            dirs::data_dir().ok_or_else(|| "Failed to locate data dir".to_string())?,
            "agent-library",
        )?;
        Ok(Self { root })
    }

    pub fn save_spec(&self, spec: &AgentSpec) -> Result<AgentLibraryEntry, String> {
        let id = validate_agent_id_component(&spec.id)?;

        let file = self.root.join(format!("{}.json", id));
        let serialized = serde_json::to_string_pretty(spec)
            .map_err(|e| format!("Failed to serialize AgentSpec: {}", e))?;
        fs::write(&file, serialized)
            .map_err(|e| format!("Failed to persist AgentSpec to library: {}", e))?;

        Ok(AgentLibraryEntry {
            id: spec.id.clone(),
            name: spec.soul.name.clone(),
            path: file.to_string_lossy().to_string(),
            updated_at_ms: now_ms(),
        })
    }

    pub fn load_spec(&self, id: &str) -> Result<AgentSpec, String> {
        let id = validate_agent_id_component(id)?;
        let file = self.root.join(format!("{}.json", id));
        let raw = fs::read_to_string(&file)
            .map_err(|e| format!("Failed to read AgentSpec from library: {}", e))?;
        serde_json::from_str(&raw).map_err(|e| format!("Invalid AgentSpec JSON: {}", e))
    }

    pub fn list_specs(&self) -> Result<Vec<AgentLibraryEntry>, String> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.root)
            .map_err(|e| format!("Failed to read agent library: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Invalid library entry: {}", e))?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            let raw = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read library AgentSpec: {}", e))?;
            let spec: AgentSpec = serde_json::from_str(&raw)
                .map_err(|e| format!("Invalid AgentSpec in library: {}", e))?;

            let metadata = fs::metadata(&path)
                .map_err(|e| format!("Failed to read AgentSpec metadata: {}", e))?;
            let updated_at_ms = metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or_else(now_ms);

            entries.push(AgentLibraryEntry {
                id: spec.id,
                name: spec.soul.name,
                path: path.to_string_lossy().to_string(),
                updated_at_ms,
            });
        }

        entries.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms));
        Ok(entries)
    }

    #[cfg(test)]
    pub(crate) fn from_root(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root)
            .map_err(|e| format!("Failed to create test agent library dir: {}", e))?;
        Ok(Self { root })
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn validate_agent_id_component(id: &str) -> Result<&str, String> {
    const MAX_AGENT_ID_LEN: usize = 128;

    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err("Agent id cannot be empty".to_string());
    }
    if trimmed.len() > MAX_AGENT_ID_LEN {
        return Err(format!(
            "Agent id is too long (max {} characters)",
            MAX_AGENT_ID_LEN
        ));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err(
            "Agent id contains invalid characters. Allowed: letters, numbers, '_' and '-'"
                .to_string(),
        );
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::AgentLibraryService;
    use crate::ai::specs::manifest::AgentSpec;
    use crate::ai::specs::skills::AgentSkills;
    use crate::ai::specs::soul::AgentSoul;

    #[test]
    fn save_list_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "rainy-agent-library-test-{}",
            uuid::Uuid::new_v4()
        ));
        let service = AgentLibraryService::from_root(dir).expect("service should initialize");

        let spec = AgentSpec {
            id: "agent_test_roundtrip".to_string(),
            version: "3.0.0".to_string(),
            soul: AgentSoul {
                name: "Roundtrip Agent".to_string(),
                ..Default::default()
            },
            skills: AgentSkills::default(),
            airlock: Default::default(),
            memory_config: Default::default(),
            connectors: Default::default(),
            runtime: Default::default(),
            model: None,
            temperature: None,
            max_tokens: None,
            provider: None,
            signature: None,
        };

        service.save_spec(&spec).expect("save should succeed");
        let list = service.list_specs().expect("list should succeed");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, spec.id);

        let loaded = service.load_spec(&spec.id).expect("load should succeed");
        assert_eq!(loaded.id, spec.id);
        assert_eq!(loaded.soul.name, spec.soul.name);
    }

    #[test]
    fn save_rejects_path_traversal_and_invalid_ids() {
        let dir = std::env::temp_dir().join(format!(
            "rainy-agent-library-test-invalid-save-{}",
            uuid::Uuid::new_v4()
        ));
        let service = AgentLibraryService::from_root(dir).expect("service should initialize");

        let invalid_ids = vec![
            "../outside",
            "..",
            "agent/one",
            "agent\\one",
            "agent.one",
            "agent one",
            "",
        ];

        for invalid_id in invalid_ids {
            let spec = AgentSpec {
                id: invalid_id.to_string(),
                version: "3.0.0".to_string(),
                soul: AgentSoul {
                    name: "Invalid".to_string(),
                    ..Default::default()
                },
                skills: AgentSkills::default(),
                airlock: Default::default(),
                memory_config: Default::default(),
                connectors: Default::default(),
                runtime: Default::default(),
                model: None,
                temperature: None,
                max_tokens: None,
                provider: None,
                signature: None,
            };
            assert!(service.save_spec(&spec).is_err(), "id should fail: {}", invalid_id);
        }
    }

    #[test]
    fn load_rejects_path_traversal_and_invalid_ids() {
        let dir = std::env::temp_dir().join(format!(
            "rainy-agent-library-test-invalid-load-{}",
            uuid::Uuid::new_v4()
        ));
        let service = AgentLibraryService::from_root(dir).expect("service should initialize");

        let invalid_ids = vec![
            "../outside",
            "..",
            "agent/one",
            "agent\\one",
            "agent.one",
            "agent one",
            "",
        ];

        for invalid_id in invalid_ids {
            assert!(
                service.load_spec(invalid_id).is_err(),
                "id should fail: {}",
                invalid_id
            );
        }
    }

    #[test]
    fn save_and_load_accept_valid_slug_ids() {
        let dir = std::env::temp_dir().join(format!(
            "rainy-agent-library-test-valid-slugs-{}",
            uuid::Uuid::new_v4()
        ));
        let service = AgentLibraryService::from_root(dir).expect("service should initialize");
        let valid_ids = vec!["agent_1", "AGENT-Prod_2026", "a1_b2-C3"];

        for id in valid_ids {
            let spec = AgentSpec {
                id: id.to_string(),
                version: "3.0.0".to_string(),
                soul: AgentSoul {
                    name: "Valid".to_string(),
                    ..Default::default()
                },
                skills: AgentSkills::default(),
                airlock: Default::default(),
                memory_config: Default::default(),
                connectors: Default::default(),
                runtime: Default::default(),
                model: None,
                temperature: None,
                max_tokens: None,
                provider: None,
                signature: None,
            };
            service.save_spec(&spec).expect("valid save should succeed");
            let loaded = service.load_spec(id).expect("valid load should succeed");
            assert_eq!(loaded.id, id);
        }
    }
}
