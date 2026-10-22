//! Integer translation previews share immutable pixels and the resident presentation mesh.
use super::*;

#[derive(Debug)]
pub(super) struct LayerMoveState {
    ids: Vec<LayerId>,
    baseline: Vec<(usize, crate::LayerAppearance)>,
    delta: (i32, i32),
    source: Option<Arc<ResidentPreviewSource>>,
}

impl Document {
    pub fn begin_layer_move(&mut self, id: LayerId, deferred: bool) -> Result<()> {
        self.begin_layer_moves(&[id], deferred)
    }

    fn layer_move_baseline(&self, ids: &[LayerId]) -> Result<Vec<(usize, crate::LayerAppearance)>> {
        let mut baseline = Vec::new();
        for id in self.batch_roots(ids)? {
            self.check_ancestors(id, crate::LOCK_POSITION | crate::LOCK_ALL)?;
            let at = self.layer_index(id)?;
            let range = if self.layers[at].group || self.mask_for(id).is_some() {
                self.subtree(id)?
            } else {
                at..at + 1
            };
            for i in range {
                let layer = &self.layers[i];
                if layer.mask && !layer.mask_linked && !ids.contains(&layer.id) {
                    continue;
                }
                self.check_ancestors(self.layers[i].id, crate::LOCK_POSITION | crate::LOCK_ALL)?;
                if !self.layers[i].group {
                    baseline.push((i, self.layers[i].appearance));
                }
            }
        }
        Ok(baseline)
    }

    pub fn begin_layer_moves(&mut self, ids: &[LayerId], deferred: bool) -> Result<()> {
        self.idle()?;
        let baseline = self.layer_move_baseline(ids)?;
        self.layer_move = Some(LayerMoveState {
            ids: ids.to_vec(),
            baseline,
            delta: (0, 0),
            source: if deferred {
                self.resident_move_source(ids)
            } else {
                None
            },
        });
        self.dirty_all();
        Ok(())
    }

    pub fn layer_move_active(&self) -> bool {
        self.layer_move.is_some()
    }
    pub fn layer_move_deferred(&self) -> bool {
        self.layer_move.as_ref().is_some_and(|s| s.source.is_some())
    }

    /// Absolute delta from the captured baseline. Validation precedes every mutation.
    pub fn update_layer_move(&mut self, dx: i32, dy: i32) -> Result<()> {
        let state = self.layer_move.as_ref().ok_or(Error::Busy)?;
        if state.delta == (dx, dy) {
            return Ok(());
        }
        let changes = state
            .baseline
            .iter()
            .map(|&(i, appearance)| Ok((i, appearance.translated(dx, dy)?)))
            .collect::<Result<Vec<_>>>()?;
        for (i, appearance) in changes {
            self.layers[i].appearance = appearance;
        }
        self.layer_move.as_mut().expect("state").delta = (dx, dy);
        self.dirty_all();
        Ok(())
    }

    pub fn cancel_layer_move(&mut self) -> Result<()> {
        let state = self.layer_move.take().ok_or(Error::Busy)?;
        for (i, appearance) in state.baseline {
            self.layers[i].appearance = appearance;
        }
        self.dirty_all();
        Ok(())
    }

    /// Final coordinates are part of the barrier, even when previews were coalesced.
    pub fn commit_layer_move(&mut self, dx: i32, dy: i32) -> Result<()> {
        if let Err(error) = self.update_layer_move(dx, dy) {
            self.cancel_layer_move()?;
            return Err(error);
        }
        let state = self.layer_move.take().expect("state");
        for (i, appearance) in state.baseline {
            self.layers[i].appearance = appearance;
        }
        let result = self.move_layers(&state.ids, dx, dy);
        // A zero-distance completion still publishes a final frame for handoff.
        if dx == 0 && dy == 0 || result.is_err() {
            self.dirty_all();
        }
        result
    }

