// Workflow Engine v2 — Step-based execution model for the agent's ReAct loop.
// ThinkStep (LLM interaction) lives here. ActStep (tool execution) lives in act_step.rs.
use crate::ai::agent::events::AgentEvent;
use crate::ai::agent::memory::AgentMemory;
use crate::ai::agent::runtime::{AgentContent, AgentMessage, RuntimeOptions};
use crate::ai::router::IntelligentRouter;
use crate::ai::specs::manifest::AgentSpec;
use crate::services::agent_kill_switch::AgentKillSwitch;
use crate::services::SkillExecutor;
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_MODEL_MESSAGE_BYTES: usize = 95 * 1024;
pub const FILESYSTEM_TOOL_NAMES: &[&str] = &[
    "read_file",
    "read_many_files",
    "write_file",
    "append_file",
    "list_files",
    "list_files_detailed",
    "file_exists",
    "get_file_info",
    "search_files",
    "read_file_chunk",
    "mkdir",
    "delete_file",
    "move_file",
];

pub(crate) fn is_tool_allowed_by_spec(spec: &AgentSpec, tool_name: &str) -> bool {
    spec.airlock.is_tool_allowed(tool_name)
}

pub(crate) fn truncate_to_max_bytes(input: &str, max_bytes: usize) -> String {
    if input.len() <= max_bytes {
        return input.to_string();
    }
    let mut cut = 0usize;
    for (idx, _) in input.char_indices() {
        if idx <= max_bytes {
            cut = idx;
        } else {
            break;
        }
    }
    let mut out = input[..cut].to_string();
    out.push_str("\n\n[TRUNCATED: content exceeded size limits]");
    out
}

/// Shared state passed between workflow steps
#[derive(Clone, Debug)]
pub struct AgentState {
    pub messages: Vec<AgentMessage>,
    #[allow(dead_code)] // @TODO Future context sharing
    pub context: HashMap<String, String>,
    pub workspace_id: String,
    pub allowed_paths: Vec<String>,
    pub memory: Arc<AgentMemory>,
    #[allow(dead_code)] // Used by steps
    pub spec: Arc<AgentSpec>,
    pub airlock_service: Arc<Option<crate::services::airlock::AirlockService>>,
    pub kill_switch: Option<AgentKillSwitch>,
}

impl AgentState {
    pub fn new(
        workspace_id: String,
        allowed_paths: Vec<String>,
        memory: Arc<AgentMemory>,
        spec: Arc<AgentSpec>,
        airlock_service: Arc<Option<crate::services::airlock::AirlockService>>,
        kill_switch: Option<AgentKillSwitch>,
    ) -> Self {
        Self {
            messages: Vec::new(),
            context: HashMap::new(),
            workspace_id,
            allowed_paths,
            memory,
            spec,
            airlock_service,
            kill_switch,
        }
    }
}

/// Result of a workflow step execution
#[derive(Clone, Debug)]
pub struct StepResult {
    /// The next step to execute (None = workflow complete)
    pub next_step: Option<String>,
    /// Whether the execution was successful
    pub success: bool,
    /// Optional output to log
    pub output: Option<String>,
}

/// A single step in the workflow graph
#[async_trait::async_trait]
pub trait WorkflowStep: Debug + Send + Sync {
    /// Unique identifier for this step
    fn id(&self) -> String;

    /// Execute the step logic
    async fn execute(
        &self,
        state: &mut AgentState,
        skills: Arc<SkillExecutor>,
        on_event: Box<dyn Fn(AgentEvent) + Send + Sync>,
    ) -> Result<StepResult, String>;
}

/// The Workflow Graph Container
pub struct Workflow {
    #[allow(dead_code)] // @TODO Configuration usage in steps
    pub spec: AgentSpec,
    #[allow(dead_code)]
    pub options: RuntimeOptions,
    pub steps: HashMap<String, Box<dyn WorkflowStep>>,
    pub start_step: String,
}

impl Workflow {
    pub fn new(spec: AgentSpec, options: RuntimeOptions, start_step: String) -> Self {
        Self {
            spec,
            options,
            steps: HashMap::new(),
            start_step,
        }
    }

    pub fn add_step(&mut self, step: Box<dyn WorkflowStep>) {
        self.steps.insert(step.id(), step);
    }

    pub async fn execute<F>(
        &self,
        initial_state: AgentState,
        skills: Arc<SkillExecutor>,
        on_event: F,
    ) -> Result<AgentState, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        let mut state = initial_state;
        let mut current_step_id = Some(self.start_step.clone());
        let mut steps_count = 0;
        const DEFAULT_MAX_STEPS: usize = 50;
        const ABSOLUTE_MAX_STEPS: usize = 200;
        let max_steps = self
            .options
            .max_steps
            .unwrap_or(DEFAULT_MAX_STEPS)
            .clamp(4, ABSOLUTE_MAX_STEPS);

