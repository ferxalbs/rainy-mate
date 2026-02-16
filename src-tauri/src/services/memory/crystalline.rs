use crate::agents::MemoryEntry;
use notify::{Config, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{error, info};
use walkdir::WalkDir;

/// Crystalline Memory - Markdown File-based Memory System
///
/// Watches a directory of markdown files, caches their content,
/// and provides search capabilities.
#[derive(Debug, Clone)]
pub struct CrystallineMemory {
    root_path: PathBuf,
    /// In-memory cache of file content: Path -> Content
    cache: Arc<RwLock<HashMap<PathBuf, String>>>,
    // Inverted index or Vector index could go here.
    // For now, we do a full-scan search which is fast for <10MB text.
}

impl CrystallineMemory {
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize by scanning files and starting the watcher
    pub async fn init(&self) -> Result<(), String> {
        self.scan_all().await;
        self.start_watcher();
        Ok(())
    }

    /// Scan all markdown files in the root path and populate cache
    pub async fn scan_all(&self) {
        info!("Scanning Crystalline memory at {:?}", self.root_path);
        if !self.root_path.exists() {
            if let Err(e) = std::fs::create_dir_all(&self.root_path) {
                error!("Failed to create memory directory: {}", e);
                return;
            }
        }

        let mut cache = self.cache.write().await;
        for entry in WalkDir::new(&self.root_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file()
                && entry.path().extension().map_or(false, |ext| ext == "md")
            {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    cache.insert(entry.path().to_path_buf(), content);
                }
            }
        }
        info!("Crystalline memory loaded {} files", cache.len());
    }

    /// Start the file watcher in a background task
    fn start_watcher(&self) {
        let path = self.root_path.clone();
        let cache = self.cache.clone();

        std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();

            let mut watcher = match notify::RecommendedWatcher::new(tx, Config::default()) {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed to create watcher: {}", e);
                    return;
                }
            };

            if let Err(e) = watcher.watch(&path, RecursiveMode::Recursive) {
                error!("Failed to watch directory {:?}: {}", path, e);
                return;
            }

            info!("Crystalline watcher started on {:?}", path);

            for res in rx {
                match res {
                    Ok(event) => {
                        // Handle event in a blocking way since this is a dedicated thread
                        // For a real production system, debouncing would be added here.
                        use notify::EventKind;
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                for path in event.paths {
                                    if path.extension().map_or(false, |ext| ext == "md") {
                                        if let Ok(content) = std::fs::read_to_string(&path) {
                                            let mut cache_guard =
                                                futures::executor::block_on(cache.write());
                                            cache_guard.insert(path.clone(), content);
                                            // TODO: Trigger vector re-indexing for this file
                                            info!("Updated Crystalline memory for {:?}", path);
                                        }
                                    }
                                }
                            }
                            EventKind::Remove(_) => {
                                for path in event.paths {
                                    let mut cache_guard =
                                        futures::executor::block_on(cache.write());
                                    cache_guard.remove(&path);
                                    info!("Removed from Crystalline memory {:?}", path);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => error!("Watch error: {}", e),
                }
            }
        });
    }

    /// Search for content containing the query string (Case-insensitive)
    /// Returns MemoryEntry objects wrapping the matches.
    pub async fn search(&self, query: &str, limit: usize) -> Vec<MemoryEntry> {
        let cache = self.cache.read().await;
        // Simple linear scan - surprisingly fast for typical agent notes
        // TODO: Upgrade to Tantivy or Vector Search for scalability
        let query_lower = query.to_lowercase();

        let mut results = Vec::new();

        for (path, content) in cache.iter() {
            if content.to_lowercase().contains(&query_lower) {
                results.push(MemoryEntry {
                    id: path.to_string_lossy().to_string(), // Use path as ID
                    content: content.clone(), // Return full content or snippet? Full for now.
                    embedding: None,
                    timestamp: chrono::Utc::now(), // We don't track file mtime in cache yet
                    tags: vec!["crystalline".to_string(), "markdown".to_string()],
                });
            }
        }

        // Sort by relevance? For simple contains, maybe length or just random.
        results.truncate(limit);
        results
    }
}
