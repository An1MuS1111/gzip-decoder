use super::*;
use crate::error::GzipError;
use crate::{CompressionMethod, ExtraFlags, Flags, OperatingSystem, header_crc16};
use bytes::Bytes;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;

fn make_gzip_bytes(payload: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(payload).expect("gzip payload should encode");
    encoder.finish().expect("gzip encoder should finish")
}

#[test]
fn test_parse_valid_header_with_fname() {
    let raw = [
        0x1F, 0x8B, // ID1, ID2
        0x08, // CM = Deflate
        0x08, // FLG = FNAME
        0x70, 0xF0, 0x95, 0x6A, // MTIME
        0x02, // XFL = MaximumCompression
        0x03, // OS = Unix
        b't', b'e', b's', b't', b'.', b't', b'x', b't', 0x00, // FNAME
    ];

    let parser = Parser::new(Bytes::copy_from_slice(&raw));
    let parsed = parser.parse_header().expect("should parse header");
    assert_eq!(
        parsed.state.header.compression_method,
        CompressionMethod::Deflate
    );
    assert_eq!(parsed.state.header.flags, Flags::FNAME);
    assert_eq!(
        parsed.state.header.extra_flags,
        ExtraFlags::MaximumCompression
    );
    assert_eq!(parsed.state.header.os, OperatingSystem::Unix);
    assert_eq!(
        parsed.state.header.optionals.name.as_deref(),
        Some("test.txt")
    );
    assert_eq!(parsed.buf.len(), 0);
}

#[test]
fn test_minimal_valid_header_no_optionals() {
    let raw = [
        0x1F, 0x8B, // ID1, ID2
        0x08, // CM = Deflate
        0x00, // FLG = No flags
        0x00, 0x00, 0x00, 0x00, // MTIME = 0 (no timestamp)
        0x00, // XFL = Other(0)
        0xFF, // OS = Unknown (255)
    ];

    let parser = Parser::new(Bytes::copy_from_slice(&raw));
    let parsed = parser.parse_header().expect("should parse minimal header");

    assert_eq!(
        parsed.state.header.compression_method,
        CompressionMethod::Deflate
    );
    assert_eq!(parsed.state.header.flags, Flags::empty());
    assert_eq!(parsed.state.header.modification_time, 0);
    assert_eq!(parsed.state.header.os, OperatingSystem::Unknown);
    assert_eq!(parsed.state.header.optionals.extra, None);
    assert_eq!(parsed.state.header.optionals.name, None);
    assert_eq!(parsed.state.header.optionals.comment, None);
    assert_eq!(parsed.state.header.optionals.header_crc, None);
    assert_eq!(parsed.buf.len(), 0);
}

#[test]
fn test_header_with_fextra() {
    let mut raw = vec![
        0x1F, 0x8B, // ID1, ID2
        0x08, // CM = Deflate
        0x04, // FLG = FEXTRA (bit 2)
        0x01, 0x02, 0x03, 0x04, // MTIME
        0x04, // XFL = FastestAlgorithm
        0x03, // OS = Unix
        0x06, 0x00, // XLEN = 6 bytes (LE)
        b'A', b'P', 0x02, 0x00, b'o', b'k', // Extra data
    ];
    raw.extend_from_slice(b"compressed_payload");

    let parser = Parser::new(Bytes::from(raw));
    let parsed = parser.parse_header().expect("should parse FEXTRA header");

    assert_eq!(parsed.state.header.flags, Flags::FEXTRA);
    assert_eq!(
        parsed.state.header.extra_flags,
        ExtraFlags::FastestAlgorithm
    );
    assert_eq!(
        parsed.state.header.optionals.extra,
        Some(vec![b'A', b'P', 0x02, 0x00, b'o', b'k'])
    );
    assert_eq!(parsed.buf, Bytes::from_static(b"compressed_payload"));
}

