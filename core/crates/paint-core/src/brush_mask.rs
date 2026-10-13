use crate::{Error, Result};

pub const MAX_BRUSH_MASK_EDGE: u32 = 4096;
pub const MAX_BRUSH_MASK_BYTES: usize = 16 * 1024 * 1024;
#[derive(Debug)]
struct Level {
    width: u32,
    height: u32,
    coverage: Box<[u8]>,
}
/// Immutable coverage resource with bounded minification levels.
#[derive(Debug)]
pub struct BrushMask {
    levels: Vec<Level>,
    resource_id: Option<String>,
}
impl BrushMask {
    pub fn new(width: u32, height: u32, coverage: Vec<u8>) -> Result<Self> {
        if width == 0 || height == 0 || width > MAX_BRUSH_MASK_EDGE || height > MAX_BRUSH_MASK_EDGE
        {
            return Err(Error::InvalidArgument("brush mask dimensions"));
        }
        let count = width as usize * height as usize;
        if count > MAX_BRUSH_MASK_BYTES || coverage.len() != count {
            return Err(Error::InvalidArgument("brush mask byte count"));
        }
        let mut levels = vec![Level {
            width,
            height,
            coverage: coverage.into_boxed_slice(),
        }];
        while levels.last().is_some_and(|l| l.width > 1 || l.height > 1) {
            let before = levels.last().expect("base level");
            let width = before.width.div_ceil(2);
            let height = before.height.div_ceil(2);
            let mut pixels = vec![0; width as usize * height as usize];
            for y in 0..height {
                for x in 0..width {
                    let mut sum = 0u32;
                    let mut count = 0;
                    for sy in y * 2..(y * 2 + 2).min(before.height) {
                        for sx in x * 2..(x * 2 + 2).min(before.width) {
                            sum += u32::from(before.coverage[(sy * before.width + sx) as usize]);
                            count += 1;
                        }
                    }
                    pixels[(y * width + x) as usize] = ((sum + count / 2) / count) as u8;
                }
            }
            levels.push(Level {
                width,
                height,
                coverage: pixels.into_boxed_slice(),
            });
        }
        Ok(Self {
            levels,
            resource_id: None,
        })
    }
    pub fn with_resource_id(mut self, id: String) -> Result<Self> {
        if id.len() != 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::InvalidArgument("brush mask resource identity"));
        }
        self.resource_id = Some(id);
        Ok(self)
    }
    pub fn resource_id(&self) -> Option<&str> {
        self.resource_id.as_deref()
    }
    pub fn dimensions(&self) -> (u32, u32) {
        (self.levels[0].width, self.levels[0].height)
    }
    pub fn coverage(&self) -> &[u8] {
        &self.levels[0].coverage
    }
    pub fn resident_bytes(&self) -> usize {
        self.levels.iter().map(|l| l.coverage.len()).sum()
    }
    pub(crate) fn lod(&self, diameter: f64) -> (usize, usize, f32) {
        let (w, h) = self.dimensions();
        let lod = (f64::from(w.max(h)) / diameter.max(0.001))
            .log2()
            .clamp(0., (self.levels.len() - 1) as f64);
        let lower = lod.floor() as usize;
        (
            lower,
            (lower + 1).min(self.levels.len() - 1),
            (lod - lower as f64) as f32,
        )
    }
    #[inline]
    pub(crate) fn sample(&self, u: f64, v: f64, lod: (usize, usize, f32)) -> f32 {
        let a = self.sample_level(u, v, lod.0);
        if lod.2 == 0. || lod.0 == lod.1 {
            return a;
        }
        a + (self.sample_level(u, v, lod.1) - a) * lod.2
    }
    fn sample_level(&self, u: f64, v: f64, index: usize) -> f32 {
        let level = &self.levels[index];
        let x = (u * f64::from(level.width) - 0.5).clamp(0., f64::from(level.width - 1));
        let y = (v * f64::from(level.height) - 0.5).clamp(0., f64::from(level.height - 1));
        let (x0, y0) = (x.floor() as u32, y.floor() as u32);
        let (x1, y1) = (
            (x0 + 1).min(level.width - 1),
            (y0 + 1).min(level.height - 1),
        );
        let at = |x, y| f64::from(level.coverage[(y * level.width + x) as usize]) / 255.;
        let a = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * (x - f64::from(x0));
        let b = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * (x - f64::from(x0));
        (a + (b - a) * (y - f64::from(y0))) as f32
    }
}
