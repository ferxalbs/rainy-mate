use crate::services::embedder::{EmbedderService, EmbeddingTaskType};
use crate::services::memory::{
    IngestionResult, MemoryEntry, MemoryError, MemoryStats, SemanticRetrievalMode,
    SemanticSearchResult,
};
use crate::services::memory_vault::{MemorySensitivity, MemoryVaultService, StoreMemoryInput};
use crate::services::memory_vault::{EMBEDDING_MODEL, EMBEDDING_PROVIDER};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

const SEMANTIC_SEARCH_TIMEOUT_MS: u64 = 4000;

#[derive(Debug, Clone)]
pub struct MemoryManager {
    short_term: Arc<RwLock<VecDeque<MemoryEntry>>>,
    short_term_capacity: usize,
    vault_dir: PathBuf,
    vault: Arc<RwLock<Option<Arc<MemoryVaultService>>>>,
    embedder_cache: Arc<OnceLock<Option<Arc<EmbedderService>>>>,
}

impl MemoryManager {
    const MAX_INGEST_CHUNKS: usize = 2048;
    const DEFAULT_CHUNK_CHARS: usize = 1500;

    pub fn new(short_term_size: usize, vault_dir: PathBuf) -> Self {
        Self {
            short_term: Arc::new(RwLock::new(VecDeque::with_capacity(short_term_size))),
            short_term_capacity: short_term_size.max(1),
            vault_dir,
            vault: Arc::new(RwLock::new(None)),
            embedder_cache: Arc::new(OnceLock::new()),
        }
    }

    pub async fn init(&self) {
        // Pre-warm embedder cache so the first search doesn't pay Keychain init cost.
        let _ = self.resolve_gemini_embedder();
        if let Ok(vault) = self.ensure_vault().await {
            vault.spawn_reembed_backfill();
            // Startup safety-net: prune entries older than 365 days across all workspaces.
            let vault_for_prune = vault.clone();
            tokio::spawn(async move {
                match vault_for_prune.prune_global_expired(365).await {
                    Ok(0) => {}
                    Ok(n) => {
                        tracing::info!("Startup pruning: removed {} globally expired entries", n)
                    }
                    Err(e) => tracing::warn!("Startup global pruning failed: {}", e),
                }
            });
        }
    }

