//! Pure numeric requests shared by CPU and GPU. Coordinates are tile-local f32;
//! document ownership, histories and publishing deliberately live above this layer.
use std::fmt::Debug;

pub type Rgba = [f32; 4];
pub const MAX_SAMPLES: usize = 8_000_000;
pub const MAX_SOURCE_TILES: usize = 2048;

#[derive(Clone, Debug)]
pub struct Grid {
    pub origin: [i32; 2],
    pub width: u32,
    pub height: u32,
    pub cell: f32,
    pub vectors: Vec<[f32; 2]>,
}
impl Grid {
    pub fn at(&self, x: i32, y: i32) -> [f32; 2] {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            [0.; 2]
        } else {
            self.vectors[(y as u32 * self.width + x as u32) as usize]
        }
    }
    pub fn sample(&self, x: f32, y: f32) -> [f32; 2] {
        let x = (x - self.origin[0] as f32) / self.cell;
        let y = (y - self.origin[1] as f32) / self.cell;
        let (ix, iy) = (x.floor() as i32, y.floor() as i32);
        let (fx, fy) = (x - x.floor(), y - y.floor());
        let (a, b, c, d) = (
            self.at(ix, iy),
            self.at(ix + 1, iy),
            self.at(ix, iy + 1),
            self.at(ix + 1, iy + 1),
        );
        [0, 1].map(|k| lerp(lerp(a[k], b[k], fx), lerp(c[k], d[k], fx), fy))
    }
    fn validate(&self) -> Result<(), String> {
        let count = u64::from(self.width) * u64::from(self.height);
        if self.width == 0
            || self.height == 0
            || count > MAX_SAMPLES as u64
            || count as usize != self.vectors.len()
            || !self.cell.is_finite()
            || self.cell < 1.
            || self.origin.iter().any(|v| v.unsigned_abs() > 4_000_000)
            || self
                .vectors
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 2_000_000.)
        {
            return Err("invalid displacement grid".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ElasticOp {
    pub grid: Grid,
    /// Output node rectangle, in the grid's node coordinates.
    pub start: [u32; 2],
    pub size: [u32; 2],
    pub center: [f32; 2],
    pub delta: [f32; 2],
    pub radius: f32,
    pub strength: f32,
    pub density: f32,
    pub time: f32,
    pub speed: f32,
    pub tool: u32,
    pub selection: Vec<f32>,
}
#[derive(Clone, Debug)]
pub struct SourceTile {
    pub coord: [i32; 2],
    pub pixels: Vec<Rgba>,
}
#[derive(Clone, Debug)]
pub struct RemapOp {
    pub grid: Grid,
    /// Source tiles sorted lexicographically by (x, y). Missing tiles use white/transparent.
    pub source: Vec<SourceTile>,
    pub white: [u32; 2],
    pub tiles: Vec<[i32; 2]>,
    /// One coverage per output pixel; empty means no selection.
    pub selection: Vec<f32>,
    /// 1 = final, 2/4 = inexpensive local block preview.
    pub stride: u32,
    pub cubic: bool,
}
#[derive(Clone, Debug)]
pub enum Operator {
    Elastic(ElasticOp),
    Remap(RemapOp),
}

pub trait Executor: Debug + Send + Sync {
    fn execute(&self, operator: &Operator) -> Result<Vec<Rgba>, String>;
    fn name(&self) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_block_reuse_matches_independent_pixels_exactly() {
        for stride in [2, 4] {
            for cubic in [false, true] {
                let op = RemapOp {
                    grid: Grid {
                        origin: [-70, -6],
                        width: 72,
                        height: 38,
                        cell: 2.,
                        vectors: vec![[0.3, -0.7]; 72 * 38],
                    },
                    source: vec![SourceTile {
                        coord: [-1, 0],
                        pixels: (0..4096)
                            .map(|i| [(i % 64) as f32 / 128., 0.1, 0.2, 0.7])
                            .collect(),
                    }],
                    white: [64, 64],
                    tiles: vec![[-1, 0], [0, 0]],
                    selection: (0..8192).map(|i| (i % 5) as f32 / 4.).collect(),
                    stride,
                    cubic,
                };
                assert_eq!(
                    op.pixels(),
                    (0..8192).map(|i| op.pixel(i)).collect::<Vec<_>>()
                );
            }
        }
    }
}
#[derive(Debug, Default)]
pub struct Cpu;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1. - t) + b * t
}
pub fn smooth_kernel(r: f32, density: f32) -> f32 {
    let inner = density * 0.85;
    let t = ((r - inner) / (1. - inner)).clamp(0., 1.);
    1. - t * t * (3. - 2. * t)
}
impl ElasticOp {
    fn pixel(&self, index: usize) -> Rgba {
        let x = self.grid.origin[0] as f32
            + (self.start[0] + index as u32 % self.size[0]) as f32 * self.grid.cell;
        let y = self.grid.origin[1] as f32
            + (self.start[1] + index as u32 / self.size[0]) as f32 * self.grid.cell;
        let v = [x - self.center[0], y - self.center[1]];
        let r = (v[0] * v[0] + v[1] * v[1]).sqrt() / self.radius;
        let before = self.grid.sample(x, y);
        let weight = self.strength * smooth_kernel(r, self.density) * self.selection[index];
        if weight <= 0. {
            return [before[0], before[1], 0., 0.];
        }
        if self.tool == 3 {
            let d = before.map(|v| {
                let v = v * (1. - weight).max(0.);
                if v.abs() < 0.0001 {
                    0.
                } else {
                    v
                }
            });
            return [d[0], d[1], 0., 0.];
        }
        let shift = match self.tool {
            0 => [-self.delta[0] * weight, -self.delta[1] * weight],
            1 => {
                let s = (-weight * self.speed * self.time * 2.).exp() - 1.;
                [v[0] * s, v[1] * s]
            }
            _ => {
                let angle = -weight * self.speed * self.time * std::f32::consts::TAU;
                let (s, c) = angle.sin_cos();
                [v[0] * c - v[1] * s - v[0], v[0] * s + v[1] * c - v[1]]
            }
        };
        let q = [x + shift[0], y + shift[1]];
        let d = self.grid.sample(q[0], q[1]);
        // Local stiffness damps grid curvature, without pulling a uniform translation
        // toward zero. Reconstruction is deliberately exempt from this relaxation.
        let h = self.grid.cell;
        let neighbours = [
            self.grid.sample(q[0] - h, q[1]),
            self.grid.sample(q[0] + h, q[1]),
            self.grid.sample(q[0], q[1] - h),
            self.grid.sample(q[0], q[1] + h),
        ];
        let next = [0, 1].map(|k| {
            shift[k]
                + lerp(
                    d[k],
                    neighbours.iter().map(|n| n[k]).sum::<f32>() * 0.25,
                    weight * 0.12,
                )
        });
        [next[0], next[1], 0., 0.]
    }
}
fn cubic_weight(x: f32) -> f32 {
    let x = x.abs();
    if x < 1. {
        1.5 * x * x * x - 2.5 * x * x + 1.
    } else if x < 2. {
        -0.5 * x * x * x + 2.5 * x * x - 4. * x + 2.
    } else {
        0.
    }
}
impl RemapOp {
    fn pixels(&self) -> Vec<Rgba> {
        if self.stride == 1 {
            return (0..self.tiles.len() * 4096)
                .map(|i| self.pixel(i))
                .collect();
        }
        let mut out = vec![[0.; 4]; self.tiles.len() * 4096];
        let stride = self.stride as usize;
        for (t, tile) in self.tiles.iter().enumerate() {
            for y in (0..64).step_by(stride) {
                for x in (0..64).step_by(stride) {
                    let px = tile[0] * 64 + x as i32;
                    let py = tile[1] * 64 + y as i32;
                    let d = self.grid.sample(px as f32, py as f32);
                    let sample = if d == [0.; 2] {
                        [0.; 4]
                    } else {
                        self.sample(px as f32 + d[0], py as f32 + d[1])
                    };
                    for dy in 0..stride {
                        for dx in 0..stride {
                            let index = t * 4096 + (y + dy) * 64 + x + dx;
                            let original = self.source_at(px + dx as i32, py + dy as i32);
                            let selected = if self.selection.is_empty() {
                                1.
                            } else {
                                self.selection[index]
                            };
                            out[index] = if d == [0.; 2] || selected == 0. {
                                original
                            } else {
                                [0, 1, 2, 3].map(|k| lerp(original[k], sample[k], selected))
                            };
                        }
                    }
                }
            }
        }
        out
    }
    fn source_at(&self, x: i32, y: i32) -> Rgba {
        let key = [x.div_euclid(64), y.div_euclid(64)];
        if let Ok(i) = self.source.binary_search_by_key(&key, |t| t.coord) {
            self.source[i].pixels[(y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize]
        } else if x >= 0 && y >= 0 && x < self.white[0] as i32 && y < self.white[1] as i32 {
            [1.; 4]
        } else {
            [0.; 4]
        }
    }
    fn sample(&self, x: f32, y: f32) -> Rgba {
        let (ix, iy) = (x.floor() as i32, y.floor() as i32);
        let (fx, fy) = (x - x.floor(), y - y.floor());
        if !self.cubic {
            let (a, b, c, d) = (
                self.source_at(ix, iy),
                self.source_at(ix + 1, iy),
                self.source_at(ix, iy + 1),
                self.source_at(ix + 1, iy + 1),
            );
            return [0, 1, 2, 3].map(|k| lerp(lerp(a[k], b[k], fx), lerp(c[k], d[k], fx), fy));
        }
        let mut out = [0.; 4];
        let first = self.source_at(ix, iy);
        let mut uniform = true;
        for dy in -1..=2 {
            for dx in -1..=2 {
                let w = cubic_weight(dx as f32 - fx) * cubic_weight(dy as f32 - fy);
                let p = self.source_at(ix + dx, iy + dy);
                uniform &= p == first;
                for k in 0..4 {
                    out[k] += p[k] * w;
                }
            }
        }
        if uniform {
            return first;
        }
        out[3] = out[3].clamp(0., 1.);
        for k in 0..3 {
            out[k] = out[k].clamp(0., out[3]);
        }
        out
    }
    fn pixel(&self, index: usize) -> Rgba {
        let tile = self.tiles[index / 4096];
        let offset = (index % 4096) as i32;
        let (x, y) = (tile[0] * 64 + offset % 64, tile[1] * 64 + offset / 64);
        let selected = if self.selection.is_empty() {
            1.
        } else {
            self.selection[index]
        };
        let original = self.source_at(x, y);
        if selected == 0. {
            return original;
        }
        let stride = self.stride as i32;
        let (px, py) = (x.div_euclid(stride) * stride, y.div_euclid(stride) * stride);
        let d = self.grid.sample(px as f32, py as f32);
        if d == [0., 0.] {
            return original;
        }
        let sampled = self.sample(px as f32 + d[0], py as f32 + d[1]);
        [0, 1, 2, 3].map(|k| lerp(original[k], sampled[k], selected))
    }
}
impl Operator {
    pub fn count(&self) -> usize {
        match self {
            Self::Elastic(v) => (v.size[0] as usize) * (v.size[1] as usize),
            Self::Remap(v) => v.tiles.len() * 4096,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let count = self.count();
        if count == 0 || count > MAX_SAMPLES {
            return Err("operator work budget".into());
        }
        let (grid, weights) = match self {
            Self::Elastic(v) => {
                if v.size.contains(&0)
                    || v.start[0]
                        .checked_add(v.size[0])
                        .is_none_or(|x| x > v.grid.width)
                    || v.start[1]
                        .checked_add(v.size[1])
                        .is_none_or(|y| y > v.grid.height)
                    || v.tool > 3
                    || !v.radius.is_finite()
                    || !(0.5..=1000.).contains(&v.radius)
                    || [v.strength, v.density, v.speed]
                        .iter()
                        .any(|x| !x.is_finite() || !(0. ..=1.).contains(x))
                    || !v.time.is_finite()
                    || !(0. ..=0.25).contains(&v.time)
                    || v.center
                        .iter()
                        .chain(&v.delta)
                        .any(|x| !x.is_finite() || x.abs() > 4_000_000.)
                    || v.selection.len() != count
                {
                    return Err("invalid elastic request".into());
                }
                (&v.grid, &v.selection)
            }
            Self::Remap(v) => {
                if ![1, 2, 4].contains(&v.stride)
                    || v.white.iter().any(|x| *x > 1_000_000)
                    || v.source.len() > MAX_SOURCE_TILES
                    || (!v.selection.is_empty() && v.selection.len() != count)
                    || v.tiles.iter().flatten().any(|x| x.unsigned_abs() > 62500)
                    || v.source.windows(2).any(|w| w[0].coord >= w[1].coord)
                    || v.source.iter().any(|t| {
                        t.coord.iter().any(|x| x.unsigned_abs() > 62500)
                            || t.pixels.len() != 4096
                            || t.pixels.iter().any(|p| {
                                p.iter().any(|x| !x.is_finite() || !(0. ..=1.).contains(x))
                                    || p[..3].iter().any(|x| *x > p[3])
                            })
                    })
                {
                    return Err("invalid remap request".into());
                }
                (&v.grid, &v.selection)
            }
        };
        grid.validate()?;
        if weights
            .iter()
            .any(|x| !x.is_finite() || !(0. ..=1.).contains(x))
        {
            return Err("invalid coverage".into());
        }
        Ok(())
    }
}
impl Executor for Cpu {
    fn name(&self) -> String {
        "CPU".into()
    }
    fn execute(&self, op: &Operator) -> Result<Vec<Rgba>, String> {
        op.validate()?;
        Ok(match op {
            Operator::Elastic(v) => (0..op.count()).map(|i| v.pixel(i)).collect(),
            Operator::Remap(v) => v.pixels(),
        })
    }
}

/// Shared host/shader wire plan. Integer metadata uses bit patterns, never lossy float casts.
/// First 12 vec4s are the header, followed by grid, weights, output keys, source keys and pixels.
pub fn pack(op: &Operator) -> Result<Vec<Rgba>, String> {
    op.validate()?;
    let (grid, weights) = match op {
        Operator::Elastic(v) => (&v.grid, &v.selection),
        Operator::Remap(v) => (&v.grid, &v.selection),
    };
    let mut data = vec![[0.; 4]; 12];
    data[0] = [
        f32::from_bits(u32::from(matches!(op, Operator::Remap(_)))),
        f32::from_bits(op.count() as u32),
        f32::from_bits(grid.width),
        f32::from_bits(grid.height),
    ];
    data[1] = [grid.origin[0] as f32, grid.origin[1] as f32, grid.cell, 0.];
    data.extend(grid.vectors.iter().map(|v| [v[0], v[1], 0., 0.]));
    data[2][0] = f32::from_bits(data.len() as u32);
    data[2][1] = f32::from_bits(weights.len() as u32);
    data.extend(weights.iter().map(|w| [*w, 0., 0., 0.]));
    match op {
        Operator::Elastic(v) => {
            data[3] = [
                v.start[0] as f32,
                v.start[1] as f32,
                v.size[0] as f32,
                v.size[1] as f32,
            ];
            data[4] = [v.center[0], v.center[1], v.delta[0], v.delta[1]];
            data[5] = [v.radius, v.strength, v.density, v.time];
            data[6] = [v.speed, f32::from_bits(v.tool), 0., 0.];
        }
        Operator::Remap(v) => {
            data[3] = [
                f32::from_bits(data.len() as u32),
                f32::from_bits(v.tiles.len() as u32),
                f32::from_bits(v.stride),
                f32::from_bits(u32::from(v.cubic)),
            ];
            data.extend(v.tiles.iter().map(|p| [p[0] as f32, p[1] as f32, 0., 0.]));
            data[4] = [
                f32::from_bits(data.len() as u32),
                f32::from_bits(v.source.len() as u32),
                v.white[0] as f32,
                v.white[1] as f32,
            ];
            let pixels = data.len() + v.source.len();
            data.extend(v.source.iter().enumerate().map(|(i, t)| {
                [
                    t.coord[0] as f32,
                    t.coord[1] as f32,
                    f32::from_bits((pixels + i * 4096) as u32),
                    0.,
                ]
            }));
            for t in &v.source {
                data.extend_from_slice(&t.pixels);
            }
        }
    }
    Ok(data)
}
