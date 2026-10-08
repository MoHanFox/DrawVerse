use crate::{
    check, dimensions,
    raster::{self, Placement},
    Error, Result, MAX_PIXELS,
};
use paint_core::{Document, DocumentOptions, DocumentSnapshot, LayerProperties};
use paint_render::{CpuRenderer, FramePixels, PixelFormat, Region, RenderRequest, Renderer};
use paint_task::CancellationToken;
use quick_xml::{events::Event, Reader};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read, Seek, Write},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

fn safe_path(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 1024
        && !name.contains(['\\', ':', '\0'])
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
fn entry(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str, max: u64) -> Result<Vec<u8>> {
    if !safe_path(name) {
        return Err(Error::Invalid("unsafe archive path"));
    }
    let mut file = archive.by_name(name)?;
    if file.size() > max {
        return Err(Error::Limit("archive entry bytes"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(max + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(Error::Limit("archive entry bytes"));
    }
    Ok(bytes)
}
fn attributes(event: &quick_xml::events::BytesStart<'_>) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for a in event.attributes() {
        let a = a.map_err(|e| Error::Codec(e.to_string()))?;
        let name = std::str::from_utf8(a.key.as_ref())
            .map_err(|_| Error::Invalid("XML attribute UTF-8"))?
            .to_owned();
        let value = a
            .unescape_value()
            .map_err(|e| Error::Codec(e.to_string()))?
            .into_owned();
        if result.insert(name, value).is_some() {
            return Err(Error::Invalid("duplicate XML attribute"));
        }
    }
    Ok(result)
}
fn number<T: std::str::FromStr>(
    attrs: &mut BTreeMap<String, String>,
    name: &str,
    default: Option<T>,
) -> Result<T> {
    match attrs.remove(name) {
        Some(v) => v
            .parse()
            .map_err(|_| Error::Invalid("invalid XML numeric attribute")),
        None => default.ok_or(Error::Invalid("missing XML numeric attribute")),
    }
}
struct LayerSpec {
    parent: Option<usize>,
    group: bool,
    mask: bool,
    clipped: bool,
    white: (u32, u32),
    explicit_id: u64,
    explicit_parent: u64,
    placement: Placement,
    src: String,
    selected: bool,
    appearance: Option<paint_core::LayerAppearance>,
    local: Option<(i32, i32)>,
}
fn parse_node(
    e: &quick_xml::events::BytesStart<'_>,
    width: u32,
    height: u32,
    owned: bool,
    group: bool,
    index: usize,
    parent: Option<usize>,
) -> Result<LayerSpec> {
    let mut a = attributes(e)?;
    if group && a.remove("isolation").as_deref().unwrap_or("isolate") != "isolate" {
        return Err(Error::Unsupported("non-isolated OpenRaster group".into()));
    }
    let src = if group {
        String::new()
    } else {
        a.remove("src")
            .ok_or(Error::Invalid("missing layer source"))?
    };
    if !group && !safe_path(&src) {
        return Err(Error::Invalid("unsafe layer path"));
    }
    let name = a
        .remove("name")
        .unwrap_or_else(|| format!("Layer {}", index + 1));
    let x = number(&mut a, "x", Some(0i32))?;
    let y = number(&mut a, "y", Some(0i32))?;
    if group && (x != 0 || y != 0) {
        return Err(Error::Unsupported("nonzero group position".into()));
    }
    let mut opacity = number(&mut a, "opacity", Some(1f32))?;
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err(Error::Invalid("layer opacity"));
    }
    let visible = match a.remove("visibility").as_deref().unwrap_or("visible") {
        "visible" => true,
        "hidden" => false,
        _ => return Err(Error::Invalid("layer visibility")),
    };
    let standard = parse_blend(
        a.remove("composite-op")
            .as_deref()
            .unwrap_or("svg:src-over"),
    )?;
    let (appearance, local) = if owned {
        let fill = number(&mut a, "dv:fill", Some(1f32))?;
        let blend =
            paint_core::BlendMode::from_id(number(&mut a, "dv:blend", Some(standard as u32))?)?;
        let locks = number(&mut a, "dv:locks", Some(0u32))?;
        let offset_x = number(&mut a, "dv:offset-x", Some(0i32))?;
        let offset_y = number(&mut a, "dv:offset-y", Some(0i32))?;
        let lx = number(&mut a, "dv:local-x", Some(x))?;
        let ly = number(&mut a, "dv:local-y", Some(y))?;
        let dissolve_seed = number(&mut a, "dv:seed", Some(0u32))?;
        opacity = number(&mut a, "dv:opacity", Some(opacity))?;
        let options = paint_core::LayerAppearance {
            fill,
            blend,
            locks,
            offset_x,
            offset_y,
            dissolve_seed,
        };
        if standard != parse_blend(standard_blend(blend))? {
            return Err(Error::Invalid("conflicting blend extension"));
        }
        options.validate()?;
        if lx.checked_add(offset_x) != Some(x)
            || ly.checked_add(offset_y) != Some(y)
            || !opacity.is_finite()
            || !(0. ..=1.).contains(&opacity)
        {
            return Err(Error::Invalid("layer extension position/opacity"));
        }
        (Some(options), Some((lx, ly)))
    } else {
        (
            if standard == paint_core::BlendMode::Normal {
                None
            } else {
                Some(paint_core::LayerAppearance {
                    blend: standard,
                    ..Default::default()
                })
            },
            None,
        )
    };
    let clipped = if owned {
        number::<u32>(&mut a, "dv:clipped", Some(0))?
    } else {
        0
    };
    if clipped > 1 {
        return Err(Error::Invalid("invalid clipping flag"));
    }
    let (mask, white, explicit_id, explicit_parent) = if owned {
        let mask = number::<u32>(&mut a, "dv:mask", Some(0))?;
        let white = (
            number(&mut a, "dv:white-width", Some(0u32))?,
            number(&mut a, "dv:white-height", Some(0u32))?,
        );
        if mask > 1 || (group && mask != 0) {
            return Err(Error::Invalid("invalid mask kind"));
        }
        (
            mask == 1,
            white,
            number(&mut a, "dv:id", Some(0u64))?,
            number(&mut a, "dv:parent", Some(0u64))?,
        )
    } else {
        (false, (0, 0), 0, 0)
    };
    let selected = match a.remove("selected").as_deref().unwrap_or("false") {
        "true" => true,
        "false" => false,
        _ => return Err(Error::Invalid("layer selected flag")),
    };
    if !a.is_empty() {
        return Err(Error::Unsupported("layer metadata".into()));
    }
    Ok(LayerSpec {
        parent,
        group,
        mask,
        clipped: clipped == 1,
        white,
        explicit_id,
        explicit_parent,
        placement: Placement {
            name,
            properties: LayerProperties { visible, opacity },
            x,
            y,
            width,
            height,
        },
        src,
        selected,
        appearance,
        local,
    })
}
fn parse_stack(xml: &[u8]) -> Result<(u32, u32, Vec<LayerSpec>, bool)> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut width = 0;
    let mut height = 0;
    let mut owned = false;
    let mut extension_version = 0;
    let mut editable = false;
    let mut image_seen = false;
    let mut image_open = false;
    let mut stack_seen = false;
    let mut stacks = Vec::<Option<usize>>::new();
    let mut paired_layer = false;
    let mut layers = Vec::new();
    loop {
        let event = reader
            .read_event()
            .map_err(|e| Error::Codec(e.to_string()))?;
        match event {
            Event::Start(ref e) if e.name().as_ref() == b"image" && !image_seen => {
                image_seen = true;
                image_open = true;
                let mut a = attributes(e)?;
                width = number(&mut a, "w", None)?;
                height = number(&mut a, "h", None)?;
                dimensions(width, height)?;
                let marker = number::<u32>(&mut a, "dv:editable", Some(0))?;
                if marker != 0 && marker != 2 {
                    return Err(Error::Unsupported("editable extension version".into()));
                }
                editable = marker == 2;
                if let Some(ns) = a.remove("xmlns:dv") {
                    extension_version = number::<u32>(&mut a, "dv:version", Some(1))?;
                    if ns != "urn:drawverse:layers:1" || !matches!(extension_version, 1..=3) {
                        return Err(Error::Unsupported(
                            "DrawVerse layer extension version".into(),
                        ));
                    }
                    owned = true;
                }
                a.remove("version");
                a.remove("name");
                let xr: u32 = number(&mut a, "xres", Some(72))?;
                let yr: u32 = number(&mut a, "yres", Some(72))?;
                if xr != 72 || yr != 72 {
                    return Err(Error::Unsupported("document DPI metadata".into()));
                }
                if !a.is_empty() {
                    return Err(Error::Unsupported("image metadata".into()));
                }
            }
            Event::Start(ref e)
                if e.name().as_ref() == b"stack"
                    && image_open
                    && !paired_layer
                    && stacks.is_empty()
                    && !stack_seen =>
            {
                if !attributes(e)?.is_empty() {
                    return Err(Error::Unsupported("root stack properties".into()));
                }
                stack_seen = true;
                stacks.push(None);
            }
            Event::Start(ref e) | Event::Empty(ref e)
                if image_open
                    && !paired_layer
                    && !stacks.is_empty()
                    && matches!(e.name().as_ref(), b"stack" | b"layer") =>
            {
                if layers.len() >= 256 {
                    return Err(Error::Limit("more than 256 OpenRaster nodes"));
                }
                let group = e.name().as_ref() == b"stack";
                let paired = matches!(event, Event::Start(_));
                if group && stacks.len() > 16 {
                    return Err(Error::Limit("group nesting exceeds 16"));
                }
                let index = layers.len();
                let parent = *stacks.last().expect("open root stack");
                layers.push(parse_node(e, width, height, owned, group, index, parent)?);
                if paired {
                    if group {
                        stacks.push(Some(index));
                    } else {
                        paired_layer = true;
                    }
                }
            }
            Event::End(ref e) if e.name().as_ref() == b"layer" && paired_layer => {
                paired_layer = false;
            }
            Event::End(ref e)
                if e.name().as_ref() == b"stack" && !paired_layer && !stacks.is_empty() =>
            {
                stacks.pop();
            }
            Event::End(ref e)
                if e.name().as_ref() == b"image"
                    && image_open
                    && !paired_layer
                    && stacks.is_empty()
                    && stack_seen =>
            {
                image_open = false;
            }
            Event::Decl(_) | Event::Comment(_) => {}
            Event::Text(ref e) if e.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Event::Eof => break,
            _ => {
                return Err(Error::Unsupported(
                    "OpenRaster XML entities or unsupported content".into(),
                ))
            }
        }
    }
    if image_open
        || !image_seen
        || !stack_seen
        || !stacks.is_empty()
        || paired_layer
        || layers.is_empty()
    {
        return Err(Error::Invalid("incomplete layer stack"));
    }
    if layers.iter().filter(|l| l.selected).count() > 1 {
        return Err(Error::Invalid("multiple selected layers"));
    }
    if editable && !owned {
        return Err(Error::Invalid("editable marker namespace"));
    }
    if extension_version >= 2 && layers.iter().any(|l| l.explicit_id == 0) {
        return Err(Error::Invalid("version 2 requires explicit IDs"));
    }
    if extension_version < 3 && layers.iter().any(|l| l.clipped) {
        return Err(Error::Invalid("clipping requires extension version 3"));
    }
    if extension_version < 2
        && layers
            .iter()
            .any(|l| l.mask || l.white != (0, 0) || l.explicit_id != 0 || l.explicit_parent != 0)
    {
        return Err(Error::Invalid("new metadata requires extension version 2"));
    }
    if layers.iter().any(|l| l.explicit_id != 0) {
        let ids: BTreeMap<_, _> = layers
            .iter()
            .enumerate()
            .map(|(i, l)| (l.explicit_id, i))
            .collect();
        if ids.len() != layers.len() || ids.contains_key(&0) {
            return Err(Error::Invalid("duplicate/missing extension IDs"));
        }
        for layer in &mut layers {
            layer.parent = if layer.explicit_parent == 0 {
                None
            } else {
                Some(
                    *ids.get(&layer.explicit_parent)
                        .ok_or(Error::Invalid("missing extension parent"))?,
                )
            };
        }
    } else if layers
        .iter()
        .any(|l| l.mask || l.white != (0, 0) || l.explicit_parent != 0)
    {
        return Err(Error::Invalid("mask extension requires explicit IDs"));
    }
    Ok((width, height, layers, editable))
}
pub(crate) fn decode(
    bytes: &[u8],
    token: &CancellationToken,
    options: DocumentOptions,
    storage: std::sync::Arc<paint_storage::ScratchSpace>,
) -> Result<Document> {
    // zip's filename index collapses duplicate entries. Inspect the original
    // single-disk ZIP32 entry count as well; ZIP64 is unnecessary within our bounds.
    let footer = bytes
        .windows(4)
        .enumerate()
        .rev()
        .find_map(|(i, p)| {
            if p == b"PK\x05\x06" && i + 22 <= bytes.len() {
                let comment = u16::from_le_bytes([bytes[i + 20], bytes[i + 21]]) as usize;
                (i + 22 + comment == bytes.len()).then_some(i)
            } else {
                None
            }
        })
        .ok_or(Error::Invalid("missing ZIP footer"))?;
    let count = u16::from_le_bytes([bytes[footer + 10], bytes[footer + 11]]) as usize;
    if bytes[footer + 4..footer + 8] != [0; 4]
        || bytes[footer + 8..footer + 10] != bytes[footer + 10..footer + 12]
        || count == 65535
    {
        return Err(Error::Unsupported("multi-disk or ZIP64 archive".into()));
    }
    if count > 1024 {
        return Err(Error::Limit("archive entry count"));
    }
    if bytes.len() < 38
        || &bytes[30..38] != b"mimetype"
        || bytes[8..10] != [0; 2]
        || bytes[26..28] != [8, 0]
    {
        return Err(Error::Invalid("first ZIP entry must be stored mimetype"));
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes))?;
    if zip.len() != count {
        return Err(Error::Invalid("duplicate archive entry"));
    }
    if zip.len() > 1024 {
        return Err(Error::Limit("archive entry count"));
    }
    let mut names = BTreeSet::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        check(token)?;
        let f = zip.by_index(i)?;
        if !safe_path(f.name().trim_end_matches('/')) || !names.insert(f.name().to_owned()) {
            return Err(Error::Invalid("unsafe/duplicate archive entry"));
        }
        if !matches!(
            f.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(Error::Unsupported("ZIP compression".into()));
        }
        total = total
            .checked_add(f.size())
            .ok_or(Error::Limit("archive size overflow"))?;
        if total > 512 * 1024 * 1024 {
            return Err(Error::Limit("archive exceeds 512 MiB uncompressed"));
        }
    }
    {
        let first = zip.by_index(0)?;
        if first.name() != "mimetype" || first.compression() != CompressionMethod::Stored {
            return Err(Error::Invalid(
                "OpenRaster mimetype must be first and stored",
            ));
        }
    }
    if entry(&mut zip, "mimetype", 64)? != b"image/openraster" {
        return Err(Error::Invalid("OpenRaster mimetype"));
    }
    if !names.contains("mergedimage.png") || !names.contains("Thumbnails/thumbnail.png") {
        return Err(Error::Invalid("missing merged image/thumbnail"));
    }
    let standard = entry(&mut zip, "stack.xml", 1024 * 1024)?;
    let (sw, sh, standard_specs, use_manifest) = parse_stack(&standard)?;
    let (w, h, specs) = if use_manifest {
        let (w, h, specs, nested) =
            parse_stack(&entry(&mut zip, "drawverse/stack.xml", 1024 * 1024)?)?;
        if nested || (sw, sh) != (w, h) {
            return Err(Error::Invalid("manifest/fallback conflict"));
        }
        (w, h, specs)
    } else {
        (sw, sh, standard_specs)
    };
    let active = specs.iter().position(|l| l.selected).unwrap_or(0);
    let mut layers = Vec::new();
    let mut appearances = Vec::new();
    let mut has_extension = false;
    let mut pixels = 0u64;
    let mut tiles = 0;
    let node_count = specs.len();
    let mut clipping: Vec<_> = specs.iter().map(|s| s.clipped).collect();
    clipping.reverse();
    let mut hierarchy: Vec<_> = specs
        .iter()
        .map(|s| {
            (
                s.parent.map_or(0, |i| (node_count - i) as u64),
                s.group,
                s.mask,
                s.white,
            )
        })
        .collect();
    hierarchy.reverse();
    for spec in specs {
        if spec.group {
            has_extension |= spec.appearance.is_some();
            appearances.push(spec.appearance.unwrap_or_default());
            layers.push(paint_core::ImportedLayer {
                name: spec.placement.name,
                properties: spec.placement.properties,
                tiles: BTreeMap::new(),
            });
            continue;
        }
        check(token)?;
        let bytes = entry(&mut zip, &spec.src, crate::MAX_INPUT)?;
        if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(Error::Invalid("OpenRaster layer must be PNG"));
        }
        let image = raster::decode_image(&bytes, token)?;
        pixels += u64::from(image.width()) * u64::from(image.height());
        if pixels > MAX_PIXELS {
            return Err(Error::Limit("total layer raster pixels"));
        }
        has_extension |= spec.appearance.is_some();
        appearances.push(spec.appearance.unwrap_or_default());
        layers.push(raster::stage_local_with_white(
            &image,
            spec.placement,
            &mut tiles,
            token,
            spec.local,
            spec.white,
        )?);
    }
    let active = layers.len() - 1 - active;
    layers.reverse();
    appearances.reverse();
    let mut document = Document::from_import_with_appearance(
        w,
        h,
        layers,
        active,
        options,
        storage,
        if has_extension {
            Some(appearances)
        } else {
            None
        },
    )?;
    document.set_import_nodes(&hierarchy)?;
    document.set_import_clipping(&clipping)?;
    Ok(document)
}
fn escaped(s: &str) -> Result<String> {
    if s.chars()
        .any(|c| c < ' ' && !matches!(c, '\t' | '\n' | '\r'))
    {
        return Err(Error::Unsupported(
            "XML control character in layer name".into(),
        ));
    }
    Ok(s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;")
        .replace('\t', "&#9;"))
}
pub(crate) fn encode(
    writer: impl Write + Seek,
    snapshot: &DocumentSnapshot,
    active: u64,
    token: &CancellationToken,
) -> Result<()> {
    if snapshot.layers().len() > 256 {
        return Err(Error::Limit("more than 256 OpenRaster layers"));
    }
    let mut zip = ZipWriter::new(writer);
    zip.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )?;
    zip.write_all(b"image/openraster")?;
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut xml=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><image w=\"{}\" h=\"{}\" version=\"0.0.6\" xmlns:dv=\"urn:drawverse:layers:1\" dv:version=\"3\"><stack>",snapshot.width,snapshot.height);
    let mut pixels = 0u64;
    let mut open_groups = Vec::new();
    for (index, layer) in snapshot.layers().iter().rev().enumerate() {
        check(token)?;
        let xml_parent = if layer.is_mask() {
            let owner = snapshot
                .layers()
                .iter()
                .find(|l| l.id() == layer.parent_id())
                .ok_or(Error::Invalid("mask owner"))?;
            if owner.is_group() {
                owner.id()
            } else {
                owner.parent_id()
            }
        } else {
            layer.parent_id()
        };
        while open_groups.last().copied().unwrap_or(0) != xml_parent {
            if open_groups.pop().is_none() {
                return Err(Error::Invalid("invalid group order"));
            }
            xml.push_str("</stack>");
        }
        if layer.is_group() {
            let p = layer.properties();
            let a = layer.appearance();
            let extension = format!(
                " dv:id=\"{}\" dv:parent=\"{}\" dv:clipped=\"{}\"",
                layer.id(),
                layer.parent_id(),
                u32::from(layer.is_clipped())
            );
            xml.push_str(&format!("<stack{extension} name=\"{}\" opacity=\"{}\" visibility=\"{}\" composite-op=\"{}\" isolation=\"isolate\" selected=\"{}\" dv:fill=\"{}\" dv:blend=\"{}\" dv:locks=\"{}\" dv:opacity=\"{}\" dv:seed=\"{}\">",escaped(layer.name())?,p.opacity*a.fill,if p.visible {"visible"} else {"hidden"},standard_blend(a.blend),layer.id()==active,a.fill,a.blend as u32,a.locks,p.opacity,a.dissolve_seed));
            open_groups.push(layer.id());
            continue;
        }
        let mut bounds: Option<(i64, i64, i64, i64)> = None;
        for (coord, tile) in layer.tiles() {
            check(token)?;
            if layer.white_extent() != (0, 0) {
                let x = i64::from(coord.signed_x()) * 64;
                let y = i64::from(coord.signed_y()) * 64;
                bounds = Some(bounds.map_or((x, y, x + 63, y + 63), |(a, b, c, d)| {
                    (a.min(x), b.min(y), c.max(x + 63), d.max(y + 63))
                }));
                continue;
            }
            let tile_pixels = tile.try_pixels()?;
            for y in 0..64 {
                for x in 0..64 {
                    if tile_pixels[(y * 64 + x) as usize].alpha() > 0. {
                        let x = i64::from(coord.signed_x()) * 64 + i64::from(x);
                        let y = i64::from(coord.signed_y()) * 64 + i64::from(y);
                        bounds = Some(bounds.map_or((x, y, x, y), |(a, b, c, d)| {
                            (a.min(x), b.min(y), c.max(x), d.max(y))
                        }));
                    }
                }
            }
        }
        let (x, y, max_x, max_y) = bounds.unwrap_or((0, 0, 0, 0));
        let w = (max_x - x + 1) as u32;
        let h = (max_y - y + 1) as u32;
        dimensions(w, h)?;
        pixels += u64::from(w) * u64::from(h);
        if pixels > MAX_PIXELS {
            return Err(Error::Limit("total layer raster pixels"));
        }
        let mut rgba = Vec::with_capacity((u64::from(w) * u64::from(h) * 4) as usize);
        for py in y..=max_y {
            check(token)?;
            for pixel in layer.read_local_row(x, py, w)? {
                rgba.extend(raster::straight8(pixel.components()));
            }
        }
        let path = format!("data/layer{index}.png");
        zip.start_file(&path, options)?;
        raster::png(&mut zip, &rgba, w, h)?;
        let props = layer.properties();
        let a = layer.appearance();
        let extension=format!(" dv:id=\"{}\" dv:parent=\"{}\" dv:mask=\"{}\" dv:white-width=\"{}\" dv:white-height=\"{}\" dv:clipped=\"{}\"",layer.id(),layer.parent_id(),u32::from(layer.is_mask()),layer.white_extent().0,layer.white_extent().1,u32::from(layer.is_clipped()));
        xml.push_str(&format!("<layer{extension} name=\"{}\" src=\"{path}\" x=\"{}\" y=\"{}\" opacity=\"{}\" visibility=\"{}\" composite-op=\"{}\" selected=\"{}\" dv:fill=\"{}\" dv:blend=\"{}\" dv:locks=\"{}\" dv:offset-x=\"{}\" dv:offset-y=\"{}\" dv:local-x=\"{x}\" dv:local-y=\"{y}\" dv:opacity=\"{}\" dv:seed=\"{}\"/>",escaped(layer.name())?,x+i64::from(a.offset_x),y+i64::from(a.offset_y),props.opacity*a.fill,if props.visible {"visible"}else{"hidden"},standard_blend(a.blend),layer.id()==active,a.fill,a.blend as u32,a.locks,a.offset_x,a.offset_y,props.opacity,a.dissolve_seed));
    }
    for _ in open_groups {
        xml.push_str("</stack>");
    }
    xml.push_str("</stack></image>");
    if xml.len() > 1024 * 1024 {
        return Err(Error::Limit("stack XML bytes"));
    }
    let extended = snapshot
        .layers()
        .iter()
        .any(|l| l.is_mask() || l.is_clipped() || l.white_extent() != (0, 0));
    zip.start_file(
        if extended {
            "drawverse/stack.xml"
        } else {
            "stack.xml"
        },
        options,
    )?;
    zip.write_all(xml.as_bytes())?;
    if extended {
        let fallback=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><image w=\"{}\" h=\"{}\" version=\"0.0.6\" xmlns:dv=\"urn:drawverse:layers:1\" dv:editable=\"2\"><stack><layer name=\"DrawVerse 合成预览\" src=\"mergedimage.png\"/></stack></image>",snapshot.width,snapshot.height);
        zip.start_file("stack.xml", options)?;
        zip.write_all(fallback.as_bytes())?;
    }
    zip.start_file("mergedimage.png", options)?;
    raster::png(
        &mut zip,
        &raster::merged(snapshot, token)?,
        snapshot.width,
        snapshot.height,
    )?;
    let scale = (256f64 / f64::from(snapshot.width.max(snapshot.height))).min(1.);
    let frame = CpuRenderer
        .render(
            snapshot,
            RenderRequest {
                region: Region {
                    x: 0.,
                    y: 0.,
                    width: f64::from(snapshot.width),
                    height: f64::from(snapshot.height),
                },
                width: (f64::from(snapshot.width) * scale).round().max(1.) as u32,
                height: (f64::from(snapshot.height) * scale).round().max(1.) as u32,
                format: PixelFormat::LinearRgba32F,
            },
            token,
        )
        .map_err(|e| match e {
            paint_render::RenderError::Cancelled => Error::Cancelled,
            _ => Error::Limit("thumbnail render"),
        })?;
    let FramePixels::Linear(pixels) = frame.pixels else {
        unreachable!()
    };
    let bytes: Vec<u8> = pixels.into_iter().flat_map(raster::straight8).collect();
    zip.start_file("Thumbnails/thumbnail.png", options)?;
    raster::png(&mut zip, &bytes, frame.width, frame.height)?;
    check(token)?;
    zip.finish()?;
    Ok(())
}

