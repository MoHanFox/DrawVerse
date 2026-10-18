use crate::{
    api::*,
    brush_preparation::CatalogTicket,
    error::{boundary, ApiError, ApiResult},
    pointers, runtime,
};
use paint_core::BrushPreset;
use std::{
    mem::size_of,
    sync::{Arc, Mutex},
};
#[derive(Default)]
pub(crate) struct Pose {
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    ticket: Option<Arc<CatalogTicket>>,
    preset: Option<Arc<BrushPreset>>,
    id: String,
    radius: f32,
}
impl Pose {
    fn bind(&self, ticket: Arc<CatalogTicket>, id: String, radius: f32) -> ApiResult<()> {
        *self.state.lock().map_err(|_| ApiError::internal())? = State {
            ticket: Some(ticket),
            preset: None,
            id,
            radius,
        };
        Ok(())
    }
    fn snapshot(&self) -> ApiResult<(Arc<BrushPreset>, f32)> {
        let (ticket, id, radius) = {
            let s = self.state.lock().map_err(|_| ApiError::internal())?;
            if let Some(preset) = &s.preset {
                return Ok((preset.clone(), s.radius));
            }
            (s.ticket.clone(), s.id.clone(), s.radius)
        };
        let ticket = ticket.ok_or_else(|| ApiError::invalid("pose is not bound"))?;
        let catalog = ticket.try_result()?;
        let preset = catalog
            .preset(&id)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "pose preset"))?;
        let mut s = self.state.lock().map_err(|_| ApiError::internal())?;
        if s.ticket
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &ticket))
            && s.id == id
            && s.radius == radius
        {
            s.preset = Some(preset.clone());
            s.ticket = None;
        }
        Ok((preset, radius))
    }
}
/// # Safety
/// Nonoverlapping writable output pointer required. No caller memory retained.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_pose_create(out: *mut *mut PaintBrushPose) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out, std::ptr::null_mut())?;
        runtime::outside_callback()?;
        let handle = runtime::registry()?.create_brush_pose()?;
        pointers::write(out, handle)
    })
}
/// # Safety
/// Live handles/readable bounded ID required. Binding never waits for preparation.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_pose_bind_prepared(
    pose: *mut PaintBrushPose,
    library: *mut PaintBrushLibrary,
    preparation: u64,
    id: *const u8,
    length: u64,
    radius: f32,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        if !radius.is_finite()
            || !(0.5..=paint_core::MAX_BRUSH_RADIUS).contains(&radius)
            || length == 0
            || length > 64
        {
            return Err(ApiError::invalid("pose parameters"));
        }
        let id = pointers::utf8(id, length)?;
        let (pose, ticket) = {
            let registry = runtime::registry()?;
            (
                registry.brush_pose(pose)?,
                registry.brush_library(library)?.ticket(preparation)?,
            )
        };
        pose.bind(ticket, id, radius)
    })
}
/// # Safety
/// Live pose/sized readable point and nonoverlapping aligned writable output required.
/// Busy/error clears output. Never reads files or waits for compilation on caller thread.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_pose_read(
    pose: *mut PaintBrushPose,
    point: *const PaintPoint,
    pressure_known: u32,
    direction_known: u32,
    dx: f64,
    dy: f64,
    out: *mut PaintTipPose,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(
            out,
            PaintTipPose {
                struct_size: size_of::<PaintTipPose>() as u32,
                ..Default::default()
            },
        )?;
        runtime::outside_callback()?;
        if pressure_known > 1
            || direction_known > 1
            || (!dx.is_finite() || !dy.is_finite())
            || (direction_known == 0 && (dx != 0. || dy != 0.))
        {
            return Err(ApiError::invalid("pose input flags/vector"));
        }
        let point = crate::point(pointers::read_sized(point)?)?;
        point.validate()?;
        let pose = runtime::registry()?.brush_pose(pose)?;
        let (preset, radius) = pose.snapshot()?;
        let value = preset.program().cursor_pose(
            radius,
            point,
            pressure_known == 1,
            (direction_known == 1).then_some((dx, dy)),
        )?;
        pointers::write(
            out,
            PaintTipPose {
                struct_size: size_of::<PaintTipPose>() as u32,
                radius: value.radius,
                angle: value.angle,
                roundness: value.roundness,
                ..Default::default()
            },
        )
    })
}
/// # Safety
/// Live pose, sized readable point and aligned nonoverlapping writable output required.
/// Busy/error clears output. No files read or compilation awaited on the caller thread.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_pose_read_composite(
    pose: *mut PaintBrushPose,
    point: *const PaintPoint,
    pressure_known: u32,
    direction_known: u32,
    dx: f64,
    dy: f64,
    out: *mut PaintCompositeTipPose,
) -> PaintStatus {
    boundary(|| unsafe {
        let empty_tip = PaintTipPose {
            struct_size: size_of::<PaintTipPose>() as u32,
            ..Default::default()
        };
        let mut output = PaintCompositeTipPose {
            struct_size: size_of::<PaintCompositeTipPose>() as u32,
            primary: empty_tip,
            secondary: empty_tip,
            ..Default::default()
        };
        pointers::write(out, output)?;
        runtime::outside_callback()?;
        if pressure_known > 1
            || direction_known > 1
            || !dx.is_finite()
            || !dy.is_finite()
            || (direction_known == 0 && (dx != 0. || dy != 0.))
        {
            return Err(ApiError::invalid("pose input flags/vector"));
        }
        let point = crate::point(pointers::read_sized(point)?)?;
        point.validate()?;
        let pose = runtime::registry()?.brush_pose(pose)?;
        let (preset, radius) = pose.snapshot()?;
        let value = preset.program().cursor_poses(
            radius,
            point,
            pressure_known == 1,
            (direction_known == 1).then_some((dx, dy)),
        )?;
        let tip = |p: paint_core::BrushPose| PaintTipPose {
            radius: p.radius,
            angle: p.angle,
            roundness: p.roundness,
            ..empty_tip
        };
        output.primary = tip(value.primary);
        if let Some(secondary) = value.secondary {
            output.secondary = tip(secondary);
            output.flags |= PAINT_POSE_SECONDARY;
        }
        if value.primary_image {
            output.flags |= PAINT_POSE_PRIMARY_IMAGE;
        }
        if value.secondary_image {
            output.flags |= PAINT_POSE_SECONDARY_IMAGE;
        }
        if value.secondary_overflow {
            output.flags |= PAINT_POSE_SECONDARY_OVERFLOW;
        }
        pointers::write(out, output)
    })
}
/// # Safety
/// Writable pointer containing a live pose handle required; clears it on success.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_pose_destroy(pose: *mut *mut PaintBrushPose) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let handle = pointers::read(pose)?;
        runtime::registry()?.destroy_brush_pose(handle)?;
        pointers::write(pose, std::ptr::null_mut())
    })
}
/// Release captured resources while reusing the handle token.
/// # Safety
/// Live pose handle required. No caller memory accessed or retained.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_pose_clear(pose: *mut PaintBrushPose) -> PaintStatus {
    boundary(|| {
        runtime::outside_callback()?;
        let pose = runtime::registry()?.brush_pose(pose)?;
        *pose.state.lock().map_err(|_| ApiError::internal())? = State::default();
        Ok(())
    })
}
