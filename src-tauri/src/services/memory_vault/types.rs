use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const EMBEDDING_PROVIDER: &str = "gemini";
pub const EMBEDDING_MODEL: &str =
    crate::services::memory_vault::profiles::ACTIVE_EMBEDDING_PROFILE.model;
pub const EMBEDDING_DIM: usize = crate::services::memory_vault::profiles::ACTIVE_EMBEDDING_PROFILE.dim;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemorySensitivity {
    Public,
    Internal,
    Confidential,
}

impl MemorySensitivity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Confidential => "confidential",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "public" => Self::Public,
            "confidential" => Self::Confidential,
            _ => Self::Internal,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdditionalEmbeddingInput {
    pub embedding: Vec<f32>,
    pub embedding_model: String,
    pub embedding_provider: String,
    pub embedding_dim: usize,
}

#[derive(Debug, Clone)]
pub struct StoreMemoryInput {
    pub id: String,
    pub workspace_id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source: String,
    pub sensitivity: MemorySensitivity,
    pub metadata: HashMap<String, String>,
    pub created_at: i64,
    pub embedding: Option<Vec<f32>>,
    pub embedding_model: Option<String>,
    pub embedding_provider: Option<String>,
    pub embedding_dim: Option<usize>,
    pub additional_embeddings: Vec<AdditionalEmbeddingInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptedMemoryEntry {
    pub id: String,
    pub workspace_id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source: String,
    pub sensitivity: MemorySensitivity,
    pub created_at: i64,
    pub last_accessed: i64,
    pub access_count: i64,
    pub metadata: HashMap<String, String>,
    pub embedding: Option<Vec<f32>>,
    pub embedding_model: Option<String>,
    pub embedding_provider: Option<String>,
    pub embedding_dim: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVaultStats {
    pub total_entries: usize,
    pub workspace_entries: usize,
}
