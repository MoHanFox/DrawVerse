//! Selection edits share one coverage/offset/tile path; candidates publish atomically.
use super::*;

impl Document {
    fn selection_pixel_bounds(&self) -> Result<[i64; 4]> {
        if !self.selection.enabled() {
            return Err(Error::InvalidArgument("pixel selection required"));
        }
        let full = [0, 0, i64::from(self.width), i64::from(self.height)];
        let mut bounds = [0; 4];
        for step in self.selection.steps() {
            use crate::SelectionOperation::*;
            if step.operation == Invert {
                bounds = full;
                continue;
            }
            let shape = step.shape.as_ref().expect("validated shape");
            let margin = shape.feather + 1.;
            let b = [
                (shape.x - margin).floor() as i64,
                (shape.y - margin).floor() as i64,
                (shape.x + shape.width + margin).ceil() as i64,
                (shape.y + shape.height + margin).ceil() as i64,
            ];
            bounds = match step.operation {
                Replace => b,
                Add if bounds[2] <= bounds[0] || bounds[3] <= bounds[1] => b,
                Add => [
                    bounds[0].min(b[0]),
                    bounds[1].min(b[1]),
                    bounds[2].max(b[2]),
                    bounds[3].max(b[3]),
                ],
                Intersect => [
                    bounds[0].max(b[0]),
                    bounds[1].max(b[1]),
                    bounds[2].min(b[2]),
                    bounds[3].min(b[3]),
                ],
                Subtract => bounds,
                Invert => unreachable!(),
            };
        }
        Ok([
            bounds[0].max(0),
            bounds[1].max(0),
            bounds[2].min(full[2]),
            bounds[3].min(full[3]),
        ])
    }

    fn selection_pixel_layer(
        &self,
        original: &Layer,
        keep: bool,
        bounds: [i64; 4],
    ) -> Result<(Layer, bool)> {
        let mut result = original.clone();
        if keep {
            result.tiles.clear();
            result.white = (0, 0);
            result.placed = None;
        }
        let [left, top, right, bottom] = bounds;
        if right <= left || bottom <= top {
            return Ok((result, false));
        }
        let ox = i64::from(original.appearance.offset_x);
        let oy = i64::from(original.appearance.offset_y);
        let mut keys = BTreeSet::new();
        for key in original.tiles.keys() {
            let x = i64::from(key.signed_x()) * 64 + ox;
            let y = i64::from(key.signed_y()) * 64 + oy;
            if x < right && x + 64 > left && y < bottom && y + 64 > top {
                keys.insert(*key);
            }
        }
        let x0 = left.max(ox);
        let y0 = top.max(oy);
        let x1 = right.min(ox + i64::from(original.white.0));
        let y1 = bottom.min(oy + i64::from(original.white.1));
        if x1 > x0 && y1 > y0 {
            let columns = ((x1 - 1 - ox).div_euclid(64) - (x0 - ox).div_euclid(64) + 1) as usize;
            let rows = ((y1 - 1 - oy).div_euclid(64) - (y0 - oy).div_euclid(64) + 1) as usize;
            if columns.saturating_mul(rows) > self.options.max_document_tiles {
                return Err(Error::ResourceLimit("selection pixel tiles"));
            }
            for ty in (y0 - oy).div_euclid(64)..=(y1 - 1 - oy).div_euclid(64) {
                for tx in (x0 - ox).div_euclid(64)..=(x1 - 1 - ox).div_euclid(64) {
                    keys.insert(TileCoord::from_signed(tx as i32, ty as i32));
                }
            }
        }
        if keys.len() > self.options.max_document_tiles {
            return Err(Error::ResourceLimit("selection pixel tiles"));
        }
        let mut changed = false;
        for key in keys {
            let input = original
                .tiles
                .get(&key)
                .map(|tile| tile.try_pixels())
                .transpose()?;
            let x0 = i64::from(key.signed_x()) * 64;
            let y0 = i64::from(key.signed_y()) * 64;
            let mut pixels = Vec::new();
            pixels
                .try_reserve_exact(4096)
                .map_err(|_| Error::ResourceLimit("selection pixel allocation"))?;
            let mut modified = false;
            let mut nonempty = false;
            for row in 0..64 {
                let mut coverage = [0.; 64];
                let y = y0 + row + oy;
                if y >= 0 && y < i64::from(self.height) {
                    self.selection.coverage_span(
                        (x0 + ox) as f64 + 0.5,
                        y as f64 + 0.5,
                        &mut coverage,
                    );
                }
                for (col, mut alpha) in coverage.into_iter().enumerate() {
                    let x = x0 + col as i64 + ox;
                    if x < 0 || x >= i64::from(self.width) {
                        alpha = 0.;
                    }
                    let i = row as usize * 64 + col;
                    let pixel = input.as_ref().map_or_else(
                        || original.default_pixel(x0 + col as i64, y0 + row),
                        |values| values[i],
                    );
                    let next = pixel.scaled(if keep { alpha } else { 1. - alpha });
                    modified |= next != pixel;
                    nonempty |= next != Pixel::TRANSPARENT;
                    pixels.push(next);
                }
            }
            drop(input);
            if (keep && nonempty) || (!keep && modified) {
                let tile = Arc::new(Tile::from_pixels(pixels)?);
                self.pool.attach(&tile)?;
                result.tiles.insert(key, tile);
                self.pool.trim()?;
            }
            changed |= modified;
        }
        if changed && !keep {
            result.placed = None;
        }
        Ok((result, changed))
    }

