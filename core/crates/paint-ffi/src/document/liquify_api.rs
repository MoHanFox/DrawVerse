use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
use paint_render::{CachedCpuRenderer, FramePixels, PixelFormat, Region, RenderRequest, Renderer};

/// ABI 1.63: shared immutable resident presentation metadata (move or liquify).
/// # Safety
/// Follow paint_session_liquify_preview_info's caller-memory contract.
#[no_mangle]
pub unsafe extern "C" fn paint_session_resident_preview_info(
    core: *mut PaintCore,
    session: *mut PaintSession,
    out_info: *mut PaintResidentPreviewInfo,
) -> PaintStatus {
    unsafe { paint_session_liquify_preview_info(core, session, out_info) }
}

/// ABI 1.63: shared resident mesh/assets, with the ABI 1.62 packet format.
/// # Safety
/// Follow paint_session_read_liquify_preview's capacity and lifetime contract.
#[no_mangle]
pub unsafe extern "C" fn paint_session_read_resident_preview(
    core: *mut PaintCore,
    session: *mut PaintSession,
    generation: u64,
    revision: u64,
    source_id: u64,
    kind: u32,
    buffer: *mut u8,
    capacity: u64,
    out_required: *mut u64,
) -> PaintStatus {
    unsafe {
        paint_session_read_liquify_preview(
            core,
            session,
            generation,
            revision,
            source_id,
            kind,
            buffer,
            capacity,
            out_required,
        )
    }
}

pub(crate) fn preview_assets(
    source: &paint_core::ResidentPreviewSource,
) -> crate::error::ApiResult<Vec<u8>> {
    encode_preview_assets(source, None, &paint_task::CancellationToken::default())
}

pub(crate) fn move_preview_assets(
    source: &paint_core::ResidentPreviewSource,
    display_scale: f64,
) -> crate::error::ApiResult<Vec<u8>> {
    move_preview_assets_cancellable(
        source,
        display_scale,
        &paint_task::CancellationToken::default(),
    )
}

pub(crate) fn move_preview_assets_cancellable(
    source: &paint_core::ResidentPreviewSource,
    display_scale: f64,
    cancel: &paint_task::CancellationToken,
) -> crate::error::ApiResult<Vec<u8>> {
    encode_preview_assets(source, Some(display_scale), cancel)
}

fn encode_preview_assets(
    source: &paint_core::ResidentPreviewSource,
    sampled: Option<f64>,
    cancel: &paint_task::CancellationToken,
) -> crate::error::ApiResult<Vec<u8>> {
    let mut output = Vec::new();
    let mut images = Vec::new();
    for image in [&source.image, &source.below, &source.above] {
        // An empty split is identically transparent at every UV. Avoid both
        // compositing and transferring a full-canvas transparent GPU texture.
        if sampled.is_some() && image.visible_pixel_layers().is_empty() {
            output.extend(1_u32.to_le_bytes());
            output.extend(1_u32.to_le_bytes());
            images.push(vec![0; 4]);
            continue;
        }
        let scale = (f64::from(paint_render::MOVE_PREVIEW_MAX_EDGE)
            / image.width.max(image.height) as f64)
            .min(1.)
            .min(sampled.unwrap_or(1.));
        let width = (image.width as f64 * scale).ceil().max(1.) as u32;
        let height = (image.height as f64 * scale).ceil().max(1.) as u32;
        output.extend(width.to_le_bytes());
        output.extend(height.to_le_bytes());
        let request = RenderRequest {
            region: Region {
                x: 0.,
                y: 0.,
                width: image.width as f64,
                height: image.height as f64,
            },
            width,
            height,
            format: PixelFormat::SrgbRgba8Premultiplied,
        };
        let frame = if sampled.is_some() {
            paint_render::render_sampled_preview(image, request, cancel)
        } else {
            CachedCpuRenderer::default().render(image, request, cancel)
        }
        .map_err(|e| ApiError::new(PAINT_IO_ERROR, e.to_string()))?;
        let FramePixels::Srgb(bytes) = frame.pixels else {
            return Err(ApiError::internal());
        };
        images.push(bytes);
    }
    for bytes in images {
        output.extend(bytes);
    }
    Ok(output)
}

