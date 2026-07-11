//! Huffman array decode.
//!
//! An Oodle huffman array is: a code-length table (transmitted "old" or "new" form) followed
//! by the entropy-coded data. The data is decoded with a 2048-entry reverse LUT and a
//! triple-stream interleaved reader (a forward stream, a backward stream, and a "mid"
//! forward stream) so three symbols are produced per loop iteration. Two layouts exist:
//!   * type 1 (`chunk_type == 2`): one output region, three streams (`src`, `src_mid`, `src_end`).
//!   * type 2 (`chunk_type == 4`): the output is split in half; each half is its own triple.
//!
//! `decode_huff` receives the array *payload* (everything after the size header), the payload
//! length, the output buffer, and `huff_type` (`chunk_type >> 1`, i.e. 1 or 2). It returns the
//! number of payload bytes consumed (which the caller checks equals the payload length).

#![allow(dead_code)]

use super::bitio::{clz32, BitReader};
use super::hufftable::{convert_to_ranges, read_fluff, HuffRange};
use super::rice::{decode_golomb_rice_bits, decode_golomb_rice_lengths, Rice2};
use crate::tables::REVERSE_BITS;
use crate::{Error, Result};

/// The running first-code index per code length 1..=11.
const CODE_PREFIX_ORG: [u32; 12] = [
    0x0, 0x0, 0x2, 0x6, 0xE, 0x1E, 0x3E, 0x7E, 0xFE, 0x1FE, 0x2FE, 0x3FE,
];

/// The 2048-entry forward huffman LUT, code-bits → (len, sym).
struct HuffLut {
    bits2len: [u8; 2048],
    bits2sym: [u8; 2048],
}

impl HuffLut {
    fn zeroed() -> Self {
        HuffLut {
            bits2len: [0u8; 2048],
            bits2sym: [0u8; 2048],
        }
    }
}

/// Decode one huffman array payload. `huff_type` is 1 or 2. Returns payload bytes consumed.
pub(crate) fn decode_huff(src: &[u8], out: &mut [u8], huff_type: u32) -> Result<usize> {
    let mut syms = [0u8; 1280];
    let mut code_prefix = CODE_PREFIX_ORG;

    // Two selector bits are read up front: 0 => old lengths, 10 => new lengths, 11 => error.
    let mut bits = BitReader::new(src);
    let num_syms = if bits.read_bit_no_refill() == 0 {
        read_code_lengths_old(&mut bits, &mut syms, &mut code_prefix)?
    } else if bits.read_bit_no_refill() == 0 {
        read_code_lengths_new(&mut bits, &mut syms, &mut code_prefix)?
    } else {
        return Err(Error::Corrupt("huff: bad table selector"));
    };

    // Recover the byte position where the table ended.
    let mut p = bits.cursor_after();

    if num_syms == 1 {
        // RLE-of-one: the entire output is a single symbol.
        out.iter_mut().for_each(|b| *b = syms[0]);
        return Ok(p);
    }

    let lut = make_lut(&CODE_PREFIX_ORG, &code_prefix, &syms)?;
    let rev = build_reverse_lut(&lut);

    let out_len = out.len();
    if huff_type == 1 {
        if p + 3 > src.len() {
            return Err(Error::Truncated);
        }
        let split_mid = u16::from_le_bytes([src[p], src[p + 1]]) as usize;
        p += 2;
        // streams: forward = [p..], end(backward) = [..src.len()], mid_org = p+split_mid.
        decode_core(src, p, src.len(), p + split_mid, &rev, out, 0, out_len)?;
        Ok(src.len())
    } else {
        if p + 6 > src.len() {
            return Err(Error::Truncated);
        }
        let half = (out_len + 1) >> 1;
        let split_mid = u32::from_le_bytes([src[p], src[p + 1], src[p + 2], 0]) as usize;
        p += 3;
        if split_mid > src.len() - p {
            return Err(Error::Corrupt("huff: split_mid"));
        }
        let src_mid = p + split_mid;
        let split_left = u16::from_le_bytes([src[p], src[p + 1]]) as usize;
        p += 2;
        if src_mid < p + split_left + 2 || src.len() - src_mid < 3 {
            return Err(Error::Corrupt("huff: split_left"));
        }
        let split_right = u16::from_le_bytes([src[src_mid], src[src_mid + 1]]) as usize;
        if src.len() - (src_mid + 2) < split_right + 2 {
            return Err(Error::Corrupt("huff: split_right"));
        }
        // First half: forward=[p..src_mid], mid_org=p+split_left, end=src_mid.
        decode_core(src, p, src_mid, p + split_left, &rev, out, 0, half)?;
        // Second half: forward=[src_mid+2..src.len()], mid_org=src_mid+2+split_right.
        decode_core(
            src,
            src_mid + 2,
            src.len(),
            src_mid + 2 + split_right,
            &rev,
            out,
            half,
            out_len,
        )?;
        Ok(src.len())
    }
}

