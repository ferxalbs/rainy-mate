export interface KnowledgeFile {
  id: string;
  name: string;
  path: string;
  size_bytes: number;
  indexed_at: number;
  chunk_count: number;
}

// ─── Memory Vault Explorer Types ──────────────────────────────────────

export interface DecryptedMemoryEntry {
  id: string;
  workspace_id: string;
  content: string;
  tags: string[];
  source: string;
  sensitivity: "public" | "internal" | "confidential";
  created_at: number;
  last_accessed: number;
  access_count: number;
  metadata: Record<string, string>;
  embedding?: number[] | null;
  embedding_model?: string | null;
  embedding_provider?: string | null;
  embedding_dim?: number | null;
}

export interface PaginatedVaultEntries {
  entries: DecryptedMemoryEntry[];
  total_count: number;
  offset: number;
  limit: number;
}

export interface WorkspaceSummary {
  workspace_id: string;
  entry_count: number;
}

export interface VaultDetailedStats {
  total_entries: number;
  workspace_entries: number;
  entries_by_sensitivity: Record<string, number>;
  entries_by_source: [string, number][];
  has_embeddings: number;
  missing_embeddings: number;
  oldest_entry: number | null;
  newest_entry: number | null;
}

export interface DeleteBatchResult {
  deleted: number;
}

export interface MemoryConfig {
  strategy: "vector" | "simple_buffer" | "hybrid";
  retrieval: {
    retention_days: number;
    max_tokens: number;
  };
  persistence: {
    cross_session: boolean;
    per_connector_isolation: boolean;
    session_scope: "per_user" | "per_channel" | "global";
  };
  knowledge: {
    enabled: boolean;
    indexed_files: KnowledgeFile[];
  };
}
