use super::events::{
    AgentEvent, SpecialistCompletedPayload, SpecialistEventPayload, SpecialistFailedPayload,
    SupervisorSummaryPayload,
};
use super::protocol::{
    SpecialistAssignment, SpecialistOutcome, SpecialistRole, SpecialistStatus, SupervisorMessage,
    SupervisorPlan,
};
use super::runtime::RuntimeOptions;
use super::runtime_registry::RuntimeRegistry;
use super::specialist::SpecialistAgent;
use crate::ai::agent::memory::AgentMemory;
use crate::ai::router::IntelligentRouter;
use crate::ai::specs::manifest::{AgentSpec, RuntimeConfig};
use crate::services::{agent_kill_switch::AgentKillSwitch, airlock::AirlockService, SkillExecutor};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::task::JoinSet;
use tokio::sync::{mpsc, RwLock};

pub struct SupervisorAgent {
    pub spec: AgentSpec,
    pub options: RuntimeOptions,
    pub router: Arc<RwLock<IntelligentRouter>>,
    pub skills: Arc<SkillExecutor>,
    pub memory: Arc<AgentMemory>,
    pub airlock_service: Arc<Option<AirlockService>>,
    pub kill_switch: Option<AgentKillSwitch>,
    pub runtime_registry: Option<Arc<RuntimeRegistry>>,
}

impl SupervisorAgent {
    const MAX_DEPENDENCY_CONTEXT_CHARS: usize = 4 * 1024;

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
            "openclaw",
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
            "what do you know",
            "what did i tell",
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

    fn build_plan(&self, input: &str) -> SupervisorPlan {
        Self::build_plan_for_runtime(&self.spec.runtime, input)
    }

    fn build_plan_for_runtime(runtime: &RuntimeConfig, input: &str) -> SupervisorPlan {
        let mut steps = vec!["Assess the request and allocate specialist roles".to_string()];
        let mut base_assignments = Vec::new();

        let use_memory_scribe = Self::should_use_memory_scribe(input);
        let use_research = Self::should_use_research(input)
            || (!Self::should_use_executor(input) && !use_memory_scribe);
        let use_executor = Self::should_use_executor(input);

        if use_research {
            base_assignments.push(SpecialistAssignment {
                agent_id: "research-1".to_string(),
                role: SpecialistRole::Research,
                title: "Gather supporting context".to_string(),
                instructions: "Review relevant code, docs, and external references needed to execute the task safely.".to_string(),
                depends_on: vec![],
            });
            steps.push("Research Agent reviews code and supporting references".to_string());
        }

        let executor_depends_on = if use_research && Self::should_gate_executor_on_research(input) {
            vec!["research-1".to_string()]
        } else {
            vec![]
        };

        if use_executor {
            base_assignments.push(SpecialistAssignment {
                agent_id: "executor-1".to_string(),
                role: SpecialistRole::Executor,
                title: "Execute the requested changes".to_string(),
                instructions: "Carry out the implementation work using the minimum necessary edits and tool calls.".to_string(),
                depends_on: executor_depends_on.clone(),
            });
            steps.push(if executor_depends_on.is_empty() {
                "Executor Agent performs workspace actions while Research Agent gathers parallel context".to_string()
            } else {
                "Executor Agent performs the requested workspace actions after research context is complete".to_string()
            });
        }

        if use_memory_scribe {
            base_assignments.push(SpecialistAssignment {
                agent_id: "memory-scribe-1".to_string(),
                role: SpecialistRole::MemoryScribe,
                title: "Persist or recall memory facts".to_string(),
                instructions: "Identify every fact, name, preference, or piece of user context explicitly stated in this request. Save each one with save_memory and appropriate tags. If retrieval was requested, use recall_memory and return the results.".to_string(),
                depends_on: vec![],
            });
            steps.push("Memory Scribe persists facts and user context to long-term memory".to_string());
        }

        let has_executor = base_assignments
            .iter()
            .any(|assignment| assignment.role == SpecialistRole::Executor);
        let max_specialists = runtime.max_specialists.clamp(1, 4) as usize;
        let verification_required = runtime.verification_required
            && has_executor
            && max_specialists >= 2;

        let max_non_verifier = if verification_required {
            max_specialists.saturating_sub(1).max(1)
        } else {
            max_specialists
        };

        let mut assignments: Vec<SpecialistAssignment> = if base_assignments.len() <= max_non_verifier {
            base_assignments
        } else {
            let mut prioritized = Vec::new();
            if let Some(exec) = base_assignments
                .iter()
                .find(|assignment| assignment.role == SpecialistRole::Executor)
                .cloned()
            {
                prioritized.push(exec);
            }
            if prioritized.len() < max_non_verifier {
                if let Some(research) = base_assignments
                    .iter()
                    .find(|assignment| assignment.role == SpecialistRole::Research)
                    .cloned()
                {
                    prioritized.push(research);
                }
            }
            prioritized
        };

        if verification_required
            && assignments
                .iter()
                .any(|assignment| assignment.role == SpecialistRole::Executor)
        {
            assignments.push(SpecialistAssignment {
                agent_id: "verifier-1".to_string(),
                role: SpecialistRole::Verifier,
                title: "Verify the resulting state".to_string(),
                instructions: "Validate the final state using read-only checks and report any mismatch or residual risk.".to_string(),
                depends_on: vec!["executor-1".to_string()],
            });
            steps.push("Verifier Agent validates the resulting state with read-only checks".to_string());
        }

        SupervisorPlan {
            summary: "Supervisor orchestration activated".to_string(),
            steps,
            assignments,
            verification_required,
        }
    }

