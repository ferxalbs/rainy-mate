use super::events::{
    AgentEvent, SpecialistCompletedPayload, SpecialistEventPayload, SpecialistFailedPayload,
    SupervisorSummaryPayload,
};
use super::protocol::{SpecialistAssignment, SpecialistOutcome, SpecialistRole, SpecialistStatus, SupervisorPlan};
use super::runtime::{AgentRuntime, RuntimeOptions};
use super::runtime_registry::{RuntimeRegistry, RuntimeRegistryAssignment};
use super::specialist::SpecialistAgent;
use crate::ai::agent::memory::AgentMemory;
use crate::ai::router::IntelligentRouter;
use crate::ai::specs::manifest::{AgentSpec, RuntimeConfig, RuntimeMode};
use crate::services::{agent_kill_switch::AgentKillSwitch, airlock::AirlockService, SkillExecutor};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinSet;

pub struct HierarchicalSupervisorAgent {
    pub spec: AgentSpec,
    pub options: RuntimeOptions,
    pub router: Arc<RwLock<IntelligentRouter>>,
    pub skills: Arc<SkillExecutor>,
    pub memory: Arc<AgentMemory>,
    pub airlock_service: Arc<Option<AirlockService>>,
    pub kill_switch: Option<AgentKillSwitch>,
    pub runtime_registry: Option<Arc<RuntimeRegistry>>,
}

#[derive(Clone)]
struct BranchNode {
    assignment: SpecialistAssignment,
    child: Option<Box<BranchNode>>,
}

#[derive(Clone)]
struct HierarchicalPlan {
    summary: String,
    should_delegate: bool,
    roots: Vec<BranchNode>,
    final_synthesis_required: bool,
}

#[derive(Clone, Serialize)]
struct BranchArtifact {
    agent_id: String,
    role: String,
    depth: u8,
    branch_id: String,
    status: String,
    summary: String,
    response: String,
    tool_count: u32,
    write_like_used: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spawn_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    child: Option<Box<BranchArtifact>>,
}

impl HierarchicalSupervisorAgent {
    const MAX_SYNTHESIS_CONTEXT_CHARS: usize = 14 * 1024;
    const MAX_DEPENDENCY_SNIPPET_CHARS: usize = 2 * 1024;

    fn should_use_research(input: &str) -> bool {
        let input = input.to_ascii_lowercase();
        [
            "research",
            "investigate",
            "review",
            "understand",
            "compare",
            "search",
            "web",
            "browser",
            "analyze",
        ]
        .iter()
        .any(|needle| input.contains(needle))
    }

    fn should_use_executor(input: &str) -> bool {
        let input = input.to_ascii_lowercase();
        [
            "implement",
            "fix",
            "update",
            "create",
            "write",
            "refactor",
            "add",
            "remove",
            "delete",
            "patch",
        ]
        .iter()
        .any(|needle| input.contains(needle))
    }

    fn should_use_memory_scribe(input: &str) -> bool {
        let input = input.to_ascii_lowercase();
        [
            "remember",
            "recall",
            "save",
            "my name",
            "my preference",
            "note that",
            "don't forget",
            "keep in mind",
            "store",
            "memorize",
        ]
        .iter()
        .any(|needle| input.contains(needle))
    }

    fn should_gate_executor_on_research(input: &str) -> bool {
        let input = input.to_ascii_lowercase();
        [
            "first",
            "before",
            "root cause",
            "investigate why",
            "understand current",
            "analyze current",
            "review current",
        ]
        .iter()
        .any(|needle| input.contains(needle))
    }

