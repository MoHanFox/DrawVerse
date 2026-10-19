use super::*;
const MAX_FIELD_TILES: usize = 8192;
const MAX_VISITS: usize = 8_000_000;
type Field = BTreeMap<TileCoord, Arc<Vec<[f32; 2]>>>;
#[derive(Clone, Debug)]
pub(super) struct LiquifyState {
    original: Layer,
    field: Field,
    focus: bool,
    checkpoint: Option<Arc<Checkpoint>>,
    history: Arc<LocalHistory>,
}
#[derive(Clone, Debug)]
struct Checkpoint {
    layer: Layer,
    field: Field,
    action: crate::HistoryAction,
}
#[derive(Debug)]
struct LocalStep {
    pixels: Vec<TileChange>,
    fields: crate::history_storage::Payload,
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
            || !(0.5..=1000.).contains(&self.radius)
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
struct FieldSampler<'a> {
    field: &'a Field,
    last: Option<(TileCoord, Option<&'a [[f32; 2]]>)>,
}
impl FieldSampler<'_> {
    fn at(&mut self, x: i64, y: i64) -> [f32; 2] {
        let key = coord(x, y);
        if let Some((old, pixels)) = self.last {
            if old == key {
                return pixels.map_or([0., 0.], |v| v[index(x, y)]);
            }
        }
        let tile = self.field.get(&key).map(|v| v.as_slice());
        self.last = Some((key, tile));
        tile.map_or([0., 0.], |v| v[index(x, y)])
    }
    fn sample(&mut self, x: f64, y: f64) -> [f32; 2] {
        let ix = x.floor() as i64;
        let iy = y.floor() as i64;
        let fx = (x - ix as f64) as f32;
        let fy = (y - iy as f64) as f32;
        let a = self.at(ix, iy);
        let (b, c, d) = if ix.rem_euclid(64) < 63 && iy.rem_euclid(64) < 63 {
            self.last.and_then(|(_, pixels)| pixels).map_or(
                ([0., 0.], [0., 0.], [0., 0.]),
                |pixels| {
                    let i = index(ix, iy);
                    (pixels[i + 1], pixels[i + 64], pixels[i + 65])
                },
            )
        } else {
            (
                self.at(ix + 1, iy),
                self.at(ix, iy + 1),
                self.at(ix + 1, iy + 1),
            )
        };
        [0, 1].map(|k| {
            (a[k] * (1. - fx) + b[k] * fx) * (1. - fy) + (c[k] * (1. - fx) + d[k] * fx) * fy
        })
    }
}
type PinnedPixels = Arc<Box<[Pixel]>>;
struct Source<'a> {
    layer: &'a Layer,
    pin_limit: usize,
    pins: BTreeMap<TileCoord, PinnedPixels>,
    order: std::collections::VecDeque<TileCoord>,
    last: Option<(TileCoord, Option<PinnedPixels>)>,
}

#[cfg(test)]
mod sampling_tests {
    use super::*;

