use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite};
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

#[derive(Debug, Clone)]
pub struct MemoryVaultRepository {
    pool: Pool<Sqlite>,
}

impl MemoryVaultRepository {
    pub async fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        let _ = std::fs::create_dir_all(&app_data_dir);
        let db_path = app_data_dir.join("rainy_cowork_v2.db");
        let db_url = format!("sqlite://{}", db_path.to_string_lossy());

        if !db_path.exists() {
            std::fs::File::create(&db_path)
                .map_err(|e| format!("Failed to create db file: {}", e))?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .map_err(|e| format!("Failed to connect to sqlite: {}", e))?;

        // Initialize schema
        // Note: sqlx sqlite doesn't support vector types natively without extension.
        // We store embedding as BLOB.
        sqlx::query(
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
                embedding BLOB,
                embedding_model TEXT,
                embedding_provider TEXT,
                embedding_dim INTEGER,
                key_version INTEGER NOT NULL DEFAULT 1
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to create vault table: {}", e))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_memory_vault_workspace_time
             ON memory_vault_entries(workspace_id, created_at DESC)",
        )
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to create vault index: {}", e))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memory_vault_migrations (
                id TEXT PRIMARY KEY,
                completed_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to create vault migration table: {}", e))?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn upsert_encrypted(&self, row: &VaultRow, key_version: i64) -> Result<(), String> {
        sqlx::query(
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
        )
        .bind(&row.id)
        .bind(&row.workspace_id)
        .bind(&row.source)
        .bind(&row.sensitivity)
        .bind(row.created_at)
        .bind(row.last_accessed)
        .bind(row.access_count)
        .bind(&row.content_ciphertext)
        .bind(&row.content_nonce)
        .bind(&row.tags_ciphertext)
        .bind(&row.tags_nonce)
        .bind(&row.metadata_ciphertext)
        .bind(&row.metadata_nonce)
        .bind(&row.embedding)
        .bind(&row.embedding_model)
        .bind(&row.embedding_provider)
        .bind(row.embedding_dim.map(|v| v as i64))
        .bind(key_version)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to upsert vault entry: {}", e))?;

        Ok(())
    }

