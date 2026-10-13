//! Immutable prepared revisions; compilation runs outside document execution pools.
use crate::{
    api::*,
    error::{ApiError, ApiResult},
    file_job::io_error,
};
use paint_core::BrushCatalog;
use paint_io::brush_resource::{Interpretation, ResourceStore, StoredMask};
use paint_io::brush_source::ValidatedSource;
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
    anchors: Arc<Mutex<Vec<Arc<ValidatedSource>>>>,
}
impl Recipe {
    fn start(self) -> ApiResult<TaskTicket<ApiResult<Arc<BrushCatalog>>>> {
        pool()?
            .submit(Priority::Interactive, move |token| {
                let result = match self.resources {
                    Some(resources) => resources.source_store().and_then(|sources| {
                        paint_io::brush_state::decode_captured(
                            self.json.as_bytes(),
                            self.budget,
                            &resources,
                            &sources,
                            &token,
                        )
                    }),
                    None => paint_io::brush_state::decode(self.json.as_bytes(), self.budget)
                        .map(|catalog| (catalog, Vec::new())),
                }
                .map_err(io_error)
                .and_then(|(catalog, receipts)| {
                    *self.anchors.lock().map_err(|_| ApiError::internal())? = receipts;
                    Ok(Arc::new(catalog))
                });
                Ok(result)
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
    fn source_receipts(&self) -> ApiResult<Vec<Arc<ValidatedSource>>> {
        let state = self.state.lock().map_err(|_| ApiError::internal())?;
        match &state.recipe {
            Some(recipe) => Ok(recipe
                .anchors
                .lock()
                .map_err(|_| ApiError::internal())?
                .clone()),
            None => Ok(Vec::new()),
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
                            catalog.preset(id).is_some_and(|p| {
                                p.settings().mask.is_some() || p.source_resource().is_some()
                            })
                        })
                })
            })
        {
            state.result = None;
            if let Some(recipe) = &state.recipe {
                recipe
                    .anchors
                    .lock()
                    .map_err(|_| ApiError::internal())?
                    .clear();
            }
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
    current_receipts: Mutex<Vec<Arc<ValidatedSource>>>,
    records: Mutex<Records>,
    resources: Mutex<Option<Arc<ResourceStore>>>,
    imported: Mutex<Option<Vec<StoredMask>>>,
    imported_sources: Mutex<Option<Vec<Arc<ValidatedSource>>>>,
}
impl PreparedCatalog {
    pub fn new(catalog: BrushCatalog) -> Self {
        Self {
            current: Mutex::new(catalog),
            current_receipts: Mutex::new(Vec::new()),
            records: Mutex::new(Records {
                next: 1,
                published: 0,
                entries: VecDeque::new(),
            }),
            resources: Mutex::new(None),
            imported: Mutex::new(None),
            imported_sources: Mutex::new(None),
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
            anchors: Arc::new(Mutex::new(Vec::new())),
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
        if bundle.assets().is_empty() && bundle.sources().is_empty() {
            return Ok(());
        }
        self.retire_unused_images()?;
        self.imported
            .lock()
            .map_err(|_| ApiError::internal())?
            .take();
        let store = self.resource_store()?;
        let token = paint_task::CancellationToken::default();
        self.imported_sources
            .lock()
            .map_err(|_| ApiError::internal())?
            .take();
        let mut imported = Vec::new();
        let needed = bundle
            .package
            .brushes
            .iter()
            .filter_map(|p| p.tip_resource.as_deref())
            .collect::<std::collections::BTreeSet<_>>();
        for (id, asset) in bundle.assets() {
            let resource = store
                .put_verified(id, asset.bytes(), &token)
                .map_err(io_error)?;
            if needed.contains(id.as_str()) {
                imported.push(resource);
            }
        }
        let mut source_ids = std::collections::BTreeSet::new();
        for batch in bundle.sources() {
            source_ids.extend(
                store
                    .source_store()
                    .map_err(io_error)?
                    .save_batch(batch, &token)
                    .map_err(io_error)?,
            );
        }
        if !source_ids.is_empty() {
            let receipts = store
                .source_store()
                .map_err(io_error)?
                .resolve_references(&source_ids, &token)
                .map_err(io_error)?;
            *self
                .imported_sources
                .lock()
                .map_err(|_| ApiError::internal())? = Some(receipts);
        }
        *self.imported.lock().map_err(|_| ApiError::internal())? = Some(imported);
        Ok(())
    }
    pub fn import_abr(&self, batch: paint_io::abr_import::ImportBatch) -> ApiResult<()> {
        self.retire_unused_images()?;
        self.imported
            .lock()
            .map_err(|_| ApiError::internal())?
            .take();
        self.imported_sources
            .lock()
            .map_err(|_| ApiError::internal())?
            .take();
        let store = self.resource_store()?;
        let token = paint_task::CancellationToken::default();
        let presets = batch.persist(&store, &token).map_err(io_error)?;
        let ids = presets.iter().map(|p| p.source_id().to_owned()).collect();
        let receipts = store
            .source_store()
            .map_err(io_error)?
            .resolve_references(&ids, &token)
            .map_err(io_error)?;
        let mut masks = std::collections::BTreeMap::new();
        for preset in presets {
            if let Some(mask) = &preset.settings().mask {
                let id = mask
                    .resource_id()
                    .ok_or_else(ApiError::internal)?
                    .to_owned();
                masks.entry(id.clone()).or_insert(StoredMask {
                    id,
                    mask: mask.clone(),
                    source_depth: 8,
                });
            }
        }
        *self.imported.lock().map_err(|_| ApiError::internal())? =
            Some(masks.into_values().collect());
        *self
            .imported_sources
            .lock()
            .map_err(|_| ApiError::internal())? = Some(receipts);
        Ok(())
    }
    fn adopt_imports(&self, snapshot: &BrushCatalog) -> ApiResult<()> {
        let mut source_handoff = self
            .imported_sources
            .lock()
            .map_err(|_| ApiError::internal())?;
        if source_handoff.as_ref().is_some_and(|sources| {
            sources.iter().all(|source| {
                snapshot.ids().iter().any(|id| {
                    snapshot
                        .preset(id)
                        .is_some_and(|p| p.source_resource() == Some(source.id()))
                })
            })
        }) {
            source_handoff.take();
        }
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
                let receipts = ticket.source_receipts()?;
                let mut held = self
                    .current_receipts
                    .lock()
                    .map_err(|_| ApiError::internal())?;
                self.current
                    .lock()
                    .map_err(|_| ApiError::internal())?
                    .replace_validated((*snapshot).clone());
                *held = receipts;
                self.adopt_imports(&snapshot)?;
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
                        let receipts = ticket.source_receipts()?;
                        let mut held = self
                            .current_receipts
                            .lock()
                            .map_err(|_| ApiError::internal())?;
                        current.replace_validated((*snapshot).clone());
                        *held = receipts;
                        self.adopt_imports(&snapshot)?;
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
#[path = "../../paint-io/tests/support/abr_fixture.rs"]
mod abr_fixture;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_only_bundle_does_not_keep_baseline_pixels_alive_after_import() {
        use paint_io::{
            abr_conversion::ConvertedPreset,
            brush_source::{SourceArchive, SourceBatch},
            ps_descriptor::{Class, Descriptor},
            vbr::{Package, Preset},
        };
        let descriptor = Descriptor {
            class: Class {
                name: String::new(),
                id: b"null".to_vec(),
            },
            entries: Vec::new(),
        };
        let original = Arc::new(vec![0, 6, 0, 2, 37]);
        let token = paint_task::CancellationToken::default();
        let mask = Arc::new(paint_core::BrushMask::new(8, 8, vec![255; 64]).unwrap());
        let settings = paint_core::BrushSettings {
            mask: Some(mask.clone()),
            ..Default::default()
        };
        let converted = ConvertedPreset {
            name: "baseline",
            settings,
            source: &descriptor,
            issues: Vec::new(),
        };
        let archive = SourceArchive::capture(original.clone(), 0, &converted).unwrap();
        let record = archive.record().unwrap();
        let source = paint_io::brush_resource::hash(&record);
        let png = paint_io::brush_resource::encode_mask(&mask).unwrap();
        let tip = paint_io::brush_resource::hash(&png);
        let batch = SourceBatch::from_records(
            original,
            std::collections::BTreeMap::from([(source.clone(), record)]),
            &token,
        )
        .unwrap();
        let package = Package {
            package_id: "12345678-1234-4234-8234-123456789abc".into(),
            brushes: vec![Preset {
                id: "12345678-1234-4234-8234-123456789abd".into(),
                name: "current procedural".into(),
                engine_profile: "drawverse.brush".into(),
                behavior_version: 1,
                required_capabilities: vec!["procedural-tip".into()],
                parameters: archive.info.baseline,
                tip_resource: None,
                source_resource: Some(source.clone()),
            }],
        };
        let bundle = paint_io::vbr_image::ImageBundle::with_sources(
            package,
            std::collections::BTreeMap::from([(tip.clone(), png)]),
            vec![batch],
            &token,
        )
        .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let library = PreparedCatalog::new(BrushCatalog::default());
        library.bind_resources(temp.path().into()).unwrap();
        library.import_bundle(&bundle).unwrap();
        assert!(library
            .imported
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .is_empty());
        assert_eq!(
            library
                .imported_sources
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        let loaded = library
            .resource_store()
            .unwrap()
            .load(&tip, &token)
            .unwrap();
        let weak = Arc::downgrade(&loaded.mask);
        drop(loaded);
        assert!(weak.upgrade().is_none());
        let mut catalog = BrushCatalog::default();
        let preset = &bundle.package.brushes[0];
        catalog
            .save_copy(preset.id.clone(), preset.name.clone())
            .unwrap();
        catalog
            .edit(&preset.id, preset.parameters.settings())
            .unwrap();
        catalog.set_source(&preset.id, Some(source)).unwrap();
        let seq = library
            .submit(String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap())
            .unwrap();
        library.ticket(seq).unwrap().wait().unwrap();
        drop(library.current().unwrap());
        assert!(library.imported.lock().unwrap().is_none());
        assert!(library.imported_sources.lock().unwrap().is_none());
        assert_eq!(library.current_receipts.lock().unwrap().len(), 1);
    }
    #[test]
    fn abr_handoff_is_adopted_by_mutation_and_sources_remain_owned_by_current() {
        let temp = tempfile::tempdir().unwrap();
        let library = PreparedCatalog::new(BrushCatalog::default());
        library.bind_resources(temp.path().into()).unwrap();
        let token = paint_task::CancellationToken::default();
        let batch = paint_io::abr_import::ImportBatch::decode(
            Arc::new(super::abr_fixture::fixture(6, 2, false)),
            paint_core::MAX_CATALOG_MASK_BYTES,
            &token,
        )
        .unwrap();
        let inputs = batch
            .presets()
            .iter()
            .map(|p| {
                (
                    p.name().to_owned(),
                    p.settings().clone(),
                    p.source_id().to_owned(),
                )
            })
            .collect::<Vec<_>>();
        library.import_abr(batch).unwrap();
        assert_eq!(library.imported.lock().unwrap().as_ref().unwrap().len(), 1);
        assert_eq!(
            library
                .imported_sources
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .len(),
            3
        );
        let weak = Arc::downgrade(&library.imported_sources.lock().unwrap().as_ref().unwrap()[0]);
        let store = library.resource_store().unwrap();
        let mut catalog = BrushCatalog::default();
        for (index, (name, mut settings, source)) in inputs.into_iter().enumerate() {
            if let Some(mask) = &settings.mask {
                let id = paint_io::brush_resource::hash(
                    &paint_io::brush_resource::encode_mask(mask).unwrap(),
                );
                settings.mask = Some(store.load(&id, &token).unwrap().mask);
            }
            let id = format!("00000000-0000-4000-8000-{index:012x}");
            catalog.save_copy(id.clone(), name).unwrap();
            catalog.edit(&id, settings).unwrap();
            catalog.set_source(&id, Some(source)).unwrap();
        }
        let json = String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap();
        let seq = library.submit(json).unwrap();
        library.ticket(seq).unwrap().wait().unwrap();
        library
            .mutate(|current| {
                assert_eq!(current.ids().len(), 7);
                Ok(())
            })
            .unwrap();
        assert!(library.imported.lock().unwrap().is_none());
        assert!(library.imported_sources.lock().unwrap().is_none());
        assert!(weak.upgrade().is_some());
        assert_eq!(library.current_receipts.lock().unwrap().len(), 3);
        drop(library);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn source_receipts_follow_current_and_captured_tickets_then_revalidate_cold() {
        use paint_io::{
            abr_conversion::ConvertedPreset,
            brush_source::SourceArchive,
            ps_descriptor::{Class, Descriptor},
        };
        let directory = tempfile::tempdir().unwrap();
        let library = PreparedCatalog::new(BrushCatalog::default());
        library
            .bind_resources(directory.path().join("assets"))
            .unwrap();
        let store = library.resource_store().unwrap();
        let sources = store.source_store().unwrap();
        assert!(Arc::ptr_eq(&sources, &store.source_store().unwrap()));
        let descriptor = Descriptor {
            class: Class {
                name: String::new(),
                id: b"null".to_vec(),
            },
            entries: Vec::new(),
        };
        let converted = ConvertedPreset {
            name: "source",
            settings: paint_core::BrushSettings::default(),
            source: &descriptor,
            issues: Vec::new(),
        };
        let archive =
            SourceArchive::capture(Arc::new(vec![0, 6, 0, 2, 37]), 0, &converted).unwrap();
        let token = paint_task::CancellationToken::default();
        let id = sources.save(&archive, &token).unwrap();
        let mut catalog = BrushCatalog::default();
        catalog
            .set_source("round-pressure", Some(id.clone()))
            .unwrap();
        let json = String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap();
        let seq = library.submit(json.clone()).unwrap();
        let captured = library.ticket(seq).unwrap();
        let snapshot = captured.wait().unwrap();
        assert_eq!(snapshot.selected().source_resource(), Some(id.as_str()));
        let receipt = captured.source_receipts().unwrap().pop().unwrap();
        let weak = Arc::downgrade(&receipt);
        drop(receipt);
        drop(snapshot);
        drop(library.current().unwrap());
        let record = directory
            .path()
            .join("assets/sources")
            .join(format!("{id}.json"));
        std::fs::write(&record, b"corrupt").unwrap();
        let second = library.submit(json.clone()).unwrap();
        let second_ticket = library.ticket(second).unwrap();
        second_ticket.wait().unwrap();
        assert!(Arc::ptr_eq(
            &captured.source_receipts().unwrap()[0],
            &second_ticket.source_receipts().unwrap()[0]
        ));
        drop(library.current().unwrap());
        assert!(sources.verified_record(&id, &token).is_err());
        let round =
            String::from_utf8(paint_io::brush_state::encode(&BrushCatalog::default()).unwrap())
                .unwrap();
        let third = library.submit(round).unwrap();
        library.ticket(third).unwrap().wait().unwrap();
        drop(library.current().unwrap());
        drop(second_ticket);
        library.retire_unused_images().unwrap();
        assert!(weak.upgrade().is_some()); // captured ticket independently owns proof
        drop(library);
        assert!(weak.upgrade().is_some());
        drop(captured);
        assert!(weak.upgrade().is_none());
        let cold = PreparedCatalog::new(BrushCatalog::default());
        cold.share_resources(store).unwrap();
        let bad = cold.submit(json).unwrap();
        assert!(cold.ticket(bad).unwrap().wait().is_err());
        assert!(cold
            .current()
            .unwrap()
            .selected()
            .source_resource()
            .is_none());
    }
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
