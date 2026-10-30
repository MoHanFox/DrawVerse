//! Existing-selection operations. All candidates and history are prepared before publication.
use super::*;

impl Document {
    /// 0 feather, 1 copy to layer, 2 cut to layer, 3 fill, 4 inside stroke.
    pub fn edit_selected_pixels(&mut self, kind: u32, amount: f64, rgba: [f32; 4]) -> Result<()> {
        self.idle()?;
        if !self.selection.enabled()
            || kind > 4
            || !amount.is_finite()
            || !(0.0..=1000.0).contains(&amount)
        {
            return Err(Error::InvalidArgument("selection operation/amount"));
        }
        let color = Pixel::from_straight(rgba)?;
        use crate::HistoryAction::*;
        let action = [
            FeatherSelection,
            CopySelectionLayer,
            CutSelectionLayer,
            FillSelection,
            StrokeSelection,
        ][kind as usize];
        if kind == 0 {
            if amount == 0. {
                return Ok(());
            }
            let radius = (amount / 2.).ceil() as usize;
            let b = self.selection_pixel_bounds()?;
            let margin = (radius * 3) as i64;
            let left = (b[0] - margin).max(0);
            let top = (b[1] - margin).max(0);
            let right = (b[2] + margin).min(i64::from(self.width));
            let bottom = (b[3] + margin).min(i64::from(self.height));
            if right <= left || bottom <= top {
                return Ok(());
            }
            let (w, h) = ((right - left) as usize, (bottom - top) as usize);
            if w.saturating_mul(h) > crate::MAX_SELECTION_MASK_BYTES {
                return Err(Error::ResourceLimit("selection feather mask"));
            }
            let mut values = vec![0.; w * h];
            for y in 0..h {
                self.selection.coverage_span(
                    left as f64 + 0.5,
                    top as f64 + y as f64 + 0.5,
                    &mut values[y * w..(y + 1) * w],
                );
            }
            // Three separable box passes approximate a Gaussian without radius-dependent work.
            for _ in 0..3 {
                values = blur_axis(&values, w, h, radius, false);
                values = blur_axis(&values, w, h, radius, true);
            }
            let mut shape = crate::SelectionShape::geometry(
                crate::SelectionKind::Mask,
                left as f64,
                top as f64,
                w as f64,
                h as f64,
                false,
            );
            shape.mask = Some(crate::SelectionMask::from_alpha(
                w as u32,
                h as u32,
                values
                    .into_iter()
                    .map(|v| (v.clamp(0., 1.) * 255.).round() as u8)
                    .collect(),
            )?);
            let next = crate::Selection::default().apply(
                shape,
                crate::SelectionOperation::Replace,
                self.width,
                self.height,
            )?;
            if next.to_text().len() > 1024 * 1024 {
                return Err(Error::ResourceLimit("selection feather encoding"));
            }
            return self.set_selection_with_action(next, action);
        }
        let id = self.active_layer;
        self.check_ancestors(
            id,
            crate::LOCK_ALL
                | if kind == 1 {
                    0
                } else {
                    crate::LOCK_TRANSPARENCY
                },
        )?;
        let source = self.layer(id).ok_or(Error::LayerNotFound(id))?.clone();
        if source.group || source.mask {
            return Err(Error::InvalidArgument(
                "selection operation requires a content layer",
            ));
        }
        let bounds = self.selection_pixel_bounds()?;
        let mut result = self.layers.clone();
        if kind == 1 || kind == 2 {
            if self.next_layer_id == u64::MAX {
                return Err(Error::ResourceLimit("selection layer ID"));
            }
            if result.len() >= 4096 {
                return Err(Error::ResourceLimit("selection layer count"));
            }
            let mut copy = self.selection_pixel_layer(&source, true, bounds)?.0;
            copy.id = self.next_layer_id;
            copy.name = if kind == 1 {
                "通过复制创建图层"
            } else {
                "通过剪切创建图层"
            }
            .into();
            let at = self.layer_index(id)?;
            if kind == 2 {
                result[at] = self.selection_pixel_layer(&source, false, bounds)?.0;
            }
            result.insert(at + 1, copy);
        } else {
            if kind == 4 && amount == 0. {
                return Ok(());
            }
            let next = self.selection_color_layer(
                &source,
                bounds,
                color,
                if kind == 4 { amount } else { 0. },
            )?;
            result[self.layer_index(id)?] = next;
        }
        if result.iter().map(Layer::retained_tile_count).sum::<usize>()
            > self.options.max_document_tiles
        {
            return Err(Error::ResourceLimit("selection operation tiles"));
        }
        // Full layer forests make the new layer and (for cut) the cleared original
        // one result transaction. Pixel-only edits use the same immutable result storage.
        self.commit_with_action(
            Command::LayerEdit {
                before: self.layers.clone(),
                after: result.clone(),
                selection: None,
            },
            action,
        )?;
        self.layers = result;
        if kind == 1 || kind == 2 {
            self.active_layer = self.next_layer_id;
            self.next_layer_id = self
                .next_layer_id
                .checked_add(1)
                .ok_or(Error::ResourceLimit("selection layer ID"))?;
        }
        self.dirty_all();
        Ok(())
    }
    fn selection_color_layer(
        &self,
        source: &Layer,
        b: [i64; 4],
        color: Pixel,
        stroke: f64,
    ) -> Result<Layer> {
        let mut result = source.clone();
        result.placed = None;
        let [l, t, r, bottom] = b;
        if r <= l || bottom <= t {
            return Ok(result);
        }
        let ox = i64::from(source.appearance.offset_x);
        let oy = i64::from(source.appearance.offset_y);
        let x0 = (l - ox).div_euclid(64);
        let y0 = (t - oy).div_euclid(64);
        let x1 = (r - 1 - ox).div_euclid(64);
        let y1 = (bottom - 1 - oy).div_euclid(64);
        if ((x1 - x0 + 1) as usize).saturating_mul((y1 - y0 + 1) as usize)
            > self.options.max_document_tiles
        {
            return Err(Error::ResourceLimit("selection color tiles"));
        }
        let distance = if stroke > 0. {
            Some(self.selection_inside_distances()?)
        } else {
            None
        };
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let key = TileCoord::from_signed(tx as i32, ty as i32);
                let input = source.tiles.get(&key).map(|t| t.try_pixels()).transpose()?;
                let mut pixels = Vec::with_capacity(4096);
                let mut changed = false;
                for row in 0..64 {
                    let y = ty * 64 + row + oy;
                    let mut coverage = [0.; 64];
                    if y >= 0 && y < i64::from(self.height) {
                        self.selection.coverage_span(
                            (tx * 64 + ox) as f64 + 0.5,
                            y as f64 + 0.5,
                            &mut coverage,
                        );
                    }
                    for (col, mut alpha) in coverage.into_iter().enumerate() {
                        let x = tx * 64 + col as i64 + ox;
                        let i = row as usize * 64 + col;
                        let old = input.as_ref().map_or_else(
                            || source.default_pixel(tx * 64 + col as i64, ty * 64 + row),
                            |p| p[i],
                        );
                        if x < 0
                            || x >= i64::from(self.width)
                            || y < 0
                            || y >= i64::from(self.height)
                        {
                            alpha = 0.;
                        } else if let Some(d) = &distance {
                            alpha *= ((stroke + 1.
                                - d[y as usize * self.width as usize + x as usize].sqrt() as f64)
                                .clamp(0., 1.)) as f32;
                        }
                        let next = color.scaled(alpha).over(old);
                        changed |= next != old;
                        pixels.push(next);
                    }
                }
                drop(input);
                if changed {
                    let tile = Arc::new(Tile::from_pixels(pixels)?);
                    self.pool.attach(&tile)?;
                    result.tiles.insert(key, tile);
                    self.pool.trim()?;
                }
            }
        }
        Ok(result)
    }
    fn selection_inside_distances(&self) -> Result<Vec<f32>> {
        let (w, h) = (self.width as usize, self.height as usize);
        if w.saturating_mul(h) > crate::MAX_SELECTION_MASK_BYTES {
            return Err(Error::ResourceLimit("selection stroke mask"));
        }
        let mut d = vec![0.; w * h];
        for y in 0..h {
            let mut row = vec![0.; w];
            self.selection.coverage_span(0.5, y as f64 + 0.5, &mut row);
            for x in 0..w {
                d[y * w + x] = if row[x] < 0.5 { 0. } else { 1e12 };
            }
        }
        // Exact squared Euclidean distance to the outside, including the document boundary.
        for y in 0..h {
            let mut row = Vec::with_capacity(w + 2);
            row.push(0.);
            row.extend_from_slice(&d[y * w..(y + 1) * w]);
            row.push(0.);
            let out = distance_line(&row);
            d[y * w..(y + 1) * w].copy_from_slice(&out[1..w + 1]);
        }
        for x in 0..w {
            let mut col = Vec::with_capacity(h + 2);
            col.push(0.);
            col.extend((0..h).map(|y| d[y * w + x]));
            col.push(0.);
            let out = distance_line(&col);
            for y in 0..h {
                d[y * w + x] = out[y + 1];
            }
        }
        Ok(d)
    }
}
fn blur_axis(input: &[f32], w: usize, h: usize, r: usize, vertical: bool) -> Vec<f32> {
    let mut output = vec![0.; input.len()];
    let (lines, n) = if vertical { (w, h) } else { (h, w) };
    for line in 0..lines {
        let index = |i: usize| if vertical { i * w + line } else { line * w + i };
        let mut sum: f64 = (0..=r.min(n - 1)).map(|i| input[index(i)] as f64).sum();
        for i in 0..n {
            output[index(i)] = (sum / (2 * r + 1) as f64) as f32;
            if i >= r {
                sum -= input[index(i - r)] as f64;
            }
            if i + r + 1 < n {
                sum += input[index(i + r + 1)] as f64;
            }
        }
    }
    output
}
fn distance_line(f: &[f32]) -> Vec<f32> {
    let n = f.len();
    let mut v = vec![0usize; n];
    let mut z = vec![0f64; n + 1];
    let mut k = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in 1..n {
        let mut s;
        loop {
            let p = v[k];
            s = ((f[q] as f64 + (q * q) as f64) - (f[p] as f64 + (p * p) as f64))
                / (2. * (q - p) as f64);
            if s > z[k] || k == 0 {
                break;
            }
            k -= 1;
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    (0..n)
        .map(|q| {
            while z[k + 1] < (q as f64) {
                k += 1;
            }
            let delta = q as f64 - v[k] as f64;
            (delta * delta + f[v[k]] as f64) as f32
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Document {
        let mut d = Document::new(64, 48).unwrap();
        d.selection = crate::Selection::default()
            .apply(
                crate::SelectionShape::geometry(
                    crate::SelectionKind::Rectangle,
                    12.,
                    10.,
                    24.,
                    20.,
                    false,
                ),
                crate::SelectionOperation::Replace,
                64,
                48,
            )
            .unwrap();
        d
    }
    fn pixels(d: &Document) -> Vec<Pixel> {
        (0..d.height)
            .flat_map(|y| (0..d.width).map(move |x| d.pixel(x, y)))
            .collect()
    }
    #[test]
    fn selection_actions_fill_stroke_offsets_opacity_and_single_history() {
        for kind in [3, 4] {
            let mut d = fixture();
            d.layers[0].appearance.offset_x = -5;
            d.layers[0].appearance.offset_y = 3;
            let depth = d.history_depth().0;
            let original = pixels(&d);
            let selection = d.selection.clone();
            d.edit_selected_pixels(kind, 3., [1., 0., 0., 0.5]).unwrap();
            assert_eq!(d.history_depth().0, depth + 1);
            assert_eq!(d.pixel(12, 10).alpha(), 0.5);
            assert_eq!(d.pixel(11, 10).alpha(), 0.);
            assert_eq!(d.pixel(20, 18).alpha(), if kind == 3 { 0.5 } else { 0. });
            assert_eq!(d.selection, selection);
            let result = pixels(&d);
            d.undo().unwrap();
            assert_eq!(pixels(&d), original);
            d.redo().unwrap();
            assert_eq!(pixels(&d), result);
        }
    }
    #[test]
    fn selection_actions_copy_cut_selected_coverage_one_history_and_repeated_travel() {
        for kind in [1, 2] {
            let mut d = fixture();
            d.initialize_white_background().unwrap();
            d.layers[0].appearance.offset_x = -4;
            let mut shape = crate::SelectionShape::geometry(
                crate::SelectionKind::Ellipse,
                12.,
                10.,
                24.,
                20.,
                true,
            );
            shape.feather = 3.;
            d.selection = crate::Selection::default()
                .apply(shape, crate::SelectionOperation::Replace, 64, 48)
                .unwrap();
            let source = d.layers[0].clone();
            let original = pixels(&d);
            let depth = d.history_depth().0;
            d.edit_selected_pixels(kind, 0., [0.; 4]).unwrap();
            assert_eq!(d.layers.len(), 2);
            assert_eq!(d.history_depth().0, depth + 1);
            let copy = d.layers[1].clone();
            let cleared = d.layers[0].clone();
            for y in 0..48 {
                for x in 0..64 {
                    let alpha = d.selection.coverage(x as f64 + 0.5, y as f64 + 0.5);
                    let old = source.canvas_pixel(x, y).unwrap();
                    assert_eq!(copy.canvas_pixel(x, y).unwrap(), old.scaled(alpha));
                    assert_eq!(
                        cleared.canvas_pixel(x, y).unwrap(),
                        old.scaled(if kind == 2 { 1. - alpha } else { 1. })
                    );
                }
            }
            let result = pixels(&d);
            for _ in 0..3 {
                d.undo().unwrap();
                assert_eq!(d.layers.len(), 1);
                assert_eq!(pixels(&d), original);
                d.redo().unwrap();
                assert_eq!(d.layers.len(), 2);
                assert_eq!(pixels(&d), result);
            }
        }
    }
    #[test]
    fn selection_actions_feather_composite_with_hole_and_cancel_no_mutation() {
        let mut d = fixture();
        let hole = crate::SelectionShape::geometry(
            crate::SelectionKind::Rectangle,
            20.,
            16.,
            8.,
            8.,
            false,
        );
        d.selection = d
            .selection
            .apply(hole, crate::SelectionOperation::Subtract, 64, 48)
            .unwrap();
        let before = d.selection.clone();
        let original = pixels(&d);
        let depth = d.history_depth().0;
        d.edit_selected_pixels(0, 3., [0.; 4]).unwrap();
        let after = d.selection.clone();
        assert!(after.coverage(11.5, 15.5) > 0.);
        assert!(after.coverage(12.5, 15.5) < 1.);
        assert!(after.coverage(23.5, 19.5) < after.coverage(16.5, 19.5));
        assert_eq!(pixels(&d), original);
        assert_eq!(d.history_depth().0, depth + 1);
        d.undo().unwrap();
        assert_eq!(d.selection, before);
        d.redo().unwrap();
        assert_eq!(d.selection, after);
        let revision = d.revision();
        for (kind, amount) in [(5, 0.), (0, f64::NAN), (4, 1001.)] {
            assert!(d.edit_selected_pixels(kind, amount, [0.; 4]).is_err());
            assert_eq!(d.revision(), revision);
        }
    }
    #[test]
    fn selection_actions_lock_and_budget_failure_are_atomic() {
        let mut d = fixture();
        d.layers[0].appearance.locks = crate::LOCK_ALL;
        let revision = d.revision();
        let depth = d.history_depth();
        assert!(d.edit_selected_pixels(2, 0., [0.; 4]).is_err());
        assert_eq!(d.revision(), revision);
        assert_eq!(d.history_depth(), depth);
        assert_eq!(d.layers.len(), 1);
        d.layers[0].appearance.locks = 0;
        d.options.max_document_tiles = 0;
        assert!(d.edit_selected_pixels(3, 0., [1.; 4]).is_err());
        assert_eq!(d.revision(), revision);
        assert_eq!(d.history_depth(), depth);
    }
    #[test]
    fn selection_actions_added_layers_restore_from_cold_and_encoded_history() {
        let layer_pixels = |d: &Document| {
            d.layers
                .iter()
                .map(|l| {
                    (0..128)
                        .flat_map(|y| l.read_canvas_row(0, y, 128).unwrap())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        for resident in [1, 16] {
            for budget in [1024, 2 * TILE_BYTES + 256] {
                for kind in [1, 2] {
                    let mut d = super::super::liquify_result_history_tests::document(resident);
                    d.options.max_history_bytes = budget;
                    d.selection = crate::Selection::all(128, 128);
                    let original = layer_pixels(&d);
                    d.edit_selected_pixels(kind, 0., [0.; 4]).unwrap();
                    let result = layer_pixels(&d);
                    if budget == 1024 {
                        assert!(matches!(
                            d.history.undo.back(),
                            Some(StoredCommand::Encoded(..))
                        ));
                    } else {
                        assert!(matches!(
                            d.history.undo.back(),
                            Some(StoredCommand::Paged(..))
                        ));
                    }
                    for _ in 0..3 {
                        d.undo().unwrap();
                        assert_eq!(d.layers.len(), 1);
                        assert_eq!(layer_pixels(&d), original);
                        d.redo().unwrap();
                        assert_eq!(d.layers.len(), 2);
                        assert_eq!(layer_pixels(&d), result);
                    }
                    d.trim_storage().unwrap();
                    assert!(d.storage_stats().resident_tiles <= resident);
                }
            }
        }
    }
    #[test]
    fn selection_actions_distance_is_euclidean_and_blur_zero_padded() {
        assert_eq!(
            distance_line(&[0., 1e12, 1e12, 1e12, 0.]),
            vec![0., 1., 4., 1., 0.]
        );
        let blurred = blur_axis(&[0., 0., 1., 0., 0.], 5, 1, 1, false);
        assert_eq!(blurred, vec![0., 1. / 3., 1. / 3., 1. / 3., 0.]);
    }
}
