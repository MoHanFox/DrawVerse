//! Bounded ABR sampled records and 6/10 section framing; no settings guesses or IO.
use crate::{check, Error, Result};
use paint_core::{BrushMask, MAX_BRUSH_MASK_EDGE, MAX_CATALOG_MASK_BYTES};
use paint_task::CancellationToken;
use std::sync::Arc;

pub struct SampledTip {
    pub source_version: u16,
    pub misc: u32,
    pub short_bounds: [i16; 4],
    pub name: Option<String>,
    pub spacing_percent: u16,
    pub antialias: bool,
    pub bounds: [i32; 4],
    pub mask: Arc<BrushMask>,
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
            .ok_or(Error::Limit("ABR offset"))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::Invalid("truncated ABR record"))?;
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
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_be_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }
    fn done(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
fn row(bytes: &[u8], width: usize) -> Result<Vec<u8>> {
    let mut r = Reader { bytes, offset: 0 };
    let mut output = Vec::with_capacity(width);
    while !r.done() {
        let control = r.u8()? as i8;
        match control {
            0..=127 => {
                let count = control as usize + 1;
                if count > width - output.len() {
                    return Err(Error::Invalid("ABR literal row overflow"));
                }
                output.extend_from_slice(r.take(count)?);
            }
            -127..=-1 => {
                let count = (1 - i16::from(control)) as usize;
                if count > width - output.len() {
                    return Err(Error::Invalid("ABR repeated row overflow"));
                }
                let value = r.u8()?;
                output.resize(output.len() + count, value);
            }
            _ => {}
        }
    }
    if output.len() != width {
        return Err(Error::Invalid("ABR incomplete row"));
    }
    Ok(output)
}
fn mask(
    record: &mut Reader<'_>,
    bounds: [i32; 4],
    token: &CancellationToken,
    resident: &mut usize,
) -> Result<Arc<BrushMask>> {
    let width = i64::from(bounds[3]) - i64::from(bounds[1]);
    let height = i64::from(bounds[2]) - i64::from(bounds[0]);
    if width <= 0
        || height <= 0
        || width > i64::from(MAX_BRUSH_MASK_EDGE)
        || height > i64::from(MAX_BRUSH_MASK_EDGE)
    {
        return Err(Error::Limit("ABR tip dimensions"));
    }
    if record.u16()? != 8 {
        return Err(Error::Unsupported("ABR sampled bit depth".into()));
    }
    let (width, height) = (width as usize, height as usize);
    let (mut w, mut h) = (width, height);
    let mut cost = 0;
    loop {
        cost += w * h;
        if w == 1 && h == 1 {
            break;
        }
        w = w.div_ceil(2);
        h = h.div_ceil(2);
    }
    if cost > MAX_CATALOG_MASK_BYTES - *resident {
        return Err(Error::Limit("ABR decoded tip budget"));
    }
    let pixels = match record.u8()? {
        0 => record.take(width * height)?.to_vec(),
        1 => {
            let mut lengths = Vec::with_capacity(height);
            for _ in 0..height {
                lengths.push(record.u16()? as usize);
            }
            let mut pixels = Vec::with_capacity(width * height);
            for length in lengths {
                check(token)?;
                pixels.extend(row(record.take(length)?, width)?);
            }
            pixels
        }
        _ => return Err(Error::Unsupported("ABR sample compression".into())),
    };
    let mask = Arc::new(BrushMask::new(width as u32, height as u32, pixels)?);
    *resident += mask.resident_bytes();
    Ok(mask)
}
pub fn decode(bytes: &[u8], token: &CancellationToken) -> Result<Vec<SampledTip>> {
    check(token)?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err(Error::Limit("ABR file bytes"));
    }
    let mut file = Reader { bytes, offset: 0 };
    let version = file.u16()?;
    if !matches!(version, 1 | 2) {
        return Err(Error::Unsupported(
            "ABR sampled reader currently requires version 1/2".into(),
        ));
    }
    let count = file.u16()?;
    if count == 0 || count > 1024 {
        return Err(Error::Limit("ABR brush count"));
    }
    let mut tips = Vec::new();
    let mut resident = 0;
    for _ in 0..count {
        check(token)?;
        let kind = file.u16()?;
        let length = file.u32()? as usize;
        let mut record = Reader {
            bytes: file.take(length)?,
            offset: 0,
        };
        if kind != 2 {
            return Err(Error::Unsupported(format!("ABR brush record type {kind}")));
        }
        let misc = record.u32()?;
        let spacing_percent = record.u16()?;
        let name = if version == 2 {
            let count = record.u32()? as usize;
            if count > 1024 {
                return Err(Error::Limit("ABR UTF16 name"));
            }
            let mut units = Vec::with_capacity(count);
            for _ in 0..count {
                units.push(record.u16()?);
            }
            if units.last() == Some(&0) {
                units.pop();
            }
            if units.contains(&0) {
                return Err(Error::Invalid("ABR embedded NUL name"));
            }
            Some(String::from_utf16(&units).map_err(|_| Error::Invalid("ABR UTF16 name"))?)
        } else {
            None
        };
        let aa = record.u8()?;
        if aa > 1 {
            return Err(Error::Unsupported("ABR antialias flag".into()));
        }
        let short_bounds = [
            record.u16()? as i16,
            record.u16()? as i16,
            record.u16()? as i16,
            record.u16()? as i16,
        ];
        let bounds = [record.i32()?, record.i32()?, record.i32()?, record.i32()?];
        let mask = mask(&mut record, bounds, token, &mut resident)?;
        if !record.done() {
            return Err(Error::Unsupported("ABR trailing record metadata".into()));
        }
        tips.push(SampledTip {
            source_version: version,
            misc,
            short_bounds,
            name,
            spacing_percent,
            antialias: aa == 1,
            bounds,
            mask,
        });
    }
    if !file.done() {
        return Err(Error::Invalid("ABR trailing file data"));
    }
    check(token)?;
    Ok(tips)
}

