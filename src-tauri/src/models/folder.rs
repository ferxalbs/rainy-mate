// Rainy Cowork - User Folder Models
// Persisted folder entries for sidebar quick access

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User-added folder for sidebar display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFolder {
    pub id: String,
    pub path: String,
    pub name: String,
    pub access_type: FolderAccess,
    pub added_at: DateTime<Utc>,
}

/// Folder access level (used for UI display)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FolderAccess {
    ReadOnly,
    FullAccess,
}

impl Default for FolderAccess {
    fn default() -> Self {
        Self::FullAccess
    }
}