/// Build the bit-reversed LUT used by the core decode. The core reads `k = bits & 0x7FF`
/// where `bits` was filled LSB-first relative to the natural code order, so the natural LUT
/// must be indexed by the bit-reversed `k`: `rev[i] = lut[reverse11(i)]`. We do this
/// directly with the shared reversal table.
fn build_reverse_lut(lut: &HuffLut) -> HuffLut {
    let mut rev = HuffLut::zeroed();
    for i in 0..2048usize {
        let j = REVERSE_BITS[i] as usize;
        rev.bits2len[i] = lut.bits2len[j];
        rev.bits2sym[i] = lut.bits2sym[j];
    }
    rev
}

/// Expand the per-length symbol prefixes into the 2048-slot natural LUT.
fn make_lut(prefix_org: &[u32; 12], prefix_cur: &[u32; 12], syms: &[u8]) -> Result<HuffLut> {
    let mut lut = HuffLut::zeroed();
    let mut currslot = 0usize;
    for i in 1..11u32 {
        let start = prefix_org[i as usize];
        let count = prefix_cur[i as usize] - start;
        if count != 0 {
            let stepsize = 1usize << (11 - i);
            let num_to_set = (count as usize) << (11 - i);
            if currslot + num_to_set > 2048 {
                return Err(Error::Corrupt("huff: lut overflow"));
            }
            for s in &mut lut.bits2len[currslot..currslot + num_to_set] {
                *s = i as u8;
            }
            let mut off = currslot;
            for j in 0..count as usize {
                let sym = syms[start as usize + j];
                for s in &mut lut.bits2sym[off..off + stepsize] {
                    *s = sym;
                }
                off += stepsize;
            }
            currslot += num_to_set;
        }
    }
    // Length-11 codes occupy one slot each.
    let n11 = (prefix_cur[11] - prefix_org[11]) as usize;
    if n11 != 0 {
        if currslot + n11 > 2048 {
            return Err(Error::Corrupt("huff: lut overflow"));
        }
        for s in &mut lut.bits2len[currslot..currslot + n11] {
            *s = 11;
        }
        let base = prefix_org[11] as usize;
        lut.bits2sym[currslot..currslot + n11].copy_from_slice(&syms[base..base + n11]);
        currslot += n11;
    }
    if currslot != 2048 {
        return Err(Error::Corrupt("huff: lut not full"));
    }
    Ok(lut)
}

/// A forward LSB-first symbol-stream accumulator: bytes are loaded little-endian into rising
/// bit positions, the low 11 bits form the LUT index, and consuming shifts right. (The core
/// reads `bits & 0x7FF`, not the top bits — the reverse LUT is built for this LSB-first order.)
struct FwdAcc {
    bits: u32,
    /// next byte index to load
    pos: i64,
    /// number of valid bits currently buffered
    nbits: i32,
}
impl FwdAcc {
    fn new(pos: usize) -> Self {
        FwdAcc {
            bits: 0,
            pos: pos as i64,
            nbits: 0,
        }
    }
    #[inline]
    fn refill(&mut self, src: &[u8]) {
        // Keep at least 24 bits buffered (enough for an 11-bit code).
        while self.nbits <= 24 {
            let byte = byte_at(src, self.pos);
            self.bits |= byte << self.nbits;
            self.pos += 1;
            self.nbits += 8;
        }
    }
    #[inline]
    fn peek11(&self) -> u32 {
        self.bits & 0x7FF
    }
    #[inline]
    fn consume(&mut self, n: u8) {
        self.bits >>= n;
        self.nbits -= n as i32;
    }
    /// Logical byte cursor: bytes loaded minus whole buffered bytes (exact at byte alignment).
    fn cursor(&self) -> i64 {
        self.pos - (self.nbits as i64) / 8
    }
}

