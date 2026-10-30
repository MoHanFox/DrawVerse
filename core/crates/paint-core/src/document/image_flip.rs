//! Exact integer reflection uses transform source sampling and the same atomic layer history.
use super::*;

impl Document {
    pub fn flip_image(&mut self, horizontal: bool) -> Result<()> {
        self.idle()?;
        let axis = usize::from(!horizontal);
        let extent = if horizontal { self.width } else { self.height } as i64;
        let mut after = self.layers.clone();
        // Mirror local pixels around the implicit-white extent, keeping that default rectangle.
        // Empty transparent layers have extent zero; document offset carries the reflection.
        for layer in &mut after {
            if layer.group {
                continue;
            }
            let size = if horizontal {
                layer.white.0
            } else {
                layer.white.1
            } as i64;
            let old = if horizontal {
                layer.appearance.offset_x
            } else {
                layer.appearance.offset_y
            } as i64;
            // Raster masks also store absolute canvas offsets, just like their owner.
            let offset = extent - size - old;
            let offset =
                i32::try_from(offset).map_err(|_| Error::InvalidArgument("image flip position"))?;
            if horizontal {
                layer.appearance.offset_x = offset;
            } else {
                layer.appearance.offset_y = offset;
            }
            layer.appearance.validate()?;
            if let Some(image) = &mut layer.placed {
                let image = Arc::make_mut(image);
                for p in &mut image.geometry.points {
                    p[axis] = size as f64 - p[axis];
                }
                image.geometry.validate()?;
            }
        }
        let mut count = after
            .iter()
            .map(|l| l.placed.as_ref().map_or(0, |p| p.source.tiles.len()))
            .sum::<usize>();
        for layer in &mut after {
            if layer.group {
                continue;
            }
            let original = self.layer(layer.id).expect("target");
            let size = if horizontal {
                original.white.0
            } else {
                original.white.1
            } as i64;
            self.progress_stage(ComputationPhase::Preparing, 0);
            let mut keys = BTreeSet::new();
            for key in original.tiles.keys() {
                let x = key.signed_x() as i64 * 64;
                let y = key.signed_y() as i64 * 64;
                let (x, y) = if horizontal {
                    (size - x - 64, y)
                } else {
                    (x, size - y - 64)
                };
                for ty in y.div_euclid(64)..=(y + 63).div_euclid(64) {
                    for tx in x.div_euclid(64)..=(x + 63).div_euclid(64) {
                        let tx = i32::try_from(tx)
                            .map_err(|_| Error::ResourceLimit("image flip tile coordinates"))?;
                        let ty = i32::try_from(ty)
                            .map_err(|_| Error::ResourceLimit("image flip tile coordinates"))?;
                        keys.insert(TileCoord::from_signed(tx, ty));
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
                    let x = key.signed_x() as i64 * 64 + (i % 64) as i64;
                    let y = key.signed_y() as i64 * 64 + (i / 64) as i64;
                    let (sx, sy) = if horizontal {
                        (size - 1 - x, y)
                    } else {
                        (x, size - 1 - y)
                    };
                    let p = sampler.pixel(sx, sy)?;
                    changed |= p != layer.default_pixel(x, y);
                    pixels.push(p);
                }
                if changed {
                    count += 1;
                    if count > self.options.max_document_tiles {
                        return Err(Error::ResourceLimit("image flip tiles"));
                    }
                    let tile = Arc::new(Tile::from_pixels(pixels)?);
                    self.pool.attach(&tile)?;
                    layer.tiles.insert(key, tile);
                    self.pool.trim()?;
                }
                self.progress_advance(1);
            }
        }
        self.progress_stage(ComputationPhase::Selection, 0);
        let selection = self
            .selection
            .reflected(self.width, self.height, horizontal)?;
        self.progress_stage(ComputationPhase::History, 0);
        self.commit_with_action(
            Command::LayerEdit {
                before: self.layers.clone(),
                after: after.clone(),
                selection: Some((self.selection.clone(), selection.clone())),
            },
            if horizontal {
                crate::HistoryAction::FlipHorizontal
            } else {
                crate::HistoryAction::FlipVertical
            },
        )?;
        self.layers = after;
        self.selection = selection;
        self.selection_cache.clear();
        self.dirty_all();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quarter_turn_nested_masks_hidden_locked_and_offsets() {
        for clockwise in [true, false] {
            let mut d = patterned();
            let before = pixels(&d);
            let selection = d.selection.clone();
            let depth = d.history_depth().0;
            let (w, h) = d.dimensions();
            d.rotate_image(clockwise).unwrap();
            for y in 0..h {
                for x in 0..w {
                    let (nx, ny) = if clockwise {
                        (h - 1 - y, x)
                    } else {
                        (y, w - 1 - x)
                    };
                    let expected = before[(y * w + x) as usize];
                    let actual = d.pixel(nx, ny);
                    assert_eq!(actual, expected, "at {x},{y} clockwise={clockwise}");
                }
            }
            d.select_history(depth).unwrap();
            assert_eq!(pixels(&d), before);
            assert_eq!(d.selection, selection);
            d.redo().unwrap();
            for _ in 0..3 {
                d.rotate_image(clockwise).unwrap();
            }
            assert_eq!(pixels(&d), before);
        }
    }
    #[test]
    fn quarter_turn_preserves_placed_source_geometry() {
        let mut source = Document::new(16, 16).unwrap();
        source
            .begin_stroke(
                Brush {
                    radius: 3.,
                    ..Default::default()
                },
                InputPoint::new(4., 6., 1.),
            )
            .unwrap();
        source.end_stroke().unwrap();
        let mut d = Document::new(129, 77).unwrap();
        let id = d
            .place_image(source.layers[0].clone(), 16, 16, "placed".into())
            .unwrap();
        d.commit_transform().unwrap();
        let source = d
            .layer(id)
            .unwrap()
            .placed
            .as_ref()
            .unwrap()
            .source
            .tiles
            .values()
            .next()
            .unwrap()
            .clone();
        let before = pixels(&d);
        for _ in 0..4 {
            d.rotate_image(true).unwrap();
        }
        assert_eq!(pixels(&d), before);
        assert!(Arc::ptr_eq(
            &source,
            d.layer(id)
                .unwrap()
                .placed
                .as_ref()
                .unwrap()
                .source
                .tiles
                .values()
                .next()
                .unwrap()
        ));
    }
    fn pixels(d: &Document) -> Vec<Pixel> {
        (0..d.height)
            .flat_map(|y| (0..d.width).map(move |x| d.pixel(x, y)))
            .collect()
    }
    fn patterned() -> Document {
        let mut d = Document::new(129, 77).unwrap();
        d.initialize_white_background().unwrap();
        let mut p = vec![Pixel::WHITE; 4096];
        p[1] = Pixel::TRANSPARENT;
        p[2] = Pixel::from_straight([0.9, 0.2, 0.4, 0.5]).unwrap();
        d.layers[0].tiles.insert(
            TileCoord::from_signed(0, 0),
            Arc::new(Tile::from_pixels(p).unwrap()),
        );
        let owner = d.add_layer("upper").unwrap();
        d.begin_stroke(
            Brush {
                radius: 4.,
                color: Pixel::from_straight([0.1, 0.8, 0.3, 0.7]).unwrap(),
                ..Default::default()
            },
            InputPoint::new(30., 21., 1.),
        )
        .unwrap();
        d.end_stroke().unwrap();
        let mask = d.add_mask(owner).unwrap();
        let at = d.layer_index(mask).unwrap();
        d.layers[at].appearance.offset_x = 5;
        d.layers[at].appearance.offset_y = -2;
        let mask_pixels = (0..4096)
            .map(|i| Pixel::from_straight([0., 0., 0., (i % 64) as f32 / 63.]).unwrap())
            .collect();
        d.layers[at].tiles.insert(
            TileCoord::from_signed(0, 0),
            Arc::new(Tile::from_pixels(mask_pixels).unwrap()),
        );
        let group = d.group_layer(owner, "group").unwrap();
        let at = d.layer_index(owner).unwrap();
        d.layers[at].appearance.offset_x = -3;
        let hidden = d.add_layer("hidden").unwrap();
        let at = d.layer_index(hidden).unwrap();
        d.layers[at].properties.visible = false;
        let p = vec![Pixel::from_straight([0.4, 0.5, 0.6, 0.8]).unwrap(); 4096];
        d.layers[at].tiles.insert(
            TileCoord::from_signed(-1, 1),
            Arc::new(Tile::from_pixels(p).unwrap()),
        );
        let mut shape =
            crate::SelectionShape::geometry(crate::SelectionKind::Mask, 2., 3., 3., 2., false);
        shape.mask =
            Some(crate::SelectionMask::from_alpha(3, 2, vec![0, 10, 255, 90, 130, 200]).unwrap());
        let polygon =
            crate::SelectionShape::polygon(vec![[4., 5.], [19., 7.], [12., 30.]], true, 2.)
                .unwrap();
        let group_mask = d.add_mask(group).unwrap();
        let at = d.layer_index(group_mask).unwrap();
        let mask_pixels = (0..4096)
            .map(|i| Pixel::from_straight([0., 0., 0., (i / 64) as f32 / 127.]).unwrap())
            .collect();
        d.layers[at].tiles.insert(
            TileCoord::from_signed(0, 0),
            Arc::new(Tile::from_pixels(mask_pixels).unwrap()),
        );
        let at = d.layer_index(group).unwrap();
        d.layers[at].appearance.locks = crate::LOCK_ALL;
        d.set_selection(
            crate::Selection::from_steps(
                true,
                vec![
                    crate::SelectionStep {
                        operation: crate::SelectionOperation::Replace,
                        shape: Some(shape),
                    },
                    crate::SelectionStep {
                        operation: crate::SelectionOperation::Add,
                        shape: Some(polygon),
                    },
                ],
            )
            .unwrap(),
        )
        .unwrap();
        d
    }
    #[test]
    fn image_flip_all_layers_masks_selection_and_history_are_exact() {
        for horizontal in [true, false] {
            let mut d = patterned();
            let before = d.layers.clone();
            let original = pixels(&d);
            let selection = d.selection.clone();
            let depth = d.history_depth();
            let active = d.active_layer();
            d.flip_image(horizontal).unwrap();
            assert_eq!(
                *d.history_actions().last().unwrap(),
                if horizontal {
                    crate::HistoryAction::FlipHorizontal
                } else {
                    crate::HistoryAction::FlipVertical
                }
            );
            assert_eq!(d.history_depth().0, depth.0 + 1);
            assert_eq!(d.active_layer(), active);
            for y in 0..d.height {
                for x in 0..d.width {
                    let (sx, sy) = if horizontal {
                        (d.width - 1 - x, y)
                    } else {
                        (x, d.height - 1 - y)
                    };
                    assert_eq!(
                        d.pixel(x, y),
                        original[(sy * d.width + sx) as usize],
                        "{horizontal}: {x},{y}"
                    );
                    assert!(
                        (d.selection.coverage(x as f64 + 0.5, y as f64 + 0.5)
                            - selection.coverage(sx as f64 + 0.5, sy as f64 + 0.5))
                        .abs()
                            < 1e-6
                    );
                }
            }
            // Hidden and off-canvas pixels also move; locks/names/tree/default white survive.
            for layer in &d.layers {
                let old = before.iter().find(|l| l.id == layer.id).unwrap();
                assert_eq!(layer.name, old.name);
                assert_eq!(layer.parent, old.parent);
                assert_eq!(layer.white, old.white);
                assert_eq!(layer.appearance.locks, old.appearance.locks);
                if !layer.group {
                    let mut actual = SourceSampler::new(layer);
                    let mut expected = SourceSampler::new(old);
                    for y in -70..150 {
                        for x in -70..200 {
                            let (sx, sy) = if horizontal {
                                (d.width as i64 - 1 - x, y)
                            } else {
                                (x, d.height as i64 - 1 - y)
                            };
                            let a = actual
                                .pixel(
                                    x - layer.appearance.offset_x as i64,
                                    y - layer.appearance.offset_y as i64,
                                )
                                .unwrap();
                            let b = expected
                                .pixel(
                                    sx - old.appearance.offset_x as i64,
                                    sy - old.appearance.offset_y as i64,
                                )
                                .unwrap();
                            assert_eq!(a, b);
                        }
                    }
                }
            }
            let flipped = pixels(&d);
            d.undo().unwrap();
            assert_eq!(pixels(&d), original);
            assert_eq!(d.selection, selection);
            d.redo().unwrap();
            assert_eq!(pixels(&d), flipped);
            d.flip_image(horizontal).unwrap();
            assert_eq!(pixels(&d), original);
            assert_eq!(d.selection, selection);
        }
    }
    #[test]
    fn image_flip_keeps_placed_source_and_rolls_back_resource_failure() {
        let mut source = Document::new(16, 16).unwrap();
        source
            .begin_stroke(
                Brush {
                    radius: 3.,
                    ..Default::default()
                },
                InputPoint::new(4., 6., 1.),
            )
            .unwrap();
        source.end_stroke().unwrap();
        let mut d = Document::new(129, 77).unwrap();
        let id = d
            .place_image(source.layers[0].clone(), 16, 16, "placed".into())
            .unwrap();
        d.commit_transform().unwrap();
        let source = d
            .layer(id)
            .unwrap()
            .placed
            .as_ref()
            .unwrap()
            .source
            .tiles
            .values()
            .next()
            .unwrap()
            .clone();
        let before = pixels(&d);
        d.flip_image(true).unwrap();
        assert!(Arc::ptr_eq(
            &source,
            d.layer(id)
                .unwrap()
                .placed
                .as_ref()
                .unwrap()
                .source
                .tiles
                .values()
                .next()
                .unwrap()
        ));
        d.begin_transform().unwrap();
        let g = d.transform_geometry().unwrap();
        d.update_transform(g).unwrap();
        d.commit_transform().unwrap();
        d.flip_image(true).unwrap();
        assert_eq!(pixels(&d), before);
        assert!(d.layer(id).unwrap().is_image());
        let mut d = patterned();
        let pixels = pixels(&d);
        let selection = d.selection.clone();
        let rev = d.revision();
        let depth = d.history_depth();
        let tile = d.layers[0].tiles.values().next().unwrap().clone();
        d.options.max_document_tiles = 0;
        assert!(matches!(d.flip_image(true), Err(Error::ResourceLimit(_))));
        assert_eq!(d.revision(), rev);
        assert_eq!(d.history_depth(), depth);
        assert_eq!(d.selection, selection);
        assert_eq!(super::tests::pixels(&d), pixels);
        assert!(Arc::ptr_eq(
            &tile,
            d.layers[0].tiles.values().next().unwrap()
        ));
    }
    #[test]
    fn image_flip_matches_free_transform_nearest_reflection() {
        let mut source = Document::new(16, 16).unwrap();
        source.initialize_white_background().unwrap();
        source
            .begin_stroke(
                Brush {
                    radius: 2.,
                    ..Default::default()
                },
                InputPoint::new(4., 6., 1.),
            )
            .unwrap();
        source.end_stroke().unwrap();
        let bytes = source.embedded_layers().unwrap();
        for horizontal in [true, false] {
            let mut transformed = Document::from_embedded_layers(
                16,
                16,
                &bytes,
                1,
                DocumentOptions::default(),
                paint_storage::ScratchSpace::system(),
            )
            .unwrap();
            transformed.begin_transform().unwrap();
            let mut g = transformed.transform_geometry().unwrap();
            let axis = usize::from(!horizontal);
            for p in &mut g.points {
                p[axis] = 16. - p[axis];
            }
            g.interpolation = 0;
            transformed.update_transform(g).unwrap();
            transformed.commit_transform().unwrap();
            let mut direct = Document::from_embedded_layers(
                16,
                16,
                &bytes,
                1,
                DocumentOptions::default(),
                paint_storage::ScratchSpace::system(),
            )
            .unwrap();
            direct.flip_image(horizontal).unwrap();
            assert_eq!(pixels(&direct), pixels(&transformed));
        }
    }
}
