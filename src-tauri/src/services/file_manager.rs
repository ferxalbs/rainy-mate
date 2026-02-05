// Rainy Cowork - File Manager Service
// File operations with workspace-based versioning

use crate::commands::file::FileEntry;
use crate::models::{FileChange, FileOperation, FileVersion, Workspace, WorkspaceAccess};
use chrono::Utc;
use dashmap::DashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;
use uuid::Uuid;

/// File manager with workspace-based versioning
pub struct FileManager {
    workspace: RwLock<Option<Workspace>>,
    versions: DashMap<String, FileVersion>,
    changes: DashMap<String, FileChange>,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            workspace: RwLock::new(None),
            versions: DashMap::new(),
            changes: DashMap::new(),
        }
    }

    /// Set the active workspace
    pub async fn set_workspace(&self, path: String, name: String) -> Result<Workspace, String> {
        // Verify the path exists and is a directory
        let path_buf = PathBuf::from(&path);
        if !path_buf.exists() {
            return Err(format!("Path does not exist: {}", path));
        }
        if !path_buf.is_dir() {
            return Err(format!("Path is not a directory: {}", path));
        }

        let workspace = Workspace {
            id: Uuid::new_v4().to_string(),
            path,
            name,
            access_type: WorkspaceAccess::FullAccess,
            created_at: Utc::now(),
        };

        // Create versions directory within the workspace
        let versions_dir = path_buf.join(".rainy-versions");
        if !versions_dir.exists() {
            fs::create_dir_all(&versions_dir)
                .await
                .map_err(|e| format!("Failed to create versions directory: {}", e))?;
        }

        *self.workspace.write().await = Some(workspace.clone());
        Ok(workspace)
    }

    /// Get current workspace
    pub async fn get_workspace(&self) -> Option<Workspace> {
        self.workspace.read().await.clone()
    }

    /// List directory contents
    pub async fn list_directory(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        let path_buf = PathBuf::from(path);
        if !path_buf.exists() {
            return Err(format!("Path does not exist: {}", path));
        }

        let mut entries = Vec::new();
        let mut dir = fs::read_dir(&path_buf)
            .await
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        while let Some(entry) = dir
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read entry: {}", e))?
        {
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files and .rainy-versions directory
            if file_name.starts_with('.') {
                continue;
            }

            let metadata = entry
                .metadata()
                .await
                .map_err(|e| format!("Failed to read metadata: {}", e))?;

            entries.push(FileEntry {
                name: file_name,
                path: entry.path().to_string_lossy().to_string(),
                is_directory: metadata.is_dir(),
                size: if metadata.is_file() {
                    Some(metadata.len())
                } else {
                    None
                },
                modified: metadata.modified().ok().map(|t| {
                    chrono::DateTime::<Utc>::from(t)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                }),
            });
        }

        // Sort: directories first, then files
        entries.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(entries)
    }

    /// Read file content
    pub async fn read_file(&self, path: &str) -> Result<String, String> {
        fs::read_to_string(path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))
    }

    /// Write file with automatic versioning
    pub async fn write_file(
        &self,
        path: &str,
        content: &str,
        task_id: Option<String>,
    ) -> Result<FileChange, String> {
        let path_buf = PathBuf::from(path);
        let exists = path_buf.exists();

        // Create snapshot if file exists
        let version_id = if exists {
            if let Some(ref tid) = task_id {
                Some(self.create_snapshot(path, tid).await?)
            } else {
                None
            }
        } else {
            None
        };

        // Write the file
        fs::write(&path_buf, content)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        // Record the change
        let operation = if exists {
            FileOperation::Modify
        } else {
            FileOperation::Create
        };

        let mut change = FileChange::new(path.to_string(), operation, task_id);
        change.version_id = version_id;

        self.changes.insert(change.id.clone(), change.clone());

        Ok(change)
    }

    /// Append content to file with automatic versioning
    pub async fn append_file(
        &self,
        path: &str,
        content: &str,
        task_id: Option<String>,
    ) -> Result<FileChange, String> {
        let path_buf = PathBuf::from(path);

        if !path_buf.exists() {
            return Err(format!("File does not exist: {}", path));
        }

        // Create snapshot
        let version_id = if let Some(ref tid) = task_id {
            Some(self.create_snapshot(path, tid).await?)
        } else {
            None
        };

        // Append to file
        use tokio::io::AsyncWriteExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&path_buf)
            .await
            .map_err(|e| format!("Failed to open file for appending: {}", e))?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| format!("Failed to append content: {}", e))?;

        // Record change
        let mut change = FileChange::new(path.to_string(), FileOperation::Modify, task_id);
        change.version_id = version_id;
        self.changes.insert(change.id.clone(), change.clone());

        Ok(change)
    }

    /// Create a version snapshot before modification
    pub async fn create_snapshot(&self, path: &str, task_id: &str) -> Result<String, String> {
        let workspace = self
            .workspace
            .read()
            .await
            .clone()
            .ok_or("No workspace set")?;

        let workspace_path = PathBuf::from(&workspace.path);
        let versions_dir = workspace_path.join(".rainy-versions");

        // Generate unique snapshot filename
        let version_id = Uuid::new_v4().to_string();
        let original_path = PathBuf::from(path);
        let original_name = original_path
            .file_name()
            .ok_or("Invalid file path")?
            .to_string_lossy();

        let snapshot_name = format!("{}.{}", version_id, original_name);
        let snapshot_path = versions_dir.join(&snapshot_name);

        // Copy the original file to the snapshot
        fs::copy(&original_path, &snapshot_path)
            .await
            .map_err(|e| format!("Failed to create snapshot: {}", e))?;

        // Record the version
        let version = FileVersion {
            id: version_id.clone(),
            original_path: path.to_string(),
            snapshot_path: snapshot_path.to_string_lossy().to_string(),
            task_id: task_id.to_string(),
            created_at: Utc::now(),
        };

        self.versions.insert(version_id.clone(), version);

        Ok(version_id)
    }

    /// Rollback a file to a previous version
    pub async fn rollback(&self, version_id: &str) -> Result<(), String> {
        let version = self
            .versions
            .get(version_id)
            .ok_or(format!("Version not found: {}", version_id))?
            .clone();

        // Restore the original file from snapshot
        fs::copy(&version.snapshot_path, &version.original_path)
            .await
            .map_err(|e| format!("Failed to rollback: {}", e))?;

        Ok(())
    }

    /// List file changes, optionally filtered by task
    pub async fn list_changes(&self, task_id: Option<String>) -> Result<Vec<FileChange>, String> {
        let changes: Vec<FileChange> = self
            .changes
            .iter()
            .map(|r| r.value().clone())
            .filter(|c| match &task_id {
                Some(tid) => c.task_id.as_ref() == Some(tid),
                None => true,
            })
            .collect();

        Ok(changes)
    }
}

impl Default for FileManager {
    fn default() -> Self {
        Self::new()
    }
}