    fn should_run_verifier(outcomes: &[SpecialistOutcome]) -> bool {
        outcomes.iter().any(|outcome| {
            outcome.role == SpecialistRole::Executor && outcome.used_write_like_tools
        })
    }

    fn synthesize_final_response(
        plan: &SupervisorPlan,
        outcomes: &[SpecialistOutcome],
        failures: &[LaneFailure],
    ) -> String {
        let mut sections = vec![format!("Supervisor completed {} specialist lane(s).", plan.assignments.len())];
        let outcomes_by_id: HashMap<&str, &SpecialistOutcome> = outcomes
            .iter()
            .map(|outcome| (outcome.agent_id.as_str(), outcome))
            .collect();
        let failures_by_id: HashMap<&str, &LaneFailure> = failures
            .iter()
            .map(|failure| (failure.agent_id.as_str(), failure))
            .collect();

        for assignment in &plan.assignments {
            if let Some(outcome) = outcomes_by_id.get(assignment.agent_id.as_str()) {
                sections.push(format!(
                    "[{}] {}",
                    outcome.role.display_name(),
                    outcome.response.trim()
                ));
            }
        }

        if !failures.is_empty() {
            let failure_lines = plan
                .assignments
                .iter()
                .filter_map(|assignment| {
                    failures_by_id
                        .get(assignment.agent_id.as_str())
                        .map(|failure| {
                            format!("[{}] {}", failure.role.display_name(), failure.error)
                        })
                })
                .collect::<Vec<_>>()
                .join("\n");
            sections.push(format!("Failures:\n{}", failure_lines));
        }

        sections.join("\n\n")
    }

    fn truncate_chars(input: &str, max_chars: usize) -> String {
        if input.chars().count() <= max_chars {
            return input.to_string();
        }

        let mut truncated = input.chars().take(max_chars).collect::<String>();
        truncated.push_str("\n[TRUNCATED]");
        truncated
    }

    fn build_dependency_context(
        assignment: &SpecialistAssignment,
        completed_outcomes: &HashMap<String, SpecialistOutcome>,
    ) -> Option<String> {
        if assignment.depends_on.is_empty() {
            return None;
        }

        let mut sections = Vec::new();
        for dependency_id in &assignment.depends_on {
            let outcome = completed_outcomes.get(dependency_id)?;
            sections.push(format!(
                "[{}]\n{}",
                outcome.role.display_name(),
                Self::truncate_chars(&outcome.response, 1200)
            ));
        }

        let context = format!(
            "Prior lane outcomes:\n{}",
            sections.join("\n\n")
        );
        Some(Self::truncate_chars(
            &context,
            Self::MAX_DEPENDENCY_CONTEXT_CHARS,
        ))
    }

