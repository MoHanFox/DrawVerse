//! Bounded local decoder for ABR `patt` records, including 16-bit channels.
use crate::{check, Error, Result};
use paint_core::{MAX_BRUSH_MASK_EDGE, MAX_CATALOG_MASK_BYTES};
use paint_task::CancellationToken;

pub(crate) struct DecodedPattern {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub gray: Vec<u8>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(Error::Limit("ABR pattern offset"))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::Invalid("truncated ABR pattern"))?;
        self.offset = end;
        Ok(bytes)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(
            self.take(2)?.try_into().expect("two bytes"),
        ))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }
    fn skip(&mut self, count: usize) -> Result<()> {
        self.take(count).map(|_| ())
    }
    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
    fn done(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

fn rect(reader: &mut Reader<'_>) -> Result<(u32, u32)> {
    let top = reader.u32()?;
    let left = reader.u32()?;
    let bottom = reader.u32()?;
    let right = reader.u32()?;
    let width = right
        .checked_sub(left)
        .ok_or(Error::Invalid("ABR pattern channel bounds"))?;
    let height = bottom
        .checked_sub(top)
        .ok_or(Error::Invalid("ABR pattern channel bounds"))?;
    if width == 0 || height == 0 || width > MAX_BRUSH_MASK_EDGE || height > MAX_BRUSH_MASK_EDGE {
        return Err(Error::Limit("ABR pattern channel dimensions"));
    }
    Ok((width, height))
}

fn packbits_row(bytes: &[u8], expected: usize) -> Result<Vec<u8>> {
    let mut reader = Reader { bytes, offset: 0 };
    let mut output = Vec::with_capacity(expected);
    while !reader.done() && output.len() < expected {
        let control = reader.u8()? as i8;
        match control {
            0..=127 => {
                let count = control as usize + 1;
                let values = reader.take(count)?;
                if output.len() + count > expected {
                    return Err(Error::Invalid("ABR pattern RLE overflow"));
                }
                output.extend_from_slice(values);
            }
            -127..=-1 => {
                let count = (1 - i16::from(control)) as usize;
                let value = reader.u8()?;
                if output.len() + count > expected {
                    return Err(Error::Invalid("ABR pattern RLE overflow"));
                }
                output.resize(output.len() + count, value);
            }
            _ => {}
        }
    }
    if output.len() != expected {
        return Err(Error::Invalid("ABR pattern RLE row length"));
    }
    Ok(output)
}

fn channel(bytes: &[u8], depth: u32, width: u32, height: u32, compression: u8) -> Result<Vec<u16>> {
    let bpp = match depth {
        8 => 1,
        16 => 2,
        _ => return Err(Error::Unsupported("ABR pattern channel depth".into())),
    };
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(bpp))
        .ok_or(Error::Limit("ABR pattern channel bytes"))?;
    let raw = match compression {
        0 => {
            if bytes.len() < expected {
                return Err(Error::Invalid("truncated ABR pattern raw channel"));
            }
            bytes[..expected].to_vec()
        }
        1 => {
            let mut reader = Reader { bytes, offset: 0 };
            let mut counts = Vec::with_capacity(height as usize);
            for _ in 0..height {
                counts.push(reader.u16()? as usize);
            }
            let mut output = Vec::with_capacity(expected);
            for count in counts {
                let row = reader.take(count)?;
                output.extend(packbits_row(row, width as usize * bpp)?);
            }
            output
        }
        _ => return Err(Error::Unsupported("ABR pattern compression".into())),
    };
    Ok(if bpp == 1 {
        raw.into_iter().map(u16::from).collect()
    } else {
        raw.chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect()
    })
}

fn scale(value: u16, depth: u32) -> u8 {
    if depth == 8 {
        value as u8
    } else {
        ((u32::from(value) * 255 + 32767) / 65535) as u8
    }
}

fn luma(r: u8, g: u8, b: u8) -> u8 {
    (0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)).round() as u8
}

fn gray(
    mode: u32,
    palette: Option<&[u8; 768]>,
    planes: &[(u32, Vec<u16>)],
    width: u32,
    height: u32,
) -> Result<Vec<u8>> {
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or(Error::Limit("ABR pattern pixels"))?;
    let plane = |index: usize| -> Result<&[u16]> {
        planes
            .get(index)
            .map(|(_, values)| values.as_slice())
            .filter(|values| values.len() >= count)
            .ok_or(Error::Invalid("ABR pattern channel data"))
    };
    match mode {
        1 => {
            let (depth, values) = &planes[0];
            if values.len() < count {
                return Err(Error::Invalid("ABR pattern channel data"));
            }
            Ok(values[..count].iter().map(|v| scale(*v, *depth)).collect())
        }
        2 => {
            let palette = palette.ok_or(Error::Invalid("ABR pattern palette"))?;
            let values = plane(0)?;
            Ok(values[..count]
                .iter()
                .map(|v| {
                    let index = (*v as usize).min(255) * 3;
                    luma(palette[index], palette[index + 1], palette[index + 2])
                })
                .collect())
        }
        3 => {
            if planes.len() == 1 {
                let (depth, values) = &planes[0];
                return Ok(values[..count].iter().map(|v| scale(*v, *depth)).collect());
            }
            let (rd, r) = &planes[0];
            let (gd, g) = &planes[1];
            let (bd, b) = &planes[2];
            if r.len() < count || g.len() < count || b.len() < count {
                return Err(Error::Invalid("ABR pattern RGB channels"));
            }
            Ok((0..count)
                .map(|i| luma(scale(r[i], *rd), scale(g[i], *gd), scale(b[i], *bd)))
                .collect())
        }
        _ => Err(Error::Unsupported("ABR pattern image mode".into())),
    }
}

