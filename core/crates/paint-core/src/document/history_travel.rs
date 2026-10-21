//! Travel privately across retained commands; publish only the final document.
use super::*;

impl Document {
    /// Depth is relative to the retained boundary, including a truncated boundary.
    /// Pixel pages stay shared. Only one command is decoded at a time.
    pub fn select_history(&mut self, depth: usize) -> Result<bool> {
        self.idle()?;
        let (undo, redo) = self.history_depth();
        if depth > undo + redo {
            return Err(Error::InvalidArgument("history depth"));
        }
        if depth == undo {
            return Ok(false);
        }
        let layers = self.layers.clone();
        let selection = self.selection.clone();
        let active = self.active_layer;
        let revision = self.revision;
        let dirty = self.dirty.clone();
        let forward = depth > undo;
        let steps = depth.abs_diff(undo);
        // Leave both history stacks intact until every payload has been checked.
        for index in 0..steps {
            let stored = if forward {
                &self.history.redo[redo - 1 - index]
            } else {
                &self.history.undo[undo - 1 - index]
            };
            let command = match stored.materialize_in(Some(&self.pool)) {
                Ok(command) => command,
                Err(error) => {
                    self.layers = layers;
                    self.selection = selection;
                    self.selection_cache.clear();
                    self.active_layer = active;
                    self.revision = revision;
                    self.dirty = dirty;
                    return Err(error);
                }
            };
            self.apply_history(&command, forward);
        }
        for _ in 0..steps {
            if forward {
                let stored = self.history.redo.pop().expect("validated history depth");
                self.history.undo.push_back(stored);
            } else {
                let stored = self
                    .history
                    .undo
                    .pop_back()
                    .expect("validated history depth");
                self.history.redo.push(stored);
            }
        }
        // apply_history accumulated the correct dirty union, including selection-only.
        self.revision = revision.saturating_add(1);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn paint(doc: &mut Document, x: f64) {
        doc.begin_stroke(
            Brush {
                radius: 96.,
                ..Default::default()
            },
            InputPoint::new(x, 16., 1.),
        )
        .unwrap();
        doc.end_stroke().unwrap();
    }
    fn image(doc: &Document) -> Vec<Pixel> {
        (0..doc.height)
            .flat_map(|y| (0..doc.width).map(move |x| doc.pixel(x, y)))
            .collect()
    }
    fn populate(doc: &mut Document) {
        paint(doc, 8.);
        doc.set_selection(crate::Selection::all(doc.width, doc.height))
            .unwrap();
        let layer = doc.add_layer("another").unwrap();
        paint(doc, 24.);
        doc.add_mask(layer).unwrap();
        doc.set_layer_appearance(
            layer,
            doc.layer(layer)
                .unwrap()
                .appearance
                .translated(3, -2)
                .unwrap(),
        )
        .unwrap();
        doc.group_layer(layer, "group").unwrap();
        doc.remove_layer(layer).unwrap();
        doc.set_selection(crate::Selection::default()).unwrap();
        doc.set_active_layer(1).unwrap();
        paint(doc, 140.);
    }
    #[test]
    fn direct_travel_matches_steps_keeps_redo_and_publishes_one_revision() {
        for encoded in [false, true] {
            let options = DocumentOptions {
                max_history_bytes: if encoded {
                    2 * TILE_BYTES + 256
                } else {
                    64 * 1024 * 1024
                },
                ..Default::default()
            };
            let mut direct = Document::with_options(192, 128, options).unwrap();
            let mut stepped = Document::with_options(192, 128, options).unwrap();
            populate(&mut direct);
            populate(&mut stepped);
            let total = direct.history_depth().0;
            let actions = direct.history_actions();
            for target in [0, total, 2, total - 1, 1, total] {
                let old_depth = direct.history_depth().0;
                let revision = direct.revision();
                assert_eq!(direct.select_history(target).unwrap(), old_depth != target);
                assert_eq!(direct.revision(), revision + u64::from(old_depth != target));
                while stepped.history_depth().0 > target {
                    stepped.undo().unwrap();
                }
                while stepped.history_depth().0 < target {
                    stepped.redo().unwrap();
                }
                assert_eq!(image(&direct), image(&stepped));
                assert_eq!(direct.layers.len(), stepped.layers.len());
                for (a, b) in direct.layers.iter().zip(&stepped.layers) {
                    assert_eq!(
                        (a.id, a.parent, a.mask, a.group, a.clipped),
                        (b.id, b.parent, b.mask, b.group, b.clipped)
                    );
                    assert_eq!(a.name, b.name);
                    assert_eq!(a.properties, b.properties);
                    assert_eq!(a.appearance, b.appearance);
                    assert_eq!(
                        a.tiles.keys().collect::<Vec<_>>(),
                        b.tiles.keys().collect::<Vec<_>>()
                    );
                    for (coord, tile) in &a.tiles {
                        assert_eq!(
                            &*tile.try_pixels().unwrap(),
                            &*b.tiles[coord].try_pixels().unwrap()
                        );
                    }
                }
                assert_eq!(direct.selection(), stepped.selection());
                assert_eq!(direct.active_layer(), stepped.active_layer());
                assert_eq!(direct.history_depth(), stepped.history_depth());
                assert_eq!(direct.history_actions(), actions);
            }
            let revision = direct.revision();
            assert!(direct.select_history(total + 1).is_err());
            assert_eq!(direct.revision(), revision);
            direct.select_history(1).unwrap();
            paint(&mut direct, 50.);
            assert_eq!(direct.history_depth(), (2, 0));
        }
    }
    #[test]
    fn corrupt_payload_after_a_successful_private_step_rolls_back_everything() {
        let mut doc = Document::with_options(
            192,
            128,
            DocumentOptions {
                max_history_bytes: 2 * TILE_BYTES + 256,
                ..Default::default()
            },
        )
        .unwrap();
        paint(&mut doc, 8.);
        doc.set_selection(crate::Selection::all(doc.width, doc.height))
            .unwrap();
        let before = image(&doc);
        let revision = doc.revision();
        let depth = doc.history_depth();
        let selection = doc.selection.clone();
        let dirty = doc.dirty.clone();
        if let StoredCommand::Encoded(payload, _) = &mut doc.history.undo[0] {
            payload.truncate_for_test();
        } else {
            panic!("expected encoded stroke");
        }
        assert!(doc.select_history(0).is_err());
        assert_eq!(image(&doc), before);
        assert_eq!(doc.revision(), revision);
        assert_eq!(doc.history_depth(), depth);
        assert_eq!(doc.selection, selection);
        assert_eq!(doc.dirty, dirty);
    }
}

#[cfg(test)]
mod transform_tests {
    use super::*;
    #[test]
    fn direct_transform_travel_materializes_only_final_geometry_and_keeps_redo() {
        let mut direct = Document::new(48, 48).unwrap();
        direct
            .begin_stroke(
                Brush {
                    radius: 5.,
                    ..Default::default()
                },
                InputPoint::new(16., 16., 1.),
            )
            .unwrap();
        direct.end_stroke().unwrap();
        let mut stepped = Document::new(48, 48).unwrap();
        stepped
            .begin_stroke(
                Brush {
                    radius: 5.,
                    ..Default::default()
                },
                InputPoint::new(16., 16., 1.),
            )
            .unwrap();
        stepped.end_stroke().unwrap();
        direct.begin_transform().unwrap();
        stepped.begin_transform().unwrap();
        let initial = direct.transform_geometry().unwrap();
        for step in 1..=5 {
            let mut geometry = initial.clone();
            for point in &mut geometry.points {
                point[0] += step as f64 * 2.;
            }
            direct.update_transform(geometry.clone()).unwrap();
            direct.checkpoint_transform().unwrap();
            stepped.update_transform(geometry).unwrap();
            stepped.checkpoint_transform().unwrap();
        }
        for target in [0, 5, 1, 4, 0, 5] {
            let revision = direct.revision();
            assert!(direct.select_transform_history(target).unwrap());
            assert_eq!(direct.revision(), revision + 1);
            while stepped.transform_history_depth().unwrap().0 > target {
                stepped.undo_transform().unwrap();
            }
            while stepped.transform_history_depth().unwrap().0 < target {
                stepped.redo_transform().unwrap();
            }
            assert_eq!(direct.transform_geometry(), stepped.transform_geometry());
            assert_eq!(
                direct.transform_history_depth(),
                stepped.transform_history_depth()
            );
            for y in 0..48 {
                for x in 0..48 {
                    assert_eq!(direct.pixel(x, y), stepped.pixel(x, y));
                }
            }
        }
        let revision = direct.revision();
        assert!(!direct.select_transform_history(5).unwrap());
        assert!(direct.select_transform_history(6).is_err());
        assert_eq!(direct.revision(), revision);
    }
}
