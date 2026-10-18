use crate::api::*;
use std::{
    cell::RefCell,
    panic::{catch_unwind, AssertUnwindSafe},
};

thread_local! { static LAST_ERROR: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) }; }

#[derive(Debug, Clone)]
pub(crate) struct ApiError {
    pub status: PaintStatus,
    pub message: String,
}
pub(crate) type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn new(status: PaintStatus, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
    pub fn invalid(message: &'static str) -> Self {
        Self::new(PAINT_INVALID_ARGUMENT, message)
    }
    pub fn handle() -> Self {
        Self::new(
            PAINT_INVALID_HANDLE,
            "invalid, released, wrong-type or cross-core handle",
        )
    }
    pub fn internal() -> Self {
        Self::new(
            PAINT_INTERNAL_ERROR,
            "internal state poisoned; release the affected handle",
        )
    }
}

impl From<paint_core::Error> for ApiError {
    fn from(error: paint_core::Error) -> Self {
        let status = match error {
            paint_core::Error::InvalidArgument(_)
            | paint_core::Error::NoStroke
            | paint_core::Error::LastLayer => PAINT_INVALID_ARGUMENT,
            paint_core::Error::LayerLocked => PAINT_LAYER_LOCKED,
            paint_core::Error::Busy => PAINT_BUSY,
            paint_core::Error::LayerNotFound(_) => PAINT_NOT_FOUND,
            paint_core::Error::ResourceLimit(_) => PAINT_LIMIT_EXCEEDED,
            paint_core::Error::HistoryStorage(_) => PAINT_IO_ERROR,
            paint_core::Error::Storage(_) => PAINT_IO_ERROR,
        };
        Self::new(status, error.to_string())
    }
}

pub(crate) fn last_error() -> Vec<u8> {
    LAST_ERROR.with(|value| value.borrow().clone())
}

pub(crate) fn boundary(operation: impl FnOnce() -> ApiResult<()>) -> PaintStatus {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => {
            LAST_ERROR.with(|value| value.borrow_mut().clear());
            PAINT_OK
        }
        Ok(Err(error)) => {
            LAST_ERROR.with(|value| *value.borrow_mut() = error.message.into_bytes());
            error.status
        }
        Err(_) => {
            LAST_ERROR.with(|value| {
                *value.borrow_mut() =
                    b"Rust panic contained at C ABI boundary; affected state may be poisoned"
                        .to_vec()
            });
            PAINT_INTERNAL_ERROR
        }
    }
}

/// Error-message queries preserve the original TLS error, including a small-buffer result.
pub(crate) fn query_boundary(operation: impl FnOnce() -> ApiResult<()>) -> PaintStatus {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => PAINT_OK,
        Ok(Err(error)) => error.status,
        Err(_) => PAINT_INTERNAL_ERROR,
    }
}
