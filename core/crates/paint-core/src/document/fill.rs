//! Paint-bucket fill.
//!
//! The bucket is the magic wand plus paint: the region comes from the same scanline flood fill, and
//! the pixels are written through the same dab path as a brush stroke, so one fill is one history
//! entry with per-tile before/after snapshots and the usual rollback guarantees.

use crate::{Brush, Document, Error, InputPoint, Selection, SelectionOperation, SelectionShape};

impl Document {
    /// Install the temporary fill region. Paired with [`Document::end_fill_selection`] so every path
    /// restores the document's own selection.
    pub(crate) fn begin_fill_selection(&mut self, region: Selection) {
        self.fill_selection = Some(region);
        self.filling = true;
    }

    /// Drop the temporary fill region and stop labelling strokes as fills.
    pub(crate) fn end_fill_selection(&mut self) {
        self.fill_selection = None;
        self.filling = false;
    }

    /// Fill the region similar to the pixel under `(seed_x, seed_y)` with the given colour.
    ///
    /// `tolerance` is 0..=1 over the normalized channel distance, `opacity` is the brush flow, and
    /// `contiguous` chooses between the connected region and every similar pixel in the document.
    /// Returns whether any pixel changed, so an empty fill produces no history entry.
    ///
    /// The user's own selection is **not** touched: the region drives a temporary fill selection that
    /// exists only for the duration of the call, on every path including failures.
    pub fn fill_region(
        &mut self,
        seed_x: u32,
        seed_y: u32,
        tolerance: f32,
        contiguous: bool,
        color: crate::Pixel,
        opacity: f32,
    ) -> crate::Result<bool> {
        self.idle()?;
        if !opacity.is_finite() || !(0. ..=1.).contains(&opacity) {
            return Err(Error::InvalidArgument(
                "fill opacity must be finite in 0..=1",
            ));
        }
        if opacity == 0. {
            return Err(Error::InvalidArgument("fill opacity must be above zero"));
        }
        if color.alpha() == 0. {
            return Err(Error::InvalidArgument("fill colour is fully transparent"));
        }
        for channel in color.components() {
            if !channel.is_finite() {
                return Err(Error::InvalidArgument("fill colour must be finite"));
            }
        }
        let shape = crate::wand_shape_with(self, seed_x, seed_y, tolerance, contiguous)?;
        // The region itself is resolved before anything is written, so an invalid mask cannot leave a
        // half-filled layer behind.
        let (min_x, min_y, max_x, max_y) = shape_bounds(&shape)?;
        let selection = Selection::default().apply(
            shape,
            SelectionOperation::Replace,
            self.width,
            self.height,
        )?;
        if min_x > max_x || min_y > max_y {
            return Ok(false);
        }
        // Sweep the region with a grid of dabs small enough to pass brush validation. One huge
        // circle is not an option: its radius exceeds the brush limit as soon as the region is wider
        // than a few hundred pixels, which is most fills. The mask still decides the exact pixels.
        let brush = Brush {
            radius: DAB_RADIUS,
            color,
            opacity,
            ..Default::default()
        };
        // A guard, not a plain assignment: every exit path has to restore the document's own
        // selection, including the error paths inside the stroke lifecycle.
        self.begin_fill_selection(selection);
        let result = (|| -> crate::Result<bool> {
            let mut first = true;
            let mut changed = false;
            // Start at the centre of the bounds so a small region is always covered by one dab, then
            // step outwards on the grid from there.
            let centre_x = (f64::from(min_x) + f64::from(max_x) + 1.) / 2.;
            let centre_y = (f64::from(min_y) + f64::from(max_y) + 1.) / 2.;
            let step = f64::from(DAB_STEP);
            let mut y = centre_y - step * ((centre_y - f64::from(min_y)) / step).floor();
            while y <= f64::from(max_y) {
                let mut x = centre_x - step * ((centre_x - f64::from(min_x)) / step).floor();
                while x <= f64::from(max_x) {
                    let point = InputPoint::new(x, y, 1.);
                    if first {
                        self.begin_stroke(brush, point)?;
                        first = false;
                    } else {
                        self.stroke_to(point)?;
                    }
                    x += step;
                }
                y += step;
            }
            if !first {
                changed = self.end_stroke()?;
            }
            Ok(changed)
        })();
        self.end_fill_selection();
        result
    }
}

/// Dab size and spacing for the fill sweep. Radius 48 with a 64 step gives each dab a 68px
/// diagonal reach against a 45px cell half-diagonal, so the grid tiles the region without gaps.
const DAB_RADIUS: f32 = 48.;
const DAB_STEP: u32 = 64;

/// Bounding box of a rasterized shape, in document pixels.
fn shape_bounds(shape: &SelectionShape) -> crate::Result<(u32, u32, u32, u32)> {    if !shape.validate().is_ok() {
        return Err(Error::InvalidArgument("invalid fill region"));
    }
    let min_x = shape.x.max(0.).floor() as u32;
    let min_y = shape.y.max(0.).floor() as u32;
    let max_x = (shape.x + shape.width).max(0.).ceil() as u32;
    let max_y = (shape.y + shape.height).max(0.).ceil() as u32;
    if max_x == 0 || max_y == 0 {
        return Ok((1, 1, 0, 0));
    }
    Ok((min_x, min_y, max_x - 1, max_y - 1))
}
