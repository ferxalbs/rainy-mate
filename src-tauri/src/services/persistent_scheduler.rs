use chrono::Utc;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScheduledJob {
    pub id: String,
    pub schedule: String,
    pub agent_id: String,
    pub payload_json: String,
    pub created_at: i64,
    pub next_run_at: i64,
}

pub struct PersistentScheduler {
    db: Arc<Pool<Sqlite>>,
    task_manager: Arc<crate::services::task_manager::TaskManager>,
}

impl PersistentScheduler {
    pub fn new(
        pool: Pool<Sqlite>,
        task_manager: Arc<crate::services::task_manager::TaskManager>,
    ) -> Self {
        Self {
            db: Arc::new(pool),
            task_manager,
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
             VALUES (?, ?, ?, ?, ?, ?)"
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

    pub async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, String> {
        let jobs = sqlx::query_as::<_, ScheduledJob>(
            "SELECT * FROM scheduled_jobs ORDER BY created_at DESC",
        )
        .fetch_all(&*self.db)
        .await
        .map_err(|e| e.to_string())?;
        Ok(jobs)
    }

    pub async fn remove_job(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM scheduled_jobs WHERE id = ?")
            .bind(id)
            .execute(&*self.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn start_loop(&self) {
        let db = self.db.clone();
        let task_manager = self.task_manager.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(60)).await;
                let now = Utc::now().timestamp();

                // Find due jobs
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
                    println!("TRIGGERING JOB: {}", job.id);

                    // Parse the payload back to Task and enqueue
                    if let Ok(task) = serde_json::from_str::<crate::models::Task>(&job.payload_json)
                    {
                        task_manager.add_task(task).await;
                    }

                    // Update next run
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
