use crate::services::memory::{MemoryEntry, MemoryError, MemoryStats};
use crate::services::memory_vault::{MemorySensitivity, MemoryVaultService, StoreMemoryInput};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct MemoryManager {
    short_term: Arc<RwLock<VecDeque<MemoryEntry>>>,
    short_term_capacity: usize,
    app_data_dir: PathBuf,
    vault: Arc<RwLock<Option<Arc<MemoryVaultService>>>>,
}

impl MemoryManager {
    pub fn new(short_term_size: usize, long_term_path: PathBuf) -> Self {
        let app_data_dir = long_term_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(long_term_path);
        Self {
            short_term: Arc::new(RwLock::new(VecDeque::with_capacity(short_term_size))),
            short_term_capacity: short_term_size.max(1),
            app_data_dir,
            vault: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn init(&self) {
        let _ = self.ensure_vault().await;
    }

    async fn ensure_vault(&self) -> Result<Arc<MemoryVaultService>, MemoryError> {
        {
            let guard = self.vault.read().await;
            if let Some(vault) = guard.as_ref() {
                return Ok(vault.clone());
            }
        }

        let mut guard = self.vault.write().await;
        if let Some(vault) = guard.as_ref() {
            return Ok(vault.clone());
        }

        let created = Arc::new(
            MemoryVaultService::new(self.app_data_dir.clone())
                .await
                .map_err(MemoryError::Other)?,
        );
        *guard = Some(created.clone());
        Ok(created)
    }

    pub async fn store(&self, entry: MemoryEntry) -> Result<(), MemoryError> {
        {
            let mut stm = self.short_term.write().await;
            stm.push_back(entry.clone());
            while stm.len() > self.short_term_capacity {
                let _ = stm.pop_front();
            }
        }

        let vault = self.ensure_vault().await?;
        let now = entry.timestamp.timestamp();
        let workspace_id = derive_workspace_id(&entry.tags);
        let metadata = HashMap::new();
        let source = derive_source(&entry.tags);

        vault
            .put(StoreMemoryInput {
                id: entry.id,
                workspace_id,
                content: entry.content,
                tags: entry.tags,
                source,
                sensitivity: MemorySensitivity::Internal,
                metadata,
                created_at: now,
                embedding: None,
            })
            .await
            .map_err(MemoryError::Other)?;

        Ok(())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>, MemoryError> {
        let workspace_id = derive_workspace_id_from_query(query);
        let vault = self.ensure_vault().await?;
        let results = vault
            .search_workspace(&workspace_id, query, limit.max(1))
            .await
            .map_err(MemoryError::Other)?;

        Ok(results
            .into_iter()
            .map(|entry| MemoryEntry {
                id: entry.id,
                content: entry.content,
                embedding: None,
                timestamp: chrono::DateTime::from_timestamp(entry.created_at, 0)
                    .unwrap_or_else(chrono::Utc::now),
                tags: entry.tags,
            })
            .collect())
    }

    pub async fn get_recent(&self, count: usize) -> Vec<MemoryEntry> {
        let stm = self.short_term.read().await;
        stm.iter().rev().take(count).cloned().collect()
    }

    pub async fn get_all_short_term(&self) -> Vec<MemoryEntry> {
        let stm = self.short_term.read().await;
        stm.iter().cloned().collect()
    }

    pub async fn clear_short_term(&self) {
        let mut stm = self.short_term.write().await;
        stm.clear();
    }

    pub async fn get_stats(&self) -> Result<MemoryStats, MemoryError> {
        let vault = self.ensure_vault().await?;
        let stats = vault.stats(None).await.map_err(MemoryError::Other)?;

        Ok(MemoryStats {
            total_entries: stats.total_entries,
            total_size: 0,
        })
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let vault = self.ensure_vault().await?;
        let maybe = vault.get_by_id(id).await.map_err(MemoryError::Other)?;

        Ok(maybe.map(|entry| MemoryEntry {
            id: entry.id,
            content: entry.content,
            embedding: None,
            timestamp: chrono::DateTime::from_timestamp(entry.created_at, 0)
                .unwrap_or_else(chrono::Utc::now),
            tags: entry.tags,
        }))
    }

    pub async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        let vault = self.ensure_vault().await?;
        vault.delete_by_id(id).await.map_err(MemoryError::Other)
    }

    pub async fn short_term_size(&self) -> usize {
        let stm = self.short_term.read().await;
        stm.len()
    }

    pub async fn is_short_term_empty(&self) -> bool {
        self.short_term_size().await == 0
    }

    pub async fn query_workspace_memory(
        &self,
        workspace_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let vault = self.ensure_vault().await?;
        let rows = vault
            .search_workspace(workspace_id, query, limit.max(1))
            .await
            .map_err(MemoryError::Other)?;

        Ok(rows
            .into_iter()
            .map(|entry| MemoryEntry {
                id: entry.id,
                content: entry.content,
                embedding: None,
                timestamp: chrono::DateTime::from_timestamp(entry.created_at, 0)
                    .unwrap_or_else(chrono::Utc::now),
                tags: entry.tags,
            })
            .collect())
    }
    pub async fn search_semantic(
        &self,
        workspace_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        // We will need the embedder
        // To be safe and clean, we instantiate it dynamically matching the Vault backfill logic
        // Ideally we'd pull these from settings
        let provider = std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "gemini".to_string());
        let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
        let model =
            std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| "gemini-embedding-001".to_string());

        // Return fallback context query if no API key is available
        if api_key.is_empty() {
            return self
                .query_workspace_memory(workspace_id, query, limit)
                .await;
        }

        let embedder =
            crate::services::embedder::EmbedderService::new(provider, api_key, Some(model));
        let query_embedding = embedder
            .embed_text(query)
            .await
            .map_err(|e| MemoryError::Other(e))?;

        let vault = self.ensure_vault().await?;
        let rows = vault
            .search_workspace_vector(workspace_id, &query_embedding, limit.max(1))
            .await
            .map_err(|e| MemoryError::Other(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(entry, _distance)| MemoryEntry {
                id: entry.id,
                content: entry.content,
                embedding: None,
                timestamp: chrono::DateTime::from_timestamp(entry.created_at, 0)
                    .unwrap_or_else(chrono::Utc::now),
                tags: entry.tags,
            })
            .collect())
    }

