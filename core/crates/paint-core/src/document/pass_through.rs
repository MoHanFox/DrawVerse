use super::*;

pub(super) fn composite_row(layers: &[Layer], x: u32, y: u32, count: u32) -> Result<Vec<Pixel>> {
    let mut renderer = Rows {
        shown: layers
            .iter()
            .zip(visibility(layers))
            .map(|(l, v)| (l.id, v))
            .collect(),
        intrinsic: std::cell::RefCell::new(BTreeMap::new()),
        children: BTreeMap::new(),
        masks: BTreeMap::new(),
        x,
        y,
        count,
    };
    for layer in layers {
        if layer.mask {
            renderer.masks.insert(layer.parent, layer);
        } else {
            renderer
                .children
                .entry(layer.parent)
                .or_default()
                .push(layer);
        }
    }
    renderer.stack(0, vec![Pixel::TRANSPARENT; count as usize])
}
struct Rows<'a> {
    shown: BTreeMap<u64, bool>,
    intrinsic: std::cell::RefCell<BTreeMap<u64, Vec<Pixel>>>,
    children: BTreeMap<u64, Vec<&'a Layer>>,
    masks: BTreeMap<u64, &'a Layer>,
    x: u32,
    y: u32,
    count: u32,
}
impl Rows<'_> {
    fn shown(&self, l: &Layer) -> bool {
        self.shown[&l.id]
    }
    fn mask(&self, l: &Layer) -> Result<Vec<f32>> {
        if let Some(mask) = self.masks.get(&l.id).filter(|m| self.shown(m)) {
            Ok(mask
                .read_canvas_row(self.x, self.y, self.count)?
                .iter()
                .map(|p| 1. - p.alpha() * mask.properties.opacity)
                .collect())
        } else {
            Ok(vec![1.; self.count as usize])
        }
    }
    fn raw(&self, l: &Layer) -> Result<Vec<Pixel>> {
        if !self.shown(l) {
            return Ok(vec![Pixel::TRANSPARENT; self.count as usize]);
        }
        if l.group {
            if let Some(row) = self.intrinsic.borrow().get(&l.id) {
                return Ok(row.clone());
            }
            let row = self.stack(l.id, vec![Pixel::TRANSPARENT; self.count as usize])?;
            self.intrinsic.borrow_mut().insert(l.id, row.clone());
            Ok(row)
        } else {
            l.read_canvas_row(self.x, self.y, self.count)
        }
    }
    fn source(&self, l: &Layer) -> Result<Vec<Pixel>> {
        let mut pixels = self.raw(l)?;
        for (p, m) in pixels.iter_mut().zip(self.mask(l)?) {
            *p = p.scaled(m);
        }
        Ok(pixels)
    }
    fn clipped(&self, l: &Layer, pixels: &mut [Pixel], coverage: &[f32]) -> Result<()> {
        if !self.shown(l) {
            return Ok(());
        }
        let weight = l.properties.opacity * l.appearance.fill;
        if l.appearance.blend == crate::BlendMode::PassThrough {
            let after = self.stack(l.id, pixels.to_vec())?;
            for (((dst, src), mask), coverage) in pixels
                .iter_mut()
                .zip(after)
                .zip(self.mask(l)?)
                .zip(coverage)
            {
                if *dst == src {
                    continue;
                }
                let a = src.alpha();
                let color = if a == 0. {
                    Pixel::TRANSPARENT
                } else {
                    let c = src.components();
                    Pixel::from_straight([c[0] / a, c[1] / a, c[2] / a, dst.alpha()])?
                };
                *dst = dst.mix(color, weight * mask * coverage);
            }
        } else {
            let source = self.source(l)?;
            for (i, ((dst, src), coverage)) in
                pixels.iter_mut().zip(source).zip(coverage).enumerate()
            {
                let a = dst.alpha();
                if a == 0. {
                    continue;
                }
                let next = if l.appearance.blend == crate::BlendMode::Normal {
                    dst.paint_preserving_alpha(src, weight)
                } else {
                    let c = dst.components();
                    let opaque = Pixel::from_straight([
                        (c[0] / a).clamp(0., 1.),
                        (c[1] / a).clamp(0., 1.),
                        (c[2] / a).clamp(0., 1.),
                        1.,
                    ])?;
                    crate::blend_pixel(
                        src.scaled(weight),
                        opaque,
                        l.appearance.blend,
                        self.x + i as u32,
                        self.y,
                        u64::from(l.appearance.dissolve_seed),
                    )
                    .scaled(a)
                };
                *dst = dst.mix(next, *coverage);
            }
        }
        Ok(())
    }
    fn stack(&self, parent: u64, mut output: Vec<Pixel>) -> Result<Vec<Pixel>> {
        let Some(children) = self.children.get(&parent) else {
            return Ok(output);
        };
        let mut at = 0;
        while at < children.len() {
            let base = children[at];
            let mut end = at + 1;
            while end < children.len() && children[end].clipped {
                end += 1;
            }
            if base.clipped || !self.shown(base) {
                at = end;
                continue;
            }
            let weight = base.properties.opacity * base.appearance.fill;
            if base.appearance.blend == crate::BlendMode::PassThrough {
                let mut after = self.stack(base.id, output.clone())?;
                if end > at + 1 {
                    let coverage: Vec<_> = self.raw(base)?.iter().map(|p| p.alpha()).collect();
                    for clipped in &children[at + 1..end] {
                        self.clipped(clipped, &mut after, &coverage)?;
                    }
                }
                for ((dst, src), mask) in output.iter_mut().zip(after).zip(self.mask(base)?) {
                    if *dst != src {
                        *dst = dst.mix(src, weight * mask);
                    }
                }
            } else {
                let mut source = self.source(base)?;
                let coverage = vec![1.; self.count as usize];
                for clipped in &children[at + 1..end] {
                    self.clipped(clipped, &mut source, &coverage)?;
                }
                for (i, (dst, src)) in output.iter_mut().zip(source).enumerate() {
                    *dst = crate::blend_pixel(
                        src.scaled(weight),
                        *dst,
                        base.appearance.blend,
                        self.x + i as u32,
                        self.y,
                        u64::from(base.appearance.dissolve_seed),
                    );
                }
            }
            at = end;
        }
        Ok(output)
    }
}