    /// Get the vault reference if initialized, without creating it.
    pub async fn get_vault(&self) -> Option<Arc<MemoryVaultService>> {
        self.vault.read().await.clone()
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
            MemoryVaultService::new(self.vault_dir.clone())
                .await
                .map_err(MemoryError::Other)?,
        );
        *guard = Some(created.clone());
        Ok(created)
    }

    pub async fn store(&self, entry: MemoryEntry) -> Result<(), MemoryError> {
        let workspace_id = derive_workspace_id(&entry.tags);
        let source = derive_source(&entry.tags);
        self.store_workspace_memory(
            &workspace_id,
            entry.id,
            entry.content,
            source,
            entry.tags,
            HashMap::new(),
            entry.timestamp.timestamp(),
            MemorySensitivity::Internal,
        )
        .await
    }

    pub async fn store_workspace_memory(
        &self,
        workspace_id: &str,
        id: String,
        content: String,
        source: String,
        tags: Vec<String>,
        metadata: HashMap<String, String>,
        created_at: i64,
        sensitivity: MemorySensitivity,
    ) -> Result<(), MemoryError> {
        {
            let mut stm = self.short_term.write().await;
            stm.push_back(MemoryEntry {
                id: id.clone(),
                content: content.clone(),
                embedding: None,
                timestamp: chrono::DateTime::from_timestamp(created_at, 0)
                    .unwrap_or_else(chrono::Utc::now),
                tags: tags.clone(),
            });
            while stm.len() > self.short_term_capacity {
                let _ = stm.pop_front();
            }
        }

        let vault = self.ensure_vault().await?;

        vault
            .put(StoreMemoryInput {
                id: id.clone(),
                workspace_id: workspace_id.to_string(),
                content: content.clone(),
                tags: tags.clone(),
                source: source.clone(),
                sensitivity: sensitivity.clone(),
                metadata: metadata.clone(),
                created_at,
                embedding: None,
                embedding_model: Some(EMBEDDING_MODEL.to_string()),
                embedding_provider: Some(EMBEDDING_PROVIDER.to_string()),
                embedding_dim: Some(crate::services::memory_vault::EMBEDDING_DIM),
                additional_embeddings: Vec::new(),
            })
            .await
            .map_err(MemoryError::Other)?;

        // Embed in the background so the write path is non-blocking. The upsert in put()
        // uses INSERT OR REPLACE so the second call simply attaches the embedding.
        if let Ok(Some(embedder)) = self.resolve_gemini_embedder() {
            let vault_bg = vault.clone();
            let workspace_id = workspace_id.to_string();
            tokio::spawn(async move {
                if let Ok(vec) = embedder
                    .embed_text_with_task(&content, EmbeddingTaskType::RetrievalDocument)
                    .await
                {
                    let _ = vault_bg
                        .put(StoreMemoryInput {
                            id,
                            workspace_id,
                            content,
                            tags,
                            source,
                            sensitivity,
                            metadata,
                            created_at,
                            embedding: Some(vec),
                            embedding_model: Some(EMBEDDING_MODEL.to_string()),
                            embedding_provider: Some(EMBEDDING_PROVIDER.to_string()),
                            embedding_dim: Some(crate::services::memory_vault::EMBEDDING_DIM),
                            additional_embeddings: Vec::new(),
                        })
                        .await;
                }
            });
        }

        Ok(())
    }

    pub async fn search(
        &self,
        workspace_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let result = self
            .search_semantic_detailed(workspace_id, query, limit, "hybrid")
            .await?;
        Ok(result.entries)
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

    /// Delete vault entries older than `retention_days` for a workspace.
    /// No-op when `retention_days` is 0. Spawned as a background task by the agent runtime.
    pub async fn prune_expired(
        &self,
        workspace_id: &str,
        retention_days: u32,
    ) -> Result<u64, MemoryError> {
        if retention_days == 0 {
            return Ok(0);
        }
        let vault = self.ensure_vault().await?;
        vault
            .prune_workspace_expired(workspace_id, retention_days)
            .await
            .map_err(MemoryError::Other)
    }

    pub async fn short_term_size(&self) -> usize {
        let stm = self.short_term.read().await;
        stm.len()
    }

    pub async fn is_short_term_empty(&self) -> bool {
        self.short_term_size().await == 0
    }

    /// Expose the underlying MemoryVaultService for direct vault queries.
    pub async fn vault(&self) -> Result<Arc<MemoryVaultService>, MemoryError> {
        self.ensure_vault().await
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
        strategy: &str,
    ) -> Result<SemanticSearchResult, MemoryError> {
        // simple_buffer: ring buffer only — no vault I/O, no embedding cost
        if strategy == "simple_buffer" {
            let stm = self.short_term.read().await;
            let entries: Vec<MemoryEntry> = stm
                .iter()
                .filter(|e| e.tags.contains(&format!("workspace:{}", workspace_id)))
                .rev()
                .take(limit)
                .cloned()
                .collect();
            return Ok(SemanticSearchResult {
                entries,
                mode: SemanticRetrievalMode::SimpleBuffer,
                reason: None,
                confidential_entry_ids: Vec::new(),
            });
        }

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
                    confidential_entry_ids: Vec::new(),
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
                    confidential_entry_ids: Vec::new(),
                });
            }
        };

        let embed_future = embedder.embed_text_with_task(query, EmbeddingTaskType::RetrievalQuery);
        let query_embedding = match tokio::time::timeout(
            std::time::Duration::from_millis(SEMANTIC_SEARCH_TIMEOUT_MS),
            embed_future,
        )
        .await
        {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                let entries = self
                    .query_workspace_memory(workspace_id, query, limit)
                    .await?;
                return Ok(SemanticSearchResult {
                    entries,
                    mode: SemanticRetrievalMode::LexicalFallback,
                    reason: Some(format!("Gemini embedding request failed: {}", e)),
                    confidential_entry_ids: Vec::new(),
                });
            }
            Err(_elapsed) => {
                let entries = self
                    .query_workspace_memory(workspace_id, query, limit)
                    .await?;
                return Ok(SemanticSearchResult {
                    entries,
                    mode: SemanticRetrievalMode::LexicalFallback,
                    reason: Some("Semantic search timed out; using lexical fallback".to_string()),
                    confidential_entry_ids: Vec::new(),
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

        // vector strategy: skip lexical, score by semantic + recency + access only
        if strategy == "vector" {
            let now = chrono::Utc::now().timestamp();
            let mut merged: HashMap<
                String,
                (
                    f64,
                    crate::services::memory_vault::types::DecryptedMemoryEntry,
                ),
            > = HashMap::new();

            for (entry, distance) in rows {
                let recency = recency_score(entry.created_at, now);
                let access = access_score(entry.access_count);
                let semantic = semantic_score(distance);
                let importance = entry
                    .metadata
                    .get("_importance")
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.5);
                let cat_boost = category_boost(entry.metadata.get("_category").map(|s| s.as_str()));
                let score = 0.55 * semantic
                    + 0.13 * recency
                    + 0.05 * access
                    + 0.15 * importance
                    + 0.12 * cat_boost;
                upsert_ranked_entry(&mut merged, entry, score);
            }

            let mut ranked = merged.into_values().collect::<Vec<_>>();
            ranked.sort_by(|(a_score, _), (b_score, _)| {
                b_score
                    .partial_cmp(a_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut confidential_entry_ids = Vec::new();
            for (_score, entry) in ranked.iter().take(limit.max(1)) {
                if matches!(entry.sensitivity, MemorySensitivity::Confidential) {
                    confidential_entry_ids.push(entry.id.clone());
                }
            }

            return Ok(SemanticSearchResult {
                entries: ranked
                    .into_iter()
                    .take(limit.max(1))
                    .map(|(_score, entry)| MemoryEntry {
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
                confidential_entry_ids,
            });
        }

        // hybrid (default): vector + lexical merged by scoring
        let lexical_rows = vault
            .search_workspace(workspace_id, query, limit.saturating_mul(3).max(20))
            .await
            .map_err(MemoryError::Other)?;

        let query_tokens = normalize_query_tokens(query);
        let now = chrono::Utc::now().timestamp();
        let mut merged: HashMap<
            String,
            (
                f64,
                crate::services::memory_vault::types::DecryptedMemoryEntry,
            ),
        > = HashMap::new();

        for (entry, distance) in rows {
            let lexical = lexical_overlap_score(&query_tokens, &entry.content);
            let recency = recency_score(entry.created_at, now);
            let access = access_score(entry.access_count);
            let semantic = semantic_score(distance);
            let importance = entry
                .metadata
                .get("_importance")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.5);
            let cat_boost = category_boost(entry.metadata.get("_category").map(|s| s.as_str()));
            let score = 0.50 * semantic
                + 0.12 * lexical
                + 0.12 * recency
                + 0.04 * access
                + 0.12 * importance
                + 0.10 * cat_boost;
            upsert_ranked_entry(&mut merged, entry, score);
        }

        for entry in lexical_rows {
            let lexical = lexical_overlap_score(&query_tokens, &entry.content);
            let recency = recency_score(entry.created_at, now);
            let access = access_score(entry.access_count);
            let score = 0.70 * lexical + 0.20 * recency + 0.10 * access;
            upsert_ranked_entry(&mut merged, entry, score);
        }

        let mut ranked = merged.into_values().collect::<Vec<_>>();
        ranked.sort_by(|(a_score, _), (b_score, _)| {
            b_score
                .partial_cmp(a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut confidential_entry_ids = Vec::new();
        for (_score, entry) in ranked.iter().take(limit.max(1)) {
            if matches!(entry.sensitivity, MemorySensitivity::Confidential) {
                confidential_entry_ids.push(entry.id.clone());
            }
        }

        Ok(SemanticSearchResult {
            entries: ranked
                .into_iter()
                .take(limit.max(1))
                .map(|(_score, entry)| MemoryEntry {
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
            confidential_entry_ids,
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
            warnings.push(
                "Gemini embedding API key unavailable; storing chunks without embeddings"
                    .to_string(),
            );
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
        let active_model = crate::services::memory_vault::profiles::ACTIVE_EMBEDDING_PROFILE.model;
        let fallback_model =
            crate::services::memory_vault::profiles::FALLBACK_EMBEDDING_PROFILE.model;
        let mut primary_embeddings: Vec<Option<Vec<f32>>> = vec![None; chunk_count];
        let mut primary_models: Vec<String> = vec![active_model.to_string(); chunk_count];
        let mut fallback_embeddings: Vec<Option<Vec<f32>>> = vec![None; chunk_count];

        if let Some(ref e) = embedder {
            match e
                .embed_texts_for_model_with_task(
                    &chunks,
                    active_model,
                    EmbeddingTaskType::RetrievalDocument,
                )
                .await
            {
                Ok(vecs) if vecs.len() == chunk_count => {
                    for (idx, v) in vecs.into_iter().enumerate() {
                        primary_embeddings[idx] = Some(v);
                    }
                    if fallback_model != active_model {
                        match e
                            .embed_texts_for_model_with_task(
                                &chunks,
                                fallback_model,
                                EmbeddingTaskType::RetrievalDocument,
                            )
                            .await
                        {
                            Ok(extra_vecs) if extra_vecs.len() == chunk_count => {
                                for (idx, v) in extra_vecs.into_iter().enumerate() {
                                    fallback_embeddings[idx] = Some(v);
                                }
                            }
                            Ok(_) => warnings.push(
                                "Fallback batch embeddings size mismatch during ingestion"
                                    .to_string(),
                            ),
                            Err(err) => warnings.push(format!(
                                "Fallback batch embeddings failed during ingestion: {}",
                                err
                            )),
                        }
                    }
                }
                Ok(_) => warnings.push(
                    "Primary batch embeddings size mismatch during ingestion; using fallback path"
                        .to_string(),
                ),
                Err(err) => {
                    if fallback_model != active_model {
                        match e
                            .embed_texts_for_model_with_task(
                                &chunks,
                                fallback_model,
                                EmbeddingTaskType::RetrievalDocument,
                            )
                            .await
                        {
                            Ok(vecs) if vecs.len() == chunk_count => {
                                for (idx, v) in vecs.into_iter().enumerate() {
                                    primary_embeddings[idx] = Some(v);
                                    primary_models[idx] = fallback_model.to_string();
                                }
                                warnings.push(format!(
                                    "Primary batch embedding failed; used fallback model '{}': {}",
                                    fallback_model, err
                                ));
                            }
                            Ok(_) => warnings.push(
                                "Fallback batch embeddings size mismatch during ingestion"
                                    .to_string(),
                            ),
                            Err(fallback_err) => warnings.push(format!(
                                "Batch embeddings unavailable; storing without embeddings: {} | {}",
                                err, fallback_err
                            )),
                        }
                    } else {
                        warnings.push(format!(
                            "Batch embeddings unavailable; storing without embeddings: {}",
                            err
                        ));
                    }
                }
            }
        }

        for (idx, chunk) in chunks.iter().enumerate() {
            let embedding = primary_embeddings[idx].clone();
            let embedding_model = primary_models[idx].clone();
            let mut additional_embeddings = Vec::new();

            if let Some(extra) = fallback_embeddings[idx].clone() {
                additional_embeddings.push(
                    crate::services::memory_vault::AdditionalEmbeddingInput {
                        embedding: extra,
                        embedding_model: fallback_model.to_string(),
                        embedding_provider: EMBEDDING_PROVIDER.to_string(),
                        embedding_dim: crate::services::memory_vault::EMBEDDING_DIM,
                    },
                );
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

    pub fn resolve_gemini_embedder(&self) -> Result<Option<Arc<EmbedderService>>, String> {
        let cached = self.embedder_cache.get_or_init(|| {
            let settings = crate::services::settings::SettingsManager::new();
            let provider_raw = settings.get_embedder_provider().to_string();
            let provider = match provider_raw.trim().to_lowercase().as_str() {
                "g" | "google" | "gemini" => EMBEDDING_PROVIDER.to_string(),
                _ => return None,
            };
            let keychain = crate::ai::keychain::KeychainManager::new();
            let api_key = keychain
                .get_key(EMBEDDING_PROVIDER)
                .or_else(|_| keychain.get_key(&provider_raw))
                .unwrap_or_default()
                .unwrap_or_default();
            if api_key.trim().is_empty() {
                return None;
            }
            Some(Arc::new(EmbedderService::new(
                provider,
                api_key,
                Some(EMBEDDING_MODEL.to_string()),
            )))
        });
        Ok(cached.clone())
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

fn normalize_query_tokens(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn lexical_overlap_score(tokens: &[String], content: &str) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }
    let haystack = content.to_lowercase();
    let mut hits = 0usize;
    for token in tokens {
        if haystack.contains(token) {
            hits += 1;
        }
    }
    hits as f64 / tokens.len() as f64
}

fn recency_score(created_at: i64, now: i64) -> f64 {
    let age_seconds = now.saturating_sub(created_at).max(0);
    let age_hours = age_seconds as f64 / 3600.0;
    (-age_hours / 168.0).exp().clamp(0.0, 1.0)
}

fn access_score(access_count: i64) -> f64 {
    let v = (access_count.max(0) as f64 + 1.0).ln();
    let norm = (64.0_f64 + 1.0).ln();
    (v / norm).clamp(0.0, 1.0)
}

fn semantic_score(distance: f32) -> f64 {
    let d = distance.max(0.0) as f64;
    (1.0 / (1.0 + d)).clamp(0.0, 1.0)
}

fn category_boost(cat: Option<&str>) -> f64 {
    match cat {
        Some("preference") => 1.0,
        Some("correction") => 0.95,
        Some("fact") => 0.7,
        Some("procedure") => 0.5,
        Some("observation") | _ => 0.2,
    }
}

fn upsert_ranked_entry(
    merged: &mut HashMap<
        String,
        (
            f64,
            crate::services::memory_vault::types::DecryptedMemoryEntry,
        ),
    >,
    entry: crate::services::memory_vault::types::DecryptedMemoryEntry,
    score: f64,
) {
    let id = entry.id.clone();
    match merged.get(&id) {
        Some((existing_score, _)) if *existing_score >= score => {}
        _ => {
            merged.insert(id, (score, entry));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_manager() -> MemoryManager {
        MemoryManager::new(100, PathBuf::from(":memory:"))
    }

    #[tokio::test]
    async fn test_simple_buffer_returns_ring_buffer_only() {
        let mm = make_manager();
        let workspace_id = "test-ws-simple";
        let now = chrono::Utc::now().timestamp();

        // Write directly to ring buffer via store_workspace_memory
        mm.store_workspace_memory(
            workspace_id,
            uuid::Uuid::new_v4().to_string(),
            "ring buffer content".to_string(),
            "test".to_string(),
            vec![format!("workspace:{}", workspace_id)],
            HashMap::new(),
            now,
            crate::services::memory_vault::MemorySensitivity::Internal,
        )
        .await
        .unwrap();

        // simple_buffer strategy reads from ring buffer without vault I/O
        let result = mm
            .search_semantic_detailed(workspace_id, "ring buffer", 5, "simple_buffer")
            .await
            .unwrap();

        assert!(
            !result.entries.is_empty(),
            "simple_buffer should return ring buffer entries"
        );
        assert_eq!(
            result.mode,
            crate::services::memory::SemanticRetrievalMode::SimpleBuffer
        );
        assert!(result
            .entries
            .iter()
            .any(|e| e.content.contains("ring buffer content")));
    }

    #[tokio::test]
    async fn test_prune_expired_removes_old_entries() {
        let mm = make_manager();
        let workspace_id = "test-ws-prune";

        // Store an entry with created_at 60 days ago
        let old_ts = chrono::Utc::now().timestamp() - 60 * 24 * 3600;
        mm.store_workspace_memory(
            workspace_id,
            "old-entry-id".to_string(),
            "old content to prune".to_string(),
            "test".to_string(),
            vec![format!("workspace:{}", workspace_id)],
            HashMap::new(),
            old_ts,
            crate::services::memory_vault::MemorySensitivity::Internal,
        )
        .await
        .unwrap();

        // Store a fresh entry (should survive)
        let now = chrono::Utc::now().timestamp();
        mm.store_workspace_memory(
            workspace_id,
            "fresh-entry-id".to_string(),
            "fresh content to keep".to_string(),
            "test".to_string(),
            vec![format!("workspace:{}", workspace_id)],
            HashMap::new(),
            now,
            crate::services::memory_vault::MemorySensitivity::Internal,
        )
        .await
        .unwrap();

        // Prune with 30 day retention — should remove the 60-day-old entry
        let pruned = mm.prune_expired(workspace_id, 30).await.unwrap();
        assert_eq!(pruned, 1, "should have pruned exactly 1 old entry");

        // Verify fresh entry still present
        let fresh = mm.get_by_id("fresh-entry-id").await.unwrap();
        assert!(fresh.is_some(), "fresh entry should survive pruning");

        // Verify old entry gone
        let old = mm.get_by_id("old-entry-id").await.unwrap();
        assert!(old.is_none(), "old entry should be pruned");
    }

    #[tokio::test]
    async fn test_simple_buffer_no_cross_workspace_leak() {
        let mm = make_manager();
        let now = chrono::Utc::now().timestamp();

        mm.store_workspace_memory(
            "workspace-a",
            uuid::Uuid::new_v4().to_string(),
            "workspace A secret".to_string(),
            "test".to_string(),
            vec!["workspace:workspace-a".to_string()],
            HashMap::new(),
            now,
            crate::services::memory_vault::MemorySensitivity::Internal,
        )
        .await
        .unwrap();

        let result = mm
            .search_semantic_detailed("workspace-b", "secret", 5, "simple_buffer")
            .await
            .unwrap();

        assert!(
            result.entries.is_empty(),
            "workspace-b must not see workspace-a entries"
        );
    }
}