/// Build a display-only texture packet at caller-negotiated device/memory limits.
/// Chunking bounds renderer scratch space without bounding the complete texture.
pub(crate) fn transform_preview_assets(
    source: &paint_core::ResidentPreviewSource,
    max_edge: u32,
    max_bytes: u64,
) -> crate::error::ApiResult<Vec<u8>> {
    if max_edge == 0 || max_edge > i32::MAX as u32 || max_bytes < 36 {
        return Err(ApiError::invalid("transform texture limits"));
    }
    let images = [&source.image, &source.below, &source.above];
    let dimensions = images.map(|image| {
        if image.visible_pixel_layers().is_empty() {
            (1, 1)
        } else {
            (image.width, image.height)
        }
    });
    let sizes = transform_texture_sizes(dimensions, max_edge, max_bytes)?;
    let total = 24
        + sizes
            .iter()
            .map(|&(w, h)| u64::from(w) * u64::from(h) * 4)
            .sum::<u64>();
    let length = usize::try_from(total)
        .map_err(|_| ApiError::new(PAINT_LIMIT_EXCEEDED, "transform address space"))?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(|_| ApiError::new(PAINT_LIMIT_EXCEEDED, "transform texture allocation"))?;
    output.resize(length, 0);
    let cancel = paint_task::CancellationToken::default();
    let renderer = CachedCpuRenderer::default();
    let mut offset = 24;
    for (i, (image, &(width, height))) in images.iter().zip(&sizes).enumerate() {
        output[i * 8..i * 8 + 4].copy_from_slice(&width.to_le_bytes());
        output[i * 8 + 4..i * 8 + 8].copy_from_slice(&height.to_le_bytes());
        if dimensions[i] != (1, 1) || !image.visible_pixel_layers().is_empty() {
            // Render bounded strips; no new full-frame limit or point-sampling proxy.
            let chunk_width = width.min(paint_render::MAX_OUTPUT_EDGE);
            let chunk_height = height
                .min(paint_render::MAX_OUTPUT_EDGE)
                .min((paint_render::MAX_OUTPUT_PIXELS / u64::from(chunk_width)) as u32);
            for y in (0..height).step_by(chunk_height as usize) {
                for x in (0..width).step_by(chunk_width as usize) {
                    let w = chunk_width.min(width - x);
                    let h = chunk_height.min(height - y);
                    let sx = f64::from(image.width) / f64::from(width);
                    let sy = f64::from(image.height) / f64::from(height);
                    let request = RenderRequest {
                        region: Region {
                            x: f64::from(x) * sx,
                            y: f64::from(y) * sy,
                            width: f64::from(w) * sx,
                            height: f64::from(h) * sy,
                        },
                        width: w,
                        height: h,
                        format: PixelFormat::SrgbRgba8Premultiplied,
                    };
                    let frame = paint_render::render_native_preview(image, request, &cancel)
                        .and_then(|native| match native {
                            Some(frame) => Ok(frame),
                            None => renderer.render(image, request, &cancel),
                        })
                        .map_err(|e| ApiError::new(PAINT_IO_ERROR, e.to_string()))?;
                    let FramePixels::Srgb(bytes) = frame.pixels else {
                        return Err(ApiError::internal());
                    };
                    for row in 0..h as usize {
                        let target =
                            offset + ((y as usize + row) * width as usize + x as usize) * 4;
                        let from = row * w as usize * 4;
                        output[target..target + w as usize * 4]
                            .copy_from_slice(&bytes[from..from + w as usize * 4]);
                    }
                }
            }
        }
        offset += width as usize * height as usize * 4;
    }
    Ok(output)
}

