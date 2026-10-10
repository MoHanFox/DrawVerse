use crate::{
    history_storage::Payload, Layer, LayerId, LayerProperties, Result, Tile, TileCoord, TILE_BYTES,
};
use std::{collections::VecDeque, sync::Arc};

/// Stable operation metadata; reading it never materializes history pixel payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum HistoryAction {
    Initial = 0,
    Brush = 1,
    Eraser = 2,
    SelectAll = 3,
    Selection = 4,
    Deselect = 5,
    InvertSelection = 6,
    AddLayer = 7,
    RemoveLayer = 8,
    LayerProperties = 9,
    LayerBlend = 10,
    LayerFill = 11,
    LayerLocks = 12,
    MoveLayer = 13,
    Group = 14,
    Ungroup = 15,
    Reparent = 16,
    Mask = 17,
    Clipping = 18,
    Truncated = 19,
    EllipseSelection = 20,
    /// Freehand lasso path.
    LassoSelection = 21,
    /// Colour-similarity flood fill.
    MagicSelection = 22,
    /// Automatic release of clipped layers whose base went away.
    ReleaseClipping = 23,
    /// Whole-canvas rotate or flip.
    TransformCanvas = 24,
    /// Paint-bucket fill of a similar region.
    Fill = 25,
}

#[derive(Clone, Debug)]
pub(crate) struct TileChange {
    pub coord: TileCoord,
    pub before: Option<Arc<Tile>>,
    pub after: Option<Arc<Tile>>,
}

#[derive(Clone, Debug)]
pub(crate) enum Command {
    Selection {
        before: crate::Selection,
        after: crate::Selection,
    },
    /// Only empty groups may be added/removed by this metadata-only command.
    Structure {
        before: Vec<Layer>,
        after: Vec<Layer>,
    },
    RemoveTree {
        index: usize,
        layers: Vec<Layer>,
    },
    Stroke {
        layer: LayerId,
        changes: Vec<TileChange>,
    },
    /// Whole-canvas orientation change: the layer stacks plus both canvas dimensions.
    Transform {
        before: Vec<Layer>,
        after: Vec<Layer>,
        before_size: (u32, u32),
        after_size: (u32, u32),
    },
    AddLayer {
        index: usize,
        layer: Layer,
    },
    RemoveLayer {
        index: usize,
        layer: Layer,
    },
    Appearance {
        layer: LayerId,
        before: crate::LayerAppearance,
        after: crate::LayerAppearance,
    },
    Properties {
        layer: LayerId,
        before: LayerProperties,
        after: LayerProperties,
    },
}

#[derive(Debug)]
pub(crate) enum StoredCommand {
    Inline(Command, HistoryAction),
    Encoded(Payload, HistoryAction),
}
impl StoredCommand {
    #[cfg(test)]
    pub fn prepare(command: &Command, max_bytes: usize) -> Result<Self> {
        Self::prepare_in(command, max_bytes, paint_storage::ScratchSpace::system())
    }
    pub fn prepare_in(
        command: &Command,
        max_bytes: usize,
        storage: Arc<paint_storage::ScratchSpace>,
    ) -> Result<Self> {
        if command.bytes() <= max_bytes {
            Ok(Self::Inline(command.clone(), command.action()))
        } else {
            Ok(Self::Encoded(
                Payload::encode_in(command, max_bytes - 128, storage)?,
                command.action(),
            ))
        }
    }
    #[cfg(test)]
    pub fn materialize(&self) -> Result<Command> {
        self.materialize_in(None)
    }
    pub fn materialize_in(
        &self,
        pool: Option<&Arc<crate::page_pool::PagePool>>,
    ) -> Result<Command> {
        match self {
            Self::Inline(command, _) => {
                // Inline history now retains identities, including cold pages.
                // Validate those pages before moving either history stack.
                command.verify_tiles()?;
                if let Some(pool) = pool {
                    pool.trim()?;
                }
                Ok(command.clone())
            }
            Self::Encoded(payload, _) => payload.decode_in(pool),
        }
    }
    fn bytes(&self) -> usize {
        match self {
            Self::Inline(command, _) => command.bytes(),
            Self::Encoded(payload, _) => payload.memory_bytes() + 128,
        }
    }
    fn disk_bytes(&self) -> u64 {
        match self {
            Self::Inline(..) => 0,
            Self::Encoded(payload, _) => payload.disk_bytes(),
        }
    }
    pub fn action(&self) -> HistoryAction {
        match self {
            Self::Inline(_, a) | Self::Encoded(_, a) => *a,
        }
    }
    pub fn with_action(mut self, action: HistoryAction) -> Self {
        match &mut self {
            Self::Inline(_, a) | Self::Encoded(_, a) => *a = action,
        };
        self
    }
}

