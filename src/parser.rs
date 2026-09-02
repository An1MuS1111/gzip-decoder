use crate::error::{GzipError, GzipResult};
use crate::{
    CompressionMethod, ExtraFlags, Flags, Header, ID1, ID2, OperatingSystem, Optionals, Trailer,
    crc32, header_crc16,
};
use bitflags::Flag;
use bytes::{Buf, Bytes, BytesMut};
use memchr::memchr;
use miniz_oxide::inflate::decompress_to_vec;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use byteorder::{ReadBytesExt, LittleEndian};

pub struct Parser<S> {
    pub buf: Bytes,
    pub state: S,
}

pub struct Decoder<R, S> {
    pub reader: R,
    pub state: S,
    pub buf: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Start;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderParsed {
    pub header: Header,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decompressed {
    pub header: Header,
    pub uncompressed_data: Bytes,
    pub calculated_crc32: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub header: Header,
    pub trailer: Trailer,
    pub uncompressed_data: Vec<u8>,
}

impl<R: BufRead> Decoder<R, Start> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            state: Start,
            buf: Vec::with_capacity(256),
        }
    }

    pub fn read_header(&mut self) -> GzipResult<Decoder<R, Header>> {
        let mut fixed_header = [0u8; 10];
        self.reader
            .read_exact(&mut fixed_header)
            .map_err(|e| GzipError::UnexpectedEof {
                err: format!("failed to read header: {}", e),
            })?;

        // verify the identification bytes ID1 & ID2
        if fixed_header[0] != ID1 || fixed_header[1] != {
            return Err(GzipError::InvalidMagic(fixed_header[0], fixed_header[1]))   
        }

        // verify the compression method
        let compression_method = match fixed_header[2] {
           8 => CompressionMethod::Deflate,
            other => return Err(GzipError::UnsupportedMethod(other))
        };

        // parse the flags
        let flags = match fixed_header[3] {
            raw_flags if raw_flags & 0xE0 != 0 =>  return Err(GzipError::ReservedFlags(raw_flags)),
            raw_flags => Flags::from_bits_truncate(raw_flags),
        };

        // parse the modification time u32 (4 bytes)
        let modification_time = fixed_header[4..8];
        
        // parse extra flags
        let extra_flags = ExtraFlags::from(fixed_header[8]);

        // parse operating system
        let operating_system = OperatingSystem::from(fixed_header[9]);

        // parse optional fields
        let optionals = Optionals::try_from(flags)?;

        todo!("need to finish the prasing Optionals")
        
    }
}

impl TryFrom<Flags> for Optionals {
    type Error = GzipError;

    fn try_from(value: Flags) -> Result<Self, Self::Error> {
       let mut optionals = Optionals::default(); 
    }
}

impl Parser<Start> {
    pub fn new(data: Bytes) -> Self {
        Self {
            buf: data,
            state: Start,
        }
    }

    pub fn parse_header(mut self) -> GzipResult<Parser<HeaderParsed>> {
        let initial_buf = self.buf.clone();

        // Check fixed 10-byte header length
        if self.buf.remaining() < 10 {
            return Err(GzipError::UnexpectedEof {
                expected: 10,
                found: self.buf.remaining(),
            });
        }

        // Check identification magic bytes (0x1F, 0x8B)
        let id1 = self.buf.get_u8();
        let id2 = self.buf.get_u8();
        if id1 != ID1 || id2 != ID2 {
            return Err(GzipError::InvalidMagic(id1, id2));
        }

        // Check compression method (must be Deflate / 8)
        let cm = self.buf.get_u8();
        let compression_method = match cm {
            8 => CompressionMethod::Deflate,
            other => return Err(GzipError::UnsupportedMethod(other)),
        };

        // Check flags and ensure reserved bits 5, 6, 7 are zero
        let raw_flags = self.buf.get_u8();
        if raw_flags & 0xE0 != 0 {
            return Err(GzipError::ReservedFlags(raw_flags));
        }
        let flags = Flags::from_bits_truncate(raw_flags);

        let modification_time = self.buf.get_u32_le();
        let extra_flags = ExtraFlags::from(self.buf.get_u8());
        let os = OperatingSystem::from(self.buf.get_u8());

        // Parse optional fields according to flags
        let optionals = self.parse_optionals(flags, &initial_buf)?;

        let header = Header {
            compression_method,
            flags,
            modification_time,
            extra_flags,
            os,
            optionals,
        };

        Ok(Parser {
            buf: self.buf,
            state: HeaderParsed { header },
        })
    }

