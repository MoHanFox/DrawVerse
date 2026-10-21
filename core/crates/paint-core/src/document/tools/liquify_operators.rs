//! Elastic mesh editing is independent of the clipped viewport and preview pixels.
use super::*;
use paint_ops::{Cpu, ElasticOp, Executor, Grid, Operator, RemapOp, SourceTile};

const BATCH_TILES: usize = 128;

fn mesh_cell(radius: f32) -> i64 {
    let mut cell = 2;
    while cell < 128 && radius > (cell * 64) as f32 {
        cell *= 2;
    }
    cell
}

// Refine only when a smaller brush needs more detail. Interpolation preserves the
// existing membrane; histories retain both resolutions, including off-canvas nodes.
fn refine(state: &mut LiquifyState, cell: i64) -> Result<()> {
    let mut output = BTreeSet::new();
    for key in state.field.keys() {
        let x = i64::from(key.signed_x()) * 64 * state.cell;
        let y = i64::from(key.signed_y()) * 64 * state.cell;
        output.extend(keys(
            [
                (x - state.cell).div_euclid(cell),
                (y - state.cell).div_euclid(cell),
                (x + 64 * state.cell).div_euclid(cell) + 1,
                (y + 64 * state.cell).div_euclid(cell) + 1,
            ],
            MAX_FIELD_TILES,
        )?);
        if output.len() * 4096 > MAX_VISITS {
            return Err(Error::ResourceLimit("liquify refinement budget"));
        }
    }
    let mut field = Field::new();
    for key in output {
        let x = i64::from(key.signed_x()) * 64 * cell;
        let y = i64::from(key.signed_y()) * 64 * cell;
        let old = grid(
            &state.field,
            [x, y, x + 64 * cell, y + 64 * cell],
            state.cell,
        )?;
        let values = (0..4096)
            .map(|i| old.sample((x + i % 64 * cell) as f32, (y + i / 64 * cell) as f32))
            .collect::<Vec<_>>();
        if values.iter().any(|v| *v != [0.; 2]) {
            field.insert(key, Arc::new(values));
        }
    }
    state.field = field;
    state.cell = cell;
    Ok(())
}

fn grid(field: &Field, rect: [i64; 4], cell: i64) -> Result<Grid> {
    let [left, top, right, bottom] = rect;
    let gx = left.div_euclid(cell) - 2;
    let gy = top.div_euclid(cell) - 2;
    let width = (right.div_euclid(cell) - gx + 3) as u32;
    let height = (bottom.div_euclid(cell) - gy + 3) as u32;
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
        origin: [(gx * cell) as i32, (gy * cell) as i32],
        width,
        height,
        cell: cell as f32,
        vectors,
    })
}
fn keys(rect: [i64; 4], limit: usize) -> Result<Vec<TileCoord>> {
    let [l, t, r, b] = rect;
    if r <= l || b <= t {
        return Ok(Vec::new());
    }
    let (l, t, r, b) = (
        l.div_euclid(64),
        t.div_euclid(64),
        (r - 1).div_euclid(64),
        (b - 1).div_euclid(64),
    );
    if r < l || b < t {
        return Ok(Vec::new());
    }
    if (r - l + 1) * (b - t + 1) > limit as i64 {
        return Err(Error::ResourceLimit("liquify document tile quota"));
    }
    Ok((l..=r)
        .flat_map(|x| (t..=b).map(move |y| TileCoord::from_signed(x as i32, y as i32)))
        .collect())
}

fn batch_bounds(batch: &[TileCoord]) -> [i64; 4] {
    let mut bounds = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
    for key in batch {
        let x = i64::from(key.signed_x()) * 64;
        let y = i64::from(key.signed_y()) * 64;
        bounds = [
            bounds[0].min(x),
            bounds[1].min(y),
            bounds[2].max(x + 64),
            bounds[3].max(y + 64),
        ];
    }
    bounds
}

