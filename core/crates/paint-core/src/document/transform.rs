use super::*;
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub struct TransformGeometry {
    /// Row-major 4x4 control points in document coordinates.
    pub points: [[f64; 2]; 16],
    pub warp: bool,
    pub skew_h: f64,
    pub skew_v: f64,
    /// Nearest, linear, cubic, smoother, sharper, automatic.
    pub interpolation: u32,
}
impl TransformGeometry {
    pub fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            points: std::array::from_fn(|i| {
                [x + w * (i % 4) as f64 / 3., y + h * (i / 4) as f64 / 3.]
            }),
            warp: false,
            skew_h: 0.,
            skew_v: 0.,
            interpolation: 2,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if !self.skew_h.is_finite()
            || !self.skew_v.is_finite()
            || self.skew_h.abs() >= 90.
            || self.skew_v.abs() >= 90.
            || self.interpolation > 5
            || self
                .points
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 2_000_000.)
        {
            return Err(Error::InvalidArgument(
                "transform coordinates/interpolation",
            ));
        }
        if !self.warp {
            let mut sign = 0.;
            let q = [
                self.points[0],
                self.points[3],
                self.points[15],
                self.points[12],
            ];
            for i in 0..4 {
                let a = q[i];
                let b = q[(i + 1) % 4];
                let c = q[(i + 2) % 4];
                let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
                if cross.abs() < 1e-8 || (sign != 0. && cross * sign < 0.) {
                    return Err(Error::InvalidArgument("degenerate or crossed transform"));
                }
                sign = cross;
            }
        }
        Ok(())
    }
    pub fn map(&self, u: f64, v: f64) -> [f64; 2] {
        if !self.warp {
            let [a, b, c, d] = [
                self.points[0],
                self.points[3],
                self.points[15],
                self.points[12],
            ];
            let dx = b[0] + d[0] - a[0] - c[0];
            let dy = b[1] + d[1] - a[1] - c[1];
            let bx = c[0] - b[0];
            let by = c[1] - b[1];
            let cx = c[0] - d[0];
            let cy = c[1] - d[1];
            let det = bx * cy - cx * by;
            let (g, h) = if dx.abs() + dy.abs() < 1e-10 {
                (0., 0.)
            } else if det.abs() > 1e-12 {
                ((dx * cy - cx * dy) / det, (bx * dy - dx * by) / det)
            } else {
                (0., 0.)
            };
            let z = 1. + g * u + h * v;
            return [
                ((b[0] - a[0] + g * b[0]) * u + (d[0] - a[0] + h * d[0]) * v + a[0]) / z,
                ((b[1] - a[1] + g * b[1]) * u + (d[1] - a[1] + h * d[1]) * v + a[1]) / z,
            ];
        }
        // Interpolating cubic surface: visible nodes lie on the actual image.
        let (ix, tx) = cell(u);
        let (iy, ty) = cell(v);
        let mut rows = [[0.; 2]; 4];
        for (r, row) in rows.iter_mut().enumerate() {
            let yy = iy + r as i32 - 1;
            for (axis, value) in row.iter_mut().enumerate() {
                let p = std::array::from_fn(|k| self.extended_node(ix + k as i32 - 1, yy, axis));
                *value = catmull(p, tx);
            }
        }
        std::array::from_fn(|axis| catmull(std::array::from_fn(|i| rows[i][axis]), ty))
    }
    fn extended_node(&self, x: i32, y: i32, axis: usize) -> f64 {
        let row = |yy: usize| {
            if x < 0 {
                2. * self.points[yy * 4][axis] - self.points[yy * 4 + 1][axis]
            } else if x > 3 {
                2. * self.points[yy * 4 + 3][axis] - self.points[yy * 4 + 2][axis]
            } else {
                self.points[yy * 4 + x as usize][axis]
            }
        };
        if y < 0 {
            2. * row(0) - row(1)
        } else if y > 3 {
            2. * row(3) - row(2)
        } else {
            row(y as usize)
        }
    }
    fn inverse_matrix(&self) -> Option<[f64; 9]> {
        if self.warp {
            return None;
        }
        let [a, b, c, d] = [
            self.points[0],
            self.points[3],
            self.points[15],
            self.points[12],
        ];
        let dx = b[0] + d[0] - a[0] - c[0];
        let dy = b[1] + d[1] - a[1] - c[1];
        let bx = c[0] - b[0];
        let by = c[1] - b[1];
        let cx = c[0] - d[0];
        let cy = c[1] - d[1];
        let determinant = bx * cy - cx * by;
        let (g, h) = if dx.abs() + dy.abs() < 1e-10 {
            (0., 0.)
        } else {
            (
                (dx * cy - cx * dy) / determinant,
                (bx * dy - dx * by) / determinant,
            )
        };
        let m = [
            b[0] - a[0] + g * b[0],
            d[0] - a[0] + h * d[0],
            a[0],
            b[1] - a[1] + g * b[1],
            d[1] - a[1] + h * d[1],
            a[1],
            g,
            h,
            1.,
        ];
        // Adjugate: the common determinant cancels on homogeneous division.
        Some([
            m[4] * m[8] - m[5] * m[7],
            m[2] * m[7] - m[1] * m[8],
            m[1] * m[5] - m[2] * m[4],
            m[5] * m[6] - m[3] * m[8],
            m[0] * m[8] - m[2] * m[6],
            m[2] * m[3] - m[0] * m[5],
            m[3] * m[7] - m[4] * m[6],
            m[1] * m[6] - m[0] * m[7],
            m[0] * m[4] - m[1] * m[3],
        ])
    }
    fn inverse(&self, x: f64, y: f64, guess: [f64; 2]) -> Option<[f64; 2]> {
        let [mut u, mut v] = guess;
        for _ in 0..14 {
            let p = self.map(u, v);
            let dx = x - p[0];
            let dy = y - p[1];
            if dx.abs() + dy.abs() < 1e-5 {
                return (u >= -1e-7 && v >= -1e-7 && u <= 1.0000001 && v <= 1.0000001)
                    .then_some([u.clamp(0., 1.), v.clamp(0., 1.)]);
            }
            let a = self.map(u + 0.0001, v);
            let b = self.map(u, v + 0.0001);
            let ax = (a[0] - p[0]) * 10000.;
            let ay = (a[1] - p[1]) * 10000.;
            let bx = (b[0] - p[0]) * 10000.;
            let by = (b[1] - p[1]) * 10000.;
            let det = ax * by - bx * ay;
            if det.abs() < 1e-9 {
                return None;
            }
            u += (dx * by - bx * dy) / det;
            v += (ax * dy - dx * ay) / det;
            if u.abs() > 4. || v.abs() > 4. {
                return None;
            }
        }
        None
    }
}
fn cell(v: f64) -> (i32, f64) {
    let q = v.clamp(0., 1.) * 3.;
    let i = (q.floor() as i32).min(2);
    (i, q - i as f64)
}
fn catmull(p: [f64; 4], t: f64) -> f64 {
    0.5 * ((2. * p[1])
        + (-p[0] + p[2]) * t
        + (2. * p[0] - 5. * p[1] + 4. * p[2] - p[3]) * t * t
        + (-p[0] + 3. * p[1] - 3. * p[2] + p[3]) * t * t * t)
}