fn transform_texture_sizes(
    dimensions: [(u32, u32); 3],
    max_edge: u32,
    max_bytes: u64,
) -> crate::error::ApiResult<[(u32, u32); 3]> {
    let mut scale = 1_f64;
    for (w, h) in dimensions {
        scale = scale.min(f64::from(max_edge) / f64::from(w.max(h)));
    }
    let at = |s: f64| {
        dimensions.map(|(w, h)| {
            (
                (f64::from(w) * s).floor().max(1.) as u32,
                (f64::from(h) * s).floor().max(1.) as u32,
            )
        })
    };
    let fits = |sizes: [(u32, u32); 3]| {
        24_u128
            + sizes
                .iter()
                .map(|&(w, h)| u128::from(w) * u128::from(h) * 4)
                .sum::<u128>()
            <= u128::from(max_bytes)
    };
    if !fits(at(0.)) {
        return Err(ApiError::new(
            PAINT_LIMIT_EXCEEDED,
            "transform texture budget",
        ));
    }
    if !fits(at(scale)) {
        let (mut low, mut high) = (0., scale);
        for _ in 0..64 {
            let middle = (low + high) * 0.5;
            if fits(at(middle)) {
                low = middle;
            } else {
                high = middle;
            }
        }
        scale = low;
    }
    Ok(at(scale))
}

/// ABI1.90: opt-in source texture limits; old resident packets remain unchanged.
/// # Safety
/// Caller-memory capacity/lifetime contract is identical to read_resident_preview.
#[no_mangle]
pub unsafe extern "C" fn paint_session_read_transform_preview_assets(
    core: *mut PaintCore,
    session: *mut PaintSession,
    generation: u64,
    source_id: u64,
    max_edge: u32,
    max_bytes: u64,
    buffer: *mut u8,
    capacity: u64,
    out_required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_required, 0)?;
        let session = runtime::registry()?.session(core, session)?;
        let publication = session.publication();
        publication
            .resident_preview
            .as_ref()
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "pixel preview"))?;
        if publication.info.document_generation != generation {
            return Err(ApiError::new(PAINT_BUSY, "transform source replaced"));
        }
        let source = publication
            .resident_source(source_id)
            .ok_or_else(|| ApiError::new(PAINT_BUSY, "transform source replaced"))?;
        pointers::copy_buffer(
            &session.transform_source_assets(&source, max_edge, max_bytes)?,
            buffer,
            capacity,
            out_required,
        )
    })
}

pub(crate) fn preview_selection(
    source: &paint_core::ResidentPreviewSource,
) -> crate::error::ApiResult<Vec<u8>> {
    if !source.selected {
        return Err(ApiError::new(PAINT_NOT_FOUND, "no resident selection"));
    }
    let (dw, dh) = (source.below.width, source.below.height);
    let scale = (2048. / f64::from(dw.max(dh))).min(1.);
    let (width, height) = (
        (f64::from(dw) * scale).ceil().max(1.) as u32,
        (f64::from(dh) * scale).ceil().max(1.) as u32,
    );
    let mut bytes = Vec::with_capacity(24 + (width * height) as usize);
    bytes.extend(width.to_le_bytes());
    bytes.extend(height.to_le_bytes());
    let [x, y, w, h] = source.bounds;
    for value in [
        -(x as f32) / w as f32,
        -(y as f32) / h as f32,
        dw as f32 / w as f32,
        dh as f32 / h as f32,
    ] {
        bytes.extend(value.to_le_bytes());
    }
    let (sx, sy) = (
        f64::from(dw) / f64::from(width),
        f64::from(dh) / f64::from(height),
    );
    let mut row = vec![0_f32; width as usize];
    for y in 0..height {
        source.image.selection().coverage_samples(
            sx * 0.5,
            (f64::from(y) + 0.5) * sy,
            sx,
            &mut row,
        );
        bytes.extend(row.iter().map(|v| (v.clamp(0., 1.) * 255.).round() as u8));
    }
    Ok(bytes)
}

