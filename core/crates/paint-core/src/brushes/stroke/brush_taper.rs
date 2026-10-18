//! Distance envelopes and a transactional bounded tail replay plan.
//! Pixel checkpoints and document mutation are deliberately owned by the caller.
use crate::{brush_dynamics::CompiledPressureControl, Error, InputPoint, PressureControl, Result};
use std::sync::Arc;

pub const TAPER_MODEL: &str = "distance-envelope-v1";
pub const MAX_TAPER_RECORDS: usize = 65_536;
pub const MAX_TAPER_DISTANCE: f64 = 1_000_000_000.;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TaperEnd {
    length: f64,
    relative_length: Option<(bool, f64)>,
    size: PressureControl,
    opacity: PressureControl,
}
impl TaperEnd {
    pub fn new(length: f64, size: PressureControl, opacity: PressureControl) -> Result<Self> {
        if !length.is_finite() || !(0. ..=4096.).contains(&length) {
            return Err(Error::InvalidArgument("taper length"));
        }
        for control in [&size, &opacity] {
            let points = control.curve().points();
            if points[0].1 != 0. || points.last().expect("validated curve").1 != 1. {
                return Err(Error::InvalidArgument("taper curve output endpoints"));
            }
        }
        Ok(Self {
            length,
            relative_length: None,
            size,
            opacity,
        })
    }
    pub fn length(&self) -> f64 {
        self.length
    }
    /// Preserve both modes' values, including while the relative mode is closed.
    pub fn with_relative_length(mut self, relative: bool, percent: f64) -> Result<Self> {
        if !percent.is_finite() || !(0. ..=10000.).contains(&percent) {
            return Err(Error::InvalidArgument("taper length percent"));
        }
        self.relative_length = Some((relative, percent));
        Ok(self)
    }
    pub fn relative_length(&self) -> Option<(bool, f64)> {
        self.relative_length
    }
    pub fn effective_length(&self, diameter: f64) -> f64 {
        match self.relative_length {
            Some((true, percent)) => (diameter * percent / 100.).min(4096.),
            _ => self.length,
        }
    }
    pub fn size(&self) -> &PressureControl {
        &self.size
    }
    pub fn opacity(&self) -> &PressureControl {
        &self.opacity
    }
    fn compile(&self, enabled: bool, diameter: f64) -> CompiledEnd {
        CompiledEnd {
            length: if enabled {
                self.effective_length(diameter)
            } else {
                0.
            },
            size: self.size.compile(),
            opacity: self.opacity.compile(),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StrokeTaper {
    enabled: bool,
    start: TaperEnd,
    end: TaperEnd,
}
impl StrokeTaper {
    pub fn new(enabled: bool, start: TaperEnd, end: TaperEnd) -> Self {
        Self {
            enabled,
            start,
            end,
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn start(&self) -> &TaperEnd {
        &self.start
    }
    pub fn end(&self) -> &TaperEnd {
        &self.end
    }
    pub fn has_relative_length(&self) -> bool {
        self.start.relative_length.is_some() || self.end.relative_length.is_some()
    }
    pub fn compile(&self, diameter: f64) -> Result<CompiledTaper> {
        if !diameter.is_finite()
            || !(1. ..=f64::from(crate::MAX_BRUSH_RADIUS) * 2.).contains(&diameter)
        {
            return Err(Error::InvalidArgument("taper brush diameter"));
        }
        Ok(CompiledTaper {
            start: self.start.compile(self.enabled, diameter),
            end: self.end.compile(self.enabled, diameter),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TaperFactors {
    pub diameter: f32,
    pub opacity: f32,
}
#[derive(Clone, Debug)]
struct CompiledEnd {
    length: f64,
    size: Option<CompiledPressureControl>,
    opacity: Option<CompiledPressureControl>,
}
impl CompiledEnd {
    fn factors(&self, distance: f64) -> TaperFactors {
        if self.length == 0. || distance >= self.length {
            return TaperFactors {
                diameter: 1.,
                opacity: 1.,
            };
        }
        let ratio = (distance / self.length) as f32;
        TaperFactors {
            diameter: self
                .size
                .as_ref()
                .map_or(ratio, |curve| curve.response(ratio)),
            opacity: self
                .opacity
                .as_ref()
                .map_or(ratio, |curve| curve.response(ratio)),
        }
    }
}
#[derive(Clone, Debug)]
pub struct CompiledTaper {
    start: CompiledEnd,
    end: CompiledEnd,
}
impl CompiledTaper {
    pub fn factors(&self, distance: f64, endpoint: f64) -> Result<TaperFactors> {
        validate_distance(endpoint)?;
        if !distance.is_finite() || distance < 0. || distance > endpoint {
            return Err(Error::InvalidArgument("taper sample distance"));
        }
        Ok(self.evaluate(distance, endpoint))
    }
    fn evaluate(&self, distance: f64, endpoint: f64) -> TaperFactors {
        let start = self.start.factors(distance);
        let end = self.end.factors(endpoint - distance);
        TaperFactors {
            diameter: start.diameter.min(end.diameter),
            opacity: start.opacity.min(end.opacity),
        }
    }
}
fn validate_distance(distance: f64) -> Result<()> {
    if !distance.is_finite() || !(0. ..=MAX_TAPER_DISTANCE).contains(&distance) {
        Err(Error::InvalidArgument("taper path distance"))
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaperStream {
    Primary,
    Secondary,
}
/// IDs are the original dispatch order; they are never renumbered on replay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TaperRecord {
    pub id: u64,
    pub distance: f64,
    pub stream: TaperStream,
    pub point: InputPoint,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TaperedRecord {
    pub record: TaperRecord,
    pub factors: TaperFactors,
}
#[derive(Debug)]
struct TailState {
    pending: Arc<[TaperRecord]>,
    endpoint: f64,
    last_id: Option<u64>,
    last_time: Option<u64>,
    finished: bool,
}
#[derive(Clone, Debug)]
pub struct TaperBuffer {
    program: Arc<CompiledTaper>,
    state: Arc<TailState>,
    capacity: usize,
}
/// Read-only preparation. Apply the replay to private pixels before committing this plan.
#[derive(Debug)]
pub struct TaperUpdate {
    base: Arc<TailState>,
    next: Arc<TailState>,
    program: Arc<CompiledTaper>,
    replay: Vec<TaperRecord>,
    retired: usize,
}
impl TaperUpdate {
    pub fn replay(&self) -> impl ExactSizeIterator<Item = TaperedRecord> + '_ {
        self.render(&self.replay)
    }
    /// Constant-time indexed replay for a bounded execution page.
    pub fn replay_at(&self, index: usize) -> Option<TaperedRecord> {
        self.replay.get(index).map(|&record| TaperedRecord {
            factors: self.program.evaluate(record.distance, self.next.endpoint),
            record,
        })
    }
    /// A checkpoint may advance past this prefix after these records have been rendered.
    pub fn retired(&self) -> impl ExactSizeIterator<Item = TaperedRecord> + '_ {
        self.render(&self.replay[..self.retired])
    }
    pub fn pending(&self) -> impl ExactSizeIterator<Item = TaperedRecord> + '_ {
        self.render(&self.next.pending)
    }
    fn render<'a>(
        &'a self,
        records: &'a [TaperRecord],
    ) -> impl ExactSizeIterator<Item = TaperedRecord> + 'a {
        records.iter().map(|&record| TaperedRecord {
            factors: self.program.evaluate(record.distance, self.next.endpoint),
            record,
        })
    }
}
impl TaperBuffer {
    pub fn new(settings: &StrokeTaper, capacity: usize, diameter: f64) -> Result<Self> {
        if !(1..=MAX_TAPER_RECORDS).contains(&capacity) {
            return Err(Error::ResourceLimit("taper buffer capacity"));
        }
        Ok(Self {
            program: Arc::new(settings.compile(diameter)?),
            capacity,
            state: Arc::new(TailState {
                pending: Arc::from([]),
                endpoint: 0.,
                last_id: None,
                last_time: None,
                finished: false,
            }),
        })
    }
    pub fn pending_len(&self) -> usize {
        self.state.pending.len()
    }
    pub fn endpoint(&self) -> f64 {
        self.state.endpoint
    }
    pub fn finished(&self) -> bool {
        self.state.finished
    }
    pub fn prepare_append(&self, records: &[TaperRecord], endpoint: f64) -> Result<TaperUpdate> {
        self.prepare(records, endpoint, false)
    }
    pub fn prepare_finish(&self, endpoint: f64) -> Result<TaperUpdate> {
        self.prepare(&[], endpoint, true)
    }
    fn prepare(&self, records: &[TaperRecord], endpoint: f64, finish: bool) -> Result<TaperUpdate> {
        if self.state.finished {
            return Err(Error::InvalidArgument("taper already finished"));
        }
        validate_distance(endpoint)?;
        if endpoint < self.state.endpoint {
            return Err(Error::InvalidArgument("taper endpoint must be monotonic"));
        }
        if records.len() > MAX_TAPER_RECORDS {
            return Err(Error::ResourceLimit("taper input batch"));
        }
        let mut last_distance = self.state.endpoint;
        let mut last_id = self.state.last_id;
        let mut last_time = self.state.last_time;
        for record in records {
            record.point.validate()?;
            if !record.distance.is_finite()
                || record.distance < last_distance
                || record.distance > endpoint
                || last_id.is_some_and(|id| record.id <= id)
                || last_time.is_some_and(|time| record.point.timestamp_ns < time)
            {
                return Err(Error::InvalidArgument("taper record order/distance/time"));
            }
            last_distance = record.distance;
            last_id = Some(record.id);
            last_time = Some(record.point.timestamp_ns);
        }
        if records.is_empty() && endpoint == self.state.endpoint && !finish {
            return Ok(TaperUpdate {
                base: self.state.clone(),
                next: self.state.clone(),
                program: self.program.clone(),
                replay: Vec::new(),
                retired: 0,
            });
        }
        let tail_length = self.program.end.length;
        let retire = |record: &&TaperRecord| finish || record.distance + tail_length <= endpoint;
        let retired = self
            .state
            .pending
            .iter()
            .chain(records)
            .take_while(retire)
            .count();
        let count = self.state.pending.len() + records.len();
        if count - retired > self.capacity {
            return Err(Error::ResourceLimit("taper pending records"));
        }
        // All validation and budgets precede allocation and state adoption.
        let mut replay = Vec::with_capacity(count);
        replay.extend_from_slice(&self.state.pending);
        replay.extend_from_slice(records);
        let pending = Arc::from(&replay[retired..]);
        Ok(TaperUpdate {
            base: self.state.clone(),
            program: self.program.clone(),
            retired,
            replay,
            next: Arc::new(TailState {
                pending,
                endpoint,
                last_id,
                last_time,
                finished: finish,
            }),
        })
    }
    pub fn commit(&mut self, update: TaperUpdate) -> Result<()> {
        if !Arc::ptr_eq(&self.state, &update.base) || !Arc::ptr_eq(&self.program, &update.program) {
            return Err(Error::InvalidArgument("stale or foreign taper update"));
        }
        self.state = update.next;
        Ok(())
    }
}
