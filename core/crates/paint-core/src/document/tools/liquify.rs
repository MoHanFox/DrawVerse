use super::*;
const MAX_FIELD_TILES: usize = 8192;
const MAX_VISITS: usize = 8_000_000;
type Field = BTreeMap<TileCoord, Arc<Vec<[f32; 2]>>>;
#[derive(Clone, Debug)]
pub(super) struct LiquifyState {
    original: Layer,
    field: Field,
    cell: i64,
    sources: Arc<std::sync::Mutex<BTreeMap<TileCoord, paint_ops::SourceTile>>>,
    focus: bool,
    preview_source: Option<Arc<LiquifyPreviewSource>>,
    checkpoint: Option<Arc<Checkpoint>>,
    history: Arc<LocalHistory>,
}
#[derive(Clone, Debug)]
struct Checkpoint {
    layer: Layer,
    field: Field,
    cell: i64,
    action: crate::HistoryAction,
}
#[derive(Debug)]
struct LocalStep {
    pixels: Vec<TileChange>,
    fields: crate::history_storage::Payload,
    cells: [i64; 2],
    action: crate::HistoryAction,
}
#[derive(Clone, Debug, Default)]
struct LocalHistory {
    undo: std::collections::VecDeque<Arc<LocalStep>>,
    redo: Vec<Arc<LocalStep>>,
    evicted: bool,
}
fn same_field(a: Option<&Arc<Vec<[f32; 2]>>>, b: Option<&Arc<Vec<[f32; 2]>>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a == b,
        _ => false,
    }
}

