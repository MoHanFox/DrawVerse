use super::*;

impl Document {
    /// Normalize overlapping selections to independent subtree roots, in stack order.
    fn batch_roots(&self, ids: &[u64]) -> Result<Vec<u64>> {
        if ids.is_empty() || ids.len() > 4096 {
            return Err(Error::InvalidArgument("invalid batch size"));
        }
        let selected: BTreeSet<_> = ids.iter().copied().collect();
        if selected.len() != ids.len() {
            return Err(Error::InvalidArgument("duplicate batch IDs"));
        }
        for &id in ids {
            self.layer_index(id)?;
        }
        Ok(self
            .layers
            .iter()
            .filter(|l| {
                if !selected.contains(&l.id) {
                    return false;
                }
                let mut parent = l.parent;
                while parent != 0 {
                    if selected.contains(&parent) {
                        return false;
                    }
                    parent = self.layer(parent).expect("validated parent").parent;
                }
                true
            })
            .map(|l| l.id)
            .collect())
    }

    /// Wrap selected subtrees once, at their highest branch in the common parent.
    pub fn group_layers(&mut self, ids: &[u64], name: impl Into<String>) -> Result<u64> {
        self.idle()?;
        let name = name.into();
        if name.is_empty() || name.len() > 1024 || name.contains('\0') {
            return Err(Error::InvalidArgument("invalid group name"));
        }
        if self.layers.len() >= 4096 {
            return Err(Error::ResourceLimit("layer count"));
        }
        let roots = self.batch_roots(ids)?;
        let mut moved_ids = BTreeSet::new();
        for &id in &roots {
            if self.layer(id).expect("selection").mask {
                return Err(Error::InvalidArgument("cannot group a mask"));
            }
            for node in &self.layers[self.subtree(id)?] {
                self.check_ancestors(node.id, crate::LOCK_ALL)?;
                moved_ids.insert(node.id);
            }
        }
        let mut parent = self.layer(roots[0]).expect("selection").parent;
        while parent != 0
            && roots
                .iter()
                .any(|&id| !self.descendant(self.layer(id).expect("selection"), parent))
        {
            parent = self.layer(parent).expect("ancestor").parent;
        }
        self.check_ancestors(parent, crate::LOCK_ALL)?;
        let mut branch = *roots.last().expect("nonempty selection");
        while self.layer(branch).expect("branch").parent != parent {
            branch = self.layer(branch).expect("branch").parent;
        }
        let anchor = self.subtree(branch)?.end;
        let group_id = self.next_layer_id;
        let next = group_id
            .checked_add(1)
            .ok_or(Error::ResourceLimit("layer IDs"))?;
        let before = self.structure();
        let at = before[..anchor]
            .iter()
            .filter(|l| !moved_ids.contains(&l.id))
            .count();
        let mut nodes: Vec<_> = before
            .iter()
            .filter(|l| !moved_ids.contains(&l.id))
            .cloned()
            .collect();
        let mut moved: Vec<_> = before
            .iter()
            .filter(|l| moved_ids.contains(&l.id))
            .cloned()
            .collect();
        for node in &mut moved {
            if roots.contains(&node.id) {
                node.parent = group_id;
            }
        }
        let mut group = Layer::new(group_id, name);
        group.group = true;
        group.appearance.blend = crate::BlendMode::PassThrough;
        group.parent = parent;
        moved.push(group);
        nodes.splice(at..at, moved);
        let (nodes, _) = crate::clipping::repair_changed_bases(&before, &nodes);
        self.commit_structure_entry(nodes, crate::HistoryAction::Group)?;
        self.next_layer_id = next;
        self.active_layer = group_id;
        Ok(group_id)
    }

