use super::protocol::{SpecialistRole, SpecialistStatus};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupervisorRunSnapshot {
    pub run_id: String,
    pub status: String,
    pub specialist_count: usize,
}

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolUsageByRole {
    pub research: u64,
    pub executor: u64,
    pub verifier: u64,
}

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatsSnapshot {
    pub active_supervisor_runs: usize,
    pub active_specialists: usize,
    #[serde(default)]
    pub supervisors: Vec<SupervisorRunSnapshot>,
    pub tool_usage_by_role: ToolUsageByRole,
}

#[derive(Clone, Debug, Default)]
struct ActiveSupervisorRun {
    status: String,
    specialists: HashMap<String, SpecialistRole>,
}

#[derive(Default)]
struct RuntimeRegistryState {
    supervisors: HashMap<String, ActiveSupervisorRun>,
    tool_usage_by_role: ToolUsageByRole,
}

#[derive(Clone, Default)]
pub struct RuntimeRegistry {
    state: Arc<RwLock<RuntimeRegistryState>>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start_supervisor_run(&self, run_id: &str, specialist_roles: &[SpecialistRole]) {
        let mut state = self.state.write().await;
        state.supervisors.insert(
            run_id.to_string(),
            ActiveSupervisorRun {
                status: "planning".to_string(),
                specialists: specialist_roles
                    .iter()
                    .enumerate()
                    .map(|(idx, role)| (format!("{}-{}", role.as_str(), idx + 1), role.clone()))
                    .collect(),
            },
        );
    }

    pub async fn update_supervisor_status(&self, run_id: &str, status: &str) {
        let mut state = self.state.write().await;
        if let Some(run) = state.supervisors.get_mut(run_id) {
            run.status = status.to_string();
        }
    }

    pub async fn update_specialist_status(
        &self,
        run_id: &str,
        agent_id: &str,
        role: &SpecialistRole,
        status: &SpecialistStatus,
    ) {
        let mut state = self.state.write().await;
        if let Some(run) = state.supervisors.get_mut(run_id) {
            if matches!(status, SpecialistStatus::Cancelled | SpecialistStatus::Failed | SpecialistStatus::Completed) {
                run.specialists.insert(agent_id.to_string(), role.clone());
            } else {
                run.specialists.insert(agent_id.to_string(), role.clone());
                if status == &SpecialistStatus::Running {
                    run.status = "running".to_string();
                }
            }
        }
    }

    pub async fn record_tool_use(&self, role: &SpecialistRole) {
        let mut state = self.state.write().await;
        match role {
            SpecialistRole::Research => state.tool_usage_by_role.research += 1,
            SpecialistRole::Executor => state.tool_usage_by_role.executor += 1,
            SpecialistRole::Verifier => state.tool_usage_by_role.verifier += 1,
        }
    }

    pub async fn finish_supervisor_run(&self, run_id: &str, status: &str) {
        let mut state = self.state.write().await;
        if let Some(mut run) = state.supervisors.remove(run_id) {
            run.status = status.to_string();
        }
    }

    pub async fn snapshot(&self) -> RuntimeStatsSnapshot {
        let state = self.state.read().await;
        RuntimeStatsSnapshot {
            active_supervisor_runs: state.supervisors.len(),
            active_specialists: state
                .supervisors
                .values()
                .map(|run| run.specialists.len())
                .sum(),
            supervisors: state
                .supervisors
                .iter()
                .map(|(run_id, run)| SupervisorRunSnapshot {
                    run_id: run_id.clone(),
                    status: run.status.clone(),
                    specialist_count: run.specialists.len(),
                })
                .collect(),
            tool_usage_by_role: state.tool_usage_by_role.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agent::protocol::{SpecialistRole, SpecialistStatus};

    #[tokio::test]
    async fn snapshot_reflects_active_supervisor_and_tool_usage() {
        let registry = RuntimeRegistry::new();
        registry
            .start_supervisor_run("run-1", &[SpecialistRole::Research, SpecialistRole::Executor])
            .await;
        registry
            .update_specialist_status(
                "run-1",
                "research-1",
                &SpecialistRole::Research,
                &SpecialistStatus::Running,
            )
            .await;
        registry.record_tool_use(&SpecialistRole::Research).await;

        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.active_supervisor_runs, 1);
        assert_eq!(snapshot.active_specialists, 2);
        assert_eq!(snapshot.tool_usage_by_role.research, 1);
    }
}
