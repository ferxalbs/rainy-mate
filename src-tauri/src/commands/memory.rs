//! Tauri commands for memory management
//!
//! This module provides Tauri commands that expose the memory system to the frontend.
//! All commands are thread-safe and use the MemoryManager for operations.

use crate::services::memory::MemoryEntry;
use crate::services::memory::MemoryManager;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

/// State wrapper for MemoryManager
///
/// Wraps the MemoryManager in an Arc for thread-safe access across Tauri commands.
#[derive(Debug, Clone)]
pub struct MemoryManagerState(pub std::sync::Arc<MemoryManager>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeFile {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub indexed_at: i64,
    pub chunk_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeIndexResult {
    pub file: KnowledgeFile,
    pub chunks_indexed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResult {
    pub id: String,
    pub file_id: String,
    pub file_name: String,
    pub file_path: String,
    pub content: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStrategy {
    Vector,
    SimpleBuffer,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AgentKnowledgeStore {
    files: Vec<KnowledgeFile>,
    chunks: Vec<KnowledgeChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeChunk {
    id: String,
    file_id: String,
    content: String,
    indexed_at: i64,
}

fn knowledge_base_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
    Ok(app_dir.join("agent_knowledge"))
}

fn knowledge_store_path(app_handle: &AppHandle, agent_id: &str) -> Result<PathBuf, String> {
    let base = knowledge_base_dir(app_handle)?;
    Ok(base.join(format!("{}.json", agent_id)))
}

fn load_knowledge_store(app_handle: &AppHandle, agent_id: &str) -> Result<AgentKnowledgeStore, String> {
    let path = knowledge_store_path(app_handle, agent_id)?;
    if !path.exists() {
        return Ok(AgentKnowledgeStore::default());
    }
    let body = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read knowledge store {}: {}", path.to_string_lossy(), e))?;
    serde_json::from_str(&body).map_err(|e| format!("Invalid knowledge store json: {}", e))
}

fn save_knowledge_store(
    app_handle: &AppHandle,
    agent_id: &str,
    store: &AgentKnowledgeStore,
) -> Result<(), String> {
    let dir = knowledge_base_dir(app_handle)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create knowledge dir: {}", e))?;
    let path = knowledge_store_path(app_handle, agent_id)?;
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Failed to serialize knowledge store: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write knowledge store {}: {}", path.to_string_lossy(), e))
}

fn split_text_into_chunks(content: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if content.trim().is_empty() {
        return Vec::new();
    }

    let chars: Vec<char> = content.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0usize;

    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        if !chunk.trim().is_empty() {
            chunks.push(chunk);
        }
        if end == chars.len() {
            break;
        }
        start = end.saturating_sub(overlap);
    }

    chunks
}

fn compute_text_score(query: &str, content: &str) -> f32 {
    let query_tokens: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();

    if query_tokens.is_empty() {
        return 0.0;
    }

    let haystack = content.to_lowercase();
    let hits = query_tokens
        .iter()
        .filter(|token| haystack.contains(token.as_str()))
        .count();

    hits as f32 / query_tokens.len() as f32
}

fn file_name_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("knowledge-file")
        .to_string()
}

/// Store an entry in memory
///
/// Stores the entry in both short-term and long-term memory.
///
/// # Arguments
///
/// * `manager` - Memory manager state
/// * `content` - The content of the memory entry
/// * `tags` - Optional tags for categorization
///
/// # Returns
///
/// Success message if stored successfully
///
/// # Example
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const result = await invoke('store_memory', {
///   content: 'Test entry',
///   tags: ['test', 'example']
/// });
/// ```
#[tauri::command]
pub async fn store_memory(
    manager: State<'_, MemoryManagerState>,
    content: String,
    tags: Vec<String>,
) -> Result<String, String> {
    let entry = MemoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        content,
        embedding: None,
        timestamp: chrono::Utc::now(),
        tags,
    };

    manager.0.store(entry).await.map_err(|e| e.to_string())?;

    Ok("Stored successfully".to_string())
}

/// Search memory
///
/// Performs semantic search across both short-term and long-term memory.
///
/// # Arguments
///
/// * `manager` - Memory manager state
/// * `query` - The search query string
/// * `limit` - Maximum number of results to return (default: 10)
///
/// # Returns
///
/// A vector of matching memory entries
///
/// # Example
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const results = await invoke<MemoryEntry[]>('search_memory', {
///   query: 'test query',
///   limit: 10
/// });
/// ```
#[tauri::command]
pub async fn search_memory(
    manager: State<'_, MemoryManagerState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<MemoryEntry>, String> {
    let limit = limit.unwrap_or(10);
    manager
        .0
        .search(&query, limit)
        .await
        .map_err(|e| e.to_string())
}

/// Get recent entries from short-term memory
///
/// Returns the most recent entries from short-term memory only.
///
/// # Arguments
///
/// * `manager` - Memory manager state
/// * `count` - Maximum number of entries to return (default: 10)
///
/// # Returns
///
/// A vector of the most recent entries
///
/// # Example
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const recent = await invoke<MemoryEntry[]>('get_recent_memory', {
///   count: 10
/// });
/// ```
#[tauri::command]
pub async fn get_recent_memory(
    manager: State<'_, MemoryManagerState>,
    count: Option<usize>,
) -> Result<Vec<MemoryEntry>, String> {
    let count = count.unwrap_or(10);
    Ok(manager.0.get_recent(count).await)
}

/// Get all entries from short-term memory
///
/// Returns all entries currently in short-term memory.
///
/// # Arguments
///
/// * `manager` - Memory manager state
///
/// # Returns
///
/// A vector of all entries in short-term memory
///
/// # Example
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const all = await invoke<MemoryEntry[]>('get_all_short_term_memory');
/// ```
#[tauri::command]
pub async fn get_all_short_term_memory(
    manager: State<'_, MemoryManagerState>,
) -> Result<Vec<MemoryEntry>, String> {
    Ok(manager.0.get_all_short_term().await)
}

/// Clear short-term memory
///
/// Removes all entries from short-term memory.
/// Long-term memory is not affected.
///
/// # Arguments
///
/// * `manager` - Memory manager state
///
/// # Returns
///
/// Success message if cleared successfully
///
/// # Example
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('clear_short_term_memory');
/// ```
#[tauri::command]
pub async fn clear_short_term_memory(manager: State<'_, MemoryManagerState>) -> Result<(), String> {
    manager.0.clear_short_term().await;
    Ok(())
}

/// Get memory statistics
///
/// Returns statistics from long-term memory.
///
/// # Arguments
///
/// * `manager` - Memory manager state
///
/// # Returns
///
/// Memory statistics including total entries and storage size
///
/// # Example
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const stats = await invoke<MemoryStats>('get_memory_stats');
/// console.log(`Total entries: ${stats.total_entries}`);
/// ```
#[tauri::command]
pub async fn get_memory_stats(
    manager: State<'_, MemoryManagerState>,
) -> Result<crate::services::memory::long_term::MemoryStats, String> {
    manager.0.get_stats().await.map_err(|e| e.to_string())
}

/// Get entry by ID
///
/// Retrieves a specific memory entry by its unique identifier.
///
/// # Arguments
///
/// * `manager` - Memory manager state
/// * `id` - The unique identifier of the entry
///
/// # Returns
///
/// The memory entry if found, null otherwise
///
/// # Example
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const entry = await invoke<MemoryEntry | null>('get_memory_by_id', {
///   id: 'entry-id'
/// });
/// ```
#[tauri::command]
pub async fn get_memory_by_id(
    manager: State<'_, MemoryManagerState>,
    id: String,
) -> Result<Option<MemoryEntry>, String> {
    manager.0.get_by_id(&id).await.map_err(|e| e.to_string())
}

/// Delete entry from long-term memory
///
/// Removes a memory entry from long-term memory.
///
/// # Arguments
///
/// * `manager` - Memory manager state
/// * `id` - The unique identifier of the entry to delete
///
/// # Returns
///
/// Success message if deleted successfully
///
/// # Example
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('delete_memory', {
///   id: 'entry-id'
/// });
/// ```
#[tauri::command]
pub async fn delete_memory(
    manager: State<'_, MemoryManagerState>,
    id: String,
) -> Result<(), String> {
    manager.0.delete(&id).await.map_err(|e| e.to_string())
}

/// Get short-term memory size
///
/// Returns the current number of entries in short-term memory.
///
/// # Arguments
///
/// * `manager` - Memory manager state
///
/// # Returns
///
/// The number of entries in short-term memory
///
/// # Example
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const size = await invoke<number>('get_short_term_memory_size');
/// console.log(`Short-term memory size: ${size}`);
/// ```
#[tauri::command]
pub async fn get_short_term_memory_size(
    manager: State<'_, MemoryManagerState>,
) -> Result<usize, String> {
    Ok(manager.0.short_term_size().await)
}

/// Check if short-term memory is empty
///
/// # Arguments
///
/// * `manager` - Memory manager state
///
/// # Returns
///
/// `true` if short-term memory is empty, `false` otherwise
///
/// # Example
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const isEmpty = await invoke<boolean>('is_short_term_memory_empty');
/// console.log(`Is empty: ${isEmpty}`);
/// ```
#[tauri::command]
pub async fn is_short_term_memory_empty(
    manager: State<'_, MemoryManagerState>,
) -> Result<bool, String> {
    Ok(manager.0.is_short_term_empty().await)
}

#[tauri::command]
pub async fn index_knowledge_file(
    app_handle: AppHandle,
    manager: State<'_, MemoryManagerState>,
    agent_id: String,
    file_path: String,
) -> Result<KnowledgeIndexResult, String> {
    if agent_id.trim().is_empty() {
        return Err("agent_id is required".to_string());
    }

    let path = PathBuf::from(file_path.clone());
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    let chunks = split_text_into_chunks(&text, 1200, 150);
    if chunks.is_empty() {
        return Err("No indexable text content found in file".to_string());
    }

    let indexed_at = chrono::Utc::now().timestamp();
    let mut store = load_knowledge_store(&app_handle, &agent_id)?;

    let previous_file_ids: std::collections::HashSet<String> = store
        .files
        .iter()
        .filter(|file| file.path == file_path)
        .map(|file| file.id.clone())
        .collect();
    if !previous_file_ids.is_empty() {
        store
            .chunks
            .retain(|chunk| !previous_file_ids.contains(&chunk.file_id));
    }
    store.files.retain(|file| file.path != file_path);

    let file = KnowledgeFile {
        id: uuid::Uuid::new_v4().to_string(),
        name: file_name_from_path(&path),
        path: file_path.clone(),
        size_bytes: bytes.len() as u64,
        indexed_at,
        chunk_count: chunks.len(),
    };

    for (idx, chunk) in chunks.iter().enumerate() {
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content: chunk.clone(),
            embedding: None,
            timestamp: chrono::Utc::now(),
            tags: vec![
                format!("agent:{}", agent_id),
                "knowledge".to_string(),
                format!("knowledge_file:{}", file.id),
                format!("chunk:{}", idx),
            ],
        };
        manager.0.store(entry).await.map_err(|e| e.to_string())?;

        store.chunks.push(KnowledgeChunk {
            id: uuid::Uuid::new_v4().to_string(),
            file_id: file.id.clone(),
            content: chunk.clone(),
            indexed_at,
        });
    }

    store.files.push(file.clone());
    save_knowledge_store(&app_handle, &agent_id, &store)?;

    Ok(KnowledgeIndexResult {
        file: file.clone(),
        chunks_indexed: file.chunk_count,
    })
}

#[tauri::command]
pub async fn query_agent_memory(
    app_handle: AppHandle,
    agent_id: String,
    query: String,
    strategy: Option<MemoryStrategy>,
    limit: Option<usize>,
) -> Result<Vec<MemoryResult>, String> {
    if agent_id.trim().is_empty() {
        return Err("agent_id is required".to_string());
    }
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let store = load_knowledge_store(&app_handle, &agent_id)?;
    if store.chunks.is_empty() {
        return Ok(Vec::new());
    }

    let max_results = limit.unwrap_or(8).max(1).min(50);
    let selected_strategy = strategy.unwrap_or(MemoryStrategy::Hybrid);

    let mut scored: Vec<(KnowledgeChunk, f32)> = match selected_strategy {
        MemoryStrategy::SimpleBuffer => {
            let mut ordered = store.chunks.clone();
            ordered.sort_by(|a, b| b.indexed_at.cmp(&a.indexed_at));
            ordered
                .into_iter()
                .map(|chunk| {
                    let score = compute_text_score(&query, &chunk.content) * 0.7 + 0.3;
                    (chunk, score)
                })
                .collect()
        }
        MemoryStrategy::Vector | MemoryStrategy::Hybrid => store
            .chunks
            .clone()
            .into_iter()
            .map(|chunk| {
                let score = compute_text_score(&query, &chunk.content);
                (chunk, score)
            })
            .collect(),
    };

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(max_results);

    let mut results = Vec::with_capacity(scored.len());
    for (chunk, score) in scored {
        let file = store
            .files
            .iter()
            .find(|file| file.id == chunk.file_id)
            .cloned()
            .unwrap_or(KnowledgeFile {
                id: chunk.file_id.clone(),
                name: "Unknown".to_string(),
                path: "".to_string(),
                size_bytes: 0,
                indexed_at: chunk.indexed_at,
                chunk_count: 0,
            });

        results.push(MemoryResult {
            id: chunk.id,
            file_id: file.id,
            file_name: file.name,
            file_path: file.path,
            content: chunk.content,
            score,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::memory::MemoryManager;
    use tempfile::TempDir;

    fn create_test_manager() -> MemoryManagerState {
        let temp_dir = TempDir::new().unwrap();
        MemoryManagerState(std::sync::Arc::new(MemoryManager::new(
            10,
            temp_dir.path().to_path_buf(),
        )))
    }

    // Helper function to simulate Tauri State
    fn as_state<'a>(manager: &'a MemoryManagerState) -> State<'a, MemoryManagerState> {
        unsafe { std::mem::transmute_copy(&manager) }
    }

    #[tokio::test]
    async fn test_store_memory_command() {
        let manager = create_test_manager();

        let result = store_memory(
            as_state(&manager),
            "Test entry".to_string(),
            vec!["test".to_string()],
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Stored successfully");
    }

    #[tokio::test]
    async fn test_search_memory_command() {
        let manager = create_test_manager();

        // Store an entry first
        store_memory(as_state(&manager), "Test entry".to_string(), vec![])
            .await
            .unwrap();

        let result = search_memory(as_state(&manager), "test".to_string(), Some(10)).await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_get_recent_memory_command() {
        let manager = create_test_manager();

        // Store some entries
        for i in 0..3 {
            store_memory(as_state(&manager), format!("Entry {}", i), vec![])
                .await
                .unwrap();
        }

        let result = get_recent_memory(as_state(&manager), Some(2)).await;

        assert!(result.is_ok());
        let recent = result.unwrap();
        assert_eq!(recent.len(), 2);
    }

    #[tokio::test]
    async fn test_clear_short_term_memory_command() {
        let manager = create_test_manager();

        // Store an entry
        store_memory(as_state(&manager), "Test entry".to_string(), vec![])
            .await
            .unwrap();

        // Clear memory
        let result = clear_short_term_memory(as_state(&manager)).await;
        assert!(result.is_ok());

        // Verify it's empty
        let size = get_short_term_memory_size(as_state(&manager))
            .await
            .unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_get_memory_stats_command() {
        let manager = create_test_manager();

        let result = get_memory_stats(as_state(&manager)).await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_get_short_term_memory_size_command() {
        let manager = create_test_manager();

        let size = get_short_term_memory_size(as_state(&manager))
            .await
            .unwrap();
        assert_eq!(size, 0);

        // Store an entry
        store_memory(as_state(&manager), "Test entry".to_string(), vec![])
            .await
            .unwrap();

        let size = get_short_term_memory_size(as_state(&manager))
            .await
            .unwrap();
        assert_eq!(size, 1);
    }

    #[tokio::test]
    async fn test_is_short_term_memory_empty_command() {
        let manager = create_test_manager();

        let is_empty = is_short_term_memory_empty(as_state(&manager))
            .await
            .unwrap();
        assert!(is_empty);

        // Store an entry
        store_memory(as_state(&manager), "Test entry".to_string(), vec![])
            .await
            .unwrap();

        let is_empty = is_short_term_memory_empty(as_state(&manager))
            .await
            .unwrap();
        assert!(!is_empty);
    }
}
