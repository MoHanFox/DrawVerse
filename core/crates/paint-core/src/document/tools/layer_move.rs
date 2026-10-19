//! Integer translation previews share immutable pixels and the resident presentation mesh.
use super::*;

#[derive(Debug)]
pub(super) struct LayerMoveState {
    id: LayerId,
    baseline: Vec<(usize, crate::LayerAppearance)>,
    delta: (i32, i32),
    source: Option<Arc<ResidentPreviewSource>>,
}

impl Document {
    pub fn begin_layer_move(&mut self, id: LayerId, deferred: bool) -> Result<()> {
        self.idle()?;
        self.check_ancestors(id, crate::LOCK_POSITION | crate::LOCK_ALL)?;
        let at = self.layer_index(id)?;
        let range = if self.layers[at].group || self.mask_for(id).is_some() {
            self.subtree(id)?
        } else {
            at..at + 1
        };
        let mut baseline = Vec::new();
        for i in range {
            self.check_ancestors(self.layers[i].id, crate::LOCK_POSITION | crate::LOCK_ALL)?;
            if !self.layers[i].group {
                baseline.push((i, self.layers[i].appearance));
            }
        }
        self.layer_move = Some(LayerMoveState {
            id,
            baseline,
            delta: (0, 0),
            source: if deferred {
                self.resident_source(id)
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
        let result = self.move_layer(state.id, dx, dy);
        // A zero-distance completion still publishes a final frame for handoff.
        if dx == 0 && dy == 0 || result.is_err() {
            self.dirty_all();
        }
        result
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
        assert!(!d.layer_move_deferred());
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
}
