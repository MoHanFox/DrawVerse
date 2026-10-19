//! Shared bounded scanline region discovery for the wand and bucket.
use super::bucket::MAX_FILL_TILES;
use crate::{Error, Result};
use std::collections::HashMap;

pub(super) struct RegionTile<W> {
    pub weights: [W; 4096],
    eligible: [u64; 64],
    classified: u64,
    pub selected: [u64; 64],
}

/// Classify each touched source row once, with batch reads (including group/mask composition).
/// Only scalar weights survive the read: no pixel pins or full-size composite are retained.
pub(super) struct RegionDiscovery<W, F> {
    pub tiles: HashMap<(u32, u32), Box<RegionTile<W>>>,
    read_row: F,
    width: u32,
    height: u32,
}

impl<W: Copy + Default, F: FnMut(u32, u32, u32) -> Result<Vec<(W, bool)>>> RegionDiscovery<W, F> {
    pub fn new(width: u32, height: u32, read_row: F) -> Self {
        Self {
            tiles: HashMap::new(),
            read_row,
            width,
            height,
        }
    }
    pub fn global(&mut self) -> Result<()> {
        for ty in 0..self.height.div_ceil(64) {
            for tx in 0..self.width.div_ceil(64) {
                // Global scans retain matching tiles only. A long thin canvas may touch more
                // than 4096 source tiles while remaining within the existing pixel budget.
                let mut tile = Box::new(RegionTile {
                    weights: [W::default(); 4096],
                    eligible: [0; 64],
                    selected: [0; 64],
                    classified: 0,
                });
                for y in ty * 64..self.height.min(ty * 64 + 64) {
                    let row = (self.read_row)(tx * 64, y, (self.width - tx * 64).min(64))?;
                    for (x, (weight, eligible)) in row.into_iter().enumerate() {
                        tile.weights[(y % 64) as usize * 64 + x] = weight;
                        if eligible {
                            tile.selected[(y % 64) as usize] |= 1 << x;
                        }
                    }
                }
                if tile.selected.iter().any(|&bits| bits != 0) {
                    if self.tiles.len() == MAX_FILL_TILES {
                        return Err(Error::ResourceLimit("region match tiles"));
                    }
                    self.tiles.insert((tx, ty), tile);
                }
            }
        }
        Ok(())
    }
    fn row(&mut self, tx: u32, y: u32) -> Result<&mut RegionTile<W>> {
        let key = (tx, y / 64);
        if !self.tiles.contains_key(&key) {
            if self.tiles.len() == MAX_FILL_TILES {
                return Err(Error::ResourceLimit("region visited tiles"));
            }
            self.tiles.insert(
                key,
                Box::new(RegionTile {
                    weights: [W::default(); 4096],
                    eligible: [0; 64],
                    selected: [0; 64],
                    classified: 0,
                }),
            );
        }
        let row_bit = 1 << (y % 64);
        if self.tiles[&key].classified & row_bit == 0 {
            let count = (self.width - tx * 64).min(64);
            let row = (self.read_row)(tx * 64, y, count)?;
            let tile = self.tiles.get_mut(&key).expect("allocated tile");
            for (x, (weight, eligible)) in row.into_iter().enumerate() {
                tile.weights[(y % 64) as usize * 64 + x] = weight;
                if eligible {
                    tile.eligible[(y % 64) as usize] |= 1 << x;
                }
            }
            tile.classified |= row_bit;
        }
        Ok(self.tiles.get_mut(&key).expect("classified tile"))
    }
    fn remaining(&mut self, tx: u32, y: u32) -> Result<u64> {
        let t = self.row(tx, y)?;
        Ok(t.eligible[(y % 64) as usize] & !t.selected[(y % 64) as usize])
    }
    fn select_run(&mut self, left: u32, right: u32, y: u32) -> Result<()> {
        let mut x = left;
        while x <= right {
            let end = right.min(x / 64 * 64 + 63);
            let bits = (u64::MAX << (x % 64)) & (u64::MAX >> (63 - end % 64));
            self.row(x / 64, y)?.selected[(y % 64) as usize] |= bits;
            x = end + 1;
        }
        Ok(())
    }
    pub fn contiguous(&mut self, x: u32, y: u32) -> Result<()> {
        let mut stack = vec![(x, y)];
        while let Some((x, y)) = stack.pop() {
            if self.remaining(x / 64, y)? & (1 << (x % 64)) == 0 {
                continue;
            }
            let (mut left, mut right) = (x, x);
            loop {
                let bits = self.remaining(left / 64, y)? << (63 - left % 64);
                let count = bits.leading_ones();
                if count == 0 {
                    left += 1;
                    break;
                }
                left = left + 1 - count;
                if left == 0 || left % 64 != 0 {
                    break;
                }
                left -= 1;
            }
            loop {
                let bits = self.remaining(right / 64, y)? >> (right % 64);
                let count = bits.trailing_ones();
                if count == 0 {
                    right -= 1;
                    break;
                }
                right += count - 1;
                if right + 1 == self.width || right % 64 != 63 {
                    break;
                }
                right += 1;
            }
            self.select_run(left, right, y)?;
            for ny in [y.wrapping_sub(1), y + 1] {
                if ny >= self.height {
                    continue;
                }
                let mut in_run = false;
                let mut start = left;
                while start <= right {
                    let end = right.min(start / 64 * 64 + 63);
                    let mut bits = self.remaining(start / 64, ny)?
                        & (u64::MAX << (start % 64))
                        & (u64::MAX >> (63 - end % 64));
                    while bits != 0 {
                        let offset = bits.trailing_zeros();
                        let length = (bits >> offset).trailing_ones();
                        if offset != start % 64 || !in_run {
                            if stack.len() == MAX_FILL_TILES * 64 {
                                return Err(Error::ResourceLimit("region pending spans"));
                            }
                            stack.push((start / 64 * 64 + offset, ny));
                        }
                        in_run = offset + length == end % 64 + 1;
                        bits &= !(u64::MAX >> (64 - length) << offset);
                    }
                    if self.remaining(start / 64, ny)? & (1 << (end % 64)) == 0 {
                        in_run = false;
                    }
                    start = end + 1;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, collections::VecDeque};
    #[test]
    fn spans_match_four_neighbor_reference_across_tile_edges_and_holes() {
        for (w, h) in [(1, 1), (63, 3), (64, 65), (65, 64), (137, 83)] {
            for pattern in 0..8 {
                let weight = |x: u32, y: u32| {
                    let hash = x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663) ^ pattern;
                    if x % 7 == 0 || y % 11 == 0 || hash % 5 < 2 {
                        if hash % 2 == 0 {
                            255u8
                        } else {
                            1
                        }
                    } else {
                        0
                    }
                };
                for (sx, sy) in [(0, 0), (w / 2, h / 2), (w - 1, h - 1)] {
                    let reads = Cell::new(0);
                    let mut region = RegionDiscovery::new(w, h, |x, y, count| {
                        reads.set(reads.get() + 1);
                        Ok((x..x + count)
                            .map(|x| {
                                let v = weight(x, y);
                                (v, v != 0)
                            })
                            .collect())
                    });
                    region.contiguous(sx, sy).unwrap();
                    let mut expected = vec![false; (w * h) as usize];
                    let mut queue = VecDeque::from([(sx, sy)]);
                    while let Some((x, y)) = queue.pop_front() {
                        let i = (y * w + x) as usize;
                        if expected[i] || weight(x, y) == 0 {
                            continue;
                        }
                        expected[i] = true;
                        for (x, y) in [
                            (x.wrapping_sub(1), y),
                            (x + 1, y),
                            (x, y.wrapping_sub(1)),
                            (x, y + 1),
                        ] {
                            if x < w && y < h {
                                queue.push_back((x, y));
                            }
                        }
                    }
                    for y in 0..h {
                        for x in 0..w {
                            let tile = region.tiles.get(&(x / 64, y / 64));
                            let selected = tile.is_some_and(|t| {
                                t.selected[(y % 64) as usize] & (1 << (x % 64)) != 0
                            });
                            assert_eq!(
                                selected,
                                expected[(y * w + x) as usize],
                                "{w}x{h} pattern={pattern} seed={sx},{sy} pixel={x},{y}"
                            );
                            if selected {
                                assert_eq!(
                                    tile.unwrap().weights[((y % 64) * 64 + x % 64) as usize],
                                    weight(x, y)
                                );
                            }
                        }
                    }
                    assert!(reads.get() <= region.tiles.len() * 64);
                }
            }
        }
    }
    #[test]
    fn weight_precision_and_connectivity_are_independent() {
        let mut region = RegionDiscovery::new(65, 1, |_, _, count| {
            Ok(vec![(f32::MIN_POSITIVE, true); count as usize])
        });
        region.contiguous(64, 0).unwrap();
        assert_eq!(region.tiles[&(0, 0)].selected[0], u64::MAX);
        assert_eq!(region.tiles[&(0, 0)].weights[0], f32::MIN_POSITIVE);
        assert_eq!(region.tiles[&(1, 0)].selected[0], 1);
    }
    #[test]
    fn tile_budget_is_enforced_before_sampling_or_allocating_another_tile() {
        let mut region =
            RegionDiscovery::new((MAX_FILL_TILES as u32 + 1) * 64, 1, |_, _, count| {
                Ok(vec![(255u8, true); count as usize])
            });
        assert!(matches!(
            region.contiguous(0, 0),
            Err(Error::ResourceLimit(_))
        ));
        assert_eq!(region.tiles.len(), MAX_FILL_TILES);
    }
    #[test]
    fn global_scan_retains_matches_only_on_long_thin_documents() {
        let mut region =
            RegionDiscovery::new((MAX_FILL_TILES as u32 + 1) * 64, 1, |x, _, count| {
                Ok((x..x + count).map(|x| (255u8, x == 0)).collect())
            });
        region.global().unwrap();
        assert_eq!(region.tiles.len(), 1);
        assert_eq!(region.tiles[&(0, 0)].selected[0], 1);
    }
}
