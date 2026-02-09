use crate::ai::specs::manifest::AgentSpec;
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
    pub spec_json: Option<String>,
    pub version: Option<String>,
}

#[derive(Clone)]
pub struct AgentManager {
    db: Arc<Pool<Sqlite>>,
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
        // ... (existing code)
        // 1. Ensure a default agent exists for this runtime context
        // prevent duplicate key error by using INSERT OR IGNORE
        let default_agent_id = "rainy-agent-v1";
        // Need to create a default Spec to insert if missing
        // For now, simpler query just to satisfy the constraint
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
                skills: AgentSkills {
                    capabilities: vec![],
                    tools: std::collections::HashMap::new(),
                },
                memory_config: Default::default(),
                connectors: Default::default(),
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
        version: "2.0.0".to_string(),
        soul: AgentSoul {
            name,
            description: description.unwrap_or_default(),
            soul_content: soul.unwrap_or_default(),
            ..Default::default()
        },
        skills: AgentSkills {
            capabilities: vec![],
            tools: std::collections::HashMap::new(),
        },
        memory_config: Default::default(),
        connectors: Default::default(),
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
