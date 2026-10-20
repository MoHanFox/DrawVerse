//! Immutable presentation data. No colour sampling is performed while dragging.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SOURCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct LiquifyPreviewSource {
    pub id: u64,
    /// Canvas-space rectangle including off-canvas source pixels and transparent padding.
    pub bounds: [i32; 4],
    pub image: DocumentSnapshot,
    pub below: DocumentSnapshot,
    pub above: DocumentSnapshot,
}

#[derive(Clone, Debug)]
pub struct LiquifyPreview {
    pub source: Arc<LiquifyPreviewSource>,
    pub revision: u64,
    pub focus: bool,
    /// Forward source mesh, captured before full-document linear compositing.
    pub forward: bool,
    pub columns: u32,
    pub rows: u32,
    /// Regular destination positions and inverse source UVs: x, y, u, v.
    pub vertices: Vec<[f32; 4]>,
}

pub(super) fn source_bounds(layer: &Layer) -> [i64; 4] {
    let mut rect = [0, 0, i64::from(layer.white.0), i64::from(layer.white.1)];
    for key in layer.tiles.keys() {
        let x = i64::from(key.signed_x()) * 64;
        let y = i64::from(key.signed_y()) * 64;
        rect = [
            rect[0].min(x),
            rect[1].min(y),
            rect[2].max(x + 64),
            rect[3].max(y + 64),
        ];
    }
    rect
}

impl Document {
    /// Opt in to mesh-only presentation where the stack has an exact normal-over split.
    /// Unsupported stacks keep the established pixel preview and transaction semantics.
    pub fn begin_liquify_deferred(&mut self) -> Result<()> {
        self.begin_liquify_internal(false)?;
        let source = if self.selection.enabled() {
            None
        } else {
            self.resident_source(self.active_layer)
        };
        self.liquify.as_mut().expect("state").preview_source = source;
        if !self.liquify_deferred() {
            return Ok(());
        }
        if let Err(error) = self.warm_liquify_mesh() {
            self.liquify = None;
            return Err(error);
        }
        self.dirty_all();
        Ok(())
    }

    /// Immutable split shared by deformation and translation previews.
    pub(in crate::document) fn resident_source(
        &self,
        id: LayerId,
    ) -> Option<Arc<LiquifyPreviewSource>> {
        let original = self.layers.get(self.layer_index(id).ok()?)?;
        if original.is_image() {
            return None;
        }
        self.resident_source_layer(id, original.clone())
    }

    pub(in crate::document) fn resident_source_layer(
        &self,
        id: LayerId,
        original: Layer,
    ) -> Option<Arc<LiquifyPreviewSource>> {
        let at = self.layer_index(id).ok()?;
        // A root is a complete subtree boundary. Its backdrop can be arbitrarily
        // complex; only the moving source and foreground must be normal-over.
        // Do not split an owner from its mask, an ancestor group, or a clip chain.
        if original.group
            || original.mask
            || original.clipped
            || original.parent != 0
            || original.appearance.blend != crate::BlendMode::Normal
            || self.mask_for(id).is_some()
            || !original.properties.visible
            || !self.resident_foreground_is_normal(at + 1)
        {
            return None;
        }
        let [l, t, r, b] = source_bounds(&original);
        let ox = i64::from(original.appearance.offset_x);
        let oy = i64::from(original.appearance.offset_y);
        self.partition_resident_source(at..at + 1, vec![original], [l + ox, t + oy, r + ox, b + oy])
    }

