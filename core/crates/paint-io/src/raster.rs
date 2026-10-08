use crate::{check, dimensions, Error, ExportOptions, Format, Result};
use image::{
    DynamicImage, ExtendedColorType, GenericImageView, ImageDecoder, ImageEncoder, ImageFormat,
    ImageReader, Limits,
};
use paint_core::{
    Document, DocumentOptions, DocumentSnapshot, ImportedLayer, LayerProperties, Pixel, Tile,
    TileCoord,
};
use paint_task::CancellationToken;
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
};

fn linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn encoded(v: f32) -> u8 {
    let v = v.clamp(0., 1.);
    let v = if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    };
    (v * 255.).round() as u8
}
pub(crate) fn straight8(p: [f32; 4]) -> [u8; 4] {
    if p[3] <= 0. {
        return [0; 4];
    }
    [
        encoded(p[0] / p[3]),
        encoded(p[1] / p[3]),
        encoded(p[2] / p[3]),
        (p[3] * 255.).round() as u8,
    ]
}

pub(crate) fn decode_image(bytes: &[u8], token: &CancellationToken) -> Result<DynamicImage> {
    check(token)?;
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let format = reader
        .format()
        .ok_or(Error::Invalid("unknown file signature"))?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) {
        return Err(Error::Unsupported("image format".into()));
    }
    // Animated files cannot be imported as a silently flattened first frame.
    if format == ImageFormat::WebP
        && image::codecs::webp::WebPDecoder::new(Cursor::new(bytes))?.has_animation()
    {
        return Err(Error::Unsupported("animated WebP".into()));
    }
    if format == ImageFormat::Png {
        let mut offset = 8usize;
        let mut gamma = None;
        let mut chromaticities = None;
        let mut srgb = false;
        while offset.checked_add(12).is_some_and(|end| end <= bytes.len()) {
            let count = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            if &bytes[offset + 4..offset + 8] == b"acTL" {
                return Err(Error::Unsupported("animated PNG".into()));
            }
            if count == 4 && offset + 16 <= bytes.len() && &bytes[offset + 4..offset + 8] == b"gAMA"
            {
                gamma = Some(u32::from_be_bytes(
                    bytes[offset + 8..offset + 12].try_into().unwrap(),
                ));
            }
            if count == 32
                && offset + 44 <= bytes.len()
                && &bytes[offset + 4..offset + 8] == b"cHRM"
            {
                chromaticities = Some(
                    bytes[offset + 8..offset + 40]
                        .chunks_exact(4)
                        .map(|v| u32::from_be_bytes(v.try_into().unwrap()))
                        .collect::<Vec<_>>(),
                );
            }
            if &bytes[offset + 4..offset + 8] == b"sRGB" {
                srgb = true;
            }
            offset = offset
                .checked_add(count)
                .and_then(|v| v.checked_add(12))
                .ok_or(Error::Invalid("PNG chunk length"))?;
        }
        if !srgb
            && (gamma.is_some_and(|g| g != 45455)
                || chromaticities
                    .is_some_and(|c| c != [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000]))
        {
            return Err(Error::Unsupported(
                "non-sRGB PNG gamma/chromaticities require color management".into(),
            ));
        }
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(crate::MAX_EDGE);
    limits.max_image_height = Some(crate::MAX_EDGE);
    limits.max_alloc = Some(1024 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder()?;
    let (w, h) = decoder.dimensions();
    dimensions(w, h)?;
    if decoder.icc_profile()?.is_some() {
        return Err(Error::Unsupported(
            "embedded ICC requires the color-management module".into(),
        ));
    }
    let orientation = decoder.orientation()?;
    check(token)?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    check(token)?;
    Ok(image)
}
fn rgba(image: &DynamicImage, x: u32, y: u32) -> [f32; 4] {
    let p = match image {
        DynamicImage::ImageRgba16(i) => i.get_pixel(x, y).0.map(|v| f32::from(v) / 65535.),
        DynamicImage::ImageRgb16(i) => {
            let p = i.get_pixel(x, y).0;
            [
                f32::from(p[0]) / 65535.,
                f32::from(p[1]) / 65535.,
                f32::from(p[2]) / 65535.,
                1.,
            ]
        }
        DynamicImage::ImageLuma16(i) => {
            let v = f32::from(i.get_pixel(x, y).0[0]) / 65535.;
            [v, v, v, 1.]
        }
        DynamicImage::ImageLumaA16(i) => {
            let p = i.get_pixel(x, y).0;
            let v = f32::from(p[0]) / 65535.;
            [v, v, v, f32::from(p[1]) / 65535.]
        }
        _ => image.get_pixel(x, y).0.map(|v| f32::from(v) / 255.),
    };
    [linear(p[0]), linear(p[1]), linear(p[2]), p[3]]
}
pub(crate) struct Placement {
    pub name: String,
    pub properties: LayerProperties,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
pub(crate) fn stage(
    image: &DynamicImage,
    placement: Placement,
    total: &mut usize,
    token: &CancellationToken,
) -> Result<ImportedLayer> {
    stage_local(image, placement, total, token, None)
}
pub(crate) fn stage_local(
    image: &DynamicImage,
    placement: Placement,
    total: &mut usize,
    token: &CancellationToken,
    local: Option<(i32, i32)>,
) -> Result<ImportedLayer> {
    stage_local_with_white(image, placement, total, token, local, (0, 0))
}
pub(crate) fn stage_local_with_white(
    image: &DynamicImage,
    placement: Placement,
    total: &mut usize,
    token: &CancellationToken,
    local: Option<(i32, i32)>,
    white: (u32, u32),
) -> Result<ImportedLayer> {
    let mut tiles: BTreeMap<TileCoord, Vec<Pixel>> = BTreeMap::new();
    for y in 0..image.height() {
        check(token)?;
        for x in 0..image.width() {
            let color = rgba(image, x, y);
            if color[3] == 0. && white == (0, 0) {
                continue;
            }
            let px = i64::from(x) + i64::from(local.map_or(placement.x, |v| v.0));
            let py = i64::from(y) + i64::from(local.map_or(placement.y, |v| v.1));
            if (local.is_none()
                && (px < 0
                    || py < 0
                    || px >= i64::from(placement.width)
                    || py >= i64::from(placement.height)))
                || px.abs() >= 2 * i64::from(paint_core::MAX_DIMENSION)
                || py.abs() >= 2 * i64::from(paint_core::MAX_DIMENSION)
            {
                return Err(Error::Unsupported(
                    "nontransparent layer content outside canvas".into(),
                ));
            }
            let coord = TileCoord::from_signed(px.div_euclid(64) as i32, py.div_euclid(64) as i32);
            if let std::collections::btree_map::Entry::Vacant(entry) = tiles.entry(coord) {
                *total += 1;
                if *total > 1_000_000 {
                    return Err(Error::Limit("logical document tile limit"));
                }
                entry.insert(
                    (0..4096)
                        .map(|i| {
                            let x = px.div_euclid(64) * 64 + i % 64;
                            let y = py.div_euclid(64) * 64 + i / 64;
                            if x >= 0 && y >= 0 && x < i64::from(white.0) && y < i64::from(white.1)
                            {
                                Pixel::WHITE
                            } else {
                                Pixel::TRANSPARENT
                            }
                        })
                        .collect(),
                );
            }
            tiles.get_mut(&coord).unwrap()[(py.rem_euclid(64) * 64 + px.rem_euclid(64)) as usize] =
                Pixel::from_straight(color)?;
        }
    }
    let tiles = tiles
        .into_iter()
        .map(|(c, p)| Ok((c, Tile::from_pixels(p)?)))
        .collect::<Result<_>>()?;
    Ok(ImportedLayer {
        name: placement.name,
        properties: placement.properties,
        tiles,
    })
}
pub(crate) fn decode_document(
    bytes: &[u8],
    token: &CancellationToken,
    options: DocumentOptions,
    storage: std::sync::Arc<paint_storage::ScratchSpace>,
) -> Result<Document> {
    let image = decode_image(bytes, token)?;
    let (w, h) = image.dimensions();
    let layer = stage(
        &image,
        Placement {
            name: "Imported image".into(),
            properties: LayerProperties::default(),
            x: 0,
            y: 0,
            width: w,
            height: h,
        },
        &mut 0,
        token,
    )?;
    Ok(Document::from_import_with_storage(
        w,
        h,
        vec![layer],
        0,
        options,
        storage,
    )?)
}
pub(crate) fn png(writer: impl Write, bytes: &[u8], w: u32, h: u32) -> Result<()> {
    image::codecs::png::PngEncoder::new(writer).write_image(
        bytes,
        w,
        h,
        ExtendedColorType::Rgba8,
    )?;
    Ok(())
}
pub(crate) fn merged(snapshot: &DocumentSnapshot, token: &CancellationToken) -> Result<Vec<u8>> {
    let mut bytes =
        Vec::with_capacity((u64::from(snapshot.width) * u64::from(snapshot.height) * 4) as usize);
    for y in 0..snapshot.height {
        check(token)?;
        for pixel in snapshot.read_row(0, y, snapshot.width)? {
            bytes.extend(straight8(pixel.components()));
        }
    }
    Ok(bytes)
}
pub(crate) fn encode_snapshot(
    writer: impl Write,
    snapshot: &DocumentSnapshot,
    options: ExportOptions,
    token: &CancellationToken,
) -> Result<()> {
    check(token)?;
    match options.format {
        Format::Png | Format::WebP => {
            let bytes = merged(snapshot, token)?;
            if options.format == Format::Png {
                png(writer, &bytes, snapshot.width, snapshot.height)?;
            } else {
                image::codecs::webp::WebPEncoder::new_lossless(writer).encode(
                    &bytes,
                    snapshot.width,
                    snapshot.height,
                    ExtendedColorType::Rgba8,
                )?;
            }
        }
        Format::Jpeg => {
            // Flatten in linear space, before gamma encoding; transparent black is not a matte.
            let mut rgb = Vec::with_capacity(
                (u64::from(snapshot.width) * u64::from(snapshot.height) * 3) as usize,
            );
            for y in 0..snapshot.height {
                check(token)?;
                for pixel in snapshot.read_row(0, y, snapshot.width)? {
                    let p = pixel.components();
                    for (c, bg) in p[..3].iter().zip(options.background) {
                        rgb.push(encoded(c + bg * (1. - p[3])));
                    }
                }
            }
            image::codecs::jpeg::JpegEncoder::new_with_quality(writer, options.quality).encode(
                &rgb,
                snapshot.width,
                snapshot.height,
                ExtendedColorType::Rgb8,
            )?;
        }
        Format::OpenRaster => unreachable!(),
    }
    check(token)?;
    Ok(())
}
