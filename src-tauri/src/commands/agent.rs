use crate::ai::agent::runtime::{AgentContent, AgentMessage, AgentRuntime, RuntimeOptions};
use crate::ai::agent::runtime_registry::RuntimeRegistry;
use crate::ai::specs::AgentSpec;
use crate::ai::{
    keychain::KeychainManager,
    provider_trait::{AIProviderFactory, ProviderWithStats},
    provider_types::{ProviderConfig, ProviderId, ProviderType},
    providers::{GeminiProviderFactory, RainySDKProviderFactory},
};
use crate::commands::ai_providers::ProviderRegistryState;
use crate::commands::airlock::AirlockServiceState;
use crate::commands::memory::MemoryManagerState;
use crate::commands::router::IntelligentRouterState;
use crate::services::SkillExecutor;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

const MAX_HISTORY_MESSAGES: usize = 30;
const MAX_HISTORY_MESSAGE_CHARS: usize = 4000;

fn is_valid_rainy_api_key(api_key: &str) -> bool {
    api_key.trim_start().starts_with("ra-")
}

fn truncate_text(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let out: String = input.chars().take(max_chars).collect();
    format!("{}\n\n[TRUNCATED]", out)
}

fn build_runtime_history(rows: Vec<(String, String, String)>) -> Vec<AgentMessage> {
    let mut selected: Vec<AgentMessage> = rows
        .into_iter()
        .filter_map(|(_, role, content)| {
            if role != "user" && role != "assistant" {
                return None;
            }
            Some(AgentMessage {
                role,
                content: AgentContent::text(truncate_text(&content, MAX_HISTORY_MESSAGE_CHARS)),
                tool_calls: None,
                tool_call_id: None,
            })
        })
        .collect();

    if selected.len() > MAX_HISTORY_MESSAGES {
        let skip = selected.len() - MAX_HISTORY_MESSAGES;
        selected = selected.into_iter().skip(skip).collect();
    }

    selected
}

fn default_instructions(workspace_id: &str) -> String {
    format!(
        "You are Rainy Agent, an autonomous AI assistant capable of performing complex tasks in the workspace.
        
        Workspace Path: {}
        
        CAPABILITIES:
        - You can read, write, list, and search files in the workspace.
        - **MULTIMODAL: You can SEE images.** If you use `read_file` on an image, you will receive its visual content.
        - You can plan multi-step tasks.
        - You may use shell tools only when available through the provided tools.
        - Shell `execute_command` is restricted by an allowlist. Typical allowed commands include: `npm`, `cargo`, `git`, `ls`, `grep`, `echo`, `cat`. Commands like `find` may be blocked.
        
        GUIDELINES:
        1. PLAN: Before executing, briefly state your plan.
        2. EXECUTE: Use the provided tools to carry out the plan.
        3. VERIFY: After critical operations, verify the result (e.g., read_file after write_file).
        4. TOOL AWARENESS: Never claim you executed a command unless the corresponding tool call succeeded.
        5. FAILURE HONESTY: If a tool fails or is blocked by policy, tell the user exactly what failed and why.
        6. NO FABRICATION: Do not invent scan results, file contents, diffs, hashes, or command output.
        7. FALLBACKS ONLY: After a tool failure, either try a permitted alternative tool or ask the user for the missing data.
        
        Tools are provided natively. Use them for all file operations.
        Trust tool outputs over assumptions.
        If a tool fails, analyze the error and try a different permitted approach. If no permitted approach exists, stop and report the limitation clearly.",
        workspace_id
    )
}

