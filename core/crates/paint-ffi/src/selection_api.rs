use crate::{
    api::*,
    error::{boundary, ApiError, ApiResult},
    pointers, runtime,
    session::Operation,
};
use paint_core::{SelectionKind, SelectionOperation, SelectionShape};
use std::mem::size_of;

#[derive(Clone)]
pub(crate) enum Edit {
    Shape(SelectionShape, SelectionOperation),
    All,
    Clear,
    Invert,
}
fn model(value: PaintSelectionEdit) -> ApiResult<Edit> {
    if value.reserved != [0; 3] {
        return Err(ApiError::invalid("selection reserved fields"));
    }
    if value.action != PAINT_SELECTION_SHAPE {
        if value.operation != 0
            || value.shape != 0
            || value.antialias != 0
            || [value.x, value.y, value.width, value.height] != [0.; 4]
        {
            return Err(ApiError::invalid("unexpected non-shape selection fields"));
        }
        return match value.action {
            PAINT_SELECTION_ALL => Ok(Edit::All),
            PAINT_SELECTION_CLEAR => Ok(Edit::Clear),
            PAINT_SELECTION_INVERT => Ok(Edit::Invert),
            _ => Err(ApiError::invalid("selection action")),
        };
    }
    let operation = match value.operation {
        PAINT_SELECTION_REPLACE => SelectionOperation::Replace,
        PAINT_SELECTION_ADD => SelectionOperation::Add,
        PAINT_SELECTION_SUBTRACT => SelectionOperation::Subtract,
        PAINT_SELECTION_INTERSECT => SelectionOperation::Intersect,
        _ => return Err(ApiError::invalid("selection operation")),
    };
    let shape = SelectionShape::geometry(
        match value.shape {
            PAINT_SELECTION_RECTANGLE => SelectionKind::Rectangle,
            PAINT_SELECTION_ELLIPSE => SelectionKind::Ellipse,
            _ => return Err(ApiError::invalid("selection shape")),
        },
        value.x,
        value.y,
        value.width,
        value.height,
        match value.antialias {
            0 => false,
            1 => true,
            _ => return Err(ApiError::invalid("selection antialias")),
        },
    );
    shape.validate()?;
    Ok(Edit::Shape(shape, operation))
}
/// ABI 1.8: enqueue a geometric selection edit, copied and validated before returning.
/// # Safety
/// Initialize request size and writable sequence; follow contracts/abi.md ownership rules.
#[no_mangle]
pub unsafe extern "C" fn paint_session_edit_selection(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintSelectionEdit,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let edit = model(pointers::read_sized(request)?)?;
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Selection(edit))?;
        pointers::write(out_sequence, sequence)
    })
}
#[derive(Clone)]
pub(crate) struct Path {
    /// Validated for protocol compatibility; only the magic wand resolves a region now.
    #[allow(dead_code)]
    pub kind: u32,
    pub points: Vec<[f64; 2]>,
    pub tolerance: u32,
    /// Validated for protocol compatibility; the wand always replaces the selection.
    #[allow(dead_code)]
    pub operation: SelectionOperation,
    /// Validated for protocol compatibility; rasterized paths are intentionally hard-edged.
    #[allow(dead_code)]
    pub antialias: bool,
}
fn path_model(value: PaintSelectionPath) -> ApiResult<Path> {
    if value.reserved != [0; 2] {
        return Err(ApiError::invalid("selection path reserved fields"));
    }
    let operation = match value.operation {
        PAINT_SELECTION_REPLACE => SelectionOperation::Replace,
        PAINT_SELECTION_ADD => SelectionOperation::Add,
        PAINT_SELECTION_SUBTRACT => SelectionOperation::Subtract,
        PAINT_SELECTION_INTERSECT => SelectionOperation::Intersect,
        _ => return Err(ApiError::invalid("selection operation")),
    };
    let antialias = match value.antialias {
        0 => false,
        1 => true,
        _ => return Err(ApiError::invalid("selection antialias")),
    };
    let points = unsafe { pointers::dto_slice(value.points, value.point_count as usize)? };
    let points: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
    match value.edit_kind {
        PAINT_SELECTION_PATH_MAGIC => {
            if points.len() != 1 {
                return Err(ApiError::invalid("magic selection takes one seed point"));
            }
            if !points[0][0].is_finite() || !points[0][1].is_finite() {
                return Err(ApiError::invalid("selection path point"));
            }
            if value.tolerance > paint_core::MAX_WAND_TOLERANCE {
                return Err(ApiError::invalid("magic selection tolerance"));
            }
        }
        _ => return Err(ApiError::invalid("selection path kind")),
    }
    Ok(Path {
        kind: value.edit_kind,
        points,
        tolerance: value.tolerance,
        operation,
        antialias,
    })
}
/// ABI 1.12: enqueue a freehand-path (lasso) or content-derived (magic wand) selection edit.
/// Points are copied and validated before returning; the session resolves the region off the UI thread.
/// # Safety
/// `request` must point to an initialized `PaintSelectionPath`; `points` must be a readable array of
/// `point_count` initialized points; `out_sequence` must be writable.
#[no_mangle]
pub unsafe extern "C" fn paint_session_edit_selection_path(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintSelectionPath,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let value = pointers::read_sized(request)?;
        let path = path_model(value)?;
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::SelectionPath(path))?;
        pointers::write(out_sequence, sequence)
    })
}
/// ABI 1.8: query selection summary tied to exactly one immutable publication.
/// # Safety
/// Initialize output size and follow aligned writable memory/handle contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_selection_info(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    out: *mut PaintSelectionInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out,
            PaintSelectionInfo {
                struct_size: size_of::<PaintSelectionInfo>() as u32,
                ..Default::default()
            },
        )?;
        let state = runtime::registry()?.session(core, session)?.publication();
        if publication != state.info.publication {
            return Err(ApiError::new(PAINT_BUSY, "publication changed"));
        }
        pointers::write(
            out,
            PaintSelectionInfo {
                struct_size: size_of::<PaintSelectionInfo>() as u32,
                enabled: u32::from(state.selection.enabled()),
                step_count: state.selection.steps().len() as u32,
                reserved: 0,
            },
        )
    })
}
/// ABI 1.8: query one bounded selection step; inversion uses zero geometry.
/// # Safety
/// Initialize output size; index must belong to the same queried publication.
#[no_mangle]
pub unsafe extern "C" fn paint_session_selection_step(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    index: u32,
    out: *mut PaintSelectionStep,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out,
            PaintSelectionStep {
                struct_size: size_of::<PaintSelectionStep>() as u32,
                ..Default::default()
            },
        )?;
        let state = runtime::registry()?.session(core, session)?.publication();
        if publication != state.info.publication {
            return Err(ApiError::new(PAINT_BUSY, "publication changed"));
        }
        let step = state
            .selection
            .steps()
            .get(index as usize)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "selection step index"))?;
        let mut value = PaintSelectionStep {
            struct_size: size_of::<PaintSelectionStep>() as u32,
            operation: match step.operation {
                SelectionOperation::Replace => PAINT_SELECTION_REPLACE,
                SelectionOperation::Add => PAINT_SELECTION_ADD,
                SelectionOperation::Subtract => PAINT_SELECTION_SUBTRACT,
                SelectionOperation::Intersect => PAINT_SELECTION_INTERSECT,
                SelectionOperation::Invert => PAINT_SELECTION_STEP_INVERT,
            },
            ..Default::default()
        };
        if let Some(s) = step.shape.as_ref() {
            value.shape = match s.kind {
                SelectionKind::Rectangle => PAINT_SELECTION_RECTANGLE,
                SelectionKind::Ellipse => PAINT_SELECTION_ELLIPSE,
                SelectionKind::Mask => PAINT_SELECTION_MASK,
            };
            value.antialias = u32::from(s.antialias);
            value.x = s.x;
            value.y = s.y;
            value.width = s.width;
            value.height = s.height;
        }
        pointers::write(out, value)
    })
}