#[test]
fn test_header_with_fcomment() {
    let mut raw = vec![
        0x1F, 0x8B, 0x08, 0x10, // FLG = FCOMMENT (bit 4)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // OS = FAT
    ];
    raw.extend_from_slice(b"This is a gzip comment\0");
    raw.extend_from_slice(b"payload_bytes");

    let parser = Parser::new(Bytes::from(raw));
    let parsed = parser.parse_header().expect("should parse FCOMMENT");

    assert_eq!(parsed.state.header.flags, Flags::FCOMMENT);
    assert_eq!(
        parsed.state.header.optionals.comment.as_deref(),
        Some("This is a gzip comment")
    );
    assert_eq!(parsed.buf, Bytes::from_static(b"payload_bytes"));
}

#[test]
fn test_header_with_valid_fhcrc() {
    let header_prefix = [
        0x1F, 0x8B, 0x08, 0x02, // FLG = FHCRC (bit 1)
        0x12, 0x34, 0x56, 0x78, 0x02, // XFL = MaximumCompression
        0x03, // OS = Unix
    ];

    // Compute valid CRC16 over the 10 header bytes
    let calculated_crc16 = header_crc16(&header_prefix);

    let mut raw = header_prefix.to_vec();
    raw.extend_from_slice(&calculated_crc16.to_le_bytes()); // FHCRC (2 bytes LE)
    raw.extend_from_slice(b"deflate_data");

    let parser = Parser::new(Bytes::from(raw));
    let parsed = parser
        .parse_header()
        .expect("should validate FHCRC successfully");

    assert_eq!(parsed.state.header.flags, Flags::FHCRC);
    assert_eq!(
        parsed.state.header.optionals.header_crc,
        Some(calculated_crc16)
    );
    assert_eq!(parsed.buf, Bytes::from_static(b"deflate_data"));
}

#[test]
fn test_header_with_all_optional_flags_combined() {
    // Build a header containing FEXTRA + FNAME + FCOMMENT + FHCRC
    let mut header_bytes = vec![
        0x1F, 0x8B, 0x08,
        0x1E, // FLG = FHCRC | FEXTRA | FNAME | FCOMMENT (0x02 | 0x04 | 0x08 | 0x10)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // OS = Unix
        0x04, 0x00, b'T', b'E', 0x00, 0x00, // XLEN=4 + extra subfield
    ];
    header_bytes.extend_from_slice(b"archive.tar\0"); // FNAME
    header_bytes.extend_from_slice(b"created by test\0"); // FCOMMENT

    // CRC16 covers everything up to FHCRC
    let expected_crc16 = header_crc16(&header_bytes);
    header_bytes.extend_from_slice(&expected_crc16.to_le_bytes());

    let parser = Parser::new(Bytes::from(header_bytes));
    let parsed = parser.parse_header().expect("should parse all optionals");

    assert_eq!(
        parsed.state.header.optionals.extra,
        Some(vec![b'T', b'E', 0x00, 0x00])
    );
    assert_eq!(
        parsed.state.header.optionals.name.as_deref(),
        Some("archive.tar")
    );
    assert_eq!(
        parsed.state.header.optionals.comment.as_deref(),
        Some("created by test")
    );
    assert_eq!(
        parsed.state.header.optionals.header_crc,
        Some(expected_crc16)
    );
}

#[test]
fn test_parse_header_on_real_gzip_stream() {
    let payload = b"hello from a real gzip stream\nhello again\n";
    let compressed = make_gzip_bytes(payload);

    let parsed = Parser::new(Bytes::from(compressed)).parse_header().unwrap();

    assert_eq!(
        parsed.state.header.compression_method,
        CompressionMethod::Deflate
    );
    assert_eq!(parsed.state.header.flags, Flags::empty());
    assert_eq!(parsed.state.header.modification_time, 0);
    assert_eq!(parsed.state.header.optionals.name, None);
    assert!(parsed.buf.len() > 0);
}