pub(crate) const MAX_DISK_HISTORY_BYTES: u64 = 4 * 1024 * 1024 * 1024;

impl Command {
    pub fn action(&self) -> HistoryAction {
        use HistoryAction::*;
        match self {
            Self::Stroke { .. } => Brush,
            Self::Transform { .. } => TransformCanvas,
            Self::Selection { .. } => Selection,
            Self::AddLayer { layer, .. } => {
                if layer.mask {
                    Mask
                } else {
                    AddLayer
                }
            }
            Self::RemoveLayer { .. } | Self::RemoveTree { .. } => RemoveLayer,
            Self::Properties { .. } => LayerProperties,
            Self::Appearance { before, after, .. } => {
                if before.offset_x != after.offset_x || before.offset_y != after.offset_y {
                    MoveLayer
                } else if before.blend != after.blend {
                    LayerBlend
                } else if before.fill != after.fill {
                    LayerFill
                } else {
                    LayerLocks
                }
            }
            Self::Structure { before, after } => {
                if after.len() > before.len() {
                    Group
                } else if after.len() < before.len() {
                    Ungroup
                } else if before
                    .iter()
                    .zip(after)
                    .any(|(a, b)| a.clipped != b.clipped)
                {
                    Clipping
                } else {
                    Reparent
                }
            }
        }
    }
    fn verify_tiles(&self) -> Result<()> {
        match self {
            Self::Structure { .. } | Self::Selection { .. } => {}
            Self::Transform { before, after, .. } => {
                for layer in before.iter().chain(after) {
                    for tile in layer.tiles.values() {
                        drop(tile.try_pixels()?);
                    }
                }
            }
            Self::RemoveTree { layers, .. } => {
                for layer in layers {
                    for tile in layer.tiles.values() {
                        drop(tile.try_pixels()?);
                    }
                }
            }
            Self::Stroke { changes, .. } => {
                for change in changes {
                    for tile in change.before.iter().chain(&change.after) {
                        drop(tile.try_pixels()?);
                    }
                }
            }
            Self::AddLayer { layer, .. } | Self::RemoveLayer { layer, .. } => {
                for tile in layer.tiles.values() {
                    drop(tile.try_pixels()?);
                }
            }
            Self::Properties { .. } | Self::Appearance { .. } => {}
        }
        Ok(())
    }
    pub fn bytes(&self) -> usize {
        // Conservative bound: count both before and after even when shared by adjacent commands.
        let payload = match self {
            Self::Selection { before, after } => (before.steps().len() + after.steps().len()) * 64,
            Self::Structure { before, after } => {
                before.iter().chain(after).map(|l| l.name.len() + 128).sum()
            }
            Self::RemoveTree { layers, .. } => layers
                .iter()
                .map(|l| l.tiles.len() * TILE_BYTES + l.name.len() + 128)
                .sum(),
            Self::Stroke { changes, .. } => changes.len() * (2 * TILE_BYTES + 128),
            Self::Transform { before, after, .. } => before
                .iter()
                .chain(after)
                .map(|l| l.tiles.len() * TILE_BYTES + l.name.len() + 128)
                .sum(),
            Self::AddLayer { layer, .. } | Self::RemoveLayer { layer, .. } => {
                layer.tiles.len() * TILE_BYTES + layer.name.len()
            }
            Self::Properties { .. } | Self::Appearance { .. } => 0,
        };
        payload + 128
    }
}

#[derive(Debug, Default)]
pub(crate) struct History {
    pub undo: VecDeque<StoredCommand>,
    pub redo: Vec<StoredCommand>,
    pub bytes: usize,
    pub disk_bytes: u64,
    pub evicted: bool,
}

