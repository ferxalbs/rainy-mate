// AgentRuntime v2 — Core runtime orchestrating the agent's ReAct workflow.
// Manages state, history, memory persistence, and the Think→Act execution loop.
use crate::ai::agent::context_window::ContextWindow;
use crate::ai::agent::events::AgentEvent;
use crate::ai::agent::memory::AgentMemory;
use crate::ai::agent::runtime_registry::RuntimeRegistry;
use crate::ai::agent::supervisor::SupervisorAgent;
use crate::ai::agent::workflow::{ActStep, AgentState, ThinkStep, Workflow};
use crate::ai::router::IntelligentRouter;
use crate::ai::specs::manifest::{AgentSpec, RuntimeMode};
use crate::services::agent_kill_switch::AgentKillSwitch;
use crate::services::SkillExecutor;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RuntimeOptions {
    pub model: Option<String>,
    pub workspace_id: String,
    pub max_steps: Option<usize>,
    pub allowed_paths: Option<Vec<String>>,
    pub custom_system_prompt: Option<String>,
    pub streaming_enabled: Option<bool>,
    pub reasoning_effort: Option<String>,
}

/// The core runtime that orchestrates the agent's thinking process
pub struct AgentRuntime {
    pub spec: AgentSpec,
    pub options: RuntimeOptions,
    router: Arc<tokio::sync::RwLock<IntelligentRouter>>,
    skills: Arc<SkillExecutor>,
    memory: Arc<AgentMemory>,
    airlock_service: Arc<Option<crate::services::airlock::AirlockService>>,
    kill_switch: Option<AgentKillSwitch>,
    runtime_registry: Option<Arc<RuntimeRegistry>>,
    history: Arc<Mutex<Vec<AgentMessage>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentMessage {
    pub role: String,
    pub content: AgentContent,
    pub tool_calls: Option<Vec<crate::ai::provider_types::ToolCall>>,
    pub tool_call_id: Option<String>,
}

/// Content for agent messages - supports text and multimodal
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum AgentContent {
    /// Simple text content
    Text(String),
    /// Multimodal content with text and/or images
    Parts(Vec<AgentContentPart>),
}

impl AgentContent {
    /// Create text content
    pub fn text(s: impl Into<String>) -> Self {
        AgentContent::Text(s.into())
    }

    /// Create image content from a data URI
    pub fn image(data_uri: impl Into<String>) -> Self {
        AgentContent::Parts(vec![AgentContentPart::ImageUrl {
            image_url: AgentImageUrl {
                url: data_uri.into(),
                detail: Some("auto".to_string()),
            },
        }])
    }

    /// Create mixed content (text + image)
    #[allow(dead_code)] // @RESERVED - will be used for user-provided images
    pub fn mixed(text: impl Into<String>, image_url: impl Into<String>) -> Self {
        AgentContent::Parts(vec![
            AgentContentPart::Text { text: text.into() },
            AgentContentPart::ImageUrl {
                image_url: AgentImageUrl {
                    url: image_url.into(),
                    detail: Some("auto".to_string()),
                },
            },
        ])
    }

