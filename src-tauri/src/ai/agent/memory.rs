use chrono::{TimeZone, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

/// Represents a unit of information in the agent's memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub source: String, // e.g., "user", "web:https://google.com", "file:/path/to/file"
    pub timestamp: i64,
    pub metadata: HashMap<String, String>,
    pub importance: f32, // 0.0 to 1.0
}

/// Managing agent context and knowledge
#[derive(Debug, Clone)]
pub struct AgentMemory {
    workspace_id: String,
    db: Arc<sqlx::SqlitePool>,
    /// Web client for fetching external info
    #[allow(dead_code)]
    http_client: Client,
}

impl AgentMemory {
    pub async fn new(workspace_id: &str, app_data_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&app_data_dir);
        let db_path = app_data_dir.join("rainy_cowork_v2.db");
        if !db_path.exists() {
            let _ = std::fs::File::create(&db_path);
        }
        let db_url = format!("sqlite://{}", db_path.to_string_lossy());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("failed to connect to sqlite db for agent memory");

        // Safety net in case migration has not run yet in early startup race conditions.
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS memory_entries (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                content TEXT NOT NULL,
                source TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                metadata_json TEXT NOT NULL DEFAULT '{}',
                importance REAL NOT NULL DEFAULT 0.5
            )",
        )
        .execute(&pool)
        .await;

        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_entities (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                entity_key TEXT NOT NULL,
                entity_value TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.5,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await;

        let memory = Self {
            workspace_id: workspace_id.to_string(),
            db: Arc::new(pool),
            http_client: Client::builder()
                .user_agent("Rainy-MaTE-Agent/1.0")
                .build()
                .unwrap_or_default(),
        };

        memory.migrate_legacy_json_if_present(app_data_dir).await;

        memory
    }

    pub async fn store(
        &self,
        content: String,
        source: String,
        metadata: Option<HashMap<String, String>>,
    ) {
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            source,
            timestamp: Utc::now().timestamp(),
            metadata: metadata.unwrap_or_default(),
            importance: 0.5, // Default importance
        };

        if let Ok(metadata_json) = serde_json::to_string(&entry.metadata) {
            let _ = sqlx::query(
                "INSERT INTO memory_entries (id, workspace_id, content, source, timestamp, metadata_json, importance)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&entry.id)
            .bind(&self.workspace_id)
            .bind(&entry.content)
            .bind(&entry.source)
            .bind(entry.timestamp)
            .bind(metadata_json)
            .bind(entry.importance)
            .execute(&*self.db)
            .await;
        }

        // Optional entity persistence if the caller provides a structured hint.
        if let (Some(entity_key), Some(entity_value)) = (
            entry.metadata.get("entity_key"),
            entry.metadata.get("entity_value"),
        ) {
            let _ = sqlx::query(
                "INSERT INTO agent_entities (id, workspace_id, entity_key, entity_value, confidence)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&self.workspace_id)
            .bind(entity_key)
            .bind(entity_value)
            .bind(entry.importance)
            .execute(&*self.db)
            .await;
        }
    }

    pub async fn retrieve(&self, query: &str) -> Vec<MemoryEntry> {
        let like_query = format!("%{}%", query.to_lowercase());
        let rows = sqlx::query_as::<_, (String, String, String, i64, String, f64)>(
            "SELECT id, content, source, timestamp, metadata_json, importance
             FROM memory_entries
             WHERE workspace_id = ? AND LOWER(content) LIKE ?
             ORDER BY importance DESC, timestamp DESC
             LIMIT 20",
        )
        .bind(&self.workspace_id)
        .bind(&like_query)
        .fetch_all(&*self.db)
        .await
        .unwrap_or_default();

        rows.into_iter()
            .map(
                |(id, content, source, timestamp, metadata_json, importance)| MemoryEntry {
                    id,
                    content,
                    source,
                    timestamp,
                    metadata: serde_json::from_str(&metadata_json).unwrap_or_default(),
                    importance: importance as f32,
                },
            )
            .collect()
    }

    #[allow(dead_code)] // @RESERVED — will be wired to a Tauri command for web page ingestion
    pub async fn ingest_web_page(&self, url: &str) -> Result<String, String> {
        let res = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("HTTP Error: {}", res.status()));
        }

        let html_content = res
            .text()
            .await
            .map_err(|e| format!("Failed to read text: {}", e))?;

        let document = Html::parse_document(&html_content);

        let selector = Selector::parse("body").unwrap();
        let body = document.select(&selector).next();

        let text_content = if let Some(node) = body {
            node.text().collect::<Vec<_>>().join(" ")
        } else {
            "No body content found".to_string()
        };

        let cleaned_text = text_content
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let truncated_text: String = cleaned_text.chars().take(10000).collect();

        let mut metadata = HashMap::new();
        metadata.insert("original_url".to_string(), url.to_string());
        metadata.insert("type".to_string(), "web_crawl".to_string());

        self.store(
            truncated_text.clone(),
            format!("web:{}", url),
            Some(metadata),
        )
        .await;

        Ok(format!(
            "Successfully ingested {} chars from {}",
            truncated_text.len(),
            url
        ))
    }

    #[allow(dead_code)] // @RESERVED — will be wired to a Tauri command for memory debugging
    pub async fn dump_context(&self) -> String {
        let rows = sqlx::query_as::<_, (String, i64, String)>(
            "SELECT source, timestamp, content
             FROM memory_entries
             WHERE workspace_id = ?
             ORDER BY timestamp DESC
             LIMIT 100",
        )
        .bind(&self.workspace_id)
        .fetch_all(&*self.db)
        .await
        .unwrap_or_default();

        rows.iter()
            .map(|(source, timestamp, content)| {
                format!(
                    "[{}] {}: {}",
                    source,
                    Utc.timestamp_opt(*timestamp, 0).unwrap().format("%H:%M:%S"),
                    content
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    async fn migrate_legacy_json_if_present(&self, app_data_dir: PathBuf) {
        let legacy_path = app_data_dir
            .join("memory")
            .join(&self.workspace_id)
            .join("short_term.json");

        if !legacy_path.exists() {
            return;
        }

        let legacy_content = match fs::read_to_string(&legacy_path).await {
            Ok(content) => content,
            Err(_) => return,
        };

        let legacy_entries: Vec<MemoryEntry> = match serde_json::from_str(&legacy_content) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in legacy_entries {
            if let Ok(metadata_json) = serde_json::to_string(&entry.metadata) {
                let _ = sqlx::query(
                    "INSERT OR IGNORE INTO memory_entries
                     (id, workspace_id, content, source, timestamp, metadata_json, importance)
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(entry.id)
                .bind(&self.workspace_id)
                .bind(entry.content)
                .bind(entry.source)
                .bind(entry.timestamp)
                .bind(metadata_json)
                .bind(entry.importance)
                .execute(&*self.db)
                .await;
            }
        }

        // Keep a backup, but remove legacy active file to avoid dual-write confusion.
        let backup_path = legacy_path.with_extension("json.migrated");
        let _ = fs::rename(&legacy_path, backup_path).await;
    }
}
