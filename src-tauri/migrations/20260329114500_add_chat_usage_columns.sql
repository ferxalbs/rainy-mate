ALTER TABLE chat_runtime_telemetry
    ADD COLUMN last_model TEXT;

ALTER TABLE chat_runtime_telemetry
    ADD COLUMN prompt_tokens INTEGER NOT NULL DEFAULT 0;

ALTER TABLE chat_runtime_telemetry
    ADD COLUMN completion_tokens INTEGER NOT NULL DEFAULT 0;

ALTER TABLE chat_runtime_telemetry
    ADD COLUMN total_tokens INTEGER NOT NULL DEFAULT 0;
