use super::*;
use std::collections::VecDeque;

const MAX_FILL_TILES: usize = 4096;
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
fn compare_color(pixel: Pixel) -> [f32; 4] {
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
fn match_weight(reference: [f32; 4], pixel: Pixel, options: BucketFill) -> f32 {
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
fn enqueue(
    visited: &mut BTreeMap<(u32, u32), [u64; 64]>,
    queue: &mut VecDeque<(u32, u32)>,
    x: u32,
    y: u32,
) -> Result<()> {
    let key = (x / 64, y / 64);
    if !visited.contains_key(&key) && visited.len() == MAX_FILL_TILES {
        return Err(Error::ResourceLimit(
            "bucket visited tile limit; no pixels changed",
        ));
    }
    let row = &mut visited.entry(key).or_insert([0; 64])[(y % 64) as usize];
    let bit = 1u64 << (x % 64);
    if *row & bit == 0 {
        *row |= bit;
        queue.push_back((x, y));
    }
    Ok(())
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
        let mut matches: BTreeMap<(u32, u32), Box<[f32; 4096]>> = BTreeMap::new();
        let mut visited = BTreeMap::new();
        let mut queue = VecDeque::new();
        if options.contiguous {
            enqueue(&mut visited, &mut queue, options.x, options.y)?;
        }
        let mut scan = 0u64;
        loop {
            let (x, y) = if options.contiguous {
                let Some(point) = queue.pop_front() else {
                    break;
                };
                point
            } else {
                if scan == u64::from(self.width) * u64::from(self.height) {
                    break;
                }
                let point = (
                    (scan % u64::from(self.width)) as u32,
                    (scan / u64::from(self.width)) as u32,
                );
                scan += 1;
                point
            };
            let selected = if self.selection.enabled() {
                self.selection_cache
                    .row(&self.selection, i64::from(x), i64::from(y), 1)[0]
            } else {
                1.
            };
            if selected == 0. {
                continue;
            }
            let weight = match_weight(reference, sample(self, x, y)?, options);
            if weight == 0. {
                continue;
            }
            let key = (x / 64, y / 64);
            if !matches.contains_key(&key) && matches.len() == MAX_FILL_TILES {
                return Err(Error::ResourceLimit(
                    "bucket match tile limit; no pixels changed",
                ));
            }
            matches.entry(key).or_insert_with(|| Box::new([0.; 4096]))
                [((y % 64) * 64 + x % 64) as usize] = weight * selected;
            if options.contiguous {
                if x > 0 {
                    enqueue(&mut visited, &mut queue, x - 1, y)?;
                }
                if x + 1 < self.width {
                    enqueue(&mut visited, &mut queue, x + 1, y)?;
                }
                if y > 0 {
                    enqueue(&mut visited, &mut queue, x, y - 1)?;
                }
                if y + 1 < self.height {
                    enqueue(&mut visited, &mut queue, x, y + 1)?;
                }
            }
        }
        drop(queue);
        drop(visited);
        let appearance = layer.appearance;
        let alpha_locked = !layer.mask && appearance.locks & crate::LOCK_TRANSPARENCY != 0;
        let color = compare_color(options.color);
        let mask_tone = 1. - (0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2]) / 255.;
        let mut staged: BTreeMap<TileCoord, Vec<Pixel>> = BTreeMap::new();
        for ((tx, ty), weights) in matches {
            for (offset, &weight) in weights.iter().enumerate() {
                if weight == 0. {
                    continue;
                }
                let x = i64::from(tx * 64) + (offset % 64) as i64 - i64::from(appearance.offset_x);
                let y = i64::from(ty * 64) + (offset / 64) as i64 - i64::from(appearance.offset_y);
                let coord =
                    TileCoord::from_signed(x.div_euclid(64) as i32, y.div_euclid(64) as i32);
                let before = layer.canvas_pixel(
                    (tx * 64) + (offset % 64) as u32,
                    (ty * 64) + (offset / 64) as u32,
                )?;
                let amount = weight * options.opacity;
                let after = if layer.mask {
                    Pixel::from_straight([
                        0.,
                        0.,
                        0.,
                        before.alpha() * (1. - amount * options.color.alpha())
                            + mask_tone * amount * options.color.alpha(),
                    ])?
                } else if alpha_locked {
                    before.paint_preserving_alpha(options.color, amount)
                } else {
                    options.color.scaled(amount).over(before)
                };
                if after == before {
                    continue;
                }
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
                                    x.div_euclid(64) * 64 + i % 64,
                                    y.div_euclid(64) * 64 + i / 64,
                                )
                            })
                            .collect()
                    };
                    staged.insert(coord, pixels);
                }
                staged.get_mut(&coord).expect("staged tile")
                    [(y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize] = after;
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
    fn visited_working_set_and_history_failure_are_bounded_and_atomic() {
        let mut visited = BTreeMap::new();
        let mut queue = VecDeque::new();
        for x in 0..MAX_FILL_TILES as u32 {
            enqueue(&mut visited, &mut queue, x * 64, 0).unwrap();
        }
        assert!(enqueue(&mut visited, &mut queue, MAX_FILL_TILES as u32 * 64, 0).is_err());
        assert_eq!(visited.len(), MAX_FILL_TILES);
        assert_eq!(queue.len(), MAX_FILL_TILES);
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
