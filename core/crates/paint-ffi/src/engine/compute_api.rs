//! Additive ABI 1.61. Enumeration/device creation run on the caller's backend worker.
use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
use std::sync::Arc;

pub(crate) fn executor(id: &str) -> Arc<dyn paint_ops::Executor> {
    if id == "cpu" {
        return Arc::new(paint_ops::Cpu);
    }
    #[cfg(feature = "gpu")]
    {
        Arc::new(paint_gpu::Accelerated::new(id))
    }
    #[cfg(not(feature = "gpu"))]
    {
        Arc::new(paint_ops::Cpu)
    }
}

/// UTF-8 JSON hardware list. Includes CPU; GPU builds also include auto and hardware. No terminator.
/// # Safety
/// Follow the caller-memory contract. out_required must be writable; buffer has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn paint_compute_hardware(
    core: *mut PaintCore,
    buffer: *mut u8,
    capacity: u64,
    out_required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_required, 0)?;
        runtime::outside_callback()?;
        runtime::registry()?.core(core)?;
        let mut devices = vec![
            serde_json::json!({"id":"cpu","name":"CPU（关闭 GPU 加速）","backend":"CPU"}),
            serde_json::json!({"id":"auto","name":"自动选择 GPU","backend":"Auto"}),
        ];
        #[cfg(feature="gpu")]
        devices.extend(paint_gpu::hardware().into_iter().map(|v|serde_json::json!({"id":v.id,"name":format!("{} ({})",v.name,v.backend),"backend":v.backend})));
        #[cfg(not(feature = "gpu"))]
        devices.truncate(1);
        let bytes = serde_json::to_vec(&devices).map_err(|_| ApiError::internal())?;
        pointers::copy_buffer(&bytes, buffer, capacity, out_required)
    })
}
/// Queue a backend preference. CPU works without wgpu; unknown/lost devices fall back visibly.
/// Takes effect when idle; active liquify/strokes reject it without replacing their executor.
/// # Safety
/// Provide readable UTF-8 id_length bytes, valid handles and writable out_sequence.
#[no_mangle]
pub unsafe extern "C" fn paint_session_set_compute(
    core: *mut PaintCore,
    session: *mut PaintSession,
    id: *const u8,
    id_length: u64,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let id = pointers::utf8(id, id_length)?;
        if id.is_empty() || id.len() > 1024 {
            return Err(ApiError::invalid("compute device id"));
        }
        let session = runtime::registry()?.session(core, session)?;
        let sequence = session.submit(Operation::Compute(id))?;
        pointers::write(out_sequence, sequence)
    })
}
/// Current selected executor, including the reason for a CPU fallback. No terminator.
/// # Safety
/// Follow the caller-memory contract; capacity and out_required use bytes.
#[no_mangle]
pub unsafe extern "C" fn paint_session_compute_status(
    core: *mut PaintCore,
    session: *mut PaintSession,
    buffer: *mut u8,
    capacity: u64,
    out_required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_required, 0)?;
        let session = runtime::registry()?.session(core, session)?;
        pointers::copy_buffer(
            session.publication().compute.as_bytes(),
            buffer,
            capacity,
            out_required,
        )
    })
}
