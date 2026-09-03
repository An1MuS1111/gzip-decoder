use crate::error::{GzipError, GzipResult};
use crate::{
    CompressionMethod, ExtraFlags, Flags, GZIP_CRC, Header, ID1, ID2, OperatingSystem, Optionals,
};
use byteorder::{ByteOrder, LittleEndian};
use crc::Digest;
use memchr::memchr;
use static_str_ops::staticize;
use std::io::BufRead;

// pub struct Parser<S> {
//     pub buf: Bytes,
//     pub state: S,
// }

pub struct Decoder<R, S> {
    pub reader: R,
    pub state: S,
    pub buf: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Start /* state_1 */;

#[derive(Clone)]
pub struct HeaderParsed /* state_2 */ {
    pub header: Header,
    pub fhcrc: Digest<'static, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalParsed /* state_3 */ {
    pub header: Header,
    pub optionals: Optionals,
}

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct Decompressed {
//     pub header: Header,
//     pub uncompressed_data: Bytes,
//     pub calculated_crc32: u32,
// }

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct Finished {
//     pub header: Header,
//     pub trailer: Trailer,
//     pub uncompressed_data: Vec<u8>,
// }

impl<R: BufRead> Decoder<R, Start> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            state: Start,
            buf: Vec::with_capacity(256),
        }
    }

    pub fn parse_header(mut self) -> GzipResult<Decoder<R, HeaderParsed>> {
        let mut fhcrc = GZIP_CRC.digest();

        let mut fixed_header = [0u8; 10];
        self.reader
            .read_exact(&mut fixed_header)
            .map_err(|e| GzipError::UnexpectedEof {
                err: staticize(format!("failed to read header: {}", e)),
            })?;
        fhcrc.update(&fixed_header);

        // verify the identification bytes ID1 & ID2
        if fixed_header[0] != ID1 || fixed_header[1] != ID2 {
            return Err(GzipError::InvalidMagic(fixed_header[0], fixed_header[1]));
        }

        // verify the compression method
        let compression_method = match fixed_header[2] {
            8 => CompressionMethod::Deflate,
            other => return Err(GzipError::UnsupportedMethod(other)),
        };

        // parse the flags
        let flags = match fixed_header[3] {
            raw_flags if raw_flags & 0xE0 != 0 => return Err(GzipError::ReservedFlags(raw_flags)),
            raw_flags => Flags::from_bits_truncate(raw_flags),
        };

        // parse the modification time u32 (4 bytes)
        let modification_time = LittleEndian::read_u32(&fixed_header[4..8]);

        // parse extra flags
        let extra_flags = ExtraFlags::from(fixed_header[8]);

        // parse operating system
        let operating_system = OperatingSystem::from(fixed_header[9]);

        Ok(Decoder {
            reader: self.reader,
            buf: self.buf,
            state: HeaderParsed {
                header: Header {
                    compression_method,
                    flags,
                    modification_time,
                    extra_flags,
                    os: operating_system,
                },
                fhcrc,
            },
        })
    }
}

impl<R: BufRead> Decoder<R, HeaderParsed> {
    pub fn read_optionals(mut self) -> GzipResult<Decoder<R, OptionalParsed>> {
        let mut optionals = Optionals::default();

        let flags = self.state.header.flags;

        // FEXTRA: Extra field
        if flags.contains(Flags::FEXTRA) {
            optionals.extra = self.read_fhcrc()?;
        }

        // FNAME: Original filename
        if flags.contains(Flags::FNAME) {
            let name = self.read_null_terminated()?;
            optionals.name = Some(name);
        }

        // FCOMMENT: File comment
        if flags.contains(Flags::FCOMMENT) {
            let comment = self.read_null_terminated()?;
            optionals.comment = Some(comment);
        }

        // FHCRC: Header CRC-16
        if flags.contains(Flags::FHCRC) {
            optionals.header_crc = self.verify_headercrc16()?;
        }

        Ok(Decoder {
            reader: self.reader,
            state: OptionalParsed {
                header: self.state.header,
                optionals,
            },
            buf: self.buf,
        })
    }

    #[inline]
    fn read_fhcrc(&mut self) -> GzipResult<Option<Vec<u8>>> {
        let mut xlen_buf = [0u8; 2];
        self.reader.read_exact(&mut xlen_buf)?;
        self.state.fhcrc.update(&xlen_buf);

        let xlen = LittleEndian::read_u16(&xlen_buf) as usize;
        self.buf.resize(xlen, 0);
        self.reader.read_exact(&mut self.buf)?;
        self.state.fhcrc.update(&self.buf);
        Ok(Some(self.buf.clone()))
    }

    #[inline]
    fn verify_headercrc16(&mut self) -> GzipResult<Option<u16>> {
        let mut fhcrc_buf = [0u8; 2];
        self.reader.read_exact(&mut fhcrc_buf)?;

        let expected = u16::from_le_bytes(fhcrc_buf);
        let calculated = (self.state.fhcrc.clone().finalize() & 0xFFFF) as u16;

        if expected != calculated {
            return Err(GzipError::HeaderCrcMismatch {
                expected,
                calculated,
            });
        }

        Ok(Some(calculated))
    }

    // reads the latin1 string from the buffer
    fn read_null_terminated(&mut self) -> GzipResult<String> {
        self.buf.clear();
        loop {
            // Peek at buffer
            let buf = self.reader.fill_buf()?;
            if buf.is_empty() {
                return Err(GzipError::UnexpectedEof {
                    err: "null terminator",
                });
            }

            if let Some(pos) = memchr(b'\0', buf) {
                // Wire bytes: [string_tail][\0]
                self.state.fhcrc.update(&buf[..pos]);
                // updating the null terminator in the digest
                self.state.fhcrc.update(b"\0");
                self.buf.extend_from_slice(&buf[..pos]);
                // Advance past '\0'
                self.reader.consume(pos + 1);
                break;
            } else {
                // Wire bytes: [string_chunk]
                self.state.fhcrc.update(buf);
                self.buf.extend_from_slice(buf);
                let len = buf.len();
                self.reader.consume(len);
            }
        }

        Ok(self.buf.iter().map(|&b| b as char).collect())
    }
}

#[cfg(test)]
#[path = "tests/parser_tests.rs"]
mod parser_tests;
