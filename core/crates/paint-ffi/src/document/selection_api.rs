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
    Styled(
        SelectionShape,
        SelectionOperation,
        paint_core::HistoryAction,
    ),
    All,
    Layer(u64, SelectionOperation),
    Clear,
    Invert,
}

/// ABI 1.92: load a layer's intrinsic alpha (a mask's white coverage) as a selection.
/// Operation is REPLACE, ADD, SUBTRACT or INTERSECT. Does not change the active layer.
/// # Safety
/// Valid handles and writable sequence; ownership follows contracts/ffi/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_session_select_layer_alpha(
    core: *mut PaintCore,
    session: *mut PaintSession,
    layer: u64,
    operation: u32,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let operation = match operation {
            PAINT_SELECTION_REPLACE => SelectionOperation::Replace,
            PAINT_SELECTION_ADD => SelectionOperation::Add,
            PAINT_SELECTION_SUBTRACT => SelectionOperation::Subtract,
            PAINT_SELECTION_INTERSECT => SelectionOperation::Intersect,
            _ => return Err(ApiError::invalid("layer alpha selection operation")),
        };
        if layer == 0 {
            return Err(ApiError::invalid("layer alpha selection id"));
        }
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Selection(Edit::Layer(layer, operation)))?;
        pointers::write(out_sequence, sequence)
    })
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
/// Initialize request size and writable sequence; follow contracts/ffi/abi.md ownership rules.
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
/// Read the published selection summary for one publication id.
/// # Safety
/// `out` must point to an initialized `PaintSelectionInfo`; follow session lifetime contracts.
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
                SelectionKind::Polygon => PAINT_SELECTION_POLYGON,
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

/// ABI 1.15: submit any supported selection shape through the common selection engine.
/// # Safety
/// Sized request, readable point_count points for polygons, writable sequence and valid handles.
#[no_mangle]
pub unsafe extern "C" fn paint_session_edit_selection_shape(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintSelectionShapeEdit,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let value: PaintSelectionShapeEdit = pointers::read_sized(request)?;
        let operation = match value.operation {
            0 => SelectionOperation::Replace,
            1 => SelectionOperation::Add,
            2 => SelectionOperation::Subtract,
            3 => SelectionOperation::Intersect,
            _ => return Err(ApiError::invalid("selection operation")),
        };
        let antialias = match value.antialias {
            0 => false,
            1 => true,
            _ => return Err(ApiError::invalid("selection antialias")),
        };
        let (shape, action) = if value.shape == PAINT_SELECTION_POLYGON {
            if [value.x, value.y, value.width, value.height] != [0.; 4] {
                return Err(ApiError::invalid("polygon geometry fields"));
            }
            let action = match value.tool {
                1 => paint_core::HistoryAction::LassoSelection,
                2 => paint_core::HistoryAction::PolygonSelection,
                _ => return Err(ApiError::invalid("selection tool")),
            };
            (
                SelectionShape::polygon(
                    pointers::selection_points(value.points, value.point_count)?,
                    antialias,
                    value.feather,
                )?,
                action,
            )
        } else {
            if value.tool != 0 || value.point_count != 0 || !value.points.is_null() {
                return Err(ApiError::invalid("unexpected selection path"));
            }
            let (kind, action) = match value.shape {
                PAINT_SELECTION_RECTANGLE => (
                    SelectionKind::Rectangle,
                    paint_core::HistoryAction::Selection,
                ),
                PAINT_SELECTION_ELLIPSE => (
                    SelectionKind::Ellipse,
                    paint_core::HistoryAction::EllipseSelection,
                ),
                _ => return Err(ApiError::invalid("selection shape")),
            };
            let mut shape = SelectionShape::geometry(
                kind,
                value.x,
                value.y,
                value.width,
                value.height,
                antialias,
            );
            shape.feather = value.feather;
            shape.validate()?;
            (shape, action)
        };
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Selection(Edit::Styled(shape, operation, action)))?;
        pointers::write(out_sequence, sequence)
    })
}
/// ABI 1.15: copied feather/path for the same publication as selection_step.
/// On BUFFER_TOO_SMALL details contains the required point count; no point is written.
/// # Safety
/// Initialize details size; supply capacity writable points (or null with capacity zero).
#[no_mangle]
pub unsafe extern "C" fn paint_session_selection_details(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    index: u32,
    out: *mut PaintSelectionDetails,
    points: *mut PaintSelectionPoint,
    capacity: u32,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out,
            PaintSelectionDetails {
                struct_size: size_of::<PaintSelectionDetails>() as u32,
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
        let Some(shape) = &step.shape else {
            return Ok(());
        };
        pointers::write(
            out,
            PaintSelectionDetails {
                struct_size: size_of::<PaintSelectionDetails>() as u32,
                point_count: shape.points.len() as u32,
                feather: shape.feather,
            },
        )?;
        if (capacity as usize) < shape.points.len() {
            return Err(ApiError::new(
                PAINT_BUFFER_TOO_SMALL,
                "selection path capacity",
            ));
        }
        if !shape.points.is_empty() {
            let copied: Vec<_> = shape
                .points
                .iter()
                .map(|p| PaintSelectionPoint { x: p[0], y: p[1] })
                .collect();
            pointers::write_slice(points, &copied)?;
        }
        Ok(())
    })
}

/// ABI 1.95: current-selection operation: 0 feather, 1 copy layer, 2 cut layer,
/// 3 fill, 4 inside stroke; amount in pixels, straight linear RGBA.
/// # Safety
/// Valid session handles and writable sequence; standard session ownership applies.
#[no_mangle]
pub unsafe extern "C" fn paint_session_selection_pixels(
    core: *mut PaintCore,
    session: *mut PaintSession,
    kind: u32,
    amount: f64,
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        if kind > 4 || !amount.is_finite() || !(0.0..=1000.0).contains(&amount) {
            return Err(ApiError::invalid("selection pixels parameters"));
        }
        let rgba = [red, green, blue, alpha];
        paint_core::Pixel::from_straight(rgba)?;
        let seq = runtime::registry()?
            .session(core, session)?
            .submit(Operation::SelectionPixels(kind, amount, rgba))?;
        pointers::write(out_sequence, seq)
    })
}