    /// Translate all normalized roots with one metadata-only history transaction.
    pub fn move_layers(&mut self, ids: &[LayerId], dx: i32, dy: i32) -> Result<()> {
        self.idle()?;
        let baseline = self.layer_move_baseline(ids)?;
        let changes = baseline
            .iter()
            .map(|&(i, a)| Ok((i, a.translated(dx, dy)?)))
            .collect::<Result<Vec<_>>>()?;
        if (dx == 0 && dy == 0) || changes.is_empty() {
            return Ok(());
        }
        let mut nodes = self.structure();
        for (i, a) in changes {
            nodes[i].appearance = a;
        }
        self.commit_structure_entry(nodes, crate::HistoryAction::MoveLayer)
    }

    /// Common resident packet: deformation uses a bounded grid, translation four vertices.
    pub fn resident_preview(&self) -> Option<ResidentPreview> {
        if let Some(state) = self.layer_move.as_ref() {
            let source = state.source.as_ref()?.clone();
            let mut vertices = Vec::with_capacity(4);
            for y in [0., self.height as f32] {
                for x in [0., self.width as f32] {
                    vertices.push([
                        x,
                        y,
                        (x - state.delta.0 as f32 - source.bounds[0] as f32)
                            / source.bounds[2] as f32,
                        (y - state.delta.1 as f32 - source.bounds[1] as f32)
                            / source.bounds[3] as f32,
                    ]);
                }
            }
            return Some(ResidentPreview {
                source,
                revision: self.revision,
                focus: false,
                forward: false,
                columns: 2,
                rows: 2,
                vertices,
            });
        }
        self.transform_preview().or_else(|| self.liquify_preview())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> Document {
        let mut d = Document::new(128, 96).unwrap();
        // Include a retained tile outside the visible canvas.
        d.layers[0].tiles.insert(
            TileCoord::from_signed(-1, 0),
            Arc::new(
                Tile::from_pixels(vec![Pixel::from_straight([1., 0., 0., 1.]).unwrap(); 4096])
                    .unwrap(),
            ),
        );
        d
    }

    #[test]
    fn layer_moves_normalize_overlaps_and_commit_one_history() {
        let mut d = seed();
        let a = d.active_layer();
        let b = d.add_layer("other").unwrap();
        let mask = d.add_mask(a).unwrap();
        let group = d.group_layer(a, "group").unwrap();
        let ids = [group, a, mask, b];
        let depth = d.history_depth();
        let tiles = d.layer(a).unwrap().tiles.clone();
        d.begin_layer_moves(&ids, true).unwrap();
        assert!(d.layer_move_deferred());
        for (x, y) in [(3, 5), (-18, 12), (0, 0), (9, -7)] {
            d.update_layer_move(x, y).unwrap();
            for id in [a, mask, b] {
                let a = d.layer(id).unwrap().appearance;
                assert_eq!((a.offset_x, a.offset_y), (x, y));
            }
            assert_eq!(d.history_depth(), depth);
        }
        d.cancel_layer_move().unwrap();
        for id in [a, mask, b] {
            assert_eq!(d.layer(id).unwrap().appearance.offset_x, 0);
        }
        d.begin_layer_moves(&ids, true).unwrap();
        d.update_layer_move(20, 1).unwrap();
        d.commit_layer_move(11, -4).unwrap();
        assert_eq!(d.history_depth().0, depth.0 + 1);
        for id in [a, mask, b] {
            assert_eq!(d.layer(id).unwrap().appearance.offset_x, 11);
        }
        d.undo().unwrap();
        for id in [a, mask, b] {
            assert_eq!(d.layer(id).unwrap().appearance.offset_x, 0);
        }
        d.redo().unwrap();
        for id in [a, mask, b] {
            assert_eq!(d.layer(id).unwrap().appearance.offset_y, -4);
        }
        for (key, tile) in tiles {
            assert!(Arc::ptr_eq(&tile, &d.layer(a).unwrap().tiles[&key]));
        }
        let depth = d.history_depth();
        d.begin_layer_moves(&ids, false).unwrap();
        d.update_layer_move(5, 5).unwrap();
        d.commit_layer_move(0, 0).unwrap();
        assert_eq!(d.history_depth(), depth);
    }
    #[test]
    fn layer_moves_validate_every_target_before_changes() {
        let mut d = seed();
        let a = d.active_layer();
        let b = d.add_layer("other").unwrap();
        let mut locked = d.layer(b).unwrap().appearance;
        locked.locks = crate::LOCK_POSITION;
        d.set_layer_appearance(b, locked).unwrap();
        let depth = d.history_depth();
        assert!(d.begin_layer_moves(&[a, b], false).is_err());
        assert!(!d.layer_move_active());
        assert!(d.move_layers(&[a, b], 5, 2).is_err());
        assert_eq!(d.layer(a).unwrap().appearance.offset_x, 0);
        assert_eq!(d.history_depth(), depth);
        locked.locks = 0;
        d.set_layer_appearance(b, locked).unwrap();
        assert!(d.begin_layer_moves(&[a, a], false).is_err());
        assert!(d.begin_layer_moves(&[a, 9999], false).is_err());
        assert!(d.begin_layer_moves(&[], false).is_err());
        let depth = d.history_depth();
        d.begin_layer_moves(&[a, b], false).unwrap();
        d.update_layer_move(3, 2).unwrap();
        assert!(d.commit_layer_move(i32::MAX, 2).is_err());
        assert!(!d.layer_move_active());
        assert_eq!(d.history_depth(), depth);
        for id in [a, b] {
            assert_eq!(d.layer(id).unwrap().appearance.offset_x, 0);
        }
    }
    #[test]
    fn layer_move_absolute_preview_cancel_commit_and_history() {
        for deferred in [false, true] {
            let mut d = seed();
            let id = d.active_layer();
            let depth = d.history_depth();
            let tiles = d.layers[0].tiles.clone();
            d.begin_layer_move(id, deferred).unwrap();
            let source = d.resident_preview().map(|p| p.source);
            assert_eq!(d.layer_move_deferred(), deferred);
            d.update_layer_move(64, 0).unwrap();
            assert_eq!(d.pixel(10, 10).alpha(), 1.);
            d.update_layer_move(0, 0).unwrap();
            assert_eq!(d.pixel(10, 10).alpha(), 0.);
            for (x, y) in [(140, 2), (-80, -3), (12, 5)] {
                d.update_layer_move(x, y).unwrap();
                assert_eq!(d.layer(id).unwrap().appearance.offset_x, x);
                assert_eq!(d.history_depth(), depth);
                if let Some(source) = &source {
                    let packet = d.resident_preview().unwrap();
                    assert!(Arc::ptr_eq(source, &packet.source));
                    assert_eq!(packet.vertices.len(), 4);
                    assert!(source.bounds[0] < 0);
                    let uv = packet.vertices[0][2];
                    assert_eq!(
                        uv,
                        (-x as f32 - source.bounds[0] as f32) / source.bounds[2] as f32
                    );
                }
            }
            assert!(d.move_layer(id, 1, 1).is_err());
            d.cancel_layer_move().unwrap();
            assert_eq!(d.layer(id).unwrap().appearance.offset_x, 0);
            assert_eq!(d.history_depth(), depth);
            d.begin_layer_move(id, deferred).unwrap();
            d.update_layer_move(-140, 30).unwrap();
            d.commit_layer_move(20, -7).unwrap();
            assert_eq!(d.history_depth().0, depth.0 + 1);
            assert_eq!(d.layer(id).unwrap().appearance.offset_x, 20);
            d.undo().unwrap();
            assert_eq!(d.layer(id).unwrap().appearance.offset_x, 0);
            d.redo().unwrap();
            assert_eq!(d.layer(id).unwrap().appearance.offset_y, -7);
            for (key, tile) in &tiles {
                assert!(Arc::ptr_eq(tile, &d.layers[0].tiles[key]));
            }
            let depth = d.history_depth();
            d.begin_layer_move(id, deferred).unwrap();
            d.update_layer_move(5, 3).unwrap();
            d.commit_layer_move(0, 0).unwrap();
            assert_eq!(d.history_depth(), depth);
        }
    }

    #[test]
    fn layer_move_mask_group_locks_and_invalid_final_are_atomic() {
        let mut d = seed();
        let owner = d.active_layer();
        let mask = d.add_mask(owner).unwrap();
        d.begin_layer_move(owner, true).unwrap();
        assert!(d.layer_move_deferred());
        d.update_layer_move(7, -5).unwrap();
        for id in [owner, mask] {
            assert_eq!(d.layer(id).unwrap().appearance.offset_x, 7);
        }
        d.cancel_layer_move().unwrap();
        let group = d.group_layer(owner, "group").unwrap();
        let depth = d.history_depth();
        d.begin_layer_move(group, true).unwrap();
        d.update_layer_move(3, 4).unwrap();
        assert_eq!(d.layer(owner).unwrap().appearance.offset_y, 4);
        assert_eq!(d.layer(mask).unwrap().appearance.offset_y, 4);
        assert!(d.update_layer_move(i32::MAX, 1).is_err());
        assert_eq!(d.layer(owner).unwrap().appearance.offset_x, 3);
        assert!(d.commit_layer_move(i32::MAX, 1).is_err());
        assert!(!d.layer_move_active());
        assert_eq!(d.layer(owner).unwrap().appearance.offset_x, 0);
        assert_eq!(d.history_depth(), depth);
        let mut appearance = d.layer(group).unwrap().appearance;
        appearance.locks = crate::LOCK_POSITION;
        d.set_layer_appearance(group, appearance).unwrap();
        assert!(d.begin_layer_move(owner, true).is_err());
        assert!(!d.layer_move_active());
    }

    #[test]
    fn layer_move_placed_image_retains_model_and_uses_pixel_fallback() {
        let source = seed();
        let mut d = Document::new(128, 96).unwrap();
        let id = d
            .place_image(source.layers[0].clone(), 8, 8, "image".into())
            .unwrap();
        d.commit_transform().unwrap();
        let model = d.layer(id).unwrap().placed.as_ref().unwrap().clone();
        let depth = d.history_depth();
        d.begin_layer_move(id, true).unwrap();
        assert!(!d.layer_move_deferred());
        d.update_layer_move(8, -3).unwrap();
        d.commit_layer_move(9, 4).unwrap();
        assert_eq!(d.history_depth().0, depth.0 + 1);
        assert!(Arc::ptr_eq(
            &model,
            d.layer(id).unwrap().placed.as_ref().unwrap()
        ));
        d.undo().unwrap();
        assert_eq!(d.layer(id).unwrap().appearance.offset_x, 0);
    }
    fn masked_seed() -> (Document, LayerId, LayerId) {
        let mut d = Document::new(128, 96).unwrap();
        let owner = d.active_layer();
        d.layers[0].tiles.insert(
            TileCoord::from_signed(0, 0),
            Arc::new(Tile::from_pixels(vec![Pixel::WHITE; 4096]).unwrap()),
        );
        d.move_layer(owner, 12, 7).unwrap();
        let mask = d.add_mask(owner).unwrap();
        let i = d.layer_index(mask).unwrap();
        let pixels = (0..4096)
            .map(|i| {
                if i % 64 < 30 {
                    Pixel::from_straight([0., 0., 0., 0.75]).unwrap()
                } else {
                    Pixel::TRANSPARENT
                }
            })
            .collect();
        d.layers[i].tiles.insert(
            TileCoord::from_signed(0, 0),
            Arc::new(Tile::from_pixels(pixels).unwrap()),
        );
        d.layers[i].appearance.offset_x = 4;
        d.layers[i].appearance.offset_y = 9;
        (d, owner, mask)
    }
    #[test]
    fn mask_link_history_movement_locks_and_bundle_preserve_independent_coordinates() {
        let (mut d, owner, mask) = masked_seed();
        assert!(d.layer(mask).unwrap().mask_linked());
        assert!(d.set_mask_linked(owner, false).is_err());
        let depth = d.history_depth();
        let tile = d
            .layer(mask)
            .unwrap()
            .tiles
            .values()
            .next()
            .unwrap()
            .clone();
        d.set_mask_linked(mask, false).unwrap();
        assert_eq!(d.history_depth().0, depth.0 + 1);
        d.set_mask_linked(mask, false).unwrap();
        assert_eq!(d.history_depth().0, depth.0 + 1);
        d.undo().unwrap();
        assert!(d.layer(mask).unwrap().mask_linked());
        d.redo().unwrap();
        assert!(!d.layer(mask).unwrap().mask_linked());
        assert!(Arc::ptr_eq(
            &tile,
            d.layer(mask).unwrap().tiles.values().next().unwrap()
        ));
        let mut a = d.layer(mask).unwrap().appearance();
        a.locks = crate::LOCK_POSITION;
        d.set_layer_appearance(mask, a).unwrap();
        d.begin_layer_move(owner, false).unwrap();
        d.update_layer_move(5, 3).unwrap();
        assert_eq!(d.layer(owner).unwrap().appearance.offset_x, 17);
        assert_eq!(d.layer(mask).unwrap().appearance.offset_x, 4);
        d.cancel_layer_move().unwrap();
        d.move_layer(owner, 5, 3).unwrap();
        assert_eq!(d.layer(mask).unwrap().appearance.offset_y, 9);
        assert!(d.move_layers(&[owner, mask], 1, 1).is_err());
        a.locks = 0;
        d.set_layer_appearance(mask, a).unwrap();
        d.move_layer(mask, 2, 1).unwrap();
        assert_eq!(d.layer(owner).unwrap().appearance.offset_x, 17);
        assert_eq!(d.layer(mask).unwrap().appearance.offset_x, 6);
        let group = d.group_layer(owner, "group").unwrap();
        let gm = d.add_mask(group).unwrap();
        d.set_mask_linked(gm, false).unwrap();
        d.move_layer(group, 3, 2).unwrap();
        assert_eq!(d.layer(owner).unwrap().appearance.offset_x, 20);
        assert_eq!(d.layer(mask).unwrap().appearance.offset_x, 6);
        assert_eq!(d.layer(gm).unwrap().appearance.offset_x, 0);
        d.set_mask_linked(mask, true).unwrap();
        d.move_layer(group, 1, 1).unwrap();
        assert_eq!(d.layer(mask).unwrap().appearance.offset_x, 7);
        let bytes = d.embedded_layers().unwrap();
        let restored = Document::from_embedded_layers(
            128,
            96,
            &bytes,
            owner,
            DocumentOptions::default(),
            paint_storage::ScratchSpace::system(),
        )
        .unwrap();
        assert!(restored.layer(mask).unwrap().mask_linked());
        assert!(!restored.layer(gm).unwrap().mask_linked());
        let copied = d.copy_layers(&[group]).unwrap();
        let pasted = d.paste_layers(&copied, group).unwrap();
        assert!(!d.mask_for(pasted).unwrap().mask_linked());
    }
    #[test]
    fn mask_transform_uses_absolute_coordinates_and_respects_link_policy() {
        for linked in [false, true] {
            for mask_only in [false, true] {
                let (mut d, owner, mask) = masked_seed();
                d.set_mask_linked(mask, linked).unwrap();
                let bytes = d.embedded_layers().unwrap();
                let mut expected = Document::from_embedded_layers(
                    128,
                    96,
                    &bytes,
                    owner,
                    DocumentOptions::default(),
                    paint_storage::ScratchSpace::system(),
                )
                .unwrap();
                let target = if mask_only { mask } else { owner };
                expected.move_layer(target, 5, 3).unwrap();
                if mask_only {
                    d.set_active_layer(mask).unwrap();
                    d.begin_transform().unwrap();
                } else {
                    d.begin_transform_layers(&[owner]).unwrap();
                }
                let mut g = d.transform_geometry().unwrap();
                g.interpolation = 0;
                for p in &mut g.points {
                    p[0] += 5.;
                    p[1] += 3.;
                }
                d.update_transform(g).unwrap();
                d.commit_transform().unwrap();
                for y in 0..96 {
                    for x in 0..128 {
                        assert_eq!(
                            d.pixel(x, y),
                            expected.pixel(x, y),
                            "linked={linked} mask_only={mask_only} at {x},{y}"
                        );
                    }
                }
                d.undo().unwrap();
                let before = Document::from_embedded_layers(
                    128,
                    96,
                    &bytes,
                    owner,
                    DocumentOptions::default(),
                    paint_storage::ScratchSpace::system(),
                )
                .unwrap();
                for y in 0..96 {
                    for x in 0..128 {
                        assert_eq!(d.pixel(x, y), before.pixel(x, y));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod compound_tests {
    use super::*;
    fn fill(d: &mut Document, id: u64, color: [f32; 4], offset: (i32, i32)) {
        let at = d.layer_index(id).unwrap();
        d.layers[at].tiles.insert(
            TileCoord::from_signed(0, 0),
            Arc::new(Tile::from_pixels(vec![Pixel::from_straight(color).unwrap(); 4096]).unwrap()),
        );
        d.layers[at].appearance.offset_x = offset.0;
        d.layers[at].appearance.offset_y = offset.1;
    }
    fn seed(kind: u8) -> (Document, Vec<u64>) {
        let mut d = Document::new(64, 64).unwrap();
        let background = d.active_layer();
        fill(&mut d, background, [0.2, 0.4, 0.8, 0.7], (0, 0));
        let a = d.add_layer("moving").unwrap();
        fill(&mut d, a, [1., 0.1, 0., 0.6], (-9, 4));
        let mask = d.add_mask(a).unwrap();
        fill(&mut d, mask, [1., 1., 1., 0.35], (3, -2));
        let mut roots = vec![a];
        if kind >= 4 {
            let inner = d.group_layer(a, "inner context").unwrap();
            let sibling = d.add_layer("stationary sibling").unwrap();
            d.reparent_layer(sibling, inner).unwrap();
            fill(&mut d, sibling, [0.4, 0.2, 0.9, 0.3], (13, -3));
            let outer = d.group_layer(inner, "outer context").unwrap();
            for id in [inner, outer] {
                let at = d.layer_index(id).unwrap();
                d.layers[at].appearance.blend = if kind == 4 {
                    crate::BlendMode::Normal
                } else {
                    crate::BlendMode::PassThrough
                };
            }
        }
        if kind == 1 || kind == 2 {
            let group = d.group_layer(a, "moving group").unwrap();
            let at = d.layer_index(group).unwrap();
            d.layers[at].appearance.blend = if kind == 1 {
                crate::BlendMode::Normal
            } else {
                crate::BlendMode::PassThrough
            };
            d.layers[at].properties.opacity = 0.55;
            let gm = d.add_mask(group).unwrap();
            fill(&mut d, gm, [1., 1., 1., 0.25], (-4, 3));
            d.set_active_layer(a).unwrap();
            let child = d.add_layer("inside").unwrap();
            fill(&mut d, child, [0.3, 1., 0.1, 0.5], (8, 7));
            let at = d.layer_index(child).unwrap();
            if kind == 1 {
                d.layers[at].appearance.blend = crate::BlendMode::Multiply;
                d.layers[at].clipped = true;
            }
            roots = vec![group];
            d.set_active_layer(group).unwrap();
        }
        let partner = d.add_layer("partner").unwrap();
        d.reparent_layer(partner, 0).unwrap();
        fill(&mut d, partner, [0., 1., 0.5, 0.4], (3, -6));
        if kind == 3 {
            roots.push(partner);
        }
        let above = d.add_layer("above").unwrap();
        fill(&mut d, above, [0.8, 0.5, 0.1, 0.2], (0, 0));
        (d, roots)
    }
    fn pixels(d: &Document) -> Vec<Pixel> {
        (0..64)
            .flat_map(|y| d.snapshot().read_row(0, y, 64).unwrap())
            .collect()
    }
    #[test]
    fn compound_move_sources_match_pixels_reuse_tiles_and_commit_one_history() {
        for kind in 0..6 {
            let (mut d, ids) = seed(kind);
            let before = pixels(&d);
            let originals = d.layers.clone();
            let depth = d.history_depth();
            d.begin_layer_moves(&ids, true).unwrap();
            assert!(d.layer_move_deferred(), "kind {kind}");
            let source = d.resident_preview().unwrap().source;
            assert!(source.image.layers.iter().any(|l| l.mask));
            for (dx, dy) in [(0, 0), (9, -4), (-3, 5), (6, 2)] {
                d.update_layer_move(dx, dy).unwrap();
                assert!(Arc::ptr_eq(&source, &d.resident_preview().unwrap().source));
                for layer in &originals {
                    for (key, tile) in &layer.tiles {
                        assert!(Arc::ptr_eq(tile, &d.layer(layer.id).unwrap().tiles[key]));
                    }
                }
                for y in 0..64 {
                    for x in 0..64 {
                        let image = source.image.pixel(
                            (x as i32 - dx - source.bounds[0]) as u32,
                            (y as i32 - dy - source.bounds[1]) as u32,
                        );
                        let split = source
                            .above
                            .pixel(x, y)
                            .over(image.over(source.below.pixel(x, y)));
                        for (a, b) in split
                            .components()
                            .into_iter()
                            .zip(d.pixel(x, y).components())
                        {
                            assert!(
                                (a - b).abs() < 1e-6,
                                "kind {kind}, {dx},{dy} at {x},{y}: {a}/{b}"
                            );
                        }
                    }
                }
                assert_eq!(d.history_depth(), depth);
            }
            d.cancel_layer_move().unwrap();
            assert_eq!(pixels(&d), before);
            d.begin_layer_moves(&ids, true).unwrap();
            d.commit_layer_move(6, 2).unwrap();
            let after = pixels(&d);
            assert_eq!(d.history_depth().0, depth.0 + 1);
            d.undo().unwrap();
            assert_eq!(pixels(&d), before);
            d.redo().unwrap();
            assert_eq!(pixels(&d), after);
        }
    }
    #[test]
    fn compound_move_rejects_fixed_masks_gaps_and_backdrop_dependencies() {
        let (mut d, ids) = seed(0);
        let mask = d.mask_for(ids[0]).unwrap().id;
        let at = d.layer_index(mask).unwrap();
        d.layers[at].mask_linked = false;
        d.begin_layer_moves(&ids, true).unwrap();
        assert!(!d.layer_move_deferred());
        d.cancel_layer_move().unwrap();
        let (mut d, ids) = seed(2);
        let child = d.layers.iter().find(|l| l.name == "inside").unwrap().id;
        let at = d.layer_index(child).unwrap();
        d.layers[at].appearance.blend = crate::BlendMode::Multiply;
        d.begin_layer_moves(&ids, true).unwrap();
        assert!(!d.layer_move_deferred());
        d.cancel_layer_move().unwrap();
        let (mut d, ids) = seed(0);
        let above = d.layers.iter().find(|l| l.name == "above").unwrap().id;
        d.begin_layer_moves(&[ids[0], above], true).unwrap();
        assert!(!d.layer_move_deferred());
        d.cancel_layer_move().unwrap();
        let mask = d.mask_for(ids[0]).unwrap().id;
        d.begin_layer_move(mask, true).unwrap();
        assert!(!d.layer_move_deferred());
        d.cancel_layer_move().unwrap();
        let (mut isolated, roots) = seed(1);
        let child = isolated
            .layers
            .iter()
            .position(|l| l.name == "inside")
            .unwrap();
        isolated.layers[child].appearance.blend = crate::BlendMode::Dissolve;
        isolated.begin_layer_moves(&roots, true).unwrap();
        assert!(!isolated.layer_move_deferred());
        let above = d.layer_index(above).unwrap();
        d.layers[above].clipped = true;
        d.begin_layer_moves(&ids, true).unwrap();
        assert!(!d.layer_move_deferred());
    }

    #[test]
    fn nested_move_keeps_ancestor_masks_opacity_and_backdrop_dependencies_on_cpu() {
        for kind in 0..5 {
            let (mut d, ids) = seed(4);
            let parent = d.layer(ids[0]).unwrap().parent;
            let at = d.layer_index(parent).unwrap();
            match kind {
                0 => d.layers[at].properties.opacity = 0.5,
                1 => d.layers[at].appearance.fill = 0.5,
                2 => {
                    let mask = d.add_mask(parent).unwrap();
                    fill(&mut d, mask, [0., 0., 0., 0.5], (0, 0));
                }
                3 => d.layers[at].appearance.blend = crate::BlendMode::Multiply,
                _ => {
                    let sibling = d
                        .layers
                        .iter()
                        .position(|l| l.name == "stationary sibling")
                        .unwrap();
                    d.layers[sibling].appearance.blend = crate::BlendMode::Multiply;
                }
            }
            let before = pixels(&d);
            d.begin_layer_moves(&ids, true).unwrap();
            assert!(!d.layer_move_deferred(), "unsupported ancestor case {kind}");
            d.update_layer_move(3, 2).unwrap();
            d.cancel_layer_move().unwrap();
            assert_eq!(pixels(&d), before);
        }
    }
}
