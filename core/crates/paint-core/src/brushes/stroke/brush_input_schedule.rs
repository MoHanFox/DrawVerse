//! Ordered pages over original input segments; page boundaries never resample input.
use crate::{Error, InputPoint, Result};
use std::sync::Arc;

pub const MAX_INPUT_PAGE_DABS: usize = 8192;
pub const MAX_INPUT_PAGE_VISITS: usize = 8_000_000;
pub const MAX_INPUT_SEGMENTS: usize = 4096;
pub const MAX_INPUT_SCHEDULE_DABS: usize = 131_072;
pub const MAX_INPUT_SCHEDULE_VISITS: usize = 512_000_000;

// Keep a stream's original distance lattice across input/page boundaries.
// Repeated local += step accumulates a different error after splitting a line;
// wet feedback can amplify that error when sampling a sharp image mask.
pub(crate) fn next_input_position(position: f64, path_distance: f64, step: f64) -> f64 {
    let index = (path_distance + position) / step;
    let rounded = index.round();
    if (index - rounded).abs() <= 1e-9 {
        (rounded + 1.) * step - path_distance
    } else {
        position + step
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputScheduleWork {
    pub dabs: usize,
    pub visits: usize,
}
impl InputScheduleWork {
    pub fn repeated(self, count: usize) -> Self {
        Self {
            dabs: self.dabs.saturating_mul(count),
            visits: self.visits.saturating_mul(count),
        }
    }
    pub fn combined(self, other: Self) -> Self {
        Self {
            dabs: self.dabs.saturating_add(other.dabs),
            visits: self.visits.saturating_add(other.visits),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct InputScheduleBudget {
    pub(crate) work: InputScheduleWork,
    pub(crate) events: usize,
}
impl Default for InputScheduleBudget {
    fn default() -> Self {
        Self {
            work: InputScheduleWork {
                dabs: MAX_INPUT_PAGE_DABS,
                visits: MAX_INPUT_PAGE_VISITS,
            },
            events: MAX_INPUT_PAGE_DABS,
        }
    }
}
impl InputScheduleBudget {
    pub fn new(dabs: usize, visits: usize, events: usize) -> Result<Self> {
        if !(1..=MAX_INPUT_PAGE_DABS).contains(&dabs)
            || !(1..=MAX_INPUT_PAGE_VISITS).contains(&visits)
            || !(1..=MAX_INPUT_PAGE_DABS).contains(&events)
        {
            return Err(Error::InvalidArgument("input page budget"));
        }
        Ok(Self {
            work: InputScheduleWork { dabs, visits },
            events,
        })
    }
    pub(crate) fn fits(self, work: InputScheduleWork) -> bool {
        work.dabs <= self.work.dabs && work.visits <= self.work.visits
    }
}
#[derive(Clone, Copy, Debug)]
pub struct InputScheduleStart {
    pub point: InputPoint,
    pub primary_next: f64,
    pub secondary_next: f64,
    pub path_distance: f64,
    pub direction: Option<f64>,
}
#[derive(Clone, Copy, Debug)]
pub struct InputScheduleSteps {
    pub primary: f64,
    pub secondary: Option<f64>,
    /// Includes scatter, paired children, wet sampling and contribution work.
    pub primary_work: InputScheduleWork,
    pub secondary_work: InputScheduleWork,
}
#[derive(Clone, Copy, Debug)]
pub struct InputScheduleSegment {
    pub point: InputPoint,
    pub stationary_primary: bool,
    pub stationary_secondary: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputScheduleStream {
    Primary,
    Secondary,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputScheduleEvent {
    Dab {
        stream: InputScheduleStream,
        point: InputPoint,
        path_distance: f64,
        direction: Option<f64>,
    },
    SegmentEnd {
        point: InputPoint,
        path_distance: f64,
        primary_next: f64,
        secondary_next: f64,
        direction: Option<f64>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Active {
    end: InputPoint,
    distance: f64,
    primary_position: f64,
    secondary_position: f64,
    primary_left: usize,
    secondary_left: usize,
    stationary_primary: bool,
    stationary_secondary: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cursor {
    segment: usize,
    point: InputPoint,
    primary_next: f64,
    secondary_next: f64,
    path_distance: f64,
    direction: Option<f64>,
    active: Option<Active>,
}
/// Small, immutable prepared page. Inspect events, execute, then adopt on success.
#[derive(Debug)]
pub struct InputSchedulePage {
    owner: Arc<()>,
    generation: u64,
    after: Cursor,
    events: Vec<InputScheduleEvent>,
    work: InputScheduleWork,
    complete: bool,
}
impl InputSchedulePage {
    pub fn events(&self) -> &[InputScheduleEvent] {
        &self.events
    }
    pub fn work(&self) -> InputScheduleWork {
        self.work
    }
    pub fn complete(&self) -> bool {
        self.complete
    }
}
#[derive(Debug)]
pub struct BrushInputSchedule {
    owner: Arc<()>,
    generation: u64,
    segments: Vec<InputScheduleSegment>,
    steps: InputScheduleSteps,
    cursor: Cursor,
}
fn count(distance: f64, next: f64, step: f64) -> usize {
    if distance + 1e-9 < next {
        0
    } else {
        (((distance - next + 1e-9) / step).floor() as usize).saturating_add(1)
    }
}
impl BrushInputSchedule {
    pub fn new(
        start: InputScheduleStart,
        steps: InputScheduleSteps,
        segments: Vec<InputScheduleSegment>,
    ) -> Result<Self> {
        start.point.validate()?;
        if !steps.primary.is_finite()
            || steps.primary <= 0.
            || steps.secondary.is_some_and(|s| !s.is_finite() || s <= 0.)
            || !start.primary_next.is_finite()
            || start.primary_next <= 0.
            || start.primary_next > steps.primary * (1. + 1e-8) + 1e-8
            || (!start.secondary_next.is_finite()
                && !(steps.secondary.is_none() && start.secondary_next == f64::INFINITY))
            || start.secondary_next < 0.
            || (steps.secondary.is_some() && start.secondary_next == 0.)
            || steps
                .secondary
                .is_some_and(|s| start.secondary_next > s * (1. + 1e-8) + 1e-8)
            || !start.path_distance.is_finite()
            || start.path_distance < 0.
            || start.direction.is_some_and(|d| !d.is_finite())
        {
            return Err(Error::InvalidArgument("input schedule geometry"));
        }
        if segments.len() > MAX_INPUT_SEGMENTS {
            return Err(Error::ResourceLimit("input schedule segment count"));
        }
        let mut previous = start.point;
        for segment in &segments {
            segment.point.validate()?;
            if segment.point.timestamp_ns < previous.timestamp_ns
                || (segment.stationary_secondary && steps.secondary.is_none())
                || ((segment.stationary_primary || segment.stationary_secondary)
                    && (segment.point.x - previous.x).hypot(segment.point.y - previous.y) > 1e-9)
            {
                return Err(Error::InvalidArgument("input schedule segment"));
            }
            previous = segment.point;
        }
        let schedule = Self {
            owner: Arc::new(()),
            generation: 0,
            segments,
            steps,
            cursor: Cursor {
                segment: 0,
                point: start.point,
                primary_next: start.primary_next,
                secondary_next: start.secondary_next,
                path_distance: start.path_distance,
                direction: start.direction,
                active: None,
            },
        };
        // No whole-input dab array: preflight bounded arithmetic before any caller mutation.
        let mut cursor = schedule.cursor;
        let mut emitted = 0usize;
        let mut work = InputScheduleWork::default();
        while let Some(event) = schedule.next(&mut cursor)? {
            if matches!(event, InputScheduleEvent::Dab { .. }) {
                emitted += 1;
                let cost = schedule.cost(event);
                if !InputScheduleBudget::default().fits(cost) {
                    return Err(Error::ResourceLimit("single input dab exceeds page budget"));
                }
                work = work.combined(cost);
                if emitted > MAX_INPUT_SCHEDULE_DABS
                    || work.visits > MAX_INPUT_SCHEDULE_VISITS
                    || work.dabs > MAX_INPUT_SCHEDULE_DABS
                {
                    return Err(Error::ResourceLimit("input schedule total work"));
                }
            }
        }
        Ok(schedule)
    }
    pub fn complete(&self) -> bool {
        self.cursor.segment == self.segments.len()
    }
    pub fn prepare_page(&self, budget: InputScheduleBudget) -> Result<InputSchedulePage> {
        let mut cursor = self.cursor;
        let mut events = Vec::new();
        let mut work = InputScheduleWork::default();
        while events.len() < budget.events {
            let mut after = cursor;
            let Some(event) = self.next(&mut after)? else {
                break;
            };
            let next = work.combined(self.cost(event));
            if !budget.fits(next) {
                if events.is_empty() {
                    return Err(Error::ResourceLimit("single input dab exceeds page budget"));
                }
                break;
            }
            work = next;
            cursor = after;
            events.push(event);
        }
        Ok(InputSchedulePage {
            owner: self.owner.clone(),
            generation: self.generation,
            after: cursor,
            complete: cursor.segment == self.segments.len(),
            events,
            work,
        })
    }
    pub fn adopt(&mut self, page: InputSchedulePage) -> Result<()> {
        if !Arc::ptr_eq(&self.owner, &page.owner) || self.generation != page.generation {
            return Err(Error::InvalidArgument("foreign or stale input page"));
        }
        let next = self
            .generation
            .checked_add(1)
            .ok_or(Error::ResourceLimit("input page serial"))?;
        self.cursor = page.after;
        self.generation = next;
        Ok(())
    }
    fn cost(&self, event: InputScheduleEvent) -> InputScheduleWork {
        match event {
            InputScheduleEvent::Dab {
                stream: InputScheduleStream::Primary,
                ..
            } => self.steps.primary_work,
            InputScheduleEvent::Dab {
                stream: InputScheduleStream::Secondary,
                ..
            } => self.steps.secondary_work,
            InputScheduleEvent::SegmentEnd { .. } => InputScheduleWork::default(),
        }
    }
    fn next(&self, cursor: &mut Cursor) -> Result<Option<InputScheduleEvent>> {
        if cursor.segment == self.segments.len() {
            return Ok(None);
        }
        if cursor.active.is_none() {
            let segment = self.segments[cursor.segment];
            let distance =
                (segment.point.x - cursor.point.x).hypot(segment.point.y - cursor.point.y);
            let primary_left = count(distance, cursor.primary_next, self.steps.primary);
            let secondary_left = self
                .steps
                .secondary
                .map_or(0, |s| count(distance, cursor.secondary_next, s));
            if primary_left.saturating_add(secondary_left) > MAX_INPUT_SCHEDULE_DABS {
                return Err(Error::ResourceLimit("input schedule segment work"));
            }
            if distance > 1e-9 {
                cursor.direction = Some(
                    (segment.point.y - cursor.point.y).atan2(segment.point.x - cursor.point.x),
                );
            }
            cursor.active = Some(Active {
                end: segment.point,
                distance,
                primary_position: cursor.primary_next,
                secondary_position: cursor.secondary_next,
                primary_left,
                secondary_left,
                stationary_primary: segment.stationary_primary,
                stationary_secondary: segment.stationary_secondary,
            });
        }
        let active = cursor.active.as_mut().expect("initialized");
        let (stream, point, distance) = if active.primary_left > 0 || active.secondary_left > 0 {
            if active.secondary_left > 0
                && (active.primary_left == 0
                    || active.secondary_position <= active.primary_position + 1e-9)
            {
                let distance = active.secondary_position;
                let point = cursor
                    .point
                    .interpolate(active.end, (distance / active.distance).min(1.));
                active.secondary_position = next_input_position(
                    active.secondary_position,
                    cursor.path_distance,
                    self.steps.secondary.expect("independent stream"),
                );
                active.secondary_left -= 1;
                (
                    InputScheduleStream::Secondary,
                    point,
                    distance.min(active.distance),
                )
            } else {
                let distance = active.primary_position;
                let point = cursor
                    .point
                    .interpolate(active.end, (distance / active.distance).min(1.));
                active.primary_position = next_input_position(
                    active.primary_position,
                    cursor.path_distance,
                    self.steps.primary,
                );
                active.primary_left -= 1;
                (
                    InputScheduleStream::Primary,
                    point,
                    distance.min(active.distance),
                )
            }
        } else if active.stationary_secondary {
            active.stationary_secondary = false;
            (InputScheduleStream::Secondary, active.end, active.distance)
        } else if active.stationary_primary {
            active.stationary_primary = false;
            (InputScheduleStream::Primary, active.end, active.distance)
        } else {
            cursor.point = active.end;
            cursor.path_distance += active.distance;
            if !cursor.path_distance.is_finite() {
                return Err(Error::ResourceLimit("input path distance"));
            }
            cursor.primary_next =
                (active.primary_position - active.distance).max(self.steps.primary * 1e-9);
            if let Some(step) = self.steps.secondary {
                cursor.secondary_next =
                    (active.secondary_position - active.distance).max(step * 1e-9);
            }
            cursor.active = None;
            cursor.segment += 1;
            return Ok(Some(InputScheduleEvent::SegmentEnd {
                point: cursor.point,
                path_distance: cursor.path_distance,
                primary_next: cursor.primary_next,
                secondary_next: cursor.secondary_next,
                direction: cursor.direction,
            }));
        };
        Ok(Some(InputScheduleEvent::Dab {
            stream,
            point,
            path_distance: cursor.path_distance + distance,
            direction: cursor.direction,
        }))
    }
}

#[cfg(test)]
#[path = "brush_input_schedule_tests.rs"]
mod tests;
