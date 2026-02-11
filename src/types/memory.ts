export interface KnowledgeFile {
  id: string;
  name: string;
  path: string;
  size_bytes: number;
  indexed_at: number;
  chunk_count: number;
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
