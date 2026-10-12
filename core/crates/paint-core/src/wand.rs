use super::bucket::{compare_color, enqueue, match_weight, MAX_FILL_TILES};
use super::*;
use crate::{HistoryAction, SelectionMask, SelectionOperation};
use std::collections::VecDeque;

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
        let sample = |x, y| {
            if options.all_layers {
                self.try_pixel(x, y)
            } else {
                layer.canvas_pixel(x, y)
            }
        };
        let radius = options.sample_size / 2;
        let mut reference = [0.; 4];
        let mut count = 0.;
        for y in
            options.y.saturating_sub(radius)..=options.y.saturating_add(radius).min(self.height - 1)
        {
            for x in options.x.saturating_sub(radius)
                ..=options.x.saturating_add(radius).min(self.width - 1)
            {
                let c = compare_color(sample(x, y)?);
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
        let mut visited = BTreeMap::new();
        let mut queue = VecDeque::new();
        let mut matches: BTreeMap<(u32, u32), Box<[u8; 4096]>> = BTreeMap::new();
        let (mut left, mut top, mut right, mut bottom) = (self.width, self.height, 0, 0);
        if options.contiguous {
            enqueue(&mut visited, &mut queue, options.x, options.y)?;
        }
        let mut scan = 0u64;
        loop {
            let (x, y) = if options.contiguous {
                let Some(p) = queue.pop_front() else { break };
                p
            } else {
                if scan == u64::from(self.width) * u64::from(self.height) {
                    break;
                }
                let p = (
                    (scan % u64::from(self.width)) as u32,
                    (scan / u64::from(self.width)) as u32,
                );
                scan += 1;
                p
            };
            let weight = (match_weight(reference, sample(x, y)?, compare) * 255.).round() as u8;
            if weight == 0 {
                continue;
            }
            let key = (x / 64, y / 64);
            if !matches.contains_key(&key) && matches.len() == MAX_FILL_TILES {
                return Err(Error::ResourceLimit("wand match tiles"));
            }
            matches.entry(key).or_insert_with(|| Box::new([0; 4096]))
                [((y % 64) * 64 + x % 64) as usize] = weight;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
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
        // A zero mask is a real empty region, so all four combination rules remain consistent.
        let (left, top, width, height) = if matches.is_empty() {
            (0, 0, 1, 1)
        } else {
            (left, top, right - left + 1, bottom - top + 1)
        };
        let bytes = u64::from(width) * u64::from(height);
        if bytes > crate::MAX_SELECTION_MASK_BYTES as u64 {
            return Err(Error::ResourceLimit("wand bounding mask"));
        }
        let mut alpha = vec![0; bytes as usize];
        for ((tx, ty), weights) in matches {
            for (i, &v) in weights.iter().enumerate() {
                if v == 0 {
                    continue;
                }
                let x = tx * 64 + (i % 64) as u32;
                let y = ty * 64 + (i / 64) as u32;
                alpha[((y - top) * width + x - left) as usize] = v;
            }
        }
        let mut shape = crate::mask_shape(left, top, width, height, vec![0; bytes as usize])?;
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
