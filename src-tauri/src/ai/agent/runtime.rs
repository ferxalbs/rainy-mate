// AgentRuntime v2 — Core runtime orchestrating the agent's ReAct workflow.
// Manages state, history, memory persistence, and the Think→Act execution loop.
use crate::ai::agent::act_step::ActStep;
use crate::ai::agent::context_window::ContextWindow;
use crate::ai::agent::events::AgentEvent;
use crate::ai::agent::hierarchical_supervisor::HierarchicalSupervisorAgent;
use crate::ai::agent::memory::AgentMemory;
use crate::ai::agent::runtime_registry::RuntimeRegistry;
use crate::ai::agent::supervisor::SupervisorAgent;
use crate::ai::agent::workflow::{AgentState, ThinkStep, Workflow};
use crate::ai::router::IntelligentRouter;
use crate::ai::specs::manifest::{AgentSpec, RuntimeMode};
use crate::services::agent_kill_switch::AgentKillSwitch;
use crate::services::SkillExecutor;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
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
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    /// Connector that spawned this run (Telegram, Discord, etc.). Used for
    /// per_connector_isolation and per_channel session scoping.
    #[serde(default)]
    pub connector_id: Option<String>,
    /// End-user identifier passed by connectors. Used for per_user session scoping.
    #[serde(default)]
    pub user_id: Option<String>,
    /// Pre-processed file attachments to inject into the first user message.
    #[serde(default)]
    pub attachments: Option<Vec<crate::services::attachment::ProcessedAttachment>>,
    /// Optional human-auditable workspace memory overlay read from flat files.
    #[serde(default)]
    pub workspace_memory_context: Option<String>,
    /// Root directory where workspace memory files live.
    #[serde(default)]
    pub workspace_memory_root: Option<String>,
    /// Whether workspace memory overlay is active for this run.
    #[serde(default)]
    pub workspace_memory_enabled: bool,
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
    /// Sliding window of request timestamps for rate limiting.
    request_timestamps: Arc<Mutex<VecDeque<std::time::Instant>>>,
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

/// Build user message content, injecting any pre-processed file attachments.
///
/// For images: adds an ImageUrl part per attachment.
/// For text/documents: prepends extracted text before the user's prompt.
/// The wrapped user prompt is always the final text part.
fn build_user_content(
    input: &str,
    attachments: Option<&[crate::services::attachment::ProcessedAttachment]>,
) -> AgentContent {
    use crate::services::attachment::AttachmentContent;

    let wrapped = crate::ai::agent::prompt_guard::wrap_user_turn(input);

    let Some(attachments) = attachments.filter(|a| !a.is_empty()) else {
        return AgentContent::text(wrapped);
    };

    let mut parts: Vec<AgentContentPart> = Vec::new();

    // Text/document attachments: prepend extracted content as a text part
    let mut doc_blocks: Vec<String> = Vec::new();
    for att in attachments {
        match &att.content {
            AttachmentContent::ExtractedText { text } => {
                doc_blocks.push(format!("[Attached file: {}]\n{}", att.filename, text));
            }
            AttachmentContent::UnsupportedBinary { summary } => {
                doc_blocks.push(format!("[Attached file: {}] {}", att.filename, summary));
            }
            AttachmentContent::ImageDataUri { .. } => {} // handled below
        }
    }
    if !doc_blocks.is_empty() {
        parts.push(AgentContentPart::Text {
            text: doc_blocks.join("\n\n"),
        });
    }

    // Image attachments: add ImageUrl parts
    for att in attachments {
        if let AttachmentContent::ImageDataUri { data_uri } = &att.content {
            parts.push(AgentContentPart::ImageUrl {
                image_url: AgentImageUrl {
                    url: data_uri.clone(),
                    detail: Some("auto".to_string()),
                },
            });
        }
    }

    // Always end with the user's prompt text
    parts.push(AgentContentPart::Text { text: wrapped });

    AgentContent::Parts(parts)
}