        while let Some(step_id) = current_step_id {
            if state
                .kill_switch
                .as_ref()
                .is_some_and(|switch| switch.is_triggered())
            {
                on_event(AgentEvent::Status(
                    "Execution terminated by fleet kill switch".to_string(),
                ));
                state.messages.push(AgentMessage {
                    role: "assistant".to_string(),
                    content: AgentContent::text(
                        "Execution was terminated by Fleet Kill Switch. Partial progress has been preserved.",
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                });
                return Ok(state);
            }

            if steps_count >= max_steps {
                on_event(AgentEvent::Status(format!(
                    "Stopping workflow after {} steps to prevent infinite tool loops.",
                    max_steps
                )));

                let last_assistant_text = state
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == "assistant")
                    .map(|m| m.content.as_text())
                    .unwrap_or_default();

                let fallback = if last_assistant_text.trim().is_empty() {
                    format!(
                        "I could not complete this request within {} workflow steps. \
Please narrow the task or break it into smaller steps.",
                        max_steps
                    )
                } else {
                    format!(
                        "{}\n\n[Execution stopped after {} steps to prevent an infinite loop. \
Please narrow the task or ask me to continue with a focused next step.]",
                        last_assistant_text, max_steps
                    )
                };

                state.messages.push(AgentMessage {
                    role: "assistant".to_string(),
                    content: AgentContent::text(fallback),
                    tool_calls: None,
                    tool_call_id: None,
                });
                return Ok(state);
            }

            let step = self
                .steps
                .get(&step_id)
                .ok_or(format!("Step not found: {}", step_id))?;

            // Execute the step
            // We pass a clone of skills for now. State is mutable.
            let result = step
                .execute(&mut state, skills.clone(), Box::new(on_event.clone()))
                .await?;

            if !result.success {
                return Err(format!("Step {} failed: {:?}", step_id, result.output));
            }

            // Transition
            current_step_id = result.next_step;
            steps_count += 1;
        }

        Ok(state)
    }
}

// --- Concrete Step Implementations ---

#[derive(Debug)]
pub struct ThinkStep {
    pub router: Arc<RwLock<IntelligentRouter>>,
    pub model: String,
    pub allow_streaming: bool,
    pub reasoning_effort: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[async_trait::async_trait]
impl WorkflowStep for ThinkStep {
    fn id(&self) -> String {
        "think".to_string()
    }

    async fn execute(
        &self,
        state: &mut AgentState,
        skills: Arc<SkillExecutor>,
        on_event: Box<dyn Fn(AgentEvent) + Send + Sync>,
    ) -> Result<StepResult, String> {
        on_event(AgentEvent::Status("Thinking...".to_string()));

        // 1. Prepare messages
        let mut messages: Vec<crate::ai::provider_types::ChatMessage> = state
            .messages
            .iter()
            .map(|m| {
                if m.role == "system" {
                    // System messages are text-only, use as_text()
                    crate::ai::provider_types::ChatMessage::system(truncate_to_max_bytes(
                        &m.content.as_text(),
                        MAX_MODEL_MESSAGE_BYTES,
                    ))
                } else if m.role == "user" {
                    // User messages support multimodal, convert AgentContent -> MessageContent
                    crate::ai::provider_types::ChatMessage::user(m.content.clone())
                } else {
                    // Other roles (assistant, tool) also support multimodal
                    crate::ai::provider_types::ChatMessage {
                        role: m.role.clone(),
                        content: m.content.clone().into(),
                        name: None,
                        tool_calls: m.tool_calls.clone(),
                        tool_call_id: m.tool_call_id.clone(),
                    }
                }
            })
            .collect();

        // 1.5. Push user input to distillation buffer for intelligent extraction
        if let Some(last_user_msg) = state.messages.iter().rfind(|m| m.role == "user") {
            let user_text = last_user_msg.content.as_text();
            if state.spec.memory_config.persistence.cross_session
                && !user_text.is_empty()
                && user_text.len() > 10
                && !crate::services::memory_vault::distiller::MemoryDistiller::is_trivial_conversation_turn(&user_text)
            {
                state
                    .memory
                    .push_for_distillation(crate::services::memory_vault::types::RawMemoryTurn {
                        content: user_text.chars().take(2000).collect(),
                        role: "user".to_string(),
                        source: "agent_conversation".to_string(),
                        workspace_id: state.workspace_id.clone(),
                        timestamp: chrono::Utc::now().timestamp(),
                    })
                    .await;
            }
        }

        // Final guardrail: enforce per-message text size limits before provider call.
        for msg in messages.iter_mut() {
            match &mut msg.content {
                crate::ai::provider_types::MessageContent::Text(text) => {
                    if text.len() > MAX_MODEL_MESSAGE_BYTES {
                        *text = truncate_to_max_bytes(text, MAX_MODEL_MESSAGE_BYTES);
                    }
                }
                crate::ai::provider_types::MessageContent::Parts(parts) => {
                    for part in parts.iter_mut() {
                        if let crate::ai::provider_types::ContentPart::Text { text } = part {
                            if text.len() > MAX_MODEL_MESSAGE_BYTES {
                                *text = truncate_to_max_bytes(text, MAX_MODEL_MESSAGE_BYTES);
                            }
                        }
                    }
                }
            }
        }

        // 2. Prepare tools
        let mut tools = skills.get_tool_definitions().await;
        if state.allowed_paths.is_empty() {
            tools.retain(|tool| !FILESYSTEM_TOOL_NAMES.contains(&tool.function.name.as_str()));
        }
        // Enforce AgentSpec Airlock tool policy at tool-advertisement time.
        tools.retain(|tool| is_tool_allowed_by_spec(state.spec.as_ref(), &tool.function.name));
        let has_tools = !tools.is_empty();

        let request = crate::ai::provider_types::ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(self.temperature.unwrap_or(0.7)),
            max_tokens: self.max_tokens,
            top_p: None,
            stream: self.allow_streaming && !has_tools,
            tools: if has_tools { Some(tools) } else { None },
            tool_choice: if has_tools {
                Some(crate::ai::provider_types::ToolChoice::Auto)
            } else {
                None
            },
            json_mode: false,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            reasoning_effort: self.reasoning_effort.clone(),
        };

