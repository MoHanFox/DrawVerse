use super::*;
const MAX_FIELD_TILES: usize = 8192;
const MAX_VISITS: usize = 8_000_000;
type Field = BTreeMap<TileCoord, Arc<Vec<[f32; 2]>>>;
#[derive(Clone, Debug)]
pub(super) struct LiquifyState {
    original: Layer,
    field: Field,
    focus: bool,
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
fn field_at(field: &Field, x: i64, y: i64) -> [f32; 2] {
    field.get(&coord(x, y)).map_or([0., 0.], |t| t[index(x, y)])
}
fn sample_field(field: &Field, x: f64, y: f64) -> [f32; 2] {
    let ix = x.floor() as i64;
    let iy = y.floor() as i64;
    let fx = (x - ix as f64) as f32;
    let fy = (y - iy as f64) as f32;
    let a = field_at(field, ix, iy);
    let b = field_at(field, ix + 1, iy);
    let c = field_at(field, ix, iy + 1);
    let d = field_at(field, ix + 1, iy + 1);
    [0, 1].map(|k| (a[k] * (1. - fx) + b[k] * fx) * (1. - fy) + (c[k] * (1. - fx) + d[k] * fx) * fy)
}
struct Source<'a> {
    layer: &'a Layer,
    pins: BTreeMap<TileCoord, Arc<Box<[Pixel]>>>,
}
impl Source<'_> {
    fn pixel(&mut self, x: i64, y: i64) -> Result<Pixel> {
        let key = coord(x, y);
        if let Some(tile) = self.layer.tiles.get(&key) {
            if !self.pins.contains_key(&key) {
                if self.pins.len() >= 256 {
                    self.pins.pop_first();
                }
                self.pins.insert(key, tile.try_pixels()?);
            }
            Ok(self.pins[&key][index(x, y)])
        } else {
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
        let top = a.mix(self.pixel(ix + 1, iy)?, fx);
        let bottom = self.pixel(ix, iy + 1)?.mix(self.pixel(ix + 1, iy + 1)?, fx);
        Ok(top.mix(bottom, fy))
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
        if original.group {
            return Err(Error::InvalidArgument("liquify needs raster layer or mask"));
        }
        self.liquify = Some(LiquifyState {
            original,
            field: Field::new(),
            focus: false,
        });
        Ok(())
    }
    pub fn reset_liquify(&mut self) -> Result<()> {
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
    pub fn cancel_liquify(&mut self) -> Result<()> {
        self.reset_liquify()?;
        self.liquify = None;
        Ok(())
    }
    pub fn commit_liquify(&mut self) -> Result<bool> {
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
    pub fn liquify_dab(&mut self, options: LiquifyDab) -> Result<bool> {
        options.validate()?;
        let mut state = self
            .liquify
            .as_ref()
            .ok_or(Error::InvalidArgument("no liquify transaction"))?
            .clone();
        let at = self.layer_index(state.original.id)?;
        let mut layer = self.layers[at].clone();
        let radius = f64::from(options.radius);
        let distance = (options.to.x - options.from.x).hypot(options.to.y - options.from.y);
        let steps = (distance / (radius * 0.2).max(1.)).ceil().max(1.) as usize;
        let area = ((radius * 2. + 2.).min(f64::from(self.width))
            * (radius * 2. + 2.).min(f64::from(self.height))) as usize;
        if steps > 4096 || steps.saturating_mul(area) > MAX_VISITS {
            return Err(Error::ResourceLimit("liquify input work budget"));
        }
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
            let old_field = state.field.clone();
            let mut fields = BTreeMap::<TileCoord, Vec<[f32; 2]>>::new();
            let mut pixels = BTreeMap::<TileCoord, Vec<Pixel>>::new();
            let mut source = Source {
                layer: &state.original,
                pins: BTreeMap::new(),
            };
            for py in top..bottom {
                let mut coverage = vec![0.; (right - left).max(0) as usize];
                self.selection
                    .coverage_span(left as f64, py as f64, &mut coverage);
                for px in left..right {
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
                    let weight = amount * falloff * f64::from(coverage[(px - left) as usize]);
                    if weight <= 0. {
                        continue;
                    }
                    let lx = px - ox;
                    let ly = py - oy;
                    let key = coord(lx, ly);
                    let i = index(lx, ly);
                    let before = field_at(&old_field, lx, ly);
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
                        let sampled = sample_field(&old_field, lx as f64 + tx, ly as f64 + ty);
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
                    if !fields.contains_key(&key) {
                        if old_field.len()
                            + fields.keys().filter(|k| !old_field.contains_key(k)).count()
                            >= MAX_FIELD_TILES
                            && !old_field.contains_key(&key)
                        {
                            return Err(Error::ResourceLimit("liquify displacement memory"));
                        }
                        fields.insert(
                            key,
                            old_field
                                .get(&key)
                                .map_or_else(|| vec![[0., 0.]; 4096], |v| v.as_ref().clone()),
                        );
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
                        pixels.insert(key, original);
                    }
                    fields.get_mut(&key).expect("field")[i] = after;
                    pixels.get_mut(&key).expect("pixels")[i] = source.sample(
                        lx as f64 + f64::from(after[0]),
                        ly as f64 + f64::from(after[1]),
                    )?;
                }
            }
            drop(source);
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
                    self.pool.trim()?;
                }
            }
        }
        if changed {
            self.layers[at] = layer;
            self.liquify = Some(state);
            self.dirty_all();
        }
        Ok(changed)
    }
}
