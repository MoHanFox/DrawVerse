//! The only module performing caller-memory reads/writes. Never dereference handle tokens.
use crate::{
    api::*,
    error::{ApiError, ApiResult},
};
use std::{
    mem::{align_of, size_of},
    ptr,
};

pub(crate) fn valid_pointer<T>(pointer: *const T) -> ApiResult<()> {
    if pointer.is_null() || pointer.addr() % align_of::<T>() != 0 {
        return Err(ApiError::invalid("NULL or misaligned parameter pointer"));
    }
    Ok(())
}

pub(crate) unsafe fn read<T: Copy>(pointer: *const T) -> ApiResult<T> {
    valid_pointer(pointer)?;
    // SAFETY: the caller guarantees readable initialized T after alignment/NULL validation.
    Ok(unsafe { pointer.read() })
}

pub(crate) unsafe fn write<T>(pointer: *mut T, value: T) -> ApiResult<()> {
    valid_pointer(pointer)?;
    // SAFETY: caller guarantees writable T, nonoverlapping with all input parameters.
    unsafe { pointer.write(value) };
    Ok(())
}

/// Copy a fixed-size slice into caller memory, after validating pointer and alignment.
pub(crate) unsafe fn write_slice<T: Copy>(pointer: *mut T, values: &[T]) -> ApiResult<()> {
    valid_pointer(pointer)?;
    if pointer.addr() % std::mem::align_of::<T>() != 0 {
        return Err(ApiError::invalid("output pointer is misaligned"));
    }
    // SAFETY: the caller guarantees writable storage for exactly `values.len()` elements.
    unsafe { std::ptr::copy_nonoverlapping(values.as_ptr(), pointer, values.len()) };
    Ok(())
}

pub(crate) unsafe fn sized_pointer<T>(pointer: *const T) -> ApiResult<()> {
    valid_pointer(pointer)?;
    // SAFETY: each public ABI DTO begins with initialized u32 struct_size.
    let declared = unsafe { pointer.cast::<u32>().read() };
    if declared < size_of::<T>() as u32 {
        return Err(ApiError::invalid(
            "struct_size is smaller than the ABI v1 prefix",
        ));
    }
    Ok(())
}

pub(crate) unsafe fn read_sized<T: Copy>(pointer: *const T) -> ApiResult<T> {
    unsafe {
        sized_pointer(pointer)?;
        read(pointer)
    }
}

pub(crate) unsafe fn reset_sized<T>(pointer: *mut T, value: T) -> ApiResult<()> {
    unsafe {
        sized_pointer(pointer)?;
        write(pointer, value)
    }
}

/// Read a caller-owned array of POD DTOs. `count` is bounded by the caller's protocol limit before
/// any memory is touched, so a hostile count cannot make the backend read out of bounds.
pub(crate) unsafe fn dto_slice<T: Copy>(
    pointer: *const T,
    count: usize,
) -> ApiResult<&'static [T]> {
    const MAX_DTO_ITEMS: usize = 65536;
    if count > MAX_DTO_ITEMS {
        return Err(ApiError::invalid("DTO array exceeds the item limit"));
    }
    if count == 0 {
        return Ok(&[]);
    }
    if pointer.is_null() || pointer.addr() % align_of::<T>() != 0 {
        return Err(ApiError::invalid("NULL or misaligned DTO array"));
    }
    // SAFETY: the caller guarantees `count` readable initialized T values behind this pointer.
    Ok(unsafe { std::slice::from_raw_parts(pointer, count) })
}

pub(crate) fn length(value: u64) -> ApiResult<usize> {
    if value > isize::MAX as u64 {
        return Err(ApiError::invalid("buffer length exceeds addressable range"));
    }
    usize::try_from(value).map_err(|_| ApiError::invalid("buffer length exceeds platform range"))
}

pub(crate) unsafe fn utf8(pointer: *const u8, len: u64) -> ApiResult<String> {
    unsafe { bounded_utf8(pointer, len, 1024) }
}
pub(crate) unsafe fn path_utf8(pointer: *const u8, len: u64) -> ApiResult<String> {
    unsafe { bounded_utf8(pointer, len, 32768) }
}
unsafe fn bounded_utf8(pointer: *const u8, len: u64, max: usize) -> ApiResult<String> {
    let len = length(len)?;
    if len == 0 || len > max {
        return Err(ApiError::invalid(
            "UTF-8 string exceeds byte limit or is empty",
        ));
    }
    valid_pointer(pointer)?;
    // SAFETY: caller guarantees len readable bytes; application bound was checked first.
    let bytes = unsafe { std::slice::from_raw_parts(pointer, len) };
    if bytes.contains(&0) {
        return Err(ApiError::invalid("UTF-8 name contains NUL"));
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| ApiError::invalid("invalid UTF-8"))
}

pub(crate) unsafe fn copy_buffer(
    bytes: &[u8],
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> ApiResult<()> {
    unsafe { write(required, bytes.len() as u64)? };
    let capacity = length(capacity)?;
    if capacity < bytes.len() {
        return Err(ApiError::new(PAINT_BUFFER_TOO_SMALL, "buffer too small"));
    }
    if !bytes.is_empty() {
        valid_pointer(buffer)?;
        // SAFETY: validated byte count; caller guarantees writable nonoverlapping buffer.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len()) };
    }
    Ok(())
}

pub(crate) unsafe fn copy_row(bytes: &[u8], buffer: *mut u8, offset: usize) {
    // SAFETY: full required capacity/offset validated by read_tile before any copying.
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.add(offset), bytes.len()) };
}