        // 3. Call Router — streaming when no tools, blocking otherwise
        let router_guard = self.router.read().await;

        let (assistant_content, tool_calls) = if has_tools || !self.allow_streaming {
            let event_fn: Arc<dyn Fn(AgentEvent) + Send + Sync> = Arc::from(on_event);

            // Emit a single status so the UI shows active planning.
            event_fn(AgentEvent::Status(
                "Analizando y planificando herramientas...".to_string(),
            ));

            let mut blocking_request = request.clone();
            blocking_request.stream = false;

            let response = router_guard
                .complete(blocking_request)
                .await
                .map_err(|e| format!("ThinkStep Failed: {}", e))?;

            let mut content = response.content.clone().unwrap_or_default();
            let mut resolved_tool_calls = response.tool_calls.clone();

            // Recovery guard: some providers may emit an empty assistant turn after tool execution.
            // Force a final textual answer from accumulated tool results with tools disabled.
            if content.trim().is_empty()
                && resolved_tool_calls
                    .as_ref()
                    .map(|calls| calls.is_empty())
                    .unwrap_or(true)
            {
                let mut recovery_request = request.clone();
                recovery_request.stream = false;
                recovery_request.tools = None;
                recovery_request.tool_choice = None;
                recovery_request.messages.push(crate::ai::provider_types::ChatMessage::user(
                    "Using the previous tool results, provide the final answer in plain text. Do not call tools.",
                ));

                let recovery = router_guard
                    .complete(recovery_request)
                    .await
                    .map_err(|e| format!("ThinkStep Recovery Failed: {}", e))?;

                if let Some(recovered_text) = recovery.content {
                    content = recovered_text;
                }
                resolved_tool_calls = recovery.tool_calls;
            }

            if !content.is_empty() {
                event_fn(AgentEvent::Thought(content.clone()));
            }

            (content, resolved_tool_calls)
        } else {
            // Streaming path: emit token-by-token chunks to the frontend
            let accumulated = Arc::new(std::sync::Mutex::new(String::new()));
            let acc_clone = Arc::clone(&accumulated);
            let event_fn: Arc<dyn Fn(AgentEvent) + Send + Sync> = Arc::from(on_event);
            let event_clone = Arc::clone(&event_fn);

            let callback: crate::ai::provider_types::StreamingCallback =
                Arc::new(move |chunk: crate::ai::provider_types::StreamingChunk| {
                    if !chunk.content.is_empty() {
                        event_clone(AgentEvent::StreamChunk(chunk.content.clone()));
                        if let Ok(mut guard) = acc_clone.lock() {
                            guard.push_str(&chunk.content);
                        }
                    }
                });

            router_guard
                .complete_stream(request, callback)
                .await
                .map_err(|e| format!("ThinkStep Streaming Failed: {}", e))?;

            let content = accumulated
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            if !content.is_empty() {
                // Emit full thought after streaming completes
                event_fn(AgentEvent::Thought(content.clone()));
            }
            (content, None)
        };
        drop(router_guard);

