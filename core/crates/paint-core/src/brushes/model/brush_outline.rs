//! Display geometry for the immutable, untransformed 50% tip coverage.
use crate::{BrushMask, Error, Result};

pub const MAX_BRUSH_OUTLINE_SEGMENTS: usize = 65_536;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushOutlineSegment {
    pub start: [f64; 2],
    pub end: [f64; 2],
}

impl BrushMask {
    /// Bounded display geometry. Conservative pooling keeps thin features visible;
    /// coordinates retain the original aspect even when a reduced axis is one pixel.
    pub fn display_outline(&self) -> Result<Vec<BrushOutlineSegment>> {
        const MAX_DISPLAY_SEGMENTS: usize = 2048;
        let (original_width, original_height) = self.dimensions();
        let original_extent = f64::from(original_width.max(original_height));
        if original_extent <= 128. {
            let exact = self.outline()?;
            if exact.len() <= MAX_DISPLAY_SEGMENTS {
                return Ok(exact);
            }
        }
        let mut reduced: Option<BrushMask> = None;
        let mut edge = 128u32;
        loop {
            let source = reduced.as_ref().unwrap_or(self);
            let (width, height) = source.dimensions();
            let extent = width.max(height);
            let target_width = (width * edge).div_ceil(extent).max(1);
            let target_height = (height * edge).div_ceil(extent).max(1);
            let mut coverage = vec![0; (target_width * target_height) as usize];
            for y in 0..target_height {
                for x in 0..target_width {
                    let mut maximum = 0;
                    for sy in y * height / target_height..((y + 1) * height).div_ceil(target_height)
                    {
                        for sx in x * width / target_width..((x + 1) * width).div_ceil(target_width)
                        {
                            maximum = maximum.max(source.coverage()[(sy * width + sx) as usize]);
                        }
                    }
                    coverage[(y * target_width + x) as usize] = maximum;
                }
            }
            let mask = BrushMask::new(target_width, target_height, coverage)?;
            let mut outline = mask.outline()?;
            if outline.len() <= MAX_DISPLAY_SEGMENTS {
                let extent = f64::from(target_width.max(target_height));
                let scale_x =
                    f64::from(original_width) / original_extent * extent / f64::from(target_width);
                let scale_y = f64::from(original_height) / original_extent * extent
                    / f64::from(target_height);
                for segment in &mut outline {
                    for point in [&mut segment.start, &mut segment.end] {
                        point[0] = 0.5 + (point[0] - 0.5) * scale_x;
                        point[1] = 0.5 + (point[1] - 0.5) * scale_y;
                    }
                }
                return Ok(outline);
            }
            reduced = Some(mask);
            edge /= 2;
        }
    }

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
