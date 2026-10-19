use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
use paint_render::{CachedCpuRenderer, FramePixels, PixelFormat, Region, RenderRequest, Renderer};

pub(crate) fn preview_assets(
    source: &paint_core::LiquifyPreviewSource,
) -> crate::error::ApiResult<Vec<u8>> {
    let mut output = Vec::new();
    let mut images = Vec::new();
    for image in [&source.image, &source.below, &source.above] {
        let scale = (2048. / image.width.max(image.height) as f64).min(1.);
        let width = (image.width as f64 * scale).ceil().max(1.) as u32;
        let height = (image.height as f64 * scale).ceil().max(1.) as u32;
        output.extend(width.to_le_bytes());
        output.extend(height.to_le_bytes());
        let frame = CachedCpuRenderer::default()
            .render(
                image,
                RenderRequest {
                    region: Region {
                        x: 0.,
                        y: 0.,
                        width: image.width as f64,
                        height: image.height as f64,
                    },
                    width,
                    height,
                    format: PixelFormat::SrgbRgba8Premultiplied,
                },
                &paint_task::CancellationToken::default(),
            )
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
            .liquify_preview
            .as_ref()
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "pixel preview"))?;
        pointers::write(
            out_info,
            PaintLiquifyPreviewInfo {
                struct_size: std::mem::size_of::<PaintLiquifyPreviewInfo>() as u32,
                focus: u32::from(preview.focus),
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
            .liquify_preview
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
                &session.liquify_assets(preview)?,
                buffer,
                capacity,
                out_required,
            ),
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