type RemapPlan = (Grid, Vec<TileCoord>);
fn plan_remap(
    state: &LiquifyState,
    batch: &[TileCoord],
    limit: usize,
) -> Result<Option<RemapPlan>> {
    let [left, top, right, bottom] = batch_bounds(batch);
    // Sparse output can span a huge rectangle; split before allocating its grid.
    let nodes =
        ((right - left).div_euclid(state.cell) + 6) * ((bottom - top).div_euclid(state.cell) + 6);
    if nodes > paint_ops::MAX_SAMPLES as i64 {
        return Ok(None);
    }
    let input = grid(&state.field, [left, top, right, bottom], state.cell)?;
    // Include identity/source-at-output reads, selection blending and cubic support.
    let extrema = input.vectors.iter().fold([0_f32; 4], |m, v| {
        [
            m[0].min(v[0]),
            m[1].min(v[1]),
            m[2].max(v[0]),
            m[3].max(v[1]),
        ]
    });
    let bounds = [
        (left + extrema[0].floor() as i64 - 3).div_euclid(64),
        (top + extrema[1].floor() as i64 - 3).div_euclid(64),
        (right + extrema[2].ceil() as i64 + 3).div_euclid(64),
        (bottom + extrema[3].ceil() as i64 + 3).div_euclid(64),
    ];
    let mut source = Vec::new();
    for key in state.original.tiles.keys() {
        let (x, y) = (i64::from(key.signed_x()), i64::from(key.signed_y()));
        if x >= bounds[0] && y >= bounds[1] && x <= bounds[2] && y <= bounds[3] {
            if source.len() == limit {
                return Ok(None);
            }
            source.push(*key);
        }
    }
    Ok(Some((input, source)))
}

