use super::color_region::RegionDiscovery;
use super::*;

pub(super) const MAX_FILL_TILES: usize = 4096;
const MAX_FILL_PIXELS: u64 = MAX_FILL_TILES as u64 * 4096;

#[derive(Clone, Copy, Debug)]
pub struct BucketFill {
    pub x: u32,
    pub y: u32,
    pub color: Pixel,
    pub opacity: f32,
    pub tolerance: u32,
    pub antialias: bool,
    pub contiguous: bool,
    pub all_layers: bool,
}
impl BucketFill {
    pub fn validate(&self) -> Result<()> {
        if !self.opacity.is_finite() || !(0. ..=1.).contains(&self.opacity) || self.tolerance > 255
        {
            return Err(Error::InvalidArgument("bucket opacity/tolerance"));
        }
        Ok(())
    }
}
fn srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1. / 2.4) - 0.055
    }
}
pub(super) fn compare_color(pixel: Pixel) -> [f32; 4] {
    let c = pixel.components();
    if c[3] == 0. {
        return [0.; 4];
    }
    [
        srgb(c[0] / c[3]) * 255.,
        srgb(c[1] / c[3]) * 255.,
        srgb(c[2] / c[3]) * 255.,
        c[3] * 255.,
    ]
}
pub(super) fn match_weight(reference: [f32; 4], pixel: Pixel, options: BucketFill) -> f32 {
    let delta = reference
        .into_iter()
        .zip(compare_color(pixel))
        .map(|(a, b)| (a - b).abs())
        .fold(0., f32::max);
    if delta <= options.tolerance as f32 {
        1.
    } else if options.antialias {
        (options.tolerance as f32 + 1. - delta).clamp(0., 1.)
    } else {
        0.
    }
}
impl Document {
    /// Atomic fill of the active pixel layer. Discovery always samples pre-fill pixels.
    pub fn bucket_fill(&mut self, options: BucketFill) -> Result<bool> {
        self.idle()?;
        options.validate()?;
        if options.x >= self.width || options.y >= self.height {
            return Err(Error::InvalidArgument("bucket seed outside document"));
        }
        let index = self.layer_index(self.active_layer)?;
        if self.layers[index].is_image() {
            return Err(Error::InvalidArgument("rasterize image before filling"));
        }
        if self.layers[index].group {
            return Err(Error::InvalidArgument("select a pixel layer to fill"));
        }
        self.check_ancestors(self.active_layer, crate::LOCK_ALL)?;
        let layer = self.layers[index].clone();
        if layer.mask && options.all_layers {
            return Err(Error::InvalidArgument(
                "all-layer bucket sampling on a mask",
            ));
        }
        if options.opacity == 0. || options.color.alpha() == 0. {
            return Ok(false);
        }
        if !options.contiguous && u64::from(self.width) * u64::from(self.height) > MAX_FILL_PIXELS {
            return Err(Error::ResourceLimit(
                "global bucket scan limit; no pixels changed",
            ));
        }
        let sample = |doc: &Self, x, y| {
            if options.all_layers {
                doc.try_pixel(x, y)
            } else {
                layer.canvas_pixel(x, y)
            }
        };
        if self.selection.enabled()
            && self.selection_cache.row(
                &self.selection,
                i64::from(options.x),
                i64::from(options.y),
                1,
            )[0] == 0.
        {
            return Ok(false);
        }
        let reference = compare_color(sample(self, options.x, options.y)?);
        let mut selection_cache = crate::selection::SelectionCoverageCache::default();
        let mut discovery = RegionDiscovery::new(self.width, self.height, |x, y, count| {
            let selection = if self.selection.enabled() {
                selection_cache.row(&self.selection, x.into(), y.into(), count as usize)
            } else {
                [1.; 64]
            };
            let pixels = if options.all_layers {
                self.read_row(x, y, count)?
            } else {
                layer.read_canvas_row(x, y, count)?
            };
            Ok(pixels
                .into_iter()
                .enumerate()
                .map(|(i, pixel)| {
                    let weight = if selection[i] == 0. {
                        0.
                    } else {
                        match_weight(reference, pixel, options)
                    };
                    (weight * selection[i], weight != 0.)
                })
                .collect())
        });
        if options.contiguous {
            discovery.contiguous(options.x, options.y)?;
        } else {
            discovery.global()?;
        }
        let appearance = layer.appearance;
        let alpha_locked = !layer.mask && appearance.locks & crate::LOCK_TRANSPARENCY != 0;
        let color = compare_color(options.color);
        let mask_tone = 1. - (0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2]) / 255.;
        let paint = |before: Pixel, weight: f32| -> Result<Pixel> {
            let amount = weight * options.opacity;
            if layer.mask {
                Pixel::from_straight([
                    0.,
                    0.,
                    0.,
                    before.alpha() * (1. - amount * options.color.alpha())
                        + mask_tone * amount * options.color.alpha(),
                ])
            } else if alpha_locked {
                Ok(before.paint_preserving_alpha(options.color, amount))
            } else {
                Ok(options.color.scaled(amount).over(before))
            }
        };
        let mut staged: BTreeMap<TileCoord, Vec<Pixel>> = BTreeMap::new();
        for ((tx, ty), region) in discovery.tiles {
            if region.selected.iter().all(|&row| row == 0) {
                continue;
            }
            if appearance.offset_x % 64 == 0 && appearance.offset_y % 64 == 0 {
                let coord = TileCoord::from_signed(
                    tx as i32 - appearance.offset_x / 64,
                    ty as i32 - appearance.offset_y / 64,
                );
                let original = layer
                    .tiles
                    .get(&coord)
                    .map(|tile| tile.try_pixels())
                    .transpose()?;
                let whole = region.selected.iter().all(|&row| row == u64::MAX)
                    && region.weights.iter().all(|&weight| weight == 1.)
                    && !layer.mask
                    && !alpha_locked
                    && options.opacity == 1.
                    && options.color.alpha() == 1.;
                if whole {
                    if original
                        .as_ref()
                        .is_none_or(|pixels| pixels.iter().any(|&pixel| pixel != options.color))
                    {
                        staged.insert(coord, vec![options.color; 4096]);
                    }
                    continue;
                }
                let mut pixels = original.as_ref().map_or_else(
                    || {
                        (0..4096)
                            .map(|i| {
                                layer.default_pixel(
                                    i64::from(coord.signed_x()) * 64 + i % 64,
                                    i64::from(coord.signed_y()) * 64 + i / 64,
                                )
                            })
                            .collect()
                    },
                    |pixels| pixels.to_vec(),
                );
                let mut changed = false;
                for (y, mut bits) in region.selected.into_iter().enumerate() {
                    while bits != 0 {
                        let x = bits.trailing_zeros() as usize;
                        bits &= bits - 1;
                        let i = y * 64 + x;
                        if region.weights[i] == 0. {
                            continue;
                        }
                        let after = paint(pixels[i], region.weights[i])?;
                        changed |= after != pixels[i];
                        pixels[i] = after;
                    }
                }
                if changed {
                    staged.insert(coord, pixels);
                }
                continue;
            }

            for (ly, mut bits) in region.selected.into_iter().enumerate() {
                if bits == 0 {
                    continue;
                }
                let canvas_y = ty * 64 + ly as u32;
                let count = (self.width - tx * 64).min(64);
                let before_row = layer.read_canvas_row(tx * 64, canvas_y, count)?;
                // An offset canvas row touches at most two local tiles. Collect real changes,
                // then look up each output tile once per row, rather than once per pixel.
                let mut changes: BTreeMap<TileCoord, Vec<(usize, Pixel)>> = BTreeMap::new();
                while bits != 0 {
                    let lx = bits.trailing_zeros();
                    bits &= bits - 1;
                    let weight = region.weights[ly * 64 + lx as usize];
                    if weight == 0. {
                        continue;
                    }
                    let x = i64::from(tx * 64 + lx) - i64::from(appearance.offset_x);
                    let y = i64::from(canvas_y) - i64::from(appearance.offset_y);
                    let coord =
                        TileCoord::from_signed(x.div_euclid(64) as i32, y.div_euclid(64) as i32);
                    let before = before_row[lx as usize];
                    let after = paint(before, weight)?;
                    if after != before {
                        changes
                            .entry(coord)
                            .or_default()
                            .push(((y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize, after));
                    }
                }
                for (coord, row_changes) in changes {
                    if !staged.contains_key(&coord) {
                        if staged.len() == MAX_FILL_TILES {
                            return Err(Error::ResourceLimit(
                                "bucket output tile limit; no pixels changed",
                            ));
                        }
                        let pixels = if let Some(tile) = layer.tiles.get(&coord) {
                            tile.try_pixels()?.to_vec()
                        } else {
                            (0..4096)
                                .map(|i| {
                                    layer.default_pixel(
                                        i64::from(coord.signed_x()) * 64 + i % 64,
                                        i64::from(coord.signed_y()) * 64 + i / 64,
                                    )
                                })
                                .collect()
                        };
                        staged.insert(coord, pixels);
                    }
                    let pixels = staged.get_mut(&coord).expect("staged tile");
                    for (offset, after) in row_changes {
                        pixels[offset] = after;
                    }
                }
            }
        }
        if staged.is_empty() {
            return Ok(false);
        }
        let added = staged
            .keys()
            .filter(|coord| !layer.tiles.contains_key(coord))
            .count();
        if self.tile_count() + added > self.options.max_document_tiles {
            return Err(Error::ResourceLimit(
                "bucket logical tile limit; no pixels changed",
            ));
        }
        let mut changes = Vec::with_capacity(staged.len());
        for (coord, pixels) in staged {
            let after = Arc::new(Tile::from_pixels(pixels)?);
            self.pool.attach(&after)?;
            changes.push(TileChange {
                coord,
                before: layer.tiles.get(&coord).cloned(),
                after: Some(after),
            });
            self.pool.trim()?;
        }
        let command = Command::Stroke {
            layer: self.active_layer,
            changes,
        };
        self.commit_with_action(command.clone(), crate::HistoryAction::Bucket)?;
        self.apply_history(&command, true);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_failure_is_atomic() {
        let mut doc = Document::with_options(
            128,
            128,
            DocumentOptions {
                max_history_bytes: 2 * TILE_BYTES + 256,
                ..DocumentOptions::default()
            },
        )
        .unwrap();
        let options = BucketFill {
            x: 0,
            y: 0,
            color: Pixel::WHITE,
            opacity: 1.,
            tolerance: 0,
            antialias: true,
            contiguous: true,
            all_layers: false,
        };
        crate::history_storage::fail_writes_for_test(true);
        let result = doc.bucket_fill(options);
        crate::history_storage::fail_writes_for_test(false);
        assert!(result.is_err());
        assert_eq!(doc.revision(), 0);
        assert_eq!(doc.history_depth(), (0, 0));
        assert_eq!(doc.pixel(0, 0), Pixel::TRANSPARENT);
        assert!(doc.bucket_fill(options).unwrap());
        doc.undo().unwrap();
        assert_eq!(doc.pixel(63, 63), Pixel::TRANSPARENT);
        doc.redo().unwrap();
        assert_eq!(doc.pixel(63, 63), Pixel::WHITE);
    }
}
