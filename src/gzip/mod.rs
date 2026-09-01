use bitflags::bitflags;

pub const ID1: u8 = 0x1f;
pub const ID2: u8 = 0x8b;

bitflags! {
    struct Flags: u8 {
            const FTEXT      = 1 <<  0;
            const FHCRC      = 1 <<  1;
            const FEXTRA     = 1 <<  2;
            const FNAME      = 1 <<  3;
            const FCOMMENT   = 1 <<  4;
    }
}

#[repr(u8)]
pub enum CM {
    Deflate,
    Unknown(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    Fat,
    Amiga,
    Vms,
    Unix,
    VmCms,
    AtariTos,
    Hpfs,
    Macintosh,
    ZSystem,
    CpM,
    Tops20,
    Ntfs,
    Qdos,
    AcornRiscOs,
    Unknown,
    Other(u8),
}

impl From<u8> for OperatingSystem {
    fn from(val: u8) -> Self {
        match val {
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
            255 => Self::Unknown,
            other => Self::Other(other),
        }
    }
}

#[repr(u8)]
enum ExtraFlags {
    MaximumConpression = 2,
    FastestAlgorithms = 4,
    Default(u8),
}

struct Header {
    compression_method: CM,

    flags: Flags,

    modification_time: u32,

    extra_flags: ExtraFlags,

    os: OperatingSystem,

    file_name: Option<String>,

    comment: Option<String>,
}

fn main() {
    println!("Hello, world!");
}
