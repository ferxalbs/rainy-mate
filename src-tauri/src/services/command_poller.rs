use crate::ai::agent::memory::AgentMemory;
use crate::ai::agent::runtime::{AgentEvent, AgentRuntime};
use crate::ai::router::IntelligentRouter;
use crate::models::neural::CommandResult;
use crate::services::airlock::AirlockService;
use crate::services::neural_service::NeuralService;
use crate::services::settings::SettingsManager;
use crate::services::skill_executor::SkillExecutor;
use crate::services::tool_manifest::build_skill_manifest_from_runtime;
use rand::Rng;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, Notify, RwLock};
use tokio::time::sleep;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAX_PROGRESS_PREVIEW_CHARS: usize = 300;
const AGENT_PROGRESS_CHANNEL_CAPACITY: usize = 128;
const AGENT_PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(250);
const AGENT_PROGRESS_MAX_SUPPRESSED: u32 = 12;

fn with_jitter(duration: Duration) -> Duration {
    let base_ms = duration.as_millis() as u64;
    if base_ms == 0 {
        return duration;
    }
    let min_ms = (base_ms * 80) / 100;
    let max_ms = (base_ms * 120) / 100;
    let jittered_ms = rand::thread_rng().gen_range(min_ms..=max_ms);
    Duration::from_millis(jittered_ms)
}

fn progress_preview(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with("data:") {
        return "[binary content omitted]".to_string();
    }
    if trimmed.len() <= MAX_PROGRESS_PREVIEW_CHARS {
        return trimmed.to_string();
    }
    let preview: String = trimmed.chars().take(MAX_PROGRESS_PREVIEW_CHARS).collect();
    format!("{}...", preview)
}

fn map_agent_event(event: &AgentEvent) -> (String, serde_json::Value) {
    match event {
        AgentEvent::Status(text) => (
            "Agent status".to_string(),
            serde_json::json!({
                "type": "status",
                "text": progress_preview(text),
            }),
        ),
        AgentEvent::Thought(text) => (
            "Agent thought".to_string(),
            serde_json::json!({
                "type": "thought",
                "text": progress_preview(text),
            }),
        ),
        AgentEvent::StreamChunk(text) => (
            "Stream chunk".to_string(),
            serde_json::json!({
                "type": "stream_chunk",
                "text": text,
            }),
        ),
        AgentEvent::ToolCall(call) => (
            format!("Tool call: {}", call.function.name),
            serde_json::json!({
                "type": "tool_call",
                "toolCallId": call.id,
                "toolName": call.function.name,
            }),
        ),
        AgentEvent::ToolResult { id, result } => (
            "Tool result".to_string(),
            serde_json::json!({
                "type": "tool_result",
                "toolCallId": id,
                "resultPreview": progress_preview(result),
            }),
        ),
        AgentEvent::Error(text) => (
            "Agent error".to_string(),
            serde_json::json!({
                "type": "error",
                "text": progress_preview(text),
            }),
        ),
        AgentEvent::MemoryStored(text) => (
            "Memory stored".to_string(),
            serde_json::json!({
                "type": "memory_stored",
                "text": text,
            }),
        ),
    }
}

#[derive(Default)]
struct ProgressThrottle {
    last_emit_at: Option<Instant>,
    suppressed_count: u32,
}

async fn report_agent_progress_event(
    neural_service: &NeuralService,
    command_id: &str,
    throttle: &mut ProgressThrottle,
    message: String,
    mut data: serde_json::Value,
    dropped_events: usize,
) {
    let now = Instant::now();

    if let Some(last_emit_at) = throttle.last_emit_at {
        let since_last = now.saturating_duration_since(last_emit_at);
        if since_last < AGENT_PROGRESS_MIN_INTERVAL
            && throttle.suppressed_count < AGENT_PROGRESS_MAX_SUPPRESSED
        {
            throttle.suppressed_count += 1;
            return;
        }
    }

    if throttle.suppressed_count > 0 || dropped_events > 0 {
        if !data.is_object() {
            data = serde_json::json!({ "value": data });
        }
        if let Some(obj) = data.as_object_mut() {
            if throttle.suppressed_count > 0 {
                obj.insert(
                    "suppressedCount".to_string(),
                    serde_json::json!(throttle.suppressed_count),
                );
            }
            if dropped_events > 0 {
                obj.insert(
                    "droppedEvents".to_string(),
                    serde_json::json!(dropped_events),
                );
            }
        }
        throttle.suppressed_count = 0;
    }

    throttle.last_emit_at = Some(now);
    let _ = neural_service
        .report_command_progress(command_id, "info", &message, Some(data))
        .await;
}

