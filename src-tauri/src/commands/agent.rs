use crate::ai::agent::runtime::{AgentContent, AgentMessage, AgentRuntime, RuntimeOptions};
use crate::ai::agent::runtime_registry::RuntimeRegistry;
use crate::ai::specs::manifest::RuntimeMode;
use crate::ai::specs::AgentSpec;
use crate::ai::{
    agent::context_window::ContextWindow,
    agent::events::AgentEvent,
    agent::manager::ChatCompactionStateDto,
    keychain::KeychainManager,
    provider_trait::{AIProviderFactory, ProviderWithStats},
    provider_types::{ChatCompletionRequest, ChatMessage, ProviderConfig, ProviderId, ProviderType},
    providers::{GeminiProviderFactory, RainySDKProviderFactory},
};
use crate::commands::ai_providers::ProviderRegistryState;
use crate::commands::airlock::AirlockServiceState;
use crate::commands::agent_frontend_events::{FrontendAgentEvent, FrontendEventProjector};
use crate::commands::memory::MemoryManagerState;
use crate::commands::router::IntelligentRouterState;
use crate::services::agent_kill_switch::AgentKillSwitch;
use crate::services::agent_run_control::{AgentRunControl, CancelRunResult};
use crate::services::SkillExecutor;
use chrono::Utc;
use serde::Serialize;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{Emitter, Manager, State};

const MAX_HISTORY_MESSAGE_CHARS: usize = 12_000;
const AUTO_COMPACTION_TRIGGER_TOKENS: usize = 80_000;
const AUTO_COMPACTION_KEEP_RECENT_MESSAGES: usize = 24;
const MAX_COMPACTION_TRANSCRIPT_CHARS: usize = 160_000;
const MAX_COMPACTION_SUMMARY_CHARS: usize = 12_000;
const CHAT_TITLE_MODEL_ID: &str = "openai/gpt-5-nano";
const MAX_CHAT_TITLE_CHARS: usize = 72;