    fn build_plan_for_runtime(runtime: &RuntimeConfig, input: &str) -> HierarchicalPlan {
        let use_memory = Self::should_use_memory_scribe(input);
        let use_research = Self::should_use_research(input);
        let use_executor = Self::should_use_executor(input);
        let needs_verification = runtime.verification_required && use_executor;
        let should_delegate = use_memory || (use_research && use_executor) || needs_verification;

        if !should_delegate {
            return HierarchicalPlan {
                summary: "Hierarchical supervisor stayed on the main agent".to_string(),
                should_delegate: false,
                roots: Vec::new(),
                final_synthesis_required: false,
            };
        }

        let mut roots = Vec::new();

        if use_research && use_executor {
            let sequential = Self::should_gate_executor_on_research(input);
            let child_role = if needs_verification && !sequential {
                SpecialistRole::Verifier
            } else {
                SpecialistRole::Executor
            };
            let child_title = match child_role {
                SpecialistRole::Verifier => "Verify the resulting state",
                _ => "Implement after the investigation",
            };
            let child_instructions = match child_role {
                SpecialistRole::Verifier => {
                    "Validate the parent branch using read-only checks. Report mismatches and residual risks."
                }
                _ => {
                    "Use the parent branch findings as your primary context. Carry out the requested changes with the minimum necessary edits and verify critical writes."
                }
            };
            roots.push(BranchNode {
                assignment: SpecialistAssignment {
                    agent_id: "research-1".to_string(),
                    role: SpecialistRole::Research,
                    title: "Investigate the task before execution".to_string(),
                    instructions: "Gather evidence, inspect the current implementation, and return concise findings for the next branch.".to_string(),
                    parent_agent_id: None,
                    branch_id: Some("research".to_string()),
                    spawn_reason: Some(if sequential {
                        "research_required_before_execution".to_string()
                    } else {
                        "research_and_execution_split".to_string()
                    }),
                    depth: 1,
                    depends_on: vec![],
                },
                child: Some(Box::new(BranchNode {
                    assignment: SpecialistAssignment {
                        agent_id: match child_role {
                            SpecialistRole::Verifier => "verifier-2".to_string(),
                            _ => "executor-2".to_string(),
                        },
                        role: child_role,
                        title: child_title.to_string(),
                        instructions: child_instructions.to_string(),
                        parent_agent_id: Some("research-1".to_string()),
                        branch_id: Some(if sequential {
                            "research>executor".to_string()
                        } else {
                            "research>verifier".to_string()
                        }),
                        spawn_reason: Some(if sequential {
                            "delegated_from_parent_research".to_string()
                        } else {
                            "verification_child_requested".to_string()
                        }),
                        depth: 2,
                        depends_on: vec!["research-1".to_string()],
                    },
                    child: None,
                })),
            });
        } else if use_executor {
            roots.push(BranchNode {
                assignment: SpecialistAssignment {
                    agent_id: "executor-1".to_string(),
                    role: SpecialistRole::Executor,
                    title: "Execute the requested changes".to_string(),
                    instructions: "Carry out the implementation with the minimum necessary edits and verify critical writes.".to_string(),
                    parent_agent_id: None,
                    branch_id: Some("execution".to_string()),
                    spawn_reason: Some("execution_required".to_string()),
                    depth: 1,
                    depends_on: vec![],
                },
                child: if needs_verification {
                    Some(Box::new(BranchNode {
                        assignment: SpecialistAssignment {
                            agent_id: "verifier-2".to_string(),
                            role: SpecialistRole::Verifier,
                            title: "Verify the resulting state".to_string(),
                            instructions: "Validate the parent branch using read-only checks and report residual risks.".to_string(),
                            parent_agent_id: Some("executor-1".to_string()),
                            branch_id: Some("execution>verification".to_string()),
                            spawn_reason: Some("verification_child_requested".to_string()),
                            depth: 2,
                            depends_on: vec!["executor-1".to_string()],
                        },
                        child: None,
                    }))
                } else {
                    None
                },
            });
        }

        if use_memory {
            roots.push(BranchNode {
                assignment: SpecialistAssignment {
                    agent_id: "memory-scribe-1".to_string(),
                    role: SpecialistRole::MemoryScribe,
                    title: "Persist or recall memory facts".to_string(),
                    instructions: "Persist important facts explicitly stated by the user, or recall them if requested. Return only precise memory facts.".to_string(),
                    parent_agent_id: None,
                    branch_id: Some("memory".to_string()),
                    spawn_reason: Some("memory_required".to_string()),
                    depth: 1,
                    depends_on: vec![],
                },
                child: None,
            });
        }

        HierarchicalPlan {
            summary: "Hierarchical delegation plan activated".to_string(),
            should_delegate: true,
            roots,
            final_synthesis_required: runtime.delegation.final_synthesis_required,
        }
    }

    fn flatten_assignments(nodes: &[BranchNode], out: &mut Vec<SpecialistAssignment>) {
        for node in nodes {
            out.push(node.assignment.clone());
            if let Some(child) = node.child.as_ref() {
                Self::flatten_assignments(std::slice::from_ref(child.as_ref()), out);
            }
        }
    }

