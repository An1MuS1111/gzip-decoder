/// +---+---+---+---+---+---+---+---+---+---+
/// |ID1|ID2|CM |FLG|     MTIME     |XFL|OS | (10-byte Fixed Header)
/// +---+---+---+---+---+---+---+---+---+---+
/// | (Optional Extra Fields - FEXTRA)      |
/// +---------------------------------------+
/// | (Optional Original Filename - FNAME)  |
/// +---------------------------------------+
/// | (Optional File Comment - FCOMMENT)    |
/// +---------------------------------------+
/// | (Optional Header CRC - FHCRC)         |
/// +=======================================+
/// |          Compressed Payload           | (DEFLATE blocks)
/// +=======================================+
/// |     CRC32     |     ISIZE     |         (8-byte Trailer)
/// +---+---+---+---+---+---+---+---+
pub mod error;
pub mod helper;
pub mod lz77;
pub mod parser;

use bitflags::bitflags;
use crc::{CRC_32_ISO_HDLC, Crc};

pub use error::{GzipError, GzipResult};
pub use parser::{Decoder, HeaderParsed, OptionalParsed, Start};

pub const GZIP_CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

pub const ID1: u8 = 0x1F;
pub const ID2: u8 = 0x8B;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Flags: u8 {
        const FTEXT    = 1 << 0;
        const FHCRC    = 1 << 1;
        const FEXTRA   = 1 << 2;
        const FNAME    = 1 << 3;
        const FCOMMENT = 1 << 4;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMethod {
    Deflate,
    Unknown(u8),
}

impl From<u8> for CompressionMethod {
    fn from(value: u8) -> Self {
        match value {
            8 => Self::Deflate,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OperatingSystem {
    Fat = 0,
    Amiga = 1,
    Vms = 2,
    Unix = 3,
    VmCms = 4,
    AtariTos = 5,
    Hpfs = 6,
    Macintosh = 7,
    ZSystem = 8,
    CpM = 9,
    Tops20 = 10,
    Ntfs = 11,
    Qdos = 12,
    AcornRiscOs = 13,
    Unknown = 255,
}

impl From<u8> for OperatingSystem {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Fat,
            1 => Self::Amiga,
            2 => Self::Vms,
            3 => Self::Unix,
            4 => Self::VmCms,
            5 => Self::AtariTos,
            6 => Self::Hpfs,
            7 => Self::Macintosh,
            8 => Self::ZSystem,
            9 => Self::CpM,
            10 => Self::Tops20,
            11 => Self::Ntfs,
            12 => Self::Qdos,
            13 => Self::AcornRiscOs,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtraFlags {
    MaximumCompression,
    FastestAlgorithm,
    Other(u8),
}

impl From<u8> for ExtraFlags {
    fn from(value: u8) -> Self {
        match value {
            2 => Self::MaximumCompression,
            4 => Self::FastestAlgorithm,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Optionals {
    pub extra: Option<Vec<u8>>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub header_crc: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub compression_method: CompressionMethod,
    pub flags: Flags,
    pub modification_time: u32,
    pub extra_flags: ExtraFlags,
    pub os: OperatingSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trailer {
    pub crc32: u32,
    pub isize: u32,
}
