use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SpecialistRole {
    Research,
    Executor,
    Verifier,
    /// Dedicated sub-agent for explicit memory read/write operations (save facts, recall context)
    MemoryScribe,
}

impl SpecialistRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpecialistRole::Research => "research",
            SpecialistRole::Executor => "executor",
            SpecialistRole::Verifier => "verifier",
            SpecialistRole::MemoryScribe => "memory_scribe",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SpecialistRole::Research => "Research Agent",
            SpecialistRole::Executor => "Executor Agent",
            SpecialistRole::Verifier => "Verifier Agent",
            SpecialistRole::MemoryScribe => "Memory Scribe",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpecialistStatus {
    Pending,
    Planning,
    Running,
    WaitingOnAirlock,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistAssignment {
    pub agent_id: String,
    pub role: SpecialistRole,
    pub title: String,
    pub instructions: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupervisorPlan {
    pub summary: String,
    pub steps: Vec<String>,
    pub assignments: Vec<SpecialistAssignment>,
    pub verification_required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistOutcome {
    pub agent_id: String,
    pub role: SpecialistRole,
    pub status: SpecialistStatus,
    pub summary: String,
    pub response: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub used_write_like_tools: bool,
    #[serde(default)]
    pub tool_count: u32,
    pub started_at_ms: i64,
    pub finished_at_ms: i64,
}

#[derive(Clone, Debug)]
pub enum SupervisorMessage {
    SpecialistStarted {
        run_id: String,
        agent_id: String,
        role: SpecialistRole,
        depends_on: Vec<String>,
        started_at_ms: i64,
    },
    SpecialistStatus {
        run_id: String,
        agent_id: String,
        role: SpecialistRole,
        status: SpecialistStatus,
        detail: Option<String>,
        active_tool: Option<String>,
        depends_on: Vec<String>,
        started_at_ms: Option<i64>,
        finished_at_ms: Option<i64>,
        tool_count: Option<u32>,
        write_like_used: Option<bool>,
    },
    SpecialistCompleted {
        run_id: String,
        outcome: SpecialistOutcome,
    },
    SpecialistFailed {
        run_id: String,
        agent_id: String,
        role: SpecialistRole,
        error: String,
        depends_on: Vec<String>,
        started_at_ms: Option<i64>,
        finished_at_ms: Option<i64>,
        tool_count: Option<u32>,
        write_like_used: Option<bool>,
    },
}
