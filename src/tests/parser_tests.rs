use crate::error::GzipError;
use crate::helper::header_crc16;
use crate::{CompressionMethod, Decoder, ExtraFlags, Flags, OperatingSystem};
use flate2::{Compression, write::GzEncoder};
use std::io::{BufReader, Cursor, Write};

fn decoder(raw: Vec<u8>) -> Decoder<BufReader<Cursor<Vec<u8>>>, crate::parser::Start> {
    Decoder::new(BufReader::new(Cursor::new(raw)))
}
fn make_gzip_bytes(payload: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(payload)
        .expect("gzip payload should encode");
    encoder.finish().expect("gzip encoder should finish")
}

#[test]
fn test_parse_valid_header_with_fname() {
    let raw = vec![
        0x1F, 0x8B, 0x08, 0x08, 0x70, 0xF0, 0x95, 0x6A, 0x02, 0x03, b't', b'e', b's', b't', b'.',
        b't', b'x', b't', 0x00,
    ];

    let parsed = decoder(raw)
        .parse_header()
        .expect("should parse header")
        .read_optionals()
        .expect("should parse optional fields");

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
    assert_eq!(parsed.state.optionals.name.as_deref(), Some("test.txt"));
}

#[test]
fn test_minimal_valid_header_no_optionals() {
    let raw = vec![0x1F, 0x8B, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF];

    let parsed = decoder(raw)
        .parse_header()
        .expect("should parse minimal header")
        .read_optionals()
        .expect("should parse optional fields");

    assert_eq!(
        parsed.state.header.compression_method,
        CompressionMethod::Deflate
    );
    assert_eq!(parsed.state.header.flags, Flags::empty());
    assert_eq!(parsed.state.header.modification_time, 0);
    assert_eq!(parsed.state.header.os, OperatingSystem::Unknown);
    assert_eq!(parsed.state.optionals, Default::default());
}

#[test]
fn test_header_with_fextra() {
    let mut raw = vec![
        0x1F, 0x8B, 0x08, 0x04, 0x01, 0x02, 0x03, 0x04, 0x04, 0x03, 0x06, 0x00, b'A', b'P', 0x02,
        0x00, b'o', b'k',
    ];
    raw.extend_from_slice(b"compressed_payload");

    let parsed = decoder(raw)
        .parse_header()
        .expect("should parse header")
        .read_optionals()
        .expect("should parse FEXTRA");

    assert_eq!(parsed.state.header.flags, Flags::FEXTRA);
    assert_eq!(
        parsed.state.header.extra_flags,
        ExtraFlags::FastestAlgorithm
    );
    assert_eq!(
        parsed.state.optionals.extra,
        Some(vec![b'A', b'P', 0x02, 0x00, b'o', b'k'])
    );
}

