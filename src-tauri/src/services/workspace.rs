use chrono::{DateTime, Utc};
use dirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use uuid::Uuid;

use crate::services::mate_launchpad::WorkspaceLaunchSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub allowed_paths: Vec<String>,
    pub permissions: WorkspacePermissions,
    pub permission_overrides: Vec<PermissionOverride>,
    pub agents: Vec<AgentConfig>,
    pub memory: WorkspaceMemory,
    pub settings: WorkspaceSettings,
    #[serde(default)]
    pub launchpad: WorkspaceLaunchSettings,
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
pub struct PermissionOverride {
    pub path: String,
    pub permissions: WorkspacePermissions,
    pub inherited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub default_permissions: WorkspacePermissions,
    pub default_settings: WorkspaceSettings,
    pub default_memory: WorkspaceMemory,
    pub suggested_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceAnalytics {
    pub workspace_id: String,
    pub total_files: u64,
    pub total_folders: u64,
    pub total_operations: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub memory_used: u64,
    pub last_activity: DateTime<Utc>,
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
        let app_data_dir = dirs::data_dir().ok_or("Could not find app data directory")?;
        let workspaces_dir =
            crate::services::app_identity::resolve_child_dir(app_data_dir, "workspaces")?;

        // Create the directory if it doesn't exist
        fs::create_dir_all(&workspaces_dir)?;

        Ok(Self { workspaces_dir })
    }

