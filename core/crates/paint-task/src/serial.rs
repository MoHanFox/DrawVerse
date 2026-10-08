use crate::CancellationToken;
use std::{
    collections::VecDeque,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerState {
    Running,
    Stopped,
    Panicked,
}
#[derive(Debug, PartialEq, Eq)]
pub enum SendError {
    Full,
    Stopped,
    Panicked,
}
struct Queue<C> {
    items: VecDeque<C>,
    state: WorkerState,
}
struct Shared<C> {
    queue: Mutex<Queue<C>>,
    wake: Condvar,
    capacity: usize,
    token: CancellationToken,
}
pub struct SerialSender<C>(Arc<Shared<C>>);
impl<C> Clone for SerialSender<C> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl<C> SerialSender<C> {
    pub fn try_send(&self, command: C) -> Result<(), SendError> {
        self.enqueue(command, false, |_| false)
    }
    /// One cancellation slot, optionally removing only the contiguous obsolete input tail.
    pub fn send_cancel(&self, command: C, obsolete: impl Fn(&C) -> bool) -> Result<(), SendError> {
        self.enqueue(command, true, obsolete)
    }
    fn enqueue(
        &self,
        command: C,
        reserve: bool,
        obsolete: impl Fn(&C) -> bool,
    ) -> Result<(), SendError> {
        let mut queue = self.0.queue.lock().unwrap_or_else(|e| e.into_inner());
        match queue.state {
            WorkerState::Stopped => return Err(SendError::Stopped),
            WorkerState::Panicked => return Err(SendError::Panicked),
            WorkerState::Running => {}
        }
        if reserve {
            while queue.items.back().is_some_and(&obsolete) {
                queue.items.pop_back();
            }
        }
        if queue.items.len() >= self.0.capacity + usize::from(reserve) {
            return Err(SendError::Full);
        }
        queue.items.push_back(command);
        self.0.wake.notify_one();
        Ok(())
    }
    pub fn pending(&self) -> usize {
        self.0
            .queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .items
            .len()
    }
    pub fn state(&self) -> WorkerState {
        self.0.queue.lock().unwrap_or_else(|e| e.into_inner()).state
    }
}
pub struct SerialWorker<C> {
    sender: SerialSender<C>,
    worker: Option<JoinHandle<()>>,
}
impl<C: Send + 'static> SerialWorker<C> {
    /// None is a periodic publication/render tick. Commands remain strictly FIFO.
    pub fn spawn(
        capacity: usize,
        tick: Duration,
        mut process: impl FnMut(Option<C>, &CancellationToken) + Send + 'static,
    ) -> Result<Self, std::io::Error> {
        if capacity == 0 || capacity > 65536 || tick.is_zero() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid actor limits",
            ));
        }
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue {
                items: VecDeque::new(),
                state: WorkerState::Running,
            }),
            wake: Condvar::new(),
            capacity,
            token: CancellationToken::default(),
        });
        let runtime = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("paint-document".into())
            .spawn(move || {
                let mut next_tick = Instant::now();
                loop {
                    let mut queue = runtime.queue.lock().unwrap_or_else(|e| e.into_inner());
                    if queue.state != WorkerState::Running {
                        break;
                    }
                    let command = if Instant::now() >= next_tick {
                        next_tick = Instant::now() + tick;
                        None
                    } else if let Some(command) = queue.items.pop_front() {
                        Some(command)
                    } else {
                        let _ = runtime
                            .wake
                            .wait_timeout(
                                queue,
                                next_tick.saturating_duration_since(Instant::now()),
                            )
                            .unwrap_or_else(|e| e.into_inner());
                        continue;
                    };
                    drop(queue);
                    if catch_unwind(AssertUnwindSafe(|| process(command, &runtime.token))).is_err()
                    {
                        let mut queue = runtime.queue.lock().unwrap_or_else(|e| e.into_inner());
                        queue.state = WorkerState::Panicked;
                        let removed = std::mem::take(&mut queue.items);
                        drop(queue);
                        drop(removed);
                        break;
                    }
                }
            })?;
        Ok(Self {
            sender: SerialSender(shared),
            worker: Some(worker),
        })
    }
    pub fn sender(&self) -> SerialSender<C> {
        self.sender.clone()
    }
    pub fn shutdown(&mut self) {
        self.sender.0.token.cancel();
        let mut queue = self
            .sender
            .0
            .queue
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if queue.state == WorkerState::Running {
            queue.state = WorkerState::Stopped;
        }
        let removed = std::mem::take(&mut queue.items);
        self.sender.0.wake.notify_all();
        drop(queue);
        drop(removed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl<C> Drop for SerialWorker<C> {
    fn drop(&mut self) {
        self.sender.0.token.cancel();
        let mut queue = self
            .sender
            .0
            .queue
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if queue.state == WorkerState::Running {
            queue.state = WorkerState::Stopped;
        }
        let removed = std::mem::take(&mut queue.items);
        self.sender.0.wake.notify_all();
        drop(queue);
        drop(removed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
