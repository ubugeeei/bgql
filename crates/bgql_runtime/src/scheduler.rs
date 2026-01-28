//! Priority-based query scheduler.
//!
//! This module implements a multi-level feedback queue scheduler
//! for query execution with resource awareness.

use crate::resource::{ResourceManager, ResourceRequirements};
use crate::state::{ExecutionId, ExecutionState};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

/// Task priority level (1 = highest, 10 = lowest).
pub type PriorityLevel = u8;

/// Scheduler configuration.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Number of worker threads.
    pub worker_count: usize,

    /// Maximum tasks in each priority queue.
    pub max_queue_size: usize,

    /// Time slice for each priority level (lower = longer slice).
    pub time_slices: [Duration; 10],

    /// Interval for priority boosting (to prevent starvation).
    pub boost_interval: Duration,

    /// Enable preemption for higher priority tasks.
    pub enable_preemption: bool,

    /// Maximum time a task can run before being preempted.
    pub max_task_runtime: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            worker_count: num_cpus::get(),
            max_queue_size: 1000,
            time_slices: [
                Duration::from_millis(100),  // P1
                Duration::from_millis(150),  // P2
                Duration::from_millis(200),  // P3
                Duration::from_millis(250),  // P4
                Duration::from_millis(300),  // P5 (default)
                Duration::from_millis(400),  // P6
                Duration::from_millis(500),  // P7
                Duration::from_millis(750),  // P8
                Duration::from_millis(1000), // P9
                Duration::from_millis(1500), // P10
            ],
            boost_interval: Duration::from_secs(10),
            enable_preemption: true,
            max_task_runtime: Duration::from_secs(30),
        }
    }
}

/// Get the number of CPUs.
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4)
    }
}

/// Task priority with deadline support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPriority {
    /// Priority level (1-10, 1 = highest).
    pub level: PriorityLevel,

    /// Optional deadline.
    pub deadline: Option<SystemTime>,

    /// Whether task can be preempted.
    pub preemptible: bool,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self {
            level: 5,
            deadline: None,
            preemptible: true,
        }
    }
}

impl TaskPriority {
    /// Creates a critical priority task.
    pub fn critical() -> Self {
        Self {
            level: 1,
            deadline: None,
            preemptible: false,
        }
    }

    /// Creates a high priority task.
    pub fn high() -> Self {
        Self {
            level: 2,
            deadline: None,
            preemptible: true,
        }
    }

    /// Creates a normal priority task.
    pub fn normal() -> Self {
        Self::default()
    }

    /// Creates a low priority task.
    pub fn low() -> Self {
        Self {
            level: 8,
            deadline: None,
            preemptible: true,
        }
    }

    /// Creates a background priority task.
    pub fn background() -> Self {
        Self {
            level: 10,
            deadline: None,
            preemptible: true,
        }
    }

    /// Sets a deadline.
    pub fn with_deadline(mut self, deadline: SystemTime) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Calculates effective priority considering deadline.
    pub fn effective_priority(&self) -> u8 {
        if let Some(deadline) = self.deadline {
            if let Ok(remaining) = deadline.duration_since(SystemTime::now()) {
                // Boost priority as deadline approaches
                if remaining < Duration::from_secs(1) {
                    return 1;
                } else if remaining < Duration::from_secs(5) {
                    return self.level.saturating_sub(2).max(1);
                } else if remaining < Duration::from_secs(10) {
                    return self.level.saturating_sub(1).max(1);
                }
            } else {
                // Deadline passed - highest priority
                return 1;
            }
        }
        self.level
    }
}

/// A scheduled task.
pub struct ScheduledTask {
    /// Unique task ID.
    pub id: String,

    /// Execution ID this task belongs to.
    pub execution_id: ExecutionId,

    /// Task priority.
    pub priority: TaskPriority,

    /// Resource requirements.
    pub requirements: ResourceRequirements,

    /// When the task was submitted.
    pub submitted_at: Instant,

    /// Number of times this task has been queued (for MLFQ).
    pub queue_count: usize,

    /// Task payload/closure.
    task_fn: Box<dyn FnOnce() -> TaskResult + Send>,
}

impl ScheduledTask {
    /// Creates a new scheduled task.
    pub fn new<F>(
        execution_id: ExecutionId,
        priority: TaskPriority,
        requirements: ResourceRequirements,
        task_fn: F,
    ) -> Self
    where
        F: FnOnce() -> TaskResult + Send + 'static,
    {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            execution_id,
            priority,
            requirements,
            submitted_at: Instant::now(),
            queue_count: 0,
            task_fn: Box::new(task_fn),
        }
    }

    /// Executes the task.
    pub fn execute(self) -> TaskResult {
        (self.task_fn)()
    }
}

