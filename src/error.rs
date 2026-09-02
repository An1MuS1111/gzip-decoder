use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GzipError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Unexpected EOF: {err}")]
    UnexpectedEof { err: &'static str },

    #[error("Invalid GZIP magic bytes: expected [0x1F, 0x8B], found [0x{0:02X}, 0x{1:02X}]")]
    InvalidMagic(u8, u8),

    #[error("Unsupported compression method: {0} (only DEFLATE/8 is supported)")]
    UnsupportedMethod(u8),

    #[error("Reserved flag bits are non-zero: 0x{0:02X}")]
    ReservedFlags(u8),

    #[error("Header CRC16 mismatch: expected 0x{expected:04X}, calculated 0x{calculated:04X}")]
    HeaderCrcMismatch { expected: u16, calculated: u16 },

    #[error("Data CRC32 mismatch: expected 0x{expected:08X}, calculated 0x{calculated:08X}")]
    DataCrcMismatch { expected: u32, calculated: u32 },

    #[error("Uncompressed size mismatch: expected {expected} bytes, calculated {calculated} bytes")]
    SizeMismatch { expected: u32, calculated: u32 },
}

pub type GzipResult<T> = Result<T, GzipError>;