    pub async fn list_workspace_rows(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> Result<Vec<VaultRow>, String> {
        let rows = sqlx::query(
            "SELECT * FROM memory_vault_entries
             WHERE workspace_id = ?
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .bind(workspace_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query list_workspace_rows: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row_to_vault(&row)?);
        }

        Ok(results)
    }

    /// Perform Approximate Nearest Neighbor search
    /// Since we removed `libsql` (and thus native vector search), we implement exact search in Rust for now.
    /// This is acceptable for local desktop scale (< 100k items).
    pub async fn search_workspace_vector_ann(
        &self,
        workspace_id: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(VaultRow, f32)>, String> {
        // Fallback to exact search
        self.search_workspace_vector_exact(workspace_id, query_embedding, limit)
            .await
    }

    pub async fn search_workspace_vector_exact(
        &self,
        workspace_id: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(VaultRow, f32)>, String> {
        // Fetch all embeddings for the workspace
        let rows = sqlx::query(
            "SELECT * FROM memory_vault_entries
             WHERE workspace_id = ? AND embedding IS NOT NULL AND embedding_dim = 3072",
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch vectors for exact search: {}", e))?;

        let mut scored_rows: Vec<(VaultRow, f32)> = Vec::with_capacity(rows.len());

        for row in rows {
            let vault_row = row_to_vault(&row)?;
            if let Some(emb_bytes) = &vault_row.embedding {
                let emb_vec = bytes_to_f32_vec(emb_bytes);
                if emb_vec.len() == query_embedding.len() {
                    let distance = cosine_distance(query_embedding, &emb_vec);
                    scored_rows.push((vault_row, distance));
                }
            }
        }

        // Sort by distance (ascending)
        scored_rows.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top K
        Ok(scored_rows.into_iter().take(limit).collect())
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<VaultRow>, String> {
        let row = sqlx::query("SELECT * FROM memory_vault_entries WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("Failed to get vault entry: {}", e))?;

        if let Some(r) = row {
            Ok(Some(row_to_vault(&r)?))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_by_id(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM memory_vault_entries WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
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
        sqlx::query(
            "UPDATE memory_vault_entries
             SET last_accessed = ?, access_count = ?
             WHERE id = ?",
        )
        .bind(last_accessed)
        .bind(access_count)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update vault access counters: {}", e))?;
        Ok(())
    }

    pub async fn counts(&self, workspace_id: Option<&str>) -> Result<(usize, usize), String> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_vault_entries")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let workspace: i64 = if let Some(ws) = workspace_id {
            sqlx::query_scalar("SELECT COUNT(*) FROM memory_vault_entries WHERE workspace_id = ?")
                .bind(ws)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0)
        } else {
            total
        };

        Ok((total as usize, workspace as usize))
    }

    pub async fn migration_completed(&self, id: &str) -> Result<bool, String> {
        let row = sqlx::query("SELECT id FROM memory_vault_migrations WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("Failed to check vault migration marker: {}", e))?;

        Ok(row.is_some())
    }

    pub async fn mark_migration_completed(&self, id: &str) -> Result<(), String> {
        sqlx::query(
            "INSERT OR REPLACE INTO memory_vault_migrations (id, completed_at) VALUES (?, ?)",
        )
        .bind(id)
        .bind(chrono::Utc::now().timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to mark vault migration: {}", e))?;
        Ok(())
    }
}

fn row_to_vault(row: &sqlx::sqlite::SqliteRow) -> Result<VaultRow, String> {
    Ok(VaultRow {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        source: row.get("source"),
        sensitivity: row.get("sensitivity"),
        created_at: row.get("created_at"),
        last_accessed: row.get("last_accessed"),
        access_count: row.get("access_count"),
        content_ciphertext: row.get("content_ciphertext"),
        content_nonce: row.get("content_nonce"),
        tags_ciphertext: row.get("tags_ciphertext"),
        tags_nonce: row.get("tags_nonce"),
        metadata_ciphertext: row.get("metadata_ciphertext"),
        metadata_nonce: row.get("metadata_nonce"),
        embedding: row.get("embedding"),
        embedding_model: row.get("embedding_model"),
        embedding_provider: row.get("embedding_provider"),
        embedding_dim: row
            .get::<Option<i64>, _>("embedding_dim")
            .map(|v| v as usize),
    })
}

fn bytes_to_f32_vec(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
        .collect()
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 1.0; // Maximum distance if zero vector
    }

    let cosine_similarity = dot_product / (norm_a * norm_b);
    // Distance = 1 - Similarity
    // Clamp to 0.0-2.0 to handle floating point errors
    (1.0 - cosine_similarity).max(0.0).min(2.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
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
    async fn test_sqlx_vector_search_logic() {
        // Unit test for our manual vector search implementation
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let repo = MemoryVaultRepository::new(temp_dir.clone())
            .await
            .expect("Failed to init repo");

        // Helper to create bytes from f32
        let to_bytes = |v: Vec<f32>| {
            let mut bytes = Vec::new();
            for f in v {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            bytes
        };

        // Insert row 1 (Identity)
        let row1 = VaultRow {
            id: "vec-1".to_string(),
            workspace_id: "vec-ws".to_string(),
            source: "test".to_string(),
            sensitivity: "Safe".to_string(),
            created_at: 1,
            last_accessed: 1,
            access_count: 0,
            content_ciphertext: vec![],
            content_nonce: vec![],
            tags_ciphertext: vec![],
            tags_nonce: vec![],
            metadata_ciphertext: None,
            metadata_nonce: None,
            embedding: Some(to_bytes(vec![1.0; 3072])), // All 1s
            embedding_model: None,
            embedding_provider: None,
            embedding_dim: Some(3072),
        };
        repo.upsert_encrypted(&row1, 1).await.unwrap();

        // Insert row 2 (Opposite)
        let row2 = VaultRow {
            id: "vec-2".to_string(),
            workspace_id: "vec-ws".to_string(),
            source: "test".to_string(),
            sensitivity: "Safe".to_string(),
            created_at: 2,
            last_accessed: 1,
            access_count: 0,
            content_ciphertext: vec![],
            content_nonce: vec![],
            tags_ciphertext: vec![],
            tags_nonce: vec![],
            metadata_ciphertext: None,
            metadata_nonce: None,
            embedding: Some(to_bytes(vec![-1.0; 3072])), // All -1s
            embedding_model: None,
            embedding_provider: None,
            embedding_dim: Some(3072),
        };
        repo.upsert_encrypted(&row2, 1).await.unwrap();

        // Search for query close to row 1
        let query = vec![0.9; 3072];
        let results = repo
            .search_workspace_vector_exact("vec-ws", &query, 10)
            .await
            .unwrap();

        // Should match row1 first (distance close to 0), row2 last (distance close to 2)
        assert!(results.len() >= 2);
        assert_eq!(results[0].0.id, "vec-1");
        assert!(results[0].1 < 0.1); // Close distance

        assert_eq!(results[1].0.id, "vec-2");
        assert!(results[1].1 > 1.9); // Far distance

        let _ = fs::remove_dir_all(temp_dir);
    }
}
