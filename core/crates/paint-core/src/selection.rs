use crate::{Error, Result};
use std::sync::Arc;

pub const MAX_SELECTION_STEPS: usize = 64;
pub const MAX_SELECTION_POINTS: usize = 4096;
pub const MAX_SELECTION_FEATHER: f64 = 1000.;
/// Masks are per-pixel coverage; the cap keeps a single selection step bounded (16 MiB at 1 byte
/// per pixel, i.e. 4096x4096). Larger regions must fail loudly instead of exhausting memory.
pub const MAX_SELECTION_MASK_BYTES: usize = 16 * 1024 * 1024;
/// Lasso paths are sampled by the UI; the cap bounds both memory and rasterization work.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionKind {
    Rectangle,
    Ellipse,
    Polygon,
    /// Pixel region carried by a mask, as persisted by earlier versions.
    Mask,
}

impl SelectionKind {
    /// Geometry-only shapes answer `coverage` analytically; a mask region is stored per pixel.
    pub fn is_geometric(self) -> bool {
        matches!(
            self,
            SelectionKind::Rectangle | SelectionKind::Ellipse | SelectionKind::Polygon
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionOperation {
    Replace,
    Add,
    Subtract,
    Intersect,
    Invert,
}

/// Bounding-box coverage for a rasterized shape. `coverage` is clipped per row so that
/// `coverage.len() == width * height` and only 0/1 values are stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionMask {
    width: u32,
    height: u32,
    coverage: Arc<[u8]>,
}

impl SelectionMask {
    pub fn from_coverage(width: u32, height: u32, coverage: Vec<u8>) -> Result<Self> {
        let bytes = (width as usize).checked_mul(height as usize);
        let Some(bytes) = bytes else {
            return Err(Error::ResourceLimit("selection mask size"));
        };
        if width == 0 || height == 0 || bytes > MAX_SELECTION_MASK_BYTES {
            return Err(Error::ResourceLimit("selection mask size"));
        }
        if coverage.len() != bytes || coverage.iter().any(|value| *value > 1) {
            return Err(Error::InvalidArgument("invalid selection mask"));
        }
        Ok(Self {
            width,
            height,
            coverage: coverage.into(),
        })
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn bytes(&self) -> usize {
        self.coverage.len()
    }
    /// Coverage at a pixel center. `x`/`y` are document pixel indices inside the bounding box.
    pub fn at(&self, x: u32, y: u32) -> f32 {
        if x >= self.width || y >= self.height {
            return 0.;
        }
        f32::from(self.coverage[(y as usize) * (self.width as usize) + (x as usize)])
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PolygonIndex {
    points: Arc<[[f64; 2]]>,
    feather: f64,
    y: f64,
    height: f64,
    rows: Vec<Vec<usize>>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionShape {
    pub kind: SelectionKind,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub antialias: bool,
    pub feather: f64,
    pub points: Arc<[[f64; 2]]>,
    index: Option<Arc<PolygonIndex>>,
    /// Present for `Mask`; the geometry above is then the bounding box.
    pub mask: Option<SelectionMask>,
}

impl SelectionShape {
    /// Rectangular or elliptical geometry with no pixel payload.
    pub fn geometry(
        kind: SelectionKind,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        antialias: bool,
    ) -> Self {
        Self {
            kind,
            x,
            y,
            width,
            height,
            antialias,
            feather: 0.,
            points: Arc::from([]),
            index: None,
            mask: None,
        }
    }
    pub fn polygon(points: Vec<[f64; 2]>, antialias: bool, feather: f64) -> Result<Self> {
        if points.len() < 3 || points.len() > MAX_SELECTION_POINTS {
            return Err(Error::InvalidArgument("selection path count"));
        }
        let mut left = f64::INFINITY;
        let mut top = f64::INFINITY;
        let mut right = f64::NEG_INFINITY;
        let mut bottom = f64::NEG_INFINITY;
        for [x, y] in &points {
            if !x.is_finite() || !y.is_finite() || x.abs() > 2_000_000. || y.abs() > 2_000_000. {
                return Err(Error::InvalidArgument("selection path coordinate"));
            }
            left = left.min(*x);
            top = top.min(*y);
            right = right.max(*x);
            bottom = bottom.max(*y);
        }
        let mut shape = Self::geometry(
            SelectionKind::Polygon,
            left,
            top,
            right - left,
            bottom - top,
            antialias,
        );
        shape.points = points.into();
        shape.feather = feather;
        shape.validate()?;
        shape.index_polygon();
        Ok(shape)
    }
    fn index_polygon(&mut self) {
        if self.kind != SelectionKind::Polygon {
            return;
        }
        let margin = self.feather + 1.;
        let top = self.y - margin;
        let step = (self.height + 2. * margin) / 64.;
        let mut rows = vec![Vec::new(); 64];
        for i in 0..self.points.len() {
            let a = self.points[(i + self.points.len() - 1) % self.points.len()];
            let b = self.points[i];
            let first = ((a[1].min(b[1]) - margin - top) / step)
                .floor()
                .clamp(0., 63.) as usize;
            let last = ((a[1].max(b[1]) + margin - top) / step)
                .floor()
                .clamp(0., 63.) as usize;
            for row in &mut rows[first..=last] {
                row.push(i);
            }
        }
        self.index = Some(Arc::new(PolygonIndex {
            points: self.points.clone(),
            feather: self.feather,
            y: self.y,
            height: self.height,
            rows,
        }));
    }
    pub fn validate(&self) -> Result<()> {
        if !self.feather.is_finite() || !(0. ..=MAX_SELECTION_FEATHER).contains(&self.feather) {
            return Err(Error::InvalidArgument("selection feather"));
        }
        if self.kind == SelectionKind::Polygon {
            if self.points.len() < 3 || self.points.len() > MAX_SELECTION_POINTS {
                return Err(Error::InvalidArgument("selection path count"));
            }
            for [x, y] in self.points.iter() {
                if !x.is_finite()
                    || !y.is_finite()
                    || *x < self.x
                    || *x > self.x + self.width
                    || *y < self.y
                    || *y > self.y + self.height
                {
                    return Err(Error::InvalidArgument("selection path bounds"));
                }
            }
            let a = self.points[0];
            let b = self
                .points
                .iter()
                .find(|p| **p != a)
                .ok_or(Error::InvalidArgument("degenerate selection"))?;
            if !self.points.iter().any(|p| {
                ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).abs() > 1e-9
            }) {
                return Err(Error::InvalidArgument("degenerate selection"));
            }
        } else if !self.points.is_empty() {
            return Err(Error::InvalidArgument("unexpected selection path"));
        }
        if self.kind == SelectionKind::Mask && self.feather != 0. {
            return Err(Error::InvalidArgument("mask feather unsupported"));
        }
        if !self.kind.is_geometric() && self.mask.is_none() {
            return Err(Error::InvalidArgument("rasterized selection needs a mask"));
        }
        if self.kind.is_geometric() && self.mask.is_some() {
            return Err(Error::InvalidArgument("geometric selection carries a mask"));
        }
        if [self.x, self.y, self.width, self.height]
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 2_000_000.)
            || self.width <= 0.
            || self.height <= 0.
        {
            return Err(Error::InvalidArgument("invalid selection geometry"));
        }
        Ok(())
    }
    /// Shared shape boundary distance, positive inside. Polygon fill uses the even-odd rule.
    fn distance(&self, x: f64, y: f64) -> f64 {
        if self.kind == SelectionKind::Polygon {
            let mut inside = false;
            let mut nearest = f64::INFINITY;
            let indexed = self.index.as_ref().filter(|index| {
                Arc::ptr_eq(&index.points, &self.points)
                    && index.feather == self.feather
                    && index.y == self.y
                    && index.height == self.height
            });
            let row = indexed.map(|index| {
                let margin = self.feather + 1.;
                let bin = ((y - self.y + margin) / (self.height + 2. * margin) * 64.)
                    .floor()
                    .clamp(0., 63.) as usize;
                index.rows[bin].as_slice()
            });
            let count = row.map_or(self.points.len(), |indices| indices.len());
            for position in 0..count {
                let i = row.map_or(position, |indices| indices[position]);
                let a = self.points[(i + self.points.len() - 1) % self.points.len()];
                let b = self.points[i];
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let length = dx * dx + dy * dy;
                let t = if length > 0. {
                    ((x - a[0]) * dx + (y - a[1]) * dy) / length
                } else {
                    0.
                }
                .clamp(0., 1.);
                nearest = nearest.min((x - a[0] - t * dx).hypot(y - a[1] - t * dy));
                if (a[1] > y) != (b[1] > y) && x < a[0] + (y - a[1]) * dx / dy {
                    inside = !inside;
                }
            }
            return if inside { nearest } else { -nearest };
        }
        let rx = self.width * 0.5;
        let ry = self.height * 0.5;
        let dx = (x - self.x - rx).abs();
        let dy = (y - self.y - ry).abs();
        if self.kind == SelectionKind::Rectangle {
            let qx = dx - rx;
            let qy = dy - ry;
            return -(qx.max(0.).hypot(qy.max(0.)) + qx.max(qy).min(0.));
        }
        // Closest point on the first-quadrant ellipse, bounded search independent of canvas size.
        let metric = |angle: f64| (dx - rx * angle.cos()).hypot(dy - ry * angle.sin());
        let mut best = 0usize;
        for i in 1..=16 {
            if metric(i as f64 * std::f64::consts::FRAC_PI_2 / 16.)
                < metric(best as f64 * std::f64::consts::FRAC_PI_2 / 16.)
            {
                best = i;
            }
        }
        let mut low = best.saturating_sub(1) as f64 * std::f64::consts::FRAC_PI_2 / 16.;
        let mut high = (best + 1).min(16) as f64 * std::f64::consts::FRAC_PI_2 / 16.;
        for _ in 0..24 {
            let a = low + (high - low) / 3.;
            let b = high - (high - low) / 3.;
            if metric(a) < metric(b) {
                high = b;
            } else {
                low = a;
            }
        }
        let distance = metric((low + high) * 0.5)
            .min(metric(0.))
            .min(metric(std::f64::consts::FRAC_PI_2));
        if (dx / rx).powi(2) + (dy / ry).powi(2) <= 1. {
            distance
        } else {
            -distance
        }
    }

    fn polygon_edges(&self, y: f64, mut visit: impl FnMut([f64; 2], [f64; 2])) {
        let index = self.index.as_ref().filter(|index| {
            Arc::ptr_eq(&index.points, &self.points)
                && index.feather == self.feather
                && index.y == self.y
                && index.height == self.height
        });
        let row = index.map(|index| {
            let margin = self.feather + 1.;
            let bin = ((y - self.y + margin) / (self.height + 2. * margin) * 64.)
                .floor()
                .clamp(0., 63.) as usize;
            index.rows[bin].as_slice()
        });
        for position in 0..row.map_or(self.points.len(), |r| r.len()) {
            let i = row.map_or(position, |r| r[position]);
            visit(
                self.points[(i + self.points.len() - 1) % self.points.len()],
                self.points[i],
            );
        }
    }
    /// Fill one subpixel sample row with the even-odd polygon rule. Edge work is per row,
    /// not per pixel; spans share exactly the same sample positions as scalar coverage.
    fn polygon_sample_row(&self, x: f64, y: f64, weight: f32, output: &mut [f32]) {
        let mut crossings = Vec::new();
        let mut boundaries = Vec::new();
        self.polygon_edges(y, |a, b| {
            if (a[1] > y) != (b[1] > y) {
                crossings.push(a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
            }
            if a[1] == y && b[1] == y {
                boundaries.push((a[0].min(b[0]), a[0].max(b[0])));
            }
            // Vertices on the last scanline still belong to a closed boundary.
            if a[1] == y {
                boundaries.push((a[0], a[0]));
            }
        });
        crossings.sort_unstable_by(f64::total_cmp);
        let mut covered = vec![false; output.len()];
        for (left, right) in crossings
            .chunks_exact(2)
            .map(|pair| (pair[0], pair[1]))
            .chain(boundaries.iter().copied())
        {
            let start = (left - x).ceil().clamp(0., output.len() as f64) as usize;
            let end = ((right - x).floor() + 1.).clamp(0., output.len() as f64) as usize;
            if start < end {
                covered[start..end].fill(true);
            }
            // At exact subpixel crossings floating point projection may distinguish the
            // boundary from its mathematical line; retain the existing scalar decision.
            for edge in [left, right] {
                let offset = (edge - x).round();
                if offset >= 0. && offset < output.len() as f64 && (x + offset - edge).abs() < 1e-10
                {
                    covered[offset as usize] = self.distance(x + offset, y) >= 0.;
                }
            }
        }
        // A horizontal boundary's projection can round just outside the shape.
        // Preserve the scalar result for this exceptional row, after all spans.
        for (left, right) in boundaries {
            let start = (left - x).ceil().clamp(0., output.len() as f64) as usize;
            let end = ((right - x).floor() + 1.).clamp(0., output.len() as f64) as usize;
            for (i, inside) in covered.iter_mut().enumerate().take(end).skip(start) {
                *inside = self.distance(x + i as f64, y) >= 0.;
            }
        }
        for (value, inside) in output.iter_mut().zip(covered) {
            if inside {
                *value += weight;
            }
        }
    }
    fn coverage_span(&self, x: f64, y: f64, output: &mut [f32]) {
        output.fill(0.);
        if y < self.y - self.feather - 1.
            || y > self.y + self.height + self.feather + 1.
            || x + (output.len() as f64) < self.x - self.feather - 1.
            || x > self.x + self.width + self.feather + 1.
        {
            return;
        }
        if self.kind != SelectionKind::Polygon {
            for (i, value) in output.iter_mut().enumerate() {
                *value = self.coverage(x + i as f64, y);
            }
            return;
        }
        if self.feather > 0. {
            self.polygon_sample_row(x, y, 1., output);
            let mut distances = vec![self.feather * self.feather; output.len()];
            self.polygon_edges(y, |a, b| {
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let length = dx * dx + dy * dy;
                let start = (a[0].min(b[0]) - self.feather - x)
                    .ceil()
                    .clamp(0., output.len() as f64) as usize;
                let end = ((a[0].max(b[0]) + self.feather - x).floor() + 1.)
                    .clamp(0., output.len() as f64) as usize;
                for (i, nearest) in distances.iter_mut().enumerate().take(end).skip(start) {
                    let px = x + i as f64 - a[0];
                    let py = y - a[1];
                    let t = if length > 0. {
                        (px * dx + py * dy) / length
                    } else {
                        0.
                    }
                    .clamp(0., 1.);
                    let ex = px - t * dx;
                    let ey = py - t * dy;
                    *nearest = nearest.min(ex * ex + ey * ey);
                }
            });
            for (value, squared) in output.iter_mut().zip(distances) {
                let sign = if *value > 0. { 1. } else { -1. };
                let t = (0.5 + sign * squared.sqrt() / (2. * self.feather)).clamp(0., 1.);
                *value = (t * t * (3. - 2. * t)) as f32;
            }
        } else if self.antialias {
            for sy in 0..4 {
                for sx in 0..4 {
                    self.polygon_sample_row(
                        x - 0.5 + (sx as f64 + 0.5) * 0.25,
                        y - 0.5 + (sy as f64 + 0.5) * 0.25,
                        1. / 16.,
                        output,
                    );
                }
            }
        } else {
            self.polygon_sample_row(x, y, 1., output);
        }
    }
    pub fn coverage(&self, x: f64, y: f64) -> f32 {
        if x < self.x - self.feather - 1.
            || x > self.x + self.width + self.feather + 1.
            || y < self.y - self.feather - 1.
            || y > self.y + self.height + self.feather + 1.
        {
            return 0.;
        }
        if self.feather > 0. {
            let t = (0.5 + self.distance(x, y) / (2. * self.feather)).clamp(0., 1.);
            return (t * t * (3. - 2. * t)) as f32;
        }
        if self.kind == SelectionKind::Polygon {
            let d = self.distance(x, y);
            if !self.antialias || d.abs() > std::f64::consts::FRAC_1_SQRT_2 {
                return f32::from(d >= 0.);
            }
            let mut inside = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    inside += u32::from(
                        self.distance(
                            x - 0.5 + (sx as f64 + 0.5) * 0.25,
                            y - 0.5 + (sy as f64 + 0.5) * 0.25,
                        ) >= 0.,
                    );
                }
            }
            return inside as f32 / 16.;
        }
        if let Some(mask) = &self.mask {
            // Mask shapes are addressed by integer pixel: the stored value is the pixel center's
            // coverage, produced by the flood fill (wand).
            let px = x.floor();
            let py = y.floor();
            if !px.is_finite() || !py.is_finite() {
                return 0.;
            }
            let local_x = px - self.x.floor();
            let local_y = py - self.y.floor();
            if local_x < 0. || local_y < 0. {
                return 0.;
            }
            return mask.at(local_x as u32, local_y as u32);
        }
        if self.kind == SelectionKind::Rectangle {
            if !self.antialias {
                return f32::from(
                    x >= self.x
                        && x < self.x + self.width
                        && y >= self.y
                        && y < self.y + self.height,
                );
            }
            return ((self.x + self.width).min(x + 0.5) - self.x.max(x - 0.5)).clamp(0., 1.) as f32
                * ((self.y + self.height).min(y + 0.5) - self.y.max(y - 0.5)).clamp(0., 1.) as f32;
        }
        let rx = self.width * 0.5;
        let ry = self.height * 0.5;
        let dx = (x - self.x - rx).abs();
        let dy = (y - self.y - ry).abs();
        if !self.antialias {
            return f32::from((dx / rx).powi(2) + (dy / ry).powi(2) <= 1.);
        }
        if ((dx + 0.5) / rx).powi(2) + ((dy + 0.5) / ry).powi(2) <= 1. {
            return 1.;
        }
        if ((dx - 0.5).max(0.) / rx).powi(2) + ((dy - 0.5).max(0.) / ry).powi(2) >= 1. {
            return 0.;
        }
        let mut inside = 0;
        for sy in 0..4 {
            for sx in 0..4 {
                let px = (x - 0.5 + (f64::from(sx) + 0.5) * 0.25 - self.x - rx) / rx;
                let py = (y - 0.5 + (f64::from(sy) + 0.5) * 0.25 - self.y - ry) / ry;
                inside += u32::from(px * px + py * py <= 1.);
            }
        }
        inside as f32 / 16.
    }
}

/// Wrap a flood-filled pixel region as a `Mask` shape anchored at `(left, top)`.
pub fn mask_shape(
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    coverage: Vec<u8>,
) -> Result<SelectionShape> {
    let mask = SelectionMask::from_coverage(width, height, coverage)?;
    Ok(SelectionShape {
        kind: SelectionKind::Mask,
        x: f64::from(left),
        y: f64::from(top),
        width: f64::from(width),
        height: f64::from(height),
        antialias: false,
        feather: 0.,
        points: Arc::from([]),
        index: None,
        mask: Some(mask),
    })
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 0xf) as usize] as char);
    }
    text
}