    #[test]
    fn bilinear_fast_paths_match_four_independent_reads_bit_for_bit() {
        let mut field = Field::new();
        let mut layer = Layer::new(1, "Sampling".into());
        layer.white = (69, 77);
        for ty in [-2, 0, 1] {
            for tx in [-2, 0, 2] {
                let key = TileCoord::from_signed(tx, ty);
                field.insert(
                    key,
                    Arc::new(
                        (0..4096)
                            .map(|i| [i as f32 / 19. - 80., (i % 123) as f32 / 7.])
                            .collect(),
                    ),
                );
                let pixels = (0..4096)
                    .map(|i| {
                        Pixel::from_straight([
                            (i % 31) as f32 / 30.,
                            (i % 17) as f32 / 16.,
                            (i % 13) as f32 / 12.,
                            (i % 7) as f32 / 6.,
                        ])
                        .unwrap()
                    })
                    .collect();
                layer
                    .tiles
                    .insert(key, Arc::new(Tile::from_pixels(pixels).unwrap()));
            }
        }
        let mut sampler = FieldSampler {
            field: &field,
            last: None,
        };
        let mut source = Source {
            layer: &layer,
            pin_limit: 256,
            pins: BTreeMap::new(),
            order: Default::default(),
            last: None,
        };
        for ix in [
            -130, -129, -128, -65, -64, -1, 0, 1, 62, 63, 64, 68, 69, 76, 77, 127, 128, 256,
        ] {
            for iy in [-129, -64, -1, 0, 62, 63, 64, 68, 69, 76, 77, 127, 128] {
                for fx in [0., 0.125, 0.5, 0.999] {
                    for fy in [0., 0.125, 0.5, 0.999] {
                        let at = |x, y| {
                            field
                                .get(&coord(x, y))
                                .map_or([0., 0.], |pixels| pixels[index(x, y)])
                        };
                        let (a, b, c, d) = (
                            at(ix, iy),
                            at(ix + 1, iy),
                            at(ix, iy + 1),
                            at(ix + 1, iy + 1),
                        );
                        let expected = [0, 1].map(|k| {
                            (a[k] * (1. - fx) + b[k] * fx) * (1. - fy)
                                + (c[k] * (1. - fx) + d[k] * fx) * fy
                        });
                        let actual =
                            sampler.sample(ix as f64 + f64::from(fx), iy as f64 + f64::from(fy));
                        // Recover the actual f32 fractions after f64 coordinate addition.
                        let x = ix as f64 + f64::from(fx);
                        let y = iy as f64 + f64::from(fy);
                        let rx = (x - x.floor()) as f32;
                        let ry = (y - y.floor()) as f32;
                        let expected = if rx == fx && ry == fy {
                            expected
                        } else {
                            [0, 1].map(|k| {
                                (a[k] * (1. - rx) + b[k] * rx) * (1. - ry)
                                    + (c[k] * (1. - rx) + d[k] * rx) * ry
                            })
                        };
                        assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
                        let pixel = |x, y| {
                            layer.tiles.get(&coord(x, y)).map_or_else(
                                || layer.default_pixel(x, y),
                                |tile| tile.try_pixels().unwrap()[index(x, y)],
                            )
                        };
                        let a = pixel(ix, iy);
                        let expected = if rx == 0. && ry == 0. {
                            a
                        } else {
                            a.mix(pixel(ix + 1, iy), rx)
                                .mix(pixel(ix, iy + 1).mix(pixel(ix + 1, iy + 1), rx), ry)
                        };
                        assert_eq!(
                            source.sample(x, y).unwrap().components().map(f32::to_bits),
                            expected.components().map(f32::to_bits)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn source_fast_path_keeps_the_256_tile_pin_limit() {
        let mut layer = Layer::new(1, "Pins".into());
        for x in 0..300 {
            layer.tiles.insert(
                TileCoord::from_signed(x, 0),
                Arc::new(Tile::from_pixels(vec![Pixel::WHITE; 4096]).unwrap()),
            );
        }
        let mut source = Source {
            layer: &layer,
            pin_limit: 256,
            pins: BTreeMap::new(),
            order: Default::default(),
            last: None,
        };
        for x in (0..300).chain(0..300) {
            assert_eq!(
                source.sample(f64::from(x * 64) + 0.25, 0.75).unwrap(),
                Pixel::WHITE
            );
            assert!(source.pins.len() <= 256);
            assert_eq!(source.pins.len(), source.order.len());
        }
    }
}
impl Source<'_> {
    fn pixel(&mut self, x: i64, y: i64) -> Result<Pixel> {
        let key = coord(x, y);
        if let Some((old, pixels)) = &self.last {
            if *old == key {
                return Ok(pixels
                    .as_ref()
                    .map_or_else(|| self.layer.default_pixel(x, y), |v| v[index(x, y)]));
            }
        }
        if let Some(tile) = self.layer.tiles.get(&key) {
            if !self.pins.contains_key(&key) {
                if self.pins.len() >= self.pin_limit {
                    if let Some(old) = self.order.pop_front() {
                        self.pins.remove(&old);
                    }
                }
                self.pins.insert(key, tile.try_pixels()?);
                self.order.push_back(key);
            }
            let pixels = self.pins[&key].clone();
            let result = pixels[index(x, y)];
            self.last = Some((key, Some(pixels)));
            Ok(result)
        } else {
            self.last = Some((key, None));
            Ok(self.layer.default_pixel(x, y))
        }
    }
    fn sample(&mut self, x: f64, y: f64) -> Result<Pixel> {
        let ix = x.floor() as i64;
        let iy = y.floor() as i64;
        let fx = (x - ix as f64) as f32;
        let fy = (y - iy as f64) as f32;
        let a = self.pixel(ix, iy)?;
        if fx == 0. && fy == 0. {
            return Ok(a);
        }
        let (b, c, d) = if ix.rem_euclid(64) < 63 && iy.rem_euclid(64) < 63 {
            if let Some((_, Some(pixels))) = &self.last {
                let i = index(ix, iy);
                (pixels[i + 1], pixels[i + 64], pixels[i + 65])
            } else {
                (
                    self.layer.default_pixel(ix + 1, iy),
                    self.layer.default_pixel(ix, iy + 1),
                    self.layer.default_pixel(ix + 1, iy + 1),
                )
            }
        } else {
            (
                self.pixel(ix + 1, iy)?,
                self.pixel(ix, iy + 1)?,
                self.pixel(ix + 1, iy + 1)?,
            )
        };
        let top = a.mix(b, fx);
        let bottom = c.mix(d, fx);
        Ok(top.mix(bottom, fy))
    }
}

fn claim_field(counter: &std::sync::atomic::AtomicUsize, existing: usize) -> bool {
    use std::sync::atomic::Ordering::Relaxed;
    let mut count = counter.load(Relaxed);
    loop {
        if existing + count >= MAX_FIELD_TILES {
            return false;
        }
        match counter.compare_exchange_weak(count, count + 1, Relaxed, Relaxed) {
            Ok(_) => return true,
            Err(actual) => count = actual,
        }
    }
}
type RasterOutput = (
    BTreeMap<TileCoord, Vec<[f32; 2]>>,
    BTreeMap<TileCoord, Vec<Pixel>>,
);
#[derive(Clone, Copy)]
struct RasterStep {
    options: LiquifyDab,
    x: f64,
    y: f64,
    radius: f64,
    amount: f64,
    dx: f64,
    dy: f64,
    time: f64,
    ox: i64,
    oy: i64,
    left: i64,
    right: i64,
    top: i64,
    bottom: i64,
}
impl RasterStep {
    fn raster(
        &self,
        layer: &Layer,
        old_field: &Field,
        source: &mut Source<'_>,
        selection: &crate::Selection,
        cache: &mut crate::selection::SelectionCoverageCache,
        new_fields: &std::sync::atomic::AtomicUsize,
    ) -> Result<RasterOutput> {
        let Self {
            options,
            x,
            y,
            radius,
            amount,
            dx,
            dy,
            time,
            ox,
            oy,
            left,
            right,
            top,
            bottom,
        } = *self;
        let mut fields = BTreeMap::<TileCoord, Vec<[f32; 2]>>::new();
        let mut pixels = BTreeMap::<TileCoord, Vec<Pixel>>::new();
        let mut sampler = FieldSampler {
            field: old_field,
            last: None,
        };
        for ty in (top - oy).div_euclid(64)..=(bottom - 1 - oy).div_euclid(64) {
            for tx in (left - ox).div_euclid(64)..=(right - 1 - ox).div_euclid(64) {
                let key = TileCoord::from_signed(tx as i32, ty as i32);
                let x0 = left.max(tx * 64 + ox);
                let x1 = right.min(tx * 64 + ox + 64);
                let y0 = top.max(ty * 64 + oy);
                let y1 = bottom.min(ty * 64 + oy + 64);
                let mut buffers: Option<(Vec<[f32; 2]>, Vec<Pixel>)> = None;
                for py in y0..y1 {
                    let coverage = cache.corner_row(selection, x0, py, (x1 - x0) as usize);
                    for px in x0..x1 {
                        let vx = px as f64 + 0.5 - x;
                        let vy = py as f64 + 0.5 - y;
                        let r = vx.hypot(vy) / radius;
                        if r >= 1. {
                            continue;
                        }
                        let inner = f64::from(options.density) * 0.9;
                        let falloff = if r <= inner {
                            1.
                        } else {
                            ((1. - r) / (1. - inner)).powi(2)
                        };
                        let weight = amount * falloff * f64::from(coverage[(px - x0) as usize]);
                        if weight <= 0. {
                            continue;
                        }
                        let lx = px - ox;
                        let ly = py - oy;
                        let i = index(lx, ly);
                        let before = sampler.at(lx, ly);
                        let after = if options.tool == 3 {
                            let factor = (1. - weight).max(0.) as f32;
                            before.map(|v| {
                                if (v * factor).abs() < 0.0001 {
                                    0.
                                } else {
                                    v * factor
                                }
                            })
                        } else {
                            let (tx, ty) = match options.tool {
                                0 => (-dx * weight, -dy * weight),
                                1 => {
                                    let factor =
                                        (-weight * f64::from(options.speed) * time * 2.).exp() - 1.;
                                    (vx * factor, vy * factor)
                                }
                                2 => {
                                    let angle = -weight
                                        * f64::from(options.speed)
                                        * time
                                        * std::f64::consts::TAU;
                                    let (s, c) = angle.sin_cos();
                                    (vx * c - vy * s - vx, vx * s + vy * c - vy)
                                }
                                _ => unreachable!(),
                            };
                            let sampled = sampler.sample(lx as f64 + tx, ly as f64 + ty);
                            [
                                (tx + f64::from(sampled[0])) as f32,
                                (ty + f64::from(sampled[1])) as f32,
                            ]
                        };
                        if after.iter().any(|v| !v.is_finite() || v.abs() > 2_000_000.) {
                            return Err(Error::ResourceLimit("liquify displacement range"));
                        }
                        if after == before {
                            continue;
                        }
                        if buffers.is_none() {
                            if !old_field.contains_key(&key)
                                && !claim_field(new_fields, old_field.len())
                            {
                                return Err(Error::ResourceLimit("liquify displacement memory"));
                            }
                            let field = old_field
                                .get(&key)
                                .map_or_else(|| vec![[0., 0.]; 4096], |v| v.as_ref().clone());
                            let original = if let Some(tile) = layer.tiles.get(&key) {
                                tile.try_pixels()?.to_vec()
                            } else {
                                let mut out = Vec::with_capacity(4096);
                                for sy in 0..64 {
                                    for sx in 0..64 {
                                        out.push(layer.default_pixel(
                                            i64::from(key.signed_x()) * 64 + sx,
                                            i64::from(key.signed_y()) * 64 + sy,
                                        ));
                                    }
                                }
                                out
                            };
                            buffers = Some((field, original));
                        }
                        let (field, pixel) = buffers.as_mut().expect("tile buffers");
                        field[i] = after;
                        pixel[i] = source.sample(
                            lx as f64 + f64::from(after[0]),
                            ly as f64 + f64::from(after[1]),
                        )?;
                    }
                }
                if let Some((field, pixel)) = buffers {
                    fields.insert(key, field);
                    pixels.insert(key, pixel);
                }
            }
        }
        Ok((fields, pixels))
    }
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
        self.liquify = Some(LiquifyState {
            original,
            field: Field::new(),
            focus: false,
            checkpoint: None,
            history: Arc::default(),
        });
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
        let state = self
            .liquify
            .as_ref()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?;
        let current = self.layer(state.original.id).expect("transaction layer");
        let keys: BTreeSet<_> = current
            .tiles
            .keys()
            .chain(state.original.tiles.keys())
            .copied()
            .collect();
        let mut changes = Vec::new();
        let mut unchanged = Vec::new();
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
        }
        let original = state.original.clone();
        let focused = state.focus;
        let changed = !changes.is_empty();
        if changed {
            self.commit_with_action(
                Command::Stroke {
                    layer: original.id,
                    changes,
                },
                crate::HistoryAction::Liquify,
            )?;
        }
        let at = self.layer_index(original.id)?;
        for key in unchanged {
            if let Some(tile) = original.tiles.get(&key) {
                self.layers[at].tiles.insert(key, tile.clone());
            } else {
                self.layers[at].tiles.remove(&key);
            }
        }
        self.liquify = None;
        if focused {
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
            .filter(|key| !same_field(before.field.get(key), state.field.get(key)))
            .collect();
        if changed.is_empty() {
            self.liquify.as_mut().expect("state").checkpoint = None;
            return Ok(false);
        }
        let pixels = changed
            .iter()
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
    pub fn liquify_dab(&mut self, options: LiquifyDab) -> Result<bool> {
        self.liquify_dab_impl(options, true)
    }
    fn liquify_dab_impl(&mut self, options: LiquifyDab, parallel: bool) -> Result<bool> {
        options.validate()?;
        let mut state = self
            .liquify
            .as_ref()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?
            .clone();
        let at = self.layer_index(state.original.id)?;
        if let Some(checkpoint) = state.checkpoint.as_mut() {
            if checkpoint.action == crate::HistoryAction::Liquify {
                Arc::make_mut(checkpoint).action = match options.tool {
                    0 => crate::HistoryAction::LiquifyPush,
                    1 => crate::HistoryAction::LiquifyBloat,
                    2 => crate::HistoryAction::LiquifyTwirl,
                    _ => crate::HistoryAction::LiquifyReconstruct,
                };
            }
        }
        let mut layer = self.layers[at].clone();
        let radius = f64::from(options.radius);
        let distance = (options.to.x - options.from.x).hypot(options.to.y - options.from.y);
        let steps = (distance / (radius * 0.2).max(1.)).ceil().max(1.) as usize;
        let area = ((radius * 2. + 2.).min(f64::from(self.width))
            * (radius * 2. + 2.).min(f64::from(self.height))) as usize;
        if steps > 4096 || steps.saturating_mul(area) > MAX_VISITS {
            return Err(Error::ResourceLimit("liquify input work budget"));
        }
        let mut source = Source {
            layer: &state.original,
            pin_limit: 256,
            pins: BTreeMap::new(),
            order: Default::default(),
            last: None,
        };
        let mut previous = options.from;
        let mut changed = false;
        for step in 1..=steps {
            let t = step as f64 / steps as f64;
            let x = options.from.x + (options.to.x - options.from.x) * t;
            let y = options.from.y + (options.to.y - options.from.y) * t;
            let pressure = if options.pressure_enabled {
                options.from.pressure + (options.to.pressure - options.from.pressure) * t as f32
            } else {
                1.
            };
            let amount = f64::from(options.strength * pressure);
            let dx = (x - previous.x).clamp(-radius, radius);
            let dy = (y - previous.y).clamp(-radius, radius);
            let time = f64::from(options.elapsed_ms) / 1000. / steps as f64;
            previous.x = x;
            previous.y = y;
            if amount == 0. || (options.tool == 0 && dx == 0. && dy == 0.) {
                continue;
            }
            let ox = i64::from(layer.appearance.offset_x);
            let oy = i64::from(layer.appearance.offset_y);
            let left = (x - radius).floor().max(0.) as i64;
            let top = (y - radius).floor().max(0.) as i64;
            let right = (x + radius).ceil().min(f64::from(self.width)) as i64;
            let bottom = (y + radius).ceil().min(f64::from(self.height)) as i64;
            if left >= right || top >= bottom {
                continue;
            }
            let old_field = &state.field;
            let raster = RasterStep {
                options,
                x,
                y,
                radius,
                amount,
                dx,
                dy,
                time,
                ox,
                oy,
                left,
                right,
                top,
                bottom,
            };
            let new_fields = std::sync::atomic::AtomicUsize::new(0);
            let first_ty = (top - oy).div_euclid(64);
            let end_ty = (bottom - 1 - oy).div_euclid(64) + 1;
            let (fields, mut pixels) = if parallel
                && !self.selection.enabled()
                && (right - left) * (bottom - top) >= 262_144
                && end_ty - first_ty >= 2
            {
                // Release serial pins before two 128-tile worker caches take over.
                source.last = None;
                source.pins.clear();
                source.order.clear();
                // Workers read the same immutable input. Publish only after both succeed.
                let split = ((first_ty + end_ty) / 2) * 64 + oy;
                let upper = RasterStep {
                    bottom: split,
                    ..raster
                };
                let lower = RasterStep {
                    top: split,
                    ..raster
                };
                let selection = &self.selection;
                std::thread::scope(|scope| -> Result<RasterOutput> {
                    let make_source = || Source {
                        layer: &state.original,
                        pin_limit: 128,
                        pins: BTreeMap::new(),
                        order: Default::default(),
                        last: None,
                    };
                    let lower_layer = &layer;
                    let count = &new_fields;
                    let worker = std::thread::Builder::new()
                        .name("liquify-tiles".into())
                        .spawn_scoped(scope, move || {
                            lower.raster(
                                lower_layer,
                                old_field,
                                &mut make_source(),
                                selection,
                                &mut crate::selection::SelectionCoverageCache::default(),
                                count,
                            )
                        })
                        .map_err(|_| Error::ResourceLimit("liquify worker unavailable"))?;
                    let upper_result = upper.raster(
                        &layer,
                        old_field,
                        &mut make_source(),
                        selection,
                        &mut self.selection_cache,
                        &new_fields,
                    );
                    let lower_result = worker
                        .join()
                        .map_err(|_| Error::ResourceLimit("liquify worker failed"))?;
                    let (mut fields, mut pixels) = upper_result?;
                    let (mut more_fields, mut more_pixels) = lower_result?;
                    fields.append(&mut more_fields);
                    pixels.append(&mut more_pixels);
                    Ok((fields, pixels))
                })?
            } else {
                raster.raster(
                    &layer,
                    old_field,
                    &mut source,
                    &self.selection,
                    &mut self.selection_cache,
                    &new_fields,
                )?
            };
            let new_tiles = pixels
                .keys()
                .filter(|key| !layer.tiles.contains_key(key))
                .count();
            let other_tiles = self.tile_count() - self.layers[at].tiles.len();
            if other_tiles + layer.tiles.len() + new_tiles > self.options.max_document_tiles {
                return Err(Error::ResourceLimit("liquify pixel tile budget"));
            }
            for (key, field) in fields {
                changed = true;
                if field.iter().all(|v| *v == [0., 0.]) {
                    state.field.remove(&key);
                    if let Some(tile) = state.original.tiles.get(&key) {
                        layer.tiles.insert(key, tile.clone());
                    } else {
                        layer.tiles.remove(&key);
                    }
                } else {
                    let tile = Arc::new(Tile::from_pixels(pixels.remove(&key).expect("pixels"))?);
                    self.pool.attach(&tile)?;
                    // Transparent override tiles are retained for implicit white layers.
                    if tile.is_empty() && layer.white == (0, 0) {
                        layer.tiles.remove(&key);
                    } else {
                        layer.tiles.insert(key, tile);
                    }
                    state.field.insert(key, Arc::new(field));
                }
            }
            self.pool.trim()?;
        }
        drop(source);
        if changed {
            self.layers[at] = layer;
            self.liquify = Some(state);
            self.dirty_all();
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod parallel_tests {
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
    fn four_tools_match_serial_fields_pixels_history_offsets_and_implicit_white() {
        for white in [false, true] {
            for tool in 0..4 {
                let mut a = seed(white);
                let mut b = seed(white);
                let original = pixels(&a);
                for tool in [0, tool] {
                    assert_eq!(
                        a.liquify_dab_impl(dab(tool), false).unwrap(),
                        b.liquify_dab_impl(dab(tool), true).unwrap()
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
    fn shared_field_budget_failure_keeps_both_halves_unpublished() {
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
            d.liquify_dab_impl(dab(0), true),
            Err(Error::ResourceLimit("liquify displacement memory"))
        ));
        assert_eq!(d.liquify.as_ref().unwrap().field, before_field);
        assert_eq!(pixels(&d), before);
        assert_eq!(d.liquify_history_depth(), Some((0, 0)));
    }
}
