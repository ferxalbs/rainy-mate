use crate::ai::router::IntelligentRouter;
use crate::models::neural::{
    AirlockLevel, CommandPriority, CommandResult, CommandStatus, QueuedCommand, RainyPayload,
};
use crate::services::SkillExecutor;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
    pub instructions: String,
    pub workspace_id: String,
    // Future: tools list, memory settings
}

/// The core runtime that orchestrates the agent's thinking process
pub struct AgentRuntime {
    config: AgentConfig,
    router: Arc<tokio::sync::RwLock<IntelligentRouter>>,
    skills: Arc<SkillExecutor>,
    history: Arc<Mutex<Vec<AgentMessage>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<crate::ai::provider_types::ToolCall>>,
    pub tool_call_id: Option<String>,
}

impl AgentRuntime {
    pub fn new(
        config: AgentConfig,
        router: Arc<tokio::sync::RwLock<IntelligentRouter>>,
        skills: Arc<SkillExecutor>,
    ) -> Self {
        Self {
            config,
            router,
            skills,
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Primary entry point: Run a workflow/turn
    pub async fn run(&self, input: &str) -> Result<String, String> {
        // 1. Add User Message
        {
            let mut hist = self.history.lock().await;
            hist.push(AgentMessage {
                role: "user".to_string(),
                content: input.to_string(),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let max_turns = 10;
        let mut current_turn = 0;

        loop {
            if current_turn >= max_turns {
                return Err("Maximum conversation turns exceeded".to_string());
            }
            current_turn += 1;

            // 2. Prepare Request for Router
            let mut messages: Vec<crate::ai::provider_types::ChatMessage> = Vec::new();

            // Add System Message
            messages.push(crate::ai::provider_types::ChatMessage {
                role: "system".to_string(),
                content: self.config.instructions.clone(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });

            {
                let hist = self.history.lock().await;
                messages.extend(hist.iter().map(|m| crate::ai::provider_types::ChatMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                    name: None,
                    tool_calls: m.tool_calls.clone(),
                    tool_call_id: m.tool_call_id.clone(),
                }));
            }

            let tools = self.skills.get_tool_definitions();
            let has_tools = !tools.is_empty();

            let request = crate::ai::provider_types::ChatCompletionRequest {
                model: self.config.model.clone(),
                messages,
                temperature: Some(0.7),
                max_tokens: None,
                top_p: None,
                stream: false,
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
            };

            // 3. Call Router
            let router_guard: tokio::sync::RwLockReadGuard<IntelligentRouter> =
                self.router.read().await;

            let response: crate::ai::provider_types::ChatCompletionResponse = router_guard
                .complete(request)
                .await
                .map_err(|e| format!("Router execution failed: {}", e))?;

            // 4. Update History
            let assistant_content = response.content.clone().unwrap_or_default();
            let tool_calls = response.tool_calls.clone();

            {
                let mut hist = self.history.lock().await;
                hist.push(AgentMessage {
                    role: "assistant".to_string(),
                    content: assistant_content.clone(),
                    tool_calls: tool_calls.clone(),
                    tool_call_id: None,
                });
            }

            // 5. Handle Tool Calls
            if let Some(calls) = tool_calls {
                if calls.is_empty() {
                    // No tool calls, we are done
                    return Ok(assistant_content);
                }

                for call in calls {
                    let function_name = call.function.name;
                    let arguments_str = call.function.arguments;

                    println!(
                        "Executing tool: {} with args: {}",
                        function_name, arguments_str
                    );

                    let params: serde_json::Value = serde_json::from_str(&arguments_str)
                        .map_err(|e| format!("Failed to parse tool arguments: {}", e))?;

                    // Map tool name to SkillExecutor skill/method
                    // Currently assume all are filesystem
                    let skill = "filesystem";
                    let method = match function_name.as_str() {
                        "read_file" => "read_file",
                        "write_file" => "write_file",
                        "list_files" => "list_files",
                        "search_files" => "search_files",
                        _ => return Err(format!("Unknown tool: {}", function_name)),
                    };

                    let command = QueuedCommand {
                        id: uuid::Uuid::new_v4().to_string(),
                        intent: format!("{}.{}", skill, method),
                        payload: RainyPayload {
                            skill: Some(skill.to_string()),
                            method: Some(method.to_string()),
                            params: Some(params),
                            content: None,
                            allowed_paths: vec![self.config.workspace_id.clone()],
                        },
                        status: CommandStatus::Pending,
                        priority: CommandPriority::Normal,
                        airlock_level: AirlockLevel::Safe,
                        created_at: Some(Utc::now().timestamp()),
                        started_at: None,
                        completed_at: None,
                        result: None,
                        workspace_id: Some(self.config.workspace_id.clone()),
                        desktop_node_id: None,
                        approved_by: None,
                    };

                    let result = self.skills.execute(&command).await;

                    let tool_output = if result.success {
                        result
                            .output
                            .unwrap_or_else(|| "Tool executed successfully".to_string())
                    } else {
                        format!(
                            "Tool execution failed: {}",
                            result.error.unwrap_or_else(|| "Unknown error".to_string())
                        )
                    };

                    // Add Tool Result to History
                    {
                        let mut hist = self.history.lock().await;
                        hist.push(AgentMessage {
                            role: "tool".to_string(),
                            content: tool_output,
                            tool_calls: None,
                            tool_call_id: Some(call.id),
                        });
                    }
                }
                // Loop continues to feed tool outputs back to LLM
            } else {
                // No tool calls, we are done
                return Ok(assistant_content);
            }
        }
    }
}
