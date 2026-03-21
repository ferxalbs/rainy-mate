use crate::ai::agent::memory::AgentMemory;
use crate::ai::agent::events::AgentEvent;
use crate::ai::agent::runtime::AgentRuntime;
use crate::ai::agent::runtime_registry::RuntimeRegistry;
use crate::ai::router::IntelligentRouter;
use crate::models::neural::CommandResult;
use crate::services::airlock::AirlockService;
use crate::services::agent_kill_switch::AgentKillSwitch;
use crate::services::audit_emitter::{AuditEmitter, FleetAuditEvent};
use crate::services::atm_client::ATMClient;
use crate::services::fleet_control::{apply_fleet_policy, FleetPolicyEnvelope};
use crate::services::neural_service::NeuralService;
use crate::services::settings::SettingsManager;
use crate::services::skill_executor::SkillExecutor;
use crate::services::tool_manifest::build_skill_manifest_from_runtime;
use crate::services::MemoryManager;
use crate::services::session_coordinator::SessionCoordinator;
use rand::Rng;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, Notify, RwLock, Semaphore};
use tokio::time::sleep;

const ACTIVE_POLL_INTERVAL: Duration = Duration::from_secs(2);
const IDLE_POLL_INTERVAL: Duration = Duration::from_secs(10);
const MAX_PROGRESS_PREVIEW_CHARS: usize = 300;
const AGENT_PROGRESS_CHANNEL_CAPACITY: usize = 128;
const AGENT_PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(250);
const AGENT_PROGRESS_MAX_SUPPRESSED: u32 = 12;
const DEFAULT_REMOTE_AGENT_MAX_STEPS: usize = 80;
const MIN_REMOTE_AGENT_MAX_STEPS: usize = 4;
const MAX_REMOTE_AGENT_MAX_STEPS: usize = 200;

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

