//! Internal bit readers/writers *with the cursor exposed*, which the entropy headers need
//! for their pointer arithmetic.
//!
//! The spine's [`crate::bits::BitReader`] hides its byte cursor, but the huffman/tANS
//! header parsing recovers the post-header byte position with expressions like
//! `src = bits.p - ((24 - bits.bitpos) / 8)`. To stay byte-faithful we expose the
//! `BitReader` state (`bits`, `bitpos`, `p`) here, where `p` is an index into the source
//! slice. Refills are byte-granular, keeping the accumulator MSB-aligned with ≥24 valid bits.

#![allow(dead_code)]

use alloc::vec::Vec;

/// Forward, MSB-first bit reader with an exposed byte cursor `p`.
///
/// Invariant after `refill`: at least 24 bits are valid in the high end of `bits`, and
/// `bitpos` (bits consumed since the accumulator was full at bit 24) is in
/// `-8..=0` worth of slack but kept `<= 24` after the loop. We track `p` as the index of the
/// next byte to load; bytes past the end zero-fill (`p < p_end ? *p : 0`).
pub(crate) struct BitReader<'a> {
    pub(crate) data: &'a [u8],
    /// Index of the next byte to load.
    pub(crate) p: usize,
    /// End index; equals `data.len()`. Bytes at/after are read as 0.
    pub(crate) p_end: usize,
    /// MSB-aligned accumulator.
    pub(crate) bits: u32,
    /// Bits consumed since the last refill.
    pub(crate) bitpos: i32,
}

impl<'a> BitReader<'a> {
    /// Create at the start: `bitpos=24; bits=0; p=src` + a refill.
    pub(crate) fn new(data: &'a [u8]) -> Self {
        let mut br = BitReader {
            data,
            p: 0,
            p_end: data.len(),
            bits: 0,
            bitpos: 24,
        };
        br.refill();
        br
    }

    /// Create at an explicit cursor offset (no refill), for resuming a sub-stream.
    pub(crate) fn at(data: &'a [u8], p: usize) -> Self {
        BitReader {
            data,
            p,
            p_end: data.len(),
            bits: 0,
            bitpos: 24,
        }
    }

    #[inline]
    fn load_byte(&mut self) -> u32 {
        let b = if self.p < self.p_end {
            self.data[self.p] as u32
        } else {
            0
        };
        self.p += 1;
        b
    }

    /// While `bitpos > 0`, shift in the next byte at `bitpos`.
    #[inline]
    pub(crate) fn refill(&mut self) {
        while self.bitpos > 0 {
            let b = self.load_byte();
            self.bits |= b << self.bitpos;
            self.bitpos -= 8;
        }
    }

    /// Top bit, no refill.
    #[inline]
    pub(crate) fn read_bit_no_refill(&mut self) -> u32 {
        let r = self.bits >> 31;
        self.bits <<= 1;
        self.bitpos += 1;
        r
    }

    /// Refill then top bit.
    #[inline]
    pub(crate) fn read_bit(&mut self) -> u32 {
        self.refill();
        self.read_bit_no_refill()
    }

    /// Top `n` bits, no refill. `n` in `1..=24`.
    #[inline]
    pub(crate) fn read_bits_no_refill(&mut self, n: u32) -> u32 {
        // `bits >> 1 >> (31 - n)` — valid for n>=1 and avoids the n==32 UB; with n==0
        // this would read 1 bit, so callers use read_bits_no_refill_zero for the n==0 case.
        let r = (self.bits >> 1) >> (31 - n);
        self.bits <<= n;
        self.bitpos += n as i32;
        r
    }

    /// Like above but returns 0 for `n == 0`.
    #[inline]
    pub(crate) fn read_bits_no_refill_zero(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        self.read_bits_no_refill(n)
    }

    /// Refill then read `n` bits. `n<=24`.
    #[inline]
    pub(crate) fn read_bits(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        self.refill();
        self.read_bits_no_refill(n)
    }

    /// Byte index where a sub-stream ended: `p - ((24 - bitpos) / 8)`.
    ///
    /// `(24 - bitpos)` is the number of bits currently buffered (0..=24); dividing by 8 gives
    /// the count of whole *unconsumed* loaded bytes to step the cursor back over.
    #[inline]
    pub(crate) fn cursor_after(&self) -> usize {
        self.p - ((24 - self.bitpos) as usize / 8)
    }
}

/// Count leading zeros of `bits` (`x==0` ⇒ 32).
#[inline]
pub(crate) fn clz32(x: u32) -> u32 {
    x.leading_zeros()
}

/// Index of the highest set bit. `x` must be non-zero.
#[inline]
pub(crate) fn bsr32(x: u32) -> u32 {
    debug_assert!(x != 0);
    31 - x.leading_zeros()
}

