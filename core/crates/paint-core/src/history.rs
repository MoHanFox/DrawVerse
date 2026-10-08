use crate::{
    history_storage::Payload, Layer, LayerId, LayerProperties, Result, Tile, TileCoord, TILE_BYTES,
};
use std::{collections::VecDeque, sync::Arc};

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
    Inline(Command),
    Encoded(Payload),
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
            Ok(Self::Inline(command.clone()))
        } else {
            Ok(Self::Encoded(Payload::encode_in(
                command,
                max_bytes - 128,
                storage,
            )?))
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
            Self::Inline(command) => {
                // Inline history now retains identities, including cold pages.
                // Validate those pages before moving either history stack.
                command.verify_tiles()?;
                if let Some(pool) = pool {
                    pool.trim()?;
                }
                Ok(command.clone())
            }
            Self::Encoded(payload) => payload.decode_in(pool),
        }
    }
    fn bytes(&self) -> usize {
        match self {
            Self::Inline(command) => command.bytes(),
            Self::Encoded(payload) => payload.memory_bytes() + 128,
        }
    }
    fn disk_bytes(&self) -> u64 {
        match self {
            Self::Inline(_) => 0,
            Self::Encoded(payload) => payload.disk_bytes(),
        }
    }
}

pub(crate) const MAX_DISK_HISTORY_BYTES: u64 = 4 * 1024 * 1024 * 1024;

impl Command {
    fn verify_tiles(&self) -> Result<()> {
        match self {
            Self::Structure { .. } | Self::Selection { .. } => {}
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
        let second = StoredCommand::prepare(&command, 128).unwrap();
        let StoredCommand::Encoded(a) = &first else {
            unreachable!()
        };
        let StoredCommand::Encoded(b) = &second else {
            unreachable!()
        };
        let paths = (a.disk_path_for_test(), b.disk_path_for_test());
        let length = b.disk_bytes();
        let mut history = History::default();
        history.commit(first, 1024, 100);
        history.commit(second, 1024, 100);
        history.trim(1024, 100, length);
        assert_eq!(history.undo.len(), 1);
        assert_eq!(history.disk_bytes, length);
        assert!(!paths.0.exists());
        assert!(paths.1.exists());
        history.undo.back().unwrap().materialize().unwrap();
        drop(history);
        assert!(!paths.1.exists());
    }
}
