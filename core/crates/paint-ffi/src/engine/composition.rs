//! Bounded display leases. Pixel IO is performed by the calling worker, outside
//! the service mutex; cancellation retains accounting until that read releases.
use crate::{
    api::*,
    error::{ApiError, ApiResult},
};
use paint_render::{
    composition::{self, ContentIdentities, Limits, Plan, PlanError},
    PixelFormat, Region, RenderRequest,
};
use paint_task::CancellationToken;
use std::{
    collections::BTreeMap,
    mem::size_of,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
};

pub const PROCESS_PLAN_BYTES: usize = 512 * 1024 * 1024;
pub const PROCESS_READ_BYTES: usize = 16 * 1024 * 1024;
static PLAN_BYTES: AtomicUsize = AtomicUsize::new(0);
static READ_BYTES: AtomicUsize = AtomicUsize::new(0);
static PLAN_PEAK: AtomicUsize = AtomicUsize::new(0);
static READ_PEAK: AtomicUsize = AtomicUsize::new(0);
static NEXT_LEASE: AtomicU64 = AtomicU64::new(1);
static IDENTITIES: OnceLock<Mutex<ContentIdentities>> = OnceLock::new();

pub struct Charge {
    counter: &'static AtomicUsize,
    bytes: usize,
}
impl Charge {
    fn take(
        counter: &'static AtomicUsize,
        peak: &AtomicUsize,
        bytes: usize,
        limit: usize,
    ) -> ApiResult<Self> {
        let mut used = counter.load(Ordering::Acquire);
        loop {
            let total = used
                .checked_add(bytes)
                .filter(|n| *n <= limit)
                .ok_or_else(|| {
                    ApiError::new(PAINT_LIMIT_EXCEEDED, "composition process byte budget")
                })?;
            match counter.compare_exchange_weak(used, total, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => {
                    peak.fetch_max(total, Ordering::Relaxed);
                    return Ok(Self { counter, bytes });
                }
                Err(current) => used = current,
            }
        }
    }
    pub fn read() -> ApiResult<Self> {
        Self::take(
            &READ_BYTES,
            &READ_PEAK,
            paint_core::TILE_BYTES,
            PROCESS_READ_BYTES,
        )
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        self.counter.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

pub fn error(error: PlanError) -> ApiError {
    match error {
        PlanError::Invalid => ApiError::invalid("invalid composition request"),
        PlanError::Unsupported(reason) => ApiError::new(PAINT_UNSUPPORTED, reason),
        PlanError::Budget => ApiError::new(PAINT_LIMIT_EXCEEDED, "composition plan budget"),
        PlanError::Cancelled => cancelled(),
        PlanError::Storage(message) => ApiError::new(PAINT_IO_ERROR, message),
    }
}
fn cancelled() -> ApiError {
    ApiError::new(
        PAINT_CANCELLED,
        "composition lease cancelled, stale or closed",
    )
}

#[derive(Clone)]
pub struct Request {
    pub key: PaintCompositionKey,
    pub latest_snapshot: bool,
    pub render: RenderRequest,
    pub limits: Limits,
    pub cancel: CancellationToken,
}
impl Request {
    pub fn accepts_revision(&self, revision: u64) -> bool {
        self.key.revision == revision || (self.latest_snapshot && self.key.revision < revision)
    }
    pub fn parse(desc: PaintCompositionRequest) -> ApiResult<Self> {
        if desc.version != composition::VERSION {
            return Err(ApiError::new(PAINT_UNSUPPORTED, "composition version"));
        }
        let view = desc.viewport;
        if desc.reserved != [0; 2]
            || view.reserved != 0
            || view.enabled != 1
            || view.view_id >= 4
            || view.struct_size < size_of::<PaintViewport>() as u32
            || desc.request_id == 0
            || desc.device_epoch == 0
            || view.document_generation == 0
        {
            return Err(ApiError::invalid(
                "composition request identity or reserved fields",
            ));
        }
        let limits = Limits {
            features: desc.features & !PAINT_COMPOSITION_LATEST_SNAPSHOT,
            nodes: desc.max_nodes as usize,
            tiles: desc.max_tiles as usize,
            pinned_bytes: usize::try_from(desc.max_pinned_bytes)
                .map_err(|_| ApiError::invalid("pin byte size"))?,
            metadata_bytes: usize::try_from(desc.max_metadata_bytes)
                .map_err(|_| ApiError::invalid("metadata byte size"))?,
        };
        limits.validate().map_err(error)?;
        let render = RenderRequest {
            region: Region {
                x: view.x,
                y: view.y,
                width: view.width,
                height: view.height,
            },
            width: view.pixel_width,
            height: view.pixel_height,
            format: PixelFormat::LinearRgba32F,
        };
        render.validate().map_err(|e| error(e.into()))?;
        Ok(Self {
            key: PaintCompositionKey {
                struct_size: size_of::<PaintCompositionKey>() as u32,
                view_id: view.view_id,
                lease_id: 0,
                request_id: desc.request_id,
                device_epoch: desc.device_epoch,
                document_generation: view.document_generation,
                revision: desc.revision,
                publication: 0,
            },
            latest_snapshot: desc.features & PAINT_COMPOSITION_LATEST_SNAPSHOT != 0,
            render,
            limits,
            cancel: CancellationToken::default(),
        })
    }
}
pub struct LeasedPlan {
    pub plan: Plan,
    _charge: Charge,
}
struct Slot {
    request: Request,
    key: PaintCompositionKey,
    started: bool,
    result: Option<ApiResult<Arc<LeasedPlan>>>,
}
#[derive(Default)]
struct State {
    closed: bool,
    generation: u64,
    // Per-view monotonic caller request and device epoch, independent of revision.
    views: [(u64, u64); 4],
    slots: BTreeMap<u64, Slot>,
}
#[derive(Default)]
pub struct Service {
    state: Mutex<State>,
}
impl Service {
    pub fn reserve(
        &self,
        mut request: Request,
        generation: u64,
        revision: u64,
    ) -> ApiResult<Request> {
        let mut state = self.state.lock().map_err(|_| ApiError::internal())?;
        if state.closed {
            return Err(cancelled());
        }
        if request.key.document_generation != generation || !request.accepts_revision(revision) {
            return Err(ApiError::new(
                PAINT_BUSY,
                "stale composition document revision",
            ));
        }
        if state.generation > generation {
            return Err(ApiError::new(PAINT_BUSY, "stale composition generation"));
        }
        if state.generation < generation {
            Self::reset(&mut state, generation);
        }
        let view = request.key.view_id as usize;
        let (epoch, last) = state.views[view];
        if request.key.device_epoch < epoch || request.key.request_id <= last {
            return Err(ApiError::new(
                PAINT_BUSY,
                "stale composition request or device epoch",
            ));
        }
        if request.key.device_epoch > epoch {
            state.slots.retain(|_, slot| {
                if slot.key.view_id == view as u32 {
                    slot.request.cancel.cancel();
                    false
                } else {
                    true
                }
            });
        }
        // Superseded pending display work may coalesce; ready leases remain
        // alive for atomic adoption until the consumer explicitly releases.
        state.slots.retain(|_, slot| {
            if slot.key.view_id == view as u32 && slot.result.is_none() {
                slot.request.cancel.cancel();
                false
            } else {
                true
            }
        });
        if state.slots.len() >= 2 {
            return Err(ApiError::new(
                PAINT_LIMIT_EXCEEDED,
                "two composition leases per session",
            ));
        }
        let mut id = NEXT_LEASE.load(Ordering::Relaxed);
        loop {
            let next = id.checked_add(1).ok_or_else(|| {
                ApiError::new(PAINT_LIMIT_EXCEEDED, "composition lease IDs exhausted")
            })?;
            match NEXT_LEASE.compare_exchange_weak(id, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(current) => id = current,
            }
        }
        request.key.lease_id = id;
        state.views[view] = (request.key.device_epoch, request.key.request_id);
        state.slots.insert(
            id,
            Slot {
                key: request.key,
                request: request.clone(),
                started: false,
                result: None,
            },
        );
        Ok(request)
    }
    fn reset(state: &mut State, generation: u64) {
        for slot in state.slots.values() {
            slot.request.cancel.cancel();
        }
        state.slots.clear();
        state.generation = generation;
    }
    pub fn generation(&self, generation: u64) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.closed && state.generation < generation {
            Self::reset(&mut state, generation);
        }
    }
    pub fn close(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.closed = true;
        Self::reset(&mut state, 0);
    }
    pub fn release(&self, id: u64) -> ApiResult<()> {
        let mut state = self.state.lock().map_err(|_| ApiError::internal())?;
        let slot = state.slots.remove(&id).ok_or_else(cancelled)?;
        slot.request.cancel.cancel();
        Ok(())
    }
    pub fn complete(
        &self,
        request: &Request,
        publication: u64,
        result: ApiResult<Arc<LeasedPlan>>,
    ) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(slot) = state.slots.get_mut(&request.key.lease_id) {
            if !slot.request.cancel.is_cancelled() {
                slot.key.publication = publication;
                slot.key.revision = request.key.revision;
                slot.result = Some(result);
            }
        }
    }
    pub fn take_pending(&self) -> Vec<Request> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state
            .slots
            .values_mut()
            .filter_map(|slot| {
                if slot.started || slot.result.is_some() {
                    None
                } else {
                    slot.started = true;
                    Some(slot.request.clone())
                }
            })
            .collect()
    }
    pub fn compile(
        request: &Request,
        dimensions: (u32, u32),
        revision: u64,
        layers: &[paint_core::Layer],
    ) -> ApiResult<Arc<LeasedPlan>> {
        if request.cancel.is_cancelled() {
            return Err(cancelled());
        }
        let (pins, metadata) = composition::estimate(layers, request.limits).map_err(error)?;
        let charge = Charge::take(&PLAN_BYTES, &PLAN_PEAK, pins + metadata, PROCESS_PLAN_BYTES)?;
        let mut ids = IDENTITIES
            .get_or_init(|| Mutex::new(ContentIdentities::default()))
            .lock()
            .map_err(|_| ApiError::internal())?;
        let plan = composition::prepare(
            dimensions,
            revision,
            layers,
            request.render,
            request.limits,
            &mut ids,
            &request.cancel,
        )
        .map_err(error)?;
        Ok(Arc::new(LeasedPlan {
            plan,
            _charge: charge,
        }))
    }
    pub fn info(&self, id: u64) -> ApiResult<PaintCompositionInfo> {
        let state = self.state.lock().map_err(|_| ApiError::internal())?;
        let slot = state.slots.get(&id).ok_or_else(cancelled)?;
        let mut info = PaintCompositionInfo {
            struct_size: size_of::<PaintCompositionInfo>() as u32,
            version: composition::VERSION,
            key: slot.key,
            ..Default::default()
        };
        match &slot.result {
            None => info.state = PAINT_COMPOSITION_PENDING,
            Some(Err(e)) => {
                info.state = PAINT_COMPOSITION_FAILED;
                info.status = e.status;
            }
            Some(Ok(result)) => {
                let p = &result.plan;
                info.state = PAINT_COMPOSITION_READY;
                info.required_features = p.required_features;
                info.document_width = p.dimensions.0;
                info.document_height = p.dimensions.1;
                info.node_count = p.nodes.len() as u32;
                info.tile_count = p.tiles.len() as u32;
                info.pinned_bytes = p.pinned_bytes as u64;
                info.metadata_bytes = p.metadata_bytes as u64;
                info.budget_bytes = result._charge.bytes as u64;
                if !p.operations.is_empty() {
                    info.reserved = [
                        p.operations.len() as u64,
                        (u64::from(p.surface_count) << 32) | u64::from(p.output_surface),
                    ];
                }
            }
        }
        Ok(info)
    }
    pub fn get(&self, key: PaintCompositionKey) -> ApiResult<(Arc<LeasedPlan>, CancellationToken)> {
        let state = self.state.lock().map_err(|_| ApiError::internal())?;
        let slot = state.slots.get(&key.lease_id).ok_or_else(cancelled)?;
        if slot.key != key {
            return Err(ApiError::new(PAINT_BUSY, "composition identity mismatch"));
        }
        match &slot.result {
            None => Err(ApiError::new(PAINT_BUSY, "composition pending")),
            Some(Err(e)) => Err(e.clone()),
            Some(Ok(plan)) => Ok((Arc::clone(plan), slot.request.cancel.clone())),
        }
    }
    /// Caller performs only a bounded memory copy while holding this guard.
    /// A cancelled read cannot publish bytes after cancellation has linearized.
    pub fn publish_read<T>(
        &self,
        key: PaintCompositionKey,
        copy: impl FnOnce() -> ApiResult<T>,
    ) -> ApiResult<T> {
        let state = self.state.lock().map_err(|_| ApiError::internal())?;
        let slot = state.slots.get(&key.lease_id).ok_or_else(cancelled)?;
        if slot.key != key || slot.request.cancel.is_cancelled() {
            return Err(cancelled());
        }
        copy()
    }
}