    fn registration_assignments(nodes: &[BranchNode]) -> Vec<RuntimeRegistryAssignment> {
        let mut flattened = Vec::new();
        Self::flatten_assignments(nodes, &mut flattened);
        flattened
            .into_iter()
            .map(|assignment| RuntimeRegistryAssignment {
                agent_id: assignment.agent_id,
                role: assignment.role,
                depends_on: assignment.depends_on,
                parent_agent_id: assignment.parent_agent_id,
                branch_id: assignment.branch_id,
                spawn_reason: assignment.spawn_reason,
                depth: Some(assignment.depth),
            })
            .collect()
    }

    fn plan_for_events(plan: &HierarchicalPlan) -> SupervisorPlan {
        let mut assignments = Vec::new();
        Self::flatten_assignments(&plan.roots, &mut assignments);
        SupervisorPlan {
            summary: plan.summary.clone(),
            steps: assignments
                .iter()
                .map(|assignment| {
                    format!(
                        "{} at depth {}",
                        assignment.role.display_name(),
                        assignment.depth.max(1)
                    )
                })
                .collect(),
            assignments,
            verification_required: plan
                .roots
                .iter()
                .any(|node| node.assignment.role == SpecialistRole::Verifier)
                || plan
                    .roots
                    .iter()
                    .any(|node| node.child.as_ref().is_some_and(|child| child.assignment.role == SpecialistRole::Verifier)),
        }
    }

    fn build_child_input(parent: &SpecialistOutcome, original_input: &str) -> String {
        let mut context = parent.response.clone();
        if context.chars().count() > Self::MAX_DEPENDENCY_SNIPPET_CHARS {
            context = context
                .chars()
                .take(Self::MAX_DEPENDENCY_SNIPPET_CHARS)
                .collect();
        }
        format!(
            "Parent branch outcome from {}:\n{}\n\nOriginal user request:\n{}",
            parent.role.display_name(),
            context,
            original_input
        )
    }

