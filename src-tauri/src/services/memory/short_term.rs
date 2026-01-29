//! Short-term memory implementation using a ring buffer
//!
//! This module provides fast, in-memory storage for recent memory entries.
//! It uses a ring buffer (VecDeque) to maintain a fixed-size cache of the most
//! recent entries, automatically evicting the oldest entries when the buffer is full.

use crate::agents::MemoryEntry;
use std::collections::VecDeque;

/// Short-term memory with ring buffer implementation
///
/// Maintains a fixed-size buffer of the most recent memory entries.
/// When the buffer reaches capacity, the oldest entry is automatically removed.
///
/// # Example
///
/// ```rust,no_run
/// use crate::services::memory::short_term::ShortTermMemory;
/// use crate::agents::MemoryEntry;
/// use chrono::Utc;
///
/// let mut memory = ShortTermMemory::new(100);
///
/// let entry = MemoryEntry {
///     id: "1".to_string(),
///     content: "Test entry".to_string(),
///     embedding: None,
///     timestamp: Utc::now(),
///     tags: vec!["test".to_string()],
/// };
///
/// memory.add(entry);
/// let recent = memory.get_recent(10);
/// ```
#[derive(Debug, Clone)]
pub struct ShortTermMemory {
    /// Ring buffer storing memory entries
    buffer: VecDeque<MemoryEntry>,
    /// Maximum number of entries to store
    max_size: usize,
}

impl ShortTermMemory {
    /// Create a new short-term memory with the specified capacity
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum number of entries to store
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::services::memory::short_term::ShortTermMemory;
    ///
    /// let memory = ShortTermMemory::new(100);
    /// assert_eq!(memory.size(), 0);
    /// ```
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Add an entry to memory
    ///
    /// If the buffer is at capacity, the oldest entry is removed before adding the new one.
    ///
    /// # Arguments
    ///
    /// * `entry` - The memory entry to add
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::services::memory::short_term::ShortTermMemory;
    /// use crate::agents::MemoryEntry;
    /// use chrono::Utc;
    ///
    /// let mut memory = ShortTermMemory::new(2);
    ///
    /// let entry1 = MemoryEntry {
    ///     id: "1".to_string(),
    ///     content: "First".to_string(),
    ///     embedding: None,
    ///     timestamp: Utc::now(),
    ///     tags: vec![],
    /// };
    ///
    /// let entry2 = MemoryEntry {
    ///     id: "2".to_string(),
    ///     content: "Second".to_string(),
    ///     embedding: None,
    ///     timestamp: Utc::now(),
    ///     tags: vec![],
    /// };
    ///
    /// let entry3 = MemoryEntry {
    ///     id: "3".to_string(),
    ///     content: "Third".to_string(),
    ///     embedding: None,
    ///     timestamp: Utc::now(),
    ///     tags: vec![],
    /// };
    ///
    /// memory.add(entry1);
    /// memory.add(entry2);
    /// memory.add(entry3);
    ///
    /// // First entry should be evicted
    /// assert_eq!(memory.size(), 2);
    /// ```
    pub fn add(&mut self, entry: MemoryEntry) {
        if self.buffer.len() >= self.max_size {
            self.buffer.pop_front();
        }
        self.buffer.push_back(entry);
    }

    /// Get the most recent entries
    ///
    /// Returns entries in reverse chronological order (most recent first).
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of entries to return
    ///
    /// # Returns
    ///
    /// A vector of the most recent entries, up to `count` entries
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::services::memory::short_term::ShortTermMemory;
    /// use crate::agents::MemoryEntry;
    /// use chrono::Utc;
    ///
    /// let mut memory = ShortTermMemory::new(10);
    ///
    /// for i in 0..5 {
    ///     memory.add(MemoryEntry {
    ///         id: i.to_string(),
    ///         content: format!("Entry {}", i),
    ///         embedding: None,
    ///         timestamp: Utc::now(),
    ///         tags: vec![],
    ///     });
    /// }
    ///
    /// let recent = memory.get_recent(3);
    /// assert_eq!(recent.len(), 3);
    /// assert_eq!(recent[0].id, "4"); // Most recent
    /// ```
    pub fn get_recent(&self, count: usize) -> Vec<MemoryEntry> {
        self.buffer.iter().rev().take(count).cloned().collect()
    }

