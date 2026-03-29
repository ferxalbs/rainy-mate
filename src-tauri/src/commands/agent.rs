use crate::ai::agent::runtime::{AgentContent, AgentMessage, AgentRuntime, RuntimeOptions};
use crate::ai::agent::runtime_registry::RuntimeRegistry;
use crate::ai::specs::manifest::{DelegationPolicy, RuntimeMode};
use crate::ai::specs::{AgentSpec, PromptSkillBinding, PromptSkillKind};
use crate::ai::{
    agent::context_window::ContextWindow,
    agent::events::AgentEvent,
    agent::manager::ChatCompactionStateDto,
    provider_trait::{AIProviderFactory, ProviderWithStats},
    provider_types::{
        ChatCompletionRequest, ChatMessage, ProviderConfig, ProviderId, ProviderType,
    },
    providers::{GeminiProviderFactory, RainySDKProviderFactory},
};
use crate::commands::agent_frontend_events::{FrontendAgentEvent, FrontendEventProjector};
use crate::commands::ai_providers::ProviderRegistryState;
use crate::commands::airlock::AirlockServiceState;
use crate::commands::memory::MemoryManagerState;
use crate::commands::router::IntelligentRouterState;
use crate::services::chat_artifacts::{
    artifact_from_tool_result, push_unique_artifact, ChatArtifact,
};
use crate::services::agent_kill_switch::AgentKillSwitch;
use crate::services::agent_run_control::{AgentRunControl, CancelRunResult};
use crate::services::{KeychainAccessService, PromptSkillDiscoveryService, SkillExecutor};
use chrono::Utc;
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;

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
        RuntimeMode::ParallelSupervisor
    }
}

fn normalize_runtime_invariants(spec: &mut AgentSpec) {
    spec.runtime.normalize_for_execution();
    if spec.runtime.mode == RuntimeMode::HierarchicalSupervisor {
        spec.runtime.delegation.policy = DelegationPolicy::ExplicitOnly;
        spec.runtime.delegation.final_synthesis_required = true;
        spec.runtime.language_policy.internal_coordination_language = "english".to_string();
        if spec.runtime.language_policy.final_response_language_mode != "english" {
            spec.runtime.language_policy.final_response_language_mode = "user".to_string();
        }
    }
    if matches!(
        spec.runtime.mode,
        RuntimeMode::ParallelSupervisor | RuntimeMode::Supervisor
    ) {
        spec.runtime.mode = RuntimeMode::ParallelSupervisor;
        spec.runtime.max_specialists = spec.runtime.max_specialists.clamp(1, 2);
        spec.runtime.delegation.max_parallel_subagents =
            spec.runtime.delegation.max_parallel_subagents.clamp(1, 2);
        spec.runtime.language_policy.internal_coordination_language = "english".to_string();
        spec.runtime.language_policy.final_response_language_mode = "english".to_string();
    }
}

enum SkillInvocationIntent {
    None,
    ListSkills,
    ForceSkill { query: String },
}

enum ResolvedSkillSelection {
    None,
    Forced(Vec<PromptSkillBinding>),
    ListResponse(String),
    NotFound {
        query: String,
        available: Vec<String>,
    },
    Ambiguous {
        query: String,
        matches: Vec<String>,
    },
}

fn merge_runtime_prompt_skills(
    existing: Vec<PromptSkillBinding>,
    discovered: Vec<crate::services::DiscoveredPromptSkill>,
) -> Vec<PromptSkillBinding> {
    let mut merged: BTreeMap<String, PromptSkillBinding> = existing
        .into_iter()
        .filter(|binding| binding.enabled)
        .map(|binding| (binding.source_path.clone(), binding))
        .collect();

    for skill in discovered
        .into_iter()
        .filter(|skill| skill.valid && skill.all_agents_enabled)
    {
        merged
            .entry(skill.source_path.clone())
            .or_insert_with(|| skill.to_binding());
    }

    merged.into_values().collect()
}

fn instruction_priority(path: &str) -> usize {
    let path = path.to_lowercase();
    if path.ends_with("/claude.md") {
        0
    } else if path.ends_with("/agents.md") {
        1
    } else if path.ends_with("/gemini.md") {
        2
    } else {
        99
    }
}