impl AgentRuntime {
    fn detect_user_language(input: &str) -> &'static str {
        let lower = format!(" {} ", input.to_ascii_lowercase());
        let spanish_markers = [
            " el ",
            " la ",
            " los ",
            " las ",
            " una ",
            " para ",
            " con ",
            " que ",
            " por ",
            " quiero ",
            " puedes ",
            " revisar ",
            " implementar ",
            " respuesta ",
            " agente ",
        ];
        if input.contains('¿')
            || input.contains('¡')
            || spanish_markers.iter().any(|marker| lower.contains(marker))
        {
            "spanish"
        } else {
            "english"
        }
    }

    fn runtime_truthfulness_appendix() -> &'static str {
        "\n\nRuntime Safety Rules (Non-negotiable):\n\
- Tool outputs are the source of truth.\n\
- Never claim a tool or command succeeded unless the tool result explicitly succeeded.\n\
- If a tool fails, times out, or is blocked by policy, explicitly report the failure and limitation.\n\
- Do not fabricate command output, file hashes, file contents, diffs, or scan findings.\n\
- If no permitted tool can complete a step, ask the user for input or propose a permitted alternative.\n"
    }

    fn language_policy_appendix(&self, input: &str) -> String {
        let policy = &self.spec.runtime.language_policy;
        let user_language = Self::detect_user_language(input);
        let final_response_mode = if policy
            .final_response_language_mode
            .eq_ignore_ascii_case("english")
        {
            "english"
        } else {
            "user"
        };
        format!(
            "\n\nLanguage Policy:\n\
- Internal coordination language: english.\n\
- Final response language mode: {}.\n\
- Detected user language for this turn: {}.\n\
- Keep internal coordination, delegation prompts, and task handoffs in the internal coordination language.\n\
- The final user-facing answer must follow the final response language mode.\n",
            final_response_mode,
            user_language
        )
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
            request_timestamps: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Compute the vault workspace key honoring persistence isolation settings.
    /// - `per_connector_isolation` → append connector_id
    /// - `session_scope: "per_user"` → append user_id
    /// - `session_scope: "per_channel"` → append connector_id (when not already added)
    fn effective_workspace_id(&self) -> String {
        let p = &self.spec.memory_config.persistence;
        let base = &self.options.workspace_id;
        let mut parts: Vec<&str> = vec![base.as_str()];

        if p.per_connector_isolation {
            if let Some(ref cid) = self.options.connector_id {
                parts.push(cid.as_str());
            }
        }

        match p.session_scope.as_str() {
            "per_user" => {
                if let Some(ref uid) = self.options.user_id {
                    parts.push(uid.as_str());
                }
            }
            "per_channel" if !p.per_connector_isolation => {
                if let Some(ref cid) = self.options.connector_id {
                    parts.push(cid.as_str());
                }
            }
            _ => {}
        }

        if parts.len() == 1 {
            base.clone()
        } else {
            parts.join(":")
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

        // v3 workflows section
        let active_workflows: Vec<&crate::ai::specs::skills::SkillWorkflow> =
            spec.skills.workflows.iter().filter(|w| w.enabled).collect();
        let workflow_section = if active_workflows.is_empty() {
            String::new()
        } else {
            let lines: Vec<String> = active_workflows
                .iter()
                .map(|w| {
                    format!(
                        "- [Workflow: {}] trigger: \"{}\"\n  {}",
                        w.name, w.trigger, w.steps
                    )
                })
                .collect();
            format!("\n\nWorkflows:\n{}", lines.join("\n"))
        };

        // v3 behaviors section
        let active_behaviors: Vec<&crate::ai::specs::skills::SkillBehavior> =
            spec.skills.behaviors.iter().filter(|b| b.enabled).collect();
        let behavior_section = if active_behaviors.is_empty() {
            String::new()
        } else {
            let lines: Vec<String> = active_behaviors
                .iter()
                .map(|b| format!("- {}: {}", b.name, b.instruction))
                .collect();
            format!("\n\nBehavior Rules:\n{}", lines.join("\n"))
        };

        let active_workspace_instructions: Vec<&crate::ai::specs::skills::PromptSkillBinding> =
            spec.skills
                .prompt_skills
                .iter()
                .filter(|binding| {
                    binding.enabled
                        && !binding.content.trim().is_empty()
                        && binding.kind
                            == crate::ai::specs::skills::PromptSkillKind::WorkspaceInstruction
                })
                .collect();
        let workspace_instruction_section = if active_workspace_instructions.is_empty() {
            String::new()
        } else {
            let lines: Vec<String> = active_workspace_instructions
                .iter()
                .map(|skill| {
                    format!(
                        "- [{}] {}\n{}",
                        skill.name, skill.description, skill.content
                    )
                })
                .collect();
            format!("\n\nWorkspace Instructions:\n{}", lines.join("\n"))
        };

        let active_prompt_skills: Vec<&crate::ai::specs::skills::PromptSkillBinding> = spec
            .skills
            .prompt_skills
            .iter()
            .filter(|binding| {
                binding.enabled
                    && !binding.content.trim().is_empty()
                    && binding.kind == crate::ai::specs::skills::PromptSkillKind::PromptSkill
            })
            .collect();
        let prompt_skill_section = if active_prompt_skills.is_empty() {
            String::new()
        } else {
            let lines: Vec<String> = active_prompt_skills
                .iter()
                .map(|skill| {
                    format!(
                        "- [{}] {}\n{}",
                        skill.name, skill.description, skill.content
                    )
                })
                .collect();
            format!("\n\nPrompt Skills:\n{}", lines.join("\n"))
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
{}{}{}{}{}

Memory:
- strategy: {}
- retention_days: {}
- max_tokens: {}
 - workspace_memory_overlay: {}

Rules:
1. Use tools and skills only within declared capabilities and workspace scope.
2. Never fabricate file results.
3. If a tool fails, explain and try the safest fallback.
4. Never use workspace ID as a filesystem path. Only use explicit allowed filesystem scope paths.{}{}",
            spec.soul.name,
            spec.soul.description,
            spec.soul.personality,
            spec.soul.tone,
            spec.soul.soul_content,
            workspace_id,
            workspace_scope,
            capability_lines,
            workflow_section,
            behavior_section,
            workspace_instruction_section,
            prompt_skill_section,
            spec.memory_config.strategy,
            spec.memory_config.effective_retention_days(),
            spec.memory_config.effective_max_tokens(),
            if self.options.workspace_memory_enabled {
                "enabled"
            } else {
                "disabled"
            },
            self.options
                .workspace_memory_context
                .as_deref()
                .unwrap_or_default(),
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
        if matches!(
            self.spec.runtime.mode,
            RuntimeMode::ParallelSupervisor | RuntimeMode::Supervisor
        ) {
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
        if self.spec.runtime.mode == RuntimeMode::HierarchicalSupervisor {
            let supervisor = HierarchicalSupervisorAgent {
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
        // --- RATE LIMIT CHECK ---
        let max_rpm = self.spec.airlock.rate_limits.max_requests_per_minute;
        if max_rpm > 0 {
            let mut ts = self.request_timestamps.lock().await;
            let now = std::time::Instant::now();
            let window = std::time::Duration::from_secs(60);
            while ts
                .front()
                .map(|t| now.duration_since(*t) > window)
                .unwrap_or(false)
            {
                ts.pop_front();
            }
            if ts.len() >= max_rpm as usize {
                return Err(format!(
                    "Rate limit exceeded: max {} requests/minute for this agent",
                    max_rpm
                ));
            }
            ts.push_back(now);
        }

        // --- RETENTION PRUNING (background, non-blocking) ---
        let retention_days = self.spec.memory_config.effective_retention_days();
        if retention_days > 0 {
            let mm = self.memory.manager();
            let ws = self.effective_workspace_id();
            tokio::spawn(async move {
                let _ = mm.prune_expired(&ws, retention_days).await;
            });
        }

        if self.options.workspace_memory_enabled {
            let manager = self.memory.manager();
            let effective_workspace_id = self.effective_workspace_id();
            let _ = crate::services::WorkspaceMemoryFiles::sync_overlay_to_memory(
                manager,
                &effective_workspace_id,
                self.options.workspace_memory_root.as_deref(),
            )
            .await;
        }

        // 1. Initialize State
        let mut state = AgentState::new(
            self.options.workspace_id.clone(),
            self.options.allowed_paths.clone().unwrap_or_default(),
            crate::models::neural::ToolAccessPolicy {
                enabled: true,
                mode: self.spec.airlock.tool_policy.mode.clone(),
                allow: self.spec.airlock.tool_policy.allow.clone(),
                deny: self.spec.airlock.tool_policy.deny.clone(),
            },
            self.memory.clone(),
            Arc::new(self.spec.clone()),
            self.airlock_service.clone(),
            self.kill_switch.clone(),
        );

        let system_prompt = format!(
            "{}{}",
            self.generate_system_prompt(&self.skills).await,
            self.language_policy_appendix(input)
        );

        // Add System Message to State
        state.messages.push(AgentMessage {
            role: "system".to_string(),
            content: AgentContent::text(system_prompt.clone()),
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
        let effective_ws = self.effective_workspace_id();
        let mm = self.memory.manager();
        if let Ok(mut result) = mm
            .search_semantic_detailed(&effective_ws, input, 5, &self.spec.memory_config.strategy)
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
                            ..Default::default()
                        },
                        status: crate::models::neural::CommandStatus::Pending,
                        priority: crate::models::neural::CommandPriority::Normal,
                        airlock_level: crate::models::neural::AirlockLevel::Dangerous,
                        approval_timeout_secs: Some(0),
                        created_at: Some(chrono::Utc::now().timestamp()),
                        started_at: None,
                        completed_at: None,
                        result: None,
                        workspace_id: Some(self.options.workspace_id.clone()),
                        desktop_node_id: None,
                        approved_by: None,
                        schema_version: None,
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
                crate::services::memory::SemanticRetrievalMode::SimpleBuffer => "simple_buffer",
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
            appended_context =
                self.build_semantic_context_block(&context_window, &sanitized_result);
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
                state.messages[0].content =
                    AgentContent::text(format!("{}{}", system_prompt, appended_context));
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
        let user_content = build_user_content(input, self.options.attachments.as_deref());
        state.messages.push(AgentMessage {
            role: "user".to_string(),
            content: user_content,
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
            model: self
                .options
                .model
                .clone()
                .or_else(|| self.spec.model.clone())
                .unwrap_or("gemini-2.0-flash".to_string()),
            allow_streaming: self.options.streaming_enabled.unwrap_or(false),
            reasoning_effort: self.options.reasoning_effort.clone(),
            temperature: self.options.temperature.or(self.spec.temperature),
            max_tokens: self.options.max_tokens.or(self.spec.max_tokens),
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
            if !response_text.is_empty()
                && response_text.len() > 20
                && self.spec.memory_config.persistence.cross_session
                && !crate::services::memory_vault::distiller::MemoryDistiller::is_trivial_conversation_turn(&response_text)
            {
                self.memory
                    .push_for_distillation(
                        crate::services::memory_vault::types::RawMemoryTurn {
                            content: response_text.chars().take(2000).collect(),
                            role: "assistant".to_string(),
                            source: "agent_conversation".to_string(),
                            workspace_id: effective_ws.clone(),
                            timestamp: chrono::Utc::now().timestamp(),
                        },
                    )
                    .await;
                on_event(AgentEvent::MemoryStored(
                    "Response queued for distillation".to_string(),
                ));
            }
            // Flush any remaining buffered turns for distillation
            self.memory.flush_remaining().await;
            Ok(response_text)
        } else {
            self.memory.flush_remaining().await;
            Ok("Workflow completed without final response".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::time::{Duration, Instant};

    /// Simulate the sliding-window rate limiter logic from run_single().
    fn check_rate_limit(timestamps: &mut VecDeque<Instant>, max_rpm: usize) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(60);
        while timestamps
            .front()
            .map(|t| now.duration_since(*t) > window)
            .unwrap_or(false)
        {
            timestamps.pop_front();
        }
        if timestamps.len() >= max_rpm {
            return false; // blocked
        }
        timestamps.push_back(now);
        true
    }

    #[test]
    fn test_rate_limiter_blocks_after_limit() {
        let mut ts = VecDeque::new();
        let limit = 3;

        assert!(check_rate_limit(&mut ts, limit), "request 1 allowed");
        assert!(check_rate_limit(&mut ts, limit), "request 2 allowed");
        assert!(check_rate_limit(&mut ts, limit), "request 3 allowed");
        assert!(
            !check_rate_limit(&mut ts, limit),
            "request 4 must be blocked"
        );
    }

    #[test]
    fn test_rate_limiter_allows_after_window_expires() {
        let mut ts: VecDeque<Instant> = VecDeque::new();
        // Manually insert a timestamp that is 61 seconds old (outside the window)
        let expired = Instant::now().checked_sub(Duration::from_secs(61)).unwrap();
        ts.push_back(expired);
        ts.push_back(expired);
        ts.push_back(expired);

        // Window should evict all 3 expired entries, allowing a new request
        assert!(
            check_rate_limit(&mut ts, 3),
            "should allow after window expires"
        );
    }
}