    fn build_specialist_input(
        base_input: &str,
        assignment: &SpecialistAssignment,
        completed_outcomes: &HashMap<String, SpecialistOutcome>,
    ) -> String {
        match Self::build_dependency_context(assignment, completed_outcomes) {
            Some(context) => format!("{}\n\n{}", context, base_input),
            None => base_input.to_string(),
        }
    }

    fn missing_dependencies(
        assignment: &SpecialistAssignment,
        completed_outcomes: &HashMap<String, SpecialistOutcome>,
    ) -> Vec<String> {
        assignment
            .depends_on
            .iter()
            .filter(|dependency_id| !completed_outcomes.contains_key(*dependency_id))
            .cloned()
            .collect()
    }

    fn failed_dependencies(
        assignment: &SpecialistAssignment,
        failed_agent_ids: &HashSet<String>,
    ) -> Vec<String> {
        assignment
            .depends_on
            .iter()
            .filter(|dependency_id| failed_agent_ids.contains(*dependency_id))
            .cloned()
            .collect()
    }

    async fn emit_messages<F>(
        mut rx: mpsc::Receiver<SupervisorMessage>,
        on_event: Arc<F>,
        runtime_registry: Option<Arc<RuntimeRegistry>>,
    ) where
        F: Fn(AgentEvent) + Send + Sync + 'static,
    {
        while let Some(message) = rx.recv().await {
            match message {
                SupervisorMessage::SpecialistStarted {
                    run_id,
                    agent_id,
                    role,
                    depends_on,
                    started_at_ms,
                } => {
                    if let Some(registry) = runtime_registry.as_ref() {
                        registry
                            .update_specialist_status(
                                &run_id,
                                &agent_id,
                                &role,
                                &SpecialistStatus::Planning,
                                &depends_on,
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
                        run_id,
                        agent_id,
                        role,
                        status: SpecialistStatus::Planning,
                        depends_on,
                        detail: Some("Specialist started".to_string()),
                        active_tool: None,
                        started_at_ms: Some(started_at_ms),
                        finished_at_ms: None,
                        tool_count: Some(0),
                        write_like_used: Some(false),
                    }));
                }
                SupervisorMessage::SpecialistStatus {
                    run_id,
                    agent_id,
                    role,
                    status,
                    detail,
                    active_tool,
                    depends_on,
                    started_at_ms,
                    finished_at_ms,
                    tool_count,
                    write_like_used,
                } => {
                    if let Some(registry) = runtime_registry.as_ref() {
                        registry
                            .update_specialist_status(
                                &run_id,
                                &agent_id,
                                &role,
                                &status,
                                &depends_on,
                                detail.clone(),
                                active_tool.clone(),
                                started_at_ms,
                                finished_at_ms,
                                tool_count,
                                write_like_used,
                            )
                            .await;
                        if active_tool.is_some() {
                            registry.record_tool_use(&role).await;
                        }
                    }
                    on_event(AgentEvent::SpecialistStatusChanged(SpecialistEventPayload {
                        run_id,
                        agent_id,
                        role,
                        status,
                        depends_on,
                        detail,
                        active_tool,
                        started_at_ms,
                        finished_at_ms,
                        tool_count,
                        write_like_used,
                    }));
                }
                SupervisorMessage::SpecialistCompleted { run_id, outcome } => {
                    if let Some(registry) = runtime_registry.as_ref() {
                        registry
                            .update_specialist_status(
                                &run_id,
                                &outcome.agent_id,
                                &outcome.role,
                                &SpecialistStatus::Completed,
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
                        run_id,
                        agent_id: outcome.agent_id,
                        role: outcome.role,
                        summary: outcome.summary,
                        response_preview: outcome.response.chars().take(240).collect(),
                        depends_on: outcome.depends_on,
                        tool_count: outcome.tool_count,
                        write_like_used: outcome.used_write_like_tools,
                        started_at_ms: outcome.started_at_ms,
                        finished_at_ms: outcome.finished_at_ms,
                    }));
                }
                SupervisorMessage::SpecialistFailed {
                    run_id,
                    agent_id,
                    role,
                    error,
                    depends_on,
                    started_at_ms,
                    finished_at_ms,
                    tool_count,
                    write_like_used,
                } => {
                    if let Some(registry) = runtime_registry.as_ref() {
                        registry
                            .update_specialist_status(
                                &run_id,
                                &agent_id,
                                &role,
                                &SpecialistStatus::Failed,
                                &depends_on,
                                Some(error.clone()),
                                None,
                                started_at_ms,
                                finished_at_ms,
                                tool_count,
                                write_like_used,
                            )
                            .await;
                    }
                    on_event(AgentEvent::SpecialistFailed(SpecialistFailedPayload {
                        run_id,
                        agent_id,
                        role,
                        error,
                        depends_on,
                        started_at_ms,
                        finished_at_ms,
                        tool_count,
                        write_like_used,
                    }));
                }
            }
        }
    }

    async fn emit_pending_assignments(
        run_id: &str,
        tx: &mpsc::Sender<SupervisorMessage>,
        assignments: &[SpecialistAssignment],
    ) {
        for assignment in assignments {
            let detail = if assignment.depends_on.is_empty() {
                "Queued for execution".to_string()
            } else {
                format!(
                    "Waiting on dependencies: {}",
                    assignment.depends_on.join(", ")
                )
            };
            let _ = tx
                .send(SupervisorMessage::SpecialistStatus {
                    run_id: run_id.to_string(),
                    agent_id: assignment.agent_id.clone(),
                    role: assignment.role.clone(),
                    status: SpecialistStatus::Pending,
                    detail: Some(detail),
                    active_tool: None,
                    depends_on: assignment.depends_on.clone(),
                    started_at_ms: None,
                    finished_at_ms: None,
                    tool_count: Some(0),
                    write_like_used: Some(false),
                })
                .await;
        }
    }

    pub async fn run<F>(&self, input: &str, on_event: F) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        let run_id = uuid::Uuid::new_v4().to_string();
        let plan = self.build_plan(input);
        let assignments_for_registry: Vec<(String, SpecialistRole, Vec<String>)> = plan
            .assignments
            .iter()
            .map(|assignment| {
                (
                    assignment.agent_id.clone(),
                    assignment.role.clone(),
                    assignment.depends_on.clone(),
                )
            })
            .collect();

        if let Some(registry) = self.runtime_registry.as_ref() {
            registry
                .start_supervisor_run(&run_id, &assignments_for_registry)
                .await;
            registry.update_supervisor_status(&run_id, "planning").await;
        }

        on_event(AgentEvent::SupervisorPlanCreated(plan.clone()));

        let (tx, rx) = mpsc::channel::<SupervisorMessage>(128);
        let on_event_arc = Arc::new(on_event.clone());
        let registry_for_events = self.runtime_registry.clone();
        let emitter = tokio::spawn(Self::emit_messages(rx, on_event_arc, registry_for_events));
        Self::emit_pending_assignments(&run_id, &tx, &plan.assignments).await;

        let mut outcomes = Vec::new();
        let mut completed_outcomes: HashMap<String, SpecialistOutcome> = HashMap::new();
        let mut failures: Vec<LaneFailure> = Vec::new();
        let mut failed_agent_ids: HashSet<String> = HashSet::new();
        let max_parallel = self.spec.runtime.max_specialists.clamp(1, 4) as usize;

        if let Some(registry) = self.runtime_registry.as_ref() {
            registry.update_supervisor_status(&run_id, "running").await;
        }
        let mut remaining: HashMap<String, SpecialistAssignment> = plan
            .assignments
            .clone()
            .into_iter()
            .filter(|a| a.role != SpecialistRole::Verifier)
            .map(|assignment| (assignment.agent_id.clone(), assignment))
            .collect();
        let mut running_ids: HashSet<String> = HashSet::new();
        let mut join_set: JoinSet<(SpecialistAssignment, Result<SpecialistOutcome, String>)> =
            JoinSet::new();

        loop {
            let skipped_ids: Vec<String> = remaining
                .values()
                .filter(|assignment| {
                    !Self::failed_dependencies(assignment, &failed_agent_ids).is_empty()
                })
                .map(|assignment| assignment.agent_id.clone())
                .collect();

            for skipped_id in skipped_ids {
                if let Some(assignment) = remaining.remove(&skipped_id) {
                    let failed_dependencies =
                        Self::failed_dependencies(&assignment, &failed_agent_ids);
                    let error = format!(
                        "Skipped because required prior lane(s) failed: {}",
                        failed_dependencies.join(", ")
                    );
                    failures.push(LaneFailure {
                        agent_id: assignment.agent_id.clone(),
                        role: assignment.role.clone(),
                        error: error.clone(),
                    });
                    failed_agent_ids.insert(assignment.agent_id.clone());
                    tx.send(SupervisorMessage::SpecialistFailed {
                        run_id: run_id.clone(),
                        agent_id: assignment.agent_id,
                        role: assignment.role,
                        error,
                        depends_on: assignment.depends_on,
                        started_at_ms: None,
                        finished_at_ms: Some(Utc::now().timestamp_millis()),
                        tool_count: Some(0),
                        write_like_used: Some(false),
                    })
                    .await
                    .ok();
                }
            }

            let ready_ids: Vec<String> = remaining
                .values()
                .filter(|assignment| {
                    !running_ids.contains(&assignment.agent_id)
                        && Self::missing_dependencies(assignment, &completed_outcomes).is_empty()
                })
                .map(|assignment| assignment.agent_id.clone())
                .collect();

            for ready_id in ready_ids {
                if join_set.len() >= max_parallel {
                    break;
                }
                let Some(assignment) = remaining.remove(&ready_id) else {
                    continue;
                };
                running_ids.insert(assignment.agent_id.clone());
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
                let specialist_input =
                    Self::build_specialist_input(input, &assignment, &completed_outcomes);
                let tx_clone = tx.clone();
                let run_id_clone = run_id.clone();
                let assignment_clone = assignment.clone();
                join_set.spawn(async move {
                    let result = specialist
                        .run(&run_id_clone, assignment_clone.clone(), specialist_input, tx_clone)
                        .await;
                    (assignment_clone, result)
                });
            }

            if join_set.is_empty() {
                if remaining.is_empty() {
                    break;
                }

                for assignment in remaining.into_values() {
                    let unresolved =
                        Self::missing_dependencies(&assignment, &completed_outcomes);
                    let error = if unresolved.is_empty() {
                        "Lane could not be scheduled".to_string()
                    } else {
                        format!(
                            "Lane stalled waiting on unresolved dependencies: {}",
                            unresolved.join(", ")
                        )
                    };
                    failures.push(LaneFailure {
                        agent_id: assignment.agent_id.clone(),
                        role: assignment.role.clone(),
                        error: error.clone(),
                    });
                    failed_agent_ids.insert(assignment.agent_id.clone());
                    tx.send(SupervisorMessage::SpecialistFailed {
                        run_id: run_id.clone(),
                        agent_id: assignment.agent_id,
                        role: assignment.role,
                        error,
                        depends_on: assignment.depends_on,
                        started_at_ms: None,
                        finished_at_ms: Some(Utc::now().timestamp_millis()),
                        tool_count: Some(0),
                        write_like_used: Some(false),
                    })
                    .await
                    .ok();
                }
                break;
            }

            if let Some(joined) = join_set.join_next().await {
                match joined {
                    Ok((assignment, Ok(outcome))) => {
                        running_ids.remove(&assignment.agent_id);
                        tx.send(SupervisorMessage::SpecialistCompleted {
                            run_id: run_id.clone(),
                            outcome: outcome.clone(),
                        })
                        .await
                        .ok();
                        completed_outcomes.insert(assignment.agent_id.clone(), outcome.clone());
                        outcomes.push(outcome);
                    }
                    Ok((assignment, Err(error))) => {
                        running_ids.remove(&assignment.agent_id);
                        failed_agent_ids.insert(assignment.agent_id.clone());
                        failures.push(LaneFailure {
                            agent_id: assignment.agent_id.clone(),
                            role: assignment.role.clone(),
                            error: error.clone(),
                        });
                        tx.send(SupervisorMessage::SpecialistFailed {
                            run_id: run_id.clone(),
                            agent_id: assignment.agent_id,
                            role: assignment.role,
                            error,
                            depends_on: assignment.depends_on,
                            started_at_ms: None,
                            finished_at_ms: Some(Utc::now().timestamp_millis()),
                            tool_count: None,
                            write_like_used: None,
                        })
                        .await
                        .ok();
                    }
                    Err(join_error) => {
                        let error = format!("Specialist task join failure: {}", join_error);
                        failures.push(LaneFailure {
                            agent_id: "unknown".to_string(),
                            role: SpecialistRole::Executor,
                            error,
                        });
                    }
                }
            }
        }

        if plan.verification_required && Self::should_run_verifier(&outcomes) {
            if let Some(registry) = self.runtime_registry.as_ref() {
                registry.update_supervisor_status(&run_id, "verifying").await;
            }
            on_event(AgentEvent::SpecialistStatusChanged(SpecialistEventPayload {
                run_id: run_id.clone(),
                agent_id: "verifier-1".to_string(),
                role: SpecialistRole::Verifier,
                status: SpecialistStatus::Verifying,
                depends_on: vec!["executor-1".to_string()],
                detail: Some("Verifier Agent validating resulting state".to_string()),
                active_tool: None,
                started_at_ms: None,
                finished_at_ms: None,
                tool_count: Some(0),
                write_like_used: Some(false),
            }));
            if let Some(assignment) = plan
                .assignments
                .iter()
                .find(|assignment| assignment.role == SpecialistRole::Verifier)
                .cloned()
            {
                let missing_dependencies =
                    Self::missing_dependencies(&assignment, &completed_outcomes);
                if !missing_dependencies.is_empty() {
                    let error = format!(
                        "Skipped because required prior lane(s) did not complete successfully: {}",
                        missing_dependencies.join(", ")
                    );
                    failures.push(LaneFailure {
                        agent_id: assignment.agent_id.clone(),
                        role: SpecialistRole::Verifier,
                        error: error.clone(),
                    });
                    tx.send(SupervisorMessage::SpecialistFailed {
                        run_id: run_id.clone(),
                        agent_id: assignment.agent_id,
                        role: SpecialistRole::Verifier,
                        error,
                        depends_on: assignment.depends_on,
                        started_at_ms: None,
                        finished_at_ms: Some(Utc::now().timestamp_millis()),
                        tool_count: Some(0),
                        write_like_used: Some(false),
                    })
                    .await
                    .ok();
                    drop(tx);
                    let _ = emitter.await;

                    let summary = Self::synthesize_final_response(&plan, &outcomes, &failures);
                    on_event(AgentEvent::SupervisorSummary(SupervisorSummaryPayload {
                        run_id: run_id.clone(),
                        summary: summary.clone(),
                    }));
                    if let Some(registry) = self.runtime_registry.as_ref() {
                        registry.finish_supervisor_run(&run_id, "failed").await;
                    }
                    return Ok(summary);
                }

                let verifier = SpecialistAgent::new(
                    SpecialistRole::Verifier,
                    self.spec.clone(),
                    self.options.clone(),
                    self.router.clone(),
                    self.skills.clone(),
                    self.memory.clone(),
                    self.airlock_service.clone(),
                    self.kill_switch.clone(),
                );
                let verifier_input =
                    Self::build_specialist_input(input, &assignment, &completed_outcomes);
                match verifier
                    .run(&run_id, assignment.clone(), verifier_input, tx.clone())
                    .await
                {
                    Ok(outcome) => {
                        tx.send(SupervisorMessage::SpecialistCompleted {
                            run_id: run_id.clone(),
                            outcome: outcome.clone(),
                        })
                        .await
                        .ok();
                        completed_outcomes.insert(assignment.agent_id.clone(), outcome.clone());
                        outcomes.push(outcome);
                    }
                    Err(error) => {
                        failures.push(LaneFailure {
                            agent_id: assignment.agent_id.clone(),
                            role: SpecialistRole::Verifier,
                            error: error.clone(),
                        });
                        tx.send(SupervisorMessage::SpecialistFailed {
                            run_id: run_id.clone(),
                            agent_id: assignment.agent_id,
                            role: SpecialistRole::Verifier,
                            error,
                            depends_on: assignment.depends_on,
                            started_at_ms: None,
                            finished_at_ms: Some(Utc::now().timestamp_millis()),
                            tool_count: None,
                            write_like_used: None,
                        })
                        .await
                        .ok();
                    }
                }
            }
        } else if plan.verification_required {
            if let Some(assignment) = plan
                .assignments
                .iter()
                .find(|assignment| assignment.role == SpecialistRole::Verifier)
            {
                tx.send(SupervisorMessage::SpecialistStatus {
                    run_id: run_id.clone(),
                    agent_id: assignment.agent_id.clone(),
                    role: SpecialistRole::Verifier,
                    status: SpecialistStatus::Completed,
                    detail: Some(
                        "Verification skipped: executor completed without write-like actions"
                            .to_string(),
                    ),
                    active_tool: None,
                    depends_on: assignment.depends_on.clone(),
                    started_at_ms: None,
                    finished_at_ms: Some(Utc::now().timestamp_millis()),
                    tool_count: Some(0),
                    write_like_used: Some(false),
                })
                .await
                .ok();
            }
        }

        drop(tx);
        let _ = emitter.await;

        let summary = Self::synthesize_final_response(&plan, &outcomes, &failures);
        on_event(AgentEvent::SupervisorSummary(SupervisorSummaryPayload {
            run_id: run_id.clone(),
            summary: summary.clone(),
        }));
        if let Some(registry) = self.runtime_registry.as_ref() {
            let final_status = if failures.is_empty() {
                "completed"
            } else {
                "failed"
            };
            registry.finish_supervisor_run(&run_id, final_status).await;
        }
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::specs::manifest::{RuntimeConfig, RuntimeMode};

    fn test_runtime(max_specialists: u8, verification_required: bool) -> RuntimeConfig {
        RuntimeConfig {
            mode: RuntimeMode::Supervisor,
            max_specialists,
            verification_required,
        }
    }

    #[test]
    fn build_plan_prioritizes_executor_when_limited() {
        let runtime = test_runtime(2, true);
        let plan = SupervisorAgent::build_plan_for_runtime(&runtime, "research and implement feature x");
        assert!(plan.assignments.iter().any(|a| a.role == SpecialistRole::Executor));
        assert!(plan.assignments.iter().any(|a| a.role == SpecialistRole::Verifier));
        assert_eq!(plan.assignments.len(), 2);
    }

    #[test]
    fn build_plan_makes_executor_wait_for_research() {
        let runtime = test_runtime(3, true);
        let plan = SupervisorAgent::build_plan_for_runtime(&runtime, "research the root cause first and implement feature x");
        let executor = plan
            .assignments
            .iter()
            .find(|assignment| assignment.role == SpecialistRole::Executor)
            .expect("executor assignment");
        assert_eq!(executor.depends_on, vec!["research-1".to_string()]);
    }

    #[test]
    fn build_plan_can_disable_verifier_when_capacity_one() {
        let runtime = test_runtime(1, true);
        let plan = SupervisorAgent::build_plan_for_runtime(&runtime, "implement feature y");
        assert!(!plan.assignments.iter().any(|a| a.role == SpecialistRole::Verifier));
        assert!(!plan.verification_required);
    }

    #[test]
    fn build_specialist_input_includes_dependency_context() {
        let assignment = SpecialistAssignment {
            agent_id: "executor-1".to_string(),
            role: SpecialistRole::Executor,
            title: "Execute".to_string(),
            instructions: "Apply the fix".to_string(),
            depends_on: vec!["research-1".to_string()],
        };
        let mut outcomes = HashMap::new();
        outcomes.insert(
            "research-1".to_string(),
            SpecialistOutcome {
                agent_id: "research-1".to_string(),
                role: SpecialistRole::Research,
                status: SpecialistStatus::Completed,
                summary: "Research".to_string(),
                response: "Found the root cause in the scheduler.".to_string(),
                depends_on: vec![],
                used_write_like_tools: false,
                tool_count: 1,
                started_at_ms: 1,
                finished_at_ms: 2,
            },
        );

        let input = SupervisorAgent::build_specialist_input("Patch the bug", &assignment, &outcomes);
        assert!(input.contains("Prior lane outcomes:"));
        assert!(input.contains("Research Agent"));
        assert!(input.contains("Found the root cause"));
        assert!(input.ends_with("Patch the bug"));
    }

    #[test]
    fn synthesize_final_response_follows_assignment_order() {
        let plan = SupervisorPlan {
            summary: "Supervisor orchestration activated".to_string(),
            steps: vec![],
            assignments: vec![
                SpecialistAssignment {
                    agent_id: "research-1".to_string(),
                    role: SpecialistRole::Research,
                    title: "Research".to_string(),
                    instructions: String::new(),
                    depends_on: vec![],
                },
                SpecialistAssignment {
                    agent_id: "executor-1".to_string(),
                    role: SpecialistRole::Executor,
                    title: "Execute".to_string(),
                    instructions: String::new(),
                    depends_on: vec!["research-1".to_string()],
                },
            ],
            verification_required: false,
        };
        let outcomes = vec![
            SpecialistOutcome {
                agent_id: "executor-1".to_string(),
                role: SpecialistRole::Executor,
                status: SpecialistStatus::Completed,
                summary: "Execute".to_string(),
                response: "Applied patch".to_string(),
                depends_on: vec!["research-1".to_string()],
                used_write_like_tools: true,
                tool_count: 2,
                started_at_ms: 2,
                finished_at_ms: 3,
            },
            SpecialistOutcome {
                agent_id: "research-1".to_string(),
                role: SpecialistRole::Research,
                status: SpecialistStatus::Completed,
                summary: "Research".to_string(),
                response: "Found bug".to_string(),
                depends_on: vec![],
                used_write_like_tools: false,
                tool_count: 1,
                started_at_ms: 1,
                finished_at_ms: 2,
            },
        ];

        let response = SupervisorAgent::synthesize_final_response(&plan, &outcomes, &[]);
        let research_idx = response.find("[Research Agent]").expect("research section");
        let executor_idx = response.find("[Executor Agent]").expect("executor section");
        assert!(research_idx < executor_idx);
    }

    #[test]
    fn missing_dependencies_reports_unfinished_upstreams() {
        let assignment = SpecialistAssignment {
            agent_id: "executor-1".to_string(),
            role: SpecialistRole::Executor,
            title: "Execute".to_string(),
            instructions: String::new(),
            depends_on: vec!["research-1".to_string()],
        };

        let missing = SupervisorAgent::missing_dependencies(&assignment, &HashMap::new());
        assert_eq!(missing, vec!["research-1".to_string()]);
    }

    #[test]
    fn build_plan_allows_parallel_research_and_execution_when_not_ordered() {
        let runtime = test_runtime(3, true);
        let plan = SupervisorAgent::build_plan_for_runtime(&runtime, "research and implement feature x");
        let executor = plan
            .assignments
            .iter()
            .find(|assignment| assignment.role == SpecialistRole::Executor)
            .expect("executor assignment");
        assert!(executor.depends_on.is_empty());
    }

    #[test]
    fn verifier_runs_only_when_executor_used_write_like_tools() {
        let without_writes = vec![SpecialistOutcome {
            agent_id: "executor-1".to_string(),
            role: SpecialistRole::Executor,
            status: SpecialistStatus::Completed,
            summary: "Execute".to_string(),
            response: "Reviewed files only".to_string(),
            depends_on: vec![],
            used_write_like_tools: false,
            tool_count: 3,
            started_at_ms: 1,
            finished_at_ms: 2,
        }];
        assert!(!SupervisorAgent::should_run_verifier(&without_writes));

        let with_writes = vec![SpecialistOutcome {
            used_write_like_tools: true,
            ..without_writes[0].clone()
        }];
        assert!(SupervisorAgent::should_run_verifier(&with_writes));
    }
}

#[derive(Clone, Debug)]
struct LaneFailure {
    agent_id: String,
    role: SpecialistRole,
    error: String,
}
