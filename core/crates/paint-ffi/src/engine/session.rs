//! Serial document ownership and immutable publication. No caller pointers are retained.
#[path = "session_input_paging.rs"]
mod input_paging;
use crate::{
    api::*,
    error::{ApiError, ApiResult},
};
use paint_core::{Document, InputPoint, LayerProperties};
use paint_render::{
    CachedCpuRenderer, FramePixels, PixelFormat, Region, RenderError, RenderFrame, RenderRequest,
    Renderer,
};
use paint_task::{
    Priority, SendError, SerialProgress, SerialSender, SerialWorker, TaskError, TaskPool,
    TaskTicket, WorkerState,
};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(crate) enum Operation {
    Compute(String),
    File(crate::file_job::Request),
    #[cfg(test)]
    PanicForTest,
    New(u32, u32),
    NewWhite(u32, u32),
    Mask(u64),
    Clipping(u64, bool),
    MaskLink(u64, bool),
    Selection(crate::selection_api::Edit),
    Bucket(paint_core::BucketFill),
    Wand(paint_core::WandSelection),
    Drop(u64, u64, u32),
    DropBatch(Vec<u64>, u64, u32),
    RemoveBatch(Vec<u64>, bool),
    Transform(u32, Option<paint_core::TransformGeometry>),
    TransformLayers(Vec<u64>, bool),
    Liquify(u32, Option<paint_core::LiquifyDab>),
    LayerMove(u32, u64, i32, i32),
    LayerMoves(Vec<u64>, bool),
    CopyLayers(Vec<u64>),
    PasteLayers(Vec<u8>, u64),
    DuplicateLayers(Vec<u64>),
    MergeLayers(Vec<u64>),
    Begin(Box<paint_core::BrushDefinition>, InputPoint, u64),
    BindWetTool(Arc<paint_core::WetTool>),
    BeginPrepared(
        Arc<crate::brush_preparation::CatalogTicket>,
        String,
        PaintBrushToolDesc,
        paint_core::Pixel,
        paint_core::BlendMode,
        InputPoint,
    ),
    Move(InputPoint),
    AirbrushTick(InputPoint),
    End,
    Cancel(bool),
    Undo,
    Redo,
    Add(String),
    Remove(u64),
    Select(u64),
    PickLayer(u32, u32, i32, i32),
    Properties(u64, LayerProperties),
    Appearance(u64, paint_core::LayerAppearance),
    Translate(u64, i32, i32),
    Group(u64, String),
    RenameLayer(u64, String),
    GroupBatch(Vec<u64>, String),
    Ungroup(u64),
    Reparent(u64, u64),
    HistoryLimit(usize),
    HistorySelect {
        generation: u64,
        revision: u64,
        scope: u32,
        depth: usize,
    },
}
#[derive(Clone)]
struct Command {
    sequence: u64,
    operation: Operation,
}
pub(crate) struct LayerState {
    pub image: bool,
    pub mask_linked: bool,
    pub info: PaintLayerInfo,
    pub name: String,
    pub appearance: paint_core::LayerAppearance,
    pub hierarchy: PaintLayerHierarchy,
    pub clipping: PaintLayerClipping,
}
pub(crate) struct Publication {
    pub committed_strokes: Arc<VecDeque<u64>>,
    pub resident_preview: Option<Arc<paint_core::ResidentPreview>>,
    pub compute: String,
    pub transform: Option<paint_core::TransformGeometry>,
    pub history: Vec<PaintHistoryEntry>,
    pub selection: paint_core::Selection,
    pub info: PaintSessionInfo,
    pub layers: Vec<LayerState>,
    pub error: String,
}
struct ViewSlot {
    preview_layer: u64,
    blend_preview: Option<(u64, paint_core::BlendMode)>,
    request_id: u64,
    generation: u64,
    request: Option<RenderRequest>,
    frame: Option<Arc<StampedFrame>>,
}
#[derive(Debug)]
pub(crate) struct StampedFrame {
    pub id: u64,
    pub request_id: u64,
    pub generation: u64,
    pub revision: u64,
    pub frame: Arc<RenderFrame>,
}
struct TaskProgressState {
    sequence: u64,
    generation: u64,
    kind: u32,
    started: Instant,
    finished: Option<Instant>,
    status: PaintStatus,
    progress: Arc<paint_core::ComputationProgress>,
}
struct Shared {
    task_progress: Mutex<Option<TaskProgressState>>,
    preview_selection: Mutex<Option<(u64, Arc<Vec<u8>>)>>,
    // Identity and deformed selection assets, bounded to one active session.
    preview_assets: Mutex<Vec<(u64, Arc<Vec<u8>>)>>,
    publication: Mutex<Arc<Publication>>,
    clipboard: Mutex<Option<(u64, Arc<Vec<u8>>)>>,
    views: Mutex<[ViewSlot; 4]>,
    generation: AtomicU64,
    floor: AtomicU64,
}
struct Ingress {
    sequence: u64,
    aborted: bool,
}
pub(crate) struct Session {
    pub file: Arc<Mutex<crate::file_job::FileState>>,
    file_pool: Arc<TaskPool>,
    shared: Arc<Shared>,
    sender: SerialSender<Command>,
    worker: Mutex<SerialWorker<Command>>,
    pool: Arc<TaskPool>,
    ingress: Mutex<Ingress>,
}
struct RenderJob {
    ticket: TaskTicket<ApiResult<RenderFrame>>,
    request_id: u64,
    generation: u64,
    revision: u64,
}
struct Engine {
    committed_strokes: Arc<VecDeque<u64>>,
    files: crate::file_job::FileService,
    document: Document,
    active_input: Option<input_paging::Active>,
    deferred: VecDeque<Command>,
    next_service: Instant,
    wet_tool: Arc<paint_core::WetTool>,
    wet_lease: Option<paint_core::WetLease>,
    shared: Arc<Shared>,
    pool: Arc<TaskPool>,
    jobs: [Option<RenderJob>; 4],
    renderers: [Arc<CachedCpuRenderer>; 4],
    generation: u64,
    publication: u64,
    completed: u64,
    error: Option<(u64, ApiError)>,
    modified: bool,
    dirty: bool,
    aborted: bool,
    floor: u64,
    frames: u64,
    // (latest document revision, oldest revision with identical rendered pixels).
    equivalent_pixels: (u64, u64),
    failed_views: [Option<(u64, u64, u64)>; 4],
}
impl Session {
    #[cfg(test)]
    pub fn new(width: u32, height: u32) -> ApiResult<Self> {
        Self::with_wet_tool(width, height, paint_core::WetTool::default())
    }
    #[cfg(test)]
    fn with_wet_tool(width: u32, height: u32, wet_tool: paint_core::WetTool) -> ApiResult<Self> {
        Self::with_storage_and_wet_tool(
            width,
            height,
            crate::storage_api::CoreStorage::system(),
            Arc::new(wet_tool),
        )
    }
    pub fn with_storage(
        width: u32,
        height: u32,
        storage: Arc<crate::storage_api::CoreStorage>,
    ) -> ApiResult<Self> {
        Self::with_storage_and_wet_tool(
            width,
            height,
            storage,
            Arc::new(paint_core::WetTool::default()),
        )
    }
    pub(crate) fn with_storage_and_wet_tool(
        width: u32,
        height: u32,
        storage: Arc<crate::storage_api::CoreStorage>,
        wet_tool: Arc<paint_core::WetTool>,
    ) -> ApiResult<Self> {
        let document =
            Document::with_storage(width, height, storage.options, storage.space.clone())?;
        let initial = publish(&document, 1, 1, 0, None, false);
        let shared = Arc::new(Shared {
            task_progress: Mutex::new(None),
            preview_selection: Mutex::new(None),
            preview_assets: Mutex::new(Vec::new()),
            generation: AtomicU64::new(1),
            floor: AtomicU64::new(0),
            publication: Mutex::new(Arc::new(initial)),
            clipboard: Mutex::new(None),
            views: Mutex::new(std::array::from_fn(|_| ViewSlot {
                preview_layer: 0,
                blend_preview: None,
                request_id: 0,
                generation: 1,
                request: None,
                frame: None,
            })),
        });
        let pool = Arc::new(TaskPool::new(2, 8).map_err(task_error)?);
        let file_pool = Arc::new(TaskPool::new(1, 1).map_err(task_error)?);
        let file = Arc::new(Mutex::new(crate::file_job::FileState::default()));
        let mut engine = Engine {
            files: crate::file_job::FileService::new(Arc::clone(&file), Arc::clone(&file_pool)),
            document,
            active_input: None,
            deferred: VecDeque::new(),
            next_service: Instant::now(),
            wet_tool,
            wet_lease: None,
            shared: Arc::clone(&shared),
            pool: Arc::clone(&pool),
            jobs: std::array::from_fn(|_| None),
            renderers: std::array::from_fn(|_| Arc::new(CachedCpuRenderer::default())),
            generation: 1,
            publication: 1,
            completed: 0,
            committed_strokes: Arc::new(VecDeque::new()),
            error: None,
            modified: false,
            dirty: true,
            aborted: false,
            floor: 0,
            frames: 0,
            equivalent_pixels: (0, 0),
            failed_views: std::array::from_fn(|_| None),
        };
        engine.default_view();
        let worker =
            SerialWorker::spawn_resumable(4096, Duration::from_millis(8), move |command, token| {
                if token.is_cancelled() {
                    return SerialProgress::default();
                }
                if let Some(command) = command {
                    engine.receive(command);
                } else {
                    engine.resume_input();
                    if (engine.active_input.is_none() && engine.deferred.is_empty())
                        || Instant::now() >= engine.next_service
                    {
                        engine.tick();
                        engine.next_service = Instant::now() + Duration::from_millis(8);
                    }
                }
                SerialProgress {
                    ready: engine.active_input.is_some() || !engine.deferred.is_empty(),
                    retained: engine.deferred.len() + usize::from(engine.active_input.is_some()),
                }
            })
            .map_err(|error| ApiError::new(PAINT_INTERNAL_ERROR, error.to_string()))?;
        Ok(Self {
            file,
            file_pool,
            shared,
            sender: worker.sender(),
            worker: Mutex::new(worker),
            pool,
            ingress: Mutex::new(Ingress {
                sequence: 0,
                aborted: false,
            }),
        })
    }
    pub fn stop(&self) {
        self.file
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .cancel
            .cancel();
        self.worker
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .shutdown();
        self.pool.shutdown();
        self.file_pool.shutdown();
    }
    pub fn submit_file(&self, request: crate::file_job::Request) -> ApiResult<u64> {
        if request.generation != self.shared.generation.load(Ordering::Acquire) {
            return Err(ApiError::new(PAINT_BUSY, "stale file document generation"));
        }
        let id = self
            .file
            .lock()
            .map_err(|_| ApiError::internal())?
            .reserve(&request)?;
        if let Err(error) = self.submit(Operation::File(request)) {
            self.file
                .lock()
                .map_err(|_| ApiError::internal())?
                .finish(Some(error.clone()), 0, 0);
            return Err(error);
        }
        Ok(id)
    }
    pub fn submit(&self, operation: Operation) -> ApiResult<u64> {
        let mut ingress = self.ingress.lock().map_err(|_| ApiError::internal())?;
        if ingress.aborted
            && matches!(
                operation,
                Operation::Move(_) | Operation::AirbrushTick(_) | Operation::End
            )
        {
            return Err(ApiError::new(PAINT_CANCELLED, "stroke already rolled back"));
        }
        let sequence = ingress
            .sequence
            .checked_add(1)
            .ok_or_else(ApiError::internal)?;
        let command = Command {
            sequence,
            operation: operation.clone(),
        };
        let result = if matches!(operation, Operation::Cancel(_)) {
            self.sender.send_cancel(command, |c| {
                matches!(c.operation, Operation::Move(_) | Operation::AirbrushTick(_))
            })
        } else if matches!(operation, Operation::Liquify(PAINT_LIQUIFY_STROKE_END, _)) {
            // Reserve closure without discarding any deformation samples.
            self.sender.send_cancel(command, |_| false)
        } else if matches!(operation, Operation::LayerMove(PAINT_LAYER_MOVE_UPDATE, ..)) {
            self.sender.send_cancel(command, |c| {
                matches!(
                    c.operation,
                    Operation::LayerMove(PAINT_LAYER_MOVE_UPDATE, ..)
                )
            })
        } else if matches!(
            operation,
            Operation::LayerMove(PAINT_LAYER_MOVE_COMMIT | PAINT_LAYER_MOVE_CANCEL, ..)
        ) {
            self.sender.send_cancel(command, |_| false)
        } else if matches!(operation, Operation::Transform(PAINT_TRANSFORM_UPDATE, _)) {
            // Absolute previews supersede only an adjacent preview tail, never a checkpoint.
            self.sender.send_cancel(command, |c| {
                matches!(c.operation, Operation::Transform(PAINT_TRANSFORM_UPDATE, _))
            })
        } else if matches!(
            operation,
            Operation::Transform(
                PAINT_TRANSFORM_CHECKPOINT | PAINT_TRANSFORM_CANCEL | PAINT_TRANSFORM_COMMIT,
                _
            )
        ) {
            self.sender.send_cancel(command, |_| false)
        } else {
            self.sender.try_send(command)
        };
        match result {
            Ok(()) => {
                ingress.sequence = sequence;
                if matches!(
                    operation,
                    Operation::Begin(..) | Operation::BeginPrepared(..)
                ) {
                    ingress.aborted = false;
                }
                if matches!(operation, Operation::Cancel(_)) {
                    ingress.aborted = true;
                }
                Ok(sequence)
            }
            Err(SendError::Full)
                if matches!(
                    operation,
                    Operation::Move(_) | Operation::AirbrushTick(_) | Operation::End
                ) =>
            {
                self.sender
                    .send_cancel(
                        Command {
                            sequence,
                            operation: Operation::Cancel(true),
                        },
                        |c| matches!(c.operation, Operation::Move(_) | Operation::AirbrushTick(_)),
                    )
                    .map_err(send_error)?;
                ingress.sequence = sequence;
                ingress.aborted = true;
                Err(ApiError::new(
                    PAINT_LIMIT_EXCEEDED,
                    "input queue full; entire stroke cancelled",
                ))
            }
            Err(error) => Err(send_error(error)),
        }
    }
    pub fn layer_clipboard(&self, sequence: u64) -> ApiResult<Arc<Vec<u8>>> {
        let result = self
            .shared
            .clipboard
            .lock()
            .map_err(|_| ApiError::invalid("clipboard poisoned"))?;
        match result.as_ref() {
            Some((id, bytes)) if *id == sequence => Ok(bytes.clone()),
            _ => Err(ApiError::new(PAINT_NOT_FOUND, "copy result not available")),
        }
    }
    pub fn publication(&self) -> Arc<Publication> {
        Arc::clone(
            &self
                .shared
                .publication
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        )
    }
    pub fn resident_assets(
        &self,
        preview: &paint_core::ResidentPreview,
    ) -> ApiResult<Arc<Vec<u8>>> {
        if let Some((_, bytes)) = self
            .shared
            .preview_assets
            .lock()
            .map_err(|_| ApiError::internal())?
            .iter()
            .find(|(id, _)| *id == preview.source.id)
        {
            return Ok(bytes.clone());
        }
        let bytes = Arc::new(crate::liquify_api::preview_assets(&preview.source)?);
        let current = self.publication();
        if current
            .resident_preview
            .as_ref()
            .is_none_or(|p| p.source.id != preview.source.id)
        {
            return Err(ApiError::new(PAINT_BUSY, "liquify source replaced"));
        }
        let mut assets = self
            .shared
            .preview_assets
            .lock()
            .map_err(|_| ApiError::internal())?;
        if assets.len() == 2 {
            assets.remove(0);
        }
        assets.push((preview.source.id, bytes.clone()));
        Ok(bytes)
    }
    pub fn resident_alternate_assets(&self, current_source: u64) -> ApiResult<(u64, Arc<Vec<u8>>)> {
        self.shared
            .preview_assets
            .lock()
            .map_err(|_| ApiError::internal())?
            .iter()
            .find(|(id, _)| *id != current_source)
            .cloned()
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "no alternate resident source"))
    }
    pub fn resident_selection(
        &self,
        preview: &paint_core::ResidentPreview,
    ) -> ApiResult<Arc<Vec<u8>>> {
        if let Some((id, bytes)) = self
            .shared
            .preview_selection
            .lock()
            .map_err(|_| ApiError::internal())?
            .as_ref()
        {
            if *id == preview.source.id {
                return Ok(bytes.clone());
            }
        }
        let bytes = Arc::new(crate::liquify_api::preview_selection(&preview.source)?);
        if self
            .publication()
            .resident_preview
            .as_ref()
            .is_none_or(|p| p.source.id != preview.source.id)
        {
            return Err(ApiError::new(PAINT_BUSY, "resident selection replaced"));
        }
        *self
            .shared
            .preview_selection
            .lock()
            .map_err(|_| ApiError::internal())? = Some((preview.source.id, bytes.clone()));
        Ok(bytes)
    }
    pub fn task_progress(&self) -> PaintTaskProgress {
        let task = self
            .shared
            .task_progress
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut result = PaintTaskProgress {
            struct_size: std::mem::size_of::<PaintTaskProgress>() as u32,
            ..Default::default()
        };
        if let Some(task) = task.as_ref() {
            let progress = task.progress.snapshot();
            result.state = if task.finished.is_none() {
                PAINT_TASK_RUNNING
            } else if task.status == PAINT_OK {
                PAINT_TASK_SUCCEEDED
            } else {
                PAINT_TASK_FAILED
            };
            result.kind = task.kind;
            result.phase = progress.phase as u32;
            result.status = task.status;
            result.sequence = task.sequence;
            result.document_generation = task.generation;
            result.stage = progress.stage;
            result.completed = progress.completed;
            result.total = progress.total;
            result.elapsed_ms = task
                .finished
                .unwrap_or_else(Instant::now)
                .duration_since(task.started)
                .as_millis()
                .min(u64::MAX as u128) as u64;
            if result.state == PAINT_TASK_RUNNING && self.sender.state() == WorkerState::Panicked {
                result.state = PAINT_TASK_FAILED;
                result.status = PAINT_INTERNAL_ERROR;
            }
        }
        result
    }
    pub fn info(&self) -> PaintSessionInfo {
        let mut info = self.publication().info;
        info.queue_length = self.sender.pending() as u32;
        let state = self.sender.state();
        if state != WorkerState::Running {
            info.flags &= !PAINT_SESSION_RUNNING;
        }
        if state == WorkerState::Panicked {
            info.flags |= PAINT_SESSION_FAILED;
            info.last_error_status = PAINT_INTERNAL_ERROR;
            info.last_error_sequence = info.completed_sequence;
        }
        info
    }
    pub fn execution_error(&self, sequence: u64) -> ApiResult<String> {
        if self.sender.state() == WorkerState::Panicked {
            if sequence != self.publication().info.completed_sequence {
                return Err(ApiError::new(PAINT_BUSY, "execution error changed"));
            }
            return Ok("document worker panicked; session quarantined".into());
        }
        let publication = self.publication();
        if publication.info.last_error_sequence != sequence {
            return Err(ApiError::new(PAINT_BUSY, "execution error changed"));
        }
        Ok(publication.error.clone())
    }
    pub fn viewport(&self, view: PaintViewport) -> ApiResult<u64> {
        self.viewport_for_layer(view, 0)
    }
    pub fn viewport_for_layer(&self, view: PaintViewport, layer: u64) -> ApiResult<u64> {
        self.viewport_options(view, layer, None)
    }
    pub fn blend_preview(
        &self,
        view: PaintViewport,
        layer: u64,
        blend: paint_core::BlendMode,
    ) -> ApiResult<u64> {
        if view.view_id > 1 || layer == 0 {
            return Err(ApiError::invalid(
                "blend previews require slot 0/1 and an explicit layer",
            ));
        }
        let publication = self.publication();
        if !publication
            .layers
            .iter()
            .any(|l| l.info.layer_id == layer && l.hierarchy.kind != PAINT_LAYER_MASK)
        {
            return Err(ApiError::new(
                PAINT_NOT_FOUND,
                "blend preview layer not found",
            ));
        }
        self.viewport_options(view, 0, Some((layer, blend)))
    }
    fn viewport_options(
        &self,
        view: PaintViewport,
        layer: u64,
        blend_preview: Option<(u64, paint_core::BlendMode)>,
    ) -> ApiResult<u64> {
        if layer != 0 && (view.view_id < 2 || view.pixel_width > 96 || view.pixel_height > 96) {
            return Err(ApiError::invalid(
                "previews require slot 2/3 and at most 96px per edge",
            ));
        }
        if self.sender.state() == WorkerState::Panicked {
            return Err(ApiError::internal());
        }
        if view.view_id >= 4 || view.enabled > 1 || view.reserved != 0 {
            return Err(ApiError::invalid("invalid viewport slot/flags"));
        }
        let request = RenderRequest {
            region: Region {
                x: view.x,
                y: view.y,
                width: view.width,
                height: view.height,
            },
            width: view.pixel_width,
            height: view.pixel_height,
            format: PixelFormat::SrgbRgba8Premultiplied,
        };
        if view.enabled == 1 {
            request.validate().map_err(render_error)?;
        }
        let publication = self.publication();
        if layer != 0 && !publication.layers.iter().any(|l| l.info.layer_id == layer) {
            return Err(ApiError::new(PAINT_NOT_FOUND, "preview layer not found"));
        }
        if publication.info.document_generation != view.document_generation {
            return Err(ApiError::new(PAINT_BUSY, "document generation changed"));
        }
        let mut slots = self.shared.views.lock().map_err(|_| ApiError::internal())?;
        if self.shared.generation.load(Ordering::Acquire) != view.document_generation {
            return Err(ApiError::new(PAINT_BUSY, "document generation changed"));
        }
        let slot = &mut slots[view.view_id as usize];
        slot.request_id = slot
            .request_id
            .checked_add(1)
            .ok_or_else(ApiError::internal)?;
        slot.generation = view.document_generation;
        slot.preview_layer = layer;
        slot.blend_preview = blend_preview;
        slot.request = (view.enabled == 1).then_some(request);
        let old = slot.frame.take();
        let id = slot.request_id;
        drop(slots);
        drop(old);
        Ok(id)
    }
    /// One pixel of a rendered frame, for the eyedropper. Read-only: no history, no dirty tiles.
    pub fn sample_pixel(&self, slot: u32, x: i64, y: i64) -> ApiResult<[f32; 4]> {
        self.frame(slot)?.sample(x, y)
    }
    pub fn frame(&self, slot: u32) -> ApiResult<Arc<StampedFrame>> {
        if self.sender.state() == WorkerState::Panicked {
            return Err(ApiError::internal());
        }
        if slot >= 4 {
            return Err(ApiError::invalid("invalid frame slot"));
        }
        let frame = self.shared.views.lock().map_err(|_| ApiError::internal())?[slot as usize]
            .frame
            .clone()
            .ok_or_else(|| ApiError::new(PAINT_BUSY, "frame pending or viewport disabled"))?;
        if frame.generation != self.shared.generation.load(Ordering::Acquire)
            || frame.revision < self.shared.floor.load(Ordering::Acquire)
        {
            return Err(ApiError::new(PAINT_BUSY, "frame predates document change"));
        }
        Ok(frame)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}