fn standard_blend(mode: paint_core::BlendMode) -> &'static str {
    use paint_core::BlendMode::*;
    match mode {
        Multiply => "svg:multiply",
        Screen => "svg:screen",
        Overlay => "svg:overlay",
        Darken => "svg:darken",
        Lighten => "svg:lighten",
        ColorDodge => "svg:color-dodge",
        ColorBurn => "svg:color-burn",
        HardLight => "svg:hard-light",
        SoftLight => "svg:soft-light",
        Difference => "svg:difference",
        Color => "svg:color",
        Luminosity => "svg:luminosity",
        Hue => "svg:hue",
        Saturation => "svg:saturation",
        _ => "svg:src-over",
    }
}
fn parse_blend(name: &str) -> Result<paint_core::BlendMode> {
    use paint_core::BlendMode::*;
    Ok(match name {
        "svg:src-over" => Normal,
        "svg:multiply" => Multiply,
        "svg:screen" => Screen,
        "svg:overlay" => Overlay,
        "svg:darken" => Darken,
        "svg:lighten" => Lighten,
        "svg:color-dodge" => ColorDodge,
        "svg:color-burn" => ColorBurn,
        "svg:hard-light" => HardLight,
        "svg:soft-light" => SoftLight,
        "svg:difference" => Difference,
        "svg:color" => Color,
        "svg:luminosity" => Luminosity,
        "svg:hue" => Hue,
        "svg:saturation" => Saturation,
        _ => {
            return Err(Error::Unsupported(
                "layer blend/compositing operator".into(),
            ))
        }
    })
}
