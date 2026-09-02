use crate::GZIP_CRC;

/// Compute standard CRC-32
#[inline]
pub fn crc32(data: &[u8]) -> u32 {
    GZIP_CRC.checksum(data)
}

/// Compute Header CRC-16 (least-significant 16 bits of CRC-32)
#[inline]
pub fn header_crc16(header_bytes: &[u8]) -> u16 {
    (GZIP_CRC.checksum(header_bytes) & 0xFFFF) as u16
}