/// ABI 1.62: immutable mesh-only preview metadata, or NOT_FOUND for the pixel path.
/// # Safety
/// Valid serialized handles and an initialized writable sized output are required.
#[no_mangle]
pub unsafe extern "C" fn paint_session_liquify_preview_info(
    core: *mut PaintCore,
    session: *mut PaintSession,
    out_info: *mut PaintLiquifyPreviewInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::sized_pointer(out_info)?;
        let publication = runtime::registry()?.session(core, session)?.publication();
        let preview = publication
            .resident_preview
            .as_ref()
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "pixel preview"))?;
        pointers::write(
            out_info,
            PaintLiquifyPreviewInfo {
                struct_size: std::mem::size_of::<PaintLiquifyPreviewInfo>() as u32,
                focus: u32::from(preview.focus)
                    | (u32::from(preview.forward) << 1)
                    | if preview.source.selected {
                        PAINT_RESIDENT_PREVIEW_SELECTED
                    } else {
                        0
                    },
                document_generation: publication.info.document_generation,
                revision: preview.revision,
                source_id: preview.source.id,
                columns: preview.columns,
                rows: preview.rows,
                document_width: preview.source.below.width,
                document_height: preview.source.below.height,
            },
        )
    })
}

/// ABI 1.62: kind 0 = LE f32 x/y/u/v vertices; kind 1 = six LE u32 image
/// dimensions followed by three tightly packed sRGB premultiplied RGBA8 images.
/// ABI 1.77: kind 2 = LE u32 width/height, four LE f32 identity UV coefficients,
/// then width*height R8 destination selection coverage (only with the selected flag).
/// ABI 1.78: kind 3 = LE u64 alternate source ID followed by its kind-1 static assets.
/// ABI 1.91: kind 4 = alternate/identity source IDs (two LE u64, zero if absent).
/// Full-resolution transform reads accept either source in the active pair.
/// Mesh reads require the exact revision; static assets only require source/generation.
/// Capacity probes write required only; insufficient buffers never receive partial bytes.
/// # Safety
/// Follow the caller-memory contract. The output buffer and required must not overlap.
#[no_mangle]
pub unsafe extern "C" fn paint_session_read_liquify_preview(
    core: *mut PaintCore,
    session: *mut PaintSession,
    generation: u64,
    revision: u64,
    source_id: u64,
    kind: u32,
    buffer: *mut u8,
    capacity: u64,
    out_required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_required, 0)?;
        let session = runtime::registry()?.session(core, session)?;
        let publication = session.publication();
        let preview = publication
            .resident_preview
            .as_ref()
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "pixel preview"))?;
        if publication.info.document_generation != generation
            || publication.resident_source(source_id).is_none()
            || kind == 0 && (preview.revision != revision || preview.source.id != source_id)
        {
            return Err(ApiError::new(PAINT_BUSY, "liquify preview replaced"));
        }
        match kind {
            0 => {
                let needed = preview.vertices.len() as u64 * 16;
                pointers::write(out_required, needed)?;
                if capacity < needed {
                    return Err(ApiError::new(PAINT_BUFFER_TOO_SMALL, "buffer too small"));
                }
                let bytes = preview
                    .vertices
                    .iter()
                    .flatten()
                    .flat_map(|f| f.to_le_bytes())
                    .collect::<Vec<_>>();
                pointers::copy_buffer(&bytes, buffer, capacity, out_required)
            }
            1 => pointers::copy_buffer(
                &if source_id == preview.source.id {
                    session.resident_assets(preview)?
                } else {
                    session.resident_source_assets(
                        &publication
                            .resident_source(source_id)
                            .expect("validated source"),
                    )?
                },
                buffer,
                capacity,
                out_required,
            ),
            2 => pointers::copy_buffer(
                &session.resident_selection(preview)?,
                buffer,
                capacity,
                out_required,
            ),
            3 => {
                let (id, assets) = session.resident_alternate_assets(source_id)?;
                let needed = 8 + assets.len() as u64;
                pointers::write(out_required, needed)?;
                if capacity < needed {
                    return Err(ApiError::new(PAINT_BUFFER_TOO_SMALL, "buffer too small"));
                }
                let mut packet = Vec::with_capacity(needed as usize);
                packet.extend(id.to_le_bytes());
                packet.extend_from_slice(&assets);
                pointers::copy_buffer(&packet, buffer, capacity, out_required)
            }
            4 => {
                let mut packet = Vec::with_capacity(16);
                let sources = publication.transform_sources.as_ref();
                let alternate = sources
                    .and_then(|sources| sources.iter().find(|source| source.id != source_id))
                    .map_or(0, |source| source.id);
                let identity = sources.map_or(0, |sources| sources[1].id);
                packet.extend(alternate.to_le_bytes());
                packet.extend(identity.to_le_bytes());
                pointers::copy_buffer(&packet, buffer, capacity, out_required)
            }
            _ => Err(ApiError::invalid("liquify preview packet kind")),
        }
    })
}
/// ABI 1.21: begin/preview/reset/cancel/commit a single atomic liquify transaction.
/// Only DAB requires a descriptor. All other actions require null. OK means queued.
/// # Safety
/// Provide initialized readable descriptor/points, writable nonoverlapping sequence and valid
/// serialized core/session ownership. Input is copied, never retained. No callbacks may reenter.
#[no_mangle]
pub unsafe extern "C" fn paint_session_liquify(
    core: *mut PaintCore,
    session: *mut PaintSession,
    kind: u32,
    dab: *const PaintLiquifyDab,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        if kind > PAINT_LIQUIFY_BEGIN_DEFERRED {
            return Err(ApiError::invalid("liquify action"));
        }
        let options = if kind == PAINT_LIQUIFY_DAB {
            let value = pointers::read_sized(dab)?;
            if value.pressure_enabled > 1 || value.reserved != 0 || value.reserved_tail != [0; 3] {
                return Err(ApiError::invalid("liquify flags/reserved"));
            }
            pointers::sized_pointer(&value.from)?;
            pointers::sized_pointer(&value.to)?;
            let model = paint_core::LiquifyDab {
                tool: value.tool,
                radius: value.radius,
                strength: value.strength,
                density: value.density,
                speed: value.speed,
                pressure_enabled: value.pressure_enabled != 0,
                from: crate::point(value.from)?,
                to: crate::point(value.to)?,
                elapsed_ms: value.elapsed_ms,
            };
            model.validate()?;
            Some(model)
        } else {
            if !dab.is_null() {
                return Err(ApiError::invalid("unexpected liquify descriptor"));
            }
            None
        };
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Liquify(kind, options))?;
        pointers::write(out_sequence, sequence)
    })
}

