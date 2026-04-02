use super::args::{
    DeleteRecurringTaskArgs, ListRecurringTasksArgs, ScheduleRecurringTaskArgs,
    UpdateRecurringTaskArgs,
};
use super::SkillExecutor;
use crate::models::neural::CommandResult;
use chrono::Utc;
use cron::Schedule;
use serde::Serialize;
use serde_json::Value;
use std::str::FromStr;

const DEFAULT_SCHEDULE_HOUR: u8 = 9;
const DEFAULT_SCHEDULE_MINUTE: u8 = 0;
const DEFAULT_SCHEDULE_DAY_OF_WEEK: u8 = 1;
const DEFAULT_SCHEDULE_DAY_OF_MONTH: u8 = 1;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScheduledTaskToolResponse {
    message: String,
    scheduled_run: crate::services::persistent_scheduler::WorkspaceScheduledRun,
}

struct ResolvedWorkspaceTarget {
    workspace: crate::services::Workspace,
    workspace_path: String,
}

fn derive_prompt_title(prompt: &str) -> String {
    let compact = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = compact.trim();
    if trimmed.is_empty() {
        return "Recurring chat task".to_string();
    }
    trimmed.chars().take(72).collect()
}

fn build_schedule_expression(args: &ScheduleRecurringTaskArgs) -> Result<String, String> {
    let kind = args
        .schedule_kind
        .as_deref()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| {
            if args
                .cron_expression
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "custom".to_string()
            } else {
                "daily".to_string()
            }
        });

    let hour = args.hour.unwrap_or(DEFAULT_SCHEDULE_HOUR).min(23);
    let minute = args.minute.unwrap_or(DEFAULT_SCHEDULE_MINUTE).min(59);
    let day_of_week = args
        .day_of_week
        .unwrap_or(DEFAULT_SCHEDULE_DAY_OF_WEEK)
        .min(6);
    let day_of_month = args
        .day_of_month
        .unwrap_or(DEFAULT_SCHEDULE_DAY_OF_MONTH)
        .clamp(1, 28);

    let schedule = match kind.as_str() {
        "daily" => format!("0 {} {} * * * *", minute, hour),
        "weekdays" => format!("0 {} {} * * 1-5 *", minute, hour),
        "weekly" => format!("0 {} {} * * {} *", minute, hour, day_of_week),
        "monthly" => format!("0 {} {} {} * * *", minute, hour, day_of_month),
        "custom" => args
            .cron_expression
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "custom schedules require a non-empty cron_expression".to_string())?
            .to_string(),
        other => {
            return Err(format!(
                "Unsupported schedule_kind '{}'. Use daily, weekdays, weekly, monthly, or custom",
                other
            ))
        }
    };

    Schedule::from_str(&schedule).map_err(|error| format!("Invalid schedule: {}", error))?;
    Ok(schedule)
}

fn resolve_workspace_target(
    workspace_manager: &crate::services::WorkspaceManager,
    workspace_ref: &str,
) -> Result<ResolvedWorkspaceTarget, String> {
    if let Ok(workspace) = workspace_manager.ensure_workspace_for_path(workspace_ref) {
        return Ok(ResolvedWorkspaceTarget {
            workspace,
            workspace_path: workspace_ref.to_string(),
        });
    }

    let workspace = workspace_manager
        .load_workspace(workspace_ref)
        .map_err(|error| error.to_string())?;
    let workspace_path = workspace
        .allowed_paths
        .first()
        .cloned()
        .unwrap_or_else(|| workspace_ref.to_string());

    Ok(ResolvedWorkspaceTarget {
        workspace,
        workspace_path,
    })
}

impl SkillExecutor {
    pub(super) async fn execute_workspace_tools(
        &self,
        workspace_ref: String,
        method: &str,
        params: &Option<Value>,
    ) -> CommandResult {
        match method {
            "schedule_recurring_task" => {
                let params = match params {
                    Some(value) => value.clone(),
                    None => return self.error("Missing parameters"),
                };
                let args: ScheduleRecurringTaskArgs = match serde_json::from_value(params) {
                    Ok(value) => value,
                    Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
                };
                self.handle_schedule_recurring_task(&workspace_ref, args)
                    .await
            }
            "list_recurring_tasks" => {
                let args = match params {
                    Some(value) => {
                        match serde_json::from_value::<ListRecurringTasksArgs>(value.clone()) {
                            Ok(parsed) => Ok(parsed),
                            Err(error) => Err(format!("Invalid parameters: {}", error)),
                        }
                    }
                    None => Ok(ListRecurringTasksArgs::default()),
                };

                match args {
                    Ok(value) => {
                        self.handle_list_recurring_tasks(&workspace_ref, value)
                            .await
                    }
                    Err(error) => self.error(&error),
                }
            }
            "delete_recurring_task" => {
                let params = match params {
                    Some(value) => value.clone(),
                    None => return self.error("Missing parameters"),
                };
                let args: DeleteRecurringTaskArgs = match serde_json::from_value(params) {
                    Ok(value) => value,
                    Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
                };
                self.handle_delete_recurring_task(&workspace_ref, args)
                    .await
            }
            "update_recurring_task" => {
                let params = match params {
                    Some(value) => value.clone(),
                    None => return self.error("Missing parameters"),
                };
                let args: UpdateRecurringTaskArgs = match serde_json::from_value(params) {
                    Ok(value) => value,
                    Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
                };
                self.handle_update_recurring_task(&workspace_ref, args)
                    .await
            }
            _ => self.error(&format!("Unknown workspace method: {}", method)),
        }
    }