fn auto_apply_instruction(skills: &[PromptSkillBinding]) -> Option<PromptSkillBinding> {
    skills
        .iter()
        .filter(|skill| skill.kind == PromptSkillKind::WorkspaceInstruction)
        .min_by(|a, b| {
            instruction_priority(&a.source_path)
                .cmp(&instruction_priority(&b.source_path))
                .then(a.name.cmp(&b.name))
        })
        .cloned()
}

fn dedupe_bindings(bindings: Vec<PromptSkillBinding>) -> Vec<PromptSkillBinding> {
    let mut merged = BTreeMap::new();
    for binding in bindings {
        merged.insert(binding.source_path.clone(), binding);
    }
    merged.into_values().collect()
}

fn parse_skill_invocation_intent(prompt: &str) -> SkillInvocationIntent {
    let trimmed = prompt.trim();
    let lower = trimmed.to_lowercase();
    if matches!(
        lower.as_str(),
        "/skills"
            | "list my skills"
            | "list your skills"
            | "list me your skills"
            | "what skills are available here?"
    ) || lower.starts_with("what skills are available")
        || lower.starts_with("show my skills")
        || lower.starts_with("show available skills")
        || lower.starts_with("list available skills")
        || lower.starts_with("what skills do you have")
    {
        return SkillInvocationIntent::ListSkills;
    }

    if let Some(rest) = trimmed.strip_prefix("/skill ") {
        let query = rest.trim();
        if !query.is_empty() {
            return SkillInvocationIntent::ForceSkill {
                query: query.to_string(),
            };
        }
    }

    let natural_patterns = [
        r"(?i)^\s*use skill\s+(.+?)\s*$",
        r"(?i)^\s*invoke skill\s+(.+?)\s*$",
        r"(?i)^\s*invoke\s+(.+?)\s*$",
        r"(?i)^\s*apply skill\s+(.+?)\s*$",
        r"(?i)^\s*apply\s+(.+?)\s*(?:to this task)?\s*$",
    ];
    for pattern in natural_patterns {
        let re = Regex::new(pattern).expect("valid manual skill regex");
        if let Some(captures) = re.captures(trimmed) {
            if let Some(query) = captures.get(1) {
                let query = query.as_str().trim();
                if !query.is_empty() {
                    return SkillInvocationIntent::ForceSkill {
                        query: query.to_string(),
                    };
                }
            }
        }
    }

    SkillInvocationIntent::None
}

fn render_skill_registry_response(
    discovered: &[crate::services::DiscoveredPromptSkill],
    attached: &[PromptSkillBinding],
) -> String {
    if discovered.is_empty() {
        return "No prompt skills or workspace instruction files were detected for the current workspace.".to_string();
    }

    let attached_paths: std::collections::HashSet<&str> = attached
        .iter()
        .map(|skill| skill.source_path.as_str())
        .collect();
    let mut lines = vec!["Available workspace skills:".to_string()];
    for skill in discovered {
        let kind = match skill.kind {
            PromptSkillKind::WorkspaceInstruction => "workspace-instruction",
            PromptSkillKind::PromptSkill => "skill",
        };
        let scope = match skill.scope {
            crate::ai::specs::PromptSkillScope::Project => "project",
            crate::ai::specs::PromptSkillScope::Global => "global",
            crate::ai::specs::PromptSkillScope::MateManaged => "mate",
        };
        let mut status = Vec::new();
        if skill.all_agents_enabled {
            status.push("all-agents");
        }
        if attached_paths.contains(skill.source_path.as_str()) {
            status.push("attached");
        }
        if !skill.valid {
            status.push("invalid");
        }
        let status_suffix = if status.is_empty() {
            String::new()
        } else {
            format!(" [{}]", status.join(", "))
        };
        lines.push(format!(
            "- {} ({}, {}){}",
            skill.name, kind, scope, status_suffix
        ));
        lines.push(format!("  {}", skill.description));
    }
    lines.join("\n")
}

