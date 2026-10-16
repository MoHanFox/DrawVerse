//! Bounded display-only mask raster, in the same unit square as the tip outline.
use crate::BrushMask;

pub const BRUSH_CURSOR_IMAGE_EDGE: u32 = 256;

// Twice-source coordinates avoid fractional padding for odd aspect ratios.
// Integral of a linear tent, scaled by 2*radius: every weight is an exact integer.
fn tent_integral(position: i64, center: i64, radius: i64) -> u64 {
    let z = position - center;
    if z <= -radius {
        0
    } else if z < 0 {
        ((z + radius) * (z + radius)) as u64
    } else if z < radius {
        (2 * radius * radius - (radius - z) * (radius - z)) as u64
    } else {
        (2 * radius * radius) as u64
    }
}
fn axis_weights(size: u32, extent: u32) -> Vec<Vec<(usize, u64)>> {
    let edge = i64::from(BRUSH_CURSOR_IMAGE_EDGE);
    let extent = i64::from(extent);
    let size = i64::from(size);
    let padding = (extent - size) * edge;
    (0..edge)
        .map(|i| {
            let start = (2 * i * extent - padding).clamp(0, size * 2 * edge);
            let end = (2 * (i + 1) * extent - padding).clamp(0, size * 2 * edge);
            if start >= end {
                return Vec::new();
            }
            let first = (start - edge).div_euclid(2 * edge).clamp(0, size);
            let last = (end + 3 * edge - 1).div_euclid(2 * edge).clamp(0, size);
            let weight = |center| {
                tent_integral(end, center, 2 * edge) - tent_integral(start, center, 2 * edge)
            };
            (first..last)
                .filter_map(|s| {
                    let mut amount = weight((2 * s + 1) * edge);
                    // Same repeated edge samples as BrushMask's bilinear reconstruction.
                    if s == 0 {
                        amount += weight(-edge);
                    }
                    if s == size - 1 {
                        amount += weight((2 * size + 1) * edge);
                    }
                    (amount > 0).then_some((s as usize, amount))
                })
                .collect()
        })
        .collect()
}