    async fn handle_schedule_recurring_task(
        &self,
        workspace_ref: &str,
        args: ScheduleRecurringTaskArgs,
    ) -> CommandResult {
        let scheduler = {
            let lock = self.scheduler.read().await;
            match lock.as_ref() {
                Some(value) => value.clone(),
                None => return self.error("Persistent scheduler not initialized"),
            }
        };

        let resolved = match resolve_workspace_target(&self.workspace_manager, workspace_ref) {
            Ok(value) => value,
            Err(error) => return self.error(&error),
        };
        let workspace = resolved.workspace;
        let workspace_path = resolved.workspace_path;

        let schedule = match build_schedule_expression(&args) {
            Ok(value) => value,
            Err(error) => return self.error(&error),
        };

        let created = if let Some(scenario_id) = args
            .scenario_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            scheduler
                .add_workspace_run(
                    workspace.id.clone(),
                    workspace_path.clone(),
                    scenario_id.to_string(),
                    schedule,
                    workspace.launchpad.trust_preset.clone(),
                    workspace.launchpad.enabled_pack_ids.clone(),
                )
                .await
        } else {
            let prompt = match args
                .task_prompt
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                Some(value) => value.to_string(),
                None => {
                    return self.error(
                        "schedule_recurring_task requires either scenario_id or a non-empty task_prompt",
                    );
                }
            };

            let title = args
                .title
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .unwrap_or_else(|| derive_prompt_title(&prompt));

