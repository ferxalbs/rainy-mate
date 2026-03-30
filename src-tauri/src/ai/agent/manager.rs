use crate::ai::specs::manifest::AgentSpec;
use crate::db::Database;
use crate::services::chat_artifacts::ChatArtifact;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

/// @deprecated — kept only for backward-compatible migration of the legacy single-scope chat.
pub const DEFAULT_LONG_CHAT_SCOPE_ID: &str = "global:long_chat:v1";

const DEFAULT_WORKSPACE_ID: &str = "default";

fn build_default_agent_spec_json(id: &str, name: &str) -> String {
    crate::services::default_agent_spec::build_default_local_agent_spec_json(id, name)
}

#[derive(Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentEntity {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub soul: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub spec_json: Option<String>,
    pub version: Option<String>,
}

#[derive(Clone)]
pub struct AgentManager {
    db: Arc<Pool<Sqlite>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatHistoryMessageDto {
    pub id: String,
    pub chat_scope_id: String,
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<ChatArtifact>>,
    pub created_at: String,
    pub cursor_rowid: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatHistoryWindowDto {
    pub messages: Vec<ChatHistoryMessageDto>,
    pub has_more: bool,
    pub next_cursor_rowid: Option<i64>,
}

#[derive(Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatCompactionStateDto {
    pub chat_id: String,
    pub summary_content: String,
    pub source_message_count: i64,
    pub source_estimated_tokens: i64,
    pub kept_recent_count: i64,
    pub compression_model: String,
    pub compaction_count: i64,
    pub compressed_at: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatRuntimeTelemetryDto {
    pub chat_id: String,
    pub history_source: String,
    pub retrieval_mode: String,
    pub embedding_profile: String,
    pub execution_mode: String,
    pub workspace_memory_enabled: bool,
    pub workspace_memory_root: Option<String>,
    pub updated_at: String,
}

#[derive(Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatSessionDto {
    pub id: String,
    pub title: Option<String>,
    pub workspace_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
    pub last_message_at: Option<String>,
}

impl AgentManager {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { db: Arc::new(pool) }
    }

