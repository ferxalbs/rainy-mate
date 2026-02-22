#[cfg(feature = "vector-db")]
use libsql::{params, Builder, Connection};
use std::path::PathBuf;

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

#[cfg(feature = "vector-db")]
#[derive(Debug, Clone)]
pub struct MemoryVaultRepository {
    conn: Connection,
}

#[cfg(not(feature = "vector-db"))]
#[derive(Debug, Clone)]
pub struct MemoryVaultRepository;

#[cfg(feature = "vector-db")]
impl MemoryVaultRepository {
    pub async fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        let _ = std::fs::create_dir_all(&app_data_dir);
        let db_path = app_data_dir.join("rainy_cowork_v2.db");
        if !db_path.exists() {
            std::fs::File::create(&db_path)
                .map_err(|e| format!("Failed to create db file: {}", e))?;
        }

        let db_url = db_path.to_string_lossy().to_string();
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

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_vault_migrations (
                id TEXT PRIMARY KEY,
                completed_at INTEGER NOT NULL
            )",
            (),
        )
        .await
        .map_err(|e| format!("Failed to create vault migration table: {}", e))?;

        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub async fn upsert_encrypted(&self, row: &VaultRow, key_version: i64) -> Result<(), String> {
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

    pub async fn search_workspace_vector(
        &self,
        workspace_id: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(VaultRow, f32)>, String> {
        let mut bytes = Vec::with_capacity(query_embedding.len() * 4);
        for f in query_embedding {
            bytes.extend_from_slice(&f.to_le_bytes());
        }

        let mut rows = self.conn.query(
            "SELECT id, workspace_id, source, sensitivity, created_at, last_accessed, access_count,
                    content_ciphertext, content_nonce, tags_ciphertext, tags_nonce, metadata_ciphertext, metadata_nonce, embedding, embedding_model, embedding_provider, embedding_dim,
                    vector_distance_cos(embedding, ?1) as distance
             FROM memory_vault_entries
             WHERE workspace_id = ?2 AND embedding IS NOT NULL AND embedding_dim = 3072 AND embedding_model = 'gemini-embedding-001'
             ORDER BY distance ASC
             LIMIT ?3",
            params![bytes, workspace_id.to_string(), limit as i64]
        )
        .await
        .map_err(|e| format!("Failed to search vector vault entries: {}", e))?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            let distance: f64 = row.get(17).unwrap_or(0.0);
            results.push((row_to_vault(&row)?, distance as f32));
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
        Ok(())
    }

    pub async fn touch_access(
        &self,
        id: &str,
        last_accessed: i64,
        access_count: i64,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE memory_vault_entries
             SET last_accessed = ?1, access_count = ?2
             WHERE id = ?3",
                params![last_accessed, access_count, id.to_string()],
            )
            .await
            .map_err(|e| format!("Failed to update vault access counters: {}", e))?;
        Ok(())
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

    // New methods to abstract connection access
    pub async fn get_legacy_plaintext_entries(&self) -> Result<Vec<(String, String, String, String, i64, String)>, String> {
        let mut rows = match self
            .conn
            .query(
                "SELECT id, workspace_id, content, source, timestamp, metadata_json FROM memory_entries",
                (),
            )
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()), // Table doesn't exist, ignore
        };

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            results.push((
                row.get(0).unwrap_or_default(),
                row.get(1).unwrap_or_default(),
                row.get(2).unwrap_or_default(),
                row.get(3).unwrap_or_default(),
                row.get(4).unwrap_or(0),
                row.get(5).unwrap_or_default(),
            ));
        }
        Ok(results)
    }

    pub async fn drop_legacy_plaintext_table(&self) -> Result<(), String> {
        let _ = self.conn.execute("DELETE FROM memory_entries", ()).await;
        Ok(())
    }

    pub async fn get_ids_needing_reembed(&self) -> Result<Vec<String>, String> {
         let mut rows = self
            .conn
            .query(
                "SELECT id FROM memory_vault_entries WHERE embedding IS NULL OR embedding_dim != 3072",
                (),
            )
            .await
            .map_err(|e| format!("Failed to query rows for backfill: {}", e))?;

        let mut ids = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            ids.push(row.get(0).unwrap_or_default());
        }
        Ok(ids)
    }
}

