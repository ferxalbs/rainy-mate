// @deprecated: This module is being replaced by the new native AgentSpec v2 system.
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
    ToolCall(crate::ai::provider_types::ToolCall),
    ToolResult {
        id: String,
        result: String,
    },
    #[allow(dead_code)]
    Error(String),
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

        // Add History to State
        {
            let hist = self.history.lock().await;
            state.messages.extend(hist.clone());
        }

        // Add the new User Message
        state.messages.push(AgentMessage {
            role: "user".to_string(),
            content: AgentContent::text(input),
            tool_calls: None,
            tool_call_id: None,
        });

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
        // 3. Execute Workflow
        let final_state = workflow
            .execute(state, self.skills.clone(), on_event)
            .await
            .map_err(|e| format!("Workflow execution failed: {}", e))?;

        // 4. Update History and Return Result
        // We take the DIFFERENCE between final state messages and original history + 1 (user msg)
        // Actually, let's just grab the last message content if it's from assistant
        let last_message = final_state.messages.last().ok_or("No response generated")?;

        // Update persistent history
        {
            let mut hist = self.history.lock().await;
            // Append the User Message (input) first
            hist.push(AgentMessage {
                role: "user".to_string(),
                content: AgentContent::text(input),
                tool_calls: None,
                tool_call_id: None,
            });

            // Append all NEW messages generated during workflow
            // Note: simple approach is to append everything after the user message we just added
            // But state.messages includes system + old history + new user msg + new agent msgs...
            // So we skip (1 + old_history_len + 1)
            // Let's refine:
            // The state started with: System + History + User Input.
            // Any message AFTER that index is new.

            // To be safe, let's just grab the new assistant/tool messages.
            // We know the input was the last "user" message added.

            // Actually, simpler:
            // We pushed `input` to `hist` above.
            // Now we push the RESPONSES.

            // Find messages after the last "user" message that matches our input?
            // Or just iterate from the known start index?

            // Let's iterate final_state.messages.
            // Only keep messages that are NOT in the initial set.
            // We know we added System + Old History + User Input.
            // So we skip `1 + old_hist_len + 1`.

            // We can calculate offset:
            // 1 (System)
            // + (final_state.messages.len() - new_generated_count) ?? No

            // Better: state.messages indices.
            // Index 0: System
            // Index 1..N: Old History
            // Index N+1: User Input
            // Index N+2...: New Responses

            // We need to know N (Old History Length).
            // We can get it from locking history again, but it might have changed (race condition? unlikely here).
            // `hist` lock held above is different scope.

            // Let's assume we are the only writer for this session.
            // We can match by content or just append the last few.

            // For MVP: Just check the last message. If it's assistant, return it.
            // And append `last_message` to history?
            // BUT: What if there were multiple tool calls? We need to save the whole chain in history.

            // Correct approach:
            // Identify all messages *after* the User Input we added to state.
            let user_msg_index = final_state
                .messages
                .iter()
                .rposition(|m| m.role == "user")
                .unwrap();

            for msg in final_state.messages.iter().skip(user_msg_index + 1) {
                hist.push(msg.clone());
            }
        }

        if last_message.role == "assistant" {
            Ok(last_message.content.as_text())
        } else {
            // Workflow ended on a Tool output or something?
            // Usually ThinkStep (Assistant) is the last one.
            Ok("Workflow completed without final response".to_string())
        }
    }
}
