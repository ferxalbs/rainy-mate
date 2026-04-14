// Act step — tool execution phase of the ReAct loop.
// Extracted from workflow.rs to keep module size bounded (<400 lines).
use crate::ai::agent::events::AgentEvent;
use crate::ai::agent::runtime::{AgentContent, AgentMessage};
use crate::ai::agent::workflow::{
    is_tool_allowed_by_spec, truncate_to_max_bytes, AgentState, StepResult, WorkflowStep,
    CANCELLED_RUN_MESSAGE,
};
use crate::ai::specs::manifest::AgentSpec;
use crate::models::neural::{
    AirlockLevel, CommandPriority, CommandStatus, QueuedCommand, RainyPayload,
};
use crate::services::{get_tool_policy, SkillExecutor};
use chrono::Utc;
use std::sync::Arc;

const MAX_TOOL_TEXT_BYTES: usize = 48 * 1024;

fn is_image_data_uri(s: &str) -> bool {
    s.starts_with("data:image/") && s.contains("base64,")
}

/// Convert tool output to appropriate AgentContent
fn tool_output_to_content(output: String) -> AgentContent {
    if is_image_data_uri(&output) {
        AgentContent::image(output)
    } else {
        // Screenshot tool currently returns JSON with a huge data_uri payload.
        // Keep only metadata in the model context to avoid >100KB message failures.
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
            if json.get("data_uri").and_then(|v| v.as_str()).is_some() {
                let width = json.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
                let height = json.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                return AgentContent::text(format!(
                    "Screenshot captured successfully ({}x{}).",
                    width, height
                ));
            }
        }
        AgentContent::text(truncate_to_max_bytes(&output, MAX_TOOL_TEXT_BYTES))
    }
}

fn resolve_airlock_level_for_tool(spec: &AgentSpec, tool_name: &str) -> AirlockLevel {
    if let Some(level) = spec.airlock.tool_levels.get(tool_name) {
        return match (*level).clamp(0, 2) {
            0 => AirlockLevel::Safe,
            1 => AirlockLevel::Sensitive,
            _ => AirlockLevel::Dangerous,
        };
    }
    if crate::services::mcp_service::McpService::is_mcp_tool(tool_name) {
        return AirlockLevel::Safe;
    }
    get_tool_policy(tool_name)
        .map(|policy| policy.airlock_level)
        .unwrap_or(AirlockLevel::Dangerous)
}