impl History {
    pub fn commit(&mut self, command: StoredCommand, max_bytes: usize, max_commands: usize) {
        self.bytes -= self.redo.iter().map(StoredCommand::bytes).sum::<usize>();
        self.disk_bytes -= self.redo.iter().map(StoredCommand::disk_bytes).sum::<u64>();
        self.redo.clear();
        self.bytes += command.bytes();
        self.disk_bytes += command.disk_bytes();
        self.undo.push_back(command);
        self.trim(max_bytes, max_commands, MAX_DISK_HISTORY_BYTES);
    }
    fn trim(&mut self, max_bytes: usize, max_commands: usize, max_disk_bytes: u64) {
        while self.bytes > max_bytes
            || self.disk_bytes > max_disk_bytes
            || self.undo.len() > max_commands
        {
            if let Some(oldest) = self.undo.pop_front() {
                self.evicted = true;
                self.bytes -= oldest.bytes();
                self.disk_bytes -= oldest.disk_bytes();
            } else {
                break;
            }
        }
    }
    /// Applying a smaller command limit evicts the oldest undo records and marks the boundary
    /// unreachable, exactly like committing past the limit. The redo branch is dropped, since a
    /// record that no longer fits behind the new limit cannot stay redoable.
    pub fn apply_command_limit(&mut self, max_commands: usize) {
        if max_commands == 0 {
            return;
        }
        self.bytes -= self.redo.iter().map(StoredCommand::bytes).sum::<usize>();
        self.disk_bytes -= self.redo.iter().map(StoredCommand::disk_bytes).sum::<u64>();
        self.redo.clear();
        while self.undo.len() > max_commands {
            if let Some(oldest) = self.undo.pop_front() {
                self.evicted = true;
                self.bytes -= oldest.bytes();
                self.disk_bytes -= oldest.disk_bytes();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disk_quota_evicts_oldest_and_deletes_only_its_backing() {
        let command = Command::Stroke {
            layer: 1,
            changes: vec![TileChange {
                coord: TileCoord { x: 0, y: 0 },
                before: None,
                after: Some(Arc::new(Tile::default())),
            }],
        };
        let first = StoredCommand::prepare(&command, 128).unwrap();
        let second = StoredCommand::prepare(&command, 128)
            .unwrap()
            .with_action(HistoryAction::Eraser);
        let StoredCommand::Encoded(a, _) = &first else {
            unreachable!()
        };
        let StoredCommand::Encoded(b, _) = &second else {
            unreachable!()
        };
        let paths = (a.disk_path_for_test(), b.disk_path_for_test());
        let length = b.disk_bytes();
        let mut history = History::default();
        history.commit(first, 1024, 100);
        history.commit(second, 1024, 100);
        history.trim(1024, 100, length);
        assert_eq!(history.undo.len(), 1);
        assert_eq!(history.undo.back().unwrap().action(), HistoryAction::Eraser);
        assert_eq!(history.disk_bytes, length);
        assert!(!paths.0.exists());
        assert!(paths.1.exists());
        history.undo.back().unwrap().materialize().unwrap();
        drop(history);
        assert!(!paths.1.exists());
    }
    #[test]
    fn lowering_the_command_limit_evicts_oldest_and_drops_redo() {
        let command = Command::Stroke {
            layer: 1,
            changes: vec![TileChange {
                coord: TileCoord { x: 0, y: 0 },
                before: None,
                after: Some(Arc::new(Tile::default())),
            }],
        };
        let mut history = History::default();
        for index in 0..5 {
            let action = if index % 2 == 0 {
                HistoryAction::Brush
            } else {
                HistoryAction::Eraser
            };
            history.commit(
                StoredCommand::prepare(&command, 4096)
                    .unwrap()
                    .with_action(action),
                1 << 20,
                100,
            );
        }
        assert_eq!(history.undo.len(), 5);
        assert!(!history.evicted);
        // A smaller limit keeps the newest records and marks the boundary unreachable.
        history.apply_command_limit(2);
        assert_eq!(history.undo.len(), 2);
        assert!(history.evicted);
        // Command 5 (index 4) is a brush, so the brush is the newest surviving record.
        assert_eq!(history.undo.back().unwrap().action(), HistoryAction::Brush);
        // Dropping the newest three leaves an eraser at the evicted boundary.
        assert_eq!(history.redo.len(), 0);
        // Undoing twice would present the erased records again; the limit change drops that branch.
        let first = StoredCommand::prepare(&command, 4096)
            .unwrap()
            .with_action(HistoryAction::Brush);
        let mut branched = History::default();
        branched.commit(first, 1 << 20, 100);
        branched.commit(
            StoredCommand::prepare(&command, 4096)
                .unwrap()
                .with_action(HistoryAction::Eraser),
            1 << 20,
            100,
        );
        // Simulate one undo: the newest record moves to the redo branch.
        let moved = branched.undo.pop_back().unwrap();
        branched.bytes -= moved.bytes();
        branched.disk_bytes -= moved.disk_bytes();
        branched.redo.push(moved);
        assert_eq!((branched.undo.len(), branched.redo.len()), (1, 1));
        branched.apply_command_limit(1);
        assert_eq!((branched.undo.len(), branched.redo.len()), (1, 0));
        // A zero limit is rejected by the caller, so it must never silently wipe the history.
        history.apply_command_limit(0);
        assert_eq!(history.undo.len(), 2);
        // Raising the limit only affects future commits.
        let before = history.bytes;
        history.apply_command_limit(64);
        assert_eq!(history.undo.len(), 2);
        assert_eq!(history.bytes, before);
    }
}
