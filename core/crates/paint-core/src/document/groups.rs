use super::*;

pub(crate) fn validate(layers: &[Layer]) -> Result<()> {
    if layers.is_empty() || layers.len() > 4096 || !layers.iter().any(|l| !l.group && !l.mask) {
        return Err(Error::InvalidArgument("invalid hierarchy node count"));
    }
    let index: BTreeMap<_, _> = layers.iter().enumerate().map(|(i, l)| (l.id, i)).collect();
    if index.len() != layers.len() || index.contains_key(&0) {
        return Err(Error::InvalidArgument("duplicate layer IDs"));
    }
    let mut masks = BTreeSet::new();
    let mut spans = BTreeMap::<u64, (usize, usize)>::new();
    for (i, l) in layers.iter().enumerate() {
        if l.mask
            && (l.group
                || l.clipped
                || l.parent == 0
                || l.white != (0, 0)
                || !masks.insert(l.parent))
        {
            return Err(Error::InvalidArgument("invalid or duplicate mask"));
        }
        if (l.white.0 == 0) != (l.white.1 == 0)
            || l.white.0 > MAX_DIMENSION
            || l.white.1 > MAX_DIMENSION
            || (l.group && l.white != (0, 0))
        {
            return Err(Error::InvalidArgument("invalid background extent"));
        }
        if l.group
            && (!l.tiles.is_empty()
                || l.appearance.offset_x != 0
                || l.appearance.offset_y != 0
                || l.appearance.locks & crate::LOCK_TRANSPARENCY != 0)
        {
            return Err(Error::InvalidArgument("invalid group content"));
        }
        let mut parent = l.parent;
        let mut depth = 0;
        while parent != 0 {
            let p = *index
                .get(&parent)
                .ok_or(Error::InvalidArgument("missing parent"))?;
            if p <= i || (!layers[p].group && !(l.mask && parent == l.parent)) || layers[p].mask {
                return Err(Error::InvalidArgument("invalid parent/order"));
            }
            depth += 1;
            if depth > 16 {
                return Err(Error::ResourceLimit("group nesting exceeds 16"));
            }
            let span = spans.entry(parent).or_insert((i, 0));
            span.0 = span.0.min(i);
            span.1 += 1;
            parent = layers[p].parent;
        }
    }
    for (id, (start, count)) in spans {
        if index[&id] - start != count {
            return Err(Error::InvalidArgument("non-contiguous group"));
        }
    }
    Ok(())
}
fn parent_layer(layers: &[Layer], id: u64) -> &Layer {
    layers
        .iter()
        .find(|l| l.id == id)
        .expect("validated hierarchy")
}
pub(crate) fn visible(layers: &[Layer], layer: &Layer) -> bool {
    let mut node = layer;
    loop {
        if !node.properties.visible || node.properties.opacity * node.appearance.fill <= 0. {
            return false;
        }
        if node.parent == 0 {
            return true;
        }
        node = parent_layer(layers, node.parent);
    }
}
fn visibility(layers: &[Layer]) -> Vec<bool> {
    let mut groups = BTreeMap::new();
    let mut values = Vec::with_capacity(layers.len());
    for layer in layers.iter().rev() {
        let shown = layer.properties.visible
            && layer.properties.opacity * layer.appearance.fill > 0.
            && (layer.parent == 0 || *groups.get(&layer.parent).expect("postorder parent"));
        if !layer.mask {
            groups.insert(layer.id, shown);
        }
        values.push(shown);
    }
    values.reverse();
    values
}
/// Postorder permits isolated group rows to be completed before blending into
/// their parent. Only one scan and bounded rows; cold pages are pinned per row.
pub(crate) fn composite_row(layers: &[Layer], x: u32, y: u32, count: u32) -> Result<Vec<Pixel>> {
    if layers.iter().any(|l| l.clipped) {
        return clipped_row(layers, x, y, count);
    }
    let mut rows = BTreeMap::<u64, Vec<Pixel>>::new();
    let mut masks = BTreeMap::<u64, Vec<Pixel>>::new();
    for (layer, shown) in layers.iter().zip(visibility(layers)) {
        if !shown {
            continue;
        }
        if layer.mask {
            masks.insert(
                layer.parent,
                layer
                    .read_canvas_row(x, y, count)?
                    .into_iter()
                    .map(|p| p.scaled(layer.properties.opacity))
                    .collect(),
            );
            continue;
        }
        let mut source = if layer.group {
            rows.remove(&layer.id)
                .unwrap_or_else(|| vec![Pixel::TRANSPARENT; count as usize])
        } else {
            layer.read_canvas_row(x, y, count)?
        };
        if let Some(mask) = masks.remove(&layer.id) {
            for (p, m) in source.iter_mut().zip(mask) {
                *p = p.scaled(1. - m.alpha());
            }
        }
        let output = rows
            .entry(layer.parent)
            .or_insert_with(|| vec![Pixel::TRANSPARENT; count as usize]);
        for (i, (destination, source)) in output.iter_mut().zip(source).enumerate() {
            *destination = crate::blend_pixel(
                source.scaled(layer.properties.opacity * layer.appearance.fill),
                *destination,
                layer.appearance.blend,
                x + i as u32,
                y,
                u64::from(layer.appearance.dissolve_seed),
            );
        }
    }
    Ok(rows
        .remove(&0)
        .unwrap_or_else(|| vec![Pixel::TRANSPARENT; count as usize]))
}

