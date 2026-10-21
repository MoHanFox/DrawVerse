//! Budgeted immutable field deltas, with the existing checked blob as fallback.
use super::{history_codec, Field, MAX_FIELD_TILES};
use crate::{history_storage::Payload, Error, Result, TileCoord};
use paint_storage::ScratchSpace;
use std::{io::Write, sync::Arc};

#[derive(Debug)]
struct Change {
    key: TileCoord,
    values: [Option<Arc<Vec<[f32; 2]>>>; 2],
}

#[derive(Debug)]
pub(super) struct FieldHistory(Backing);

#[derive(Debug)]
enum Backing {
    References(Vec<Change>),
    Encoded(Payload),
}

impl FieldHistory {
    pub(super) fn capture(
        keys: &[TileCoord],
        before: &Field,
        after: &Field,
        memory_limit: usize,
        storage: Arc<ScratchSpace>,
    ) -> Result<Self> {
        // Charge every side independently even when records share an allocation.
        // Vec capacity, not length, is the memory kept alive by each Arc.
        let mut bytes = 128usize.saturating_add(keys.len().saturating_mul(size_of::<Change>()));
        for key in keys {
            for values in [before.get(key), after.get(key)].into_iter().flatten() {
                bytes = bytes.saturating_add(
                    64usize.saturating_add(values.capacity().saturating_mul(size_of::<[f32; 2]>())),
                );
            }
        }
        if bytes <= memory_limit {
            let mut changes = Vec::new();
            changes
                .try_reserve_exact(keys.len())
                .map_err(|_| Error::ResourceLimit("liquify field history references"))?;
            for key in keys {
                let values = [before.get(key).cloned(), after.get(key).cloned()];
                for value in values.iter().flatten() {
                    history_codec::validate(value)?;
                }
                changes.push(Change { key: *key, values });
            }
            return Ok(Self(Backing::References(changes)));
        }
        Ok(Self(Backing::Encoded(Payload::blob_in(
            memory_limit,
            storage,
            |writer| {
                writer.write_all(&(keys.len() as u32).to_le_bytes())?;
                for key in keys {
                    write_change(writer, *key, [before.get(key), after.get(key)])?;
                }
                Ok(())
            },
        )?)))
    }

    pub(super) fn restore(&self, field: &mut Field, redo: bool) -> Result<()> {
        match &self.0 {
            Backing::References(changes) => {
                // Both sides were validated before capture. A retained Arc is
                // immutable; subsequent edits allocate a new field tile.
                for change in changes {
                    set(field, change.key, change.values[usize::from(redo)].clone());
                }
                Ok(())
            }
            Backing::Encoded(payload) => payload.read_blob(|reader| {
                let mut bytes = [0; 4];
                let read_u32 = |reader: &mut dyn std::io::Read, bytes: &mut [u8; 4]| {
                    reader
                        .read_exact(bytes)
                        .map_err(|e| Error::HistoryStorage(e.to_string()))?;
                    Ok::<_, Error>(u32::from_le_bytes(*bytes))
                };
                let count = read_u32(reader, &mut bytes)? as usize;
                if count > MAX_FIELD_TILES * 2 {
                    return Err(Error::HistoryStorage(
                        "invalid liquify history count".into(),
                    ));
                }
                for _ in 0..count {
                    let key = TileCoord {
                        x: read_u32(reader, &mut bytes)?,
                        y: read_u32(reader, &mut bytes)?,
                    };
                    for side in 0..2 {
                        let mut present = [0];
                        reader
                            .read_exact(&mut present)
                            .map_err(|e| Error::HistoryStorage(e.to_string()))?;
                        let value = match present[0] {
                            0 => None,
                            1 => history_codec::read(reader, side == usize::from(redo))?,
                            _ => {
                                return Err(Error::HistoryStorage(
                                    "invalid liquify history flag".into(),
                                ))
                            }
                        };
                        if side == usize::from(redo) {
                            set(field, key, value);
                        }
                    }
                }
                Ok(())
            }),
        }
    }

    pub(super) fn disk_bytes(&self) -> u64 {
        match &self.0 {
            Backing::References(_) => 0,
            Backing::Encoded(payload) => payload.disk_bytes(),
        }
    }

    #[cfg(test)]
    pub(super) fn encoded_for_test(payload: Payload) -> Self {
        Self(Backing::Encoded(payload))
    }

    #[cfg(test)]
    pub(super) fn truncate_for_test(&mut self) {
        if let Backing::References(changes) = &self.0 {
            let payload = Payload::blob_in(usize::MAX, ScratchSpace::system(), |writer| {
                writer.write_all(&(changes.len() as u32).to_le_bytes())?;
                for change in changes {
                    write_change(
                        writer,
                        change.key,
                        change.values.each_ref().map(Option::as_ref),
                    )?;
                }
                Ok(())
            })
            .unwrap();
            self.0 = Backing::Encoded(payload);
        }
        let Backing::Encoded(payload) = &mut self.0 else {
            unreachable!()
        };
        payload.truncate_for_test();
    }
}

