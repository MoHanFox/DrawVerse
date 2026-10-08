use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidArgument(&'static str),
    Busy,
    LayerLocked,
    NoStroke,
    LayerNotFound(u64),
    LastLayer,
    ResourceLimit(&'static str),
    HistoryStorage(String),
    Storage(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(message) => write!(f, "invalid argument: {message}"),
            Self::LayerLocked => write!(f, "layer is locked; unlock it before editing"),
            Self::Busy => write!(f, "finish or cancel the active stroke first"),
            Self::NoStroke => write!(f, "no active stroke"),
            Self::LayerNotFound(id) => write!(f, "layer {id} does not exist"),
            Self::LastLayer => write!(f, "cannot remove the last layer"),
            Self::ResourceLimit(message) => write!(f, "resource limit: {message}"),
            Self::HistoryStorage(message) => write!(f, "history storage: {message}"),
            Self::Storage(message) => write!(f, "document storage: {message}"),
        }
    }
}

impl std::error::Error for Error {}
