// Rainy Cowork - Data Models
// Core data structures for tasks, files, and AI providers

pub mod folder;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// AI Provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    RainyApi, // Primary backend from Enosis Labs
    Gemini,   // User's own Google API key
}

/// A single task step for detailed progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStep {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Core task structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub progress: u8, // 0-100
    pub provider: ProviderType,
    pub model: String,
    pub workspace_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub steps: Vec<TaskStep>,
}

impl Task {
    pub fn new(description: String, provider: ProviderType, model: String) -> Self {
        let id = Uuid::new_v4().to_string();
        let title = if description.len() > 50 {
            format!("{}...", &description[..47])
        } else {
            description.clone()
        };

        Self {
            id,
            title,
            description,
            status: TaskStatus::Queued,
            progress: 0,
            provider,
            model,
            workspace_path: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
            steps: Vec::new(),
        }
    }
}

/// File operation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileOperation {
    Create,
    Modify,
    Delete,
    Move,
    Rename,
}

/// Record of a file change made by a task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub operation: FileOperation,
    pub timestamp: DateTime<Utc>,
    pub task_id: Option<String>,
    pub previous_path: Option<String>,
    pub version_id: Option<String>, // Reference to version for rollback
}

impl FileChange {
    pub fn new(path: String, operation: FileOperation, task_id: Option<String>) -> Self {
        let filename = std::path::Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());

        Self {
            id: Uuid::new_v4().to_string(),
            path,
            filename,
            operation,
            timestamp: Utc::now(),
            task_id,
            previous_path: None,
            version_id: None,
        }
    }
}

/// Workspace folder with access permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub path: String,
    pub name: String,
    pub access_type: WorkspaceAccess,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceAccess {
    ReadOnly,
    FullAccess,
}

/// AI Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIProviderConfig {
    pub provider: ProviderType,
    pub name: String,
    pub model: String,
    pub is_available: bool,
    pub requires_api_key: bool,
}

/// Progress event sent to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum TaskEvent {
    Started {
        task_id: String,
    },
    Progress {
        task_id: String,
        progress: u8,
        message: Option<String>,
    },
    StepCompleted {
        task_id: String,
        step_id: String,
    },
    Completed {
        task_id: String,
    },
    Failed {
        task_id: String,
        error: String,
    },
}

/// File version snapshot for undo/redo
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileVersion {
    pub id: String,
    pub original_path: String,
    pub snapshot_path: String,
    pub task_id: String,
    pub created_at: DateTime<Utc>,
}