    fn emit_status<F>(&self, on_event: &F, assignment: &SpecialistAssignment, status: SpecialistStatus, detail: Option<String>)
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        on_event(AgentEvent::SpecialistStatusChanged(SpecialistEventPayload {
            run_id: "hierarchical".to_string(),
            agent_id: assignment.agent_id.clone(),
            role: assignment.role.clone(),
            status,
            parent_agent_id: assignment.parent_agent_id.clone(),
            branch_id: assignment.branch_id.clone(),
            spawn_reason: assignment.spawn_reason.clone(),
            depth: Some(assignment.depth),
            depends_on: assignment.depends_on.clone(),
            detail,
            active_tool: None,
            started_at_ms: None,
            finished_at_ms: None,
            tool_count: Some(0),
            write_like_used: Some(false),
        }));
    }

    async fn emit_branch_messages<F>(
        assignment: SpecialistAssignment,
        run_id: String,
        mut rx: mpsc::Receiver<super::protocol::SupervisorMessage>,
        on_event: Arc<F>,
        runtime_registry: Option<Arc<RuntimeRegistry>>,
    ) where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        while let Some(message) = rx.recv().await {
            match message {
                super::protocol::SupervisorMessage::SpecialistStarted {
                    started_at_ms, ..
                } => {
                    if let Some(registry) = runtime_registry.as_ref() {
                        registry
                            .update_specialist_status_with_hierarchy(
                                &run_id,
                                &assignment.agent_id,
                                &assignment.role,
                                &SpecialistStatus::Planning,
                                assignment.parent_agent_id.clone(),
                                assignment.branch_id.clone(),
                                assignment.spawn_reason.clone(),
                                Some(assignment.depth),
                                &assignment.depends_on,
                                Some("Specialist started".to_string()),
                                None,
                                Some(started_at_ms),
                                None,
                                Some(0),
                                Some(false),
                            )
                            .await;
                    }
                    on_event(AgentEvent::SpecialistSpawned(SpecialistEventPayload {
                        run_id: run_id.clone(),
                        agent_id: assignment.agent_id.clone(),
                        role: assignment.role.clone(),
                        status: SpecialistStatus::Planning,
                        parent_agent_id: assignment.parent_agent_id.clone(),
                        branch_id: assignment.branch_id.clone(),
                        spawn_reason: assignment.spawn_reason.clone(),
                        depth: Some(assignment.depth),
                        depends_on: assignment.depends_on.clone(),
                        detail: Some("Specialist started".to_string()),
                        active_tool: None,
                        started_at_ms: Some(started_at_ms),
                        finished_at_ms: None,
                        tool_count: Some(0),
                        write_like_used: Some(false),
                    }));
                }
                super::protocol::SupervisorMessage::SpecialistStatus {
                    status,
                    detail,
                    active_tool,
                    started_at_ms,
                    finished_at_ms,
                    tool_count,
                    write_like_used,
                    ..
                } => {
                    if let Some(registry) = runtime_registry.as_ref() {
                        registry
                            .update_specialist_status_with_hierarchy(
                                &run_id,
                                &assignment.agent_id,
                                &assignment.role,
                                &status,
                                assignment.parent_agent_id.clone(),
                                assignment.branch_id.clone(),
                                assignment.spawn_reason.clone(),
                                Some(assignment.depth),
                                &assignment.depends_on,
                                detail.clone(),
                                active_tool.clone(),
                                started_at_ms,
                                finished_at_ms,
                                tool_count,
                                write_like_used,
                            )
                            .await;
                        if active_tool.is_some() {
                            registry.record_tool_use(&assignment.role).await;
                        }
                    }
                    on_event(AgentEvent::SpecialistStatusChanged(SpecialistEventPayload {
                        run_id: run_id.clone(),
                        agent_id: assignment.agent_id.clone(),
                        role: assignment.role.clone(),
                        status,
                        parent_agent_id: assignment.parent_agent_id.clone(),
                        branch_id: assignment.branch_id.clone(),
                        spawn_reason: assignment.spawn_reason.clone(),
                        depth: Some(assignment.depth),
                        depends_on: assignment.depends_on.clone(),
                        detail,
                        active_tool,
                        started_at_ms,
                        finished_at_ms,
                        tool_count,
                        write_like_used,
                    }));
                }
                _ => {}
            }
        }
    }

    async fn execute_single_assignment<F>(
        &self,
        run_id: &str,
        assignment: &SpecialistAssignment,
        input: String,
        on_event: &F,
    ) -> Result<SpecialistOutcome, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        let assignment = assignment.clone();
        self.emit_status(on_event, &assignment, SpecialistStatus::Pending, Some("Queued for execution".to_string()));

        let specialist = SpecialistAgent::new(
            assignment.role.clone(),
            self.spec.clone(),
            self.options.clone(),
            self.router.clone(),
            self.skills.clone(),
            self.memory.clone(),
            self.airlock_service.clone(),
            self.kill_switch.clone(),
        );

        let (tx, rx) = mpsc::channel(128);
        let emitter = tokio::spawn(Self::emit_branch_messages(
            assignment.clone(),
            run_id.to_string(),
            rx,
            Arc::new(on_event.clone()),
            self.runtime_registry.clone(),
        ));

        let outcome = specialist
            .run(run_id, assignment.clone(), input, tx)
            .await;
        let _ = emitter.await;

        match outcome {
            Ok(outcome) => {
                if let Some(registry) = self.runtime_registry.as_ref() {
                    registry
                        .update_specialist_status_with_hierarchy(
                            run_id,
                            &outcome.agent_id,
                            &outcome.role,
                            &SpecialistStatus::Completed,
                            outcome.parent_agent_id.clone(),
                            outcome.branch_id.clone(),
                            outcome.spawn_reason.clone(),
                            Some(outcome.depth),
                            &outcome.depends_on,
                            Some(outcome.summary.clone()),
                            None,
                            Some(outcome.started_at_ms),
                            Some(outcome.finished_at_ms),
                            Some(outcome.tool_count),
                            Some(outcome.used_write_like_tools),
                        )
                        .await;
                }
                on_event(AgentEvent::SpecialistCompleted(SpecialistCompletedPayload {
                    run_id: run_id.to_string(),
                    agent_id: outcome.agent_id.clone(),
                    role: outcome.role.clone(),
                    summary: outcome.summary.clone(),
                    response_preview: outcome.response.chars().take(240).collect(),
                    parent_agent_id: outcome.parent_agent_id.clone(),
                    branch_id: outcome.branch_id.clone(),
                    spawn_reason: outcome.spawn_reason.clone(),
                    depth: Some(outcome.depth),
                    depends_on: outcome.depends_on.clone(),
                    tool_count: outcome.tool_count,
                    write_like_used: outcome.used_write_like_tools,
                    started_at_ms: outcome.started_at_ms,
                    finished_at_ms: outcome.finished_at_ms,
                }));

                Ok(outcome)
            }
            Err(error) => {
                if let Some(registry) = self.runtime_registry.as_ref() {
                    registry
                        .update_specialist_status_with_hierarchy(
                            run_id,
                            &assignment.agent_id,
                            &assignment.role,
                            &SpecialistStatus::Failed,
                            assignment.parent_agent_id.clone(),
                            assignment.branch_id.clone(),
                            assignment.spawn_reason.clone(),
                            Some(assignment.depth),
                            &assignment.depends_on,
                            Some(error.clone()),
                            None,
                            None,
                            Some(Utc::now().timestamp_millis()),
                            None,
                            None,
                        )
                        .await;
                }
                on_event(AgentEvent::SpecialistFailed(SpecialistFailedPayload {
                    run_id: run_id.to_string(),
                    agent_id: assignment.agent_id.clone(),
                    role: assignment.role.clone(),
                    error: error.clone(),
                    parent_agent_id: assignment.parent_agent_id.clone(),
                    branch_id: assignment.branch_id.clone(),
                    spawn_reason: assignment.spawn_reason.clone(),
                    depth: Some(assignment.depth),
                    depends_on: assignment.depends_on.clone(),
                    started_at_ms: None,
                    finished_at_ms: Some(Utc::now().timestamp_millis()),
                    tool_count: None,
                    write_like_used: None,
                }));
                Err(error)
            }
        }
    }

    async fn execute_branch<F>(
        &self,
        run_id: &str,
        node: &BranchNode,
        input: String,
        on_event: &F,
    ) -> Result<BranchArtifact, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        let parent = match self
            .execute_single_assignment(run_id, &node.assignment, input.clone(), on_event)
            .await
        {
            Ok(outcome) => outcome,
            Err(error) => {
                return Ok(BranchArtifact {
                    agent_id: node.assignment.agent_id.clone(),
                    role: node.assignment.role.as_str().to_string(),
                    depth: node.assignment.depth,
                    branch_id: node
                        .assignment
                        .branch_id
                        .clone()
                        .unwrap_or_else(|| node.assignment.role.as_str().to_string()),
                    status: "failed".to_string(),
                    summary: node.assignment.title.clone(),
                    response: error,
                    tool_count: 0,
                    write_like_used: false,
                    parent_agent_id: node.assignment.parent_agent_id.clone(),
                    spawn_reason: node.assignment.spawn_reason.clone(),
                    child: None,
                });
            }
        };

        let child = if let Some(child_node) = node.child.as_ref() {
            match self
                .execute_single_assignment(
                    run_id,
                    &child_node.assignment,
                    Self::build_child_input(&parent, &input),
                    on_event,
                )
                .await
            {
                Ok(child_outcome) => Some(Box::new(BranchArtifact {
                    agent_id: child_outcome.agent_id,
                    role: child_outcome.role.as_str().to_string(),
                    depth: child_outcome.depth,
                    branch_id: child_outcome
                        .branch_id
                        .clone()
                        .unwrap_or_else(|| child_outcome.role.as_str().to_string()),
                    status: "completed".to_string(),
                    summary: child_outcome.summary,
                    response: child_outcome.response,
                    tool_count: child_outcome.tool_count,
                    write_like_used: child_outcome.used_write_like_tools,
                    parent_agent_id: child_outcome.parent_agent_id,
                    spawn_reason: child_outcome.spawn_reason,
                    child: None,
                })),
                Err(error) => Some(Box::new(BranchArtifact {
                    agent_id: child_node.assignment.agent_id.clone(),
                    role: child_node.assignment.role.as_str().to_string(),
                    depth: child_node.assignment.depth,
                    branch_id: child_node
                        .assignment
                        .branch_id
                        .clone()
                        .unwrap_or_else(|| child_node.assignment.role.as_str().to_string()),
                    status: "failed".to_string(),
                    summary: child_node.assignment.title.clone(),
                    response: error,
                    tool_count: 0,
                    write_like_used: false,
                    parent_agent_id: child_node.assignment.parent_agent_id.clone(),
                    spawn_reason: child_node.assignment.spawn_reason.clone(),
                    child: None,
                })),
            }
        } else {
            None
        };

        Ok(BranchArtifact {
            agent_id: parent.agent_id,
            role: parent.role.as_str().to_string(),
            depth: parent.depth,
            branch_id: parent
                .branch_id
                .clone()
                .unwrap_or_else(|| parent.role.as_str().to_string()),
            status: "completed".to_string(),
            summary: parent.summary,
            response: parent.response,
            tool_count: parent.tool_count,
            write_like_used: parent.used_write_like_tools,
            parent_agent_id: parent.parent_agent_id,
            spawn_reason: parent.spawn_reason,
            child,
        })
    }

    fn build_synthesis_payload(input: &str, artifacts: &[BranchArtifact]) -> String {
        let mut payload = serde_json::json!({
            "user_request": input,
            "branch_results": artifacts,
        })
        .to_string();
        if payload.chars().count() > Self::MAX_SYNTHESIS_CONTEXT_CHARS {
            payload = payload
                .chars()
                .take(Self::MAX_SYNTHESIS_CONTEXT_CHARS)
                .collect();
        }
        payload
    }

    async fn synthesize_with_main_agent<F>(
        &self,
        input: &str,
        artifacts: &[BranchArtifact],
        on_event: F,
    ) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        let mut synthesis_spec = self.spec.clone();
        synthesis_spec.runtime.mode = RuntimeMode::Single;
        synthesis_spec.airlock.tool_policy.mode = "allowlist".to_string();
        synthesis_spec.airlock.tool_policy.allow.clear();
        synthesis_spec.airlock.tool_policy.deny.clear();

        let mut synthesis_options = self.options.clone();
        synthesis_options.custom_system_prompt = Some(format!(
            "You are the principal coordinating agent.\n\
Internal coordination language: English.\n\
Final response language: match the user.\n\
You are receiving structured branch artifacts from delegated specialists.\n\
Return one complete, precise answer for the user.\n\
- Reconcile overlapping findings.\n\
- Lead with what actually happened.\n\
- If any branch failed, explain the limitation clearly.\n\
- Do not call tools.\n\
- Do not mention internal chain mechanics unless relevant.\n\
- Use concise prose and preserve file references or concrete evidence when present."
        ));

        let runtime = AgentRuntime::new(
            synthesis_spec,
            synthesis_options,
            self.router.clone(),
            self.skills.clone(),
            self.memory.clone(),
            self.airlock_service.clone(),
            self.kill_switch.clone(),
            None,
        );

        runtime
            .run_single(&Self::build_synthesis_payload(input, artifacts), on_event)
            .await
    }

    async fn fallback_to_single<F>(&self, input: &str, on_event: F) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        let mut single_spec = self.spec.clone();
        single_spec.runtime.mode = RuntimeMode::Single;
        let runtime = AgentRuntime::new(
            single_spec,
            self.options.clone(),
            self.router.clone(),
            self.skills.clone(),
            self.memory.clone(),
            self.airlock_service.clone(),
            self.kill_switch.clone(),
            self.runtime_registry.clone(),
        );
        runtime.run_single(input, on_event).await
    }

    pub async fn run<F>(&self, input: &str, on_event: F) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        let plan = Self::build_plan_for_runtime(&self.spec.runtime, input);
        if !plan.should_delegate {
            return self.fallback_to_single(input, on_event).await;
        }

        let run_id = uuid::Uuid::new_v4().to_string();
        let event_plan = Self::plan_for_events(&plan);
        on_event(AgentEvent::SupervisorPlanCreated(event_plan));

        if let Some(registry) = self.runtime_registry.as_ref() {
            registry
                .start_hierarchical_run(&run_id, &Self::registration_assignments(&plan.roots))
                .await;
            registry.update_supervisor_status(&run_id, "running").await;
        }

        let max_parallel = self
            .spec
            .runtime
            .delegation
            .max_parallel_subagents
            .clamp(1, self.spec.runtime.delegation.max_threads.max(1)) as usize;

        let mut join_set: JoinSet<Result<BranchArtifact, String>> = JoinSet::new();
        let mut roots_iter = plan.roots.iter();
        let mut artifacts = Vec::new();

        for _ in 0..max_parallel {
            if let Some(root) = roots_iter.next() {
                let root = root.clone();
                let input = input.to_string();
                let this = self.clone_for_spawn();
                let on_event_clone = on_event.clone();
                let run_id_clone = run_id.clone();
                join_set.spawn(async move {
                    this.execute_branch(&run_id_clone, &root, input, &on_event_clone).await
                });
            }
        }

        while let Some(joined) = join_set.join_next().await {
            let artifact = joined.map_err(|e| format!("Hierarchical branch join failure: {}", e))??;
            artifacts.push(artifact);
            if let Some(root) = roots_iter.next() {
                let root = root.clone();
                let input = input.to_string();
                let this = self.clone_for_spawn();
                let on_event_clone = on_event.clone();
                let run_id_clone = run_id.clone();
                join_set.spawn(async move {
                    this.execute_branch(&run_id_clone, &root, input, &on_event_clone).await
                });
            }
        }

        let summary = if plan.final_synthesis_required {
            self.synthesize_with_main_agent(input, &artifacts, on_event.clone())
                .await?
        } else {
            artifacts
                .iter()
                .map(|artifact| format!("[{}] {}", artifact.role, artifact.summary))
                .collect::<Vec<_>>()
                .join("\n")
        };

        on_event(AgentEvent::SupervisorSummary(SupervisorSummaryPayload {
            run_id: run_id.clone(),
            summary: summary.clone(),
        }));

        if let Some(registry) = self.runtime_registry.as_ref() {
            let final_status = if artifacts.iter().all(|artifact| artifact.status == "completed") {
                "completed"
            } else {
                "failed"
            };
            registry.finish_supervisor_run(&run_id, final_status).await;
        }

        Ok(summary)
    }

    fn clone_for_spawn(&self) -> Self {
        Self {
            spec: self.spec.clone(),
            options: self.options.clone(),
            router: self.router.clone(),
            skills: self.skills.clone(),
            memory: self.memory.clone(),
            airlock_service: self.airlock_service.clone(),
            kill_switch: self.kill_switch.clone(),
            runtime_registry: self.runtime_registry.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::specs::manifest::RuntimeConfig;

    fn runtime_config() -> RuntimeConfig {
        RuntimeConfig {
            mode: RuntimeMode::HierarchicalSupervisor,
            max_specialists: 3,
            verification_required: true,
            ..Default::default()
        }
    }

    #[test]
    fn plan_stays_on_main_agent_for_simple_requests() {
        let plan = HierarchicalSupervisorAgent::build_plan_for_runtime(
            &runtime_config(),
            "Summarize this file for me",
        );
        assert!(!plan.should_delegate);
        assert!(plan.roots.is_empty());
    }

    #[test]
    fn plan_builds_research_to_executor_chain_when_ordered() {
        let plan = HierarchicalSupervisorAgent::build_plan_for_runtime(
            &runtime_config(),
            "Investigate the root cause first and implement the fix",
        );
        assert!(plan.should_delegate);
        assert_eq!(plan.roots.len(), 1);
        let root = &plan.roots[0];
        assert_eq!(root.assignment.role, SpecialistRole::Research);
        let child = root.child.as_ref().expect("child branch");
        assert_eq!(child.assignment.role, SpecialistRole::Executor);
        assert_eq!(child.assignment.depth, 2);
    }

    #[test]
    fn plan_builds_executor_to_verifier_chain_for_execution_only() {
        let plan = HierarchicalSupervisorAgent::build_plan_for_runtime(
            &runtime_config(),
            "Implement the patch and update the file",
        );
        assert!(plan.should_delegate);
        assert_eq!(plan.roots.len(), 1);
        let root = &plan.roots[0];
        assert_eq!(root.assignment.role, SpecialistRole::Executor);
        let child = root.child.as_ref().expect("verifier child");
        assert_eq!(child.assignment.role, SpecialistRole::Verifier);
    }

    #[test]
    fn plan_for_events_flattens_hierarchy() {
        let plan = HierarchicalSupervisorAgent::build_plan_for_runtime(
            &runtime_config(),
            "Investigate first and implement the fix",
        );
        let event_plan = HierarchicalSupervisorAgent::plan_for_events(&plan);
        assert_eq!(event_plan.assignments.len(), 2);
        assert_eq!(event_plan.assignments[0].depth, 1);
        assert_eq!(event_plan.assignments[1].depth, 2);
    }
}
