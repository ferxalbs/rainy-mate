// Agent run processing — extracted from command_poller.rs to keep module size bounded.
// Contains the agent.run branch: spec loading, AgentRuntime construction, and session management.
use crate::ai::agent::events::AgentEvent;
use crate::ai::agent::memory::AgentMemory;
use crate::ai::agent::runtime::AgentRuntime;
use crate::models::folder::FolderAccess;
use crate::models::neural::{AirlockLevel, CommandPriority, CommandResult, CommandStatus, QueuedCommand, RainyPayload};
use crate::services::agent_kill_switch::AgentKillSwitch;
use crate::services::airlock::AirlockService;
use crate::services::audit_emitter::{AuditEmitter, FleetAuditEvent};
use crate::services::command_poller::{progress_preview, AgentRuntimeContext};
use crate::services::neural_service::NeuralService;
use crate::services::settings::SettingsManager;
use crate::services::skill_executor::SkillExecutor;
use crate::services::FolderManager;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::Manager;
use tokio::sync::{mpsc, Mutex, RwLock};

const AGENT_PROGRESS_CHANNEL_CAPACITY: usize = 128;
const AGENT_PROGRESS_MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
const AGENT_PROGRESS_MAX_SUPPRESSED: u32 = 12;
pub(crate) const DEFAULT_REMOTE_AGENT_MAX_STEPS: usize = 80;
pub(crate) const MIN_REMOTE_AGENT_MAX_STEPS: usize = 4;
pub(crate) const MAX_REMOTE_AGENT_MAX_STEPS: usize = 200;
const REMOTE_WORKSPACE_APPROVAL_METHOD: &str = "remote_workspace_access";

#[derive(Debug, Clone)]
struct RemoteWorkspaceResolution {
    workspace_id: String,
    display_name: String,
    canonical_path: String,
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

fn make_remote_workspace_command(
    parent_command: &QueuedCommand,
    requested_path: &str,
) -> QueuedCommand {
    QueuedCommand {
        id: format!("{}::remote-workspace", parent_command.id),
        workspace_id: parent_command.workspace_id.clone(),
        desktop_node_id: parent_command.desktop_node_id.clone(),
        intent: format!("remote.{}", REMOTE_WORKSPACE_APPROVAL_METHOD),
        payload: RainyPayload {
            skill: Some("remote".to_string()),
            method: Some(REMOTE_WORKSPACE_APPROVAL_METHOD.to_string()),
            params: Some(serde_json::json!({ "path": requested_path })),
            connector_id: parent_command.payload.connector_id.clone(),
            user_id: parent_command.payload.user_id.clone(),
            ..Default::default()
        },
        priority: CommandPriority::High,
        status: CommandStatus::Pending,
        airlock_level: AirlockLevel::Dangerous,
        approval_timeout_secs: Some(0),
        approved_by: None,
        result: None,
        created_at: parent_command.created_at,
        started_at: None,
        completed_at: None,
        schema_version: parent_command.schema_version.clone(),
    }
}

fn canonicalize_remote_path(requested_path: &str) -> Result<String, String> {
    let trimmed = requested_path.trim();
    if trimmed.is_empty() {
        return Err("Requested workspace path is empty".to_string());
    }
    let canonical = Path::new(trimmed)
        .canonicalize()
        .map_err(|error| format!("Cannot access requested path '{}': {}", trimmed, error))?;
    if !canonical.is_dir() {
        return Err(format!(
            "Requested workspace path is not a directory: {}",
            canonical.to_string_lossy()
        ));
    }
    Ok(canonical.to_string_lossy().to_string())
}

async fn ensure_folder_visible(
    folder_manager: &FolderManager,
    canonical_path: &str,
) -> Result<(), String> {
    if let Some(folder) = folder_manager.get_folder_by_path(canonical_path).await {
        folder_manager.update_last_accessed(&folder.id).await?;
        return Ok(());
    }

    let name = Path::new(canonical_path)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| canonical_path.to_string());

    match folder_manager
        .add_folder(canonical_path.to_string(), name, FolderAccess::FullAccess)
        .await
    {
        Ok(_) => Ok(()),
        Err(error) if error.contains("already added") => {
            folder_manager.update_last_accessed_by_path(canonical_path).await
        }
        Err(error) => Err(error),
    }
}

