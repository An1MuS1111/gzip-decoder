use std::{io, string::FromUtf8Error};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GzipError {
    #[error("Io error: {0}")]
    Io(#[from] io::Error),

    #[error("Insufficiant header bits: Need more data")]
    InsufficantHeaderBits,

    #[error("Need more data")]
    NeedMoreData,

    #[error("Unsupported method")]
    UnsupportedMethod,

    #[error("Invalid identification bits: {0}")]
    InvalidIDBits(u8),
}

pub type GzipResult<T> = Result<T, GzipError>;
