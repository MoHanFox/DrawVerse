use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers,
};
/// Pure bounded query; may run on the UI thread. JSON is 2–17 [x,y] pairs.
/// model: 0 legacy linear, 1 smooth-pchip-v1. Returns 257 evenly spaced samples.
/// # Safety
/// Input readable for length bytes; output aligned writable for 257 f32 values.
/// Buffers must not overlap. No caller memory retained. Errors clear valid output.
#[no_mangle]
pub unsafe extern "C" fn paint_pressure_curve_samples(
    json: *const u8,
    length: u64,
    model: u32,
    out: *mut f32,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write_slice(out, &[0.; 257])?;
        if length == 0 || length > 4096 || model > 1 {
            return Err(ApiError::invalid("pressure curve parameters"));
        }
        let text = pointers::utf8(json, length)?;
        let samples = paint_io::brush_pressure::curve_samples(text.as_bytes(), model == 1)
            .map_err(|e| ApiError::new(PAINT_INVALID_ARGUMENT, e.to_string()))?;
        pointers::write_slice(out, &samples)
    })
}

#[cfg(test)]
#[test]
fn native_samples_match_core_and_reject_invalid_caller_data() {
    let data = b"[[0,0],[0.5,0.25],[1,1]]";
    let mut out = [-1.; 257];
    unsafe {
        assert_eq!(
            paint_pressure_curve_samples(data.as_ptr(), data.len() as u64, 1, out.as_mut_ptr()),
            PAINT_OK
        );
        assert_eq!(out[64], 0.078125);
        assert_eq!(out[128], 0.25);
        for (ptr, len, mode) in [
            (std::ptr::null(), data.len() as u64, 1),
            (data.as_ptr(), 4097, 1),
            (data.as_ptr(), data.len() as u64, 2),
        ] {
            assert_ne!(
                paint_pressure_curve_samples(ptr, len, mode, out.as_mut_ptr()),
                PAINT_OK
            );
            assert_eq!(out, [0.; 257]);
        }
        assert_ne!(
            paint_pressure_curve_samples(data.as_ptr(), data.len() as u64, 1, std::ptr::null_mut()),
            PAINT_OK
        );
        let mut unaligned = [0u8; 1032];
        let address = unaligned.as_mut_ptr();
        let misaligned = address.add(usize::from(address.addr() % 4 == 0));
        assert_ne!(
            paint_pressure_curve_samples(data.as_ptr(), data.len() as u64, 1, misaligned.cast()),
            PAINT_OK
        );
        let invalid = b"[[0,0],[0,1],[1,1]]";
        assert_ne!(
            paint_pressure_curve_samples(
                invalid.as_ptr(),
                invalid.len() as u64,
                1,
                out.as_mut_ptr()
            ),
            PAINT_OK
        );
    }
}
