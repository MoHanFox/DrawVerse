use super::{allocated, count, Error, Result};

/// Squared distance to the closest finite parabola. Infinite sites are absent.
fn distance_line(input: &[f64], output: &mut [f64], sites: &mut [usize], edges: &mut [f64]) {
    let mut count = 0;
    for (q, &value) in input.iter().enumerate().filter(|(_, v)| v.is_finite()) {
        let mut crossing = f64::NEG_INFINITY;
        while count > 0 {
            let p = sites[count - 1];
            crossing = ((value + (q as f64).powi(2)) - (input[p] + (p as f64).powi(2)))
                / (2. * (q - p) as f64);
            if crossing > edges[count - 1] {
                break;
            }
            count -= 1;
        }
        sites[count] = q;
        edges[count] = if count == 0 {
            f64::NEG_INFINITY
        } else {
            crossing
        };
        count += 1;
    }
    if count == 0 {
        output.fill(f64::INFINITY);
        return;
    }
    let mut site = 0;
    for (q, result) in output.iter_mut().enumerate() {
        while site + 1 < count && edges[site + 1] < q as f64 {
            site += 1;
        }
        let p = sites[site];
        *result = (q as f64 - p as f64).powi(2) + input[p];
    }
}

/// In-place separable Euclidean transform, columns then rows. Seeds are
/// nonnegative squared distances (or +infinity). Validation and all scratch
/// allocation precede mutation; no failure occurs after the first write.
pub fn squared_distance_transform(width: u32, height: u32, values: &mut [f64]) -> Result<()> {
    let n = count(width, height)?;
    if values.len() != n || values.iter().any(|v| v.is_nan() || *v < 0.) {
        return Err(Error::Invalid("distance seeds"));
    }
    let (width, height) = (width as usize, height as usize);
    let longest = width.max(height);
    let mut line = allocated(longest, 0_f64)?;
    let mut output = allocated(longest, 0_f64)?;
    let mut sites = allocated(longest, 0_usize)?;
    let mut edges = allocated(longest, 0_f64)?;
    for x in 0..width {
        for y in 0..height {
            line[y] = values[y * width + x];
        }
        distance_line(
            &line[..height],
            &mut output[..height],
            &mut sites,
            &mut edges,
        );
        for y in 0..height {
            values[y * width + x] = output[y];
        }
    }
    for y in 0..height {
        let row = &mut values[y * width..(y + 1) * width];
        distance_line(row, &mut output[..width], &mut sites, &mut edges);
        row.copy_from_slice(&output[..width]);
    }
    Ok(())
}

/// Signed distance of 50%-coverage geometry, negative inside. A remains a
/// separate immutable mask for final alpha composition. Outside the raster is
/// transparent; the empty raster has a finite positive sentinel.
#[derive(Debug)]
pub struct DistanceField {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) values: Vec<f32>,
}
impl DistanceField {
    pub fn from_alpha(width: u32, height: u32, alpha: &[u8]) -> Result<Self> {
        let n = count(width, height)?;
        if alpha.len() != n {
            return Err(Error::Invalid("alpha size"));
        }
        let far = (f64::from(width).hypot(f64::from(height)) + 1.) as f32;
        let mut values = allocated(n, far)?;
        if !alpha.iter().any(|a| *a >= 128) {
            return Ok(Self {
                width,
                height,
                values,
            });
        }
        let mut distance = allocated(n, f64::INFINITY)?;
        for target_inside in [true, false] {
            for (d, a) in distance.iter_mut().zip(alpha) {
                *d = if (*a >= 128) == target_inside {
                    0.
                } else {
                    f64::INFINITY
                };
            }
            squared_distance_transform(width, height, &mut distance)?;
            for (i, &d) in distance.iter().enumerate() {
                if (alpha[i] >= 128) == target_inside {
                    continue;
                }
                let (x, y) = (i % width as usize, i / width as usize);
                let border = (x + 1)
                    .min(width as usize - x)
                    .min(y + 1)
                    .min(height as usize - y) as f64;
                let d = if target_inside {
                    d.sqrt().min(f64::from(far))
                } else {
                    d.sqrt().min(border)
                } - 0.5;
                values[i] = if target_inside { d as f32 } else { -d as f32 };
            }
        }
        Ok(Self {
            width,
            height,
            values,
        })
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    pub(super) fn sample(&self, p: [f32; 2]) -> f32 {
        let x = (p[0] - 0.5).clamp(0., self.width as f32 - 1.);
        let y = (p[1] - 0.5).clamp(0., self.height as f32 - 1.);
        let (ix, iy) = (x.floor() as usize, y.floor() as usize);
        let (nx, ny) = (
            (ix + 1).min(self.width as usize - 1),
            (iy + 1).min(self.height as usize - 1),
        );
        let row = |y: usize| {
            let a = self.values[y * self.width as usize + ix];
            a + (self.values[y * self.width as usize + nx] - a) * (x - ix as f32)
        };
        row(iy) + (row(ny) - row(iy)) * (y - iy as f32)
    }
    pub(super) fn normal(&self, p: [f32; 2], fallback: [f32; 2]) -> [f32; 2] {
        let dx = self.sample([p[0] + 0.5, p[1]]) - self.sample([p[0] - 0.5, p[1]]);
        let dy = self.sample([p[0], p[1] + 0.5]) - self.sample([p[0], p[1] - 0.5]);
        let len = dx.hypot(dy);
        if len > 1e-6 {
            [dx / len, dy / len]
        } else {
            fallback
        }
    }
}
