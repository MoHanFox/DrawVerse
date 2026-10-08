use crate::CancellationToken;
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicU64, Ordering as AtomicOrdering},
        Arc, Condvar, Mutex,
    },
    thread::{self, JoinHandle},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Background,
    Normal,
    Interactive,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskError {
    Cancelled,
    QueueFull,
    Stopped,
    Panicked,
    Failed(String),
}
impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TaskError {}

pub struct TaskTicket<T> {
    id: u64,
    token: CancellationToken,
    result: Arc<Mutex<Option<Result<T, TaskError>>>>,
}
impl<T> TaskTicket<T> {
    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn cancel(&self) {
        self.token.cancel();
    }
    pub fn try_take(&self) -> Option<Result<T, TaskError>> {
        self.result.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}
struct Job {
    id: u64,
    priority: Priority,
    token: CancellationToken,
    run: Option<Box<dyn FnOnce() + Send>>,
    cancel: Option<Box<dyn FnOnce() + Send>>,
}
impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Job {}
impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.id.cmp(&self.id))
    }
}
struct Queue {
    jobs: BinaryHeap<Job>,
    active: Vec<CancellationToken>,
    stopped: bool,
}
struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
    capacity: usize,
    ids: AtomicU64,
}
pub struct TaskPool {
    shared: Arc<Shared>,
    workers: Mutex<Vec<JoinHandle<()>>>,
}
impl TaskPool {
    pub fn new(threads: usize, capacity: usize) -> Result<Self, TaskError> {
        if !(1..=32).contains(&threads) || !(1..=65536).contains(&capacity) {
            return Err(TaskError::Failed("invalid pool limits".into()));
        }
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue {
                jobs: BinaryHeap::new(),
                active: Vec::new(),
                stopped: false,
            }),
            wake: Condvar::new(),
            capacity,
            ids: AtomicU64::new(1),
        });
        let pool = Self {
            shared,
            workers: Mutex::new(Vec::new()),
        };
        for index in 0..threads {
            let shared = Arc::clone(&pool.shared);
            let worker = thread::Builder::new()
                .name(format!("paint-cpu-{index}"))
                .spawn(move || worker(shared))
                .map_err(|error| TaskError::Failed(error.to_string()))?;
            pool.workers
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(worker);
        }
        Ok(pool)
    }
    pub fn submit<T: Send + 'static>(
        &self,
        priority: Priority,
        operation: impl FnOnce(CancellationToken) -> Result<T, TaskError> + Send + 'static,
    ) -> Result<TaskTicket<T>, TaskError> {
        let token = CancellationToken::default();
        let result = Arc::new(Mutex::new(None));
        let id = self.shared.ids.fetch_add(1, AtomicOrdering::Relaxed);
        let run_result = Arc::clone(&result);
        let run_token = token.clone();
        let cancel_result = Arc::clone(&result);
        let job = Job {
            id,
            priority,
            token: token.clone(),
            run: Some(Box::new(move || {
                let outcome = if run_token.is_cancelled() {
                    Err(TaskError::Cancelled)
                } else {
                    catch_unwind(AssertUnwindSafe(|| operation(run_token.clone())))
                        .unwrap_or(Err(TaskError::Panicked))
                };
                let outcome = if run_token.is_cancelled() {
                    Err(TaskError::Cancelled)
                } else {
                    outcome
                };
                *run_result.lock().unwrap_or_else(|e| e.into_inner()) = Some(outcome);
            })),
            cancel: Some(Box::new(move || {
                *cancel_result.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(Err(TaskError::Cancelled));
            })),
        };
        let mut queue = self.shared.queue.lock().unwrap_or_else(|e| e.into_inner());
        if queue.stopped {
            return Err(TaskError::Stopped);
        }
        if queue.jobs.len() >= self.shared.capacity {
            return Err(TaskError::QueueFull);
        }
        queue.jobs.push(job);
        self.shared.wake.notify_one();
        Ok(TaskTicket { id, token, result })
    }
    /// Remove a queued closure immediately, releasing any captured pixel snapshots.
    pub fn cancel<T>(&self, ticket: &TaskTicket<T>) {
        ticket.cancel();
        let mut queue = self.shared.queue.lock().unwrap_or_else(|e| e.into_inner());
        let mut removed = None;
        let jobs = std::mem::take(&mut queue.jobs).into_vec();
        for job in jobs {
            if job.id == ticket.id {
                removed = Some(job);
            } else {
                queue.jobs.push(job);
            }
        }
        drop(queue);
        if let Some(mut job) = removed {
            if let Some(cancel) = job.cancel.take() {
                cancel();
            }
        }
    }
    pub fn shutdown(&self) {
        let mut queue = self.shared.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.stopped = true;
        for token in &queue.active {
            token.cancel();
        }
        let jobs = std::mem::take(&mut queue.jobs).into_vec();
        self.shared.wake.notify_all();
        drop(queue);
        for mut job in jobs {
            job.token.cancel();
            if let Some(cancel) = job.cancel.take() {
                cancel();
            }
        }
        let workers = std::mem::take(&mut *self.workers.lock().unwrap_or_else(|e| e.into_inner()));
        for worker in workers {
            let _ = worker.join();
        }
    }
}
impl Drop for TaskPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}
fn worker(shared: Arc<Shared>) {
    loop {
        let mut queue = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
        while queue.jobs.is_empty() && !queue.stopped {
            queue = shared.wake.wait(queue).unwrap_or_else(|e| e.into_inner());
        }
        if queue.stopped {
            return;
        }
        let mut job = queue.jobs.pop().expect("nonempty queue");
        queue.active.push(job.token.clone());
        drop(queue);
        if let Some(run) = job.run.take() {
            run();
        }
        let mut queue = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
        // At most 32 workers; finished tokens are identified by their shared flag address.
        queue
            .active
            .retain(|token| !Arc::ptr_eq(&token.0, &job.token.0));
    }
}
