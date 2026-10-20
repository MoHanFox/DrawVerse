use super::{
    allocated, push, valid_width, DistanceField, Error, Placement, Result, MAX_SEGMENTS, MAX_STAMPS,
};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stamp {
    pub center: [f32; 2],
    pub tangent: [f32; 2],
    pub normal: [f32; 2],
    pub radius: f32,
    /// Radians, aligned with the tangent.
    pub texture_rotation: f32,
}
#[derive(Debug)]
struct Path {
    range: Range<usize>,
    closed: bool,
}
#[derive(Debug)]
pub struct Isolines {
    points: Vec<[f32; 2]>,
    paths: Vec<Path>,
}
impl Isolines {
    pub fn paths(&self) -> impl Iterator<Item = (&[[f32; 2]], bool)> {
        self.paths
            .iter()
            .map(|p| (&self.points[p.range.clone()], p.closed))
    }

    /// Equally spaced arclength samples, target spacing .4W. Closed paths do
    /// not duplicate their seam. Short contours still receive one stamp.
    pub fn stamps(&self, field: &DistanceField, width: f32) -> Result<Vec<Stamp>> {
        valid_width(width)?;
        let mut stamps = Vec::new();
        if width == 0. {
            return Ok(stamps);
        }
        for (points, closed) in self.paths() {
            let edges = points.len().saturating_sub(usize::from(!closed));
            let edge = |i: usize| (points[i], points[(i + 1) % points.len()]);
            let length = |i: usize| {
                let (a, b) = edge(i);
                f64::from(b[0] - a[0]).hypot(f64::from(b[1] - a[1]))
            };
            let total: f64 = (0..edges).map(length).sum();
            if total <= 1e-9 {
                continue;
            }
            let intervals = (total / (f64::from(width) * 0.4)).ceil().max(1.);
            let count = intervals + if closed { 0. } else { 1. };
            if count > (MAX_STAMPS - stamps.len()) as f64 {
                return Err(Error::Limit("stamps"));
            }
            let count = count as usize;
            let step = total / intervals;
            let (mut segment, mut start) = (0, 0.);
            for i in 0..count {
                let target = (i as f64 * step).min(total);
                while segment + 1 < edges && start + length(segment) < target {
                    start += length(segment);
                    segment += 1;
                }
                while segment + 1 < edges && length(segment) <= 1e-9 {
                    segment += 1;
                }
                let (a, b) = edge(segment);
                let len = length(segment);
                if len <= 1e-9 {
                    continue;
                }
                let t = ((target - start) / len).clamp(0., 1.) as f32;
                let center = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                let fallback = [(b[1] - a[1]) / len as f32, (a[0] - b[0]) / len as f32];
                let normal = field.normal(center, fallback);
                let tangent = [normal[1], -normal[0]];
                push(
                    &mut stamps,
                    Stamp {
                        center,
                        tangent,
                        normal,
                        radius: width * 0.5,
                        texture_rotation: tangent[1].atan2(tangent[0]),
                    },
                    MAX_STAMPS,
                )?;
            }
        }
        Ok(stamps)
    }
}

