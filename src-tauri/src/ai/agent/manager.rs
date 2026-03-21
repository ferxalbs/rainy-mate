use crate::ai::specs::manifest::AgentSpec;
use crate::db::Database;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tauri::State;

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
        let id = uuid::Uuid::new_v4().to_string();
        let mut tx = self.db.begin().await?;

        sqlx::query("INSERT INTO messages (id, chat_id, role, content) VALUES (?, ?, ?, ?)")
            .bind(&id)
            .bind(chat_id)
            .bind(role)
            .bind(content)
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
            "SELECT id, role, content FROM messages WHERE chat_id = ? ORDER BY created_at ASC",
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
            sqlx::query_as::<_, (i64, String, String, String, String, String)>(
                "SELECT rowid, id, chat_id, role, content, created_at
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
            sqlx::query_as::<_, (i64, String, String, String, String, String)>(
                "SELECT rowid, id, chat_id, role, content, created_at
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
                |(cursor_rowid, id, chat_id, role, content, created_at)| ChatHistoryMessageDto {
                    id,
                    chat_scope_id: chat_id,
                    role,
                    content,
                    created_at,
                    cursor_rowid,
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
        sqlx::query("INSERT INTO messages (id, chat_id, role, content) VALUES (?, ?, 'system', ?)")
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
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO chat_runtime_telemetry
                (chat_id, history_source, retrieval_mode, embedding_profile, updated_at)
             VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(chat_id) DO UPDATE SET
                history_source = excluded.history_source,
                retrieval_mode = excluded.retrieval_mode,
                embedding_profile = excluded.embedding_profile,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(chat_id)
        .bind(history_source)
        .bind(retrieval_mode)
        .bind(embedding_profile)
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
        let default_agent_id = crate::services::default_agent_spec::DEFAULT_LOCAL_AGENT_ID;

        // Ensure default agent exists
        let default_spec_json = build_default_agent_spec_json(default_agent_id, "Rainy Agent");
        sqlx::query(
            "INSERT INTO agents (id, name, description, soul, spec_json, version) VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
               name=excluded.name, \
               description=excluded.description, \
               soul=excluded.soul, \
               spec_json=excluded.spec_json, \
               version=excluded.version",
        )
        .bind(default_agent_id)
        .bind("Rainy Agent")
        .bind("Default Rainy agent — spawns Research, Executor, Verifier, and Memory Scribe sub-agents")
        .bind(crate::services::default_agent_spec::DEFAULT_AGENT_SOUL_MARKDOWN)
        .bind(&default_spec_json)
        .bind("3.0.0")
        .execute(&*self.db)
        .await?;

        sqlx::query("INSERT INTO chats (id, agent_id, title, workspace_id) VALUES (?, ?, NULL, ?)")
            .bind(&chat_id)
            .bind(default_agent_id)
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
        // Cascade: messages, compaction, telemetry, then chat itself
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
        sqlx::query("DELETE FROM chats WHERE id = ?")
            .bind(chat_id)
            .execute(&*self.db)
            .await?;
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
        let default_agent_id = crate::services::default_agent_spec::DEFAULT_LOCAL_AGENT_ID;
        let default_soul = crate::services::default_agent_spec::DEFAULT_AGENT_SOUL_MARKDOWN;
        let default_spec_json = build_default_agent_spec_json(default_agent_id, agent_name);

        sqlx::query(
            "INSERT INTO agents (id, name, description, soul, spec_json, version) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING",
        )
        .bind(default_agent_id)
        .bind(agent_name)
        .bind("Default Rainy agent — spawns Research, Executor, Verifier, and Memory Scribe sub-agents")
        .bind(default_soul)
        .bind(&default_spec_json)
        .bind("3.0.0")
        .execute(&*self.db)
        .await?;

        // Ensure the chat session exists with workspace_id
        sqlx::query(
            "INSERT INTO chats (id, agent_id, title, workspace_id) VALUES (?, ?, ?, ?) ON CONFLICT(id) DO NOTHING",
        )
        .bind(chat_id)
        .bind(default_agent_id)
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
        let default_agent_id = crate::services::default_agent_spec::DEFAULT_LOCAL_AGENT_ID;
        let default_soul = crate::services::default_agent_spec::DEFAULT_AGENT_SOUL_MARKDOWN;
        let default_spec_json = build_default_agent_spec_json(default_agent_id, "Rainy Agent");

        sqlx::query(
            "INSERT INTO agents (id, name, description, soul, spec_json, version) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING",
        )
        .bind(default_agent_id)
        .bind("Rainy Agent")
        .bind("Default Rainy agent — parallel supervisor")
        .bind(default_soul)
        .bind(&default_spec_json)
        .bind("3.0.0")
        .execute(&*self.db)
        .await?;

        sqlx::query(
            "INSERT INTO chats (id, agent_id, title, workspace_id, source, connector_id, remote_session_peer) \
             VALUES (?, ?, NULL, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING",
        )
        .bind(chat_id)
        .bind(default_agent_id)
        .bind(workspace_id)
        .bind(source)
        .bind(connector_id)
        .bind(remote_session_peer)
        .execute(&*self.db)
        .await?;

        Ok(())
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

// Commands to be exposed to Frontend
#[tauri::command]
pub async fn save_agent_to_db(
    state: State<'_, AgentManager>,
    id: String,
    name: String,
    description: Option<String>,
    soul: Option<String>,
) -> Result<String, String> {
    use crate::ai::specs::skills::AgentSkills;
    use crate::ai::specs::soul::AgentSoul;

    let spec = AgentSpec {
        id,
        version: "3.0.0".to_string(),
        soul: AgentSoul {
            name,
            description: description.unwrap_or_default(),
            soul_content: soul.unwrap_or_default(),
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

    state.create_agent(&spec).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_agents_from_db(
    state: State<'_, AgentManager>,
) -> Result<Vec<AgentEntity>, String> {
    state.list_agents().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_chat_message(
    state: State<'_, AgentManager>,
    chat_id: String,
    role: String,
    content: String,
) -> Result<String, String> {
    state
        .save_message(&chat_id, &role, &content)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_chat_history(
    state: State<'_, AgentManager>,
    chat_id: String,
) -> Result<Vec<(String, String, String)>, String> {
    state.get_history(&chat_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_chat_history(
    state: State<'_, AgentManager>,
    chat_id: String,
) -> Result<(), String> {
    state
        .clear_history(&chat_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn compact_session_cmd(
    state: State<'_, AgentManager>,
    chat_id: String,
    summary_content: String,
    keep_recent_count: usize,
) -> Result<(), String> {
    state
        .compact_session(&chat_id, &summary_content, keep_recent_count)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_chat_compaction_state(
    state: State<'_, AgentManager>,
    chat_scope_id: String,
) -> Result<Option<ChatCompactionStateDto>, String> {
    state
        .get_latest_chat_compaction(&chat_scope_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_chat_runtime_telemetry(
    state: State<'_, AgentManager>,
    chat_scope_id: String,
) -> Result<Option<ChatRuntimeTelemetryDto>, String> {
    state
        .get_chat_runtime_telemetry(&chat_scope_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_default_chat_scope() -> Result<String, String> {
    Ok(DEFAULT_LONG_CHAT_SCOPE_ID.to_string())
}

#[tauri::command]
pub async fn get_or_create_workspace_chat(
    state: State<'_, AgentManager>,
    workspace_id: String,
) -> Result<String, String> {
    let ws = if workspace_id.trim().is_empty() {
        DEFAULT_WORKSPACE_ID.to_string()
    } else {
        workspace_id
    };

    // Try to find the latest chat for this workspace
    if let Some(chat) = state
        .get_latest_workspace_chat(&ws)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(chat.id);
    }

    // No existing chat — create one
    let chat = state
        .create_chat_session(&ws)
        .await
        .map_err(|e| e.to_string())?;
    Ok(chat.id)
}

#[tauri::command]
pub async fn get_chat_history_window(
    state: State<'_, AgentManager>,
    chat_scope_id: String,
    cursor_rowid: Option<i64>,
    limit: Option<usize>,
) -> Result<ChatHistoryWindowDto, String> {
    state
        .get_history_window(&chat_scope_id, cursor_rowid, limit.unwrap_or(100))
        .await
        .map_err(|e| e.to_string())
}
