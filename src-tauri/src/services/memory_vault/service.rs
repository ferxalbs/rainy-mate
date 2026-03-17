use super::crypto::{decrypt_bytes, encrypt_bytes};
use super::key_provider::{MacOSKeychainVaultKeyProvider, VaultKeyProvider};
use super::repository::{MemoryVaultRepository, VaultRow};
use super::types::{DecryptedMemoryEntry, MemorySensitivity, MemoryVaultStats, StoreMemoryInput};
use crate::services::embedder::EmbeddingTaskType;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

const MIGRATION_PLAINTEXT_DB: &str = "migrate_plaintext_memory_entries_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorSearchMode {
    Ann,
    Exact,
}

#[derive(Debug, Clone)]
pub struct MemoryVaultService {
    repository: Arc<MemoryVaultRepository>,
    master_key: Arc<Vec<u8>>,
}

impl MemoryVaultService {
    pub async fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        Self::new_with_provider(
            app_data_dir,
            Arc::new(MacOSKeychainVaultKeyProvider::new()) as Arc<dyn VaultKeyProvider>,
        )
        .await
    }

    pub async fn new_with_provider(
        app_data_dir: PathBuf,
        provider: Arc<dyn VaultKeyProvider>,
    ) -> Result<Self, String> {
        let repository = Arc::new(MemoryVaultRepository::new(app_data_dir).await?);
        let master_key = Arc::new(provider.get_or_create_master_key()?);
        let service = Self {
            repository,
            master_key,
        };
        if !cfg!(test) {
            service.run_plaintext_migration().await?;
        }
        Ok(service)
    }

    pub async fn put(&self, input: StoreMemoryInput) -> Result<(), String> {
        let tags_json = serde_json::to_vec(&input.tags)
            .map_err(|e| format!("Failed to serialize tags: {}", e))?;
        let metadata_json = serde_json::to_vec(&input.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        let content = encrypt_bytes(
            self.master_key.as_slice(),
            &input.workspace_id,
            &input.id,
            input.content.as_bytes(),
        )?;
        let tags = encrypt_bytes(
            self.master_key.as_slice(),
            &input.workspace_id,
            &input.id,
            &tags_json,
        )?;
        let metadata = encrypt_bytes(
            self.master_key.as_slice(),
            &input.workspace_id,
            &input.id,
            &metadata_json,
        )?;

        let embedding_dim = input.embedding_dim.unwrap_or(super::types::EMBEDDING_DIM);
        let embedding_model = input
            .embedding_model
            .clone()
            .unwrap_or_else(|| super::types::EMBEDDING_MODEL.to_string());
        let embedding_provider = input
            .embedding_provider
            .clone()
            .unwrap_or_else(|| super::types::EMBEDDING_PROVIDER.to_string());

        let valid_embedding = if let Some(emb) = input.embedding {
            if emb.len() != embedding_dim {
                println!(
                    "Warning: Invalid embedding dimension {} (expected {}) for vault entry {}. Storing without embedding.",
                    emb.len(),
                    embedding_dim,
                    input.id
                );
                None
            } else {
                Some(emb)
            }
        } else {
            None
        };

        let embedding_bytes = valid_embedding.as_ref().map(|v| {
            let mut bytes = Vec::with_capacity(v.len() * 4);
            for f in v {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            bytes
        });

        let row = VaultRow {
            id: input.id,
            workspace_id: input.workspace_id,
            source: input.source,
            sensitivity: input.sensitivity.as_str().to_string(),
            created_at: input.created_at,
            last_accessed: input.created_at,
            access_count: 0,
            content_ciphertext: content.ciphertext,
            content_nonce: content.nonce,
            tags_ciphertext: tags.ciphertext,
            tags_nonce: tags.nonce,
            metadata_ciphertext: Some(metadata.ciphertext),
            metadata_nonce: Some(metadata.nonce),
            embedding: embedding_bytes,
            embedding_model: Some(embedding_model),
            embedding_provider: Some(embedding_provider),
            embedding_dim: Some(embedding_dim),
        };

        let mut vector_rows = Vec::new();

        if let Some(primary_emb) = valid_embedding {
            let mut bytes = Vec::with_capacity(primary_emb.len() * 4);
            for f in primary_emb {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            vector_rows.push((
                row.embedding_model
                    .clone()
                    .unwrap_or_else(|| super::types::EMBEDDING_MODEL.to_string()),
                row.embedding_provider
                    .clone()
                    .unwrap_or_else(|| super::types::EMBEDDING_PROVIDER.to_string()),
                row.embedding_dim.unwrap_or(super::types::EMBEDDING_DIM),
                bytes,
            ));
        }

        for extra in input.additional_embeddings {
            if extra.embedding.len() != extra.embedding_dim {
                continue;
            }
            let mut bytes = Vec::with_capacity(extra.embedding.len() * 4);
            for f in extra.embedding {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            vector_rows.push((
                extra.embedding_model,
                extra.embedding_provider,
                extra.embedding_dim,
                bytes,
            ));
        }

        self.repository
            .upsert_encrypted_atomic(&row, 1, vector_rows)
            .await?;

        Ok(())
    }

    pub fn spawn_reembed_backfill(&self) {
        let service = self.clone();
        tokio::spawn(async move {
            if let Err(err) = service.run_reembed_backfill().await {
                tracing::warn!("Memory re-embedding backfill failed: {}", err);
            }
        });
    }

    /// Delete vault entries older than `retention_days` for a workspace.
    pub async fn prune_workspace_expired(
        &self,
        workspace_id: &str,
        retention_days: u32,
    ) -> Result<u64, String> {
        let cutoff = chrono::Utc::now().timestamp()
            - (retention_days as i64) * 24 * 3600;
        self.repository
            .delete_workspace_entries_older_than(workspace_id, cutoff)
            .await
    }

    /// Delete ALL entries older than `max_retention_days` across every workspace.
    /// Called once at startup as a safety net for abandoned workspaces.
    /// Does nothing when `max_retention_days` is 0.
    pub async fn prune_global_expired(&self, max_retention_days: u32) -> Result<u64, String> {
        if max_retention_days == 0 {
            return Ok(0);
        }
        let cutoff = chrono::Utc::now().timestamp() - (max_retention_days as i64) * 24 * 3600;
        self.repository.delete_all_entries_older_than(cutoff).await
    }

    pub async fn search_workspace(
        &self,
        workspace_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<DecryptedMemoryEntry>, String> {
        let rows = self
            .repository
            .list_workspace_rows(workspace_id, limit.saturating_mul(10).max(50))
            .await?;
        let query_lc = query.to_lowercase();
        let mut results = Vec::new();

        let now = chrono::Utc::now().timestamp();
        for row in rows {
            let entry = self.decrypt_row(&row)?;
            if query_lc.is_empty() || entry.content.to_lowercase().contains(&query_lc) {
                results.push(DecryptedMemoryEntry {
                    access_count: entry.access_count + 1,
                    last_accessed: now,
                    ..entry
                });
            }
            if results.len() >= limit {
                break;
            }
        }

        let ids: Vec<String> = results.iter().map(|e| e.id.clone()).collect();
        let _ = self.repository.touch_access_batch(&ids, now).await;

        Ok(results)
    }

    #[allow(dead_code)]
    pub async fn search_workspace_vector(
        &self,
        workspace_id: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(DecryptedMemoryEntry, f32)>, String> {
        let (rows, _mode) = self
            .search_workspace_vector_with_mode(workspace_id, query_embedding, limit)
            .await?;
        Ok(rows)
    }

    pub async fn search_workspace_vector_with_mode(
        &self,
        workspace_id: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<(Vec<(DecryptedMemoryEntry, f32)>, VectorSearchMode), String> {
        let primary_model = crate::services::memory_vault::profiles::ACTIVE_EMBEDDING_PROFILE.model;
        let fallback_model =
            crate::services::memory_vault::profiles::FALLBACK_EMBEDDING_PROFILE.model;
        let embedding_dim = super::types::EMBEDDING_DIM;

        let (rows, mode) = match self
            .repository
            .search_workspace_vector_ann_for_model(
                workspace_id,
                query_embedding,
                limit,
                primary_model,
                embedding_dim,
            )
            .await
        {
            Ok(rows) if !rows.is_empty() => (rows, VectorSearchMode::Ann),
            _ => match self
                .repository
                .search_workspace_vector_ann_for_model(
                    workspace_id,
                    query_embedding,
                    limit,
                    fallback_model,
                    embedding_dim,
                )
                .await
            {
                Ok(rows) if !rows.is_empty() => (rows, VectorSearchMode::Ann),
                _ => match self
                    .repository
                    .search_workspace_vector_exact_for_model(
                        workspace_id,
                        query_embedding,
                        limit,
                        primary_model,
                        embedding_dim,
                    )
                    .await
                {
                    Ok(rows) if !rows.is_empty() => (rows, VectorSearchMode::Exact),
                    _ => (
                        self.repository
                            .search_workspace_vector_exact_for_model(
                                workspace_id,
                                query_embedding,
                                limit,
                                fallback_model,
                                embedding_dim,
                            )
                            .await?,
                        VectorSearchMode::Exact,
                    ),
                },
            },
        };
        let now = chrono::Utc::now().timestamp();
        let mut results = Vec::new();

        for (row, distance) in rows {
            let entry = self.decrypt_row(&row)?;
            results.push((
                DecryptedMemoryEntry {
                    access_count: entry.access_count + 1,
                    last_accessed: now,
                    ..entry
                },
                distance,
            ));
        }

        let ids: Vec<String> = results.iter().map(|(e, _)| e.id.clone()).collect();
        let _ = self.repository.touch_access_batch(&ids, now).await;

        Ok((results, mode))
    }

    #[allow(dead_code)]
    pub async fn recent_workspace(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> Result<Vec<DecryptedMemoryEntry>, String> {
        let rows = self
            .repository
            .list_workspace_rows(workspace_id, limit)
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.decrypt_row(&row)?);
        }
        Ok(out)
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<DecryptedMemoryEntry>, String> {
        let row = self.repository.get_by_id(id).await?;
        row.map(|r| self.decrypt_row(&r)).transpose()
    }

    pub async fn delete_by_id(&self, id: &str) -> Result<(), String> {
        self.repository.delete_by_id(id).await
    }

    pub async fn delete_workspace(&self, workspace_id: &str) -> Result<(), String> {
        self.repository.delete_workspace(workspace_id).await
    }

    pub async fn stats(&self, workspace_id: Option<&str>) -> Result<MemoryVaultStats, String> {
        let (total_entries, workspace_entries) = self.repository.counts(workspace_id).await?;
        Ok(MemoryVaultStats {
            total_entries,
            workspace_entries,
        })
    }

    /// Paginated listing with server-side filtering on unencrypted columns.
    pub async fn list_filtered(
        &self,
        opts: super::types::ListFilteredOpts,
    ) -> Result<super::types::PaginatedEntries, String> {
        use super::orm::{OrderBy, VaultQuery};

        let limit = opts.limit.unwrap_or(20).min(100);
        let offset = opts.offset.unwrap_or(0);

        let query = VaultQuery {
            workspace_id: opts.workspace_id,
            sensitivity: opts.sensitivity,
            source_prefix: opts.source_prefix,
            created_after: opts.created_after,
            created_before: opts.created_before,
            order_by: OrderBy::from_str_loose(opts.order_by.as_deref().unwrap_or("created_at")),
            limit,
            offset,
        };

        let total_count = self.repository.count_entries_filtered(&query).await?;
        let rows = self.repository.list_entries_filtered(&query).await?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in &rows {
            entries.push(self.decrypt_row(row)?);
        }

        Ok(super::types::PaginatedEntries {
            entries,
            total_count,
            offset,
            limit,
        })
    }

    /// List distinct workspace IDs with entry counts.
    pub async fn list_workspaces(&self) -> Result<Vec<super::types::WorkspaceSummary>, String> {
        let raw = self.repository.list_workspaces().await?;
        Ok(raw
            .into_iter()
            .map(|(workspace_id, entry_count)| super::types::WorkspaceSummary {
                workspace_id,
                entry_count,
            })
            .collect())
    }

    /// Detailed vault statistics.
    pub async fn detailed_stats(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<super::types::VaultDetailedStats, String> {
        self.repository.detailed_stats(workspace_id).await
    }

    /// Bulk delete entries by IDs.
    pub async fn delete_batch(&self, ids: &[String]) -> Result<u64, String> {
        self.repository.delete_batch(ids).await
    }

    fn decrypt_row(&self, row: &VaultRow) -> Result<DecryptedMemoryEntry, String> {
        let content_bytes = decrypt_bytes(
            self.master_key.as_slice(),
            &row.workspace_id,
            &row.id,
            &row.content_ciphertext,
            &row.content_nonce,
        )?;
        let tags_bytes = decrypt_bytes(
            self.master_key.as_slice(),
            &row.workspace_id,
            &row.id,
            &row.tags_ciphertext,
            &row.tags_nonce,
        )?;

        let metadata_bytes = match (&row.metadata_ciphertext, &row.metadata_nonce) {
            (Some(cipher), Some(nonce)) => decrypt_bytes(
                self.master_key.as_slice(),
                &row.workspace_id,
                &row.id,
                cipher,
                nonce,
            )?,
            _ => b"{}".to_vec(),
        };

        let content = String::from_utf8(content_bytes)
            .map_err(|e| format!("Invalid decrypted content encoding: {}", e))?;
        let tags: Vec<String> = serde_json::from_slice(&tags_bytes)
            .map_err(|e| format!("Invalid decrypted tags json: {}", e))?;
        let metadata: HashMap<String, String> = serde_json::from_slice(&metadata_bytes)
            .map_err(|e| format!("Invalid decrypted metadata json: {}", e))?;

        let embedding = row.embedding.as_ref().map(|bytes| {
            let mut floats = Vec::with_capacity(bytes.len() / 4);
            for chunk in bytes.chunks_exact(4) {
                let f = f32::from_le_bytes(chunk.try_into().unwrap());
                floats.push(f);
            }
            floats
        });

        Ok(DecryptedMemoryEntry {
            id: row.id.clone(),
            workspace_id: row.workspace_id.clone(),
            content,
            tags,
            source: row.source.clone(),
            sensitivity: MemorySensitivity::from_db(&row.sensitivity),
            created_at: row.created_at,
            last_accessed: row.last_accessed,
            access_count: row.access_count,
            metadata,
            embedding,
            embedding_model: row.embedding_model.clone(),
            embedding_provider: row.embedding_provider.clone(),
            embedding_dim: row.embedding_dim,
        })
    }

    async fn run_plaintext_migration(&self) -> Result<(), String> {
        if self
            .repository
            .migration_completed(MIGRATION_PLAINTEXT_DB)
            .await?
        {
            return Ok(());
        }

        let mut rows = match self
            .repository
            .conn()
            .query(
                "SELECT id, workspace_id, content, source, timestamp, metadata_json
             FROM memory_entries",
                (),
            )
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(()), // Table doesn't exist, ignore
        };

        while let Ok(Some(row)) = rows.next().await {
            let id: String = row.get(0).unwrap_or_default();
            if self.repository.get_by_id(&id).await?.is_some() {
                continue;
            }
            let workspace_id: String = row.get(1).unwrap_or_default();
            let content: String = row.get(2).unwrap_or_default();
            let source: String = row.get(3).unwrap_or_default();
            let timestamp: i64 = row.get(4).unwrap_or(0);
            let metadata_json: String = row.get(5).unwrap_or_default();
            let metadata: HashMap<String, String> =
                serde_json::from_str(&metadata_json).unwrap_or_default();

            self.put(StoreMemoryInput {
                id,
                workspace_id,
                content,
                tags: vec!["legacy".to_string()],
                source: if source.trim().is_empty() {
                    "legacy".to_string()
                } else {
                    source
                },
                sensitivity: MemorySensitivity::Internal,
                metadata,
                created_at: timestamp,
                embedding: None,
                embedding_model: None,
                embedding_provider: None,
                embedding_dim: None,
                additional_embeddings: Vec::new(),
            })
            .await?;
        }

        let _ = self
            .repository
            .conn()
            .execute("DELETE FROM memory_entries", ())
            .await;

        self.repository
            .mark_migration_completed(MIGRATION_PLAINTEXT_DB)
            .await
    }

    async fn run_reembed_backfill(&self) -> Result<(), String> {
        const BACKFILL_BATCH_SIZE: usize = 16;
        let migration_key = "migrate_memory_reembed_active_profile_v2";
        if self.repository.migration_completed(migration_key).await? {
            return Ok(());
        }

        // Paginate ID collection to avoid loading millions of rows into RAM.
        const PAGE_SIZE: usize = 500;
        let mut ids_to_reembed = Vec::new();
        let mut offset: usize = 0;
        loop {
            let mut rows = self
                .repository
                .conn()
                .query(
                    "SELECT e.id
                     FROM memory_vault_entries e
                     LEFT JOIN memory_vault_embedding_vectors v
                       ON v.entry_id = e.id AND v.embedding_model = ?2
                     WHERE e.embedding IS NULL
                        OR e.embedding_dim != ?1
                        OR e.embedding_model != ?2
                        OR v.entry_id IS NULL
                     LIMIT ?3 OFFSET ?4",
                    (
                        super::types::EMBEDDING_DIM as i64,
                        super::types::EMBEDDING_MODEL.to_string(),
                        PAGE_SIZE as i64,
                        offset as i64,
                    ),
                )
                .await
                .map_err(|e| format!("Failed to query rows for backfill (offset {}): {}", offset, e))?;

            let mut page_count = 0usize;
            while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
                let id: String = row.get(0).unwrap_or_default();
                ids_to_reembed.push(id);
                page_count += 1;
            }
            if page_count < PAGE_SIZE {
                break;
            }
            offset += PAGE_SIZE;
        }

        if ids_to_reembed.is_empty() {
            return self
                .repository
                .mark_migration_completed(migration_key)
                .await;
        }

        println!(
            "Found {} rows needing '{}' re-embedding (dim {}).",
            ids_to_reembed.len(),
            super::types::EMBEDDING_MODEL,
            super::types::EMBEDDING_DIM
        );

        // In a real production system, this should be done in a background task queue.
        // For Tauri startup, we will process them using the global EmbedderService.
        // Since MemoryVaultService doesn't have an embedder attached natively, we will
        // just set up a local embedder for the backfill using standard environment/keychain.

        let settings = crate::services::settings::SettingsManager::new();
        let provider_raw = settings.get_embedder_provider().to_string();
        let provider = match provider_raw.trim().to_lowercase().as_str() {
            "g" | "google" | "gemini" => super::types::EMBEDDING_PROVIDER.to_string(),
            _ => {
                println!(
                    "Memory vault backfill forcing Gemini embedding provider; configured '{}' is unsupported for Step 3",
                    provider_raw
                );
                super::types::EMBEDDING_PROVIDER.to_string()
            }
        };
        let model = super::types::EMBEDDING_MODEL.to_string();

        let keychain = crate::ai::keychain::KeychainManager::new();
        let api_key = keychain
            .get_key(&provider)
            .or_else(|_| keychain.get_key(&provider_raw))
            .unwrap_or_default()
            .unwrap_or_default();

        let embedder = crate::services::embedder::EmbedderService::new(
            provider,
            api_key.clone(),
            Some(model),
        );

        if api_key.is_empty() {
            println!("Skipping re-embedding backfill: No API key available.");
            return Ok(());
        }

        for id_batch in ids_to_reembed.chunks(BACKFILL_BATCH_SIZE) {
            let mut entries = Vec::new();
            for id in id_batch {
                if let Ok(Some(entry)) = self.get_by_id(id).await {
                    if entry.embedding_dim == Some(super::types::EMBEDDING_DIM)
                        && entry.embedding_model.as_deref() == Some(super::types::EMBEDDING_MODEL)
                    {
                        continue;
                    }
                    entries.push(entry);
                }
            }

            if entries.is_empty() {
                continue;
            }

            let texts = entries
                .iter()
                .map(|entry| entry.content.clone())
                .collect::<Vec<_>>();

            let embeddings = match embedder
                .embed_texts_for_model_with_task(
                    &texts,
                    super::types::EMBEDDING_MODEL,
                    EmbeddingTaskType::RetrievalDocument,
                )
                .await
            {
                Ok(v) if v.len() == entries.len() => Some(v),
                Ok(_) => {
                    println!(
                        "Batch re-embed size mismatch for {} entries; falling back to per-entry path",
                        entries.len()
                    );
                    None
                }
                Err(err) => {
                    println!(
                        "Batch re-embed failed for {} entries: {}. Falling back to per-entry path",
                        entries.len(),
                        err
                    );
                    None
                }
            };

            for (idx, entry) in entries.into_iter().enumerate() {
                let new_embedding = if let Some(ref batched) = embeddings {
                    batched.get(idx).cloned()
                } else {
                    embedder
                        .embed_text_for_model_with_task_strict(
                            &entry.content,
                            super::types::EMBEDDING_MODEL,
                            EmbeddingTaskType::RetrievalDocument,
                        )
                        .await
                        .ok()
                };

                if let Some(vec) = new_embedding {
                    let _ = self
                        .put(StoreMemoryInput {
                            id: entry.id,
                            workspace_id: entry.workspace_id,
                            content: entry.content,
                            tags: entry.tags,
                            source: entry.source,
                            sensitivity: entry.sensitivity,
                            metadata: entry.metadata,
                            created_at: entry.created_at,
                            embedding: Some(vec),
                            embedding_model: Some(super::types::EMBEDDING_MODEL.to_string()),
                            embedding_provider: Some(super::types::EMBEDDING_PROVIDER.to_string()),
                            embedding_dim: Some(super::types::EMBEDDING_DIM),
                            additional_embeddings: Vec::new(),
                        })
                        .await;
                }
            }
        }

        self.repository
            .mark_migration_completed(migration_key)
            .await
    }
}