impl BrushMask {
    /// White straight RGBA8; source aspect is centred in a unit-diameter square.
    /// Area coverage is a display representation, independent of painting mip selection.
    pub fn cursor_rgba(&self) -> Vec<u8> {
        let (width, height) = self.dimensions();
        let extent = width.max(height);
        let xs = axis_weights(width, extent);
        let ys = axis_weights(height, extent);
        let denominator = (2 * u64::from(extent) * 4 * u64::from(BRUSH_CURSOR_IMAGE_EDGE)).pow(2);
        let edge = BRUSH_CURSOR_IMAGE_EDGE as usize;
        let mut rgba = vec![255; edge * edge * 4];
        for (y, rows) in ys.iter().enumerate() {
            for (x, columns) in xs.iter().enumerate() {
                let mut sum = 0_u64;
                for &(sy, wy) in rows {
                    for &(sx, wx) in columns {
                        sum += u64::from(self.coverage()[sy * width as usize + sx]) * wx * wy;
                    }
                }
                rgba[(y * edge + x) * 4 + 3] = ((sum + denominator / 2) / denominator) as u8;
            }
        }
        rgba
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn alpha(image: &[u8], x: usize, y: usize) -> u8 {
        image[(y * 256 + x) * 4 + 3]
    }
    #[test]
    fn cursor_image_square_preserves_white_rgb_and_all_alpha_levels() {
        for a in [0, 1, 128, 254, 255] {
            let mask = BrushMask::new(1, 1, vec![a]).unwrap();
            assert!(mask
                .cursor_rgba()
                .chunks_exact(4)
                .all(|p| p == [255, 255, 255, a]));
            assert_eq!(mask.coverage(), &[a]);
        }
    }
    #[test]
    fn cursor_image_non_square_centered_holes_and_intermediate_alpha() {
        let mask = BrushMask::new(4, 2, vec![255, 0, 128, 255, 0, 255, 255, 128]).unwrap();
        let image = mask.cursor_rgba();
        assert_eq!(image.len(), 256 * 256 * 4);
        for y in 0..256 {
            for x in 0..256 {
                let expected = if (64..192).contains(&y) {
                    let sx = ((x as f64 + 0.5) / 64. - 0.5).clamp(0., 3.);
                    let sy = ((y as f64 - 64. + 0.5) / 64. - 0.5).clamp(0., 1.);
                    let ix = sx.floor() as usize;
                    let iy = sy.floor() as usize;
                    let fx = sx - ix as f64;
                    let fy = sy - iy as f64;
                    let at = |x, y| f64::from(mask.coverage()[y * 4 + x]);
                    let top = at(ix, iy) * (1. - fx) + at((ix + 1).min(3), iy) * fx;
                    let bottom = at(ix, (iy + 1).min(1)) * (1. - fx)
                        + at((ix + 1).min(3), (iy + 1).min(1)) * fx;
                    (top * (1. - fy) + bottom * fy).round() as u8
                } else {
                    0
                };
                assert_eq!(alpha(&image, x, y), expected);
            }
        }
    }
    #[test]
    fn cursor_image_odd_aspect_area_edges_are_symmetric() {
        let image = BrushMask::new(3, 1, vec![255; 3]).unwrap().cursor_rgba();
        for x in 0..256 {
            assert_eq!(alpha(&image, x, 85), 170);
            assert_eq!(alpha(&image, x, 170), 170);
            assert_eq!(alpha(&image, x, 84), 0);
            assert_eq!(alpha(&image, x, 171), 0);
            for y in 86..170 {
                assert_eq!(alpha(&image, x, y), 255);
            }
        }
    }
    #[test]
    fn cursor_image_integral_weights_partition_pixels_and_maximum_mask_stays_bounded() {
        let edge = i64::from(BRUSH_CURSOR_IMAGE_EDGE);
        for extent in [1, 3, 17, 256, 511, 4096] {
            for size in [1, extent / 2 + 1, extent] {
                let weights = axis_weights(size, extent);
                let padding = i64::from(extent - size) * edge;
                let mut entries = 0;
                for (i, values) in weights.iter().enumerate() {
                    let start = (2 * i as i64 * i64::from(extent) - padding)
                        .clamp(0, i64::from(size) * 2 * edge);
                    let end = (2 * (i as i64 + 1) * i64::from(extent) - padding)
                        .clamp(0, i64::from(size) * 2 * edge);
                    assert_eq!(
                        values.iter().map(|(_, w)| w).sum::<u64>(),
                        ((end - start) * 4 * edge) as u64
                    );
                    assert!(values.iter().all(|(s, w)| *s < size as usize && *w > 0));
                    entries += values.len();
                }
                assert!(entries <= size as usize + 2 * BRUSH_CURSOR_IMAGE_EDGE as usize);
            }
        }
        let mask = BrushMask::new(4096, 4096, vec![255; 4096 * 4096]).unwrap();
        assert!(mask.cursor_rgba().iter().all(|c| *c == 255));
    }
    #[test]
    fn cursor_image_high_frequency_average_and_extreme_thin_masks_survive() {
        let mask = BrushMask::new(
            512,
            512,
            (0..512 * 512)
                .map(|i| if (i % 512 + i / 512) % 2 == 0 { 0 } else { 255 })
                .collect(),
        )
        .unwrap();
        let image = mask.cursor_rgba();
        for y in 1..255 {
            for x in 1..255 {
                assert_eq!(alpha(&image, x, y), 128);
            }
        }
        assert_eq!(alpha(&image, 0, 0), 126);
        assert_eq!(alpha(&image, 255, 0), 129);
        for (w, h) in [(4096, 1), (1, 4096)] {
            let mask = BrushMask::new(w, h, vec![255; 4096]).unwrap();
            let image = mask.cursor_rgba();
            assert_eq!(alpha(&image, 127, 127), 8);
            assert_eq!(alpha(&image, 128, 128), 8);
            assert_eq!(alpha(&image, 0, 0), 0);
            let sum: u32 = image.chunks_exact(4).map(|p| u32::from(p[3])).sum();
            assert_eq!(sum, 4096);
            assert!(
                axis_weights(w, w.max(h))
                    .iter()
                    .map(Vec::len)
                    .sum::<usize>()
                    <= w as usize + 2 * 256
            );
        }
    }
}
