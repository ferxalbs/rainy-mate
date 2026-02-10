// AgentRuntime v2 — Core runtime orchestrating the agent's ReAct workflow.
// Manages state, history, memory persistence, and the Think→Act execution loop.
use crate::ai::agent::context_window::ContextWindow;
use crate::ai::agent::memory::AgentMemory;
use crate::ai::agent::workflow::{ActStep, AgentState, ThinkStep, Workflow};
use crate::ai::router::IntelligentRouter;
use crate::ai::specs::manifest::AgentSpec;
use crate::services::SkillExecutor;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RuntimeOptions {
    pub model: Option<String>,
    pub workspace_id: String,
    pub max_steps: Option<usize>,
}

/// The core runtime that orchestrates the agent's thinking process
pub struct AgentRuntime {
    pub spec: AgentSpec,
    pub options: RuntimeOptions,
    router: Arc<tokio::sync::RwLock<IntelligentRouter>>,
    skills: Arc<SkillExecutor>,
    memory: Arc<AgentMemory>,
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

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AgentEvent {
    Status(String),
    Thought(String),
    /// Token-by-token streaming chunks during LLM generation
    StreamChunk(String),
    ToolCall(crate::ai::provider_types::ToolCall),
    ToolResult {
        id: String,
        result: String,
    },
    #[allow(dead_code)] // @RESERVED — will be emitted by error handling in workflow
    Error(String),
    /// Emitted when the agent stores a memory entry
    MemoryStored(String),
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
    pub fn new(
        spec: AgentSpec,
        options: RuntimeOptions,
        router: Arc<tokio::sync::RwLock<IntelligentRouter>>,
        skills: Arc<SkillExecutor>,
        memory: Arc<AgentMemory>,
    ) -> Self {
        Self {
            spec,
            options,
            router,
            skills,
            memory,
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Replace in-memory history for this runtime instance.
    pub async fn set_history(&self, messages: Vec<AgentMessage>) {
        let mut hist = self.history.lock().await;
        *hist = messages;
    }

    fn generate_system_prompt(&self) -> String {
        let spec = &self.spec;
        let workspace_id = &self.options.workspace_id;

        let capability_lines = if spec.skills.capabilities.is_empty() {
            "- No explicit capabilities configured".to_string()
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

Workspace Path: {}

Capabilities:
{}

Memory:
- strategy: {}
- retention_days: {}
- max_tokens: {}

Rules:
1. Use tools and skills only within declared capabilities and workspace scope.
2. Never fabricate file results.
3. If a tool fails, explain and try the safest fallback.",
            spec.soul.name,
            spec.soul.description,
            spec.soul.personality,
            spec.soul.tone,
            spec.soul.soul_content,
            workspace_id,
            capability_lines,
            spec.memory_config.strategy,
            spec.memory_config.retention_days,
            spec.memory_config.max_tokens
        )
    }

    /// Primary entry point: Run a workflow/turn
    pub async fn run<F>(&self, input: &str, on_event: F) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        // 1. Initialize State
        let mut state = AgentState::new(
            self.options.workspace_id.clone(),
            self.memory.clone(),
            Arc::new(self.spec.clone()),
        );

        // Add System Message to State
        state.messages.push(AgentMessage {
            role: "system".to_string(),
            content: AgentContent::text(self.generate_system_prompt()),
            tool_calls: None,
            tool_call_id: None,
        });

        // Add History to State (capture length for offset calculation later)
        let history_len;
        {
            let hist = self.history.lock().await;
            history_len = hist.len();
            state.messages.extend(hist.clone());
        }

        // Add the new User Message
        state.messages.push(AgentMessage {
            role: "user".to_string(),
            content: AgentContent::text(input),
            tool_calls: None,
            tool_call_id: None,
        });

        // Apply sliding context window — trim old messages to stay within token budget
        let context_window = ContextWindow::new(self.spec.memory_config.max_tokens as usize);
        let pre_trim_len = state.messages.len();
        state.messages = context_window.trim_history(state.messages);
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
            let response_text = last_message.content.as_text();
            if !response_text.is_empty() && response_text.len() > 20 {
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
            Ok(last_message.content.as_text())
        } else {
            Ok("Workflow completed without final response".to_string())
        }
    }
}
