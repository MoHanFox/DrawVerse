//! Immutable prepared revisions; compilation runs outside document execution pools.
use crate::{
    api::*,
    error::{ApiError, ApiResult},
    file_job::io_error,
};
use paint_core::BrushCatalog;
use paint_io::brush_resource::{Interpretation, ResourceStore, StoredMask};
use paint_task::{Priority, TaskError, TaskPool, TaskTicket};
use std::{
    collections::VecDeque,
    path::PathBuf,
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
    recipe: Option<Recipe>,
}
#[derive(Clone)]
struct Recipe {
    json: String,
    budget: usize,
    resources: Option<Arc<ResourceStore>>,
}
impl Recipe {
    fn start(self) -> ApiResult<TaskTicket<ApiResult<Arc<BrushCatalog>>>> {
        pool()?
            .submit(Priority::Interactive, move |token| {
                Ok(match self.resources {
                    Some(resources) => paint_io::brush_state::decode_with_resources(
                        self.json.as_bytes(),
                        self.budget,
                        &resources,
                        &token,
                    ),
                    None => paint_io::brush_state::decode(self.json.as_bytes(), self.budget),
                }
                .map(Arc::new)
                .map_err(io_error))
            })
            .map_err(task_error)
    }
}
pub(crate) struct CatalogTicket {
    state: Mutex<TicketState>,
}
impl CatalogTicket {
    pub fn try_result(&self) -> ApiResult<Arc<BrushCatalog>> {
        let mut state = self.state.lock().map_err(|_| ApiError::internal())?;
        if state.result.is_none() && state.task.is_none() {
            state.task = Some(
                state
                    .recipe
                    .clone()
                    .ok_or_else(ApiError::internal)?
                    .start()?,
            );
        }
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
    fn retire_image(&self) -> ApiResult<()> {
        let mut state = self.state.lock().map_err(|_| ApiError::internal())?;
        if state.result.is_none() {
            if let Some(outcome) = state.task.as_ref().and_then(TaskTicket::try_take) {
                state.result = Some(outcome.map_err(task_error).and_then(|result| result));
                state.task = None;
            }
        }
        if state.recipe.is_some()
            && state.result.as_ref().is_some_and(|result| {
                result.as_ref().is_ok_and(|catalog| {
                    Arc::strong_count(catalog) == 1
                        && catalog.ids().iter().any(|id| {
                            catalog
                                .preset(id)
                                .is_some_and(|p| p.settings().mask.is_some())
                        })
                })
            })
        {
            state.result = None;
        }
        Ok(())
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
    resources: Mutex<Option<Arc<ResourceStore>>>,
    imported: Mutex<Option<Vec<StoredMask>>>,
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
            resources: Mutex::new(None),
            imported: Mutex::new(None),
        }
    }
    pub fn submit(&self, json: String) -> ApiResult<u64> {
        let current = self.current.lock().map_err(|_| ApiError::internal())?;
        let budget = current.mask_budget();
        drop(current);
        self.retire_unused_images()?;
        let recipe = Recipe {
            json,
            budget,
            resources: self
                .resources
                .lock()
                .map_err(|_| ApiError::internal())?
                .clone(),
        };
        let task = recipe.clone().start()?;
        let ticket = Arc::new(CatalogTicket {
            state: Mutex::new(TicketState {
                task: Some(task),
                result: None,
                recipe: Some(recipe),
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
    fn retire_unused_images(&self) -> ApiResult<()> {
        let records = self.records.lock().map_err(|_| ApiError::internal())?;
        for (_, ticket) in records
            .entries
            .iter()
            .take(records.entries.len().saturating_sub(1))
        {
            if Arc::strong_count(ticket) == 1 {
                ticket.retire_image()?;
            }
        }
        Ok(())
    }
    pub fn bind_resources(&self, directory: PathBuf) -> ApiResult<()> {
        if let Some(store) = self
            .resources
            .lock()
            .map_err(|_| ApiError::internal())?
            .as_ref()
        {
            return if store.directory() == directory {
                Ok(())
            } else {
                Err(ApiError::new(
                    PAINT_BUSY,
                    "brush resources are already bound",
                ))
            };
        }
        let budget = self.current()?.mask_budget();
        let store = Arc::new(ResourceStore::open(directory, budget).map_err(io_error)?);
        self.share_resources(store)
    }
    pub fn share_resources(&self, store: Arc<ResourceStore>) -> ApiResult<()> {
        let mut resources = self.resources.lock().map_err(|_| ApiError::internal())?;
        if let Some(before) = resources.as_ref() {
            if !Arc::ptr_eq(before, &store) {
                return Err(ApiError::new(
                    PAINT_BUSY,
                    "brush resources are already bound",
                ));
            }
        }
        *resources = Some(store);
        Ok(())
    }
    pub fn resource_store(&self) -> ApiResult<Arc<ResourceStore>> {
        self.resources
            .lock()
            .map_err(|_| ApiError::internal())?
            .clone()
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "brush resource directory not bound"))
    }
    pub fn import_file(
        &self,
        path: PathBuf,
        interpretation: Interpretation,
    ) -> ApiResult<StoredMask> {
        self.retire_unused_images()?;
        self.imported
            .lock()
            .map_err(|_| ApiError::internal())?
            .take();
        let resource = self
            .resource_store()?
            .import_file(
                &path,
                interpretation,
                &paint_task::CancellationToken::default(),
            )
            .map_err(io_error)?;
        *self.imported.lock().map_err(|_| ApiError::internal())? = Some(vec![resource.clone()]);
        Ok(resource)
    }
    pub fn import_bundle(&self, bundle: &paint_io::vbr_image::ImageBundle) -> ApiResult<()> {
        if bundle.assets().is_empty() {
            return Ok(());
        }
        self.retire_unused_images()?;
        self.imported
            .lock()
            .map_err(|_| ApiError::internal())?
            .take();
        let store = self.resource_store()?;
        let token = paint_task::CancellationToken::default();
        let mut imported = Vec::new();
        for (id, asset) in bundle.assets() {
            imported.push(
                store
                    .put_verified(id, asset.bytes(), &token)
                    .map_err(io_error)?,
            );
        }
        *self.imported.lock().map_err(|_| ApiError::internal())? = Some(imported);
        Ok(())
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
                let mut imported = self.imported.lock().map_err(|_| ApiError::internal())?;
                if imported.as_ref().is_some_and(|resources| {
                    resources.iter().all(|resource| {
                        snapshot.ids().iter().any(|id| {
                            snapshot.preset(id).is_some_and(|p| {
                                p.settings().mask.as_ref().and_then(|m| m.resource_id())
                                    == Some(resource.id.as_str())
                            })
                        })
                    })
                }) {
                    imported.take();
                }
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
    fn image_revisions_release_unused_pixels_and_rebuild_exact_resources() {
        use paint_core::{BrushMask, Document, InputPoint};
        let directory = tempfile::tempdir().unwrap();
        let library = PreparedCatalog::new(BrushCatalog::default());
        library
            .bind_resources(directory.path().join("assets"))
            .unwrap();
        let preview = PreparedCatalog::new(BrushCatalog::default());
        preview
            .share_resources(library.resource_store().unwrap())
            .unwrap();
        let mut coverage = vec![255; 256];
        for y in 6..10 {
            for x in 6..10 {
                coverage[y * 16 + x] = 0;
            }
        }
        let source = directory.path().join("source.png");
        std::fs::write(
            &source,
            paint_io::brush_resource::encode_mask(&BrushMask::new(16, 16, coverage).unwrap())
                .unwrap(),
        )
        .unwrap();
        let resource = library
            .import_file(source.clone(), Interpretation::Coverage)
            .unwrap();
        let weak = Arc::downgrade(&resource.mask);
        let mut catalog = BrushCatalog::default();
        let mut settings = catalog.selected().settings().clone();
        settings.mask = Some(resource.mask.clone());
        settings.diameter = 16.;
        catalog.edit("round-pressure", settings).unwrap();
        let json = String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap();
        let first = library.submit(json.clone()).unwrap();
        library.ticket(first).unwrap().wait().unwrap();
        drop(library.current().unwrap());
        drop(resource);
        drop(catalog);
        std::fs::remove_file(source).unwrap();
        let round =
            String::from_utf8(paint_io::brush_state::encode(&BrushCatalog::default()).unwrap())
                .unwrap();
        let second = library.submit(round).unwrap();
        library.ticket(second).unwrap().wait().unwrap();
        drop(library.current().unwrap());
        library.retire_unused_images().unwrap();
        assert!(weak.upgrade().is_none());
        let rebuilt = library.ticket(first).unwrap().wait().unwrap();
        assert_eq!(
            paint_io::brush_state::encode(&rebuilt).unwrap(),
            json.as_bytes()
        );
        let captured = library.ticket(first).unwrap();
        library.retire_unused_images().unwrap();
        assert!(Arc::ptr_eq(&rebuilt, &captured.wait().unwrap()));
        drop(library);
        let seq = preview.submit(json).unwrap();
        let restored = preview.ticket(seq).unwrap().wait().unwrap();
        let mut document = Document::new(40, 40).unwrap();
        document
            .begin_brush_stroke(
                restored.selected().settings().definition(),
                InputPoint::new(20.5, 20.5, 1.),
            )
            .unwrap();
        document.end_stroke().unwrap();
        assert_eq!(document.pixel(20, 20).alpha(), 0.);
        assert!(document.pixel(25, 20).alpha() > 0.9);
    }
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
                recipe: None,
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