/// Uninterpreted source metadata, retained in file order, never executed as settings.
pub struct OpaqueSection {
    pub key: [u8; 4],
    pub bytes: Box<[u8]>,
}
pub struct SectionedTip {
    pub prefix: Box<[u8]>,
    pub bounds: [i32; 4],
    pub mask: Arc<BrushMask>,
    pub trailing_metadata: Box<[u8]>,
}
pub struct SectionedAbr {
    pub version: u16,
    pub subversion: u16,
    pub tips: Vec<SectionedTip>,
    pub metadata: Vec<OpaqueSection>,
}
impl SectionedAbr {
    /// Parse all descriptor sections explicitly; sampling alone proves no setting support.
    pub fn descriptors(
        &self,
        token: &CancellationToken,
    ) -> Result<Vec<crate::ps_descriptor::Descriptor>> {
        let mut parsed = Vec::new();
        for section in &self.metadata {
            if &section.key == b"desc" {
                parsed.push(crate::ps_descriptor::decode(&section.bytes, token)?);
            }
        }
        if parsed.is_empty() {
            return Err(Error::Invalid("ABR descriptor section missing"));
        }
        Ok(parsed)
    }
}
pub fn decode_sectioned(bytes: &[u8], token: &CancellationToken) -> Result<SectionedAbr> {
    check(token)?;
    if bytes.len() > 128 * 1024 * 1024 {
        return Err(Error::Limit("ABR file bytes"));
    }
    let mut file = Reader { bytes, offset: 0 };
    let version = file.u16()?;
    let subversion = file.u16()?;
    if !matches!(version, 6 | 10) || !matches!(subversion, 1 | 2) {
        return Err(Error::Unsupported(
            "ABR sectioned version/subversion".into(),
        ));
    }
    let mut tips = Vec::new();
    let mut metadata = Vec::new();
    let mut sampled = false;
    let mut resident = 0;
    let mut sections = 0;
    while !file.done() {
        check(token)?;
        sections += 1;
        if sections > 4096 {
            return Err(Error::Limit("ABR section count"));
        }
        if file.take(4)? != b"8BIM" {
            return Err(Error::Invalid("ABR section signature"));
        }
        let key: [u8; 4] = file.take(4)?.try_into().expect("four bytes");
        let length = file.u32()? as usize;
        let payload = file.take(length)?;
        if &key != b"samp" {
            metadata.push(OpaqueSection {
                key,
                bytes: payload.into(),
            });
            continue;
        }
        if sampled {
            return Err(Error::Unsupported("multiple ABR sample sections".into()));
        }
        sampled = true;
        let mut samples = Reader {
            bytes: payload,
            offset: 0,
        };
        while !samples.done() {
            check(token)?;
            if tips.len() == 1024 {
                return Err(Error::Limit("ABR sample count"));
            }
            let length = samples.u32()? as usize;
            let mut record = Reader {
                bytes: samples.take(length)?,
                offset: 0,
            };
            let prefix = record.take(if subversion == 1 { 47 } else { 301 })?.into();
            let bounds = [record.i32()?, record.i32()?, record.i32()?, record.i32()?];
            let mask = mask(&mut record, bounds, token, &mut resident)?;
            let trailing_metadata = record.take(record.bytes.len() - record.offset)?.into();
            samples.take((4 - length % 4) % 4)?;
            tips.push(SectionedTip {
                prefix,
                bounds,
                mask,
                trailing_metadata,
            });
        }
    }
    if tips.is_empty() {
        return Err(Error::Invalid("ABR missing sampled tips"));
    }
    check(token)?;
    Ok(SectionedAbr {
        version,
        subversion,
        tips,
        metadata,
    })
}
