use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const EMBEDDING_PROVIDER: &str = "gemini";
pub const EMBEDDING_MODEL: &str =
    crate::services::memory_vault::profiles::ACTIVE_EMBEDDING_PROFILE.model;
pub const EMBEDDING_DIM: usize =
    crate::services::memory_vault::profiles::ACTIVE_EMBEDDING_PROFILE.dim;

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

#[derive(Debug, Clone, Deserialize)]
pub struct ListFilteredOpts {
    pub workspace_id: Option<String>,
    pub sensitivity: Option<String>,
    pub source_prefix: Option<String>,
    pub created_after: Option<i64>,
    pub created_before: Option<i64>,
    pub order_by: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedEntries {
    pub entries: Vec<DecryptedMemoryEntry>,
    pub total_count: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub workspace_id: String,
    pub entry_count: usize,
}

// ─── Distillation Types ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryCategory {
    Preference,
    Correction,
    Fact,
    Procedure,
    Observation,
}

impl MemoryCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preference => "preference",
            Self::Correction => "correction",
            Self::Fact => "fact",
            Self::Procedure => "procedure",
            Self::Observation => "observation",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s {
            "preference" => Self::Preference,
            "correction" => Self::Correction,
            "fact" => Self::Fact,
            "procedure" => Self::Procedure,
            _ => Self::Observation,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledMemory {
    pub content: String,
    pub category: MemoryCategory,
    pub importance: f32,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RawMemoryTurn {
    pub content: String,
    pub role: String,
    pub source: String,
    pub workspace_id: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultDetailedStats {
    pub total_entries: usize,
    pub workspace_entries: usize,
    pub entries_by_sensitivity: HashMap<String, usize>,
    pub entries_by_source: Vec<(String, usize)>,
    pub has_embeddings: usize,
    pub missing_embeddings: usize,
    pub oldest_entry: Option<i64>,
    pub newest_entry: Option<i64>,
}
