use bitflags::bitflags;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

// +---+---+---+---+---+---+---+---+---+---+
// |ID1|ID2|CM |FLG|     MTIME     |XFL|OS | (10-byte Fixed Header)
// +---+---+---+---+---+---+---+---+---+---+
// | (Optional Extra Fields - FEXTRA)      |
// +---------------------------------------+
// | (Optional Original Filename - FNAME)  |
// +---------------------------------------+
// | (Optional File Comment - FCOMMENT)    |
// +---------------------------------------+
// | (Optional Header CRC - FHCRC)         |
// +=======================================+
// |          Compressed Payload           | (DEFLATE blocks)
// +=======================================+
// |     CRC32     |     ISIZE     |         (8-byte Trailer)
// +---+---+---+---+---+---+---+---+

pub const ID1: u8 = 0x1f;
pub const ID2: u8 = 0x8b;

bitflags! {
    struct Flags: u8 /* FLG */ {
            const FTEXT      = 1 <<  0;
            const FHCRC      = 1 <<  1;
            const FEXTRA     = 1 <<  2;
            const FNAME      = 1 <<  3;
            const FCOMMENT   = 1 <<  4;
            const _reserved_1 = 1 << 5;
            const _reserved_2 = 1 << 6;
            const _reserved_3 = 1 << 7;
    }
}

#[repr(u8)]
pub enum CompressionMethod /* CM */ {
    Deflate = 8,
    Unknown(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OperatingSystem /* OS */ {
    Fat = 0,
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
    Unknown = 255,
}

#[repr(u8)]
enum ExtraFlags /* XFL */ {
    MaximumCompression = 2,
    FastestAlgorithms = 4,
    Default(u8),
}

// 10 byte fixed header
struct Header {
    compression_method: CompressionMethod,
    flags: Flags,
    modification_time: u32,
    extra_flags: ExtraFlags,
    os: OperatingSystem,
}

struct Optionals {
    pub extra: Option<Vec<u8>>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub header_crc: Option<u16>,
}

struct Trailer {
    crc32: u32,
    isize: u32,
}

pub struct Gzip<R = BufReader<File>>
where
    R: BufRead,
{
    pub header: Header,
    pub optionals: Optionals,
    pub trailer: Option<Trailer>,
    pub reader: R,
}

impl Gzip<BufReader<File>> {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }
}

impl<R: BufRead> Gzip<R> {
    pub fn from_reader(mut reader: R) -> io::Result<Self> {
        todo!()
    }
}
