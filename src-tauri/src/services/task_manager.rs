// Rainy Cowork - Task Manager Service
// Core task orchestration with queue and parallel execution

use crate::ai::AIProviderManager;
use crate::models::{Task, TaskEvent, TaskStatus};
use dashmap::DashMap;
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::sync::Mutex;

/// Task manager for orchestrating AI task execution
pub struct TaskManager {
    tasks: DashMap<String, Task>,
    ai_provider: Arc<Mutex<AIProviderManager>>,
}

impl TaskManager {
    pub fn new(ai_provider: Arc<Mutex<AIProviderManager>>) -> Self {
        Self {
            tasks: DashMap::new(),
            ai_provider,
        }
    }

    /// Add a task to the manager
    pub async fn add_task(&self, task: Task) {
        self.tasks.insert(task.id.clone(), task);
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        self.tasks.get(task_id).map(|r| r.clone())
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<Task> {
        self.tasks.iter().map(|r| r.value().clone()).collect()
    }

    /// Execute a task with progress channel
    pub async fn execute_task(
        &self,
        task_id: &str,
        on_event: Channel<TaskEvent>,
    ) -> Result<(), String> {
        // Update task status to running
        let task = match self.tasks.get_mut(task_id) {
            Some(mut task) => {
                task.status = TaskStatus::Running;
                task.started_at = Some(chrono::Utc::now());
                task.clone()
            }
            None => return Err(format!("Task not found: {}", task_id)),
        };

        // Emit started event
        on_event
            .send(TaskEvent::Started {
                task_id: task_id.to_string(),
            })
            .map_err(|e| e.to_string())?;

        // Execute the task using AI provider (acquire lock)
        let task_id_for_closure = task_id.to_string();
        let result = {
            let mut provider = self.ai_provider.lock().await;
            provider
                .execute_prompt(
                    &task.provider,
                    &task.model,
                    &task.description,
                    move |progress, _message| {
                        tracing::debug!("Task {} progress: {}%", task_id_for_closure, progress);
                    },
                )
                .await
        };

        // Update task status based on result
        match result {
            Ok(_response) => {
                if let Some(mut task) = self.tasks.get_mut(task_id) {
                    task.status = TaskStatus::Completed;
                    task.progress = 100;
                    task.completed_at = Some(chrono::Utc::now());
                }
                on_event
                    .send(TaskEvent::Completed {
                        task_id: task_id.to_string(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            Err(error) => {
                if let Some(mut task) = self.tasks.get_mut(task_id) {
                    task.status = TaskStatus::Failed;
                    task.error = Some(error.clone());
                    task.completed_at = Some(chrono::Utc::now());
                }
                on_event
                    .send(TaskEvent::Failed {
                        task_id: task_id.to_string(),
                        error: error.clone(),
                    })
                    .map_err(|e| e.to_string())?;
                return Err(error);
            }
        }

        Ok(())
    }

    /// Pause a running task
    pub async fn pause_task(&self, task_id: &str) -> Result<(), String> {
        match self.tasks.get_mut(task_id) {
            Some(mut task) => {
                if task.status == TaskStatus::Running {
                    task.status = TaskStatus::Paused;
                    Ok(())
                } else {
                    Err("Task is not running".to_string())
                }
            }
            None => Err(format!("Task not found: {}", task_id)),
        }
    }

    /// Resume a paused task
    pub async fn resume_task(&self, task_id: &str) -> Result<(), String> {
        match self.tasks.get_mut(task_id) {
            Some(mut task) => {
                if task.status == TaskStatus::Paused {
                    task.status = TaskStatus::Running;
                    Ok(())
                } else {
                    Err("Task is not paused".to_string())
                }
            }
            None => Err(format!("Task not found: {}", task_id)),
        }
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), String> {
        match self.tasks.get_mut(task_id) {
            Some(mut task) => {
                task.status = TaskStatus::Cancelled;
                task.completed_at = Some(chrono::Utc::now());
                Ok(())
            }
            None => Err(format!("Task not found: {}", task_id)),
        }
    }
}