async fn ensure_provider_ready_for_model(
    model_id: &str,
    registry: &ProviderRegistryState,
    router: &IntelligentRouterState,
) -> Result<(), String> {
    let keychain = KeychainManager::new();
    let normalized_model = crate::ai::model_catalog::normalize_model_slug(model_id).to_string();

    let (provider_id, provider_factory_kind, key_aliases): (
        &str,
        &str,
        &[&str],
    ) = if crate::ai::model_catalog::requires_rainy_provider(model_id) {
        ("rainy_api", "rainy", &["rainy_api", "rainyapi"])
    } else if crate::ai::model_catalog::is_explicit_gemini_model(model_id)
        || crate::ai::model_catalog::is_unprefixed_gemini_model(model_id)
    {
        ("gemini_byok", "gemini", &["gemini"])
    } else {
        return Ok(());
    };

    if registry.0.get(&ProviderId::new(provider_id)).is_err() {
        let api_key = key_aliases
            .iter()
            .find_map(|alias| keychain.get_key(alias).ok().flatten())
            .filter(|key| {
                provider_factory_kind != "rainy" || is_valid_rainy_api_key(key)
            });

        let api_key = api_key.ok_or_else(|| {
            if provider_factory_kind == "rainy" {
                format!(
                    "Rainy API key/provider unavailable for model '{}'. Configure 'rainy_api' with a current 'ra-' key before running this agent.",
                    model_id
                )
            } else {
                format!(
                    "Gemini BYOK key/provider unavailable for model '{}'. Configure 'gemini' before running this agent.",
                    model_id
                )
            }
        })?;

        let config = ProviderConfig {
            id: ProviderId::new(provider_id),
            provider_type: if provider_factory_kind == "rainy" {
                ProviderType::RainySDK
            } else {
                ProviderType::Google
            },
            api_key: Some(api_key),
            base_url: None,
            model: normalized_model,
            params: std::collections::HashMap::new(),
            enabled: true,
            priority: if provider_factory_kind == "rainy" { 10 } else { 20 },
            rate_limit: None,
            timeout: 120,
        };

        let provider = if provider_factory_kind == "rainy" {
            <RainySDKProviderFactory as AIProviderFactory>::create(config)
                .await
                .map_err(|e| format!("Failed to initialize Rainy provider: {}", e))?
        } else {
            <GeminiProviderFactory as AIProviderFactory>::create(config)
                .await
                .map_err(|e| format!("Failed to initialize Gemini provider: {}", e))?
        };

        registry
            .0
            .register(provider.clone())
            .map_err(|e| format!("Failed to register provider '{}': {}", provider_id, e))?;
    }

    let mut router_guard = router.0.write().await;
    let already_present = router_guard
        .get_all_providers()
        .iter()
        .any(|p| p.provider().id().as_str() == provider_id);

    if !already_present {
        let provider = registry
            .0
            .get(&ProviderId::new(provider_id))
            .map_err(|e| format!("Provider '{}' not available after registration: {}", provider_id, e))?;
        router_guard.add_provider(Arc::new(ProviderWithStats::new(provider.provider.clone())));
    }

    Ok(())
}