#[cfg(test)]
mod move_asset_tests {
    use super::*;

    #[test]
    fn actual_transform_asset_entry_diagnostic() {
        let Some(path) = std::env::var_os("DRAWVERSE_TRANSFORM_QUALITY_IMAGE") else {
            return;
        };
        let cancel = paint_task::CancellationToken::default();
        let mut doc = paint_io::load(std::path::Path::new(&path), &cancel).unwrap();
        let start = std::time::Instant::now();
        doc.begin_transform_layers_deferred(&[doc.active_layer()])
            .unwrap();
        let begun = start.elapsed();
        let source = doc.resident_preview().unwrap().source;
        let image = &source.image;
        let request = RenderRequest {
            region: Region {
                x: 0.,
                y: 0.,
                width: f64::from(image.width),
                height: f64::from(image.height),
            },
            width: image.width,
            height: image.height,
            format: PixelFormat::SrgbRgba8Premultiplied,
        };
        let frame = paint_render::render_native_preview(image, request, &cancel).unwrap();
        eprintln!(
            "BEGIN {begun:?} native {:?} layers {} qualified {}",
            start.elapsed() - begun,
            image.layers().len(),
            frame.is_some()
        );
        for layer in image.layers() {
            eprintln!("layer {:?} {:?}", layer.properties(), layer.appearance());
        }
        let start = std::time::Instant::now();
        let packet = transform_preview_assets(&source, 16384, 512 * 1024 * 1024).unwrap();
        eprintln!("packet {:?} bytes {}", start.elapsed(), packet.len());
        assert!(frame.is_some());
        doc.cancel_transform().unwrap();
    }

