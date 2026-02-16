//! Memory manager that coordinates short-term and long-term memory
//!
//! This module provides a unified interface for managing both short-term and long-term memory.
//! It coordinates operations between the two memory types and provides a simple API for
//! storing, searching, and retrieving memory entries.

use crate::agents::MemoryEntry;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Memory manager that coordinates short-term and long-term memory
///
/// Provides a unified interface for managing both memory types.
/// All operations are thread-safe using Arc<RwLock>.
///
/// # Example
///
/// ```rust,no_run
/// use crate::services::memory::MemoryManager;
/// use std::path::PathBuf;
///
/// let manager = MemoryManager::new(
///     100,  // short-term memory size
///     PathBuf::from("./memory_db"),  // long-term storage path
/// );
///
/// // Store entry
/// manager.store(entry).await?;
///
/// // Search memory
/// let results = manager.search("query", 10).await?;
/// ```
#[derive(Debug, Clone)]
pub struct MemoryManager {
    /// Short-term memory (ring buffer)
    short_term: Arc<RwLock<super::short_term::ShortTermMemory>>,
    /// Long-term memory (persistent storage)
    long_term: Arc<super::long_term::LongTermMemory>,
    /// Crystalline memory (Markdown file watcher)
    crystalline: Arc<super::crystalline::CrystallineMemory>,
}

impl MemoryManager {
    /// Create a new memory manager
    ///
    /// # Arguments
    ///
    /// * `short_term_size` - Maximum number of entries in short-term memory
    /// * `long_term_path` - Path to the long-term memory database
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::services::memory::MemoryManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = MemoryManager::new(
    ///     100,
    ///     PathBuf::from("./memory_db"),
    /// );
    /// ```
    pub fn new(short_term_size: usize, long_term_path: std::path::PathBuf) -> Self {
        Self {
            short_term: Arc::new(RwLock::new(super::short_term::ShortTermMemory::new(
                short_term_size,
            ))),
            long_term: Arc::new(super::long_term::LongTermMemory::new(
                long_term_path.clone(),
            )),
            crystalline: Arc::new(super::crystalline::CrystallineMemory::new(
                long_term_path.parent().unwrap().join("memory_markdown"),
            )),
        }
    }

    /// Initialize subsystems (Crystalline watcher)
    pub async fn init(&self) {
        if let Err(e) = self.crystalline.init().await {
            tracing::error!("Failed to init Crystalline memory: {}", e);
        }
    }

    /// Store an entry in both short-term and long-term memory
    ///
    /// The entry is added to short-term memory and persisted in long-term memory.
    ///
    /// # Arguments
    ///
    /// * `entry` - The memory entry to store
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful, `Err(MemoryError)` otherwise
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::services::memory::MemoryManager;
    /// use crate::agents::MemoryEntry;
    /// use chrono::Utc;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = MemoryManager::new(100, PathBuf::from("./memory_db"));
    ///
    /// let entry = MemoryEntry {
    ///     id: "1".to_string(),
    ///     content: "Test entry".to_string(),
    ///     embedding: None,
    ///     timestamp: Utc::now(),
    ///     tags: vec!["test".to_string()],
    /// };
    ///
    /// manager.store(entry).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn store(&self, entry: MemoryEntry) -> Result<(), super::long_term::MemoryError> {
        // Add to short-term memory
        {
            let mut stm = self.short_term.write().await;
            stm.add(entry.clone());
        }

        // Store in long-term memory
        self.long_term.store(entry).await?;

        Ok(())
    }

    /// Search memory (short-term + long-term)
    ///
    /// Searches both short-term and long-term memory, combining results.
    /// Short-term results are returned first (most recent).
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// A vector of matching memory entries
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::services::memory::MemoryManager;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = MemoryManager::new(100, PathBuf::from("./memory_db"));
    ///
    /// let results = manager.search("test query", 10).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, super::long_term::MemoryError> {
        // Search short-term memory
        let recent = {
            let stm = self.short_term.read().await;
            stm.get_recent(limit)
        };

        // Search long-term memory
        let long_term_results = self.long_term.search(query, limit).await?;

        // Search Crystalline memory
        let crystalline_results = self.crystalline.search(query, limit).await;

        // Combine results (recent first)
        let mut results = recent;
        results.extend(crystalline_results);
        results.extend(long_term_results);

        // Limit total results
        results.truncate(limit);

