use crate::services::memory_vault::{MemorySensitivity, MemoryVaultService, StoreMemoryInput};
use chrono::{TimeZone, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub source: String,
    pub timestamp: i64,
    pub metadata: HashMap<String, String>,
    pub importance: f32,
    pub sensitivity: crate::services::memory_vault::MemorySensitivity,
}

#[derive(Debug, Clone)]
pub struct AgentMemory {
    workspace_id: String,
    db: Arc<sqlx::SqlitePool>,
    vault: Arc<MemoryVaultService>,
    manager: Option<Arc<crate::services::MemoryManager>>,
    embedder: Arc<crate::services::embedder::EmbedderService>,
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

        let vault = Arc::new(
            MemoryVaultService::new(app_data_dir.clone())
                .await
                .expect("failed to initialize memory vault"),
        );

        let settings = crate::services::settings::SettingsManager::new();
        let provider_raw = settings.get_embedder_provider().to_string();
        let provider = match provider_raw.trim().to_lowercase().as_str() {
            "g" | "google" | "gemini" => "gemini".to_string(),
            "oai" | "openai" => "openai".to_string(),
            other => other.to_string(),
        };
        let model = settings.get_embedder_model().to_string();

        let keychain = crate::ai::keychain::KeychainManager::new();
        let api_key = keychain
            .get_key(&provider)
            .or_else(|_| keychain.get_key(&provider_raw))
            .unwrap_or_default()
            .unwrap_or_default();

        let memory = Self {
            workspace_id: workspace_id.to_string(),
            db: Arc::new(pool),
            vault,
            manager: None, // Will be set by set_manager if provided
            embedder: Arc::new(crate::services::embedder::EmbedderService::new(
                provider,
                api_key,
                Some(model),
            )),
            http_client: Client::builder()
                .user_agent("Rainy-MaTE-Agent/1.0")
                .build()
                .unwrap_or_default(),
        };

        memory.migrate_legacy_json_if_present(app_data_dir).await;
        memory
    }

    pub fn manager(&self) -> Option<Arc<crate::services::MemoryManager>> {
        self.manager.clone()
    }

    // @RESERVED - will be implemented for agent runtime init cycle
    #[allow(dead_code)]
    pub fn set_manager(&mut self, manager: Arc<crate::services::MemoryManager>) {
        self.manager = Some(manager);
    }

    pub async fn store(
        &self,
        content: String,
        source: String,
        metadata: Option<HashMap<String, String>>,
    ) {
        let entry_id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();
        let metadata = metadata.unwrap_or_default();

        let mut tags = vec![
            format!("workspace:{}", self.workspace_id),
            format!("source:{}", source),
            "agent_memory".to_string(),
        ];
        if let Some(role) = metadata.get("role") {
            tags.push(format!("role:{}", role));
        }
        if let Some(tool) = metadata.get("tool") {
            tags.push(format!("tool:{}", tool));
        }

        let embed_res = self.embedder.embed_text(&content).await;
        let embedding = match embed_res {
            Ok(vec) => Some(vec),
            Err(e) => {
                println!("Failed to embed memory: {}", e);
                None
            }
        };

        let _ = self
            .vault
            .put(StoreMemoryInput {
                id: entry_id,
                workspace_id: self.workspace_id.clone(),
                content,
                tags,
                source: source.clone(),
                sensitivity: MemorySensitivity::Internal,
                metadata: metadata.clone(),
                created_at: timestamp,
                embedding,
            })
            .await;

        if let (Some(entity_key), Some(entity_value)) =
            (metadata.get("entity_key"), metadata.get("entity_value"))
        {
            let _ = sqlx::query(
                "INSERT INTO agent_entities (id, workspace_id, entity_key, entity_value, confidence)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&self.workspace_id)
            .bind(entity_key)
            .bind(entity_value)
            .bind(0.5_f32)
            .execute(&*self.db)
            .await;
        }
    }

    pub async fn retrieve(&self, query: &str) -> Vec<MemoryEntry> {
        let embed_res = self.embedder.embed_text(query).await;

        let rows = if let Ok(query_embedding) = embed_res {
            let limit = 20;
            self.vault
                .search_workspace_vector(&self.workspace_id, &query_embedding, limit)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|(entry, _dist)| entry)
                .collect()
        } else {
            // Fallback to basic keyword search if embedding fails
            self.vault
                .search_workspace(&self.workspace_id, query, 20)
                .await
                .unwrap_or_default()
        };

        rows.into_iter()
            .map(|row| {
                let importance = row
                    .metadata
                    .get("importance")
                    .and_then(|v| v.parse::<f32>().ok())
                    .unwrap_or(0.5);
                MemoryEntry {
                    id: row.id,
                    content: row.content,
                    source: row.source,
                    timestamp: row.created_at,
                    metadata: row.metadata,
                    importance,
                    sensitivity: row.sensitivity,
                }
            })
            .collect()
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub async fn dump_context(&self) -> String {
        let rows = self
            .vault
            .recent_workspace(&self.workspace_id, 100)
            .await
            .unwrap_or_default();

        rows.iter()
            .map(|entry| {
                format!(
                    "[{}] {}: {}",
                    entry.source,
                    Utc.timestamp_opt(entry.created_at, 0)
                        .unwrap()
                        .format("%H:%M:%S"),
                    entry.content
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
            let mut tags = vec![
                format!("workspace:{}", self.workspace_id),
                "legacy".to_string(),
                "agent_memory".to_string(),
            ];
            if !entry.source.trim().is_empty() {
                tags.push(format!("source:{}", entry.source));
            }

            let _ = self
                .vault
                .put(StoreMemoryInput {
                    id: entry.id,
                    workspace_id: self.workspace_id.clone(),
                    content: entry.content,
                    tags,
                    source: "legacy".to_string(),
                    sensitivity: MemorySensitivity::Internal,
                    metadata: entry.metadata,
                    created_at: entry.timestamp,
                    embedding: None,
                })
                .await;
        }

        let backup_path = legacy_path.with_extension("json.migrated");
        let _ = fs::rename(&legacy_path, backup_path).await;
    }
}
