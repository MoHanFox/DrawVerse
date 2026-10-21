//! Outline geometry belongs to the current stroke, never to destination pixel differences.
use super::*;
use crate::{OutlineAlgorithm, OutlineSource, StrokeOutline, MAX_STROKE_OUTLINE_PIXELS};
#[path = "outline_brush.rs"]
mod outline_brush;

#[path = "outline_replay.rs"]
mod outline_replay;
#[path = "outline_sdf.rs"]
mod outline_sdf;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ShapeCoverage {
    rows: [u64; 64],
    // Empty for the original binary path. Allocated only for SDF shape tiles.
    alpha: Vec<u8>,
    replay: Vec<paint_ops::replay_outline::Coverage>,
}
pub(super) type ShapeTile = Arc<ShapeCoverage>;
pub(super) type ShapeTiles = BTreeMap<TileCoord, ShapeTile>;
pub(super) type ShapeUpdates = Vec<(TileCoord, ShapeCoverage)>;

#[derive(Debug)]
pub(super) struct OutlineStroke {
    settings: StrokeOutline,
    width: f32,
    first: InputPoint,
    pub(super) shape: ShapeTiles,
    sdf_cache: Option<outline_sdf::Cache>,
    preview: BTreeMap<TileCoord, Option<Arc<Tile>>>,
    replay_program: Option<BrushProgram>,
}
impl OutlineStroke {
    pub(super) fn new(
        settings: &StrokeOutline,
        diameter: f32,
        first: InputPoint,
    ) -> Result<Option<Self>> {
        let width = settings.width().effective(diameter)?;
        if !settings.enabled() || width == 0. {
            return Ok(None);
        }
        let replay_program = if settings.algorithm() == OutlineAlgorithm::Replay {
            match settings.source() {
                OutlineSource::Existing(tip) => Some(BrushProgram::compile(BrushDefinition {
                    tip: tip.tip(),
                    mask: tip.mask().cloned(),
                    texture: tip.texture().clone(),
                    ..Default::default()
                })?),
                _ => None,
            }
        } else {
            None
        };
        Ok(Some(Self {
            settings: settings.clone(),
            width,
            first,
            shape: BTreeMap::new(),
            preview: BTreeMap::new(),
            sdf_cache: None,
            replay_program,
        }))
    }
    pub(super) fn invalidate_cache(&mut self) {
        self.sdf_cache = None;
    }
    fn padding(&self) -> i64 {
        let (outside, inside) = self.settings.placement().extents(self.width);
        outside.max(inside).ceil() as i64 + 1
    }
}
impl Document {
    pub(super) fn remove_outline_preview(&mut self, stroke: &mut Stroke) {
        let Some(outline) = &mut stroke.outline else {
            return;
        };
        let preview = std::mem::take(&mut outline.preview);
        if preview.is_empty() {
            return;
        }
        let index = self.layer_index(stroke.layer).expect("stroke layer");
        let shifted = self.layers[index].appearance.offset_x != 0
            || self.layers[index].appearance.offset_y != 0;
        for (coord, tile) in preview {
            Self::restore_tile(&mut self.layers[index], coord, tile);
            if shifted {
                self.dirty.all = true;
                self.dirty.tiles.clear();
            } else {
                self.dirty_tile(coord);
            }
        }
    }

    pub(super) fn prepare_outline_input(&mut self) {
        if let Some(mut stroke) = self.stroke.take() {
            self.remove_outline_preview(&mut stroke);
            self.stroke = Some(stroke);
        }
    }

    pub(super) fn publish_outline(&mut self) -> Result<()> {
        let Some(mut stroke) = self.stroke.take() else {
            return Ok(());
        };
        match self.render_stroke_outline(&mut stroke) {
            Ok(changed) => {
                if changed {
                    self.bump_revision();
                }
                self.stroke = Some(stroke);
                Ok(())
            }
            Err(error) => {
                self.restore_stroke(&stroke);
                Err(error)
            }
        }
    }