    pub fn create_workspace(
        &self,
        name: String,
        allowed_paths: Vec<String>,
    ) -> Result<Workspace, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4().to_string();
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
            permission_overrides: Vec::new(),
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
            launchpad: WorkspaceLaunchSettings::default(),
        };

        // Save the workspace
        self.save_workspace(&workspace, ConfigFormat::Json)?;

        Ok(workspace)
    }

    pub fn load_workspace(&self, id: &str) -> Result<Workspace, Box<dyn std::error::Error>> {
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

    pub fn find_workspace_by_path(
        &self,
        workspace_path: &str,
    ) -> Result<Option<Workspace>, Box<dyn std::error::Error>> {
        for workspace_id in self.list_workspaces()? {
            let workspace = self.load_workspace(&workspace_id)?;
            if workspace
                .allowed_paths
                .iter()
                .any(|allowed| allowed == workspace_path)
            {
                return Ok(Some(workspace));
            }
        }

        Ok(None)
    }

    pub fn ensure_workspace_for_path(
        &self,
        workspace_path: &str,
    ) -> Result<Workspace, Box<dyn std::error::Error>> {
        if let Some(existing) = self.find_workspace_by_path(workspace_path)? {
            return Ok(existing);
        }

        let name = Path::new(workspace_path)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("Workspace")
            .to_string();

        self.create_workspace(name, vec![workspace_path.to_string()])
    }

    pub fn save_workspace(
        &self,
        workspace: &Workspace,
        format: ConfigFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
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

    pub fn list_workspaces(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut workspaces = Vec::new();

        for entry in fs::read_dir(&self.workspaces_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(extension) = path.extension() {
                if extension == "json" || extension == "toml" {
                    if let Some(stem) = path.file_stem() {
                        // Accept any string ID now
                        workspaces.push(stem.to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(workspaces)
    }

    pub fn delete_workspace(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
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
    pub fn validate_path(
        &self,
        workspace: &Workspace,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::path::Path;

        let path_buf = Path::new(path);
        let canonical_path = path_buf
            .canonicalize()
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
    #[allow(dead_code)]
    pub fn validate_operation(
        &self,
        workspace: &Workspace,
        path: &str,
        operation: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get effective permissions for the specific path
        let effective_permissions = self.get_effective_permissions(workspace, path);

        let permitted = match operation {
            "read" => effective_permissions.can_read,
            "write" => effective_permissions.can_write,
            "execute" => effective_permissions.can_execute,
            "delete" => effective_permissions.can_delete,
            "create_agents" => effective_permissions.can_create_agents,
            _ => return Err(format!("Unknown operation: {}", operation).into()),
        };

        if !permitted {
            return Err(format!(
                "Operation '{}' is not permitted for path '{}' in this workspace",
                operation, path
            )
            .into());
        }

        Ok(())
    }

    /// Validate both path and operation for a workspace
    #[allow(dead_code)]
    pub fn validate_path_and_operation(
        &self,
        workspace: &Workspace,
        path: &str,
        operation: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.validate_path(workspace, path)?;
        self.validate_operation(workspace, path, operation)
    }

    /// Get effective permissions for a specific path, considering overrides
    pub fn get_effective_permissions(
        &self,
        workspace: &Workspace,
        path: &str,
    ) -> WorkspacePermissions {
        use std::path::Path;

        // Check for path-specific overrides
        for perm_override in &workspace.permission_overrides {
            let override_path = Path::new(&perm_override.path);
            let target_path = Path::new(path);

            // Check if the target path is within the override path
            if let Ok(canonical_override) = override_path.canonicalize() {
                if let Ok(canonical_target) = target_path.canonicalize() {
                    if canonical_target.starts_with(&canonical_override) {
                        return perm_override.permissions.clone();
                    }
                }
            }
        }

        // Fall back to workspace-level permissions
        workspace.permissions.clone()
    }

    /// Add a permission override for a specific path
    pub fn add_permission_override(
        &self,
        workspace: &mut Workspace,
        path: String,
        permissions: WorkspacePermissions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Validate that the path is within allowed paths
        self.validate_path(workspace, &path)?;

        // Check if an override already exists for this path
        if let Some(existing) = workspace
            .permission_overrides
            .iter_mut()
            .find(|o| o.path == path)
        {
            existing.permissions = permissions;
            existing.inherited = false;
        } else {
            workspace.permission_overrides.push(PermissionOverride {
                path,
                permissions,
                inherited: false,
            });
        }

        Ok(())
    }

    /// Remove a permission override for a specific path
    pub fn remove_permission_override(
        &self,
        workspace: &mut Workspace,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        workspace.permission_overrides.retain(|o| o.path != path);
        Ok(())
    }

    /// Get all permission overrides for a workspace
    pub fn get_permission_overrides(&self, workspace: &Workspace) -> Vec<PermissionOverride> {
        workspace.permission_overrides.clone()
    }

    /// Get all available workspace templates
    pub fn get_templates(&self) -> Result<Vec<WorkspaceTemplate>, Box<dyn std::error::Error>> {
        let templates_dir = self.workspaces_dir.join("templates");

        // Create templates directory if it doesn't exist
        if !templates_dir.exists() {
            fs::create_dir_all(&templates_dir)?;
        }

        let mut templates = Vec::new();

        // Define default templates
        templates.push(WorkspaceTemplate {
            id: "development".to_string(),
            name: "Development Workspace".to_string(),
            description:
                "Full-featured workspace for software development with code analysis agents"
                    .to_string(),
            category: "Development".to_string(),
            default_permissions: WorkspacePermissions {
                can_read: true,
                can_write: true,
                can_execute: true,
                can_delete: false,
                can_create_agents: true,
            },
            default_settings: WorkspaceSettings {
                theme: "dark".to_string(),
                language: "en".to_string(),
                auto_save: true,
                notifications_enabled: true,
            },
            default_memory: WorkspaceMemory {
                max_size: 1024 * 1024 * 100, // 100MB
                current_size: 0,
                retention_policy: "fifo".to_string(),
            },
            suggested_paths: vec!["src".to_string(), "tests".to_string(), "docs".to_string()],
        });

        templates.push(WorkspaceTemplate {
            id: "research".to_string(),
            name: "Research Workspace".to_string(),
            description:
                "Workspace optimized for research and documentation with AI research agents"
                    .to_string(),
            category: "Research".to_string(),
            default_permissions: WorkspacePermissions {
                can_read: true,
                can_write: true,
                can_execute: false,
                can_delete: false,
                can_create_agents: true,
            },
            default_settings: WorkspaceSettings {
                theme: "light".to_string(),
                language: "en".to_string(),
                auto_save: true,
                notifications_enabled: true,
            },
            default_memory: WorkspaceMemory {
                max_size: 1024 * 1024 * 500, // 500MB
                current_size: 0,
                retention_policy: "lru".to_string(),
            },
            suggested_paths: vec![
                "research".to_string(),
                "notes".to_string(),
                "references".to_string(),
            ],
        });

        templates.push(WorkspaceTemplate {
            id: "minimal".to_string(),
            name: "Minimal Workspace".to_string(),
            description: "Basic workspace with minimal permissions for simple file operations"
                .to_string(),
            category: "General".to_string(),
            default_permissions: WorkspacePermissions {
                can_read: true,
                can_write: true,
                can_execute: false,
                can_delete: false,
                can_create_agents: false,
            },
            default_settings: WorkspaceSettings {
                theme: "system".to_string(),
                language: "en".to_string(),
                auto_save: false,
                notifications_enabled: false,
            },
            default_memory: WorkspaceMemory {
                max_size: 1024 * 1024 * 10, // 10MB
                current_size: 0,
                retention_policy: "fifo".to_string(),
            },
            suggested_paths: vec![],
        });

        // Load custom templates from files
        if templates_dir.exists() {
            for entry in fs::read_dir(&templates_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path
                    .extension()
                    .map_or(None, |ext| Some(ext == "json" || ext == "toml"))
                    .is_some()
                {
                    let content = fs::read_to_string(&path)?;
                    // Try JSON first, then TOML
                    if let Ok(template) = serde_json::from_str::<WorkspaceTemplate>(&content) {
                        templates.push(template);
                    } else if let Ok(template) = toml::from_str::<WorkspaceTemplate>(&content) {
                        templates.push(template);
                    }
                }
            }
        }

        Ok(templates)
    }

    /// Create a workspace from a template
    pub fn create_from_template(
        &self,
        template_id: &str,
        name: String,
        custom_paths: Option<Vec<String>>,
    ) -> Result<Workspace, Box<dyn std::error::Error>> {
        let templates = self.get_templates()?;

        let template = templates
            .iter()
            .find(|t| t.id == template_id)
            .ok_or_else(|| format!("Template '{}' not found", template_id))?;

        let allowed_paths = custom_paths.unwrap_or_else(|| template.suggested_paths.clone());

        let workspace = Workspace {
            id: Uuid::new_v4().to_string(),
            name,
            allowed_paths,
            permissions: template.default_permissions.clone(),
            permission_overrides: Vec::new(),
            agents: Vec::new(),
            memory: template.default_memory.clone(),
            settings: template.default_settings.clone(),
            launchpad: WorkspaceLaunchSettings::default(),
        };

        // Save the workspace
        self.save_workspace(&workspace, ConfigFormat::Json)?;

        Ok(workspace)
    }

    /// Save a custom template
    pub fn save_template(
        &self,
        template: &WorkspaceTemplate,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let templates_dir = self.workspaces_dir.join("templates");

        // Create templates directory if it doesn't exist
        if !templates_dir.exists() {
            fs::create_dir_all(&templates_dir)?;
        }

        let template_path = templates_dir.join(format!("{}.json", template.id));

        let content = serde_json::to_string_pretty(template)?;
        fs::write(template_path, content)?;

        Ok(())
    }

    /// Delete a custom template
    pub fn delete_template(&self, template_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let templates_dir = self.workspaces_dir.join("templates");

        let json_path = templates_dir.join(format!("{}.json", template_id));
        let toml_path = templates_dir.join(format!("{}.toml", template_id));

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
            return Err(format!("Template '{}' not found", template_id).into());
        }

        Ok(())
    }

    /// Get analytics for a workspace
    pub fn get_analytics(
        &self,
        workspace_id: &str,
    ) -> Result<WorkspaceAnalytics, Box<dyn std::error::Error>> {
        // Load workspace
        let workspace = self.load_workspace(workspace_id)?;

        // Calculate analytics (simplified for now)
        let analytics = WorkspaceAnalytics {
            workspace_id: workspace.id.clone(),
            total_files: workspace.allowed_paths.len() as u64,
            total_folders: workspace.allowed_paths.len() as u64,
            total_operations: 0, // Would need to track operations separately
            tasks_completed: 0,  // Would need to track tasks separately
            tasks_failed: 0,     // Would need to track tasks separately
            memory_used: workspace.memory.current_size,
            last_activity: Utc::now(),
        };

        Ok(analytics)
    }
}
