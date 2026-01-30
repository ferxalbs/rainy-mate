// Rainy Cowork - Task Manager Service
// Core task orchestration with queue and parallel execution

use crate::ai::AIProviderManager;
use crate::models::{Task, TaskEvent, TaskPriority, TaskStatus};
use crate::services::workspace::Workspace;
use dashmap::DashMap;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::sync::Mutex;

/// Priority queue item for tasks
#[derive(Debug, Clone)]
struct PriorityTask {
    task: Task,
    priority: TaskPriority,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then earlier creation time
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.created_at.cmp(&other.created_at))
    }
}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for PriorityTask {}

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.id == other.task.id
    }
}

/// Serializable queue state for persistence
/// Reserved for future state persistence feature
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct QueueState {
    pending: Vec<Task>,
    running: Vec<Task>,
    completed: Vec<Task>,
    failed: Vec<Task>,
    dependency_graph: HashMap<String, HashSet<String>>,
    reverse_dependencies: HashMap<String, HashSet<String>>,
}

#[allow(dead_code)]
impl QueueState {
    async fn from_queue(queue: &TaskQueue) -> Self {
        let pending = queue
            .pending
            .lock()
            .await
            .iter()
            .map(|pt| pt.task.clone())
            .collect();

        let running = queue.running.iter().map(|r| r.value().clone()).collect();

        let completed = queue.completed.iter().map(|r| r.value().clone()).collect();

        let failed = queue.failed.iter().map(|r| r.value().clone()).collect();

        let dependency_graph = queue.dependency_graph.lock().await.clone();
        let reverse_dependencies = queue.reverse_dependencies.lock().await.clone();

        Self {
            pending,
            running,
            completed,
            failed,
            dependency_graph,
            reverse_dependencies,
        }
    }

    async fn restore_to_queue(&self, queue: &TaskQueue) {
        // Clear existing state
        *queue.pending.lock().await = BinaryHeap::new();
        queue.running.clear();
        queue.completed.clear();
        queue.failed.clear();
        *queue.dependency_graph.lock().await = HashMap::new();
        *queue.reverse_dependencies.lock().await = HashMap::new();

        // Restore pending tasks
        for task in &self.pending {
            queue.enqueue(task.clone()).await;
        }

        // Restore running tasks
        for task in &self.running {
            queue.running.insert(task.id.clone(), task.clone());
        }

        // Restore completed tasks
        for task in &self.completed {
            queue.completed.insert(task.id.clone(), task.clone());
        }

        // Restore failed tasks
        for task in &self.failed {
            queue.failed.insert(task.id.clone(), task.clone());
        }

        // Restore dependency graphs
        *queue.dependency_graph.lock().await = self.dependency_graph.clone();
        *queue.reverse_dependencies.lock().await = self.reverse_dependencies.clone();
    }
}

