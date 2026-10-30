//! Exact quarter turns; candidates and cold history are completed before publication.
use super::*;
impl Document {
    pub fn rotate_image(&mut self, clockwise: bool) -> Result<()> {
        self.idle()?;
        let mut after = self.layers.clone();
        let mut count = after
            .iter()
            .map(|l| l.placed.as_ref().map_or(0, |p| p.source.tiles.len()))
            .sum::<usize>();
        for layer in &mut after {
            if layer.group {
                continue;
            }
            let original = self.layer(layer.id).expect("target");
            let (w, h) = (i64::from(original.white.0), i64::from(original.white.1));
            let (ox, oy) = (
                i64::from(original.appearance.offset_x),
                i64::from(original.appearance.offset_y),
            );
            let (nx, ny) = if clockwise {
                (i64::from(self.height) - oy - h, ox)
            } else {
                (oy, i64::from(self.width) - ox - w)
            };
            layer.appearance.offset_x =
                i32::try_from(nx).map_err(|_| Error::InvalidArgument("rotation offset"))?;
            layer.appearance.offset_y =
                i32::try_from(ny).map_err(|_| Error::InvalidArgument("rotation offset"))?;
            layer.appearance.validate()?;
            layer.white = (h as u32, w as u32);
            if let Some(image) = &mut layer.placed {
                let image = Arc::make_mut(image);
                for p in &mut image.geometry.points {
                    *p = if clockwise {
                        [h as f64 - p[1], p[0]]
                    } else {
                        [p[1], w as f64 - p[0]]
                    };
                }
                image.geometry.validate()?;
            }
            let mut keys = BTreeSet::new();
            for key in original.tiles.keys() {
                let (x, y) = (
                    i64::from(key.signed_x()) * 64,
                    i64::from(key.signed_y()) * 64,
                );
                let (x, y) = if clockwise {
                    (h - y - 64, x)
                } else {
                    (y, w - x - 64)
                };
                for ty in y.div_euclid(64)..=(y + 63).div_euclid(64) {
                    for tx in x.div_euclid(64)..=(x + 63).div_euclid(64) {
                        keys.insert(TileCoord::from_signed(
                            i32::try_from(tx)
                                .map_err(|_| Error::ResourceLimit("rotation coordinate"))?,
                            i32::try_from(ty)
                                .map_err(|_| Error::ResourceLimit("rotation coordinate"))?,
                        ));
                    }
                }
            }
            layer.tiles.clear();
            let mut sampler = SourceSampler::new(original);
            self.progress_stage(ComputationPhase::Resampling, keys.len());
            for key in keys {
                let mut pixels = Vec::with_capacity(4096);
                let mut changed = false;
                for i in 0..4096 {
                    let (x, y) = (
                        i64::from(key.signed_x()) * 64 + (i % 64) as i64,
                        i64::from(key.signed_y()) * 64 + (i / 64) as i64,
                    );
                    let (sx, sy) = if clockwise {
                        (y, h - 1 - x)
                    } else {
                        (w - 1 - y, x)
                    };
                    let pixel = sampler.pixel(sx, sy)?;
                    changed |= pixel != layer.default_pixel(x, y);
                    pixels.push(pixel);
                }
                if changed {
                    count += 1;
                    if count > self.options.max_document_tiles {
                        return Err(Error::ResourceLimit("rotation tiles"));
                    }
                    let tile = Arc::new(Tile::from_pixels(pixels)?);
                    self.pool.attach(&tile)?;
                    layer.tiles.insert(key, tile);
                    self.pool.trim()?;
                }
                self.progress_advance(1);
            }
        }
        let selection = self.selection.rotated(self.width, self.height, clockwise)?;
        self.progress_stage(ComputationPhase::History, 0);
        let command = Command::Canvas {
            before_size: [self.width, self.height, self.dpi],
            after_size: [self.height, self.width, self.dpi],
            before: self.layers.clone(),
            after,
            selection: (self.selection.clone(), selection),
        };
        self.commit_with_action(
            command.clone(),
            if clockwise {
                crate::HistoryAction::RotateClockwise
            } else {
                crate::HistoryAction::RotateCounterclockwise
            },
        )?;
        self.apply_history(&command, true);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quarter_turn_mask_selection_and_atomic_limit() {
        let mut d = Document::new(131, 79).unwrap();
        let mut shape =
            crate::SelectionShape::geometry(crate::SelectionKind::Mask, 2., 3., 3., 2., false);
        shape.mask =
            Some(crate::SelectionMask::from_alpha(3, 2, vec![0, 10, 255, 90, 130, 200]).unwrap());
        d.set_selection(
            crate::Selection::from_steps(
                true,
                vec![crate::SelectionStep {
                    operation: crate::SelectionOperation::Replace,
                    shape: Some(shape),
                }],
            )
            .unwrap(),
        )
        .unwrap();
        let selection = d.selection.clone();
        d.rotate_image(true).unwrap();
        for y in 0..79 {
            for x in 0..131 {
                assert_eq!(
                    selection.coverage(x as f64 + 0.5, y as f64 + 0.5),
                    d.selection.coverage((78 - y) as f64 + 0.5, x as f64 + 0.5)
                );
            }
        }
        d.undo().unwrap();
        assert_eq!(d.selection, selection);
        let tile = Arc::new(Tile::from_pixels(vec![Pixel::WHITE; 4096]).unwrap());
        d.layers[0]
            .tiles
            .insert(TileCoord::from_signed(-1, 0), tile);
        d.options.max_document_tiles = 0;
        let depth = d.history_depth();
        let revision = d.revision;
        assert!(d.rotate_image(true).is_err());
        assert_eq!(d.dimensions(), (131, 79));
        assert_eq!(d.history_depth(), depth);
        assert_eq!(d.revision, revision);
        assert_eq!(d.selection, selection);
    }
    #[test]
    fn quarter_turn_cold_history_pixels_selection_and_four_turns() {
        for clockwise in [true, false] {
            let mut d = Document::with_options(
                129,
                77,
                DocumentOptions {
                    max_history_bytes: 2 * TILE_BYTES + 256,
                    max_resident_tiles: 1,
                    ..Default::default()
                },
            )
            .unwrap();
            d.initialize_white_background().unwrap();
            let mut pixels = vec![Pixel::WHITE; 4096];
            pixels[65] = Pixel::TRANSPARENT;
            pixels[100] = Pixel::from_straight([0.2, 0.4, 0.8, 0.7]).unwrap();
            let tile = Arc::new(Tile::from_pixels(pixels).unwrap());
            d.pool.attach(&tile).unwrap();
            d.layers[0].tiles.insert(TileCoord::from_signed(0, 0), tile);
            d.set_selection(crate::Selection::all(129, 77)).unwrap();
            let original = d.layers.clone();
            let selection = d.selection.clone();
            let depth = d.history_depth().0;
            d.rotate_image(clockwise).unwrap();
            assert_eq!(d.dimensions(), (77, 129));
            assert!(matches!(
                d.history.undo.back(),
                Some(StoredCommand::Paged(..))
            ));
            for y in 0..77 {
                for x in 0..129 {
                    let (nx, ny) = if clockwise { (76 - y, x) } else { (y, 128 - x) };
                    assert_eq!(
                        d.pixel(nx, ny),
                        SourceSampler::new(&original[0])
                            .pixel(x as i64, y as i64)
                            .unwrap()
                    );
                }
            }
            d.select_history(depth).unwrap();
            assert_eq!(d.dimensions(), (129, 77));
            assert_eq!(d.selection, selection);
            d.redo().unwrap();
            assert_eq!(d.dimensions(), (77, 129));
            for _ in 0..3 {
                d.rotate_image(clockwise).unwrap();
            }
            assert_eq!(d.dimensions(), (129, 77));
            assert_eq!(d.selection, selection);
            for y in 0..77 {
                for x in 0..129 {
                    assert_eq!(
                        d.pixel(x, y),
                        SourceSampler::new(&original[0])
                            .pixel(x as i64, y as i64)
                            .unwrap()
                    );
                }
            }
        }
    }
}