struct ClipRow<'a> {
    base: &'a Layer,
    pixels: Vec<Pixel>,
}
fn flush_clip(
    parent: u64,
    pending: &mut BTreeMap<u64, ClipRow<'_>>,
    rows: &mut BTreeMap<u64, Vec<Pixel>>,
    x: u32,
    y: u32,
) {
    let Some(chain) = pending.remove(&parent) else {
        return;
    };
    let output = rows
        .entry(parent)
        .or_insert_with(|| vec![Pixel::TRANSPARENT; chain.pixels.len()]);
    for (i, (dst, src)) in output.iter_mut().zip(chain.pixels).enumerate() {
        *dst = crate::blend_pixel(
            src.scaled(chain.base.properties.opacity * chain.base.appearance.fill),
            *dst,
            chain.base.appearance.blend,
            x + i as u32,
            y,
            u64::from(chain.base.appearance.dissolve_seed),
        );
    }
}
/// Resolve each sibling chain before compositing its base into the parent.
/// Hidden bases remain structural boundaries; clipping never expands their alpha.
fn clipped_row(layers: &[Layer], x: u32, y: u32, count: u32) -> Result<Vec<Pixel>> {
    let mut rows = BTreeMap::new();
    let mut pending = BTreeMap::<u64, ClipRow<'_>>::new();
    let mut masks = BTreeMap::<u64, Vec<Pixel>>::new();
    for (layer, shown) in layers.iter().zip(visibility(layers)) {
        if layer.mask {
            if shown {
                masks.insert(
                    layer.parent,
                    layer
                        .read_canvas_row(x, y, count)?
                        .into_iter()
                        .map(|p| p.scaled(layer.properties.opacity))
                        .collect(),
                );
            }
            continue;
        }
        if layer.group {
            flush_clip(layer.id, &mut pending, &mut rows, x, y);
        }
        let mut source = if !shown {
            rows.remove(&layer.id);
            vec![Pixel::TRANSPARENT; count as usize]
        } else if layer.group {
            rows.remove(&layer.id)
                .unwrap_or_else(|| vec![Pixel::TRANSPARENT; count as usize])
        } else {
            layer.read_canvas_row(x, y, count)?
        };
        if let Some(mask) = masks.remove(&layer.id) {
            for (p, m) in source.iter_mut().zip(mask) {
                *p = p.scaled(1. - m.alpha());
            }
        }
        if layer.clipped {
            if let Some(chain) = pending.get_mut(&layer.parent) {
                for (i, (dst, src)) in chain.pixels.iter_mut().zip(source).enumerate() {
                    let a = dst.alpha();
                    if a == 0. {
                        continue;
                    }
                    if layer.appearance.blend == crate::BlendMode::Normal {
                        *dst = dst.paint_preserving_alpha(
                            src,
                            layer.properties.opacity * layer.appearance.fill,
                        );
                        continue;
                    }
                    let c = dst.components();
                    let opaque = Pixel::from_straight([
                        (c[0] / a).clamp(0., 1.),
                        (c[1] / a).clamp(0., 1.),
                        (c[2] / a).clamp(0., 1.),
                        1.,
                    ])?;
                    *dst = crate::blend_pixel(
                        src.scaled(layer.properties.opacity * layer.appearance.fill),
                        opaque,
                        layer.appearance.blend,
                        x + i as u32,
                        y,
                        u64::from(layer.appearance.dissolve_seed),
                    )
                    .scaled(a);
                }
            }
        } else {
            flush_clip(layer.parent, &mut pending, &mut rows, x, y);
            pending.insert(
                layer.parent,
                ClipRow {
                    base: layer,
                    pixels: source,
                },
            );
        }
    }
    for parent in pending.keys().copied().collect::<Vec<_>>() {
        flush_clip(parent, &mut pending, &mut rows, x, y);
    }
    Ok(rows
        .remove(&0)
        .unwrap_or_else(|| vec![Pixel::TRANSPARENT; count as usize]))
}

