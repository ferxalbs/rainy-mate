-- Step 1: Create the new structure for memory_vault_entries_v3
CREATE TABLE IF NOT EXISTS memory_vault_entries_v3 (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    source TEXT NOT NULL,
    sensitivity TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_accessed INTEGER NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    content_ciphertext BLOB NOT NULL,
    content_nonce BLOB NOT NULL,
    tags_ciphertext BLOB NOT NULL,
    tags_nonce BLOB NOT NULL,
    metadata_ciphertext BLOB,
    metadata_nonce BLOB,
    embedding F32_BLOB(3072),
    embedding_model TEXT,
    embedding_provider TEXT,
    embedding_dim INTEGER,
    key_version INTEGER NOT NULL DEFAULT 1
);

-- Step 2: Copy old rows into the new schema
INSERT INTO memory_vault_entries_v3 (
    id, workspace_id, source, sensitivity, created_at, last_accessed, access_count,
    content_ciphertext, content_nonce, tags_ciphertext, tags_nonce, metadata_ciphertext, metadata_nonce, key_version
)
SELECT 
    id, workspace_id, source, sensitivity, created_at, last_accessed, access_count,
    content_ciphertext, content_nonce, tags_ciphertext, tags_nonce, metadata_ciphertext, metadata_nonce, key_version
FROM memory_vault_entries;

-- Note: We are explicitly skipping `embedding` because the old geometry was 1536 
-- and libSQL strictly asserts sizes on insertion/read against F32_BLOB.
-- Allowing `NULL` means the backfill script will run over these properties.

-- Step 3: Rename to swap tables atomically
DROP TABLE memory_vault_entries;
ALTER TABLE memory_vault_entries_v3 RENAME TO memory_vault_entries;

-- Step 4: Recreate necessary indexes
CREATE INDEX IF NOT EXISTS idx_memory_vault_workspace_time
  ON memory_vault_entries(workspace_id, created_at DESC);