/// UUID generation (simple implementation).
mod uuid {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub struct Uuid(u128);

    impl Uuid {
        pub fn new_v4() -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
            let random = (timestamp ^ counter) | (counter << 48);
            Self((timestamp as u128) << 64 | random as u128)
        }
    }

    impl std::fmt::Display for Uuid {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:032x}", self.0)
        }
    }
}

/// Task execution result.
#[derive(Debug, Clone)]
pub enum TaskResult {
    /// Task completed successfully.
    Completed(serde_json::Value),

    /// Task yielded and should be rescheduled.
    Yielded {
        /// Progress so far.
        progress: f64,
        /// Continuation data.
        checkpoint: Option<serde_json::Value>,
    },

    /// Task failed with error.
    Failed(String),

    /// Task was cancelled.
    Cancelled,
}

/// Task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Waiting in queue.
    Queued,
    /// Currently running.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed with error.
    Failed,
    /// Cancelled.
    Cancelled,
    /// Paused.
    Paused,
}

/// Entry in the priority queue.
struct QueueEntry {
    task: ScheduledTask,
    effective_priority: u8,
}

impl Eq for QueueEntry {}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.task.id == other.task.id
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower priority number = higher priority
        // Use submission time as tiebreaker (earlier = higher priority)
        other
            .effective_priority
            .cmp(&self.effective_priority)
            .then_with(|| other.task.submitted_at.cmp(&self.task.submitted_at))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Execution handle for tracking and controlling a submitted task.
pub struct ExecutionHandle {
    /// Task ID.
    pub task_id: String,

    /// Execution ID.
    pub execution_id: ExecutionId,

    /// Channel for receiving status updates.
    status_rx: broadcast::Receiver<TaskStatusUpdate>,
}

impl std::fmt::Debug for ExecutionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionHandle")
            .field("task_id", &self.task_id)
            .field("execution_id", &self.execution_id)
            .finish()
    }
}

impl Clone for ExecutionHandle {
    fn clone(&self) -> Self {
        Self {
            task_id: self.task_id.clone(),
            execution_id: self.execution_id.clone(),
            status_rx: self.status_rx.resubscribe(),
        }
    }
}

impl ExecutionHandle {
    /// Waits for the task to complete and returns the result.
    pub async fn wait(mut self) -> TaskResult {
        loop {
            match self.status_rx.recv().await {
                Ok(update) if update.task_id == self.task_id => match update.status {
                    TaskStatus::Completed => {
                        return update
                            .result
                            .unwrap_or(TaskResult::Completed(serde_json::Value::Null));
                    }
                    TaskStatus::Failed => {
                        return update
                            .result
                            .unwrap_or(TaskResult::Failed("Unknown error".to_string()));
                    }
                    TaskStatus::Cancelled => {
                        return TaskResult::Cancelled;
                    }
                    _ => continue,
                },
                Err(broadcast::error::RecvError::Closed) => {
                    return TaskResult::Failed("Scheduler closed".to_string());
                }
                _ => continue,
            }
        }
    }

    /// Gets the current status.
    pub async fn status(&mut self) -> Option<TaskStatus> {
        match self.status_rx.try_recv() {
            Ok(update) if update.task_id == self.task_id => Some(update.status),
            _ => None,
        }
    }
}

/// Status update message.
#[derive(Debug, Clone)]
struct TaskStatusUpdate {
    task_id: String,
    status: TaskStatus,
    result: Option<TaskResult>,
}

/// The query scheduler.
pub struct QueryScheduler {
    /// Configuration.
    config: SchedulerConfig,

    /// Resource manager.
    resource_manager: Arc<ResourceManager>,

    /// Priority queue.
    queue: Arc<Mutex<BinaryHeap<QueueEntry>>>,

    /// Running tasks.
    running: Arc<RwLock<HashMap<String, RunningTask>>>,

    /// Execution states.
    executions: Arc<RwLock<HashMap<ExecutionId, ExecutionState>>>,

    /// Status broadcast channel.
    status_tx: broadcast::Sender<TaskStatusUpdate>,

    /// Shutdown signal.
    shutdown_tx: Option<mpsc::Sender<()>>,

    /// Statistics.
    stats: Arc<SchedulerStats>,

