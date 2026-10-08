use crate::{
    api::*,
    error::{boundary, ApiError, ApiResult},
    pointers, runtime,
};
use paint_core::{DocumentOptions, TILE_BYTES};
use paint_storage::{Config, ScratchSpace};
use std::{mem::size_of, path::PathBuf, ptr, sync::Arc};

pub(crate) struct CoreStorage {
    pub options: DocumentOptions,
    pub space: Arc<ScratchSpace>,
}
impl CoreStorage {
    pub fn system() -> Arc<Self> {
        Arc::new(Self {
            options: DocumentOptions::default(),
            space: ScratchSpace::system(),
        })
    }
}
fn io_error(e: std::io::Error) -> ApiError {
    ApiError::new(
        if e.kind() == std::io::ErrorKind::WouldBlock {
            PAINT_BUSY
        } else {
            PAINT_IO_ERROR
        },
        e.to_string(),
    )
}
unsafe fn configuration(options: *const PaintStorageOptions) -> ApiResult<Arc<CoreStorage>> {
    let value = unsafe { pointers::read_sized(options)? };
    if value.reserved != 0
        || value.resident_bytes < TILE_BYTES as u64
        || value.resident_bytes > 1024 * 1024 * 1024
        || value.resident_bytes % TILE_BYTES as u64 != 0
        || !(65552..=64 * 1024 * 1024 * 1024).contains(&value.scratch_bytes)
        || value.min_free_bytes > 1024 * 1024 * 1024 * 1024
    {
        return Err(ApiError::invalid(
            "invalid storage budgets or reserved field",
        ));
    }
    let directory = if value.path_length == 0 {
        std::env::temp_dir()
    } else {
        PathBuf::from(unsafe { pointers::path_utf8(value.path, value.path_length)? })
    };
    if !directory.is_absolute() {
        return Err(ApiError::invalid("scratch directory must be absolute"));
    }
    Ok(Arc::new(CoreStorage {
        options: DocumentOptions {
            max_resident_tiles: (value.resident_bytes / TILE_BYTES as u64) as usize,
            max_scratch_bytes: value.scratch_bytes,
            ..Default::default()
        },
        space: ScratchSpace::new(Config {
            directory,
            min_free_bytes: value.min_free_bytes,
        }),
    }))
}
/// ABI 1.3: validate/probe/clean configured scratch space, then create a core. Worker thread only.
/// # Safety
/// Follow initialized DTO, writable output and nonoverlapping UTF-8 contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_create_with_storage(
    options: *const PaintStorageOptions,
    out_core: *mut *mut PaintCore,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_core, ptr::null_mut())?;
        runtime::outside_callback()?;
        let storage = configuration(options)?;
        storage.space.inspect().map_err(io_error)?;
        let core = runtime::registry()?.create_core(storage)?;
        pointers::write(out_core, core)
    })
}
/// ABI 1.3: inspect a candidate configuration without changing a running core. Worker thread only.
/// Validation creates a writable run and cleans marked unlocked stale runs; no document recovery.
/// # Safety
/// Follow initialized DTO and writable output contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_storage_inspect(
    options: *const PaintStorageOptions,
    out_info: *mut PaintStorageInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_info,
            PaintStorageInfo {
                struct_size: size_of::<PaintStorageInfo>() as u32,
                ..Default::default()
            },
        )?;
        runtime::outside_callback()?;
        let report = configuration(options)?.space.inspect().map_err(io_error)?;
        pointers::write(
            out_info,
            PaintStorageInfo {
                struct_size: size_of::<PaintStorageInfo>() as u32,
                removed_runs: report.removed_runs,
                available_bytes: report.available_bytes,
                removed_bytes: report.removed_bytes,
                skipped_runs: report.skipped_runs,
                reserved: 0,
            },
        )
    })
}
