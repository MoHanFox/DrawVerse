//! The existing little-endian field format, streamed in bounded blocks.
use crate::{Error, Result};
use std::{
    io::{self, Read, Write},
    sync::Arc,
};

const VECTORS: usize = 4096;
const BLOCK_VECTORS: usize = 512;

pub(super) fn write(writer: &mut dyn Write, values: &[[f32; 2]]) -> io::Result<()> {
    debug_assert_eq!(values.len(), VECTORS);
    let mut bytes = [0; BLOCK_VECTORS * 8];
    for chunk in values.chunks(BLOCK_VECTORS) {
        for (vector, output) in chunk.iter().zip(bytes.chunks_exact_mut(8)) {
            output[..4].copy_from_slice(&vector[0].to_le_bytes());
            output[4..].copy_from_slice(&vector[1].to_le_bytes());
        }
        writer.write_all(&bytes[..chunk.len() * 8])?;
    }
    Ok(())
}

pub(super) fn read(reader: &mut dyn Read, retain: bool) -> Result<Option<Arc<Vec<[f32; 2]>>>> {
    let mut bytes = [0; BLOCK_VECTORS * 8];
    let mut values = retain.then(|| Vec::with_capacity(VECTORS));
    for _ in 0..VECTORS / BLOCK_VECTORS {
        reader
            .read_exact(&mut bytes)
            .map_err(|e| Error::HistoryStorage(e.to_string()))?;
        for vector in bytes.chunks_exact(8) {
            let value = [
                f32::from_le_bytes(vector[..4].try_into().expect("four bytes")),
                f32::from_le_bytes(vector[4..].try_into().expect("four bytes")),
            ];
            if value.iter().any(|v| !v.is_finite() || v.abs() > 2_000_000.) {
                return Err(Error::HistoryStorage(
                    "invalid liquify history displacement".into(),
                ));
            }
            if let Some(values) = &mut values {
                values.push(value);
            }
        }
    }
    Ok(values.map(Arc::new))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scalar_format_and_float_bits_are_preserved() {
        let field: Vec<_> = (0..VECTORS)
            .map(|i| match i % 4 {
                0 => [-0., f32::from_bits(1)],
                1 => [-2_000_000., 2_000_000.],
                _ => [i as f32 / 7., -(i as f32) / 9.],
            })
            .collect();
        let expected: Vec<_> = field
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let mut bytes = Vec::new();
        write(&mut bytes, &field).unwrap();
        assert_eq!(bytes, expected);
        let restored = read(&mut bytes.as_slice(), true).unwrap().unwrap();
        for (a, b) in restored.iter().zip(field) {
            assert_eq!(a.map(f32::to_bits), b.map(f32::to_bits));
        }
        assert!(read(&mut bytes.as_slice(), false).unwrap().is_none());
    }
    #[test]
    fn inactive_side_still_rejects_invalid_displacements_and_truncation() {
        let bytes = vec![0; VECTORS * 8];
        for retain in [false, true] {
            for value in [
                f32::NAN,
                f32::INFINITY,
                -f32::INFINITY,
                2_000_001.,
                -2_000_001.,
            ] {
                let mut invalid = bytes.clone();
                invalid[bytes.len() - 4..].copy_from_slice(&value.to_le_bytes());
                assert!(read(&mut invalid.as_slice(), retain).is_err());
            }
            for end in [0, 1, 4095, 4096, bytes.len() - 1] {
                assert!(read(&mut &bytes[..end], retain).is_err());
            }
        }
    }
}