#[tauri::command]
pub async fn run_agent_workflow(
    app_handle: tauri::AppHandle,
    prompt: String,
    model_id: String,
    workspace_id: String,
    agent_spec_id: Option<String>,
    chat_scope_id: Option<String>,
    router: State<'_, IntelligentRouterState>,
    airlock_state: State<'_, AirlockServiceState>,
    provider_registry: State<'_, ProviderRegistryState>,
    memory_manager: State<'_, MemoryManagerState>,
    skills: State<'_, Arc<SkillExecutor>>,
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    runtime_registry: State<'_, Arc<RuntimeRegistry>>,
) -> Result<String, String> {
    crate::ai::model_catalog::ensure_supported_model_slug(&model_id)?;
    ensure_provider_ready_for_model(&model_id, &provider_registry, &router).await?;
    let selected_model_id = model_id.clone();

    let workspace_path = workspace_id.clone();
    let chat_id = chat_scope_id.unwrap_or_else(|| {
        crate::ai::agent::manager::DEFAULT_LONG_CHAT_SCOPE_ID.to_string()
    });

    // 0. Ensure Chat Session Exists (Persist Metadata)
    let _ = agent_manager
        .ensure_chat_session(&chat_id, "Rainy Agent")
        .await
        .map_err(|e| format!("Failed to initialize chat session: {}", e))?;

    // 1. Initialize Runtime (Ephemeral for now, persistent later)
    // 1. Initialize Runtime (Ephemeral for now, persistent later)
    let spec = if let Some(spec_id) = agent_spec_id {
        // Try DB first, then fall back to file-based spec storage
        let db_spec = match agent_manager.get_agent_spec(&spec_id).await {
            Ok(Some(s)) => Some(s),
            Ok(None) => None,
            Err(e) => {
                eprintln!(
                    "[AgentWorkflow] DB lookup for spec {} failed: {}, trying file fallback",
                    spec_id, e
                );
                None
            }
        };

        // Fallback: try loading from agent_specs/ JSON files (canonical source from AgentBuilder)
        let spec = match db_spec {
            Some(s) => s,
            None => {
                let app_data_dir = app_handle
                    .path()
                    .app_data_dir()
                    .map_err(|e| format!("Failed to get app data dir: {}", e))?;
                let spec_path = app_data_dir
                    .join("agent_specs")
                    .join(format!("{}.json", spec_id));

                if spec_path.exists() {
                    let body = std::fs::read_to_string(&spec_path)
                        .map_err(|e| format!("Failed to read spec file: {}", e))?;
                    serde_json::from_str(&body)
                        .map_err(|e| format!("Invalid agent spec JSON: {}", e))?
                } else {
                    eprintln!(
                        "[AgentWorkflow] Spec {} not found in DB or files, falling back to default",
                        spec_id
                    );
                    // Fallback
                    use crate::ai::specs::skills::AgentSkills;
                    use crate::ai::specs::soul::AgentSoul;
                    AgentSpec {
                        id: "default".to_string(),
                        version: "3.0.0".to_string(),
                        soul: AgentSoul {
                            name: "Rainy Agent".to_string(),
                            description: "Default fallback agent".to_string(),
                            soul_content: default_instructions(&workspace_path),
                            ..Default::default()
                        },
                        skills: AgentSkills::default(),
                        airlock: Default::default(),
                        memory_config: Default::default(),
                        connectors: Default::default(),
                        runtime: Default::default(),
                        signature: None,
                    }
                }
            }
        };
        spec
    } else {
        use crate::ai::specs::skills::AgentSkills;
        use crate::ai::specs::soul::AgentSoul;
        AgentSpec {
            id: "default".to_string(),
            version: "3.0.0".to_string(),
            soul: AgentSoul {
                name: "Rainy Agent".to_string(),
                description: "Default agent".to_string(),
                soul_content: default_instructions(&workspace_path),
                ..Default::default()
            },
            skills: AgentSkills::default(),
            airlock: Default::default(),
            memory_config: Default::default(),
            connectors: Default::default(),
            runtime: Default::default(),
            signature: None,
        }
    };

    // Extract allowed paths from spec. If absent, derive a safe local default
    // from the provided workspace identifier when it looks like an absolute path.
    // Without at least one allowed path, filesystem tools are intentionally filtered
    // out in ThinkStep, which leads to "simulated" responses instead of real tool calls.
    let mut derived_allowed_paths = spec.airlock.scopes.allowed_paths.clone();
    if derived_allowed_paths.is_empty() {
        let ws = workspace_path.trim();
        let is_unix_abs = ws.starts_with('/');
        let is_windows_abs = ws.len() > 2 && ws.as_bytes()[1] == b':' && ws.as_bytes()[2] == b'\\';
        if is_unix_abs || is_windows_abs {
            derived_allowed_paths.push(ws.to_string());
        }
    }

    let options = RuntimeOptions {
        model: Some(selected_model_id),
        workspace_id: chat_id.clone(),
        max_steps: None,
        allowed_paths: if derived_allowed_paths.is_empty() {
            None
        } else {
            Some(derived_allowed_paths.clone())
        },
        custom_system_prompt: None,
        streaming_enabled: Some(false),
    };

    // Initialize Persistent Memory
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let mut memory_obj = crate::ai::agent::memory::AgentMemory::new(&chat_id, app_data_dir).await;
    memory_obj.set_manager(memory_manager.0.clone());
    let memory = Arc::new(memory_obj);

    let airlock_service = {
        let guard = airlock_state.0.lock().await;
        Arc::new(guard.clone())
    };

    let runtime = AgentRuntime::new(
        spec,
        options,
        router.0.clone(),
        skills.inner().clone(),
        memory,
        airlock_service,
        None,
        Some(runtime_registry.inner().clone()),
    );

    // Load persisted conversation history into runtime so local Native Runtime
    // preserves context across turns.
    let history_rows = agent_manager
        .get_history(&chat_id)
        .await
        .map_err(|e| format!("Failed to load chat history: {}", e))?;
    runtime
        .set_history(build_runtime_history(history_rows))
        .await;

    // 2. Run Workflow with Persistence
    let app_handle_clone = app_handle.clone();

    // Persist Initial User Prompt
    let _ = agent_manager
        .save_message(&chat_id, "user", &prompt)
        .await
        .map_err(|e| format!("Failed to save user message: {}", e))?;

    let response = runtime
        .run(&prompt, move |event| {
            // Emit to frontend
            let _ = app_handle_clone.emit("agent://event", event.clone());
        })
        .await?;

    // Persist final assistant response only (avoid noisy intermediate event spam).
    let _ = agent_manager
        .save_message(&chat_id, "assistant", &response)
        .await
        .map_err(|e| format!("Failed to save assistant message: {}", e))?;

    Ok(response)
}
