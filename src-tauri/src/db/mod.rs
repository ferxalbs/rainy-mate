use libsql::{Builder, Connection};
use std::fs;

use tauri::AppHandle;
use tauri::Manager;

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub async fn init(app_handle: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let app_dir = app_handle.path().app_data_dir()?;
        fs::create_dir_all(&app_dir)?;

        let db_path = app_dir.join("rainy_cowork_v2.db");
        if !db_path.exists() {
            fs::File::create(&db_path)?;
        }

        let db_url = db_path.to_string_lossy().to_string();
        let db = Builder::new_local(db_url)
            .build()
            .await?;
        let conn = db.connect()?;

        // Run migrations manually since we removed sqlx
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                soul TEXT,
                created_at INTEGER NOT NULL,
                spec_json TEXT,
                version TEXT
            )",
            ()
        ).await?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                chat_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            ()
        ).await?;

        conn.execute(
             "CREATE INDEX IF NOT EXISTS idx_messages_chat_id ON messages(chat_id, created_at)",
             ()
        ).await?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS chats (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                title TEXT,
                created_at INTEGER NOT NULL
            )",
            ()
        ).await?;

        Ok(Self { conn })
    }
}
