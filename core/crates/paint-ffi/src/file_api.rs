use crate::{
    api::*,
    error::{boundary, ApiError},
    file_job::Request,
    pointers, runtime,
};
use std::{mem::size_of, path::PathBuf};
/// Queue file IO; at most one job per session. Job ID remains valid until the next submission.
/// # Safety
/// Initialize DTO sizes; UTF-8 path bytes must remain readable for this call only.
#[no_mangle]
pub unsafe extern "C" fn paint_session_file_submit(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintFileRequest,
    out_job: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_job, 0)?;
        runtime::outside_callback()?;
        let r = pointers::read_sized(request)?;
        if r.reserved != [0; 2] || !matches!(r.kind, PAINT_FILE_OPEN | PAINT_FILE_SAVE) {
            return Err(ApiError::invalid("invalid file request"));
        }
        let format = match r.format {
            PAINT_FILE_AUTO if r.kind == PAINT_FILE_OPEN => paint_io::Format::Png,
            PAINT_FILE_PNG => paint_io::Format::Png,
            PAINT_FILE_JPEG => paint_io::Format::Jpeg,
            PAINT_FILE_WEBP => paint_io::Format::WebP,
            PAINT_FILE_OPENRASTER => paint_io::Format::OpenRaster,
            _ => return Err(ApiError::new(PAINT_UNSUPPORTED, "file format")),
        };
        if r.kind == PAINT_FILE_OPEN && r.format != PAINT_FILE_AUTO {
            return Err(ApiError::invalid("open requires signature auto-detection"));
        }
        if r.quality == 0 || r.quality > 100 || r.linear_background[3] != 1. {
            return Err(ApiError::invalid("invalid file encoding parameters"));
        }
        let options = paint_io::ExportOptions {
            format,
            quality: r.quality as u8,
            background: [
                r.linear_background[0],
                r.linear_background[1],
                r.linear_background[2],
            ],
        };
        options
            .validate()
            .map_err(|_| ApiError::invalid("invalid background"))?;
        let request = Request {
            path: PathBuf::from(pointers::path_utf8(r.path, r.path_length)?),
            kind: r.kind,
            format: r.format,
            generation: r.document_generation,
            options,
        };
        let state = runtime::registry()?.session(core, session)?;
        pointers::write(out_job, state.submit_file(request)?)
    })
}
/// Query a small retained result without waiting for disk access.
/// # Safety
/// Follow handle and writable initialized DTO contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_session_file_info(
    core: *mut PaintCore,
    session: *mut PaintSession,
    job: u64,
    out: *mut PaintFileJobInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out,
            PaintFileJobInfo {
                struct_size: size_of::<PaintFileJobInfo>() as u32,
                ..Default::default()
            },
        )?;
        let state = runtime::registry()?.session(core, session)?;
        let file = state.file.lock().map_err(|_| ApiError::internal())?;
        if job == 0 || job != file.info.job_id {
            return Err(ApiError::new(
                PAINT_NOT_FOUND,
                "file job expired or unknown",
            ));
        }
        pointers::write(out, file.info)
    })
}
/// Cooperative cancellation. Successful atomic commits stay successful.
/// # Safety
/// Follow the handle lifetime and serialization contract in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_session_file_cancel(
    core: *mut PaintCore,
    session: *mut PaintSession,
    job: u64,
) -> PaintStatus {
    boundary(|| {
        let state = runtime::registry()?.session(core, session)?;
        let file = state.file.lock().map_err(|_| ApiError::internal())?;
        if job == 0 || job != file.info.job_id {
            return Err(ApiError::new(
                PAINT_NOT_FOUND,
                "file job expired or unknown",
            ));
        }
        if !matches!(file.info.state, PAINT_FILE_QUEUED | PAINT_FILE_RUNNING) {
            return Err(ApiError::new(PAINT_BUSY, "file job already completed"));
        }
        file.cancel.cancel();
        Ok(())
    })
}
/// Copy asynchronous file error text as UTF-8, without a trailing NUL.
/// # Safety
/// Output capacity/required pointers must meet the buffer contract in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_session_file_message(
    core: *mut PaintCore,
    session: *mut PaintSession,
    job: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        let state = runtime::registry()?.session(core, session)?;
        let file = state.file.lock().map_err(|_| ApiError::internal())?;
        if job == 0 || job != file.info.job_id {
            return Err(ApiError::new(
                PAINT_NOT_FOUND,
                "file job expired or unknown",
            ));
        }
        pointers::copy_buffer(file.message.as_bytes(), buffer, capacity, required)
    })
}