fn set(field: &mut Field, key: TileCoord, value: Option<Arc<Vec<[f32; 2]>>>) {
    if let Some(value) = value {
        field.insert(key, value);
    } else {
        field.remove(&key);
    }
}

fn write_change(
    writer: &mut dyn Write,
    key: TileCoord,
    values: [Option<&Arc<Vec<[f32; 2]>>>; 2],
) -> std::io::Result<()> {
    writer.write_all(&key.x.to_le_bytes())?;
    writer.write_all(&key.y.to_le_bytes())?;
    for values in values {
        writer.write_all(&[u8::from(values.is_some())])?;
        if let Some(values) = values {
            history_codec::write(writer, values)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(key: TileCoord, value: f32) -> Field {
        [(key, Arc::new(vec![[value, -value]; 4096]))].into()
    }

    #[test]
    fn references_preserve_bits_and_identity_and_cannot_be_modified_by_later_edits() {
        let key = TileCoord::from_signed(-3, 7);
        let before = field(key, -0.);
        let mut after = field(key, f32::from_bits(1));
        let history =
            FieldHistory::capture(&[key], &before, &after, 100_000, ScratchSpace::system())
                .unwrap();
        assert!(matches!(history.0, Backing::References(_)));
        let saved = after[&key].clone();
        Arc::make_mut(after.get_mut(&key).unwrap())[0] = [17., 19.];
        history.restore(&mut after, true).unwrap();
        assert!(Arc::ptr_eq(&after[&key], &saved));
        assert_eq!(after[&key][0].map(f32::to_bits), [1, 0x80000001]);
        history.restore(&mut after, false).unwrap();
        assert!(Arc::ptr_eq(&after[&key], &before[&key]));
        assert_eq!(after[&key][0].map(f32::to_bits), [0x80000000, 0]);
    }

    #[test]
    fn budget_counts_capacity_and_both_sides_and_falls_back_without_losing_data() {
        let key = TileCoord::from_signed(0, 0);
        let before = field(key, 1.);
        let mut after = field(key, 2.);
        let bytes = 128 + size_of::<Change>() + 2 * (64 + 4096 * 8);
        for (limit, referenced) in [(bytes, true), (bytes - 1, false), (0, false)] {
            let history =
                FieldHistory::capture(&[key], &before, &after, limit, ScratchSpace::system())
                    .unwrap();
            assert_eq!(matches!(history.0, Backing::References(_)), referenced);
            assert_eq!(history.disk_bytes() > 0, limit == 0);
            let mut restored = Field::new();
            history.restore(&mut restored, false).unwrap();
            assert_eq!(restored, before);
            history.restore(&mut restored, true).unwrap();
            assert_eq!(restored, after);
        }
        Arc::make_mut(after.get_mut(&key).unwrap()).reserve_exact(4096);
        let history =
            FieldHistory::capture(&[key], &before, &after, bytes, ScratchSpace::system()).unwrap();
        assert!(matches!(history.0, Backing::Encoded(_)));
    }

    #[test]
    fn inserted_and_removed_tiles_restore_on_either_backing() {
        let keys = [TileCoord::from_signed(-1, -2), TileCoord::from_signed(3, 4)];
        let before = field(keys[0], 1.);
        let after = field(keys[1], 2.);
        for limit in [0, usize::MAX] {
            let history =
                FieldHistory::capture(&keys, &before, &after, limit, ScratchSpace::system())
                    .unwrap();
            let mut restored = after.clone();
            history.restore(&mut restored, false).unwrap();
            assert_eq!(restored, before);
            history.restore(&mut restored, true).unwrap();
            assert_eq!(restored, after);
        }
    }

    #[test]
    fn references_validate_both_sides_before_capture() {
        let key = TileCoord::from_signed(0, 0);
        let valid = field(key, 0.);
        for value in [f32::NAN, f32::INFINITY, -2_000_001., 2_000_001.] {
            let invalid = field(key, value);
            for (before, after) in [(&valid, &invalid), (&invalid, &valid)] {
                assert!(matches!(
                    FieldHistory::capture(
                        &[key],
                        before,
                        after,
                        usize::MAX,
                        ScratchSpace::system(),
                    ),
                    Err(Error::HistoryStorage(_))
                ));
            }
        }
        let invalid = [(key, Arc::new(vec![[0.; 2]; 4095]))].into();
        assert!(FieldHistory::capture(
            &[key],
            &valid,
            &invalid,
            usize::MAX,
            ScratchSpace::system(),
        )
        .is_err());
    }
}
