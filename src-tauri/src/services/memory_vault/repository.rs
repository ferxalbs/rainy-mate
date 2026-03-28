use libsql::{params, Builder, Connection};
use std::path::PathBuf;

const MEMORY_VAULT_VECTOR_INDEX: &str = "idx_memory_vault_embedding_gemini_3072";
const MEMORY_VAULT_VECTORS_TABLE_INDEX: &str = "idx_memory_vault_vectors_embedding_gemini_3072";

#[derive(Debug, Clone)]
pub struct VaultRow {
    pub id: String,
    pub workspace_id: String,
    pub source: String,
    pub sensitivity: String,
    pub created_at: i64,
    pub last_accessed: i64,
    pub access_count: i64,
    pub content_ciphertext: Vec<u8>,
    pub content_nonce: Vec<u8>,
    pub tags_ciphertext: Vec<u8>,
    pub tags_nonce: Vec<u8>,
    pub metadata_ciphertext: Option<Vec<u8>>,
    pub metadata_nonce: Option<Vec<u8>>,
    pub embedding: Option<Vec<u8>>,
    pub embedding_model: Option<String>,
    pub embedding_provider: Option<String>,
    pub embedding_dim: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct MemoryVaultRepository {
    conn: Connection,
}

impl MemoryVaultRepository {
    pub async fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        let _ = std::fs::create_dir_all(&app_data_dir);
        let db_path = app_data_dir.join("rainy_mate_v2.db");

        #[cfg(test)]
        if db_path.exists() {
            let _ = std::fs::remove_file(&db_path);
        }

        #[cfg(test)]
        let db_url = ":memory:".to_string();

        #[cfg(not(test))]
        let db = {
            let turso_url = std::env::var("RAINY_MEMORY_TURSO_URL")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());
            let turso_token = std::env::var("RAINY_MEMORY_TURSO_AUTH_TOKEN")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());

            if let (Some(url), Some(token)) = (turso_url, turso_token) {
                let sync_secs = std::env::var("RAINY_MEMORY_TURSO_SYNC_SECS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .filter(|v| *v > 0);

                let mut builder =
                    Builder::new_remote_replica(&db_path, url, token).read_your_writes(true);
                if let Some(seconds) = sync_secs {
                    builder = builder.sync_interval(std::time::Duration::from_secs(seconds));
                }

                builder
                    .build()
                    .await
                    .map_err(|e| format!("Failed to open libsql remote replica: {}", e))?
            } else {
                if !db_path.exists() {
                    let _ = std::fs::File::create(&db_path)
                        .map_err(|e| format!("Failed to create db file: {}", e));
                }

                let db_url = db_path.to_string_lossy().to_string();
                Builder::new_local(db_url)
                    .build()
                    .await
                    .map_err(|e| format!("Failed to open libsql builder: {}", e))?
            }
        };

        #[cfg(test)]
        let db = Builder::new_local(db_url)
            .build()
            .await
            .map_err(|e| format!("Failed to open libsql builder: {}", e))?;