    pub async fn create_agent(&self, spec: &AgentSpec) -> Result<String, sqlx::Error> {
        let spec_json = serde_json::to_string(spec).unwrap_or_default();

        sqlx::query("INSERT INTO agents (id, name, description, soul, spec_json, version) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&spec.id)
            .bind(&spec.soul.name)
            .bind(&spec.soul.description)
            .bind(&spec.soul.soul_content)
            .bind(spec_json)
            .bind(&spec.version)
            .execute(&*self.db)
            .await?;

        Ok(spec.id.clone())
    }

    pub async fn list_agents(&self) -> Result<Vec<AgentEntity>, sqlx::Error> {
        let agents =
            sqlx::query_as::<_, AgentEntity>("SELECT * FROM agents ORDER BY created_at DESC")
                .fetch_all(&*self.db)
                .await?;
        Ok(agents)
    }

    pub async fn save_message(
        &self,
        chat_id: &str,
        role: &str,
        content: &str,
    ) -> Result<String, sqlx::Error> {
        self.save_message_with_artifacts(chat_id, role, content, None)
            .await
    }

    pub async fn save_message_with_artifacts(
        &self,
        chat_id: &str,
        role: &str,
        content: &str,
        artifacts: Option<&[ChatArtifact]>,
    ) -> Result<String, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let mut tx = self.db.begin().await?;
        let artifacts_json = artifacts
            .filter(|items| !items.is_empty())
            .and_then(|items| serde_json::to_string(items).ok());

        sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, artifacts_json) VALUES (?, ?, ?, ?, ?)",
        )
            .bind(&id)
            .bind(chat_id)
            .bind(role)
            .bind(content)
            .bind(artifacts_json)
            .execute(&mut *tx)
            .await?;

        sqlx::query("UPDATE chats SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(chat_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(id)
    }

    pub async fn get_history(
        &self,
        chat_id: &str,
    ) -> Result<Vec<(String, String, String)>, sqlx::Error> {
        let messages = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, role, content FROM messages WHERE chat_id = ? ORDER BY rowid ASC",
        )
        .bind(chat_id)
        .fetch_all(&*self.db)
        .await?;
        Ok(messages)
    }

    pub async fn get_history_window(
        &self,
        chat_scope_id: &str,
        cursor_rowid: Option<i64>,
        limit: usize,
    ) -> Result<ChatHistoryWindowDto, sqlx::Error> {
        let safe_limit = limit.clamp(1, 200) as i64;
        let rows = if let Some(cursor) = cursor_rowid {
            sqlx::query_as::<_, (i64, String, String, String, String, Option<String>, String)>(
                "SELECT rowid, id, chat_id, role, content, artifacts_json, created_at
                 FROM messages
                 WHERE chat_id = ? AND rowid < ?
                 ORDER BY rowid DESC
                 LIMIT ?",
            )
            .bind(chat_scope_id)
            .bind(cursor)
            .bind(safe_limit)
            .fetch_all(&*self.db)
            .await?
        } else {
            sqlx::query_as::<_, (i64, String, String, String, String, Option<String>, String)>(
                "SELECT rowid, id, chat_id, role, content, artifacts_json, created_at
                 FROM messages
                 WHERE chat_id = ?
                 ORDER BY rowid DESC
                 LIMIT ?",
            )
            .bind(chat_scope_id)
            .bind(safe_limit)
            .fetch_all(&*self.db)
            .await?
        };

        let oldest_cursor = rows.last().map(|row| row.0);
        let has_more = if let Some(oldest) = oldest_cursor {
            let (count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(1) as count FROM messages WHERE chat_id = ? AND rowid < ?",
            )
            .bind(chat_scope_id)
            .bind(oldest)
            .fetch_one(&*self.db)
            .await?;
            count > 0
        } else {
            false
        };

        let mut messages: Vec<ChatHistoryMessageDto> = rows
            .into_iter()
            .map(
                |(cursor_rowid, id, chat_id, role, content, artifacts_json, created_at)| {
                    ChatHistoryMessageDto {
                        id,
                        chat_scope_id: chat_id,
                        role,
                        content,
                        artifacts: artifacts_json.as_deref().and_then(|value| {
                            serde_json::from_str::<Vec<ChatArtifact>>(value).ok()
                        }),
                        created_at,
                        cursor_rowid,
                    }
                },
            )
            .collect();
        messages.reverse();

        Ok(ChatHistoryWindowDto {
            messages,
            has_more,
            next_cursor_rowid: oldest_cursor,
        })
    }

    pub async fn clear_history(&self, chat_id: &str) -> Result<(), sqlx::Error> {
        // Clear chat messages (per session)
        sqlx::query("DELETE FROM messages WHERE chat_id = ?")
            .bind(chat_id)
            .execute(&*self.db)
            .await?;

        sqlx::query("DELETE FROM chat_compaction_state WHERE chat_id = ?")
            .bind(chat_id)
            .execute(&*self.db)
            .await?;

        sqlx::query("DELETE FROM chat_runtime_telemetry WHERE chat_id = ?")
            .bind(chat_id)
            .execute(&*self.db)
            .await?;

        sqlx::query("UPDATE chats SET title = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(chat_id)
            .execute(&*self.db)
            .await?;

        Ok(())
    }

    // @TODO: This will be called by context_budget overflow triggered via TaskManager background loop
    pub async fn compact_session(
        &self,
        chat_id: &str,
        summary_content: &str,
        keep_recent_count: usize,
    ) -> Result<(), sqlx::Error> {
        let _ = self
            .compact_session_with_rolling_summary(
                chat_id,
                summary_content,
                keep_recent_count,
                0,
                "legacy_compactor",
            )
            .await?;
        Ok(())
    }

    pub async fn compact_session_with_rolling_summary(
        &self,
        chat_id: &str,
        summary_content: &str,
        keep_recent_count: usize,
        source_estimated_tokens: usize,
        compression_model: &str,
    ) -> Result<Option<ChatCompactionStateDto>, sqlx::Error> {
        let mut tx = self.db.begin().await?;

        let messages = sqlx::query_as::<_, (i64, String)>(
            "SELECT rowid, id FROM messages WHERE chat_id = ? ORDER BY rowid ASC",
        )
        .bind(chat_id)
        .fetch_all(&mut *tx)
        .await?;

        if messages.len() <= keep_recent_count {
            tx.rollback().await?;
            return Ok(None);
        }

        let source_message_count = messages.len();
        let delete_count = source_message_count - keep_recent_count;
        let to_delete: Vec<String> = messages
            .iter()
            .take(delete_count)
            .map(|(_, id)| id.clone())
            .collect();

        for id in to_delete {
            sqlx::query("DELETE FROM messages WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }

        let summary_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, artifacts_json) VALUES (?, ?, 'system', ?, NULL)",
        )
            .bind(summary_id)
            .bind(chat_id)
            .bind(format!("SESSION COMPACTION SUMMARY:\n{}", summary_content))
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO chat_compaction_state (
                chat_id,
                summary_content,
                source_message_count,
                source_estimated_tokens,
                kept_recent_count,
                compression_model,
                compaction_count,
                compressed_at,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(chat_id) DO UPDATE SET
                summary_content = excluded.summary_content,
                source_message_count = excluded.source_message_count,
                source_estimated_tokens = excluded.source_estimated_tokens,
                kept_recent_count = excluded.kept_recent_count,
                compression_model = excluded.compression_model,
                compaction_count = chat_compaction_state.compaction_count + 1,
                compressed_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(chat_id)
        .bind(summary_content)
        .bind(source_message_count as i64)
        .bind(source_estimated_tokens as i64)
        .bind(keep_recent_count as i64)
        .bind(compression_model)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_latest_chat_compaction(chat_id).await
    }

    pub async fn get_latest_chat_compaction(
        &self,
        chat_id: &str,
    ) -> Result<Option<ChatCompactionStateDto>, sqlx::Error> {
        sqlx::query_as::<_, ChatCompactionStateDto>(
            "SELECT
                chat_id,
                summary_content,
                source_message_count,
                source_estimated_tokens,
                kept_recent_count,
                compression_model,
                compaction_count,
                compressed_at,
                updated_at
             FROM chat_compaction_state
             WHERE chat_id = ?",
        )
        .bind(chat_id)
        .fetch_optional(&*self.db)
        .await
    }

    pub async fn upsert_chat_runtime_telemetry(
        &self,
        chat_id: &str,
        history_source: &str,
        retrieval_mode: &str,
        embedding_profile: &str,
        execution_mode: &str,
        workspace_memory_enabled: bool,
        workspace_memory_root: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO chat_runtime_telemetry
                (chat_id, history_source, retrieval_mode, embedding_profile, execution_mode, workspace_memory_enabled, workspace_memory_root, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(chat_id) DO UPDATE SET
                history_source = excluded.history_source,
                retrieval_mode = excluded.retrieval_mode,
                embedding_profile = excluded.embedding_profile,
                execution_mode = excluded.execution_mode,
                workspace_memory_enabled = excluded.workspace_memory_enabled,
                workspace_memory_root = excluded.workspace_memory_root,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(chat_id)
        .bind(history_source)
        .bind(retrieval_mode)
        .bind(embedding_profile)
        .bind(execution_mode)
        .bind(workspace_memory_enabled)
        .bind(workspace_memory_root)
        .execute(&*self.db)
        .await?;
        Ok(())
    }

    pub async fn get_chat_runtime_telemetry(
        &self,
        chat_id: &str,
    ) -> Result<Option<ChatRuntimeTelemetryDto>, sqlx::Error> {
        sqlx::query_as::<_, ChatRuntimeTelemetryDto>(
            "SELECT
                chat_id,
                history_source,
                retrieval_mode,
                embedding_profile,
                execution_mode,
                workspace_memory_enabled,
                workspace_memory_root,
                updated_at
             FROM chat_runtime_telemetry
             WHERE chat_id = ?",
        )
        .bind(chat_id)
        .fetch_optional(&*self.db)
        .await
    }

    pub async fn get_chat_session(
        &self,
        chat_id: &str,
    ) -> Result<Option<ChatSessionDto>, sqlx::Error> {
        sqlx::query_as::<_, ChatSessionDto>(
            "SELECT
                chats.id,
                chats.title,
                chats.workspace_id,
                chats.created_at,
                chats.updated_at,
                COUNT(messages.id) AS message_count,
                MAX(messages.created_at) AS last_message_at
             FROM chats
             LEFT JOIN messages ON messages.chat_id = chats.id
             WHERE chats.id = ?
             GROUP BY chats.id, chats.title, chats.workspace_id, chats.created_at, chats.updated_at",
        )
        .bind(chat_id)
        .fetch_optional(&*self.db)
        .await
    }

    pub async fn list_chat_sessions(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<ChatSessionDto>, sqlx::Error> {
        sqlx::query_as::<_, ChatSessionDto>(
            "SELECT
                chats.id,
                chats.title,
                chats.workspace_id,
                chats.created_at,
                chats.updated_at,
                COUNT(messages.id) AS message_count,
                MAX(messages.created_at) AS last_message_at
             FROM chats
             LEFT JOIN messages ON messages.chat_id = chats.id
             WHERE chats.workspace_id = ?
             GROUP BY chats.id, chats.title, chats.workspace_id, chats.created_at, chats.updated_at
             ORDER BY chats.updated_at DESC",
        )
        .bind(workspace_id)
        .fetch_all(&*self.db)
        .await
    }

    pub async fn create_chat_session(
        &self,
        workspace_id: &str,
    ) -> Result<ChatSessionDto, sqlx::Error> {
        let chat_id = uuid::Uuid::new_v4().to_string();
        let default_agent_id = self.ensure_default_local_agent().await?;

        sqlx::query("INSERT INTO chats (id, agent_id, title, workspace_id) VALUES (?, ?, NULL, ?)")
            .bind(&chat_id)
            .bind(&default_agent_id)
            .bind(workspace_id)
            .execute(&*self.db)
            .await?;

        self.get_chat_session(&chat_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn find_latest_empty_workspace_chat(
        &self,
        workspace_id: &str,
    ) -> Result<Option<ChatSessionDto>, sqlx::Error> {
        sqlx::query_as::<_, ChatSessionDto>(
            "SELECT
                chats.id,
                chats.title,
                chats.workspace_id,
                chats.created_at,
                chats.updated_at,
                COUNT(messages.id) AS message_count,
                MAX(messages.created_at) AS last_message_at
             FROM chats
             LEFT JOIN messages ON messages.chat_id = chats.id
             WHERE chats.workspace_id = ?
             GROUP BY chats.id, chats.title, chats.workspace_id, chats.created_at, chats.updated_at
             HAVING COUNT(messages.id) = 0
             ORDER BY chats.updated_at DESC
             LIMIT 1",
        )
        .bind(workspace_id)
        .fetch_optional(&*self.db)
        .await
    }

    pub async fn create_or_reuse_empty_chat_session(
        &self,
        workspace_id: &str,
    ) -> Result<ChatSessionDto, sqlx::Error> {
        if let Some(existing) = self.find_latest_empty_workspace_chat(workspace_id).await? {
            return Ok(existing);
        }

        self.create_chat_session(workspace_id).await
    }

    pub async fn delete_chat_session(&self, chat_id: &str) -> Result<(), sqlx::Error> {
        // Cascade inside a transaction so a mid-deletion crash leaves no orphaned rows
        let mut tx = self.db.begin().await?;
        sqlx::query("DELETE FROM messages WHERE chat_id = ?")
            .bind(chat_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM chat_compaction_state WHERE chat_id = ?")
            .bind(chat_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM chat_runtime_telemetry WHERE chat_id = ?")
            .bind(chat_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM chats WHERE id = ?")
            .bind(chat_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_latest_workspace_chat(
        &self,
        workspace_id: &str,
    ) -> Result<Option<ChatSessionDto>, sqlx::Error> {
        sqlx::query_as::<_, ChatSessionDto>(
            "SELECT
                chats.id,
                chats.title,
                chats.workspace_id,
                chats.created_at,
                chats.updated_at,
                COUNT(messages.id) AS message_count,
                MAX(messages.created_at) AS last_message_at
             FROM chats
             LEFT JOIN messages ON messages.chat_id = chats.id
             WHERE chats.workspace_id = ?
             GROUP BY chats.id, chats.title, chats.workspace_id, chats.created_at, chats.updated_at
             ORDER BY chats.updated_at DESC
             LIMIT 1",
        )
        .bind(workspace_id)
        .fetch_optional(&*self.db)
        .await
    }

    pub async fn update_chat_title(
        &self,
        chat_id: &str,
        title: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE chats SET title = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(title)
            .bind(chat_id)
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    pub async fn ensure_chat_session(
        &self,
        chat_id: &str,
        agent_name: &str,
    ) -> Result<(), sqlx::Error> {
        self.ensure_chat_session_with_workspace(chat_id, agent_name, DEFAULT_WORKSPACE_ID)
            .await
    }

    pub async fn ensure_chat_session_with_workspace(
        &self,
        chat_id: &str,
        agent_name: &str,
        workspace_id: &str,
    ) -> Result<(), sqlx::Error> {
        let default_agent_id = self.ensure_default_local_agent_named(agent_name).await?;

        // Ensure the chat session exists with workspace_id
        sqlx::query(
            "INSERT INTO chats (id, agent_id, title, workspace_id) VALUES (?, ?, ?, ?) ON CONFLICT(id) DO NOTHING",
        )
        .bind(chat_id)
        .bind(&default_agent_id)
        .bind(Option::<String>::None)
        .bind(workspace_id)
        .execute(&*self.db)
        .await?;

        Ok(())
    }

    pub async fn ensure_chat_session_with_source(
        &self,
        chat_id: &str,
        workspace_id: &str,
        source: &str,
        connector_id: Option<&str>,
        remote_session_peer: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let default_agent_id = self.ensure_default_local_agent().await?;

        sqlx::query(
            "INSERT INTO chats (id, agent_id, title, workspace_id, source, connector_id, remote_session_peer) \
             VALUES (?, ?, NULL, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING",
        )
        .bind(chat_id)
        .bind(&default_agent_id)
        .bind(workspace_id)
        .bind(source)
        .bind(connector_id)
        .bind(remote_session_peer)
        .execute(&*self.db)
        .await?;

        Ok(())
    }

    pub async fn ensure_default_local_agent(&self) -> Result<String, sqlx::Error> {
        self.ensure_default_local_agent_named("Rainy Agent").await
    }

    pub async fn ensure_default_local_agent_named(
        &self,
        agent_name: &str,
    ) -> Result<String, sqlx::Error> {
        let default_agent_id = crate::services::default_agent_spec::DEFAULT_LOCAL_AGENT_ID;
        let default_spec_json = build_default_agent_spec_json(default_agent_id, agent_name);

        sqlx::query(
            "INSERT INTO agents (id, name, description, soul, spec_json, version) VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(default_agent_id)
        .bind(agent_name)
        .bind("Default Rainy agent — local-first secure runtime with bounded specialist work")
        .bind(crate::services::default_agent_spec::DEFAULT_AGENT_SOUL_MARKDOWN)
        .bind(&default_spec_json)
        .bind("3.0.0")
        .execute(&*self.db)
        .await?;

        Ok(default_agent_id.to_string())
    }

    pub async fn get_agent(&self, id: &str) -> Result<Option<AgentEntity>, sqlx::Error> {
        let agent = sqlx::query_as::<_, AgentEntity>("SELECT * FROM agents WHERE id = ?")
            .bind(id)
            .fetch_optional(&*self.db)
            .await?;
        Ok(agent)
    }

    pub async fn get_agent_spec(&self, id: &str) -> Result<Option<AgentSpec>, String> {
        let entity = self.get_agent(id).await.map_err(|e| e.to_string())?;

        if let Some(agent) = entity {
            if let Some(json) = agent.spec_json {
                let spec: AgentSpec = serde_json::from_str(&json).map_err(|e| e.to_string())?;
                return Ok(Some(spec));
            }

            // Fallback for legacy agents or migrations
            use crate::ai::specs::skills::AgentSkills;
            use crate::ai::specs::soul::AgentSoul;

            let spec = AgentSpec {
                id: agent.id,
                version: "1.0.0".to_string(),
                soul: AgentSoul {
                    name: agent.name,
                    description: agent.description.unwrap_or_default(),
                    soul_content: agent.soul.unwrap_or_default(),
                    ..Default::default()
                },
                skills: AgentSkills::default(),
                airlock: Default::default(),
                memory_config: Default::default(),
                connectors: Default::default(),
                runtime: Default::default(),
                model: None,
                temperature: None,
                max_tokens: None,
                provider: None,
                signature: None,
            };
            Ok(Some(spec))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_manager() -> AgentManager {
        // Initialize libsql C-state before sqlx to prevent "Once poisoned" panic.
        // libsql and sqlx both embed SQLite; whichever initializes first wins.
        // manager tests run first alphabetically, so this fix covers the whole binary.
        let _ = libsql::Builder::new_local(":memory:").build().await;

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite pool");

        sqlx::query(
            "CREATE TABLE agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                soul TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                spec_json TEXT,
                version TEXT
            )",
        )
        .execute(&pool)
        .await
        .expect("create agents table");

        sqlx::query(
            "CREATE TABLE chats (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                title TEXT,
                workspace_id TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .expect("create chats table");

        sqlx::query(
            "CREATE TABLE messages (
                id TEXT PRIMARY KEY,
                chat_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                artifacts_json TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .expect("create messages table");

        sqlx::query(
            "CREATE TABLE chat_compaction_state (
                chat_id TEXT PRIMARY KEY,
                summary_content TEXT NOT NULL,
                source_message_count INTEGER NOT NULL,
                source_estimated_tokens INTEGER NOT NULL,
                kept_recent_count INTEGER NOT NULL,
                compression_model TEXT NOT NULL,
                compaction_count INTEGER NOT NULL,
                compressed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .expect("create compaction table");

        sqlx::query(
            "CREATE TABLE chat_runtime_telemetry (
                chat_id TEXT PRIMARY KEY,
                history_source TEXT NOT NULL,
                retrieval_mode TEXT NOT NULL,
                embedding_profile TEXT NOT NULL,
                execution_mode TEXT NOT NULL DEFAULT 'local',
                workspace_memory_enabled INTEGER NOT NULL DEFAULT 0,
                workspace_memory_root TEXT,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .expect("create telemetry table");

        AgentManager::new(pool)
    }

    #[tokio::test]
    async fn get_history_preserves_insert_order_for_same_second_messages() {
        let manager = setup_manager().await;
        let chat_id = "chat-history-order";

        manager
            .ensure_chat_session_with_workspace(chat_id, "Rainy Agent", "workspace")
            .await
            .expect("create chat");

        manager
            .save_message(chat_id, "user", "first")
            .await
            .expect("save first");
        manager
            .save_message(chat_id, "assistant", "second")
            .await
            .expect("save second");
        manager
            .save_message(chat_id, "user", "third")
            .await
            .expect("save third");

        let history = manager.get_history(chat_id).await.expect("load history");
        let contents: Vec<String> = history.into_iter().map(|(_, _, content)| content).collect();
        assert_eq!(contents, vec!["first", "second", "third"]);
    }
}
