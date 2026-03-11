CREATE TABLE IF NOT EXISTS chat_runtime_telemetry (
    chat_id TEXT PRIMARY KEY,
    history_source TEXT NOT NULL DEFAULT 'persisted_long_chat',
    retrieval_mode TEXT NOT NULL DEFAULT 'unavailable',
    embedding_profile TEXT NOT NULL DEFAULT 'gemini-embedding-2-preview',
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(chat_id) REFERENCES chats(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chat_runtime_telemetry_updated_at
    ON chat_runtime_telemetry(updated_at DESC);
