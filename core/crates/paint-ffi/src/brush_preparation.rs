//! Immutable prepared revisions; compilation runs outside document execution pools.
use crate::{
    api::*,
    error::{ApiError, ApiResult},
    file_job::io_error,
};
use paint_core::BrushCatalog;
use paint_task::{Priority, TaskError, TaskPool, TaskTicket};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard, OnceLock},
    thread,
    time::Duration,
};
fn task_error(e: TaskError) -> ApiError {
    ApiError::new(
        match e {
            TaskError::Cancelled | TaskError::Stopped => PAINT_CANCELLED,
            TaskError::QueueFull => PAINT_BUSY,
            _ => PAINT_INTERNAL_ERROR,
        },
        e.to_string(),
    )
}
fn pool() -> ApiResult<&'static TaskPool> {
    static POOL: OnceLock<Result<TaskPool, ApiError>> = OnceLock::new();
    POOL.get_or_init(|| TaskPool::new(2, 32).map_err(task_error))
        .as_ref()
        .map_err(Clone::clone)
}
pub(crate) fn warm_pool() -> ApiResult<()> {
    pool().map(|_| ())
}
struct TicketState {
    task: Option<TaskTicket<ApiResult<Arc<BrushCatalog>>>>,
    result: Option<ApiResult<Arc<BrushCatalog>>>,
}
pub(crate) struct CatalogTicket {
    state: Mutex<TicketState>,
}
impl CatalogTicket {
    pub fn try_result(&self) -> ApiResult<Arc<BrushCatalog>> {
        let mut state = self.state.lock().map_err(|_| ApiError::internal())?;
        if state.result.is_none() {
            if let Some(outcome) = state.task.as_ref().and_then(TaskTicket::try_take) {
                state.result = Some(outcome.map_err(task_error).and_then(|result| result));
                state.task = None;
            }
        }
        state
            .result
            .clone()
            .unwrap_or_else(|| Err(ApiError::new(PAINT_BUSY, "brush revision is preparing")))
    }
    pub fn wait(&self) -> ApiResult<Arc<BrushCatalog>> {
        loop {
            match self.try_result() {
                Err(e) if e.status == PAINT_BUSY => thread::sleep(Duration::from_millis(1)),
                result => return result,
            }
        }
    }
}
struct Records {
    next: u64,
    published: u64,
    entries: VecDeque<(u64, Arc<CatalogTicket>)>,
}
pub(crate) struct PreparedCatalog {
    current: Mutex<BrushCatalog>,
    records: Mutex<Records>,
}
impl PreparedCatalog {
    pub fn new(catalog: BrushCatalog) -> Self {
        Self {
            current: Mutex::new(catalog),
            records: Mutex::new(Records {
                next: 1,
                published: 0,
                entries: VecDeque::new(),
            }),
        }
    }
    pub fn submit(&self, json: String) -> ApiResult<u64> {
        let current = self.current.lock().map_err(|_| ApiError::internal())?;
        if current.ids().iter().any(|id| {
            current
                .preset(id)
                .is_some_and(|p| p.settings().mask.is_some())
        }) {
            return Err(ApiError::new(
                PAINT_UNSUPPORTED,
                "brush resource state codec required",
            ));
        }
        let budget = current.mask_budget();
        drop(current);
        let task = pool()?
            .submit(Priority::Interactive, move |_| {
                Ok(paint_io::brush_state::decode(json.as_bytes(), budget)
                    .map(Arc::new)
                    .map_err(io_error))
            })
            .map_err(task_error)?;
        let ticket = Arc::new(CatalogTicket {
            state: Mutex::new(TicketState {
                task: Some(task),
                result: None,
            }),
        });
        let mut records = self.records.lock().map_err(|_| ApiError::internal())?;
        let sequence = records.next;
        records.next += 1;
        records.entries.push_back((sequence, ticket));
        while records.entries.len() > 64 {
            records.entries.pop_front();
        }
        Ok(sequence)
    }
    pub fn ticket(&self, sequence: u64) -> ApiResult<Arc<CatalogTicket>> {
        self.records
            .lock()
            .map_err(|_| ApiError::internal())?
            .entries
            .iter()
            .find(|(seq, _)| *seq == sequence)
            .map(|(_, ticket)| ticket.clone())
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "brush preparation record expired"))
    }
    fn publish_ready(&self) -> ApiResult<()> {
        let mut records = self.records.lock().map_err(|_| ApiError::internal())?;
        let Some((sequence, ticket)) = records.entries.back() else {
            return Ok(());
        };
        if *sequence <= records.published {
            return Ok(());
        }
        let sequence = *sequence;
        match ticket.try_result() {
            Ok(snapshot) => {
                self.current
                    .lock()
                    .map_err(|_| ApiError::internal())?
                    .replace_validated((*snapshot).clone());
                records.published = sequence;
            }
            Err(e) if e.status == PAINT_BUSY => {}
            Err(_) => {
                records.published = sequence;
            }
        }
        Ok(())
    }
    pub fn current(&self) -> ApiResult<MutexGuard<'_, BrushCatalog>> {
        self.publish_ready()?;
        self.current.lock().map_err(|_| ApiError::internal())
    }
    pub fn mutate<T>(&self, op: impl FnOnce(&mut BrushCatalog) -> ApiResult<T>) -> ApiResult<T> {
        let mut records = self.records.lock().map_err(|_| ApiError::internal())?;
        let mut current = self.current.lock().map_err(|_| ApiError::internal())?;
        if let Some((sequence, ticket)) = records.entries.back() {
            let sequence = *sequence;
            match ticket.try_result() {
                Ok(snapshot) => {
                    if sequence > records.published {
                        current.replace_validated((*snapshot).clone());
                        records.published = sequence;
                    }
                }
                Err(e) if e.status == PAINT_BUSY => return Err(e),
                Err(_) => records.published = sequence,
            }
        }
        op(&mut current)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prepared_task_panic_resolves_as_error_instead_of_waiting_forever() {
        let task = pool()
            .unwrap()
            .submit(
                Priority::Interactive,
                |_| -> Result<ApiResult<Arc<BrushCatalog>>, TaskError> { panic!("fixture panic") },
            )
            .unwrap();
        let ticket = CatalogTicket {
            state: Mutex::new(TicketState {
                task: Some(task),
                result: None,
            }),
        };
        assert_eq!(ticket.wait().unwrap_err().status, PAINT_INTERNAL_ERROR);
    }
    #[test]
    fn retained_ticket_survives_record_eviction_and_failed_draft_preserves_current() {
        let library = PreparedCatalog::new(BrushCatalog::default());
        let json =
            String::from_utf8(paint_io::brush_state::encode(&BrushCatalog::default()).unwrap())
                .unwrap();
        let first = library.submit(json.clone()).unwrap();
        let retained = library.ticket(first).unwrap();
        retained.wait().unwrap();
        for _ in 0..65 {
            let seq = library.submit(json.clone()).unwrap();
            library.ticket(seq).unwrap().wait().unwrap();
        }
        assert_eq!(library.ticket(first).err().unwrap().status, PAINT_NOT_FOUND);
        assert_eq!(retained.wait().unwrap().ids().len(), 4);
        let seq = library.submit("{}".into()).unwrap();
        assert_eq!(
            library.ticket(seq).unwrap().wait().unwrap_err().status,
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(library.current().unwrap().ids().len(), 4);
    }
}
