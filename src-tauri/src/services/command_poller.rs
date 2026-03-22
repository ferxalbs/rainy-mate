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
use dashmap::DashSet;
use rand::Rng;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify, RwLock, Semaphore};
use tokio::time::sleep;

const ACTIVE_POLL_INTERVAL: Duration = Duration::from_secs(2);
const IDLE_POLL_INTERVAL: Duration = Duration::from_secs(10);
pub(crate) const MAX_PROGRESS_PREVIEW_CHARS: usize = 300;

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

pub(crate) fn progress_preview(value: &str) -> String {
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
    /// Shared SettingsManager — avoids creating a fresh instance (disk read) per command
    settings: Arc<Mutex<SettingsManager>>,
    agent_context: Arc<RwLock<Option<AgentRuntimeContext>>>,
    is_running: Arc<Mutex<bool>>,
    airlock_service: Arc<RwLock<Option<AirlockService>>>,
    notify: Arc<Notify>,
    kill_switch: AgentKillSwitch,
    audit_emitter: AuditEmitter,
    concurrent_runs: Arc<Semaphore>,
    /// Tracks recently processed command IDs to skip duplicate deliveries
    seen_commands: Arc<DashSet<String>>,
}

impl CommandPoller {
    pub fn new(
        neural_service: NeuralService,
        atm_client: Arc<ATMClient>,
        skill_executor: Arc<SkillExecutor>,
        settings: Arc<Mutex<SettingsManager>>,
    ) -> Self {
        Self {
            neural_service,
            atm_client,
            skill_executor,
            settings,
            agent_context: Arc::new(RwLock::new(None)),
            is_running: Arc::new(Mutex::new(false)),
            airlock_service: Arc::new(RwLock::new(None)),
            notify: Arc::new(Notify::new()),
            kill_switch: AgentKillSwitch::new(),
            audit_emitter: AuditEmitter::new(),
            concurrent_runs: Arc::new(Semaphore::new(3)),
            seen_commands: Arc::new(DashSet::new()),
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

                // 2. Process commands if any — spawn immediately so the poll loop
                //    returns without waiting for permits (semaphore acquired inside spawn).
                let command_count = commands.len();
                for command in commands {
                    // Skip commands we've already dispatched (dedup against network retries)
                    if !self.seen_commands.insert(command.id.clone()) {
                        tracing::debug!("[CommandPoller] Skipping duplicate command: {}", command.id);
                        continue;
                    }
                    // Prune seen_commands to avoid unbounded growth (keep last ~500)
                    if self.seen_commands.len() > 500 {
                        if let Some(entry) = self.seen_commands.iter().next() {
                            let key = entry.key().clone();
                            drop(entry);
                            self.seen_commands.remove(&key);
                        }
                    }
                    let sem = self.concurrent_runs.clone();
                    let poller = self.clone();
                    tokio::spawn(async move {
                        // Acquire semaphore inside the spawn so the poll loop is not blocked
                        let permit = match sem.acquire_owned().await {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("[CommandPoller] semaphore error: {}", e);
                                return;
                            }
                        };
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
            let settings = self.settings.lock().await;
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
                    crate::services::command_poller_agent::process_agent_run(
                        &command,
                        &command_for_execution,
                        &self.kill_switch,
                        self.settings.clone(),
                        self.agent_context.clone(),
                        self.airlock_service.clone(),
                        self.skill_executor.clone(),
                        self.neural_service.clone(),
                        self.audit_emitter.clone(),
                    ).await
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