fn default_runtime_mode_for_chat(agent_spec_id: Option<&str>) -> RuntimeMode {
    if agent_spec_id.is_some() {
        RuntimeMode::Single
    } else {
        RuntimeMode::Supervisor
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAgentWorkflowResponse {
    pub run_id: String,
    pub response: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelAgentRunResponse {
    pub run_id: String,
    pub status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnsureChatTitleResponse {
    pub chat: crate::ai::agent::manager::ChatSessionDto,
    pub status: String,
}

fn is_placeholder_chat_title(value: &str) -> bool {
    let normalized = value.trim().to_lowercase();
    normalized.is_empty()
        || normalized == "new thread"
        || normalized == "new chat"
        || normalized.starts_with("workspace session:")
}

fn truncate_plain_text(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect::<String>()
}

fn build_fallback_chat_title(seed: &str) -> String {
    let compact = seed
        .split_whitespace()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let trimmed = compact
        .trim_matches(|char: char| matches!(char, '"' | '\'' | '`' | '.' | ':' | ';' | ',' | '-'))
        .trim();

    if trimmed.is_empty() {
        return "New thread".to_string();
    }

    truncate_plain_text(trimmed, MAX_CHAT_TITLE_CHARS)
}

fn sanitize_chat_title(raw: &str, fallback_seed: &str) -> String {
    let single_line = raw
        .replace('\n', " ")
        .replace('\r', " ")
        .replace('"', "")
        .replace('\'', "")
        .trim()
        .trim_matches(|char: char| matches!(char, '-' | ':' | '.' | ',' | '`'))
        .to_string();

    if single_line.is_empty() {
        return build_fallback_chat_title(fallback_seed);
    }

    truncate_plain_text(&single_line, MAX_CHAT_TITLE_CHARS)
}

async fn generate_chat_title(
    router: &IntelligentRouterState,
    provider_registry: &ProviderRegistryState,
    chat_id: &str,
    seed_prompt: &str,
    assistant_response: Option<&str>,
) -> Result<String, String> {
    ensure_provider_ready_for_model(CHAT_TITLE_MODEL_ID, provider_registry, router).await?;

    let response_excerpt = assistant_response
        .map(|value| truncate_text(value, 600))
        .unwrap_or_else(|| "No assistant response yet.".to_string());

    let request = ChatCompletionRequest {
        messages: vec![
            ChatMessage::system(
                "Create a concise chat title for a desktop AI workspace conversation.
Return only the title, no quotes, no markdown, no prefix.
Use 2 to 6 words, preserve the user's language, and keep it specific."
                    .to_string(),
            ),
            ChatMessage::user(format!(
                "Chat ID: {}\nUser request:\n{}\n\nAssistant response:\n{}",
                chat_id,
                truncate_text(seed_prompt, 600),
                response_excerpt
            )),
        ],
        model: CHAT_TITLE_MODEL_ID.to_string(),
        temperature: Some(0.2),
        max_tokens: Some(24),
        top_p: Some(1.0),
        frequency_penalty: Some(0.0),
        presence_penalty: Some(0.0),
        stop: None,
        stream: false,
        tools: None,
        tool_choice: None,
        json_mode: false,
        reasoning_effort: None,
    };

    let response = router
        .0
        .read()
        .await
        .complete(request)
        .await
        .map_err(|e| format!("Chat title request failed: {}", e))?;

    let title = response.content.unwrap_or_default();
    Ok(sanitize_chat_title(&title, seed_prompt))
}

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
    rows
        .into_iter()
        .filter_map(|(_, role, content)| {
            if role != "user" && role != "assistant" && role != "system" {
                return None;
            }
            Some(AgentMessage {
                role,
                content: AgentContent::text(truncate_text(&content, MAX_HISTORY_MESSAGE_CHARS)),
                tool_calls: None,
                tool_call_id: None,
            })
        })
        .collect()
}

fn estimate_history_tokens(rows: &[(String, String, String)], prompt: &str) -> usize {
    let mut messages = build_runtime_history(rows.to_vec());
    messages.push(AgentMessage {
        role: "user".to_string(),
        content: AgentContent::text(prompt),
        tool_calls: None,
        tool_call_id: None,
    });
    ContextWindow::estimate_total_tokens(&messages)
}

fn build_compaction_transcript(
    rows: &[(String, String, String)],
    keep_recent_count: usize,
) -> Option<(String, usize)> {
    if rows.len() <= keep_recent_count + 2 {
        return None;
    }
    let split_index = rows.len().saturating_sub(keep_recent_count);
    if split_index == 0 {
        return None;
    }

    let to_summarize = &rows[..split_index];
    let mut transcript = String::new();

    for (_, role, content) in to_summarize {
        if role == "system" && content.starts_with("SESSION COMPACTION SUMMARY:") {
            continue;
        }
        let role_label = match role.as_str() {
            "user" => "USER",
            "assistant" => "ASSISTANT",
            _ => "SYSTEM",
        };
        let line = format!("{}: {}\n", role_label, truncate_text(content, 1200));
        if transcript.len() + line.len() > MAX_COMPACTION_TRANSCRIPT_CHARS {
            break;
        }
        transcript.push_str(&line);
    }

    if transcript.trim().is_empty() {
        None
    } else {
        Some((transcript, to_summarize.len()))
    }
}

fn fallback_compaction_summary(rows_to_summarize: usize, prompt: &str) -> String {
    format!(
        "Auto-compaction fallback summary.\n\
         - Summarized prior turns: {}\n\
         - Current user objective: {}\n\
         - Preserve unresolved tasks, constraints, and file-specific context from recent turns.",
        rows_to_summarize,
        truncate_text(prompt, 800)
    )
}

async fn generate_compaction_summary(
    router: &IntelligentRouterState,
    model_id: &str,
    transcript: &str,
    prompt: &str,
) -> Result<String, String> {
    let instruction = "You are a context compaction engine for long-running agent chats.
Return a concise, structured rolling summary to preserve continuity after compression.
Include only durable context needed for future turns.

Output sections exactly:
1) User Intent
2) Completed Work
3) Decisions and Constraints
4) Active Work
5) Pending Tasks
6) Critical Artifacts (paths, ids, commands, settings)";

    let payload = format!(
        "Current user message:\n{}\n\nConversation excerpt to compact:\n{}",
        truncate_text(prompt, 1200),
        transcript
    );

    let request = ChatCompletionRequest {
        messages: vec![
            ChatMessage::system(instruction.to_string()),
            ChatMessage::user(payload),
        ],
        model: model_id.to_string(),
        temperature: Some(0.1),
        max_tokens: Some(1800),
        top_p: Some(1.0),
        frequency_penalty: Some(0.0),
        presence_penalty: Some(0.0),
        stop: None,
        stream: false,
        tools: None,
        tool_choice: None,
        json_mode: false,
        reasoning_effort: None,
    };

    let response = router
        .0
        .read()
        .await
        .complete(request)
        .await
        .map_err(|e| format!("Compaction summary request failed: {}", e))?;

    let summary = response
        .content
        .unwrap_or_default()
        .trim()
        .to_string();
    if summary.is_empty() {
        return Err("Compaction summary returned empty content".to_string());
    }
    Ok(truncate_text(&summary, MAX_COMPACTION_SUMMARY_CHARS))
}

