use super::*;

pub(super) struct SmudgeDab {
    pixels: Vec<Pixel>,
    x: i64,
    y: i64,
    width: usize,
    pub(super) finger: bool,
}
impl SmudgeDab {
    pub(super) fn at(&self, x: i64, y: i64) -> Pixel {
        self.pixels[(y - self.y) as usize * self.width + (x - self.x) as usize]
    }
}
impl Document {
    pub(super) fn prepare_smudge(
        &self,
        stroke: &Stroke,
        point: InputPoint,
        x0: i64,
        y0: i64,
        x1: i64,
        y1: i64,
    ) -> Result<Option<SmudgeDab>> {
        let BrushMode::Smudge {
            all_layers,
            finger_painting,
            ..
        } = stroke.brush.mode
        else {
            return Ok(None);
        };
        let layer = self
            .layer(stroke.layer)
            .ok_or(Error::LayerNotFound(stroke.layer))?;
        let finger = stroke.first_dab && finger_painting;
        let width = (x1 - x0) as usize;
        let height = (y1 - y0) as usize;
        let mut pixels = Vec::with_capacity(width * height);
        let sample = |x: i64, y: i64| -> Result<Pixel> {
            if x < 0 || y < 0 || x >= i64::from(self.width) || y >= i64::from(self.height) {
                return Ok(Pixel::TRANSPARENT);
            }
            if all_layers {
                self.try_pixel(x as u32, y as u32)
            } else {
                layer.canvas_pixel(x as u32, y as u32)
            }
        };
        let (dx, dy) = (
            point.x - stroke.last_carrier.x,
            point.y - stroke.last_carrier.y,
        );
        for y in y0..y1 {
            for x in x0..x1 {
                if finger {
                    pixels.push(if let Some((tone, _)) = stroke.mask_paint {
                        Pixel::from_straight([0., 0., 0., tone])?
                    } else {
                        stroke.color_sample
                    });
                    continue;
                }
                let sx = x as f64 - dx;
                let sy = y as f64 - dy;
                let ix = sx.floor() as i64;
                let iy = sy.floor() as i64;
                let fx = (sx - ix as f64) as f32;
                let fy = (sy - iy as f64) as f32;
                let top = if fx == 0. {
                    sample(ix, iy)?
                } else {
                    sample(ix, iy)?.mix(sample(ix + 1, iy)?, fx)
                };
                let result = if fy == 0. {
                    top
                } else {
                    let bottom = if fx == 0. {
                        sample(ix, iy + 1)?
                    } else {
                        sample(ix, iy + 1)?.mix(sample(ix + 1, iy + 1)?, fx)
                    };
                    top.mix(bottom, fy)
                };
                pixels.push(result);
            }
        }
        Ok(Some(SmudgeDab {
            pixels,
            x: x0,
            y: y0,
            width,
            finger,
        }))
    }
}