        let conn = db
            .connect()
            .map_err(|e| format!("Failed to connect to libsql: {}", e))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_vault_entries (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                source TEXT NOT NULL,
                sensitivity TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_accessed INTEGER NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 0,
                content_ciphertext BLOB NOT NULL,
                content_nonce BLOB NOT NULL,
                tags_ciphertext BLOB NOT NULL,
                tags_nonce BLOB NOT NULL,
                metadata_ciphertext BLOB,
                metadata_nonce BLOB,
                embedding F32_BLOB(3072),
                embedding_model TEXT,
                embedding_provider TEXT,
                embedding_dim INTEGER,
                key_version INTEGER NOT NULL DEFAULT 1
            )",
            (),
        )
        .await
        .map_err(|e| format!("Failed to create vault table: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_vault_workspace_time
             ON memory_vault_entries(workspace_id, created_at DESC)",
            (),
        )
        .await
        .map_err(|e| format!("Failed to create vault index: {}", e))?;

        // Best-effort ANN vector index. Older libsql builds may not support this yet.
        // We keep exact vector search as a fallback path.
        let _ = conn
            .execute(
                &format!(
                    "CREATE INDEX IF NOT EXISTS {} ON memory_vault_entries(libsql_vector_idx(embedding))",
                    MEMORY_VAULT_VECTOR_INDEX
                ),
                (),
            )
            .await;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_vault_migrations (
                id TEXT PRIMARY KEY,
                completed_at INTEGER NOT NULL
            )",
            (),
        )
        .await
        .map_err(|e| format!("Failed to create vault migration table: {}", e))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_vault_embedding_vectors (
                entry_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                embedding_model TEXT NOT NULL,
                embedding_provider TEXT NOT NULL,
                embedding_dim INTEGER NOT NULL,
                embedding F32_BLOB(3072) NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY(entry_id, embedding_model)
            )",
            (),
        )
        .await
        .map_err(|e| format!("Failed to create embedding vectors table: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_vault_vectors_workspace_model
             ON memory_vault_embedding_vectors(workspace_id, embedding_model, embedding_dim)",
            (),
        )
        .await
        .map_err(|e| format!("Failed to create vectors lookup index: {}", e))?;

        let _ = conn
            .execute(
                &format!(
                    "CREATE INDEX IF NOT EXISTS {} ON memory_vault_embedding_vectors(libsql_vector_idx(embedding))",
                    MEMORY_VAULT_VECTORS_TABLE_INDEX
                ),
                (),
            )
            .await;

        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    async fn upsert_encrypted_row(&self, row: &VaultRow, key_version: i64) -> Result<(), String> {
        self.conn.execute(
            "INSERT INTO memory_vault_entries
             (id, workspace_id, source, sensitivity, created_at, last_accessed, access_count,
              content_ciphertext, content_nonce, tags_ciphertext, tags_nonce, metadata_ciphertext, metadata_nonce, embedding, embedding_model, embedding_provider, embedding_dim, key_version)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               workspace_id = excluded.workspace_id,
               source = excluded.source,
               sensitivity = excluded.sensitivity,
               created_at = excluded.created_at,
               last_accessed = excluded.last_accessed,
               access_count = excluded.access_count,
               content_ciphertext = excluded.content_ciphertext,
               content_nonce = excluded.content_nonce,
               tags_ciphertext = excluded.tags_ciphertext,
               tags_nonce = excluded.tags_nonce,
               metadata_ciphertext = excluded.metadata_ciphertext,
               metadata_nonce = excluded.metadata_nonce,
               embedding = excluded.embedding,
               embedding_model = excluded.embedding_model,
               embedding_provider = excluded.embedding_provider,
               embedding_dim = excluded.embedding_dim,
               key_version = excluded.key_version",
            params![
                row.id.clone(),
                row.workspace_id.clone(),
                row.source.clone(),
                row.sensitivity.clone(),
                row.created_at,
                row.last_accessed,
                row.access_count,
                row.content_ciphertext.clone(),
                row.content_nonce.clone(),
                row.tags_ciphertext.clone(),
                row.tags_nonce.clone(),
                row.metadata_ciphertext.clone(),
                row.metadata_nonce.clone(),
                row.embedding.clone(),
                row.embedding_model.clone(),
                row.embedding_provider.clone(),
                row.embedding_dim.map(|v| v as i64),
                key_version
            ]
        )
        .await
        .map_err(|e| format!("Failed to upsert vault entry: {}", e))?;

        Ok(())
    }

    async fn upsert_embedding_vector_row(
        &self,
        entry_id: &str,
        workspace_id: &str,
        embedding_model: &str,
        embedding_provider: &str,
        embedding_dim: usize,
        embedding_bytes: Vec<u8>,
        created_at: i64,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO memory_vault_embedding_vectors
                 (entry_id, workspace_id, embedding_model, embedding_provider, embedding_dim, embedding, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(entry_id, embedding_model) DO UPDATE SET
                    workspace_id = excluded.workspace_id,
                    embedding_provider = excluded.embedding_provider,
                    embedding_dim = excluded.embedding_dim,
                    embedding = excluded.embedding,
                    created_at = excluded.created_at",
                params![
                    entry_id.to_string(),
                    workspace_id.to_string(),
                    embedding_model.to_string(),
                    embedding_provider.to_string(),
                    embedding_dim as i64,
                    embedding_bytes,
                    created_at
                ],
            )
            .await
            .map_err(|e| format!("Failed to upsert embedding vector: {}", e))?;
        Ok(())
    }

    pub async fn upsert_encrypted_atomic(
        &self,
        row: &VaultRow,
        key_version: i64,
        embedding_rows: Vec<(String, String, usize, Vec<u8>)>,
    ) -> Result<(), String> {
        self.conn
            .execute("BEGIN IMMEDIATE TRANSACTION", ())
            .await
            .map_err(|e| format!("Failed to begin memory transaction: {}", e))?;

        let result = async {
            self.upsert_encrypted_row(row, key_version).await?;

            for (model, provider, dim, bytes) in embedding_rows {
                self.upsert_embedding_vector_row(
                    &row.id,
                    &row.workspace_id,
                    &model,
                    &provider,
                    dim,
                    bytes,
                    row.created_at,
                )
                .await?;
            }

            Ok::<(), String>(())
        }
        .await;

        match result {
            Ok(()) => {
                self.conn
                    .execute("COMMIT", ())
                    .await
                    .map_err(|e| format!("Failed to commit memory transaction: {}", e))?;
                Ok(())
            }
            Err(err) => {
                let _ = self.conn.execute("ROLLBACK", ()).await;
                Err(err)
            }
        }
    }

    pub async fn list_workspace_rows(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> Result<Vec<VaultRow>, String> {
        let mut rows = self.conn.query(
            "SELECT id, workspace_id, source, sensitivity, created_at, last_accessed, access_count,
                    content_ciphertext, content_nonce, tags_ciphertext, tags_nonce, metadata_ciphertext, metadata_nonce, embedding, embedding_model, embedding_provider, embedding_dim
             FROM memory_vault_entries
             WHERE workspace_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
            params![workspace_id.to_string(), limit as i64]
        )
        .await
        .map_err(|e| format!("Failed to query list_workspace_rows: {}", e))?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            results.push(row_to_vault(&row)?);
        }

        Ok(results)
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<VaultRow>, String> {
        let mut rows = self.conn.query(
            "SELECT id, workspace_id, source, sensitivity, created_at, last_accessed, access_count,
                    content_ciphertext, content_nonce, tags_ciphertext, tags_nonce, metadata_ciphertext, metadata_nonce, embedding, embedding_model, embedding_provider, embedding_dim
             FROM memory_vault_entries WHERE id = ?1",
            params![id.to_string()]
        )
        .await
        .map_err(|e| format!("Failed to get vault entry: {}", e))?;

        if let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            Ok(Some(row_to_vault(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_by_id(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM memory_vault_entries WHERE id = ?1",
                params![id.to_string()],
            )
            .await
            .map_err(|e| format!("Failed to delete vault entry: {}", e))?;
        let _ = self
            .conn
            .execute(
                "DELETE FROM memory_vault_embedding_vectors WHERE entry_id = ?1",
                params![id.to_string()],
            )
            .await;
        Ok(())
    }

    pub async fn delete_workspace(&self, workspace_id: &str) -> Result<(), String> {
        self.conn
            .execute("BEGIN IMMEDIATE TRANSACTION", ())
            .await
            .map_err(|e| format!("Failed to begin workspace delete transaction: {}", e))?;

        let result = async {
            self.conn
                .execute(
                    "DELETE FROM memory_vault_entries WHERE workspace_id = ?1",
                    params![workspace_id.to_string()],
                )
                .await
                .map_err(|e| format!("Failed to delete vault entries by workspace: {}", e))?;

            self.conn
                .execute(
                    "DELETE FROM memory_vault_embedding_vectors WHERE workspace_id = ?1",
                    params![workspace_id.to_string()],
                )
                .await
                .map_err(|e| format!("Failed to delete vault vectors by workspace: {}", e))?;

            Ok::<(), String>(())
        }
        .await;

        match result {
            Ok(()) => {
                self.conn
                    .execute("COMMIT", ())
                    .await
                    .map_err(|e| format!("Failed to commit workspace delete transaction: {}", e))?;
                Ok(())
            }
            Err(err) => {
                let _ = self.conn.execute("ROLLBACK", ()).await;
                Err(err)
            }
        }
    }

    pub async fn search_workspace_vector_ann_for_model(
        &self,
        workspace_id: &str,
        query_embedding: &[f32],
        limit: usize,
        model: &str,
        dim: usize,
    ) -> Result<Vec<(VaultRow, f32)>, String> {
        let mut bytes = Vec::with_capacity(query_embedding.len() * 4);
        for f in query_embedding {
            bytes.extend_from_slice(&f.to_le_bytes());
        }

        let mut rows = self
            .conn
            .query(
                &format!(
                    "SELECT m.id, m.workspace_id, m.source, m.sensitivity, m.created_at, m.last_accessed, m.access_count,
                            m.content_ciphertext, m.content_nonce, m.tags_ciphertext, m.tags_nonce, m.metadata_ciphertext, m.metadata_nonce, m.embedding, m.embedding_model, m.embedding_provider, m.embedding_dim,
                            vector_distance_cos(v.embedding, ?1) as distance
                     FROM vector_top_k('{}', ?1, ?3) nn
                     JOIN memory_vault_embedding_vectors v ON v.rowid = nn.id
                     JOIN memory_vault_entries m ON m.id = v.entry_id
                     WHERE v.workspace_id = ?2
                       AND v.embedding_dim = ?4
                       AND v.embedding_model = ?5
                     ORDER BY distance ASC",
                    MEMORY_VAULT_VECTORS_TABLE_INDEX
                ),
                params![bytes, workspace_id.to_string(), limit as i64, dim as i64, model.to_string()],
            )
            .await
            .map_err(|e| format!("Failed ANN vector search for model '{}': {}", model, e))?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            let distance: f64 = row.get(17).unwrap_or(0.0);
            results.push((row_to_vault(&row)?, distance as f32));
        }
        Ok(results)
    }

    pub async fn search_workspace_vector_exact_for_model(
        &self,
        workspace_id: &str,
        query_embedding: &[f32],
        limit: usize,
        model: &str,
        dim: usize,
    ) -> Result<Vec<(VaultRow, f32)>, String> {
        let mut bytes = Vec::with_capacity(query_embedding.len() * 4);
        for f in query_embedding {
            bytes.extend_from_slice(&f.to_le_bytes());
        }

        let mut rows = self
            .conn
            .query(
                "SELECT m.id, m.workspace_id, m.source, m.sensitivity, m.created_at, m.last_accessed, m.access_count,
                        m.content_ciphertext, m.content_nonce, m.tags_ciphertext, m.tags_nonce, m.metadata_ciphertext, m.metadata_nonce, m.embedding, m.embedding_model, m.embedding_provider, m.embedding_dim,
                        vector_distance_cos(v.embedding, ?1) as distance
                 FROM memory_vault_embedding_vectors v
                 JOIN memory_vault_entries m ON m.id = v.entry_id
                 WHERE v.workspace_id = ?2
                   AND v.embedding_dim = ?4
                   AND v.embedding_model = ?5
                 ORDER BY distance ASC
                 LIMIT ?3",
                params![bytes, workspace_id.to_string(), limit as i64, dim as i64, model.to_string()],
            )
            .await
            .map_err(|e| format!("Failed exact vector search for model '{}': {}", model, e))?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            let distance: f64 = row.get(17).unwrap_or(0.0);
            results.push((row_to_vault(&row)?, distance as f32));
        }
        Ok(results)
    }

    /// Batch-increment access counters for multiple entries in a single transaction.
    pub async fn touch_access_batch(&self, ids: &[String], now: i64) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        self.conn
            .execute("BEGIN IMMEDIATE TRANSACTION", ())
            .await
            .map_err(|e| format!("Failed to begin touch_access_batch transaction: {}", e))?;

        let result: Result<(), String> = async {
            for id in ids {
                self.conn
                    .execute(
                        "UPDATE memory_vault_entries
                         SET last_accessed = ?1, access_count = access_count + 1
                         WHERE id = ?2",
                        params![now, id.clone()],
                    )
                    .await
                    .map_err(|e| format!("Failed to touch access for {}: {}", id, e))?;
            }
            Ok(())
        }
        .await;

        match result {
            Ok(()) => {
                self.conn
                    .execute("COMMIT", ())
                    .await
                    .map_err(|e| format!("Failed to commit touch_access_batch: {}", e))?;
                Ok(())
            }
            Err(err) => {
                let _ = self.conn.execute("ROLLBACK", ()).await;
                Err(err)
            }
        }
    }

    pub async fn counts(&self, workspace_id: Option<&str>) -> Result<(usize, usize), String> {
        let mut total_rows = self
            .conn
            .query("SELECT COUNT(*) FROM memory_vault_entries", ())
            .await
            .map_err(|e| e.to_string())?;
        let total: i64 = if let Some(r) = total_rows.next().await.map_err(|e| e.to_string())? {
            r.get::<i64>(0).unwrap_or(0)
        } else {
            0
        };

        let workspace: i64 = if let Some(ws) = workspace_id {
            let mut ws_rows = self
                .conn
                .query(
                    "SELECT COUNT(*) FROM memory_vault_entries WHERE workspace_id = ?1",
                    params![ws.to_string()],
                )
                .await
                .map_err(|e| e.to_string())?;
            if let Some(r) = ws_rows.next().await.map_err(|e| e.to_string())? {
                r.get::<i64>(0).unwrap_or(0)
            } else {
                0
            }
        } else {
            total
        };

        Ok((total as usize, workspace as usize))
    }

    /// Delete all entries for a workspace whose created_at is older than `cutoff` (unix seconds).
    /// Returns the number of rows deleted.
    pub async fn delete_workspace_entries_older_than(
        &self,
        workspace_id: &str,
        cutoff: i64,
    ) -> Result<u64, String> {
        self.conn
            .execute("BEGIN IMMEDIATE TRANSACTION", ())
            .await
            .map_err(|e| format!("Failed to begin retention prune transaction: {}", e))?;

        let result: Result<u64, String> = async {
            // Collect IDs to cascade-delete from embedding_vectors table.
            let mut id_rows = self
                .conn
                .query(
                    "SELECT id FROM memory_vault_entries
                     WHERE workspace_id = ?1 AND created_at < ?2",
                    params![workspace_id.to_string(), cutoff],
                )
                .await
                .map_err(|e| format!("Failed to collect expired ids: {}", e))?;

            let mut ids: Vec<String> = Vec::new();
            while let Some(row) = id_rows.next().await.map_err(|e| e.to_string())? {
                ids.push(row.get::<String>(0).map_err(|e| e.to_string())?);
            }

            for id in &ids {
                self.conn
                    .execute(
                        "DELETE FROM memory_vault_embedding_vectors WHERE entry_id = ?1",
                        params![id.clone()],
                    )
                    .await
                    .map_err(|e| {
                        format!("Failed to delete expired embedding vector {}: {}", id, e)
                    })?;
            }

            self.conn
                .execute(
                    "DELETE FROM memory_vault_entries
                     WHERE workspace_id = ?1 AND created_at < ?2",
                    params![workspace_id.to_string(), cutoff],
                )
                .await
                .map_err(|e| format!("Failed to delete expired entries: {}", e))?;

            Ok(ids.len() as u64)
        }
        .await;

        match result {
            Ok(count) => {
                self.conn
                    .execute("COMMIT", ())
                    .await
                    .map_err(|e| format!("Failed to commit retention prune: {}", e))?;
                Ok(count)
            }
            Err(err) => {
                let _ = self.conn.execute("ROLLBACK", ()).await;
                Err(err)
            }
        }
    }

    /// Delete entries across ALL workspaces older than `cutoff` (unix seconds).
    /// Used as a startup safety net for very stale entries.
    pub async fn delete_all_entries_older_than(&self, cutoff: i64) -> Result<u64, String> {
        self.conn
            .execute("BEGIN IMMEDIATE TRANSACTION", ())
            .await
            .map_err(|e| format!("Failed to begin global prune transaction: {}", e))?;

        let result: Result<u64, String> = async {
            let mut id_rows = self
                .conn
                .query(
                    "SELECT id FROM memory_vault_entries WHERE created_at < ?1",
                    params![cutoff],
                )
                .await
                .map_err(|e| format!("Failed to collect globally expired ids: {}", e))?;

            let mut ids: Vec<String> = Vec::new();
            while let Some(row) = id_rows.next().await.map_err(|e| e.to_string())? {
                ids.push(row.get::<String>(0).map_err(|e| e.to_string())?);
            }

            for id in &ids {
                self.conn
                    .execute(
                        "DELETE FROM memory_vault_embedding_vectors WHERE entry_id = ?1",
                        params![id.clone()],
                    )
                    .await
                    .map_err(|e| {
                        format!("Failed to delete expired embedding vector {}: {}", id, e)
                    })?;
            }

            self.conn
                .execute(
                    "DELETE FROM memory_vault_entries WHERE created_at < ?1",
                    params![cutoff],
                )
                .await
                .map_err(|e| format!("Failed to delete globally expired entries: {}", e))?;

            Ok(ids.len() as u64)
        }
        .await;

        match result {
            Ok(count) => {
                self.conn
                    .execute("COMMIT", ())
                    .await
                    .map_err(|e| format!("Failed to commit global prune: {}", e))?;
                Ok(count)
            }
            Err(err) => {
                let _ = self.conn.execute("ROLLBACK", ()).await;
                Err(err)
            }
        }
    }

    /// Paginated, filtered listing using VaultQuery.
    pub async fn list_entries_filtered(
        &self,
        query: &super::orm::VaultQuery,
    ) -> Result<Vec<VaultRow>, String> {
        query.execute(&self.conn).await
    }

    /// Count entries matching a VaultQuery (ignores limit/offset).
    pub async fn count_entries_filtered(
        &self,
        query: &super::orm::VaultQuery,
    ) -> Result<usize, String> {
        query.count(&self.conn).await
    }

    /// List distinct workspace IDs with their entry counts.
    pub async fn list_workspaces(&self) -> Result<Vec<(String, usize)>, String> {
        let mut rows = self
            .conn
            .query(
                "SELECT workspace_id, COUNT(*) as cnt FROM memory_vault_entries GROUP BY workspace_id ORDER BY cnt DESC",
                (),
            )
            .await
            .map_err(|e| format!("Failed to list workspaces: {}", e))?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            let ws: String = row.get(0).unwrap_or_default();
            let count: i64 = row.get(1).unwrap_or(0);
            results.push((ws, count as usize));
        }
        Ok(results)
    }

    /// Detailed statistics for the vault, optionally filtered by workspace.
    pub async fn detailed_stats(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<super::types::VaultDetailedStats, String> {
        let (total, workspace_entries) = self.counts(workspace_id).await?;

        let ws_filter = workspace_id.map(|ws| ws.to_string());

        // Entries by sensitivity
        let sensitivity_sql = if ws_filter.is_some() {
            "SELECT sensitivity, COUNT(*) FROM memory_vault_entries WHERE workspace_id = ?1 GROUP BY sensitivity"
        } else {
            "SELECT sensitivity, COUNT(*) FROM memory_vault_entries GROUP BY sensitivity"
        };
        let mut sensitivity_map = std::collections::HashMap::new();
        {
            let mut rows = if let Some(ref ws) = ws_filter {
                self.conn.query(sensitivity_sql, params![ws.clone()]).await
            } else {
                self.conn.query(sensitivity_sql, ()).await
            }
            .map_err(|e| format!("Failed to query sensitivity stats: {}", e))?;

            while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
                let s: String = row.get(0).unwrap_or_default();
                let c: i64 = row.get(1).unwrap_or(0);
                sensitivity_map.insert(s, c as usize);
            }
        }

        // Entries by source (top 20)
        let source_sql = if ws_filter.is_some() {
            "SELECT source, COUNT(*) as cnt FROM memory_vault_entries WHERE workspace_id = ?1 GROUP BY source ORDER BY cnt DESC LIMIT 20"
        } else {
            "SELECT source, COUNT(*) as cnt FROM memory_vault_entries GROUP BY source ORDER BY cnt DESC LIMIT 20"
        };
        let mut entries_by_source = Vec::new();
        {
            let mut rows = if let Some(ref ws) = ws_filter {
                self.conn.query(source_sql, params![ws.clone()]).await
            } else {
                self.conn.query(source_sql, ()).await
            }
            .map_err(|e| format!("Failed to query source stats: {}", e))?;

            while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
                let s: String = row.get(0).unwrap_or_default();
                let c: i64 = row.get(1).unwrap_or(0);
                entries_by_source.push((s, c as usize));
            }
        }

        // Embedding coverage
        let emb_sql = if ws_filter.is_some() {
            "SELECT COUNT(*) FROM memory_vault_entries WHERE workspace_id = ?1 AND embedding IS NOT NULL"
        } else {
            "SELECT COUNT(*) FROM memory_vault_entries WHERE embedding IS NOT NULL"
        };
        let has_embeddings: usize = {
            let mut rows = if let Some(ref ws) = ws_filter {
                self.conn.query(emb_sql, params![ws.clone()]).await
            } else {
                self.conn.query(emb_sql, ()).await
            }
            .map_err(|e| format!("Failed to query embedding stats: {}", e))?;

            if let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
                row.get::<i64>(0).unwrap_or(0) as usize
            } else {
                0
            }
        };

        // Min/max created_at
        let range_sql = if ws_filter.is_some() {
            "SELECT MIN(created_at), MAX(created_at) FROM memory_vault_entries WHERE workspace_id = ?1"
        } else {
            "SELECT MIN(created_at), MAX(created_at) FROM memory_vault_entries"
        };
        let (oldest, newest) = {
            let mut rows = if let Some(ref ws) = ws_filter {
                self.conn.query(range_sql, params![ws.clone()]).await
            } else {
                self.conn.query(range_sql, ()).await
            }
            .map_err(|e| format!("Failed to query time range: {}", e))?;

            if let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
                let min: Option<i64> = row.get::<Option<i64>>(0).unwrap_or(None);
                let max: Option<i64> = row.get::<Option<i64>>(1).unwrap_or(None);
                (min, max)
            } else {
                (None, None)
            }
        };

        Ok(super::types::VaultDetailedStats {
            total_entries: total,
            workspace_entries,
            entries_by_sensitivity: sensitivity_map,
            entries_by_source,
            has_embeddings,
            missing_embeddings: workspace_entries.saturating_sub(has_embeddings),
            oldest_entry: oldest,
            newest_entry: newest,
        })
    }

    /// Delete multiple entries by ID in a single transaction.
    pub async fn delete_batch(&self, ids: &[String]) -> Result<u64, String> {
        if ids.is_empty() {
            return Ok(0);
        }

        self.conn
            .execute("BEGIN IMMEDIATE TRANSACTION", ())
            .await
            .map_err(|e| format!("Failed to begin delete_batch transaction: {}", e))?;

        let result: Result<u64, String> = async {
            for id in ids {
                self.conn
                    .execute(
                        "DELETE FROM memory_vault_embedding_vectors WHERE entry_id = ?1",
                        params![id.clone()],
                    )
                    .await
                    .map_err(|e| format!("Failed to delete vectors for {}: {}", id, e))?;

                self.conn
                    .execute(
                        "DELETE FROM memory_vault_entries WHERE id = ?1",
                        params![id.clone()],
                    )
                    .await
                    .map_err(|e| format!("Failed to delete entry {}: {}", id, e))?;
            }
            Ok(ids.len() as u64)
        }
        .await;

        match result {
            Ok(count) => {
                self.conn
                    .execute("COMMIT", ())
                    .await
                    .map_err(|e| format!("Failed to commit delete_batch: {}", e))?;
                Ok(count)
            }
            Err(err) => {
                let _ = self.conn.execute("ROLLBACK", ()).await;
                Err(err)
            }
        }
    }

    pub async fn migration_completed(&self, id: &str) -> Result<bool, String> {
        let mut rows = self
            .conn
            .query(
                "SELECT id FROM memory_vault_migrations WHERE id = ?1",
                params![id.to_string()],
            )
            .await
            .map_err(|e| format!("Failed to check vault migration marker: {}", e))?;

        Ok(rows.next().await.map_err(|e| e.to_string())?.is_some())
    }

    pub async fn mark_migration_completed(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO memory_vault_migrations (id, completed_at) VALUES (?1, ?2)",
                params![id.to_string(), chrono::Utc::now().timestamp()],
            )
            .await
            .map_err(|e| format!("Failed to mark vault migration: {}", e))?;
        Ok(())
    }
}

