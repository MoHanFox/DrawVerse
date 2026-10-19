//! Elastic mesh editing is independent of the clipped viewport and preview pixels.
use super::*;
use paint_ops::{Cpu, ElasticOp, Executor, Grid, Operator, RemapOp, SourceTile};

const CELL: i64 = 2;
const BATCH_TILES: usize = 128;

fn grid(field: &Field, rect: [i64; 4]) -> Result<Grid> {
    let [left, top, right, bottom] = rect;
    let gx = left.div_euclid(CELL) - 2;
    let gy = top.div_euclid(CELL) - 2;
    let width = (right.div_euclid(CELL) - gx + 3) as u32;
    let height = (bottom.div_euclid(CELL) - gy + 3) as u32;
    if u64::from(width) * u64::from(height) > paint_ops::MAX_SAMPLES as u64 {
        return Err(Error::ResourceLimit("liquify operator grid"));
    }
    let mut vectors = Vec::with_capacity(width as usize * height as usize);
    for y in gy..gy + i64::from(height) {
        for x in gx..gx + i64::from(width) {
            vectors.push(field.get(&coord(x, y)).map_or([0.; 2], |v| v[index(x, y)]));
        }
    }
    Ok(Grid {
        origin: [(gx * CELL) as i32, (gy * CELL) as i32],
        width,
        height,
        cell: CELL as f32,
        vectors,
    })
}
fn keys(rect: [i64; 4]) -> Result<Vec<TileCoord>> {
    let [l, t, r, b] = rect;
    let (l, t, r, b) = (
        l.div_euclid(64),
        t.div_euclid(64),
        (r - 1).div_euclid(64),
        (b - 1).div_euclid(64),
    );
    if r < l || b < t {
        return Ok(Vec::new());
    }
    if (r - l + 1) * (b - t + 1) > 8192 {
        return Err(Error::ResourceLimit("liquify output tile budget"));
    }
    Ok((l..=r)
        .flat_map(|x| (t..=b).map(move |y| TileCoord::from_signed(x as i32, y as i32)))
        .collect())
}
fn execute(executor: &dyn Executor, op: &Operator) -> Result<Vec<[f32; 4]>> {
    let values = executor
        .execute(op)
        .or_else(|_| Cpu.execute(op))
        .map_err(Error::Storage)?;
    if values.len() != op.count()
        || values
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 2_000_000.)
    {
        return Err(Error::ResourceLimit("invalid compute result"));
    }
    Ok(values)
}
impl Document {
    pub(super) fn elastic_liquify_dab(&mut self, options: LiquifyDab) -> Result<bool> {
        options.validate()?;
        let mut state = self
            .liquify
            .as_ref()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?
            .clone();
        let at = self.layer_index(state.original.id)?;
        let radius = f64::from(options.radius);
        let distance = (options.to.x - options.from.x).hypot(options.to.y - options.from.y);
        let steps = (distance / (radius * 0.08).max(0.5)).ceil().max(1.) as usize;
        let area = ((radius * 2. + 2.).min(f64::from(self.width))
            * (radius * 2. + 2.).min(f64::from(self.height))) as usize;
        if steps > 4096 || steps.saturating_mul(area) > MAX_VISITS {
            return Err(Error::ResourceLimit("liquify input work budget"));
        }
        let mut layer = self.layers[at].clone();
        let ox = i64::from(layer.appearance.offset_x);
        let oy = i64::from(layer.appearance.offset_y);
        let mut previous = options.from;
        let mut changed = false;
        let mut dirty = BTreeSet::new();
        for step in 1..=steps {
            let t = step as f64 / steps as f64;
            let x = options.from.x + (options.to.x - options.from.x) * t;
            let y = options.from.y + (options.to.y - options.from.y) * t;
            let pressure = if options.pressure_enabled {
                options.from.pressure + (options.to.pressure - options.from.pressure) * t as f32
            } else {
                1.
            };
            let strength = options.strength * pressure;
            let dx = (x - previous.x).clamp(-radius, radius);
            let dy = (y - previous.y).clamp(-radius, radius);
            previous.x = x;
            previous.y = y;
            if strength == 0.
                || (options.tool == 0 && dx == 0. && dy == 0.)
                || ((options.tool == 1 || options.tool == 2) && options.elapsed_ms == 0.)
            {
                continue;
            }
            // Editing is deliberately not intersected with canvas bounds. The field is
            // a signed mesh over document-local coordinates, with identity outside it.
            let cx = x - ox as f64;
            let cy = y - oy as f64;
            let support = radius.max(CELL as f64);
            let left = ((cx - support).floor() as i64).div_euclid(CELL) * CELL;
            let top = ((cy - support).floor() as i64).div_euclid(CELL) * CELL;
            let right = ((cx + support).ceil() as i64).div_euclid(CELL) * CELL + CELL;
            let bottom = ((cy + support).ceil() as i64).div_euclid(CELL) * CELL + CELL;
            // Only the inverse step and the stiffness stencil read neighbours.
            // A brush-sized halo would prepare four times as many mesh nodes.
            let time = options.elapsed_ms / 1000. / steps as f32;
            let reach = match options.tool {
                0 => dx.abs().max(dy.abs()) * f64::from(strength),
                1 => support * (1. - (-f64::from(strength * options.speed * time * 2.)).exp()),
                2 => support * f64::from(strength * options.speed * time) * std::f64::consts::TAU,
                _ => 0.,
            };
            let margin = reach.ceil() as i64 + CELL * 3;
            let input = grid(
                &state.field,
                [left - margin, top - margin, right + margin, bottom + margin],
            )?;
            let start = [
                ((left - i64::from(input.origin[0])) / CELL) as u32,
                ((top - i64::from(input.origin[1])) / CELL) as u32,
            ];
            let size = [
                ((right - left) / CELL) as u32,
                ((bottom - top) / CELL) as u32,
            ];
            let mut selection = Vec::with_capacity(size[0] as usize * size[1] as usize);
            for gy in 0..size[1] {
                for gx in 0..size[0] {
                    selection.push(self.selection.coverage(
                        (left + i64::from(gx) * CELL + ox) as f64 + 0.5,
                        (top + i64::from(gy) * CELL + oy) as f64 + 0.5,
                    ));
                }
            }
            let op = Operator::Elastic(ElasticOp {
                grid: input,
                start,
                size,
                center: [cx as f32, cy as f32],
                delta: [dx as f32, dy as f32],
                radius: support as f32,
                strength,
                density: options.density,
                time,
                speed: options.speed,
                tool: options.tool,
                selection,
            });
            let values = execute(self.compute.as_ref(), &op)?;
            let mut updated = BTreeMap::<TileCoord, Vec<[f32; 2]>>::new();
            for (i, value) in values.iter().enumerate() {
                let gx = left / CELL + (i as i64 % i64::from(size[0]));
                let gy = top / CELL + (i as i64 / i64::from(size[0]));
                let key = coord(gx, gy);
                let offset = index(gx, gy);
                let old = state.field.get(&key).map_or([0.; 2], |v| v[offset]);
                let value = [value[0], value[1]];
                if old != value {
                    updated.entry(key).or_insert_with(|| {
                        state
                            .field
                            .get(&key)
                            .map_or_else(|| vec![[0.; 2]; 4096], |v| v.as_ref().clone())
                    })[offset] = value;
                }
            }
            if updated.is_empty() {
                continue;
            }
            let additional = updated
                .keys()
                .filter(|k| !state.field.contains_key(k))
                .count();
            if state.field.len() + additional > MAX_FIELD_TILES {
                return Err(Error::ResourceLimit("liquify displacement memory"));
            }
            for (key, values) in updated {
                if values.iter().all(|v| *v == [0.; 2]) {
                    state.field.remove(&key);
                } else {
                    state.field.insert(key, Arc::new(values));
                }
            }
            dirty.extend(keys([
                left - CELL * 2,
                top - CELL * 2,
                right + CELL * 2,
                bottom + CELL * 2,
            ])?);
            if dirty.len() > 8192 {
                return Err(Error::ResourceLimit("liquify output tile budget"));
            }
            changed = true;
        }
        if !changed {
            return Ok(false);
        }
        // Small brushes retain full preview precision. Large brushes use a 2x proxy;
        // final sampling always reads the immutable source at full pixel resolution.
        let stride = if options.radius >= 64. { 2 } else { 1 };
        // The viewport is clipped, but the editing field and immutable source are not.
        // Off-canvas pixels are only materialized during the full-quality commit.
        let preview_tiles = dirty
            .into_iter()
            .filter(|key| {
                let x = i64::from(key.signed_x()) * 64 + ox;
                let y = i64::from(key.signed_y()) * 64 + oy;
                x < i64::from(self.width) && y < i64::from(self.height) && x + 64 > 0 && y + 64 > 0
            })
            .collect::<Vec<_>>();
        self.remap_liquify(&state, &mut layer, &preview_tiles, stride, false)?;
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
        self.layers[at] = layer;
        self.liquify = Some(state);
        self.dirty_all();
        Ok(true)
    }
    pub(super) fn final_liquify_layer(&self) -> Result<Layer> {
        let state = self
            .liquify
            .as_ref()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?;
        let current = self.layer(state.original.id).expect("liquify layer");
        let mut result = state.original.clone();
        let mut affected = current
            .tiles
            .keys()
            .chain(state.original.tiles.keys())
            .filter(
                |key| match (current.tiles.get(key), state.original.tiles.get(key)) {
                    (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
                    (None, None) => false,
                    _ => true,
                },
            )
            .copied()
            .collect::<BTreeSet<_>>();
        for (key, values) in &state.field {
            let mut bounds = [64_i64, 64, 0, 0];
            for (i, value) in values.iter().enumerate().filter(|(_, v)| **v != [0.; 2]) {
                let _ = value;
                let x = (i % 64) as i64;
                let y = (i / 64) as i64;
                bounds = [
                    bounds[0].min(x),
                    bounds[1].min(y),
                    bounds[2].max(x + 1),
                    bounds[3].max(y + 1),
                ];
            }
            if bounds[2] > 0 {
                let x = i64::from(key.signed_x()) * 64;
                let y = i64::from(key.signed_y()) * 64;
                affected.extend(keys([
                    (x + bounds[0] - 1) * CELL,
                    (y + bounds[1] - 1) * CELL,
                    (x + bounds[2] + 1) * CELL,
                    (y + bounds[3] + 1) * CELL,
                ])?);
                if affected.len() > 8192 {
                    return Err(Error::ResourceLimit("liquify final tile budget"));
                }
            }
        }
        self.remap_liquify(
            state,
            &mut result,
            &affected.into_iter().collect::<Vec<_>>(),
            1,
            true,
        )?;
        Ok(result)
    }
    fn remap_liquify(
        &self,
        state: &LiquifyState,
        layer: &mut Layer,
        tiles: &[TileCoord],
        stride: u32,
        cubic: bool,
    ) -> Result<()> {
        for batch in tiles.chunks(BATCH_TILES) {
            let left = batch
                .iter()
                .map(|k| i64::from(k.signed_x()) * 64)
                .min()
                .expect("batch");
            let top = batch
                .iter()
                .map(|k| i64::from(k.signed_y()) * 64)
                .min()
                .expect("batch");
            let right = batch
                .iter()
                .map(|k| i64::from(k.signed_x()) * 64 + 64)
                .max()
                .expect("batch");
            let bottom = batch
                .iter()
                .map(|k| i64::from(k.signed_y()) * 64 + 64)
                .max()
                .expect("batch");
            let input = grid(&state.field, [left, top, right, bottom])?;
            let maximum = input.vectors.iter().fold([0_f32; 2], |m, v| {
                [m[0].max(v[0].abs()), m[1].max(v[1].abs())]
            });
            let source_bounds = [
                (left - maximum[0].ceil() as i64 - 3).div_euclid(64),
                (top - maximum[1].ceil() as i64 - 3).div_euclid(64),
                (right + maximum[0].ceil() as i64 + 3).div_euclid(64),
                (bottom + maximum[1].ceil() as i64 + 3).div_euclid(64),
            ];
            let mut source = Vec::new();
            for (key, tile) in &state.original.tiles {
                let x = i64::from(key.signed_x());
                let y = i64::from(key.signed_y());
                if x < source_bounds[0]
                    || y < source_bounds[1]
                    || x > source_bounds[2]
                    || y > source_bounds[3]
                {
                    continue;
                }
                if source.len() >= paint_ops::MAX_SOURCE_TILES {
                    return Err(Error::ResourceLimit("liquify source staging budget"));
                }
                source.push(SourceTile {
                    coord: [key.signed_x(), key.signed_y()],
                    pixels: tile.try_pixels()?.iter().map(|p| p.components()).collect(),
                });
            }
            source.sort_by_key(|t| t.coord);
            let mut selection = Vec::new();
            if self.selection.enabled() {
                selection.reserve(batch.len() * 4096);
                for key in batch {
                    for y in 0..64 {
                        for x in 0..64 {
                            selection.push(self.selection.coverage(
                                (i64::from(key.signed_x()) * 64
                                    + x
                                    + i64::from(layer.appearance.offset_x))
                                    as f64
                                    + 0.5,
                                (i64::from(key.signed_y()) * 64
                                    + y
                                    + i64::from(layer.appearance.offset_y))
                                    as f64
                                    + 0.5,
                            ));
                        }
                    }
                }
            }
            let op = Operator::Remap(RemapOp {
                grid: input,
                source,
                white: [state.original.white.0, state.original.white.1],
                tiles: batch.iter().map(|k| [k.signed_x(), k.signed_y()]).collect(),
                selection,
                stride,
                cubic,
            });
            let values = execute(self.compute.as_ref(), &op)?;
            for (key, rgba) in batch.iter().zip(values.chunks_exact(4096)) {
                let pixels = rgba
                    .iter()
                    .map(|v| Pixel::from_premultiplied(*v))
                    .collect::<Result<Vec<_>>>()?;
                // Identity restores source tile identities, including explicit transparent
                // overrides on implicit white layers. Empty off-canvas output is sparse.
                let same = if let Some(original) = state.original.tiles.get(key) {
                    original.try_pixels()?.as_ref().as_ref() == pixels.as_slice()
                } else {
                    pixels.iter().enumerate().all(|(i, p)| {
                        *p == state.original.default_pixel(
                            i64::from(key.signed_x()) * 64 + (i % 64) as i64,
                            i64::from(key.signed_y()) * 64 + (i / 64) as i64,
                        )
                    })
                };
                if same {
                    if let Some(original) = state.original.tiles.get(key) {
                        layer.tiles.insert(*key, original.clone());
                    } else {
                        layer.tiles.remove(key);
                    }
                    continue;
                }
                let tile = Arc::new(Tile::from_pixels(pixels)?);
                if tile.is_empty() && layer.white == (0, 0) {
                    layer.tiles.remove(key);
                } else {
                    let others = self.tile_count()
                        - self
                            .layer(state.original.id)
                            .expect("liquify layer")
                            .tiles
                            .len();
                    if !layer.tiles.contains_key(key)
                        && others + layer.tiles.len() >= self.options.max_document_tiles
                    {
                        return Err(Error::ResourceLimit("liquify pixel tile budget"));
                    }
                    self.pool.attach(&tile)?;
                    layer.tiles.insert(*key, tile);
                }
                self.pool.trim()?;
            }
        }
        Ok(())
    }
}