#[derive(Clone, Debug)]
pub struct PlacedImage {
    pub(crate) source: Layer,
    pub(crate) bounds: [f64; 4],
    pub(crate) geometry: TransformGeometry,
}
#[derive(Clone, Debug)]
pub(super) struct TransformState {
    original: Layer,
    source: Arc<PlacedImage>,
    current: TransformGeometry,
    checkpoint: TransformGeometry,
    undo: VecDeque<TransformGeometry>,
    redo: Vec<TransformGeometry>,
    placed: Option<usize>,
    previous_active: u64,
    evicted: bool,
    selection: Option<crate::Selection>,
    extra: Vec<(Layer, Arc<PlacedImage>)>,
    initial: TransformGeometry,
}

fn layer_bounds(layer: &Layer) -> Result<[f64; 4]> {
    let mut b: Option<[i64; 4]> = None;
    if layer.white != (0, 0) {
        b = Some([0, 0, layer.white.0 as i64, layer.white.1 as i64]);
    }
    for (key, tile) in &layer.tiles {
        let pixels = tile.try_pixels()?;
        for (i, p) in pixels.iter().enumerate() {
            if p.alpha() > 0. {
                let x = key.signed_x() as i64 * 64 + (i % 64) as i64;
                let y = key.signed_y() as i64 * 64 + (i / 64) as i64;
                b = Some(b.map_or([x, y, x + 1, y + 1], |b| {
                    [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)]
                }));
            }
        }
    }
    let b = b.ok_or(Error::InvalidArgument("empty layer cannot transform"))?;
    Ok([
        b[0] as f64,
        b[1] as f64,
        (b[2] - b[0]) as f64,
        (b[3] - b[1]) as f64,
    ])
}
struct SourceSampler<'a> {
    source: &'a Layer,
    cache: BTreeMap<TileCoord, Vec<Pixel>>,
    order: VecDeque<TileCoord>,
    selection: Option<&'a crate::Selection>,
    offset: [f64; 2],
}
impl<'a> SourceSampler<'a> {
    fn new(source: &'a Layer) -> Self {
        Self {
            source,
            cache: BTreeMap::new(),
            order: VecDeque::new(),
            selection: None,
            offset: [0., 0.],
        }
    }
    fn covered(&self, p: Pixel, x: i64, y: i64) -> Pixel {
        self.selection.map_or(p, |s| {
            p.scaled(s.coverage(
                x as f64 + 0.5 + self.offset[0],
                y as f64 + 0.5 + self.offset[1],
            ))
        })
    }
    fn pixel(&mut self, x: i64, y: i64) -> Result<Pixel> {
        let key = TileCoord::from_signed(x.div_euclid(64) as i32, y.div_euclid(64) as i32);
        if !self.cache.contains_key(&key) {
            let Some(tile) = self.source.tiles.get(&key) else {
                return Ok(self.covered(self.source.default_pixel(x, y), x, y));
            };
            let pixels = tile.try_pixels()?.to_vec();
            if self.order.len() == 64 {
                self.cache.remove(&self.order.pop_front().expect("cache"));
            }
            self.order.push_back(key);
            self.cache.insert(key, pixels);
        }
        Ok(self.covered(
            self.cache[&key][(y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize],
            x,
            y,
        ))
    }
}
fn cubic_weight(x: f64, a: f64) -> f64 {
    let x = x.abs();
    if a == 0. {
        return if x < 1. {
            (4. - 6. * x * x + 3. * x * x * x) / 6.
        } else if x < 2. {
            (2. - x).powi(3) / 6.
        } else {
            0.
        };
    }
    if x < 1. {
        (a + 2.) * x * x * x - (a + 3.) * x * x + 1.
    } else if x < 2. {
        a * x * x * x - 5. * a * x * x + 8. * a * x - 4. * a
    } else {
        0.
    }
}
fn sample(source: &mut SourceSampler<'_>, x: f64, y: f64, kind: u32, scale: f64) -> Result<Pixel> {
    if kind == 0 {
        return source.pixel((x + 0.5).floor() as i64, (y + 0.5).floor() as i64);
    }
    let ix = x.floor() as i64;
    let iy = y.floor() as i64;
    let fx = x - ix as f64;
    let fy = y - iy as f64;
    if kind == 1 {
        return Ok(source
            .pixel(ix, iy)?
            .mix(source.pixel(ix + 1, iy)?, fx as f32)
            .mix(
                source
                    .pixel(ix, iy + 1)?
                    .mix(source.pixel(ix + 1, iy + 1)?, fx as f32),
                fy as f32,
            ));
    }
    let kind = if kind == 5 {
        if scale < 1. {
            4
        } else {
            3
        }
    } else {
        kind
    };
    let a = match kind {
        3 => 0.,
        4 => -0.75,
        _ => -0.5,
    };
    let mut sum = [0.; 4];
    for dy in -1..=2 {
        for dx in -1..=2 {
            let weight = (cubic_weight(dx as f64 - fx, a) * cubic_weight(dy as f64 - fy, a)) as f32;
            let p = source.pixel(ix + dx, iy + dy)?.components();
            for k in 0..4 {
                sum[k] += p[k] * weight;
            }
        }
    }
    sum[3] = sum[3].clamp(0., 1.);
    for k in 0..3 {
        sum[k] = sum[k].clamp(0., sum[3]);
    }
    Pixel::from_premultiplied(sum)
}
impl Document {
    pub fn transform_active(&self) -> bool {
        self.transform.is_some()
    }
    pub fn transform_geometry(&self) -> Option<TransformGeometry> {
        self.transform.as_ref().map(|s| s.current.clone())
    }
    pub fn transform_history_depth(&self) -> Option<(usize, usize)> {
        self.transform
            .as_ref()
            .map(|s| (s.undo.len(), s.redo.len()))
    }
    pub fn transform_history_actions(&self) -> Option<Vec<crate::HistoryAction>> {
        self.transform.as_ref().map(|s| {
            std::iter::once(if s.evicted {
                crate::HistoryAction::Truncated
            } else {
                crate::HistoryAction::Initial
            })
            .chain(std::iter::repeat_n(
                crate::HistoryAction::Transform,
                s.undo.len() + s.redo.len(),
            ))
            .collect()
        })
    }
    pub fn begin_transform(&mut self) -> Result<()> {
        self.idle()?;
        self.check_ancestors(self.active_layer, crate::LOCK_ALL | crate::LOCK_POSITION)?;
        let original = self
            .layer(self.active_layer)
            .ok_or(Error::LayerNotFound(self.active_layer))?
            .clone();
        if original.group {
            return self.begin_transform_layers(&[original.id]);
        }
        let source = if let Some(p) = &original.placed {
            let mut p = (**p).clone();
            p.geometry.skew_h = 0.;
            p.geometry.skew_v = 0.;
            let a = original.appearance;
            for v in &mut p.geometry.points {
                v[0] += a.offset_x as f64;
                v[1] += a.offset_y as f64;
            }
            Arc::new(p)
        } else {
            let bounds = layer_bounds(&original)?;
            let geometry = TransformGeometry::rectangle(
                bounds[0] + original.appearance.offset_x as f64,
                bounds[1] + original.appearance.offset_y as f64,
                bounds[2],
                bounds[3],
            );
            Arc::new(PlacedImage {
                source: original.clone(),
                bounds,
                geometry,
            })
        };
        let mut source = source;
        if original.mask {
            let owner = self.layer(original.parent).expect("mask owner");
            let p = Arc::make_mut(&mut source);
            for point in &mut p.geometry.points {
                point[0] += owner.appearance.offset_x as f64;
                point[1] += owner.appearance.offset_y as f64;
            }
        }
        let selected = if self.selection.enabled() && !original.is_image() && !original.mask {
            Some(self.selection.clone())
        } else {
            None
        };
        if let Some(selection) = &selected {
            if (self.width as usize).saturating_mul(self.height as usize)
                > crate::MAX_SELECTION_MASK_BYTES
            {
                return Err(Error::ResourceLimit("selection transform coverage"));
            }
            let mut bounds = None::<[u32; 4]>;
            for y in 0..self.height {
                for x in 0..self.width {
                    if selection.coverage(x as f64 + 0.5, y as f64 + 0.5) > 0. {
                        bounds = Some(bounds.map_or([x, y, x + 1, y + 1], |b| {
                            [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)]
                        }));
                    }
                }
            }
            let b = bounds.ok_or(Error::InvalidArgument("empty selection"))?;
            let image = Arc::make_mut(&mut source);
            image.bounds = [
                b[0] as f64 - original.appearance.offset_x as f64,
                b[1] as f64 - original.appearance.offset_y as f64,
                (b[2] - b[0]) as f64,
                (b[3] - b[1]) as f64,
            ];
            image.geometry = TransformGeometry::rectangle(
                b[0] as f64,
                b[1] as f64,
                (b[2] - b[0]) as f64,
                (b[3] - b[1]) as f64,
            );
        }
        let current = source.geometry.clone();
        self.transform = Some(TransformState {
            original,
            source,
            current: current.clone(),
            checkpoint: current.clone(),
            initial: current,
            selection: None,
            extra: Vec::new(),
            undo: VecDeque::new(),
            redo: Vec::new(),
            previous_active: self.active_layer,
            placed: None,
            evicted: false,
        });
        self.transform.as_mut().expect("session").selection = selected;
        Ok(())
    }
    pub fn begin_transform_layers(&mut self, ids: &[u64]) -> Result<()> {
        self.idle()?;
        let roots = self.batch_roots(ids)?;
        if roots.len() == 1
            && !self.layer(roots[0]).expect("root").group
            && self.selection.enabled()
            && !self.layer(roots[0]).expect("root").is_image()
        {
            let active = self.active_layer;
            self.active_layer = roots[0];
            let result = self.begin_transform();
            self.active_layer = active;
            return result;
        }
        let mut targets = Vec::new();
        for id in roots {
            for layer in &self.layers[self.subtree(id)?] {
                if !layer.group && (!layer.tiles.is_empty() || layer.white != (0, 0)) {
                    targets.push(layer.id);
                }
            }
        }
        if targets.is_empty() {
            return Err(Error::InvalidArgument("empty transform targets"));
        }
        let active = self.active_layer;
        let mut states = Vec::new();
        for id in targets {
            self.active_layer = id;
            let selection = std::mem::take(&mut self.selection);
            let result = self.begin_transform();
            self.selection = selection;
            self.active_layer = active;
            result?;
            states.push(self.transform.take().expect("created transform"));
        }
        let mut first = states.remove(0);
        if !states.is_empty() {
            first.extra = states.into_iter().map(|s| (s.original, s.source)).collect();
            let mut b = [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ];
            for source in std::iter::once(&first.source).chain(first.extra.iter().map(|(_, s)| s)) {
                for p in source.geometry.points {
                    b = [
                        b[0].min(p[0]),
                        b[1].min(p[1]),
                        b[2].max(p[0]),
                        b[3].max(p[1]),
                    ];
                }
            }
            first.current = TransformGeometry::rectangle(b[0], b[1], b[2] - b[0], b[3] - b[1]);
            first.initial = first.current.clone();
            first.checkpoint = first.current.clone();
        }
        first.previous_active = active;
        self.transform = Some(first);
        Ok(())
    }
    pub fn place_image(
        &mut self,
        mut source: Layer,
        width: u32,
        height: u32,
        name: String,
    ) -> Result<u64> {
        self.idle()?;
        if source.group || source.mask || source.placed.is_some() || width == 0 || height == 0 {
            return Err(Error::InvalidArgument("placed raster source"));
        }
        if name.is_empty() || name.len() > 1024 || name.contains('\0') || self.layers.len() >= 4096
        {
            return Err(Error::InvalidArgument("image name/layer limit"));
        }
        let id = self.next_layer_id;
        let next_id = id.checked_add(1).ok_or(Error::ResourceLimit("layer IDs"))?;
        let previous_active = self.active_layer;
        let mut selected = self.layer_index(self.active_layer)?;
        if self.layers[selected].mask {
            selected = self.layer_index(self.layers[selected].parent)?;
        }
        let parent = if self.layers[selected].group {
            self.layers[selected].id
        } else if self.layers[selected].mask {
            self.layers[self.layer_index(self.layers[selected].parent)?].parent
        } else {
            self.layers[selected].parent
        };
        self.check_ancestors(parent, crate::LOCK_ALL)?;
        if self.tile_count() + source.tiles.len() > self.options.max_document_tiles {
            return Err(Error::ResourceLimit("placed source tiles"));
        }
        // File loading owns a different page pool: import immutable pixels into this document.
        for tile in source.tiles.values_mut() {
            let owned = Arc::new(Tile::from_pixels(tile.try_pixels()?.to_vec())?);
            self.pool.attach(&owned)?;
            *tile = owned;
            self.pool.trim()?;
        }
        let mut layer = source.clone();
        layer.id = id;
        layer.name = name;
        layer.parent = parent;
        layer.appearance = crate::LayerAppearance::default();
        layer.placed = None;
        let scale = (self.width as f64 / width as f64)
            .min(self.height as f64 / height as f64)
            .min(1.);
        let geometry = TransformGeometry::rectangle(
            (self.width as f64 - width as f64 * scale) * 0.5,
            (self.height as f64 - height as f64 * scale) * 0.5,
            width as f64 * scale,
            height as f64 * scale,
        );
        let at = if self.layers[selected].group {
            selected
        } else {
            selected + 1
        };
        let mut proposed = self.layers.clone();
        proposed.insert(at, layer.clone());
        groups::validate(&proposed)?;
        self.layers = proposed;
        self.active_layer = id;
        self.next_layer_id = next_id;
        let image = Arc::new(PlacedImage {
            source,
            bounds: [0., 0., width as f64, height as f64],
            geometry: geometry.clone(),
        });
        self.transform = Some(TransformState {
            original: layer,
            source: image,
            current: geometry.clone(),
            checkpoint: geometry.clone(),
            initial: geometry.clone(),
            selection: None,
            extra: Vec::new(),
            undo: VecDeque::new(),
            redo: Vec::new(),
            previous_active,
            placed: Some(at),
            evicted: false,
        });
        if let Err(e) = self.update_transform(geometry) {
            self.next_layer_id = id;
            self.cancel_transform()?;
            return Err(e);
        }
        Ok(id)
    }
    pub fn update_transform(&mut self, geometry: TransformGeometry) -> Result<()> {
        geometry.validate()?;
        let state = self
            .transform
            .as_ref()
            .ok_or(Error::InvalidArgument("no transform session"))?;
        let mut output = Vec::new();
        for (original, source) in std::iter::once((&state.original, &state.source))
            .chain(state.extra.iter().map(|(l, s)| (l, s)))
        {
            let g = if state.extra.is_empty() {
                geometry.clone()
            } else {
                let a = state.initial.points[0];
                let w = state.initial.points[3][0] - a[0];
                let h = state.initial.points[12][1] - a[1];
                TransformGeometry {
                    points: source
                        .geometry
                        .points
                        .map(|p| geometry.map((p[0] - a[0]) / w, (p[1] - a[1]) / h)),
                    skew_h: geometry.skew_h,
                    skew_v: geometry.skew_v,
                    warp: geometry.warp || source.geometry.warp,
                    interpolation: geometry.interpolation,
                }
            };
            g.validate()?;
            output.push(if state.placed.is_none() && geometry == state.initial {
                original.clone()
            } else {
                self.render_transform_target(original, source, &g)?
            });
        }
        let removed = output
            .iter()
            .map(|l| self.layer(l.id).expect("target").retained_tile_count())
            .sum::<usize>();
        if self.tile_count() - removed
            + output.iter().map(Layer::retained_tile_count).sum::<usize>()
            > self.options.max_document_tiles
        {
            return Err(Error::ResourceLimit("transform target tiles"));
        }
        let selected = state
            .selection
            .as_ref()
            .map(|selection| self.transformed_selection(selection, &state.initial, &geometry))
            .transpose()?;
        for layer in output {
            let at = self.layer_index(layer.id)?;
            self.layers[at] = layer;
        }
        if let Some(selection) = selected {
            self.selection = selection;
            self.selection_cache.clear();
        }
        self.transform.as_mut().expect("session").current = geometry;
        self.dirty_all();
        Ok(())
    }
    fn transformed_selection(
        &self,
        selection: &crate::Selection,
        initial: &TransformGeometry,
        geometry: &TransformGeometry,
    ) -> Result<crate::Selection> {
        if initial == geometry {
            return Ok(selection.clone());
        }
        let count = (self.width as usize).saturating_mul(self.height as usize);
        if count > crate::MAX_SELECTION_MASK_BYTES {
            return Err(Error::ResourceLimit("selection transform coverage"));
        }
        let mut coverage = vec![0u8; count];
        let inverse = geometry.inverse_matrix();
        let origin = initial.points[0];
        let w = initial.points[3][0] - origin[0];
        let h = initial.points[12][1] - origin[1];
        for y in 0..self.height {
            for x in 0..self.width {
                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;
                let uv = if let Some(m) = inverse {
                    let z = m[6] * px + m[7] * py + m[8];
                    let u = (m[0] * px + m[1] * py + m[2]) / z;
                    let v = (m[3] * px + m[4] * py + m[5]) / z;
                    ((0. ..=1.).contains(&u) && (0. ..=1.).contains(&v)).then_some([u, v])
                } else {
                    geometry.inverse(px, py, [(px - origin[0]) / w, (py - origin[1]) / h])
                };
                if let Some([u, v]) = uv {
                    coverage[(y * self.width + x) as usize] =
                        (selection.coverage(origin[0] + u * w, origin[1] + v * h) * 255.).round()
                            as u8;
                }
            }
        }
        let mut shape = crate::SelectionShape::geometry(
            crate::SelectionKind::Rectangle,
            0.,
            0.,
            self.width as f64,
            self.height as f64,
            false,
        );
        shape.kind = crate::SelectionKind::Mask;
        shape.mask = Some(crate::SelectionMask::from_alpha(
            self.width,
            self.height,
            coverage,
        )?);
        crate::Selection::from_steps(
            true,
            vec![crate::SelectionStep {
                operation: crate::SelectionOperation::Replace,
                shape: Some(shape),
            }],
        )
    }
    fn render_transform_target(
        &self,
        original: &Layer,
        source: &Arc<PlacedImage>,
        geometry: &TransformGeometry,
    ) -> Result<Layer> {
        let state = self.transform.as_ref().expect("session");
        if state.selection.is_some() && *geometry == state.initial {
            return Ok(original.clone());
        }
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for y in 0..=24 {
            for x in 0..=24 {
                let p = geometry.map(x as f64 / 24., y as f64 / 24.);
                b = [
                    b[0].min(p[0]),
                    b[1].min(p[1]),
                    b[2].max(p[0]),
                    b[3].max(p[1]),
                ];
            }
        }
        let mut extent = b;
        if state.selection.is_some() {
            let o = layer_bounds(original)?;
            extent = [
                extent[0].min(o[0] + original.appearance.offset_x as f64),
                extent[1].min(o[1] + original.appearance.offset_y as f64),
                extent[2].max(o[0] + o[2] + original.appearance.offset_x as f64),
                extent[3].max(o[1] + o[3] + original.appearance.offset_y as f64),
            ];
        }
        let x0 = extent[0].floor() as i64;
        let y0 = extent[1].floor() as i64;
        let x1 = extent[2].ceil() as i64;
        let y1 = extent[3].ceil() as i64;
        if x1 <= x0 || y1 <= y0 || (x1 - x0).saturating_mul(y1 - y0) > 16_777_216 {
            return Err(Error::ResourceLimit("transform output pixels"));
        }
        let mut layer = original.clone();
        layer.tiles.clear();
        layer.white = (0, 0);
        layer.appearance.offset_x = 0;
        layer.appearance.offset_y = 0;
        if original.mask {
            let owner = self.layer(original.parent).expect("owner");
            let owner_transformed = std::iter::once(&state.original)
                .chain(state.extra.iter().map(|(l, _)| l))
                .any(|l| l.id == owner.id);
            if !owner_transformed {
                layer.appearance.offset_x = owner
                    .appearance
                    .offset_x
                    .checked_neg()
                    .ok_or(Error::InvalidArgument("mask position"))?;
                layer.appearance.offset_y = owner
                    .appearance
                    .offset_y
                    .checked_neg()
                    .ok_or(Error::InvalidArgument("mask position"))?;
            }
        }
        let bounds = source.bounds;
        let mut sampler = SourceSampler::new(&source.source);
        sampler.selection = state.selection.as_ref();
        sampler.offset = [
            original.appearance.offset_x as f64,
            original.appearance.offset_y as f64,
        ];
        let mut background = SourceSampler::new(original);
        let inverse_matrix = geometry.inverse_matrix();
        let scale = ((x1 - x0) as f64 / bounds[2]).min((y1 - y0) as f64 / bounds[3]);
        let retained = self.tile_count()
            - self.layers[self.layer_index(original.id)?].retained_tile_count()
            + if original.is_image() || state.placed.is_some() {
                source.source.tiles.len()
            } else {
                0
            };
        // Render and page one tile at a time; an off-canvas large image does not allocate
        // an unpaged full-size destination buffer. Publish only after every tile succeeds.
        for ty in y0.div_euclid(64)..=(y1 - 1).div_euclid(64) {
            for tx in x0.div_euclid(64)..=(x1 - 1).div_euclid(64) {
                let mut pixels = vec![Pixel::TRANSPARENT; 4096];
                let mut nonempty = false;
                for y in y0.max(ty * 64)..y1.min((ty + 1) * 64) {
                    for x in x0.max(tx * 64)..x1.min((tx + 1) * 64) {
                        let px = x as f64 + 0.5;
                        let py = y as f64 + 0.5;
                        let uv = if let Some(m) = inverse_matrix {
                            let z = m[6] * px + m[7] * py + m[8];
                            let u = (m[0] * px + m[1] * py + m[2]) / z;
                            let v = (m[3] * px + m[4] * py + m[5]) / z;
                            (u >= 0. && v >= 0. && u <= 1. && v <= 1.).then_some([u, v])
                        } else {
                            geometry.inverse(
                                px,
                                py,
                                [(px - b[0]) / (b[2] - b[0]), (py - b[1]) / (b[3] - b[1])],
                            )
                        };
                        let mut pixel = Pixel::TRANSPARENT;
                        if let Some([u, v]) = uv {
                            pixel = sample(
                                &mut sampler,
                                bounds[0] + u * bounds[2] - 0.5,
                                bounds[1] + v * bounds[3] - 0.5,
                                geometry.interpolation,
                                scale,
                            )?;
                        }
                        if let Some(selection) = &state.selection {
                            let old = background
                                .pixel(
                                    x - original.appearance.offset_x as i64,
                                    y - original.appearance.offset_y as i64,
                                )?
                                .scaled(1. - selection.coverage(px, py));
                            pixel = pixel.over(old);
                        }
                        if pixel.alpha() > 0. {
                            nonempty = true;
                            pixels[(y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize] = pixel;
                        }
                    }
                }
                if nonempty {
                    if retained + layer.tiles.len() >= self.options.max_document_tiles {
                        return Err(Error::ResourceLimit("transform tiles"));
                    }
                    let tile = Arc::new(Tile::from_pixels(pixels)?);
                    self.pool.attach(&tile)?;
                    layer
                        .tiles
                        .insert(TileCoord::from_signed(tx as i32, ty as i32), tile);
                    self.pool.trim()?;
                }
            }
        }
        if original.placed.is_some() || state.placed.is_some() {
            let mut image = (**source).clone();
            image.geometry = geometry.clone();
            layer.placed = Some(Arc::new(image));
        }
        Ok(layer)
    }
    pub fn checkpoint_transform(&mut self) -> Result<bool> {
        let s = self
            .transform
            .as_mut()
            .ok_or(Error::InvalidArgument("no transform session"))?;
        if s.checkpoint == s.current {
            return Ok(false);
        }
        s.undo.push_back(s.checkpoint.clone());
        s.checkpoint = s.current.clone();
        s.redo.clear();
        while s.undo.len() > self.options.max_history_commands {
            s.undo.pop_front();
            s.evicted = true;
        }
        Ok(true)
    }
    pub fn undo_transform(&mut self) -> Result<bool> {
        self.travel_transform(false)
    }
    pub fn redo_transform(&mut self) -> Result<bool> {
        self.travel_transform(true)
    }
    fn travel_transform(&mut self, redo: bool) -> Result<bool> {
        self.checkpoint_transform()?;
        let s = self.transform.as_ref().expect("session");
        let target = if redo { s.redo.last() } else { s.undo.back() }.cloned();
        let Some(target) = target else {
            return Ok(false);
        };
        let current = s.current.clone();
        self.update_transform(target.clone())?;
        let s = self.transform.as_mut().expect("session");
        if redo {
            s.redo.pop();
            s.undo.push_back(current);
        } else {
            s.undo.pop_back();
            s.redo.push(current);
        }
        s.checkpoint = target;
        Ok(true)
    }
    pub fn cancel_transform(&mut self) -> Result<()> {
        let s = self
            .transform
            .take()
            .ok_or(Error::InvalidArgument("no transform session"))?;
        let at = self.layer_index(s.original.id)?;
        if let Some(selection) = s.selection {
            self.selection = selection;
            self.selection_cache.clear();
        }
        if s.placed.is_some() {
            self.layers.remove(at);
            self.active_layer = s.previous_active;
            self.ensure_active_layer();
        } else {
            self.layers[at] = s.original;
            for (layer, _) in s.extra {
                let at = self.layer_index(layer.id)?;
                self.layers[at] = layer;
            }
        }
        self.dirty_all();
        Ok(())
    }
    pub fn commit_transform(&mut self) -> Result<bool> {
        let s = self
            .transform
            .as_ref()
            .ok_or(Error::InvalidArgument("no transform session"))?;
        let at = self.layer_index(s.original.id)?;
        let changed = s.placed.is_some() || s.current != s.initial;
        if changed {
            let after = self.layers[at].clone();
            let (command, action) = if let Some(index) = s.placed {
                (
                    Command::AddLayer {
                        index,
                        layer: after,
                    },
                    crate::HistoryAction::PlaceImage,
                )
            } else {
                (
                    Command::LayerEdit {
                        selection: s
                            .selection
                            .as_ref()
                            .map(|before| (before.clone(), self.selection.clone())),
                        before: std::iter::once(s.original.clone())
                            .chain(s.extra.iter().map(|(l, _)| l.clone()))
                            .collect(),
                        after: std::iter::once(after)
                            .chain(
                                s.extra
                                    .iter()
                                    .map(|(l, _)| self.layer(l.id).expect("target").clone()),
                            )
                            .collect(),
                    },
                    crate::HistoryAction::Transform,
                )
            };
            self.commit_with_action(command, action)?;
        } else {
            self.layers[at] = s.original.clone();
            for (layer, _) in &s.extra {
                let at = self.layer_index(layer.id)?;
                self.layers[at] = layer.clone();
            }
            if let Some(selection) = &s.selection {
                self.selection = selection.clone();
                self.selection_cache.clear();
            }
        }
        self.transform = None;
        self.dirty_all();
        Ok(changed)
    }
    pub fn rasterize_image(&mut self, id: u64) -> Result<bool> {
        self.idle()?;
        self.check_ancestors(id, crate::LOCK_ALL)?;
        let at = self.layer_index(id)?;
        if self.layers[at].placed.is_none() {
            return Ok(false);
        }
        let before = self.layers[at].clone();
        let mut after = before.clone();
        after.placed = None;
        self.commit_with_action(
            Command::LayerEdit {
                selection: None,
                before: vec![before],
                after: vec![after.clone()],
            },
            crate::HistoryAction::Rasterize,
        )?;
        self.layers[at] = after;
        self.dirty_all();
        Ok(true)
    }
    pub fn embedded_layers(&self) -> Result<Vec<u8>> {
        crate::history_storage::encode_layer_bundle(&self.layers)
    }
    pub fn from_embedded_layers(
        width: u32,
        height: u32,
        bytes: &[u8],
        active: u64,
        options: DocumentOptions,
        storage: Arc<paint_storage::ScratchSpace>,
    ) -> Result<Self> {
        let mut d = Self::with_storage(width, height, options, storage)?;
        let layers = crate::history_storage::decode_layer_bundle(
            bytes,
            options.max_document_tiles,
            d.pool.clone(),
        )?;
        groups::validate(&layers)?;
        d.next_layer_id = layers
            .iter()
            .map(|l| l.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(Error::ResourceLimit("layer IDs"))?;
        d.active_layer = if layers.iter().any(|l| l.id == active) {
            active
        } else {
            layers.last().expect("layers").id
        };
        d.layers = layers;
        Ok(d)
    }
}
impl DocumentSnapshot {
    pub fn embedded_layers(&self) -> Result<Vec<u8>> {
        crate::history_storage::encode_layer_bundle(&self.layers)
    }
}
