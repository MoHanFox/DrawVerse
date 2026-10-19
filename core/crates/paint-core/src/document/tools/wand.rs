use super::bucket::{compare_color, match_weight};
use super::color_region::RegionDiscovery;
use super::*;
use crate::{HistoryAction, SelectionMask, SelectionOperation};

#[derive(Clone, Copy, Debug)]
pub struct WandSelection {
    pub x: u32,
    pub y: u32,
    pub sample_size: u32,
    pub tolerance: u32,
    pub antialias: bool,
    pub contiguous: bool,
    pub all_layers: bool,
    pub operation: SelectionOperation,
}
impl WandSelection {
    pub fn validate(&self) -> Result<()> {
        if ![1, 3, 5, 11, 31, 51, 101].contains(&self.sample_size)
            || self.tolerance > 255
            || self.operation == SelectionOperation::Invert
        {
            return Err(Error::InvalidArgument("wand parameters"));
        }
        Ok(())
    }
}
impl Document {
    pub fn select_wand(&mut self, options: WandSelection) -> Result<()> {
        self.idle()?;
        options.validate()?;
        if options.x >= self.width || options.y >= self.height {
            return Err(Error::InvalidArgument("wand seed"));
        }
        let layer = self.layers[self.layer_index(self.active_layer)?].clone();
        if !options.all_layers && layer.group {
            return Err(Error::InvalidArgument("wand requires pixel layer"));
        }
        if options.all_layers && layer.mask {
            return Err(Error::InvalidArgument("wand composite on mask"));
        }
        if !options.contiguous
            && u64::from(self.width) * u64::from(self.height)
                > crate::MAX_SELECTION_MASK_BYTES as u64
        {
            return Err(Error::ResourceLimit("wand global scan"));
        }
        let read_row = |x, y, count| {
            if options.all_layers {
                self.read_row(x, y, count)
            } else {
                layer.read_canvas_row(x, y, count)
            }
        };
        let radius = options.sample_size / 2;
        let mut reference = [0.; 4];
        let mut count = 0.;
        for y in
            options.y.saturating_sub(radius)..=options.y.saturating_add(radius).min(self.height - 1)
        {
            let left = options.x.saturating_sub(radius);
            let right = options.x.saturating_add(radius).min(self.width - 1);
            for pixel in read_row(left, y, right - left + 1)? {
                let c = compare_color(pixel);
                for i in 0..4 {
                    reference[i] += c[i];
                }
                count += 1.;
            }
        }
        for v in &mut reference {
            *v /= count;
        }
        let compare = BucketFill {
            x: options.x,
            y: options.y,
            color: Pixel::WHITE,
            opacity: 1.,
            tolerance: options.tolerance,
            antialias: options.antialias,
            contiguous: options.contiguous,
            all_layers: options.all_layers,
        };
        let mut discovery = RegionDiscovery::new(self.width, self.height, |x, y, count| {
            Ok(read_row(x, y, count)?
                .into_iter()
                .map(|pixel| {
                    let weight = (match_weight(reference, pixel, compare) * 255.).round() as u8;
                    (weight, weight != 0)
                })
                .collect())
        });
        if options.contiguous {
            discovery.contiguous(options.x, options.y)?;
        } else {
            discovery.global()?;
        }
        let (mut left, mut top, mut right, mut bottom) = (self.width, self.height, 0, 0);
        for (&(tx, ty), tile) in &discovery.tiles {
            for (y, &bits) in tile.selected.iter().enumerate() {
                if bits != 0 {
                    left = left.min(tx * 64 + bits.trailing_zeros());
                    right = right.max(tx * 64 + 63 - bits.leading_zeros());
                    top = top.min(ty * 64 + y as u32);
                    bottom = bottom.max(ty * 64 + y as u32);
                }
            }
        }
        // A zero mask is a real empty region, so all four combination rules remain consistent.
        let (left, top, width, height) = if left == self.width {
            (0, 0, 1, 1)
        } else {
            (left, top, right - left + 1, bottom - top + 1)
        };
        let bytes = u64::from(width) * u64::from(height);
        if bytes > crate::MAX_SELECTION_MASK_BYTES as u64 {
            return Err(Error::ResourceLimit("wand bounding mask"));
        }
        let mut alpha = vec![0; bytes as usize];
        for ((tx, ty), tile) in discovery.tiles {
            for (ly, mut bits) in tile.selected.into_iter().enumerate() {
                while bits != 0 {
                    let lx = bits.trailing_zeros();
                    bits &= bits - 1;
                    let (x, y) = (tx * 64 + lx, ty * 64 + ly as u32);
                    alpha[((y - top) * width + x - left) as usize] =
                        tile.weights[ly * 64 + lx as usize];
                }
            }
        }
        let mut shape = crate::SelectionShape::geometry(
            crate::SelectionKind::Mask,
            left.into(),
            top.into(),
            width.into(),
            height.into(),
            options.antialias,
        );
        shape.mask = Some(SelectionMask::from_alpha(width, height, alpha)?);
        shape.antialias = options.antialias;
        let next = self
            .selection
            .apply(shape, options.operation, self.width, self.height)?;
        if next.to_text().len() > 1024 * 1024 {
            return Err(Error::ResourceLimit("wand selection encoding"));
        }
        self.set_selection_with_action(next, HistoryAction::MagicSelection)
    }
}
