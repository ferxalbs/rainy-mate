CREATE TABLE IF NOT EXISTS chat_compaction_state (
    chat_id TEXT PRIMARY KEY,
    summary_content TEXT NOT NULL,
    source_message_count INTEGER NOT NULL DEFAULT 0,
    source_estimated_tokens INTEGER NOT NULL DEFAULT 0,
    kept_recent_count INTEGER NOT NULL DEFAULT 0,
    compression_model TEXT NOT NULL,
    compaction_count INTEGER NOT NULL DEFAULT 1,
    compressed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(chat_id) REFERENCES chats(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chat_compaction_updated_at
    ON chat_compaction_state(updated_at DESC);