fn record(data: &[u8], token: &CancellationToken) -> Result<Option<DecodedPattern>> {
    let mut reader = Reader {
        bytes: data,
        offset: 0,
    };
    let _version = reader.u32()?;
    let mode = reader.u32()?;
    let _header_height = reader.u16()?;
    let _header_width = reader.u16()?;
    let name_units = reader.u32()? as usize;
    if name_units > 1024 * 1024 {
        return Err(Error::Limit("ABR pattern name"));
    }
    reader.skip(
        name_units
            .checked_mul(2)
            .ok_or(Error::Limit("ABR pattern name"))?,
    )?;
    let id_len = reader.u8()? as usize;
    if id_len == 0 || id_len > 128 {
        return Err(Error::Invalid("ABR pattern identity"));
    }
    let id_bytes = reader.take(id_len)?;
    if !id_bytes.is_ascii() || id_bytes.contains(&0) {
        return Err(Error::Invalid("ABR pattern identity"));
    }
    let id = String::from_utf8(id_bytes.to_vec()).expect("validated ASCII");
    let palette = if mode == 2 {
        let mut palette = [0u8; 768];
        palette.copy_from_slice(reader.take(768)?);
        reader.skip(4)?;
        Some(palette)
    } else {
        None
    };
    reader.skip(8 + 16)?;
    let _channel_slots = reader.u32()?;
    let mut planes = Vec::new();
    let mut dimensions = None;
    while reader.remaining() >= 4 {
        check(token)?;
        let written = reader.u32()?;
        if written == 0 {
            continue;
        }
        let array_len = reader.u32()? as usize;
        if array_len < 23 {
            return Err(Error::Invalid("ABR pattern channel length"));
        }
        let body_start = reader.offset;
        let body_end = body_start
            .checked_add(array_len)
            .filter(|end| *end <= data.len())
            .ok_or(Error::Invalid("truncated ABR pattern channel"))?;
        let depth = reader.u32()?;
        let (width, height) = rect(&mut reader)?;
        let depth2 = reader.u16()?;
        if depth != u32::from(depth2) || !matches!(depth, 8 | 16) {
            return Err(Error::Unsupported("ABR pattern channel depth".into()));
        }
        let compression = reader.u8()?;
        let pixel_start = reader.offset;
        let values = channel(
            &data[pixel_start..body_end],
            depth,
            width,
            height,
            compression,
        )?;
        if let Some((cw, ch)) = dimensions {
            if cw != width || ch != height {
                return Err(Error::Invalid("ABR pattern channel dimensions"));
            }
        } else {
            dimensions = Some((width, height));
        }
        planes.push((depth, values));
        reader.offset = body_end;
    }
    let (width, height) = dimensions.ok_or(Error::Invalid("ABR pattern channels"))?;
    if planes.is_empty() {
        return Ok(None);
    }
    Ok(Some(DecodedPattern {
        id,
        width,
        height,
        gray: gray(mode, palette.as_ref(), &planes, width, height)?,
    }))
}

pub(crate) fn decode(
    sections: &[crate::abr_sampled::OpaqueSection],
    token: &CancellationToken,
    resident: &mut usize,
) -> Result<Vec<DecodedPattern>> {
    let mut patterns = Vec::new();
    for section in sections.iter().filter(|section| &section.key == b"patt") {
        let data = &section.bytes;
        let mut at = 0usize;
        while at + 4 <= data.len() {
            check(token)?;
            let length =
                u32::from_be_bytes(data[at..at + 4].try_into().expect("four bytes")) as usize;
            let start = at
                .checked_add(4)
                .ok_or(Error::Limit("ABR pattern offset"))?;
            let end = start
                .checked_add(length)
                .filter(|end| *end <= data.len())
                .ok_or(Error::Invalid("truncated ABR pattern record"))?;
            if let Some(pattern) = record(&data[start..end], token)? {
                let pixels = (pattern.width as usize)
                    .checked_mul(pattern.height as usize)
                    .ok_or(Error::Limit("ABR pattern pixels"))?;
                *resident = resident
                    .checked_add(pixels)
                    .ok_or(Error::Limit("ABR pattern decode budget"))?;
                if *resident > MAX_CATALOG_MASK_BYTES {
                    return Err(Error::Limit("ABR pattern decode budget"));
                }
                patterns.push(pattern);
            }
            at = end
                .checked_add((4 - length % 4) % 4)
                .ok_or(Error::Limit("ABR pattern offset"))?;
            if at > data.len() {
                return Err(Error::Invalid("ABR pattern padding"));
            }
        }
        if at != data.len() {
            return Err(Error::Invalid("ABR pattern trailing bytes"));
        }
    }
    Ok(patterns)
}
