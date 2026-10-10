//! Magic-wand region extraction: flood fill over the composited document.
//!
//! The wand is a *document-dependent* selection, so the region is resolved once, here, and the
//! result is stored as an ordinary pixel mask in the selection. Everything downstream (painting
//! constraints, history, ORA) then treats it like any other selection step.

use crate::selection::MAX_SELECTION_MASK_BYTES;
use crate::{Document, Error, Result, SelectionShape};

/// Channel distance where two colours still count as the same region. Premultiplied alpha makes
/// fully transparent pixels all equal, so an empty area floods as one region.
fn similar(a: [f32; 4], b: [f32; 4], tolerance: f32) -> bool {
    let unpremultiply = |value: [f32; 4]| {
        let alpha = value[3];
        if alpha <= 0.000_01 {
            [0., 0., 0., 0.]
        } else {
            [value[0] / alpha, value[1] / alpha, value[2] / alpha, alpha]
        }
    };
    let left = unpremultiply(a);
    let right = unpremultiply(b);
    left.iter()
        .zip(right.iter())
        .all(|(x, y)| (x - y).abs() <= tolerance)
}

/// Flood fill the connected region around `(seed_x, seed_y)` and return it as a `Mask` shape.
/// `tolerance` is 0..=1 over the normalized channel distance.
pub fn wand_shape(
    document: &Document,
    seed_x: u32,
    seed_y: u32,
    tolerance: f32,
) -> Result<SelectionShape> {
    if !tolerance.is_finite() || !(0. ..=1.).contains(&tolerance) {
        return Err(Error::InvalidArgument("invalid wand tolerance"));
    }
    let (width, height) = document.dimensions();
    if seed_x >= width || seed_y >= height {
        return Err(Error::InvalidArgument("wand seed outside document"));
    }
    let bytes = (width as usize) * (height as usize);
    if bytes > MAX_SELECTION_MASK_BYTES {
        return Err(Error::ResourceLimit("selection mask size"));
    }
    let seed = document.try_pixel(seed_x, seed_y)?.components();
    let mut filled = vec![0u8; bytes];
    let index = |x: u32, y: u32| (y as usize) * (width as usize) + (x as usize);
    let matches = |document: &Document, x: u32, y: u32| -> Result<bool> {
        Ok(similar(
            document.try_pixel(x, y)?.components(),
            seed,
            tolerance,
        ))
    };

    // Scanline flood fill: each stack entry is a horizontal run of matching pixels.
    let mut stack = vec![(seed_x, seed_y)];
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    while let Some((start_x, row)) = stack.pop() {
        if filled[index(start_x, row)] != 0 {
            continue;
        }
        let mut left = start_x;
        while left > 0 && filled[index(left - 1, row)] == 0 && matches(document, left - 1, row)? {
            left -= 1;
        }
        let mut right = start_x;
        while right + 1 < width
            && filled[index(right + 1, row)] == 0
            && matches(document, right + 1, row)?
        {
            right += 1;
        }
        if !matches(document, start_x, row)? {
            continue;
        }
        for x in left..=right {
            filled[index(x, row)] = 1;
        }
        min_x = min_x.min(left);
        max_x = max_x.max(right);
        min_y = min_y.min(row);
        max_y = max_y.max(row);
        if row > 0 {
            seed_row(
                document,
                &filled,
                left,
                right,
                row - 1,
                width,
                &matches,
                &mut stack,
            )?;
        }
        if row + 1 < height {
            seed_row(
                document,
                &filled,
                left,
                right,
                row + 1,
                width,
                &matches,
                &mut stack,
            )?;
        }
    }
    if max_x < min_x || max_y < min_y {
        return Err(Error::InvalidArgument("empty wand region"));
    }
    // A wand that selects everything is a rectangle: no mask payload, no extra memory.
    if min_x == 0 && min_y == 0 && max_x == width - 1 && max_y == height - 1 {
        return Ok(SelectionShape::geometry(
            crate::SelectionKind::Rectangle,
            0.,
            0.,
            f64::from(width),
            f64::from(height),
            false,
        ));
    }
    let mask_width = max_x - min_x + 1;
    let mask_height = max_y - min_y + 1;
    let mut coverage = vec![0u8; (mask_width as usize) * (mask_height as usize)];
    for row in min_y..=max_y {
        for x in min_x..=max_x {
            if filled[index(x, row)] == 1 {
                coverage
                    [((row - min_y) as usize) * (mask_width as usize) + ((x - min_x) as usize)] = 1;
            }
        }
    }
    crate::selection::mask_shape(min_x, min_y, mask_width, mask_height, coverage)
}

#[allow(clippy::too_many_arguments)]
fn seed_row<F>(
    document: &Document,
    filled: &[u8],
    left: u32,
    right: u32,
    row: u32,
    width: u32,
    matches: &F,
    stack: &mut Vec<(u32, u32)>,
) -> Result<()>
where
    F: Fn(&Document, u32, u32) -> Result<bool>,
{
    let index = |x: u32, y: u32| (y as usize) * (width as usize) + (x as usize);
    let mut x = left;
    while x <= right {
        if filled[index(x, row)] == 0 && matches(document, x, row)? {
            stack.push((x, row));
            // The popped run expands left and right by itself, so skip this whole matching run.
            while x <= right && matches(document, x, row)? {
                x += 1;
            }
        }
        x += 1;
    }
    Ok(())
}
