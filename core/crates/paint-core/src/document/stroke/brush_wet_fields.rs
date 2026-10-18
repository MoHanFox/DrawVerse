//! One coverage evaluation feeds the two independently journaled wet fields.
use super::*;
impl Document {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn wet_color_fields(
        &self,
        geometry: &mut BTreeMap<TileCoord, Arc<Tile>>,
        color: &mut BTreeMap<TileCoord, Arc<Tile>>,
        bounds: (f64, f64, f64, f64),
        offset: (i64, i64),
        limit: f32,
        coverage: impl Fn(f64, f64) -> f32,
        ink: impl Fn(f64, f64) -> Pixel,
        mut checkpoint: (Option<&mut taper::TailReplay>, bool),
    ) -> Result<ChangedPixels> {
        #[cfg(test)]
        if self.reference_wet_fields {
            let mut touched = self.color_field(
                geometry,
                bounds,
                offset,
                Pixel::WHITE,
                limit,
                &coverage,
                (checkpoint.0.as_deref_mut(), checkpoint.1),
                None,
            )?;
            touched.extend(self.color_field(
                color,
                bounds,
                offset,
                Pixel::WHITE,
                limit,
                coverage,
                checkpoint,
                Some(&ink),
            )?);
            return Ok(touched);
        }
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
        for ty in y0.div_euclid(64)..=(y1 - 1).div_euclid(64) {
            for tx in x0.div_euclid(64)..=(x1 - 1).div_euclid(64) {
                let coord = TileCoord::from_signed(tx as i32, ty as i32);
                let old_geometry = geometry.get(&coord).map(|t| t.try_pixels()).transpose()?;
                let old_color = color.get(&coord).map(|t| t.try_pixels()).transpose()?;
                let mut geometry_updates = Vec::new();
                let mut color_updates = Vec::new();
                for y in y0.max(ty * 64)..y1.min((ty + 1) * 64) {
                    for x in x0.max(tx * 64)..x1.min((tx + 1) * 64) {
                        let i = (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize;
                        let wx = x as f64 + ox as f64 + 0.5;
                        let wy = y as f64 + oy as f64 + 0.5;
                        let amount = coverage(wx, wy);
                        if amount == 0. {
                            continue;
                        }
                        let before = old_geometry.as_ref().map_or(Pixel::TRANSPARENT, |p| p[i]);
                        let previous = before.alpha();
                        if previous < limit {
                            let accumulated = previous + amount * (limit - previous).max(0.);
                            if accumulated != previous {
                                let old = before.components();
                                let delta = accumulated - previous;
                                // Keep the original white color-field arithmetic, including rounding.
                                let values = std::array::from_fn(|j| {
                                    if j == 3 {
                                        accumulated
                                    } else {
                                        (old[j] + delta).min(accumulated)
                                    }
                                });
                                geometry_updates.push((i, Pixel::from_premultiplied(values)?));
                            }
                        }
                        // Color still mixes after geometric saturation.
                        let before = old_color.as_ref().map_or(Pixel::TRANSPARENT, |p| p[i]);
                        let after = Self::wet_contribution(before, ink(wx, wy), amount, limit)?;
                        if after != before {
                            color_updates.push((i, after));
                        }
                    }
                }
                for (field, source, updates, wet) in [
                    (&mut *geometry, old_geometry, geometry_updates, false),
                    (&mut *color, old_color, color_updates, true),
                ] {
                    if !updates.is_empty() {
                        (if wet {
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
                        if !field.contains_key(&coord)
                            && field.len() >= self.options.max_document_tiles
                        {
                            return Err(Error::ResourceLimit(
                                "dual-color field tile limit; entire stroke rolled back",
                            ));
                        }
                        touched.insert(coord, &updates);
                        if field.get(&coord).is_some_and(Tile::can_edit_private) {
                            drop(source);
                            Tile::edit_private(
                                field.get_mut(&coord).expect("wet color field"),
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
        }
        Ok(touched)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn wet_fields_skip_zero_ink_samples_share_coverage_and_keep_cow_sources() {
        let mut doc = Document::new(16, 16).unwrap();
        let run = |doc: &Document,
                   geometry: &mut BTreeMap<TileCoord, Arc<Tile>>,
                   color: &mut BTreeMap<TileCoord, Arc<Tile>>| {
            let coverage = Cell::new(0);
            let samples = Cell::new(0);
            doc.wet_color_fields(
                geometry,
                color,
                (0., 0., 16., 16.),
                (0, 0),
                1.,
                |x, _| {
                    coverage.set(coverage.get() + 1);
                    if x < 8. {
                        0.
                    } else {
                        0.25
                    }
                },
                |_, _| {
                    samples.set(samples.get() + 1);
                    Pixel::WHITE
                },
                (None, false),
            )
            .unwrap();
            (coverage.get(), samples.get())
        };
        let mut geometry = BTreeMap::new();
        let mut color = BTreeMap::new();
        assert_eq!(run(&doc, &mut geometry, &mut color), (256, 128));
        let old_geometry = geometry.clone();
        let old_color = color.clone();
        let old_pixels = old_color.values().next().unwrap().try_pixels().unwrap();
        let mut reference_geometry = geometry.clone();
        let mut reference_color = color.clone();
        assert_eq!(run(&doc, &mut geometry, &mut color), (256, 128));
        assert_eq!(
            old_color.values().next().unwrap().try_pixels().unwrap(),
            old_pixels
        );
        assert_ne!(
            color.values().next().unwrap().try_pixels().unwrap(),
            old_pixels
        );
        doc.reference_wet_fields = true;
        assert_eq!(
            run(&doc, &mut reference_geometry, &mut reference_color),
            (512, 256)
        );
        for (a, b) in [(&geometry, &reference_geometry), (&color, &reference_color)] {
            for (coord, tile) in a {
                assert_eq!(tile.try_pixels().unwrap(), b[coord].try_pixels().unwrap());
            }
        }
        assert_ne!(
            old_geometry.values().next().unwrap().try_pixels().unwrap(),
            geometry.values().next().unwrap().try_pixels().unwrap()
        );
    }
}