    /// Get all entries in memory
    ///
    /// Returns all entries in reverse chronological order (most recent first).
    ///
    /// # Returns
    ///
    /// A vector of all entries
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::services::memory::short_term::ShortTermMemory;
    /// use crate::agents::MemoryEntry;
    /// use chrono::Utc;
    ///
    /// let mut memory = ShortTermMemory::new(10);
    ///
    /// for i in 0..3 {
    ///     memory.add(MemoryEntry {
    ///         id: i.to_string(),
    ///         content: format!("Entry {}", i),
    ///         embedding: None,
    ///         timestamp: Utc::now(),
    ///         tags: vec![],
    ///     });
    /// }
    ///
    /// let all = memory.get_all();
    /// assert_eq!(all.len(), 3);
    /// ```
    pub fn get_all(&self) -> Vec<MemoryEntry> {
        self.buffer.iter().rev().cloned().collect()
    }

    /// Clear all entries from memory
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::services::memory::short_term::ShortTermMemory;
    /// use crate::agents::MemoryEntry;
    /// use chrono::Utc;
    ///
    /// let mut memory = ShortTermMemory::new(10);
    ///
    /// memory.add(MemoryEntry {
    ///     id: "1".to_string(),
    ///     content: "Test".to_string(),
    ///     embedding: None,
    ///     timestamp: Utc::now(),
    ///     tags: vec![],
    /// });
    ///
    /// assert_eq!(memory.size(), 1);
    /// memory.clear();
    /// assert_eq!(memory.size(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get the current number of entries in memory
    ///
    /// # Returns
    ///
    /// The number of entries currently stored
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::services::memory::short_term::ShortTermMemory;
    ///
    /// let memory = ShortTermMemory::new(10);
    /// assert_eq!(memory.size(), 0);
    /// ```
    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    /// Check if memory is empty
    ///
    /// # Returns
    ///
    /// `true` if no entries are stored, `false` otherwise
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get the maximum capacity of the memory
    ///
    /// # Returns
    ///
    /// The maximum number of entries that can be stored
    #[allow(dead_code)]
    pub fn capacity(&self) -> usize {
        self.max_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_entry(id: &str, content: &str) -> MemoryEntry {
        MemoryEntry {
            id: id.to_string(),
            content: content.to_string(),
            embedding: None,
            timestamp: Utc::now(),
            tags: vec![],
        }
    }

    #[test]
    fn test_new_memory() {
        let memory = ShortTermMemory::new(10);
        assert_eq!(memory.size(), 0);
        assert_eq!(memory.capacity(), 10);
        assert!(memory.is_empty());
    }

    #[test]
    fn test_add_entry() {
        let mut memory = ShortTermMemory::new(10);
        let entry = create_test_entry("1", "Test");
        memory.add(entry);
        assert_eq!(memory.size(), 1);
        assert!(!memory.is_empty());
    }

    #[test]
    fn test_ring_buffer_eviction() {
        let mut memory = ShortTermMemory::new(2);

        memory.add(create_test_entry("1", "First"));
        memory.add(create_test_entry("2", "Second"));
        assert_eq!(memory.size(), 2);

        memory.add(create_test_entry("3", "Third"));
        assert_eq!(memory.size(), 2);

        let recent = memory.get_recent(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, "3");
        assert_eq!(recent[1].id, "2");
    }

    #[test]
    fn test_get_recent() {
        let mut memory = ShortTermMemory::new(10);

        for i in 0..5 {
            memory.add(create_test_entry(&i.to_string(), &format!("Entry {}", i)));
        }

        let recent = memory.get_recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].id, "4");
        assert_eq!(recent[1].id, "3");
        assert_eq!(recent[2].id, "2");
    }

    #[test]
    fn test_get_recent_more_than_available() {
        let mut memory = ShortTermMemory::new(10);

        for i in 0..3 {
            memory.add(create_test_entry(&i.to_string(), &format!("Entry {}", i)));
        }

        let recent = memory.get_recent(10);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_get_all() {
        let mut memory = ShortTermMemory::new(10);

        for i in 0..3 {
            memory.add(create_test_entry(&i.to_string(), &format!("Entry {}", i)));
        }

        let all = memory.get_all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "1");
        assert_eq!(all[2].id, "0");
    }

    #[test]
    fn test_clear() {
        let mut memory = ShortTermMemory::new(10);

        memory.add(create_test_entry("1", "Test"));
        assert_eq!(memory.size(), 1);

        memory.clear();
        assert_eq!(memory.size(), 0);
        assert!(memory.is_empty());
    }

    #[test]
    fn test_capacity() {
        let memory = ShortTermMemory::new(100);
        assert_eq!(memory.capacity(), 100);
    }

    #[test]
    fn test_empty_memory_get_recent() {
        let memory = ShortTermMemory::new(10);
        let recent = memory.get_recent(5);
        assert_eq!(recent.len(), 0);
    }

    #[test]
    fn test_empty_memory_get_all() {
        let memory = ShortTermMemory::new(10);
        let all = memory.get_all();
        assert_eq!(all.len(), 0);
    }
}
