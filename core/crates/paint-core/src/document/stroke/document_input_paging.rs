//! Resumable input and end transactions over the existing pixel operators.
use super::*;
use crate::{
    BrushInputSchedule, InputScheduleBudget, InputScheduleEvent, InputScheduleSegment,
    InputScheduleStart, InputScheduleSteps, InputScheduleStream, InputScheduleWork, WetInk,
};
use std::collections::VecDeque;

#[derive(Debug)]
pub enum StrokePageResult {
    Pending,
    InputComplete,
    StrokeComplete {
        changed: bool,
        wet_ink: Option<WetInk>,
    },
}
#[derive(Debug)]
enum Kind {
    Input(Box<BrushInputSchedule>),
    Finish(VecDeque<InputScheduleEvent>),
}
#[derive(Debug)]
pub(super) struct PendingInput {
    kind: Kind,
    dispatched: bool,
    work: InputScheduleWork,
}
impl PendingInput {
    fn charge(&mut self, work: InputScheduleWork) -> Result<()> {
        let next = self.work.combined(work);
        if next.visits > crate::MAX_INPUT_SCHEDULE_VISITS
            || next.dabs > crate::MAX_INPUT_SCHEDULE_DABS
        {
            return Err(Error::ResourceLimit(
                "input execution including tail exceeds total budget",
            ));
        }
        self.work = next;
        Ok(())
    }
}
impl Document {
    /// A partial tail restore/replay must not be published as a completed preview.
    pub fn stroke_page_ready_for_publication(&self) -> bool {
        if self.pending_input.is_some() && self.stroke.as_ref().is_some_and(|s| s.outline.is_some())
        {
            return false;
        }
        !self
            .stroke
            .as_ref()
            .and_then(|s| s.taper.as_ref())
            .is_some_and(|t| t.pending_render())
    }
    pub fn stroke_input_pending(&self) -> bool {
        self.pending_input.is_some()
    }
    /// Capture and validate one raw input. Page boundaries do not become input samples.
    pub fn begin_stroke_input(&mut self, point: InputPoint) -> Result<()> {
        if self.pending_input.is_some() {
            return Err(Error::Busy);
        }
        self.prepare_outline_input();
        let result = self.prepare_stroke_input(point);
        if result.is_err() {
            if let Some(stroke) = self.stroke.take() {
                self.restore_stroke(&stroke);
            }
        }
        result
    }
    fn prepare_stroke_input(&mut self, point: InputPoint) -> Result<()> {
        point.validate()?;
        let stroke = self.stroke.as_ref().ok_or(Error::NoStroke)?;
        Self::validate_input_tool(stroke, point)?;
        let mut filter = stroke.path_filter.clone();
        let points = if let Some(filter) = &mut filter {
            filter.push(point)?
        } else {
            vec![point]
        };
        let mut previous = stroke.last;
        let mut segments = Vec::with_capacity(points.len());
        for point in points {
            Self::validate_input_tool(stroke, point)?;
            segments.push(InputScheduleSegment {
                point,
                stationary_primary: Self::stationary_primary(stroke, previous, point),
                stationary_secondary: Self::stationary_secondary(stroke, previous, point),
            });
            previous = point;
        }
        let (primary_work, secondary_work) = Self::stroke_input_work(
            stroke.brush,
            &stroke.program,
            stroke.wet.as_ref().map_or(0, |w| w.work_per_dab),
        );
        let schedule = BrushInputSchedule::new(
            InputScheduleStart {
                point: stroke.last,
                primary_next: stroke.distance_to_next,
                secondary_next: stroke.dual_next,
                path_distance: stroke.path_distance,
                direction: stroke.direction,
            },
            InputScheduleSteps {
                primary: stroke.program.step(stroke.brush),
                secondary: stroke.program.dual.as_ref().and_then(|d| d.step()),
                primary_work,
                secondary_work,
            },
            segments,
        )?;
        self.stroke.as_mut().expect("validated stroke").path_filter = filter;
        self.pending_input = Some(PendingInput {
            kind: Kind::Input(Box::new(schedule)),
            dispatched: false,
            work: InputScheduleWork::default(),
        });
        Ok(())
    }
    /// Capture the real filtered endpoint once. No release line or synthetic pressure is added.
    pub fn begin_stroke_finish(&mut self) -> Result<()> {
        if self.pending_input.is_some() {
            return Err(Error::Busy);
        }
        self.prepare_outline_input();
        let stroke = self.stroke.as_ref().ok_or(Error::NoStroke)?;
        let secondary = stroke
            .program
            .dual
            .as_ref()
            .is_some_and(|d| d.step().is_some())
            && (stroke.last.x - stroke.dual_last.x).hypot(stroke.last.y - stroke.dual_last.y)
                > 1e-7;
        let primary = (stroke.last.x - stroke.last_dab.x).hypot(stroke.last.y - stroke.last_dab.y)
            > 1e-7
            || secondary;
        let mut events = VecDeque::new();
        for (enabled, stream) in [
            (secondary, InputScheduleStream::Secondary),
            (primary, InputScheduleStream::Primary),
        ] {
            if enabled {
                events.push_back(InputScheduleEvent::Dab {
                    stream,
                    point: stroke.last,
                    path_distance: stroke.path_distance,
                    direction: stroke.direction,
                });
            }
        }
        self.pending_input = Some(PendingInput {
            kind: Kind::Finish(events),
            dispatched: false,
            work: InputScheduleWork::default(),
        });
        Ok(())
    }
    /// Execute one bounded dispatch or active-tail replay page, then return control to the caller.
    /// Any failure rolls back the entire stroke, including earlier successful pages.
    pub fn continue_stroke_page(
        &mut self,
        budget: InputScheduleBudget,
    ) -> Result<StrokePageResult> {
        let mut pending = self
            .pending_input
            .take()
            .ok_or(Error::InvalidArgument("no pending stroke input"))?;
        let mut stroke = self.stroke.take().ok_or(Error::NoStroke)?;
        let result = self.execute_stroke_page(&mut stroke, &mut pending, budget);
        match result {
            Err(error) => {
                self.restore_stroke(&stroke);
                Err(error)
            }
            Ok(false) => {
                self.stroke = Some(stroke);
                self.pending_input = Some(pending);
                Ok(StrokePageResult::Pending)
            }
            Ok(true) => {
                self.stroke = Some(stroke);
                if matches!(pending.kind, Kind::Finish(_)) {
                    let (changed, wet_ink) = self.finish_stroke_with_end(false)?;
                    Ok(StrokePageResult::StrokeComplete { changed, wet_ink })
                } else {
                    self.publish_outline()?;
                    Ok(StrokePageResult::InputComplete)
                }
            }
        }
    }
    fn execute_stroke_page(
        &mut self,
        stroke: &mut Stroke,
        pending: &mut PendingInput,
        budget: InputScheduleBudget,
    ) -> Result<bool> {
        // Replays are real work too: shrink this page to the transaction's remaining allowance.
        let budget = InputScheduleBudget {
            work: InputScheduleWork {
                dabs: budget
                    .work
                    .dabs
                    .min(crate::MAX_INPUT_SCHEDULE_DABS.saturating_sub(pending.work.dabs)),
                visits: budget
                    .work
                    .visits
                    .min(crate::MAX_INPUT_SCHEDULE_VISITS.saturating_sub(pending.work.visits)),
            },
            events: budget.events,
        };
        if stroke.taper.as_ref().is_some_and(|t| t.pending_render()) {
            let (complete, changed, work) = self.continue_taper_render(stroke, budget)?;
            pending.charge(work)?;
            if complete && changed {
                self.bump_revision();
            }
            return Ok(complete && pending.dispatched);
        }
        if pending.dispatched {
            return Ok(true);
        }
        if let Some(tail) = &mut stroke.taper {
            tail.reset_work();
        }
        let mut changed = false;
        let used;
        match &mut pending.kind {
            Kind::Input(schedule) => {
                let page = schedule.prepare_page(budget)?;
                if page.complete() && page.events().is_empty() {
                    schedule.adopt(page)?;
                    pending.dispatched = true;
                    return Ok(true);
                }
                used = page.work();
                for &event in page.events() {
                    changed |= self.execute_input_event(stroke, event)?;
                }
                let complete = page.complete();
                schedule.adopt(page)?;
                pending.dispatched = complete;
            }
            Kind::Finish(events) => {
                let (primary, secondary) = Self::stroke_input_work(
                    stroke.brush,
                    &stroke.program,
                    stroke.wet.as_ref().map_or(0, |w| w.work_per_dab),
                );
                let mut work = InputScheduleWork::default();
                let mut count = 0;
                while let Some(&event) = events.front() {
                    let cost = match event {
                        InputScheduleEvent::Dab {
                            stream: InputScheduleStream::Primary,
                            ..
                        } => primary,
                        _ => secondary,
                    };
                    let next = work.combined(cost);
                    if !budget.fits(next) || count == budget.events {
                        if count == 0 {
                            return Err(Error::ResourceLimit(
                                "single final dab exceeds page budget",
                            ));
                        }
                        break;
                    }
                    changed |= self.execute_input_event(stroke, event)?;
                    events.pop_front();
                    work = next;
                    count += 1;
                }
                used = work;
                pending.dispatched = events.is_empty();
            }
        }
        pending.charge(used)?;
        if changed {
            self.bump_revision();
        }
        let finish = matches!(pending.kind, Kind::Finish(_)) && pending.dispatched;
        if self.prepare_taper_render(stroke, finish, false)? {
            return Ok(false);
        }
        Ok(pending.dispatched)
    }
    fn execute_input_event(
        &mut self,
        stroke: &mut Stroke,
        event: InputScheduleEvent,
    ) -> Result<bool> {
        match event {
            InputScheduleEvent::Dab {
                stream,
                point,
                path_distance,
                direction,
            } => {
                stroke.dab_distance = path_distance;
                stroke.path_distance = path_distance;
                stroke.direction = direction;
                if stroke.initial_direction.is_none() {
                    stroke.initial_direction = direction;
                }
                match stream {
                    InputScheduleStream::Primary => {
                        let changed = self.dab(stroke, point)?;
                        stroke.last_dab = point;
                        Ok(changed)
                    }
                    InputScheduleStream::Secondary => {
                        self.secondary_dab(stroke, point)?;
                        Ok(false)
                    }
                }
            }
            InputScheduleEvent::SegmentEnd {
                point,
                path_distance,
                primary_next,
                secondary_next,
                direction,
            } => {
                stroke.last = point;
                stroke.path_distance = path_distance;
                stroke.dab_distance = path_distance;
                stroke.distance_to_next = primary_next;
                stroke.dual_next = secondary_next;
                stroke.direction = direction;
                if stroke.initial_direction.is_none() {
                    stroke.initial_direction = direction;
                }
                Ok(false)
            }
        }
    }
    pub(super) fn validate_input_tool(stroke: &Stroke, point: InputPoint) -> Result<()> {
        if stroke.wet.is_some() && point.tool == Tool::Eraser {
            return Err(Error::InvalidArgument("wet eraser input is not supported"));
        }
        Ok(())
    }
    pub(super) fn stationary_primary(stroke: &Stroke, last: InputPoint, point: InputPoint) -> bool {
        (point.x - last.x).hypot(point.y - last.y) <= 1e-9
            && point.pressure != last.pressure
            && ((stroke.wet.is_some()
                && (stroke.program.tip.pressure_size || stroke.program.tip.pressure_flow))
                || stroke.program.opacity_pressure(point.pressure).is_some()
                || stroke.program.dual.as_ref().is_some_and(|d| {
                    d.pressure_changed(last.pressure, point.pressure)
                        || (d.color.enabled()
                            && stroke.program.opacity_pressure(last.pressure)
                                != stroke.program.opacity_pressure(point.pressure))
                }))
            && !matches!(stroke.brush.mode, BrushMode::Smudge { .. })
    }
    pub(super) fn stationary_secondary(
        stroke: &Stroke,
        last: InputPoint,
        point: InputPoint,
    ) -> bool {
        (point.x - last.x).hypot(point.y - last.y) <= 1e-9
            && stroke.program.dual.as_ref().is_some_and(|d| {
                d.step().is_some()
                    && (d.pressure_changed(last.pressure, point.pressure)
                        || (d.color.enabled()
                            && stroke.program.opacity_pressure(last.pressure)
                                != stroke.program.opacity_pressure(point.pressure)))
            })
    }
}

#[cfg(test)]
#[path = "document_input_paging_tests.rs"]
mod tests;