fn resolve_manual_skill_selection(
    prompt: &str,
    discovered: &[crate::services::DiscoveredPromptSkill],
    attached: &[PromptSkillBinding],
) -> ResolvedSkillSelection {
    match parse_skill_invocation_intent(prompt) {
        SkillInvocationIntent::None => ResolvedSkillSelection::None,
        SkillInvocationIntent::ListSkills => ResolvedSkillSelection::ListResponse(
            render_skill_registry_response(discovered, attached),
        ),
        SkillInvocationIntent::ForceSkill { query } => {
            let query_lc = query.trim().to_lowercase();
            let mut exact = Vec::new();
            let mut partial = Vec::new();
            for skill in discovered.iter().filter(|skill| skill.valid) {
                let name_lc = skill.name.to_lowercase();
                let id_lc = skill.id.to_lowercase();
                if name_lc == query_lc || id_lc == query_lc {
                    exact.push(skill);
                } else if name_lc.contains(&query_lc) || id_lc.contains(&query_lc) {
                    partial.push(skill);
                }
            }

            let selected = if exact.len() == 1 {
                vec![exact[0].to_binding()]
            } else if exact.len() > 1 {
                return ResolvedSkillSelection::Ambiguous {
                    query,
                    matches: exact.iter().map(|skill| skill.name.clone()).collect(),
                };
            } else if partial.len() == 1 {
                vec![partial[0].to_binding()]
            } else if partial.len() > 1 {
                return ResolvedSkillSelection::Ambiguous {
                    query,
                    matches: partial.iter().map(|skill| skill.name.clone()).collect(),
                };
            } else {
                return ResolvedSkillSelection::NotFound {
                    query,
                    available: discovered
                        .iter()
                        .filter(|skill| skill.valid)
                        .map(|skill| skill.name.clone())
                        .collect(),
                };
            };

            ResolvedSkillSelection::Forced(selected)
        }
    }
}

fn select_relevant_prompt_skills(
    prompt: &str,
    skills: Vec<PromptSkillBinding>,
) -> Vec<PromptSkillBinding> {
    let mut pinned = auto_apply_instruction(&skills)
        .into_iter()
        .collect::<Vec<_>>();
    let candidates = skills
        .into_iter()
        .filter(|skill| skill.kind != PromptSkillKind::WorkspaceInstruction)
        .collect::<Vec<_>>();
    if candidates.len() + pinned.len() <= 3 {
        pinned.extend(candidates);
        return pinned;
    }

    let prompt_lc = prompt.to_lowercase();
    let explicitly_requests_skills = !matches!(
        parse_skill_invocation_intent(prompt),
        SkillInvocationIntent::None
    );
    if explicitly_requests_skills {
        let mut out = pinned;
        out.extend(candidates);
        return out;
    }
    let prompt_tokens: Vec<&str> = prompt_lc
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
        .collect();

    let mut scored = candidates
        .into_iter()
        .map(|skill| {
            let haystack = format!(
                "{} {}",
                skill.name.to_lowercase(),
                skill.description.to_lowercase()
            );
            let mut score = 0usize;
            for token in &prompt_tokens {
                if haystack.contains(token) {
                    score += 1;
                }
            }
            if prompt_lc.contains(&skill.name.to_lowercase()) {
                score += 3;
            }
            (score, skill)
        })
        .collect::<Vec<_>>();

    let has_positive = scored.iter().any(|(score, _)| *score > 0);
    if !has_positive {
        let mut out = pinned;
        out.extend(
            scored
                .into_iter()
                .take(3usize.saturating_sub(out.len()))
                .map(|(_, skill)| skill),
        );
        return out;
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.name.cmp(&b.1.name)));
    let mut out = pinned;
    out.extend(
        scored
            .into_iter()
            .filter(|(score, _)| *score > 0)
            .take(3usize.saturating_sub(out.len()))
            .map(|(_, skill)| skill),
    );
    out
}

