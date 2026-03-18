-- Add workspace_id to chats for multi-chat per workspace
ALTER TABLE chats ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'default';
CREATE INDEX IF NOT EXISTS idx_chats_workspace_id ON chats(workspace_id);
