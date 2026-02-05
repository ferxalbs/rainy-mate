use chrono::{TimeZone, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

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
    /// Working memory (reset per session or short duration)
    short_term: Arc<RwLock<Vec<MemoryEntry>>>,
    /// Web client for fetching external info
    #[allow(dead_code)]
    // @TODO used by ingest_web_page - will be fully utilized when search tool is added
    // @TODO used by ingest_web_page - will be fully utilized when search tool is added
    http_client: Client,
    /// Path to persistent storage
    storage_path: PathBuf,
}

impl AgentMemory {
    pub async fn new(workspace_id: &str, app_data_dir: PathBuf) -> Self {
        let storage_path = app_data_dir
            .join("memory")
            .join(workspace_id)
            .join("short_term.json");

        // Ensure directory exists
        if let Some(parent) = storage_path.parent() {
            let _ = fs::create_dir_all(parent).await;
        }

        // Load existing memory or create new
        let entries = if storage_path.exists() {
            match fs::read_to_string(&storage_path).await {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        Self {
            short_term: Arc::new(RwLock::new(entries)),
            http_client: Client::builder()
                .user_agent("Rainy-MaTE-Agent/1.0")
                .build()
                .unwrap_or_default(),
            storage_path,
        }
    }

    /// Add a new entry to memory
    #[allow(dead_code)]
    // @TODO Internal helper for memory storage
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

        let mut store = self.short_term.write().await;
        store.push(entry);

        // Persist to disk
        self.save_to_disk(&store).await;
    }

    async fn save_to_disk(&self, entries: &[MemoryEntry]) {
        if let Ok(content) = serde_json::to_string_pretty(entries) {
            let _ = fs::write(&self.storage_path, content).await;
        }
    }

    /// Retrieve relevant memory entries (Simple keyword match for now)
    /// In a full "Mastra" implementation, this would use vector embeddings.
    pub async fn retrieve(&self, query: &str) -> Vec<MemoryEntry> {
        let store = self.short_term.read().await;
        let query_lower = query.to_lowercase();

        // Simple relevance matching
        store
            .iter()
            .filter(|entry| entry.content.to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    }

    /// The "OpenClaw" feature: Fetch and digest web content into memory
    #[allow(dead_code)]
    // @TODO Integrates with forthcoming WebSearch tool
    pub async fn ingest_web_page(&self, url: &str) -> Result<String, String> {
        // 1. Fetch content
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

        // 2. Parse HTML
        let document = Html::parse_document(&html_content);

        // Remove scripts and styles
        let selector = Selector::parse("body").unwrap();
        let body = document.select(&selector).next();

        let text_content = if let Some(node) = body {
            // Very naive text extraction - just getting text nodes
            // Ideally use a library like `readability` port or refined scraper logic
            node.text().collect::<Vec<_>>().join(" ")
        } else {
            "No body content found".to_string()
        };

        // Clean up whitespace
        let cleaned_text = text_content
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let truncated_text: String = cleaned_text.chars().take(10000).collect(); // Limit size

        // 3. Store in Memory
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

    /// Formatting memory for LLM Context Window
    #[allow(dead_code)]
    // @TODO logic for memory visualization
    pub async fn dump_context(&self) -> String {
        let store = self.short_term.read().await;
        store
            .iter()
            .map(|e| {
                format!(
                    "[{}] {}: {}",
                    e.source,
                    Utc.timestamp_opt(e.timestamp, 0)
                        .unwrap()
                        .format("%H:%M:%S"),
                    e.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
