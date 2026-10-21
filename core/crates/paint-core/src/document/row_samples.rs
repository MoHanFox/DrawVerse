//! The same compositor accepts dense rows and ordered display sample coordinates.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum RowSamples<'a> {
    Dense { x: u32, count: u32 },
    Sparse(&'a [u32]),
}

impl RowSamples<'_> {
    pub fn len(self) -> usize {
        match self {
            Self::Dense { count, .. } => count as usize,
            Self::Sparse(xs) => xs.len(),
        }
    }

    pub fn x(self, index: usize) -> u32 {
        match self {
            Self::Dense { x, .. } => x + index as u32,
            Self::Sparse(xs) => xs[index],
        }
    }

    pub fn read(self, layer: &Layer, y: u32) -> Result<Vec<Pixel>> {
        let xs = match self {
            Self::Dense { x, count } => return layer.read_canvas_row(x, y, count),
            Self::Sparse(xs) => xs,
        };
        let y = i64::from(y) - i64::from(layer.appearance.offset_y);
        let mut output = vec![Pixel::TRANSPARENT; xs.len()];
        let mut first = 0;
        while first < xs.len() {
            let x = i64::from(xs[first]) - i64::from(layer.appearance.offset_x);
            let tile_x = x.div_euclid(64);
            let mut end = first + 1;
            while end < xs.len()
                && (i64::from(xs[end]) - i64::from(layer.appearance.offset_x)).div_euclid(64)
                    == tile_x
            {
                end += 1;
            }
            // One fallible page pin per sampled tile run; no reads of intervening
            // tiles or compositing of pixels that the display will discard.
            if let Some(tile) = layer.tiles.get(&TileCoord::from_signed(
                tile_x as i32,
                y.div_euclid(64) as i32,
            )) {
                let pixels = tile.try_pixels()?;
                for i in first..end {
                    let x = i64::from(xs[i]) - i64::from(layer.appearance.offset_x);
                    output[i] = pixels[(y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize];
                }
            } else {
                for i in first..end {
                    output[i] = layer
                        .default_pixel(i64::from(xs[i]) - i64::from(layer.appearance.offset_x), y);
                }
            }
            first = end;
        }
        Ok(output)
    }
}
