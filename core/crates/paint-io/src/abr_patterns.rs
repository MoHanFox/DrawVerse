//! Bounded ABR `patt` resource decoding and descriptor-to-pattern association.
use crate::{
    brush_resource::{encode_mask, hash},
    check, Error, Result,
};
use paint_core::{BrushMask, MAX_BRUSH_MASK_EDGE, MAX_CATALOG_MASK_BYTES};
use paint_task::CancellationToken;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone)]
pub struct PatternResource {
    pub id: String,
    pub mask: Arc<BrushMask>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextureSource {
    pub pattern_id: String,
    pub depth: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub minimum_depth: f32,
    pub depth_jitter: f32,
    pub scale: f32,
    pub invert: bool,
    pub each_tip: bool,
    pub mode: Option<String>,
}

pub struct DecodedTextures {
    pub patterns: BTreeMap<String, PatternResource>,
    pub presets: BTreeMap<usize, TextureSource>,
}

fn be_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or(Error::Invalid("truncated ABR pattern"))?
            .try_into()
            .expect("checked four bytes"),
    ))
}

fn skip(offset: &mut usize, amount: usize, end: usize) -> Result<()> {
    *offset = offset
        .checked_add(amount)
        .filter(|next| *next <= end)
        .ok_or(Error::Invalid("truncated ABR pattern header"))?;
    Ok(())
}

