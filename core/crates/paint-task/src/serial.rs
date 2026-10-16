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
    retained: usize,
    executing: usize,
}
struct Shared<C> {
    queue: Mutex<Queue<C>>,
    wake: Condvar,
    capacity: usize,
    token: CancellationToken,
    count_running: bool,
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
        if queue
            .items
            .len()
            .saturating_add(queue.retained)
            .saturating_add(queue.executing)
            >= self.0.capacity + usize::from(reserve)
        {
            return Err(SendError::Full);
        }
        queue.items.push_back(command);
        self.0.wake.notify_one();
        Ok(())
    }
    pub fn pending(&self) -> usize {
        let queue = self.0.queue.lock().unwrap_or_else(|e| e.into_inner());
        if queue.state != WorkerState::Running {
            return 0;
        }
        queue
            .items
            .len()
            .saturating_add(queue.retained)
            .saturating_add(queue.executing)
    }
    pub fn state(&self) -> WorkerState {
        self.0.queue.lock().unwrap_or_else(|e| e.into_inner()).state
    }
}
/// Outstanding work held by a resumable callback, excluding the shared mailbox.
#[derive(Clone, Copy, Debug, Default)]
pub struct SerialProgress {
    pub ready: bool,
    pub retained: usize,
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
        Self::spawn_inner(capacity, tick, false, move |command, token| {
            process(command, token);
            SerialProgress::default()
        })
    }
    /// Continue ready work without an idle timer; queued commands get a turn between pages.
    /// Retained command credit must include every command held outside this mailbox.
    pub fn spawn_resumable(
        capacity: usize,
        tick: Duration,
        process: impl FnMut(Option<C>, &CancellationToken) -> SerialProgress + Send + 'static,
    ) -> Result<Self, std::io::Error> {
        Self::spawn_inner(capacity, tick, true, process)
    }
    fn spawn_inner(
        capacity: usize,
        tick: Duration,
        count_running: bool,
        mut process: impl FnMut(Option<C>, &CancellationToken) -> SerialProgress + Send + 'static,
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
                retained: 0,
                executing: 0,
            }),
            wake: Condvar::new(),
            capacity,
            token: CancellationToken::default(),
            count_running,
        });
        let runtime = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("paint-document".into())
            .spawn(move || {
                let mut next_tick = Instant::now();
                let mut ready = false;
                let mut last_was_tick = false;
                loop {
                    let mut queue = runtime.queue.lock().unwrap_or_else(|e| e.into_inner());
                    if queue.state != WorkerState::Running {
                        break;
                    }
                    let command = if Instant::now() >= next_tick
                        && (!last_was_tick || queue.items.is_empty())
                    {
                        None
                    } else if let Some(command) = queue.items.pop_front() {
                        queue.executing = usize::from(runtime.count_running);
                        Some(command)
                    } else if ready {
                        None
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
                    let is_tick = command.is_none();
                    last_was_tick = is_tick;
                    let result =
                        catch_unwind(AssertUnwindSafe(|| process(command, &runtime.token)));
                    if is_tick {
                        next_tick = Instant::now() + tick;
                    }
                    let mut queue = runtime.queue.lock().unwrap_or_else(|e| e.into_inner());
                    queue.executing = 0;
                    match result {
                        Ok(progress)
                            if progress.retained.saturating_add(queue.items.len())
                                <= runtime.capacity + 1 =>
                        {
                            ready = progress.ready;
                            if queue.state == WorkerState::Running {
                                queue.retained = progress.retained;
                            } else {
                                queue.retained = 0;
                            }
                        }
                        _ => {
                            queue.state = WorkerState::Panicked;
                            queue.retained = 0;
                            let removed = std::mem::take(&mut queue.items);
                            drop(queue);
                            drop(removed);
                            break;
                        }
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
