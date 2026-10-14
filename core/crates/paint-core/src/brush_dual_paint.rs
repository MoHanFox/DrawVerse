//! Whole-stroke dual-color fields, sharing Document paging and transaction history.
use super::*;
impl Document {
    fn color_field(
        &self,
        field: &mut BTreeMap<TileCoord, Arc<Tile>>,
        bounds: (f64, f64, f64, f64),
        offset: (i64, i64),
        color: Pixel,
        limit: f32,
        sample: impl Fn(f64, f64) -> f32,
    ) -> Result<BTreeSet<TileCoord>> {
        let (ox, oy) = offset;
        let (x0, y0, x1, y1) = (
            (bounds.0.floor() as i64).clamp(0, i64::from(self.width)) - ox,
            (bounds.1.floor() as i64).clamp(0, i64::from(self.height)) - oy,
            (bounds.2.ceil() as i64).clamp(0, i64::from(self.width)) - ox,
            (bounds.3.ceil() as i64).clamp(0, i64::from(self.height)) - oy,
        );
        let mut touched = BTreeSet::new();
        if x0 >= x1 || y0 >= y1 {
            return Ok(touched);
        }
        let rgba = color.components();
        for ty in y0.div_euclid(64)..=(y1 - 1).div_euclid(64) {
            for tx in x0.div_euclid(64)..=(x1 - 1).div_euclid(64) {
                let coord = TileCoord::from_signed(tx as i32, ty as i32);
                let source = field.get(&coord).map(|t| t.try_pixels()).transpose()?;
                let mut updates = Vec::new();
                for y in y0.max(ty * 64)..y1.min((ty + 1) * 64) {
                    for x in x0.max(tx * 64)..x1.min((tx + 1) * 64) {
                        let i = (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize;
                        let before = source.as_ref().map_or(Pixel::TRANSPARENT, |p| p[i]);
                        let previous = before.alpha();
                        if previous >= limit {
                            continue;
                        }
                        let coverage =
                            sample(x as f64 + ox as f64 + 0.5, y as f64 + oy as f64 + 0.5);
                        let accumulated = previous + coverage * (limit - previous).max(0.);
                        if accumulated == previous {
                            continue;
                        }
                        let old = before.components();
                        let delta = accumulated - previous;
                        let values = std::array::from_fn(|j| {
                            if j == 3 {
                                accumulated
                            } else {
                                (old[j]
                                    + if rgba[3] > 0. {
                                        rgba[j] / rgba[3] * delta
                                    } else {
                                        0.
                                    })
                                .min(accumulated)
                            }
                        });
                        updates.push((i, Pixel::from_premultiplied(values)?));
                    }
                }
                if !updates.is_empty() {
                    if !field.contains_key(&coord) && field.len() >= self.options.max_document_tiles
                    {
                        return Err(Error::ResourceLimit(
                            "dual-color field tile limit; entire stroke rolled back",
                        ));
                    }
                    if field.get(&coord).is_some_and(Tile::can_edit_private) {
                        Tile::edit_private(
                            field.get_mut(&coord).expect("color field"),
                            &self.pool,
                            &updates,
                        )?;
                    } else {
                        let mut pixels = source
                            .as_ref()
                            .map_or_else(|| vec![Pixel::TRANSPARENT; 4096], |p| p.to_vec());
                        for (i, value) in updates {
                            pixels[i] = value;
                        }
                        let tile = Arc::new(Tile::from_pixels(pixels)?);
                        self.pool.attach(&tile)?;
                        field.insert(coord, tile);
                    }
                    touched.insert(coord);
                }
                drop(source);
                self.pool.trim()?;
            }
        }
        Ok(touched)
    }
    fn recompose_color(
        &mut self,
        stroke: &mut Stroke,
        touched: BTreeSet<TileCoord>,
    ) -> Result<bool> {
        let index = self.layer_index(stroke.layer)?;
        let layer = &self.layers[index];
        let appearance = layer.appearance;
        let mask = layer.mask;
        let alpha_locked = !mask && appearance.locks & crate::LOCK_TRANSPARENCY != 0;
        let settings = stroke.program.dual.as_ref().expect("dual color").color;
        let secondary = if settings.use_background() {
            stroke.brush.background_color
        } else {
            stroke.brush.color
        };
        let paper = stroke
            .program
            .texture
            .as_ref()
            .filter(|t| t.document_space())
            .map(|t| t.dab(0., 0., 0., 1.));
        let (ox, oy) = (
            i64::from(appearance.offset_x),
            i64::from(appearance.offset_y),
        );
        let mut changed = false;
        for coord in touched {
            let (tx, ty) = (coord.signed_x(), coord.signed_y());
            let current = self.layers[index]
                .tiles
                .get(&coord)
                .map(|t| t.try_pixels())
                .transpose()?;
            let original = if let Some(before) = stroke.before.get(&coord) {
                before.clone()
            } else {
                self.layers[index].tiles.get(&coord).cloned()
            };
            let original_pixels = original.as_ref().map(|t| t.try_pixels()).transpose()?;
            let main = stroke
                .coverage
                .get(&coord)
                .map(|t| t.try_pixels())
                .transpose()?;
            let second = stroke
                .dual_coverage
                .get(&coord)
                .map(|t| t.try_pixels())
                .transpose()?;
            let mut pixels = current.as_ref().map_or_else(
                || {
                    (0..4096)
                        .map(|i| {
                            self.layers[index].default_pixel(
                                i64::from(tx) * 64 + (i % 64) as i64,
                                i64::from(ty) * 64 + (i / 64) as i64,
                            )
                        })
                        .collect::<Vec<_>>()
                },
                |p| p.to_vec(),
            );
            let mut tile_changed = false;
            for row in 0..64 {
                let y = i64::from(ty) * 64 + row;
                let selected = if self.selection.enabled() {
                    self.selection_cache
                        .row(&self.selection, i64::from(tx) * 64 + ox, y + oy, 64)
                } else {
                    [1.; 64]
                };
                for column in 0..64 {
                    let x = i64::from(tx) * 64 + column;
                    let wx = x + ox;
                    let wy = y + oy;
                    if wx < 0
                        || wy < 0
                        || wx >= i64::from(self.width)
                        || wy >= i64::from(self.height)
                    {
                        continue;
                    }
                    let i = (row * 64 + column) as usize;
                    let moment = main.as_ref().map_or(Pixel::TRANSPARENT, |p| p[i]);
                    let amount = second.as_ref().map_or(0., |p| p[i].alpha());
                    let position = (wx as u32, wy as u32);
                    let contribution = settings.compose(
                        moment.scaled(stroke.brush.color.alpha()),
                        moment.alpha(),
                        secondary,
                        amount,
                        position,
                        stroke.random_stroke,
                    );
                    let weight = stroke.brush.opacity
                        * selected[column as usize]
                        * paper
                            .as_ref()
                            .map_or(1., |p| p.coverage(wx as f64 + 0.5, wy as f64 + 0.5));
                    let base = original_pixels
                        .as_ref()
                        .map_or(self.layers[index].default_pixel(x, y), |p| p[i]);
                    let after = if mask {
                        let (tone, a) = Self::mask_tone(contribution);
                        let weight = weight * a;
                        let value = if stroke.program.write_blend == crate::BlendMode::Normal {
                            base.alpha() * (1. - weight) + tone * weight
                        } else {
                            1. - crate::blend_pixel(
                                Pixel::from_straight([1. - tone, 1. - tone, 1. - tone, weight])?,
                                Pixel::from_straight([
                                    1. - base.alpha(),
                                    1. - base.alpha(),
                                    1. - base.alpha(),
                                    1.,
                                ])?,
                                stroke.program.write_blend,
                                position.0,
                                position.1,
                                stroke.random_stroke,
                            )
                            .components()[0]
                        };
                        Pixel::from_straight([0., 0., 0., value])?
                    } else if alpha_locked {
                        base.paint_blended_preserving_alpha(
                            contribution,
                            weight,
                            stroke.program.write_blend,
                            position,
                            stroke.random_stroke,
                        )?
                    } else {
                        crate::blend_pixel(
                            contribution.scaled(weight),
                            base,
                            stroke.program.write_blend,
                            position.0,
                            position.1,
                            stroke.random_stroke,
                        )
                    };
                    if pixels[i] != after {
                        pixels[i] = after;
                        tile_changed = true;
                    }
                }
            }
            if tile_changed {
                if !self.layers[index].tiles.contains_key(&coord)
                    && self.tile_count() >= self.options.max_document_tiles
                {
                    return Err(Error::ResourceLimit(
                        "dual-color document tile limit; entire stroke rolled back",
                    ));
                }
                stroke.before.entry(coord).or_insert(original);
                let tile = Arc::new(Tile::from_pixels(pixels)?);
                self.pool.attach(&tile)?;
                self.layers[index].tiles.insert(coord, tile);
                if ox == 0 && oy == 0 {
                    self.dirty_tile(coord);
                } else {
                    self.dirty.all = true;
                    self.dirty.tiles.clear();
                }
                changed = true;
            }
            drop(current);
            drop(original_pixels);
            drop(main);
            drop(second);
            self.pool.trim()?;
        }
        Ok(changed)
    }
    pub(super) fn colored_secondary_dab(
        &mut self,
        stroke: &mut Stroke,
        point: InputPoint,
    ) -> Result<()> {
        let program = stroke.program.clone();
        let dual = program.dual.as_ref().expect("dual color");
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
        );
        let plan = dual.dab(
            point,
            stroke.direction,
            stroke.random_stroke,
            stroke.dual_index,
            0,
            radius,
        );
        let appearance = self.layers[self.layer_index(stroke.layer)?].appearance;
        let offset = (
            i64::from(appearance.offset_x),
            i64::from(appearance.offset_y),
        );
        let limit = program.opacity_pressure(point.pressure).unwrap_or(1.);
        let mut touched = BTreeSet::new();
        for child in 0..plan.count() {
            touched.extend(self.color_field(
                &mut stroke.dual_coverage,
                plan.bounds(child),
                offset,
                Pixel::WHITE,
                limit,
                |x, y| plan.coverage(child, x, y),
            )?);
        }
        if self.recompose_color(stroke, touched)? {
            self.bump_revision();
        }
        stroke.dual_index = next;
        stroke.dual_last = point;
        Ok(())
    }
    pub(super) fn colored_primary_dab(
        &mut self,
        stroke: &mut Stroke,
        point: InputPoint,
    ) -> Result<bool> {
        let program = stroke.program.clone();
        let dual = program.dual.as_ref().expect("dual color");
        let radius = f64::from(
            stroke.brush.radius
                * stroke.shape_sample.size
                * if program.tip.pressure_size {
                    program.size_pressure(point.pressure)
                } else {
                    1.
                },
        );
        let geometry = program.dab_geometry(point, stroke.direction, stroke.shape_sample);
        let (ex, ey) = program.extent(radius, geometry);
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
        let limit = program.opacity_pressure(point.pressure).unwrap_or(1.);
        let appearance = self.layers[self.layer_index(stroke.layer)?].appearance;
        let offset = (
            i64::from(appearance.offset_x),
            i64::from(appearance.offset_y),
        );
        let mut touched = self.color_field(
            &mut stroke.coverage,
            (
                point.x - ex - 1.,
                point.y - ey - 1.,
                point.x + ex + 1.,
                point.y + ey + 1.,
            ),
            offset,
            stroke.color_sample,
            limit,
            |x, y| {
                let cover = if radius <= 0. {
                    0.
                } else if let Some(mask) = &mask {
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
                    * flow
                    * if paper {
                        1.
                    } else {
                        texture.as_ref().map_or(1., |t| t.coverage(x, y))
                    }
            },
        )?;
        if dual.step().is_none() {
            let plan = dual.dab(
                point,
                stroke.direction,
                stroke.random_stroke,
                stroke.random_index.saturating_sub(1),
                stroke.random_child,
                radius,
            );
            for child in 0..plan.count() {
                touched.extend(self.color_field(
                    &mut stroke.dual_coverage,
                    plan.bounds(child),
                    offset,
                    Pixel::WHITE,
                    limit,
                    |x, y| plan.coverage(child, x, y),
                )?);
            }
        }
        stroke.first_dab = false;
        self.recompose_color(stroke, touched)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_storage_failure_rolls_back_the_whole_active_stroke() {
        let mut d = Document::with_options(
            192,
            64,
            DocumentOptions {
                max_resident_tiles: 1,
                ..Default::default()
            },
        )
        .unwrap();
        let tip = crate::BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        };
        let color = crate::DualColor::new(
            true,
            true,
            true,
            crate::DualColorOrder::SecondBehind,
            crate::BlendMode::Normal,
            crate::BlendMode::Normal,
        )
        .unwrap();
        let b = BrushDefinition {
            brush: Brush {
                radius: 4.,
                spacing: 0.5,
                ..Default::default()
            },
            tip,
            dual: crate::DualBrush::new(
                true,
                crate::DualMode::Multiply,
                crate::SecondaryTip::new(8., tip, None, Default::default()).unwrap(),
            )
            .with_color(color),
            ..Default::default()
        };
        d.begin_brush_stroke(b.clone(), InputPoint::new(32.5, 32.5, 1.))
            .unwrap();
        crate::page_pool::fail_writes_for_test(true);
        let result = d.stroke_to(InputPoint::new(150.5, 32.5, 1.));
        crate::page_pool::fail_writes_for_test(false);
        assert!(matches!(result, Err(Error::Storage(_))));
        assert!(!d.stroke_active());
        assert_eq!(d.history_depth(), (0, 0));
        assert_eq!(d.tile_count(), 0);
        d.begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.))
            .unwrap();
        d.end_stroke().unwrap();
        assert_eq!(d.history_depth(), (1, 0));
    }
}
