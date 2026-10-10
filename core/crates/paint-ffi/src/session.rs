//! Serial document ownership and immutable publication. No caller pointers are retained.
use crate::{
    api::*,
    error::{ApiError, ApiResult},
};
use paint_core::{Brush, Document, InputPoint, LayerProperties};
use paint_render::{
    CachedCpuRenderer, FramePixels, PixelFormat, Region, RenderError, RenderFrame, RenderRequest,
    Renderer,
};
use paint_task::{
    Priority, SendError, SerialSender, SerialWorker, TaskError, TaskPool, TaskTicket, WorkerState,
};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[derive(Clone)]
pub(crate) enum Operation {
    File(crate::file_job::Request),
    #[cfg(test)]
    PanicForTest,
    New(u32, u32),
    NewWhite(u32, u32),
    Mask(u64),
    Clipping(u64, bool),
    Selection(crate::selection_api::Edit),
    Drop(u64, u64, u32),
    Begin(Brush, InputPoint, u64),
    Move(InputPoint),
    End,
    Cancel(bool),
    Undo,
    Redo,
    Add(String),
    Remove(u64),
    Select(u64),
    Properties(u64, LayerProperties),
    Appearance(u64, paint_core::LayerAppearance),
    Translate(u64, i32, i32),
    Group(u64, String),
    Ungroup(u64),
    Reparent(u64, u64),
    HistoryLimit(usize),
    /// ABI 1.12: freehand lasso path or magic-wand seed, resolved on the session worker.
    SelectionPath(crate::selection_api::Path),
}
#[derive(Clone)]
struct Command {
    sequence: u64,
    operation: Operation,
}
pub(crate) struct LayerState {
    pub info: PaintLayerInfo,
    pub name: String,
    pub appearance: paint_core::LayerAppearance,
    pub hierarchy: PaintLayerHierarchy,
    pub clipping: PaintLayerClipping,
}
pub(crate) struct Publication {
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
    pub frame: RenderFrame,
}
struct Shared {
    publication: Mutex<Arc<Publication>>,
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
    files: crate::file_job::FileService,
    document: Document,
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
    failed_views: [Option<(u64, u64, u64)>; 4],
}
impl Session {
    #[cfg(test)]
    pub fn new(width: u32, height: u32) -> ApiResult<Self> {
        Self::with_storage(width, height, crate::storage_api::CoreStorage::system())
    }
    pub fn with_storage(
        width: u32,
        height: u32,
        storage: Arc<crate::storage_api::CoreStorage>,
    ) -> ApiResult<Self> {
        let document =
            Document::with_storage(width, height, storage.options, storage.space.clone())?;
        let initial = publish(&document, 1, 1, 0, None, false);
        let shared = Arc::new(Shared {
            generation: AtomicU64::new(1),
            floor: AtomicU64::new(0),
            publication: Mutex::new(Arc::new(initial)),
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
            shared: Arc::clone(&shared),
            pool: Arc::clone(&pool),
            jobs: std::array::from_fn(|_| None),
            renderers: std::array::from_fn(|_| Arc::new(CachedCpuRenderer::default())),
            generation: 1,
            publication: 1,
            completed: 0,
            error: None,
            modified: false,
            dirty: true,
            aborted: false,
            floor: 0,
            frames: 0,
            failed_views: std::array::from_fn(|_| None),
        };
        engine.default_view();
        let worker = SerialWorker::spawn(4096, Duration::from_millis(8), move |command, token| {
            if token.is_cancelled() {
                return;
            }
            if let Some(command) = command {
                engine.execute(command);
            } else {
                engine.tick();
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
        if ingress.aborted && matches!(operation, Operation::Move(_) | Operation::End) {
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
            self.sender
                .send_cancel(command, |c| matches!(c.operation, Operation::Move(_)))
        } else {
            self.sender.try_send(command)
        };
        match result {
            Ok(()) => {
                ingress.sequence = sequence;
                if matches!(operation, Operation::Begin(..)) {
                    ingress.aborted = false;
                }
                if matches!(operation, Operation::Cancel(_)) {
                    ingress.aborted = true;
                }
                Ok(sequence)
            }
            Err(SendError::Full) if matches!(operation, Operation::Move(_) | Operation::End) => {
                self.sender
                    .send_cancel(
                        Command {
                            sequence,
                            operation: Operation::Cancel(true),
                        },
                        |c| matches!(c.operation, Operation::Move(_)),
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
    pub fn publication(&self) -> Arc<Publication> {
        Arc::clone(
            &self
                .shared
                .publication
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        )
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
            || frame.frame.revision < self.shared.floor.load(Ordering::Acquire)
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
    let (undo, redo) = doc.history_depth();
    let layers = doc
        .layers()
        .iter()
        .zip(doc.layer_hierarchies())
        .zip(doc.layer_clipping_bases())
        .map(|((layer, (parent, depth, locks)), base)| LayerState {
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
        selection: doc.selection().clone(),
        info: PaintSessionInfo {
            struct_size: std::mem::size_of::<PaintSessionInfo>() as u32,
            flags: PAINT_SESSION_RUNNING | if modified { PAINT_SESSION_MODIFIED } else { 0 },
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
            .history_actions()
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
    fn execute(&mut self, command: Command) {
        let old_revision = self.document.revision();
        let stroke_command = matches!(command.operation, Operation::Move(_) | Operation::End)
            || (matches!(command.operation, Operation::Begin(..))
                && !self.document.stroke_active());
        let barrier = matches!(
            command.operation,
            Operation::New(..)
                | Operation::NewWhite(..)
                | Operation::Mask(_)
                | Operation::Clipping(..)
                | Operation::Selection(..)
                | Operation::Drop(..)
                | Operation::Cancel(_)
                | Operation::Undo
                | Operation::Redo
                | Operation::Remove(_)
                | Operation::Properties(..)
                | Operation::Appearance(..)
                | Operation::Translate(..)
                | Operation::Group(..)
                | Operation::Ungroup(..)
                | Operation::Reparent(..)
                | Operation::Add(_)
        );
        let result = self.apply(command.operation);
        if let Err(error) = result {
            if error.status != PAINT_CANCELLED || self.error.is_none() {
                self.error = Some((command.sequence, error));
            }
            if stroke_command && self.document.stroke_active() {
                let _ = self.document.cancel_stroke();
            }
            if stroke_command {
                self.aborted = true;
                self.floor = self.document.revision();
            }
        }
        if barrier && old_revision != self.document.revision() {
            self.floor = self.document.revision();
        }
        self.shared.floor.store(self.floor, Ordering::Release);
        self.completed = command.sequence;
        self.dirty = true;
    }
    fn apply(&mut self, operation: Operation) -> ApiResult<()> {
        match operation {
            Operation::File(request) => self.files.start(request, &self.document, self.generation),
            #[cfg(test)]
            Operation::PanicForTest => panic!("actor panic injection"),
            Operation::Mask(id) => {
                self.document.add_mask(id)?;
                self.modified = true;
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
                    crate::selection_api::Edit::All => paint_core::HistoryAction::SelectAll,
                    crate::selection_api::Edit::Clear => paint_core::HistoryAction::Deselect,
                    crate::selection_api::Edit::Invert => {
                        paint_core::HistoryAction::InvertSelection
                    }
                };
                let next = match edit {
                    crate::selection_api::Edit::Shape(shape, op) => {
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
            Operation::Drop(id, target, placement) => {
                let r = self.document.revision();
                self.document.drop_layer(id, target, placement)?;
                self.modified |= r != self.document.revision();
            }
            Operation::HistoryLimit(max_commands) => {
                self.document.set_max_history_commands(max_commands)?;
            }
            Operation::SelectionPath(path) => {
                let revision = self.document.revision();
                // Only the magic wand resolves a region now: the freehand lasso is not supported.
                let [x, y] = path.points[0];
                self.document.set_selection_magic(
                    x.max(0.) as u32,
                    y.max(0.) as u32,
                    path.tolerance,
                )?;
                self.modified |= revision != self.document.revision();
            }
            Operation::NewWhite(w, h) => {
                self.apply(Operation::New(w, h))?;
                self.document.initialize_white_background()?;
            }
            Operation::New(w, h) => {
                self.document = Document::with_storage(
                    w,
                    h,
                    self.document.document_options(),
                    self.document.storage(),
                )?;
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
            Operation::Begin(brush, point, layer) => {
                self.aborted = false;
                if self.document.stroke_active() {
                    return Err(paint_core::Error::Busy.into());
                }
                let previous = self.document.active_layer();
                if layer != 0 {
                    self.document.set_active_layer(layer)?;
                }
                if let Err(error) = self.document.begin_stroke(brush, point) {
                    // Selection must follow the actor's immediate state, not a stale UI hint.
                    if self.document.layer(previous).is_some() {
                        self.document.set_active_layer(previous)?;
                    }
                    return Err(error.into());
                }
            }
            Operation::Move(point) => {
                if self.aborted {
                    return Err(ApiError::new(PAINT_CANCELLED, "stroke rolled back"));
                }
                self.document.stroke_to(point)?;
            }
            Operation::End => {
                if self.aborted {
                    return Err(ApiError::new(PAINT_CANCELLED, "stroke rolled back"));
                }
                self.modified |= self.document.end_stroke()?;
            }
            Operation::Cancel(overload) => {
                if self.document.stroke_active() {
                    self.document.cancel_stroke()?;
                }
                self.aborted = true;
                if overload {
                    return Err(ApiError::new(
                        PAINT_LIMIT_EXCEEDED,
                        "input queue full; entire stroke cancelled",
                    ));
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
        if let Some((document, clear_modified)) = self.files.poll(
            self.generation,
            self.document.revision(),
            self.document.stroke_active(),
        ) {
            if let Some(document) = document {
                self.document = document;
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
            let publication = Arc::new(publish(
                &self.document,
                self.generation,
                self.publication,
                self.completed,
                self.error.as_ref(),
                self.modified,
            ));
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
            let mut slots = self.shared.views.lock().unwrap_or_else(|e| e.into_inner());
            let slot = &mut slots[index];
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
                            && slot
                                .frame
                                .as_ref()
                                .is_none_or(|old| frame.revision >= old.frame.revision)
                        {
                            self.frames += 1;
                            let old = slot.frame.replace(Arc::new(StampedFrame {
                                id: self.frames,
                                request_id: job.request_id,
                                generation: job.generation,
                                frame,
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
            if slot.preview_layer != 0 && self.document.stroke_active() {
                continue;
            }
            if slot
                .frame
                .as_ref()
                .is_some_and(|frame| frame.frame.revision == self.document.revision())
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
                self.document.snapshot()
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
            let revision = snapshot.revision;
            let priority = if index == 0 {
                Priority::Interactive
            } else {
                Priority::Background
            };
            if let Ok(ticket) = self.pool.submit(priority, move |token| {
                match renderer.render(&snapshot, request, &token) {
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
        // The region is in document coordinates and may be fractional; sample its pixel grid.
        let origin_x = self.frame.region.x.floor() as i64;
        let origin_y = self.frame.region.y.floor() as i64;
        let width = self.frame.width;
        let height = self.frame.height;
        if width == 0 || height == 0 {
            return Err(ApiError::new(PAINT_NOT_FOUND, "frame has no pixels"));
        }
        // Clamp to the rendered region so a pointer just outside still samples the edge pixel.
        let px = (x - origin_x).clamp(0, i64::from(width) - 1) as u32;
        let py = (y - origin_y).clamp(0, i64::from(height) - 1) as u32;
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
            revision: self.frame.revision,
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
