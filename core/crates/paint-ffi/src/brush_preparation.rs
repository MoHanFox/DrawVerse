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
                                p.settings().mask.is_some()
                                    || p.settings().texture.resource().is_some()
                                    || p.settings().dual.secondary().mask().is_some()
                                    || p.source_resource().is_some()
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
            .flat_map(|p| {
                p.tip_resource
                    .as_deref()
                    .into_iter()
                    .chain(p.texture.as_ref().and_then(|t| t.resource_id()))
                    .chain(p.dual_brush.as_ref().and_then(|d| d.resource_id()))
            })
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
                            || p.settings()
                                .texture
                                .resource()
                                .and_then(|t| t.source().resource_id())
                                == Some(resource.id.as_str())
                            || p.settings()
                                .dual
                                .secondary()
                                .mask()
                                .and_then(|m| m.resource_id())
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
    fn native_relative_sizing_preparation_capture_and_package_entry() {
        for enabled in [false, true] {
            let sizing = paint_core::DualSizing::new(
                paint_core::DualSizeMode::Relative,
                8.,
                paint_core::DualWidthRule::FixedWidth,
            )
            .unwrap();
            let mut catalog = BrushCatalog::default();
            let mut settings = catalog.selected().settings().clone();
            settings.dual = paint_core::DualBrush::new(
                enabled,
                paint_core::DualMode::Multiply,
                paint_core::SecondaryTip::default(),
            )
            .with_sizing(sizing);
            catalog.edit("round-pressure", settings).unwrap();
            let directory = tempfile::tempdir().unwrap();
            let library = PreparedCatalog::new(BrushCatalog::default());
            library.bind_resources(directory.path().into()).unwrap();
            let sequence = library
                .submit(
                    String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap(),
                )
                .unwrap();
            let captured = library.ticket(sequence).unwrap().wait().unwrap();
            assert_eq!(captured.selected().settings().dual.sizing(), sizing);
            assert_eq!(
                library
                    .current()
                    .unwrap()
                    .selected()
                    .settings()
                    .dual
                    .sizing(),
                sizing
            );
            library
                .mutate(|c| {
                    let mut s = c.selected().settings().clone();
                    s.dual = Default::default();
                    c.edit("round-pressure", s)?;
                    Ok(())
                })
                .unwrap();
            assert_eq!(captured.selected().settings().dual.sizing(), sizing);
            assert!(library
                .current()
                .unwrap()
                .selected()
                .settings()
                .dual
                .is_default());
            let base = texture_bundle(false, false);
            let mut package = base.package.clone();
            package.brushes[0].dual_brush = Some(
                paint_io::brush_dual::DualParameters::from_core(
                    &catalog.selected().settings().dual,
                )
                .unwrap(),
            );
            package.brushes[0].behavior_version = 12;
            package.brushes[0]
                .required_capabilities
                .extend(["dual-cover".into(), "dual-relative-size".into()]);
            let bundle = paint_io::vbr_image::ImageBundle::new(
                package,
                base.assets()
                    .iter()
                    .map(|(id, a)| (id.clone(), a.bytes().to_vec()))
                    .collect(),
                &paint_task::CancellationToken::default(),
            )
            .unwrap();
            library.import_bundle(&bundle).unwrap();
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("sizing.vbr");
            std::fs::write(&path, b"keep").unwrap();
            let path_bytes = path.to_str().unwrap().as_bytes();
            let json = bundle.package.to_json().unwrap();
            let mut handle = std::ptr::null_mut();
            unsafe {
                assert_eq!(crate::paint_brush_library_create(0, &mut handle), PAINT_OK);
                let resource_path = directory.path().to_str().unwrap().as_bytes();
                assert_eq!(
                    crate::paint_brush_library_bind_resources(
                        handle,
                        resource_path.as_ptr(),
                        resource_path.len() as u64
                    ),
                    PAINT_OK
                );
                assert_eq!(
                    crate::paint_brush_library_package_write(
                        handle,
                        path_bytes.as_ptr(),
                        path_bytes.len() as u64,
                        json.as_ptr(),
                        json.len() as u64
                    ),
                    PAINT_OK
                );
                let reopened =
                    paint_io::vbr_image::load(&path, &paint_task::CancellationToken::default())
                        .unwrap();
                assert_eq!(reopened.definition(0).unwrap().dual.sizing(), sizing);
                assert_eq!(crate::paint_brush_library_destroy(&mut handle), PAINT_OK);
            }
        }
    }
    #[test]
    fn native_smoothing_preparation_capture_and_package_entry() {
        for enabled in [false, true] {
            let smoothing = paint_core::BrushSmoothing::new(enabled, 24., 12., 0.7, 3.).unwrap();
            let mut catalog = BrushCatalog::default();
            let mut settings = catalog.selected().settings().clone();
            settings.smoothing = smoothing;
            catalog.edit("round-pressure", settings).unwrap();
            let library = PreparedCatalog::new(BrushCatalog::default());
            let sequence = library
                .submit(
                    String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap(),
                )
                .unwrap();
            let captured = library.ticket(sequence).unwrap().wait().unwrap();
            assert_eq!(captured.selected().settings().smoothing, smoothing);
            assert_eq!(
                library.current().unwrap().selected().settings().smoothing,
                smoothing
            );
            library
                .mutate(|c| {
                    let mut s = c.selected().settings().clone();
                    s.smoothing = Default::default();
                    c.edit("round-pressure", s)?;
                    Ok(())
                })
                .unwrap();
            assert_eq!(captured.selected().settings().smoothing, smoothing);
            assert!(library
                .current()
                .unwrap()
                .selected()
                .settings()
                .smoothing
                .is_default());
            let mut package = texture_bundle(false, false).package;
            package.brushes[0].texture = None;
            package.brushes[0]
                .required_capabilities
                .retain(|c| c != "texture-cover");
            package.brushes[0].smoothing = Some(
                paint_io::brush_smoothing::SmoothingParameters::from_core(smoothing),
            );
            package.brushes[0].behavior_version = 11;
            package.brushes[0]
                .required_capabilities
                .push("path-smoothing".into());
            let bundle = paint_io::vbr_image::ImageBundle::new(
                package,
                std::collections::BTreeMap::new(),
                &paint_task::CancellationToken::default(),
            )
            .unwrap();
            library.import_bundle(&bundle).unwrap();
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("smoothing.vbr");
            let path_bytes = path.to_str().unwrap().as_bytes();
            let json = bundle.package.to_json().unwrap();
            let mut handle = std::ptr::null_mut();
            unsafe {
                assert_eq!(crate::paint_brush_library_create(0, &mut handle), PAINT_OK);
                assert_eq!(
                    crate::paint_brush_library_package_write(
                        handle,
                        path_bytes.as_ptr(),
                        path_bytes.len() as u64,
                        json.as_ptr(),
                        json.len() as u64
                    ),
                    PAINT_OK
                );
                let before = std::fs::read(&path).unwrap();
                assert_eq!(
                    crate::paint_brush_package_write(
                        path_bytes.as_ptr(),
                        path_bytes.len() as u64,
                        json.as_ptr(),
                        json.len() as u64
                    ),
                    PAINT_UNSUPPORTED
                );
                assert_eq!(std::fs::read(&path).unwrap(), before);
                assert_eq!(crate::paint_brush_library_destroy(&mut handle), PAINT_OK);
            }
            let reopened =
                paint_io::vbr_image::load(&path, &paint_task::CancellationToken::default())
                    .unwrap();
            assert_eq!(reopened.definition(0).unwrap().smoothing, smoothing);
        }
    }
    fn texture_bundle(enabled: bool, shared_tip: bool) -> paint_io::vbr_image::ImageBundle {
        let png = paint_io::brush_resource::encode_mask(
            &paint_core::BrushMask::new(4, 1, vec![0, 255, 0, 255]).unwrap(),
        )
        .unwrap();
        let id = paint_io::brush_resource::hash(&png);
        let mask = paint_core::BrushMask::new(4, 1, vec![0, 255, 0, 255])
            .unwrap()
            .with_resource_id(id.clone())
            .unwrap();
        let texture = paint_core::BrushTexture::new(
            enabled,
            Default::default(),
            Some(Arc::new(paint_core::TextureResource::new(Arc::new(mask)))),
        )
        .unwrap();
        let package = paint_io::vbr::Package {
            package_id: "12345678-1234-4234-8234-123456789abd".into(),
            brushes: vec![paint_io::vbr::Preset {
                id: "12345678-1234-4234-8234-123456789abc".into(),
                name: "texture".into(),
                engine_profile: "drawverse.brush".into(),
                behavior_version: 8,
                required_capabilities: vec![
                    if shared_tip {
                        "image-tip"
                    } else {
                        "procedural-tip"
                    }
                    .into(),
                    "texture-cover".into(),
                ],
                parameters: paint_io::vbr::Parameters {
                    diameter: 16.,
                    spacing: 0.02,
                    dab_opacity: 0.4,
                    hardness: 1.,
                    roundness: 1.,
                    angle: 0.,
                    flow: 1.,
                    pressure_size: false,
                    pressure_flow: false,
                    pressure_dynamics: None,
                    tip_dynamics: None,
                    scatter_dynamics: None,
                    shape_jitter: None,
                    color_dynamics: None,
                },
                smoothing: None,
                dual_brush: None,
                texture: Some(
                    paint_io::brush_texture::TextureParameters::from_core(&texture).unwrap(),
                ),
                tip_resource: shared_tip.then(|| id.clone()),
                source_resource: None,
            }],
        };
        paint_io::vbr_image::ImageBundle::new(
            package,
            std::collections::BTreeMap::from([(id, png)]),
            &paint_task::CancellationToken::default(),
        )
        .unwrap()
    }
    fn texture_pixels(definition: paint_core::BrushDefinition) -> Vec<paint_core::Pixel> {
        let mut doc = paint_core::Document::new(40, 32).unwrap();
        doc.begin_brush_stroke(definition, paint_core::InputPoint::new(12.5, 16.5, 1.))
            .unwrap();
        doc.stroke_to(paint_core::InputPoint::new(24.5, 16.5, 1.))
            .unwrap();
        doc.end_stroke().unwrap();
        let pixels = (0..32)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .map(|(x, y)| doc.pixel(x, y))
            .collect::<Vec<_>>();
        doc.undo().unwrap();
        assert_eq!(doc.pixel(16, 16).alpha(), 0.);
        doc.redo().unwrap();
        assert_eq!(doc.pixel(16, 16), pixels[16 * 40 + 16]);
        pixels
    }
    fn dual_bundle(enabled: bool, path: bool, roles: u32) -> paint_io::vbr_image::ImageBundle {
        let mut bundle = texture_bundle(false, roles & 1 != 0);
        let definition = bundle.definition(0).unwrap();
        let mask = definition.texture.resource().unwrap().source().clone();
        let mut secondary =
            paint_core::SecondaryTip::new(4., definition.tip, Some(mask), Default::default())
                .unwrap();
        if path {
            secondary = secondary.with_spacing(0.25).unwrap();
        }
        let dual = paint_core::DualBrush::new(enabled, paint_core::DualMode::Multiply, secondary);
        let preset = &mut bundle.package.brushes[0];
        preset.dual_brush = Some(paint_io::brush_dual::DualParameters::from_core(&dual).unwrap());
        preset.behavior_version = 9;
        preset.required_capabilities.push("dual-cover".into());
        if roles & 2 == 0 {
            preset.texture = None;
            preset
                .required_capabilities
                .retain(|cap| cap != "texture-cover");
        }
        bundle.validate().unwrap();
        bundle
    }
    #[test]
    fn secondary_handoff_covers_both_models_disabled_and_shared_roles_and_mutation() {
        for enabled in [false, true] {
            for path in [false, true] {
                for roles in 0..4 {
                    let directory = tempfile::tempdir().unwrap();
                    let library = PreparedCatalog::new(BrushCatalog::default());
                    library.bind_resources(directory.path().into()).unwrap();
                    let bundle = dual_bundle(enabled, path, roles);
                    library.import_bundle(&bundle).unwrap();
                    assert_eq!(library.imported.lock().unwrap().as_ref().unwrap().len(), 1);
                    library.adopt_imports(&BrushCatalog::default()).unwrap();
                    assert!(library.imported.lock().unwrap().is_some());
                    let definition = bundle.definition(0).unwrap();
                    let mut settings = bundle.package.brushes[0].parameters.settings();
                    settings.mask = definition.mask;
                    settings.texture = definition.texture;
                    settings.dual = definition.dual;
                    let mut catalog = BrushCatalog::default();
                    catalog.edit("round-pressure", settings).unwrap();
                    let json = String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap())
                        .unwrap();
                    let sequence = library.submit(json.clone()).unwrap();
                    let prepared = library.ticket(sequence).unwrap().wait().unwrap();
                    if roles & 1 == 0 {
                        library.mutate(|_| Ok(())).unwrap();
                    } else {
                        drop(library.current().unwrap());
                    }
                    assert!(library.imported.lock().unwrap().is_none());
                    let selected = prepared.selected();
                    assert_eq!(selected.settings().dual.enabled(), enabled);
                    assert_eq!(
                        selected.settings().dual.secondary().spacing().is_some(),
                        path
                    );
                    let secondary = selected.settings().dual.secondary().mask().unwrap();
                    if roles & 1 != 0 {
                        assert!(Arc::ptr_eq(
                            secondary,
                            selected.settings().mask.as_ref().unwrap()
                        ));
                    }
                    if roles & 2 != 0 {
                        assert!(Arc::ptr_eq(
                            secondary,
                            selected.settings().texture.resource().unwrap().source()
                        ));
                    }
                    assert_eq!(
                        paint_io::brush_state::encode(&library.current().unwrap()).unwrap(),
                        json.as_bytes()
                    );
                }
            }
        }
    }
    #[test]
    fn secondary_only_revisions_release_idle_masks_preserve_capture_and_rebuild_pixels() {
        for enabled in [false, true] {
            for path in [false, true] {
                let directory = tempfile::tempdir().unwrap();
                let library = PreparedCatalog::new(BrushCatalog::default());
                library.bind_resources(directory.path().into()).unwrap();
                let bundle = dual_bundle(enabled, path, 0);
                library.import_bundle(&bundle).unwrap();
                let definition = bundle.definition(0).unwrap();
                let mut settings = bundle.package.brushes[0].parameters.settings();
                settings.dual = definition.dual.clone();
                let expected = texture_pixels(definition.clone());
                assert!(expected.iter().any(|p| p.alpha() > 0.));
                let mut catalog = BrushCatalog::default();
                catalog.edit("round-pressure", settings).unwrap();
                let json =
                    String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap();
                let first = library.submit(json.clone()).unwrap();
                let captured = library.ticket(first).unwrap().wait().unwrap();
                drop(library.current().unwrap());
                let weak = Arc::downgrade(
                    captured
                        .selected()
                        .settings()
                        .dual
                        .secondary()
                        .mask()
                        .unwrap(),
                );
                let identity = captured
                    .selected()
                    .settings()
                    .dual
                    .secondary()
                    .mask()
                    .unwrap()
                    .resource_id()
                    .unwrap()
                    .to_owned();
                drop(catalog);
                drop(definition);
                drop(bundle);
                let bad = library
                    .submit(json.replace(&identity, &"0".repeat(64)))
                    .unwrap();
                assert!(library.ticket(bad).unwrap().wait().is_err());
                assert_eq!(
                    paint_io::brush_state::encode(&library.current().unwrap()).unwrap(),
                    json.as_bytes()
                );
                let plain = String::from_utf8(
                    paint_io::brush_state::encode(&BrushCatalog::default()).unwrap(),
                )
                .unwrap();
                let next = library.submit(plain).unwrap();
                library.ticket(next).unwrap().wait().unwrap();
                drop(library.current().unwrap());
                library.retire_unused_images().unwrap();
                assert!(weak.upgrade().is_some());
                assert_eq!(
                    texture_pixels(captured.selected().settings().definition()),
                    expected
                );
                drop(captured);
                library.retire_unused_images().unwrap();
                assert!(weak.upgrade().is_none());
                let rebuilt = library.ticket(first).unwrap().wait().unwrap();
                assert_eq!(
                    paint_io::brush_state::encode(&rebuilt).unwrap(),
                    json.as_bytes()
                );
                assert_eq!(
                    texture_pixels(rebuilt.selected().settings().definition()),
                    expected
                );
            }
        }
    }
    #[test]
    fn texture_import_handoff_adopts_only_referenced_resources_including_disabled() {
        for enabled in [false, true] {
            for shared_tip in [false, true] {
                let directory = tempfile::tempdir().unwrap();
                let library = PreparedCatalog::new(BrushCatalog::default());
                library.bind_resources(directory.path().into()).unwrap();
                let bundle = texture_bundle(enabled, shared_tip);
                library.import_bundle(&bundle).unwrap();
                assert_eq!(library.imported.lock().unwrap().as_ref().unwrap().len(), 1);
                library.adopt_imports(&BrushCatalog::default()).unwrap();
                assert!(library.imported.lock().unwrap().is_some());
                let definition = bundle.definition(0).unwrap();
                let mut catalog = BrushCatalog::default();
                let mut settings = catalog.selected().settings().clone();
                settings.texture = definition.texture;
                settings.mask = definition.mask;
                catalog.edit("round-pressure", settings).unwrap();
                let json =
                    String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap();
                let sequence = library.submit(json).unwrap();
                let prepared = library.ticket(sequence).unwrap().wait().unwrap();
                drop(library.current().unwrap());
                assert!(library.imported.lock().unwrap().is_none());
                assert_eq!(prepared.selected().settings().texture.enabled(), enabled);
                if shared_tip {
                    assert!(Arc::ptr_eq(
                        prepared.selected().settings().mask.as_ref().unwrap(),
                        prepared
                            .selected()
                            .settings()
                            .texture
                            .resource()
                            .unwrap()
                            .source()
                    ));
                }
            }
        }
    }
    #[test]
    fn texture_only_revisions_keep_captured_pixels_release_idle_mips_and_rebuild_exactly() {
        let directory = tempfile::tempdir().unwrap();
        let library = PreparedCatalog::new(BrushCatalog::default());
        library.bind_resources(directory.path().into()).unwrap();
        let bundle = texture_bundle(true, false);
        library.import_bundle(&bundle).unwrap();
        let mut catalog = BrushCatalog::default();
        let definition = bundle.definition(0).unwrap();
        let mut settings = bundle.package.brushes[0].parameters.settings();
        settings.texture = definition.texture.clone();
        catalog.edit("round-pressure", settings).unwrap();
        let expected = texture_pixels(definition.clone());
        assert!(expected.iter().any(|p| p.alpha() > 0.));
        let json = String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap();
        let first = library.submit(json.clone()).unwrap();
        let captured = library.ticket(first).unwrap().wait().unwrap();
        drop(library.current().unwrap());
        let weak_texture =
            Arc::downgrade(captured.selected().settings().texture.resource().unwrap());
        let weak_source = Arc::downgrade(
            captured
                .selected()
                .settings()
                .texture
                .resource()
                .unwrap()
                .source(),
        );
        drop(catalog);
        drop(definition);
        drop(bundle);
        let resource_id = captured
            .selected()
            .settings()
            .texture
            .resource()
            .unwrap()
            .source()
            .resource_id()
            .unwrap()
            .to_owned();
        let bad = library
            .submit(json.replace(&resource_id, &"0".repeat(64)))
            .unwrap();
        assert!(library.ticket(bad).unwrap().wait().is_err());
        assert_eq!(
            paint_io::brush_state::encode(&library.current().unwrap()).unwrap(),
            json.as_bytes()
        );
        let round =
            String::from_utf8(paint_io::brush_state::encode(&BrushCatalog::default()).unwrap())
                .unwrap();
        let second = library.submit(round).unwrap();
        library.ticket(second).unwrap().wait().unwrap();
        drop(library.current().unwrap());
        library.retire_unused_images().unwrap();
        assert!(weak_texture.upgrade().is_some());
        assert_eq!(
            texture_pixels(captured.selected().settings().definition()),
            expected
        );
        drop(captured);
        library.retire_unused_images().unwrap();
        assert!(weak_texture.upgrade().is_none());
        assert!(weak_source.upgrade().is_none());
        let rebuilt = library.ticket(first).unwrap().wait().unwrap();
        assert_eq!(
            paint_io::brush_state::encode(&rebuilt).unwrap(),
            json.as_bytes()
        );
        assert_eq!(
            texture_pixels(rebuilt.selected().settings().definition()),
            expected
        );
    }
    #[test]
    fn pressure_state_preparation_compiles_real_pixels_and_bad_draft_keeps_current() {
        let library = PreparedCatalog::new(BrushCatalog::default());
        let mut catalog = BrushCatalog::default();
        let mut settings = catalog.selected().settings().clone();
        settings.tip.pressure_size = false;
        settings.dynamics.flow = paint_core::PressureControl::new(
            paint_core::PressureCurve::new(vec![(0., 0.3), (1., 0.3)]).unwrap(),
            0.,
        )
        .unwrap();
        catalog.edit("round-pressure", settings).unwrap();
        let json = String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap();
        let seq = library.submit(json.clone()).unwrap();
        let ticket = library.ticket(seq).unwrap();
        let captured = ticket.wait().unwrap();
        let mut document = paint_core::Document::new(32, 32).unwrap();
        document
            .begin_brush_stroke(
                captured.selected().settings().definition(),
                paint_core::InputPoint::new(16.5, 16.5, 0.8),
            )
            .unwrap();
        document.end_stroke().unwrap();
        assert_eq!(document.pixel(16, 16).alpha(), 0.3);
        assert_eq!(
            paint_io::brush_state::encode(&library.current().unwrap()).unwrap(),
            json.as_bytes()
        );
        let bad = library
            .submit(json.replace("\"minimum\":0.0", "\"minimum\":-1.0"))
            .unwrap();
        assert!(library.ticket(bad).unwrap().wait().is_err());
        assert_eq!(
            paint_io::brush_state::encode(&library.current().unwrap()).unwrap(),
            json.as_bytes()
        );
        assert_eq!(
            paint_io::brush_state::encode(&ticket.wait().unwrap()).unwrap(),
            json.as_bytes()
        );
    }
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
                texture: None,
                smoothing: None,
                dual_brush: None,
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
