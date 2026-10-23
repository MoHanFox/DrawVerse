//! Conservative remapping of this renderer's existing composed Mips, not a layer cache.
use super::*;

/// A certificate stamped when an entry is built. Every explicit source belongs to this
/// one raster; all other contributors are spatially uniform throughout the canvas.
pub(super) fn certificate(snapshot: &DocumentSnapshot) -> Option<u64> {
    let mut raster = None;
    for layer in snapshot.layers() {
        let a = layer.appearance();
        if layer.is_group()
            || layer.is_mask()
            || layer.is_clipped()
            || layer.parent_id() != 0
            || a.blend != BlendMode::Normal
        {
            return None;
        }
        if layer.tile_count() != 0 {
            if raster.is_some() || layer.white_extent() != (0, 0) {
                return None;
            }
            raster = Some(layer.id());
        } else if layer.white_extent() != (0, 0)
            && (layer.white_extent() != (snapshot.width, snapshot.height)
                || a.offset_x != 0
                || a.offset_y != 0)
        {
            return None;
        }
    }
    raster
}

#[derive(Default)]
pub(super) struct Outcome {
    pub reason: u32,
    pub remapped: u64,
    pub rejected: u64,
}

pub(super) fn checkpoint(
    stage: &'static str,
    cancel: &CancellationToken,
) -> Result<(), RenderError> {
    #[cfg(test)]
    CANCEL_AT.with(|at| {
        if let Some((target, remaining)) = at.get() {
            if target == stage {
                if remaining == 1 {
                    at.set(None);
                    cancel.cancel();
                } else {
                    at.set(Some((target, remaining - 1)));
                }
            }
        }
    });
    #[cfg(not(test))]
    let _ = stage;
    super::checkpoint(cancel)
}
#[cfg(test)]
thread_local! { static CANCEL_AT: std::cell::Cell<Option<(&'static str,usize)>> = const { std::cell::Cell::new(None) }; }

fn full(coord: TileCoord, dimensions: (u32, u32)) -> bool {
    (u64::from(coord.x) + 1) * u64::from(TILE_SIZE) <= u64::from(dimensions.0)
        && (u64::from(coord.y) + 1) * u64::from(TILE_SIZE) <= u64::from(dimensions.1)
}
impl Cache {
    pub(super) fn reconfigure(
        &mut self,
        snapshot: &DocumentSnapshot,
        configuration: Vec<LayerConfiguration>,
        coordinates: &BTreeSet<TileCoord>,
        lod: u32,
        format: PixelFormat,
        cancel: &CancellationToken,
    ) -> Result<Outcome, RenderError> {
        checkpoint("qualification", cancel)?;
        let qualification = (|| {
            if self.entries.is_empty() {
                return Err(3);
            }
            if self.dimensions != (snapshot.width, snapshot.height) || lod > TILE_SIZE.ilog2() {
                return Err(7);
            }
            let id = self.translation_layer.ok_or(4u32)?;
            if self.configuration.len() != configuration.len() {
                return Err(5);
            }
            let mut changed = None;
            for (i, (old, new)) in self.configuration.iter().zip(&configuration).enumerate() {
                if old != new {
                    if changed.is_some() {
                        return Err(5);
                    }
                    changed = Some(i);
                }
            }
            let i = changed.ok_or(5u32)?;
            let old = &self.configuration[i];
            let new = &configuration[i];
            let mut expected = old.clone();
            expected.offset = new.offset;
            if expected != *new || new.id != id {
                return Err(5);
            }
            let dx = i64::from(new.offset.0) - i64::from(old.offset.0);
            let dy = i64::from(new.offset.1) - i64::from(old.offset.1);
            let tile = i64::from(TILE_SIZE);
            if (dx == 0 && dy == 0) || dx % tile != 0 || dy % tile != 0 {
                return Err(6);
            }
            Ok((i, dx / tile, dy / tile, id))
        })();
        checkpoint("after_qualification", cancel)?;
        let (i, dx, dy, id) = match qualification {
            Ok(value) => value,
            Err(reason) => {
                self.entries.clear();
                self.stats.retained_bytes = 0;
                self.stats.entries = 0;
                self.configuration = configuration;
                return Ok(Outcome {
                    reason,
                    ..Default::default()
                });
            }
        };
        // Detach before destructive work. Cancellation leaves EMPTY + OLD configuration.
        // Never expose a partly remapped table labelled as the new configuration.
        let previous = std::mem::take(&mut self.entries);
        self.stats.retained_bytes = 0;
        self.stats.entries = 0;
        let mut next = HashMap::with_capacity(previous.len());
        let mut outcome = Outcome {
            reason: 1,
            ..Default::default()
        };
        let mut bytes = 0;
        let layer = &snapshot.layers()[i];
        let opacity = layer.properties().opacity * layer.appearance().fill;
        for ((coord, level, kind), mut entry) in previous {
            checkpoint("remap", cancel)?;
            let target = i64::from(coord.x)
                .checked_add(dx)
                .zip(i64::from(coord.y).checked_add(dy))
                .and_then(|(x, y)| {
                    Some(TileCoord {
                        x: u32::try_from(x).ok()?,
                        y: u32::try_from(y).ok()?,
                    })
                });
            let target = target.filter(|&target| {
                entry.translation_layer == Some(id)
                    && level == lod
                    && kind == format
                    && full(coord, self.dimensions)
                    && full(target, self.dimensions)
                    && coordinates.contains(&target)
            });
            let Some(target) = target else {
                outcome.rejected += 1;
                continue;
            };
            let sources = layer.canvas_tile_sources(target);
            if sources.len() != entry.sources.len()
                || !sources.iter().zip(&entry.sources).all(|(tile, old)| {
                    Arc::as_ptr(tile) == old.tile.as_ptr() && opacity == old.opacity
                })
            {
                outcome.rejected += 1;
                continue;
            }
            entry.translated = true;
            bytes += entry.bytes;
            // A checked translation is injective for the same level/format; no in-place collision.
            debug_assert!(!next.contains_key(&(target, level, kind)));
            next.insert((target, level, kind), entry);
            outcome.remapped += 1;
        }
        checkpoint("commit", cancel)?;
        self.entries = next;
        self.configuration = configuration;
        self.stats.retained_bytes = bytes;
        self.stats.entries = self.entries.len();
        checkpoint("after_commit", cancel)?;
        Ok(outcome)
    }
}

#[cfg(test)]
#[path = "translation_tests.rs"]
mod tests;
