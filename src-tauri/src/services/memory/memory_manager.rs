use crate::services::memory::{
    IngestionResult, MemoryEntry, MemoryError, MemoryStats, SemanticRetrievalMode,
    SemanticSearchResult,
};
use crate::services::memory_vault::{MemorySensitivity, MemoryVaultService, StoreMemoryInput};
use crate::services::memory_vault::{EMBEDDING_MODEL, EMBEDDING_PROVIDER};
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
    const MAX_INGEST_CHUNKS: usize = 2048;
    const DEFAULT_CHUNK_CHARS: usize = 1500;

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
                embedding_model: Some(EMBEDDING_MODEL.to_string()),
                embedding_provider: Some(EMBEDDING_PROVIDER.to_string()),
                embedding_dim: Some(crate::services::memory_vault::EMBEDDING_DIM),
                additional_embeddings: Vec::new(),
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
    pub async fn search_semantic_detailed(
        &self,
        workspace_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<SemanticSearchResult, MemoryError> {
        let embedder = match self.resolve_gemini_embedder() {
            Ok(Some(embedder)) => embedder,
            Ok(None) => {
                let entries = self
                    .query_workspace_memory(workspace_id, query, limit)
                    .await?;
                return Ok(SemanticSearchResult {
                    entries,
                    mode: SemanticRetrievalMode::LexicalFallback,
                    reason: Some("Missing Gemini embedding API key".to_string()),
                });
            }
            Err(reason) => {
                let entries = self
                    .query_workspace_memory(workspace_id, query, limit)
                    .await?;
                return Ok(SemanticSearchResult {
                    entries,
                    mode: SemanticRetrievalMode::LexicalFallback,
                    reason: Some(reason),
                });
            }
        };

        let query_embedding = match embedder.embed_text(query).await {
            Ok(v) => v,
            Err(e) => {
                let entries = self
                    .query_workspace_memory(workspace_id, query, limit)
                    .await?;
                return Ok(SemanticSearchResult {
                    entries,
                    mode: SemanticRetrievalMode::LexicalFallback,
                    reason: Some(format!("Gemini embedding request failed: {}", e)),
                });
            }
        };

        let vault = self.ensure_vault().await?;
        let (rows, mode) = vault
            .search_workspace_vector_with_mode(workspace_id, &query_embedding, limit.max(1))
            .await
            .map_err(MemoryError::Other)?;

        let mode = match mode {
            crate::services::memory_vault::service::VectorSearchMode::Ann => {
                SemanticRetrievalMode::Ann
            }
            crate::services::memory_vault::service::VectorSearchMode::Exact => {
                SemanticRetrievalMode::Exact
            }
        };

        Ok(SemanticSearchResult {
            entries: rows
                .into_iter()
                .map(|(entry, _distance)| MemoryEntry {
                    id: entry.id,
                    content: entry.content,
                    embedding: None,
                    timestamp: chrono::DateTime::from_timestamp(entry.created_at, 0)
                        .unwrap_or_else(chrono::Utc::now),
                    tags: entry.tags,
                })
                .collect(),
            mode,
            reason: None,
        })
    }

    pub async fn ingest_text_detailed(
        &self,
        workspace_id: &str,
        source_path: &str,
        text: &str,
        mut raw_tags: Option<Vec<String>>,
    ) -> Result<IngestionResult, MemoryError> {
        let vault = self.ensure_vault().await?;

        let embedder = self.resolve_gemini_embedder().map_err(MemoryError::Other)?;
        let mut warnings = Vec::new();
        if embedder.is_none() {
            warnings.push("Gemini embedding API key unavailable; storing chunks without embeddings".to_string());
        }

        let chunks: Vec<String> = text
            .chars()
            .collect::<Vec<char>>()
            .chunks(Self::DEFAULT_CHUNK_CHARS)
            .map(|c| c.into_iter().collect())
            .filter(|c: &String| !c.trim().is_empty())
            .take(Self::MAX_INGEST_CHUNKS)
            .collect();

        let total_possible_chunks = text.chars().count().div_ceil(Self::DEFAULT_CHUNK_CHARS);
        if total_possible_chunks > Self::MAX_INGEST_CHUNKS {
            warnings.push(format!(
                "Document exceeded max chunk limit ({}); ingestion truncated",
                Self::MAX_INGEST_CHUNKS
            ));
        }

        let mut ingested_count = 0;
        let mut embedded_count = 0;
        let doc_id = uuid::Uuid::new_v4().to_string();

        let mut tags_out = vec![
            format!("workspace:{}", workspace_id),
            format!("source:{}", source_path),
            "type:document".to_string(),
            format!("doc:{}", doc_id),
        ];

        if let Some(mut user_tags) = raw_tags.take() {
            tags_out.append(&mut user_tags);
        }

        let chunk_count = chunks.len();
        for (idx, chunk) in chunks.iter().enumerate() {
            let mut embedding = None;
            let mut embedding_model = EMBEDDING_MODEL.to_string();
            let mut additional_embeddings = Vec::new();
            if let Some(ref e) = embedder {
                let active_model = crate::services::memory_vault::profiles::ACTIVE_EMBEDDING_PROFILE.model;
                let fallback_model =
                    crate::services::memory_vault::profiles::FALLBACK_EMBEDDING_PROFILE.model;
                match e.embed_text_for_model(chunk, active_model).await {
                    Ok(vec) => {
                        embedding = Some(vec);
                        if fallback_model != active_model {
                            if let Ok(fallback_vec) =
                                e.embed_text_for_model(chunk, fallback_model).await
                            {
                                additional_embeddings.push(
                                    crate::services::memory_vault::AdditionalEmbeddingInput {
                                        embedding: fallback_vec,
                                        embedding_model: fallback_model.to_string(),
                                        embedding_provider: EMBEDDING_PROVIDER.to_string(),
                                        embedding_dim: crate::services::memory_vault::EMBEDDING_DIM,
                                    },
                                );
                            }
                        }
                    }
                    Err(_) => {
                        if fallback_model != active_model {
                            if let Ok(vec) = e.embed_text_for_model(chunk, fallback_model).await {
                                embedding = Some(vec);
                                embedding_model = fallback_model.to_string();
                            }
                        }
                    }
                }
            }
            if embedding.is_some() {
                embedded_count += 1;
            }

            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().timestamp();
            let mut metadata = HashMap::new();
            metadata.insert("doc_id".to_string(), doc_id.clone());
            metadata.insert("source_path".to_string(), source_path.to_string());
            metadata.insert("chunk_index".to_string(), idx.to_string());
            metadata.insert("chunk_count".to_string(), chunk_count.to_string());

            vault
                .put(StoreMemoryInput {
                    id: id.clone(),
                    workspace_id: workspace_id.to_string(),
                    content: chunk.clone(),
                    tags: tags_out.clone(),
                    source: source_path.to_string(),
                    sensitivity: MemorySensitivity::Internal,
                    metadata,
                    created_at: now,
                    embedding,
                    embedding_model: Some(embedding_model),
                    embedding_provider: Some(EMBEDDING_PROVIDER.to_string()),
                    embedding_dim: Some(crate::services::memory_vault::EMBEDDING_DIM),
                    additional_embeddings,
                })
                .await
                .map_err(MemoryError::Other)?;

            ingested_count += 1;
        }

        Ok(IngestionResult {
            chunks_ingested: ingested_count,
            chunks_embedded: embedded_count,
            embedding_mode: if embedded_count > 0 {
                format!("{}:{}", EMBEDDING_PROVIDER, EMBEDDING_MODEL)
            } else {
                "none".to_string()
            },
            warnings,
        })
    }

    fn resolve_gemini_embedder(
        &self,
    ) -> Result<Option<crate::services::embedder::EmbedderService>, String> {
        let settings = crate::services::settings::SettingsManager::new();
        let provider_raw = settings.get_embedder_provider().to_string();
        let provider = match provider_raw.trim().to_lowercase().as_str() {
            "g" | "google" | "gemini" => EMBEDDING_PROVIDER.to_string(),
            _ => {
                return Err(format!(
                    "Unsupported embedding provider '{}' for STEP 3; Gemini is required",
                    provider_raw
                ));
            }
        };

        let keychain = crate::ai::keychain::KeychainManager::new();
        let api_key = keychain
            .get_key(EMBEDDING_PROVIDER)
            .or_else(|_| keychain.get_key(&provider_raw))
            .unwrap_or_default()
            .unwrap_or_default();

        if api_key.trim().is_empty() {
            return Ok(None);
        }

        Ok(Some(crate::services::embedder::EmbedderService::new(
            provider,
            api_key,
            Some(EMBEDDING_MODEL.to_string()),
        )))
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