/// Index of the lowest set bit. `x` must be non-zero.
#[inline]
pub(crate) fn bsf32(x: u32) -> u32 {
    debug_assert!(x != 0);
    x.trailing_zeros()
}

/// A second, byte-oriented bit reader (used by the Golomb-Rice
/// length decoder). It reads bytes forward with a separate `bitpos` measured from the MSB.
pub(crate) struct BitReader2<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) p: usize,
    pub(crate) p_end: usize,
    pub(crate) bitpos: u32,
}

/// Forward bit writer used by the encoders: packs bits MSB-first into a byte
/// buffer. Bit order matches what [`BitReader`] reads back (the top bits go out first).
///
/// We keep it simple and exact: the produced byte stream packs MSB-first and pads the final
/// byte with zero low bits.
pub(crate) struct BitWriterFwd {
    out: Vec<u8>,
    acc: u64,
    nbits: u32,
}

impl BitWriterFwd {
    pub(crate) fn new() -> Self {
        BitWriterFwd {
            out: Vec::new(),
            acc: 0,
            nbits: 0,
        }
    }

    /// Append the low `n` bits of `v` (`n` in `0..=32`), MSB-first.
    #[inline]
    pub(crate) fn write(&mut self, v: u32, n: u32) {
        debug_assert!(n <= 32);
        if n == 0 {
            return;
        }
        let masked = if n == 32 {
            v as u64
        } else {
            (v as u64) & ((1u64 << n) - 1)
        };
        self.nbits += n;
        self.acc |= masked << (64 - self.nbits);
        while self.nbits >= 8 {
            self.out.push((self.acc >> 56) as u8);
            self.acc <<= 8;
            self.nbits -= 8;
        }
    }

    /// Number of whole bytes already flushed plus the pending partial bits.
    pub(crate) fn bit_len(&self) -> usize {
        self.out.len() * 8 + self.nbits as usize
    }

    /// Finish, flushing a trailing partial byte (zero-padded low), returning the bytes.
    pub(crate) fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.out.push((self.acc >> 56) as u8);
        }
        self.out
    }
}

/// An LSB-first bit writer matching the huffman data-stream order: bits are
/// packed into rising bit positions of rising byte addresses, so the decoder's `bits & 0x7FF`
/// (low-bits) reader reproduces the emission order. Used for the huffman data streams.
pub(crate) struct BitWriterLsb {
    out: Vec<u8>,
    acc: u64,
    nbits: u32,
}

impl BitWriterLsb {
    pub(crate) fn new() -> Self {
        BitWriterLsb {
            out: Vec::new(),
            acc: 0,
            nbits: 0,
        }
    }

    /// Append the low `n` bits of `v` (`n` in `0..=32`), least-significant-bit first.
    #[inline]
    pub(crate) fn write(&mut self, v: u32, n: u32) {
        debug_assert!(n <= 32);
        if n == 0 {
            return;
        }
        let masked = if n == 32 {
            v as u64
        } else {
            (v as u64) & ((1u64 << n) - 1)
        };
        // Place new bits just above the bits already pending (LSB-first packing).
        self.acc |= masked << self.nbits;
        self.nbits += n;
        while self.nbits >= 8 {
            self.out.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.nbits -= 8;
        }
    }

    /// Finish, flushing a trailing partial byte (zero-padded high), returning the bytes.
    pub(crate) fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.out.push((self.acc & 0xFF) as u8);
        }
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fwd_reader_matches_msb_order() {
        let mut br = BitReader::new(&[0b1011_0010, 0b1100_0001]);
        assert_eq!(br.read_bits(3), 0b101);
        assert_eq!(br.read_bits(5), 0b10010);
        assert_eq!(br.read_bits(8), 0b1100_0001);
    }

    #[test]
    fn cursor_after_recovers_byte_boundary() {
        // After consuming exactly 8 bits (one byte) of a 4-byte buffer, the cursor should
        // point at byte 1.
        let mut br = BitReader::new(&[0xAA, 0xBB, 0xCC, 0xDD]);
        let _ = br.read_bits(8);
        assert_eq!(br.cursor_after(), 1);
        let _ = br.read_bits(8);
        assert_eq!(br.cursor_after(), 2);
    }

    #[test]
    fn writer_reader_roundtrip() {
        let mut w = BitWriterFwd::new();
        let pairs: &[(u32, u32)] = &[(1, 1), (0b101, 3), (0xAB, 8), (0xBEEF, 16), (0, 5)];
        for &(v, n) in pairs {
            w.write(v, n);
        }
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        for &(v, n) in pairs {
            let mask = if n == 32 { u32::MAX } else { (1u32 << n) - 1 };
            assert_eq!(r.read_bits(n), v & mask, "v={v:#x} n={n}");
        }
    }
}