    /// Remove selected subtrees, or just their group containers, as one history record.
    /// A group-only removal discards group attributes/mask and preserves its children.
    pub fn remove_layers(&mut self, ids: &[u64], keep_group_children: bool) -> Result<()> {
        self.idle()?;
        let roots = self.batch_roots(ids)?;
        let mut removed_ids = BTreeSet::new();
        let mut unwrapped = BTreeSet::new();
        for id in roots {
            let layer = self.layer(id).expect("validated selection");
            let range = self.subtree(id)?;
            // Changing a container affects its descendants too; validate before committing.
            for node in &self.layers[range.clone()] {
                self.check_ancestors(node.id, crate::LOCK_ALL)?;
            }
            if keep_group_children && layer.group {
                removed_ids.insert(id);
                unwrapped.insert(id);
                if let Some(mask) = self.mask_for(id) {
                    removed_ids.insert(mask.id);
                }
            } else {
                removed_ids.extend(self.layers[range].iter().map(|l| l.id));
            }
        }
        if !self
            .layers
            .iter()
            .any(|l| !l.group && !l.mask && !removed_ids.contains(&l.id))
        {
            return Err(Error::LastLayer);
        }
        let before = self.structure();
        let mut after: Vec<_> = before
            .iter()
            .filter(|l| !removed_ids.contains(&l.id))
            .cloned()
            .collect();
        for node in &mut after {
            while unwrapped.contains(&node.parent) {
                node.parent = self.layer(node.parent).expect("removed group").parent;
            }
        }
        let (after, _) = crate::clipping::repair_changed_bases(&before, &after);
        groups::validate(&after)?;
        let removed = self
            .layers
            .iter()
            .filter(|l| removed_ids.contains(&l.id))
            .cloned()
            .collect();
        let command = Command::RemoveBatch {
            before,
            after,
            removed,
        };
        self.commit_with_action(
            command.clone(),
            if unwrapped.is_empty() {
                crate::HistoryAction::RemoveLayer
            } else {
                crate::HistoryAction::Ungroup
            },
        )?;
        self.apply_history(&command, true);
        Ok(())
    }

    /// Atomic subtree reorder/reparent preserving the selected roots' stack order.
    pub fn drop_layers(&mut self, ids: &[u64], target: u64, placement: u32) -> Result<()> {
        self.idle()?;
        let roots = self.batch_roots(ids)?;
        if placement > 2 || (target == 0 && placement != 0) {
            return Err(Error::InvalidArgument("invalid drop placement"));
        }
        let mut moved_ids = BTreeSet::new();
        for &id in &roots {
            if self.layer(id).expect("selection").mask {
                return Err(Error::InvalidArgument("mask belongs to its owner"));
            }
            for node in &self.layers[self.subtree(id)?] {
                self.check_ancestors(node.id, crate::LOCK_ALL)?;
                moved_ids.insert(node.id);
            }
        }
        let parent = if target == 0 {
            0
        } else {
            let to = self.layer(target).ok_or(Error::LayerNotFound(target))?;
            if to.mask || moved_ids.contains(&target) || (placement == 0 && !to.group) {
                return Err(Error::InvalidArgument("invalid drop target/cycle"));
            }
            if placement == 0 {
                target
            } else {
                to.parent
            }
        };
        self.check_ancestors(parent, crate::LOCK_ALL)?;
        let before = self.structure();
        let mut nodes: Vec<_> = before
            .iter()
            .filter(|l| !moved_ids.contains(&l.id))
            .cloned()
            .collect();
        let mut moved: Vec<_> = before
            .iter()
            .filter(|l| moved_ids.contains(&l.id))
            .cloned()
            .collect();
        for node in &mut moved {
            if roots.contains(&node.id) {
                node.parent = parent;
            }
        }
        let at = if target == 0 {
            nodes.len()
        } else if placement == 0 {
            nodes.iter().position(|l| l.id == target).expect("target")
        } else if placement == 1 {
            nodes.iter().position(|l| l.id == target).expect("target") + 1
        } else {
            let subtree = self.subtree(target)?;
            nodes
                .iter()
                .position(|l| {
                    self.layers[subtree.clone()]
                        .iter()
                        .any(|child| child.id == l.id)
                })
                .expect("target subtree")
        };
        nodes.splice(at..at, moved);
        if nodes
            .iter()
            .map(|l| (l.id, l.parent, l.clipped))
            .eq(before.iter().map(|l| (l.id, l.parent, l.clipped)))
        {
            return Ok(());
        }
        let old_bases = crate::clipping::bases(&before);
        // Process top to bottom so adjacent selected roots inherit the upper chain together.
        for &id in roots.iter().rev() {
            let index = nodes.iter().position(|l| l.id == id).expect("moved root");
            let upper_clipped = nodes[index + 1..]
                .iter()
                .find(|l| !l.mask && l.parent == parent)
                .is_some_and(|l| l.clipped);
            let base = nodes[..index]
                .iter()
                .rev()
                .find(|l| !l.mask && l.parent == parent && !l.clipped)
                .map(|l| l.id);
            nodes[index].clipped =
                base.is_some() && (upper_clipped || old_bases.get(&id).copied() == base);
        }
        // New clipping flags belong to the destination chain, not the old one.
        let mut comparison = before;
        for layer in &mut comparison {
            if roots.contains(&layer.id) {
                layer.clipped = false;
            }
        }
        let (nodes, released) = crate::clipping::repair_changed_bases(&comparison, &nodes);
        self.commit_structure_entry(
            nodes,
            if released.is_empty() {
                crate::HistoryAction::Reparent
            } else {
                crate::HistoryAction::ReleaseClipping
            },
        )
    }
}

