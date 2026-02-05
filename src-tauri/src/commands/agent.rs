use crate::ai::agent::runtime::{AgentConfig, AgentRuntime};
use crate::commands::router::IntelligentRouterState;
use crate::services::SkillExecutor;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn run_agent_workflow(
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
        workspace_id,
    };

    let runtime = AgentRuntime::new(config, router.0.clone(), skills.inner().clone());

    // 2. Run Workflow
    // For MVP, this just echoes or does a basic LLM call if wired
    let response = runtime.run(&prompt).await?;

    Ok(response)
}
