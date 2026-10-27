use super::*;

impl Document {
    pub(super) fn copied_nodes(&self, ids: &[u64]) -> Result<Vec<Layer>> {
        let roots = self.batch_roots(ids)?;
        let mut nodes = Vec::new();
        for id in roots {
            if self.layer(id).expect("root").mask {
                return Err(Error::InvalidArgument("copy mask with its owner"));
            }
            let mut tree = self.layers[self.subtree(id)?].to_vec();
            tree.last_mut().expect("root").parent = 0;
            nodes.extend(tree);
        }
        // An external clipping base cannot travel with the copied subtree.
        let (nodes, _) = crate::clipping::repair_changed_bases(&self.layers, &nodes);
        Ok(nodes)
    }
    pub fn copy_layers(&self, ids: &[u64]) -> Result<Vec<u8>> {
        self.idle()?;
        crate::history_storage::encode_layer_bundle(&self.copied_nodes(ids)?)
    }
    /// Prepare the complete clipboard before the single authoritative mutation.
    /// Failed encoding, locks or history preparation leave the document intact.
    pub fn cut_layers(&mut self, ids: &[u64], selection_only: bool) -> Result<Vec<u8>> {
        let bytes = if selection_only {
            self.copy_selected_pixels(ids)?
        } else {
            self.copy_layers(ids)?
        };
        if selection_only {
            self.clear_selected_pixels(ids)?;
        } else {
            self.remove_layers(ids, false)?;
        }
        Ok(bytes)
    }
    pub fn paste_layers(&mut self, bytes: &[u8], target: u64) -> Result<u64> {
        self.idle()?;
        let nodes = crate::history_storage::decode_layer_bundle(
            bytes,
            self.options
                .max_document_tiles
                .saturating_sub(self.tile_count()),
            self.pool.clone(),
        )?;
        self.insert_copies(nodes, target, crate::HistoryAction::PasteLayers)
    }
    pub fn duplicate_layers(&mut self, ids: &[u64]) -> Result<u64> {
        self.idle()?;
        let roots = self.batch_roots(ids)?;
        let target = *roots.last().expect("roots");
        self.insert_copies(
            self.copied_nodes(ids)?,
            target,
            crate::HistoryAction::DuplicateLayers,
        )
    }
    fn insert_copies(
        &mut self,
        mut nodes: Vec<Layer>,
        target: u64,
        action: crate::HistoryAction,
    ) -> Result<u64> {
        if self.layers.len() + nodes.len() > 4096
            || self.tile_count() + nodes.iter().map(Layer::retained_tile_count).sum::<usize>()
                > self.options.max_document_tiles
        {
            return Err(Error::ResourceLimit("layer copy count/tiles"));
        }
        if nodes.iter().any(|l| l.mask && l.parent == 0) {
            return Err(Error::InvalidArgument("clipboard mask root"));
        }
        // Validate the standalone forest before mapping IDs into this document.
        let mut forest = nodes.clone();
        if !forest.iter().any(|l| !l.group && !l.mask) {
            let dummy = (1..=4097)
                .find(|id| !forest.iter().any(|l| l.id == *id))
                .expect("free ID");
            forest.push(Layer::new(dummy, "validation".into()));
        }
        groups::validate(&forest)?;
        let (parent, at) = if target == 0 && self.layers.is_empty() {
            (0, 0)
        } else {
            let target = self.layer(target).ok_or(Error::LayerNotFound(target))?;
            let target = if target.mask {
                self.layer(target.parent).expect("owner")
            } else {
                target
            };
            (target.parent, self.subtree(target.id)?.end)
        };
        self.check_ancestors(parent, crate::LOCK_ALL)?;
        let next = self
            .next_layer_id
            .checked_add(nodes.len() as u64)
            .ok_or(Error::ResourceLimit("layer IDs"))?;
        let ids: BTreeMap<_, _> = nodes
            .iter()
            .enumerate()
            .map(|(i, l)| (l.id, self.next_layer_id + i as u64))
            .collect();
        if ids.len() != nodes.len() || ids.contains_key(&0) {
            return Err(Error::InvalidArgument("clipboard IDs"));
        }
        let mut names: BTreeSet<_> = self.layers.iter().map(|l| l.name.clone()).collect();
        for node in &mut nodes {
            node.id = ids[&node.id];
            if node.parent == 0 {
                node.parent = parent;
                let mut base = node.name.clone();
                while base.len() > 990 {
                    base.pop();
                }
                let mut name = format!("{base} 副本");
                let mut n = 2;
                while names.contains(&name) {
                    name = format!("{base} 副本 {n}");
                    n += 1;
                }
                names.insert(name.clone());
                node.name = name;
            } else {
                node.parent = *ids
                    .get(&node.parent)
                    .ok_or(Error::InvalidArgument("clipboard parent"))?;
            }
        }
        let active = nodes.last().expect("nonempty codec").id;
        let mut after = self.layers.clone();
        after.splice(at..at, nodes);
        groups::validate(&after)?;
        self.replace_layers(after, action)?;
        self.next_layer_id = next;
        self.active_layer = active;
        Ok(active)
    }
    fn replace_layers(&mut self, after: Vec<Layer>, action: crate::HistoryAction) -> Result<()> {
        groups::validate(&after)?;
        let old: BTreeSet<_> = self.layers.iter().map(|l| l.id).collect();
        let new: BTreeSet<_> = after.iter().map(|l| l.id).collect();
        let pixels: Vec<_> = self
            .layers
            .iter()
            .filter(|l| !new.contains(&l.id))
            .chain(after.iter().filter(|l| !old.contains(&l.id)))
            .cloned()
            .collect();
        for layer in &pixels {
            for tile in layer.tiles.values() {
                self.pool.attach(tile)?;
                drop(tile.try_pixels()?);
            }
        }
        self.pool.trim()?;
        self.commit_with_action(
            Command::ReplaceBatch {
                before: self.structure(),
                after: after.iter().map(Layer::metadata).collect(),
                pixels,
            },
            action,
        )?;
        self.layers = after;
        self.ensure_active_layer();
        self.dirty_all();
        Ok(())
    }
    /// Single raster root merges with its lower sibling; a single group flattens itself.
    pub fn merge_layers(&mut self, ids: &[u64]) -> Result<u64> {
        self.idle()?;
        let mut roots = self.batch_roots(ids)?;
        if roots.iter().any(|&id| self.layer(id).expect("root").mask) {
            return Err(Error::InvalidArgument("cannot merge mask"));
        }
        if roots.len() == 1 && !self.layer(roots[0]).expect("root").group {
            let upper = self.layer_index(roots[0])?;
            let parent = self.layers[upper].parent;
            let lower = self.layers[..self.subtree(roots[0])?.start]
                .iter()
                .rev()
                .find(|l| !l.mask && l.parent == parent)
                .ok_or(Error::InvalidArgument("no lower layer"))?
                .id;
            roots.insert(0, lower);
        }
        let mut removed = BTreeSet::new();
        for &id in &roots {
            for node in &self.layers[self.subtree(id)?] {
                self.check_ancestors(node.id, crate::LOCK_ALL)?;
                removed.insert(node.id);
            }
        }
        let mut parent = self.layer(roots[0]).expect("root").parent;
        while parent != 0
            && roots
                .iter()
                .any(|&id| !self.descendant(self.layer(id).expect("root"), parent))
        {
            parent = self.layer(parent).expect("ancestor").parent;
        }
        self.check_ancestors(parent, crate::LOCK_ALL)?;
        let top = *roots.last().expect("roots");
        let mut branch = top;
        while self.layer(branch).expect("branch").parent != parent {
            branch = self.layer(branch).expect("branch").parent;
        }
        let anchor = self.subtree(branch)?.end;
        let mut source: Vec<_> = self
            .layers
            .iter()
            .filter(|l| removed.contains(&l.id))
            .cloned()
            .collect();
        for node in &mut source {
            if roots.contains(&node.id) {
                node.parent = 0;
            }
        }
        let (source, _) = crate::clipping::repair_changed_bases(&self.layers, &source);
        // Sparse candidate tiles include shifted raster extents and implicit white backgrounds.
        let mut candidates = BTreeSet::new();
        for layer in source.iter().filter(|l| !l.group && !l.mask) {
            let mut ranges = Vec::new();
            for coord in layer.tiles.keys() {
                let x = i64::from(coord.signed_x()) * 64 + i64::from(layer.appearance.offset_x);
                let y = i64::from(coord.signed_y()) * 64 + i64::from(layer.appearance.offset_y);
                ranges.push((x, y, x + 64, y + 64));
            }
            if layer.white != (0, 0) {
                let x = i64::from(layer.appearance.offset_x);
                let y = i64::from(layer.appearance.offset_y);
                ranges.push((
                    x,
                    y,
                    x + i64::from(layer.white.0),
                    y + i64::from(layer.white.1),
                ));
            }
            for (x, y, right, bottom) in ranges {
                let left = x.max(0);
                let top = y.max(0);
                let right = right.min(i64::from(self.width));
                let bottom = bottom.min(i64::from(self.height));
                if right <= left || bottom <= top {
                    continue;
                }
                let count = ((right - 1) / 64 - left / 64 + 1) * ((bottom - 1) / 64 - top / 64 + 1);
                if count > self.options.max_document_tiles as i64 {
                    return Err(Error::ResourceLimit("merge tiles"));
                }
                for ty in top / 64..=(bottom - 1) / 64 {
                    for tx in left / 64..=(right - 1) / 64 {
                        candidates.insert(TileCoord {
                            x: tx as u32,
                            y: ty as u32,
                        });
                    }
                }
                if candidates.len() > self.options.max_document_tiles {
                    return Err(Error::ResourceLimit("merge tiles"));
                }
            }
        }
        let id = self.next_layer_id;
        let next = id.checked_add(1).ok_or(Error::ResourceLimit("layer IDs"))?;
        let mut merged = Layer::new(id, self.layer(top).expect("root").name.clone());
        merged.parent = parent;
        for coord in candidates {
            let x = coord.x * 64;
            let y = coord.y * 64;
            let count = (self.width - x).min(64);
            let rows = (self.height - y).min(64);
            let mut pixels = vec![Pixel::TRANSPARENT; 4096];
            for row in 0..rows {
                let values = groups::composite_row(&source, x, y + row, count)?;
                pixels[row as usize * 64..row as usize * 64 + count as usize]
                    .copy_from_slice(&values);
            }
            let tile = Arc::new(Tile::from_pixels(pixels)?);
            if !tile.is_empty() {
                self.pool.attach(&tile)?;
                merged.tiles.insert(coord, tile);
                self.pool.trim()?;
            }
        }
        let remaining: usize = self
            .layers
            .iter()
            .filter(|l| !removed.contains(&l.id))
            .map(|l| l.tiles.len())
            .sum();
        if remaining + merged.tiles.len() > self.options.max_document_tiles {
            return Err(Error::ResourceLimit("merge tile budget"));
        }
        let at = self.layers[..anchor]
            .iter()
            .filter(|l| !removed.contains(&l.id))
            .count();
        let mut after: Vec<_> = self
            .layers
            .iter()
            .filter(|l| !removed.contains(&l.id))
            .cloned()
            .collect();
        after.insert(at, merged);
        let (after, _) = crate::clipping::repair_changed_bases(&self.layers, &after);
        self.replace_layers(after, crate::HistoryAction::MergeLayers)?;
        self.next_layer_id = next;
        self.active_layer = id;
        Ok(id)
    }
}