async fn resolve_remote_workspace(
    context: &AgentRuntimeContext,
    airlock: Option<&AirlockService>,
    command: &QueuedCommand,
    cloud_workspace_id: &str,
    connector_id: &str,
    session_peer: &str,
    requested_path: Option<&str>,
) -> Result<Option<RemoteWorkspaceResolution>, String> {
    let existing = context
        .remote_workspace_grants
        .get_active(cloud_workspace_id, connector_id, session_peer)
        .await;

    let canonical_path = if let Some(path) = requested_path {
        let canonical = canonicalize_remote_path(path)?;
        if existing
            .as_ref()
            .map(|grant| grant.canonical_path == canonical)
            .unwrap_or(false)
        {
            context
                .remote_workspace_grants
                .touch(cloud_workspace_id, connector_id, session_peer)
                .await;
            canonical
        } else {
            let approval_command = make_remote_workspace_command(command, &canonical);
            let Some(airlock) = airlock else {
                return Err("Airlock service unavailable for remote workspace approval".to_string());
            };
            if !airlock.check_permission(&approval_command).await? {
                return Err(format!(
                    "Remote workspace access was not approved for {}",
                    canonical
                ));
            }
            context
                .remote_workspace_grants
                .insert(cloud_workspace_id, connector_id, session_peer, &canonical)
                .await;
            canonical
        }
    } else if let Some(grant) = context
        .remote_workspace_grants
        .touch(cloud_workspace_id, connector_id, session_peer)
        .await
    {
        canonicalize_remote_path(&grant.canonical_path)?
    } else {
        return Ok(None);
    };

    let folder_manager = context.app_handle.state::<FolderManager>();
    ensure_folder_visible(folder_manager.inner(), &canonical_path).await?;

    Ok(Some(RemoteWorkspaceResolution {
        display_name: Path::new(&canonical_path)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| canonical_path.clone()),
        workspace_id: canonical_path.clone(),
        canonical_path,
    }))
}

