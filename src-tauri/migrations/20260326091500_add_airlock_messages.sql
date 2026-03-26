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
);

CREATE INDEX IF NOT EXISTS idx_airlock_messages_status_created
ON airlock_messages(status, created_at DESC);