impl DocumentSnapshot {
    pub fn has_groups(&self) -> bool {
        self.layers
            .iter()
            .any(|l| l.group || l.mask || l.clipped || l.white != (0, 0))
    }
    pub fn visible_pixel_layers(&self) -> Vec<usize> {
        if self.has_groups() {
            return self
                .layers
                .iter()
                .zip(visibility(&self.layers))
                .enumerate()
                .filter(|(_, (l, shown))| !l.group && *shown)
                .map(|(i, _)| i)
                .collect();
        }
        self.layers
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.group && visible(&self.layers, l))
            .map(|(i, _)| i)
            .collect()
    }
}
impl Document {
    /// Dynamic same-parent bases in one bottom-to-top pass, including hidden nodes.
    pub fn layer_clipping_bases(&self) -> Vec<u64> {
        let mut bases = BTreeMap::new();
        self.layers
            .iter()
            .map(|l| {
                if l.mask {
                    return 0;
                }
                if l.clipped {
                    bases.get(&l.parent).copied().unwrap_or(0)
                } else {
                    bases.insert(l.parent, l.id);
                    0
                }
            })
            .collect()
    }
    pub fn set_layer_clipping(&mut self, id: u64, enabled: bool) -> Result<()> {
        self.idle()?;
        let i = self.layer_index(id)?;
        if self.layers[i].mask {
            return Err(Error::InvalidArgument("mask cannot be clipped"));
        }
        self.check_ancestors(id, crate::LOCK_ALL)?;
        if self.layers[i].clipped == enabled {
            return Ok(());
        }
        let mut nodes = self.structure();
        nodes[i].clipped = enabled;
        if enabled
            && !nodes[..i]
                .iter()
                .any(|n| !n.mask && !n.clipped && n.parent == nodes[i].parent)
        {
            return Err(Error::InvalidArgument(
                "clipping requires a lower sibling base",
            ));
        }
        self.commit_structure(nodes)
    }
    /// Construction-only clipping extension; no partial imported document is published.
    pub fn set_import_clipping(&mut self, flags: &[bool]) -> Result<()> {
        self.idle()?;
        if self.revision != 0 || self.history_depth() != (0, 0) || flags.len() != self.layers.len()
        {
            return Err(Error::InvalidArgument("clipping requires fresh import"));
        }
        let mut nodes = self.structure();
        for (node, &flag) in nodes.iter_mut().zip(flags) {
            node.clipped = flag;
        }
        validate(&nodes)?;
        self.apply_structure(&nodes);
        Ok(())
    }
    /// One postorder pass for publication; do not search all layers once per
    /// row on every input tick (quadratic even in a flat document).
    pub fn layer_hierarchies(&self) -> Vec<(u64, u32, u32)> {
        let mut groups = BTreeMap::<u64, (u32, u32)>::new();
        let mut values = Vec::with_capacity(self.layers.len());
        for layer in self.layers.iter().rev() {
            let (depth, inherited) = if layer.parent == 0 {
                (0, 0)
            } else {
                let &(depth, locks) = groups.get(&layer.parent).expect("postorder parent");
                (depth + 1, locks & (crate::LOCK_POSITION | crate::LOCK_ALL))
            };
            let locks = layer.appearance.locks | inherited;
            if !layer.mask {
                groups.insert(layer.id, (depth, locks));
            }
            values.push((layer.parent, depth, locks));
        }
        values.reverse();
        values
    }
    pub fn layer_hierarchy(&self, id: u64) -> Result<(u64, u32, u32)> {
        let layer = self.layer(id).ok_or(Error::LayerNotFound(id))?;
        let mut node = layer;
        let mut depth = 0;
        let mut locks = node.appearance.locks;
        while node.parent != 0 {
            node = parent_layer(&self.layers, node.parent);
            depth += 1;
            // Transparency lock is local; position/full locks are inherited.
            locks |= node.appearance.locks & (crate::LOCK_POSITION | crate::LOCK_ALL);
        }
        Ok((layer.parent, depth, locks))
    }
    pub(super) fn check_ancestors(&self, id: u64, flags: u32) -> Result<()> {
        if id != 0 && self.layer_hierarchy(id)?.2 & flags != 0 {
            return Err(Error::LayerLocked);
        }
        Ok(())
    }
    fn descendant(&self, child: &Layer, group: u64) -> bool {
        let mut parent = child.parent;
        while parent != 0 {
            if parent == group {
                return true;
            }
            parent = parent_layer(&self.layers, parent).parent;
        }
        false
    }
    pub(super) fn subtree(&self, id: u64) -> Result<std::ops::Range<usize>> {
        let end = self.layer_index(id)?;
        let start = self
            .layers
            .iter()
            .position(|l| self.descendant(l, id))
            .unwrap_or(end);
        Ok(start..end + 1)
    }
    fn structure(&self) -> Vec<Layer> {
        self.layers.iter().map(Layer::metadata).collect()
    }
    pub(super) fn apply_structure(&mut self, nodes: &[Layer]) {
        let mut previous: BTreeMap<_, _> = std::mem::take(&mut self.layers)
            .into_iter()
            .map(|l| (l.id, l))
            .collect();
        self.layers = nodes
            .iter()
            .map(|n| {
                let mut node = n.metadata();
                if let Some(old) = previous.remove(&n.id) {
                    node.tiles = old.tiles;
                }
                node
            })
            .collect();
        debug_assert!(previous.values().all(|l| l.group && l.tiles.is_empty()));
    }
    pub(crate) fn commit_structure(&mut self, after: Vec<Layer>) -> Result<()> {
        let action = Command::Structure {
            before: self.structure(),
            after: after.clone(),
        }
        .action();
        self.commit_structure_entry(after, action)
    }
    /// Commit a validated structure with an explicit history label, so a repair can explain itself.
    pub(crate) fn commit_structure_entry(
        &mut self,
        after: Vec<Layer>,
        action: crate::HistoryAction,
    ) -> Result<()> {
        validate(&after)?;
        self.commit_with_action(
            Command::Structure {
                before: self.structure(),
                after: after.clone(),
            },
            action,
        )?;
        self.apply_structure(&after);
        self.ensure_active_layer();
        self.dirty_all();
        Ok(())
    }
    /// Wrap one layer or an existing subtree; no pixels are copied.
    pub fn group_layer(&mut self, id: u64, name: impl Into<String>) -> Result<u64> {
        self.idle()?;
        let name = name.into();
        if name.is_empty() || name.len() > 1024 || name.contains('\0') {
            return Err(Error::InvalidArgument("invalid group name"));
        }
        if self.layers.len() >= 4096 {
            return Err(Error::ResourceLimit("layer count"));
        }
        let range = self.subtree(id)?;
        if self.layer(id).expect("root").mask {
            return Err(Error::InvalidArgument("cannot group a mask"));
        }
        self.check_ancestors(id, crate::LOCK_ALL)?;
        let group_id = self.next_layer_id;
        let next = group_id
            .checked_add(1)
            .ok_or(Error::ResourceLimit("layer IDs"))?;
        let mut nodes = self.structure();
        let mut group = Layer::new(group_id, name);
        group.group = true;
        group.parent = nodes[range.end - 1].parent;
        nodes[range.end - 1].parent = group_id;
        nodes.insert(range.end, group);
        self.commit_structure(nodes)?;
        self.next_layer_id = next;
        self.active_layer = group_id;
        Ok(group_id)
    }
    /// Move a contiguous subtree to the top of another group (or the root).
    /// Pixel positions remain in document coordinates.
    pub fn reparent_layer(&mut self, id: u64, parent: u64) -> Result<()> {
        self.idle()?;
        if self.layer(id).ok_or(Error::LayerNotFound(id))?.mask {
            return Err(Error::InvalidArgument("mask belongs to its owner"));
        }
        self.check_ancestors(id, crate::LOCK_ALL)?;
        if parent != 0 {
            let target = self.layer(parent).ok_or(Error::LayerNotFound(parent))?;
            if !target.group {
                return Err(Error::InvalidArgument("parent must be a group"));
            }
            self.check_ancestors(parent, crate::LOCK_ALL)?;
            if parent == id || self.descendant(target, id) {
                return Err(Error::InvalidArgument("group cycle"));
            }
        }
        if self.layer(id).expect("checked layer").parent == parent {
            return Ok(());
        }
        let range = self.subtree(id)?;
        let mut nodes = self.structure();
        let mut moved: Vec<_> = nodes.drain(range).collect();
        moved.last_mut().expect("subtree root").parent = parent;
        let index = if parent == 0 {
            nodes.len()
        } else {
            nodes
                .iter()
                .position(|l| l.id == parent)
                .expect("target outside subtree")
        };
        nodes.splice(index..index, moved);
        self.commit_structure(nodes)
    }
    /// Removing the group discards its compositing properties; children stay unbaked.
    pub fn ungroup_layer(&mut self, id: u64) -> Result<()> {
        self.idle()?;
        let index = self.layer_index(id)?;
        if self.mask_for(id).is_some() {
            return Err(Error::InvalidArgument(
                "delete group mask before ungrouping",
            ));
        }
        if !self.layers[index].group {
            return Err(Error::InvalidArgument("not a group"));
        }
        self.check_ancestors(id, crate::LOCK_ALL)?;
        let parent = self.layers[index].parent;
        let mut nodes = self.structure();
        for node in &mut nodes {
            if node.parent == id {
                node.parent = parent;
            }
        }
        nodes.remove(index);
        self.commit_structure(nodes)
    }
    pub(super) fn remove_group(&mut self, id: u64) -> Result<()> {
        let range = self.subtree(id)?;
        for layer in &self.layers[range.clone()] {
            self.check_ancestors(layer.id, crate::LOCK_ALL)?;
        }
        if !self
            .layers
            .iter()
            .enumerate()
            .any(|(i, l)| !range.contains(&i) && !l.group && !l.mask)
        {
            return Err(Error::LastLayer);
        }
        self.commit(Command::RemoveTree {
            index: range.start,
            layers: self.layers[range.clone()].to_vec(),
        })?;
        self.layers.drain(range);
        self.ensure_active_layer();
        self.dirty_all();
        Ok(())
    }
    pub(super) fn move_group(&mut self, id: u64, dx: i32, dy: i32) -> Result<()> {
        self.check_ancestors(id, crate::LOCK_POSITION | crate::LOCK_ALL)?;
        if dx == 0 && dy == 0 {
            return Ok(());
        }
        let range = self.subtree(id)?;
        let mut nodes = self.structure();
        for node in &mut nodes[range] {
            self.check_ancestors(node.id, crate::LOCK_POSITION | crate::LOCK_ALL)?;
            if !node.group {
                node.appearance = node.appearance.translated(dx, dy)?;
            }
        }
        self.commit_structure(nodes)
    }
    pub fn mask_for(&self, id: u64) -> Option<&Layer> {
        self.layers.iter().find(|l| l.mask && l.parent == id)
    }
    pub fn add_mask(&mut self, id: u64) -> Result<u64> {
        self.idle()?;
        self.check_ancestors(id, crate::LOCK_ALL)?;
        let owner = self.layer(id).ok_or(Error::LayerNotFound(id))?;
        if owner.mask || self.mask_for(id).is_some() {
            return Err(Error::InvalidArgument(
                "mask already exists or invalid owner",
            ));
        }
        if self.layers.len() >= 4096 || self.layer_hierarchy(id)?.1 >= 16 {
            return Err(Error::ResourceLimit("mask depth/count"));
        }
        let index = self.layer_index(id)?;
        let mask_id = self.next_layer_id;
        let next = mask_id
            .checked_add(1)
            .ok_or(Error::ResourceLimit("layer IDs"))?;
        let mut mask = Layer::new(mask_id, "蒙版".into());
        mask.mask = true;
        mask.parent = id;
        mask.appearance.offset_x = owner.appearance.offset_x;
        mask.appearance.offset_y = owner.appearance.offset_y;
        self.commit(Command::AddLayer {
            index,
            layer: mask.clone(),
        })?;
        self.layers.insert(index, mask);
        self.next_layer_id = next;
        self.active_layer = mask_id;
        self.dirty_all();
        Ok(mask_id)
    }
    /// Drop root above/below a sibling, or onto a group's interior. All checks precede commit.
    pub fn drop_layer(&mut self, id: u64, target: u64, placement: u32) -> Result<()> {
        self.idle()?;
        let layer = self.layer(id).ok_or(Error::LayerNotFound(id))?;
        if layer.mask || placement > 2 {
            return Err(Error::InvalidArgument("invalid drop"));
        }
        self.check_ancestors(id, crate::LOCK_ALL)?;
        let (parent, anchor) = if target == 0 {
            if placement != 0 {
                return Err(Error::InvalidArgument("root drop placement"));
            }
            (0, None)
        } else {
            let to = self.layer(target).ok_or(Error::LayerNotFound(target))?;
            if to.mask || to.id == id || self.descendant(to, id) {
                return Err(Error::InvalidArgument("invalid drop target/cycle"));
            }
            if placement == 0 {
                if !to.group {
                    return Err(Error::InvalidArgument("drop inside requires group"));
                }
                (target, None)
            } else {
                (to.parent, Some(target))
            }
        };
        self.check_ancestors(parent, crate::LOCK_ALL)?;
        let mut nodes = self.structure();
        let range = self.subtree(id)?;
        let mut moved: Vec<_> = nodes.drain(range).collect();
        moved.last_mut().expect("root").parent = parent;
        let index = if let Some(anchor) = anchor {
            let end = nodes.iter().position(|l| l.id == anchor).expect("anchor") + 1;
            if placement == 1 {
                end
            } else {
                let mut start = end - 1;
                while start > 0 {
                    let mut p = nodes[start - 1].parent;
                    let mut child = false;
                    while p != 0 {
                        if p == anchor {
                            child = true;
                            break;
                        }
                        p = nodes.iter().find(|l| l.id == p).expect("parent").parent;
                    }
                    if !child {
                        break;
                    }
                    start -= 1;
                }
                start
            }
        } else if parent == 0 {
            nodes.len()
        } else {
            nodes.iter().position(|l| l.id == parent).expect("group")
        };
        nodes.splice(index..index, moved);
        // A move can leave clipped layers whose base went elsewhere. Repair that here so a dropped
        // layer never sits in a state the compositor would silently ignore, and so the repair and
        // the move are one undoable step. The parent/clip pair decides whether anything changed.
        if nodes
            .iter()
            .map(|n| (n.id, n.parent, n.clipped))
            .eq(self.layers.iter().map(|n| (n.id, n.parent, n.clipped)))
        {
            return Ok(());
        }
        self.commit_repaired_structure(nodes).map(|_| ())
    }
    /// Import validates all parent IDs/order/depth before the document is exposed.
    pub fn set_import_hierarchy(&mut self, hierarchy: &[(u64, bool)]) -> Result<()> {
        self.set_import_nodes(
            &hierarchy
                .iter()
                .map(|&(p, g)| (p, g, false, (0, 0)))
                .collect::<Vec<_>>(),
        )
    }
    pub fn set_import_nodes(&mut self, hierarchy: &[(u64, bool, bool, (u32, u32))]) -> Result<()> {
        self.idle()?;
        if self.revision != 0
            || self.history_depth() != (0, 0)
            || hierarchy.len() != self.layers.len()
        {
            return Err(Error::InvalidArgument("hierarchy requires fresh import"));
        }
        let mut nodes = self.structure();
        for (node, (parent, group, mask, white)) in nodes.iter_mut().zip(hierarchy) {
            node.parent = *parent;
            node.group = *group;
            node.mask = *mask;
            node.white = *white;
        }
        validate(&nodes)?;
        if nodes
            .iter()
            .zip(&self.layers)
            .any(|(n, l)| n.group && !l.tiles.is_empty())
        {
            return Err(Error::InvalidArgument("group raster content"));
        }
        self.apply_structure(&nodes);
        Ok(())
    }
}