#[cfg(test)]
mod failures {
    use super::*;
    #[test]
    fn group_history_write_failure_preserves_structure_redo_and_next_id() {
        let imported = (0..512)
            .map(|_| crate::ImportedLayer {
                name: "layer".repeat(200),
                properties: Default::default(),
                tiles: Default::default(),
            })
            .collect();
        let mut doc = Document::from_import(
            64,
            64,
            imported,
            0,
            DocumentOptions {
                max_history_bytes: 2 * TILE_BYTES + 256,
                ..Default::default()
            },
        )
        .unwrap();
        let added = doc.add_layer("redo").unwrap();
        doc.undo().unwrap();
        let before = doc.structure();
        let depth = doc.history_depth();
        let revision = doc.revision();
        let next = doc.next_layer_id;
        let active = doc.active_layer;
        crate::history_storage::fail_writes_for_test(true);
        let result = doc.group_layers(&[1, 512], "group");
        crate::history_storage::fail_writes_for_test(false);
        assert!(matches!(result, Err(Error::HistoryStorage(_))));
        assert_eq!(
            doc.structure()
                .iter()
                .map(|l| (l.id, l.parent))
                .collect::<Vec<_>>(),
            before.iter().map(|l| (l.id, l.parent)).collect::<Vec<_>>()
        );
        assert_eq!(doc.history_depth(), depth);
        assert_eq!(doc.revision(), revision);
        assert_eq!(doc.next_layer_id, next);
        assert_eq!(doc.active_layer, active);
        doc.redo().unwrap();
        assert!(doc.layer(added).is_some());
    }
    #[test]
    fn history_write_failure_leaves_the_entire_batch_and_redo_intact() {
        let mut doc = Document::with_options(
            192,
            64,
            DocumentOptions {
                max_history_bytes: 2 * TILE_BYTES + 256,
                ..Default::default()
            },
        )
        .unwrap();
        let a = doc.add_layer("a").unwrap();
        for x in [16., 80., 144.] {
            doc.begin_stroke(Brush::default(), InputPoint::new(x, 16., 1.))
                .unwrap();
            doc.end_stroke().unwrap();
        }
        let b = doc.add_layer("b").unwrap();
        doc.undo().unwrap();
        let ids = doc.layers.iter().map(|l| l.id).collect::<Vec<_>>();
        let depth = doc.history_depth();
        let revision = doc.revision();
        let pixels = doc.pixel(80, 16);
        crate::history_storage::fail_writes_for_test(true);
        let result = doc.remove_layers(&[a], false);
        crate::history_storage::fail_writes_for_test(false);
        assert!(matches!(result, Err(Error::HistoryStorage(_))));
        assert_eq!(doc.layers.iter().map(|l| l.id).collect::<Vec<_>>(), ids);
        assert_eq!(doc.revision(), revision);
        assert_eq!(doc.history_depth(), depth);
        assert_eq!(doc.pixel(80, 16), pixels);
        doc.redo().unwrap();
        assert!(doc.layer(b).is_some());
    }
}