fn hex_decode(text: &str) -> Result<Vec<u8>> {
    let invalid = || Error::InvalidArgument("invalid selection mask encoding");
    if text.len() % 2 != 0 {
        return Err(invalid());
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    let digits = text.as_bytes();
    let value = |c: u8| -> Result<u8> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            _ => Err(invalid()),
        }
    };
    for pair in digits.chunks(2) {
        bytes.push((value(pair[0])? << 4) | value(pair[1])?);
    }
    Ok(bytes)
}

fn mask_to_text(mask: &SelectionMask) -> String {
    let mut runs = Vec::new();
    let mut run_value = 0u8;
    let mut run_length: u16 = 0;
    for value in mask.coverage.iter() {
        if *value == run_value && run_length < u16::from(u8::MAX) {
            run_length += 1;
            continue;
        }
        if run_length > 0 {
            runs.push(run_value);
            runs.push(run_length as u8);
        }
        run_value = *value;
        run_length = 1;
    }
    if run_length > 0 {
        runs.push(run_value);
        runs.push(run_length as u8);
    }
    hex_encode(&runs)
}

fn mask_from_text(text: &str, width: u32, height: u32) -> Result<SelectionMask> {
    let bytes = (width as usize) * (height as usize);
    let runs = hex_decode(text)?;
    let mut coverage = Vec::with_capacity(bytes);
    for pair in runs.chunks(2) {
        if pair.len() != 2 {
            return Err(Error::InvalidArgument("invalid selection mask encoding"));
        }
        let value = pair[0];
        if value > 1 {
            return Err(Error::InvalidArgument("invalid selection mask encoding"));
        }
        let length = usize::from(pair[1]);
        if coverage.len() + length > bytes {
            return Err(Error::InvalidArgument("selection mask too long"));
        }
        coverage.extend(std::iter::repeat_n(value, length));
    }
    if coverage.len() != bytes {
        return Err(Error::InvalidArgument("selection mask too short"));
    }
    SelectionMask::from_coverage(width, height, coverage)
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionStep {
    pub operation: SelectionOperation,
    pub shape: Option<SelectionShape>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selection {
    enabled: bool,
    steps: Arc<[SelectionStep]>,
}
impl Selection {
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn steps(&self) -> &[SelectionStep] {
        &self.steps
    }
    pub fn from_steps(enabled: bool, steps: Vec<SelectionStep>) -> Result<Self> {
        if steps.len() > MAX_SELECTION_STEPS {
            return Err(Error::ResourceLimit("selection step budget"));
        }
        if !enabled && !steps.is_empty() {
            return Err(Error::InvalidArgument("disabled selection has steps"));
        }
        if enabled && steps.is_empty() {
            return Err(Error::InvalidArgument("enabled selection has no steps"));
        }
        if steps
            .iter()
            .filter_map(|s| s.shape.as_ref())
            .map(|s| s.points.len())
            .sum::<usize>()
            > MAX_SELECTION_POINTS
        {
            return Err(Error::ResourceLimit("selection path budget"));
        }
        for step in &steps {
            match (step.operation, step.shape.as_ref()) {
                (SelectionOperation::Invert, None) => {}
                (SelectionOperation::Invert, Some(_)) | (_, None) => {
                    return Err(Error::InvalidArgument("invalid selection step"))
                }
                (_, Some(shape)) => shape.validate()?,
            }
        }
        Ok(Self {
            enabled,
            steps: steps.into(),
        })
    }
    pub fn all(width: u32, height: u32) -> Self {
        Self {
            enabled: true,
            steps: vec![SelectionStep {
                operation: SelectionOperation::Replace,
                shape: Some(SelectionShape::geometry(
                    SelectionKind::Rectangle,
                    0.,
                    0.,
                    f64::from(width),
                    f64::from(height),
                    false,
                )),
            }]
            .into(),
        }
    }
    pub fn apply(
        &self,
        shape: SelectionShape,
        operation: SelectionOperation,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        shape.validate()?;
        if operation == SelectionOperation::Invert {
            return Err(Error::InvalidArgument("shape supplied for inversion"));
        }
        let mut steps = if operation == SelectionOperation::Replace {
            Vec::new()
        } else if !self.enabled && operation == SelectionOperation::Subtract {
            Self::all(width, height).steps.to_vec()
        } else {
            self.steps.to_vec()
        };
        let operation = if !self.enabled && operation != SelectionOperation::Subtract {
            SelectionOperation::Replace
        } else {
            operation
        };
        steps.push(SelectionStep {
            operation,
            shape: Some(shape),
        });
        Self::from_steps(true, steps)
    }
    pub fn apply_shape(
        &self,
        shape: Option<SelectionShape>,
        operation: SelectionOperation,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        match shape {
            Some(shape) => self.apply(shape, operation, width, height),
            None => {
                if operation != SelectionOperation::Invert {
                    return Err(Error::InvalidArgument("missing selection geometry"));
                }
                self.inverted(width, height)
            }
        }
    }
    pub fn inverted(&self, width: u32, height: u32) -> Result<Self> {
        if !self.enabled {
            return Ok(Self::all(width, height));
        }
        let mut steps = self.steps.to_vec();
        // Adjacent inversions cancel without consuming another step.
        if steps
            .last()
            .is_some_and(|s| s.operation == SelectionOperation::Invert)
        {
            steps.pop();
        } else {
            steps.push(SelectionStep {
                operation: SelectionOperation::Invert,
                shape: None,
            });
        }
        Self::from_steps(true, steps)
    }
    /// Pixel centers in document coordinates. No allocation, including million-pixel canvases.
    pub fn coverage(&self, x: f64, y: f64) -> f32 {
        if !self.enabled {
            return 1.;
        }
        let mut value = 0f32;
        for step in self.steps.iter() {
            let shape = step.shape.as_ref().map_or(0., |s| s.coverage(x, y));
            value = match step.operation {
                SelectionOperation::Replace => shape,
                SelectionOperation::Add => value.max(shape),
                SelectionOperation::Subtract => (value - shape).max(0.),
                SelectionOperation::Intersect => value.min(shape),
                SelectionOperation::Invert => 1. - value,
            };
        }
        value
    }

    /// Coverage for consecutive document pixel centers. Shared by all selection tools.
    pub fn coverage_span(&self, x: f64, y: f64, output: &mut [f32]) {
        if !self.enabled {
            output.fill(1.);
            return;
        }
        output.fill(0.);
        let mut shape = vec![0.; output.len()];
        for step in self.steps.iter() {
            if let Some(s) = &step.shape {
                s.coverage_span(x, y, &mut shape);
            }
            for (value, &coverage) in output.iter_mut().zip(&shape) {
                *value = match step.operation {
                    SelectionOperation::Replace => coverage,
                    SelectionOperation::Add => value.max(coverage),
                    SelectionOperation::Subtract => (*value - coverage).max(0.),
                    SelectionOperation::Intersect => value.min(coverage),
                    SelectionOperation::Invert => 1. - *value,
                };
            }
        }
    }
    pub fn to_text(&self) -> String {
        let mut text = format!("3|{}", u8::from(self.enabled));
        for step in self.steps.iter() {
            let op = match step.operation {
                SelectionOperation::Replace => 0,
                SelectionOperation::Add => 1,
                SelectionOperation::Subtract => 2,
                SelectionOperation::Intersect => 3,
                SelectionOperation::Invert => 4,
            };
            if let Some(s) = step.shape.as_ref() {
                let kind = match s.kind {
                    SelectionKind::Rectangle => 0,
                    SelectionKind::Ellipse => 1,
                    SelectionKind::Polygon => 2,
                    SelectionKind::Mask => 3,
                };
                text.push_str(&format!(
                    ";{op},{kind},{},{},{},{},{}",
                    u8::from(s.antialias),
                    s.x,
                    s.y,
                    s.width,
                    s.height
                ));
                text.push_str(&format!(",{}", s.feather));
                if s.kind == SelectionKind::Polygon {
                    let points: Vec<_> = s
                        .points
                        .iter()
                        .map(|p| format!("{}:{}", p[0], p[1]))
                        .collect();
                    text.push_str(&format!(",{}", points.join("/")));
                }
                if let Some(mask) = s.mask.as_ref() {
                    text.push_str(&format!(",{}", mask_to_text(mask)));
                }
            } else {
                text.push_str(";4,0,0,0,0,0,0");
            }
        }
        text
    }
    pub fn from_text(text: &str) -> Result<Self> {
        let invalid = || Error::InvalidArgument("invalid selection encoding");
        if text.len() > 1024 * 1024 || !text.is_ascii() {
            return Err(invalid());
        }
        let mut parts = text.split(';');
        let header = parts.next().ok_or_else(invalid)?;
        let (version, enabled) = match header {
            "1|0" => (1, false),
            "1|1" => (1, true),
            "2|0" => (2, false),
            "2|1" => (2, true),
            "3|0" => (3, false),
            "3|1" => (3, true),
            _ => return Err(invalid()),
        };
        let mut steps = Vec::new();
        for part in parts {
            if steps.len() >= MAX_SELECTION_STEPS {
                return Err(Error::ResourceLimit("selection step budget"));
            }
            let fields: Vec<_> = part.split(',').collect();
            // v1 adds nothing beyond the seven geometry fields; v2 may append a mask column.
            if fields.len() < 7 {
                return Err(invalid());
            }
            let operation = match fields[0] {
                "0" => SelectionOperation::Replace,
                "1" => SelectionOperation::Add,
                "2" => SelectionOperation::Subtract,
                "3" => SelectionOperation::Intersect,
                "4" => SelectionOperation::Invert,
                _ => return Err(invalid()),
            };
            if operation == SelectionOperation::Invert {
                if fields[1..].iter().any(|v| *v != "0") {
                    return Err(invalid());
                }
                steps.push(SelectionStep {
                    operation,
                    shape: None,
                });
                continue;
            }
            let kind = match fields[1] {
                "0" => SelectionKind::Rectangle,
                "1" => SelectionKind::Ellipse,
                "2" if version >= 3 => SelectionKind::Polygon,
                "3" if version >= 2 => SelectionKind::Mask,
                _ => return Err(invalid()),
            };
            let antialias = match fields[2] {
                "0" => false,
                "1" => true,
                _ => return Err(invalid()),
            };
            let x: f64 = fields[3].parse().map_err(|_| invalid())?;
            let y: f64 = fields[4].parse().map_err(|_| invalid())?;
            let width: f64 = fields[5].parse().map_err(|_| invalid())?;
            let height: f64 = fields[6].parse().map_err(|_| invalid())?;
            let feather = if version >= 3 {
                fields
                    .get(7)
                    .ok_or_else(invalid)?
                    .parse::<f64>()
                    .map_err(|_| invalid())?
            } else {
                0.
            };
            let payload = if version >= 3 { 8 } else { 7 };
            let points = if kind == SelectionKind::Polygon {
                let encoded = fields.get(payload).ok_or_else(invalid)?;
                if encoded.split('/').count() > MAX_SELECTION_POINTS {
                    return Err(invalid());
                }
                let parsed: Result<Vec<[f64; 2]>> = encoded
                    .split('/')
                    .map(|point| {
                        let (x, y) = point.split_once(':').ok_or_else(invalid)?;
                        Ok([
                            x.parse().map_err(|_| invalid())?,
                            y.parse().map_err(|_| invalid())?,
                        ])
                    })
                    .collect();
                Arc::from(parsed?)
            } else {
                Arc::from([])
            };
            let mask = if kind.is_geometric() {
                if fields.len() != payload + usize::from(kind == SelectionKind::Polygon) {
                    return Err(invalid());
                }
                None
            } else {
                if fields.len() != payload + 1 || width < 1. || height < 1. {
                    return Err(invalid());
                }
                Some(mask_from_text(
                    fields[payload],
                    width as u32,
                    height as u32,
                )?)
            };
            let mut shape = SelectionShape {
                kind,
                x,
                y,
                width,
                height,
                antialias,
                feather,
                points,
                mask,
                index: None,
            };
            shape.validate()?;
            shape.index_polygon();
            steps.push(SelectionStep {
                operation,
                shape: Some(shape),
            });
        }
        Self::from_steps(enabled, steps)
    }
}

// Auxiliary working set only: never serialized, retained by snapshots or stored in history.
const COVERAGE_BLOCK: usize = 64;
const MAX_COVERAGE_BLOCKS: usize = 4096;
#[derive(Debug, Default)]
pub(crate) struct SelectionCoverageCache {
    identity: Option<Selection>,
    blocks: std::collections::HashMap<(i64, i64), Arc<[f32; COVERAGE_BLOCK]>>,
    order: std::collections::VecDeque<(i64, i64)>,
}
impl SelectionCoverageCache {
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
    fn block(&mut self, selection: &Selection, x: i64, y: i64) -> Arc<[f32; COVERAGE_BLOCK]> {
        if !self.identity.as_ref().is_some_and(|old| {
            old.enabled == selection.enabled && Arc::ptr_eq(&old.steps, &selection.steps)
        }) {
            self.blocks.clear();
            self.order.clear();
            self.identity = Some(selection.clone());
        }
        let key = (x.div_euclid(COVERAGE_BLOCK as i64), y);
        if let Some(block) = self.blocks.get(&key) {
            return block.clone();
        }
        let mut values = [0.; COVERAGE_BLOCK];
        selection.coverage_span(
            (key.0 * COVERAGE_BLOCK as i64) as f64 + 0.5,
            y as f64 + 0.5,
            &mut values,
        );
        let block = Arc::new(values);
        if self.blocks.len() == MAX_COVERAGE_BLOCKS {
            let oldest = self
                .order
                .pop_front()
                .expect("nonempty bounded coverage cache");
            self.blocks.remove(&oldest);
        }
        self.order.push_back(key);
        self.blocks.insert(key, block.clone());
        block
    }
    pub(crate) fn row(
        &mut self,
        selection: &Selection,
        x: i64,
        y: i64,
        length: usize,
    ) -> [f32; COVERAGE_BLOCK] {
        debug_assert!(length <= COVERAGE_BLOCK);
        let mut result = [0.; COVERAGE_BLOCK];
        let mut done = 0;
        while done < length {
            let column = x + done as i64;
            let offset = column.rem_euclid(COVERAGE_BLOCK as i64) as usize;
            let count = (COVERAGE_BLOCK - offset).min(length - done);
            let block = self.block(selection, column, y);
            result[done..done + count].copy_from_slice(&block[offset..offset + count]);
            done += count;
        }
        result
    }
}

#[cfg(test)]
mod coverage_cache_tests {
    use super::*;
    #[test]
    fn cache_reuses_blocks_bounds_memory_and_invalidates_only_with_selection_identity() {
        let triangle =
            SelectionShape::polygon(vec![[0., 0.], [80., 0.], [0., 80.]], true, 4.).unwrap();
        let selection = Selection::default()
            .apply(triangle, SelectionOperation::Replace, 1000000, 1000000)
            .unwrap();
        let mut cache = SelectionCoverageCache::default();
        let first = cache.block(&selection, 0, 10);
        let hit = cache.block(&selection, 0, 10);
        assert!(Arc::ptr_eq(&first, &hit));
        let row = cache.row(&selection, -5, 10, 64);
        for (i, &value) in row.iter().enumerate() {
            assert!((value - selection.coverage(-4.5 + i as f64, 10.5)).abs() < 1e-6);
        }
        for y in 100..(100 + MAX_COVERAGE_BLOCKS as i64) {
            cache.row(&selection, 0, y, 64);
        }
        assert_eq!(cache.blocks.len(), MAX_COVERAGE_BLOCKS);
        assert_eq!(cache.order.len(), MAX_COVERAGE_BLOCKS);
        assert!(!cache.blocks.contains_key(&(0, 10)));
        let inverted = selection.inverted(1000000, 1000000).unwrap();
        let reverse = cache.block(&inverted, 0, 10);
        assert_eq!(cache.blocks.len(), 1);
        for (a, b) in first.iter().zip(reverse.iter()) {
            assert!((a + b - 1.).abs() < 1e-6);
        }
        let restored = cache.block(&selection, 0, 10);
        assert_eq!(&*first, &*restored);
        assert!(!Arc::ptr_eq(&first, &restored));
        cache.clear();
        assert!(cache.blocks.is_empty() && cache.order.is_empty() && cache.identity.is_none());
    }
}