    /// Parse optional GZIP header fields
    pub fn parse_optionals(&mut self, flags: Flags, initial_buf: &Bytes) -> GzipResult<Optionals> {
        let mut optionals = Optionals::default();

        // FEXTRA: Extra field
        if flags.contains(Flags::FEXTRA) {
            if self.buf.remaining() < 2 {
                return Err(GzipError::UnexpectedEof {
                    expected: 2,
                    found: self.buf.remaining(),
                });
            }
            let xlen = self.buf.get_u16_le() as usize;
            if self.buf.remaining() < xlen {
                return Err(GzipError::UnexpectedEof {
                    expected: xlen,
                    found: self.buf.remaining(),
                });
            }
            optionals.extra = Some(self.buf.copy_to_bytes(xlen).to_vec());
        }

        // FNAME: Original filename
        if flags.contains(Flags::FNAME) {
            optionals.name = Some(self.read_latin1_string()?);
        }

        // FCOMMENT: File comment
        if flags.contains(Flags::FCOMMENT) {
            optionals.comment = Some(self.read_latin1_string()?);
        }

        // FHCRC: Header CRC-16
        if flags.contains(Flags::FHCRC) {
            if self.buf.remaining() < 2 {
                return Err(GzipError::UnexpectedEof {
                    expected: 2,
                    found: self.buf.remaining(),
                });
            }

            // CRC16 covers all header bytes up to (not including) the CRC16 field
            let header_len_before_fhcrc = initial_buf.len() - self.buf.len();
            let header_bytes = &initial_buf[..header_len_before_fhcrc];
            let calculated_crc16 = header_crc16(header_bytes);

            let expected_crc16 = self.buf.get_u16_le();
            if expected_crc16 != calculated_crc16 {
                return Err(GzipError::HeaderCrcMismatch {
                    expected: expected_crc16,
                    calculated: calculated_crc16,
                });
            }

            optionals.header_crc = Some(expected_crc16);
        }
        Ok(optionals)
    }

    /// Reads a null-terminated (0x00) ISO-8859-1 string from `buf`.
    fn read_latin1_string(&mut self) -> GzipResult<String> {
        let null_pos = memchr(b'\0', &self.buf).ok_or(GzipError::UnexpectedEof {
            expected: 1,
            found: 0,
        })?;
        let bytes = self.buf.copy_to_bytes(null_pos);
        // consume the '\0' delimiter
        self.buf.advance(1);
        Ok(bytes.iter().map(|&b| b as char).collect())
    }
}

impl Parser<HeaderParsed> {
    pub fn decompress(mut self) -> GzipResult<Parser<Decompressed>> {
        let header = self.state.header;

        let mut out = BytesMut::with_capacity(64 * 1024);

        decode_deflate_to_bytesmut(&mut self.buf, &mut out)?;

        let uncompressed_data = out.freeze();

        let calculated_crc32 = crc32(&uncompressed_data);

        Ok(Parser {
            buf: self.buf,
            state: Decompressed {
                header,
                uncompressed_data,
                calculated_crc32,
            },
        })
    }
}

pub fn decode_deflate_to_bytesmut(input: &mut Bytes, output: &mut BytesMut) -> GzipResult<()> {
    let decompressed = decompress_to_vec(input.as_ref()).map_err(|e| {
        GzipError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Deflate error: {}", e),
        ))
    })?;
    output.extend_from_slice(&decompressed);
    Ok(())
}

#[cfg(test)]
#[path = "tests/parser_tests.rs"]
mod parser_tests;