        Ok(results)
    }

    /// Get recent entries from short-term memory
    ///
    /// Returns the most recent entries from short-term memory only.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of entries to return
    ///
    /// # Returns
    ///
    /// A vector of the most recent entries
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::services::memory::MemoryManager;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = MemoryManager::new(100, PathBuf::from("./memory_db"));
    ///
    /// let recent = manager.get_recent(10).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_recent(&self, count: usize) -> Vec<MemoryEntry> {
        let stm = self.short_term.read().await;
        stm.get_recent(count)
    }

    /// Get all entries from short-term memory
    ///
    /// Returns all entries from short-term memory only.
    ///
    /// # Returns
    ///
    /// A vector of all entries in short-term memory
    pub async fn get_all_short_term(&self) -> Vec<MemoryEntry> {
        let stm = self.short_term.read().await;
        stm.get_all()
    }

    /// Clear short-term memory
    ///
    /// Removes all entries from short-term memory.
    /// Long-term memory is not affected.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::services::memory::MemoryManager;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = MemoryManager::new(100, PathBuf::from("./memory_db"));
    ///
    /// manager.clear_short_term().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn clear_short_term(&self) {
        let mut stm = self.short_term.write().await;
        stm.clear();
    }

    /// Get memory statistics
    ///
    /// Returns statistics from long-term memory.
    ///
    /// # Returns
    ///
    /// Memory statistics including total entries and storage size
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::services::memory::MemoryManager;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = MemoryManager::new(100, PathBuf::from("./memory_db"));
    ///
    /// let stats = manager.get_stats().await?;
    /// println!("Total entries: {}", stats.total_entries);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_stats(
        &self,
    ) -> Result<super::long_term::MemoryStats, super::long_term::MemoryError> {
        self.long_term.get_stats().await
    }

    /// Get short-term memory size
    ///
    /// Returns the current number of entries in short-term memory.
    ///
    /// # Returns
    ///
    /// The number of entries in short-term memory
    pub async fn short_term_size(&self) -> usize {
        let stm = self.short_term.read().await;
        stm.size()
    }

    /// Check if short-term memory is empty
    ///
    /// # Returns
    ///
    /// `true` if short-term memory is empty, `false` otherwise
    pub async fn is_short_term_empty(&self) -> bool {
        let stm = self.short_term.read().await;
        stm.is_empty()
    }

    /// Get entry by ID from long-term memory
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the entry
    ///
    /// # Returns
    ///
    /// `Some(entry)` if found, `None` otherwise
    pub async fn get_by_id(
        &self,
        id: &str,
    ) -> Result<Option<MemoryEntry>, super::long_term::MemoryError> {
        self.long_term.get(id).await
    }

    /// Delete entry from long-term memory
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the entry to delete
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful, `Err(MemoryError)` otherwise
    pub async fn delete(&self, id: &str) -> Result<(), super::long_term::MemoryError> {
        self.long_term.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::MemoryEntry;
    use chrono::Utc;
    use tempfile::TempDir;

    fn create_test_entry(id: &str, content: &str) -> MemoryEntry {
        MemoryEntry {
            id: id.to_string(),
            content: content.to_string(),
            embedding: None,
            timestamp: Utc::now(),
            tags: vec![],
        }
    }

    #[tokio::test]
    async fn test_new_memory_manager() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(10, temp_dir.path().to_path_buf());

        assert_eq!(manager.short_term_size().await, 0);
        assert!(manager.is_short_term_empty().await);
    }

    #[tokio::test]
    async fn test_store_entry() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(10, temp_dir.path().to_path_buf());

        let entry = create_test_entry("1", "Test entry");
        let result = manager.store(entry).await;

        assert!(result.is_ok());
        assert_eq!(manager.short_term_size().await, 1);
        assert!(!manager.is_short_term_empty().await);
    }

    #[tokio::test]
    async fn test_get_recent() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(10, temp_dir.path().to_path_buf());

        for i in 0..5 {
            let entry = create_test_entry(&i.to_string(), &format!("Entry {}", i));
            manager.store(entry).await.unwrap();
        }

        let recent = manager.get_recent(3).await;
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].id, "4");
        assert_eq!(recent[1].id, "3");
        assert_eq!(recent[2].id, "2");
    }

    #[tokio::test]
    async fn test_search() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(10, temp_dir.path().to_path_buf());

        let entry = create_test_entry("1", "Test entry");
        manager.store(entry).await.unwrap();

        let results = manager.search("test", 10).await;
        assert!(results.is_ok());
        // Long-term search returns empty (TODO), but short-term should have results
        let results = results.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_clear_short_term() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(10, temp_dir.path().to_path_buf());

        let entry = create_test_entry("1", "Test entry");
        manager.store(entry).await.unwrap();

        assert_eq!(manager.short_term_size().await, 1);

        manager.clear_short_term().await;

        assert_eq!(manager.short_term_size().await, 0);
        assert!(manager.is_short_term_empty().await);
    }

    #[tokio::test]
    async fn test_get_all_short_term() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(10, temp_dir.path().to_path_buf());

        for i in 0..3 {
            let entry = create_test_entry(&i.to_string(), &format!("Entry {}", i));
            manager.store(entry).await.unwrap();
        }

        let all = manager.get_all_short_term().await;
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "1");
        assert_eq!(all[2].id, "0");
    }

    #[tokio::test]
    async fn test_get_stats() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(10, temp_dir.path().to_path_buf());

        let stats = manager.get_stats().await;
        assert!(stats.is_ok());
        let stats = stats.unwrap();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(10, temp_dir.path().to_path_buf());

        let result = manager.get_by_id("nonexistent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(10, temp_dir.path().to_path_buf());

        let result = manager.delete("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ring_buffer_eviction() {
        let temp_dir = TempDir::new().unwrap();
        let manager = MemoryManager::new(2, temp_dir.path().to_path_buf());

        manager
            .store(create_test_entry("1", "First"))
            .await
            .unwrap();
        manager
            .store(create_test_entry("2", "Second"))
            .await
            .unwrap();
        assert_eq!(manager.short_term_size().await, 2);

        manager
            .store(create_test_entry("3", "Third"))
            .await
            .unwrap();
        assert_eq!(manager.short_term_size().await, 2);

        let recent = manager.get_recent(10).await;
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, "3");
        assert_eq!(recent[1].id, "2");
    }
}