fn send_error(error: SendError) -> ApiError {
    match error {
        SendError::Full => ApiError::new(PAINT_LIMIT_EXCEEDED, "input queue full"),
        SendError::Stopped => ApiError::new(PAINT_CANCELLED, "session stopped"),
        SendError::Panicked => ApiError::internal(),
    }
}
fn task_error(error: TaskError) -> ApiError {
    match error {
        TaskError::Cancelled => ApiError::new(PAINT_CANCELLED, "render cancelled"),
        TaskError::QueueFull => ApiError::new(PAINT_LIMIT_EXCEEDED, "render queue full"),
        _ => ApiError::new(PAINT_INTERNAL_ERROR, error.to_string()),
    }
}
fn render_error(error: RenderError) -> ApiError {
    match error {
        RenderError::Cancelled => ApiError::new(PAINT_CANCELLED, "render cancelled"),
        RenderError::ResourceLimit => {
            ApiError::new(PAINT_LIMIT_EXCEEDED, "render work budget exceeded")
        }
        RenderError::InvalidRequest => ApiError::invalid("invalid or oversized render request"),
        RenderError::Storage(message) => ApiError::new(PAINT_IO_ERROR, message),
    }
}
fn publish(
    doc: &Document,
    generation: u64,
    publication: u64,
    completed: u64,
    error: Option<&(u64, ApiError)>,
    modified: bool,
) -> Publication {
    let (width, height) = doc.dimensions();
    let (undo, redo) = doc
        .transform_history_depth()
        .or_else(|| doc.liquify_history_depth())
        .unwrap_or_else(|| doc.history_depth());
    let layers = doc
        .layers()
        .iter()
        .zip(doc.layer_hierarchies())
        .zip(doc.layer_clipping_bases())
        .map(|((layer, (parent, depth, locks)), base)| LayerState {
            mask_linked: layer.mask_linked(),
            image: layer.is_image(),
            clipping: PaintLayerClipping {
                struct_size: std::mem::size_of::<PaintLayerClipping>() as u32,
                enabled: u32::from(layer.is_clipped()),
                base_layer_id: base,
            },
            info: PaintLayerInfo {
                struct_size: std::mem::size_of::<PaintLayerInfo>() as u32,
                visible: u32::from(layer.properties().visible),
                layer_id: layer.id(),
                opacity: layer.properties().opacity,
                name_length: layer.name().len() as u64,
                allocated_tiles: layer.tile_count() as u64,
                ..Default::default()
            },
            name: layer.name().into(),
            appearance: layer.appearance(),
            hierarchy: {
                PaintLayerHierarchy {
                    struct_size: std::mem::size_of::<PaintLayerHierarchy>() as u32,
                    kind: if layer.is_mask() {
                        PAINT_LAYER_MASK
                    } else {
                        u32::from(layer.is_group())
                    },
                    parent_id: parent,
                    depth,
                    effective_locks: locks,
                }
            },
        })
        .collect();
    Publication {
        committed_strokes: Arc::new(VecDeque::new()),
        resident_preview: doc.resident_preview().map(Arc::new),
        compute: doc.compute_backend_name(),
        transform: doc.transform_geometry(),
        selection: doc.selection().clone(),
        info: PaintSessionInfo {
            struct_size: std::mem::size_of::<PaintSessionInfo>() as u32,
            flags: PAINT_SESSION_RUNNING
                | if doc.layer_move_active() {
                    PAINT_SESSION_LAYER_MOVE
                } else {
                    0
                }
                | if doc.transform_active() {
                    PAINT_SESSION_TRANSFORM
                } else {
                    0
                }
                | if modified { PAINT_SESSION_MODIFIED } else { 0 }
                | if doc.liquify_active() {
                    PAINT_SESSION_LIQUIFY
                } else {
                    0
                },
            document_generation: generation,
            publication,
            completed_sequence: completed,
            revision: doc.revision(),
            active_layer_id: doc.active_layer(),
            last_error_sequence: error.map_or(0, |e| e.0),
            width,
            height,
            layer_count: doc.layers().len() as u32,
            undo_depth: undo as u32,
            redo_depth: redo as u32,
            stroke_active: u32::from(doc.stroke_active()),
            last_error_status: error.map_or(PAINT_OK, |e| e.1.status),
            ..Default::default()
        },
        layers,
        history: doc
            .transform_history_actions()
            .or_else(|| doc.liquify_history_actions())
            .unwrap_or_else(|| doc.history_actions())
            .into_iter()
            .enumerate()
            .map(|(depth, action)| PaintHistoryEntry {
                struct_size: std::mem::size_of::<PaintHistoryEntry>() as u32,
                kind: action as u32,
                depth: depth as u32,
                reserved: 0,
            })
            .collect(),
        error: error.map_or_else(String::new, |e| e.1.message.clone()),
    }
}
impl Engine {
    fn prepare_transform_assets(&mut self) -> ApiResult<()> {
        let Some(source) = self.document.transform_preparation_source() else {
            return Ok(());
        };
        if self
            .shared
            .preview_assets
            .lock()
            .map_err(|_| ApiError::internal())?
            .iter()
            .any(|(id, _)| *id == source.id)
        {
            return Ok(());
        }
        // Prepare on the document actor before BEGIN is acknowledged, rather
        // than making the first pointer update pay for full proxy compositing.
        let bytes = match crate::liquify_api::preview_assets(&source) {
            Ok(bytes) => Arc::new(bytes),
            Err(error) => {
                self.document.cancel_transform()?;
                return Err(error);
            }
        };
        let mut assets = self
            .shared
            .preview_assets
            .lock()
            .map_err(|_| ApiError::internal())?;
        if assets.len() == 2 {
            assets.remove(0);
        }
        assets.push((source.id, bytes));
        Ok(())
    }
    fn default_view(&self) {
        let (width, height) = self.document.dimensions();
        let scale = (f64::from(width) / 1024.)
            .max(f64::from(height) / 1024.)
            .max(1.);
        let mut slots = self.shared.views.lock().unwrap_or_else(|e| e.into_inner());
        let mut removed = Vec::new();
        for slot in slots.iter_mut() {
            slot.request_id += 1;
            slot.generation = self.generation;
            slot.request = None;
            slot.preview_layer = 0;
            slot.blend_preview = None;
            removed.push(slot.frame.take());
        }
        slots[0].request = Some(RenderRequest {
            region: Region {
                x: 0.,
                y: 0.,
                width: f64::from(width),
                height: f64::from(height),
            },
            width: (f64::from(width) / scale).ceil() as u32,
            height: (f64::from(height) / scale).ceil() as u32,
            format: PixelFormat::SrgbRgba8Premultiplied,
        });
        drop(slots);
        drop(removed);
    }
    fn pixel_revision(&self) -> u64 {
        if self.equivalent_pixels.0 == self.document.revision() {
            self.equivalent_pixels.1
        } else {
            self.document.revision()
        }
    }
    fn selection_only(&self, operation: &Operation) -> bool {
        if matches!(operation, Operation::Selection(..) | Operation::Wand(..)) {
            return true;
        }
        if self.document.liquify_active() || self.document.transform_active() {
            return false;
        }
        let (undo, _) = self.document.history_depth();
        if let Operation::HistorySelect {
            scope: PAINT_HISTORY_SCOPE_DOCUMENT,
            depth,
            ..
        } = operation
        {
            if *depth == undo {
                return false;
            }
            let actions = self.document.history_actions();
            let lo = undo.min(*depth) + 1;
            let hi = undo.max(*depth);
            return actions.get(lo..=hi).is_some_and(|actions| {
                actions.iter().all(|action| {
                    matches!(
                        action,
                        paint_core::HistoryAction::SelectAll
                            | paint_core::HistoryAction::Selection
                            | paint_core::HistoryAction::Deselect
                            | paint_core::HistoryAction::InvertSelection
                            | paint_core::HistoryAction::EllipseSelection
                            | paint_core::HistoryAction::LassoSelection
                            | paint_core::HistoryAction::MagicSelection
                            | paint_core::HistoryAction::PolygonSelection
                    )
                })
            });
        }
        let index = match operation {
            Operation::Undo if undo > 0 => undo,
            Operation::Redo => undo + 1,
            _ => return false,
        };
        matches!(
            self.document.history_actions().get(index),
            Some(
                paint_core::HistoryAction::SelectAll
                    | paint_core::HistoryAction::Selection
                    | paint_core::HistoryAction::Deselect
                    | paint_core::HistoryAction::InvertSelection
                    | paint_core::HistoryAction::EllipseSelection
                    | paint_core::HistoryAction::LassoSelection
                    | paint_core::HistoryAction::MagicSelection
                    | paint_core::HistoryAction::PolygonSelection
            )
        )
    }
    fn execute(&mut self, command: Command) {
        let old_revision = self.document.revision();
        let pixel_revision = self.pixel_revision();
        let selection_only = self.selection_only(&command.operation);
        let finish = matches!(command.operation, Operation::End);
        let stroke_command = matches!(
            command.operation,
            Operation::Move(_) | Operation::AirbrushTick(_) | Operation::End
        ) || (matches!(
            command.operation,
            Operation::Begin(..) | Operation::BeginPrepared(..)
        ) && !self.document.stroke_active());
        let liquify_control = matches!(command.operation, Operation::Liquify(kind, _) if !matches!(kind,PAINT_LIQUIFY_DAB|PAINT_LIQUIFY_STROKE_BEGIN|PAINT_LIQUIFY_STROKE_END));
        let barrier = matches!(command.operation, Operation::LayerMove(kind, ..) if kind != PAINT_LAYER_MOVE_UPDATE)
            || matches!(
                command.operation,
                Operation::TransformLayers(..) | Operation::LayerMoves(..)
            )
            || matches!(command.operation,Operation::Transform(kind,_) if kind!=PAINT_TRANSFORM_UPDATE && kind!=PAINT_TRANSFORM_CHECKPOINT)
            || matches!(
                command.operation,
                Operation::New(..)
                    | Operation::NewWhite(..)
                    | Operation::Mask(_)
                    | Operation::Clipping(..)
                    | Operation::MaskLink(..)
                    | Operation::Selection(..)
                    | Operation::Bucket(..)
                    | Operation::Wand(..)
                    | Operation::Drop(..)
                    | Operation::DropBatch(..)
                    | Operation::RemoveBatch(..)
                    | Operation::PasteLayers(..)
                    | Operation::DuplicateLayers(..)
                    | Operation::MergeLayers(..)
                    | Operation::Cancel(_)
                    | Operation::Undo
                    | Operation::HistorySelect { .. }
                    | Operation::Redo
                    | Operation::Remove(_)
                    | Operation::Properties(..)
                    | Operation::Appearance(..)
                    | Operation::Translate(..)
                    | Operation::PickLayer(..)
                    | Operation::Group(..)
                    | Operation::RenameLayer(..)
                    | Operation::GroupBatch(..)
                    | Operation::Ungroup(..)
                    | Operation::Reparent(..)
                    | Operation::Add(_)
            )
            || liquify_control;
        let task_kind = match &command.operation {
            Operation::Transform(PAINT_TRANSFORM_COMMIT, _) => PAINT_TASK_TRANSFORM,
            Operation::Liquify(PAINT_LIQUIFY_COMMIT, _) => PAINT_TASK_LIQUIFY,
            Operation::Transform(
                PAINT_TRANSFORM_FLIP_IMAGE_HORIZONTAL | PAINT_TRANSFORM_FLIP_IMAGE_VERTICAL,
                _,
            ) => PAINT_TASK_FLIP,
            _ => 0,
        };
        let progress =
            (task_kind != 0).then(|| Arc::new(paint_core::ComputationProgress::default()));
        let previous_progress = if let Some(progress) = &progress {
            *self
                .shared
                .task_progress
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(TaskProgressState {
                sequence: command.sequence,
                generation: self.generation,
                kind: task_kind,
                started: Instant::now(),
                finished: None,
                status: PAINT_OK,
                progress: Arc::clone(progress),
            });
            self.document
                .set_computation_progress(Some(Arc::clone(progress)))
        } else {
            None
        };
        let result = self.apply(command.operation, command.sequence);
        if progress.is_some() {
            self.document.set_computation_progress(previous_progress);
            if let Some(task) = self
                .shared
                .task_progress
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_mut()
            {
                task.finished = Some(Instant::now());
                task.status = result.as_ref().err().map_or(PAINT_OK, |e| e.status);
            }
        }
        let pending = result.is_ok() && self.document.stroke_input_pending();
        if let Err(error) = result {
            self.command_failed(command.sequence, error, stroke_command);
        }
        if selection_only && old_revision != self.document.revision() {
            self.equivalent_pixels = (self.document.revision(), pixel_revision);
        }
        if barrier
            && !selection_only
            && (old_revision != self.document.revision() || liquify_control)
        {
            self.floor = self.document.revision();
        }
        self.shared.floor.store(self.floor, Ordering::Release);
        if pending {
            self.active_input = Some(input_paging::Active {
                sequence: command.sequence,
                finish,
            });
            self.dirty = true;
            return;
        }
        self.completed = command.sequence;
        self.dirty = true;
    }
    fn apply(&mut self, operation: Operation, sequence: u64) -> ApiResult<()> {
        if self.document.transform_active()
            && !matches!(
                operation,
                Operation::Transform(..)
                    | Operation::TransformLayers(..)
                    | Operation::HistorySelect { .. }
                    | Operation::File(_)
            )
        {
            return Err(ApiError::new(PAINT_BUSY, "transform active"));
        }
        if self.document.liquify_active()
            && !matches!(
                operation,
                Operation::Liquify(..) | Operation::HistorySelect { .. } | Operation::File(_)
            )
        {
            return Err(paint_core::Error::Busy.into());
        }
        if self.document.layer_move_active() && !matches!(operation, Operation::LayerMove(..)) {
            return Err(paint_core::Error::Busy.into());
        }
        match operation {
            Operation::LayerMoves(ids, deferred) => {
                if self.files.running() {
                    return Err(paint_core::Error::Busy.into());
                }
                self.document.begin_layer_moves(&ids, deferred)?;
            }
            Operation::LayerMove(kind, id, x, y) => match kind {
                PAINT_LAYER_MOVE_BEGIN
                | PAINT_LAYER_MOVE_BEGIN_DEFERRED
                | PAINT_LAYER_MOVE_PICK
                | PAINT_LAYER_MOVE_PICK_DEFERRED => {
                    if self.files.running() {
                        return Err(paint_core::Error::Busy.into());
                    }
                    let target = if kind >= PAINT_LAYER_MOVE_PICK {
                        self.document.pick_layer(x as u32, y as u32, 0, 0)?
                    } else {
                        Some(id)
                    };
                    if let Some(target) = target {
                        self.document.begin_layer_move(
                            target,
                            kind == PAINT_LAYER_MOVE_BEGIN_DEFERRED
                                || kind == PAINT_LAYER_MOVE_PICK_DEFERRED,
                        )?;
                    }
                }
                PAINT_LAYER_MOVE_UPDATE => self.document.update_layer_move(x, y)?,
                PAINT_LAYER_MOVE_COMMIT => {
                    self.document.commit_layer_move(x, y)?;
                    self.modified |= x != 0 || y != 0;
                }
                PAINT_LAYER_MOVE_CANCEL => self.document.cancel_layer_move()?,
                _ => return Err(ApiError::invalid("layer move kind")),
            },
            Operation::TransformLayers(ids, deferred) => {
                if self.files.running() {
                    return Err(paint_core::Error::Busy.into());
                }
                if deferred {
                    self.document.begin_transform_layers_deferred(&ids)?;
                    self.prepare_transform_assets()?;
                } else {
                    self.document.begin_transform_layers(&ids)?;
                }
            }
            Operation::Transform(kind, g) => match kind {
                PAINT_TRANSFORM_BEGIN => self.document.begin_transform()?,
                PAINT_TRANSFORM_ENABLE_RESIDENT => {
                    self.document.enable_transform_deferred()?;
                    self.prepare_transform_assets()?;
                }
                PAINT_TRANSFORM_FLIP_IMAGE_HORIZONTAL | PAINT_TRANSFORM_FLIP_IMAGE_VERTICAL => {
                    if self.files.running() {
                        return Err(paint_core::Error::Busy.into());
                    }
                    self.document
                        .flip_image(kind == PAINT_TRANSFORM_FLIP_IMAGE_HORIZONTAL)?;
                    self.modified = true;
                }
                PAINT_TRANSFORM_UPDATE => self
                    .document
                    .update_transform(g.ok_or_else(|| ApiError::invalid("missing geometry"))?)?,
                PAINT_TRANSFORM_CHECKPOINT => {
                    self.document.checkpoint_transform()?;
                }
                PAINT_TRANSFORM_CANCEL => self.document.cancel_transform()?,
                PAINT_TRANSFORM_COMMIT => {
                    self.modified |= self.document.commit_transform()?;
                }
                PAINT_TRANSFORM_UNDO => {
                    self.document.undo_transform()?;
                }
                PAINT_TRANSFORM_REDO => {
                    self.document.redo_transform()?;
                }
                PAINT_TRANSFORM_RASTERIZE => {
                    self.modified |= self
                        .document
                        .rasterize_image(self.document.active_layer())?;
                }
                _ => return Err(ApiError::invalid("transform kind")),
            },
            Operation::Liquify(kind, dab) => match kind {
                PAINT_LIQUIFY_BEGIN | PAINT_LIQUIFY_BEGIN_DEFERRED => {
                    if self.files.running() {
                        return Err(paint_core::Error::Busy.into());
                    }
                    if kind == PAINT_LIQUIFY_BEGIN_DEFERRED {
                        self.document.begin_liquify_deferred()?;
                    } else {
                        self.document.begin_liquify()?;
                    }
                }
                PAINT_LIQUIFY_DAB => {
                    self.document
                        .liquify_dab(dab.ok_or_else(|| ApiError::invalid("missing dab"))?)?;
                }
                PAINT_LIQUIFY_STROKE_BEGIN => self.document.begin_liquify_stroke()?,
                PAINT_LIQUIFY_STROKE_END => {
                    self.document.end_liquify_stroke()?;
                }
                PAINT_LIQUIFY_UNDO => {
                    self.document.undo_liquify()?;
                }
                PAINT_LIQUIFY_REDO => {
                    self.document.redo_liquify()?;
                }
                PAINT_LIQUIFY_RESET => self.document.reset_liquify()?,
                PAINT_LIQUIFY_CANCEL => self.document.cancel_liquify()?,
                PAINT_LIQUIFY_FOCUS_ON | PAINT_LIQUIFY_FOCUS_OFF => self
                    .document
                    .set_liquify_focus(kind == PAINT_LIQUIFY_FOCUS_ON)?,
                PAINT_LIQUIFY_COMMIT => {
                    self.modified |= self.document.commit_liquify()?;
                }
                _ => return Err(ApiError::invalid("liquify action")),
            },
            Operation::Bucket(options) => {
                self.modified |= self.document.bucket_fill(options)?;
            }
            Operation::Wand(options) => {
                let revision = self.document.revision();
                self.document.select_wand(options)?;
                self.modified |= revision != self.document.revision();
            }
            Operation::File(request) => self.files.start(request, &self.document, self.generation),
            #[cfg(test)]
            Operation::PanicForTest => panic!("actor panic injection"),
            Operation::Mask(id) => {
                self.document.add_mask(id)?;
                self.modified = true;
            }
            Operation::MaskLink(id, linked) => {
                let r = self.document.revision();
                self.document.set_mask_linked(id, linked)?;
                self.modified |= r != self.document.revision();
            }
            Operation::Clipping(id, enabled) => {
                let r = self.document.revision();
                self.document.set_layer_clipping(id, enabled)?;
                self.modified |= r != self.document.revision();
            }
            Operation::Selection(edit) => {
                let revision = self.document.revision();
                let (w, h) = self.document.dimensions();
                let action = match &edit {
                    crate::selection_api::Edit::Shape(shape, _) => {
                        if shape.kind == paint_core::SelectionKind::Ellipse {
                            paint_core::HistoryAction::EllipseSelection
                        } else {
                            paint_core::HistoryAction::Selection
                        }
                    }
                    crate::selection_api::Edit::Styled(_, _, action) => *action,
                    crate::selection_api::Edit::All => paint_core::HistoryAction::SelectAll,
                    crate::selection_api::Edit::Clear => paint_core::HistoryAction::Deselect,
                    crate::selection_api::Edit::Invert => {
                        paint_core::HistoryAction::InvertSelection
                    }
                };
                let next = match edit {
                    crate::selection_api::Edit::Shape(shape, op)
                    | crate::selection_api::Edit::Styled(shape, op, _) => {
                        self.document.selection().apply(shape, op, w, h)?
                    }
                    crate::selection_api::Edit::All => paint_core::Selection::all(w, h),
                    crate::selection_api::Edit::Clear => paint_core::Selection::default(),
                    crate::selection_api::Edit::Invert => {
                        self.document.selection().inverted(w, h)?
                    }
                };
                self.document.set_selection_with_action(next, action)?;
                self.modified |= revision != self.document.revision();
            }
            Operation::DropBatch(ids, target, placement) => {
                let r = self.document.revision();
                self.document.drop_layers(&ids, target, placement)?;
                self.modified |= r != self.document.revision();
            }
            Operation::CopyLayers(ids) => {
                let bytes = self.document.copy_layers(&ids)?;
                *self
                    .shared
                    .clipboard
                    .lock()
                    .map_err(|_| ApiError::invalid("clipboard poisoned"))? =
                    Some((sequence, Arc::new(bytes)));
            }
            Operation::PasteLayers(bytes, target) => {
                self.document.paste_layers(&bytes, target)?;
                self.modified = true;
            }
            Operation::DuplicateLayers(ids) => {
                self.document.duplicate_layers(&ids)?;
                self.modified = true;
            }
            Operation::MergeLayers(ids) => {
                self.document.merge_layers(&ids)?;
                self.modified = true;
            }
            Operation::RemoveBatch(ids, keep) => {
                self.document.remove_layers(&ids, keep)?;
                self.modified = true;
            }
            Operation::Drop(id, target, placement) => {
                let r = self.document.revision();
                self.document.drop_layer(id, target, placement)?;
                self.modified |= r != self.document.revision();
            }
            Operation::Compute(id) => {
                if self.document.stroke_active() || self.files.running() {
                    return Err(paint_core::Error::Busy.into());
                }
                let executor = crate::engine::compute_api::executor(&id);
                self.document.set_compute_executor(executor)?;
            }
            Operation::HistoryLimit(max_commands) => {
                self.document.set_max_history_commands(max_commands)?;
            }
            Operation::NewWhite(w, h) => {
                self.apply(Operation::New(w, h), sequence)?;
                self.document.initialize_white_background()?;
            }
            Operation::New(w, h) => {
                let executor = self.document.compute_executor();
                let mut document = Document::with_storage(
                    w,
                    h,
                    self.document.document_options(),
                    self.document.storage(),
                )?;
                document.set_compute_executor(executor)?;
                self.document = document;
                self.wet_lease.take();
                self.generation += 1;
                self.shared
                    .generation
                    .store(self.generation, Ordering::Release);
                self.shared.floor.store(0, Ordering::Release);
                self.modified = false;
                self.aborted = false;
                self.floor = 0;
                self.error = None;
                self.default_view();
            }
            Operation::BeginPrepared(ticket, id, tool, background, blend, point) => {
                self.aborted = false;
                let library = ticket.wait()?;
                let preset = library
                    .preset(&id)
                    .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "prepared brush preset"))?;
                let mut definition = crate::brush_catalog_api::tool_definition(&preset, tool)?;
                definition.brush.background_color = background;
                definition.write_blend = blend;
                self.apply(
                    Operation::Begin(Box::new(definition), point, tool.layer_id),
                    sequence,
                )?;
            }
            Operation::BindWetTool(tool) => {
                if self.document.stroke_active() {
                    return Err(paint_core::Error::Busy.into());
                }
                self.wet_tool = tool;
            }
            Operation::Begin(brush, point, layer) => {
                self.aborted = false;
                if self.document.stroke_active() {
                    return Err(paint_core::Error::Busy.into());
                }
                let lease = if brush.wet.enabled() {
                    Some(self.wet_tool.lease()?)
                } else {
                    None
                };
                let previous = self.document.active_layer();
                if layer != 0 {
                    self.document.set_active_layer(layer)?;
                }
                let result = if let Some(lease) = &lease {
                    if brush.brush.mode == paint_core::BrushMode::Paint
                        && point.tool != paint_core::Tool::Eraser
                        && brush.dual.enabled()
                        && brush.dual.color().enabled()
                    {
                        self.document.begin_configured_dual_wet_brush_stroke(
                            *brush,
                            point,
                            lease.retained_ink(),
                        )
                    } else {
                        self.document.begin_configured_wet_brush_stroke(
                            *brush,
                            point,
                            lease.retained(),
                        )
                    }
                } else {
                    self.document.begin_brush_stroke(*brush, point)
                };
                if let Err(error) = result {
                    // Selection must follow the actor's immediate state, not a stale UI hint.
                    if self.document.layer(previous).is_some() {
                        self.document.set_active_layer(previous)?;
                    }
                    return Err(error.into());
                }
                self.wet_lease = lease;
            }
            Operation::Move(point) => {
                if self.aborted {
                    return Err(ApiError::new(PAINT_CANCELLED, "stroke rolled back"));
                }
                self.document.begin_stroke_input(point)?;
            }
            Operation::AirbrushTick(point) => {
                if self.aborted {
                    return Err(ApiError::new(PAINT_CANCELLED, "stroke rolled back"));
                }
                self.document.airbrush_tick(point)?;
            }
            Operation::End => {
                if self.aborted {
                    return Err(ApiError::new(PAINT_CANCELLED, "stroke rolled back"));
                }
                self.document.begin_stroke_finish()?;
            }
            Operation::Cancel(overload) => {
                let cancelled = if self.document.stroke_active() {
                    self.document.cancel_stroke()
                } else {
                    Ok(())
                };
                self.wet_lease.take();
                cancelled?;
                self.aborted = true;
                if overload {
                    return Err(ApiError::new(
                        PAINT_LIMIT_EXCEEDED,
                        "input queue full; entire stroke cancelled",
                    ));
                }
            }
            Operation::HistorySelect {
                generation,
                revision,
                scope,
                depth,
            } => {
                let actual_scope = if self.document.transform_active() {
                    PAINT_HISTORY_SCOPE_TRANSFORM
                } else if self.document.liquify_active() {
                    PAINT_HISTORY_SCOPE_LIQUIFY
                } else {
                    PAINT_HISTORY_SCOPE_DOCUMENT
                };
                if self.files.running()
                    || generation != self.generation
                    || revision != self.document.revision()
                    || scope != actual_scope
                {
                    return Err(ApiError::new(PAINT_BUSY, "history selection state changed"));
                }
                match scope {
                    PAINT_HISTORY_SCOPE_DOCUMENT => {
                        self.modified |= self.document.select_history(depth)?
                    }
                    PAINT_HISTORY_SCOPE_LIQUIFY => {
                        self.document.select_liquify_history(depth)?;
                    }
                    PAINT_HISTORY_SCOPE_TRANSFORM => {
                        self.document.select_transform_history(depth)?;
                    }
                    _ => return Err(ApiError::invalid("history scope")),
                }
            }
            Operation::Undo => self.modified |= self.document.undo()?,
            Operation::Redo => self.modified |= self.document.redo()?,
            Operation::Add(name) => {
                self.document.add_layer(name)?;
                self.modified = true;
            }
            Operation::Remove(id) => {
                self.document.remove_layer(id)?;
                self.modified = true;
            }
            Operation::Select(id) => self.document.set_active_layer(id)?,
            Operation::PickLayer(x, y, dx, dy) => {
                let revision = self.document.revision();
                self.document.pick_layer(x, y, dx, dy)?;
                self.modified |= revision != self.document.revision();
            }
            Operation::Appearance(id, value) => {
                let revision = self.document.revision();
                self.document.set_layer_appearance(id, value)?;
                self.modified |= revision != self.document.revision();
            }
            Operation::Translate(id, x, y) => {
                let revision = self.document.revision();
                self.document.move_layer(id, x, y)?;
                self.modified |= revision != self.document.revision();
            }
            Operation::Group(id, name) => {
                self.document.group_layer(id, name)?;
                self.modified = true;
            }
            Operation::RenameLayer(id, name) => {
                let revision = self.document.revision();
                self.document.rename_layer(id, name)?;
                self.modified |= revision != self.document.revision();
            }
            Operation::GroupBatch(ids, name) => {
                self.document.group_layers(&ids, name)?;
                self.modified = true;
            }
            Operation::Ungroup(id) => {
                self.document.ungroup_layer(id)?;
                self.modified = true;
            }
            Operation::Reparent(id, parent) => {
                let revision = self.document.revision();
                self.document.reparent_layer(id, parent)?;
                self.modified |= revision != self.document.revision();
            }
            Operation::Properties(id, properties) => {
                let revision = self.document.revision();
                self.document.set_layer_properties(id, properties)?;
                self.modified |= revision != self.document.revision();
            }
        }
        Ok(())
    }
    fn tick(&mut self) {
        if !self.document.stroke_page_ready_for_publication() {
            return;
        }
        if let Some((document, clear_modified)) = self.files.poll(
            self.generation,
            self.document.revision(),
            self.document.stroke_active(),
            Some(&mut self.document),
        ) {
            if let Some(mut document) = document {
                // Hardware belongs to the session, including file replacement.
                if let Err(error) = document.set_compute_executor(self.document.compute_executor())
                {
                    self.error = Some((self.completed, error.into()));
                    self.dirty = true;
                    return;
                }
                self.document = document;
                self.wet_lease.take();
                self.generation += 1;
                self.floor = 0;
                self.aborted = false;
                self.error = None;
                self.shared
                    .generation
                    .store(self.generation, Ordering::Release);
                self.shared.floor.store(0, Ordering::Release);
                self.default_view();
            }
            if clear_modified {
                self.modified = false;
            }
            self.dirty = true;
        }
        if self.dirty {
            self.publication += 1;
            let mut publication = publish(
                &self.document,
                self.generation,
                self.publication,
                self.completed,
                self.error.as_ref(),
                self.modified,
            );
            publication.committed_strokes = Arc::clone(&self.committed_strokes);
            let publication = Arc::new(publication);
            if publication.resident_preview.is_none() {
                self.shared
                    .preview_selection
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take();
                self.shared
                    .preview_assets
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clear();
            }
            let old = std::mem::replace(
                &mut *self
                    .shared
                    .publication
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()),
                publication,
            );
            drop(old);
            self.dirty = false;
        }
        for index in 0..4 {
            let revision = self.document.revision();
            let pixel_revision = self.pixel_revision();
            let mut slots = self.shared.views.lock().unwrap_or_else(|e| e.into_inner());
            let slot = &mut slots[index];
            if let Some(frame) = slot
                .frame
                .as_ref()
                .filter(|frame| frame.revision >= pixel_revision && frame.revision < revision)
            {
                // Keep immutable pixel storage alive across stamped publication changes.
                self.frames += 1;
                slot.frame = Some(Arc::new(StampedFrame {
                    id: self.frames,
                    request_id: frame.request_id,
                    generation: frame.generation,
                    revision,
                    frame: Arc::clone(&frame.frame),
                }));
            }
            let obsolete = self.jobs[index].as_ref().is_some_and(|job| {
                job.request_id != slot.request_id
                    || job.generation != self.generation
                    || job.revision < self.floor
                    || slot.request.is_none()
                    || (slot.preview_layer != 0
                        && (self.document.stroke_active()
                            || job.revision != self.document.revision()))
            });
            if obsolete {
                if let Some(job) = self.jobs[index].take() {
                    self.pool.cancel(&job.ticket);
                }
            }
            if let Some(result) = self.jobs[index]
                .as_ref()
                .and_then(|job| job.ticket.try_take())
            {
                let job = self.jobs[index].take().expect("job exists");
                match result {
                    Ok(Ok(frame)) => {
                        if job.generation == self.generation
                            && slot.request_id == job.request_id
                            && frame.revision >= self.floor
                            && slot.frame.as_ref().is_none_or(|old| {
                                frame.revision >= old.revision || frame.revision >= pixel_revision
                            })
                        {
                            self.frames += 1;
                            let old = slot.frame.replace(Arc::new(StampedFrame {
                                id: self.frames,
                                request_id: job.request_id,
                                generation: job.generation,
                                revision: if frame.revision >= pixel_revision {
                                    revision
                                } else {
                                    frame.revision
                                },
                                frame: Arc::new(frame),
                            }));
                            drop(slots);
                            drop(old);
                            slots = self.shared.views.lock().unwrap_or_else(|e| e.into_inner());
                        }
                    }
                    Err(TaskError::Cancelled) => {}
                    Ok(Err(error)) => {
                        self.failed_views[index] =
                            Some((job.request_id, job.generation, job.revision));
                        self.error = Some((self.completed, error));
                        self.dirty = true;
                    }
                    Err(error) => {
                        self.failed_views[index] =
                            Some((job.request_id, job.generation, job.revision));
                        self.error = Some((self.completed, task_error(error)));
                        self.dirty = true;
                    }
                }
            }
            let slot = &mut slots[index];
            if self.jobs[index].is_some() || slot.generation != self.generation {
                continue;
            }
            let Some(request) = slot.request else {
                continue;
            };
            if index < 2
                && (self.document.liquify_deferred()
                    || self.document.layer_move_deferred()
                    || self.document.transform_deferred())
            {
                // The scene graph renders resident sources using the published mesh.
                // No CPU frame jobs or preview colour readbacks while dragging.
                continue;
            }
            if slot.preview_layer != 0 && self.document.stroke_active() {
                continue;
            }
            if slot
                .frame
                .as_ref()
                .is_some_and(|frame| frame.revision == self.document.revision())
            {
                continue;
            }
            let request_id = slot.request_id;
            let preview_layer = slot.preview_layer;
            let blend_preview = slot.blend_preview;
            if self.failed_views[index]
                == Some((request_id, self.generation, self.document.revision()))
            {
                continue;
            }
            drop(slots);
            let snapshot = if preview_layer == 0 {
                if index < 2 {
                    self.document.liquify_snapshot()
                } else {
                    self.document.snapshot()
                }
            } else {
                match self.document.layer_preview(preview_layer) {
                    Ok(snapshot) => snapshot,
                    Err(_) => continue, // deleted layer; caller disables/replaces this slot
                }
            };
            let snapshot = if let Some((layer, blend)) = blend_preview {
                match snapshot.with_layer_blend(layer, blend) {
                    Ok(snapshot) => snapshot,
                    Err(_) => continue,
                }
            } else {
                snapshot
            };
            let renderer = Arc::clone(&self.renderers[index]);
            let interactive_move =
                index < 2 && preview_layer == 0 && self.document.layer_move_active();
            let revision = snapshot.revision;
            let priority = if index == 0 {
                Priority::Interactive
            } else {
                Priority::Background
            };
            if let Ok(ticket) = self.pool.submit(priority, move |token| {
                let frame = if interactive_move {
                    paint_render::render_interactive(&snapshot, request, &token)
                } else {
                    renderer.render(&snapshot, request, &token)
                };
                match frame {
                    Err(RenderError::Cancelled) => Err(TaskError::Cancelled),
                    result => Ok(result.map_err(render_error)),
                }
            }) {
                self.jobs[index] = Some(RenderJob {
                    ticket,
                    request_id,
                    generation: self.generation,
                    revision,
                });
            }
        }
    }
}
impl StampedFrame {
    /// One pixel of this already rendered frame, un-premultiplied straight RGBA. Used by the
    /// eyedropper so sampling never touches the document or the history.
    pub fn sample(&self, x: i64, y: i64) -> ApiResult<[f32; 4]> {
        let width = self.frame.width;
        let height = self.frame.height;
        if width == 0 || height == 0 {
            return Err(ApiError::new(PAINT_NOT_FOUND, "frame has no pixels"));
        }
        // Clamp to the rendered region so a pointer just outside still samples the edge pixel.
        // Document pixels and frame pixels differ with zoom, LOD and device scale. Map the
        // document pixel centre through the actual fractional region, without integer overflow.
        let region = self.frame.region;
        let px = (((x as f64 + 0.5 - region.x) / region.width * f64::from(width)).floor())
            .clamp(0., f64::from(width - 1)) as u32;
        let py = (((y as f64 + 0.5 - region.y) / region.height * f64::from(height)).floor())
            .clamp(0., f64::from(height - 1)) as u32;
        let bytes = self.bytes();
        let index = ((py as usize) * (width as usize) + (px as usize)) * 4;
        let Some(pixel) = bytes.get(index..index + 4) else {
            return Err(ApiError::new(PAINT_NOT_FOUND, "frame pixel out of range"));
        };
        let alpha = f32::from(pixel[3]) / 255.;
        if alpha <= 0. {
            return Ok([0.; 4]);
        }
        Ok([
            f32::from(pixel[0]) / 255. / alpha,
            f32::from(pixel[1]) / 255. / alpha,
            f32::from(pixel[2]) / 255. / alpha,
            alpha,
        ])
    }
    pub fn bytes(&self) -> &[u8] {
        match &self.frame.pixels {
            FramePixels::Srgb(bytes) => bytes,
            FramePixels::Linear(_) => unreachable!("display request"),
        }
    }
    pub fn info(&self) -> PaintFrameInfo {
        PaintFrameInfo {
            struct_size: std::mem::size_of::<PaintFrameInfo>() as u32,
            format: PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED,
            document_generation: self.generation,
            revision: self.revision,
            request_id: self.request_id,
            frame_id: self.id,
            pixel_width: self.frame.width,
            pixel_height: self.frame.height,
            x: self.frame.region.x,
            y: self.frame.region.y,
            width: self.frame.region.width,
            height: self.frame.region.height,
            required_bytes: self.bytes().len() as u64,
            reserved: 0,
        }
    }
}

