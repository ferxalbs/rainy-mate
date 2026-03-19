use super::protocol::{SpecialistRole, SpecialistStatus};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistRunSnapshot {
    pub agent_id: String,
    pub role: SpecialistRole,
    pub status: SpecialistStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u8>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_ms: Option<i64>,
    pub tool_count: u32,
    pub write_like_used: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupervisorRunSnapshot {
    pub run_id: String,
    pub status: String,
    pub specialist_count: usize,
    pub completed_specialists: usize,
    pub failed_specialists: usize,
    #[serde(default)]
    pub specialists: Vec<SpecialistRunSnapshot>,
}

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolUsageByRole {
    pub research: u64,
    pub executor: u64,
    pub verifier: u64,
    pub memory_scribe: u64,
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

#[derive(Clone, Debug)]
struct ActiveSpecialistRun {
    role: SpecialistRole,
    status: SpecialistStatus,
    parent_agent_id: Option<String>,
    branch_id: Option<String>,
    spawn_reason: Option<String>,
    depth: Option<u8>,
    depends_on: Vec<String>,
    detail: Option<String>,
    active_tool: Option<String>,
    started_at_ms: Option<i64>,
    finished_at_ms: Option<i64>,
    tool_count: u32,
    write_like_used: bool,
}

impl Default for ActiveSpecialistRun {
    fn default() -> Self {
        Self {
            role: SpecialistRole::Research,
            status: SpecialistStatus::Pending,
            parent_agent_id: None,
            branch_id: None,
            spawn_reason: None,
            depth: None,
            depends_on: Vec::new(),
            detail: None,
            active_tool: None,
            started_at_ms: None,
            finished_at_ms: None,
            tool_count: 0,
            write_like_used: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ActiveSupervisorRun {
    status: String,
    specialists: HashMap<String, ActiveSpecialistRun>,
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

#[derive(Clone)]
pub struct RuntimeRegistryAssignment {
    pub agent_id: String,
    pub role: SpecialistRole,
    pub depends_on: Vec<String>,
    pub parent_agent_id: Option<String>,
    pub branch_id: Option<String>,
    pub spawn_reason: Option<String>,
    pub depth: Option<u8>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start_supervisor_run(
        &self,
        run_id: &str,
        assignments: &[(String, SpecialistRole, Vec<String>)],
    ) {
        let normalized: Vec<RuntimeRegistryAssignment> = assignments
            .iter()
            .map(|(agent_id, role, depends_on)| RuntimeRegistryAssignment {
                agent_id: agent_id.clone(),
                role: role.clone(),
                depends_on: depends_on.clone(),
                parent_agent_id: None,
                branch_id: None,
                spawn_reason: None,
                depth: Some(1),
            })
            .collect();
        self.start_hierarchical_run(run_id, &normalized).await;
    }

    pub async fn start_hierarchical_run(
        &self,
        run_id: &str,
        assignments: &[RuntimeRegistryAssignment],
    ) {
        let mut state = self.state.write().await;
        state.supervisors.insert(
            run_id.to_string(),
            ActiveSupervisorRun {
                status: "planning".to_string(),
                specialists: assignments
                    .iter()
                    .map(|assignment| {
                        (
                            assignment.agent_id.clone(),
                            ActiveSpecialistRun {
                                role: assignment.role.clone(),
                                status: SpecialistStatus::Pending,
                                parent_agent_id: assignment.parent_agent_id.clone(),
                                branch_id: assignment.branch_id.clone(),
                                spawn_reason: assignment.spawn_reason.clone(),
                                depth: assignment.depth,
                                depends_on: assignment.depends_on.clone(),
                                ..Default::default()
                            },
                        )
                    })
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
        depends_on: &[String],
        detail: Option<String>,
        active_tool: Option<String>,
        started_at_ms: Option<i64>,
        finished_at_ms: Option<i64>,
        tool_count: Option<u32>,
        write_like_used: Option<bool>,
    ) {
        self.update_specialist_status_with_hierarchy(
            run_id,
            agent_id,
            role,
            status,
            None,
            None,
            None,
            None,
            depends_on,
            detail,
            active_tool,
            started_at_ms,
            finished_at_ms,
            tool_count,
            write_like_used,
        )
        .await;
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_specialist_status_with_hierarchy(
        &self,
        run_id: &str,
        agent_id: &str,
        role: &SpecialistRole,
        status: &SpecialistStatus,
        parent_agent_id: Option<String>,
        branch_id: Option<String>,
        spawn_reason: Option<String>,
        depth: Option<u8>,
        depends_on: &[String],
        detail: Option<String>,
        active_tool: Option<String>,
        started_at_ms: Option<i64>,
        finished_at_ms: Option<i64>,
        tool_count: Option<u32>,
        write_like_used: Option<bool>,
    ) {
        let mut state = self.state.write().await;
        if let Some(run) = state.supervisors.get_mut(run_id) {
            let specialist = run
                .specialists
                .entry(agent_id.to_string())
                .or_insert_with(|| ActiveSpecialistRun {
                    role: role.clone(),
                    parent_agent_id: parent_agent_id.clone(),
                    branch_id: branch_id.clone(),
                    spawn_reason: spawn_reason.clone(),
                    depth,
                    depends_on: depends_on.to_vec(),
                    ..Default::default()
                });
            specialist.role = role.clone();
            specialist.status = status.clone();
            if parent_agent_id.is_some() {
                specialist.parent_agent_id = parent_agent_id;
            }
            if branch_id.is_some() {
                specialist.branch_id = branch_id;
            }
            if spawn_reason.is_some() {
                specialist.spawn_reason = spawn_reason;
            }
            if depth.is_some() {
                specialist.depth = depth;
            }
            specialist.depends_on = depends_on.to_vec();
            specialist.detail = detail;
            specialist.active_tool = active_tool;
            if started_at_ms.is_some() {
                specialist.started_at_ms = started_at_ms;
            }
            if finished_at_ms.is_some() {
                specialist.finished_at_ms = finished_at_ms;
            }
            if let Some(tool_count) = tool_count {
                specialist.tool_count = tool_count;
            }
            if let Some(write_like_used) = write_like_used {
                specialist.write_like_used = write_like_used;
            }
            if matches!(status, SpecialistStatus::Running | SpecialistStatus::WaitingOnAirlock | SpecialistStatus::Verifying) {
                run.status = "running".to_string();
            }
        }
    }

    pub async fn record_tool_use(&self, role: &SpecialistRole) {
        let mut state = self.state.write().await;
        match role {
            SpecialistRole::Research => state.tool_usage_by_role.research += 1,
            SpecialistRole::Executor => state.tool_usage_by_role.executor += 1,
            SpecialistRole::Verifier => state.tool_usage_by_role.verifier += 1,
            SpecialistRole::MemoryScribe => state.tool_usage_by_role.memory_scribe += 1,
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
                .map(|run| {
                    run.specialists
                        .values()
                        .filter(|specialist| {
                            !matches!(
                                specialist.status,
                                SpecialistStatus::Completed
                                    | SpecialistStatus::Failed
                                    | SpecialistStatus::Cancelled
                            )
                        })
                        .count()
                })
                .sum(),
            supervisors: state
                .supervisors
                .iter()
                .map(|(run_id, run)| SupervisorRunSnapshot {
                    run_id: run_id.clone(),
                    status: run.status.clone(),
                    specialist_count: run.specialists.len(),
                    completed_specialists: run
                        .specialists
                        .values()
                        .filter(|specialist| specialist.status == SpecialistStatus::Completed)
                        .count(),
                    failed_specialists: run
                        .specialists
                        .values()
                        .filter(|specialist| {
                            matches!(
                                specialist.status,
                                SpecialistStatus::Failed | SpecialistStatus::Cancelled
                            )
                        })
                        .count(),
                    specialists: run
                        .specialists
                        .iter()
                        .map(|(agent_id, specialist)| SpecialistRunSnapshot {
                            agent_id: agent_id.clone(),
                            role: specialist.role.clone(),
                            status: specialist.status.clone(),
                            parent_agent_id: specialist.parent_agent_id.clone(),
                            branch_id: specialist.branch_id.clone(),
                            spawn_reason: specialist.spawn_reason.clone(),
                            depth: specialist.depth,
                            depends_on: specialist.depends_on.clone(),
                            detail: specialist.detail.clone(),
                            active_tool: specialist.active_tool.clone(),
                            started_at_ms: specialist.started_at_ms,
                            finished_at_ms: specialist.finished_at_ms,
                            tool_count: specialist.tool_count,
                            write_like_used: specialist.write_like_used,
                        })
                        .collect(),
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
            .start_supervisor_run(
                "run-1",
                &[
                    ("research-1".to_string(), SpecialistRole::Research, vec![]),
                    ("executor-1".to_string(), SpecialistRole::Executor, vec!["research-1".to_string()]),
                ],
            )
            .await;
        registry
            .update_specialist_status(
                "run-1",
                "research-1",
                &SpecialistRole::Research,
                &SpecialistStatus::Running,
                &[],
                Some("Reading context".to_string()),
                None,
                Some(10),
                None,
                Some(1),
                Some(false),
            )
            .await;
        registry.record_tool_use(&SpecialistRole::Research).await;

        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.active_supervisor_runs, 1);
        assert_eq!(snapshot.active_specialists, 2);
        assert_eq!(snapshot.tool_usage_by_role.research, 1);
        assert_eq!(snapshot.supervisors[0].specialists.len(), 2);
    }
}