fn materialize_runtime_prompt_skills(
    app_handle: &tauri::AppHandle,
    workspace_path: &str,
    prompt: &str,
    spec: &mut AgentSpec,
) -> Result<ResolvedSkillSelection, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let service = PromptSkillDiscoveryService::new(app_data_dir);
    let discovered = service.discover(Some(std::path::Path::new(workspace_path)))?;
    let existing = std::mem::take(&mut spec.skills.prompt_skills);
    let manual = resolve_manual_skill_selection(prompt, &discovered, &existing);
    let merged = merge_runtime_prompt_skills(existing, discovered);
    spec.skills.prompt_skills = match &manual {
        ResolvedSkillSelection::Forced(skills) => {
            let mut selected = auto_apply_instruction(&merged)
                .into_iter()
                .collect::<Vec<_>>();
            selected.extend(skills.clone());
            dedupe_bindings(selected)
        }
        _ => select_relevant_prompt_skills(prompt, merged),
    };
    Ok(manual)
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WorkflowInvocationSource {
    Local,
    NativeModal,
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
    keychain: &KeychainAccessService,
    chat_id: &str,
    seed_prompt: &str,
    assistant_response: Option<&str>,
) -> Result<String, String> {
    ensure_provider_ready_for_model(CHAT_TITLE_MODEL_ID, provider_registry, router, keychain).await?;

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
    rows.into_iter()
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

    let summary = response.content.unwrap_or_default().trim().to_string();
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
            eprintln!(
                "[AgentWorkflow] Compaction summary model request failed: {}",
                e
            );
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
    keychain: &KeychainAccessService,
) -> Result<(), String> {
    let normalized_model = crate::ai::model_catalog::normalize_model_slug(model_id).to_string();

    let (provider_id, provider_factory_kind, key_aliases): (&str, &str, &[&str]) =
        if crate::ai::model_catalog::requires_rainy_provider(model_id) {
            ("rainy_api", "rainy", &["rainy_api", "rainyapi"])
        } else if crate::ai::model_catalog::is_explicit_gemini_model(model_id)
            || crate::ai::model_catalog::is_unprefixed_gemini_model(model_id)
        {
            ("gemini_byok", "gemini", &["gemini"])
        } else {
            return Ok(());
        };

    if registry.0.get(&ProviderId::new(provider_id)).is_err() {
        let mut api_key = None;
        for alias in key_aliases {
            let candidate = keychain.get(alias).await.map_err(|e| e.to_string())?;
            if let Some(key) =
                candidate.filter(|key| provider_factory_kind != "rainy" || is_valid_rainy_api_key(key))
            {
                api_key = Some(key);
                break;
            }
        }

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
            priority: if provider_factory_kind == "rainy" {
                10
            } else {
                20
            },
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
        let provider = registry.0.get(&ProviderId::new(provider_id)).map_err(|e| {
            format!(
                "Provider '{}' not available after registration: {}",
                provider_id, e
            )
        })?;
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
    attachments: Option<Vec<crate::services::attachment::AttachmentInput>>,
    _router: State<'_, IntelligentRouterState>,
    _airlock_state: State<'_, AirlockServiceState>,
    _provider_registry: State<'_, ProviderRegistryState>,
    _memory_manager: State<'_, MemoryManagerState>,
    _skills: State<'_, Arc<SkillExecutor>>,
    _workspace_manager: State<'_, Arc<crate::services::WorkspaceManager>>,
    _agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    _runtime_registry: State<'_, Arc<RuntimeRegistry>>,
    _run_control: State<'_, Arc<AgentRunControl>>,
    _session_coordinator: State<'_, Arc<crate::services::session_coordinator::SessionCoordinator>>,
) -> Result<RunAgentWorkflowResponse, String> {
    run_agent_workflow_internal(
        app_handle,
        prompt,
        model_id,
        workspace_id,
        agent_spec_id,
        chat_scope_id,
        run_id,
        reasoning_effort,
        attachments,
        WorkflowInvocationSource::Local,
    )
    .await
}

pub async fn run_agent_workflow_internal(
    app_handle: tauri::AppHandle,
    prompt: String,
    model_id: String,
    workspace_id: String,
    agent_spec_id: Option<String>,
    chat_scope_id: Option<String>,
    run_id: Option<String>,
    reasoning_effort: Option<String>,
    attachments: Option<Vec<crate::services::attachment::AttachmentInput>>,
    invocation_source: WorkflowInvocationSource,
) -> Result<RunAgentWorkflowResponse, String> {
    let router = app_handle.state::<IntelligentRouterState>().0.clone();
    let airlock_state = app_handle.state::<AirlockServiceState>().0.clone();
    let provider_registry = app_handle.state::<ProviderRegistryState>().0.clone();
    let memory_manager = app_handle.state::<MemoryManagerState>().0.clone();
    let skills = app_handle.state::<Arc<SkillExecutor>>().inner().clone();
    let workspace_manager = app_handle
        .state::<Arc<crate::services::WorkspaceManager>>()
        .inner()
        .clone();
    let agent_manager = app_handle
        .state::<crate::ai::agent::manager::AgentManager>()
        .inner()
        .clone();
    let runtime_registry = app_handle.state::<Arc<RuntimeRegistry>>().inner().clone();
    let run_control = app_handle.state::<Arc<AgentRunControl>>().inner().clone();
    let session_coordinator = app_handle
        .state::<Arc<crate::services::session_coordinator::SessionCoordinator>>()
        .inner()
        .clone();
    let router_state = IntelligentRouterState(router.clone());
    let provider_registry_state = ProviderRegistryState(provider_registry.clone());

    crate::ai::model_catalog::ensure_supported_model_slug(&model_id)?;
    let keychain = app_handle.state::<KeychainAccessService>();
    ensure_provider_ready_for_model(
        &model_id,
        &provider_registry_state,
        &router_state,
        keychain.inner(),
    )
    .await?;
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
    if workspace_path.contains('\0')
        || workspace_path.contains('\r')
        || workspace_path.contains('\n')
    {
        return Err("Invalid workspace path: contains forbidden characters".to_string());
    }

    let chat_id = chat_scope_id
        .unwrap_or_else(|| crate::ai::agent::manager::DEFAULT_LONG_CHAT_SCOPE_ID.to_string());

    match invocation_source {
        WorkflowInvocationSource::Local => {
            session_coordinator.register_local(
                chat_id.clone(),
                run_id.clone(),
                workspace_path.clone(),
            );
            let _ = agent_manager
                .ensure_chat_session_with_workspace(&chat_id, "Rainy Agent", &workspace_path)
                .await
                .map_err(|e| format!("Failed to initialize chat session: {}", e))?;
        }
        WorkflowInvocationSource::NativeModal => {
            let _ = session_coordinator
                .start_native_modal_session(
                    Some(chat_id.clone()),
                    Some(run_id.clone()),
                    &workspace_path,
                    &prompt,
                )
                .await?;
        }
    }

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
        spec.runtime.max_specialists = spec.runtime.max_specialists.clamp(1, 2);
        spec.runtime.verification_required = true;
        spec.runtime.delegation.max_depth = spec.runtime.delegation.max_depth.clamp(1, 2);
        spec.runtime.delegation.max_threads = spec.runtime.delegation.max_threads.clamp(2, 6);
        spec.runtime.delegation.max_parallel_subagents =
            spec.runtime.delegation.max_parallel_subagents.clamp(1, 2);
        spec.runtime.delegation.policy = DelegationPolicy::ExplicitOnly;
    }

    normalize_runtime_invariants(&mut spec);

    let skill_resolution =
        materialize_runtime_prompt_skills(&app_handle, &workspace_path, &prompt, &mut spec)?;

    if matches!(
        skill_resolution,
        ResolvedSkillSelection::ListResponse(_)
            | ResolvedSkillSelection::NotFound { .. }
            | ResolvedSkillSelection::Ambiguous { .. }
    ) {
        let response = match skill_resolution {
            ResolvedSkillSelection::ListResponse(text) => text,
            ResolvedSkillSelection::NotFound { query, available } => {
                let mut out = format!(
                    "No skill named \"{}\" was found in the current workspace.",
                    query
                );
                if !available.is_empty() {
                    out.push_str("\n\nAvailable skills:\n");
                    for name in available {
                        out.push_str(&format!("- {}\n", name));
                    }
                    out.pop();
                }
                out
            }
            ResolvedSkillSelection::Ambiguous { query, matches } => {
                let mut out = format!(
                    "The skill request \"{}\" is ambiguous. Matching skills:",
                    query
                );
                for name in matches {
                    out.push_str(&format!("\n- {}", name));
                }
                out
            }
            _ => unreachable!("filtered by matches above"),
        };

        match invocation_source {
            WorkflowInvocationSource::Local => {
                let _ = agent_manager
                    .save_message(&chat_id, "user", &prompt)
                    .await
                    .map_err(|e| format!("Failed to save user message: {}", e))?;
                let _ = agent_manager
                    .save_message(&chat_id, "assistant", &response)
                    .await
                    .map_err(|e| format!("Failed to save assistant message: {}", e))?;
                session_coordinator.unregister(&chat_id);
            }
            WorkflowInvocationSource::NativeModal => {
                session_coordinator
                    .finish_native_modal_session(&chat_id, &response, &prompt)
                    .await?;
            }
        }
        return Ok(RunAgentWorkflowResponse { run_id, response });
    }

    let settings_manager = app_handle.state::<Arc<Mutex<crate::services::SettingsManager>>>();
    let effective_policy = {
        let settings = settings_manager.lock().await;
        crate::services::LocalAgentSecurityService::resolve(
            &workspace_manager,
            &settings,
            &workspace_path,
            Some(&spec),
        )
    };

    let processed_attachments = attachments
        .map(|inputs| crate::services::attachment::process_attachments(inputs));

    let options = RuntimeOptions {
        model: Some(selected_model_id),
        workspace_id: workspace_path.clone(),
        max_steps: None,
        allowed_paths: if effective_policy.allowed_paths.is_empty() {
            None
        } else {
            Some(effective_policy.allowed_paths.clone())
        },
        custom_system_prompt: None,
        streaming_enabled: Some(false),
        reasoning_effort: crate::ai::agent::prompt_guard::validate_reasoning_effort(
            reasoning_effort.as_deref(),
        ),
        temperature: spec.temperature,
        max_tokens: spec.max_tokens,
        connector_id: None,
        user_id: None,
        attachments: processed_attachments,
    };

    // Initialize Persistent Memory
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let vault = memory_manager.get_vault().await;
    let memory_obj = crate::ai::agent::memory::AgentMemory::new(
        &workspace_path,
        app_data_dir,
        memory_manager.clone(),
        Some(router.clone()),
        vault,
    )
    .await;
    let memory = Arc::new(memory_obj);

    let airlock_service = {
        let guard = airlock_state.lock().await;
        Arc::new(guard.clone())
    };

    let run_kill_switch = AgentKillSwitch::new();
    run_control
        .register_run(run_id.clone(), run_kill_switch.clone())
        .await;

    let runtime = AgentRuntime::new(
        spec,
        options,
        router.clone(),
        skills.clone(),
        memory,
        airlock_service,
        Some(run_kill_switch),
        Some(runtime_registry.clone()),
    );

    // Load persisted conversation history into runtime so local Native Runtime
    // preserves context across turns.
    let compaction_state =
        maybe_compact_chat_history(&agent_manager, &router_state, &chat_id, &model_id, &prompt)
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
    let agent_manager_clone = agent_manager.clone();
    let chat_id_for_events = chat_id.clone();
    let run_id_for_events = run_id.clone();
    let frontend_event_projector = Arc::new(StdMutex::new(FrontendEventProjector::default()));
    let tool_call_index = Arc::new(StdMutex::new(HashMap::<String, (String, String)>::new()));
    let collected_artifacts = Arc::new(StdMutex::new(Vec::<ChatArtifact>::new()));

    // Persist Initial User Prompt
    if invocation_source == WorkflowInvocationSource::Local {
        let _ = agent_manager
            .save_message(&chat_id, "user", &prompt)
            .await
            .map_err(|e| format!("Failed to save user message: {}", e))?;
    }

    let frontend_event_projector_for_events = frontend_event_projector.clone();
    let tool_call_index_for_events = tool_call_index.clone();
    let collected_artifacts_for_events = collected_artifacts.clone();
    let response_result = runtime
        .run(&prompt, move |event| {
            let projected_events = {
                let mut projector = frontend_event_projector_for_events
                    .lock()
                    .expect("frontend event projector poisoned");
                projector.project(&event)
            };

            for projected_event in projected_events {
                match &projected_event {
                    AgentEvent::ToolCall(call) => {
                        tool_call_index_for_events
                            .lock()
                            .expect("tool call index poisoned")
                            .insert(
                                call.id.clone(),
                                (
                                    call.function.name.clone(),
                                    call.function.arguments.clone(),
                                ),
                            );
                    }
                    AgentEvent::ToolResult { id, result } => {
                        let maybe_artifact = tool_call_index_for_events
                            .lock()
                            .expect("tool call index poisoned")
                            .get(id)
                            .cloned()
                            .and_then(|(tool_name, args_json)| {
                                artifact_from_tool_result(
                                    &tool_name,
                                    Some(args_json.as_str()),
                                    result,
                                )
                            });

                        if let Some(artifact) = maybe_artifact {
                            let mut artifacts = collected_artifacts_for_events
                                .lock()
                                .expect("artifact collector poisoned");
                            push_unique_artifact(&mut artifacts, artifact);
                        }
                    }
                    _ => {}
                }

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
    let response = match response_result {
        Ok(response) => response,
        Err(error) => {
            match invocation_source {
                WorkflowInvocationSource::Local => session_coordinator.unregister(&chat_id),
                WorkflowInvocationSource::NativeModal => session_coordinator.abort_session(&chat_id),
            }
            return Err(error);
        }
    };

    match invocation_source {
        WorkflowInvocationSource::Local => {
            session_coordinator.unregister(&chat_id);
            let artifacts = collected_artifacts
                .lock()
                .expect("artifact collector poisoned")
                .clone();
            let _ = agent_manager
                .save_message_with_artifacts(
                    &chat_id,
                    "assistant",
                    &response,
                    (!artifacts.is_empty()).then_some(artifacts.as_slice()),
                )
                .await
                .map_err(|e| format!("Failed to save assistant message: {}", e))?;
        }
        WorkflowInvocationSource::NativeModal => {
            session_coordinator
                .finish_native_modal_session(&chat_id, &response, &prompt)
                .await?;
        }
    }

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
    let chat_id = chat_scope_id
        .unwrap_or_else(|| crate::ai::agent::manager::DEFAULT_LONG_CHAT_SCOPE_ID.to_string());

    agent_manager
        .ensure_chat_session(&chat_id, "Rainy Agent")
        .await
        .map_err(|e| format!("Failed to initialize chat session: {}", e))?;

    agent_manager
        .get_chat_session(&chat_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "Chat session '{}' was not found after initialization",
                chat_id
            )
        })
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
pub async fn ensure_default_local_agent(
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
) -> Result<String, String> {
    agent_manager
        .ensure_default_local_agent()
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
    let chat_id = chat_scope_id
        .unwrap_or_else(|| crate::ai::agent::manager::DEFAULT_LONG_CHAT_SCOPE_ID.to_string());

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
        .ok_or_else(|| {
            format!(
                "Chat session '{}' was not found after title update",
                chat_id
            )
        })
}

#[tauri::command]
pub async fn ensure_chat_title(
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
    router: State<'_, IntelligentRouterState>,
    provider_registry: State<'_, ProviderRegistryState>,
    keychain: State<'_, KeychainAccessService>,
    chat_scope_id: Option<String>,
    prompt: Option<String>,
    response: Option<String>,
) -> Result<EnsureChatTitleResponse, String> {
    let chat_id = chat_scope_id
        .unwrap_or_else(|| crate::ai::agent::manager::DEFAULT_LONG_CHAT_SCOPE_ID.to_string());

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
        keychain.inner(),
        &chat_id,
        &seed_prompt,
        assistant_response.as_deref(),
    )
    .await
    {
        Ok(title) => (title, "generated".to_string()),
        Err(error) => {
            eprintln!("[AgentWorkflow] Chat title generation failed: {}", error);
            (
                build_fallback_chat_title(&seed_prompt),
                "fallback".to_string(),
            )
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
        .ok_or_else(|| {
            format!(
                "Chat session '{}' was not found after title generation",
                chat_id
            )
        })?;

    Ok(EnsureChatTitleResponse { chat, status })
}

#[tauri::command]
pub async fn list_active_sessions(
    session_coordinator: State<'_, Arc<crate::services::session_coordinator::SessionCoordinator>>,
) -> Result<Vec<crate::services::session_coordinator::ActiveSessionInfo>, String> {
    Ok(session_coordinator.list_active())
}

/// Build lightweight previews for file paths selected by the user.
/// This is called immediately after the file picker dialog closes, before the user submits.
/// Returns thumbnail URIs for images and file metadata for all types.
#[tauri::command]
pub async fn prepare_attachment_previews(
    paths: Vec<String>,
) -> Result<Vec<crate::services::attachment::AttachmentPreview>, String> {
    Ok(crate::services::attachment::prepare_previews(paths))
}

#[cfg(test)]
mod tests {
    use super::{build_fallback_chat_title, is_placeholder_chat_title, sanitize_chat_title};

    #[test]
    fn placeholder_titles_are_detected() {
        assert!(is_placeholder_chat_title("New thread"));
        assert!(is_placeholder_chat_title(
            "Workspace Session: global:long_chat:v1"
        ));
        assert!(!is_placeholder_chat_title("Refactor sidebar shell"));
    }

    #[test]
    fn fallback_title_compacts_prompt() {
        let title =
            build_fallback_chat_title("   Modernize   the sidebar and topbar for chat history   ");
        assert_eq!(title, "Modernize the sidebar and topbar for chat history");
    }

    #[test]
    fn sanitized_title_uses_fallback_when_empty() {
        let title = sanitize_chat_title("\"\"", "Create the new sidebar system");
        assert_eq!(title, "Create the new sidebar system");
    }
}
