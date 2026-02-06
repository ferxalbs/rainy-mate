use crate::ai::agent::memory::AgentMemory;
use crate::ai::agent::runtime::{AgentConfig, AgentRuntime};
use crate::ai::router::IntelligentRouter;
use crate::models::neural::CommandResult;
use crate::services::airlock::AirlockService;
use crate::services::neural_service::NeuralService;
use crate::services::skill_executor::SkillExecutor;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Context needed to create AgentRuntime instances on-demand
pub struct AgentRuntimeContext {
    pub router: Arc<RwLock<IntelligentRouter>>,
    pub app_data_dir: PathBuf,
}

#[derive(Clone)]
pub struct CommandPoller {
    neural_service: NeuralService,
    skill_executor: Arc<SkillExecutor>,
    agent_context: Arc<RwLock<Option<AgentRuntimeContext>>>,
    is_running: Arc<Mutex<bool>>,
    airlock_service: Arc<RwLock<Option<AirlockService>>>,
}

impl CommandPoller {
    pub fn new(neural_service: NeuralService, skill_executor: Arc<SkillExecutor>) -> Self {
        Self {
            neural_service,
            skill_executor,
            agent_context: Arc::new(RwLock::new(None)),
            is_running: Arc::new(Mutex::new(false)),
            airlock_service: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the context needed to create AgentRuntime instances
    pub async fn set_agent_context(
        &self,
        router: Arc<RwLock<IntelligentRouter>>,
        app_data_dir: PathBuf,
    ) {
        let mut lock = self.agent_context.write().await;
        *lock = Some(AgentRuntimeContext {
            router,
            app_data_dir,
        });
    }

    pub async fn set_airlock_service(&self, service: AirlockService) {
        let mut lock = self.airlock_service.write().await;
        *lock = Some(service);
    }

    pub async fn start(&self) {
        let mut running = self.is_running.lock().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let poller = self.clone();
        tokio::spawn(async move {
            println!("[CommandPoller] Started polling loop via NeuralService");

            while *poller.is_running.lock().await {
                if let Err(e) = poller.poll_and_execute().await {
                    eprintln!("[CommandPoller] Error: {}", e);
                }
                sleep(POLL_INTERVAL).await;
            }

            println!("[CommandPoller] Stopped polling loop");
        });
    }

    #[allow(dead_code)]
    pub async fn stop(&self) {
        let mut running = self.is_running.lock().await;
        *running = false;
    }

    async fn poll_and_execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Send heartbeat and get commands via NeuralService
        if !self.neural_service.has_credentials().await {
            return Ok(()); // Silently skip if not authenticated
        }

        // Check if node is registered (has node_id)
        if !self.neural_service.is_registered().await {
            // Node not registered yet - silently skip to avoid log spam
            return Ok(());
        }

        let pending_commands_result: Result<Vec<crate::models::neural::QueuedCommand>, String> =
            self.neural_service
                .heartbeat(crate::models::neural::DesktopNodeStatus::Online)
                .await;

        let commands = match pending_commands_result {
            Ok(cmds) => cmds,
            Err(e) => {
                // Only log if it's not a "Node not registered" error (already handled above)
                if !e.contains("Node not registered") {
                    eprintln!("[CommandPoller] Heartbeat error: {}", e);
                }
                return Ok(()); // Don't propagate as error to avoid log spam
            }
        };

        // 2. Process commands if any
        for command in commands {
            println!("[CommandPoller] Received command: {:?}", command.id);

            // AIRLOCK CHECK
            let allowed = {
                let lock = self.airlock_service.read().await;
                if let Some(airlock) = &*lock {
                    match airlock.check_permission(&command).await {
                        Ok(true) => true,
                        Ok(false) => false,
                        Err(e) => {
                            eprintln!(
                                "[CommandPoller] Airlock error for command {}: {}",
                                command.id, e
                            );
                            false
                        }
                    }
                } else {
                    eprintln!(
                        "[CommandPoller] Airlock service not initialized! Rejecting command {}",
                        command.id
                    );
                    false
                }
            };

            if !allowed {
                println!(
                    "[CommandPoller] Command {} REJECTED by Airlock or User",
                    command.id
                );
                let _ = self
                    .neural_service
                    .complete_command(
                        &command.id,
                        CommandResult {
                            success: false,
                            output: None,
                            error: Some("Rejected by Airlock/User".into()),
                            exit_code: Some(1),
                        },
                    )
                    .await;
                continue;
            }

            // Notify start
            if let Err(e) = self.neural_service.start_command(&command.id).await {
                eprintln!(
                    "[CommandPoller] Failed to mark command {} as started: {}",
                    command.id, e
                );
            }

            // Execute - check if this is an agent.run command for full workflow
            let result = if command.intent.starts_with("agent.") {
                // Route to AgentRuntime for full ReAct workflow
                match command.intent.as_str() {
                    "agent.run" => {
                        // Extract prompt from params
                        let prompt = command
                            .payload
                            .params
                            .as_ref()
                            .and_then(|p: &serde_json::Value| p.get("prompt"))
                            .and_then(|v: &serde_json::Value| v.as_str())
                            .unwrap_or("Hello, what can you help me with?");

                        // Get workspace_id for this command
                        let workspace_id = command
                            .workspace_id
                            .clone()
                            .unwrap_or_else(|| "default".to_string());

                        println!(
                            "[CommandPoller] Routing to AgentRuntime: '{}' (workspace: {})",
                            prompt, workspace_id
                        );

                        // Create AgentRuntime on-demand
                        let context_lock = self.agent_context.read().await;
                        if let Some(ctx) = context_lock.as_ref() {
                            // Create memory for this workspace
                            let memory = Arc::new(
                                AgentMemory::new(&workspace_id, ctx.app_data_dir.clone()).await,
                            );

                            // Create config
                            let config = AgentConfig {
                                name: "Rainy Agent".to_string(),
                                model: "gemini-2.0-flash".to_string(), // Default model
                                instructions: format!(
                                    "You are Rainy Agent, an autonomous AI assistant.

Workspace ID: {}

CAPABILITIES:
- Read, write, list, and search files in the workspace.
- Navigate web pages and take screenshots.
- Perform web research.

GUIDELINES:
1. PLAN: Before executing, briefly state your plan.
2. EXECUTE: Use the provided tools to carry out the plan.
3. VERIFY: After critical operations, verify the result.",
                                    workspace_id
                                ),
                                workspace_id: workspace_id.clone(),
                                max_steps: Some(10),
                            };

                            // Create runtime
                            let runtime = AgentRuntime::new(
                                config,
                                ctx.router.clone(),
                                self.skill_executor.clone(),
                                memory,
                            );

                            // Run the agent
                            match runtime.run(prompt).await {
                                Ok(response) => CommandResult {
                                    success: true,
                                    output: Some(response),
                                    error: None,
                                    exit_code: Some(0),
                                },
                                Err(e) => CommandResult {
                                    success: false,
                                    output: None,
                                    error: Some(format!("Agent error: {}", e)),
                                    exit_code: Some(1),
                                },
                            }
                        } else {
                            CommandResult {
                                success: false,
                                output: None,
                                error: Some("Agent context not initialized".into()),
                                exit_code: Some(1),
                            }
                        }
                    }
                    _ => CommandResult {
                        success: false,
                        output: None,
                        error: Some(format!("Unknown agent skill: {}", command.intent)),
                        exit_code: Some(1),
                    },
                }
            } else {
                // Standard skill execution
                self.skill_executor.execute(&command).await
            };

            if result.success {
                println!(
                    "[CommandPoller] Execution result for {}: success=true",
                    command.id
                );
            } else {
                println!(
                    "[CommandPoller] Execution result for {}: success=false, error={:?}",
                    command.id, result.error
                );
            }

            // Report result
            if let Err(e) = self
                .neural_service
                .complete_command(&command.id, result)
                .await
            {
                eprintln!(
                    "[CommandPoller] Failed to report completion for {}: {}",
                    command.id, e
                );
            }
        }

        Ok(())
    }
}