    /// Explicit core entry while IO and the native bridge are being connected.
    pub fn begin_outlined_brush_stroke(
        &mut self,
        definition: BrushDefinition,
        first: InputPoint,
        outline: &StrokeOutline,
    ) -> Result<()> {
        self.idle()?;
        if !outline.enabled() {
            return self.begin_brush_stroke(definition, first);
        }
        if definition.wet.enabled()
            || definition.brush.mode != BrushMode::Paint
            || first.tool == Tool::Eraser
            || (definition.dual.enabled() && definition.dual.color().enabled())
            || self
                .layer(self.active_layer)
                .is_some_and(|layer| layer.mask)
        {
            return Err(Error::InvalidArgument(
                "outline entry requires a dry paint brush on a pixel layer",
            ));
        }
        self.begin_brush_stroke_inner(definition, first, None, None, Some(outline))
    }

    pub(super) fn outline_shape(
        &self,
        stroke: &Stroke,
        bounds: (f64, f64, f64, f64),
        point: InputPoint,
        radius: f64,
        sample: impl Fn(f64, f64) -> Result<f32>,
    ) -> Result<ShapeUpdates> {
        let Some(outline) = &stroke.outline else {
            return Ok(Vec::new());
        };
        if outline.settings.algorithm() == OutlineAlgorithm::Replay {
            return outline_replay::accumulate(self, stroke, bounds, point, radius, sample);
        }
        let padding = outline.padding();
        let (left, top, right, bottom) = (
            (bounds.0.floor() as i64).max(-padding),
            (bounds.1.floor() as i64).max(-padding),
            (bounds.2.ceil() as i64).min(i64::from(self.width) + padding),
            (bounds.3.ceil() as i64).min(i64::from(self.height) + padding),
        );
        if left >= right || top >= bottom {
            return Ok(Vec::new());
        }
        if (right - left) as usize * (bottom - top) as usize > MAX_STROKE_OUTLINE_PIXELS {
            return Err(Error::ResourceLimit("stroke outline dab area"));
        }
        let mut updates = Vec::new();
        for ty in top.div_euclid(64)..=(bottom - 1).div_euclid(64) {
            for tx in left.div_euclid(64)..=(right - 1).div_euclid(64) {
                let coord = TileCoord::from_signed(tx as i32, ty as i32);
                let old = outline.shape.get(&coord);
                let sdf = outline.settings.algorithm() == OutlineAlgorithm::Sdf;
                let mut data = ShapeCoverage {
                    rows: old.map_or([0; 64], |p| p.rows),
                    alpha: Vec::new(),
                    replay: Vec::new(),
                };
                if sdf {
                    data.alpha
                        .try_reserve_exact(4096)
                        .map_err(|_| Error::ResourceLimit("outline alpha tile allocation"))?;
                    if let Some(old) = old {
                        data.alpha.extend_from_slice(&old.alpha);
                    } else {
                        data.alpha.resize(4096, 0);
                    }
                }
                for y in top.max(ty * 64)..bottom.min((ty + 1) * 64) {
                    for x in left.max(tx * 64)..right.min((tx + 1) * 64) {
                        let bit = 1_u64 << x.rem_euclid(64);
                        let row_index = y.rem_euclid(64) as usize;
                        let row = &mut data.rows[row_index];
                        if sdf {
                            let i = row_index * 64 + x.rem_euclid(64) as usize;
                            if data.alpha[i] < 255 {
                                let value = sample(x as f64 + 0.5, y as f64 + 0.5)?;
                                data.alpha[i] =
                                    data.alpha[i].max((value.clamp(0., 1.) * 255.).round() as u8);
                                if data.alpha[i] != 0 {
                                    *row |= bit;
                                }
                            }
                        } else if *row & bit == 0 && sample(x as f64 + 0.5, y as f64 + 0.5)? >= 0.5
                        {
                            *row |= bit;
                        }
                    }
                }
                if old.is_some_and(|p| **p == data) || data.rows.iter().all(|r| *r == 0) {
                    continue;
                }
                updates.push((coord, data));
            }
        }
        Ok(updates)
    }