            scheduler
                .add_workspace_prompt_run(
                    workspace.id.clone(),
                    workspace_path.clone(),
                    title,
                    prompt,
                    schedule,
                )
                .await
        };

        match created {
            Ok(scheduled_run) => {
                scheduler
                    .emit_workspace_runs_updated(
                        &workspace_path,
                        &workspace.id,
                        Some(scheduled_run.id.as_str()),
                    )
                    .await;

                let response = ScheduledTaskToolResponse {
                    message: format!(
                        "Scheduled native recurring task '{}' for workspace '{}' at {}. This task is now stored inside MaTE's workspace scheduler. Do not tell the user to create OS cron, edit crontab, chmod a script, or run shell setup unless they explicitly asked for system-level cron. If they want verification, use list_recurring_tasks as a tool call, not as a terminal command. Describe list_recurring_tasks as view-only. Describe update_recurring_task as the tool for changing an existing recurring task. Describe delete_recurring_task as removal only.",
                        scheduled_run.title,
                        workspace.name,
                        Utc::now().to_rfc3339(),
                    ),
                    scheduled_run,
                };
                CommandResult {
                    success: true,
                    output: Some(serde_json::to_string(&response).unwrap_or_default()),
                    error: None,
                    exit_code: Some(0),
                }
            }
            Err(error) => self.error(&error),
        }
    }

    async fn handle_list_recurring_tasks(
        &self,
        workspace_ref: &str,
        args: ListRecurringTasksArgs,
    ) -> CommandResult {
        let scheduler = {
            let lock = self.scheduler.read().await;
            match lock.as_ref() {
                Some(value) => value.clone(),
                None => return self.error("Persistent scheduler not initialized"),
            }
        };

        let resolved = match resolve_workspace_target(&self.workspace_manager, workspace_ref) {
            Ok(value) => value,
            Err(error) => return self.error(&error),
        };
        let workspace = resolved.workspace;

        match scheduler.list_workspace_runs(&workspace.id).await {
            Ok(mut runs) => {
                if !args.include_prompt_text.unwrap_or(false) {
                    for run in &mut runs {
                        if run.job_kind == "prompt" {
                            run.prompt_text = None;
                        }
                    }
                }

                CommandResult {
                    success: true,
                    output: Some(serde_json::to_string(&runs).unwrap_or_default()),
                    error: None,
                    exit_code: Some(0),
                }
            }
            Err(error) => self.error(&error),
        }
    }

    async fn handle_delete_recurring_task(
        &self,
        workspace_ref: &str,
        args: DeleteRecurringTaskArgs,
    ) -> CommandResult {
        let scheduler = {
            let lock = self.scheduler.read().await;
            match lock.as_ref() {
                Some(value) => value.clone(),
                None => return self.error("Persistent scheduler not initialized"),
            }
        };

        let resolved = match resolve_workspace_target(&self.workspace_manager, workspace_ref) {
            Ok(value) => value,
            Err(error) => return self.error(&error),
        };
        let workspace = resolved.workspace;
        let scheduled_run_id = args.scheduled_run_id.trim().to_string();
        if scheduled_run_id.is_empty() {
            return self.error("delete_recurring_task requires a non-empty scheduled_run_id");
        }

        match scheduler.remove_workspace_run(&scheduled_run_id).await {
            Ok(()) => {
                scheduler
                    .emit_workspace_runs_updated(
                        workspace_ref,
                        &workspace.id,
                        Some(&scheduled_run_id),
                    )
                    .await;
                CommandResult {
                    success: true,
                    output: Some(format!(
                        "Deleted recurring task '{}' from workspace '{}'",
                        scheduled_run_id, workspace.name
                    )),
                    error: None,
                    exit_code: Some(0),
                }
            }
            Err(error) => self.error(&error),
        }
    }

    async fn handle_update_recurring_task(
        &self,
        workspace_ref: &str,
        args: UpdateRecurringTaskArgs,
    ) -> CommandResult {
        let scheduler = {
            let lock = self.scheduler.read().await;
            match lock.as_ref() {
                Some(value) => value.clone(),
                None => return self.error("Persistent scheduler not initialized"),
            }
        };

        let resolved = match resolve_workspace_target(&self.workspace_manager, workspace_ref) {
            Ok(value) => value,
            Err(error) => return self.error(&error),
        };
        let workspace = resolved.workspace;
        let workspace_path = resolved.workspace_path;

        let scheduled_run_id = args.scheduled_run_id.trim().to_string();
        if scheduled_run_id.is_empty() {
            return self.error("update_recurring_task requires a non-empty scheduled_run_id");
        }

        let existing_runs = match scheduler.list_workspace_runs(&workspace.id).await {
            Ok(runs) => runs,
            Err(error) => return self.error(&error),
        };
        let existing = match existing_runs
            .into_iter()
            .find(|run| run.id == scheduled_run_id)
        {
            Some(run) => run,
            None => return self.error("Scheduled run not found in the current workspace"),
        };

        let schedule = match build_schedule_expression(&ScheduleRecurringTaskArgs {
            title: args.title.clone(),
            task_prompt: args.task_prompt.clone(),
            scenario_id: args.scenario_id.clone(),
            schedule_kind: args.schedule_kind.clone(),
            hour: args.hour,
            minute: args.minute,
            day_of_week: args.day_of_week,
            day_of_month: args.day_of_month,
            cron_expression: args.cron_expression.clone(),
        }) {
            Ok(value) => value,
            Err(error) => return self.error(&error),
        };

        let updated = scheduler
            .update_workspace_run(
                &scheduled_run_id,
                crate::services::persistent_scheduler::WorkspaceScheduledRunUpdate {
                    title: args.title,
                    prompt_text: args.task_prompt,
                    scenario_id: args.scenario_id,
                    schedule,
                    trust_preset: (existing.job_kind == "scenario")
                        .then_some(workspace.launchpad.trust_preset.clone()),
                    enabled_pack_ids: (existing.job_kind == "scenario")
                        .then_some(workspace.launchpad.enabled_pack_ids.clone()),
                },
            )
            .await;

        match updated {
            Ok(scheduled_run) => {
                scheduler
                    .emit_workspace_runs_updated(
                        &workspace_path,
                        &workspace.id,
                        Some(scheduled_run.id.as_str()),
                    )
                    .await;

                let response = ScheduledTaskToolResponse {
                    message: format!(
                        "Updated native recurring task '{}' for workspace '{}'. Use list_recurring_tasks to inspect the latest stored schedule.",
                        scheduled_run.title, workspace.name,
                    ),
                    scheduled_run,
                };
                CommandResult {
                    success: true,
                    output: Some(serde_json::to_string(&response).unwrap_or_default()),
                    error: None,
                    exit_code: Some(0),
                }
            }
            Err(error) => self.error(&error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_schedule_expression;
    use crate::services::skill_executor::args::ScheduleRecurringTaskArgs;

    #[test]
    fn builds_weekday_schedule_without_raw_cron() {
        let schedule = build_schedule_expression(&ScheduleRecurringTaskArgs {
            title: None,
            task_prompt: Some("Review release readiness".to_string()),
            scenario_id: None,
            schedule_kind: Some("weekdays".to_string()),
            hour: Some(9),
            minute: Some(30),
            day_of_week: None,
            day_of_month: None,
            cron_expression: None,
        })
        .expect("weekday schedule");

        assert_eq!(schedule, "0 30 9 * * 1-5 *");
    }

    #[test]
    fn rejects_custom_schedule_without_cron_expression() {
        let error = build_schedule_expression(&ScheduleRecurringTaskArgs {
            title: None,
            task_prompt: Some("Review release readiness".to_string()),
            scenario_id: None,
            schedule_kind: Some("custom".to_string()),
            hour: None,
            minute: None,
            day_of_week: None,
            day_of_month: None,
            cron_expression: None,
        })
        .expect_err("missing cron");

        assert!(error.contains("cron_expression"));
    }
}