#[allow(dead_code)]
fn preflight_pattern_records(
    data: &[u8],
    token: &CancellationToken,
    channel_budget: &mut usize,
) -> Result<()> {
    let mut at = 0usize;
    while at + 4 <= data.len() {
        check(token)?;
        let length = be_u32(data, at)? as usize;
        let start = at
            .checked_add(4)
            .ok_or(Error::Limit("ABR pattern offset"))?;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= data.len())
            .ok_or(Error::Invalid("truncated ABR pattern record"))?;
        let record = &data[start..end];
        let mut pos = 0usize;
        let mode = be_u32(record, pos + 4)?;
        skip(&mut pos, 12, end_record(record))?;
        let name_units = be_u32(record, pos)? as usize;
        if name_units > 1024 * 1024 {
            return Err(Error::Limit("ABR pattern name"));
        }
        pos += 4;
        skip(
            &mut pos,
            name_units
                .checked_mul(2)
                .ok_or(Error::Limit("ABR pattern name"))?,
            record.len(),
        )?;
        let id_len = *record
            .get(pos)
            .ok_or(Error::Invalid("ABR pattern identity"))? as usize;
        pos += 1;
        if id_len == 0
            || id_len > 128
            || !record
                .get(pos..pos + id_len)
                .is_some_and(|id| id.is_ascii() && !id.contains(&0))
        {
            return Err(Error::Invalid("ABR pattern identity"));
        }
        pos += id_len;
        if mode == 2 {
            skip(&mut pos, 772, record.len())?;
        }
        skip(&mut pos, 28, record.len())?;
        while pos + 4 <= record.len() {
            check(token)?;
            let written = be_u32(record, pos)?;
            pos += 4;
            if written == 0 {
                continue;
            }
            let array_len = be_u32(record, pos)? as usize;
            pos += 4;
            if array_len < 23 {
                return Err(Error::Invalid("ABR pattern channel length"));
            }
            if be_u32(record, pos)? != 8 {
                return Err(Error::Unsupported("ABR pattern channel depth".into()));
            }
            let body_end = pos
                .checked_add(array_len)
                .filter(|end| *end <= record.len())
                .ok_or(Error::Invalid("truncated ABR pattern channel"))?;
            let top = be_u32(record, pos + 4)?;
            let left = be_u32(record, pos + 8)?;
            let bottom = be_u32(record, pos + 12)?;
            let right = be_u32(record, pos + 16)?;
            if u16::from_be_bytes(
                record[pos + 20..pos + 22]
                    .try_into()
                    .map_err(|_| Error::Invalid("truncated ABR pattern channel"))?,
            ) != 8
            {
                return Err(Error::Unsupported("ABR pattern channel depth".into()));
            }
            let width = right
                .checked_sub(left)
                .ok_or(Error::Invalid("ABR pattern channel bounds"))?;
            let height = bottom
                .checked_sub(top)
                .ok_or(Error::Invalid("ABR pattern channel bounds"))?;
            if width == 0
                || height == 0
                || width > MAX_BRUSH_MASK_EDGE
                || height > MAX_BRUSH_MASK_EDGE
            {
                return Err(Error::Limit("ABR pattern channel dimensions"));
            }
            let count = (width as usize)
                .checked_mul(height as usize)
                .ok_or(Error::Limit("ABR pattern pixels"))?;
            *channel_budget = channel_budget
                .checked_add(count)
                .ok_or(Error::Limit("ABR pattern decode budget"))?;
            if *channel_budget > MAX_CATALOG_MASK_BYTES {
                return Err(Error::Limit("ABR pattern decode budget"));
            }
            pos = body_end;
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
    Ok(())
}

fn end_record(record: &[u8]) -> usize {
    record.len()
}

fn map_mode(id: Option<&str>) -> Option<String> {
    let mode = match id {
        None | Some("Nrml") | Some("normal") => "normal",
        Some("Mltp") | Some("multiply") => "multiply",
        Some("Drkn") | Some("darken") => "darken",
        Some("Lghn") | Some("lighten") => "lighten",
        Some("Sbtr") | Some("subtract") => "subtract",
        Some("Dfrn") | Some("difference") => "difference",
        Some("Scrn") | Some("screen") => "screen",
        Some("CBrn") | Some("colorBurn") => "colorBurn",
        Some("linearBurn") => "linearBurn",
        Some("CDdg") | Some("colorDodge") => "colorDodge",
        Some("linearDodge") => "linearDodge",
        Some("Ovrl") | Some("overlay") => "overlay",
        Some("SftL") | Some("softLight") => "softLight",
        Some("HrdL") | Some("hardLight") => "hardLight",
        Some("Xclu") | Some("exclusion") => "exclusion",
        Some("hardMix") => "hardMix",
        _ => return None,
    };
    Some(mode.into())
}

pub fn decode(
    bytes: &[u8],
    sections: &[crate::abr_sampled::OpaqueSection],
    tip_resident: usize,
    token: &CancellationToken,
) -> Result<DecodedTextures> {
    check(token)?;
    if !sections.iter().any(|section| &section.key == b"patt") {
        return Ok(DecodedTextures {
            patterns: BTreeMap::new(),
            presets: BTreeMap::new(),
        });
    }
    let pack = brushkit_abr::parse_abr_deferred_without_patterns(bytes)
        .map_err(|_| Error::Invalid("ABR texture parsing"))?
        .pack;
    let mut presets = BTreeMap::new();
    let mut wanted = BTreeSet::new();
    for brush in pack.brushes {
        check(token)?;
        if let Some(index) = brush.preset_index {
            if let Some(pattern_id) = brush.descriptor.texture_pattern_id {
                let depth = brush.descriptor.texture_depth_pct.unwrap_or(0.);
                let brightness = brush.descriptor.texture_brightness.unwrap_or(0) as f32 / 100.;
                let contrast = brush.descriptor.texture_contrast.unwrap_or(0) as f32 / 100.;
                let minimum_depth =
                    (brush.descriptor.texture_minimum_depth_pct.unwrap_or(0.) / 100.) as f32;
                let depth_jitter =
                    (brush.descriptor.texture_depth_jitter_pct.unwrap_or(0.) / 100.) as f32;
                let scale = brush.descriptor.texture_scale_pct.unwrap_or(100.);
                if !depth.is_finite()
                    || !(0. ..=100.).contains(&depth)
                    || !brightness.is_finite()
                    || !(-1. ..=1.).contains(&brightness)
                    || !contrast.is_finite()
                    || !(-1. ..=1.).contains(&contrast)
                    || !minimum_depth.is_finite()
                    || !(0. ..=1.).contains(&minimum_depth)
                    || !depth_jitter.is_finite()
                    || !(0. ..=1.).contains(&depth_jitter)
                    || !scale.is_finite()
                    || !(1. ..=10000.).contains(&scale)
                {
                    return Err(Error::Invalid("ABR texture parameters"));
                }
                wanted.insert(pattern_id.clone());
                presets.insert(
                    index,
                    TextureSource {
                        pattern_id,
                        depth: (depth / 100.) as f32,
                        brightness,
                        contrast,
                        minimum_depth,
                        depth_jitter,
                        scale: (scale / 100.) as f32,
                        invert: brush.descriptor.texture_invert.unwrap_or(false),
                        each_tip: brush.descriptor.texture_each_tip.unwrap_or(true),
                        mode: map_mode(brush.descriptor.texture_blend_mode.as_deref()),
                    },
                );
            }
        }
    }
    for preset in pack.computed_presets {
        check(token)?;
        if let (Some(index), Some(pattern_id)) =
            (preset.preset_index, preset.descriptor.texture_pattern_id)
        {
            let depth = preset.descriptor.texture_depth_pct.unwrap_or(0.);
            let brightness = preset.descriptor.texture_brightness.unwrap_or(0) as f32 / 100.;
            let contrast = preset.descriptor.texture_contrast.unwrap_or(0) as f32 / 100.;
            let minimum_depth =
                (preset.descriptor.texture_minimum_depth_pct.unwrap_or(0.) / 100.) as f32;
            let depth_jitter =
                (preset.descriptor.texture_depth_jitter_pct.unwrap_or(0.) / 100.) as f32;
            let scale = preset.descriptor.texture_scale_pct.unwrap_or(100.);
            if !depth.is_finite()
                || !(0. ..=100.).contains(&depth)
                || !brightness.is_finite()
                || !(-1. ..=1.).contains(&brightness)
                || !contrast.is_finite()
                || !(-1. ..=1.).contains(&contrast)
                || !minimum_depth.is_finite()
                || !(0. ..=1.).contains(&minimum_depth)
                || !depth_jitter.is_finite()
                || !(0. ..=1.).contains(&depth_jitter)
                || !scale.is_finite()
                || !(1. ..=10000.).contains(&scale)
            {
                return Err(Error::Invalid("ABR texture parameters"));
            }
            wanted.insert(pattern_id.clone());
            presets.insert(
                index,
                TextureSource {
                    pattern_id,
                    depth: (depth / 100.) as f32,
                    brightness,
                    contrast,
                    minimum_depth,
                    depth_jitter,
                    scale: (scale / 100.) as f32,
                    invert: preset.descriptor.texture_invert.unwrap_or(false),
                    each_tip: preset.descriptor.texture_each_tip.unwrap_or(true),
                    mode: map_mode(preset.descriptor.texture_blend_mode.as_deref()),
                },
            );
        }
    }
    let mut resident = tip_resident;
    let decoded = crate::abr_pattern_decode::decode(sections, token, &mut resident, &wanted)?;
    let mut patterns = BTreeMap::new();
    for pattern in decoded {
        check(token)?;
        if pattern.gray.len() != (pattern.width as usize).saturating_mul(pattern.height as usize) {
            return Err(Error::Invalid("ABR texture pixels"));
        }
        let mask = BrushMask::new(pattern.width, pattern.height, pattern.gray)?;
        let id = hash(&encode_mask(&mask)?);
        let mask = Arc::new(mask.with_resource_id(id.clone())?);
        resident = resident
            .checked_add(mask.resident_bytes())
            .ok_or(Error::Limit("ABR texture resource budget"))?;
        if resident > MAX_CATALOG_MASK_BYTES {
            return Err(Error::Limit("ABR texture resource budget"));
        }
        let id = pattern.id;
        if patterns
            .insert(id.clone(), PatternResource { id, mask })
            .is_some()
        {
            return Err(Error::Invalid("duplicate ABR texture identity"));
        }
    }
    if wanted.iter().any(|id| !patterns.contains_key(id)) {
        return Err(Error::Invalid("ABR texture pattern reference"));
    }
    Ok(DecodedTextures { patterns, presets })
}
