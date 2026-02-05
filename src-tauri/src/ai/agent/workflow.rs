use crate::ai::agent::memory::AgentMemory;
use crate::ai::agent::runtime::{AgentConfig, AgentMessage};
use crate::ai::router::IntelligentRouter;
use crate::models::neural::{
    AirlockLevel, CommandPriority, CommandStatus, QueuedCommand, RainyPayload,
};
use crate::services::SkillExecutor;
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared state passed between workflow steps
#[derive(Clone, Debug)]
pub struct AgentState {
    pub messages: Vec<AgentMessage>,
    #[allow(dead_code)] // @TODO Future context sharing
    pub context: HashMap<String, String>,
    pub workspace_id: String,
    pub memory: Arc<AgentMemory>,
}

impl AgentState {
    pub fn new(workspace_id: String) -> Self {
        Self {
            messages: Vec::new(),
            context: HashMap::new(),
            workspace_id,
            memory: Arc::new(AgentMemory::new()),
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
    ) -> Result<StepResult, String>;
}

/// The Workflow Graph Container
pub struct Workflow {
    #[allow(dead_code)] // @TODO Configuration usage in steps
    pub config: AgentConfig,
    pub steps: HashMap<String, Box<dyn WorkflowStep>>,
    pub start_step: String,
}

impl Workflow {
    pub fn new(config: AgentConfig, start_step: String) -> Self {
        Self {
            config,
            steps: HashMap::new(),
            start_step,
        }
    }

    pub fn add_step(&mut self, step: Box<dyn WorkflowStep>) {
        self.steps.insert(step.id(), step);
    }

    pub async fn execute(
        &self,
        initial_state: AgentState,
        skills: Arc<SkillExecutor>,
    ) -> Result<AgentState, String> {
        let mut state = initial_state;
        let mut current_step_id = Some(self.start_step.clone());
        let mut steps_count = 0;
        const MAX_STEPS: usize = 50;

        while let Some(step_id) = current_step_id {
            if steps_count >= MAX_STEPS {
                return Err("Workflow execution exceeded maximum step count".to_string());
            }

            let step = self
                .steps
                .get(&step_id)
                .ok_or(format!("Step not found: {}", step_id))?;

            // Execute the step
            // We pass a clone of skills for now. State is mutable.
            let result = step.execute(&mut state, skills.clone()).await?;

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
    ) -> Result<StepResult, String> {
        // 1. Prepare messages
        // 1. Prepare messages
        let mut messages: Vec<crate::ai::provider_types::ChatMessage> = state
            .messages
            .iter()
            .map(|m| crate::ai::provider_types::ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                name: None,
                tool_calls: m.tool_calls.clone(),
                tool_call_id: m.tool_call_id.clone(),
            })
            .collect();

        // 1.5. Inject Memory Context (RAG)
        if let Some(last_user_msg) = state.messages.iter().rfind(|m| m.role == "user") {
            let hits = state.memory.retrieve(&last_user_msg.content).await;
            if !hits.is_empty() {
                let ctx = hits
                    .iter()
                    .map(|h| format!("- {}", h.content))
                    .collect::<Vec<_>>()
                    .join("\n");

                let system_ctx = format!(
                    "Retrieved Memory Context:\n{}\n\nUse this context to answer the user's request if relevant.",
                    ctx
                );

                // Insert at the beginning as a system message (or append if system exists)
                // For now, simpler to just prepend
                messages.insert(
                    0,
                    crate::ai::provider_types::ChatMessage {
                        role: "system".to_string(),
                        content: system_ctx,
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    },
                );
            }
        }

        // 2. Prepare tools
        let tools = skills.get_tool_definitions();
        let has_tools = !tools.is_empty();

        let request = crate::ai::provider_types::ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: None,
            top_p: None,
            stream: false,
            tools: if has_tools { Some(tools) } else { None }, // Send tools to LLM
            tool_choice: if has_tools {
                Some(crate::ai::provider_types::ToolChoice::Auto)
            } else {
                None
            },
            json_mode: false,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
        };

