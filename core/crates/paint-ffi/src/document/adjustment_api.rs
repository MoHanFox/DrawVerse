use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
/// ABI 1.87. action: BEGIN=0, UPDATE=1, COMMIT=2, CANCEL=3. kind: curves=0,
/// levels=1, HSL=2. Only UPDATE accepts settings; all input is copied before queuing.
/// # Safety
/// Valid handles, readable sized settings on UPDATE, writable nonoverlapping sequence.
#[no_mangle]
pub unsafe extern "C" fn paint_session_adjustment(
    core: *mut PaintCore,
    session: *mut PaintSession,
    action: u32,
    kind: u32,
    settings: *const PaintAdjustment,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        if action > 3 || kind > 2 {
            return Err(ApiError::invalid("adjustment action/kind"));
        }
        let value = if action == 1 {
            let v = pointers::read_sized(settings)?;
            if v.kind != kind {
                return Err(ApiError::invalid("adjustment kind mismatch"));
            }
            let value = paint_core::Adjustment {
                kind,
                parameters: v.parameters,
                lut: v.lut.to_vec(),
            };
            value.validate()?;
            Some(value)
        } else {
            if !settings.is_null() {
                return Err(ApiError::invalid("unexpected adjustment settings"));
            }
            None
        };
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Adjustment(action, kind, value))?;
        pointers::write(out_sequence, sequence)
    })
}

/// ABI 1.88. Read four 256-bin sampled source histograms (luma, R, G, B).
/// Requires an active adjustment and exact publication. At most 262144 source samples.
/// # Safety
/// Writable output for 1024 floats, valid session handles; no overlapping arguments.
#[no_mangle]
pub unsafe extern "C" fn paint_session_adjustment_histogram(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    bins: *mut f32,
    count: u32,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        if count != 1024 {
            return Err(ApiError::invalid("adjustment histogram size"));
        }
        let p = runtime::registry()?.session(core, session)?.publication();
        if p.info.publication != publication {
            return Err(ApiError::new(PAINT_BUSY, "publication changed"));
        }
        let h = p
            .adjustment_histogram
            .as_ref()
            .ok_or_else(|| ApiError::invalid("no adjustment histogram"))?;
        pointers::write_slice(bins, h)
    })
}
