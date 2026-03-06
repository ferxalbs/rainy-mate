use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    RainyApi,
    GeminiByok,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogModel {
    pub slug: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub provider: ModelProvider,
    pub thinking_level: Option<&'static str>,
    pub streaming: bool,
    pub function_calling: bool,
    pub vision: bool,
    pub web_search: bool,
    pub max_context: usize,
    pub max_output: usize,
}

const OBSOLETE_MODEL_SLUGS: [&str; 2] = ["gemini-2.5-flash", "gemini-2.5-flash-lite"];

pub fn normalize_model_slug(model: &str) -> &str {
    model
        .strip_prefix("rainy:")
        .or_else(|| model.strip_prefix("rainy-api/"))
        .or_else(|| model.strip_prefix("gemini:"))
        .or_else(|| model.strip_prefix("cowork:"))
        .unwrap_or(model)
}

pub fn is_rainy_catalog_slug(model: &str) -> bool {
    let normalized = normalize_model_slug(model);
    normalized.contains('/')
        && !normalized.starts_with("gemini:")
        && !normalized.starts_with("ollama:")
}

pub fn ensure_supported_model_slug(model: &str) -> Result<(), String> {
    let normalized = normalize_model_slug(model);
    if OBSOLETE_MODEL_SLUGS.contains(&normalized) {
        return Err(format!(
            "Unsupported model slug '{}'. Use 'gemini-3-flash-preview' or 'gemini-3.1-flash-lite-preview'.",
            normalized
        ));
    }
    Ok(())
}

pub fn all_catalog_models() -> Vec<CatalogModel> {
    vec![
        CatalogModel {
            slug: "gemini-3-flash-preview",
            name: "Gemini 3 Flash (BYOK)",
            description: "Google Gemini using your own API key.",
            provider: ModelProvider::GeminiByok,
            thinking_level: Some("medium"),
            streaming: true,
            function_calling: true,
            vision: false,
            web_search: false,
            max_context: 1_000_000,
            max_output: 8_192,
        },
        CatalogModel {
            slug: "gemini-3.1-flash-lite-preview",
            name: "Gemini 3.1 Flash Lite (BYOK)",
            description: "Low-latency Gemini BYOK for lightweight tasks.",
            provider: ModelProvider::GeminiByok,
            thinking_level: None,
            streaming: true,
            function_calling: true,
            vision: false,
            web_search: false,
            max_context: 1_000_000,
            max_output: 8_192,
        },
    ]
}

pub fn find_catalog_model(slug: &str, provider: ModelProvider) -> Option<CatalogModel> {
    all_catalog_models()
        .into_iter()
        .find(|entry| entry.slug == slug && entry.provider == provider)
}
