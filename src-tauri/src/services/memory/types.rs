use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A piece of information stored in runtime memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier for the memory entry
    pub id: String,
    /// Content of the memory entry
    pub content: String,
    /// Optional embedding vector for semantic search
    pub embedding: Option<Vec<f32>>,
    /// When this entry was created
    pub timestamp: DateTime<Utc>,
    /// Tags for categorization and retrieval
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRetrievalMode {
    Ann,
    Exact,
    LexicalFallback,
    SimpleBuffer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub entries: Vec<MemoryEntry>,
    pub mode: SemanticRetrievalMode,
    pub reason: Option<String>,
    #[serde(default)]
    pub confidential_entry_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionResult {
    pub chunks_ingested: usize,
    pub chunks_embedded: usize,
    pub embedding_mode: String,
    pub warnings: Vec<String>,
}
