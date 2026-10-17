use super::*;
use crate::brush_dual::DualDab;

impl Document {
    fn wet_secondary_child(
        &mut self,
        stroke: &mut Stroke,
        plan: &DualDab<'_>,
        child: usize,
        limit: f32,
    ) -> Result<bool> {
        let Some(geometry) = plan.wet_geometry(child) else {
            stroke
                .wet
                .as_mut()
                .expect("wet pair")
                .secondary
                .as_mut()
                .expect("second ink")
                .reset_path()?;
            return Ok(false);
        };
        let carrier = stroke
            .wet
            .as_ref()
            .expect("wet pair")
            .secondary
            .as_ref()
            .expect("second ink");
        let wet = self.prepare_wet_carrier(stroke, carrier, geometry, |x, y| {
            Ok(plan.shape_coverage(child, x, y))
        })?;
        let appearance = self.layers[self.layer_index(stroke.layer)?].appearance;
        let offset = (
            i64::from(appearance.offset_x),
            i64::from(appearance.offset_y),
        );
        let sample = |x, y| {
            let (u, v) = plan.wet_uv(child, x, y);
            wet.sample(u.clamp(0., 1.), v.clamp(0., 1.))
        };
        let touched = self.wet_color_fields(
            &mut stroke.dual_coverage,
            &mut stroke.wet_secondary_field,
            plan.bounds(child),
            offset,
            limit,
            |x, y| plan.coverage(child, x, y),
            sample,
            (stroke.taper.as_mut(), true),
        )?;
        let changed = self.recompose_color(stroke, touched)?;
        stroke
            .wet
            .as_mut()
            .expect("wet pair")
            .secondary
            .as_mut()
            .expect("second ink")
            .commit(wet)?;
        Ok(changed)
    }
    pub(super) fn wet_colored_secondary_dab(
        &mut self,
        stroke: &mut Stroke,
        point: InputPoint,
    ) -> Result<()> {
        let program = stroke.program.clone();
        let dual = program.dual.as_ref().expect("dual color");
        let (speed_size, _, speed_opacity) = program.factor_all(
            stroke.speed,
            point.tangential_pressure,
            stroke.dab_distance as f32,
            point.tilt_magnitude(),
            point.rotation_unit(),
            BrushProgram::direction_unit(stroke.direction),
            stroke.fade_index,
        );
        let next = stroke
            .dual_index
            .checked_add(1)
            .ok_or(Error::ResourceLimit("secondary dab serial exhausted"))?;
        let radius = f64::from(
            stroke.brush.radius
                * stroke.shape_sample.size
                * if program.tip.pressure_size {
                    program.size_pressure(point.pressure)
                } else {
                    1.
                },
        ) * f64::from(speed_size);
        let plan = dual
            .dab(
                point,
                stroke.direction,
                stroke.random_stroke,
                stroke.dual_index,
                0,
                radius,
                f64::from(speed_size),
            )
            .scaled(stroke.taper_factors.diameter);
        let limit = program.opacity_pressure(point.pressure).unwrap_or(1.)
            * stroke.taper_factors.opacity
            * speed_opacity;
        let mut changed = false;
        for child in 0..plan.count() {
            changed |= self.wet_secondary_child(stroke, &plan, child, limit)?;
        }
        if changed {
            self.bump_revision();
        }
        stroke.dual_index = next;
        stroke.dual_last = point;
        Ok(())
    }
    pub(super) fn wet_colored_primary_dab(
        &mut self,
        stroke: &mut Stroke,
        point: InputPoint,
    ) -> Result<bool> {
        let program = stroke.program.clone();
        let dual = program.dual.as_ref().expect("dual color");
        let (speed_size, speed_flow, speed_opacity) = program.factor_all(
            stroke.speed,
            point.tangential_pressure,
            stroke.dab_distance as f32,
            point.tilt_magnitude(),
            point.rotation_unit(),
            BrushProgram::direction_unit(stroke.direction),
            stroke.fade_index,
        );
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
        let limit = program.opacity_pressure(point.pressure).unwrap_or(1.)
            * stroke.taper_factors.opacity
            * speed_opacity;
        let mut changed = false;
        if radius <= 0. {
            stroke
                .wet
                .as_mut()
                .expect("wet pair")
                .carrier
                .reset_path()?;
        } else {
            let geometry = program.dab_geometry(point, stroke.direction, stroke.shape_sample);
            let (ex, ey) = program.extent(radius, geometry);
            let bounds = (
                point.x - ex - 1.,
                point.y - ey - 1.,
                point.x + ex + 1.,
                point.y + ey + 1.,
            );
            let mask = if program.has_mask() {
                program.mask_dab_with_geometry(radius, geometry)
            } else {
                None
            };
            let texture = program.texture_dab(point, geometry);
            let paper = program.texture.as_ref().is_some_and(|t| t.document_space());
            let shape = |x: f64, y: f64| {
                let cover = if let Some(mask) = &mask {
                    mask.coverage(x - point.x, y - point.y)
                } else {
                    program.procedural_coverage_with_geometry(
                        x - point.x,
                        y - point.y,
                        radius,
                        geometry,
                    )
                };
                cover
                    * if paper {
                        1.
                    } else {
                        texture.as_ref().map_or(1., |t| t.coverage(x, y))
                    }
            };
            let wet = self.prepare_wet_carrier(
                stroke,
                &stroke.wet.as_ref().expect("wet pair").carrier,
                geometry.wet_geometry(point, radius),
                |x, y| Ok(shape(x, y)),
            )?;
            let flow = program.tip.flow
                * if program.tip.pressure_flow {
                    program.flow_pressure(point.pressure)
                } else {
                    1.
                }
                * speed_flow;
            let appearance = self.layers[self.layer_index(stroke.layer)?].appearance;
            let offset = (
                i64::from(appearance.offset_x),
                i64::from(appearance.offset_y),
            );
            let color = program.color.active().then(|| {
                program.color.prepare(
                    stroke.brush.background_color,
                    stroke.random_stroke,
                    stroke.random_index.saturating_sub(1),
                    stroke.random_child,
                )
            });
            let sample = |x, y| {
                let (u, v) = geometry.normalized(x - point.x, y - point.y, radius);
                let ink = wet.sample(u.clamp(0., 1.), v.clamp(0., 1.));
                color.map_or(ink, |c| c.apply(ink))
            };
            let touched = self.wet_color_fields(
                &mut stroke.coverage,
                &mut stroke.wet_primary_field,
                bounds,
                offset,
                limit,
                |x, y| shape(x, y) * flow,
                sample,
                (stroke.taper.as_mut(), false),
            )?;
            changed |= self.recompose_color(stroke, touched)?;
            stroke.wet.as_mut().expect("wet pair").carrier.commit(wet)?;
        }
        if dual.step().is_none() {
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
            for child in 0..plan.count() {
                changed |= self.wet_secondary_child(stroke, &plan, child, limit)?;
            }
        }
        stroke.first_dab = false;
        Ok(changed)
    }
}