/// A backward LSB-first accumulator over the `src_end` stream: it consumes bytes
/// from the highest remaining address downward, byteswapped so the highest-address byte lands
/// in the lowest bit position. Equivalent to loading `byte_at(pos-1)` into rising bit positions
/// while stepping `pos` down. Reads the low 11 bits like the forward stream.
struct BwdAcc {
    bits: u32,
    /// index one past the next byte to load (load happens at `pos-1`)
    pos: i64,
    nbits: i32,
}
impl BwdAcc {
    fn new(pos: usize) -> Self {
        BwdAcc {
            bits: 0,
            pos: pos as i64,
            nbits: 0,
        }
    }
    #[inline]
    fn refill(&mut self, src: &[u8]) {
        while self.nbits <= 24 {
            let byte = byte_at(src, self.pos - 1);
            self.bits |= byte << self.nbits;
            self.pos -= 1;
            self.nbits += 8;
        }
    }
    #[inline]
    fn peek11(&self) -> u32 {
        self.bits & 0x7FF
    }
    #[inline]
    fn consume(&mut self, n: u8) {
        self.bits >>= n;
        self.nbits -= n as i32;
    }
    /// Logical byte cursor (exact at byte alignment): next-unloaded byte plus buffered bytes.
    fn cursor(&self) -> i64 {
        self.pos + (self.nbits as i64) / 8
    }
}

#[inline]
fn byte_at(src: &[u8], idx: i64) -> u32 {
    if idx >= 0 && (idx as usize) < src.len() {
        src[idx as usize] as u32
    } else {
        0
    }
}