/// Execute an `agent.run` command by building an AgentRuntime and running the ReAct loop.
pub(crate) async fn process_agent_run(
    command: &QueuedCommand,
    command_for_execution: &QueuedCommand,
    kill_switch: &AgentKillSwitch,
    settings: Arc<Mutex<SettingsManager>>,
    agent_context: Arc<RwLock<Option<AgentRuntimeContext>>>,
    airlock_service: Arc<RwLock<Option<AirlockService>>>,
    skill_executor: Arc<SkillExecutor>,
    neural_service: NeuralService,
    audit_emitter: AuditEmitter,
) -> CommandResult {
    if kill_switch.is_triggered() {
        return CommandResult {
            success: false,
            output: None,
            error: Some("Kill switch active: agent.run blocked until policy reset".to_string()),
            exit_code: Some(1),
        };
    }

    // Extract prompt from params
    let prompt = command_for_execution
        .payload
        .params
        .as_ref()
        .and_then(|p: &serde_json::Value| p.get("prompt"))
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap_or("Hello, what can you help me with?");

    // Get cloud workspace_id for this command (used for routing/grant keys)
    let workspace_id = command_for_execution
        .workspace_id
        .clone()
        .unwrap_or_else(|| "default".to_string());

    // Extract model from params (Cloud command) or use user's selected model (Local AgentChat)
    let model = match command_for_execution
        .payload
        .params
        .as_ref()
        .and_then(|p| p.get("model"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
    {
        Some(m) => m,
        None => settings.lock().await.get_selected_model().to_string(),
    };

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

    let requested_remote_workspace = command_for_execution
        .payload
        .params
        .as_ref()
        .and_then(|p| p.as_object())
        .and_then(|params| {
            params
                .get("remoteWorkspacePath")
                .or_else(|| params.get("remote_workspace_path"))
                .or_else(|| params.get("workspacePath"))
                .or_else(|| {
                    params
                        .get("telegramBootstrap")
                        .and_then(|value| value.get("requestedPath"))
                })
                .or_else(|| {
                    params
                        .get("telegramBootstrap")
                        .and_then(|value| value.get("canonicalPath"))
                })
        })
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let connector_id_for_session = command
        .payload
        .connector_id
        .clone()
        .unwrap_or_else(|| "remote".to_string());

    println!(
        "[CommandPoller] Routing to AgentRuntime: agent='{}' (model: {}, workspace: {})",
        agent_name, model, workspace_id
    );
    let _ = neural_service
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
    let context_lock = agent_context.read().await;
    if let Some(ctx) = context_lock.as_ref() {
        let airlock = airlock_service.read().await.clone();
        let remote_workspace = if connector_id_for_session == "telegram"
            || connector_id_for_session == "discord"
            || connector_id_for_session == "whatsapp"
        {
            if let Some(session_peer) = session_peer.as_deref() {
                match resolve_remote_workspace(
                    ctx,
                    airlock.as_ref(),
                    command_for_execution,
                    &workspace_id,
                    &connector_id_for_session,
                    session_peer,
                    requested_remote_workspace.as_deref(),
                )
                .await
                {
                    Ok(resolution) => resolution,
                    Err(error) => {
                        return CommandResult {
                            success: false,
                            output: None,
                            error: Some(error),
                            exit_code: Some(1),
                        };
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        let runtime_workspace_id = remote_workspace
            .as_ref()
            .map(|value| value.workspace_id.clone())
            .unwrap_or_else(|| workspace_id.clone());

        // Create memory for this workspace
        let vault = ctx.memory_manager.get_vault().await;
        let memory = Arc::new(
            AgentMemory::new(
                &runtime_workspace_id,
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
                    eprintln!("[CommandPoller] Failed to load agent spec {}: {}", id, e);
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
            workspace_id: runtime_workspace_id.clone(),
            // Cloud commands often require several think/act cycles.
            // Keep a bounded ceiling but avoid premature termination.
            max_steps: Some(max_steps),
            // Resolve allowed_paths: payload > spec airlock > None
            allowed_paths: if let Some(remote_workspace) = remote_workspace.as_ref() {
                Some(vec![remote_workspace.canonical_path.clone()])
            } else if !command.payload.allowed_paths.is_empty() {
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
            attachments: {
                // Extract cloud attachments sent by ATM connectors (bytes pre-downloaded).
                let cloud_inputs: Option<Vec<crate::services::attachment::CloudAttachmentInput>> =
                    command_for_execution
                        .payload
                        .params
                        .as_ref()
                        .and_then(|p| p.get("attachments"))
                        .and_then(|v| {
                            serde_json::from_value(v.clone())
                                .map_err(|e| {
                                    eprintln!(
                                        "[CommandPoller] WARNING: Failed to parse cloud attachments (dropping): {}",
                                        e
                                    );
                                    e
                                })
                                .ok()
                        });
                cloud_inputs.map(|inputs| {
                    inputs
                        .into_iter()
                        .take(5)
                        .filter_map(crate::services::attachment::process_cloud_attachment)
                        .collect::<Vec<_>>()
                })
            },
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
                runtime_workspace_id
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
                    description: "Ephemeral agent spawned by Cloud Command".to_string(),
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

        let final_options = if options.allowed_paths.is_none() {
            let workspace_manager = match crate::services::WorkspaceManager::new() {
                Ok(manager) => Arc::new(manager),
                Err(error) => {
                    return CommandResult {
                        success: false,
                        output: None,
                        error: Some(format!(
                            "Failed to initialize workspace manager for effective policy: {}",
                            error
                        )),
                        exit_code: Some(1),
                    };
                }
            };
            let settings = crate::services::SettingsManager::new();
            let effective_policy = crate::services::LocalAgentSecurityService::resolve(
                &workspace_manager,
                &settings,
                &runtime_workspace_id,
                Some(&spec),
            );

            if effective_policy.allowed_paths.is_empty() {
                options
            } else {
                RuntimeOptions {
                    allowed_paths: Some(effective_policy.allowed_paths),
                    ..options
                }
            }
        } else {
            options
        };

        let airlock = airlock_service.read().await.clone();
        let runtime = AgentRuntime::new(
            spec,
            final_options,
            ctx.router.clone(),
            skill_executor,
            memory,
            Arc::new(airlock),
            Some(kill_switch.clone()),
            Some(ctx.runtime_registry.clone()),
        );

        // Start session via SessionCoordinator (creates chat, saves user message, emits session://started)
        let session_coordinator = ctx.session_coordinator.clone();
        let (session_chat_id, session_run_id) = session_coordinator
            .start_remote_session(
                incoming_chat_id.clone(),
                &runtime_workspace_id,
                prompt,
                &connector_id_for_session,
                session_peer.as_deref().unwrap_or("unknown"),
                Some(command.id.clone()),
            )
            .await
            .unwrap_or_else(|e| {
                eprintln!(
                    "[CommandPoller] SessionCoordinator.start_remote_session failed: {}",
                    e
                );
                (
                    uuid::Uuid::new_v4().to_string(),
                    uuid::Uuid::new_v4().to_string(),
                )
            });
        let session_coordinator_for_events = session_coordinator.clone();
        let session_run_id_for_events = session_run_id.clone();

        // Run the agent with bounded event streaming to avoid ATM overload under heavy loops.
        let command_id = command.id.clone();
        let (progress_tx, mut progress_rx) =
            mpsc::channel::<(String, serde_json::Value)>(AGENT_PROGRESS_CHANNEL_CAPACITY);
        let dropped_events = Arc::new(AtomicUsize::new(0));
        let reporter_dropped_events = dropped_events.clone();
        let reporter_service = neural_service.clone();
        let reporter_command_id = command_id.clone();
        let reporter_handle = tokio::spawn(async move {
            let mut throttle = ProgressThrottle::default();
            while let Some((message, data)) = progress_rx.recv().await {
                let dropped_since_last = reporter_dropped_events.swap(0, Ordering::Relaxed);
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

            let trailing_dropped = reporter_dropped_events.swap(0, Ordering::Relaxed);
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
        let audit_agent_id = agent_id.clone();
        match runtime
            .run(prompt, move |event| {
                println!("[Agent Event] {:?}", event);
                let (message, data) = map_agent_event(&event);
                if callback_tx.try_send((message, data)).is_err() {
                    callback_dropped_events.fetch_add(1, Ordering::Relaxed);
                }

                // Emit to frontend for live streaming
                session_coordinator_for_events
                    .emit_agent_event(&session_run_id_for_events, event.clone());

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
                    "workspaceId": runtime_workspace_id,
                    "workspaceName": remote_workspace.as_ref().map(|value| value.display_name.clone()),
                    "bootstrap": remote_workspace.as_ref().map(|value| serde_json::json!({
                        "requestedPath": requested_remote_workspace,
                        "canonicalPath": value.canonical_path,
                        "status": "active",
                        "desktopChatId": session_chat_id,
                    })),
                })
                .to_string();
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
                // Emit session://finished so the frontend clears the active-run indicator
                session_coordinator.abort_session(&session_chat_id);
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
