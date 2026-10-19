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
        let compatible = self.layers.iter().all(|l| {
            !l.group
                && !l.mask
                && !l.clipped
                && l.parent == 0
                && l.appearance.blend == crate::BlendMode::Normal
        });
        if !compatible {
            return None;
        }
        let at = self.layer_index(id).ok()?;
        if !original.properties.visible {
            return None;
        }
        let [l, t, r, b] = source_bounds(&original);
        // Keep a one-pixel transparent guard. No clipping of retained source tiles.
        let width = r - l + 2;
        let height = b - t + 2;
        if width > i64::from(MAX_DIMENSION) || height > i64::from(MAX_DIMENSION) {
            return None;
        }
        let ox = i64::from(original.appearance.offset_x);
        let oy = i64::from(original.appearance.offset_y);
        // Partition once instead of cloning every layer's tile map three times.
        let snapshot = |layers| DocumentSnapshot {
            pool: Arc::clone(&self.pool),
            width: self.width,
            height: self.height,
            revision: self.revision,
            layers,
            selection: self.selection.clone(),
        };
        let mut layer = original;
        layer.appearance.offset_x = (1 - l) as i32;
        layer.appearance.offset_y = (1 - t) as i32;
        let mut image = snapshot(vec![layer]);
        image.width = width.max(2) as u32;
        image.height = height.max(2) as u32;
        let below = snapshot(self.layers[..at].to_vec());
        let above = snapshot(self.layers[at + 1..].to_vec());
        Some(Arc::new(LiquifyPreviewSource {
            id: NEXT_SOURCE.fetch_add(1, Ordering::Relaxed),
            bounds: [
                (l - 1 + ox) as i32,
                (t - 1 + oy) as i32,
                width.max(2) as i32,
                height.max(2) as i32,
            ],
            image,
            below,
            above,
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
