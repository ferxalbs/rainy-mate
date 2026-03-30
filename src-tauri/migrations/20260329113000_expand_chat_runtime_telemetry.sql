ALTER TABLE chat_runtime_telemetry
    ADD COLUMN execution_mode TEXT NOT NULL DEFAULT 'local';

ALTER TABLE chat_runtime_telemetry
    ADD COLUMN workspace_memory_enabled INTEGER NOT NULL DEFAULT 0;

ALTER TABLE chat_runtime_telemetry
    ADD COLUMN workspace_memory_root TEXT;
