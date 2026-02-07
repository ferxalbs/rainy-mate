use crate::ai::agent::runtime::{AgentConfig, AgentRuntime};
use crate::commands::router::IntelligentRouterState;
use crate::services::SkillExecutor;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

#[tauri::command]
pub async fn run_agent_workflow(
    app_handle: tauri::AppHandle,
    prompt: String,
    model_id: String,
    workspace_id: String,
    router: State<'_, IntelligentRouterState>,
    skills: State<'_, Arc<SkillExecutor>>,
) -> Result<String, String> {
    // 1. Initialize Runtime (Ephemeral for now, persistent later)
    let config = AgentConfig {
        name: "Rainy Agent".to_string(),
        model: model_id,
        instructions: format!(
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
        ),
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

    let runtime = AgentRuntime::new(config, router.0.clone(), skills.inner().clone(), memory);

    // 2. Run Workflow
    // For MVP, this just echoes or does a basic LLM call if wired
    let app_handle_clone = app_handle.clone();
    let response = runtime
        .run(&prompt, move |event| {
            let _ = app_handle_clone.emit("agent://event", event);
        })
        .await?;

    Ok(response)
}
