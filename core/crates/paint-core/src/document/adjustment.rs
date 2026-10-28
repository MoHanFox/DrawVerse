use super::*;

/// Validated settings, independent of UI/graphics ownership. LUTs operate in display sRGB.
#[derive(Clone, Debug)]
pub struct Adjustment {
    pub kind: u32,
    pub parameters: [f32; 4],
    pub lut: Vec<[f32; 4]>,
}

#[derive(Debug)]
pub(super) struct AdjustmentState {
    original: Layer,
    candidate: Layer,
    kind: u32,
    changed: bool,
    histogram: Arc<Vec<f32>>,
}
impl Adjustment {
    pub fn validate(&self) -> Result<()> {
        if self.kind > 2 {
            return Err(Error::InvalidArgument("adjustment kind"));
        }
        self.operator(vec![[0.; 4]])
            .validate()
            .map_err(|_| Error::InvalidArgument("adjustment parameters"))
    }
    fn operator(&self, pixels: Vec<[f32; 4]>) -> paint_ops::AdjustmentOp {
        paint_ops::AdjustmentOp {
            pixels,
            kind: u32::from(self.kind == 2),
            parameters: self.parameters,
            lut: self.lut.clone(),
        }
    }
    fn neutral(&self) -> bool {
        if self.kind == 2 {
            return self.parameters == [0.; 4];
        }
        self.lut
            .iter()
            .enumerate()
            .all(|(i, p)| p[..3].iter().all(|v| *v == i as f32 / 4096.))
    }
}
impl Document {
    pub fn adjustment_active(&self) -> bool {
        self.adjustment.is_some()
    }
    pub fn adjustment_histogram(&self) -> Option<Arc<Vec<f32>>> {
        self.adjustment.as_ref().map(|s| s.histogram.clone())
    }
    pub fn begin_adjustment(&mut self, kind: u32) -> Result<()> {
        self.idle()?;
        if kind > 2 {
            return Err(Error::InvalidArgument("adjustment kind"));
        }
        self.check_ancestors(self.active_layer, crate::LOCK_ALL)?;
        let layer = self
            .layer(self.active_layer)
            .ok_or(Error::InvalidArgument("adjustment target"))?;
        if layer.group || layer.mask || layer.placed.is_some() {
            return Err(Error::InvalidArgument("adjustment requires a raster layer"));
        }
        // One bounded, deterministic source sample; cold tiles are read on the Actor.
        // Sparse transparent pixels carry no weight. Selection coverage is in canvas space.
        let mut keys: BTreeSet<_> = layer.tiles.keys().copied().collect();
        let columns = layer.white.0.div_ceil(64);
        let rows = layer.white.1.div_ceil(64);
        if u64::from(columns) * u64::from(rows) > self.options.max_document_tiles as u64 {
            return Err(Error::ResourceLimit("adjustment histogram tiles"));
        }
        for y in 0..rows {
            for x in 0..columns {
                keys.insert(TileCoord { x, y });
            }
        }
        let stride = (keys.len() as u64 * 4096).div_ceil(262144).max(1);
        let mut histogram = vec![0_f32; 1024];
        let mut sample = 0_u64;
        for (index, key) in keys.into_iter().enumerate() {
            let begin = index as u64 * 4096;
            let end = begin + 4096;
            if sample >= end {
                continue;
            }
            let pixels = layer.tiles.get(&key).map(|t| t.try_pixels()).transpose()?;
            while sample < end {
                let local = sample - begin;
                let x = i64::from(key.signed_x()) * 64 + (local % 64) as i64;
                let y = i64::from(key.signed_y()) * 64 + (local / 64) as i64;
                let cx = x + i64::from(layer.appearance.offset_x);
                let cy = y + i64::from(layer.appearance.offset_y);
                if cx >= 0 && cy >= 0 && cx < i64::from(self.width) && cy < i64::from(self.height) {
                    let p = pixels
                        .as_ref()
                        .map_or_else(|| layer.default_pixel(x, y), |p| p[local as usize])
                        .components();
                    let mut coverage = [1.];
                    if self.selection.enabled() {
                        self.selection.coverage_span(
                            cx as f64 + 0.5,
                            cy as f64 + 0.5,
                            &mut coverage,
                        );
                    }
                    let weight = p[3] * coverage[0];
                    if weight > 0. {
                        let c = [0, 1, 2]
                            .map(|i| paint_ops::adjustment::encode(p[i] / p[3]).clamp(0., 1.));
                        for i in 0..3 {
                            histogram[(i + 1) * 256 + (c[i] * 255.).round() as usize] += weight;
                        }
                        histogram[((c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114) * 255.).round()
                            as usize] += weight;
                    }
                }
                sample += stride;
            }
        }
        self.adjustment = Some(AdjustmentState {
            original: layer.clone(),
            candidate: layer.clone(),
            kind,
            changed: false,
            histogram: Arc::new(histogram),
        });
        Ok(())
    }
    pub fn update_adjustment(&mut self, settings: &Adjustment) -> Result<()> {
        settings.validate()?;
        let state = self
            .adjustment
            .as_ref()
            .ok_or(Error::InvalidArgument("no adjustment"))?;
        if state.kind != settings.kind {
            return Err(Error::InvalidArgument("adjustment session kind"));
        }
        let original = state.original.clone();
        let mut candidate = original.clone();
        let mut changed = false;
        if !settings.neutral() {
            let mut keys: BTreeSet<_> = original.tiles.keys().copied().collect();
            let columns = original.white.0.div_ceil(64);
            let rows = original.white.1.div_ceil(64);
            if u64::from(columns) * u64::from(rows) > self.options.max_document_tiles as u64 {
                return Err(Error::ResourceLimit("adjustment white tiles"));
            }
            for y in 0..rows {
                for x in 0..columns {
                    keys.insert(TileCoord { x, y });
                }
            }
            if keys.len() > self.options.max_document_tiles {
                return Err(Error::ResourceLimit("adjustment tiles"));
            }
            let keys: Vec<_> = keys.into_iter().collect();
            // Bounded batches avoid one GPU dispatch/readback per tile and full-image buffers.
            for batch in keys.chunks(16) {
                let mut source = Vec::with_capacity(batch.len() * 4096);
                for key in batch {
                    if let Some(tile) = original.tiles.get(key) {
                        source.extend(tile.try_pixels()?.iter().map(|p| p.components()));
                    } else {
                        for y in 0..64 {
                            for x in 0..64 {
                                source.push(
                                    original
                                        .default_pixel(
                                            i64::from(key.signed_x()) * 64 + x,
                                            i64::from(key.signed_y()) * 64 + y,
                                        )
                                        .components(),
                                );
                            }
                        }
                    }
                }
                let output = self
                    .compute
                    .execute(&paint_ops::Operator::Adjustment(
                        settings.operator(source.clone()),
                    ))
                    .map_err(|_| Error::ResourceLimit("adjustment execution"))?;
                if output.len() != source.len() {
                    return Err(Error::InvalidArgument("adjustment output size"));
                }
                for (tile_index, key) in batch.iter().enumerate() {
                    let mut pixels = Vec::with_capacity(4096);
                    let mut modified = false;
                    for y in 0..64 {
                        let mut coverage = [1.; 64];
                        let cy = i64::from(key.signed_y()) * 64
                            + y
                            + i64::from(original.appearance.offset_y);
                        let cx = i64::from(key.signed_x()) * 64
                            + i64::from(original.appearance.offset_x);
                        if self.selection.enabled() {
                            self.selection.coverage_span(
                                cx as f64 + 0.5,
                                cy as f64 + 0.5,
                                &mut coverage,
                            );
                        }
                        for (x, amount) in coverage.into_iter().enumerate() {
                            let i = tile_index * 4096 + y as usize * 64 + x;
                            let before = Pixel::from_premultiplied(source[i])?;
                            let mut raw = output[i];
                            raw[3] = source[i][3];
                            for c in &mut raw[..3] {
                                *c = c.clamp(0., source[i][3]);
                            }
                            let after = Pixel::from_premultiplied(raw)?;
                            let amount = if cx + x as i64 >= 0
                                && cx + (x as i64) < i64::from(self.width)
                                && cy >= 0
                                && cy < i64::from(self.height)
                            {
                                amount
                            } else {
                                0.
                            };
                            let next = if amount == 0. {
                                before
                            } else if amount == 1. {
                                after
                            } else {
                                before.mix(after, amount)
                            };
                            modified |= before != next;
                            pixels.push(next);
                        }
                    }
                    if modified {
                        let tile = Arc::new(Tile::from_pixels(pixels)?);
                        self.pool.attach(&tile)?;
                        candidate.tiles.insert(*key, tile);
                        self.pool.trim()?;
                        changed = true;
                    }
                }
            }
        }
        if self.tile_count() - self.layer(original.id).unwrap().retained_tile_count()
            + candidate.retained_tile_count()
            > self.options.max_document_tiles
        {
            return Err(Error::ResourceLimit("adjustment document tiles"));
        }
        let index = self.layer_index(original.id)?;
        self.layers[index] = candidate.clone();
        let state = self.adjustment.as_mut().unwrap();
        state.candidate = candidate;
        state.changed = changed;
        self.dirty_all();
        Ok(())
    }
    pub fn finish_adjustment(&mut self, commit: bool) -> Result<bool> {
        let state = self
            .adjustment
            .take()
            .ok_or(Error::InvalidArgument("no adjustment"))?;
        let index = self.layer_index(state.original.id)?;
        self.layers[index] = state.original.clone();
        if commit && state.changed {
            let action = match state.kind {
                0 => crate::HistoryAction::Curves,
                1 => crate::HistoryAction::Levels,
                _ => crate::HistoryAction::HueSaturation,
            };
            if let Err(e) = self.commit_with_action(
                Command::LayerEdit {
                    before: vec![state.original.clone()],
                    after: vec![state.candidate.clone()],
                    selection: None,
                },
                action,
            ) {
                self.layers[index] = state.candidate.clone();
                self.adjustment = Some(state);
                self.dirty_all();
                return Err(e);
            }
            self.layers[index] = state.candidate;
        }
        self.dirty_all();
        Ok(commit && state.changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settings(kind: u32) -> Adjustment {
        Adjustment {
            kind,
            parameters: [0.; 4],
            lut: (0..=4096)
                .map(|i| {
                    let x = i as f32 / 4096.;
                    [x, x, x, 1.]
                })
                .collect(),
        }
    }
    #[test]
    fn adjustment_histogram_tracks_original_selection_and_colorize() {
        let mut d = Document::new(65, 65).unwrap();
        d.initialize_white_background().unwrap();
        d.set_selection(crate::Selection::all(65, 65)).unwrap();
        d.begin_adjustment(2).unwrap();
        let hist = d.adjustment_histogram().unwrap();
        assert_eq!(hist.len(), 1024);
        for c in 0..4 {
            assert_eq!(hist[c * 256 + 255], 4225.);
            assert_eq!(hist[c * 256..(c + 1) * 256].iter().sum::<f32>(), 4225.);
        }
        let mut s = settings(2);
        s.parameters = [0.3, 1., -0.5, 1.];
        d.update_adjustment(&s).unwrap();
        assert_eq!(*d.adjustment_histogram().unwrap(), *hist);
        assert!(d.finish_adjustment(true).unwrap());
        assert_ne!(d.pixel(32, 32), Pixel::WHITE);
        d.undo().unwrap();
        assert_eq!(d.pixel(32, 32), Pixel::WHITE);
        assert!(d.adjustment_histogram().is_none());
    }
    #[test]
    fn adjustment_neutral_cancel_commit_history_and_selection() {
        for kind in 0..3 {
            let mut d = Document::new(96, 80).unwrap();
            d.initialize_white_background().unwrap();
            let original = d.snapshot();
            let undo = d.history_depth().0;
            d.begin_adjustment(kind).unwrap();
            assert!(d.undo().is_err());
            d.update_adjustment(&settings(kind)).unwrap();
            assert!(!d.finish_adjustment(true).unwrap());
            assert_eq!(d.history_depth().0, undo);
            let mut s = settings(kind);
            if kind == 2 {
                s.parameters = [0., 0., -0.5, 0.];
            } else {
                for p in &mut s.lut {
                    p[0] = 0.;
                    p[1] = 0.;
                    p[2] = 0.;
                }
            }
            d.begin_adjustment(kind).unwrap();
            d.update_adjustment(&s).unwrap();
            assert_ne!(d.pixel(20, 20), original.pixel(20, 20));
            assert_eq!(d.history_depth().0, undo);
            d.finish_adjustment(false).unwrap();
            assert_eq!(d.pixel(20, 20), original.pixel(20, 20));
            d.set_selection(crate::Selection::all(96, 80)).unwrap();
            let undo = d.history_depth().0;
            d.begin_adjustment(kind).unwrap();
            d.update_adjustment(&s).unwrap();
            d.finish_adjustment(true).unwrap();
            let adjusted = d.pixel(20, 20);
            assert_eq!(d.history_depth().0, undo + 1);
            d.undo().unwrap();
            assert_eq!(d.pixel(20, 20), original.pixel(20, 20));
            d.redo().unwrap();
            assert_eq!(d.pixel(20, 20), adjusted);
        }
    }
    #[test]
    fn adjustment_latest_is_from_original_invalid_request_keeps_preview() {
        let mut d = Document::new(65, 65).unwrap();
        d.initialize_white_background().unwrap();
        d.begin_adjustment(2).unwrap();
        let mut s = settings(2);
        s.parameters[2] = -0.5;
        d.update_adjustment(&s).unwrap();
        let p = d.pixel(32, 32);
        d.update_adjustment(&s).unwrap();
        assert_eq!(d.pixel(32, 32), p);
        s.parameters[2] = f32::NAN;
        assert!(d.update_adjustment(&s).is_err());
        assert_eq!(d.pixel(32, 32), p);
        d.finish_adjustment(false).unwrap();
        assert_eq!(d.pixel(32, 32), Pixel::WHITE);
    }
    #[test]
    fn adjustment_feather_offsets_alpha_and_paged_history() {
        for offset in [-7, 13] {
            let mut d = Document::with_options(
                130,
                96,
                DocumentOptions {
                    max_resident_tiles: 1,
                    max_history_bytes: 2 * TILE_BYTES + 256,
                    ..Default::default()
                },
            )
            .unwrap();
            d.initialize_white_background().unwrap();
            d.layers[0].appearance.offset_x = offset;
            let mut shape = crate::SelectionShape::geometry(
                crate::SelectionKind::Rectangle,
                23.,
                17.,
                38.,
                31.,
                true,
            );
            shape.feather = 5.;
            d.selection = crate::Selection::default()
                .apply(shape, crate::SelectionOperation::Replace, 130, 96)
                .unwrap();
            let original = d.snapshot();
            let selection = d.selection.clone();
            let mut s = settings(0);
            for p in &mut s.lut {
                p[0] = 0.;
                p[1] = 0.;
                p[2] = 0.;
            }
            d.begin_adjustment(0).unwrap();
            d.update_adjustment(&s).unwrap();
            for y in 0..96 {
                for x in 0..130 {
                    let before = original.pixel(x, y);
                    let expected = before.mix(
                        Pixel::from_straight([0., 0., 0., before.alpha()]).unwrap(),
                        selection.coverage(x as f64 + 0.5, y as f64 + 0.5),
                    );
                    assert_eq!(d.pixel(x, y), expected, "{offset} {x} {y}");
                }
            }
            d.finish_adjustment(true).unwrap();
            let after = d.snapshot();
            d.undo().unwrap();
            for y in 0..96 {
                for x in 0..130 {
                    assert_eq!(d.pixel(x, y), original.pixel(x, y));
                }
            }
            d.redo().unwrap();
            assert_eq!(d.selection, selection);
            for y in 0..96 {
                for x in 0..130 {
                    assert_eq!(d.pixel(x, y), after.pixel(x, y));
                }
            }
        }
    }
}