/// Task queue with priority scheduling and dependency management
pub struct TaskQueue {
    pending: Mutex<BinaryHeap<PriorityTask>>, // Priority queue for pending tasks
    running: DashMap<String, Task>,           // Currently running tasks
    completed: DashMap<String, Task>,         // Completed tasks
    failed: DashMap<String, Task>,            // Failed tasks
    dependency_graph: Mutex<HashMap<String, HashSet<String>>>, // task_id -> dependent task_ids
    reverse_dependencies: Mutex<HashMap<String, HashSet<String>>>, // task_id -> tasks that depend on it
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(BinaryHeap::new()),
            running: DashMap::new(),
            completed: DashMap::new(),
            failed: DashMap::new(),
            dependency_graph: Mutex::new(HashMap::new()),
            reverse_dependencies: Mutex::new(HashMap::new()),
        }
    }

    /// Add a task to the queue
    pub async fn enqueue(&self, task: Task) {
        let mut pending = self.pending.lock().await;
        let mut dep_graph = self.dependency_graph.lock().await;
        let mut rev_deps = self.reverse_dependencies.lock().await;

        // Add to dependency graph
        dep_graph.insert(task.id.clone(), HashSet::new());
        for dep_id in &task.dependencies {
            rev_deps
                .entry(dep_id.clone())
                .or_insert_with(HashSet::new)
                .insert(task.id.clone());
        }

        // Add to priority queue
        let priority_task = PriorityTask {
            task,
            priority: TaskPriority::Normal, // Default priority
            created_at: chrono::Utc::now(),
        };
        pending.push(priority_task);
    }

    /// Get the next task to execute (considering dependencies)
    pub async fn dequeue(&self) -> Option<Task> {
        let mut pending = self.pending.lock().await;
        let _rev_deps = self.reverse_dependencies.lock().await;
        let mut unsatisfied_tasks = Vec::new();

        while let Some(priority_task) = pending.pop() {
            let task = &priority_task.task;

            // Check if all dependencies are completed
            let dependencies_satisfied = task.dependencies.iter().all(|dep_id| {
                self.completed.contains_key(dep_id) || self.failed.contains_key(dep_id)
            });

            if dependencies_satisfied {
                // Move to running
                let task_clone = task.clone();
                self.running.insert(task.id.clone(), task_clone.clone());
                return Some(task_clone);
            } else {
                // Collect unsatisfied tasks to put back
                unsatisfied_tasks.push(priority_task);
            }
        }

        // Put unsatisfied tasks back in the queue
        for task in unsatisfied_tasks {
            pending.push(task);
        }

        None
    }

    /// Mark a task as completed
    pub async fn complete_task(&self, task_id: &str) {
        if let Some((_, task)) = self.running.remove(task_id) {
            self.completed.insert(task_id.to_string(), task);

            // Notify dependent tasks that this dependency is satisfied
            let rev_deps = self.reverse_dependencies.lock().await;
            if let Some(_dependents) = rev_deps.get(task_id) {
                // Dependents can now potentially be dequeued
                // This is handled by the dequeue method checking dependencies
            }
        }
    }

    /// Mark a task as failed
    pub async fn fail_task(&self, task_id: &str, error: String) {
        if let Some((_, mut task)) = self.running.remove(task_id) {
            task.status = TaskStatus::Failed;
            task.error = Some(error);
            task.completed_at = Some(chrono::Utc::now());
            self.failed.insert(task_id.to_string(), task);
        }
    }

    /// Get all tasks
    pub async fn get_all_tasks(&self) -> Vec<Task> {
        let mut tasks = Vec::new();

        // Add pending tasks
        let pending = self.pending.lock().await;
        for priority_task in pending.iter() {
            tasks.push(priority_task.task.clone());
        }

        // Add running tasks
        for task in self.running.iter() {
            tasks.push(task.value().clone());
        }

        // Add completed tasks
        for task in self.completed.iter() {
            tasks.push(task.value().clone());
        }

        // Add failed tasks
        for task in self.failed.iter() {
            tasks.push(task.value().clone());
        }

        tasks
    }

    /// Get a specific task by ID
    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        // Check running
        if let Some(task) = self.running.get(task_id) {
            return Some(task.value().clone());
        }

        // Check completed
        if let Some(task) = self.completed.get(task_id) {
            return Some(task.value().clone());
        }

        // Check failed
        if let Some(task) = self.failed.get(task_id) {
            return Some(task.value().clone());
        }

        // Check pending
        let pending = self.pending.lock().await;
        for priority_task in pending.iter() {
            if priority_task.task.id == task_id {
                return Some(priority_task.task.clone());
            }
        }

        None
    }

    /// Save queue state to disk
    /// Reserved for future state persistence feature
    #[allow(dead_code)]
    pub async fn save_to_disk(&self, path: &Path) -> Result<(), String> {
        let state = QueueState::from_queue(self).await;
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("Failed to serialize queue state: {}", e))?;

        fs::write(path, json).map_err(|e| format!("Failed to write queue state to disk: {}", e))?;

        Ok(())
    }

    /// Load queue state from disk
    /// Reserved for future state persistence feature
    #[allow(dead_code)]
    pub async fn load_from_disk(&self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(()); // No saved state, start fresh
        }

        let json = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read queue state from disk: {}", e))?;

        let state: QueueState = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize queue state: {}", e))?;

        state.restore_to_queue(self).await;
        Ok(())
    }
}