#[test]
#[ignore = "decompress() is not implemented yet"]
fn test_decompress_roundtrip_real_gzip_stream() {
    let payload = b"The quick brown fox jumps over the lazy dog";
    let compressed = make_gzip_bytes(payload);

    let header = Parser::new(Bytes::from(compressed))
        .parse_header()
        .expect("valid gzip bytes should parse");
    let decompressed = header.decompress().expect("gzip payload should inflate");

    assert_eq!(decompressed.state.uncompressed_data.as_ref(), payload);
    assert_eq!(decompressed.state.calculated_crc32, crate::crc32(payload));
}

#[test]
#[ignore = "decompress() is not implemented yet"]
fn test_decompress_empty_payload_gzip_stream() {
    let compressed = make_gzip_bytes(b"");

    let header = Parser::new(Bytes::from(compressed))
        .parse_header()
        .expect("valid empty gzip payload should parse");
    let decompressed = header.decompress().expect("empty payload should inflate");

    assert_eq!(decompressed.state.uncompressed_data.as_ref(), b"");
    assert_eq!(decompressed.state.calculated_crc32, crate::crc32(b""));
}

#[test]
fn test_error_truncated_fixed_header() {
    let short_header = [0x1F, 0x8B, 0x08]; // Only 3 bytes instead of 10
    let parser = Parser::new(Bytes::copy_from_slice(&short_header));
    let result = parser.parse_header();

    assert!(matches!(
        result,
        Err(GzipError::UnexpectedEof {
            expected: 10,
            found: 3
        })
    ));
}

#[test]
fn test_error_invalid_magic_bytes() {
    let invalid_magic = [
        0x1F, 0x99, // 0x99 instead of 0x8B
        0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
    ];
    let parser = Parser::new(Bytes::copy_from_slice(&invalid_magic));
    let result = parser.parse_header();

    assert!(matches!(result, Err(GzipError::InvalidMagic(0x1F, 0x99))));
}

#[test]
fn test_error_unsupported_compression_method() {
    let invalid_cm = [
        0x1F, 0x8B, 0x07, // CM = 7 (Reserved/Unsupported)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
    ];
    let parser = Parser::new(Bytes::copy_from_slice(&invalid_cm));
    let result = parser.parse_header();

    assert!(matches!(result, Err(GzipError::UnsupportedMethod(7))));
}

#[test]
fn test_error_reserved_flags_must_be_zero() {
    let invalid_flags = [
        0x1F, 0x8B, 0x08, 0x20, // Bit 5 is set (0b0010_0000) -> Reserved!
        0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
    ];
    let parser = Parser::new(Bytes::copy_from_slice(&invalid_flags));
    let result = parser.parse_header();

    assert!(matches!(result, Err(GzipError::ReservedFlags(0x20))));
}

#[test]
fn test_error_fhcrc_mismatch() {
    let raw = vec![
        0x1F, 0x8B, 0x08, 0x02, // FLG = FHCRC
        0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xDE, 0xAD, // Corrupted CRC16 bytes
    ];

    let parser = Parser::new(Bytes::from(raw));
    let result = parser.parse_header();

    assert!(matches!(result, Err(GzipError::HeaderCrcMismatch { .. })));
}

#[test]
fn test_error_unterminated_filename_string() {
    let raw = vec![
        0x1F, 0x8B, 0x08, 0x08, // FLG = FNAME
        0x00, 0x00, 0x00, 0x00, 0x00, 0x03, b'n', b'o', b'_', b'n', b'u', b'l',
        b'l', // No 0x00 terminator!
    ];

    let parser = Parser::new(Bytes::from(raw));
    let result = parser.parse_header();

    assert!(matches!(result, Err(GzipError::UnexpectedEof { .. })));
}

#[test]
fn test_error_truncated_extra_field_payload() {
    let raw = vec![
        0x1F, 0x8B, 0x08, 0x04, // FLG = FEXTRA
        0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x10, 0x00, // XLEN = 16 bytes
        0x01, 0x02, 0x03, // Only 3 bytes provided!
    ];

    let parser = Parser::new(Bytes::from(raw));
    let result = parser.parse_header();

    assert!(matches!(
        result,
        Err(GzipError::UnexpectedEof {
            expected: 16,
            found: 3
        })
    ));
}