fn is_transient_upstream_error(text: &str) -> bool {
    text.contains("Heartbeat failed: 502")
        || text.contains("Heartbeat failed: 503")
        || text.contains("Heartbeat failed: 504")
        || text.contains("DB_NOT_READY")
        || text.contains("NODE_REGISTER_TRANSIENT")
        || text.contains("Auth context sync failed: 503")
        || text.contains("Registration failed: 503")
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
        AgentEvent::SupervisorPlanCreated(plan) => (
            "Supervisor plan".to_string(),
            serde_json::json!({
                "type": "supervisor_plan_created",
                "summary": plan.summary,
                "steps": plan.steps,
                "verificationRequired": plan.verification_required,
                "mode": plan.mode,
                "delegationPolicy": plan.delegation_policy,
                "maxDepth": plan.max_depth,
                "maxThreads": plan.max_threads,
                "maxParallelSubagents": plan.max_parallel_subagents,
                "internalCoordinationLanguage": plan.internal_coordination_language,
                "finalResponseLanguageMode": plan.final_response_language_mode,
            }),
        ),
        AgentEvent::SpecialistSpawned(payload) => (
            format!("Specialist spawned: {}", payload.role.as_str()),
            serde_json::json!({
                "type": "specialist_spawned",
                "runId": payload.run_id,
                "agentId": payload.agent_id,
                "role": payload.role,
                "status": payload.status,
                "dependsOn": payload.depends_on,
                "detail": payload.detail,
                "startedAtMs": payload.started_at_ms,
                "finishedAtMs": payload.finished_at_ms,
                "toolCount": payload.tool_count,
                "writeLikeUsed": payload.write_like_used,
            }),
        ),
        AgentEvent::SpecialistStatusChanged(payload) => (
            format!("Specialist status: {}", payload.role.as_str()),
            serde_json::json!({
                "type": "specialist_status_changed",
                "runId": payload.run_id,
                "agentId": payload.agent_id,
                "role": payload.role,
                "status": payload.status,
                "dependsOn": payload.depends_on,
                "detail": payload.detail,
                "activeTool": payload.active_tool,
                "startedAtMs": payload.started_at_ms,
                "finishedAtMs": payload.finished_at_ms,
                "toolCount": payload.tool_count,
                "writeLikeUsed": payload.write_like_used,
            }),
        ),
        AgentEvent::SpecialistCompleted(payload) => (
            format!("Specialist completed: {}", payload.role.as_str()),
            serde_json::json!({
                "type": "specialist_completed",
                "runId": payload.run_id,
                "agentId": payload.agent_id,
                "role": payload.role,
                "summary": payload.summary,
                "responsePreview": payload.response_preview,
                "dependsOn": payload.depends_on,
                "toolCount": payload.tool_count,
                "writeLikeUsed": payload.write_like_used,
                "startedAtMs": payload.started_at_ms,
                "finishedAtMs": payload.finished_at_ms,
            }),
        ),
        AgentEvent::SpecialistFailed(payload) => (
            format!("Specialist failed: {}", payload.role.as_str()),
            serde_json::json!({
                "type": "specialist_failed",
                "runId": payload.run_id,
                "agentId": payload.agent_id,
                "role": payload.role,
                "error": payload.error,
                "dependsOn": payload.depends_on,
                "startedAtMs": payload.started_at_ms,
                "finishedAtMs": payload.finished_at_ms,
                "toolCount": payload.tool_count,
                "writeLikeUsed": payload.write_like_used,
            }),
        ),
        AgentEvent::SupervisorSummary(payload) => (
            "Supervisor summary".to_string(),
            serde_json::json!({
                "type": "supervisor_summary",
                "runId": payload.run_id,
                "summary": payload.summary,
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
    pub runtime_registry: Arc<RuntimeRegistry>,
    pub memory_manager: Arc<MemoryManager>,
    pub session_coordinator: Arc<SessionCoordinator>,
}

#[derive(Clone)]
pub struct CommandPoller {
    neural_service: NeuralService,
    atm_client: Arc<ATMClient>,
    skill_executor: Arc<SkillExecutor>,
    agent_context: Arc<RwLock<Option<AgentRuntimeContext>>>,
    is_running: Arc<Mutex<bool>>,
    airlock_service: Arc<RwLock<Option<AirlockService>>>,
    notify: Arc<Notify>,
    kill_switch: AgentKillSwitch,
    audit_emitter: AuditEmitter,
    concurrent_runs: Arc<Semaphore>,
}

impl CommandPoller {
    pub fn new(
        neural_service: NeuralService,
        atm_client: Arc<ATMClient>,
        skill_executor: Arc<SkillExecutor>,
    ) -> Self {
        Self {
            neural_service,
            atm_client,
            skill_executor,
            agent_context: Arc::new(RwLock::new(None)),
            is_running: Arc::new(Mutex::new(false)),
            airlock_service: Arc::new(RwLock::new(None)),
            notify: Arc::new(Notify::new()),
            kill_switch: AgentKillSwitch::new(),
            audit_emitter: AuditEmitter::new(),
            concurrent_runs: Arc::new(Semaphore::new(3)),
        }
    }

    pub fn trigger(&self) {
        self.notify.notify_one();
    }

    pub async fn arm_kill_switch(&self, reason: &str) {
        self.kill_switch.trigger();
        self.audit_emitter
            .enqueue(FleetAuditEvent {
                action_type: "fleet.kill_switch.armed".to_string(),
                outcome: "success".to_string(),
                agent_id: None,
                tool_name: None,
                airlock_level: Some(2),
                payload_json: Some(serde_json::json!({ "reason": reason }).to_string()),
            })
            .await;
    }

    async fn active_runtime_counts(&self) -> (usize, usize) {
        let context_lock = self.agent_context.read().await;
        if let Some(ctx) = context_lock.as_ref() {
            let snapshot = ctx.runtime_registry.snapshot().await;
            (snapshot.active_supervisor_runs, snapshot.active_specialists)
        } else {
            (0, 0)
        }
    }

    /// Set the context needed to create AgentRuntime instances
    pub async fn set_agent_context(
        &self,
        router: Arc<RwLock<IntelligentRouter>>,
        app_data_dir: PathBuf,
        agent_manager: Arc<AgentManager>,
        runtime_registry: Arc<RuntimeRegistry>,
        memory_manager: Arc<MemoryManager>,
        session_coordinator: Arc<SessionCoordinator>,
    ) {
        let mut lock = self.agent_context.write().await;
        *lock = Some(AgentRuntimeContext {
            router,
            app_data_dir,
            agent_manager,
            runtime_registry,
            memory_manager,
            session_coordinator,
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
                    Ok(processed_count) => {
                        // Reset backoff on success
                        if backoff_failures > 0 {
                            println!("[CommandPoller] Connection restored.");
                            backoff_failures = 0;
                        }
                        if processed_count > 0 {
                            ACTIVE_POLL_INTERVAL
                        } else {
                            IDLE_POLL_INTERVAL
                        }
                    }
                    Err(e) => {
                        let error_text = e.to_string();
                        backoff_failures += 1;
                        let backoff_secs = std::cmp::min(
                            ACTIVE_POLL_INTERVAL.as_secs()
                                * (2u64.pow(backoff_failures.min(6) as u32)),
                            MAX_BACKOFF_SECS,
                        );
                        let sleep_with_jitter = with_jitter(Duration::from_secs(backoff_secs));

                        if is_transient_upstream_error(&error_text) {
                            println!(
                                "[CommandPoller] Temporary upstream issue: {}. Retrying in {}ms (base={}s)...",
                                error_text,
                                sleep_with_jitter.as_millis(),
                                backoff_secs
                            );
                        } else {
                            eprintln!(
                                "[CommandPoller] Error: {}. Retrying in {}ms (base={}s)...",
                                error_text,
                                sleep_with_jitter.as_millis(),
                                backoff_secs
                            );
                        }
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

    async fn poll_and_execute(&self) -> Result<usize, Box<dyn std::error::Error>> {
        // 1. Send heartbeat and get commands via NeuralService
        if !self.neural_service.has_credentials().await {
            return Ok(0); // Silently skip if not authenticated
        }

        // Check if node is registered (has node_id)
        if !self.neural_service.is_registered().await {
            if !self.neural_service.can_attempt_registration().await {
                return Ok(0);
            }

            match self.atm_client.get_service_status().await {
                Ok(status) if !status.ready => {
                    println!(
                        "[CommandPoller] ATM warming up (code={}): {}. Skipping registration until ready.",
                        status.code.unwrap_or_else(|| "UNKNOWN".to_string()),
                        status.message
                    );
                    return Ok(0);
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[CommandPoller] ATM readiness probe failed: {}", e);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
                }
            }

            if let Err(e) = self.neural_service.sync_workspace_id_with_auth_context().await {
                eprintln!("[CommandPoller] Auth-context sync failed before register: {}", e);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
            }

            // Attempt auto-registration for seamless cloud<->desktop connectivity.
            let manifests = match build_skill_manifest_from_runtime() {
                Ok(value) => value,
                Err(e) => {
                    eprintln!(
                        "[CommandPoller] Failed to build runtime skill manifest for registration: {}",
                        e
                    );
                    return Ok(0);
                }
            };
            match self.neural_service.register(manifests, Vec::new()).await {
                Ok(node_id) => {
                    println!("[CommandPoller] Auto-registered node: {}", node_id);
                }
                Err(e) => {
                    eprintln!("[CommandPoller] Auto-registration failed: {}", e);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
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
                let command_count = commands.len();
                for command in commands {
                    let permit = self.concurrent_runs.clone().acquire_owned().await
                        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error>)?;
                    let poller = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = poller.process_command(command).await {
                            eprintln!("[CommandPoller] process_command error: {}", e);
                        }
                        drop(permit);
                    });
                }
                Ok(command_count)
            }
            Err(e) => {
                // Only log if it's not a "Node not registered" error (already handled above)
                if !e.contains("Node not registered") {
                    eprintln!("[CommandPoller] Heartbeat error: {}", e);
                    Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)))
                } else {
                    Ok(0)
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
            self.audit_emitter
                .enqueue(FleetAuditEvent {
                    action_type: "airlock.decision".to_string(),
                    outcome: "blocked".to_string(),
                    agent_id: None,
                    tool_name: command.payload.method.clone(),
                    airlock_level: Some(command.airlock_level as u8),
                    payload_json: Some(
                        serde_json::json!({
                            "intent": command.intent,
                            "reason": "Rejected by Airlock/User"
                        })
                        .to_string(),
                    ),
                })
                .await;
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
            if e.contains("404") || e.contains("409") {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
            }
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

        // Verify cloud-sourced tool_access_policy hash before applying
        if let (Some(policy), Some(expected_hash)) = (
            &command.payload.tool_access_policy,
            &command.payload.tool_access_policy_hash,
        ) {
            use sha2::{Digest, Sha256};
            let canonical = serde_json::to_string(policy)
                .map_err(|e| format!("Failed to serialize tool_access_policy: {}", e))?;
            let actual = format!("{:x}", Sha256::digest(canonical.as_bytes()));
            if actual != *expected_hash {
                return Err(format!(
                    "tool_access_policy hash mismatch for command {}",
                    command.id
                )
                .into());
            }
        }

        let mut command_for_execution = command.clone();
        if !command_for_execution.intent.starts_with("fleet.")
            && command_for_execution.payload.tool_access_policy.is_none()
        {
            let workspace_id = command_for_execution
                .workspace_id
                .clone()
                .unwrap_or_else(|| "default".to_string());
            let settings = SettingsManager::new();
            if let Some(state) = settings.get_workspace_tool_policy_state(&workspace_id) {
                command_for_execution.payload.tool_access_policy =
                    Some(state.tool_access_policy.clone());
                command_for_execution.payload.tool_access_policy_version =
                    Some(state.tool_access_policy_version);
                command_for_execution.payload.tool_access_policy_hash =
                    Some(state.tool_access_policy_hash.clone());
                let _ = self
                    .neural_service
                    .report_command_progress(
                        &command.id,
                        "info",
                        "Applied persisted fleet policy to command execution",
                        Some(serde_json::json!({
                            "toolAccessPolicyVersion": state.tool_access_policy_version,
                            "toolAccessPolicyHash": state.tool_access_policy_hash,
                        })),
                    )
                    .await;
            }
        }

        // Execute - check if this is an agent.run command for full workflow
        let result = if command_for_execution.intent.starts_with("fleet.") {
            match command_for_execution.intent.as_str() {
                "fleet.apply_policy" => {
                    let workspace_id = command_for_execution
                        .workspace_id
                        .clone()
                        .unwrap_or_else(|| "default".to_string());
                    let parsed = command_for_execution
                        .payload
                        .params
                        .clone()
                        .ok_or_else(|| "Missing fleet policy params".to_string())
                        .and_then(|value| {
                            let envelope: FleetPolicyEnvelope = serde_json::from_value(value)
                                .map_err(|e| format!("Invalid fleet policy payload: {}", e))?;
                            Ok(envelope)
                        });

                    match parsed.and_then(|envelope| apply_fleet_policy(&workspace_id, &envelope))
                    {
                        Ok(_) => {
                            self.kill_switch.clear();
                            self.audit_emitter
                                .enqueue(FleetAuditEvent {
                                    action_type: "fleet.apply_policy".to_string(),
                                    outcome: "success".to_string(),
                                    agent_id: None,
                                    tool_name: None,
                                    airlock_level: Some(command_for_execution.airlock_level as u8),
                                    payload_json: Some(
                                        serde_json::json!({
                                            "workspaceId": workspace_id,
                                        })
                                        .to_string(),
                                    ),
                                })
                                .await;
                            CommandResult {
                                success: true,
                                output: Some("Fleet policy applied atomically".to_string()),
                                error: None,
                                exit_code: Some(0),
                            }
                        }
                        Err(e) => CommandResult {
                            success: false,
                            output: None,
                            error: Some(e),
                            exit_code: Some(1),
                        },
                    }
                }
                "fleet.terminate_all_agents" => {
                    let (before_supervisor_runs, before_specialists) =
                        self.active_runtime_counts().await;
                    let started_at = Instant::now();
                    self.arm_kill_switch("fleet.terminate_all_agents command")
                        .await;
                    let deadline = Instant::now() + Duration::from_secs(5);
                    let mut after_supervisor_runs = before_supervisor_runs;
                    let mut after_specialists = before_specialists;
                    while Instant::now() < deadline {
                        let (runs, specialists) = self.active_runtime_counts().await;
                        after_supervisor_runs = runs;
                        after_specialists = specialists;
                        if runs == 0 && specialists == 0 {
                            break;
                        }
                        sleep(Duration::from_millis(100)).await;
                    }

                    let elapsed_ms = started_at.elapsed().as_millis() as u64;
                    let settled_within_sla = after_supervisor_runs == 0 && after_specialists == 0;
                    let ack_payload = serde_json::json!({
                        "before": {
                            "activeSupervisorRuns": before_supervisor_runs,
                            "activeSpecialists": before_specialists
                        },
                        "after": {
                            "activeSupervisorRuns": after_supervisor_runs,
                            "activeSpecialists": after_specialists
                        },
                        "elapsedMs": elapsed_ms,
                        "withinSla5s": settled_within_sla
                    });

                    self.audit_emitter
                        .enqueue(FleetAuditEvent {
                            action_type: "fleet.terminate_all_agents".to_string(),
                            outcome: if settled_within_sla {
                                "success".to_string()
                            } else {
                                "error".to_string()
                            },
                            agent_id: None,
                            tool_name: None,
                            airlock_level: Some(command_for_execution.airlock_level as u8),
                            payload_json: Some(ack_payload.to_string()),
                        })
                        .await;

                    if settled_within_sla {
                        CommandResult {
                            success: true,
                            output: Some(ack_payload.to_string()),
                            error: None,
                            exit_code: Some(0),
                        }
                    } else {
                        CommandResult {
                            success: false,
                            output: Some(ack_payload.to_string()),
                            error: Some(
                                "Kill switch armed but active runs remained after 5 seconds"
                                    .to_string(),
                            ),
                            exit_code: Some(1),
                        }
                    }
                }
                _ => CommandResult {
                    success: false,
                    output: None,
                    error: Some(format!(
                        "Unknown fleet intent: {}",
                        command_for_execution.intent
                    )),
                    exit_code: Some(1),
                },
            }
        } else if command_for_execution.intent.starts_with("agent.") {
            // Route to AgentRuntime for full ReAct workflow
            match command_for_execution.intent.as_str() {
                "agent.run" => {
                    if self.kill_switch.is_triggered() {
                        CommandResult {
                            success: false,
                            output: None,
                            error: Some(
                                "Kill switch active: agent.run blocked until policy reset"
                                    .to_string(),
                            ),
                            exit_code: Some(1),
                        }
                    } else {
                    // Extract prompt from params
                    let prompt = command_for_execution
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p: &serde_json::Value| p.get("prompt"))
                        .and_then(|v: &serde_json::Value| v.as_str())
                        .unwrap_or("Hello, what can you help me with?");

                    // Get workspace_id for this command
                    let workspace_id = command_for_execution
                        .workspace_id
                        .clone()
                        .unwrap_or_else(|| "default".to_string());

                    // Extract model from params (Cloud command) or use user's selected model (Local AgentChat)
                    let model = command_for_execution
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

                    let max_steps = command_for_execution
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("maxSteps").or_else(|| p.get("max_steps")))
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize)
                        .unwrap_or(DEFAULT_REMOTE_AGENT_MAX_STEPS)
                        .clamp(MIN_REMOTE_AGENT_MAX_STEPS, MAX_REMOTE_AGENT_MAX_STEPS);

                    // Optional agent ID to load persisted spec
                    let agent_id = command_for_execution
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("agentId").or_else(|| p.get("agentSpecId")))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Optional agent identity/profile provided by Rainy-ATM.
                    // If present, this becomes the primary runtime instruction set.
                    let agent_name = command_for_execution
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("agentName"))
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Rainy Agent".to_string());

                    let agent_system_prompt = command_for_execution
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("agentSystemPrompt"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());

                    // Extract optional desktop chat_id (for Telegram continuity)
                    let incoming_chat_id = command_for_execution
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("chatId").or_else(|| p.get("chat_id")))
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| s.to_string());

                    // Extract session peer (Telegram chatId / Discord channel)
                    let session_peer = command_for_execution
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("sessionPeer").or_else(|| p.get("peer")))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| command.payload.user_id.clone());

                    let connector_id_for_session = command
                        .payload
                        .connector_id
                        .clone()
                        .unwrap_or_else(|| "remote".to_string());

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
                                "maxSteps": max_steps,
                                "workspaceId": workspace_id.clone(),
                                "agentId": agent_id.clone(),
                            })),
                        )
                        .await;

                    // Create AgentRuntime on-demand
                    let context_lock = self.agent_context.read().await;
                    if let Some(ctx) = context_lock.as_ref() {
                        // Create memory for this workspace
                        let vault = ctx.memory_manager.get_vault().await;
                        let memory = Arc::new(
                            AgentMemory::new(
                                &workspace_id,
                                ctx.app_data_dir.clone(),
                                ctx.memory_manager.clone(),
                                Some(ctx.router.clone()),
                                vault,
                            )
                            .await,
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
                            max_steps: Some(max_steps),
                            // Resolve allowed_paths: payload > spec airlock > None
                            allowed_paths: if !command.payload.allowed_paths.is_empty() {
                                Some(command_for_execution.payload.allowed_paths.clone())
                            } else {
                                None // will be resolved after spec is loaded
                            },
                            // Use the agentSystemPrompt from payload if available.
                            // This ensures the Cloud's "Soul" (instructions, personality) is respected
                            reasoning_effort: None,
                            // even if we load a local (potentially stale) spec.
                            custom_system_prompt: agent_system_prompt.clone(),
                            streaming_enabled: Some(false),
                            temperature: None,
                            max_tokens: None,
                            connector_id: command.payload.connector_id.clone(),
                            user_id: command.payload.user_id.clone(),
                        };

                        // Create config
                        let base_instructions = agent_system_prompt.unwrap_or_else(|| {
                            format!(
                                "You are Rainy Agent, an autonomous AI assistant.
 
 Workspace ID: {}
 
 CAPABILITIES:
 - Read, write, list, and search files in the workspace.
 - Navigate web pages and take screenshots.
 - Perform web research.
 - Use shell tools only through provided methods; `execute_command` may reject commands outside the allowlist (e.g. `find` can be blocked).

 TOOL RELIABILITY RULES (MANDATORY):
 - Never state that a command or tool action succeeded unless the tool result explicitly succeeded.
 - If a tool fails or is blocked, report the exact failure to the user.
 - Do not invent file hashes, scan results, or command output after a tool failure.
 - If blocked, use another permitted tool or ask the user for data.",
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
3. VERIFY: After critical operations, verify the result.
4. NEVER claim a tool succeeded if it failed or was blocked.
5. If blocked by tool policy, say so explicitly and request a permitted alternative path or user input.",
                                        base_instructions, agent_name
                                    ),
                                    ..Default::default()
                                },
                                skills: AgentSkills::default(),
                                airlock: Default::default(),
                                memory_config: Default::default(),
                                connectors: Default::default(),
                                runtime: Default::default(),
                                model: None,
                                temperature: None,
                                max_tokens: None,
                                provider: None,
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

                        let airlock = self.airlock_service.read().await.clone();
                        let runtime = AgentRuntime::new(
                            spec,
                            final_options,
                            ctx.router.clone(),
                            self.skill_executor.clone(),
                            memory,
                            Arc::new(airlock),
                            Some(self.kill_switch.clone()),
                            Some(ctx.runtime_registry.clone()),
                        );

                        // Start session via SessionCoordinator (creates chat, saves user message, emits session://started)
                        let session_coordinator = ctx.session_coordinator.clone();
                        let (session_chat_id, session_run_id) = session_coordinator
                            .start_remote_session(
                                incoming_chat_id.clone(),
                                &workspace_id,
                                prompt,
                                &connector_id_for_session,
                                session_peer.as_deref().unwrap_or("unknown"),
                                Some(command.id.clone()),
                            )
                            .await
                            .unwrap_or_else(|e| {
                                eprintln!("[CommandPoller] SessionCoordinator.start_remote_session failed: {}", e);
                                (uuid::Uuid::new_v4().to_string(), uuid::Uuid::new_v4().to_string())
                            });
                        let session_coordinator_for_events = session_coordinator.clone();
                        let session_run_id_for_events = session_run_id.clone();

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
                        let audit_emitter = self.audit_emitter.clone();
                        let audit_agent_id = agent_id.clone();
                        match runtime
                            .run(prompt, move |event| {
                                println!("[Agent Event] {:?}", event);
                                let (message, data) = map_agent_event(&event);
                                if callback_tx.try_send((message, data)).is_err() {
                                    callback_dropped_events.fetch_add(1, Ordering::Relaxed);
                                }

                                // Emit to frontend for live streaming
                                session_coordinator_for_events.emit_agent_event(&session_run_id_for_events, event.clone());

                                match event {
                                    AgentEvent::ToolCall(ref call) => {
                                        let audit_emitter = audit_emitter.clone();
                                        let agent_id = audit_agent_id.clone();
                                        let tool_name = call.function.name.clone();
                                        tokio::spawn(async move {
                                            audit_emitter
                                                .enqueue(FleetAuditEvent {
                                                    action_type: "tool.execution".to_string(),
                                                    outcome: "info".to_string(),
                                                    agent_id,
                                                    tool_name: Some(tool_name),
                                                    airlock_level: None,
                                                    payload_json: None,
                                                })
                                                .await;
                                        });
                                    }
                                    AgentEvent::ToolResult { id: _, result } => {
                                        let audit_emitter = audit_emitter.clone();
                                        let agent_id = audit_agent_id.clone();
                                        let outcome = if result.to_ascii_lowercase().contains("blocked")
                                            || result.to_ascii_lowercase().contains("airlock")
                                        {
                                            "blocked"
                                        } else if result.to_ascii_lowercase().starts_with("error:") {
                                            "error"
                                        } else {
                                            "success"
                                        };
                                        tokio::spawn(async move {
                                            audit_emitter
                                                .enqueue(FleetAuditEvent {
                                                    action_type: "tool.result".to_string(),
                                                    outcome: outcome.to_string(),
                                                    agent_id,
                                                    tool_name: None,
                                                    airlock_level: None,
                                                    payload_json: Some(
                                                        serde_json::json!({
                                                            "resultPreview": progress_preview(&result),
                                                        })
                                                        .to_string(),
                                                    ),
                                                })
                                                .await;
                                        });
                                    }
                                    AgentEvent::Error(ref text) => {
                                        let lower = text.to_ascii_lowercase();
                                        if lower.contains("airlock") || lower.contains("blocked") {
                                            let audit_emitter = audit_emitter.clone();
                                            let agent_id = audit_agent_id.clone();
                                            let detail = text.clone();
                                            tokio::spawn(async move {
                                                audit_emitter
                                                    .enqueue(FleetAuditEvent {
                                                        action_type: "airlock.decision".to_string(),
                                                        outcome: "blocked".to_string(),
                                                        agent_id,
                                                        tool_name: None,
                                                        airlock_level: None,
                                                        payload_json: Some(
                                                            serde_json::json!({
                                                                "detail": detail,
                                                            })
                                                            .to_string(),
                                                        ),
                                                    })
                                                    .await;
                                            });
                                        }
                                    }
                                    _ => {}
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
                                // Finish session: save assistant message, emit session://finished
                                let _ = session_coordinator
                                    .finish_remote_session(&session_chat_id, &response, prompt)
                                    .await;
                                // Include chatId in output for ATM continuity
                                let output_with_chat_id = serde_json::json!({
                                    "response": response,
                                    "chatId": session_chat_id,
                                }).to_string();
                                CommandResult {
                                    success: true,
                                    output: Some(output_with_chat_id),
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
                                session_coordinator.unregister(&session_chat_id);
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
                }
                _ => CommandResult {
                    success: false,
                    output: None,
                    error: Some(format!(
                        "Unknown agent skill: {}",
                        command_for_execution.intent
                    )),
                    exit_code: Some(1),
                },
            }
        } else {
            // Standard skill execution
            self.skill_executor.execute(&command_for_execution).await
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

        if let Some(node_id) = command.desktop_node_id.as_deref() {
            if let Err(e) = self.audit_emitter.flush(&self.atm_client, node_id).await {
                eprintln!(
                    "[CommandPoller] Failed to flush fleet audit queue for node {}: {}",
                    node_id, e
                );
            }
        }

        Ok(())
    }
}