pub fn usage() -> PaintCompositionUsage {
    PaintCompositionUsage {
        struct_size: size_of::<PaintCompositionUsage>() as u32,
        reserved: 0,
        plan_bytes: PLAN_BYTES.load(Ordering::Acquire) as u64,
        read_bytes: READ_BYTES.load(Ordering::Acquire) as u64,
        peak_plan_bytes: PLAN_PEAK.load(Ordering::Acquire) as u64,
        peak_read_bytes: READ_PEAK.load(Ordering::Acquire) as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_core::{Brush, Document, InputPoint};
    static TEST: Mutex<()> = Mutex::new(());
    fn request(id: u64, epoch: u64, doc: &Document) -> Request {
        Request {
            key: PaintCompositionKey {
                struct_size: size_of::<PaintCompositionKey>() as u32,
                request_id: id,
                device_epoch: epoch,
                document_generation: 1,
                revision: doc.revision(),
                ..Default::default()
            },
            render: RenderRequest {
                region: Region {
                    x: 0.,
                    y: 0.,
                    width: 64.,
                    height: 64.,
                },
                width: 64,
                height: 64,
                format: PixelFormat::LinearRgba32F,
            },
            latest_snapshot: false,
            limits: Limits::default(),
            cancel: CancellationToken::default(),
        }
    }
    fn document() -> Document {
        let mut doc = Document::new(64, 64).unwrap();
        doc.begin_stroke(Brush::default(), InputPoint::new(24., 24., 1.))
            .unwrap();
        doc.end_stroke().unwrap();
        doc
    }
    #[test]
    fn latest_snapshot_adopts_capture_identity_without_weakening_exact_requests() {
        let _guard = TEST.lock().unwrap();
        let mut doc = document();
        let service = Service::default();
        let mut latest = request(1, 1, &doc);
        let exact = latest.clone();
        latest.latest_snapshot = true;
        let floor = doc.revision();
        doc.begin_stroke(Brush::default(), InputPoint::new(12., 12., 1.))
            .unwrap();
        doc.end_stroke().unwrap();
        assert!(doc.revision() > floor);
        assert!(!exact.accepts_revision(doc.revision()));
        assert!(latest.accepts_revision(doc.revision()));
        assert!(!latest.accepts_revision(floor - 1));
        assert_eq!(
            service
                .reserve(exact, 1, doc.revision())
                .err()
                .unwrap()
                .status,
            PAINT_BUSY
        );
        let mut captured = service.reserve(latest, 1, doc.revision()).unwrap();
        captured.key.revision = doc.revision();
        let plan =
            Service::compile(&captured, doc.dimensions(), doc.revision(), doc.layers()).unwrap();
        service.complete(&captured, 99, Ok(plan));
        let info = service.info(captured.key.lease_id).unwrap();
        assert_eq!(info.key.revision, doc.revision());
        assert_eq!(info.key.publication, 99);
        assert_eq!(
            info.required_features & PAINT_COMPOSITION_LATEST_SNAPSHOT,
            0
        );
        service.release(captured.key.lease_id).unwrap();
    }
    #[test]
    fn lease_close_rejects_inflight_publication_and_retains_charge_until_last_owner() {
        let _guard = TEST.lock().unwrap();
        let doc = document();
        let service = Service::default();
        let base = usage().plan_bytes;
        let request = service
            .reserve(request(1, 1, &doc), 1, doc.revision())
            .unwrap();
        let plan =
            Service::compile(&request, doc.dimensions(), doc.revision(), doc.layers()).unwrap();
        let bytes = plan._charge.bytes as u64;
        service.complete(&request, 7, Ok(plan));
        let key = service.info(request.key.lease_id).unwrap().key;
        let (reading, cancel) = service.get(key).unwrap();
        service.close();
        assert!(cancel.is_cancelled());
        assert_eq!(usage().plan_bytes, base + bytes);
        let wrote = std::sync::atomic::AtomicBool::new(false);
        assert_eq!(
            service
                .publish_read(key, || {
                    wrote.store(true, Ordering::Relaxed);
                    Ok(())
                })
                .unwrap_err()
                .status,
            PAINT_CANCELLED
        );
        assert!(!wrote.load(Ordering::Relaxed));
        drop(reading);
        assert_eq!(usage().plan_bytes, base);
    }
    #[test]
    fn pending_coalesces_outside_input_fifo_and_old_device_requests_cannot_revive() {
        let _guard = TEST.lock().unwrap();
        let doc = document();
        let service = Service::default();
        let first = service
            .reserve(request(1, 1, &doc), 1, doc.revision())
            .unwrap();
        let next = service
            .reserve(request(2, 1, &doc), 1, doc.revision())
            .unwrap();
        assert!(first.cancel.is_cancelled());
        assert_eq!(service.take_pending().len(), 1);
        assert!(service.take_pending().is_empty());
        service.complete(&next, 1, Err(ApiError::new(PAINT_UNSUPPORTED, "fixture")));
        let new_device = service
            .reserve(request(3, 2, &doc), 1, doc.revision())
            .unwrap();
        assert!(next.cancel.is_cancelled());
        assert_eq!(
            service.info(next.key.lease_id).unwrap_err().status,
            PAINT_CANCELLED
        );
        assert_eq!(
            service
                .reserve(request(4, 1, &doc), 1, doc.revision())
                .err()
                .unwrap()
                .status,
            PAINT_BUSY
        );
        service.generation(2);
        assert!(new_device.cancel.is_cancelled());
        // A late reader carrying an older atomic generation cannot roll back.
        service.generation(1);
        assert_eq!(
            service
                .reserve(request(5, 2, &doc), 1, doc.revision())
                .err()
                .unwrap()
                .status,
            PAINT_BUSY
        );
    }
    #[test]
    fn conservative_budget_reservations_are_atomic_bounded_and_release_on_errors() {
        let _guard = TEST.lock().unwrap();
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let a = Charge::take(&COUNTER, &peak, 40, 64).unwrap();
        assert!(Charge::take(&COUNTER, &peak, 25, 64).is_err());
        let b = Charge::take(&COUNTER, &peak, 24, 64).unwrap();
        assert_eq!(COUNTER.load(Ordering::Acquire), 64);
        assert_eq!(peak.load(Ordering::Relaxed), 64);
        drop(a);
        drop(b);
        assert_eq!(COUNTER.load(Ordering::Acquire), 0);
        assert!(Charge::take(&COUNTER, &peak, usize::MAX, 64).is_err());
    }
}
