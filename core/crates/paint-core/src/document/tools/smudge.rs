use super::*;

pub(super) struct SmudgeDab {
    pixels: Vec<Pixel>,
    x: i64,
    y: i64,
    width: usize,
    pub(super) finger: bool,
    pub(super) colored: bool,
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
        let dual = stroke
            .program
            .dual
            .as_ref()
            .filter(|d| finger && d.step().is_none() && d.color.enabled());
        let appearance = layer.appearance;
        let mut colored = false;
        let finger_color = if let Some(dual) = dual {
            let program = &stroke.program;
            let speed_size = program
                .factor_all(
                    stroke.speed,
                    point.tangential_pressure,
                    stroke.dab_distance as f32,
                    point.tilt_magnitude(),
                    point.rotation_unit(),
                    BrushProgram::direction_unit(stroke.direction),
                    stroke.fade_index,
                )
                .0;
            let base_radius = f64::from(
                stroke.brush.radius
                    * stroke.shape_sample.size
                    * if program.tip.pressure_size {
                        program.size_pressure(point.pressure)
                    } else {
                        1.
                    },
            ) * f64::from(speed_size);
            let radius = base_radius * f64::from(stroke.taper_factors.diameter);
            let geometry = program.dab_geometry(
                point,
                if stroke.program.tip_dynamics.angle() == crate::AngleControl::InitialDirection {
                    stroke.initial_direction
                } else {
                    stroke.tip_direction
                },
                stroke.shape_sample,
            );
            let mask = if radius > 0. && program.has_mask() {
                program.mask_dab_with_geometry(radius, geometry)
            } else {
                None
            };
            let texture = program.texture_dab(point, geometry);
            let paper = program.texture.as_ref().is_some_and(|t| t.document_space());
            let flow = program.tip.flow
                * if program.tip.pressure_flow {
                    program.flow_pressure(point.pressure)
                } else {
                    1.
                };
            let limit = program.opacity_pressure(point.pressure).unwrap_or(1.)
                * stroke.taper_factors.opacity;
            let plan = dual
                .dab(
                    point,
                    stroke.direction,
                    stroke.random_stroke,
                    stroke.random_index.saturating_sub(1),
                    stroke.random_child,
                    base_radius,
                    f64::from(speed_size),
                )
                .scaled(stroke.taper_factors.diameter);
            let secondary = if dual.color.use_background() {
                stroke.brush.background_color
            } else {
                stroke.brush.color
            };
            Some(move |x: i64, y: i64| -> Result<Pixel> {
                let wx = x as f64 + f64::from(appearance.offset_x) + 0.5;
                let wy = y as f64 + f64::from(appearance.offset_y) + 0.5;
                let primary = if radius <= 0. {
                    0.
                } else {
                    let shape = if let Some(mask) = &mask {
                        mask.coverage(wx - point.x, wy - point.y)
                    } else {
                        program.procedural_coverage_with_geometry(
                            wx - point.x,
                            wy - point.y,
                            radius,
                            geometry,
                        )
                    };
                    shape
                        * flow
                        * if paper {
                            1.
                        } else {
                            texture.as_ref().map_or(1., |t| t.coverage(wx, wy))
                        }
                } * limit;
                let mut second = 0.;
                for child in 0..plan.count() {
                    let amount = plan.coverage(child, wx, wy) * limit;
                    second += amount * (1. - second);
                }
                Ok(dual.color.compose(
                    stroke.color_sample.scaled(primary),
                    primary,
                    secondary,
                    second,
                    (wx as u32, wy as u32),
                    stroke.random_stroke,
                ))
            })
        } else {
            None
        };
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
                    } else if let Some(color) = &finger_color {
                        colored = true;
                        color(x, y)?
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
            colored,
        }))
    }
}
