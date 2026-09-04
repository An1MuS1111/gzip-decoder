// The max window size of the buffer is 32KB or 32,768 bytes
pub const MAX_WINDOW_SIZE: usize = 32 * 1024; // also known as the LOOK_AHEAD_BUFFER_SIZE

// SEARCH_BUFFER
pub const MAX_MATCH_LENGTH: usize = 258;
pub const MIN_MATCH_LENGTH: usize = 3;

pub enum Token<T> {
    Literal(T),
    Match { length: u16, offset: u16 },
}

#[derive(Default, Debug)]
pub struct Lz77 {}

impl Lz77 {
    fn new() -> Self {
        Self::with_window(MAX_WINDOW_SIZE)
    }

    fn with_window(window: usize) -> Self {
        todo!()
    }

    fn encode(buf: &[u8], window: usize) -> Vec<u8> {
        todo!()
    }
}

const HASH_BITS: usize = 15;
const HASH_SIZE: usize = 1 << HASH_BITS; // 32,768
const HASH_SHIFT: usize = 32 - HASH_BITS; // 17
const KNUTH_GOLDEN_RATIO: u32 = 0x9E3779B9;

/// Fast, branchless 3-byte multiplicative hash for DEFLATE
/// Safety: data must have at least 4 readable bytes starting at pos
#[inline(always)]
pub fn hash3_fast(buf: &[u8], pos: usize) -> usize {
    // Single 32-bit unaligned read, masked to 24 bits
    let ptr = unsafe { buf.as_ptr().add(pos) as *const u32 };
    let packed = unsafe { u32::from_le(ptr.read_unaligned()) & 0x00FF_FFFF };
    // Multiplicative hash retaining highest entropy bits
    ((packed.wrapping_mul(KNUTH_GOLDEN_RATIO)) >> HASH_SHIFT) as usize
}

#[inline(always)]
pub fn hash3(buf: &[u8], pos: usize) -> usize {
    if (pos + 4) <= buf.len() {
        return hash3_fast(buf, pos);
    }
    // if only 3 bytes left
    debug_assert_eq!(buf.len() - pos, MIN_MATCH_LENGTH);
    let packed = (buf[pos] as u32) | ((buf[pos + 1] as u32) << 8) | ((buf[pos + 2] as u32) << 16);
    ((packed.wrapping_mul(KNUTH_GOLDEN_RATIO)) >> HASH_SHIFT) as usize
}
