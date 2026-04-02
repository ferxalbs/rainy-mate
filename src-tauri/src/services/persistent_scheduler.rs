use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

use crate::commands::agent::{run_agent_workflow_internal, WorkflowInvocationSource};
use crate::services::{
    MacOSNativeNotificationBridge, MateLaunchpadService, SettingsManager, WorkspaceManager,
};

const SCHEDULER_POLL_INTERVAL: Duration = Duration::from_secs(60);
const MAX_SCHEDULED_RUN_TITLE_CHARS: usize = 120;
const MAX_SCHEDULED_PROMPT_CHARS: usize = 24_000;

async fn ensure_column(
    db: &Pool<Sqlite>,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), sqlx::Error> {
    let pragma = format!("PRAGMA table_info({})", table);
    let rows = sqlx::query(&pragma).fetch_all(db).await?;
    let exists = rows.iter().any(|row| {
        row.try_get::<String, _>("name")
            .map(|value| value == column)
            .unwrap_or(false)
    });

    if !exists {
        let alter = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition);
        sqlx::query(&alter).execute(db).await?;
    }

    Ok(())
}

fn normalize_scheduled_run_title(raw: &str) -> Result<String, String> {
    let title = raw.trim();
    if title.is_empty() {
        return Err("Scheduled run title cannot be empty".to_string());
    }

    Ok(title
        .chars()
        .take(MAX_SCHEDULED_RUN_TITLE_CHARS)
        .collect::<String>())
}