    #[test]
    fn transform_texture_plan_uses_device_and_memory_instead_of_resolution_cutoffs() {
        let dimensions = [(12000, 8000), (1, 1), (1, 1)];
        assert_eq!(
            transform_texture_sizes(dimensions, 16384, 400 * 1024 * 1024).unwrap(),
            dimensions
        );
        let by_device = transform_texture_sizes(dimensions, 6000, 400 * 1024 * 1024).unwrap();
        assert_eq!(by_device[0], (6000, 4000)); // More than the renderer's per-chunk 16M.
        let by_memory = transform_texture_sizes(dimensions, 16384, 32 * 1024 * 1024).unwrap();
        assert!(by_memory[0].0 < 6000);
        let total = 24
            + by_memory
                .iter()
                .map(|&(w, h)| u64::from(w) * u64::from(h) * 4)
                .sum::<u64>();
        assert!(total <= 32 * 1024 * 1024);
        assert!(transform_texture_sizes(dimensions, 16384, 35).is_err());
    }

    #[test]
    fn transform_texture_chunks_preserve_a_source_wider_than_renderer_limit() {
        use paint_core::*;
        let mut doc = Document::new(9000, 2).unwrap();
        doc.initialize_white_background().unwrap();
        doc.begin_transform_layers_deferred(&[doc.active_layer()])
            .unwrap();
        let source = doc.resident_preview().unwrap().source;
        let packet = transform_preview_assets(&source, 16384, 8 * 1024 * 1024).unwrap();
        let w = u32::from_le_bytes(packet[0..4].try_into().unwrap());
        let h = u32::from_le_bytes(packet[4..8].try_into().unwrap());
        assert_eq!((w, h), (source.image.width, source.image.height));
        assert!(w > 8192);
        for y in 0..h {
            for x in 0..w {
                let offset = 24 + ((y * w + x) * 4) as usize;
                assert_eq!(
                    &packet[offset..offset + 4],
                    &paint_render::display_pixel(source.image.pixel(x, y).components())
                );
            }
        }
        doc.cancel_transform().unwrap();
    }

    #[test]
    fn transform_assets_keep_original_texels_and_legacy_packet_bound() {
        use paint_core::*;
        let mut doc = Document::new(4096, 32).unwrap();
        doc.initialize_white_background().unwrap();
        doc.begin_stroke(
            Brush {
                radius: 0.7,
                color: Pixel::from_straight([0.1, 0.3, 0.8, 1.]).unwrap(),
                ..Default::default()
            },
            InputPoint::new(1000.5, 8.5, 1.),
        )
        .unwrap();
        doc.end_stroke().unwrap();
        doc.begin_transform_layers_deferred(&[doc.active_layer()])
            .unwrap();
        let source = doc.resident_preview().unwrap().source;
        let bytes = transform_preview_assets(&source, 16384, 128 * 1024 * 1024).unwrap();
        let width = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let height = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!((width, height), (source.image.width, source.image.height));
        assert!(width > 4096); // Includes original transparent guard texels.
        for y in 0..height {
            for x in 0..width {
                let at = 24 + ((y * width + x) * 4) as usize;
                let expected = paint_render::display_pixel(source.image.pixel(x, y).components());
                for (actual, expected) in bytes[at..at + 4].iter().zip(expected) {
                    assert!(actual.abs_diff(expected) <= 1, "texel {x},{y}");
                }
            }
        }
        // Empty split remains transparent, and opt-in does not change kind1.
        assert_eq!(
            &bytes[8..24],
            &[1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]
        );
        let legacy = preview_assets(&source).unwrap();
        assert_eq!(u32::from_le_bytes(legacy[0..4].try_into().unwrap()), 2048);
        doc.cancel_transform().unwrap();
    }

