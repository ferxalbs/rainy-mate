use crate::ai::agent::runtime::{AgentContent, AgentMessage, AgentRuntime, RuntimeOptions};
use crate::ai::specs::AgentSpec;
use crate::commands::router::IntelligentRouterState;
use crate::services::SkillExecutor;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

const MAX_HISTORY_MESSAGES: usize = 30;
const MAX_HISTORY_MESSAGE_CHARS: usize = 4000;

fn truncate_text(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let out: String = input.chars().take(max_chars).collect();
    format!("{}\n\n[TRUNCATED]", out)
}

fn build_runtime_history(rows: Vec<(String, String, String)>) -> Vec<AgentMessage> {
    let mut selected: Vec<AgentMessage> = rows
        .into_iter()
        .filter_map(|(_, role, content)| {
            if role != "user" && role != "assistant" {
                return None;
            }
            Some(AgentMessage {
                role,
                content: AgentContent::text(truncate_text(&content, MAX_HISTORY_MESSAGE_CHARS)),
                tool_calls: None,
                tool_call_id: None,
            })
        })
        .collect();

    if selected.len() > MAX_HISTORY_MESSAGES {
        let skip = selected.len() - MAX_HISTORY_MESSAGES;
        selected = selected.into_iter().skip(skip).collect();
    }

    selected
}

fn default_instructions(workspace_id: &str) -> String {
    format!(
        "You are Rainy Agent, an autonomous AI assistant capable of performing complex tasks in the workspace.
        
        Workspace Path: {}
        
        CAPABILITIES:
        - You can read, write, list, and search files in the workspace.
        - **MULTIMODAL: You can SEE images.** If you use `read_file` on an image, you will receive its visual content.
        - You can plan multi-step tasks.
        
        GUIDELINES:
        1. PLAN: Before executing, briefly state your plan.
        2. EXECUTE: Use the provided tools to carry out the plan.
        3. VERIFY: After critical operations, verify the result (e.g., read_file after write_file).
        
        Tools are provided natively. Use them for all file operations.
        Do not hallucinate file contents. trust the tool outputs.
        If a tool fails, analyze the error and try a different approach.",
        workspace_id
    )
}

#[tauri::command]
pub async fn run_agent_workflow(
    app_handle: tauri::AppHandle,
    prompt: String,
    model_id: String,
    workspace_id: String,
    agent_spec_id: Option<String>,
    router: State<'_, IntelligentRouterState>,
    skills: State<'_, Arc<SkillExecutor>>,
    agent_manager: State<'_, crate::ai::agent::manager::AgentManager>,
) -> Result<String, String> {
    // 0. Ensure Chat Session Exists (Persist Metadata)
    // We use workspace_id as the chat_id for this simple implementation
    let chat_id = workspace_id.clone();
    let _ = agent_manager
        .ensure_chat_session(&chat_id, "Rainy Agent")
        .await
        .map_err(|e| format!("Failed to initialize chat session: {}", e))?;

    // 1. Initialize Runtime (Ephemeral for now, persistent later)
    // 1. Initialize Runtime (Ephemeral for now, persistent later)
    let spec = if let Some(spec_id) = agent_spec_id {
        match agent_manager.get_agent_spec(&spec_id).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                eprintln!(
                    "[AgentWorkflow] Spec {} not found in DB, falling back to default",
                    spec_id
                );
                // Fallback
                use crate::ai::specs::skills::AgentSkills;
                use crate::ai::specs::soul::AgentSoul;
                AgentSpec {
                    id: "default".to_string(),
                    version: "2.0.0".to_string(),
                    soul: AgentSoul {
                        name: "Rainy Agent".to_string(),
                        description: "Default fallback agent".to_string(),
                        soul_content: default_instructions(&workspace_id),
                        ..Default::default()
                    },
                    skills: AgentSkills::default(),
                    memory_config: Default::default(),
                    connectors: Default::default(),
                    signature: None,
                }
            }
            Err(e) => {
                eprintln!("[AgentWorkflow] Failed to load spec {}: {}", spec_id, e);
                // Fallback
                use crate::ai::specs::skills::AgentSkills;
                use crate::ai::specs::soul::AgentSoul;
                AgentSpec {
                    id: "default".to_string(),
                    version: "2.0.0".to_string(),
                    soul: AgentSoul {
                        name: "Rainy Agent".to_string(),
                        description: "Default fallback agent".to_string(),
                        soul_content: default_instructions(&workspace_id),
                        ..Default::default()
                    },
                    skills: AgentSkills::default(),
                    memory_config: Default::default(),
                    connectors: Default::default(),
                    signature: None,
                }
            }
        }
    } else {
        use crate::ai::specs::skills::AgentSkills;
        use crate::ai::specs::soul::AgentSoul;
        AgentSpec {
            id: "default".to_string(),
            version: "2.0.0".to_string(),
            soul: AgentSoul {
                name: "Rainy Agent".to_string(),
                description: "Default agent".to_string(),
                soul_content: default_instructions(&workspace_id),
                ..Default::default()
            },
            skills: AgentSkills::default(),
            memory_config: Default::default(),
            connectors: Default::default(),
            signature: None,
        }
    };

    let options = RuntimeOptions {
        model: Some(model_id),
        workspace_id: workspace_id.clone(),
        max_steps: None,
    };

    // Initialize Persistent Memory
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let memory =
        Arc::new(crate::ai::agent::memory::AgentMemory::new(&workspace_id, app_data_dir).await);

    let runtime = AgentRuntime::new(
        spec,
        options,
        router.0.clone(),
        skills.inner().clone(),
        memory,
    );

    // Load persisted conversation history into runtime so local Native Runtime
    // preserves context across turns.
    let history_rows = agent_manager
        .get_history(&chat_id)
        .await
        .map_err(|e| format!("Failed to load chat history: {}", e))?;
    runtime
        .set_history(build_runtime_history(history_rows))
        .await;

    // 2. Run Workflow with Persistence
    let app_handle_clone = app_handle.clone();

    // Persist Initial User Prompt
    let _ = agent_manager
        .save_message(&chat_id, "user", &prompt)
        .await
        .map_err(|e| format!("Failed to save user message: {}", e))?;

    let response = runtime
        .run(&prompt, move |event| {
            // Emit to frontend
            let _ = app_handle_clone.emit("agent://event", event.clone());
        })
        .await?;

    // Persist final assistant response only (avoid noisy intermediate event spam).
    let _ = agent_manager
        .save_message(&chat_id, "assistant", &response)
        .await
        .map_err(|e| format!("Failed to save assistant message: {}", e))?;

    Ok(response)
}