use crate::ai::agent::manager::AgentManager;

/// Context needed to create AgentRuntime instances on-demand
pub struct AgentRuntimeContext {
    pub router: Arc<RwLock<IntelligentRouter>>,
    pub app_data_dir: PathBuf,
    pub agent_manager: Arc<AgentManager>,
}

#[derive(Clone)]
pub struct CommandPoller {
    neural_service: NeuralService,
    skill_executor: Arc<SkillExecutor>,
    agent_context: Arc<RwLock<Option<AgentRuntimeContext>>>,
    is_running: Arc<Mutex<bool>>,
    airlock_service: Arc<RwLock<Option<AirlockService>>>,
    notify: Arc<Notify>,
}

impl CommandPoller {
    pub fn new(neural_service: NeuralService, skill_executor: Arc<SkillExecutor>) -> Self {
        Self {
            neural_service,
            skill_executor,
            agent_context: Arc::new(RwLock::new(None)),
            is_running: Arc::new(Mutex::new(false)),
            airlock_service: Arc::new(RwLock::new(None)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn trigger(&self) {
        self.notify.notify_one();
    }

    /// Set the context needed to create AgentRuntime instances
    pub async fn set_agent_context(
        &self,
        router: Arc<RwLock<IntelligentRouter>>,
        app_data_dir: PathBuf,
        agent_manager: Arc<AgentManager>,
    ) {
        let mut lock = self.agent_context.write().await;
        *lock = Some(AgentRuntimeContext {
            router,
            app_data_dir,
            agent_manager,
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

            let mut backoff_failures = 0;
            const MAX_BACKOFF_SECS: u64 = 60;

            while *poller.is_running.lock().await {
                let sleep_duration = match poller.poll_and_execute().await {
                    Ok(_) => {
                        // Reset backoff on success
                        if backoff_failures > 0 {
                            println!("[CommandPoller] Connection restored.");
                            backoff_failures = 0;
                        }
                        POLL_INTERVAL
                    }
                    Err(e) => {
                        backoff_failures += 1;
                        let backoff_secs = std::cmp::min(
                            POLL_INTERVAL.as_secs() * (2u64.pow(backoff_failures.min(6) as u32)),
                            MAX_BACKOFF_SECS,
                        );
                        let sleep_with_jitter = with_jitter(Duration::from_secs(backoff_secs));

                        eprintln!(
                            "[CommandPoller] Error: {}. Retrying in {}ms (base={}s)...",
                            e,
                            sleep_with_jitter.as_millis(),
                            backoff_secs
                        );
                        sleep_with_jitter
                    }
                };

                tokio::select! {
                    _ = sleep(sleep_duration) => {}
                    _ = poller.notify.notified() => {
                        println!("[CommandPoller] Triggered by notification (Real-time event)");
                        // If triggered, likely a command is waiting, so we reset backoff
                        backoff_failures = 0;
                    }
                }
            }

            println!("[CommandPoller] Stopped polling loop");
        });
    }

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
            // Attempt auto-registration for seamless cloud<->desktop connectivity.
            let manifests = match build_skill_manifest_from_runtime() {
                Ok(value) => value,
                Err(e) => {
                    eprintln!(
                        "[CommandPoller] Failed to build runtime skill manifest for registration: {}",
                        e
                    );
                    return Ok(());
                }
            };
            match self.neural_service.register(manifests, Vec::new()).await {
                Ok(node_id) => {
                    println!("[CommandPoller] Auto-registered node: {}", node_id);
                }
                Err(e) => {
                    eprintln!("[CommandPoller] Auto-registration failed: {}", e);
                    return Ok(());
                }
            }
        }

        let pending_commands_result: Result<Vec<crate::models::neural::QueuedCommand>, String> =
            self.neural_service
                .heartbeat(crate::models::neural::DesktopNodeStatus::Online)
                .await;

        match pending_commands_result {
            Ok(commands) => {
                // Reset backoff on success
                // We should ideally have a way to reset the backoff counter in the caller loop
                // For now, we return Ok(()) which is signal for success

                // 2. Process commands if any
                for command in commands {
                    self.process_command(command).await?;
                }
                Ok(())
            }
            Err(e) => {
                // Only log if it's not a "Node not registered" error (already handled above)
                if !e.contains("Node not registered") {
                    eprintln!("[CommandPoller] Heartbeat error: {}", e);
                    Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)))
                } else {
                    Ok(())
                }
            }
        }
    }

    async fn process_command(
        &self,
        command: crate::models::neural::QueuedCommand,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
            return Ok(());
        }

        // Notify start
        if let Err(e) = self.neural_service.start_command(&command.id).await {
            eprintln!(
                "[CommandPoller] Failed to mark command {} as started: {}",
                command.id, e
            );
        }
        let _ = self
            .neural_service
            .report_command_progress(
                &command.id,
                "info",
                &format!("Started {}", command.intent),
                None,
            )
            .await;

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

                    // Extract model from params (Cloud command) or use user's selected model (Local AgentChat)
                    let model = command
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("model"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            // Local command: use user's selected model from settings
                            let settings = SettingsManager::new();
                            settings.get_selected_model().to_string()
                        });

                    // Optional agent ID to load persisted spec
                    let agent_id = command
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("agentId"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Optional agent identity/profile provided by Rainy-ATM.
                    // If present, this becomes the primary runtime instruction set.
                    let agent_name = command
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("agentName"))
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Rainy Agent".to_string());

                    let agent_system_prompt = command
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("agentSystemPrompt"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());

                    println!(
                            "[CommandPoller] Routing to AgentRuntime: agent='{}' (model: {}, workspace: {})",
                            agent_name, model, workspace_id
                        );
                    let _ = self
                        .neural_service
                        .report_command_progress(
                            &command.id,
                            "info",
                            &format!("Initializing agent runtime for '{}'", agent_name),
                            Some(serde_json::json!({
                                "model": model.clone(),
                                "workspaceId": workspace_id.clone(),
                                "agentId": agent_id.clone(),
                            })),
                        )
                        .await;

                    // Create AgentRuntime on-demand
                    let context_lock = self.agent_context.read().await;
                    if let Some(ctx) = context_lock.as_ref() {
                        // Create memory for this workspace
                        let memory = Arc::new(
                            AgentMemory::new(&workspace_id, ctx.app_data_dir.clone()).await,
                        );

                        // Try to load spec from DB if agentId is present
                        let loaded_spec = if let Some(id) = &agent_id {
                            ctx.agent_manager
                                .get_agent_spec(id)
                                .await
                                .unwrap_or_else(|e| {
                                    eprintln!(
                                        "[CommandPoller] Failed to load agent spec {}: {}",
                                        id, e
                                    );
                                    None
                                })
                        } else {
                            None
                        };

                        use crate::ai::agent::runtime::RuntimeOptions;
                        use crate::ai::specs::manifest::AgentSpec;
                        use crate::ai::specs::skills::AgentSkills;
                        use crate::ai::specs::soul::AgentSoul;

                        let options = RuntimeOptions {
                            model: Some(model),
                            workspace_id: workspace_id.clone(),
                            // Cloud commands often require several think/act cycles.
                            // Keep a bounded ceiling but avoid premature termination.
                            max_steps: Some(30),
                            // Resolve allowed_paths: payload > spec airlock > None
                            allowed_paths: if !command.payload.allowed_paths.is_empty() {
                                Some(command.payload.allowed_paths.clone())
                            } else {
                                None // will be resolved after spec is loaded
                            },
                            // Use the agentSystemPrompt from payload if available.
                            // This ensures the Cloud's "Soul" (instructions, personality) is respected
                            // even if we load a local (potentially stale) spec.
                            custom_system_prompt: agent_system_prompt.clone(),
                        };

                        // Create config
                        let base_instructions = agent_system_prompt.unwrap_or_else(|| {
                            format!(
                                "You are Rainy Agent, an autonomous AI assistant.
 
 Workspace ID: {}
 
 CAPABILITIES:
 - Read, write, list, and search files in the workspace.
 - Navigate web pages and take screenshots.
 - Perform web research.",
                                workspace_id
                            )
                        });

                        let spec = if let Some(s) = loaded_spec {
                            println!(
                                "[CommandPoller] Using persisted AgentSpec for {}",
                                s.soul.name
                            );
                            s
                        } else {
                            // Fallback / Ephemeral Construction
                            AgentSpec {
                                id: uuid::Uuid::new_v4().to_string(),
                                version: "3.0.0".to_string(),
                                soul: AgentSoul {
                                    name: agent_name.clone(),
                                    description: "Ephemeral agent spawned by Cloud Command"
                                        .to_string(),
                                    soul_content: format!(
                                        "{}

IDENTITY LOCK (MANDATORY):
- Your name is \"{}\".
- Never say you are Gemini, Google, OpenAI, Claude, or any base model/provider.
- If asked who you are, answer using your configured agent identity.

GUIDELINES:
1. PLAN: Before executing, briefly state your plan.
2. EXECUTE: Use the provided tools to carry out the plan.
3. VERIFY: After critical operations, verify the result.",
                                        base_instructions, agent_name
                                    ),
                                    ..Default::default()
                                },
                                skills: AgentSkills {
                                    capabilities: vec![],
                                    tools: std::collections::HashMap::new(),
                                },
                                airlock: Default::default(),
                                memory_config: Default::default(),
                                connectors: Default::default(),
                                signature: None,
                            }
                        };

                        // Resolve allowed_paths from spec airlock if not already set by command payload
                        let final_options = if options.allowed_paths.is_none()
                            && !spec.airlock.scopes.allowed_paths.is_empty()
                        {
                            RuntimeOptions {
                                allowed_paths: Some(spec.airlock.scopes.allowed_paths.clone()),
                                ..options
                            }
                        } else {
                            options
                        };

                        let runtime = AgentRuntime::new(
                            spec,
                            final_options,
                            ctx.router.clone(),
                            self.skill_executor.clone(),
                            memory,
                        );

                        // Run the agent with bounded event streaming to avoid ATM overload under heavy loops.
                        let neural_service = self.neural_service.clone();
                        let command_id = command.id.clone();
                        let (progress_tx, mut progress_rx) =
                            mpsc::channel::<(String, serde_json::Value)>(
                                AGENT_PROGRESS_CHANNEL_CAPACITY,
                            );
                        let dropped_events = Arc::new(AtomicUsize::new(0));
                        let reporter_dropped_events = dropped_events.clone();
                        let reporter_service = neural_service.clone();
                        let reporter_command_id = command_id.clone();
                        let reporter_handle = tokio::spawn(async move {
                            let mut throttle = ProgressThrottle::default();
                            while let Some((message, data)) = progress_rx.recv().await {
                                let dropped_since_last =
                                    reporter_dropped_events.swap(0, Ordering::Relaxed);
                                report_agent_progress_event(
                                    &reporter_service,
                                    &reporter_command_id,
                                    &mut throttle,
                                    message,
                                    data,
                                    dropped_since_last,
                                )
                                .await;
                            }

                            let trailing_dropped =
                                reporter_dropped_events.swap(0, Ordering::Relaxed);
                            if trailing_dropped > 0 {
                                let _ = reporter_service
                                    .report_command_progress(
                                        &reporter_command_id,
                                        "warn",
                                        "Some runtime events were dropped due to backpressure",
                                        Some(serde_json::json!({
                                            "droppedEvents": trailing_dropped
                                        })),
                                    )
                                    .await;
                            }
                        });

                        let callback_tx = progress_tx.clone();
                        let callback_dropped_events = dropped_events.clone();
                        match runtime
                            .run(prompt, move |event| {
                                println!("[Agent Event] {:?}", event);
                                let (message, data) = map_agent_event(&event);
                                if callback_tx.try_send((message, data)).is_err() {
                                    callback_dropped_events.fetch_add(1, Ordering::Relaxed);
                                }
                            })
                            .await
                        {
                            Ok(response) => {
                                drop(progress_tx);
                                if let Err(e) = reporter_handle.await {
                                    eprintln!(
                                        "[CommandPoller] Progress reporter join error for {}: {}",
                                        command.id, e
                                    );
                                }
                                CommandResult {
                                    success: true,
                                    output: Some(response),
                                    error: None,
                                    exit_code: Some(0),
                                }
                            }
                            Err(e) => {
                                drop(progress_tx);
                                if let Err(join_err) = reporter_handle.await {
                                    eprintln!(
                                        "[CommandPoller] Progress reporter join error for {}: {}",
                                        command.id, join_err
                                    );
                                }
                                CommandResult {
                                    success: false,
                                    output: None,
                                    error: Some(format!("Agent error: {}", e)),
                                    exit_code: Some(1),
                                }
                            }
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
            let output_preview = result
                .output
                .as_deref()
                .map(progress_preview)
                .unwrap_or_else(|| "Command completed successfully".to_string());
            let _ = self
                .neural_service
                .report_command_progress(
                    &command.id,
                    "info",
                    "Command completed",
                    Some(serde_json::json!({ "preview": output_preview })),
                )
                .await;
        } else {
            println!(
                "[CommandPoller] Execution result for {}: success=false, error={:?}",
                command.id, result.error
            );
            let error_preview = result
                .error
                .as_deref()
                .map(progress_preview)
                .unwrap_or_else(|| "Command failed".to_string());
            let _ = self
                .neural_service
                .report_command_progress(
                    &command.id,
                    "error",
                    "Command failed",
                    Some(serde_json::json!({ "error": error_preview })),
                )
                .await;
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

        Ok(())
    }
}
