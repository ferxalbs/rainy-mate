use crate::db::Database;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tauri::State;

#[derive(Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentEntity {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub soul: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Clone)]
pub struct AgentManager {
    db: Arc<Pool<Sqlite>>,
}

impl AgentManager {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { db: Arc::new(pool) }
    }

    pub async fn create_agent(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        soul: Option<&str>,
    ) -> Result<String, sqlx::Error> {
        sqlx::query("INSERT INTO agents (id, name, description, soul) VALUES (?, ?, ?, ?)")
            .bind(id)
            .bind(name)
            .bind(description)
            .bind(soul)
            .execute(&*self.db)
            .await?;

        Ok(id.to_string())
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
        sqlx::query("INSERT INTO messages (id, chat_id, role, content) VALUES (?, ?, ?, ?)")
            .bind(&id)
            .bind(chat_id)
            .bind(role)
            .bind(content)
            .execute(&*self.db)
            .await?;

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

    pub async fn clear_history(&self, chat_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM messages WHERE chat_id = ?")
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
        // 1. Ensure a default agent exists for this runtime context
        // prevent duplicate key error by using INSERT OR IGNORE
        let default_agent_id = "rainy-agent-v1";
        sqlx::query(
            "INSERT INTO agents (id, name, description, soul) VALUES (?, ?, ?, ?) ON CONFLICT(id) DO NOTHING",
        )
        .bind(default_agent_id)
        .bind(agent_name)
        .bind("Default system agent")
        .bind("System agent for workspace operations")
        .execute(&*self.db)
        .await?;

        // 2. Ensure the chat session exists
        sqlx::query(
            "INSERT INTO chats (id, agent_id, title) VALUES (?, ?, ?) ON CONFLICT(id) DO NOTHING",
        )
        .bind(chat_id)
        .bind(default_agent_id)
        .bind(format!("Workspace Session: {}", chat_id))
        .execute(&*self.db)
        .await?;

        Ok(())
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
    state
        .create_agent(&id, &name, description.as_deref(), soul.as_deref())
        .await
        .map_err(|e| e.to_string())
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