    pub(super) fn render_stroke_outline(&mut self, stroke: &mut Stroke) -> Result<bool> {
        let Some(outline) = &mut stroke.outline else {
            return Ok(false);
        };
        if outline.shape.is_empty() {
            return Ok(false);
        }
        let mut bounds = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for (coord, rows) in &outline.shape {
            let (tx, ty) = (
                i64::from(coord.signed_x()) * 64,
                i64::from(coord.signed_y()) * 64,
            );
            for (y, &row) in rows.rows.iter().enumerate().filter(|(_, row)| **row != 0) {
                bounds.0 = bounds.0.min(tx + i64::from(row.trailing_zeros()));
                bounds.1 = bounds.1.min(ty + y as i64);
                bounds.2 = bounds.2.max(tx + 64 - i64::from(row.leading_zeros()));
                bounds.3 = bounds.3.max(ty + y as i64 + 1);
            }
        }
        let replay = outline.settings.algorithm() == OutlineAlgorithm::Replay;
        let padding = if replay { 0 } else { outline.padding() };
        let (left, top, right, bottom) = (
            bounds.0 - padding,
            bounds.1 - padding,
            bounds.2 + padding,
            bounds.3 + padding,
        );
        let (width, height) = ((right - left) as usize, (bottom - top) as usize);
        let count = width
            .checked_mul(height)
            .filter(|n| *n <= MAX_STROKE_OUTLINE_PIXELS)
            .ok_or(Error::ResourceLimit("stroke outline bounds pixel limit"))?;
        let sdf = outline.settings.algorithm() == OutlineAlgorithm::Sdf;
        let band = if replay {
            outline_replay::compose(outline, left, top, width, height)?
        } else if let Some(cache) = outline.sdf_cache.as_ref().filter(|_| sdf) {
            Arc::clone(&cache.band)
        } else {
            let mut shape = Vec::new();
            shape
                .try_reserve_exact(count)
                .map_err(|_| Error::ResourceLimit("stroke outline shape allocation"))?;
            shape.resize(count, 0_u8);
            for (coord, tile) in &outline.shape {
                let (tx, ty) = (
                    i64::from(coord.signed_x()) * 64,
                    i64::from(coord.signed_y()) * 64,
                );
                for (y, &row) in tile.rows.iter().enumerate() {
                    let mut bits = row;
                    while bits != 0 {
                        let x = i64::from(bits.trailing_zeros());
                        shape[(ty + y as i64 - top) as usize * width + (tx + x - left) as usize] =
                            if sdf {
                                tile.alpha[y * 64 + x as usize]
                            } else {
                                255
                            };
                        bits &= bits - 1;
                    }
                }
            }
            if sdf {
                let cache =
                    outline_sdf::render(outline, &stroke.program, left, top, width, height, shape)?;
                let band = Arc::clone(&cache.band);
                outline.sdf_cache = Some(cache);
                band
            } else {
                let mut band = crate::stroke_outline_band(
                    width as u32,
                    height as u32,
                    &shape,
                    outline.width,
                    outline.settings.placement(),
                )?;
                outline_brush::apply_brush(
                    width,
                    height,
                    &shape,
                    &mut band,
                    &outline.settings,
                    outline.width,
                    self.options,
                )?;
                Arc::from(band)
            }
        };
        let texture = match outline.settings.source() {
            OutlineSource::Texture(texture) | OutlineSource::SelfTextured(texture) => {
                if sdf || replay {
                    None
                } else {
                    texture.compile()
                }
            }
            _ => None,
        };
        let texture = texture
            .as_ref()
            .map(|t| t.dab(outline.first.x, outline.first.y, 1., 0., 1.));
        let color = if outline.settings.secondary_color() {
            stroke.brush.background_color
        } else {
            stroke.brush.color
        };
        let index = self.layer_index(stroke.layer)?;
        let appearance = self.layers[index].appearance;
        let alpha_locked = appearance.locks & crate::LOCK_TRANSPARENCY != 0;
        let (ox, oy) = (
            i64::from(appearance.offset_x),
            i64::from(appearance.offset_y),
        );
        let (x0, y0, x1, y1) = (
            left.max(0) - ox,
            top.max(0) - oy,
            right.min(i64::from(self.width)) - ox,
            bottom.min(i64::from(self.height)) - oy,
        );
        if x0 >= x1 || y0 >= y1 {
            return Ok(false);
        }
        let mut changed = false;
        let mut preview = BTreeMap::new();
        for ty in y0.div_euclid(64)..=(y1 - 1).div_euclid(64) {
            for tx in x0.div_euclid(64)..=(x1 - 1).div_euclid(64) {
                let coord = TileCoord::from_signed(tx as i32, ty as i32);
                let source = self.layers[index]
                    .tiles
                    .get(&coord)
                    .map(|t| t.try_pixels())
                    .transpose()?;
                let mut values = None;
                for y in y0.max(ty * 64)..y1.min((ty + 1) * 64) {
                    let start = x0.max(tx * 64);
                    let end = x1.min((tx + 1) * 64);
                    let selected = self.selection_cache.row(
                        &self.selection,
                        start + ox,
                        y + oy,
                        (end - start) as usize,
                    );
                    for x in start..end {
                        let (wx, wy) = (x + ox, y + oy);
                        let coverage = band[(wy - top) as usize * width + (wx - left) as usize];
                        let amount = coverage
                            * selected[(x - start) as usize]
                            * stroke.brush.opacity
                            * texture
                                .as_ref()
                                .map_or(1., |t| t.coverage(wx as f64 + 0.5, wy as f64 + 0.5));
                        if amount == 0. {
                            continue;
                        }
                        let offset = (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize;
                        let before = source
                            .as_ref()
                            .map_or(self.layers[index].default_pixel(x, y), |p| p[offset]);
                        let after = if alpha_locked {
                            before.paint_blended_preserving_alpha(
                                color,
                                amount,
                                stroke.program.write_blend,
                                (wx as u32, wy as u32),
                                stroke.random_stroke,
                            )?
                        } else {
                            crate::blend_pixel(
                                color.scaled(amount),
                                before,
                                stroke.program.write_blend,
                                wx as u32,
                                wy as u32,
                                stroke.random_stroke,
                            )
                        };
                        if before == after {
                            continue;
                        }
                        values.get_or_insert_with(|| {
                            source.as_ref().map_or_else(
                                || {
                                    (0..4096)
                                        .map(|i| {
                                            self.layers[index]
                                                .default_pixel(tx * 64 + i % 64, ty * 64 + i / 64)
                                        })
                                        .collect::<Vec<_>>()
                                },
                                |p| p.to_vec(),
                            )
                        })[offset] = after;
                    }
                }
                drop(source);
                if let Some(values) = values {
                    if !self.layers[index].tiles.contains_key(&coord)
                        && self.tile_count() >= self.options.max_document_tiles
                    {
                        return Err(Error::ResourceLimit(
                            "outline document tile limit; entire stroke rolled back",
                        ));
                    }
                    stroke
                        .before
                        .entry(coord)
                        .or_insert_with(|| self.layers[index].tiles.get(&coord).cloned());
                    preview.insert(coord, self.layers[index].tiles.get(&coord).cloned());
                    let tile = Arc::new(Tile::from_pixels(values)?);
                    self.pool.attach(&tile)?;
                    self.layers[index].tiles.insert(coord, tile);
                    if ox == 0 && oy == 0 {
                        self.dirty_tile(coord);
                    } else {
                        self.dirty.all = true;
                        self.dirty.tiles.clear();
                    }
                    changed = true;
                    self.pool.trim()?;
                }
            }
        }
        stroke.outline.as_mut().expect("outline stroke").preview = preview;
        Ok(changed)
    }
}
pub(super) fn install_shape(
    outline: &mut Option<OutlineStroke>,
    tail: &mut Option<taper::TailReplay>,
    updates: ShapeUpdates,
    limit: usize,
) -> Result<()> {
    let Some(outline) = outline else {
        return Ok(());
    };
    let limit = limit.min(MAX_STROKE_OUTLINE_PIXELS / 4096);
    if !updates.is_empty() {
        outline.sdf_cache = None;
    }
    for (coord, rows) in updates {
        if !outline.shape.contains_key(&coord) && outline.shape.len() >= limit {
            return Err(Error::ResourceLimit("stroke outline shape tile limit"));
        }
        taper::capture_outline(tail.as_mut(), coord, &outline.shape, limit)?;
        outline.shape.insert(coord, Arc::new(rows));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BrushTip, OutlinePlacement, OutlineWidth, PressureControl, PressureCurve, StrokeTaper,
        TaperEnd,
    };

    #[test]
    fn sdf_cache_reuses_exact_shape_and_invalidates_on_new_coverage() {
        let definition = BrushDefinition {
            brush: Brush {
                radius: 8.,
                ..Default::default()
            },
            tip: BrushTip {
                hardness: 0.5,
                pressure_size: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let settings = StrokeOutline::new(
            true,
            OutlinePlacement::Outside,
            OutlineWidth::new(false, 4., 10.).unwrap(),
            OutlineSource::SelfBrush,
            true,
        )
        .unwrap()
        .with_algorithm(OutlineAlgorithm::Sdf);
        let mut doc = Document::new(96, 64).unwrap();
        doc.begin_outlined_brush_stroke(definition, InputPoint::new(24., 32., 1.), &settings)
            .unwrap();
        let cache = Arc::clone(
            &doc.stroke
                .as_ref()
                .unwrap()
                .outline
                .as_ref()
                .unwrap()
                .sdf_cache
                .as_ref()
                .unwrap()
                .band,
        );
        assert!(doc
            .stroke
            .as_ref()
            .unwrap()
            .outline
            .as_ref()
            .unwrap()
            .shape
            .values()
            .any(|tile| tile.alpha.iter().any(|a| *a > 0 && *a < 255)));
        doc.prepare_outline_input();
        doc.publish_outline().unwrap();
        assert!(Arc::ptr_eq(
            &cache,
            &doc.stroke
                .as_ref()
                .unwrap()
                .outline
                .as_ref()
                .unwrap()
                .sdf_cache
                .as_ref()
                .unwrap()
                .band
        ));
        doc.stroke_to(InputPoint::new(64., 32., 1.)).unwrap();
        assert!(!Arc::ptr_eq(
            &cache,
            &doc.stroke
                .as_ref()
                .unwrap()
                .outline
                .as_ref()
                .unwrap()
                .sdf_cache
                .as_ref()
                .unwrap()
                .band
        ));
        doc.cancel_stroke().unwrap();
        assert!(doc.stroke.is_none());
    }

    #[test]
    fn outline_tail_shape_matches_one_pass_known_final_endpoint() {
        let control = PressureControl::new(PressureCurve::default(), 0.).unwrap();
        let definition = BrushDefinition {
            brush: Brush {
                radius: 7.,
                spacing: 0.3,
                color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
                background_color: Pixel::from_straight([0., 0., 1., 1.]).unwrap(),
                ..Default::default()
            },
            tip: BrushTip {
                pressure_size: false,
                pressure_flow: false,
                ..Default::default()
            },
            taper: StrokeTaper::new(
                true,
                TaperEnd::new(15., control.clone(), PressureControl::default()).unwrap(),
                TaperEnd::new(30., control, PressureControl::default()).unwrap(),
            ),
            ..Default::default()
        };
        let points = [
            InputPoint::new(16., 32., 1.),
            InputPoint::new(38., 26., 1.),
            InputPoint::new(64., 32., 1.),
            InputPoint::new(90., 34., 1.),
        ];
        let endpoint: f64 = points
            .windows(2)
            .map(|p| (p[1].x - p[0].x).hypot(p[1].y - p[0].y))
            .sum();
        let settings = StrokeOutline::new(
            true,
            OutlinePlacement::Outside,
            OutlineWidth::default(),
            OutlineSource::SelfBrush,
            true,
        )
        .unwrap();
        let mut actual = Document::new(128, 64).unwrap();
        let mut reference = Document::new(128, 64).unwrap();
        reference.taper_reference_endpoint = Some(endpoint);
        for doc in [&mut actual, &mut reference] {
            doc.begin_outlined_brush_stroke(definition.clone(), points[0], &settings)
                .unwrap();
            for point in &points[1..] {
                doc.stroke_to(*point).unwrap();
            }
            doc.end_stroke().unwrap();
        }
        for y in 0..64 {
            for x in 0..128 {
                assert_eq!(actual.pixel(x, y), reference.pixel(x, y), "pixel {x},{y}");
            }
        }
    }
}
