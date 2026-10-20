//! Optional computation accounting; independent readers never borrow a Document.
use super::*;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ComputationPhase {
    #[default]
    Preparing = 0,
    Resampling = 1,
    Selection = 2,
    Comparing = 3,
    History = 4,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComputationSnapshot {
    pub phase: ComputationPhase,
    pub stage: u64,
    pub completed: u64,
    /// Zero denotes unknown work, never an estimated total.
    pub total: u64,
}
#[derive(Debug, Default)]
pub struct ComputationProgress {
    state: Mutex<ComputationSnapshot>,
}
impl ComputationProgress {
    pub fn snapshot(&self) -> ComputationSnapshot {
        *self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub(crate) fn stage(&self, phase: ComputationPhase, total: usize) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        *state = ComputationSnapshot {
            phase,
            stage: state.stage.saturating_add(1),
            completed: 0,
            total: total as u64,
        };
    }
    pub(crate) fn advance(&self, count: usize) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.completed = state
            .completed
            .saturating_add(count as u64)
            .min(state.total);
    }
}
impl Document {
    /// Attach only while executing one exclusive task; progress is not document state.
    pub fn set_computation_progress(
        &mut self,
        progress: Option<Arc<ComputationProgress>>,
    ) -> Option<Arc<ComputationProgress>> {
        std::mem::replace(&mut self.computation_progress, progress)
    }
    pub(super) fn progress_stage(&self, phase: ComputationPhase, total: usize) {
        if let Some(progress) = &self.computation_progress {
            progress.stage(phase, total);
        }
    }
    pub(super) fn progress_advance(&self, count: usize) {
        if let Some(progress) = &self.computation_progress {
            progress.advance(count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seed() -> Document {
        let mut d = Document::new(129, 77).unwrap();
        d.initialize_white_background().unwrap();
        d.begin_stroke(
            Brush {
                radius: 12.,
                color: Pixel::from_straight([0.7, 0.1, 0.2, 0.6]).unwrap(),
                ..Default::default()
            },
            InputPoint::new(60., 38., 1.),
        )
        .unwrap();
        d.end_stroke().unwrap();
        d
    }
    fn pixels(d: &Document) -> Vec<Pixel> {
        (0..77)
            .flat_map(|y| d.snapshot().read_row(0, y, 129).unwrap())
            .collect()
    }
    fn prepare(d: &mut Document, kind: u8) {
        if kind == 1 {
            let id = d.active_layer();
            d.begin_transform_layers_deferred(&[id]).unwrap();
            let mut g = d.transform_geometry().unwrap();
            for p in &mut g.points {
                p[0] += 9.;
                p[1] -= 3.;
            }
            g.interpolation = 3;
            d.update_transform(g).unwrap();
        }
        if kind == 2 {
            d.begin_liquify_deferred().unwrap();
            d.begin_liquify_stroke().unwrap();
            d.liquify_dab(LiquifyDab {
                tool: 0,
                radius: 48.,
                strength: 0.8,
                density: 0.6,
                speed: 1.,
                pressure_enabled: false,
                from: InputPoint::new(60., 38., 1.),
                to: InputPoint::new(82., 42., 1.),
                elapsed_ms: 16.,
            })
            .unwrap();
            d.end_liquify_stroke().unwrap();
        }
    }
    fn finish(d: &mut Document, kind: u8) -> Result<()> {
        match kind {
            0 => d.flip_image(true),
            1 => d.commit_transform().map(|_| ()),
            _ => d.commit_liquify().map(|_| ()),
        }
    }
    #[test]
    fn task_accounting_keeps_pixels_history_and_resource_failure_atomic() {
        for kind in 0..3 {
            let mut reference = seed();
            let mut reported = seed();
            prepare(&mut reference, kind);
            prepare(&mut reported, kind);
            let progress = Arc::new(ComputationProgress::default());
            reported.set_computation_progress(Some(progress.clone()));
            finish(&mut reference, kind).unwrap();
            finish(&mut reported, kind).unwrap();
            assert_eq!(pixels(&reported), pixels(&reference));
            assert_eq!(reported.history_depth(), reference.history_depth());
            let snapshot = progress.snapshot();
            assert!(snapshot.stage > 0);
            assert_eq!(snapshot.phase, ComputationPhase::History);
            assert_eq!(snapshot.total, 0);
            reported.undo().unwrap();
            reference.undo().unwrap();
            assert_eq!(pixels(&reported), pixels(&reference));
            reported.redo().unwrap();
            reference.redo().unwrap();
            assert_eq!(pixels(&reported), pixels(&reference));
            let mut failed = seed();
            prepare(&mut failed, kind);
            let before = pixels(&failed);
            let depth = failed.history_depth();
            let revision = failed.revision();
            failed.options.max_document_tiles = 0;
            let progress = Arc::new(ComputationProgress::default());
            failed.set_computation_progress(Some(progress.clone()));
            assert!(
                matches!(finish(&mut failed, kind), Err(Error::ResourceLimit(_))),
                "kind {kind}"
            );
            assert_eq!(pixels(&failed), before);
            assert_eq!(failed.history_depth(), depth);
            assert_eq!(failed.revision(), revision);
            if kind == 1 {
                assert!(failed.transform_active());
            }
            if kind == 2 {
                assert!(failed.liquify_active());
            }
            let snapshot = progress.snapshot();
            assert!(snapshot.completed <= snapshot.total);
            assert_ne!(snapshot.phase, ComputationPhase::History);
        }
    }
}
