mod error;

use bitflags::bitflags;
use bytes::{Buf, Bytes};
use error::{GzipError, GzipResult};
use memchr::memchr;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::marker::PhantomData;
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
    #[derive(Default)]
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
#[repr(u8)]
enum ExtraFlags /* XFL */ {
    MaximumCompression = 2,
    FastestAlgorithms = 4,
    Default,
}

impl From<u8> for ExtraFlags {
    fn from(value: u8) -> Self {
        match value {
            2 => Self::MaximumCompression,
            4 => Self::FastestAlgorithms,
            _ => Self::Default,
        }
    }
}

struct Optionals {
    pub extra: Option<Vec<u8>>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub header_crc: Option<u16>,
}

// pub struct Gzip<R = BufReader<File>>
// where
//     R: BufRead,
// {
//     pub header: Header,
//     pub optionals: Optionals,
//     pub trailer: Option<Trailer>,
//     pub reader: R,
// }
//
// impl Gzip<BufReader<File>> {
//     pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
//         let file = File::open(path)?;
//         let reader = BufReader::new(file);
//         Self::from_reader(reader)
//     }
// }
//
// impl<R: BufRead> Gzip<R> {
//     pub fn from_reader(mut reader: R) -> io::Result<Self> {
//         todo!()
//     }
// }

fn read_null_terminated(buf: &mut Bytes) -> GzipResult<String> {
    let null_pos = memchr(b'\0', buf).ok_or(GzipError::NeedMoreData)?;

    let bytes = buf.split_to(null_pos);
    buf.advance(1);

    Ok(bytes.iter().map(|&b| b as char).collect())
}

struct Start;

struct HeaderParsed /* 10 bit fixed header */ {
    compression_method: CompressionMethod,
    flags: Flags,
    modification_time: u32,
    extra_flags: ExtraFlags,
    os: OperatingSystem,
}

struct TrailerParsed /* 4 + 4 bit fixed trailer */ {
    crc32: u32,
    isize: u32,
}

struct Parser<S> {
    buf: Bytes,
    header: Option<HeaderParsed>,
    _marker: PhantomData<S>,
}

impl Parser<Start> {
    fn new(data: Bytes) -> Self {
        Self {
            buf: data,
            header: None,
            _marker: PhantomData,
        }
    }

    fn parse_header(self) -> GzipResult<Parser<HeaderParsed>> {
        let mut buf = self.buf;

        // Check fixed 10 byte header size
        if buf.remaining() < 10 {
            return Err(GzipError::InsufficantHeaderBits);
        }

        // Check identification bits
        if buf.get_u8() != ID1 {
            return Err(GzipError::InvalidIDBits(1));
        }
        if buf.get_u8() != ID2 {
            return Err(GzipError::InvalidIDBits(2));
        }

        // Extract the compression method
        // If compression_method != deflate then we will yeild to `GzipError::UnsupportedMethod`
        let compression_method: CompressionMethod = match buf.get_u8() {
            8 => CompressionMethod::Deflate,
            method => CompressionMethod::Unknown(method),
        };

        let flags = Flags::from_bits_retain(buf.get_u8());

        let modification_time = buf.get_u32_le();

        let extra_flags: ExtraFlags = buf.get_u8().into();

        let os: OperatingSystem = buf.get_u8().into();

        Ok(Parser {
            buf,
            header: Some(HeaderParsed {
                compression_method,
                flags,
                modification_time,
                extra_flags,
                os,
            }),
            _marker: PhantomData,
        })
    }
}