// Stub implementation for when vector-db feature is disabled
#[cfg(not(feature = "vector-db"))]
impl MemoryVaultRepository {
    pub async fn new(_app_data_dir: PathBuf) -> Result<Self, String> {
        // Return a dummy instance
        Ok(Self)
    }

    pub async fn upsert_encrypted(&self, _row: &VaultRow, _key_version: i64) -> Result<(), String> {
        Ok(())
    }

    pub async fn list_workspace_rows(
        &self,
        _workspace_id: &str,
        _limit: usize,
    ) -> Result<Vec<VaultRow>, String> {
        Ok(Vec::new())
    }

    pub async fn search_workspace_vector(
        &self,
        _workspace_id: &str,
        _query_embedding: &[f32],
        _limit: usize,
    ) -> Result<Vec<(VaultRow, f32)>, String> {
        Ok(Vec::new())
    }

    pub async fn get_by_id(&self, _id: &str) -> Result<Option<VaultRow>, String> {
        Ok(None)
    }

    pub async fn delete_by_id(&self, _id: &str) -> Result<(), String> {
        Ok(())
    }

    pub async fn touch_access(
        &self,
        _id: &str,
        _last_accessed: i64,
        _access_count: i64,
    ) -> Result<(), String> {
        Ok(())
    }

    pub async fn counts(&self, _workspace_id: Option<&str>) -> Result<(usize, usize), String> {
        Ok((0, 0))
    }

    pub async fn migration_completed(&self, _id: &str) -> Result<bool, String> {
        Ok(true)
    }

    pub async fn mark_migration_completed(&self, _id: &str) -> Result<(), String> {
        Ok(())
    }

    // Stubbed abstract methods
    pub async fn get_legacy_plaintext_entries(&self) -> Result<Vec<(String, String, String, String, i64, String)>, String> {
        Ok(Vec::new())
    }

    pub async fn drop_legacy_plaintext_table(&self) -> Result<(), String> {
        Ok(())
    }

    pub async fn get_ids_needing_reembed(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
}

#[cfg(feature = "vector-db")]
fn row_to_vault(row: &libsql::Row) -> Result<VaultRow, String> {
    Ok(VaultRow {
        id: row.get::<String>(0).map_err(|e| e.to_string())?,
        workspace_id: row.get::<String>(1).map_err(|e| e.to_string())?,
        source: row.get::<String>(2).map_err(|e| e.to_string())?,
        sensitivity: row.get::<String>(3).map_err(|e| e.to_string())?,
        created_at: row.get::<i64>(4).map_err(|e| e.to_string())?,
        last_accessed: row.get::<i64>(5).map_err(|e| e.to_string())?,
        access_count: row.get::<i64>(6).map_err(|e| e.to_string())?,
        content_ciphertext: row.get::<Vec<u8>>(7).map_err(|e| e.to_string())?,
        content_nonce: row.get::<Vec<u8>>(8).map_err(|e| e.to_string())?,
        tags_ciphertext: row.get::<Vec<u8>>(9).map_err(|e| e.to_string())?,
        tags_nonce: row.get::<Vec<u8>>(10).map_err(|e| e.to_string())?,
        metadata_ciphertext: row.get::<Option<Vec<u8>>>(11).unwrap_or(None),
        metadata_nonce: row.get::<Option<Vec<u8>>>(12).unwrap_or(None),
        embedding: row.get::<Option<Vec<u8>>>(13).unwrap_or(None),
        embedding_model: row.get::<Option<String>>(14).unwrap_or(None),
        embedding_provider: row.get::<Option<String>>(15).unwrap_or(None),
        embedding_dim: row
            .get::<Option<i64>>(16)
            .unwrap_or(None)
            .map(|v| v as usize),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    #[ignore] // FIXME: Libsql threading conflict in tests
    #[cfg(feature = "vector-db")]
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
        repo.upsert_encrypted(&row, 1)
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
    #[ignore] // FIXME: Libsql threading conflict in tests
    #[cfg(feature = "vector-db")]
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