fn build_command_for_tool_call(
    state: &AgentState,
    skill: &str,
    method_str: &str,
    params: serde_json::Value,
    airlock_level: AirlockLevel,
) -> QueuedCommand {
    QueuedCommand {
        id: uuid::Uuid::new_v4().to_string(),
        intent: format!("{}.{}", skill, method_str),
        payload: RainyPayload {
            skill: Some(skill.to_string()),
            method: Some(method_str.to_string()),
            params: Some(params),
            content: None,
            allowed_paths: state.allowed_paths.clone(),
            blocked_paths: state.spec.airlock.scopes.blocked_paths.clone(),
            allowed_domains: state.spec.airlock.scopes.allowed_domains.clone(),
            blocked_domains: state.spec.airlock.scopes.blocked_domains.clone(),
            tool_access_policy: Some(state.tool_access_policy.clone()),
            tool_access_policy_version: None,
            tool_access_policy_hash: None,
            ..Default::default()
        },
        status: CommandStatus::Pending,
        priority: CommandPriority::Normal,
        airlock_level,
        approval_timeout_secs: Some(0),
        created_at: Some(Utc::now().timestamp()),
        started_at: None,
        completed_at: None,
        result: None,
        workspace_id: Some(state.workspace_id.clone()),
        desktop_node_id: None,
        approved_by: None,
        schema_version: None,
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
        on_event: Box<dyn Fn(AgentEvent) + Send + Sync>,
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
            if state
                .kill_switch
                .as_ref()
                .is_some_and(|switch| switch.is_triggered())
            {
                on_event(AgentEvent::Status(CANCELLED_RUN_MESSAGE.to_string()));
                return Err(CANCELLED_RUN_MESSAGE.to_string());
            }

            let function_name = call.function.name.clone();
            let arguments_str = call.function.arguments.clone();

            let params: serde_json::Value = serde_json::from_str(&arguments_str)
                .map_err(|e| format!("Failed to parse args: {}", e))?;

            if !is_tool_allowed_by_spec(state.spec.as_ref(), &function_name) {
                let blocked_msg =
                    format!("Tool '{}' blocked by agent Airlock policy", function_name);
                on_event(AgentEvent::ToolResult {
                    id: call.id.clone(),
                    result: blocked_msg.clone(),
                });
                results.push(AgentMessage {
                    role: "tool".to_string(),
                    content: AgentContent::text(blocked_msg),
                    tool_calls: None,
                    tool_call_id: Some(call.id.clone()),
                });
                continue;
            }

            // Resolve the tool's skill/method routing.
            // First try the static built-in policy map; if not found, look in the
            // third-party Wasm skill registry. This makes Wasm skills fully first-class
            // citizens in the agent chat loop.
            let (skill, method_str, airlock_level) =
                if crate::services::mcp_service::McpService::is_mcp_tool(&function_name) {
                    let level = resolve_airlock_level_for_tool(state.spec.as_ref(), &function_name);
                    ("mcp".to_string(), function_name.clone(), level)
                } else if let Some(policy) = get_tool_policy(&function_name) {
                    let level = resolve_airlock_level_for_tool(state.spec.as_ref(), &function_name);
                    (
                        policy.skill.as_str().to_string(),
                        function_name.clone(),
                        level,
                    )
                } else {
                    // Check if it's a registered third-party Wasm skill method.
                    // Instantiate registry once and reuse for both the airlock check and skill_id lookup.
                    let wasm_registry = crate::services::ThirdPartySkillRegistry::new().ok();
                    let registry_check = wasm_registry.as_ref().and_then(|reg| {
                        reg.find_method_airlock_level(&function_name).ok().flatten()
                    });

                    let Some(wasm_airlock) = registry_check else {
                        let blocked_msg = format!(
                            "Tool '{}' blocked: no explicit policy entry (fail-closed)",
                            function_name
                        );
                        on_event(AgentEvent::ToolResult {
                            id: call.id.clone(),
                            result: blocked_msg.clone(),
                        });
                        results.push(AgentMessage {
                            role: "tool".to_string(),
                            content: AgentContent::text(blocked_msg),
                            tool_calls: None,
                            tool_call_id: Some(call.id.clone()),
                        });
                        continue;
                    };

                    // For Wasm skills the skill_id is derived from the registry (reuse instance above).
                    // `execute_third_party_skill` looks up the skill by (skill_id, method_name).
                    let skill_id = wasm_registry
                        .and_then(|reg| reg.list_skills().ok())
                        .and_then(|skills| {
                            skills.into_iter().find(|s| {
                                s.enabled && s.methods.iter().any(|m| m.name == function_name)
                            })
                        })
                        .map(|s| s.id)
                        .unwrap_or_else(|| function_name.clone());

                    let level = resolve_airlock_level_for_tool(state.spec.as_ref(), &function_name);
                    let effective = if level > wasm_airlock {
                        level
                    } else {
                        wasm_airlock
                    };
                    (skill_id, function_name.clone(), effective)
                };

            on_event(AgentEvent::Status(format!(
                "Executing tool: {}",
                function_name
            )));
            let mut call_with_level = call.clone();
            call_with_level.airlock_level = Some(airlock_level);
            on_event(AgentEvent::ToolCall(call_with_level));

            let command =
                build_command_for_tool_call(state, &skill, &method_str, params, airlock_level);

            // Enforce Airlock for local agent tool execution as well as cloud-dispatched commands.
            if let Some(airlock) = state.airlock_service.as_ref() {
                on_event(AgentEvent::Status(format!(
                    "Awaiting Airlock approval for {}",
                    function_name
                )));
                match airlock.check_permission(&command).await {
                    Ok(true) => {}
                    Ok(false) => {
                        let blocked_msg = format!(
                            "Tool '{}' blocked by Airlock policy or user decision",
                            function_name
                        );
                        on_event(AgentEvent::ToolResult {
                            id: call.id.clone(),
                            result: blocked_msg.clone(),
                        });
                        results.push(AgentMessage {
                            role: "tool".to_string(),
                            content: AgentContent::text(blocked_msg),
                            tool_calls: None,
                            tool_call_id: Some(call.id.clone()),
                        });
                        continue;
                    }
                    Err(e) => {
                        let blocked_msg =
                            format!("Tool '{}' blocked by Airlock error: {}", function_name, e);
                        on_event(AgentEvent::ToolResult {
                            id: call.id.clone(),
                            result: blocked_msg.clone(),
                        });
                        results.push(AgentMessage {
                            role: "tool".to_string(),
                            content: AgentContent::text(blocked_msg),
                            tool_calls: None,
                            tool_call_id: Some(call.id.clone()),
                        });
                        continue;
                    }
                }
            }

            // Implement Auto-Retry Logic.
            // Only retry L0 (read-only) tools — destructive (L1/L2) operations must not
            // be retried as a double-write or double-delete would corrupt state.
            let effective_max_retries = if airlock_level == AirlockLevel::Safe {
                MAX_RETRIES
            } else {
                0
            };
            let mut attempts = 0;
            const MAX_RETRIES: u32 = 2;
            let mut final_output = String::new();

            while attempts <= effective_max_retries {
                if state
                    .kill_switch
                    .as_ref()
                    .is_some_and(|switch| switch.is_triggered())
                {
                    on_event(AgentEvent::Status(CANCELLED_RUN_MESSAGE.to_string()));
                    return Err(CANCELLED_RUN_MESSAGE.to_string());
                }

                let result = skills.execute(&command).await;

                if result.success {
                    final_output = result.output.unwrap_or_default();
                    break;
                } else {
                    let err = result.error.unwrap_or_else(|| "Unknown error".to_string());

                    if attempts == effective_max_retries {
                        final_output = format!("Error: {}", err);
                    } else {
                        // Exponential backoff
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            500 * (attempts as u64 + 1),
                        ))
                        .await;
                    }
                    attempts += 1;
                }
            }

            on_event(AgentEvent::ToolResult {
                id: call.id.clone(),
                result: final_output.clone(),
            });

            // Convert tool output to proper multimodal content if it's an image
            let content = tool_output_to_content(final_output.clone());

            results.push(AgentMessage {
                role: "tool".to_string(),
                content,
                tool_calls: None,
                tool_call_id: Some(call.id.clone()),
            });

            // Persist web research results to long-term memory
            if matches!(function_name.as_str(), "web_search" | "read_web_page") {
                if state.spec.memory_config.persistence.cross_session {
                    let content_preview: String = final_output.chars().take(2000).collect();
                    if !content_preview.is_empty() {
                        state
                            .memory
                            .push_for_distillation(
                                crate::services::memory_vault::types::RawMemoryTurn {
                                    content: content_preview,
                                    role: "tool_result".to_string(),
                                    source: format!("tool:{}", function_name),
                                    workspace_id: state.workspace_id.clone(),
                                    timestamp: chrono::Utc::now().timestamp(),
                                },
                            )
                            .await;
                    }
                }
            }
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
    use super::build_command_for_tool_call;
    use crate::ai::agent::memory::AgentMemory;
    use crate::ai::agent::runtime::AgentMessage;
    use crate::ai::agent::workflow::AgentState;
    use crate::ai::specs::manifest::AgentSpec;
    use crate::models::neural::{AirlockLevel, ToolAccessPolicy};
    use serial_test::serial;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[tokio::test]
    #[serial]
    async fn queued_command_carries_agent_tool_access_policy() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let memory_manager = Arc::new(
            crate::services::MemoryManager::new(10, temp_dir.path().to_path_buf())
        );
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

        let policy = ToolAccessPolicy {
            enabled: true,
            mode: "all".to_string(),
            allow: Vec::new(),
            deny: Vec::new(),
        };

        let mut state = AgentState::new(
            "test-ws".to_string(),
            vec!["/tmp/test-ws".to_string()],
            policy.clone(),
            memory,
            Arc::new(AgentSpec::default()),
            Arc::new(None),
            None,
        );
        state.messages = vec![AgentMessage {
            role: "assistant".to_string(),
            content: crate::ai::agent::runtime::AgentContent::text("call tool".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];
        state.context = HashMap::new();

        let command = build_command_for_tool_call(
            &state,
            "shell",
            "execute_command",
            serde_json::json!({"command":"git pull"}),
            AirlockLevel::Dangerous,
        );

        assert_eq!(command.intent, "shell.execute_command");
        assert_eq!(
            command
                .payload
                .tool_access_policy
                .as_ref()
                .map(|value| value.mode.as_str()),
            Some("all")
        );
        assert_eq!(
            command
                .payload
                .tool_access_policy
                .as_ref()
                .map(|value| value.deny.len()),
            Some(0)
        );
        assert_eq!(command.airlock_level, AirlockLevel::Dangerous);
    }
}
