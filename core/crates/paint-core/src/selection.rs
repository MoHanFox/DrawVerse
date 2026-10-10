use crate::{Error, Result};
use std::sync::Arc;

pub const MAX_SELECTION_STEPS: usize = 64;
/// Masks are per-pixel coverage; the cap keeps a single selection step bounded (16 MiB at 1 byte
/// per pixel, i.e. 4096x4096). Larger regions must fail loudly instead of exhausting memory.
pub const MAX_SELECTION_MASK_BYTES: usize = 16 * 1024 * 1024;
/// Lasso paths are sampled by the UI; the cap bounds both memory and rasterization work.
pub const MAX_SELECTION_POINTS: usize = 4096;
/// Magic-wand tolerance is a per-channel 0..=255 threshold over the composited document.
pub const MAX_WAND_TOLERANCE: u32 = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionKind {
    Rectangle,
    Ellipse,
    /// Freehand path: rasterized once into `mask`, then behaves like any other shape.
    Polygon,
    /// Pixel region derived from document content (magic wand).
    Mask,
}

impl SelectionKind {
    /// Geometry-only shapes answer `coverage` analytically; the other two use the mask.
    pub fn is_geometric(self) -> bool {
        matches!(self, SelectionKind::Rectangle | SelectionKind::Ellipse)
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
pub struct SelectionShape {
    pub kind: SelectionKind,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub antialias: bool,
    /// Present for `Polygon`/`Mask`; the geometry above is then the bounding box.
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
            mask: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
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
    pub fn coverage(self, x: f64, y: f64) -> f32 {
        if let Some(mask) = self.mask {
            // Mask shapes are addressed by integer pixel: the stored value is the pixel center's
            // coverage, produced by rasterization (polygon) or flood fill (wand).
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

/// Even-odd point-in-polygon test at a pixel center.
fn inside_polygon(points: &[[f64; 2]], x: f64, y: f64) -> bool {
    let mut inside = false;
    let mut previous = points.len() - 1;
    for current in 0..points.len() {
        let [xi, yi] = points[current];
        let [xj, yj] = points[previous];
        if (yi > y) != (yj > y) {
            let cross = (xj - xi) * (y - yi) / (yj - yi) + xi;
            if x < cross {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn polygon_is_simple(points: &[[f64; 2]]) -> bool {
    // A closed path with at least three distinct vertices and finite coordinates. Self-touching
    // paths are allowed: the even-odd rule gives a deterministic result either way.
    let mut distinct = Vec::with_capacity(points.len());
    for point in points {
        if !point[0].is_finite()
            || !point[1].is_finite()
            || point[0].abs() > 2_000_000.
            || point[1].abs() > 2_000_000.
        {
            return false;
        }
        if distinct.last().is_none_or(|last| *last != *point) {
            distinct.push(*point);
        }
    }
    distinct.len() >= 3
}

/// Rasterize a closed polygon into a selection shape. Coordinates are document pixels and are
/// clipped to the document; the returned geometry is the bounding box of the produced mask.
pub fn polygon_shape(points: &[[f64; 2]], width: u32, height: u32) -> Result<SelectionShape> {
    if points.len() < 3 || points.len() > MAX_SELECTION_POINTS {
        return Err(Error::InvalidArgument("invalid selection point count"));
    }
    if !polygon_is_simple(points) {
        return Err(Error::InvalidArgument("invalid selection path"));
    }
    let clamp = |value: f64, limit: u32| value.clamp(0., f64::from(limit));
    let mut min_x = f64::from(width);
    let mut min_y = f64::from(height);
    let mut max_x = 0f64;
    let mut max_y = 0f64;
    for point in points {
        let x = clamp(point[0], width);
        let y = clamp(point[1], height);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    let left = min_x.floor() as u32;
    let top = min_y.floor() as u32;
    let right = (max_x.ceil() as u32).min(width);
    let bottom = (max_y.ceil() as u32).min(height);
    if right <= left || bottom <= top {
        return Err(Error::InvalidArgument("empty selection path"));
    }
    let mask_width = right - left;
    let mask_height = bottom - top;
    let bytes = (mask_width as usize) * (mask_height as usize);
    if bytes > MAX_SELECTION_MASK_BYTES {
        return Err(Error::ResourceLimit("selection mask size"));
    }
    let mut coverage = vec![0u8; bytes];
    for row in 0..mask_height {
        let y = f64::from(top + row) + 0.5;
        for column in 0..mask_width {
            let x = f64::from(left + column) + 0.5;
            if inside_polygon(points, x, y) {
                coverage[(row as usize) * (mask_width as usize) + (column as usize)] = 1;
            }
        }
    }
    if !coverage.contains(&1) {
        return Err(Error::InvalidArgument("empty selection path"));
    }
    Ok(SelectionShape {
        kind: SelectionKind::Polygon,
        x: f64::from(left),
        y: f64::from(top),
        width: f64::from(mask_width),
        height: f64::from(mask_height),
        antialias: false,
        mask: Some(SelectionMask::from_coverage(
            mask_width,
            mask_height,
            coverage,
        )?),
    })
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
            let shape = step.shape.as_ref().map_or(0., |s| s.clone().coverage(x, y));
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
    pub fn to_text(&self) -> String {
        let mut text = format!("2|{}", u8::from(self.enabled));
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
                "2" if version >= 2 => SelectionKind::Polygon,
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
            let mask = if kind.is_geometric() {
                if fields.len() != 7 {
                    return Err(invalid());
                }
                None
            } else {
                if fields.len() != 8 || width < 1. || height < 1. {
                    return Err(invalid());
                }
                Some(mask_from_text(fields[7], width as u32, height as u32)?)
            };
            steps.push(SelectionStep {
                operation,
                shape: Some(SelectionShape {
                    kind,
                    x,
                    y,
                    width,
                    height,
                    antialias,
                    mask,
                }),
            });
        }
        Self::from_steps(enabled, steps)
    }
}