    /// Copy selected pixels while preserving the copied layer forest and canvas offsets.
    /// Duplicate/copy_layers intentionally retain their whole-layer semantics.
    pub fn copy_selected_pixels(&self, ids: &[u64]) -> Result<Vec<u8>> {
        self.idle()?;
        let bounds = self.selection_pixel_bounds()?;
        let mut nodes = self.copied_nodes(ids)?;
        for node in &mut nodes {
            if !node.group && !node.mask {
                *node = self.selection_pixel_layer(node, true, bounds)?.0;
            }
        }
        if nodes.iter().map(Layer::retained_tile_count).sum::<usize>()
            > self.options.max_document_tiles
        {
            return Err(Error::ResourceLimit("selection clipboard tiles"));
        }
        crate::history_storage::encode_layer_bundle(&nodes)
    }

    /// Clear selected pixels with one history entry and no layer/selection removal.
    pub fn clear_selected_pixels(&mut self, ids: &[u64]) -> Result<bool> {
        self.idle()?;
        let bounds = self.selection_pixel_bounds()?;
        let roots = self.batch_roots(ids)?;
        let mut targets = BTreeSet::new();
        for root in roots {
            for node in &self.layers[self.subtree(root)?] {
                if !node.group && (!node.mask || node.id == root) {
                    self.check_ancestors(node.id, crate::LOCK_ALL | crate::LOCK_TRANSPARENCY)?;
                    targets.insert(node.id);
                }
            }
        }
        let mut before = Vec::new();
        let mut after = Vec::new();
        let mut count = self.tile_count();
        for id in targets {
            let source = self.layer(id).expect("target");
            let (next, changed) = self.selection_pixel_layer(source, false, bounds)?;
            if changed {
                count = count - source.retained_tile_count() + next.retained_tile_count();
                if count > self.options.max_document_tiles {
                    return Err(Error::ResourceLimit("selection pixel document tiles"));
                }
                before.push(source.clone());
                after.push(next);
            }
        }
        if after.is_empty() {
            return Ok(false);
        }
        self.commit_with_action(
            Command::LayerEdit {
                before,
                after: after.clone(),
                selection: None,
            },
            crate::HistoryAction::ClearSelectionPixels,
        )?;
        for layer in after {
            let at = self.layer_index(layer.id)?;
            self.layers[at] = layer;
        }
        self.dirty_all();
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pixels(d: &Document) -> Vec<Pixel> {
        (0..d.height)
            .flat_map(|y| (0..d.width).map(move |x| d.pixel(x, y)))
            .collect()
    }
    #[test]
    fn selected_copy_clear_offsets_feather_inverse_and_history_are_exact() {
        for feather in [0., 5.] {
            for inverted in [false, true] {
                for offset in [0, -7] {
                    let mut d = Document::new(130, 96).unwrap();
                    d.initialize_white_background().unwrap();
                    d.layers[0].appearance.offset_x = offset;
                    let id = d.layers[0].id;
                    let mut shape = crate::SelectionShape::geometry(
                        crate::SelectionKind::Rectangle,
                        23.,
                        17.,
                        38.,
                        31.,
                        true,
                    );
                    shape.feather = feather;
                    d.selection = crate::Selection::default()
                        .apply(shape, crate::SelectionOperation::Replace, 130, 96)
                        .unwrap();
                    if inverted {
                        d.selection = d.selection.inverted(d.width, d.height).unwrap();
                    }
                    let selection = d.selection.clone();
                    let original = pixels(&d);
                    let revision = d.revision();
                    let depth = d.history_depth();
                    let bytes = d.copy_selected_pixels(&[id]).unwrap();
                    assert_eq!(d.revision(), revision);
                    let mut pasted = Document::new(130, 96).unwrap();
                    pasted.paste_layers(&bytes, 1).unwrap();
                    assert!(d.clear_selected_pixels(&[id]).unwrap());
                    assert_eq!(d.layers.len(), 1);
                    assert_eq!(d.selection, selection);
                    assert_eq!(d.history_depth().0, depth.0 + 1);
                    for y in 0..96 {
                        for x in 0..130 {
                            let coverage = selection.coverage(x as f64 + 0.5, y as f64 + 0.5);
                            let p = original[(y * 130 + x) as usize];
                            assert_eq!(pasted.pixel(x, y), p.scaled(coverage), "copy {x},{y}");
                            assert_eq!(d.pixel(x, y), p.scaled(1. - coverage), "clear {x},{y}");
                        }
                    }
                    let cleared = pixels(&d);
                    d.undo().unwrap();
                    assert_eq!(pixels(&d), original);
                    assert_eq!(d.selection, selection);
                    d.redo().unwrap();
                    assert_eq!(pixels(&d), cleared);
                }
            }
        }
    }
    #[test]
    fn selected_group_clear_retains_mask_and_placed_clear_undo_restores_source() {
        let mut d = Document::new(128, 128).unwrap();
        d.initialize_white_background().unwrap();
        let group = d.group_layer(1, "group").unwrap();
        let mask = d.add_mask(1).unwrap();
        let original_mask = d.layer(mask).unwrap().clone();
        d.selection = crate::Selection::default()
            .apply(
                crate::SelectionShape::geometry(
                    crate::SelectionKind::Rectangle,
                    16.,
                    16.,
                    32.,
                    32.,
                    false,
                ),
                crate::SelectionOperation::Replace,
                128,
                128,
            )
            .unwrap();
        let count = d.layers.len();
        let bytes = d.copy_selected_pixels(&[group]).unwrap();
        let mut pasted = Document::new(128, 128).unwrap();
        pasted.paste_layers(&bytes, 1).unwrap();
        assert_eq!(pasted.pixel(32, 32), Pixel::WHITE);
        assert_eq!(pasted.pixel(96, 96), Pixel::TRANSPARENT);
        assert!(d.clear_selected_pixels(&[group]).unwrap());
        assert_eq!(d.layers.len(), count);
        assert_eq!(d.pixel(32, 32), Pixel::TRANSPARENT);
        assert_eq!(d.pixel(96, 96), Pixel::WHITE);
        assert_eq!(d.layer(mask).unwrap().white, original_mask.white);
        assert_eq!(
            d.layer(mask).unwrap().canvas_pixel(32, 32).unwrap(),
            original_mask.canvas_pixel(32, 32).unwrap()
        );
        d.undo().unwrap();
        let id = d
            .place_image(d.layer(1).unwrap().clone(), 128, 128, "placed".into())
            .unwrap();
        d.commit_transform().unwrap();
        let source = d.layer(id).unwrap().placed.clone().unwrap();
        assert!(d.clear_selected_pixels(&[id]).unwrap());
        assert!(d.layer(id).unwrap().placed.is_none());
        assert_eq!(
            d.layer(id).unwrap().canvas_pixel(32, 32).unwrap(),
            Pixel::TRANSPARENT
        );
        d.undo().unwrap();
        assert!(Arc::ptr_eq(
            d.layer(id).unwrap().placed.as_ref().unwrap(),
            &source
        ));
        assert_eq!(
            d.layer(id).unwrap().canvas_pixel(32, 32).unwrap(),
            Pixel::WHITE
        );
    }
    #[test]
    fn selected_clear_noop_and_failure_do_not_publish() {
        let mut d = Document::with_options(
            128,
            128,
            DocumentOptions {
                max_document_tiles: 1,
                ..Default::default()
            },
        )
        .unwrap();
        d.initialize_white_background().unwrap();
        d.selection = crate::Selection::all(128, 128);
        for lock in [crate::LOCK_ALL, crate::LOCK_TRANSPARENCY] {
            d.layers[0].appearance.locks = lock;
            assert!(d.clear_selected_pixels(&[1]).is_err());
        }
        d.layers[0].appearance.locks = 0;
        let before = pixels(&d);
        let revision = d.revision();
        let depth = d.history_depth();
        assert!(d.clear_selected_pixels(&[1]).is_err());
        assert_eq!(pixels(&d), before);
        assert_eq!(d.revision(), revision);
        assert_eq!(d.history_depth(), depth);
        d.selection = crate::Selection::default()
            .apply(
                crate::SelectionShape::geometry(
                    crate::SelectionKind::Rectangle,
                    300.,
                    300.,
                    2.,
                    2.,
                    false,
                ),
                crate::SelectionOperation::Replace,
                128,
                128,
            )
            .unwrap();
        assert!(!d.clear_selected_pixels(&[1]).unwrap());
        assert_eq!(d.revision(), revision);
        assert_eq!(d.history_depth(), depth);
    }
}