    /// Translation moves complete linked subtrees together; deformation cannot use
    /// this flattened source because an unwarped mask must retain its own coordinates.
    pub(in crate::document) fn resident_move_source(
        &self,
        ids: &[LayerId],
    ) -> Option<Arc<LiquifyPreviewSource>> {
        let mut roots = self.batch_roots(ids).ok()?;
        roots.sort_by_key(|id| self.layer_index(*id).ok());
        let start = self.subtree(*roots.first()?).ok()?.start;
        let mut end = start;
        for id in roots {
            let range = self.subtree(id).ok()?;
            let root = &self.layers[self.layer_index(id).ok()?];
            if range.start != end
                || root.parent != 0
                || root.mask
                || root.clipped
                || !root.properties.visible
                || !(root.appearance.blend == crate::BlendMode::Normal
                    || root.group && root.appearance.blend == crate::BlendMode::PassThrough)
            {
                return None;
            }
            // Normal groups isolate their internal blending. A pass-through source
            // may only be flattened if its descendants do not depend on the backdrop.
            if root.group
                && root.appearance.blend == crate::BlendMode::PassThrough
                && self.layers[range.clone()].iter().any(|layer| {
                    !layer.mask
                        && (layer.clipped
                            || !(layer.appearance.blend == crate::BlendMode::Normal
                                || layer.group
                                    && layer.appearance.blend == crate::BlendMode::PassThrough))
                })
            {
                return None;
            }
            end = range.end;
        }
        let layers = &self.layers[start..end];
        if layers.iter().any(|l| {
            l.is_image()
                || l.mask && !l.mask_linked
                || l.appearance.blend == crate::BlendMode::Dissolve
        }) || !self.resident_foreground_is_normal(end)
        {
            return None;
        }
        let mut bounds: Option<[i64; 4]> = None;
        for layer in layers.iter().filter(|l| !l.group && !l.mask) {
            let [l, t, r, b] = source_bounds(layer);
            let (ox, oy) = (
                i64::from(layer.appearance.offset_x),
                i64::from(layer.appearance.offset_y),
            );
            let next = [l + ox, t + oy, r + ox, b + oy];
            bounds = Some(bounds.map_or(next, |old| {
                [
                    old[0].min(next[0]),
                    old[1].min(next[1]),
                    old[2].max(next[2]),
                    old[3].max(next[3]),
                ]
            }));
        }
        self.partition_resident_source(start..end, layers.to_vec(), bounds?)
    }

    fn resident_foreground_is_normal(&self, start: usize) -> bool {
        !self.layers[start..].iter().any(|l| {
            l.clipped
                || !l.mask
                    && l.appearance.blend != crate::BlendMode::Normal
                    && !(l.group && l.appearance.blend == crate::BlendMode::PassThrough)
        })
    }

    fn partition_resident_source(
        &self,
        range: std::ops::Range<usize>,
        mut layers: Vec<Layer>,
        [l, t, r, b]: [i64; 4],
    ) -> Option<Arc<LiquifyPreviewSource>> {
        // Retain off-canvas pixels and one transparent guard pixel on every edge.
        let width = r - l + 2;
        let height = b - t + 2;
        if width > i64::from(MAX_DIMENSION) || height > i64::from(MAX_DIMENSION) {
            return None;
        }
        let snapshot = |layers| DocumentSnapshot {
            pool: Arc::clone(&self.pool),
            width: self.width,
            height: self.height,
            revision: self.revision,
            layers,
            selection: self.selection.clone(),
        };
        for layer in layers.iter_mut().filter(|l| !l.group) {
            layer.appearance.offset_x =
                i32::try_from(i64::from(layer.appearance.offset_x) + 1 - l).ok()?;
            layer.appearance.offset_y =
                i32::try_from(i64::from(layer.appearance.offset_y) + 1 - t).ok()?;
        }
        let mut image = snapshot(layers);
        image.width = width.max(2) as u32;
        image.height = height.max(2) as u32;
        Some(Arc::new(LiquifyPreviewSource {
            id: NEXT_SOURCE.fetch_add(1, Ordering::Relaxed),
            bounds: [
                i32::try_from(l - 1).ok()?,
                i32::try_from(t - 1).ok()?,
                width.max(2) as i32,
                height.max(2) as i32,
            ],
            image,
            below: snapshot(self.layers[..range.start].to_vec()),
            above: snapshot(self.layers[range.end..].to_vec()),
        }))
    }

    pub fn liquify_deferred(&self) -> bool {
        self.liquify
            .as_ref()
            .is_some_and(|s| s.preview_source.is_some())
    }