        // 3. Call Router
        let router_guard = self.router.read().await;
        // Basic error handling for now, can improve retry logic here later
        let response = router_guard
            .complete(request)
            .await
            .map_err(|e| format!("ThinkStep Failed: {}", e))?;

        // 4. Update State
        let assistant_content = response.content.clone().unwrap_or_default();
        let tool_calls = response.tool_calls.clone();

        state.messages.push(AgentMessage {
            role: "assistant".to_string(),
            content: assistant_content.clone(),
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

#[derive(Debug)]
pub struct ActStep;

#[async_trait::async_trait]
impl WorkflowStep for ActStep {
    fn id(&self) -> String {
        "act".to_string()
    }

    async fn execute(
        &self,
        state: &mut AgentState,
        skills: Arc<SkillExecutor>,
    ) -> Result<StepResult, String> {
        // Find the last assistant message with tool calls
        let last_msg = state.messages.last().ok_or("No messages in state")?;

        let tool_calls = match &last_msg.tool_calls {
            Some(calls) if !calls.is_empty() => calls.clone(),
            _ => {
                return Ok(StepResult {
                    next_step: Some("think".to_string()), // Should not happen but fallback
                    success: true,
                    output: Some("No tool calls to execute".to_string()),
                });
            }
        };

        let mut results = Vec::new();

        for call in tool_calls {
            let function_name = call.function.name.clone();
            let arguments_str = call.function.arguments.clone();

            let params: serde_json::Value = serde_json::from_str(&arguments_str)
                .map_err(|e| format!("Failed to parse args: {}", e))?;

            // Simple mapping for now
            let skill = "filesystem";
            let method = function_name.as_str(); // e.g. "read_file"

            let command = QueuedCommand {
                id: uuid::Uuid::new_v4().to_string(),
                intent: format!("{}.{}", skill, method),
                payload: RainyPayload {
                    skill: Some(skill.to_string()),
                    method: Some(method.to_string()),
                    params: Some(params),
                    content: None,
                    allowed_paths: vec![state.workspace_id.clone()],
                },
                status: CommandStatus::Pending,
                priority: CommandPriority::Normal,
                airlock_level: AirlockLevel::Safe,
                created_at: Some(Utc::now().timestamp()),
                started_at: None,
                completed_at: None,
                result: None,
                workspace_id: Some(state.workspace_id.clone()),
                desktop_node_id: None,
                approved_by: None,
            };

            let result = skills.execute(&command).await;

            let output = if result.success {
                result.output.unwrap_or_default()
            } else {
                format!(
                    "Error: {}",
                    result.error.unwrap_or_else(|| "Unknown".to_string())
                )
            };

            results.push(AgentMessage {
                role: "tool".to_string(),
                content: output,
                tool_calls: None,
                tool_call_id: Some(call.id.clone()),
            });
        }

        // Update state with all tool outputs
        state.messages.extend(results);

        // Loop back to Think
        Ok(StepResult {
            next_step: Some("think".to_string()),
            success: true,
            output: Some("Executed tools".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::AIProviderManager;
    use crate::services::workspace::WorkspaceManager;
    use crate::services::{ManagedResearchService, WebResearchService};

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
    async fn test_workflow_execution() {
        let config = AgentConfig {
            name: "Test Agent".to_string(),
            model: "test-model".to_string(),
            instructions: "test".to_string(),
            workspace_id: "test-ws".to_string(),
            max_steps: Some(10),
        };

        let mut workflow = Workflow::new(config, "start".to_string());

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

        let state = AgentState::new("test-ws".to_string());

        // Note: This relies on being able to create WorkspaceManager in test env.
        match WorkspaceManager::new() {
            Ok(wm) => {
                let provider_manager = Arc::new(AIProviderManager::new());
                let managed_research = Arc::new(ManagedResearchService::new(provider_manager));
                let web_research = Arc::new(WebResearchService::new());
                let skills = Arc::new(SkillExecutor::new(
                    Arc::new(wm),
                    managed_research,
                    web_research,
                ));
                let result = workflow.execute(state, skills).await;
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
