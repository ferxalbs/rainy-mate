use super::protocol::{
    SpecialistAssignment, SpecialistOutcome, SpecialistRole, SpecialistStatus, SupervisorMessage,
};
use crate::ai::agent::memory::AgentMemory;
use crate::ai::agent::runtime::{AgentRuntime, RuntimeOptions};
use crate::ai::router::IntelligentRouter;
use crate::ai::specs::manifest::{AgentSpec, RuntimeMode};
use crate::services::{agent_kill_switch::AgentKillSwitch, airlock::AirlockService, SkillExecutor};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, RwLock};

pub struct SpecialistAgent {
    role: SpecialistRole,
    spec: AgentSpec,
    options: RuntimeOptions,
    router: Arc<RwLock<IntelligentRouter>>,
    skills: Arc<SkillExecutor>,
    memory: Arc<AgentMemory>,
    airlock_service: Arc<Option<AirlockService>>,
    kill_switch: Option<AgentKillSwitch>,
}

impl SpecialistAgent {
    pub fn new(
        role: SpecialistRole,
        spec: AgentSpec,
        options: RuntimeOptions,
        router: Arc<RwLock<IntelligentRouter>>,
        skills: Arc<SkillExecutor>,
        memory: Arc<AgentMemory>,
        airlock_service: Arc<Option<AirlockService>>,
        kill_switch: Option<AgentKillSwitch>,
    ) -> Self {
        Self {
            role,
            spec,
            options,
            router,
            skills,
            memory,
            airlock_service,
            kill_switch,
        }
    }

