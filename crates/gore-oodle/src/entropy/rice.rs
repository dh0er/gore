//! Golomb-Rice length/bit decoders shared by huffman-"new" and the tANS table decode.
//!
//! A separate byte-oriented bit cursor ([`Rice2`]) walks the *same* source buffer the main
//! `BitReader` uses, so after the rice phase the caller resumes the main reader from
//! `br2.p`/`br2.bitpos`. We track that cursor as a slice index.

#![allow(dead_code)]

use super::bitio::bsf32;
use crate::{Error, Result};

/// A byte cursor with an MSB-measured bit offset into the source buffer.
pub(crate) struct Rice2 {
    /// Index of the current byte.
    pub(crate) p: usize,
    /// One-past-the-end index.
    pub(crate) p_end: usize,
    /// Bits already consumed in the current byte, from the MSB.
    pub(crate) bitpos: u32,
}

/// For a byte of unary data, packs the per-bit decoded run lengths.
static RICE_VALUE: [u32; 256] = [
    0x80000000, 0x00000007, 0x10000006, 0x00000006, 0x20000005, 0x00000105, 0x10000005,
    0x00000005, 0x30000004, 0x00000204, 0x10000104, 0x00000104, 0x20000004, 0x00010004,
    0x10000004, 0x00000004, 0x40000003, 0x00000303, 0x10000203, 0x00000203, 0x20000103,
    0x00010103, 0x10000103, 0x00000103, 0x30000003, 0x00020003, 0x10010003, 0x00010003,
    0x20000003, 0x01000003, 0x10000003, 0x00000003, 0x40000002, 0x00000402, 0x10000302,
    0x00000302, 0x20000202, 0x00010202, 0x10000202, 0x00000202, 0x30000102, 0x00020102,
    0x10010102, 0x00010102, 0x20000102, 0x01000102, 0x10000102, 0x00000102, 0x40000002,
    0x00030002, 0x10020002, 0x00020002, 0x20010002, 0x01010002, 0x10010002, 0x00010002,
    0x30000002, 0x02000002, 0x11000002, 0x01000002, 0x20000002, 0x00000012, 0x10000002,
    0x00000002, 0x60000001, 0x00000501, 0x10000401, 0x00000401, 0x20000301, 0x00010301,
    0x10000301, 0x00000301, 0x30000201, 0x00020201, 0x10010201, 0x00010201, 0x20000201,
    0x01000201, 0x10000201, 0x00000201, 0x40000101, 0x00030101, 0x10020101, 0x00020101,
    0x20010101, 0x01010101, 0x10010101, 0x00010101, 0x30000101, 0x02000101, 0x11000101,
    0x01000101, 0x20000101, 0x00000111, 0x10000101, 0x00000101, 0x50000001, 0x00040001,
    0x10030001, 0x00030001, 0x20020001, 0x01020001, 0x10020001, 0x00020001, 0x30010001,
    0x02010001, 0x11010001, 0x01010001, 0x20010001, 0x00010011, 0x10010001, 0x00010001,
    0x40000001, 0x03000001, 0x12000001, 0x02000001, 0x21000001, 0x01000011, 0x11000001,
    0x01000001, 0x30000001, 0x00000021, 0x10000011, 0x00000011, 0x20000001, 0x00001001,
    0x10000001, 0x00000001, 0x70000000, 0x00000600, 0x10000500, 0x00000500, 0x20000400,
    0x00010400, 0x10000400, 0x00000400, 0x30000300, 0x00020300, 0x10010300, 0x00010300,
    0x20000300, 0x01000300, 0x10000300, 0x00000300, 0x40000200, 0x00030200, 0x10020200,
    0x00020200, 0x20010200, 0x01010200, 0x10010200, 0x00010200, 0x30000200, 0x02000200,
    0x11000200, 0x01000200, 0x20000200, 0x00000210, 0x10000200, 0x00000200, 0x50000100,
    0x00040100, 0x10030100, 0x00030100, 0x20020100, 0x01020100, 0x10020100, 0x00020100,
    0x30010100, 0x02010100, 0x11010100, 0x01010100, 0x20010100, 0x00010110, 0x10010100,
    0x00010100, 0x40000100, 0x03000100, 0x12000100, 0x02000100, 0x21000100, 0x01000110,
    0x11000100, 0x01000100, 0x30000100, 0x00000120, 0x10000110, 0x00000110, 0x20000100,
    0x00001100, 0x10000100, 0x00000100, 0x60000000, 0x00050000, 0x10040000, 0x00040000,
    0x20030000, 0x01030000, 0x10030000, 0x00030000, 0x30020000, 0x02020000, 0x11020000,
    0x01020000, 0x20020000, 0x00020010, 0x10020000, 0x00020000, 0x40010000, 0x03010000,
    0x12010000, 0x02010000, 0x21010000, 0x01010010, 0x11010000, 0x01010000, 0x30010000,
    0x00010020, 0x10010010, 0x00010010, 0x20010000, 0x00011000, 0x10010000, 0x00010000,
    0x50000000, 0x04000000, 0x13000000, 0x03000000, 0x22000000, 0x02000010, 0x12000000,
    0x02000000, 0x31000000, 0x01000020, 0x11000010, 0x01000010, 0x21000000, 0x01001000,
    0x11000000, 0x01000000, 0x40000000, 0x00000030, 0x10000020, 0x00000020, 0x20000010,
    0x00001010, 0x10000010, 0x00000010, 0x30000000, 0x00002000, 0x10001000, 0x00001000,
    0x20000000, 0x00100000, 0x10000000, 0x00000000,
];

