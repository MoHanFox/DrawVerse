//! UI-independent scheduling. Destruction joins threads; call it off the UI thread.
mod pool;
mod serial;
pub use pool::{Priority, TaskError, TaskPool, TaskTicket};
pub use serial::{SendError, SerialSender, SerialWorker, WorkerState};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    pub fn checkpoint(&self) -> Result<(), TaskError> {
        if self.is_cancelled() {
            Err(TaskError::Cancelled)
        } else {
            Ok(())
        }
    }
}
