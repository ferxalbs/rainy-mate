use crate::ai::router::IntelligentRouter;
use crate::services::embedder::EmbeddingTaskType;
use crate::services::memory_vault::dedup::{DedupDecision, DedupGate};
use crate::services::memory_vault::distiller::MemoryDistiller;
use crate::services::memory_vault::types::RawMemoryTurn;
use crate::services::memory_vault::MemorySensitivity;
use chrono::{TimeZone, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

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

struct DistillationBuffer {
    turns: Vec<RawMemoryTurn>,
    max_before_flush: usize,
    distiller: Arc<MemoryDistiller>,
    dedup_gate: Arc<DedupGate>,
    manager: Arc<crate::services::MemoryManager>,
    workspace_id: String,
}

impl DistillationBuffer {
    fn new(
        distiller: Arc<MemoryDistiller>,
        dedup_gate: Arc<DedupGate>,
        manager: Arc<crate::services::MemoryManager>,
        workspace_id: String,
    ) -> Self {
        Self {
            turns: Vec::with_capacity(5),
            max_before_flush: 5,
            distiller,
            dedup_gate,
            manager,
            workspace_id,
        }
    }

    fn push(&mut self, turn: RawMemoryTurn) -> bool {
        self.turns.push(turn);
        self.turns.len() >= self.max_before_flush
    }

    fn drain(&mut self) -> Vec<RawMemoryTurn> {
        std::mem::take(&mut self.turns)
    }
}

pub struct AgentMemory {
    workspace_id: String,
    db: Arc<sqlx::SqlitePool>,
    manager: Arc<crate::services::MemoryManager>,
    #[allow(dead_code)]
    http_client: Client,
    distillation_buffer: Arc<Mutex<Option<DistillationBuffer>>>,
}

impl std::fmt::Debug for AgentMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentMemory")
            .field("workspace_id", &self.workspace_id)
            .finish_non_exhaustive()
    }
}

impl Clone for AgentMemory {
    fn clone(&self) -> Self {
        Self {
            workspace_id: self.workspace_id.clone(),
            db: self.db.clone(),
            manager: self.manager.clone(),
            http_client: self.http_client.clone(),
            distillation_buffer: self.distillation_buffer.clone(),
        }
    }
}

impl AgentMemory {
    pub async fn new(
        workspace_id: &str,
        app_data_dir: PathBuf,
        manager: Arc<crate::services::MemoryManager>,
        router: Option<Arc<tokio::sync::RwLock<IntelligentRouter>>>,
        vault: Option<Arc<crate::services::memory_vault::MemoryVaultService>>,
    ) -> Self {
        let _ = std::fs::create_dir_all(&app_data_dir);
        let db_path = app_data_dir.join("rainy_mate_v2.db");
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

        let distillation_buffer = match (router, vault) {
            (Some(r), Some(v)) => {
                let distiller = Arc::new(MemoryDistiller::new(r));
                let dedup_gate = Arc::new(DedupGate::new(v));
                Some(DistillationBuffer::new(
                    distiller,
                    dedup_gate,
                    manager.clone(),
                    workspace_id.to_string(),
                ))
            }
            _ => None,
        };

        let memory = Self {
            workspace_id: workspace_id.to_string(),
            db: Arc::new(pool),
            manager,
            http_client: Client::builder()
                .user_agent("Rainy-MaTE-Agent/1.0")
                .build()
                .unwrap_or_default(),
            distillation_buffer: Arc::new(Mutex::new(distillation_buffer)),
        };

        memory.migrate_legacy_json_if_present(app_data_dir).await;
        memory
    }

