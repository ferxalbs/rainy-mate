use rainy_sdk::{
    models::{ResearchDepth, ResearchProvider},
    ResearchConfig, ResearchResult,
};
use tauri::State;

use serde::Serialize;

#[derive(Serialize)]
pub struct ResearchCommandResponse {
    pub success: bool,
    pub content: Option<String>,
    pub error: Option<String>,
    pub network: Option<String>,
    #[serde(rename = "generatedAt")]
    pub generated_at: Option<String>,
}

#[tauri::command]
pub async fn perform_research(
    topic: String,
    depth: Option<String>,
    max_sources: Option<u32>,
    provider: Option<String>,
    model: Option<String>,
    managed_research: State<'_, crate::services::managed_research::ManagedResearchService>,
) -> Result<ResearchCommandResponse, String> {
    let depth_enum = match depth.as_deref() {
        Some("advanced") => ResearchDepth::Advanced,
        _ => ResearchDepth::Basic,
    };

    let provider_enum = match provider.as_deref() {
        Some("tavily") => ResearchProvider::Tavily,
        _ => ResearchProvider::Exa,
    };

    let mut config = ResearchConfig::default()
        .with_provider(provider_enum)
        .with_depth(depth_enum)
        .with_max_sources(max_sources.unwrap_or(10));

    if let Some(m) = model {
        config = config.with_model(m);
    }

    match managed_research.perform_research(topic, Some(config)).await {
        Ok(result) => Ok(ResearchCommandResponse {
            success: true,
            content: Some(result.content),
            error: None,
            network: Some(result.provider),
            generated_at: Some(chrono::Utc::now().to_rfc3339()),
        }),
        Err(e) => Err(e),
    }
}