/// Task manager for orchestrating AI task execution with workspace context
pub struct TaskManager {
    queue: Arc<TaskQueue>,
    ai_provider: Arc<AIProviderManager>,
    /// Current workspace context (interior mutability for shared state)
    workspace: Arc<Mutex<Option<Workspace>>>,
    /// Reserved for background processing feature
    #[allow(dead_code)]
    max_concurrent_tasks: usize,
    /// Reserved for background processing feature
    #[allow(dead_code)]
    running_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl TaskManager {
    pub fn new(ai_provider: Arc<AIProviderManager>) -> Self {
        Self {
            queue: Arc::new(TaskQueue::new()),
            ai_provider,
            workspace: Arc::new(Mutex::new(None)),
            max_concurrent_tasks: 3, // Default concurrent task limit
            running_handles: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create TaskManager with workspace context
    #[allow(dead_code)]
    pub fn with_workspace(ai_provider: Arc<AIProviderManager>, workspace: Workspace) -> Self {
        let manager = Self::new(ai_provider);
        *manager.workspace.try_lock().unwrap() = Some(workspace);
        manager
    }

    /// Set workspace context
    pub async fn set_workspace(&self, workspace: Workspace) {
        *self.workspace.lock().await = Some(workspace);
    }

    /// Get current workspace
    #[allow(dead_code)]
    pub async fn get_workspace(&self) -> Option<Workspace> {
        self.workspace.lock().await.clone()
    }

    /// Validate if a task is allowed within the current workspace
    pub async fn validate_task(&self, task: &Task) -> Result<(), String> {
        let workspace = self
            .workspace
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| "No workspace context set".to_string())?
            .clone();

        // Check if task belongs to this workspace
        if let Some(task_workspace_id) = &task.workspace_id {
            if task_workspace_id != &workspace.id.to_string() {
                return Err(format!(
                    "Task belongs to different workspace: {}",
                    task_workspace_id
                ));
            }
        }

        // Check if workspace allows task execution
        if !workspace.permissions.can_execute {
            return Err("Task execution not permitted in this workspace".to_string());
        }

        // Validate workspace path if specified
        if let Some(workspace_path) = &task.workspace_path {
            let is_allowed = workspace
                .allowed_paths
                .iter()
                .any(|allowed| workspace_path.starts_with(allowed));

            if !is_allowed {
                return Err(format!(
                    "Task workspace path {} is not within allowed paths",
                    workspace_path
                ));
            }
        }

        Ok(())
    }

    /// Add a task with workspace validation
    pub async fn add_task_with_validation(&self, mut task: Task) -> Result<(), String> {
        // Set workspace context if not already set
        if task.workspace_id.is_none() {
            if let Some(workspace) = self.workspace.lock().await.as_ref() {
                task.workspace_id = Some(workspace.id.to_string());
            }
        }

        // Validate the task
        self.validate_task(&task).await?;

        self.queue.enqueue(task).await;
        Ok(())
    }

    /// Add a task to the manager (legacy method - use add_task_with_validation for workspace-aware tasks)
    #[allow(dead_code)]
    pub async fn add_task(&self, task: Task) {
        // For backward compatibility, try validation but don't fail
        let _ = self.add_task_with_validation(task).await;
    }

    /// Add a task to the manager with validation
    pub async fn add_task_validated(&self, task: Task) -> Result<(), String> {
        self.add_task_with_validation(task).await
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        self.queue.get_task(task_id).await
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<Task> {
        self.queue.get_all_tasks().await
    }

    /// Execute the next available task from the queue with workspace validation
    pub async fn execute_next_task(&self, on_event: Channel<TaskEvent>) -> Result<(), String> {
        // Get next task from queue
        let task = match self.queue.dequeue().await {
            Some(task) => task,
            None => return Err("No tasks available to execute".to_string()),
        };

        // Validate task against workspace
        self.validate_task(&task).await?;

        let task_id = task.id.clone();

        // Emit started event
        on_event
            .send(TaskEvent::Started {
                task_id: task_id.clone(),
            })
            .map_err(|e| e.to_string())?;

        // Execute the task using AI provider
        let task_id_for_closure = task_id.clone();
        let result = self
            .ai_provider
            .execute_prompt(
                &task.provider,
                &task.model,
                &task.description,
                move |progress, _message| {
                    tracing::debug!("Task {} progress: {}%", task_id_for_closure, progress);
                },
                None::<fn(crate::ai::provider_types::StreamingChunk)>,
            )
            .await;

        // Update task status based on result
        match result {
            Ok(_response) => {
                self.queue.complete_task(&task_id).await;
                on_event
                    .send(TaskEvent::Completed {
                        task_id: task_id.clone(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            Err(error) => {
                self.queue.fail_task(&task_id, error.clone()).await;
                on_event
                    .send(TaskEvent::Failed {
                        task_id: task_id.clone(),
                        error: error.clone(),
                    })
                    .map_err(|e| e.to_string())?;
                return Err(error);
            }
        }

        Ok(())
    }

    /// Execute a specific task (for manual execution)
    pub async fn execute_task(
        &self,
        _task_id: &str,
        on_event: Channel<TaskEvent>,
    ) -> Result<(), String> {
        // For now, delegate to execute_next_task if the task is pending
        // In the future, this could be enhanced to execute specific tasks
        self.execute_next_task(on_event).await
    }

    /// Pause a running task
    pub async fn pause_task(&self, task_id: &str) -> Result<(), String> {
        if let Some(mut task) = self.queue.running.get_mut(task_id) {
            task.status = TaskStatus::Paused;
            Ok(())
        } else {
            Err(format!("Task not found or not running: {}", task_id))
        }
    }

    /// Resume a paused task
    pub async fn resume_task(&self, task_id: &str) -> Result<(), String> {
        if let Some(mut task) = self.queue.running.get_mut(task_id) {
            if task.status == TaskStatus::Paused {
                task.status = TaskStatus::Running;
                Ok(())
            } else {
                Err("Task is not paused".to_string())
            }
        } else {
            Err(format!("Task not found or not running: {}", task_id))
        }
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), String> {
        // Try to remove from running tasks
        if let Some((_, mut task)) = self.queue.running.remove(task_id) {
            task.status = TaskStatus::Cancelled;
            task.completed_at = Some(chrono::Utc::now());
            self.queue.completed.insert(task_id.to_string(), task);
            return Ok(());
        }

        // Try to remove from pending tasks
        let mut pending = self.queue.pending.lock().await;
        let mut found = false;
        let mut new_pending = BinaryHeap::new();

        while let Some(priority_task) = pending.pop() {
            if priority_task.task.id == task_id {
                let mut task = priority_task.task;
                task.status = TaskStatus::Cancelled;
                task.completed_at = Some(chrono::Utc::now());
                self.queue.completed.insert(task_id.to_string(), task);
                found = true;
            } else {
                new_pending.push(priority_task);
            }
        }

        *pending = new_pending;

        if found {
            Ok(())
        } else {
            Err(format!("Task not found: {}", task_id))
        }
    }

    /// Start background task processing
    /// Reserved for future background processing feature
    #[allow(dead_code)]
    pub async fn start_background_processing(&self) {
        let queue = Arc::clone(&self.queue);
        let ai_provider = Arc::clone(&self.ai_provider);
        let max_concurrent = self.max_concurrent_tasks;
        let running_handles = Arc::clone(&self.running_handles);

        tokio::spawn(async move {
            loop {
                // Clean up finished handles
                let mut handles = running_handles.lock().await;
                handles.retain(|handle| !handle.is_finished());

                // Start new tasks if under the limit
                while handles.len() < max_concurrent {
                    if let Some(task) = queue.dequeue().await {
                        let task_id = task.id.clone();
                        let queue_clone = Arc::clone(&queue);
                        let ai_provider_clone = Arc::clone(&ai_provider);

                        let task_id_clone = task_id.clone();
                        let handle = tokio::spawn(async move {
                            tracing::info!("Starting background execution of task: {}", task_id);

                            let result = ai_provider_clone
                                .execute_prompt(
                                    &task.provider,
                                    &task.model,
                                    &task.description,
                                    move |progress, _message| {
                                        tracing::debug!(
                                            "Task {} progress: {}%",
                                            task_id_clone,
                                            progress
                                        );
                                    },
                                    None::<fn(crate::ai::provider_types::StreamingChunk)>,
                                )
                                .await;

                            match result {
                                Ok(_) => {
                                    queue_clone.complete_task(&task_id).await;
                                    tracing::info!("Task {} completed successfully", task_id);
                                }
                                Err(error) => {
                                    queue_clone.fail_task(&task_id, error.clone()).await;
                                    tracing::error!("Task {} failed: {}", task_id, error);
                                }
                            }
                        });

                        handles.push(handle);
                    } else {
                        break; // No more tasks available
                    }
                }

                // Wait a bit before checking again
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
    }

    /// Get the number of currently running tasks
    /// Reserved for future background processing feature
    #[allow(dead_code)]
    pub async fn running_task_count(&self) -> usize {
        self.queue.running.len()
    }

    /// Get the number of pending tasks
    /// Reserved for future background processing feature
    #[allow(dead_code)]
    pub async fn pending_task_count(&self) -> usize {
        self.queue.pending.lock().await.len()
    }

    /// Save task manager state to disk
    /// Reserved for future state persistence feature
    #[allow(dead_code)]
    pub async fn save_state(&self, path: &Path) -> Result<(), String> {
        self.queue.save_to_disk(path).await
    }

    /// Load task manager state from disk
    /// Reserved for future state persistence feature
    #[allow(dead_code)]
    pub async fn load_state(&self, path: &Path) -> Result<(), String> {
        self.queue.load_from_disk(path).await
    }
}
