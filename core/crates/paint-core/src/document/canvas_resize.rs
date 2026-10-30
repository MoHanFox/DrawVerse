use super::*;
impl Document {
    pub fn dpi(&self) -> u32 {
        self.dpi
    }
    /// Construction-only metadata restored by the document reader.
    pub fn initialize_dpi(&mut self, dpi: u32) -> Result<()> {
        if !(1..=12000).contains(&dpi) {
            return Err(Error::InvalidArgument("dpi"));
        }
        self.dpi = dpi;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn resize_canvas(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        dpi: u32,
        delete: bool,
        fill: Pixel,
    ) -> Result<bool> {
        self.idle()?;
        if width == 0
            || height == 0
            || width > MAX_DIMENSION
            || height > MAX_DIMENSION
            || !(1..=12000).contains(&dpi)
            || x.unsigned_abs() > MAX_DIMENSION
            || y.unsigned_abs() > MAX_DIMENSION
        {
            return Err(Error::InvalidArgument("canvas bounds/dpi"));
        }
        if !delete
            && x == 0
            && y == 0
            && (width, height, dpi) == (self.width, self.height, self.dpi)
        {
            return Ok(false);
        }
        let mut after = self.layers.clone();
        let mut count = 0usize;
        for layer in &mut after {
            if layer.group {
                continue;
            }
            let ox = i64::from(layer.appearance.offset_x);
            let oy = i64::from(layer.appearance.offset_y);
            if delete {
                let left = i64::from(x) - ox;
                let top = i64::from(y) - oy;
                let right = left + i64::from(width);
                let bottom = top + i64::from(height);
                let mut keys: BTreeSet<_> = layer
                    .tiles
                    .keys()
                    .copied()
                    .filter(|k| {
                        let a = i64::from(k.signed_x()) * 64;
                        let b = i64::from(k.signed_y()) * 64;
                        a < right && a + 64 > left && b < bottom && b + 64 > top
                    })
                    .collect();
                let w = if layer.mask {
                    i64::from(self.width) - ox
                } else {
                    i64::from(layer.white.0)
                };
                let h = if layer.mask {
                    i64::from(self.height) - oy
                } else {
                    i64::from(layer.white.1)
                };
                let l = left.max(if layer.mask { -ox } else { 0 });
                let t = top.max(if layer.mask { -oy } else { 0 });
                let r = right.min(w);
                let b = bottom.min(h);
                if r > l && b > t {
                    if ((r - l + 127) / 64) as u64 * ((b - t + 127) / 64) as u64
                        > self.options.max_document_tiles as u64
                    {
                        return Err(Error::ResourceLimit("canvas crop tiles"));
                    }
                    for ty in t.div_euclid(64)..=(b - 1).div_euclid(64) {
                        for tx in l.div_euclid(64)..=(r - 1).div_euclid(64) {
                            keys.insert(TileCoord::from_signed(tx as i32, ty as i32));
                        }
                    }
                }
                let original = layer.clone();
                layer.tiles.clear();
                layer.white = (0, 0);
                layer.placed = None;
                for key in keys {
                    let input = original
                        .tiles
                        .get(&key)
                        .map(|t| t.try_pixels())
                        .transpose()?;
                    let mut pixels = Vec::with_capacity(4096);
                    let mut nonempty = false;
                    for i in 0..4096 {
                        let px = i64::from(key.signed_x()) * 64 + (i % 64) as i64;
                        let py = i64::from(key.signed_y()) * 64 + (i / 64) as i64;
                        let p = if px >= left && px < right && py >= top && py < bottom {
                            input
                                .as_ref()
                                .map_or_else(|| original.default_pixel(px, py), |v| v[i])
                        } else {
                            Pixel::TRANSPARENT
                        };
                        nonempty |= p != Pixel::TRANSPARENT;
                        pixels.push(p);
                    }
                    if nonempty || layer.mask {
                        count += 1;
                        if count > self.options.max_document_tiles {
                            return Err(Error::ResourceLimit("canvas tiles"));
                        }
                        let tile = Arc::new(Tile::from_pixels(pixels)?);
                        self.pool.attach(&tile)?;
                        layer.tiles.insert(key, tile);
                        self.pool.trim()?;
                    }
                }
            } else {
                count += layer.retained_tile_count();
            }
            layer.appearance.offset_x = i32::try_from(ox - i64::from(x))
                .map_err(|_| Error::InvalidArgument("canvas offset"))?;
            layer.appearance.offset_y = i32::try_from(oy - i64::from(y))
                .map_err(|_| Error::InvalidArgument("canvas offset"))?;
            layer.appearance.validate()?;
        }
        let expanded = x < 0
            || y < 0
            || i64::from(x) + i64::from(width) > i64::from(self.width)
            || i64::from(y) + i64::from(height) > i64::from(self.height);
        if expanded {
            if after.len() >= 4096 {
                return Err(Error::ResourceLimit("canvas fill layer"));
            }
            let mut background = Layer::new(self.next_layer_id, "画布扩展填充".into());
            let mut keys = BTreeSet::new();
            let l = (-i64::from(x)).clamp(0, i64::from(width));
            let t = (-i64::from(y)).clamp(0, i64::from(height));
            let r = (i64::from(self.width) - i64::from(x)).clamp(l, i64::from(width));
            let b = (i64::from(self.height) - i64::from(y)).clamp(t, i64::from(height));
            for [a, c, d, e] in [
                [0, 0, i64::from(width), t],
                [0, b, i64::from(width), i64::from(height)],
                [0, t, l, b],
                [r, t, i64::from(width), b],
            ] {
                if d <= a || e <= c {
                    continue;
                }
                if ((d - a + 127) / 64) as u64 * ((e - c + 127) / 64) as u64
                    > self.options.max_document_tiles as u64
                {
                    return Err(Error::ResourceLimit("canvas fill tiles"));
                }
                for ty in c / 64..=(e - 1) / 64 {
                    for tx in a / 64..=(d - 1) / 64 {
                        keys.insert(TileCoord::from_signed(tx as i32, ty as i32));
                    }
                }
            }
            for key in keys {
                count += 1;
                if count > self.options.max_document_tiles {
                    return Err(Error::ResourceLimit("canvas fill tiles"));
                }
                let pixels = (0..4096)
                    .map(|i| {
                        let px = i64::from(key.signed_x()) * 64 + (i % 64) as i64;
                        let py = i64::from(key.signed_y()) * 64 + (i / 64) as i64;
                        let sx = px + i64::from(x);
                        let sy = py + i64::from(y);
                        if px < i64::from(width)
                            && py < i64::from(height)
                            && (sx < 0
                                || sy < 0
                                || sx >= i64::from(self.width)
                                || sy >= i64::from(self.height))
                        {
                            fill
                        } else {
                            Pixel::TRANSPARENT
                        }
                    })
                    .collect();
                let tile = Arc::new(Tile::from_pixels(pixels)?);
                self.pool.attach(&tile)?;
                background.tiles.insert(key, tile);
                self.pool.trim()?;
            }
            after.insert(0, background);
        }
        let command = Command::Canvas {
            before_size: [self.width, self.height, self.dpi],
            after_size: [width, height, dpi],
            before: self.layers.clone(),
            after,
            selection: (self.selection.clone(), crate::Selection::default()),
        };
        self.commit_with_action(command.clone(), crate::HistoryAction::ResizeCanvas)?;
        self.apply_history(&command, true);
        if expanded {
            self.next_layer_id += 1;
        }
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crop_retains_or_deletes_and_history_restores_dimensions() {
        for delete in [false, true] {
            let mut d = Document::new(130, 80).unwrap();
            d.initialize_white_background().unwrap();
            d.resize_canvas(64, 0, 66, 80, 144, delete, Pixel::WHITE)
                .unwrap();
            assert_eq!(d.dimensions(), (66, 80));
            assert_eq!(d.dpi(), 144);
            d.resize_canvas(-64, 0, 130, 80, 144, false, Pixel::TRANSPARENT)
                .unwrap();
            assert_eq!(
                d.pixel(2, 2),
                if delete {
                    Pixel::TRANSPARENT
                } else {
                    Pixel::WHITE
                }
            );
            d.undo().unwrap();
            d.undo().unwrap();
            assert_eq!(d.dimensions(), (130, 80));
            assert_eq!(d.dpi(), 72);
            assert_eq!(d.pixel(2, 2), Pixel::WHITE);
            d.redo().unwrap();
            assert_eq!(d.dimensions(), (66, 80));
        }
    }
    #[test]
    fn crop_covers_hidden_locked_groups_masks_and_history_travel() {
        let mut d = Document::new(130, 80).unwrap();
        d.initialize_white_background().unwrap();
        let mask = d.add_mask(1).unwrap();
        d.group_layer(1, "group").unwrap();
        let hidden = d.add_layer("hidden").unwrap();
        let at = d.layer_index(hidden).unwrap();
        d.layers[at].properties.visible = false;
        d.layers[at].appearance.locks = crate::LOCK_ALL;
        let tile = Arc::new(Tile::from_pixels(vec![Pixel::WHITE; 4096]).unwrap());
        d.pool.attach(&tile).unwrap();
        d.layers[at]
            .tiles
            .insert(TileCoord::from_signed(0, 0), tile);
        d.set_selection(crate::Selection::all(130, 80)).unwrap();
        let selection = d.selection.clone();
        let before = d.embedded_layers().unwrap();
        let depth = d.history_depth().0;
        d.resize_canvas(64, 0, 66, 80, 240, true, Pixel::WHITE)
            .unwrap();
        assert_eq!(d.pixel(0, 0), Pixel::WHITE);
        assert!(d.layer(hidden).unwrap().tiles.is_empty());
        assert!(d.layer(mask).unwrap().placed.is_none());
        assert!(!d.selection.enabled());
        d.select_history(depth).unwrap();
        assert_eq!(d.embedded_layers().unwrap(), before);
        assert_eq!(d.selection, selection);
        assert_eq!(d.dimensions(), (130, 80));
        assert_eq!(d.dpi(), 72);
        d.select_history(depth + 1).unwrap();
        assert_eq!(d.dimensions(), (66, 80));
        assert_eq!(d.dpi(), 240);
    }
    #[test]
    fn encoded_history_and_failure_preserve_canvas() {
        let mut d = Document::with_options(
            192,
            128,
            DocumentOptions {
                max_history_bytes: 2 * TILE_BYTES + 256,
                ..Default::default()
            },
        )
        .unwrap();
        d.initialize_white_background().unwrap();
        d.resize_canvas(32, 0, 160, 128, 300, true, Pixel::WHITE)
            .unwrap();
        d.undo().unwrap();
        assert_eq!(d.dimensions(), (192, 128));
        assert_eq!(d.dpi(), 72);
        d.redo().unwrap();
        assert_eq!(d.dimensions(), (160, 128));
        let size = d.dimensions();
        let rev = d.revision();
        d.options.max_history_bytes = 1024; // Force the encoded fallback rather than result references.
        crate::history_storage::fail_writes_for_test(true);
        let result = d.resize_canvas(-32, 0, 192, 128, 144, false, Pixel::WHITE);
        crate::history_storage::fail_writes_for_test(false);
        assert!(result.is_err());
        assert_eq!(d.dimensions(), size);
        assert_eq!(d.revision(), rev);
        assert_eq!(d.dpi(), 300);
    }
    #[test]
    fn canvas_result_history_reuses_warm_and_cold_page_identities() {
        for resident in [1, 16] {
            for delete in [false, true] {
                let mut d = super::super::liquify_result_history_tests::document(resident);
                d.set_selection(crate::Selection::all(128, 128)).unwrap();
                let depth = d.history_depth().0;
                let selection = d.selection.clone();
                let before = d.layers.clone();
                d.resize_canvas(16, 0, 96, 128, 300, delete, Pixel::WHITE)
                    .unwrap();
                assert!(matches!(
                    d.history.undo.back(),
                    Some(StoredCommand::Paged(..))
                ));
                assert_eq!(d.history_disk_bytes(), 0);
                let after = d.layers.clone();
                let stats = d.storage_stats();
                for _ in 0..3 {
                    d.select_history(depth).unwrap();
                    assert_eq!(d.dimensions(), (128, 128));
                    assert_eq!(d.dpi(), 72);
                    assert_eq!(d.selection, selection);
                    for (a, b) in d.layers.iter().zip(&before) {
                        for (key, tile) in &a.tiles {
                            assert!(Arc::ptr_eq(tile, &b.tiles[key]));
                        }
                    }
                    d.select_history(depth + 1).unwrap();
                    assert_eq!(d.dimensions(), (96, 128));
                    assert_eq!(d.dpi(), 300);
                    for (a, b) in d.layers.iter().zip(&after) {
                        for (key, tile) in &a.tiles {
                            assert!(Arc::ptr_eq(tile, &b.tiles[key]));
                        }
                    }
                }
                assert_eq!(d.storage_stats().registered_tiles, stats.registered_tiles);
                if resident == 16 {
                    assert_eq!(d.storage_stats().page_reads, stats.page_reads);
                }
                d.trim_storage().unwrap();
                assert!(d.storage_stats().resident_tiles <= resident);
            }
        }
    }
    #[test]
    fn expansion_fills_only_new_region_and_invalid_is_atomic() {
        let mut d = Document::new(70, 65).unwrap();
        let color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
        d.resize_canvas(-3, -2, 76, 70, 300, false, color).unwrap();
        assert_eq!(d.pixel(0, 0), color);
        assert_eq!(d.pixel(4, 3), Pixel::TRANSPARENT);
        let rev = d.revision();
        assert!(d.resize_canvas(0, 0, 0, 70, 300, false, color).is_err());
        assert_eq!(d.revision(), rev);
    }
}