    /// Running state.
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

/// Information about a running task.
#[derive(Debug)]
#[allow(dead_code)]
struct RunningTask {
    task_id: String,
    execution_id: ExecutionId,
    started_at: Instant,
    priority: TaskPriority,
}

/// Scheduler statistics.
#[derive(Debug, Default)]
pub struct SchedulerStats {
    /// Total tasks submitted.
    pub tasks_submitted: AtomicUsize,

    /// Total tasks completed.
    pub tasks_completed: AtomicUsize,

    /// Total tasks failed.
    pub tasks_failed: AtomicUsize,

    /// Total tasks cancelled.
    pub tasks_cancelled: AtomicUsize,

    /// Total wait time (nanoseconds).
    pub total_wait_time: AtomicU64,

    /// Total execution time (nanoseconds).
    pub total_execution_time: AtomicU64,

    /// Current queue depth.
    pub queue_depth: AtomicUsize,
}

impl QueryScheduler {
    /// Creates a new scheduler with default configuration.
    pub fn new(resource_manager: Arc<ResourceManager>) -> Self {
        Self::with_config(resource_manager, SchedulerConfig::default())
    }

    /// Creates a scheduler with custom configuration.
    pub fn with_config(resource_manager: Arc<ResourceManager>, config: SchedulerConfig) -> Self {
        let (status_tx, _) = broadcast::channel(1024);

        Self {
            config,
            resource_manager,
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            running: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            status_tx,
            shutdown_tx: None,
            stats: Arc::new(SchedulerStats::default()),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Starts the scheduler workers.
    pub async fn start(&mut self) {
        if self.is_running.swap(true, AtomicOrdering::SeqCst) {
            return; // Already running
        }

        let (shutdown_tx, _shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Spawn worker tasks
        for worker_id in 0..self.config.worker_count {
            let queue = Arc::clone(&self.queue);
            let running = Arc::clone(&self.running);
            let resource_manager = Arc::clone(&self.resource_manager);
            let status_tx = self.status_tx.clone();
            let stats = Arc::clone(&self.stats);
            let is_running = Arc::clone(&self.is_running);

            tokio::spawn(async move {
                Self::worker_loop(
                    worker_id,
                    queue,
                    running,
                    resource_manager,
                    status_tx,
                    stats,
                    is_running,
                )
                .await;
            });
        }

        // Spawn priority boost task
        let queue = Arc::clone(&self.queue);
        let boost_interval = self.config.boost_interval;
        let is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(boost_interval);
            while is_running.load(AtomicOrdering::SeqCst) {
                interval.tick().await;
                Self::boost_priorities(&queue).await;
            }
        });
    }

    /// Stops the scheduler.
    pub async fn stop(&mut self) {
        self.is_running.store(false, AtomicOrdering::SeqCst);
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }

    /// Worker loop for processing tasks.
    async fn worker_loop(
        _worker_id: usize,
        queue: Arc<Mutex<BinaryHeap<QueueEntry>>>,
        running: Arc<RwLock<HashMap<String, RunningTask>>>,
        resource_manager: Arc<ResourceManager>,
        status_tx: broadcast::Sender<TaskStatusUpdate>,
        stats: Arc<SchedulerStats>,
        is_running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        while is_running.load(AtomicOrdering::SeqCst) {
            // Try to get a task
            let task = {
                let mut queue = queue.lock().await;
                queue.pop()
            };

            let Some(entry) = task else {
                // No task available, wait a bit
                tokio::time::sleep(Duration::from_millis(1)).await;
                continue;
            };

            let task = entry.task;
            let task_id = task.id.clone();
            let execution_id = task.execution_id.clone();
            let wait_time = task.submitted_at.elapsed();
            let priority = task.priority.clone();

            stats
                .total_wait_time
                .fetch_add(wait_time.as_nanos() as u64, AtomicOrdering::Relaxed);
            stats.queue_depth.fetch_sub(1, AtomicOrdering::Relaxed);

            // Try to allocate resources
            let guard = match resource_manager
                .try_allocate(&execution_id, task.requirements.clone())
                .await
            {
                Some(g) => g,
                None => {
                    // Re-queue the task
                    let mut queue = queue.lock().await;
                    queue.push(QueueEntry {
                        task,
                        effective_priority: entry.effective_priority,
                    });
                    stats.queue_depth.fetch_add(1, AtomicOrdering::Relaxed);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
            };

            // Mark as running
            {
                let mut running = running.write().await;
                running.insert(
                    task_id.clone(),
                    RunningTask {
                        task_id: task_id.clone(),
                        execution_id: execution_id.clone(),
                        started_at: Instant::now(),
                        priority,
                    },
                );
            }

            // Notify running
            let _ = status_tx.send(TaskStatusUpdate {
                task_id: task_id.clone(),
                status: TaskStatus::Running,
                result: None,
            });

            // Execute the task
            let start = Instant::now();
            let result = task.execute();
            let execution_time = start.elapsed();

            stats
                .total_execution_time
                .fetch_add(execution_time.as_nanos() as u64, AtomicOrdering::Relaxed);

            // Remove from running
            {
                let mut running = running.write().await;
                running.remove(&task_id);
            }

            // Drop resource guard
            drop(guard);

            // Update stats and notify
            match &result {
                TaskResult::Completed(_) => {
                    stats.tasks_completed.fetch_add(1, AtomicOrdering::Relaxed);
                    let _ = status_tx.send(TaskStatusUpdate {
                        task_id,
                        status: TaskStatus::Completed,
                        result: Some(result),
                    });
                }
                TaskResult::Failed(_) => {
                    stats.tasks_failed.fetch_add(1, AtomicOrdering::Relaxed);
                    let _ = status_tx.send(TaskStatusUpdate {
                        task_id,
                        status: TaskStatus::Failed,
                        result: Some(result),
                    });
                }
                TaskResult::Cancelled => {
                    stats.tasks_cancelled.fetch_add(1, AtomicOrdering::Relaxed);
                    let _ = status_tx.send(TaskStatusUpdate {
                        task_id,
                        status: TaskStatus::Cancelled,
                        result: Some(result),
                    });
                }
                TaskResult::Yielded { .. } => {
                    // Task yielded, would be rescheduled by caller
                    let _ = status_tx.send(TaskStatusUpdate {
                        task_id,
                        status: TaskStatus::Paused,
                        result: Some(result),
                    });
                }
            }
        }
    }

    /// Boosts priorities of long-waiting tasks.
    async fn boost_priorities(queue: &Arc<Mutex<BinaryHeap<QueueEntry>>>) {
        let mut queue = queue.lock().await;
        let mut entries: Vec<_> = std::mem::take(&mut *queue).into_vec();

        let now = Instant::now();
        for entry in &mut entries {
            let wait_time = now.duration_since(entry.task.submitted_at);
            if wait_time > Duration::from_secs(5) {
                // Boost priority by 1 for every 5 seconds of waiting
                let boost = (wait_time.as_secs() / 5) as u8;
                entry.effective_priority = entry.effective_priority.saturating_sub(boost).max(1);
            }
        }

        *queue = entries.into_iter().collect();
    }

    /// Submits a task for execution.
    pub async fn submit(&self, task: ScheduledTask) -> ExecutionHandle {
        let task_id = task.id.clone();
        let execution_id = task.execution_id.clone();
        let effective_priority = task.priority.effective_priority();

        self.stats
            .tasks_submitted
            .fetch_add(1, AtomicOrdering::Relaxed);
        self.stats.queue_depth.fetch_add(1, AtomicOrdering::Relaxed);

        {
            let mut queue = self.queue.lock().await;
            queue.push(QueueEntry {
                task,
                effective_priority,
            });
        }

        ExecutionHandle {
            task_id,
            execution_id,
            status_rx: self.status_tx.subscribe(),
        }
    }

    /// Submits a function as a task.
    pub async fn submit_fn<F, R>(
        &self,
        execution_id: ExecutionId,
        priority: TaskPriority,
        f: F,
    ) -> ExecutionHandle
    where
        F: FnOnce() -> R + Send + 'static,
        R: Into<TaskResult>,
    {
        let task = ScheduledTask::new(
            execution_id,
            priority,
            ResourceRequirements::minimal(),
            move || f().into(),
        );
        self.submit(task).await
    }

    /// Cancels a task.
    pub async fn cancel(&self, task_id: &str) -> bool {
        // Try to remove from queue
        let mut queue = self.queue.lock().await;
        let entries: Vec<_> = std::mem::take(&mut *queue).into_vec();
        let (cancelled, remaining): (Vec<_>, Vec<_>) =
            entries.into_iter().partition(|e| e.task.id == task_id);

        *queue = remaining.into_iter().collect();

        if !cancelled.is_empty() {
            self.stats
                .tasks_cancelled
                .fetch_add(1, AtomicOrdering::Relaxed);
            self.stats.queue_depth.fetch_sub(1, AtomicOrdering::Relaxed);

            let _ = self.status_tx.send(TaskStatusUpdate {
                task_id: task_id.to_string(),
                status: TaskStatus::Cancelled,
                result: Some(TaskResult::Cancelled),
            });

            return true;
        }

        // Check if running (would need cooperation to cancel)
        let running = self.running.read().await;
        running.contains_key(task_id)
    }

    /// Gets scheduler statistics.
    pub fn stats(&self) -> &SchedulerStats {
        &self.stats
    }

    /// Gets current queue depth.
    pub async fn queue_depth(&self) -> usize {
        self.queue.lock().await.len()
    }

    /// Gets number of running tasks.
    pub async fn running_count(&self) -> usize {
        self.running.read().await.len()
    }

    /// Registers an execution state.
    pub async fn register_execution(&self, state: ExecutionState) {
        let mut executions = self.executions.write().await;
        executions.insert(state.id.clone(), state);
    }

    /// Gets an execution state.
    pub async fn get_execution(&self, id: &str) -> Option<ExecutionState> {
        let executions = self.executions.read().await;
        executions.get(id).cloned()
    }

    /// Updates an execution state.
    pub async fn update_execution<F>(&self, id: &str, f: F) -> Option<ExecutionState>
    where
        F: FnOnce(&mut ExecutionState),
    {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(id) {
            f(state);
            Some(state.clone())
        } else {
            None
        }
    }
}

/// Implementation for converting basic types to TaskResult.
impl From<()> for TaskResult {
    fn from(_: ()) -> Self {
        TaskResult::Completed(serde_json::Value::Null)
    }
}

impl From<serde_json::Value> for TaskResult {
    fn from(value: serde_json::Value) -> Self {
        TaskResult::Completed(value)
    }
}

impl<T, E> From<Result<T, E>> for TaskResult
where
    T: Into<TaskResult>,
    E: std::fmt::Display,
{
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(v) => v.into(),
            Err(e) => TaskResult::Failed(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_priority() {
        let critical = TaskPriority::critical();
        let normal = TaskPriority::normal();
        let low = TaskPriority::low();

        assert!(critical.effective_priority() < normal.effective_priority());
        assert!(normal.effective_priority() < low.effective_priority());
    }

    #[test]
    fn test_priority_with_deadline() {
        let normal = TaskPriority::normal();
        let urgent =
            TaskPriority::normal().with_deadline(SystemTime::now() + Duration::from_secs(1));

        // Urgent task should have higher effective priority
        assert!(urgent.effective_priority() < normal.effective_priority());
    }

    #[tokio::test]
    async fn test_scheduler_submit_and_complete() {
        let rm = Arc::new(ResourceManager::new());
        let mut scheduler = QueryScheduler::new(rm);
        scheduler.start().await;

        let handle = scheduler
            .submit_fn(
                "exec-1".into(),
                TaskPriority::normal(),
                || serde_json::json!({"result": "success"}),
            )
            .await;

        let result = handle.wait().await;
        matches!(result, TaskResult::Completed(_));

        scheduler.stop().await;
    }

    #[tokio::test]
    async fn test_scheduler_priority_ordering() {
        let rm = Arc::new(ResourceManager::new());
        let scheduler = QueryScheduler::new(rm);

        // Submit low priority first
        let _low = scheduler
            .submit_fn("exec-low".into(), TaskPriority::low(), || ())
            .await;

        // Submit high priority second
        let _high = scheduler
            .submit_fn("exec-high".into(), TaskPriority::high(), || ())
            .await;

        // High priority should be first in queue
        let queue = scheduler.queue.lock().await;
        let first = queue.peek().unwrap();
        assert!(first.effective_priority < 8); // High priority
    }

    #[test]
    fn test_queue_entry_ordering() {
        let task1 = ScheduledTask::new(
            "exec-1".into(),
            TaskPriority::normal(),
            ResourceRequirements::minimal(),
            || TaskResult::Completed(serde_json::Value::Null),
        );

        let task2 = ScheduledTask::new(
            "exec-2".into(),
            TaskPriority::high(),
            ResourceRequirements::minimal(),
            || TaskResult::Completed(serde_json::Value::Null),
        );

        let entry1 = QueueEntry {
            task: task1,
            effective_priority: 5,
        };

        let entry2 = QueueEntry {
            task: task2,
            effective_priority: 2,
        };

        // entry2 (priority 2) should be greater (higher priority) than entry1 (priority 5)
        assert!(entry2 > entry1);
    }
}