/// Number of values terminated within a unary byte.
static RICE_LEN: [u8; 256] = [
    0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4, 1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4,
    5, 1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5,
    5, 6, 1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4,
    5, 5, 6, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6, 3, 4, 4, 5, 4, 5, 5, 6, 4, 5, 5, 6,
    5, 6, 6, 7, 1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4,
    5, 4, 5, 5, 6, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6, 3, 4, 4, 5, 4, 5, 5, 6, 4, 5,
    5, 6, 5, 6, 6, 7, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6, 3, 4, 4, 5, 4, 5, 5, 6, 4,
    5, 5, 6, 5, 6, 6, 7, 3, 4, 4, 5, 4, 5, 5, 6, 4, 5, 5, 6, 5, 6, 6, 7, 4, 5, 5, 6, 5, 6, 6, 7,
    5, 6, 6, 7, 6, 7, 7, 8,
];

/// Decode `size` unary-coded run lengths into `dst`.
///
/// `dst` must have at least 16 bytes of slack past `size` (the decoder writes 8 bytes per
/// step and may overrun before stepping back). Reads bytes forward from `br.p` in `src`.
pub(crate) fn decode_golomb_rice_lengths(dst: &mut [u8], src: &[u8], br: &mut Rice2) -> Result<()> {
    let size = dst.len();
    // Track a virtual dst index; we may write a few bytes past `size` into the slack buffer
    // the caller provides, but the slice given here is exactly `size`. To replicate the
    // overrun-then-stepback we operate on a small scratch window.
    let mut p = br.p;
    let p_end = br.p_end;
    if p >= p_end {
        return Err(Error::Truncated);
    }

    // Scratch with slack to mirror the 8-byte stores.
    let mut scratch = [0u8; 512 + 32];
    if size + 16 > scratch.len() {
        return Err(Error::Corrupt("rice: size too big"));
    }

    let mut di = 0usize; // dst write index into scratch
    let mut count: i32 = -(br.bitpos as i32);
    let mut v: u32 = (src[p] & (255u8 >> br.bitpos)) as u32;
    p += 1;

    loop {
        if v == 0 {
            count += 8;
        } else {
            let x = RICE_VALUE[v as usize];
            // `dst[0..4] = count + (x & 0x0f0f0f0f)` as a `u32`, i.e. modular (wrapping)
            // arithmetic; `count` may be negative. The result is laid out little-endian
            // (each nibble-byte carries `count`).
            let lo = (count as u32).wrapping_add(x & 0x0f0f_0f0f);
            let hi = (x >> 4) & 0x0f0f_0f0f;
            scratch[di] = lo as u8;
            scratch[di + 1] = (lo >> 8) as u8;
            scratch[di + 2] = (lo >> 16) as u8;
            scratch[di + 3] = (lo >> 24) as u8;
            scratch[di + 4] = hi as u8;
            scratch[di + 5] = (hi >> 8) as u8;
            scratch[di + 6] = (hi >> 16) as u8;
            scratch[di + 7] = (hi >> 24) as u8;
            di += RICE_LEN[v as usize] as usize;
            if di >= size {
                break;
            }
            count = (x >> 28) as i32;
        }
        if p >= p_end {
            return Err(Error::Truncated);
        }
        v = src[p] as u32;
        p += 1;
    }

    // Went too far? Step back by clearing the extra low set bits of v.
    if di > size {
        let n = di - size;
        for _ in 0..n {
            v &= v - 1;
        }
    }
    // Step back the cursor if the byte isn't finished.
    let mut bitpos = 0u32;
    if v & 1 == 0 {
        p -= 1;
        let q = bsf32(v);
        bitpos = 8 - q;
    }
    br.p = p;
    br.bitpos = bitpos;
    dst.copy_from_slice(&scratch[..size]);
    Ok(())
}

/// Append `bitcount` low bits (MSB-first) to each of `size`
/// values already in `dst`. Reads bits forward from `br.p`/`br.bitpos`.
pub(crate) fn decode_golomb_rice_bits(
    dst: &mut [u8],
    src: &[u8],
    bitcount: u32,
    br: &mut Rice2,
) -> Result<()> {
    if bitcount == 0 {
        return Ok(());
    }
    let size = dst.len();
    let mut p = br.p;
    let mut bitpos = br.bitpos;

    let bits_required = bitpos + bitcount * size as u32;
    let bytes_required = ((bits_required + 7) >> 3) as usize;
    if bytes_required > br.p_end - p {
        return Err(Error::Truncated);
    }
    br.p = p + (bits_required >> 3) as usize;
    br.bitpos = bits_required & 7;

    // Scalar MSB-first reader over the byte stream.
    #[inline]
    fn read_msb(src: &[u8], p: &mut usize, bitpos: &mut u32, n: u32) -> u32 {
        let mut acc: u32 = 0;
        for _ in 0..n {
            let byte = src[*p];
            let bit = (byte >> (7 - *bitpos)) & 1;
            acc = (acc << 1) | bit as u32;
            *bitpos += 1;
            if *bitpos == 8 {
                *bitpos = 0;
                *p += 1;
            }
        }
        acc
    }

    for d in dst.iter_mut().take(size) {
        let extra = read_msb(src, &mut p, &mut bitpos, bitcount);
        *d = ((*d as u32) << bitcount | extra) as u8;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rice_len_value_table_self_consistent() {
        // Each entry of RICE_LEN counts the set "terminator" bits implied by RICE_VALUE for
        // that byte; spot check a couple of known values.
        assert_eq!(RICE_LEN[0], 0);
        assert_eq!(RICE_LEN[255], 8);
        assert_eq!(RICE_VALUE[255], 0x00000000);
        assert_eq!(RICE_VALUE[0], 0x80000000);
    }
}
