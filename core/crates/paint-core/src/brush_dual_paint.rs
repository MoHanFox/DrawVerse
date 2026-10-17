//! Whole-stroke dual-color fields, sharing Document paging and transaction history.
use super::*;
#[path = "brush_wet_dual_paint.rs"]
mod wet_dual;
#[path = "brush_wet_fields.rs"]
mod wet_fields;
#[derive(Default)]
struct ChangedPixels(BTreeMap<TileCoord, [u64; 64]>);
impl ChangedPixels {
    fn new() -> Self {
        Self::default()
    }
    fn extend(&mut self, other: Self) {
        for (coord, rows) in other.0 {
            let entry = self.0.entry(coord).or_insert([0; 64]);
            for (dst, src) in entry.iter_mut().zip(rows) {
                *dst |= src;
            }
        }
    }
    fn insert(&mut self, coord: TileCoord, updates: &[(usize, Pixel)]) {
        let rows = self.0.entry(coord).or_insert([0; 64]);
        for (i, _) in updates {
            rows[i / 64] |= 1_u64 << (i % 64);
        }
    }
}

impl Document {
    #[allow(clippy::too_many_arguments)] // Optional checkpoint journals the same field write.
    fn color_field(
        &self,
        field: &mut BTreeMap<TileCoord, Arc<Tile>>,
        bounds: (f64, f64, f64, f64),
        offset: (i64, i64),
        color: Pixel,
        limit: f32,
        sample: impl Fn(f64, f64) -> f32,
        mut checkpoint: (Option<&mut taper::TailReplay>, bool),
        ink: Option<&dyn Fn(f64, f64) -> Pixel>,
    ) -> Result<ChangedPixels> {
        let (ox, oy) = offset;
        let (x0, y0, x1, y1) = (
            (bounds.0.floor() as i64).clamp(0, i64::from(self.width)) - ox,
            (bounds.1.floor() as i64).clamp(0, i64::from(self.height)) - oy,
            (bounds.2.ceil() as i64).clamp(0, i64::from(self.width)) - ox,
            (bounds.3.ceil() as i64).clamp(0, i64::from(self.height)) - oy,
        );
        let mut touched = ChangedPixels::new();
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
                        if let Some(ink) = ink {
                            let wx = x as f64 + ox as f64 + 0.5;
                            let wy = y as f64 + oy as f64 + 0.5;
                            let after =
                                Self::wet_contribution(before, ink(wx, wy), sample(wx, wy), limit)?;
                            if after != before {
                                updates.push((i, after));
                            }
                            continue;
                        }
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
                    (if ink.is_some() {
                        taper::capture_wet_field
                    } else {
                        taper::capture_field
                    })(
                        checkpoint.0.as_deref_mut(),
                        checkpoint.1,
                        coord,
                        field,
                        self.options.max_document_tiles,
                    )?;
                    if !field.contains_key(&coord) && field.len() >= self.options.max_document_tiles
                    {
                        return Err(Error::ResourceLimit(
                            "dual-color field tile limit; entire stroke rolled back",
                        ));
                    }
                    touched.insert(coord, &updates);
                    if field.get(&coord).is_some_and(Tile::can_edit_private) {
                        drop(source);
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
                        drop(source);
                        let tile = Arc::new(Tile::from_pixels(pixels)?);
                        self.pool.attach(&tile)?;
                        field.insert(coord, tile);
                    }
                } else {
                    drop(source);
                }
                self.pool.trim()?;
            }
        }
        Ok(touched)
    }
    fn recompose_color(&mut self, stroke: &mut Stroke, touched: ChangedPixels) -> Result<bool> {
        let wet_pair = stroke.wet.as_ref().is_some_and(|w| w.secondary.is_some());
        let index = self.layer_index(stroke.layer)?;
        let layer = &self.layers[index];
        let appearance = layer.appearance;
        let mask = layer.mask;
        let alpha_locked = !mask && appearance.locks & crate::LOCK_TRANSPARENCY != 0;
        let settings = stroke.program.dual.as_ref().expect("dual color").color;
        let interleaved = settings.model() == crate::DualColorModel::InterleavedStamp;
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
            .map(|t| t.dab(0., 0., 1., 0., 1.));
        let (ox, oy) = (
            i64::from(appearance.offset_x),
            i64::from(appearance.offset_y),
        );
        let mut changed = false;
        for (coord, rows) in touched.0 {
            self.capture_taper_tile(
                stroke.layer,
                &mut stroke.taper,
                &stroke.coverage,
                &stroke.dual_coverage,
                &stroke.stamp_field,
                coord,
            )?;
            #[cfg(test)]
            let rows = if self.reference_color_tiles {
                [u64::MAX; 64]
            } else {
                rows
            };
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
            let stamp = if interleaved {
                stroke
                    .stamp_field
                    .get(&coord)
                    .map(|t| t.try_pixels())
                    .transpose()?
            } else {
                None
            };
            let wet_main = if wet_pair {
                stroke
                    .wet_primary_field
                    .get(&coord)
                    .map(|t| t.try_pixels())
                    .transpose()?
            } else {
                None
            };
            let wet_second = if wet_pair {
                stroke
                    .wet_secondary_field
                    .get(&coord)
                    .map(|t| t.try_pixels())
                    .transpose()?
            } else {
                None
            };
            let private = stroke.before.contains_key(&coord)
                && self.layers[index]
                    .tiles
                    .get(&coord)
                    .is_some_and(Tile::can_edit_private);
            #[cfg(test)]
            let private = private && !self.reference_color_tiles;
            let mut updates = Vec::new();
            let mut pixels: Option<Vec<Pixel>> = None;
            for row in 0..64 {
                let mut bits = rows[row as usize];
                if bits == 0 {
                    continue;
                }
                let y = i64::from(ty) * 64 + row;
                let selected = if self.selection.enabled() {
                    self.selection_cache
                        .row(&self.selection, i64::from(tx) * 64 + ox, y + oy, 64)
                } else {
                    [1.; 64]
                };
                while bits != 0 {
                    let column = i64::from(bits.trailing_zeros());
                    bits &= bits - 1;
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
                    let contribution = if interleaved {
                        stamp.as_ref().map_or(Pixel::TRANSPARENT, |p| p[i])
                    } else if wet_pair {
                        settings.compose_wet(
                            wet_main.as_ref().map_or(Pixel::TRANSPARENT, |p| p[i]),
                            moment.alpha(),
                            wet_second.as_ref().map_or(Pixel::TRANSPARENT, |p| p[i]),
                            amount,
                            position,
                            stroke.random_stroke,
                        )
                    } else {
                        settings.compose(
                            moment.scaled(stroke.brush.color.alpha()),
                            moment.alpha(),
                            secondary,
                            amount,
                            position,
                            stroke.random_stroke,
                        )
                    };
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
                    let before = current
                        .as_ref()
                        .map_or(self.layers[index].default_pixel(x, y), |p| p[i]);
                    if before != after {
                        if private {
                            updates.push((i, after));
                        } else {
                            pixels.get_or_insert_with(|| {
                                current.as_ref().map_or_else(
                                    || {
                                        (0..4096)
                                            .map(|j| {
                                                self.layers[index].default_pixel(
                                                    i64::from(tx) * 64 + (j % 64) as i64,
                                                    i64::from(ty) * 64 + (j / 64) as i64,
                                                )
                                            })
                                            .collect()
                                    },
                                    |p| p.to_vec(),
                                )
                            })[i] = after;
                        }
                    }
                }
            }
            drop(current);
            let tile_changed = !updates.is_empty() || pixels.is_some();
            if !updates.is_empty() {
                Tile::edit_private(
                    self.layers[index]
                        .tiles
                        .get_mut(&coord)
                        .expect("private color tile"),
                    &self.pool,
                    &updates,
                )?;
            }
            if let Some(pixels) = pixels {
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
            }
            if tile_changed {
                if ox == 0 && oy == 0 {
                    self.dirty_tile(coord);
                } else {
                    self.dirty.all = true;
                    self.dirty.tiles.clear();
                }
                changed = true;
            }
            drop(original_pixels);
            drop(main);
            drop(second);
            drop(stamp);
            drop(wet_main);
            drop(wet_second);
            self.pool.trim()?;
        }
        Ok(changed)
    }
    pub(super) fn colored_secondary_dab(
        &mut self,
        stroke: &mut Stroke,
        point: InputPoint,
    ) -> Result<()> {
        if stroke.wet.as_ref().is_some_and(|w| w.secondary.is_some()) {
            return self.wet_colored_secondary_dab(stroke, point);
        }
        let program = stroke.program.clone();
        let dual = program.dual.as_ref().expect("dual color");
        let (speed_size, _, speed_opacity) = program.speed_factors(stroke.speed);
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
        let appearance = self.layers[self.layer_index(stroke.layer)?].appearance;
        let offset = (
            i64::from(appearance.offset_x),
            i64::from(appearance.offset_y),
        );
        let limit = program.opacity_pressure(point.pressure).unwrap_or(1.)
            * stroke.taper_factors.opacity
            * speed_opacity;
        let mut touched = ChangedPixels::new();
        for child in 0..plan.count() {
            touched.extend(self.color_field(
                &mut stroke.dual_coverage,
                plan.bounds(child),
                offset,
                Pixel::WHITE,
                limit,
                |x, y| plan.coverage(child, x, y),
                (stroke.taper.as_mut(), true),
                None,
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
        if stroke.wet.as_ref().is_some_and(|w| w.secondary.is_some()) {
            return self.wet_colored_primary_dab(stroke, point);
        }
        let program = stroke.program.clone();
        let dual = program.dual.as_ref().expect("dual color");
        let settings = dual.color;
        let (speed_size, speed_flow, speed_opacity) = program.speed_factors(stroke.speed);
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
            }
            * speed_flow;
        let limit = program.opacity_pressure(point.pressure).unwrap_or(1.)
            * stroke.taper_factors.opacity
            * speed_opacity;
        if settings.model() == crate::DualColorModel::InterleavedStamp {
            return self.interleaved_colored_dab(
                stroke,
                point,
                base_radius,
                radius,
                limit,
                speed_size,
            );
        }
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
            (stroke.taper.as_mut(), false),
            None,
        )?;
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
                touched.extend(self.color_field(
                    &mut stroke.dual_coverage,
                    plan.bounds(child),
                    offset,
                    Pixel::WHITE,
                    limit,
                    |x, y| plan.coverage(child, x, y),
                    (stroke.taper.as_mut(), true),
                    None,
                )?);
            }
        }
        stroke.first_dab = false;
        self.recompose_color(stroke, touched)
    }

    #[allow(clippy::too_many_arguments)]
    fn interleaved_colored_dab(
        &mut self,
        stroke: &mut Stroke,
        point: InputPoint,
        base_radius: f64,
        radius: f64,
        limit: f32,
        speed_size: f32,
    ) -> Result<bool> {
        let program = stroke.program.clone();
        let dual = program.dual.as_ref().expect("dual color");
        debug_assert_eq!(dual.color.model(), crate::DualColorModel::InterleavedStamp);
        let geometry = program.dab_geometry(point, stroke.direction, stroke.shape_sample);
        let (extent_x, extent_y) = program.extent(radius, geometry);
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
        let appearance = self.layers[self.layer_index(stroke.layer)?].appearance;
        let (ox, oy) = (
            f64::from(appearance.offset_x),
            f64::from(appearance.offset_y),
        );
        let mut left = point.x - extent_x - 1.;
        let mut top = point.y - extent_y - 1.;
        let mut right = point.x + extent_x + 1.;
        let mut bottom = point.y + extent_y + 1.;
        for child in 0..plan.count() {
            let (l, t, r, b) = plan.bounds(child);
            left = left.min(l);
            top = top.min(t);
            right = right.max(r);
            bottom = bottom.max(b);
        }
        let x0 = (left.floor() as i64).clamp(0, i64::from(self.width)) - appearance.offset_x as i64;
        let x1 = (right.ceil() as i64).clamp(0, i64::from(self.width)) - appearance.offset_x as i64;
        let y0 = (top.floor() as i64).clamp(0, i64::from(self.height)) - appearance.offset_y as i64;
        let y1 =
            (bottom.ceil() as i64).clamp(0, i64::from(self.height)) - appearance.offset_y as i64;
        let mut touched = ChangedPixels::new();
        if x0 < x1 && y0 < y1 && limit > 0. {
            let secondary_color = if dual.color.use_background() {
                stroke.brush.background_color
            } else {
                stroke.brush.color
            };
            for ty in y0.div_euclid(64)..=(y1 - 1).div_euclid(64) {
                for tx in x0.div_euclid(64)..=(x1 - 1).div_euclid(64) {
                    let coord = TileCoord::from_signed(tx as i32, ty as i32);
                    self.capture_taper_tile(
                        stroke.layer,
                        &mut stroke.taper,
                        &stroke.coverage,
                        &stroke.dual_coverage,
                        &stroke.stamp_field,
                        coord,
                    )?;
                    let source = stroke
                        .stamp_field
                        .get(&coord)
                        .map(|t| t.try_pixels())
                        .transpose()?;
                    let mut updates = Vec::new();
                    for y in y0.max(ty * 64)..y1.min((ty + 1) * 64) {
                        for x in x0.max(tx * 64)..x1.min((tx + 1) * 64) {
                            let wx = x as f64 + ox + 0.5;
                            let wy = y as f64 + oy + 0.5;
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
                            if primary == 0. && second == 0. {
                                continue;
                            }
                            let pair = dual.color.compose(
                                stroke.color_sample.scaled(primary),
                                primary,
                                secondary_color,
                                second,
                                (wx as u32, wy as u32),
                                stroke.random_stroke,
                            );
                            let offset = (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize;
                            let previous = source
                                .as_ref()
                                .map_or(Pixel::TRANSPARENT, |pixels| pixels[offset]);
                            let after = pair.over(previous);
                            if after != previous {
                                updates.push((offset, after));
                            }
                        }
                    }
                    if !updates.is_empty() {
                        if stroke
                            .stamp_field
                            .get(&coord)
                            .is_some_and(Tile::can_edit_private)
                        {
                            Tile::edit_private(
                                stroke.stamp_field.get_mut(&coord).expect("stamp field"),
                                &self.pool,
                                &updates,
                            )?;
                        } else {
                            if !stroke.stamp_field.contains_key(&coord)
                                && stroke.stamp_field.len() >= self.options.max_document_tiles
                            {
                                return Err(Error::ResourceLimit(
                                    "interleaved dual-color field tile limit; entire stroke rolled back",
                                ));
                            }
                            let mut pixels = source.as_ref().map_or_else(
                                || vec![Pixel::TRANSPARENT; 4096],
                                |pixels| pixels.to_vec(),
                            );
                            for (offset, value) in &updates {
                                pixels[*offset] = *value;
                            }
                            let tile = Arc::new(Tile::from_pixels(pixels)?);
                            self.pool.attach(&tile)?;
                            stroke.stamp_field.insert(coord, tile);
                        }
                        touched.insert(coord, &updates);
                    }
                    drop(source);
                    self.pool.trim()?;
                }
            }
        }
        stroke.first_dab = false;
        self.recompose_color(stroke, touched)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definition(
        path: bool,
        front: bool,
        overflow: bool,
        mode: crate::BlendMode,
    ) -> BrushDefinition {
        let tip = crate::BrushTip {
            hardness: 0.4,
            flow: 0.4,
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        };
        let second = crate::SecondaryTip::new(6., tip, None, Default::default()).unwrap();
        let second = if path {
            second.with_spacing(0.25).unwrap()
        } else {
            second
        };
        let color = crate::DualColor::new(
            true,
            true,
            overflow,
            if front {
                crate::DualColorOrder::SecondInFront
            } else {
                crate::DualColorOrder::SecondBehind
            },
            mode,
            crate::BlendMode::Screen,
        )
        .unwrap();
        BrushDefinition {
            brush: Brush {
                radius: 4.,
                spacing: 0.5,
                opacity: 0.4,
                color: Pixel::from_straight([0.8, 0.2, 0.1, 0.7]).unwrap(),
                background_color: Pixel::from_straight([0.1, 0.3, 0.8, 0.8]).unwrap(),
                ..Default::default()
            },
            tip,
            dynamics: crate::BrushDynamics {
                pressure_opacity: true,
                color: crate::ColorDynamics::new(true, 0.5, 0.1, 0.2, 0.2, true, 17).unwrap(),
                ..Default::default()
            },
            dual: crate::DualBrush::new(true, crate::DualMode::Multiply, second).with_color(color),
            write_blend: mode,
            ..Default::default()
        }
    }
    fn records(d: &Document) -> Vec<Vec<(TileCoord, Vec<Pixel>)>> {
        d.layers
            .iter()
            .map(|l| {
                l.tiles
                    .iter()
                    .map(|(c, t)| (*c, t.try_pixels().unwrap().to_vec()))
                    .collect()
            })
            .collect()
    }
    #[test]
    fn changed_pixels_match_full_tiles_all_modes_targets_paths_orders_and_overflow() {
        for id in 0..27 {
            for target in 0..3 {
                for path in [false, true] {
                    for front in [false, true] {
                        for overflow in [false, true] {
                            let mode = crate::BlendMode::from_id(id).unwrap();
                            let mut fast = Document::new(32, 32).unwrap();
                            let mut reference = Document::new(32, 32).unwrap();
                            reference.reference_color_tiles = true;
                            for d in [&mut fast, &mut reference] {
                                d.initialize_white_background().unwrap();
                                if target == 1 {
                                    let mut a = d.layer(1).unwrap().appearance();
                                    a.locks = crate::LOCK_TRANSPARENCY;
                                    d.set_layer_appearance(1, a).unwrap();
                                }
                                if target == 2 {
                                    d.add_mask(1).unwrap();
                                }
                                d.set_selection(
                                    crate::Selection::default()
                                        .apply(
                                            crate::SelectionShape::geometry(
                                                crate::SelectionKind::Ellipse,
                                                3.,
                                                3.,
                                                25.,
                                                25.,
                                                true,
                                            ),
                                            crate::SelectionOperation::Replace,
                                            32,
                                            32,
                                        )
                                        .unwrap(),
                                )
                                .unwrap();
                                d.begin_brush_stroke(
                                    definition(path, front, overflow, mode),
                                    InputPoint::new(8.5, 16.5, 0.3),
                                )
                                .unwrap();
                                for (x, p) in [(24.5, 0.8), (8.5, 0.5), (24.5, 1.)] {
                                    d.stroke_to(InputPoint::new(x, 16.5, p)).unwrap();
                                }
                                d.end_stroke().unwrap();
                            }
                            assert_eq!(records(&fast),records(&reference),"mode={id} target={target} path={path} front={front} overflow={overflow}");
                            assert_eq!(fast.history_depth(), reference.history_depth());
                            fast.undo().unwrap();
                            reference.undo().unwrap();
                            assert_eq!(records(&fast), records(&reference));
                            fast.redo().unwrap();
                            reference.redo().unwrap();
                            assert_eq!(records(&fast), records(&reference));
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn color_private_writes_keep_before_and_shared_snapshots_immutable() {
        let mut d = Document::new(64, 64).unwrap();
        d.begin_brush_stroke(
            definition(false, false, true, crate::BlendMode::Normal),
            InputPoint::new(16.5, 32.5, 1.),
        )
        .unwrap();
        let coord = TileCoord::from_signed(0, 0);
        let id = d.layers[0].tiles[&coord].id;
        d.stroke_to(InputPoint::new(24.5, 32.5, 1.)).unwrap();
        assert_eq!(d.layers[0].tiles[&coord].id, id);
        let snapshot = d.snapshot();
        let pixels = snapshot.layers[0].tiles[&coord]
            .try_pixels()
            .unwrap()
            .to_vec();
        d.stroke_to(InputPoint::new(40.5, 32.5, 1.)).unwrap();
        assert_ne!(d.layers[0].tiles[&coord].id, id);
        assert_eq!(
            snapshot.layers[0].tiles[&coord]
                .try_pixels()
                .unwrap()
                .to_vec(),
            pixels
        );
        d.cancel_stroke().unwrap();
        assert_eq!(d.tile_count(), 0);
        assert_eq!(d.history_depth(), (0, 0));
    }
    #[test]
    fn one_percent_dense_segment_is_accepted_and_matches_split_input() {
        for path in [false, true] {
            let mut b = definition(path, false, true, crate::BlendMode::Normal);
            b.brush.spacing = 0.02;
            let second = crate::SecondaryTip::new(6., b.tip, None, Default::default()).unwrap();
            let second = if path {
                second.with_spacing(0.01).unwrap()
            } else {
                second
            };
            b.dual = crate::DualBrush::new(true, crate::DualMode::Multiply, second)
                .with_color(b.dual.color());
            let mut whole = Document::new(64, 64).unwrap();
            let mut split = Document::new(64, 64).unwrap();
            for d in [&mut whole, &mut split] {
                d.begin_brush_stroke(b.clone(), InputPoint::new(8.5, 32.5, 1.))
                    .unwrap();
            }
            whole.stroke_to(InputPoint::new(56.5, 32.5, 1.)).unwrap();
            for i in 1..=48 {
                split
                    .stroke_to(InputPoint::new(8.5 + i as f64, 32.5, 1.))
                    .unwrap();
            }
            whole.end_stroke().unwrap();
            split.end_stroke().unwrap();
            assert_eq!(records(&whole), records(&split));
            assert_eq!(whole.history_depth(), (1, 0));
        }
    }
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

    #[test]
    fn interleaved_stamp_order_overlays_later_secondary_on_earlier_primary() {
        let tip = crate::BrushTip {
            hardness: 1.,
            flow: 1.,
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        };
        let red = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
        let blue = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
        for model in [
            crate::DualColorModel::SeparatedStroke,
            crate::DualColorModel::InterleavedStamp,
        ] {
            let color = crate::DualColor::new_with_model(
                model,
                true,
                true,
                true,
                crate::DualColorOrder::SecondBehind,
                crate::BlendMode::Normal,
                crate::BlendMode::Normal,
            )
            .unwrap();
            let definition = BrushDefinition {
                brush: Brush {
                    radius: 6.,
                    spacing: 1.,
                    opacity: 1.,
                    color: red,
                    background_color: blue,
                    ..Default::default()
                },
                tip,
                dual: crate::DualBrush::new(
                    true,
                    crate::DualMode::Multiply,
                    crate::SecondaryTip::new(14., tip, None, Default::default()).unwrap(),
                )
                .with_color(color),
                ..Default::default()
            };
            let mut d = Document::new(64, 64).unwrap();
            d.begin_brush_stroke(definition, InputPoint::new(-100., -100., 1.))
                .unwrap();
            let mut stroke = d.stroke.take().unwrap();
            d.dab(&mut stroke, InputPoint::new(16.5, 32.5, 1.)).unwrap();
            d.dab(&mut stroke, InputPoint::new(32.5, 32.5, 1.)).unwrap();
            d.stroke = Some(stroke);
            d.end_stroke().unwrap();
            assert_eq!(
                d.pixel(20, 32),
                if model == crate::DualColorModel::InterleavedStamp {
                    blue
                } else {
                    red
                },
                "{model:?}"
            );
        }
    }
}