#[derive(Debug)]
struct Segment {
    points: [[f32; 2]; 2],
    // Encoded other segment * 2 + entry endpoint + 1; zero is an open end.
    links: [u32; 2],
}
impl DistanceField {
    /// Extract piecewise-linear isolines of the pixel-centre grid. Provide
    /// transparent padding for closed outside outlines; field edges can clip.
    pub fn isolines(&self, level: f32) -> Result<Isolines> {
        if !level.is_finite() {
            return Err(Error::Invalid("isoline level"));
        }
        let (w, h) = (self.width as usize, self.height as usize);
        let mut result = Isolines {
            points: Vec::new(),
            paths: Vec::new(),
        };
        if w < 2 || h < 2 {
            return Ok(result);
        }
        let horizontal = (w - 1) * h;
        let mut edge_owner = allocated(horizontal + w * (h - 1), 0_u32)?;
        let mut segments: Vec<Segment> = Vec::new();
        for y in 0..h - 1 {
            for x in 0..w - 1 {
                let values = [
                    self.values[y * w + x],
                    self.values[y * w + x + 1],
                    self.values[(y + 1) * w + x + 1],
                    self.values[(y + 1) * w + x],
                ];
                let positive = values.map(|v| v >= level);
                let mut crossings = [0_usize; 4];
                let mut n = 0;
                for e in 0..4 {
                    if positive[e] != positive[(e + 1) % 4] {
                        crossings[n] = e;
                        n += 1;
                    }
                }
                if n == 0 {
                    continue;
                }
                let pairs = if n == 2 {
                    [[crossings[0], crossings[1]], [0, 0]]
                } else {
                    let v = values.map(|v| f64::from(v) - f64::from(level));
                    let determinant = v[0] * v[2] - v[1] * v[3];
                    if determinant > 0. || (determinant == 0. && !positive[0]) {
                        [[0, 1], [2, 3]]
                    } else {
                        [[0, 3], [1, 2]]
                    }
                };
                let edge_ids = [
                    y * (w - 1) + x,
                    horizontal + y * w + x + 1,
                    (y + 1) * (w - 1) + x,
                    horizontal + y * w + x,
                ];
                let corners = [
                    [x as f32 + 0.5, y as f32 + 0.5],
                    [x as f32 + 1.5, y as f32 + 0.5],
                    [x as f32 + 1.5, y as f32 + 1.5],
                    [x as f32 + 0.5, y as f32 + 1.5],
                ];
                let intersection = |e: usize| {
                    let b = (e + 1) % 4;
                    let t = ((f64::from(level) - f64::from(values[e]))
                        / (f64::from(values[b]) - f64::from(values[e])))
                    .clamp(0., 1.) as f32;
                    [
                        corners[e][0] + (corners[b][0] - corners[e][0]) * t,
                        corners[e][1] + (corners[b][1] - corners[e][1]) * t,
                    ]
                };
                for pair in pairs.into_iter().take(n / 2) {
                    let index = segments.len();
                    push(
                        &mut segments,
                        Segment {
                            points: pair.map(intersection),
                            links: [0; 2],
                        },
                        MAX_SEGMENTS,
                    )?;
                    for (endpoint, e) in pair.into_iter().enumerate() {
                        let owner = &mut edge_owner[edge_ids[e]];
                        let encoded = (index * 2 + endpoint + 1) as u32;
                        if *owner != 0 {
                            let previous = (*owner - 1) as usize;
                            segments[previous / 2].links[previous % 2] = encoded;
                            segments[index].links[endpoint] = *owner;
                        } else {
                            *owner = encoded;
                        }
                    }
                }
            }
        }
        // Open paths first so a scan starting in the middle cannot split one.
        let mut visited = allocated(segments.len(), false)?;
        for open in [true, false] {
            for first in 0..segments.len() {
                if visited[first] {
                    continue;
                }
                let start_endpoint = segments[first].links.iter().position(|l| *l == 0);
                if start_endpoint.is_some() != open {
                    continue;
                }
                let range_start = result.points.len();
                let (mut current, mut entry) = (first, start_endpoint.unwrap_or(0));
                push(
                    &mut result.points,
                    segments[current].points[entry],
                    MAX_SEGMENTS * 2,
                )?;
                let closed = loop {
                    visited[current] = true;
                    let exit = 1 - entry;
                    let link = segments[current].links[exit];
                    if link != 0 && (link as usize - 1) / 2 == first {
                        break true;
                    }
                    push(
                        &mut result.points,
                        segments[current].points[exit],
                        MAX_SEGMENTS * 2,
                    )?;
                    if link == 0 {
                        break false;
                    }
                    let next = link as usize - 1;
                    current = next / 2;
                    entry = next % 2;
                    if visited[current] {
                        return Err(Error::Invalid("contour topology"));
                    }
                };
                let range = range_start..result.points.len();
                push(&mut result.paths, Path { range, closed }, MAX_SEGMENTS)?;
            }
        }
        Ok(result)
    }
    pub fn outline_stamps(&self, width: f32, placement: Placement) -> Result<Vec<Stamp>> {
        valid_width(width)?;
        if width == 0. {
            return Ok(Vec::new());
        }
        self.isolines(placement.level(width))?.stamps(self, width)
    }
}
