//! Display geometry for the immutable, untransformed 50% tip coverage.
use crate::{BrushMask, Error, Result};

pub const MAX_BRUSH_OUTLINE_SEGMENTS: usize = 65_536;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushOutlineSegment {
    pub start: [f64; 2],
    pub end: [f64; 2],
}

impl BrushMask {
    /// Unit-diameter coordinates, centred at (.5,.5), preserving source aspect.
    /// No simplification or silent truncation occurs at the complexity limit.
    pub fn outline(&self) -> Result<Vec<BrushOutlineSegment>> {
        let (width, height) = self.dimensions();
        let extent = f64::from(width.max(height));
        let coordinate = |i: u32, size: u32| match i {
            0 | 1 => 0.,
            i if i >= size + 2 => f64::from(size),
            i => f64::from(i) - 1.5,
        };
        let value = |x: u32, y: u32| -> f64 {
            if x == 0 || y == 0 || x == width + 3 || y == height + 3 {
                return -127.5;
            }
            let x = x.saturating_sub(2).min(width - 1);
            let y = y.saturating_sub(2).min(height - 1);
            f64::from(self.coverage()[(y * width + x) as usize]) - 127.5
        };
        let normalize = |p: [f64; 2]| {
            [
                0.5 + (p[0] - f64::from(width) / 2.) / extent,
                0.5 + (p[1] - f64::from(height) / 2.) / extent,
            ]
        };
        let mut output = Vec::new();
        for y in 0..height + 3 {
            for x in 0..width + 3 {
                let points = [
                    [coordinate(x, width), coordinate(y, height)],
                    [coordinate(x + 1, width), coordinate(y, height)],
                    [coordinate(x + 1, width), coordinate(y + 1, height)],
                    [coordinate(x, width), coordinate(y + 1, height)],
                ];
                let values = [
                    value(x, y),
                    value(x + 1, y),
                    value(x + 1, y + 1),
                    value(x, y + 1),
                ];
                let mut crossings = [None; 4];
                let mut edges = [0; 4];
                let mut count = 0;
                for edge in 0..4 {
                    let next = (edge + 1) % 4;
                    if (values[edge] > 0.) != (values[next] > 0.) {
                        let t = values[edge] / (values[edge] - values[next]);
                        crossings[edge] = Some([
                            points[edge][0] + t * (points[next][0] - points[edge][0]),
                            points[edge][1] + t * (points[next][1] - points[edge][1]),
                        ]);
                        edges[count] = edge;
                        count += 1;
                    }
                }
                let mut append = |a: usize, b: usize| -> Result<()> {
                    let start = normalize(crossings[a].expect("crossed edge"));
                    let end = normalize(crossings[b].expect("crossed edge"));
                    if start == end {
                        return Ok(());
                    }
                    if output.len() == MAX_BRUSH_OUTLINE_SEGMENTS {
                        return Err(Error::ResourceLimit("brush outline complexity"));
                    }
                    output.push(BrushOutlineSegment { start, end });
                    Ok(())
                };
                match count {
                    2 => append(edges[0], edges[1])?,
                    4 => {
                        // Positive determinant connects top-left/bottom-right samples.
                        if values[0] * values[2] - values[1] * values[3] >= 0. {
                            append(0, 1)?;
                            append(2, 3)?;
                        } else {
                            append(0, 3)?;
                            append(1, 2)?;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(output)
    }
}
