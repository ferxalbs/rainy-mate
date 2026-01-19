// Rainy Cowork - Folder Manager Service
// Manages user-added folders with JSON persistence

use crate::models::folder::{FolderAccess, UserFolder};
use chrono::Utc;
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Storage filename for user folders
const FOLDERS_FILE: &str = "user_folders.json";

/// Folder manager with JSON persistence
pub struct FolderManager {
    folders: RwLock<Vec<UserFolder>>,
    storage_path: PathBuf,
}

impl FolderManager {
    /// Create a new folder manager with app data storage
    pub fn new(app_data_dir: PathBuf) -> Self {
        let storage_path = app_data_dir.join(FOLDERS_FILE);
        Self {
            folders: RwLock::new(Vec::new()),
            storage_path,
        }
    }

    /// Initialize and load folders from disk
    pub async fn init(&self) -> Result<(), String> {
        // Ensure parent directory exists
        if let Some(parent) = self.storage_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("Failed to create app data dir: {}", e))?;
            }
        }

        // Load existing folders if file exists
        if self.storage_path.exists() {
            let content = fs::read_to_string(&self.storage_path)
                .await
                .map_err(|e| format!("Failed to read folders file: {}", e))?;

            let loaded: Vec<UserFolder> = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse folders file: {}", e))?;

            *self.folders.write().await = loaded;
        }

        Ok(())
    }

    /// Add a new folder
    pub async fn add_folder(
        &self,
        path: String,
        name: String,
        access_type: FolderAccess,
    ) -> Result<UserFolder, String> {
        // Validate path exists and is a directory
        let path_buf = PathBuf::from(&path);
        if !path_buf.exists() {
            return Err(format!("Path does not exist: {}", path));
        }
        if !path_buf.is_dir() {
            return Err(format!("Path is not a directory: {}", path));
        }

        // Check for duplicates
        {
            let folders = self.folders.read().await;
            if folders.iter().any(|f| f.path == path) {
                return Err("Folder already added".to_string());
            }
        }

        let now = Utc::now();
        let folder = UserFolder {
            id: Uuid::new_v4().to_string(),
            path,
            name,
            access_type,
            added_at: now,
            last_accessed: now,
        };

        // Add and persist
        {
            let mut folders = self.folders.write().await;
            folders.push(folder.clone());
        }
        self.persist().await?;

        Ok(folder)
    }

    /// List all user folders (sorted by last accessed, most recent first)
    pub async fn list_folders(&self) -> Vec<UserFolder> {
        let mut folders = self.folders.read().await.clone();
        folders.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        folders
    }

    /// Update last accessed timestamp for a folder
    pub async fn update_last_accessed(&self, id: &str) -> Result<(), String> {
        {
            let mut folders = self.folders.write().await;
            if let Some(folder) = folders.iter_mut().find(|f| f.id == id) {
                folder.last_accessed = Utc::now();
            } else {
                return Err(format!("Folder not found: {}", id));
            }
        }
        self.persist().await?;
        Ok(())
    }

    /// Remove a folder by ID
    pub async fn remove_folder(&self, id: &str) -> Result<(), String> {
        {
            let mut folders = self.folders.write().await;
            let initial_len = folders.len();
            folders.retain(|f| f.id != id);

            if folders.len() == initial_len {
                return Err(format!("Folder not found: {}", id));
            }
        }
        self.persist().await?;

        Ok(())
    }

    /// Persist folders to disk
    async fn persist(&self) -> Result<(), String> {
        let folders = self.folders.read().await;
        let content = serde_json::to_string_pretty(&*folders)
            .map_err(|e| format!("Failed to serialize folders: {}", e))?;

        fs::write(&self.storage_path, content)
            .await
            .map_err(|e| format!("Failed to write folders file: {}", e))?;

        Ok(())
    }
}
