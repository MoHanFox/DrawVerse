//! Reversible active-tail execution over the existing Document dab operators.
use super::*;
use crate::{StrokeTaper, TaperBuffer, TaperFactors, TaperRecord, TaperStream, MAX_TAPER_RECORDS};

type SavedTiles = BTreeMap<TileCoord, Option<Arc<Tile>>>;
#[derive(Debug, Default)]
struct TileJournal {
    layer: SavedTiles,
    primary: SavedTiles,
    secondary: SavedTiles,
    stamp: SavedTiles,
    wet_primary: SavedTiles,
    wet_secondary: SavedTiles,
}
#[derive(Debug)]
struct DabCheckpoint {
    last_dab: InputPoint,
    last_carrier: InputPoint,
    random_index: u64,
    random_child: u32,
    dual_index: u64,
    dual_last: InputPoint,
    airbrush_last_ns: u64,
    airbrush_tick: bool,
    speed: f32,
    shape_sample: crate::brush_shape_jitter::ShapeSample,
    color_sample: Pixel,
    wet: Option<wet::WetStroke>,
    direction: Option<f64>,
    mask_paint: Option<(f32, f32)>,
    first_dab: bool,
    coverage_erase: bool,
    coverage_tool_stable: bool,
}
impl DabCheckpoint {
    fn capture(s: &Stroke) -> Self {
        Self {
            last_dab: s.last_dab,
            last_carrier: s.last_carrier,
            random_index: s.random_index,
            random_child: s.random_child,
            dual_index: s.dual_index,
            dual_last: s.dual_last,
            airbrush_last_ns: s.airbrush_last_ns,
            airbrush_tick: s.airbrush_tick,
            speed: s.speed,
            shape_sample: s.shape_sample,
            color_sample: s.color_sample,
            wet: s.wet.clone(),
            direction: s.direction,
            mask_paint: s.mask_paint,
            first_dab: s.first_dab,
            coverage_erase: s.coverage_erase,
            coverage_tool_stable: s.coverage_tool_stable,
        }
    }
    fn restore(self, s: &mut Stroke) {
        s.last_dab = self.last_dab;
        s.last_carrier = self.last_carrier;
        s.random_index = self.random_index;
        s.random_child = self.random_child;
        s.dual_index = self.dual_index;
        s.dual_last = self.dual_last;
        s.airbrush_last_ns = self.airbrush_last_ns;
        s.airbrush_tick = self.airbrush_tick;
        s.speed = self.speed;
        s.shape_sample = self.shape_sample;
        s.color_sample = self.color_sample;
        s.wet = self.wet;
        s.direction = self.direction;
        s.mask_paint = self.mask_paint;
        s.first_dab = self.first_dab;
        s.coverage_erase = self.coverage_erase;
        s.coverage_tool_stable = self.coverage_tool_stable;
    }
}
#[derive(Clone, Copy, Debug)]
struct Event {
    record: TaperRecord,
    direction: Option<f64>,
}
#[derive(Debug)]
pub(super) struct TailReplay {
    buffer: TaperBuffer,
    checkpoint: Option<DabCheckpoint>,
    journal: TileJournal,
    pending: Vec<Event>,
    queued: Vec<Event>,
    next_id: u64,
    inflight: Option<TailRender>,
    rendering: bool,
    recording: bool,
    work_primary: usize,
    work_secondary: usize,
    #[cfg(test)]
    reference: Option<(crate::CompiledTaper, f64)>,
}
#[derive(Debug)]
struct TailRender {
    update: crate::TaperUpdate,
    events: Vec<Event>,
    position: usize,
    retired: usize,
    finish: bool,
    direction: Option<f64>,
    changed: bool,
}
impl TailReplay {
    pub(super) fn pending_render(&self) -> bool {
        self.inflight.is_some()
    }
    pub(super) fn reset_work(&mut self) {
        self.work_primary = 0;
        self.work_secondary = 0;
    }
}
impl Stroke {
    pub(super) fn initialize_taper(&mut self, settings: Option<&StrokeTaper>) -> Result<()> {
        let Some(settings) =
            settings.filter(|s| s.enabled() && (s.start().length() > 0. || s.end().length() > 0.))
        else {
            return Ok(());
        };
        self.taper = Some(TailReplay {
            buffer: TaperBuffer::new(settings, MAX_TAPER_RECORDS)?,
            checkpoint: Some(DabCheckpoint::capture(self)),
            journal: TileJournal::default(),
            pending: Vec::new(),
            queued: Vec::new(),
            next_id: 0,
            inflight: None,
            rendering: false,
            recording: false,
            work_primary: 0,
            work_secondary: 0,
            #[cfg(test)]
            reference: None,
        });
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn taper_reference(
        &mut self,
        settings: Option<&StrokeTaper>,
        endpoint: Option<f64>,
    ) -> Result<()> {
        if let (Some(tail), Some(settings), Some(endpoint)) = (&mut self.taper, settings, endpoint)
        {
            // The reference freezes every command immediately and uses a known
            // final endpoint, so it has neither a replaceable tail nor tile restore.
            tail.reference = Some((settings.compile(), endpoint));
            tail.buffer = TaperBuffer::new(&StrokeTaper::default(), MAX_TAPER_RECORDS)?;
        }
        Ok(())
    }
    pub(super) fn queue_taper(&mut self, point: InputPoint, stream: TaperStream) -> Result<bool> {
        let Some(tail) = self.taper.as_mut().filter(|t| !t.rendering) else {
            return Ok(false);
        };
        if tail.queued.len() >= MAX_TAPER_RECORDS {
            return Err(Error::ResourceLimit("taper dispatch batch"));
        }
        let next = tail
            .next_id
            .checked_add(1)
            .ok_or(Error::ResourceLimit("taper dispatch identity"))?;
        // The original two-stream scheduler gives the secondary precedence for
        // positions within 1e-9. Canonicalize only this subpixel tie's arc value;
        // preserve its point, pressure and dispatch order without rejecting it.
        let previous = tail
            .queued
            .last()
            .map_or(tail.buffer.endpoint(), |e| e.record.distance);
        if self.dab_distance + 1e-8 < previous {
            return Err(Error::InvalidArgument("taper dispatch distance order"));
        }
        tail.queued.push(Event {
            record: TaperRecord {
                id: tail.next_id,
                distance: self.dab_distance.max(previous),
                stream,
                point,
            },
            direction: self.direction,
        });
        tail.next_id = next;
        Ok(true)
    }
}

#[cfg(test)]
#[path = "document_taper_tests.rs"]
mod tests;
fn save(
    map: &mut SavedTiles,
    coord: TileCoord,
    value: Option<Arc<Tile>>,
    limit: usize,
) -> Result<()> {
    if !map.contains_key(&coord) && map.len() >= limit {
        return Err(Error::ResourceLimit("taper checkpoint tile budget"));
    }
    map.entry(coord).or_insert(value);
    Ok(())
}
pub(super) fn capture_field(
    tail: Option<&mut TailReplay>,
    secondary: bool,
    coord: TileCoord,
    field: &BTreeMap<TileCoord, Arc<Tile>>,
    limit: usize,
) -> Result<()> {
    if let Some(tail) = tail.filter(|t| t.recording) {
        let journal = &mut tail.journal;
        save(
            if secondary {
                &mut journal.secondary
            } else {
                &mut journal.primary
            },
            coord,
            field.get(&coord).cloned(),
            limit,
        )?;
    }
    Ok(())
}
pub(super) fn capture_wet_field(
    tail: Option<&mut TailReplay>,
    secondary: bool,
    coord: TileCoord,
    field: &BTreeMap<TileCoord, Arc<Tile>>,
    limit: usize,
) -> Result<()> {
    if let Some(tail) = tail.filter(|t| t.recording) {
        save(
            if secondary {
                &mut tail.journal.wet_secondary
            } else {
                &mut tail.journal.wet_primary
            },
            coord,
            field.get(&coord).cloned(),
            limit,
        )?;
    }
    Ok(())
}
fn restore_field(field: &mut BTreeMap<TileCoord, Arc<Tile>>, saved: SavedTiles) {
    for (coord, tile) in saved {
        if let Some(tile) = tile {
            field.insert(coord, tile);
        } else {
            field.remove(&coord);
        }
    }
}
impl Document {
    pub(super) fn capture_taper_tile(
        &self,
        layer: LayerId,
        tail: &mut Option<TailReplay>,
        primary: &BTreeMap<TileCoord, Arc<Tile>>,
        secondary: &BTreeMap<TileCoord, Arc<Tile>>,
        stamp: &BTreeMap<TileCoord, Arc<Tile>>,
        coord: TileCoord,
    ) -> Result<()> {
        if let Some(tail) = tail.as_mut().filter(|t| t.recording) {
            let journal = &mut tail.journal;
            let layer = self.layer(layer).expect("stroke layer");
            let limit = self.options.max_document_tiles;
            save(
                &mut journal.layer,
                coord,
                layer.tiles.get(&coord).cloned(),
                limit,
            )?;
            save(
                &mut journal.primary,
                coord,
                primary.get(&coord).cloned(),
                limit,
            )?;
            save(
                &mut journal.secondary,
                coord,
                secondary.get(&coord).cloned(),
                limit,
            )?;
            save(&mut journal.stamp, coord, stamp.get(&coord).cloned(), limit)?;
        }
        Ok(())
    }
    pub(super) fn render_taper(&mut self, stroke: &mut Stroke, finish: bool) -> Result<bool> {
        if !self.prepare_taper_render(stroke, finish, true)? {
            return Ok(false);
        }
        loop {
            let (complete, changed, _) =
                self.continue_taper_render(stroke, crate::InputScheduleBudget::default())?;
            if complete {
                return Ok(changed);
            }
        }
    }
    pub(super) fn prepare_taper_render(
        &mut self,
        stroke: &mut Stroke,
        finish: bool,
        legacy: bool,
    ) -> Result<bool> {
        let Some(mut tail) = stroke.taper.take() else {
            return Ok(false);
        };
        let records: Vec<_> = tail.queued.iter().map(|e| e.record).collect();
        // Append once, then freeze the remaining tail on finish. The append plan is
        // sufficient for rendering; finishing changes no endpoint or factors.
        let update = tail.buffer.prepare_append(&records, stroke.path_distance)?;
        let retired = update.retired().len();
        let has_replay = update.replay().len() != 0;
        if !has_replay {
            tail.buffer.commit(update)?;
            if finish {
                let done = tail.buffer.prepare_finish(stroke.path_distance)?;
                tail.buffer.commit(done)?;
            }
            stroke.taper = Some(tail);
            return Ok(false);
        }
        let events: Vec<_> = std::mem::take(&mut tail.pending)
            .into_iter()
            .chain(std::mem::take(&mut tail.queued))
            .collect();
        debug_assert_eq!(events.len(), update.replay().len());
        let primary = events
            .iter()
            .filter(|e| e.record.stream == TaperStream::Primary)
            .count();
        if legacy {
            tail.work_primary = tail.work_primary.saturating_add(primary);
            tail.work_secondary = tail.work_secondary.saturating_add(events.len() - primary);
            Self::validate_stroke_work(
                stroke.brush,
                &stroke.program,
                tail.work_primary,
                tail.work_secondary,
                stroke.wet.as_ref().map_or(0, |w| w.work_per_dab),
            )?;
        }
        let scheduled_direction = stroke.direction;
        let index = self.layer_index(stroke.layer)?;
        let journal = std::mem::take(&mut tail.journal);
        let restored = !journal.layer.is_empty();
        for (coord, tile) in journal.layer {
            Self::restore_tile(&mut self.layers[index], coord, tile);
            if self.layers[index].appearance.offset_x != 0
                || self.layers[index].appearance.offset_y != 0
            {
                self.dirty.all = true;
                self.dirty.tiles.clear();
            } else {
                self.dirty_tile(coord);
            }
        }
        restore_field(&mut stroke.coverage, journal.primary);
        restore_field(&mut stroke.dual_coverage, journal.secondary);
        restore_field(&mut stroke.stamp_field, journal.stamp);
        restore_field(&mut stroke.wet_primary_field, journal.wet_primary);
        restore_field(&mut stroke.wet_secondary_field, journal.wet_secondary);
        tail.checkpoint
            .take()
            .expect("taper checkpoint")
            .restore(stroke);
        tail.rendering = true;
        tail.recording = false;

        tail.inflight = Some(TailRender {
            update,
            events,
            position: 0,
            retired,
            finish,
            direction: scheduled_direction,
            changed: restored,
        });
        stroke.taper = Some(tail);
        Ok(true)
    }
    pub(super) fn continue_taper_render(
        &mut self,
        stroke: &mut Stroke,
        budget: crate::InputScheduleBudget,
    ) -> Result<(bool, bool, crate::InputScheduleWork)> {
        let mut render = stroke
            .taper
            .as_mut()
            .and_then(|t| t.inflight.take())
            .ok_or(Error::InvalidArgument("no taper replay page"))?;
        let (primary, secondary) = Self::stroke_input_work(
            stroke.brush,
            &stroke.program,
            stroke.wet.as_ref().map_or(0, |w| w.work_per_dab),
        );
        let mut work = crate::InputScheduleWork::default();
        let mut count = 0;
        while render.position < render.events.len() && count < budget.events {
            let event = render.events[render.position];
            let next = work.combined(if event.record.stream == TaperStream::Primary {
                primary
            } else {
                secondary
            });
            if !budget.fits(next) {
                if count == 0 {
                    return Err(Error::ResourceLimit("single taper dab exceeds page budget"));
                }
                break;
            }
            work = next;
            count += 1;
            if render.position == render.retired {
                let checkpoint = DabCheckpoint::capture(stroke);
                let tail = stroke.taper.as_mut().expect("taper");
                tail.checkpoint = Some(checkpoint);
                tail.recording = true;
            }
            let factors = render
                .update
                .replay_at(render.position)
                .expect("replay position");
            debug_assert_eq!(event.record, factors.record);
            stroke.direction = event.direction;
            stroke.taper_factors = factors.factors;
            #[cfg(test)]
            if let Some((program, endpoint)) = &stroke.taper.as_ref().expect("taper").reference {
                stroke.taper_factors = program.factors(event.record.distance, *endpoint)?;
            }
            match event.record.stream {
                TaperStream::Primary => {
                    render.changed |= self.dab(stroke, event.record.point)?;
                    stroke.last_dab = event.record.point;
                }
                TaperStream::Secondary => {
                    self.secondary_dab(stroke, event.record.point)?;
                }
            }
            render.position += 1;
        }
        if render.position < render.events.len() {
            let changed = render.changed;
            stroke.taper.as_mut().expect("taper").inflight = Some(render);
            return Ok((false, changed, work));
        }
        let checkpoint =
            (render.retired == render.events.len()).then(|| DabCheckpoint::capture(stroke));
        stroke.direction = render.direction;
        stroke.taper_factors = TaperFactors {
            diameter: 1.,
            opacity: 1.,
        };
        let tail = stroke.taper.as_mut().expect("taper");
        if let Some(checkpoint) = checkpoint {
            tail.checkpoint = Some(checkpoint);
        }
        tail.pending = render.events.into_iter().skip(render.retired).collect();
        tail.buffer.commit(render.update)?;
        if render.finish {
            let done = tail.buffer.prepare_finish(stroke.path_distance)?;
            tail.buffer.commit(done)?;
            tail.pending.clear();
            tail.journal = TileJournal::default();
        }
        tail.rendering = false;
        tail.recording = false;
        if render.changed && render.finish {
            self.bump_revision();
        }
        Ok((true, render.changed, work))
    }
}
