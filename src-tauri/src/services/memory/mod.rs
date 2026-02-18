//! Memory System for Multi-Agent System
//!
//! This module provides a dual-layer memory system:
//! - Short-term memory: Ring buffer for recent actions (in-memory)
//! - Long-term memory: Persistent storage with semantic search (LanceDB)
//!
//! # Architecture
//!
//! The memory system consists of three main components:
//! 1. **ShortTermMemory**: Fast, in-memory ring buffer for recent entries
//! 2. **LongTermMemory**: Persistent storage with semantic search capabilities
//! 3. **MemoryManager**: Coordinates both memory types and provides unified API
//!
//! # Usage
//!
//! ```rust,no_run
//! use crate::services::memory::MemoryManager;
//! use std::path::PathBuf;
//!
//! let manager = MemoryManager::new(
//!     100,  // short-term memory size
//!     PathBuf::from("./memory_db"),  // long-term storage path
//! );
//!
//! // Store entry
//! manager.store(entry).await?;
//!
//! // Search memory
//! let results = manager.search("query", 10).await?;
//! ```

pub mod crystalline;
pub mod long_term;
pub mod memory_manager;
pub mod short_term;
pub mod types;

pub use memory_manager::MemoryManager;
pub use types::MemoryEntry;

// Not re-exported here, available via full path:
// - long_term::MemoryError
// - long_term::MemoryStats