async fn maybe_compact_chat_history(
    agent_manager: &crate::ai::agent::manager::AgentManager,
    router: &IntelligentRouterState,
    chat_id: &str,
    model_id: &str,
    prompt: &str,
) -> Result<Option<ChatCompactionStateDto>, String> {
    let history_rows = agent_manager
        .get_history(chat_id)
        .await
        .map_err(|e| format!("Failed to load chat history for compaction: {}", e))?;

    let estimated_tokens = estimate_history_tokens(&history_rows, prompt);
    if estimated_tokens < AUTO_COMPACTION_TRIGGER_TOKENS {
        return Ok(None);
    }

    let Some((transcript, rows_to_summarize)) =
        build_compaction_transcript(&history_rows, AUTO_COMPACTION_KEEP_RECENT_MESSAGES)
    else {
        return Ok(None);
    };

    let summary = match generate_compaction_summary(router, model_id, &transcript, prompt).await {
        Ok(text) => text,
        Err(e) => {
            eprintln!("[AgentWorkflow] Compaction summary model request failed: {}", e);
            fallback_compaction_summary(rows_to_summarize, prompt)
        }
    };

    agent_manager
        .compact_session_with_rolling_summary(
            chat_id,
            &summary,
            AUTO_COMPACTION_KEEP_RECENT_MESSAGES,
            estimated_tokens,
            model_id,
        )
        .await
        .map_err(|e| format!("Failed to compact session: {}", e))
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
    run_id: Option<String>,
    reasoning_effort: Option<String>,
    router: State<'_, IntelligentRouterState>,
    airlock_state: State<'_, AirlockServiceState>,
    provider_registry: State<'_, ProviderRegistryState>,
    memory_manager: State<'_, MemoryManagerState>,
    skills: State<'_, Arc<SkillExecutor>>,
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    runtime_registry: State<'_, Arc<RuntimeRegistry>>,
    run_control: State<'_, Arc<AgentRunControl>>,
) -> Result<RunAgentWorkflowResponse, String> {
    crate::ai::model_catalog::ensure_supported_model_slug(&model_id)?;
    ensure_provider_ready_for_model(&model_id, &provider_registry, &router).await?;
    let selected_model_id = model_id.clone();
    let run_id = run_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // --- Prompt injection guard: sanitize user input before any processing ---
    let guard_result = crate::ai::agent::prompt_guard::sanitize_user_input(&prompt);
    if guard_result.was_modified {
        eprintln!(
            "[PromptGuard] User input sanitized. Flags: {:?}",
            guard_result.flags
        );
    }
    let prompt = guard_result.text;

    let workspace_path = workspace_id.clone();

    // --- Sanitize workspace path before it is interpolated into the system prompt ---
    // Keep the sanitized version separate for prompt interpolation only
    let ws_guard = crate::ai::agent::prompt_guard::sanitize_workspace_id(&workspace_path);
    if ws_guard.was_modified {
        eprintln!(
            "[PromptGuard] workspace_id sanitized. Flags: {:?}",
            ws_guard.flags
        );
    }
    let prompt_safe_workspace = ws_guard.text;

    // Validate the original workspace_path for filesystem operations
    // Reject if it contains invalid UTF-8 or dangerous characters before using for paths
    if workspace_path.contains('\0') || workspace_path.contains('\r') || workspace_path.contains('\n') {
        return Err("Invalid workspace path: contains forbidden characters".to_string());
    }

    let chat_id = chat_scope_id.unwrap_or_else(|| {
        crate::ai::agent::manager::DEFAULT_LONG_CHAT_SCOPE_ID.to_string()
    });

    // 0. Ensure Chat Session Exists (Persist Metadata)
    let _ = agent_manager
        .ensure_chat_session_with_workspace(&chat_id, "Rainy Agent", &workspace_path)
        .await
        .map_err(|e| format!("Failed to initialize chat session: {}", e))?;

    // 1. Initialize Runtime (Ephemeral for now, persistent later)
    // 1. Initialize Runtime (Ephemeral for now, persistent later)
    let selected_spec_id = agent_spec_id.clone();
    let mut spec = if let Some(spec_id) = selected_spec_id.clone() {
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
                            soul_content: default_instructions(&prompt_safe_workspace),
                            ..Default::default()
                        },
                        skills: AgentSkills::default(),
                        airlock: Default::default(),
                        memory_config: Default::default(),
                        connectors: Default::default(),
                        runtime: Default::default(),
                        model: None,
                        temperature: None,
                        max_tokens: None,
                        provider: None,
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
                soul_content: default_instructions(&prompt_safe_workspace),
                ..Default::default()
            },
            skills: AgentSkills::default(),
            airlock: Default::default(),
            memory_config: Default::default(),
            connectors: Default::default(),
            runtime: crate::ai::specs::manifest::RuntimeConfig {
                mode: default_runtime_mode_for_chat(selected_spec_id.as_deref()),
                ..Default::default()
            },
            model: None,
            temperature: None,
            max_tokens: None,
            provider: None,
            signature: None,
        }
    };

    if selected_spec_id.is_none() {
        spec.runtime.mode = default_runtime_mode_for_chat(None);
        spec.runtime.max_specialists = spec.runtime.max_specialists.clamp(2, 3);
        spec.runtime.verification_required = true;
    }

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
        workspace_id: workspace_path.clone(),
        max_steps: None,
        allowed_paths: if derived_allowed_paths.is_empty() {
            None
        } else {
            Some(derived_allowed_paths.clone())
        },
        custom_system_prompt: None,
        streaming_enabled: Some(false),
        reasoning_effort: crate::ai::agent::prompt_guard::validate_reasoning_effort(
            reasoning_effort.as_deref()
        ),
        temperature: spec.temperature,
        max_tokens: spec.max_tokens,
        connector_id: None,
        user_id: None,
    };

    // Initialize Persistent Memory
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let vault = memory_manager.0.get_vault().await;
    let memory_obj =
        crate::ai::agent::memory::AgentMemory::new(
            &workspace_path,
            app_data_dir,
            memory_manager.0.clone(),
            Some(router.0.clone()),
            vault,
        )
            .await;
    let memory = Arc::new(memory_obj);

    let airlock_service = {
        let guard = airlock_state.0.lock().await;
        Arc::new(guard.clone())
    };

    let run_kill_switch = AgentKillSwitch::new();
    run_control
        .register_run(run_id.clone(), run_kill_switch.clone())
        .await;

    let runtime = AgentRuntime::new(
        spec,
        options,
        router.0.clone(),
        skills.inner().clone(),
        memory,
        airlock_service,
        Some(run_kill_switch),
        Some(runtime_registry.inner().clone()),
    );

    // Load persisted conversation history into runtime so local Native Runtime
    // preserves context across turns.
    let compaction_state = maybe_compact_chat_history(
        &agent_manager,
        &router,
        &chat_id,
        &model_id,
        &prompt,
    )
    .await?;

    if let Some(compaction) = compaction_state {
        let _ = app_handle.emit(
            "agent://event",
            FrontendAgentEvent {
                run_id: run_id.clone(),
                timestamp_ms: Utc::now().timestamp_millis(),
                payload: AgentEvent::Status(format!(
                    "CONTEXT_COMPACTION:{}",
                    serde_json::json!({
                        "applied": true,
                        "trigger_tokens": AUTO_COMPACTION_TRIGGER_TOKENS,
                        "source_estimated_tokens": compaction.source_estimated_tokens,
                        "source_message_count": compaction.source_message_count,
                        "kept_recent_count": compaction.kept_recent_count,
                        "compression_model": compaction.compression_model,
                        "best_practice": "rolling_summary_context_compaction",
                    })
                    .to_string()
                )),
            },
        );
    }

    let history_rows = agent_manager
        .get_history(&chat_id)
        .await
        .map_err(|e| format!("Failed to load chat history: {}", e))?;
    runtime
        .set_history(build_runtime_history(history_rows))
        .await;

    // 2. Run Workflow with Persistence
    let _ = agent_manager
        .upsert_chat_runtime_telemetry(
            &chat_id,
            "persisted_long_chat",
            "unavailable",
            crate::services::memory_vault::types::EMBEDDING_MODEL,
        )
        .await;

    let app_handle_clone = app_handle.clone();
    let agent_manager_clone = agent_manager.inner().clone();
    let chat_id_for_events = chat_id.clone();
    let run_id_for_events = run_id.clone();
    let frontend_event_projector = Arc::new(StdMutex::new(FrontendEventProjector::default()));

    // Persist Initial User Prompt
    let _ = agent_manager
        .save_message(&chat_id, "user", &prompt)
        .await
        .map_err(|e| format!("Failed to save user message: {}", e))?;

    let frontend_event_projector_for_events = frontend_event_projector.clone();
    let response_result = runtime
        .run(&prompt, move |event| {
            let projected_events = {
                let mut projector = frontend_event_projector_for_events
                    .lock()
                    .expect("frontend event projector poisoned");
                projector.project(&event)
            };

            for projected_event in projected_events {
                let _ = app_handle_clone.emit(
                    "agent://event",
                    FrontendAgentEvent {
                        run_id: run_id_for_events.clone(),
                        timestamp_ms: Utc::now().timestamp_millis(),
                        payload: projected_event.clone(),
                    },
                );

                if let AgentEvent::Status(text) = projected_event {
                    if text.starts_with("RAG_TELEMETRY:") {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(
                            &text["RAG_TELEMETRY:".len()..],
                        ) {
                            let history_source = value
                                .get("history_source")
                                .and_then(|v| v.as_str())
                                .unwrap_or("persisted_long_chat")
                                .to_string();
                            let retrieval_mode = value
                                .get("retrieval_mode")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unavailable")
                                .to_string();
                            let embedding_profile = value
                                .get("embedding_profile")
                                .and_then(|v| v.as_str())
                                .unwrap_or(crate::services::memory_vault::types::EMBEDDING_MODEL)
                                .to_string();
                            let manager = agent_manager_clone.clone();
                            let chat_id = chat_id_for_events.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = manager
                                    .upsert_chat_runtime_telemetry(
                                        &chat_id,
                                        &history_source,
                                        &retrieval_mode,
                                        &embedding_profile,
                                    )
                                    .await;
                            });
                        }
                    }
                }
            }
        })
        .await;

    let pending_projected_events = {
        let mut projector = frontend_event_projector
            .lock()
            .expect("frontend event projector poisoned");
        projector.flush_pending()
    };
    for projected_event in pending_projected_events {
        let _ = app_handle.emit(
            "agent://event",
            FrontendAgentEvent {
                run_id: run_id.clone(),
                timestamp_ms: Utc::now().timestamp_millis(),
                payload: projected_event,
            },
        );
    }

    run_control.unregister_run(&run_id).await;
    let response = response_result?;

    // Persist final assistant response only (avoid noisy intermediate event spam).
    let _ = agent_manager
        .save_message(&chat_id, "assistant", &response)
        .await
        .map_err(|e| format!("Failed to save assistant message: {}", e))?;

    Ok(RunAgentWorkflowResponse { run_id, response })
}