        // 4. Update State
        state.messages.push(AgentMessage {
            role: "assistant".to_string(),
            content: AgentContent::text(assistant_content.clone()),
            tool_calls: tool_calls.clone(),
            tool_call_id: None,
        });

        // 5. Determine Next Step
        if let Some(calls) = tool_calls {
            if !calls.is_empty() {
                return Ok(StepResult {
                    next_step: Some("act".to_string()),
                    success: true,
                    output: Some(format!("Generated {} tool calls", calls.len())),
                });
            }
        }

        // No tool calls -> Done
        Ok(StepResult {
            next_step: None, // End of workflow
            success: true,
            output: Some(assistant_content),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agent::memory::AgentMemory;
    use crate::ai::specs::manifest::AgentSpec;
    use crate::ai::AIProviderManager;
    use crate::services::workspace::WorkspaceManager;
    use crate::services::{BrowserController, ManagedResearchService, SkillExecutor};
    use serial_test::serial;
    use std::sync::Arc;

    #[derive(Debug)]
    struct MockStep {
        id: String,
        next: Option<String>,
        output: String,
    }

    #[async_trait::async_trait]
    impl WorkflowStep for MockStep {
        fn id(&self) -> String {
            self.id.clone()
        }

        async fn execute(
            &self,
            state: &mut AgentState,
            _skills: Arc<SkillExecutor>,
            _on_event: Box<dyn Fn(AgentEvent) + Send + Sync>,
        ) -> Result<StepResult, String> {
            state.context.insert(self.id.clone(), "visited".to_string());
            Ok(StepResult {
                next_step: self.next.clone(),
                success: true,
                output: Some(self.output.clone()),
            })
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_workflow_execution() {
        use crate::ai::specs::skills::AgentSkills;
        use crate::ai::specs::soul::AgentSoul;

        let spec = AgentSpec {
            id: "test-agent".to_string(),
            version: "1.0.0".to_string(),
            soul: AgentSoul {
                name: "Test Agent".to_string(),
                soul_content: "test".to_string(),
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
        };

        let options = RuntimeOptions {
            model: Some("test-model".to_string()),
            workspace_id: "test-ws".to_string(),
            max_steps: Some(10),
            allowed_paths: None,
            custom_system_prompt: None,
            streaming_enabled: Some(false),
            reasoning_effort: None,
            temperature: None,
            max_tokens: None,
            connector_id: None,
            user_id: None,
        };

        let mut workflow = Workflow::new(spec.clone(), options, "start".to_string());

        workflow.add_step(Box::new(MockStep {
            id: "start".to_string(),
            next: Some("middle".to_string()),
            output: "Step 1 Done".to_string(),
        }));

        workflow.add_step(Box::new(MockStep {
            id: "middle".to_string(),
            next: None,
            output: "Step 2 Done".to_string(),
        }));

        // Note: This relies on being able to create WorkspaceManager in test env.
        // We need an isolated temp dir for memory
        let temp_dir = tempfile::TempDir::new().unwrap();
        let memory_manager = Arc::new(crate::services::MemoryManager::new(
            100,
            temp_dir.path().join("memory_db"),
        ));
        memory_manager.init().await;
        let memory = Arc::new(
            AgentMemory::new(
                "test-ws",
                temp_dir.path().to_path_buf(),
                memory_manager,
                None,
                None,
            )
            .await,
        );

        let state = AgentState::new(
            "test-ws".to_string(),
            Vec::new(),
            memory,
            Arc::new(spec.clone()),
            Arc::new(None),
            None,
        );

        match WorkspaceManager::new() {
            Ok(wm) => {
                let provider_manager = Arc::new(AIProviderManager::new());
                let managed_research = Arc::new(ManagedResearchService::new(provider_manager));
                let browser = Arc::new(BrowserController::new());
                let mcp_service = Arc::new(crate::services::mcp_service::McpService::new());
                let skills = Arc::new(SkillExecutor::new(
                    Arc::new(wm),
                    managed_research,
                    browser,
                    mcp_service,
                ));
                let result = workflow.execute(state, skills, |_| {}).await;
                assert!(result.is_ok());

                let final_state = result.unwrap();
                assert_eq!(
                    final_state.context.get("start"),
                    Some(&"visited".to_string())
                );
                assert_eq!(
                    final_state.context.get("middle"),
                    Some(&"visited".to_string())
                );
            }
            Err(_) => {
                println!("Skipping test due to WorkspaceManager init failure");
            }
        }
    }
}
