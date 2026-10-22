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
    encode_preview_assets(source, None)
}

pub(crate) fn move_preview_assets(
    source: &paint_core::ResidentPreviewSource,
    display_scale: f64,
) -> crate::error::ApiResult<Vec<u8>> {
    encode_preview_assets(source, Some(display_scale))
}

fn encode_preview_assets(
    source: &paint_core::ResidentPreviewSource,
    sampled: Option<f64>,
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
        let edge = if sampled.is_some() {
            paint_render::MOVE_PREVIEW_EDGE
        } else {
            2048
        };
        let scale = (f64::from(edge) / image.width.max(image.height) as f64)
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
        let cancel = paint_task::CancellationToken::default();
        let frame = if sampled.is_some() {
            paint_render::render_sampled_preview(image, request, &cancel)
        } else {
            CachedCpuRenderer::default().render(image, request, &cancel)
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
            || preview.source.id != source_id
            || kind == 0 && preview.revision != revision
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
                &session.resident_assets(preview)?,
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
                let (id, assets) = session.resident_alternate_assets(preview.source.id)?;
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
    fn move_assets_match_proxy_centres_with_linked_mask_and_off_canvas_source() {
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
                    f64::from(paint_render::MOVE_PREVIEW_EDGE)
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
