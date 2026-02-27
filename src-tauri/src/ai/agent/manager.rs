use crate::ai::specs::manifest::AgentSpec;
use crate::db::Database;
use libsql::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Clone, Serialize, Deserialize)]
pub struct AgentEntity {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub soul: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub spec_json: Option<String>,
    pub version: Option<String>,
}

impl AgentEntity {
    fn from_row(row: &Row) -> Result<Self, String> {
        let created_at_ts: i64 = row
            .get(4)
            .map_err(|e| format!("Failed to get created_at: {}", e))?;
        let created_at = chrono::NaiveDateTime::from_timestamp_opt(created_at_ts, 0)
            .ok_or("Invalid timestamp")?;

        Ok(Self {
            id: row.get(0).unwrap_or_default(),
            name: row.get(1).unwrap_or_default(),
            description: row.get(2).unwrap_or(None),
            soul: row.get(3).unwrap_or(None),
            created_at,
            spec_json: row.get(5).unwrap_or(None),
            version: row.get(6).unwrap_or(None),
        })
    }
}

#[derive(Clone)]
pub struct AgentManager {
    db: Arc<Connection>,
}

impl AgentManager {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: Arc::new(conn),
        }
    }

    pub async fn create_agent(&self, spec: &AgentSpec) -> Result<String, String> {
        let spec_json = serde_json::to_string(spec).unwrap_or_default();
        let now = chrono::Utc::now().timestamp();

        self.db
            .execute(
                "INSERT INTO agents (id, name, description, soul, created_at, spec_json, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    spec.id.clone(),
                    spec.soul.name.clone(),
                    spec.soul.description.clone(),
                    spec.soul.soul_content.clone(),
                    now,
                    spec_json,
                    spec.version.clone()
                ],
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(spec.id.clone())
    }

    pub async fn list_agents(&self) -> Result<Vec<AgentEntity>, String> {
        let mut rows = self
            .db
            .query("SELECT * FROM agents ORDER BY created_at DESC", ())
            .await
            .map_err(|e| e.to_string())?;

        let mut agents = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            agents.push(AgentEntity::from_row(&row)?);
        }
        Ok(agents)
    }

    pub async fn save_message(
        &self,
        chat_id: &str,
        role: &str,
        content: &str,
    ) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        self.db
            .execute(
                "INSERT INTO messages (id, chat_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id.clone(), chat_id.to_string(), role.to_string(), content.to_string(), now],
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(id)
    }

    pub async fn get_history(
        &self,
        chat_id: &str,
    ) -> Result<Vec<(String, String, String)>, String> {
        let mut rows = self
            .db
            .query(
                "SELECT id, role, content FROM messages WHERE chat_id = ?1 ORDER BY created_at ASC",
                params![chat_id.to_string()],
            )
            .await
            .map_err(|e| e.to_string())?;

        let mut messages = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            let id: String = row.get(0).unwrap_or_default();
            let role: String = row.get(1).unwrap_or_default();
            let content: String = row.get(2).unwrap_or_default();
            messages.push((id, role, content));
        }
        Ok(messages)
    }

    pub async fn clear_history(&self, chat_id: &str) -> Result<(), String> {
        self.db
            .execute(
                "DELETE FROM messages WHERE chat_id = ?1",
                params![chat_id.to_string()],
            )
            .await
            .map_err(|e| e.to_string())?;

        self.db
            .execute(
                "DELETE FROM memory_vault_entries WHERE workspace_id = ?1",
                params![chat_id.to_string()],
            )
            .await
            .map_err(|e| e.to_string())?;

        self.db
            .execute(
                "DELETE FROM agent_entities WHERE workspace_id = ?1",
                params![chat_id.to_string()],
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn ensure_chat_session(
        &self,
        chat_id: &str,
        agent_name: &str,
    ) -> Result<(), String> {
        let default_agent_id = "rainy-agent-v1";
        self.db.execute(
            "INSERT INTO agents (id, name, description, soul, created_at) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(id) DO NOTHING",
            params![
                default_agent_id,
                agent_name,
                "Default system agent",
                "System agent for workspace operations",
                chrono::Utc::now().timestamp()
            ]
        ).await.map_err(|e| e.to_string())?;

        self.db.execute(
            "INSERT INTO chats (id, agent_id, title, created_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO NOTHING",
            params![
                chat_id,
                default_agent_id,
                format!("Workspace Session: {}", chat_id),
                chrono::Utc::now().timestamp()
            ]
        ).await.map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn get_agent(&self, id: &str) -> Result<Option<AgentEntity>, String> {
        let mut rows = self
            .db
            .query("SELECT * FROM agents WHERE id = ?1", params![id.to_string()])
            .await
            .map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            Ok(Some(AgentEntity::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_agent_spec(&self, id: &str) -> Result<Option<AgentSpec>, String> {
        let entity = self.get_agent(id).await?;

        if let Some(agent) = entity {
            if let Some(json) = agent.spec_json {
                let spec: AgentSpec = serde_json::from_str(&json).map_err(|e| e.to_string())?;
                return Ok(Some(spec));
            }

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
                airlock: Default::default(),
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
        version: "3.0.0".to_string(),
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
        airlock: Default::default(),
        memory_config: Default::default(),
        connectors: Default::default(),
        signature: None,
    };

    state.create_agent(&spec).await
}

#[tauri::command]
pub async fn load_agents_from_db(
    state: State<'_, AgentManager>,
) -> Result<Vec<AgentEntity>, String> {
    state.list_agents().await
}

#[tauri::command]
pub async fn save_chat_message(
    state: State<'_, AgentManager>,
    chat_id: String,
    role: String,
    content: String,
) -> Result<String, String> {
    state.save_message(&chat_id, &role, &content).await
}

#[tauri::command]
pub async fn get_chat_history(
    state: State<'_, AgentManager>,
    chat_id: String,
) -> Result<Vec<(String, String, String)>, String> {
    state.get_history(&chat_id).await
}

#[tauri::command]
pub async fn clear_chat_history(
    state: State<'_, AgentManager>,
    chat_id: String,
) -> Result<(), String> {
    state.clear_history(&chat_id).await
}
