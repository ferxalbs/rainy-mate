use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use dirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub allowed_paths: Vec<String>,
    pub permissions: WorkspacePermissions,
    pub agents: Vec<AgentConfig>,
    pub memory: WorkspaceMemory,
    pub settings: WorkspaceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePermissions {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_delete: bool,
    pub can_create_agents: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    pub theme: String,
    pub language: String,
    pub auto_save: bool,
    pub notifications_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: Uuid,
    pub name: String,
    pub agent_type: String,
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMemory {
    pub max_size: u64,
    pub current_size: u64,
    pub retention_policy: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigFormat {
    Json,
    Toml,
}

#[derive(Debug)]
pub struct WorkspaceManager {
    workspaces_dir: PathBuf,
}

impl WorkspaceManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let app_data_dir = dirs::data_dir()
            .ok_or("Could not find app data directory")?;
        let workspaces_dir = app_data_dir.join("rainy-cowork").join("workspaces");

        // Create the directory if it doesn't exist
        fs::create_dir_all(&workspaces_dir)?;

        Ok(Self { workspaces_dir })
    }

    pub fn create_workspace(&self, name: String, allowed_paths: Vec<String>) -> Result<Workspace, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();
        let workspace = Workspace {
            id,
            name,
            allowed_paths,
            permissions: WorkspacePermissions {
                can_read: true,
                can_write: true,
                can_execute: false,
                can_delete: false,
                can_create_agents: true,
            },
            agents: Vec::new(),
            memory: WorkspaceMemory {
                max_size: 1024 * 1024 * 100, // 100MB
                current_size: 0,
                retention_policy: "fifo".to_string(),
            },
            settings: WorkspaceSettings {
                theme: "default".to_string(),
                language: "en".to_string(),
                auto_save: true,
                notifications_enabled: true,
            },
        };

        // Save the workspace
        self.save_workspace(&workspace, ConfigFormat::Json)?;

        Ok(workspace)
    }

    pub fn load_workspace(&self, id: &Uuid) -> Result<Workspace, Box<dyn std::error::Error>> {
        let json_path = self.workspaces_dir.join(format!("{}.json", id));
        let toml_path = self.workspaces_dir.join(format!("{}.toml", id));

        if json_path.exists() {
            let content = fs::read_to_string(json_path)?;
            Ok(serde_json::from_str(&content)?)
        } else if toml_path.exists() {
            let content = fs::read_to_string(toml_path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Err(format!("Workspace with id {} not found", id).into())
        }
    }

    pub fn save_workspace(&self, workspace: &Workspace, format: ConfigFormat) -> Result<(), Box<dyn std::error::Error>> {
        let filename = match format {
            ConfigFormat::Json => format!("{}.json", workspace.id),
            ConfigFormat::Toml => format!("{}.toml", workspace.id),
        };
        let path = self.workspaces_dir.join(filename);

        let content = match format {
            ConfigFormat::Json => serde_json::to_string_pretty(workspace)?,
            ConfigFormat::Toml => toml::to_string(workspace)?,
        };

        fs::write(path, content)?;
        Ok(())
    }

    pub fn list_workspaces(&self) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
        let mut workspaces = Vec::new();

        for entry in fs::read_dir(&self.workspaces_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(extension) = path.extension() {
                if extension == "json" || extension == "toml" {
                    if let Some(stem) = path.file_stem() {
                        if let Ok(id) = Uuid::parse_str(&stem.to_string_lossy()) {
                            workspaces.push(id);
                        }
                    }
                }
            }
        }

        Ok(workspaces)
    }

    pub fn delete_workspace(&self, id: &Uuid) -> Result<(), Box<dyn std::error::Error>> {
        let json_path = self.workspaces_dir.join(format!("{}.json", id));
        let toml_path = self.workspaces_dir.join(format!("{}.toml", id));

        let mut deleted = false;
        if json_path.exists() {
            fs::remove_file(json_path)?;
            deleted = true;
        }
        if toml_path.exists() {
            fs::remove_file(toml_path)?;
            deleted = true;
        }

        if !deleted {
            return Err(format!("Workspace with id {} not found", id).into());
        }

        Ok(())
    }

    /// Validate if a path is allowed within a workspace
    pub fn validate_path(&self, workspace: &Workspace, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::path::Path;

        let path_buf = Path::new(path);
        let canonical_path = path_buf.canonicalize()
            .map_err(|_| format!("Cannot canonicalize path: {}", path))?;

        // Check if path is within allowed paths
        let is_allowed = workspace.allowed_paths.iter().any(|allowed| {
            let allowed_path = Path::new(allowed);
            if let Ok(canonical_allowed) = allowed_path.canonicalize() {
                canonical_path.starts_with(&canonical_allowed)
            } else {
                false
            }
        });

        if !is_allowed {
            return Err(format!("Path {} is not within allowed workspace paths", path).into());
        }

        Ok(())
    }

    /// Validate if an operation is permitted based on workspace permissions
    pub fn validate_operation(&self, workspace: &Workspace, operation: &str) -> Result<(), Box<dyn std::error::Error>> {
        let permitted = match operation {
            "read" => workspace.permissions.can_read,
            "write" => workspace.permissions.can_write,
            "execute" => workspace.permissions.can_execute,
            "delete" => workspace.permissions.can_delete,
            "create_agents" => workspace.permissions.can_create_agents,
            _ => return Err(format!("Unknown operation: {}", operation).into()),
        };

        if !permitted {
            return Err(format!("Operation '{}' is not permitted in this workspace", operation).into());
        }

        Ok(())
    }

    /// Validate both path and operation for a workspace
    pub fn validate_path_and_operation(&self, workspace: &Workspace, path: &str, operation: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.validate_path(workspace, path)?;
        self.validate_operation(workspace, operation)
    }
}