#[cfg(test)]
mod local_history_tests {
    use super::*;
    #[test]
    fn spilled_history_failure_preserves_pending_preview_and_can_retry() {
        let mut d = Document::with_options(
            96,
            96,
            DocumentOptions {
                max_history_bytes: 2 * TILE_BYTES + 256,
                ..Default::default()
            },
        )
        .unwrap();
        d.begin_stroke(
            Brush {
                radius: 9.,
                ..Default::default()
            },
            InputPoint::new(40., 40., 1.),
        )
        .unwrap();
        d.end_stroke().unwrap();
        let image = |d: &Document| {
            (0..96)
                .flat_map(|y| (0..96).map(move |x| d.pixel(x, y)))
                .collect::<Vec<_>>()
        };
        let original = image(&d);
        let formal = d.history_depth();
        d.begin_liquify().unwrap();
        d.begin_liquify_stroke().unwrap();
        d.liquify_dab(LiquifyDab {
            tool: 0,
            radius: 24.,
            strength: 0.5,
            density: 1.,
            speed: 1.,
            pressure_enabled: false,
            from: InputPoint::new(40., 40., 1.),
            to: InputPoint::new(48., 44., 1.),
            elapsed_ms: 100.,
        })
        .unwrap();
        let preview = image(&d);
        crate::history_storage::fail_writes_for_test(true);
        let result = d.end_liquify_stroke();
        crate::history_storage::fail_writes_for_test(false);
        assert!(matches!(result, Err(Error::HistoryStorage(_))));
        assert_eq!(image(&d), preview);
        assert_eq!(d.liquify_history_depth(), Some((0, 0)));
        assert!(d.end_liquify_stroke().unwrap());
        assert!(
            d.liquify.as_ref().unwrap().history.undo[0]
                .fields
                .disk_bytes()
                > 0
        );
        d.undo_liquify().unwrap();
        assert_eq!(image(&d), original);
        d.redo_liquify().unwrap();
        assert_eq!(image(&d), preview);
        d.cancel_liquify().unwrap();
        assert_eq!(d.history_depth(), formal);
        assert_eq!(image(&d), original);
    }
}
#[derive(Clone, Copy, Debug)]
pub struct LiquifyDab {
    /// 0 push, 1 bloat, 2 clockwise twirl, 3 reconstruct.
    pub tool: u32,
    pub radius: f32,
    pub strength: f32,
    pub density: f32,
    pub speed: f32,
    pub pressure_enabled: bool,
    pub from: InputPoint,
    pub to: InputPoint,
    pub elapsed_ms: f32,
}
impl LiquifyDab {
    pub fn validate(self) -> Result<()> {
        self.from.validate()?;
        self.to.validate()?;
        if self.tool > 3
            || !self.radius.is_finite()
            || !(0.5..=7500.).contains(&self.radius)
            || [self.strength, self.density, self.speed]
                .iter()
                .any(|v| !v.is_finite() || !(0.01..=1.).contains(v))
            || !self.elapsed_ms.is_finite()
            || !(0.0..=250.).contains(&self.elapsed_ms)
        {
            return Err(Error::InvalidArgument("liquify parameters"));
        }
        Ok(())
    }
}
fn coord(x: i64, y: i64) -> TileCoord {
    TileCoord::from_signed(x.div_euclid(64) as i32, y.div_euclid(64) as i32)
}
fn index(x: i64, y: i64) -> usize {
    (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize
}
impl Document {
    pub fn liquify_active(&self) -> bool {
        self.liquify.is_some()
    }
    pub fn set_liquify_focus(&mut self, focus: bool) -> Result<()> {
        let state = self
            .liquify
            .as_mut()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?;
        if state.focus != focus {
            state.focus = focus;
            self.dirty_all();
        }
        Ok(())
    }
    /// Display-only visibility overrides; document metadata and history remain unchanged.
    pub fn liquify_snapshot(&self) -> DocumentSnapshot {
        let mut snapshot = self.snapshot();
        if let Some(state) = self.liquify.as_ref().filter(|s| s.focus) {
            let mut id = state.original.id;
            if state.original.mask {
                id = state.original.parent;
            }
            let mut visible = BTreeSet::from([id]);
            let mut parent = id;
            while let Some(layer) = self.layer(parent) {
                if layer.parent == 0 {
                    break;
                }
                parent = layer.parent;
                visible.insert(parent);
            }
            for layer in &mut snapshot.layers {
                layer.properties.visible =
                    visible.contains(&layer.id) || layer.mask && visible.contains(&layer.parent);
            }
        }
        snapshot
    }
    pub fn begin_liquify(&mut self) -> Result<()> {
        self.begin_liquify_internal(true)
    }
    fn begin_liquify_internal(&mut self, warm: bool) -> Result<()> {
        self.idle()?;
        self.check_ancestors(
            self.active_layer,
            crate::LOCK_ALL | crate::LOCK_POSITION | crate::LOCK_TRANSPARENCY,
        )?;
        let original = self
            .layer(self.active_layer)
            .ok_or(Error::LayerNotFound(self.active_layer))?
            .clone();
        if original.is_image() {
            return Err(Error::InvalidArgument("rasterize image before liquify"));
        }
        if original.group {
            return Err(Error::InvalidArgument("liquify needs raster layer or mask"));
        }
        let mut state = LiquifyState {
            original,
            field: Field::new(),
            cell: 2,
            sources: Arc::default(),
            focus: false,
            preview_source: None,
            checkpoint: None,
            history: Arc::default(),
        };
        if warm {
            self.warm_liquify_sources(&mut state)?;
        }
        self.liquify = Some(state);
        Ok(())
    }
    fn restore_liquify_original(&mut self) -> Result<()> {
        let state = self
            .liquify
            .as_mut()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?;
        let id = state.original.id;
        let original = state.original.clone();
        state.field.clear();
        let at = self.layer_index(id)?;
        self.layers[at] = original;
        self.dirty_all();
        Ok(())
    }
    pub fn reset_liquify(&mut self) -> Result<()> {
        self.end_liquify_stroke()?;
        self.begin_liquify_stroke()?;
        if let Some(state) = self.liquify.as_mut() {
            Arc::make_mut(state.checkpoint.as_mut().expect("checkpoint")).action =
                crate::HistoryAction::LiquifyReset;
        }
        self.restore_liquify_original()?;
        self.end_liquify_stroke()?;
        Ok(())
    }
    pub fn cancel_liquify(&mut self) -> Result<()> {
        self.restore_liquify_original()?;
        self.liquify = None;
        Ok(())
    }
    pub fn commit_liquify(&mut self) -> Result<bool> {
        self.end_liquify_stroke()?;
        let candidate = self.final_liquify_layer()?;
        let state = self
            .liquify
            .as_ref()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?;
        let current = &candidate;
        let keys: BTreeSet<_> = current
            .tiles
            .keys()
            .chain(state.original.tiles.keys())
            .copied()
            .collect();
        let mut changes = Vec::new();
        let mut unchanged = Vec::new();
        self.progress_stage(ComputationPhase::Comparing, keys.len());
        for key in keys {
            let before = state.original.tiles.get(&key);
            let after = current.tiles.get(&key);
            let equal = match (before, after) {
                (None, None) => true,
                (Some(a), Some(b)) => {
                    Arc::ptr_eq(a, b) || a.try_pixels()?.as_ref() == b.try_pixels()?.as_ref()
                }
                (Some(tile), None) | (None, Some(tile)) => {
                    tile.try_pixels()?.iter().enumerate().all(|(i, p)| {
                        *p == state.original.default_pixel(
                            i64::from(key.signed_x()) * 64 + (i % 64) as i64,
                            i64::from(key.signed_y()) * 64 + (i / 64) as i64,
                        )
                    })
                }
            };
            if !equal {
                changes.push(TileChange {
                    coord: key,
                    before: before.cloned(),
                    after: after.cloned(),
                });
            } else {
                unchanged.push(key);
            }
            self.progress_advance(1);
        }
        let original = state.original.clone();
        let focused = state.focus;
        let changed = !changes.is_empty();
        if changed {
            self.progress_stage(ComputationPhase::History, 0);
            self.commit_with_action(
                Command::Stroke {
                    layer: original.id,
                    changes,
                },
                crate::HistoryAction::Liquify,
            )?;
        }
        let at = self.layer_index(original.id)?;
        self.layers[at] = candidate;
        for key in unchanged {
            if let Some(tile) = original.tiles.get(&key) {
                self.layers[at].tiles.insert(key, tile.clone());
            } else {
                self.layers[at].tiles.remove(&key);
            }
        }
        self.liquify = None;
        if focused || changed {
            self.dirty_all();
        }
        Ok(changed)
    }
    pub fn begin_liquify_stroke(&mut self) -> Result<()> {
        let state = self
            .liquify
            .as_ref()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?;
        if state.checkpoint.is_some() {
            return Err(Error::Busy);
        }
        let layer = self
            .layer(state.original.id)
            .expect("liquify layer")
            .clone();
        let checkpoint = Checkpoint {
            layer,
            field: state.field.clone(),
            cell: state.cell,
            action: crate::HistoryAction::Liquify,
        };
        self.liquify.as_mut().expect("state").checkpoint = Some(Arc::new(checkpoint));
        Ok(())
    }
    pub fn end_liquify_stroke(&mut self) -> Result<bool> {
        let state = self
            .liquify
            .as_ref()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?;
        let Some(before) = state.checkpoint.as_ref() else {
            return Ok(false);
        };
        let current = self.layer(state.original.id).expect("liquify layer");
        let keys: BTreeSet<_> = before
            .field
            .keys()
            .chain(state.field.keys())
            .copied()
            .collect();
        let changed: Vec<_> = keys
            .into_iter()
            .filter(|key| {
                before.cell != state.cell
                    || !same_field(before.field.get(key), state.field.get(key))
            })
            .collect();
        if changed.is_empty() {
            self.liquify.as_mut().expect("state").checkpoint = None;
            return Ok(false);
        }
        let pixel_keys: BTreeSet<_> = before
            .layer
            .tiles
            .keys()
            .chain(current.tiles.keys())
            .copied()
            .collect();
        let pixels = pixel_keys
            .iter()
            .filter(
                |key| match (before.layer.tiles.get(key), current.tiles.get(key)) {
                    (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
                    (None, None) => false,
                    _ => true,
                },
            )
            .map(|key| TileChange {
                coord: *key,
                before: before.layer.tiles.get(key).cloned(),
                after: current.tiles.get(key).cloned(),
            })
            .collect();
        let fields = crate::history_storage::Payload::blob_in(
            self.options.max_history_bytes / self.options.max_history_commands,
            self.storage(),
            |writer| {
                writer.write_all(&(changed.len() as u32).to_le_bytes())?;
                for key in &changed {
                    writer.write_all(&key.x.to_le_bytes())?;
                    writer.write_all(&key.y.to_le_bytes())?;
                    for field in [before.field.get(key), state.field.get(key)] {
                        writer.write_all(&[u8::from(field.is_some())])?;
                        if let Some(field) = field {
                            for vector in field.iter() {
                                for value in vector {
                                    writer.write_all(&value.to_le_bytes())?;
                                }
                            }
                        }
                    }
                }
                Ok(())
            },
        )?;
        let step = Arc::new(LocalStep {
            cells: [before.cell, state.cell],
            pixels,
            fields,
            action: before.action,
        });
        let state = self.liquify.as_mut().expect("state");
        let history = Arc::make_mut(&mut state.history);
        history.redo.clear();
        history.undo.push_back(step);
        while history.undo.len() > self.options.max_history_commands
            || history
                .undo
                .iter()
                .map(|s| s.fields.disk_bytes())
                .sum::<u64>()
                > crate::history::MAX_DISK_HISTORY_BYTES
        {
            history.undo.pop_front();
            history.evicted = true;
        }
        state.checkpoint = None;
        Ok(true)
    }
    pub fn liquify_history_depth(&self) -> Option<(usize, usize)> {
        self.liquify
            .as_ref()
            .map(|s| (s.history.undo.len(), s.history.redo.len()))
    }
    pub fn liquify_history_actions(&self) -> Option<Vec<crate::HistoryAction>> {
        self.liquify.as_ref().map(|s| {
            std::iter::once(if s.history.evicted {
                crate::HistoryAction::Truncated
            } else {
                crate::HistoryAction::Initial
            })
            .chain(
                s.history
                    .undo
                    .iter()
                    .chain(s.history.redo.iter().rev())
                    .map(|step| step.action),
            )
            .collect()
        })
    }
    pub fn select_liquify_history(&mut self, depth: usize) -> Result<bool> {
        let backup = self
            .liquify
            .clone()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?;
        if backup.checkpoint.is_some() {
            return Err(Error::Busy);
        }
        let undo = backup.history.undo.len();
        let redo = backup.history.redo.len();
        if depth > undo + redo {
            return Err(Error::InvalidArgument("liquify history depth"));
        }
        if depth == undo {
            return Ok(false);
        }
        let at = self.layer_index(backup.original.id)?;
        let layer = self.layers[at].clone();
        let revision = self.revision;
        let dirty = self.dirty.clone();
        // Existing local steps restore fields and immutable tiles without resampling.
        for _ in 0..depth.abs_diff(undo) {
            if let Err(error) = self.travel_liquify(depth > undo) {
                self.layers[at] = layer;
                self.liquify = Some(backup);
                self.revision = revision;
                self.dirty = dirty;
                return Err(error);
            }
        }
        self.revision = revision.saturating_add(1);
        Ok(true)
    }
    pub fn undo_liquify(&mut self) -> Result<bool> {
        self.travel_liquify(false)
    }
    pub fn redo_liquify(&mut self) -> Result<bool> {
        self.travel_liquify(true)
    }
    fn travel_liquify(&mut self, redo: bool) -> Result<bool> {
        self.end_liquify_stroke()?;
        let state = self
            .liquify
            .as_ref()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?;
        let step = if redo {
            state.history.redo.last()
        } else {
            state.history.undo.back()
        }
        .cloned();
        let Some(step) = step else { return Ok(false) };
        let mut field = state.field.clone();
        step.fields.read_blob(|reader| {
            let mut bytes = [0; 4];
            reader
                .read_exact(&mut bytes)
                .map_err(|e| Error::HistoryStorage(e.to_string()))?;
            let count = u32::from_le_bytes(bytes) as usize;
            if count > MAX_FIELD_TILES * 2 {
                return Err(Error::HistoryStorage(
                    "invalid liquify history count".into(),
                ));
            }
            for _ in 0..count {
                reader
                    .read_exact(&mut bytes)
                    .map_err(|e| Error::HistoryStorage(e.to_string()))?;
                let x = u32::from_le_bytes(bytes);
                reader
                    .read_exact(&mut bytes)
                    .map_err(|e| Error::HistoryStorage(e.to_string()))?;
                let key = TileCoord {
                    x,
                    y: u32::from_le_bytes(bytes),
                };
                for side in 0..2 {
                    let mut present = [0];
                    reader
                        .read_exact(&mut present)
                        .map_err(|e| Error::HistoryStorage(e.to_string()))?;
                    let value = match present[0] {
                        0 => None,
                        1 => {
                            let mut values = vec![[0.; 2]; 4096];
                            for vector in &mut values {
                                for value in vector {
                                    reader
                                        .read_exact(&mut bytes)
                                        .map_err(|e| Error::HistoryStorage(e.to_string()))?;
                                    *value = f32::from_le_bytes(bytes);
                                    if !value.is_finite() || value.abs() > 2_000_000. {
                                        return Err(Error::HistoryStorage(
                                            "invalid liquify history displacement".into(),
                                        ));
                                    }
                                }
                            }
                            Some(Arc::new(values))
                        }
                        _ => {
                            return Err(Error::HistoryStorage(
                                "invalid liquify history flag".into(),
                            ))
                        }
                    };
                    if side == usize::from(redo) {
                        if let Some(value) = value {
                            field.insert(key, value);
                        } else {
                            field.remove(&key);
                        }
                    }
                }
            }
            Ok(())
        })?;
        let id = state.original.id;
        let at = self.layer_index(id)?;
        let mut layer = self.layers[at].clone();
        for change in &step.pixels {
            let value = if redo { &change.after } else { &change.before };
            if let Some(tile) = value {
                layer.tiles.insert(change.coord, tile.clone());
            } else {
                layer.tiles.remove(&change.coord);
            }
        }
        self.layers[at] = layer;
        let state = self.liquify.as_mut().expect("state");
        state.field = field;
        state.cell = step.cells[usize::from(redo)];
        let history = Arc::make_mut(&mut state.history);
        if redo {
            history.redo.pop();
            history.undo.push_back(step);
        } else {
            history.undo.pop_back();
            history.redo.push(step);
        }
        self.dirty_all();
        Ok(true)
    }
    pub fn set_compute_executor(&mut self, executor: Arc<dyn paint_ops::Executor>) -> Result<()> {
        self.idle()?;
        self.compute = executor;
        Ok(())
    }
    pub fn compute_backend_name(&self) -> String {
        self.compute.name()
    }
    pub fn compute_executor(&self) -> Arc<dyn paint_ops::Executor> {
        self.compute.clone()
    }
    pub fn liquify_dab(&mut self, options: LiquifyDab) -> Result<bool> {
        self.elastic_liquify_dab(options)
    }
}
#[path = "liquify_operators.rs"]
mod operators;
#[path = "liquify_preview.rs"]
mod preview;
pub use preview::{LiquifyPreview, LiquifyPreviewSource};

#[cfg(test)]
mod operator_transaction_tests {
    use super::*;
    fn seed(white: bool) -> Document {
        let mut d = Document::new(600, 600).unwrap();
        let layer = &mut d.layers[0];
        layer.appearance.offset_x = 17;
        layer.appearance.offset_y = -11;
        if white {
            layer.white = (600, 600);
        }
        for ty in 0..10 {
            for tx in 0..10 {
                if (tx + ty) % 5 == 0 {
                    continue;
                }
                let pixels = (0..4096)
                    .map(|i| {
                        Pixel::from_straight([
                            (i % 61) as f32 / 60.,
                            (i % 67) as f32 / 66.,
                            (tx + ty) as f32 / 18.,
                            (i % 13) as f32 / 12.,
                        ])
                        .unwrap()
                    })
                    .collect();
                layer.tiles.insert(
                    TileCoord::from_signed(tx, ty),
                    Arc::new(Tile::from_pixels(pixels).unwrap()),
                );
            }
        }
        d.begin_liquify().unwrap();
        d.begin_liquify_stroke().unwrap();
        d
    }
    fn dab(tool: u32) -> LiquifyDab {
        LiquifyDab {
            tool,
            radius: 280.,
            strength: 0.6,
            density: 0.7,
            speed: 0.8,
            pressure_enabled: true,
            from: InputPoint::new(280., 290., 0.7),
            to: InputPoint::new(289., 294., 0.8),
            elapsed_ms: 16.,
        }
    }
    fn pixels(d: &Document) -> Vec<Pixel> {
        (0..600)
            .flat_map(|y| (0..600).map(move |x| d.pixel(x, y)))
            .collect()
    }
    #[test]
    fn four_tools_have_deterministic_fields_pixels_history_offsets_and_implicit_white() {
        for white in [false, true] {
            for tool in 0..4 {
                let mut a = seed(white);
                let mut b = seed(white);
                let original = pixels(&a);
                for tool in [0, tool] {
                    assert_eq!(
                        a.liquify_dab(dab(tool)).unwrap(),
                        b.liquify_dab(dab(tool)).unwrap()
                    );
                    assert_eq!(
                        a.liquify.as_ref().unwrap().field,
                        b.liquify.as_ref().unwrap().field
                    );
                    assert_eq!(pixels(&a), pixels(&b));
                }
                let preview = pixels(&a);
                assert_ne!(preview, original);
                a.end_liquify_stroke().unwrap();
                b.end_liquify_stroke().unwrap();
                a.undo_liquify().unwrap();
                b.undo_liquify().unwrap();
                assert_eq!(pixels(&a), original);
                assert_eq!(pixels(&b), original);
                a.redo_liquify().unwrap();
                b.redo_liquify().unwrap();
                assert_eq!(pixels(&a), preview);
                assert_eq!(pixels(&b), preview);
                a.cancel_liquify().unwrap();
                b.cancel_liquify().unwrap();
                assert_eq!(pixels(&a), original);
                assert_eq!(pixels(&b), original);
            }
        }
    }
    #[test]
    fn field_budget_failure_keeps_candidate_unpublished() {
        let mut d = seed(false);
        let before = pixels(&d);
        let empty = Arc::new(vec![[0.; 2]; 4096]);
        for x in 0..MAX_FIELD_TILES - 1 {
            d.liquify
                .as_mut()
                .unwrap()
                .field
                .insert(TileCoord::from_signed(1000 + x as i32, 1000), empty.clone());
        }
        let before_field = d.liquify.as_ref().unwrap().field.clone();
        assert!(matches!(
            d.liquify_dab(dab(0)),
            Err(Error::ResourceLimit("liquify displacement memory"))
        ));
        assert_eq!(d.liquify.as_ref().unwrap().field, before_field);
        assert_eq!(pixels(&d), before);
        assert_eq!(d.liquify_history_depth(), Some((0, 0)));
    }
}

#[cfg(test)]
mod history_select_tests {
    use super::*;
    fn seed() -> Document {
        let mut doc = Document::new(64, 64).unwrap();
        let pixels = (0..4096)
            .map(|i| {
                Pixel::from_straight([(i % 64) as f32 / 63., (i / 64) as f32 / 63., 0.3, 1.])
                    .unwrap()
            })
            .collect();
        doc.layers[0].tiles.insert(
            TileCoord::from_signed(0, 0),
            Arc::new(Tile::from_pixels(pixels).unwrap()),
        );
        doc.begin_liquify().unwrap();
        for i in 0..3 {
            doc.begin_liquify_stroke().unwrap();
            doc.liquify_dab(LiquifyDab {
                tool: 0,
                radius: 16.,
                strength: 0.5,
                density: 0.5,
                speed: 0.5,
                pressure_enabled: false,
                from: InputPoint::new(24., 24. + i as f64, 1.),
                to: InputPoint::new(28., 26. + i as f64, 1.),
                elapsed_ms: 16.,
            })
            .unwrap();
            doc.end_liquify_stroke().unwrap();
        }
        assert_eq!(doc.liquify_history_depth(), Some((3, 0)));
        doc
    }
    fn image(doc: &Document) -> Vec<Pixel> {
        (0..64)
            .flat_map(|y| (0..64).map(move |x| doc.pixel(x, y)))
            .collect()
    }
    #[test]
    fn direct_liquify_travel_restores_fields_and_pixels_once_with_atomic_read_failure() {
        let mut direct = seed();
        let mut stepped = seed();
        for target in [0, 3, 1, 2, 0, 3] {
            let revision = direct.revision();
            direct.select_liquify_history(target).unwrap();
            assert_eq!(direct.revision(), revision + 1);
            while stepped.liquify_history_depth().unwrap().0 > target {
                stepped.undo_liquify().unwrap();
            }
            while stepped.liquify_history_depth().unwrap().0 < target {
                stepped.redo_liquify().unwrap();
            }
            assert_eq!(image(&direct), image(&stepped));
            assert_eq!(
                direct.liquify.as_ref().unwrap().field,
                stepped.liquify.as_ref().unwrap().field
            );
            assert_eq!(
                direct.liquify_history_depth(),
                stepped.liquify_history_depth()
            );
        }
        let revision = direct.revision();
        let before = image(&direct);
        let field = direct.liquify.as_ref().unwrap().field.clone();
        let dirty = direct.dirty.clone();
        let history = Arc::make_mut(&mut direct.liquify.as_mut().unwrap().history);
        Arc::get_mut(history.undo.front_mut().unwrap())
            .unwrap()
            .fields
            .truncate_for_test();
        assert!(direct.select_liquify_history(0).is_err());
        assert_eq!(image(&direct), before);
        assert_eq!(direct.revision(), revision);
        assert_eq!(direct.dirty, dirty);
        assert_eq!(direct.liquify_history_depth(), Some((3, 0)));
        assert_eq!(direct.liquify.as_ref().unwrap().field, field);
    }
}