fn row_to_vault(row: &libsql::Row) -> Result<VaultRow, String> {
    super::orm::vault_column_map().to_vault_row(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;

    #[tokio::test]
    #[serial_test::serial]
    async fn test_create_and_query_vault_schema() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());

        // Initialize the DB
        let repo = MemoryVaultRepository::new(temp_dir.clone())
            .await
            .expect("Failed to initialize vault repository");

        // Create a dummy row
        let row = VaultRow {
            id: "test-id".to_string(),
            workspace_id: "test-ws".to_string(),
            source: "test".to_string(),
            sensitivity: "Safe".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            last_accessed: chrono::Utc::now().timestamp(),
            access_count: 0,
            content_ciphertext: vec![1, 2, 3],
            content_nonce: vec![4, 5, 6],
            tags_ciphertext: vec![7, 8],
            tags_nonce: vec![9, 10],
            metadata_ciphertext: None,
            metadata_nonce: None,
            embedding: None, // Test with no embedding first
            embedding_model: None,
            embedding_provider: None,
            embedding_dim: None,
        };

        // Test insertion
        repo.upsert_encrypted_atomic(&row, 1, Vec::new())
            .await
            .expect("Failed to upsert row");

        // Verify retrieval
        let retrieved = repo
            .get_by_id("test-id")
            .await
            .expect("Failed to get row")
            .expect("Row not found");

        assert_eq!(retrieved.workspace_id, "test-ws");

        // Clean up
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_libsql_direct_vector_api() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());

        let db = libsql::Builder::new_local(temp_dir.clone())
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();

        conn.execute("CREATE TABLE test_vec (id TEXT, embedding F32_BLOB(3))", ())
            .await
            .unwrap();

        let embedding = vec![1.0f32, 2.0f32, 3.0f32];
        let mut embedding_bytes = Vec::new();
        for f in &embedding {
            embedding_bytes.extend_from_slice(&f.to_le_bytes());
        }

        conn.execute(
            "INSERT INTO test_vec VALUES (?1, ?2)",
            libsql::params!["test-id", embedding_bytes.clone()],
        )
        .await
        .unwrap();

        let mut rows = conn
            .query(
                "SELECT id, vector_distance_cos(embedding, ?1) as dist FROM test_vec",
                libsql::params![embedding_bytes],
            )
            .await
            .unwrap();

        let row = rows.next().await.unwrap().unwrap();
        let id: String = row.get(0).unwrap();
        // Option 1: f64 or f32? libsql often returns f64 for REAL. We'll read it as f64.
        let dist: f64 = row.get(1).unwrap();

        assert_eq!(id, "test-id");
        println!("Distance: {}", dist);

        let _ = fs::remove_dir_all(temp_dir);
    }
}
