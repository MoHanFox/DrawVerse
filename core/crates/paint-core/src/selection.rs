use crate::{Error, Result};
use std::sync::Arc;

pub const MAX_SELECTION_STEPS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionKind {
    Rectangle,
    Ellipse,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionOperation {
    Replace,
    Add,
    Subtract,
    Intersect,
    Invert,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionShape {
    pub kind: SelectionKind,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub antialias: bool,
}
impl SelectionShape {
    pub fn validate(self) -> Result<()> {
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
    fn coverage(self, x: f64, y: f64) -> f32 {
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
#[derive(Clone, Copy, Debug, PartialEq)]
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
        for step in &steps {
            match (step.operation, step.shape) {
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
                shape: Some(SelectionShape {
                    kind: SelectionKind::Rectangle,
                    x: 0.,
                    y: 0.,
                    width: f64::from(width),
                    height: f64::from(height),
                    antialias: false,
                }),
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
            let shape = step.shape.map_or(0., |s| s.coverage(x, y));
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
        let mut text = format!("1|{}", u8::from(self.enabled));
        for step in self.steps.iter() {
            let op = match step.operation {
                SelectionOperation::Replace => 0,
                SelectionOperation::Add => 1,
                SelectionOperation::Subtract => 2,
                SelectionOperation::Intersect => 3,
                SelectionOperation::Invert => 4,
            };
            if let Some(s) = step.shape {
                text.push_str(&format!(
                    ";{op},{},{},{},{},{},{}",
                    u8::from(s.kind == SelectionKind::Ellipse),
                    u8::from(s.antialias),
                    s.x,
                    s.y,
                    s.width,
                    s.height
                ));
            } else {
                text.push_str(";4,0,0,0,0,0,0");
            }
        }
        text
    }
    pub fn from_text(text: &str) -> Result<Self> {
        let invalid = || Error::InvalidArgument("invalid selection encoding");
        if text.len() > 32768 || !text.is_ascii() {
            return Err(invalid());
        }
        let mut parts = text.split(';');
        let enabled = match parts.next() {
            Some("1|0") => false,
            Some("1|1") => true,
            _ => return Err(invalid()),
        };
        let mut steps = Vec::new();
        for part in parts {
            if steps.len() >= MAX_SELECTION_STEPS {
                return Err(Error::ResourceLimit("selection step budget"));
            }
            let fields: Vec<_> = part.split(',').collect();
            if fields.len() != 7 {
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
            let shape = SelectionShape {
                kind: match fields[1] {
                    "0" => SelectionKind::Rectangle,
                    "1" => SelectionKind::Ellipse,
                    _ => return Err(invalid()),
                },
                antialias: match fields[2] {
                    "0" => false,
                    "1" => true,
                    _ => return Err(invalid()),
                },
                x: fields[3].parse().map_err(|_| invalid())?,
                y: fields[4].parse().map_err(|_| invalid())?,
                width: fields[5].parse().map_err(|_| invalid())?,
                height: fields[6].parse().map_err(|_| invalid())?,
            };
            steps.push(SelectionStep {
                operation,
                shape: Some(shape),
            });
        }
        Self::from_steps(enabled, steps)
    }
}
