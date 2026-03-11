#[derive(Debug, Clone, Copy)]
pub struct EmbeddingProfile {
    pub model: &'static str,
    pub dim: usize,
}

pub const GEMINI_EMBEDDING_2_PREVIEW: EmbeddingProfile = EmbeddingProfile {
    model: "gemini-embedding-2-preview",
    dim: 3072,
};

pub const GEMINI_EMBEDDING_001: EmbeddingProfile = EmbeddingProfile {
    model: "gemini-embedding-001",
    dim: 3072,
};

pub const ACTIVE_EMBEDDING_PROFILE: EmbeddingProfile = GEMINI_EMBEDDING_2_PREVIEW;
pub const FALLBACK_EMBEDDING_PROFILE: EmbeddingProfile = GEMINI_EMBEDDING_001;
