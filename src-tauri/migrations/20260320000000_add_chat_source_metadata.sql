ALTER TABLE chats ADD COLUMN source TEXT NOT NULL DEFAULT 'local';
ALTER TABLE chats ADD COLUMN connector_id TEXT;
ALTER TABLE chats ADD COLUMN remote_session_peer TEXT;