    pub fn manager(&self) -> Arc<crate::services::MemoryManager> {
        self.manager.clone()
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

        let _ = self
            .manager
            .store_workspace_memory(
                &self.workspace_id,
                entry_id,
                content,
                source.clone(),
                tags,
                metadata.clone(),
                timestamp,
                MemorySensitivity::Internal,
            )
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

    pub async fn push_for_distillation(&self, turn: RawMemoryTurn) {
        let should_flush = {
            let mut guard = self.distillation_buffer.lock().await;
            match guard.as_mut() {
                Some(buf) => buf.push(turn),
                None => return, // No distillation available, skip silently
            }
        };

        if should_flush {
            self.flush_distillation_buffer().await;
        }
    }

    pub async fn flush_remaining(&self) {
        self.flush_distillation_buffer().await;
    }

    async fn flush_distillation_buffer(&self) {
        let (turns, distiller, dedup_gate, manager, workspace_id) = {
            let mut guard = self.distillation_buffer.lock().await;
            match guard.as_mut() {
                Some(buf) => {
                    let turns = buf.drain();
                    if turns.is_empty() {
                        return;
                    }
                    (
                        turns,
                        buf.distiller.clone(),
                        buf.dedup_gate.clone(),
                        buf.manager.clone(),
                        buf.workspace_id.clone(),
                    )
                }
                None => return,
            }
        };

        // Spawn the expensive work (LLM call + embedding + dedup) off the hot path
        tokio::spawn(async move {
            let distilled = match distiller.distill(turns).await {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Memory distillation failed: {}", e);
                    return;
                }
            };

            for mem in distilled {
                // Embed the distilled content
                let embedder = manager.resolve_gemini_embedder().ok().flatten();
                let embedding = if let Some(ref emb) = embedder {
                    emb.embed_text_for_model_with_task_strict(
                        &mem.content,
                        crate::services::memory_vault::EMBEDDING_MODEL,
                        EmbeddingTaskType::RetrievalDocument,
                    )
                    .await
                    .ok()
                } else {
                    None
                };

                // Dedup gate
                let decision = if let Some(ref emb) = embedding {
                    dedup_gate.gate(&workspace_id, mem.clone(), emb).await
                } else {
                    DedupDecision::Insert(mem.clone())
                };

                match decision {
                    DedupDecision::Insert(distilled) => {
                        let entry_id = uuid::Uuid::new_v4().to_string();
                        let mut metadata = HashMap::new();
                        metadata.insert("_category".to_string(), distilled.category.as_str().to_string());
                        metadata.insert("_importance".to_string(), format!("{:.2}", distilled.importance));

                        let tags = vec![
                            format!("workspace:{}", workspace_id),
                            "source:distilled".to_string(),
                            format!("category:{}", distilled.category.as_str()),
                            "agent_memory".to_string(),
                        ];

                        let _ = manager
                            .store_workspace_memory(
                                &workspace_id,
                                entry_id,
                                distilled.content,
                                "distilled".to_string(),
                                tags,
                                metadata,
                                chrono::Utc::now().timestamp(),
                                MemorySensitivity::Internal,
                            )
                            .await;
                    }
                    DedupDecision::Update { existing_id, merged } => {
                        let mut metadata = HashMap::new();
                        metadata.insert("_category".to_string(), merged.category.as_str().to_string());
                        metadata.insert("_importance".to_string(), format!("{:.2}", merged.importance));

                        let tags = vec![
                            format!("workspace:{}", workspace_id),
                            "source:distilled".to_string(),
                            format!("category:{}", merged.category.as_str()),
                            "agent_memory".to_string(),
                        ];

                        // Re-store with same ID to update
                        let _ = manager
                            .store_workspace_memory(
                                &workspace_id,
                                existing_id,
                                merged.content,
                                "distilled".to_string(),
                                tags,
                                metadata,
                                chrono::Utc::now().timestamp(),
                                MemorySensitivity::Internal,
                            )
                            .await;
                    }
                    DedupDecision::Skip => {}
                }
            }
        });
    }

    #[allow(dead_code)]
    pub async fn retrieve(&self, query: &str) -> Vec<MemoryEntry> {
        self.manager
            .search(&self.workspace_id, query, 20)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| {
                let importance = 0.5;
                MemoryEntry {
                    id: row.id,
                    content: row.content,
                    source: derive_source_from_tags(&row.tags),
                    timestamp: row.timestamp.timestamp(),
                    metadata: HashMap::new(),
                    importance,
                    sensitivity: MemorySensitivity::Internal,
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
            .manager
            .query_workspace_memory(&self.workspace_id, "", 100)
            .await
            .unwrap_or_default();

        rows.iter()
            .map(|entry| {
                format!(
                    "[{}] {}: {}",
                    derive_source_from_tags(&entry.tags),
                    Utc.timestamp_opt(entry.timestamp.timestamp(), 0)
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
                .manager
                .store_workspace_memory(
                    &self.workspace_id,
                    entry.id,
                    entry.content,
                    if entry.source.trim().is_empty() {
                        "legacy".to_string()
                    } else {
                        entry.source.clone()
                    },
                    tags,
                    entry.metadata,
                    entry.timestamp,
                    MemorySensitivity::Internal,
                )
                .await;
        }

        let backup_path = legacy_path.with_extension("json.migrated");
        let _ = fs::rename(&legacy_path, backup_path).await;
    }
}

fn derive_source_from_tags(tags: &[String]) -> String {
    tags.iter()
        .find_map(|tag| tag.strip_prefix("source:").map(|value| value.to_string()))
        .unwrap_or_else(|| "agent_memory".to_string())
}