#[cfg(test)]
mod actor_tests {
    use super::*;
    use std::{thread, time::Instant};
    #[test]
    fn selection_edits_reuse_pixels_but_keep_stamps_and_next_paint_fresh() {
        let session = Session::new(128, 128).unwrap();
        let run = |operation| {
            let sequence = session.submit(operation).unwrap();
            let start = Instant::now();
            loop {
                let info = session.info();
                assert!(start.elapsed() < Duration::from_secs(5));
                if info.completed_sequence >= sequence {
                    assert_eq!(info.last_error_status, PAINT_OK);
                    if let Ok(frame) = session.frame(0) {
                        if frame.revision == info.revision {
                            break frame;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(1));
            }
        };
        let base = run(Operation::Selection(crate::selection_api::Edit::Clear));
        let floor = session.shared.floor.load(Ordering::Acquire);
        let shape =
            paint_core::SelectionShape::polygon(vec![[0., 0.], [128., 0.], [128., 128.]], true, 0.)
                .unwrap();
        for operation in [
            Operation::Selection(crate::selection_api::Edit::Styled(
                shape,
                paint_core::SelectionOperation::Replace,
                paint_core::HistoryAction::LassoSelection,
            )),
            Operation::Selection(crate::selection_api::Edit::Invert),
            Operation::Undo,
            Operation::Redo,
            Operation::Wand(paint_core::WandSelection {
                x: 0,
                y: 0,
                sample_size: 1,
                tolerance: 10,
                antialias: true,
                contiguous: true,
                all_layers: false,
                operation: paint_core::SelectionOperation::Replace,
            }),
            Operation::Selection(crate::selection_api::Edit::Clear),
        ] {
            let frame = run(operation);
            assert!(Arc::ptr_eq(&base.frame, &frame.frame));
            assert_eq!(frame.info().revision, session.info().revision);
            assert_eq!(session.shared.floor.load(Ordering::Acquire), floor);
        }
        run(Operation::Begin(
            Box::new(paint_core::Brush::default().into()),
            InputPoint::new(40., 40., 1.),
            0,
        ));
        let painted = run(Operation::End);
        assert!(!Arc::ptr_eq(&base.frame, &painted.frame));
        assert_ne!(base.bytes(), painted.bytes());
        let undo = run(Operation::Undo);
        assert!(!Arc::ptr_eq(&painted.frame, &undo.frame));
        assert_eq!(base.bytes(), undo.bytes());
        let recreated = run(Operation::New(64, 64));
        assert_eq!(recreated.frame.width, 64);
        assert!(!Arc::ptr_eq(&base.frame, &recreated.frame));
    }
    #[test]
    fn transform_begin_prepares_first_drag_assets_and_keeps_both_selection_sources() {
        let session = Session::new(128, 128).unwrap();
        let run = |operation| {
            let seq = session.submit(operation).unwrap();
            let deadline = Instant::now() + Duration::from_secs(10);
            while session.info().completed_sequence < seq {
                assert!(Instant::now() < deadline);
                thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(session.info().last_error_status, PAINT_OK);
            session.publication()
        };
        run(Operation::Begin(
            Box::new(paint_core::Brush::default().into()),
            InputPoint::new(40., 40., 1.),
            0,
        ));
        run(Operation::End);
        let shape =
            paint_core::SelectionShape::polygon(vec![[10., 10.], [90., 10.], [90., 90.]], true, 2.)
                .unwrap();
        run(Operation::Selection(crate::selection_api::Edit::Styled(
            shape,
            paint_core::SelectionOperation::Replace,
            paint_core::HistoryAction::LassoSelection,
        )));
        let publication = run(Operation::TransformLayers(
            vec![session.info().active_layer_id],
            true,
        ));
        let identity = publication.resident_preview.as_ref().unwrap();
        let (prepared_id, prepared) = {
            let assets = session.shared.preview_assets.lock().unwrap();
            assert_eq!(assets.len(), 1);
            assets[0].clone()
        };
        assert_ne!(
            prepared_id, identity.source.id,
            "initial feathered image differs from moving source"
        );
        let alternate = session
            .resident_alternate_assets(identity.source.id)
            .unwrap();
        assert_eq!(alternate.0, prepared_id);
        assert!(Arc::ptr_eq(&alternate.1, &prepared));
        let identity_bytes = session.resident_assets(identity).unwrap();
        let mut geometry = publication.transform.clone().unwrap();
        for point in &mut geometry.points {
            point[0] += 4.;
        }
        for _ in 0..3 {
            let moved = run(Operation::Transform(
                PAINT_TRANSFORM_UPDATE,
                Some(geometry.clone()),
            ));
            let preview = moved.resident_preview.as_ref().unwrap();
            assert_eq!(preview.source.id, prepared_id);
            assert!(Arc::ptr_eq(
                &prepared,
                &session.resident_assets(preview).unwrap()
            ));
            let initial = run(Operation::Transform(
                PAINT_TRANSFORM_UPDATE,
                publication.transform.clone(),
            ));
            assert!(Arc::ptr_eq(
                &identity_bytes,
                &session
                    .resident_assets(initial.resident_preview.as_ref().unwrap())
                    .unwrap()
            ));
            assert_eq!(session.shared.preview_assets.lock().unwrap().len(), 2);
        }
        run(Operation::Transform(PAINT_TRANSFORM_CANCEL, None));
        assert!(session.shared.preview_assets.lock().unwrap().is_empty());
    }
    #[test]
    fn resident_preview_keeps_render_floor_but_controls_advance_it() {
        let session = Session::new(128, 128).unwrap();
        let run = |operation| {
            let seq = session.submit(operation).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            while session.info().completed_sequence < seq {
                assert!(Instant::now() < deadline);
                thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(session.info().last_error_status, PAINT_OK);
            session.info()
        };
        run(Operation::Begin(
            Box::new(paint_core::Brush::default().into()),
            paint_core::InputPoint::new(40., 40., 1.),
            0,
        ));
        run(Operation::End);
        run(Operation::Liquify(PAINT_LIQUIFY_BEGIN, None));
        let floor = session.shared.floor.load(Ordering::Acquire);
        let dab = paint_core::LiquifyDab {
            tool: 0,
            radius: 24.,
            strength: 0.5,
            density: 1.,
            speed: 1.,
            pressure_enabled: false,
            from: paint_core::InputPoint::new(40., 40., 1.),
            to: paint_core::InputPoint::new(48., 44., 1.),
            elapsed_ms: 16.,
        };
        let preview = run(Operation::Liquify(PAINT_LIQUIFY_DAB, Some(dab)));
        assert!(preview.revision > floor);
        assert_eq!(session.shared.floor.load(Ordering::Acquire), floor);
        let committed = run(Operation::Liquify(PAINT_LIQUIFY_COMMIT, None));
        assert_eq!(
            session.shared.floor.load(Ordering::Acquire),
            committed.revision
        );
        run(Operation::Liquify(PAINT_LIQUIFY_BEGIN, None));
        run(Operation::Liquify(PAINT_LIQUIFY_DAB, Some(dab)));
        let reset = run(Operation::Liquify(PAINT_LIQUIFY_RESET, None));
        assert_eq!(session.shared.floor.load(Ordering::Acquire), reset.revision);
        run(Operation::Liquify(PAINT_LIQUIFY_DAB, Some(dab)));
        let cancelled = run(Operation::Liquify(PAINT_LIQUIFY_CANCEL, None));
        assert_eq!(
            session.shared.floor.load(Ordering::Acquire),
            cancelled.revision
        );
        run(Operation::LayerMove(PAINT_LAYER_MOVE_BEGIN, 1, 0, 0));
        let floor = session.shared.floor.load(Ordering::Acquire);
        let preview = run(Operation::LayerMove(PAINT_LAYER_MOVE_UPDATE, 0, 10, 2));
        assert!(preview.revision > floor);
        assert_eq!(session.shared.floor.load(Ordering::Acquire), floor);
        let committed = run(Operation::LayerMove(PAINT_LAYER_MOVE_COMMIT, 0, 11, 3));
        assert_eq!(
            session.shared.floor.load(Ordering::Acquire),
            committed.revision
        );
    }
    #[test]
    fn eyedropper_maps_document_pixels_to_scaled_fractional_viewports() {
        let mut frame = StampedFrame {
            id: 1,
            request_id: 1,
            generation: 1,
            revision: 1,
            frame: Arc::new(RenderFrame {
                region: Region {
                    x: 10.25,
                    y: 20.25,
                    width: 2.,
                    height: 2.,
                },
                width: 2,
                height: 2,
                revision: 1,
                lod: 1,
                pixels: FramePixels::Srgb(vec![
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 64, 32, 0, 128,
                ]),
            }),
        };
        assert_eq!(frame.sample(10, 20).unwrap(), [1., 0., 0., 1.]);
        assert_eq!(frame.sample(11, 20).unwrap(), [0., 1., 0., 1.]);
        // Downscaled output: each output pixel represents two document pixels per axis.
        Arc::get_mut(&mut frame.frame).unwrap().region.width = 4.;
        Arc::get_mut(&mut frame.frame).unwrap().region.height = 4.;
        assert_eq!(frame.sample(12, 20).unwrap(), [0., 1., 0., 1.]);
        assert_eq!(frame.sample(10, 22).unwrap(), [0., 0., 1., 1.]);
        // Enlarged/DPI output: two output pixels per document pixel, with a fractional origin.
        Arc::get_mut(&mut frame.frame).unwrap().region.width = 1.;
        Arc::get_mut(&mut frame.frame).unwrap().region.height = 1.;
        assert_eq!(frame.sample(10, 20).unwrap(), [1., 0., 0., 1.]);
        assert_eq!(frame.sample(i64::MIN, i64::MIN).unwrap(), [1., 0., 0., 1.]);
        let edge = frame.sample(i64::MAX, i64::MAX).unwrap();
        assert_eq!(edge, [0.5, 0.25, 0., 128. / 255.]);
    }
    #[test]
    fn panicked_session_reports_failure_and_remains_releasable() {
        let session = Session::new(64, 64).unwrap();
        session.submit(Operation::PanicForTest).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while session.sender.state() != WorkerState::Panicked {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        }
        let info = session.info();
        assert_ne!(info.flags & PAINT_SESSION_FAILED, 0);
        assert_eq!(info.flags & PAINT_SESSION_RUNNING, 0);
        assert_eq!(info.last_error_status, PAINT_INTERNAL_ERROR);
        assert!(session
            .execution_error(info.last_error_sequence)
            .unwrap()
            .contains("quarantined"));
        assert_eq!(
            session.submit(Operation::New(32, 32)).unwrap_err().status,
            PAINT_INTERNAL_ERROR
        );
        assert_eq!(session.frame(0).unwrap_err().status, PAINT_INTERNAL_ERROR);
        assert_eq!(
            session
                .viewport(PaintViewport::default())
                .unwrap_err()
                .status,
            PAINT_INTERNAL_ERROR
        );
        session.stop();
    }
}

#[cfg(test)]
#[path = "session_wet_tests.rs"]
mod wet_tests;