    #[test]
    fn move_assets_keep_viewport_density_and_match_proxy_centres() {
        use paint_core::*;
        let mut doc = Document::new(720, 480).unwrap();
        doc.initialize_white_background().unwrap();
        let id = doc.add_layer("moving").unwrap();
        doc.begin_stroke(
            Brush {
                radius: 80.,
                color: Pixel::from_straight([0.8, 0.2, 0.4, 0.6]).unwrap(),
                ..Default::default()
            },
            InputPoint::new(25., 30., 1.),
        )
        .unwrap();
        doc.end_stroke().unwrap();
        doc.add_mask(id).unwrap();
        doc.begin_stroke(
            Brush {
                radius: 20.,
                color: Pixel::from_straight([0., 0., 0., 0.7]).unwrap(),
                ..Default::default()
            },
            InputPoint::new(30., 35., 1.),
        )
        .unwrap();
        doc.end_stroke().unwrap();
        doc.group_layer(id, "group").unwrap();
        let depth = doc.history_depth();
        doc.begin_layer_move(id, true).unwrap();
        let source = doc.resident_preview().unwrap().source;
        assert!(source.bounds[0] < 0 && source.bounds[1] < 0);
        for scale in [1., 0.25] {
            let bytes = move_preview_assets(&source, scale).unwrap();
            let mut offset = 24;
            for (i, image) in [&source.image, &source.below, &source.above]
                .into_iter()
                .enumerate()
            {
                let effective_scale = scale.min(
                    f64::from(paint_render::MOVE_PREVIEW_MAX_EDGE)
                        / f64::from(image.width.max(image.height)),
                );
                let width = u32::from_le_bytes(bytes[i * 8..i * 8 + 4].try_into().unwrap());
                let height = u32::from_le_bytes(bytes[i * 8 + 4..i * 8 + 8].try_into().unwrap());
                if image.visible_pixel_layers().is_empty() {
                    assert_eq!((width, height), (1, 1));
                    assert_eq!(&bytes[offset..offset + 4], &[0; 4]);
                } else {
                    assert_eq!(
                        (width, height),
                        (
                            (f64::from(image.width) * effective_scale).ceil() as u32,
                            (f64::from(image.height) * effective_scale).ceil() as u32
                        )
                    );
                    for y in (0..height).step_by(7) {
                        for x in (0..width).step_by(11) {
                            let at = offset + ((y * width + x) * 4) as usize;
                            for (actual, expected) in
                                bytes[at..at + 4].iter().zip(paint_render::display_pixel(
                                    image
                                        .pixel(
                                            ((f64::from(x) + 0.5) * f64::from(image.width)
                                                / f64::from(width))
                                                as u32,
                                            ((f64::from(y) + 0.5) * f64::from(image.height)
                                                / f64::from(height))
                                                as u32,
                                        )
                                        .components(),
                                ))
                            {
                                assert!(actual.abs_diff(expected) <= 1);
                            }
                        }
                    }
                }
                offset += (width * height * 4) as usize;
            }
            assert_eq!(offset, bytes.len());
        }
        // Other tools retain full ordinary LOD proxy encoding, including empty splits.
        let ordinary = preview_assets(&source).unwrap();
        assert_eq!(
            u32::from_le_bytes(ordinary[16..20].try_into().unwrap()),
            720
        );
        doc.cancel_layer_move().unwrap();
        assert_eq!(doc.history_depth(), depth);
    }

    #[test]
    #[ignore = "manual first move asset workload; excludes document creation and Qt"]
    fn first_move_asset_workload() {
        use paint_core::*;
        let mut doc = Document::new(3840, 2160).unwrap();
        doc.initialize_white_background().unwrap();
        let id = doc.add_layer("moving").unwrap();
        doc.bucket_fill(BucketFill {
            x: 0,
            y: 0,
            color: Pixel::from_straight([0.8, 0.2, 0.4, 0.7]).unwrap(),
            opacity: 1.,
            tolerance: 0,
            antialias: false,
            contiguous: true,
            all_layers: false,
        })
        .unwrap();
        doc.add_mask(id).unwrap();
        doc.begin_stroke(
            Brush {
                radius: 300.,
                color: Pixel::from_straight([0., 0., 0., 1.]).unwrap(),
                ..Default::default()
            },
            InputPoint::new(1500., 1000., 1.),
        )
        .unwrap();
        doc.end_stroke().unwrap();
        doc.group_layer(id, "group").unwrap();
        for _ in 0..3 {
            doc.begin_layer_move(id, true).unwrap();
            let preview = doc.resident_preview().unwrap();
            let start = std::time::Instant::now();
            let bytes = move_preview_assets(&preview.source, 1024. / 3840.).unwrap();
            println!(
                "4K move first static assets {:.3}ms bytes={}",
                start.elapsed().as_secs_f64() * 1000.,
                bytes.len()
            );
            std::hint::black_box(bytes);
            doc.cancel_layer_move().unwrap();
        }
    }
}