/// Triple-stream interleaved huffman decode into `out[lo..hi]`.
///
/// Stream layout: forward `a` consumes `[s_fwd, s_mid_org)`; mid `b` (forward, from
/// `s_mid_org`) and end `c` (backward, from `s_end`) consume `[s_mid_org, s_end)` meeting in
/// the middle. One symbol is produced per stream per round (a, then c, then b).
#[allow(clippy::too_many_arguments)]
fn decode_core(
    src: &[u8],
    s_fwd: usize,
    s_end: usize,
    s_mid_org: usize,
    rev: &HuffLut,
    out: &mut [u8],
    lo: usize,
    hi: usize,
) -> Result<()> {
    if s_fwd > s_mid_org || s_mid_org > s_end {
        return Err(Error::Corrupt("huff: bad stream bounds"));
    }
    let mut a = FwdAcc::new(s_fwd);
    let mut b = FwdAcc::new(s_mid_org);
    let mut c = BwdAcc::new(s_end);

    let mut dst = lo;
    while dst < hi {
        a.refill(src);
        let k = a.peek11();
        let n = rev.bits2len[k as usize];
        if n == 0 {
            return Err(Error::Corrupt("huff: zero-length code"));
        }
        out[dst] = rev.bits2sym[k as usize];
        dst += 1;
        a.consume(n);
        if dst >= hi {
            break;
        }

        c.refill(src);
        let k = c.peek11();
        let n = rev.bits2len[k as usize];
        if n == 0 {
            return Err(Error::Corrupt("huff: zero-length code"));
        }
        out[dst] = rev.bits2sym[k as usize];
        dst += 1;
        c.consume(n);
        if dst >= hi {
            break;
        }

        b.refill(src);
        let k = b.peek11();
        let n = rev.bits2len[k as usize];
        if n == 0 {
            return Err(Error::Corrupt("huff: zero-length code"));
        }
        out[dst] = rev.bits2sym[k as usize];
        dst += 1;
        b.consume(n);
    }

    // a must have consumed exactly [s_fwd, s_mid_org); b and c must meet.
    if a.cursor() != s_mid_org as i64 || b.cursor() != c.cursor() {
        return Err(Error::Corrupt("huff: streams did not meet"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Code-length transmission
// ---------------------------------------------------------------------------------------

/// Read the "new"-form code-length table.
fn read_code_lengths_new(
    bits: &mut BitReader,
    syms: &mut [u8],
    code_prefix: &mut [u32; 12],
) -> Result<i32> {
    let forced_bits = bits.read_bits_no_refill(2);
    let num_symbols = bits.read_bits_no_refill(8) as i32 + 1;
    let fluff = read_fluff(bits, num_symbols);

    let mut code_len = [0u8; 512 + 16];

    // Build the Rice2 cursor over the same buffer at the current bit position.
    let mut br2 = Rice2 {
        bitpos: ((bits.bitpos - 24) & 7) as u32,
        p: bits.p - (((24 - bits.bitpos + 7) >> 3) as usize),
        p_end: bits.p_end,
    };

    let total = (num_symbols + fluff) as usize;
    decode_golomb_rice_lengths(&mut code_len[..total], src_of(bits), &mut br2)?;
    for b in &mut code_len[total..total + 16] {
        *b = 0;
    }
    decode_golomb_rice_bits(
        &mut code_len[..num_symbols as usize],
        src_of(bits),
        forced_bits,
        &mut br2,
    )?;

    // Reset the main bit reader to br2's byte position.
    bits.bitpos = 24;
    bits.p = br2.p;
    bits.bits = 0;
    bits.refill();
    bits.bits <<= br2.bitpos;
    bits.bitpos += br2.bitpos as i32;

    // Delta-decode the code lengths (running average).
    let mut running_sum: u32 = 0x1e;
    for cl in code_len.iter_mut().take(num_symbols as usize) {
        let v0 = *cl as i32;
        let v = -(v0 & 1) ^ (v0 >> 1); // zigzag decode
        let len = v + (running_sum >> 2) as i32 + 1;
        if !(1..=11).contains(&len) {
            return Err(Error::Corrupt("huff: codelen out of range"));
        }
        *cl = len as u8;
        running_sum = running_sum.wrapping_add(v as u32);
    }

    let mut range = [HuffRange::default(); 128];
    let ranges = convert_to_ranges(
        &mut range,
        num_symbols,
        fluff,
        &code_len[num_symbols as usize..],
        bits,
    )?;

    // Emit symbols into the per-length prefix buckets.
    let mut cp = 0usize;
    for r in range.iter().take(ranges) {
        let mut sym = r.symbol as i32;
        for _ in 0..r.num {
            let len = code_len[cp] as usize;
            cp += 1;
            syms[code_prefix[len] as usize] = sym as u8;
            code_prefix[len] += 1;
            sym += 1;
        }
    }
    Ok(num_symbols)
}

/// Read the "old"-form code-length table.
fn read_code_lengths_old(
    bits: &mut BitReader,
    syms: &mut [u8],
    code_prefix: &mut [u32; 12],
) -> Result<i32> {
    if bits.read_bit_no_refill() != 0 {
        // Dense legacy encoding.
        let mut sym: i32 = 0;
        let mut num_symbols: i32 = 0;
        let mut avg_bits_x4: i32 = 32;
        let forced_bits = bits.read_bits_no_refill(2);
        let thres_for_valid_gamma_bits = 1u32 << (31 - (20u32 >> forced_bits));

        let mut skip_initial = bits.read_bit() != 0;
        loop {
            if !skip_initial {
                // Run of zeros.
                if bits.bits & 0xff00_0000 == 0 {
                    return Err(Error::Corrupt("huff: bad zero run"));
                }
                let lz = clz32(bits.bits);
                sym += bits.read_bits_no_refill(2 * (lz + 1)) as i32 - 2 + 1;
                if sym >= 256 {
                    break;
                }
            }
            skip_initial = false;
            bits.refill();
            if bits.bits & 0xff00_0000 == 0 {
                return Err(Error::Corrupt("huff: bad sym count"));
            }
            let lz = clz32(bits.bits);
            let mut n = bits.read_bits_no_refill(2 * (lz + 1)) as i32 - 2 + 1;
            if sym + n > 256 {
                return Err(Error::Corrupt("huff: sym overflow"));
            }
            bits.refill();
            num_symbols += n;
            loop {
                if bits.bits < thres_for_valid_gamma_bits {
                    return Err(Error::Corrupt("huff: gamma too big"));
                }
                let lz = clz32(bits.bits) as i32;
                let v = bits.read_bits_no_refill((lz + forced_bits as i32 + 1) as u32) as i32
                    + ((lz - 1) << forced_bits);
                let codelen = (-(v & 1) ^ (v >> 1)) + ((avg_bits_x4 + 2) >> 2);
                if !(1..=11).contains(&codelen) {
                    return Err(Error::Corrupt("huff: codelen out of range"));
                }
                avg_bits_x4 = codelen + ((3 * avg_bits_x4 + 2) >> 2);
                bits.refill();
                syms[code_prefix[codelen as usize] as usize] = sym as u8;
                code_prefix[codelen as usize] += 1;
                sym += 1;
                n -= 1;
                if n == 0 {
                    break;
                }
            }
            if sym == 256 {
                break;
            }
        }
        if sym == 256 && num_symbols >= 2 {
            Ok(num_symbols)
        } else {
            Err(Error::Corrupt("huff: old table incomplete"))
        }
    } else {
        // Sparse encoding.
        let num_symbols = bits.read_bits_no_refill(8) as i32;
        if num_symbols == 0 {
            return Err(Error::Corrupt("huff: zero symbols"));
        }
        if num_symbols == 1 {
            syms[0] = bits.read_bits_no_refill(8) as u8;
        } else {
            let codelen_bits = bits.read_bits_no_refill(3);
            if codelen_bits > 4 {
                return Err(Error::Corrupt("huff: codelen_bits"));
            }
            for _ in 0..num_symbols {
                bits.refill();
                let sym = bits.read_bits_no_refill(8) as i32;
                let codelen = bits.read_bits_no_refill_zero(codelen_bits) as i32 + 1;
                if codelen > 11 {
                    return Err(Error::Corrupt("huff: codelen out of range"));
                }
                syms[code_prefix[codelen as usize] as usize] = sym as u8;
                code_prefix[codelen as usize] += 1;
            }
        }
        Ok(num_symbols)
    }
}

/// Borrow the underlying slice from a BitReader (the rice decoder reads raw bytes).
#[inline]
fn src_of<'a>(bits: &BitReader<'a>) -> &'a [u8] {
    bits.data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_lut_two_symbol_balanced() {
        // Two symbols, each code length 1 (codes 0 and 1). code_prefix for len 1 advances
        // from CODE_PREFIX_ORG[1]=0 to 2. syms[0], syms[1] hold the two symbols.
        let mut prefix = CODE_PREFIX_ORG;
        let syms = {
            let mut s = [0u8; 4];
            s[0] = b'A';
            s[1] = b'B';
            s
        };
        prefix[1] = 2; // two length-1 codes assigned
        let lut = make_lut(&CODE_PREFIX_ORG, &prefix, &syms).unwrap();
        // Half the 2048 slots map to 'A' (len 1), the other half to 'B'.
        assert_eq!(lut.bits2len[0], 1);
        assert_eq!(lut.bits2len[2047], 1);
        assert_eq!(lut.bits2sym[0], b'A');
        assert_eq!(lut.bits2sym[1024], b'B');
    }

    #[test]
    fn reverse_lut_indexes_by_bit_reverse() {
        let mut prefix = CODE_PREFIX_ORG;
        let mut syms = [0u8; 4];
        syms[0] = 10;
        syms[1] = 20;
        prefix[1] = 2;
        let lut = make_lut(&CODE_PREFIX_ORG, &prefix, &syms).unwrap();
        let rev = build_reverse_lut(&lut);
        for i in 0..2048usize {
            let j = REVERSE_BITS[i] as usize;
            assert_eq!(rev.bits2sym[i], lut.bits2sym[j]);
        }
    }
}
