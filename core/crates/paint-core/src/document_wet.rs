use super::*;
use crate::brush_program::TipGeometry;
use crate::{WetCarrier, WetDab, WetSample};
#[derive(Clone, Debug)]
pub(super) struct WetStroke {
    pub(super) carrier: WetCarrier,
    pub(super) secondary: Option<WetCarrier>,
    pub(super) all_layers: bool,
    pub(super) work_per_dab: usize,
}
impl Document {
    pub fn begin_configured_dual_wet_brush_stroke(
        &mut self,
        definition: BrushDefinition,
        first: InputPoint,
        retained: Option<&crate::WetInk>,
    ) -> Result<()> {
        self.idle()?;
        if !definition.wet.enabled()
            || !definition.dual.enabled()
            || !definition.dual.color().enabled()
            || definition.brush.mode != BrushMode::Paint
        {
            return Err(Error::InvalidArgument("dual wet paint definition required"));
        }
        let second = if definition.dual.color().use_background() {
            definition.brush.background_color
        } else {
            definition.brush.color
        };
        let diameter = definition.dual.sizing().nominal_radius(
            definition.brush.radius,
            definition.dual.secondary().radius(),
        ) * 2.;
        let ink = crate::WetInk::for_stroke(
            definition.wet,
            definition.brush.radius * 2.,
            definition.brush.color,
            Some((diameter, second)),
            retained,
        )?;
        let all_layers = definition.wet.all_layers();
        self.begin_brush_stroke_inner(
            definition,
            first,
            Some(WetStroke {
                carrier: ink.primary,
                secondary: ink.secondary,
                all_layers,
                work_per_dab: 0,
            }),
            None,
        )
    }
    pub fn end_wet_ink_stroke(&mut self) -> Result<(bool, crate::WetInk)> {
        if self.stroke.as_ref().ok_or(Error::NoStroke)?.wet.is_none() {
            return Err(Error::InvalidArgument("not a wet stroke"));
        }
        let (changed, ink) = self.finish_stroke()?;
        Ok((changed, ink.expect("wet stroke")))
    }
    pub fn begin_wet_brush_stroke(
        &mut self,
        definition: BrushDefinition,
        first: InputPoint,
        carrier: &WetCarrier,
        all_layers: bool,
    ) -> Result<()> {
        if definition.wet.enabled() {
            return Err(Error::InvalidArgument("use configured wet stroke entry"));
        }
        self.begin_brush_stroke_inner(
            definition,
            first,
            Some(WetStroke {
                carrier: carrier.clone(),
                secondary: None,
                all_layers,
                work_per_dab: 0,
            }),
            None,
        )
    }
    /// Capture preset policy and start with a private, optionally retained reservoir.
    pub fn begin_configured_wet_brush_stroke(
        &mut self,
        definition: BrushDefinition,
        first: InputPoint,
        retained: Option<&WetCarrier>,
    ) -> Result<()> {
        self.begin_configured_wet_with_taper(definition, first, retained, None)
    }
    pub fn begin_configured_tapered_wet_brush_stroke(
        &mut self,
        definition: BrushDefinition,
        first: InputPoint,
        retained: Option<&WetCarrier>,
        taper: &crate::StrokeTaper,
    ) -> Result<()> {
        self.begin_configured_wet_with_taper(definition, first, retained, Some(taper))
    }
    fn begin_configured_wet_with_taper(
        &mut self,
        definition: BrushDefinition,
        first: InputPoint,
        retained: Option<&WetCarrier>,
        taper: Option<&crate::StrokeTaper>,
    ) -> Result<()> {
        self.idle()?;
        let policy = definition.wet;
        if !policy.enabled() {
            return Err(Error::InvalidArgument("wet preset is disabled"));
        }
        let mut carrier = if let Some(previous) = retained {
            previous.reconfigured(policy.mix(), definition.brush.radius * 2.)?
        } else {
            WetCarrier::new(
                policy.mix(),
                definition.brush.radius * 2.,
                Pixel::TRANSPARENT,
            )?
        };
        if policy.auto_clean() {
            carrier.clean()?;
        }
        if policy.auto_load()
            && !matches!(
                definition.brush.mode,
                BrushMode::Smudge {
                    finger_painting: false,
                    ..
                }
            )
        {
            carrier.reload(definition.brush.color)?;
        }
        let all_layers = match definition.brush.mode {
            BrushMode::Smudge { all_layers, .. } => all_layers,
            _ => policy.all_layers(),
        };
        self.begin_brush_stroke_inner(
            definition,
            first,
            Some(WetStroke {
                carrier,
                secondary: None,
                all_layers,
                work_per_dab: 0,
            }),
            taper,
        )
    }
    pub fn end_wet_brush_stroke(&mut self) -> Result<(bool, WetCarrier)> {
        if self
            .stroke
            .as_ref()
            .ok_or(Error::NoStroke)?
            .wet
            .as_ref()
            .is_some_and(|w| w.secondary.is_some())
        {
            return Err(Error::InvalidArgument("use full wet ink end"));
        }
        if self.stroke.as_ref().ok_or(Error::NoStroke)?.wet.is_none() {
            return Err(Error::InvalidArgument("not a wet stroke"));
        }
        let (changed, wet) = self.finish_stroke()?;
        Ok((changed, wet.expect("validated wet stroke").primary))
    }
    pub(super) fn validate_wet_start(
        &self,
        program: &BrushProgram,
        brush: Brush,
        first: InputPoint,
        wet: &mut WetStroke,
    ) -> Result<()> {
        if brush.mode == BrushMode::Erase
            || first.tool == Tool::Eraser
            || (program.dual.as_ref().is_some_and(|d| d.color.enabled()) && wet.secondary.is_none())
        {
            return Err(Error::InvalidArgument("unsupported wet dual color or tool"));
        }
        if wet.all_layers && self.layer(self.active_layer).is_some_and(|l| l.mask) {
            return Err(Error::InvalidArgument("all-layer wet sampling on mask"));
        }
        let setting = wet.carrier.settings();
        let reads = setting.wetness() > 0. && (setting.pickup() > 0. || setting.mix() > 0.);
        let layers = if wet.all_layers {
            self.layers.len().saturating_mul(16).max(1)
        } else {
            1
        };
        let sample = if reads {
            layers
                .saturating_mul(4)
                .saturating_add(if program.has_mask() { 8 } else { 1 })
                .saturating_add(if program.has_texture() { 8 } else { 0 })
                .saturating_add(if program.dual.is_some() { 8 } else { 0 })
        } else {
            0
        };
        wet.work_per_dab = wet
            .carrier
            .edge()
            .saturating_pow(2)
            .saturating_mul(6usize.saturating_add(sample))
            .saturating_add(if program.color.active() { 64 } else { 0 });
        if let Some(secondary) = &wet.secondary {
            let dual = program.dual.as_ref().expect("wet pair");
            wet.work_per_dab = wet
                .work_per_dab
                .saturating_add(
                    secondary
                        .edge()
                        .saturating_pow(2)
                        .saturating_mul(6usize.saturating_add(sample)),
                )
                .saturating_add(
                    dual.colored_work(1, crate::BlendMode::Normal)
                        .saturating_mul(8),
                )
                .saturating_add(
                    ((brush.radius * 2. + 3.).ceil() as usize)
                        .saturating_pow(2)
                        .saturating_mul(32),
                );
        }
        Ok(())
    }
    pub(super) fn prepare_wet<F>(
        &self,
        stroke: &Stroke,
        point: InputPoint,
        radius: f64,
        geometry: TipGeometry,
        coverage: F,
    ) -> Result<Option<WetDab>>
    where
        F: FnMut(f64, f64) -> Result<f32>,
    {
        let Some(wet) = &stroke.wet else {
            return Ok(None);
        };
        self.prepare_wet_carrier(
            stroke,
            &wet.carrier,
            geometry.wet_geometry(point, radius),
            coverage,
        )
        .map(Some)
    }
    pub(super) fn prepare_wet_carrier<F>(
        &self,
        stroke: &Stroke,
        carrier: &WetCarrier,
        geometry: crate::WetDabGeometry,
        mut coverage: F,
    ) -> Result<WetDab>
    where
        F: FnMut(f64, f64) -> Result<f32>,
    {
        let wet = stroke.wet.as_ref().expect("wet stroke");
        carrier.prepare_dab(geometry, |x, y| {
            let weight = coverage(x, y)?;
            if weight == 0. {
                return Ok(WetSample {
                    color: Pixel::TRANSPARENT,
                    pickup_weight: 0.,
                });
            }
            let sx = x - 0.5;
            let sy = y - 0.5;
            let ix = sx.floor() as i64;
            let iy = sy.floor() as i64;
            let fx = (sx - ix as f64) as f32;
            let fy = (sy - iy as f64) as f32;
            let sample = |x: i64, y: i64| -> Result<Pixel> {
                if x < 0 || y < 0 || x >= i64::from(self.width) || y >= i64::from(self.height) {
                    return Ok(Pixel::TRANSPARENT);
                }
                if wet.all_layers {
                    self.try_pixel(x as u32, y as u32)
                } else {
                    let layer = self
                        .layer(stroke.layer)
                        .ok_or(Error::LayerNotFound(stroke.layer))?;
                    let value = layer.canvas_pixel(x as u32, y as u32)?;
                    if layer.mask {
                        let visible = 1. - value.alpha();
                        // Mask coverage is display gray. The wet smudge carrier
                        // stores linear color, matching foreground/manual loads.
                        let visible = if matches!(stroke.brush.mode, BrushMode::Smudge { .. }) {
                            if visible <= 0.04045 {
                                visible / 12.92
                            } else {
                                ((visible + 0.055) / 1.055).powf(2.4)
                            }
                        } else {
                            visible
                        };
                        Pixel::from_straight([visible, visible, visible, 1.])
                    } else {
                        Ok(value)
                    }
                }
            };
            let top = if fx == 0. {
                sample(ix, iy)?
            } else {
                sample(ix, iy)?.mix(sample(ix + 1, iy)?, fx)
            };
            let color = if fy == 0. {
                top
            } else {
                let bottom = if fx == 0. {
                    sample(ix, iy + 1)?
                } else {
                    sample(ix, iy + 1)?.mix(sample(ix + 1, iy + 1)?, fx)
                };
                top.mix(bottom, fy)
            };
            Ok(WetSample {
                color,
                pickup_weight: weight,
            })
        })
    }
    pub(super) fn commit_wet(stroke: &mut Stroke, dab: Option<WetDab>) -> Result<()> {
        if let Some(dab) = dab {
            stroke
                .wet
                .as_mut()
                .expect("prepared wet stroke")
                .carrier
                .commit(dab)?;
        }
        Ok(())
    }
    pub(super) fn wet_secondary_coverage(
        stroke: &Stroke,
        x: f64,
        y: f64,
        appearance: crate::LayerAppearance,
    ) -> Result<f32> {
        let x = x.floor() as i64 - i64::from(appearance.offset_x);
        let y = y.floor() as i64 - i64::from(appearance.offset_y);
        let coord = TileCoord::from_signed(x.div_euclid(64) as i32, y.div_euclid(64) as i32);
        stroke.dual_coverage.get(&coord).map_or(Ok(0.), |t| {
            Ok(t.try_pixels()?[(y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize].alpha())
        })
    }
    pub(super) fn wet_contribution(
        before: Pixel,
        ink: Pixel,
        coverage: f32,
        ceiling: f32,
    ) -> Result<Pixel> {
        let old = before.components();
        let source = ink.components();
        let cap = ceiling.max(old[3]);
        let q = coverage * source[3];
        let alpha = (old[3] + q * (cap - old[3])).clamp(0., 1.);
        Pixel::from_premultiplied([
            (old[0] * (1. - q) + source[0] * coverage * cap).clamp(0., alpha),
            (old[1] * (1. - q) + source[1] * coverage * cap).clamp(0., alpha),
            (old[2] * (1. - q) + source[2] * coverage * cap).clamp(0., alpha),
            alpha,
        ])
    }
}

#[cfg(test)]
#[path = "document_wet_dual_tests.rs"]
mod dual_tests;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filtered_stationary_wet_pressure_dabs_share_one_input_budget() {
        let mut doc = Document::new(192, 128).unwrap();
        let definition = BrushDefinition {
            brush: Brush {
                radius: 256.,
                opacity: 0.4,
                ..Default::default()
            },
            tip: crate::BrushTip {
                pressure_size: true,
                pressure_flow: false,
                ..Default::default()
            },
            smoothing: crate::BrushSmoothing::new(true, 0., 8., 0., 0.).unwrap(),
            ..Default::default()
        };
        let carrier = WetCarrier::new(
            crate::WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
            512.,
            definition.brush.color,
        )
        .unwrap();
        doc.begin_wet_brush_stroke(definition, InputPoint::new(96., 64., 0.2), &carrier, false)
            .unwrap();
        let mut point = InputPoint::new(96., 64., 1.);
        point.timestamp_ns = 24_000_000;
        assert!(matches!(doc.stroke_to(point), Err(Error::ResourceLimit(_))));
        assert!(!doc.stroke_active());
        assert_eq!(doc.history_depth(), (0, 0));
        assert_eq!(carrier.revision(), 0);
        for y in 0..128 {
            for x in 0..192 {
                assert_eq!(doc.pixel(x, y), Pixel::TRANSPARENT);
            }
        }
    }
    #[test]
    fn scratch_failure_rolls_back_prior_wet_dabs_and_does_not_adopt_carrier() {
        for colored in [false, true] {
            let mut doc = Document::with_options(
                256,
                64,
                DocumentOptions {
                    max_resident_tiles: 1,
                    ..Default::default()
                },
            )
            .unwrap();
            let mut definition = BrushDefinition {
                brush: Brush {
                    radius: 4.,
                    spacing: 0.5,
                    ..Default::default()
                },
                tip: crate::BrushTip {
                    pressure_size: false,
                    pressure_flow: false,
                    ..Default::default()
                },
                ..Default::default()
            };
            let carrier = WetCarrier::new(
                crate::WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
                8.,
                definition.brush.color,
            )
            .unwrap();
            if colored {
                definition.dynamics.color =
                    crate::ColorDynamics::new(true, 0.4, 0.6, 0.2, 0.2, true, u64::MAX).unwrap();
            }
            doc.begin_wet_brush_stroke(definition, InputPoint::new(16., 32., 1.), &carrier, false)
                .unwrap();
            assert_ne!(doc.pixel(16, 32), Pixel::TRANSPARENT);
            crate::page_pool::fail_writes_for_test(true);
            let result = doc.stroke_to(InputPoint::new(192., 32., 1.));
            crate::page_pool::fail_writes_for_test(false);
            assert!(result.is_err());
            assert!(!doc.stroke_active());
            assert_eq!(doc.history_depth(), (0, 0));
            assert_eq!(carrier.revision(), 0);
            for y in 0..64 {
                for x in 0..256 {
                    assert_eq!(doc.try_pixel(x, y).unwrap(), Pixel::TRANSPARENT);
                }
            }
        }
    }
}