fn stream_remap(state: &LiquifyState, op: &RemapOp) -> Result<Vec<[f32; 4]>> {
    let mut cache = BTreeMap::new();
    let mut order = std::collections::VecDeque::new();
    let mut source = |x: i32, y: i32| -> Result<[f32; 4]> {
        let key = TileCoord::from_signed(x.div_euclid(64), y.div_euclid(64));
        let Some(tile) = state.original.tiles.get(&key) else {
            return Ok(state
                .original
                .default_pixel(i64::from(x), i64::from(y))
                .components());
        };
        if !cache.contains_key(&key) {
            if cache.len() == 64 {
                cache.remove(&order.pop_front().expect("source cache"));
            }
            cache.insert(key, tile.try_pixels()?);
            order.push_back(key);
        }
        Ok(cache[&key][(y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize].components())
    };
    (0..op.tiles.len() * 4096)
        .map(|index| op.try_pixel_with(index, &mut source))
        .collect()
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
    pub(super) fn warm_liquify_mesh(&self) -> Result<()> {
        // Warm lazy driver compilation before the first pointer movement. This
        // identity request contains no source colours and never touches the field.
        execute(
            self.compute.as_ref(),
            &Operator::Elastic(ElasticOp {
                grid: Grid {
                    origin: [0, 0],
                    width: 4,
                    height: 4,
                    cell: 2.,
                    vectors: vec![[0.; 2]; 16],
                },
                start: [1, 1],
                size: [1, 1],
                center: [2., 2.],
                delta: [0., 0.],
                radius: 2.,
                strength: 0.,
                density: 0.5,
                time: 0.,
                speed: 1.,
                tool: 0,
                selection: vec![1.],
            }),
        )?;
        Ok(())
    }
    pub(super) fn warm_liquify_sources(&self, state: &mut LiquifyState) -> Result<()> {
        if state.original.tiles.len() > paint_ops::MAX_SOURCE_TILES {
            return Ok(());
        }
        let mut cache = state
            .sources
            .lock()
            .map_err(|_| Error::Storage("liquify source cache poisoned".into()))?;
        for (key, tile) in &state.original.tiles {
            cache.insert(
                *key,
                SourceTile::new(
                    [key.signed_x(), key.signed_y()],
                    tile.try_pixels()?.iter().map(|p| p.components()).collect(),
                ),
            );
        }
        let sources = cache.values().cloned().collect::<Vec<_>>();
        self.compute
            .prepare_sources(&sources)
            .or_else(|_| Cpu.prepare_sources(&sources))
            .map_err(Error::Storage)
    }
    pub(super) fn elastic_liquify_dab(&mut self, options: LiquifyDab) -> Result<bool> {
        options.validate()?;
        let mut state = self
            .liquify
            .as_ref()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?
            .clone();
        if options.tool == 0 && options.from.x == options.to.x && options.from.y == options.to.y {
            return Ok(false);
        }
        let at = self.layer_index(state.original.id)?;
        let radius = f64::from(options.radius);
        let desired = mesh_cell(options.radius);
        if state.field.is_empty() {
            state.cell = desired;
        } else if desired < state.cell {
            refine(&mut state, desired)?;
        }
        let cell = state.cell;
        let distance = (options.to.x - options.from.x).hypot(options.to.y - options.from.y);
        let steps = (distance / (radius * 0.08).max(0.5)).ceil().max(1.) as usize;
        let deferred = state.preview_source.is_some();
        let area = if deferred {
            0
        } else {
            ((radius * 2. + 2.).min(f64::from(self.width))
                * (radius * 2. + 2.).min(f64::from(self.height))) as usize
        };
        let mesh_area = ((radius * 2. / cell as f64 + 4.).ceil() as usize).pow(2);
        if steps > 4096 || steps.saturating_mul(mesh_area).saturating_add(area) > MAX_VISITS {
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
            let support = radius.max(cell as f64);
            let left = ((cx - support).floor() as i64).div_euclid(cell) * cell;
            let top = ((cy - support).floor() as i64).div_euclid(cell) * cell;
            let right = ((cx + support).ceil() as i64).div_euclid(cell) * cell + cell;
            let bottom = ((cy + support).ceil() as i64).div_euclid(cell) * cell + cell;
            // Only the inverse step and the stiffness stencil read neighbours.
            // A brush-sized halo would prepare four times as many mesh nodes.
            let time = options.elapsed_ms / 1000. / steps as f32;
            let reach = match options.tool {
                0 => dx.abs().max(dy.abs()) * f64::from(strength),
                1 => support * (1. - (-f64::from(strength * options.speed * time * 2.)).exp()),
                2 => support * f64::from(strength * options.speed * time) * std::f64::consts::TAU,
                _ => 0.,
            };
            let margin = reach.ceil() as i64 + cell * 3;
            let input = grid(
                &state.field,
                [left - margin, top - margin, right + margin, bottom + margin],
                cell,
            )?;
            let start = [
                ((left - i64::from(input.origin[0])) / cell) as u32,
                ((top - i64::from(input.origin[1])) / cell) as u32,
            ];
            let size = [
                ((right - left) / cell) as u32,
                ((bottom - top) / cell) as u32,
            ];
            let mut selection = if self.selection.enabled() {
                vec![0.; size[0] as usize * size[1] as usize]
            } else {
                vec![1.; size[0] as usize * size[1] as usize]
            };
            if self.selection.enabled() {
                for (gy, row) in selection.chunks_mut(size[0] as usize).enumerate() {
                    self.selection.coverage_samples(
                        (left + ox) as f64 + 0.5,
                        (top + gy as i64 * cell + oy) as f64 + 0.5,
                        cell as f64,
                        row,
                    );
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
                let gx = left / cell + (i as i64 % i64::from(size[0]));
                let gy = top / cell + (i as i64 / i64::from(size[0]));
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
            if !deferred {
                dirty.extend(keys(
                    [
                        (left - cell * 2).max(-ox),
                        (top - cell * 2).max(-oy),
                        (right + cell * 2).min(i64::from(self.width) - ox),
                        (bottom + cell * 2).min(i64::from(self.height) - oy),
                    ],
                    self.options.max_document_tiles,
                )?);
            }
            if dirty.len() > self.options.max_document_tiles {
                return Err(Error::ResourceLimit("liquify document tile quota"));
            }
            changed = true;
        }
        if !changed {
            return Ok(false);
        }
        // Small brushes retain full preview precision. Large brushes use a 2x/4x proxy;
        // final sampling always reads the immutable source at full pixel resolution.
        let stride = if options.radius >= 256. {
            4
        } else if options.radius >= 64. {
            2
        } else {
            1
        };
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
        if !deferred {
            self.remap_liquify(&state, &mut layer, &preview_tiles, stride, false)?;
        }
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
        let cell = state.cell;
        let current = self.layer(state.original.id).expect("liquify layer");
        let mut result = state.original.clone();
        let source = super::preview::source_bounds(&state.original);
        let mut extrema = [0_f32; 4];
        for v in state.field.values().flat_map(|tile| tile.iter()) {
            extrema[0] = extrema[0].min(v[0]);
            extrema[1] = extrema[1].min(v[1]);
            extrema[2] = extrema[2].max(v[0]);
            extrema[3] = extrema[3].max(v[1]);
        }
        // An inverse sample can only contain colour if p + D(p) reaches the
        // immutable source. Include cubic support and retained negative tiles.
        let reachable = [
            source[0] - extrema[2].ceil() as i64 - 3,
            source[1] - extrema[3].ceil() as i64 - 3,
            source[2] - extrema[0].floor() as i64 + 3,
            source[3] - extrema[1].floor() as i64 + 3,
        ];
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
                affected.extend(keys(
                    [
                        ((x + bounds[0] - 1) * cell).max(reachable[0]),
                        ((y + bounds[1] - 1) * cell).max(reachable[1]),
                        ((x + bounds[2] + 1) * cell).min(reachable[2]),
                        ((y + bounds[3] + 1) * cell).min(reachable[3]),
                    ],
                    self.options.max_document_tiles,
                )?);
                if affected.len() > self.options.max_document_tiles {
                    return Err(Error::ResourceLimit("liquify document tile quota"));
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
        self.progress_stage(ComputationPhase::Resampling, tiles.len());
        let mut offset = 0;
        while offset < tiles.len() {
            let (count, plan) =
                paint_ops::batching::plan_batch(&tiles[offset..], BATCH_TILES, |batch| {
                    plan_remap(state, batch, paint_ops::MAX_SOURCE_TILES)
                })?;
            let batch = &tiles[offset..offset + count];
            offset += count;
            let streaming = plan.is_none();
            let (input, source_keys) = match plan {
                Some(plan) => plan,
                None => (
                    grid(&state.field, batch_bounds(batch), state.cell)?,
                    Vec::new(),
                ),
            };
            let mut source = Vec::with_capacity(source_keys.len());
            if !streaming {
                let mut cached = state
                    .sources
                    .lock()
                    .map_err(|_| Error::Storage("liquify source cache poisoned".into()))?;
                for key in source_keys {
                    if !cached.contains_key(&key) {
                        // Evict one entry instead of flushing the complete cache.
                        if cached.len() >= paint_ops::MAX_SOURCE_TILES {
                            cached.pop_first();
                        }
                        cached.insert(
                            key,
                            SourceTile::new(
                                [key.signed_x(), key.signed_y()],
                                state.original.tiles[&key]
                                    .try_pixels()?
                                    .iter()
                                    .map(|p| p.components())
                                    .collect(),
                            ),
                        );
                    }
                    source.push(cached[&key].clone());
                }
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
            let compact = !streaming && stride > 1 && selection.is_empty() && !cubic;
            let remap = RemapOp {
                grid: input,
                source,
                white: [state.original.white.0, state.original.white.1],
                tiles: batch.iter().map(|k| [k.signed_x(), k.signed_y()]).collect(),
                selection,
                stride,
                cubic,
            };
            let values = if streaming {
                stream_remap(state, &remap)?
            } else {
                let op = if compact {
                    Operator::RemapBlocks(remap)
                } else {
                    Operator::Remap(remap)
                };
                execute(self.compute.as_ref(), &op)?
            };
            let samples = if compact {
                4096 / (stride * stride) as usize
            } else {
                4096
            };
            for (key, rgba) in batch.iter().zip(values.chunks_exact(samples)) {
                let pixels = if compact {
                    let mut pixels = if let Some(original) = state.original.tiles.get(key) {
                        original.try_pixels()?.to_vec()
                    } else {
                        (0..4096)
                            .map(|i| {
                                state.original.default_pixel(
                                    i64::from(key.signed_x()) * 64 + i % 64,
                                    i64::from(key.signed_y()) * 64 + i / 64,
                                )
                            })
                            .collect()
                    };
                    let stride = stride as usize;
                    let side = 64 / stride;
                    for (i, color) in rgba.iter().enumerate() {
                        if *color == [0., 0., 0., -1.] {
                            continue;
                        }
                        let pixel = Pixel::from_premultiplied(*color)?;
                        let x = i % side * stride;
                        let y = i / side * stride;
                        for row in y..y + stride {
                            pixels[row * 64 + x..row * 64 + x + stride].fill(pixel);
                        }
                    }
                    pixels
                } else {
                    rgba.iter()
                        .map(|v| Pixel::from_premultiplied(*v))
                        .collect::<Result<Vec<_>>>()?
                };
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
            self.progress_advance(batch.len());
        }
        Ok(())
    }
}

#[cfg(test)]
mod adaptive_tests {
    use super::*;
    #[test]
    fn large_source_footprints_split_then_stream_with_exact_sampling() {
        let mut d = Document::new(3072, 3072).unwrap();
        d.begin_liquify().unwrap();
        let mut state = d.liquify.as_ref().unwrap().clone();
        let pixels = (0..4096)
            .map(|i| {
                Pixel::from_straight([(i % 64) as f32 / 64., (i / 64) as f32 / 64., 0.2, 0.75])
                    .unwrap()
            })
            .collect();
        let tile = Arc::new(Tile::from_pixels(pixels).unwrap());
        state.original.tiles = (0..48)
            .flat_map(|x| (0..48).map(move |y| TileCoord { x, y }))
            .map(|key| (key, tile.clone()))
            .collect();
        // A single output tile's conservative footprint exceeds the real 2048
        // source staging budget; only a few source tiles are sampled in fact.
        state.field.insert(
            TileCoord { x: 0, y: 0 },
            Arc::new(vec![[3000.25, 3000.75]; 4096]),
        );
        let output = [TileCoord { x: 0, y: 0 }];
        assert!(plan_remap(&state, &output, paint_ops::MAX_SOURCE_TILES)
            .unwrap()
            .is_none());
        let reference_source = state
            .original
            .tiles
            .iter()
            .filter(|(key, _)| **key == output[0] || (key.x >= 46 && key.y >= 46))
            .map(|(key, tile)| {
                SourceTile::new(
                    [key.signed_x(), key.signed_y()],
                    tile.try_pixels()
                        .unwrap()
                        .iter()
                        .map(|p| p.components())
                        .collect(),
                )
            })
            .collect();
        let mut op = RemapOp {
            grid: grid(&state.field, batch_bounds(&output), state.cell).unwrap(),
            source: reference_source,
            white: [0, 0],
            tiles: vec![[0, 0]],
            selection: Vec::new(),
            stride: 1,
            cubic: false,
        };
        for (stride, cubic, selected) in [(1, false, false), (1, true, true), (4, false, false)] {
            op.stride = stride;
            op.cubic = cubic;
            op.selection = if selected {
                (0..4096).map(|i| (i % 5) as f32 / 4.).collect()
            } else {
                Vec::new()
            };
            let expected = Cpu.execute(&Operator::Remap(op.clone())).unwrap();
            assert_eq!(stream_remap(&state, &op).unwrap(), expected);
        }
        let mut result = state.original.clone();
        d.remap_liquify(&state, &mut result, &output, 1, true)
            .unwrap();
        op.stride = 1;
        op.cubic = true;
        op.selection.clear();
        let expected = Cpu.execute(&Operator::Remap(op)).unwrap();
        assert_eq!(
            result.tiles[&output[0]]
                .try_pixels()
                .unwrap()
                .iter()
                .map(|p| p.components())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(
            state.sources.lock().unwrap().is_empty(),
            "streaming must not fill the staging cache"
        );

        // A normal large batch is divided rather than rejecting the operation.
        state.field.clear();
        let work = (0..128).map(|y| TileCoord { x: 0, y }).collect::<Vec<_>>();
        let (count, plan) =
            paint_ops::batching::plan_batch(&work, 128, |batch| plan_remap(&state, batch, 64))
                .unwrap();
        assert!(count < 128 && plan.is_some());
    }

    #[test]
    fn eight_and_sixteen_k_output_use_document_quota_not_batch_size() {
        for side in [8192, 16384] {
            let tiles = keys([0, 0, side, side], 1_000_000).unwrap();
            assert_eq!(tiles.len(), (side / 64 * side / 64) as usize);
            assert!(tiles.len() > 8192);
            assert!(keys([0, 0, side, side], tiles.len() - 1).is_err());
        }
    }
    #[test]
    fn large_mesh_refines_without_losing_outside_field_or_history() {
        let mut d = Document::new(96, 96).unwrap();
        d.begin_liquify().unwrap();
        d.begin_liquify_stroke().unwrap();
        let mut dab = LiquifyDab {
            tool: 0,
            radius: 1000.,
            strength: 0.6,
            density: 0.7,
            speed: 1.,
            pressure_enabled: false,
            from: InputPoint::new(48., 48., 1.),
            to: InputPoint::new(57., 52., 1.),
            elapsed_ms: 16.,
        };
        d.liquify_dab(dab).unwrap();
        d.end_liquify_stroke().unwrap();
        let before = d.liquify.as_ref().unwrap().clone();
        assert_eq!(before.cell, 16);
        assert!(
            before.field.len() <= 16,
            "large brush must not allocate a pixel-sized mesh"
        );
        d.begin_liquify_stroke().unwrap();
        dab.radius = 24.;
        dab.to = InputPoint::new(60., 54., 1.);
        d.liquify_dab(dab).unwrap();
        d.end_liquify_stroke().unwrap();
        let after = d.liquify.as_ref().unwrap().clone();
        assert_eq!(after.cell, 2);
        let rect = [-440, -440, -350, -350];
        let a = grid(&before.field, rect, before.cell)
            .unwrap()
            .sample(-400., -400.);
        let b = grid(&after.field, rect, after.cell)
            .unwrap()
            .sample(-400., -400.);
        assert_eq!(a, b, "refinement must preserve the off-canvas membrane");
        d.undo_liquify().unwrap();
        assert_eq!(d.liquify.as_ref().unwrap().cell, before.cell);
        assert_eq!(d.liquify.as_ref().unwrap().field, before.field);
        d.redo_liquify().unwrap();
        assert_eq!(d.liquify.as_ref().unwrap().cell, after.cell);
        assert_eq!(d.liquify.as_ref().unwrap().field, after.field);
    }
}
