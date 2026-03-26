//! Memory System
//!
//! Thin compatibility layer used by Tauri commands and the skill executor.
//! Long-term storage is backed by `memory_vault` (encrypted at rest).

pub mod memory_manager;
pub mod types;

pub use memory_manager::MemoryManager;
pub use types::{IngestionResult, MemoryEntry, SemanticRetrievalMode, SemanticSearchResult};

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub total_size: u64,
}