    pub fn allowed_tools(role: &SpecialistRole) -> &'static [&'static str] {
        match role {
            SpecialistRole::Research => &[
                "web_search",
                "read_web_page",
                "http_get_json",
                "http_get_text",
                "browse_url",
                "open_new_tab",
                "click_element",
                "wait_for_selector",
                "type_text",
                "submit_form",
                "go_back",
                "get_page_content",
                "get_page_snapshot",
                "extract_links",
                "screenshot",
                "read_file",
                "read_many_files",
                "search_files",
                "ingest_document",
            ],
            SpecialistRole::Executor => &[
                "read_file",
                "read_many_files",
                "read_file_chunk",
                "list_files",
                "list_files_detailed",
                "file_exists",
                "get_file_info",
                "search_files",
                "mkdir",
                "write_file",
                "append_file",
                "move_file",
                "delete_file",
                "git_status",
                "git_diff",
                "git_log",
                "git_show",
                "git_branch_list",
                "execute_command",
            ],
            SpecialistRole::Verifier => &[
                "read_file",
                "read_many_files",
                "read_file_chunk",
                "list_files",
                "list_files_detailed",
                "file_exists",
                "get_file_info",
                "search_files",
                "git_status",
                "git_diff",
                "git_log",
                "git_show",
                "git_branch_list",
                "web_search",
                "read_web_page",
                "http_get_json",
                "http_get_text",
                "recall_memory",
            ],
            SpecialistRole::MemoryScribe => &[
                "save_memory",
                "recall_memory",
                "read_file",
                "search_files",
                "ingest_document",
            ],
        }
    }

    fn role_prompt(&self) -> &'static str {
        match self.role {
            SpecialistRole::Research => {
                "You are the Research Agent. Gather evidence, inspect relevant sources, and return concise findings only. Do not claim code changes."
            }
            SpecialistRole::Executor => {
                "You are the Executor Agent. Make the smallest correct changes necessary using only your allowed tools. Verify critical actions with readbacks when possible."
            }
            SpecialistRole::Verifier => {
                "You are the Verifier Agent. Validate outputs using read-only tools. Never claim success without direct evidence from tool results."
            }
            SpecialistRole::MemoryScribe => {
                "You are the Memory Scribe. Your sole job is to persist important facts, preferences, and user details to long-term memory using save_memory, and to surface relevant context using recall_memory. Be precise and factual — save exactly what was stated, with appropriate tags like [\"user\", \"preference\"] or [\"project\", \"context\"]."
            }
        }
    }

    fn build_specialist_spec(&self) -> AgentSpec {
        let mut spec = self.spec.clone();
        spec.runtime.mode = RuntimeMode::Single;
        spec.airlock.tool_policy.mode = "allowlist".to_string();
        spec.airlock.tool_policy.allow = Self::allowed_tools(&self.role)
            .iter()
            .map(|tool| (*tool).to_string())
            .collect();
        spec.airlock.tool_policy.deny.clear();
        if matches!(
            self.spec.runtime.mode,
            RuntimeMode::ParallelSupervisor | RuntimeMode::Supervisor
        ) {
            spec.runtime.language_policy.internal_coordination_language = "english".to_string();
            spec.runtime.language_policy.final_response_language_mode = "english".to_string();
        }
        spec
    }

    pub async fn run(
        &self,
        run_id: &str,
        assignment: SpecialistAssignment,
        input: String,
        tx: mpsc::Sender<SupervisorMessage>,
    ) -> Result<SpecialistOutcome, String> {
        let spec = self.build_specialist_spec();
        let used_write_like_tools = Arc::new(Mutex::new(false));
        let tool_flag = used_write_like_tools.clone();
        let tool_count = Arc::new(Mutex::new(0u32));
        let tool_count_flag = tool_count.clone();
        let role = self.role.clone();
        let agent_id = assignment.agent_id.clone();
        let depends_on = assignment.depends_on.clone();
        let started_at_ms = Utc::now().timestamp_millis();
        let role_prompt = format!(
            "{}\n\nAssignment: {}\nInstructions: {}\nOutput language: English.",
            self.role_prompt(),
            assignment.title,
            assignment.instructions
        );

        let mut options = self.options.clone();
        options.custom_system_prompt = Some(role_prompt);

        let runtime = AgentRuntime::new(
            spec,
            options,
            self.router.clone(),
            self.skills.clone(),
            self.memory.clone(),
            self.airlock_service.clone(),
            self.kill_switch.clone(),
            None,
        );

        let _ = tx
            .send(SupervisorMessage::SpecialistStarted {
                run_id: run_id.to_string(),
                agent_id: agent_id.clone(),
                role: role.clone(),
                depends_on: depends_on.clone(),
                started_at_ms,
            })
            .await;

        let callback_tx = tx.clone();
        let callback_run_id = run_id.to_string();
        let callback_agent_id = agent_id.clone();
        let callback_role = role.clone();
        let callback_depends_on = depends_on.clone();
        let response = runtime
            .run_single(&input, move |event| match event {
                super::events::AgentEvent::ToolCall(ref call) => {
                    if let Ok(mut count) = tool_count_flag.lock() {
                        *count += 1;
                    }
                    if matches!(
                        call.function.name.as_str(),
                        "write_file"
                            | "append_file"
                            | "mkdir"
                            | "move_file"
                            | "delete_file"
                            | "execute_command"
                    ) {
                        if let Ok(mut flag) = tool_flag.lock() {
                            *flag = true;
                        }
                    }
                    let _ = callback_tx.try_send(SupervisorMessage::SpecialistStatus {
                        run_id: callback_run_id.clone(),
                        agent_id: callback_agent_id.clone(),
                        role: callback_role.clone(),
                        status: SpecialistStatus::Running,
                        detail: Some(format!("Executing {}", call.function.name)),
                        active_tool: Some(call.function.name.clone()),
                        depends_on: callback_depends_on.clone(),
                        started_at_ms: Some(started_at_ms),
                        finished_at_ms: None,
                        tool_count: tool_count_flag.lock().ok().map(|count| *count),
                        write_like_used: tool_flag.lock().ok().map(|flag| *flag),
                    });
                }
                super::events::AgentEvent::Status(ref text) => {
                    let status = if text.to_ascii_lowercase().contains("airlock") {
                        SpecialistStatus::WaitingOnAirlock
                    } else {
                        SpecialistStatus::Running
                    };
                    let _ = callback_tx.try_send(SupervisorMessage::SpecialistStatus {
                        run_id: callback_run_id.clone(),
                        agent_id: callback_agent_id.clone(),
                        role: callback_role.clone(),
                        status,
                        detail: Some(text.clone()),
                        active_tool: None,
                        depends_on: callback_depends_on.clone(),
                        started_at_ms: Some(started_at_ms),
                        finished_at_ms: None,
                        tool_count: tool_count_flag.lock().ok().map(|count| *count),
                        write_like_used: tool_flag.lock().ok().map(|flag| *flag),
                    });
                }
                _ => {}
            })
            .await?;

        let finished_at_ms = Utc::now().timestamp_millis();
        Ok(SpecialistOutcome {
            agent_id,
            role,
            status: SpecialistStatus::Completed,
            summary: assignment.title,
            response: response.clone(),
            parent_agent_id: assignment.parent_agent_id,
            branch_id: assignment.branch_id,
            spawn_reason: assignment.spawn_reason,
            depth: assignment.depth,
            depends_on,
            used_write_like_tools: used_write_like_tools
                .lock()
                .map(|flag| *flag)
                .unwrap_or(false),
            tool_count: tool_count.lock().map(|count| *count).unwrap_or(0),
            started_at_ms,
            finished_at_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_tools_are_read_only() {
        let tools = SpecialistAgent::allowed_tools(&SpecialistRole::Verifier);
        assert!(!tools.contains(&"write_file"));
        assert!(!tools.contains(&"append_file"));
        assert!(!tools.contains(&"delete_file"));
        assert!(!tools.contains(&"execute_command"));
    }

    #[test]
    fn executor_tools_include_mutating_actions() {
        let tools = SpecialistAgent::allowed_tools(&SpecialistRole::Executor);
        assert!(tools.contains(&"write_file"));
        assert!(tools.contains(&"append_file"));
        assert!(tools.contains(&"delete_file"));
        assert!(tools.contains(&"execute_command"));
    }
}