#[test]
fn test_header_with_fcomment() {
    let mut raw = vec![0x1F, 0x8B, 0x08, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    raw.extend_from_slice(b"This is a gzip comment\0payload_bytes");

    let parsed = decoder(raw)
        .parse_header()
        .expect("should parse header")
        .read_optionals()
        .expect("should parse FCOMMENT");

    assert_eq!(parsed.state.header.flags, Flags::FCOMMENT);
    assert_eq!(
        parsed.state.optionals.comment.as_deref(),
        Some("This is a gzip comment")
    );
}

#[test]
fn test_header_with_valid_fhcrc() {
    let header_prefix = [0x1F, 0x8B, 0x08, 0x02, 0x12, 0x34, 0x56, 0x78, 0x02, 0x03];
    let calculated_crc16 = header_crc16(&header_prefix);
    let mut raw = header_prefix.to_vec();
    raw.extend_from_slice(&calculated_crc16.to_le_bytes());

    let parsed = decoder(raw)
        .parse_header()
        .expect("should parse header")
        .read_optionals()
        .expect("should validate FHCRC successfully");

    assert_eq!(parsed.state.optionals.header_crc, Some(calculated_crc16));
}

#[test]
fn test_header_with_all_optional_flags_combined() {
    let mut header_bytes = vec![
        0x1F, 0x8B, 0x08, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x04, 0x00, b'T', b'E', 0x00,
        0x00,
    ];
    header_bytes.extend_from_slice(b"archive.tar\0created by test\0");
    let expected_crc16 = header_crc16(&header_bytes);
    header_bytes.extend_from_slice(&expected_crc16.to_le_bytes());

    let parsed = decoder(header_bytes)
        .parse_header()
        .expect("should parse header")
        .read_optionals()
        .expect("should parse all optionals");

    assert_eq!(parsed.state.optionals.extra, Some(vec![b'T', b'E', 0, 0]));
    assert_eq!(parsed.state.optionals.name.as_deref(), Some("archive.tar"));
    assert_eq!(
        parsed.state.optionals.comment.as_deref(),
        Some("created by test")
    );
    assert_eq!(parsed.state.optionals.header_crc, Some(expected_crc16));
}

#[test]
fn test_parse_header_on_real_gzip_stream() {
    let compressed = make_gzip_bytes(b"hello from a real gzip stream\nhello again\n");
    let parsed = decoder(compressed)
        .parse_header()
        .expect("should parse gzip header")
        .read_optionals()
        .expect("should parse optional fields");

    assert_eq!(
        parsed.state.header.compression_method,
        CompressionMethod::Deflate
    );
    assert_eq!(parsed.state.header.flags, Flags::empty());
    assert_eq!(parsed.state.optionals, Default::default());
}

#[test]
fn test_error_truncated_fixed_header() {
    let result = decoder(vec![0x1F, 0x8B, 0x08]).parse_header();

    assert!(matches!(result, Err(GzipError::UnexpectedEof { .. })));
}

#[test]
fn test_error_invalid_magic_bytes() {
    let raw = vec![0x1F, 0x99, 0x08, 0x00, 0, 0, 0, 0, 0, 3];
    let result = decoder(raw).parse_header();

    assert!(matches!(result, Err(GzipError::InvalidMagic(0x1F, 0x99))));
}

#[test]
fn test_error_unsupported_compression_method() {
    let raw = vec![0x1F, 0x8B, 0x07, 0x00, 0, 0, 0, 0, 0, 3];
    let result = decoder(raw).parse_header();

    assert!(matches!(result, Err(GzipError::UnsupportedMethod(7))));
}

#[test]
fn test_error_reserved_flags_must_be_zero() {
    let raw = vec![0x1F, 0x8B, 0x08, 0x20, 0, 0, 0, 0, 0, 3];
    let result = decoder(raw).parse_header();

    assert!(matches!(result, Err(GzipError::ReservedFlags(0x20))));
}

#[test]
fn test_error_fhcrc_mismatch() {
    let raw = vec![0x1F, 0x8B, 0x08, 0x02, 0, 0, 0, 0, 0, 3, 0xDE, 0xAD];
    let result = decoder(raw)
        .parse_header()
        .expect("should parse fixed header")
        .read_optionals();

    assert!(matches!(result, Err(GzipError::HeaderCrcMismatch { .. })));
}

#[test]
fn test_error_unterminated_filename_string() {
    let raw = vec![
        0x1F, 0x8B, 0x08, 0x08, 0, 0, 0, 0, 0, 3, b'n', b'o', b'_', b'n', b'u', b'l', b'l',
    ];
    let result = decoder(raw)
        .parse_header()
        .expect("should parse fixed header")
        .read_optionals();

    assert!(matches!(result, Err(GzipError::UnexpectedEof { .. })));
}

#[test]
fn test_error_truncated_extra_field_payload() {
    let raw = vec![
        0x1F, 0x8B, 0x08, 0x04, 0, 0, 0, 0, 0, 3, 0x10, 0x00, 0x01, 0x02, 0x03,
    ];
    let result = decoder(raw)
        .parse_header()
        .expect("should parse fixed header")
        .read_optionals();

    assert!(matches!(
        result,
        Err(GzipError::Io(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof
    ));
}
