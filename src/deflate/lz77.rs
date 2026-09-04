// The max window size of the buffer is 32KB or 32,768 bytes
pub const MAX_WINDOW_SIZE: usize = 32 * 1024; // also known as the LOOK_AHEAD_BUFFER_SIZE

// SEARCH_BUFFER
pub const MAX_MATCH_LENGTH: usize = 258;
pub const MIN_MATCH_LENGTH: usize = 3;

pub enum Compressor<T> {
    /* Literal: */
    Literal(T),
    Match { length: u16, offset: u16 },
}

pub struct Lz77 {}

impl Lz77 {
    fn new() -> Self {
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
pub fn hash3_fast(data: &[u8], pos: usize) -> usize {
    // Single 32-bit unaligned read, masked to 24 bits
    let ptr = unsafe { data.as_ptr().add(pos) as *const u32 };
    let packed = unsafe { u32::from_le(ptr.read_unaligned()) & 0x00FF_FFFF };

    // Multiplicative hash retaining highest entropy bits
    ((packed.wrapping_mul(KNUTH_GOLDEN_RATIO)) >> HASH_SHIFT) as usize
}