fn normalize_scheduled_prompt(raw: &str) -> Result<String, String> {
    let prompt = raw.trim();
    if prompt.is_empty() {
        return Err("Scheduled prompt cannot be empty".to_string());
    }

    Ok(prompt
        .chars()
        .take(MAX_SCHEDULED_PROMPT_CHARS)
        .collect::<String>())
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScheduledJob {
    pub id: String,
    pub schedule: String,
    pub agent_id: String,
    pub payload_json: String,
    pub created_at: i64,
    pub next_run_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceScheduledRun {
    pub id: String,
    pub workspace_id: String,
    pub workspace_path: String,
    pub job_kind: String,
    pub title: String,
    pub scenario_id: String,
    pub prompt_text: Option<String>,
    pub schedule: String,
    pub trust_preset: String,
    pub enabled_pack_ids: Vec<String>,
    pub created_at: i64,
    pub next_run_at: i64,
    pub last_run_at: Option<i64>,
    pub last_status: Option<String>,
    pub last_chat_id: Option<String>,
    pub last_error: Option<String>,
    pub last_request_id: Option<String>,
    pub last_artifact_count: u64,
    pub last_requires_explicit_approval: bool,
    pub last_blocked_by_approval: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceScheduledRunUpdate {
    pub title: Option<String>,
    pub prompt_text: Option<String>,
    pub scenario_id: Option<String>,
    pub schedule: String,
    pub trust_preset: Option<String>,
    pub enabled_pack_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
struct WorkspaceScheduledRunRecord {
    pub id: String,
    pub workspace_id: String,
    pub workspace_path: String,
    pub job_kind: String,
    pub title: String,
    pub scenario_id: String,
    pub prompt_text: Option<String>,
    pub schedule: String,
    pub trust_preset: String,
    pub enabled_pack_ids_json: String,
    pub created_at: i64,
    pub next_run_at: i64,
    pub last_run_at: Option<i64>,
    pub last_status: Option<String>,
    pub last_chat_id: Option<String>,
    pub last_error: Option<String>,
    pub last_request_id: Option<String>,
    pub last_artifact_count: i64,
    pub last_requires_explicit_approval: bool,
    pub last_blocked_by_approval: bool,
}

impl WorkspaceScheduledRunRecord {
    fn into_summary(self) -> Result<WorkspaceScheduledRun, String> {
        Ok(WorkspaceScheduledRun {
            id: self.id,
            workspace_id: self.workspace_id,
            workspace_path: self.workspace_path,
            job_kind: self.job_kind,
            title: self.title,
            scenario_id: self.scenario_id,
            prompt_text: self.prompt_text,
            schedule: self.schedule,
            trust_preset: self.trust_preset,
            enabled_pack_ids: serde_json::from_str(&self.enabled_pack_ids_json)
                .map_err(|e| format!("Failed to decode scheduled run packs: {}", e))?,
            created_at: self.created_at,
            next_run_at: self.next_run_at,
            last_run_at: self.last_run_at,
            last_status: self.last_status,
            last_chat_id: self.last_chat_id,
            last_error: self.last_error,
            last_request_id: self.last_request_id,
            last_artifact_count: self.last_artifact_count.max(0) as u64,
            last_requires_explicit_approval: self.last_requires_explicit_approval,
            last_blocked_by_approval: self.last_blocked_by_approval,
        })
    }
}

pub struct PersistentScheduler {
    db: Arc<Pool<Sqlite>>,
    task_manager: Arc<crate::services::task_manager::TaskManager>,
    app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
}

impl PersistentScheduler {
    pub fn new(
        pool: Pool<Sqlite>,
        task_manager: Arc<crate::services::task_manager::TaskManager>,
    ) -> Self {
        Self {
            db: Arc::new(pool),
            task_manager,
            app_handle: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_app_handle(&self, app_handle: tauri::AppHandle) {
        let mut guard = self.app_handle.write().await;
        *guard = Some(app_handle);
    }

    pub async fn emit_workspace_runs_updated(
        &self,
        workspace_path: &str,
        workspace_id: &str,
        job_id: Option<&str>,
    ) {
        let app_handle = self.app_handle.read().await.clone();
        if let Some(app_handle) = app_handle {
            let _ = app_handle.emit(
                "workspace://scheduled-runs-updated",
                serde_json::json!({
                    "workspacePath": workspace_path,
                    "workspaceId": workspace_id,
                    "jobId": job_id,
                }),
            );
        }
    }

    pub async fn init(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS scheduled_jobs (
                id TEXT PRIMARY KEY,
                schedule TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                created_at BIGINT NOT NULL,
                next_run_at BIGINT NOT NULL
            )",
        )
        .execute(&*self.db)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS workspace_scheduled_runs (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                workspace_path TEXT NOT NULL,
                job_kind TEXT NOT NULL DEFAULT 'scenario',
                title TEXT NOT NULL DEFAULT '',
                scenario_id TEXT NOT NULL,
                prompt_text TEXT,
                schedule TEXT NOT NULL,
                trust_preset TEXT NOT NULL,
                enabled_pack_ids_json TEXT NOT NULL,
                created_at BIGINT NOT NULL,
                next_run_at BIGINT NOT NULL,
                last_run_at BIGINT,
                last_status TEXT,
                last_chat_id TEXT,
                last_error TEXT,
                last_request_id TEXT,
                last_artifact_count BIGINT NOT NULL DEFAULT 0,
                last_requires_explicit_approval INTEGER NOT NULL DEFAULT 0,
                last_blocked_by_approval INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&*self.db)
        .await?;

        ensure_column(
            &self.db,
            "workspace_scheduled_runs",
            "job_kind",
            "TEXT NOT NULL DEFAULT 'scenario'",
        )
        .await?;
        ensure_column(
            &self.db,
            "workspace_scheduled_runs",
            "title",
            "TEXT NOT NULL DEFAULT ''",
        )
        .await?;
        ensure_column(&self.db, "workspace_scheduled_runs", "prompt_text", "TEXT").await?;

        Ok(())
    }

    pub async fn add_job(
        &self,
        id: String,
        schedule_str: String,
        agent_id: String,
        payload_json: String,
    ) -> Result<(), String> {
        let schedule = Schedule::from_str(&schedule_str)
            .map_err(|e| format!("Invalid cron schedule: {}", e))?;

        let now = Utc::now();
        let next_run = schedule
            .upcoming(Utc)
            .next()
            .ok_or("Cannot calculate next run")?;

        sqlx::query(
            "INSERT INTO scheduled_jobs (id, schedule, agent_id, payload_json, created_at, next_run_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(schedule_str)
        .bind(agent_id)
        .bind(payload_json)
        .bind(now.timestamp())
        .bind(next_run.timestamp())
        .execute(&*self.db)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn next_run_timestamp(schedule_str: &str) -> Result<i64, String> {
        let schedule = Schedule::from_str(schedule_str)
            .map_err(|e| format!("Invalid cron schedule: {}", e))?;
        schedule
            .upcoming(Utc)
            .next()
            .map(|next| next.timestamp())
            .ok_or_else(|| "Cannot calculate next run".to_string())
    }

    pub async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, String> {
        sqlx::query_as::<_, ScheduledJob>("SELECT * FROM scheduled_jobs ORDER BY created_at DESC")
            .fetch_all(&*self.db)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn remove_job(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM scheduled_jobs WHERE id = ?")
            .bind(id)
            .execute(&*self.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn add_workspace_run(
        &self,
        workspace_id: String,
        workspace_path: String,
        scenario_id: String,
        schedule_str: String,
        trust_preset: String,
        enabled_pack_ids: Vec<String>,
    ) -> Result<WorkspaceScheduledRun, String> {
        if !MateLaunchpadService::has_scenario(&scenario_id) {
            return Err(format!("Unknown first-party scenario '{}'", scenario_id));
        }
        let title = normalize_scheduled_run_title(
            &MateLaunchpadService::scenario_title(&scenario_id)
                .unwrap_or_else(|| scenario_id.clone()),
        )?;

        let now = Utc::now();
        let next_run = Self::next_run_timestamp(&schedule_str)?;
        let id = format!("scheduled_{}", uuid::Uuid::new_v4());
        let enabled_pack_ids_json =
            serde_json::to_string(&enabled_pack_ids).map_err(|e| e.to_string())?;

        sqlx::query(
            "INSERT INTO workspace_scheduled_runs (
                id,
                workspace_id,
                workspace_path,
                job_kind,
                title,
                scenario_id,
                prompt_text,
                schedule,
                trust_preset,
                enabled_pack_ids_json,
                created_at,
                next_run_at,
                last_artifact_count,
                last_requires_explicit_approval,
                last_blocked_by_approval
            ) VALUES (?, ?, ?, 'scenario', ?, ?, NULL, ?, ?, ?, ?, ?, 0, 0, 0)",
        )
        .bind(&id)
        .bind(&workspace_id)
        .bind(&workspace_path)
        .bind(&title)
        .bind(&scenario_id)
        .bind(&schedule_str)
        .bind(&trust_preset)
        .bind(enabled_pack_ids_json)
        .bind(now.timestamp())
        .bind(next_run)
        .execute(&*self.db)
        .await
        .map_err(|e| e.to_string())?;

        self.get_workspace_run(&id)
            .await?
            .ok_or_else(|| "Scheduled run was not found after creation".to_string())
    }

    pub async fn add_workspace_prompt_run(
        &self,
        workspace_id: String,
        workspace_path: String,
        title: String,
        prompt_text: String,
        schedule_str: String,
    ) -> Result<WorkspaceScheduledRun, String> {
        let title = normalize_scheduled_run_title(&title)?;
        let prompt_text = normalize_scheduled_prompt(&prompt_text)?;
        let now = Utc::now();
        let next_run = Self::next_run_timestamp(&schedule_str)?;
        let id = format!("scheduled_{}", uuid::Uuid::new_v4());
        let enabled_pack_ids_json = "[]".to_string();

        sqlx::query(
            "INSERT INTO workspace_scheduled_runs (
                id,
                workspace_id,
                workspace_path,
                job_kind,
                title,
                scenario_id,
                prompt_text,
                schedule,
                trust_preset,
                enabled_pack_ids_json,
                created_at,
                next_run_at,
                last_artifact_count,
                last_requires_explicit_approval,
                last_blocked_by_approval
            ) VALUES (?, ?, ?, 'prompt', ?, '', ?, ?, 'balanced', ?, ?, ?, 0, 0, 0)",
        )
        .bind(&id)
        .bind(&workspace_id)
        .bind(&workspace_path)
        .bind(&title)
        .bind(&prompt_text)
        .bind(&schedule_str)
        .bind(enabled_pack_ids_json)
        .bind(now.timestamp())
        .bind(next_run)
        .execute(&*self.db)
        .await
        .map_err(|e| e.to_string())?;

        self.get_workspace_run(&id)
            .await?
            .ok_or_else(|| "Scheduled prompt run was not found after creation".to_string())
    }

    pub async fn list_workspace_runs(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceScheduledRun>, String> {
        let rows = sqlx::query(
            "SELECT
                id,
                workspace_id,
                workspace_path,
                job_kind,
                title,
                scenario_id,
                prompt_text,
                schedule,
                trust_preset,
                enabled_pack_ids_json,
                created_at,
                next_run_at,
                last_run_at,
                last_status,
                last_chat_id,
                last_error,
                last_request_id,
                last_artifact_count,
                last_requires_explicit_approval,
                last_blocked_by_approval
             FROM workspace_scheduled_runs
             WHERE workspace_id = ? OR workspace_path = ?
             ORDER BY created_at DESC",
        )
        .bind(workspace_id)
        .bind(workspace_id)
        .fetch_all(&*self.db)
        .await
        .map_err(|e| e.to_string())?;

        rows.into_iter()
            .map(Self::workspace_run_record_from_row)
            .map(|row| row.and_then(WorkspaceScheduledRunRecord::into_summary))
            .collect()
    }

    pub async fn remove_workspace_run(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM workspace_scheduled_runs WHERE id = ?")
            .bind(id)
            .execute(&*self.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn update_workspace_run(
        &self,
        id: &str,
        update: WorkspaceScheduledRunUpdate,
    ) -> Result<WorkspaceScheduledRun, String> {
        let existing = self
            .get_workspace_run(id)
            .await?
            .ok_or_else(|| format!("Scheduled run '{}' was not found", id))?;
        let next_run_at = Self::next_run_timestamp(&update.schedule)?;

        match existing.job_kind.as_str() {
            "scenario" => {
                let scenario_id = update
                    .scenario_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(existing.scenario_id.as_str())
                    .to_string();
                if !MateLaunchpadService::has_scenario(&scenario_id) {
                    return Err(format!("Unknown first-party scenario '{}'", scenario_id));
                }
                let title = normalize_scheduled_run_title(
                    &MateLaunchpadService::scenario_title(&scenario_id)
                        .unwrap_or_else(|| scenario_id.clone()),
                )?;
                let trust_preset = update.trust_preset.unwrap_or(existing.trust_preset);
                let enabled_pack_ids = update.enabled_pack_ids.unwrap_or(existing.enabled_pack_ids);
                let enabled_pack_ids_json =
                    serde_json::to_string(&enabled_pack_ids).map_err(|e| e.to_string())?;

                sqlx::query(
                    "UPDATE workspace_scheduled_runs
                     SET title = ?, scenario_id = ?, prompt_text = NULL, schedule = ?, trust_preset = ?,
                         enabled_pack_ids_json = ?, next_run_at = ?, last_status = NULL, last_error = NULL,
                         last_blocked_by_approval = 0
                     WHERE id = ?",
                )
                .bind(title)
                .bind(scenario_id)
                .bind(&update.schedule)
                .bind(trust_preset)
                .bind(enabled_pack_ids_json)
                .bind(next_run_at)
                .bind(id)
                .execute(&*self.db)
                .await
                .map_err(|e| e.to_string())?;
            }
            "prompt" => {
                let title = normalize_scheduled_run_title(
                    update.title.as_deref().unwrap_or(existing.title.as_str()),
                )?;
                let prompt_text = normalize_scheduled_prompt(
                    update
                        .prompt_text
                        .as_deref()
                        .or(existing.prompt_text.as_deref())
                        .ok_or_else(|| {
                            "Scheduled prompt task is missing prompt_text".to_string()
                        })?,
                )?;

                sqlx::query(
                    "UPDATE workspace_scheduled_runs
                     SET title = ?, scenario_id = '', prompt_text = ?, schedule = ?, next_run_at = ?,
                         last_status = NULL, last_error = NULL, last_blocked_by_approval = 0
                     WHERE id = ?",
                )
                .bind(title)
                .bind(prompt_text)
                .bind(&update.schedule)
                .bind(next_run_at)
                .bind(id)
                .execute(&*self.db)
                .await
                .map_err(|e| e.to_string())?;
            }
            other => {
                return Err(format!(
                    "Unsupported scheduled run kind '{}' for update",
                    other
                ));
            }
        }

        self.get_workspace_run(id)
            .await?
            .ok_or_else(|| "Scheduled run was not found after update".to_string())
    }

    async fn get_workspace_run(&self, id: &str) -> Result<Option<WorkspaceScheduledRun>, String> {
        let row = sqlx::query(
            "SELECT
            id,
            workspace_id,
            workspace_path,
            job_kind,
            title,
            scenario_id,
            prompt_text,
            schedule,
            trust_preset,
                enabled_pack_ids_json,
                created_at,
                next_run_at,
                last_run_at,
                last_status,
                last_chat_id,
                last_error,
                last_request_id,
                last_artifact_count,
                last_requires_explicit_approval,
                last_blocked_by_approval
             FROM workspace_scheduled_runs
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&*self.db)
        .await
        .map_err(|e| e.to_string())?;

        row.map(Self::workspace_run_record_from_row)
            .transpose()?
            .map(WorkspaceScheduledRunRecord::into_summary)
            .transpose()
    }

    fn workspace_run_record_from_row(
        row: sqlx::sqlite::SqliteRow,
    ) -> Result<WorkspaceScheduledRunRecord, String> {
        Ok(WorkspaceScheduledRunRecord {
            id: row.try_get("id").map_err(|e| e.to_string())?,
            workspace_id: row.try_get("workspace_id").map_err(|e| e.to_string())?,
            workspace_path: row.try_get("workspace_path").map_err(|e| e.to_string())?,
            job_kind: row.try_get("job_kind").map_err(|e| e.to_string())?,
            title: row.try_get("title").map_err(|e| e.to_string())?,
            scenario_id: row.try_get("scenario_id").map_err(|e| e.to_string())?,
            prompt_text: row.try_get("prompt_text").map_err(|e| e.to_string())?,
            schedule: row.try_get("schedule").map_err(|e| e.to_string())?,
            trust_preset: row.try_get("trust_preset").map_err(|e| e.to_string())?,
            enabled_pack_ids_json: row
                .try_get("enabled_pack_ids_json")
                .map_err(|e| e.to_string())?,
            created_at: row.try_get("created_at").map_err(|e| e.to_string())?,
            next_run_at: row.try_get("next_run_at").map_err(|e| e.to_string())?,
            last_run_at: row.try_get("last_run_at").map_err(|e| e.to_string())?,
            last_status: row.try_get("last_status").map_err(|e| e.to_string())?,
            last_chat_id: row.try_get("last_chat_id").map_err(|e| e.to_string())?,
            last_error: row.try_get("last_error").map_err(|e| e.to_string())?,
            last_request_id: row.try_get("last_request_id").map_err(|e| e.to_string())?,
            last_artifact_count: row
                .try_get("last_artifact_count")
                .map_err(|e| e.to_string())?,
            last_requires_explicit_approval: row
                .try_get("last_requires_explicit_approval")
                .map_err(|e| e.to_string())?,
            last_blocked_by_approval: row
                .try_get("last_blocked_by_approval")
                .map_err(|e| e.to_string())?,
        })
    }

    async fn claim_due_workspace_runs(
        db: &Pool<Sqlite>,
        now: i64,
    ) -> Result<Vec<WorkspaceScheduledRun>, String> {
        let rows = sqlx::query(
            "SELECT
                id,
                workspace_id,
                workspace_path,
                job_kind,
                title,
                scenario_id,
                prompt_text,
                schedule,
                trust_preset,
                enabled_pack_ids_json,
                created_at,
                next_run_at,
                last_run_at,
                last_status,
                last_chat_id,
                last_error,
                last_request_id,
                last_artifact_count,
                last_requires_explicit_approval,
                last_blocked_by_approval
             FROM workspace_scheduled_runs
             WHERE next_run_at <= ?",
        )
        .bind(now)
        .fetch_all(db)
        .await
        .map_err(|e| e.to_string())?;

        let mut jobs = Vec::new();
        for row in rows {
            let record = Self::workspace_run_record_from_row(row)?;
            let summary = record.clone().into_summary()?;
            let next_run = Schedule::from_str(&summary.schedule)
                .map_err(|e| format!("Invalid cron schedule for {}: {}", summary.id, e))?
                .upcoming(Utc)
                .next()
                .ok_or_else(|| format!("Cannot calculate next run for {}", summary.id))?
                .timestamp();

            sqlx::query(
                "UPDATE workspace_scheduled_runs
                 SET next_run_at = ?, last_status = ?, last_error = NULL, last_blocked_by_approval = 0
                 WHERE id = ?",
            )
            .bind(next_run)
            .bind("running")
            .bind(&summary.id)
            .execute(db)
            .await
            .map_err(|e| e.to_string())?;

            jobs.push(summary);
        }

        Ok(jobs)
    }

    async fn finalize_workspace_run(
        db: &Pool<Sqlite>,
        job_id: &str,
        status: &str,
        chat_id: Option<&str>,
        request_id: Option<&str>,
        artifact_count: u64,
        requires_explicit_approval: bool,
        blocked_by_approval: bool,
        error: Option<&str>,
    ) -> Result<(), String> {
        sqlx::query(
            "UPDATE workspace_scheduled_runs
             SET last_run_at = ?, last_status = ?, last_chat_id = ?, last_request_id = ?,
                 last_artifact_count = ?, last_requires_explicit_approval = ?,
                 last_blocked_by_approval = ?, last_error = ?
             WHERE id = ?",
        )
        .bind(Utc::now().timestamp())
        .bind(status)
        .bind(chat_id)
        .bind(request_id)
        .bind(artifact_count as i64)
        .bind(requires_explicit_approval)
        .bind(blocked_by_approval)
        .bind(error)
        .bind(job_id)
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn execute_workspace_run(
        app_handle: tauri::AppHandle,
        job: WorkspaceScheduledRun,
    ) -> Result<(String, Option<String>, String, u64, bool, bool), String> {
        let workspace_manager: Arc<WorkspaceManager> =
            app_handle.state::<Arc<WorkspaceManager>>().inner().clone();
        let agent_manager: crate::ai::agent::manager::AgentManager = app_handle
            .state::<crate::ai::agent::manager::AgentManager>()
            .inner()
            .clone();
        let settings_manager: Arc<tokio::sync::Mutex<SettingsManager>> = app_handle
            .state::<Arc<tokio::sync::Mutex<SettingsManager>>>()
            .inner()
            .clone();

        let workspace = workspace_manager
            .ensure_workspace_for_path(&job.workspace_path)
            .map_err(|e| e.to_string())?;
        let chat = agent_manager
            .create_or_reuse_empty_chat_session(&job.workspace_path)
            .await
            .map_err(|e| e.to_string())?;
        let model_id = {
            let settings: tokio::sync::MutexGuard<'_, SettingsManager> =
                settings_manager.lock().await;
            settings.get_selected_model().to_string()
        };

        let (prompt, request_id, requires_explicit_approval) = if job.job_kind == "prompt" {
            (
                job.prompt_text
                    .clone()
                    .ok_or_else(|| "Scheduled prompt task is missing prompt_text".to_string())?,
                None,
                false,
            )
        } else {
            let prepared = MateLaunchpadService::prepare_workspace_launch_with_config(
                &workspace_manager,
                &workspace.id,
                &job.scenario_id,
                Some(job.trust_preset.as_str()),
                Some(job.enabled_pack_ids.as_slice()),
                "scheduled",
            )?;
            (
                prepared.prompt,
                Some(prepared.request_id),
                prepared.preflight.requires_explicit_approval,
            )
        };

        let result = run_agent_workflow_internal(
            app_handle.clone(),
            prompt,
            model_id,
            job.workspace_path.clone(),
            None,
            Some(chat.id.clone()),
            Some(format!("scheduled_run_{}", uuid::Uuid::new_v4())),
            None,
            None,
            WorkflowInvocationSource::Local,
        )
        .await;

        let result = match result {
            Ok(result) => result,
            Err(error) => {
                if let Some(request_id) = request_id.as_deref() {
                    let _ = MateLaunchpadService::record_workspace_launch(
                        &workspace_manager,
                        &workspace.id,
                        request_id,
                        &job.scenario_id,
                        Some(chat.id.as_str()),
                        false,
                        &[],
                        &[],
                        &[],
                    );
                }
                return Err(error);
            }
        };

        if let Some(request_id) = request_id.as_deref() {
            MateLaunchpadService::record_workspace_launch(
                &workspace_manager,
                &workspace.id,
                request_id,
                &job.scenario_id,
                Some(chat.id.as_str()),
                true,
                &result.actual_tool_ids,
                &result.actual_touched_paths,
                &result.produced_artifact_paths,
            )?;
        }

        Ok((
            "completed".to_string(),
            Some(chat.id),
            request_id.unwrap_or_default(),
            result.produced_artifact_paths.len() as u64,
            requires_explicit_approval,
            result.blocked_by_airlock,
        ))
    }

    async fn notifications_enabled(app_handle: &tauri::AppHandle) -> bool {
        let settings_manager: Arc<tokio::sync::Mutex<SettingsManager>> = app_handle
            .state::<Arc<tokio::sync::Mutex<SettingsManager>>>()
            .inner()
            .clone();
        let enabled = settings_manager
            .lock()
            .await
            .get_settings()
            .notifications_enabled;
        enabled
    }

    async fn notify_workspace_run_result(
        app_handle: &tauri::AppHandle,
        job: &WorkspaceScheduledRun,
        status: &str,
        chat_id: Option<&str>,
        artifact_count: u64,
        error: Option<&str>,
        blocked_by_approval: bool,
    ) {
        if !Self::notifications_enabled(app_handle).await {
            return;
        }

        let (title, body) = match status {
            "completed" => (
                format!("Scheduled run completed: {}", job.title),
                if artifact_count > 0 {
                    format!(
                        "Workspace task finished in {} and produced {} artifact{}.",
                        job.workspace_path,
                        artifact_count,
                        if artifact_count == 1 { "" } else { "s" }
                    )
                } else {
                    format!("Workspace task finished in {}.", job.workspace_path)
                },
            ),
            _ if blocked_by_approval => (
                format!("Scheduled run waiting for approval: {}", job.title),
                "The recurring task reached an Airlock approval gate and needs your decision."
                    .to_string(),
            ),
            _ => (
                format!("Scheduled run failed: {}", job.title),
                error.map(str::to_string).unwrap_or_else(|| {
                    "The recurring task did not finish successfully.".to_string()
                }),
            ),
        };

        if let Err(notification_error) = MacOSNativeNotificationBridge::send_agent_notification(
            &title,
            &body,
            Some(job.workspace_id.as_str()),
            chat_id,
        ) {
            tracing::warn!(
                "PersistentScheduler notification failed for {}: {}",
                job.id,
                notification_error
            );
        }
    }

    pub fn start_loop(&self) {
        let db = self.db.clone();
        let task_manager = self.task_manager.clone();
        let app_handle = self.app_handle.clone();
        tokio::spawn(async move {
            loop {
                let now = Utc::now().timestamp();

                let due_jobs = match sqlx::query_as::<_, ScheduledJob>(
                    "SELECT * FROM scheduled_jobs WHERE next_run_at <= ?",
                )
                .bind(now)
                .fetch_all(&*db)
                .await
                {
                    Ok(jobs) => jobs,
                    Err(e) => {
                        println!("Scheduler error: {}", e);
                        continue;
                    }
                };

                for job in due_jobs {
                    if let Ok(task) = serde_json::from_str::<crate::models::Task>(&job.payload_json)
                    {
                        task_manager.add_task(task).await;
                    }

                    if let Ok(schedule) = Schedule::from_str(&job.schedule) {
                        if let Some(next_run) = schedule.upcoming(Utc).next() {
                            let _ = sqlx::query(
                                "UPDATE scheduled_jobs SET next_run_at = ? WHERE id = ?",
                            )
                            .bind(next_run.timestamp())
                            .bind(job.id)
                            .execute(&*db)
                            .await;
                        }
                    }
                }

                let due_workspace_runs = match Self::claim_due_workspace_runs(&db, now).await {
                    Ok(jobs) => jobs,
                    Err(error) => {
                        tracing::warn!("PersistentScheduler workspace run claim failed: {}", error);
                        continue;
                    }
                };

                let maybe_app_handle = app_handle.read().await.clone();
                for job in due_workspace_runs {
                    let Some(app_handle) = maybe_app_handle.clone() else {
                        let _ = Self::finalize_workspace_run(
                            &db,
                            &job.id,
                            "failed",
                            None,
                            None,
                            0,
                            false,
                            false,
                            Some("Scheduler app handle not initialized"),
                        )
                        .await;
                        continue;
                    };

                    match Self::execute_workspace_run(app_handle.clone(), job.clone()).await {
                        Ok((
                            status,
                            chat_id,
                            request_id,
                            artifact_count,
                            requires_explicit_approval,
                            blocked_by_approval,
                        )) => {
                            let _ = Self::finalize_workspace_run(
                                &db,
                                &job.id,
                                &status,
                                chat_id.as_deref(),
                                Some(request_id.as_str()),
                                artifact_count,
                                requires_explicit_approval,
                                blocked_by_approval,
                                None,
                            )
                            .await;
                            Self::notify_workspace_run_result(
                                &app_handle,
                                &job,
                                &status,
                                chat_id.as_deref(),
                                artifact_count,
                                None,
                                blocked_by_approval,
                            )
                            .await;
                            let _ = app_handle.emit(
                                "workspace://scheduled-runs-updated",
                                serde_json::json!({
                                    "workspacePath": job.workspace_path,
                                    "workspaceId": job.workspace_id,
                                    "jobId": job.id,
                                }),
                            );
                        }
                        Err(error) => {
                            let _ = Self::finalize_workspace_run(
                                &db,
                                &job.id,
                                "failed",
                                None,
                                None,
                                0,
                                false,
                                error.contains("[Airlock]")
                                    || error.contains("blocked by Airlock")
                                    || error.contains("user decision"),
                                Some(error.as_str()),
                            )
                            .await;
                            Self::notify_workspace_run_result(
                                &app_handle,
                                &job,
                                "failed",
                                None,
                                0,
                                Some(error.as_str()),
                                error.contains("[Airlock]")
                                    || error.contains("blocked by Airlock")
                                    || error.contains("user decision"),
                            )
                            .await;
                            let _ = app_handle.emit(
                                "workspace://scheduled-runs-updated",
                                serde_json::json!({
                                    "workspacePath": job.workspace_path,
                                    "workspaceId": job.workspace_id,
                                    "jobId": job.id,
                                }),
                            );
                        }
                    }
                }

                sleep(SCHEDULER_POLL_INTERVAL).await;
            }
        });
    }
}

#[tauri::command]
pub async fn add_scheduled_job(
    state: tauri::State<'_, Arc<PersistentScheduler>>,
    id: String,
    schedule: String,
    agent_id: String,
    payload_json: String,
) -> Result<(), String> {
    state.add_job(id, schedule, agent_id, payload_json).await
}

#[tauri::command]
pub async fn list_scheduled_jobs(
    state: tauri::State<'_, Arc<PersistentScheduler>>,
) -> Result<Vec<ScheduledJob>, String> {
    state.list_jobs().await
}

#[tauri::command]
pub async fn remove_scheduled_job(
    state: tauri::State<'_, Arc<PersistentScheduler>>,
    id: String,
) -> Result<(), String> {
    state.remove_job(&id).await
}

#[cfg(test)]
mod tests {
    use super::{PersistentScheduler, WorkspaceScheduledRunUpdate};
    use crate::ai::AIProviderManager;
    use crate::services::task_manager::TaskManager;
    use crate::services::KeychainAccessService;
    use sqlx::SqlitePool;
    use std::sync::Arc;

    async fn test_scheduler() -> PersistentScheduler {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("sqlite");
        let ai_provider = Arc::new(AIProviderManager::new(KeychainAccessService::new()));
        let task_manager = Arc::new(TaskManager::new(ai_provider));
        let scheduler = PersistentScheduler::new(pool, task_manager);
        scheduler.init().await.expect("init");
        scheduler
    }

    #[tokio::test]
    async fn workspace_runs_can_be_added_listed_and_removed() {
        let scheduler = test_scheduler().await;
        let created = scheduler
            .add_workspace_run(
                "ws-1".to_string(),
                "/tmp/ws-1".to_string(),
                "release_readiness".to_string(),
                "0 * * * * *".to_string(),
                "balanced".to_string(),
                vec!["repo_guardian".to_string(), "knowledge_weaver".to_string()],
            )
            .await
            .expect("create");

        assert_eq!(created.workspace_id, "ws-1");
        assert_eq!(created.scenario_id, "release_readiness");
        assert_eq!(created.trust_preset, "balanced");

        let listed = scheduler.list_workspace_runs("ws-1").await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);

        scheduler
            .remove_workspace_run(&created.id)
            .await
            .expect("remove");
        assert!(scheduler
            .list_workspace_runs("ws-1")
            .await
            .expect("list after remove")
            .is_empty());
    }

    #[tokio::test]
    async fn invalid_scenario_is_rejected() {
        let scheduler = test_scheduler().await;
        let error = scheduler
            .add_workspace_run(
                "ws-1".to_string(),
                "/tmp/ws-1".to_string(),
                "unknown".to_string(),
                "0 * * * * *".to_string(),
                "balanced".to_string(),
                vec![],
            )
            .await
            .expect_err("invalid scenario");

        assert!(error.contains("Unknown first-party scenario"));
    }

    #[tokio::test]
    async fn prompt_runs_can_be_updated() {
        let scheduler = test_scheduler().await;
        let created = scheduler
            .add_workspace_prompt_run(
                "ws-1".to_string(),
                "/tmp/ws-1".to_string(),
                "Daily check".to_string(),
                "run release-readiness check".to_string(),
                "0 0 9 * * * *".to_string(),
            )
            .await
            .expect("create prompt run");

        let updated = scheduler
            .update_workspace_run(
                &created.id,
                WorkspaceScheduledRunUpdate {
                    title: Some("Weekly release check".to_string()),
                    prompt_text: Some("run weekly release review".to_string()),
                    scenario_id: None,
                    schedule: "0 30 10 * * 1 *".to_string(),
                    trust_preset: None,
                    enabled_pack_ids: None,
                },
            )
            .await
            .expect("update prompt run");

        assert_eq!(updated.title, "Weekly release check");
        assert_eq!(
            updated.prompt_text.as_deref(),
            Some("run weekly release review")
        );
        assert_eq!(updated.schedule, "0 30 10 * * 1 *");
    }

    #[tokio::test]
    async fn scenario_runs_can_be_updated() {
        let scheduler = test_scheduler().await;
        let created = scheduler
            .add_workspace_run(
                "ws-1".to_string(),
                "/tmp/ws-1".to_string(),
                "release_readiness".to_string(),
                "0 0 9 * * * *".to_string(),
                "balanced".to_string(),
                vec!["repo_guardian".to_string()],
            )
            .await
            .expect("create scenario run");

        let updated = scheduler
            .update_workspace_run(
                &created.id,
                WorkspaceScheduledRunUpdate {
                    title: None,
                    prompt_text: None,
                    scenario_id: Some("codebase_audit".to_string()),
                    schedule: "0 15 8 * * 1-5 *".to_string(),
                    trust_preset: Some("elevated".to_string()),
                    enabled_pack_ids: Some(vec![
                        "repo_guardian".to_string(),
                        "knowledge_weaver".to_string(),
                    ]),
                },
            )
            .await
            .expect("update scenario run");

        assert_eq!(updated.scenario_id, "codebase_audit");
        assert_eq!(updated.title, "Codebase Audit");
        assert_eq!(updated.schedule, "0 15 8 * * 1-5 *");
        assert_eq!(updated.trust_preset, "elevated");
        assert_eq!(updated.enabled_pack_ids.len(), 2);
    }
}