    pub async fn ingest_text(
        &self,
        workspace_id: &str,
        source_path: &str,
        text: &str,
        mut raw_tags: Option<Vec<String>>,
    ) -> Result<usize, MemoryError> {
        let vault = self.ensure_vault().await?;

        let provider = std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "gemini".to_string());
        let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
        let model =
            std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| "gemini-embedding-001".to_string());

        let embedder = if api_key.is_empty() {
            None
        } else {
            Some(crate::services::embedder::EmbedderService::new(
                provider,
                api_key,
                Some(model),
            ))
        };

        // Highly simplified chunking for the seed implementation (e.g. 1000 chars)
        // In a real system, we'd use semantic chunking or token-based chunking
        let chunks: Vec<String> = text
            .chars()
            .collect::<Vec<char>>()
            .chunks(1500)
            .map(|c| c.into_iter().collect())
            .filter(|c: &String| !c.trim().is_empty())
            .collect();

        let mut ingested_count = 0;

        let mut tags_out = vec![
            format!("workspace:{}", workspace_id),
            format!("source:{}", source_path),
            "type:document".to_string(),
        ];

        if let Some(mut user_tags) = raw_tags.take() {
            tags_out.append(&mut user_tags);
        }

        for chunk in &chunks {
            let embedding = if let Some(ref e) = embedder {
                e.embed_text(chunk).await.ok()
            } else {
                None
            };

            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().timestamp();

            vault
                .put(StoreMemoryInput {
                    id: id.clone(),
                    workspace_id: workspace_id.to_string(),
                    content: chunk.clone(),
                    tags: tags_out.clone(),
                    source: source_path.to_string(),
                    sensitivity: MemorySensitivity::Internal,
                    metadata: HashMap::new(),
                    created_at: now,
                    embedding,
                })
                .await
                .map_err(MemoryError::Other)?;

            ingested_count += 1;
        }

        Ok(ingested_count)
    }
}

fn derive_workspace_id(tags: &[String]) -> String {
    for tag in tags {
        if let Some(rest) = tag.strip_prefix("workspace:") {
            return rest.to_string();
        }
        if let Some(rest) = tag.strip_prefix("agent:") {
            return rest.to_string();
        }
    }
    "global".to_string()
}

fn derive_source(tags: &[String]) -> String {
    tags.iter()
        .find_map(|tag| tag.strip_prefix("source:").map(|v| v.to_string()))
        .unwrap_or_else(|| "memory_manager".to_string())
}

fn derive_workspace_id_from_query(_query: &str) -> String {
    "global".to_string()
}
