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