    pub fn liquify_preview(&self) -> Option<LiquifyPreview> {
        let state = self.liquify.as_ref()?;
        let source = state.preview_source.as_ref()?.clone();
        // A presentation mesh budget, independent of source pixels and brush diameter.
        let step = self.width.max(self.height).div_ceil(256).max(1);
        let columns = self.width.div_ceil(step) + 1;
        let rows = self.height.div_ceil(step) + 1;
        let mut vertices = Vec::with_capacity((columns * rows) as usize);
        let cell = state.cell as f32;
        let ox = state.original.appearance.offset_x as f32;
        let oy = state.original.appearance.offset_y as f32;
        for row in 0..rows {
            let y = (row * step).min(self.height) as f32;
            for col in 0..columns {
                let x = (col * step).min(self.width) as f32;
                let gx = (x - ox) / cell;
                let gy = (y - oy) / cell;
                let ix = gx.floor() as i64;
                let iy = gy.floor() as i64;
                let mut delta = [0.; 2];
                for (dx, wx) in [(0, 1. - gx.rem_euclid(1.)), (1, gx.rem_euclid(1.))] {
                    for (dy, wy) in [(0, 1. - gy.rem_euclid(1.)), (1, gy.rem_euclid(1.))] {
                        let v = state
                            .field
                            .get(&coord(ix + dx, iy + dy))
                            .map_or([0.; 2], |f| f[index(ix + dx, iy + dy)]);
                        for k in 0..2 {
                            delta[k] += v[k] * wx * wy;
                        }
                    }
                }
                vertices.push([
                    x,
                    y,
                    (x + delta[0] - source.bounds[0] as f32) / source.bounds[2] as f32,
                    (y + delta[1] - source.bounds[1] as f32) / source.bounds[3] as f32,
                ]);
            }
        }
        Some(LiquifyPreview {
            source,
            revision: self.revision,
            focus: state.focus,
            forward: false,
            columns,
            rows,
            vertices,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_ops::{Cpu, Executor, Operator};
    use std::sync::atomic::AtomicUsize;
    #[derive(Debug, Default)]
    struct Count {
        remaps: AtomicUsize,
    }
    impl Executor for Count {
        fn name(&self) -> String {
            "count".into()
        }
        fn execute(&self, op: &Operator) -> std::result::Result<Vec<[f32; 4]>, String> {
            if !matches!(op, Operator::Elastic(_)) {
                self.remaps.fetch_add(1, Ordering::Relaxed);
            }
            Cpu.execute(op)
        }
    }
    fn seed() -> Document {
        let mut d = Document::new(128, 96).unwrap();
        d.layers[0].appearance.offset_x = 7;
        d.layers[0].appearance.offset_y = -3;
        for (tx, ty) in [(-1, 0), (0, 0), (1, 0), (0, 1)] {
            let pixels = (0..4096)
                .map(|i| {
                    Pixel::from_straight([(i % 64) as f32 / 63., (i / 64) as f32 / 63., 0.2, 0.7])
                        .unwrap()
                })
                .collect();
            d.layers[0].tiles.insert(
                TileCoord::from_signed(tx, ty),
                Arc::new(Tile::from_pixels(pixels).unwrap()),
            );
        }
        d
    }

    fn subtree_seed() -> (Document, LayerId) {
        let mut d = seed();
        let bottom = d.active_layer();
        let group = d.group_layer(bottom, "below").unwrap();
        let at = d.layer_index(bottom).unwrap();
        d.layers[at].appearance.blend = crate::BlendMode::Multiply;
        d.add_mask(group).unwrap();
        let moving = d.add_layer("moving").unwrap();
        d.reparent_layer(moving, 0).unwrap();
        let above = d.add_layer("above").unwrap();
        for (id, color) in [(moving, [1., 0.1, 0.2, 0.7]), (above, [0.1, 1., 0.3, 0.4])] {
            let at = d.layer_index(id).unwrap();
            d.layers[at].tiles.insert(
                TileCoord::from_signed(0, 0),
                Arc::new(
                    Tile::from_pixels(vec![Pixel::from_straight(color).unwrap(); 4096]).unwrap(),
                ),
            );
            d.layers[at].properties.opacity = 0.65;
        }
        let group = d.group_layer(above, "above").unwrap();
        let outer = d.group_layer(group, "nested above").unwrap();
        for id in [above, group, outer] {
            let mask = d.add_mask(id).unwrap();
            let at = d.layer_index(mask).unwrap();
            d.layers[at].tiles.insert(
                TileCoord::from_signed(0, 0),
                Arc::new(Tile::from_pixels(vec![Pixel::WHITE.scaled(0.25); 4096]).unwrap()),
            );
            let at = d.layer_index(id).unwrap();
            d.layers[at].properties.opacity = 0.55;
        }
        d.set_active_layer(moving).unwrap();
        (d, moving)
    }

    #[test]
    fn unrelated_masked_subtrees_split_without_changing_compositing() {
        let (d, moving) = subtree_seed();
        let source = d.resident_source(moving).unwrap();
        assert!(source.below.layers.iter().any(|l| l.group));
        assert!(source.above.layers.iter().any(|l| l.mask));
        for y in (0..96).step_by(3) {
            for x in (0..128).step_by(3) {
                let image = source.image.pixel(
                    (x as i32 - source.bounds[0]) as u32,
                    (y as i32 - source.bounds[1]) as u32,
                );
                let split = source
                    .above
                    .pixel(x, y)
                    .over(image.over(source.below.pixel(x, y)));
                for (a, b) in split
                    .components()
                    .into_iter()
                    .zip(d.pixel(x, y).components())
                {
                    assert!((a - b).abs() < 1e-6, "split mismatch at {x},{y}: {a} / {b}");
                }
            }
        }
        let (mut d, moving) = subtree_seed();
        let above = d
            .layers
            .iter()
            .position(|l| l.name == "above" && !l.group)
            .unwrap();
        d.layers[above].appearance.blend = crate::BlendMode::Multiply;
        assert!(
            d.resident_source(moving).is_none(),
            "backdrop-dependent foreground"
        );
        d.layers[above].appearance.blend = crate::BlendMode::Normal;
        // An owner mask still needs its own presentation path, not an unrelated mask split.
        d.add_mask(moving).unwrap();
        assert!(d.resident_source(moving).is_none());
    }

    #[test]
    fn unrelated_subtrees_share_transform_move_and_liquify_resident_sources() {
        for operation in 0..3 {
            let (mut a, id) = subtree_seed();
            let (mut b, _) = subtree_seed();
            let before = a.layer(id).unwrap().tiles.clone();
            let depth = a.history_depth();
            match operation {
                0 => {
                    a.begin_transform_layers_deferred(&[id]).unwrap();
                    b.begin_transform_layers(&[id]).unwrap();
                    assert!(a.transform_deferred());
                    let source = a.transform_preview().unwrap().source;
                    let initial = a.transform_geometry().unwrap();
                    for i in 1..=8 {
                        let mut g = initial.clone();
                        g.warp = true;
                        g.points[5][0] += i as f64;
                        g.points[10][1] -= i as f64;
                        a.update_transform(g.clone()).unwrap();
                        b.update_transform(g).unwrap();
                        assert!(Arc::ptr_eq(&source, &a.transform_preview().unwrap().source));
                    }
                }
                1 => {
                    a.begin_layer_move(id, true).unwrap();
                    b.begin_layer_move(id, false).unwrap();
                    assert!(a.layer_move_deferred());
                    a.update_layer_move(7, -3).unwrap();
                    b.update_layer_move(7, -3).unwrap();
                }
                _ => {
                    a.begin_liquify_deferred().unwrap();
                    b.begin_liquify().unwrap();
                    assert!(a.liquify_deferred());
                    for d in [&mut a, &mut b] {
                        d.begin_liquify_stroke().unwrap();
                        d.liquify_dab(LiquifyDab {
                            tool: 0,
                            radius: 30.,
                            strength: 0.7,
                            density: 0.5,
                            speed: 1.,
                            pressure_enabled: false,
                            from: InputPoint::new(32., 32., 1.),
                            to: InputPoint::new(39., 29., 1.),
                            elapsed_ms: 16.,
                        })
                        .unwrap();
                        d.end_liquify_stroke().unwrap();
                    }
                }
            }
            for (key, tile) in &before {
                assert!(Arc::ptr_eq(tile, &a.layer(id).unwrap().tiles[key]));
            }
            assert_eq!(a.history_depth(), depth);
            for d in [&mut a, &mut b] {
                match operation {
                    0 => {
                        d.commit_transform().unwrap();
                    }
                    1 => {
                        d.commit_layer_move(7, -3).unwrap();
                    }
                    _ => {
                        d.commit_liquify().unwrap();
                    }
                }
            }
            for y in 0..96 {
                assert_eq!(
                    a.snapshot().read_row(0, y, 128).unwrap(),
                    b.snapshot().read_row(0, y, 128).unwrap()
                );
            }
            assert_eq!(a.history_depth(), b.history_depth());
            a.undo().unwrap();
            a.redo().unwrap();
            assert_eq!(a.pixel(40, 40), b.pixel(40, 40));
        }
    }
    #[test]
    fn deferred_drag_keeps_pixels_and_sources_until_commit_and_matches_pixel_path() {
        for tool in 0..4 {
            let mut a = seed();
            let mut b = seed();
            let original = a.layers[0].clone();
            let counter = Arc::new(Count::default());
            a.set_compute_executor(counter.clone()).unwrap();
            a.begin_liquify_deferred().unwrap();
            b.begin_liquify().unwrap();
            assert!(a.liquify_deferred());
            let source = a.liquify_preview().unwrap().source;
            assert!(
                source.bounds[0] < 0,
                "off-canvas source must remain available"
            );
            let initial = a.liquify_preview().unwrap().vertices;
            for d in [&mut a, &mut b] {
                d.begin_liquify_stroke().unwrap();
                let mut dab = LiquifyDab {
                    tool: 0,
                    radius: 7500.,
                    strength: 0.7,
                    density: 0.8,
                    speed: 1.,
                    pressure_enabled: false,
                    from: InputPoint::new(40., 40., 1.),
                    to: InputPoint::new(57., 44., 1.),
                    elapsed_ms: 16.,
                };
                d.liquify_dab(dab).unwrap();
                dab.tool = tool;
                dab.from = dab.to;
                dab.to = InputPoint::new(61., 47., 1.);
                d.liquify_dab(dab).unwrap();
                d.end_liquify_stroke().unwrap();
            }
            assert_eq!(
                counter.remaps.load(Ordering::Relaxed),
                0,
                "drag must execute only mesh operators"
            );
            assert!(a.layers[0]
                .tiles
                .iter()
                .all(|(k, t)| Arc::ptr_eq(t, &original.tiles[k])));
            let warped = a.liquify_preview().unwrap();
            assert!(Arc::ptr_eq(&source, &warped.source));
            assert_ne!(initial, warped.vertices);
            a.undo_liquify().unwrap();
            assert_eq!(a.liquify_preview().unwrap().vertices, initial);
            a.redo_liquify().unwrap();
            assert_eq!(a.liquify_preview().unwrap().vertices, warped.vertices);
            assert_eq!(counter.remaps.load(Ordering::Relaxed), 0);
            a.commit_liquify().unwrap();
            b.commit_liquify().unwrap();
            assert!(counter.remaps.load(Ordering::Relaxed) > 0);
            let keys = a.layers[0]
                .tiles
                .keys()
                .chain(b.layers[0].tiles.keys())
                .copied()
                .collect::<BTreeSet<_>>();
            for key in keys {
                assert_eq!(
                    a.layers[0]
                        .tiles
                        .get(&key)
                        .map(|t| t.try_pixels().unwrap().to_vec()),
                    b.layers[0]
                        .tiles
                        .get(&key)
                        .map(|t| t.try_pixels().unwrap().to_vec())
                );
            }
            assert_eq!(a.history_depth(), b.history_depth());
            a.undo().unwrap();
            assert_eq!(a.pixel(32, 32), original.canvas_pixel(32, 32).unwrap());
            a.redo().unwrap();
        }
        let mut d = seed();
        let before = d.snapshot();
        d.begin_liquify_deferred().unwrap();
        d.reset_liquify().unwrap();
        d.cancel_liquify().unwrap();
        assert!(!d.liquify_deferred());
        assert_eq!(d.pixel(32, 32), before.pixel(32, 32));
    }
}
