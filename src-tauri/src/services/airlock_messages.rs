use crate::models::neural::AirlockLevel;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AirlockMessage {
    pub command_id: String,
    pub intent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub payload_summary: String,
    pub airlock_level: AirlockLevel,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_at: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct AirlockMessageRow {
    command_id: String,
    intent: String,
    tool_name: Option<String>,
    payload_summary: String,
    airlock_level: i64,
    status: String,
    resolution: Option<String>,
    created_at: i64,
    updated_at: i64,
    expires_at: Option<i64>,
    resolved_at: Option<i64>,
    acknowledged_at: Option<i64>,
}

impl From<AirlockMessageRow> for AirlockMessage {
    fn from(value: AirlockMessageRow) -> Self {
        let airlock_level = match value.airlock_level {
            0 => AirlockLevel::Safe,
            1 => AirlockLevel::Sensitive,
            _ => AirlockLevel::Dangerous,
        };

        Self {
            command_id: value.command_id,
            intent: value.intent,
            tool_name: value.tool_name,
            payload_summary: value.payload_summary,
            airlock_level,
            status: value.status,
            resolution: value.resolution,
            created_at: value.created_at,
            updated_at: value.updated_at,
            expires_at: value.expires_at,
            resolved_at: value.resolved_at,
            acknowledged_at: value.acknowledged_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AirlockMessageStore {
    pool: Pool<Sqlite>,
}

impl AirlockMessageStore {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn init(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS airlock_messages (
                command_id TEXT PRIMARY KEY NOT NULL,
                intent TEXT NOT NULL,
                tool_name TEXT,
                payload_summary TEXT NOT NULL,
                airlock_level INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                resolution TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                expires_at INTEGER,
                resolved_at INTEGER,
                acknowledged_at INTEGER
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_airlock_messages_status_created ON airlock_messages(status, created_at DESC)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_pending(
        &self,
        command_id: &str,
        intent: &str,
        tool_name: Option<&str>,
        payload_summary: &str,
        airlock_level: AirlockLevel,
        created_at: i64,
        expires_at: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO airlock_messages (
                command_id, intent, tool_name, payload_summary, airlock_level,
                status, resolution, created_at, updated_at, expires_at, resolved_at, acknowledged_at
            )
            VALUES (?, ?, ?, ?, ?, 'pending', NULL, ?, ?, ?, NULL, NULL)
            ON CONFLICT(command_id) DO UPDATE SET
                intent=excluded.intent,
                tool_name=excluded.tool_name,
                payload_summary=excluded.payload_summary,
                airlock_level=excluded.airlock_level,
                status='pending',
                resolution=NULL,
                updated_at=excluded.updated_at,
                expires_at=excluded.expires_at,
                resolved_at=NULL,
                acknowledged_at=NULL
            "#,
        )
        .bind(command_id)
        .bind(intent)
        .bind(tool_name)
        .bind(payload_summary)
        .bind(airlock_level as i64)
        .bind(created_at)
        .bind(created_at)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_resolved(
        &self,
        command_id: &str,
        status: &str,
        resolution: Option<&str>,
        resolved_at: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE airlock_messages
               SET status = ?,
                   resolution = ?,
                   updated_at = ?,
                   resolved_at = ?
             WHERE command_id = ?
            "#,
        )
        .bind(status)
        .bind(resolution)
        .bind(resolved_at)
        .bind(resolved_at)
        .bind(command_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn acknowledge_message(
        &self,
        command_id: &str,
        acknowledged_at: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE airlock_messages
               SET acknowledged_at = ?, updated_at = ?
             WHERE command_id = ?
            "#,
        )
        .bind(acknowledged_at)
        .bind(acknowledged_at)
        .bind(command_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_messages(&self, limit: u32) -> Result<Vec<AirlockMessage>, sqlx::Error> {
        let rows = sqlx::query_as::<_, AirlockMessageRow>(
            r#"
            SELECT
                command_id,
                intent,
                tool_name,
                payload_summary,
                airlock_level,
                status,
                resolution,
                created_at,
                updated_at,
                expires_at,
                resolved_at,
                acknowledged_at
              FROM airlock_messages
          ORDER BY created_at DESC
             LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn count_pending(&self) -> Result<u64, sqlx::Error> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) as count FROM airlock_messages WHERE status = 'pending'",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count.max(0) as u64)
    }
}
