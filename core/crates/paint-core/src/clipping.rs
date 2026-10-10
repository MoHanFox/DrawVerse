//! Clipping-mask coherence.
//!
//! A clipped layer is only meaningful while the nearest non-mask sibling below it, in the same
//! parent, is itself unclipped: that sibling is its base. Moving, deleting or toggling one layer can
//! therefore orphan clipped layers above it. Rather than leaving those layers in a state where the
//! compositor silently ignores them, the document repairs the chain and reports what it unclipped.

use crate::{Document, Layer};

/// Unclip every orphaned clipped layer. `layers` is the candidate new stack, so the pass can run
/// before the change is committed and the whole repair still lands as one history entry.
pub(crate) fn repair(layers: &[Layer]) -> (Vec<Layer>, Vec<u64>) {
    let mut nodes = layers.to_vec();
    let mut repaired = Vec::new();
    // 0 means "no unclipped sibling seen yet for this parent", which is also the implicit root id;
    // the background is never a base, so both cases behave the same.
    let mut base: std::collections::BTreeMap<u64, u64> = std::collections::BTreeMap::new();
    for node in &mut nodes {
        if node.mask {
            continue;
        }
        let seen = base.get(&node.parent).copied().unwrap_or(0);
        if node.clipped {
            if seen == 0 {
                // No unclipped sibling below: the clip has no base left and cannot be honoured.
                node.clipped = false;
                repaired.push(node.id);
            }
        } else {
            base.insert(node.parent, node.id);
        }
    }
    (nodes, repaired)
}

impl Document {
    /// Repair orphaned clips after a structural change and record one history entry describing it.
    /// Moving the base of a clip chain moves the clipped layers with it, so a moved base keeps its
    /// chain intact and only genuinely orphaned layers are released.
    pub(crate) fn commit_repaired_structure(&mut self, nodes: Vec<Layer>) -> crate::Result<()> {
        let (nodes, repaired) = repair(&nodes);
        if repaired.is_empty() {
            return self.commit_structure(nodes);
        }
        // Same transaction as commit_structure, only the history label differs so the undo list
        // explains that clips were released.
        self.commit_structure_entry(nodes, crate::HistoryAction::ReleaseClipping)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayerId, LayerProperties};

    fn layer(id: LayerId, parent: LayerId, clipped: bool) -> Layer {
        let mut layer = Layer::new(id, format!("L{id}"));
        layer.parent = parent;
        layer.clipped = clipped;
        layer.properties = LayerProperties::default();
        layer
    }

    #[test]
    fn coherent_chains_are_left_alone() {
        let stack = vec![layer(1, 0, false), layer(2, 0, true), layer(3, 0, true)];
        let (nodes, repaired) = repair(&stack);
        assert!(repaired.is_empty());
        assert_eq!(nodes.len(), 3);
        assert!(nodes[1].clipped && nodes[2].clipped);
    }

    #[test]
    fn orphaned_leading_clips_are_released() {
        // A clip with no unclipped sibling below has no base and must be released.
        let stack = vec![layer(1, 0, true), layer(2, 0, false)];
        let (nodes, repaired) = repair(&stack);
        assert_eq!(repaired, vec![1]);
        assert!(!nodes[0].clipped);
        assert!(!nodes[1].clipped);
    }

    #[test]
    fn breaking_a_base_releases_every_clip_above_it() {
        // A clip whose only unclipped sibling sits above it has no base below and is released; the
        // pass is order-sensitive by design, so ids come back bottom-to-top.
        let stack = vec![layer(2, 0, true), layer(1, 0, true), layer(9, 0, false)];
        let (nodes, repaired) = repair(&stack);
        assert_eq!(repaired, vec![2, 1]);
        assert!(!nodes[0].clipped && !nodes[1].clipped);
    }

    #[test]
    fn each_parent_keeps_its_own_base() {
        // Clips are scoped per parent: a group's own base must not adopt the root's.
        let stack = vec![
            layer(1, 0, false),
            layer(2, 0, true),
            layer(3, 2, false),
            layer(4, 2, true),
        ];
        let (nodes, repaired) = repair(&stack);
        assert!(repaired.is_empty());
        assert!(nodes[1].clipped && nodes[3].clipped);
    }

    #[test]
    fn masks_do_not_count_as_bases() {
        // A mask sits between the base and its clip: the chain is still intact.
        let mut mask = layer(5, 0, false);
        mask.mask = true;
        let stack = vec![layer(1, 0, false), mask, layer(2, 0, true)];
        let (_, repaired) = repair(&stack);
        assert!(repaired.is_empty());
    }

    #[test]
    fn released_clips_stop_being_repaired() {
        // Idempotent: a repaired stack needs no further repair.
        let stack = vec![layer(1, 0, true), layer(2, 0, false)];
        let (nodes, _) = repair(&stack);
        let (again, repaired) = repair(&nodes);
        assert!(repaired.is_empty());
        assert_eq!(again.len(), 2);
    }
}
