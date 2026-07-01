//! Shared symbol-range machinery used by both the huffman "new" code-length reader and the
//! tANS table reader: `read_fluff` and `convert_to_ranges`.

#![allow(dead_code)]

use super::bitio::{bsr32, BitReader};
use crate::{Error, Result};

/// A decoded (symbol, count) range.
#[derive(Clone, Copy, Default)]
pub(crate) struct HuffRange {
    pub(crate) symbol: u16,
    pub(crate) num: u16,
}

/// A bias value used by the "new" length encoding and the tANS
/// table. Consumes bits from the *no-refill* accumulator (caller refills beforehand).
pub(crate) fn read_fluff(bits: &mut BitReader, num_symbols: i32) -> i32 {
    if num_symbols == 256 {
        return 0;
    }
    let mut x = 257 - num_symbols;
    if x > num_symbols {
        x = num_symbols;
    }
    x *= 2;
    let y = bsr32((x - 1) as u32) + 1;
    let v = bits.bits >> (32 - y);
    let z = (1u32 << y) - x as u32;
    if (v >> 1) >= z {
        bits.bits <<= y;
        bits.bitpos += y as i32;
        (v - z) as i32
    } else {
        bits.bits <<= y - 1;
        bits.bitpos += (y - 1) as i32;
        (v >> 1) as i32
    }
}

/// Turn transmitted run/space pairs into `(symbol, num)` ranges.
/// `symlen` holds the per-range bit widths (the bytes following the code lengths). Returns the
/// number of ranges written into `range`.
pub(crate) fn convert_to_ranges(
    range: &mut [HuffRange],
    num_symbols: i32,
    p: i32,
    symlen: &[u8],
    bits: &mut BitReader,
) -> Result<usize> {
    let num_ranges = (p >> 1) as usize;
    let mut sym_idx: i32 = 0;
    let mut si = 0usize;

    if p & 1 != 0 {
        bits.refill();
        let v = symlen[si] as i32;
        si += 1;
        if v >= 8 {
            return Err(Error::Corrupt("range: space too big"));
        }
        sym_idx = bits.read_bits_no_refill((v + 1) as u32) as i32 + (1 << (v + 1)) - 1;
    }

    let mut syms_used: i32 = 0;
    for r in range.iter_mut().take(num_ranges) {
        bits.refill();
        let v0 = symlen[si] as i32;
        if v0 >= 9 {
            return Err(Error::Corrupt("range: num too big"));
        }
        let num = bits.read_bits_no_refill_zero(v0 as u32) as i32 + (1 << v0);
        let v1 = symlen[si + 1] as i32;
        if v1 >= 8 {
            return Err(Error::Corrupt("range: space too big"));
        }
        let space = bits.read_bits_no_refill((v1 + 1) as u32) as i32 + (1 << (v1 + 1)) - 1;
        r.symbol = sym_idx as u16;
        r.num = num as u16;
        syms_used += num;
        sym_idx += num + space;
        si += 2;
    }

    if sym_idx >= 256 || syms_used >= num_symbols || sym_idx + num_symbols - syms_used > 256 {
        return Err(Error::Corrupt("range: overflow"));
    }
    range[num_ranges].symbol = sym_idx as u16;
    range[num_ranges].num = (num_symbols - syms_used) as u16;
    Ok(num_ranges + 1)
}