    /// Get text representation (for non-multimodal contexts)
    pub fn as_text(&self) -> String {
        match self {
            AgentContent::Text(s) => s.clone(),
            AgentContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    AgentContentPart::Text { text } => Some(text.clone()),
                    AgentContentPart::ImageUrl { .. } => Some("[IMAGE]".to_string()),
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    /// Check if content contains an image
    #[allow(dead_code)] // @RESERVED - will be used for conditional image processing
    pub fn has_image(&self) -> bool {
        match self {
            AgentContent::Text(_) => false,
            AgentContent::Parts(parts) => parts
                .iter()
                .any(|p| matches!(p, AgentContentPart::ImageUrl { .. })),
        }
    }
}

impl From<String> for AgentContent {
    fn from(s: String) -> Self {
        AgentContent::Text(s)
    }
}

impl From<&str> for AgentContent {
    fn from(s: &str) -> Self {
        AgentContent::Text(s.to_string())
    }
}

/// Content part for multimodal messages
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentContentPart {
    /// Text content
    Text { text: String },
    /// Image URL (including data URIs)
    ImageUrl { image_url: AgentImageUrl },
}

/// Image URL details
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AgentRuntime {
    fn runtime_truthfulness_appendix() -> &'static str {
        "\n\nRuntime Safety Rules (Non-negotiable):\n\
- Tool outputs are the source of truth.\n\
- Never claim a tool or command succeeded unless the tool result explicitly succeeded.\n\
- If a tool fails, times out, or is blocked by policy, explicitly report the failure and limitation.\n\
- Do not fabricate command output, file hashes, file contents, diffs, or scan findings.\n\
- If no permitted tool can complete a step, ask the user for input or propose a permitted alternative.\n"
    }

    pub fn new(
        spec: AgentSpec,
        options: RuntimeOptions,
        router: Arc<tokio::sync::RwLock<IntelligentRouter>>,
        skills: Arc<SkillExecutor>,
        memory: Arc<AgentMemory>,
        airlock_service: Arc<Option<crate::services::airlock::AirlockService>>,
        kill_switch: Option<AgentKillSwitch>,
        runtime_registry: Option<Arc<RuntimeRegistry>>,
    ) -> Self {
        Self {
            spec,
            options,
            router,
            skills,
            memory,
            airlock_service,
            kill_switch,
            runtime_registry,
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Replace in-memory history for this runtime instance.
    pub async fn set_history(&self, messages: Vec<AgentMessage>) {
        let mut hist = self.history.lock().await;
        *hist = messages;
    }

    async fn generate_system_prompt(&self, skills: &SkillExecutor) -> String {
        // If a custom system prompt is provided (e.g. from Cloud/ATM), use it directly.
        if let Some(custom) = &self.options.custom_system_prompt {
            return format!(
                "{}\n\n--- END OPERATOR INSTRUCTIONS ---\n{}",
                custom,
                Self::runtime_truthfulness_appendix()
            );
        }

        let spec = &self.spec;
        let workspace_id = &self.options.workspace_id;
        let allowed_paths = self.options.allowed_paths.clone().unwrap_or_default();

        let workspace_scope = if allowed_paths.is_empty() {
            // Derive scope from workspace_id when allowed_paths is not explicitly set.
            // This matches the derivation logic in run_agent_workflow (agent.rs).
            let ws = workspace_id.trim();
            let is_abs = ws.starts_with('/') || (ws.len() > 2 && ws.as_bytes()[1] == b':');
            if is_abs {
                format!("{} (derived from workspace)", ws)
            } else {
                "(workspace root — use tools to explore)".to_string()
            }
        } else {
            allowed_paths.join(", ")
        };

        // Build the available tool list dynamically, applying the same two filters ThinkStep uses:
        // 1. Filesystem guard — drop filesystem tools when no allowed_paths are configured.
        // 2. Airlock policy — respect this agent's tool_policy (allowlist / deny entries).
        // This guarantees the LLM sees EXACTLY the tools it will get — no over-promising, no under-promising.
        let mut available_tools = skills.get_tool_definitions().await;
        if allowed_paths.is_empty() {
            available_tools.retain(|t| {
                !crate::ai::agent::workflow::FILESYSTEM_TOOL_NAMES
                    .contains(&t.function.name.as_str())
            });
        }
        available_tools.retain(|t| spec.airlock.is_tool_allowed(&t.function.name));

        let capability_lines = if spec.skills.capabilities.is_empty() {
            // No explicit capability spec — describe what the agent actually has access to
            // based on the Airlock filtering above (single source of truth with ThinkStep).
            if available_tools.is_empty() {
                "- No tools available for this agent (filtered by Airlock policy or filesystem scope restrictions)".to_string()
            } else {
                let tool_names: Vec<String> = available_tools
                    .iter()
                    .map(|t| format!("`{}`", t.function.name))
                    .collect();
                format!(
                    "- Available tools (use them for all operations — never simulate results):\n  {}",
                    tool_names.join(", ")
                )
            }
        } else {
            spec.skills
                .capabilities
                .iter()
                .map(|cap| {
                    let scopes = cap.scopes.join(", ");
                    let permissions = cap
                        .permissions
                        .iter()
                        .map(|p| format!("{:?}", p))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "- {}: {} | scopes: {} | permissions: {}",
                        cap.name, cap.description, scopes, permissions
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "You are {}.

Identity:
- Description: {}
- Personality: {}
- Tone: {}

Core soul:
{}

Workspace ID: {}
Allowed Filesystem Scope: {}

Capabilities:
{}

Memory:
- strategy: {}
- retention_days: {}
- max_tokens: {}

Rules:
1. Use tools and skills only within declared capabilities and workspace scope.
2. Never fabricate file results.
3. If a tool fails, explain and try the safest fallback.
4. Never use workspace ID as a filesystem path. Only use explicit allowed filesystem scope paths.{}",
            spec.soul.name,
            spec.soul.description,
            spec.soul.personality,
            spec.soul.tone,
            spec.soul.soul_content,
            workspace_id,
            workspace_scope,
            capability_lines,
            spec.memory_config.strategy,
            spec.memory_config.effective_retention_days(),
            spec.memory_config.effective_max_tokens(),
            Self::runtime_truthfulness_appendix()
        )
    }

    fn build_semantic_context_block(
        &self,
        context_window: &ContextWindow,
        result: &crate::services::memory::SemanticSearchResult,
    ) -> String {
        if result.entries.is_empty() {
            return String::new();
        }

        let budget_tokens = context_window.semantic_context_budget_tokens();
        let mut remaining_chars = budget_tokens.saturating_mul(4);
        let mut block = String::new();
        let header = format!(
            "\n\n--- RELEVANT CONTEXT FROM WORKSPACE MEMORY ({:?}) ---\n",
            result.mode
        );
        block.push_str(&header);
        remaining_chars = remaining_chars.saturating_sub(header.len());

        for (i, entry) in result.entries.iter().enumerate() {
            if remaining_chars < 32 {
                break;
            }

            let prefix = format!("[{}] ", i + 1);
            let content_budget_tokens = (remaining_chars / 4).saturating_sub(8);
            let content =
                context_window.truncate_text_for_tokens(&entry.content, content_budget_tokens);
            let line = format!("{}{}\n", prefix, content);
            if line.len() > remaining_chars {
                break;
            }
            block.push_str(&line);
            remaining_chars = remaining_chars.saturating_sub(line.len());
        }

        if let Some(reason) = &result.reason {
            let footer = format!("fallback_reason: {}\n", reason);
            if footer.len() <= remaining_chars {
                block.push_str(&footer);
                remaining_chars = remaining_chars.saturating_sub(footer.len());
            }
        }

        let end = "----------------------------------------------\n";
        if end.len() <= remaining_chars {
            block.push_str(end);
        }
        block
    }

    /// Primary entry point: Run a workflow/turn
    pub async fn run<F>(&self, input: &str, on_event: F) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        if self.spec.runtime.mode == RuntimeMode::Supervisor {
            let supervisor = SupervisorAgent {
                spec: self.spec.clone(),
                options: self.options.clone(),
                router: self.router.clone(),
                skills: self.skills.clone(),
                memory: self.memory.clone(),
                airlock_service: self.airlock_service.clone(),
                kill_switch: self.kill_switch.clone(),
                runtime_registry: self.runtime_registry.clone(),
            };
            return supervisor.run(input, on_event).await;
        }
        self.run_single(input, on_event).await
    }

    pub async fn run_single<F>(&self, input: &str, on_event: F) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        // 1. Initialize State
        let mut state = AgentState::new(
            self.options.workspace_id.clone(),
            self.options.allowed_paths.clone().unwrap_or_default(),
            self.memory.clone(),
            Arc::new(self.spec.clone()),
            self.airlock_service.clone(),
            self.kill_switch.clone(),
        );

        // Add System Message to State
        state.messages.push(AgentMessage {
            role: "system".to_string(),
            content: AgentContent::text(self.generate_system_prompt(&self.skills).await),
            tool_calls: None,
            tool_call_id: None,
        });

        let context_window =
            ContextWindow::new(self.spec.memory_config.effective_max_tokens() as usize);

        // --- SEMANTIC RETRIEVAL (Hive Mind Seed) ---
        // Retrieve relevant context from the encrypted memory vault using the user input
        // Since we don't have direct access to memory_manager here, we use AgentMemory wrapped methods.
        // We'll add the semantic results as an invisible "system" state message or inject into the system prompt.
        let mut appended_context = String::new();
        let mut retrieval_mode = "unavailable".to_string();
        let mm = self.memory.manager();
        if let Ok(mut result) = mm
            .search_semantic_detailed(&self.options.workspace_id, input, 5)
            .await
        {
            if !result.confidential_entry_ids.is_empty() {
                let allowed = if let Some(airlock) = self.airlock_service.as_ref() {
                    let cmd = crate::models::neural::QueuedCommand {
                        id: uuid::Uuid::new_v4().to_string(),
                        intent: "memory_vault.read_confidential".to_string(),
                        payload: crate::models::neural::RainyPayload {
                            skill: Some("memory_vault".to_string()),
                            method: Some("read_confidential".to_string()),
                            params: None,
                            content: None,
                            allowed_paths: self.options.allowed_paths.clone().unwrap_or_default(),
                            blocked_paths: self.spec.airlock.scopes.blocked_paths.clone(),
                            allowed_domains: self.spec.airlock.scopes.allowed_domains.clone(),
                            blocked_domains: self.spec.airlock.scopes.blocked_domains.clone(),
                            tool_access_policy: None,
                            tool_access_policy_version: None,
                            tool_access_policy_hash: None,
                        },
                        status: crate::models::neural::CommandStatus::Pending,
                        priority: crate::models::neural::CommandPriority::Normal,
                        airlock_level: crate::models::neural::AirlockLevel::Dangerous,
                        created_at: Some(chrono::Utc::now().timestamp()),
                        started_at: None,
                        completed_at: None,
                        result: None,
                        workspace_id: Some(self.options.workspace_id.clone()),
                        desktop_node_id: None,
                        approved_by: None,
                    };

                    match airlock.check_permission(&cmd).await {
                        Ok(v) => v,
                        Err(_) => false,
                    }
                } else {
                    true
                };

                if !allowed {
                    let confidential_ids = result
                        .confidential_entry_ids
                        .iter()
                        .cloned()
                        .collect::<std::collections::HashSet<_>>();
                    result
                        .entries
                        .retain(|entry| !confidential_ids.contains(&entry.id));
                    result.reason = Some("Confidential memory filtered by Airlock".to_string());
                }
            }

            retrieval_mode = match result.mode {
                crate::services::memory::SemanticRetrievalMode::Ann => "ann",
                crate::services::memory::SemanticRetrievalMode::Exact => "exact",
                crate::services::memory::SemanticRetrievalMode::LexicalFallback => {
                    "lexical_fallback"
                }
            }
            .to_string();

            // Sanitize retrieved memory entries to prevent stored injection content
            // from surfacing into the system prompt (indirect prompt injection vector).
            let mut sanitized_result = result.clone();
            sanitized_result.entries = sanitized_result
                .entries
                .into_iter()
                .map(|mut entry| {
                    let s = crate::ai::agent::prompt_guard::sanitize_memory_context(&entry.content);
                    if s.was_modified {
                        eprintln!(
                            "[PromptGuard] Memory entry sanitized (id: {}). Flags: {:?}",
                            entry.id, s.flags
                        );
                    }
                    entry.content = s.text;
                    entry
                })
                .collect();
            appended_context = self.build_semantic_context_block(&context_window, &sanitized_result);
        }
        on_event(AgentEvent::Status(format!(
            "RAG_TELEMETRY:{}",
            serde_json::json!({
                "history_source": "persisted_long_chat",
                "retrieval_mode": retrieval_mode,
                "embedding_profile": crate::services::memory_vault::types::EMBEDDING_MODEL,
            })
            .to_string()
        )));

        if !appended_context.is_empty() {
            if let AgentContent::Text(ref mut sys_text) = state.messages[0].content {
                sys_text.push_str(&appended_context);
            } else {
                state.messages[0].content = AgentContent::text(format!(
                    "{}{}",
                    self.generate_system_prompt(&self.skills).await,
                    appended_context
                ));
            }
        }
        // -------------------------------------------

        // Add History to State (capture length for offset calculation later)
        let history_len;
        {
            let hist = self.history.lock().await;
            history_len = hist.len();
            state.messages.extend(hist.clone());
        }

        // Add the new User Message — wrap in structural delimiters so the LLM
        // can clearly identify the boundary of user-controlled text.
        state.messages.push(AgentMessage {
            role: "user".to_string(),
            content: AgentContent::text(
                crate::ai::agent::prompt_guard::wrap_user_turn(input)
            ),
            tool_calls: None,
            tool_call_id: None,
        });

        // Apply sliding context window — trim old messages to stay within token budget
        let pre_trim_len = state.messages.len();
        state.messages = context_window.trim_history(state.messages);

        // Apply robust Context Budget guard specifically for edge-case overflow recovery
        let (guarded_messages, overflowed) =
            crate::ai::agent::context_budget::ContextBudget::apply_context_guard(
                &state.messages,
                context_window.semantic_context_budget_tokens() * 5, // Full budget approximation
            );
        if overflowed {
            state.messages = crate::ai::agent::context_budget::ContextBudget::recover_from_overflow(
                &guarded_messages,
            );
        } else {
            state.messages = guarded_messages;
        }

        let trimmed_count = pre_trim_len - state.messages.len();
        let history_len = history_len.saturating_sub(trimmed_count);

        // 2. Build the Workflow Graph
        // In the future, this could be loaded from JSON.
        // For now, we build the standard "ReAct" loop: Think -> Act -> Think
        // NOTE: We pass spec and options to Workflow::new. This requires updating workflow.rs.
        let mut workflow =
            Workflow::new(self.spec.clone(), self.options.clone(), "think".to_string());

        // Step 1: Think (Router/LLM)
        let think_step = Box::new(ThinkStep {
            router: self.router.clone(),
            // Use runtime option model or default
            model: self
                .options
                .model
                .clone()
                .unwrap_or("gemini-2.0-flash".to_string()),
            allow_streaming: self.options.streaming_enabled.unwrap_or(false),
            reasoning_effort: self.options.reasoning_effort.clone(),
        });
        workflow.add_step(think_step);

        // Step 2: Act (Skill Executor)
        let act_step = Box::new(ActStep);
        workflow.add_step(act_step);

        // 3. Execute Workflow
        let on_event_clone = on_event.clone();
        let final_state = workflow
            .execute(state, self.skills.clone(), on_event_clone)
            .await
            .map_err(|e| format!("Workflow execution failed: {}", e))?;

        // 4. Update persistent history — append the user input + all new responses
        let last_message = final_state.messages.last().ok_or("No response generated")?;
        let new_messages_start = history_len + 1; // Skip system(1) + old history(N), start at user input

        {
            let mut hist = self.history.lock().await;
            for msg in final_state.messages.iter().skip(new_messages_start) {
                hist.push(msg.clone());
            }
        }

        // 5. Persist the assistant's final response into long-term memory
        if last_message.role == "assistant" {
            let mut response_text = last_message.content.as_text();
            if response_text.trim().is_empty() {
                if let Some(previous_non_empty) = final_state
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == "assistant" && !m.content.as_text().trim().is_empty())
                {
                    response_text = previous_non_empty.content.as_text();
                }
            }
            if response_text.trim().is_empty() {
                response_text =
                    "No final textual response was generated after tool execution.".to_string();
            }
            if !response_text.is_empty() && response_text.len() > 20 {
                // Airlock Gatekeeping: Check if Agent has permission to write memory.
                // We construct a synthetic command representing a memory write.
                let save_command = crate::models::neural::QueuedCommand {
                    id: uuid::Uuid::new_v4().to_string(),
                    intent: "memory_vault.write".to_string(),
                    payload: crate::models::neural::RainyPayload {
                        skill: Some("memory_vault".to_string()),
                        method: Some("write".to_string()),
                        params: None,
                        content: None,
                        allowed_paths: self.options.allowed_paths.clone().unwrap_or_default(),
                        blocked_paths: self.spec.airlock.scopes.blocked_paths.clone(),
                        allowed_domains: self.spec.airlock.scopes.allowed_domains.clone(),
                        blocked_domains: self.spec.airlock.scopes.blocked_domains.clone(),
                        tool_access_policy: None,
                        tool_access_policy_version: None,
                        tool_access_policy_hash: None,
                    },
                    status: crate::models::neural::CommandStatus::Pending,
                    priority: crate::models::neural::CommandPriority::Normal,
                    airlock_level: crate::models::neural::AirlockLevel::Sensitive, // Required for memory writes
                    created_at: Some(chrono::Utc::now().timestamp()),
                    started_at: None,
                    completed_at: None,
                    result: None,
                    workspace_id: Some(self.options.workspace_id.clone()),
                    desktop_node_id: None,
                    approved_by: None,
                };

                let allowed = if let Some(airlock) = self.airlock_service.as_ref() {
                    match airlock.check_permission(&save_command).await {
                        Ok(true) => true,
                        Ok(false) => {
                            on_event(AgentEvent::Error(
                                "Memory write blocked by Airlock".to_string(),
                            ));
                            false
                        }
                        Err(e) => {
                            on_event(AgentEvent::Error(format!("Airlock error: {}", e)));
                            false
                        }
                    }
                } else {
                    // Failsafe open if no airlock connected, preserving legacy behavior config
                    true
                };

                if allowed {
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert(
                        "source_input".to_string(),
                        input.chars().take(200).collect::<String>(),
                    );
                    metadata.insert("role".to_string(), "assistant".to_string());
                    self.memory
                        .store(
                            response_text.chars().take(2000).collect::<String>(),
                            "agent_conversation".to_string(),
                            Some(metadata),
                        )
                        .await;
                    on_event(AgentEvent::MemoryStored(
                        "Response persisted to memory".to_string(),
                    ));
                }
            }
            Ok(response_text)
        } else {
            Ok("Workflow completed without final response".to_string())
        }
    }
}
