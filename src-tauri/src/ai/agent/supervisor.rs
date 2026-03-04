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
use crate::services::{airlock::AirlockService, SkillExecutor};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinSet;

pub struct SupervisorAgent {
    pub spec: AgentSpec,
    pub options: RuntimeOptions,
    pub router: Arc<RwLock<IntelligentRouter>>,
    pub skills: Arc<SkillExecutor>,
    pub memory: Arc<AgentMemory>,
    pub airlock_service: Arc<Option<AirlockService>>,
    pub runtime_registry: Option<Arc<RuntimeRegistry>>,
}

impl SupervisorAgent {
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

    fn build_plan(&self, input: &str) -> SupervisorPlan {
        Self::build_plan_for_runtime(&self.spec.runtime, input)
    }

    fn build_plan_for_runtime(runtime: &RuntimeConfig, input: &str) -> SupervisorPlan {
        let mut steps = vec!["Assess the request and allocate specialist roles".to_string()];
        let mut base_assignments = Vec::new();

        if Self::should_use_research(input) || !Self::should_use_executor(input) {
            base_assignments.push(SpecialistAssignment {
                agent_id: "research-1".to_string(),
                role: SpecialistRole::Research,
                title: "Gather supporting context".to_string(),
                instructions: "Review relevant code, docs, and external references needed to execute the task safely.".to_string(),
                depends_on: vec![],
            });
            steps.push("Research Agent reviews code and supporting references".to_string());
        }

        if Self::should_use_executor(input) {
            base_assignments.push(SpecialistAssignment {
                agent_id: "executor-1".to_string(),
                role: SpecialistRole::Executor,
                title: "Execute the requested changes".to_string(),
                instructions: "Carry out the implementation work using the minimum necessary edits and tool calls.".to_string(),
                depends_on: vec![],
            });
            steps.push("Executor Agent performs the requested workspace actions".to_string());
        }

        let has_executor = base_assignments
            .iter()
            .any(|assignment| assignment.role == SpecialistRole::Executor);
        let max_specialists = runtime.max_specialists.clamp(1, 3) as usize;
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

    fn synthesize_final_response(plan: &SupervisorPlan, outcomes: &[SpecialistOutcome]) -> String {
        let mut sections = vec![format!("Supervisor completed {} specialist lane(s).", plan.assignments.len())];

        for outcome in outcomes {
            sections.push(format!(
                "[{}] {}",
                outcome.role.display_name(),
                outcome.response.trim()
            ));
        }

        sections.join("\n\n")
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
                } => {
                    if let Some(registry) = runtime_registry.as_ref() {
                        registry
                            .update_specialist_status(
                                &run_id,
                                &agent_id,
                                &role,
                                &SpecialistStatus::Planning,
                            )
                            .await;
                    }
                    on_event(AgentEvent::SpecialistSpawned(SpecialistEventPayload {
                        run_id,
                        agent_id,
                        role,
                        status: SpecialistStatus::Planning,
                        detail: Some("Specialist started".to_string()),
                        active_tool: None,
                    }));
                }
                SupervisorMessage::SpecialistStatus {
                    run_id,
                    agent_id,
                    role,
                    status,
                    detail,
                    active_tool,
                } => {
                    if let Some(registry) = runtime_registry.as_ref() {
                        registry
                            .update_specialist_status(&run_id, &agent_id, &role, &status)
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
                        detail,
                        active_tool,
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
                            )
                            .await;
                    }
                    on_event(AgentEvent::SpecialistCompleted(SpecialistCompletedPayload {
                        run_id,
                        agent_id: outcome.agent_id,
                        role: outcome.role,
                        summary: outcome.summary,
                        response_preview: outcome.response.chars().take(240).collect(),
                    }));
                }
                SupervisorMessage::SpecialistFailed {
                    run_id,
                    agent_id,
                    role,
                    error,
                } => {
                    if let Some(registry) = runtime_registry.as_ref() {
                        registry
                            .update_specialist_status(
                                &run_id,
                                &agent_id,
                                &role,
                                &SpecialistStatus::Failed,
                            )
                            .await;
                    }
                    on_event(AgentEvent::SpecialistFailed(SpecialistFailedPayload {
                        run_id,
                        agent_id,
                        role,
                        error,
                    }));
                }
            }
        }
    }

    pub async fn run<F>(&self, input: &str, on_event: F) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        let run_id = uuid::Uuid::new_v4().to_string();
        let plan = self.build_plan(input);
        let roles: Vec<SpecialistRole> = plan
            .assignments
            .iter()
            .map(|assignment| assignment.role.clone())
            .collect();

        if let Some(registry) = self.runtime_registry.as_ref() {
            registry.start_supervisor_run(&run_id, &roles).await;
            registry.update_supervisor_status(&run_id, "planning").await;
        }

        on_event(AgentEvent::SupervisorPlanCreated(plan.clone()));

        let (tx, rx) = mpsc::channel::<SupervisorMessage>(128);
        let on_event_arc = Arc::new(on_event.clone());
        let registry_for_events = self.runtime_registry.clone();
        let emitter = tokio::spawn(Self::emit_messages(rx, on_event_arc, registry_for_events));

        let mut outcomes = Vec::new();

        if let Some(registry) = self.runtime_registry.as_ref() {
            registry.update_supervisor_status(&run_id, "running").await;
        }
        let mut join_set = JoinSet::new();
        for assignment in plan
            .assignments
            .clone()
            .into_iter()
            .filter(|a| a.role != SpecialistRole::Verifier)
        {
            let specialist = SpecialistAgent::new(
                assignment.role.clone(),
                self.spec.clone(),
                self.options.clone(),
                self.router.clone(),
                self.skills.clone(),
                self.memory.clone(),
                self.airlock_service.clone(),
            );
            let run_id_for_task = run_id.clone();
            let input_for_task = input.to_string();
            let tx_for_task = tx.clone();
            join_set.spawn(async move {
                let outcome = specialist
                    .run(
                        &run_id_for_task,
                        assignment.clone(),
                        input_for_task,
                        tx_for_task.clone(),
                    )
                    .await;
                (assignment, outcome, tx_for_task, run_id_for_task)
            });
        }
        while let Some(join_result) = join_set.join_next().await {
            match join_result {
                Ok((assignment, Ok(outcome), tx_for_task, run_id_for_task)) => {
                    tx_for_task
                        .send(SupervisorMessage::SpecialistCompleted {
                            run_id: run_id_for_task,
                            outcome: outcome.clone(),
                        })
                        .await
                        .ok();
                    outcomes.push(outcome);
                    drop(tx_for_task);
                    let _ = assignment;
                }
                Ok((assignment, Err(error), tx_for_task, run_id_for_task)) => {
                    tx_for_task
                        .send(SupervisorMessage::SpecialistFailed {
                            run_id: run_id_for_task,
                            agent_id: assignment.agent_id,
                            role: assignment.role,
                            error,
                        })
                        .await
                        .ok();
                }
                Err(error) => {
                    on_event(AgentEvent::Error(format!(
                        "Supervisor specialist task join error: {}",
                        error
                    )));
                }
            }
        }

        if plan.verification_required
            && outcomes.iter().any(|outcome| outcome.used_write_like_tools)
        {
            if let Some(registry) = self.runtime_registry.as_ref() {
                registry.update_supervisor_status(&run_id, "verifying").await;
            }
            if let Some(assignment) = plan
                .assignments
                .iter()
                .find(|assignment| assignment.role == SpecialistRole::Verifier)
                .cloned()
            {
                let verifier = SpecialistAgent::new(
                    SpecialistRole::Verifier,
                    self.spec.clone(),
                    self.options.clone(),
                    self.router.clone(),
                    self.skills.clone(),
                    self.memory.clone(),
                    self.airlock_service.clone(),
                );
                match verifier
                    .run(&run_id, assignment.clone(), input.to_string(), tx.clone())
                    .await
                {
                    Ok(outcome) => {
                        tx.send(SupervisorMessage::SpecialistCompleted {
                            run_id: run_id.clone(),
                            outcome: outcome.clone(),
                        })
                        .await
                        .ok();
                        outcomes.push(outcome);
                    }
                    Err(error) => {
                        tx.send(SupervisorMessage::SpecialistFailed {
                            run_id: run_id.clone(),
                            agent_id: assignment.agent_id,
                            role: SpecialistRole::Verifier,
                            error,
                        })
                        .await
                        .ok();
                    }
                }
            }
        }

        drop(tx);
        let _ = emitter.await;

        let summary = Self::synthesize_final_response(&plan, &outcomes);
        on_event(AgentEvent::SupervisorSummary(SupervisorSummaryPayload {
            run_id: run_id.clone(),
            summary: summary.clone(),
        }));
        if let Some(registry) = self.runtime_registry.as_ref() {
            registry.finish_supervisor_run(&run_id, "completed").await;
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
    fn build_plan_can_disable_verifier_when_capacity_one() {
        let runtime = test_runtime(1, true);
        let plan = SupervisorAgent::build_plan_for_runtime(&runtime, "implement feature y");
        assert!(!plan.assignments.iter().any(|a| a.role == SpecialistRole::Verifier));
        assert!(!plan.verification_required);
    }
}