#[tauri::command]
pub async fn cancel_agent_run(
    run_id: String,
    run_control: State<'_, Arc<AgentRunControl>>,
) -> Result<CancelAgentRunResponse, String> {
    let status = match run_control.cancel_run(&run_id).await {
        CancelRunResult::Cancelled => "cancelled",
        CancelRunResult::UnknownRun => "unknown_run",
    }
    .to_string();

    Ok(CancelAgentRunResponse { run_id, status })
}

#[tauri::command]
pub async fn get_chat_session(
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    chat_scope_id: Option<String>,
) -> Result<crate::ai::agent::manager::ChatSessionDto, String> {
    let chat_id =
        chat_scope_id.unwrap_or_else(|| crate::ai::agent::manager::DEFAULT_LONG_CHAT_SCOPE_ID.to_string());

    agent_manager
        .ensure_chat_session(&chat_id, "Rainy Agent")
        .await
        .map_err(|e| format!("Failed to initialize chat session: {}", e))?;

    agent_manager
        .get_chat_session(&chat_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Chat session '{}' was not found after initialization", chat_id))
}

#[tauri::command]
pub async fn list_chat_sessions(
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    workspace_id: String,
) -> Result<Vec<crate::ai::agent::manager::ChatSessionDto>, String> {
    let ws = if workspace_id.trim().is_empty() {
        "default".to_string()
    } else {
        workspace_id
    };
    agent_manager
        .list_chat_sessions(&ws)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_chat_session(
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    workspace_id: String,
) -> Result<crate::ai::agent::manager::ChatSessionDto, String> {
    let ws = if workspace_id.trim().is_empty() {
        "default".to_string()
    } else {
        workspace_id
    };
    agent_manager
        .create_chat_session(&ws)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_or_reuse_empty_chat_session(
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    workspace_id: String,
) -> Result<crate::ai::agent::manager::ChatSessionDto, String> {
    let ws = if workspace_id.trim().is_empty() {
        "default".to_string()
    } else {
        workspace_id
    };
    agent_manager
        .create_or_reuse_empty_chat_session(&ws)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_chat_session(
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    chat_id: String,
) -> Result<(), String> {
    agent_manager
        .delete_chat_session(&chat_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_chat_title(
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    chat_scope_id: Option<String>,
    title: Option<String>,
) -> Result<crate::ai::agent::manager::ChatSessionDto, String> {
    let chat_id =
        chat_scope_id.unwrap_or_else(|| crate::ai::agent::manager::DEFAULT_LONG_CHAT_SCOPE_ID.to_string());

    agent_manager
        .ensure_chat_session(&chat_id, "Rainy Agent")
        .await
        .map_err(|e| format!("Failed to initialize chat session: {}", e))?;

    let normalized_title = title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| truncate_plain_text(value, MAX_CHAT_TITLE_CHARS));

    agent_manager
        .update_chat_title(&chat_id, normalized_title.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    agent_manager
        .get_chat_session(&chat_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Chat session '{}' was not found after title update", chat_id))
}

#[tauri::command]
pub async fn ensure_chat_title(
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    router: State<'_, IntelligentRouterState>,
    provider_registry: State<'_, ProviderRegistryState>,
    chat_scope_id: Option<String>,
    prompt: Option<String>,
    response: Option<String>,
) -> Result<EnsureChatTitleResponse, String> {
    let chat_id =
        chat_scope_id.unwrap_or_else(|| crate::ai::agent::manager::DEFAULT_LONG_CHAT_SCOPE_ID.to_string());

    agent_manager
        .ensure_chat_session(&chat_id, "Rainy Agent")
        .await
        .map_err(|e| format!("Failed to initialize chat session: {}", e))?;

    if let Some(existing) = agent_manager
        .get_chat_session(&chat_id)
        .await
        .map_err(|e| e.to_string())?
    {
        if let Some(title) = existing.title.as_deref() {
            if !is_placeholder_chat_title(title) {
                return Ok(EnsureChatTitleResponse {
                    chat: existing,
                    status: "ready".to_string(),
                });
            }
        }
    }

    let history_rows = agent_manager
        .get_history(&chat_id)
        .await
        .map_err(|e| format!("Failed to load chat history for title generation: {}", e))?;

    let seed_prompt = prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            history_rows
                .iter()
                .find(|(_, role, content)| role == "user" && !content.trim().is_empty())
                .map(|(_, _, content)| content.clone())
        })
        .unwrap_or_else(|| "New thread".to_string());

    let assistant_response = response.or_else(|| {
        history_rows
            .iter()
            .rev()
            .find(|(_, role, content)| role == "assistant" && !content.trim().is_empty())
            .map(|(_, _, content)| content.clone())
    });

    let (next_title, status) = match generate_chat_title(
        &router,
        &provider_registry,
        &chat_id,
        &seed_prompt,
        assistant_response.as_deref(),
    )
    .await
    {
        Ok(title) => (title, "generated".to_string()),
        Err(error) => {
            eprintln!("[AgentWorkflow] Chat title generation failed: {}", error);
            (build_fallback_chat_title(&seed_prompt), "fallback".to_string())
        }
    };

    agent_manager
        .update_chat_title(&chat_id, Some(&next_title))
        .await
        .map_err(|e| format!("Failed to persist chat title: {}", e))?;

    let chat = agent_manager
        .get_chat_session(&chat_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Chat session '{}' was not found after title generation", chat_id))?;

    Ok(EnsureChatTitleResponse { chat, status })
}

#[cfg(test)]
mod tests {
    use super::{build_fallback_chat_title, is_placeholder_chat_title, sanitize_chat_title};

    #[test]
    fn placeholder_titles_are_detected() {
        assert!(is_placeholder_chat_title("New thread"));
        assert!(is_placeholder_chat_title("Workspace Session: global:long_chat:v1"));
        assert!(!is_placeholder_chat_title("Refactor sidebar shell"));
    }

    #[test]
    fn fallback_title_compacts_prompt() {
        let title = build_fallback_chat_title("   Modernize   the sidebar and topbar for chat history   ");
        assert_eq!(title, "Modernize the sidebar and topbar for chat history");
    }

    #[test]
    fn sanitized_title_uses_fallback_when_empty() {
        let title = sanitize_chat_title("\"\"", "Create the new sidebar system");
        assert_eq!(title, "Create the new sidebar system");
    }
}
