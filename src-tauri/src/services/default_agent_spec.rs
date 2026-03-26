use crate::ai::specs::manifest::{
    AgentSpec, AirlockConfig, ConnectorsConfig, DelegationConfig, DelegationPolicy,
    LanguagePolicyConfig, MemoryConfig, PersistenceConfig, RuntimeConfig, RuntimeMode,
};
use crate::ai::specs::{AgentSkills, AgentSoul};

pub const DEFAULT_AGENT_NAME: &str = "Rainy Agent";
pub const DEFAULT_LOCAL_AGENT_ID: &str = "rainy-agent-v1";
pub const DEFAULT_CLOUD_AGENT_ID: &str = "rainy-cloud-agent-v1";
pub const DEFAULT_CLOUD_MODEL_FALLBACK: &str = "openai/gpt-5.4-nano";

const DEFAULT_CLOUD_MODEL_PREFERENCES: &[&str] = &[
    DEFAULT_CLOUD_MODEL_FALLBACK,
    "openai/gpt-5",
    "openai/gpt-5-mini",
    "openai/gpt-5-nano",
    "gemini-3.1-flash-lite-preview",
    "gemini-3-pro-preview",
    "inception/mercury-2",
    "gemini-3-flash-preview",
];

pub const DEFAULT_AGENT_SOUL_MARKDOWN: &str = r#"# Rainy — Default Agent

You are Rainy, a powerful multi-specialist AI agent. You have access to a team of sub-agents that you orchestrate:

- **Research Agent** — gathers web/file context before execution
- **Executor Agent** — implements code and filesystem changes precisely
- **Verifier Agent** — validates results with read-only checks after writes
- **Memory Scribe** — persists important facts and user preferences to long-term memory

## Memory
- When users share their name, preferences, or any important fact, ALWAYS delegate to the Memory Scribe to save it.
- Before answering questions about the user's history, use `recall_memory` to surface relevant context.
- The Memory Scribe uses `save_memory` with descriptive tags like ["user", "name"] or ["project", "preference"].

## Behavior
- Be concise and precise. Never speculate — use tools to verify.
- Only delegate when the user explicitly asks for delegation/subagents/parallel specialist work.
- Keep internal coordination in English.
- When parallel supervisor mode is active, answer in English.
- Otherwise answer in the user's language.
- The principal agent owns the final user-facing response. Sub-agents return structured findings, not the final answer.
- Always respect the user's stated preferences and remembered context.
"#;

fn build_base_default_agent_spec(id: &str, name: &str, description: &str) -> AgentSpec {
    AgentSpec {
        id: id.to_string(),
        version: "3.0.0".to_string(),
        soul: AgentSoul {
            name: name.to_string(),
            description: description.to_string(),
            version: "3.0.0".to_string(),
            personality: "Helpful".to_string(),
            tone: "Professional".to_string(),
            soul_content: DEFAULT_AGENT_SOUL_MARKDOWN.to_string(),
            embedding: None,
        },
        skills: AgentSkills::default(),
        airlock: AirlockConfig::default(),
        memory_config: MemoryConfig::default(),
        connectors: ConnectorsConfig::default(),
        runtime: RuntimeConfig {
            mode: RuntimeMode::ParallelSupervisor,
            max_specialists: 2,
            verification_required: true,
            delegation: DelegationConfig {
                policy: DelegationPolicy::ExplicitOnly,
                ..Default::default()
            },
            language_policy: LanguagePolicyConfig {
                internal_coordination_language: "english".to_string(),
                final_response_language_mode: "english".to_string(),
            },
            ..Default::default()
        },
        model: None,
        temperature: None,
        max_tokens: None,
        provider: None,
        signature: None,
    }
}

pub fn build_default_local_agent_spec(id: &str, name: &str) -> AgentSpec {
    build_base_default_agent_spec(
        id,
        name,
        "Default Rainy agent — parallel supervisor for bounded specialist work and principal final synthesis",
    )
}

pub fn build_default_local_agent_spec_json(id: &str, name: &str) -> String {
    serde_json::to_string(&build_default_local_agent_spec(id, name)).unwrap_or_default()
}

pub fn build_default_cloud_agent_spec(model: &str) -> AgentSpec {
    let mut spec = build_base_default_agent_spec(
        DEFAULT_CLOUD_AGENT_ID,
        DEFAULT_AGENT_NAME,
        "Default Rainy agent — parallel supervisor for bounded specialist work and principal final synthesis",
    );
    spec.model = Some(model.to_string());
    spec.provider = Some("rainy".to_string());
    spec.temperature = Some(0.4);
    spec.max_tokens = Some(4096);
    spec.memory_config.persistence = PersistenceConfig {
        cross_session: true,
        per_connector_isolation: true,
        session_scope: "per_user".to_string(),
    };
    spec
}

pub fn select_default_cloud_model_id<I, S>(available_models: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let available = available_models
        .into_iter()
        .map(|value| value.as_ref().trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if available.is_empty() {
        return Err(
            "Workspace model catalog is empty; no default cloud agent can be provisioned."
                .to_string(),
        );
    }

    for preferred in DEFAULT_CLOUD_MODEL_PREFERENCES {
        if let Some(found) = available
            .iter()
            .find(|item| item.eq_ignore_ascii_case(preferred))
        {
            return Ok(found.clone());
        }
    }

    Ok(available[0].clone())
}

#[cfg(test)]
mod tests {
    use super::{
        build_default_cloud_agent_spec, select_default_cloud_model_id, DEFAULT_CLOUD_MODEL_FALLBACK,
    };

    #[test]
    fn prefers_known_cloud_model_when_available() {
        let selected = select_default_cloud_model_id(vec![
            "openai/gpt-5-mini",
            DEFAULT_CLOUD_MODEL_FALLBACK,
            "inception/mercury-2",
        ])
        .expect("selected model");

        assert_eq!(selected, DEFAULT_CLOUD_MODEL_FALLBACK);
    }

    #[test]
    fn falls_back_to_first_available_catalog_model() {
        let selected = select_default_cloud_model_id(vec!["anthropic/claude-sonnet-4", "foo/bar"])
            .expect("selected model");

        assert_eq!(selected, "anthropic/claude-sonnet-4");
    }

    #[test]
    fn cloud_spec_is_remote_safe_by_default() {
        let spec = build_default_cloud_agent_spec(DEFAULT_CLOUD_MODEL_FALLBACK);

        assert_eq!(spec.model.as_deref(), Some(DEFAULT_CLOUD_MODEL_FALLBACK));
        assert_eq!(spec.provider.as_deref(), Some("rainy"));
        assert!(spec.memory_config.persistence.cross_session);
        assert!(spec.memory_config.persistence.per_connector_isolation);
        assert_eq!(spec.memory_config.persistence.session_scope, "per_user");
        assert!(spec.airlock.scopes.allowed_paths.is_empty());
    }
}
