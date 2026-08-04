//! Bit readers/writer, MSB-first.
//!
//! [`BitReader`] is a 32-bit MSB-aligned accumulator:
//! `bits` holds the unconsumed bits left-aligned, `bitpos` counts bits consumed since the
//! last refill (it may briefly go negative inside a refill), and a byte cursor walks the
//! buffer. A refill keeps at least 24 bits available. The forward reader walks the buffer
//! low→high; the reverse reader walks it high→low (Kraken interleaves a forward and a
//! backward stream). Both load from the most-significant end and read the top bits first.
//!
//! ## Bounds / zero-fill policy
//!
//! A literal `0` is read for any byte past the buffer end (`p < p_end ? *p : 0`). Real
//! Kraken streams depend on this slack — a sub-stream's final bits are decoded after `p`
//! has stepped a few bytes past the logical end. We replicate the zero-fill, but cap it: a
//! reader may zero-fill at most [`OVERRUN_LIMIT`] bytes (one accumulator's worth) before a
//! further refill is rejected with [`Error::Truncated`]. That keeps faithful decoding of
//! well-formed streams while guaranteeing a malformed/short stream can never over-read or
//! loop. Within that slack `read_*` always succeeds (it returns the implicit zeros).

#![allow(dead_code)]

use crate::Error;
use alloc::vec::Vec;

/// Maximum number of implicit zero bytes a reader will synthesise past the end of `data`
/// before refusing further refills. One `u32` accumulator (4 bytes) is enough to finish any
/// correctly terminated sub-stream.
const OVERRUN_LIMIT: usize = 4;

/// Forward, MSB-first bit reader.
pub(crate) struct BitReader<'a> {
    data: &'a [u8],
    /// Index of the next byte to load into the accumulator.
    pos: usize,
    /// How many bytes past `data.len()` have been zero-filled so far.
    overrun: usize,
    /// MSB-aligned accumulator of unconsumed bits.
    bits: u32,
    /// Bits consumed since the last refill. May go transiently negative.
    bitpos: i32,
}

impl<'a> BitReader<'a> {
    /// Create a forward reader. Init: `bitpos = 24; bits = 0; p = src;` then a
    /// refill, so the first three bytes are loaded MSB-first (`byte0` at bit 24).
    pub(crate) fn new(data: &'a [u8]) -> Self {
        let mut br = BitReader {
            data,
            pos: 0,
            overrun: 0,
            bits: 0,
            bitpos: 24,
        };
        // Best-effort initial fill (a refill runs right after init). If the buffer is
        // shorter than the accumulator it zero-fills.
        let _ = br.refill();
        br
    }

    /// Load the next byte (or an implicit zero past the end) for the accumulator, advancing
    /// the cursor. Returns `Err(Truncated)` once the overrun budget is exhausted.
    fn next_byte(&mut self) -> Result<u32, Error> {
        if self.pos < self.data.len() {
            let b = self.data[self.pos] as u32;
            self.pos += 1;
            Ok(b)
        } else {
            if self.overrun >= OVERRUN_LIMIT {
                return Err(Error::Truncated);
            }
            self.overrun += 1;
            self.pos += 1;
            Ok(0)
        }
    }

    /// Top up the accumulator to at least 24 bits.
    fn refill(&mut self) -> Result<(), Error> {
        while self.bitpos > 0 {
            let b = self.next_byte()?;
            self.bits |= b << self.bitpos;
            self.bitpos -= 8;
        }
        Ok(())
    }

    /// Refill then return the top bit.
    pub(crate) fn read_bit(&mut self) -> Result<u32, Error> {
        self.refill()?;
        let r = self.bits >> 31;
        self.bits <<= 1;
        self.bitpos += 1;
        Ok(r)
    }

    /// Read `n` bits MSB-first (`n` in `0..=24`), refilling first so the bits are available.
    /// `n` bits are read as `bits >> (32 - n)` from the top of the accumulator.
    pub(crate) fn read_bits(&mut self, n: u32) -> Result<u32, Error> {
        debug_assert!(n <= 24, "read_bits supports up to 24 bits, got {n}");
        if n == 0 {
            return Ok(0);
        }
        self.refill()?;
        // `bits >> 1 >> (31 - n)` is the no-refill-zero form; for n in 1..=24 it equals
        // `bits >> (32 - n)` without the n==32 UB. We only support n<=24 so use it directly.
        let r = (self.bits >> 1) >> (31 - n);
        self.bits <<= n;
        self.bitpos += n as i32;
        Ok(r)
    }

    /// Current MSB-aligned accumulator, without refilling.
    #[inline]
    pub(crate) fn peek_bits(&self) -> u32 {
        self.bits
    }

    /// Consume `n` bits from the top of the accumulator without refilling (advances
    /// `bitpos` and shifts `bits` left). `n` may exceed the loaded count; the caller refills.
    #[inline]
    pub(crate) fn skip_bits(&mut self, n: u32) {
        self.bits <<= n;
        self.bitpos += n as i32;
    }

    /// Read `n` bits from the top of the accumulator without refilling, then refill.
    /// `n` in `1..=24`.
    pub(crate) fn read_top(&mut self, n: u32) -> Result<u32, Error> {
        let r = self.bits >> (32 - n);
        self.bits <<= n;
        self.bitpos += n as i32;
        self.refill()?;
        Ok(r)
    }

    /// Read an `n`-bit value (forward), handling `n > 24` in two refilled halves. `n` in
    /// `0..=53` in practice (Kraken caps `cmd>>3` at 26, so `n <= 26`).
    pub(crate) fn read_more_than_24_bits(&mut self, n: u32) -> Result<u32, Error> {
        let rv = if n <= 24 {
            // ReadBitsNoRefillZero: handles n == 0.
            let r = (self.bits >> 1) >> (31 - n);
            self.bits <<= n;
            self.bitpos += n as i32;
            r
        } else {
            let hi = self.bits >> (32 - 24);
            self.bits <<= 24;
            self.bitpos += 24;
            let mut rv = hi << (n - 24);
            self.refill()?;
            let lo = self.bits >> (32 - (n - 24));
            self.bits <<= n - 24;
            self.bitpos += (n - 24) as i32;
            rv += lo;
            rv
        };
        self.refill()?;
        Ok(rv)
    }

    /// Read an offset code (forward), parametrized by `v`.
    pub(crate) fn read_distance(&mut self, v: u32) -> Result<u32, Error> {
        let rv;
        if v < 0xF0 {
            let n = (v >> 4) + 4;
            let w = (self.bits | 1).rotate_left(n);
            self.bitpos += n as i32;
            let m = (2u32 << n).wrapping_sub(1);
            self.bits = w & !m;
            rv = ((w & m) << 4).wrapping_add(v & 0xF).wrapping_sub(248);
        } else {
            let n = v - 0xF0 + 4;
            let w = (self.bits | 1).rotate_left(n);
            self.bitpos += n as i32;
            let m = (2u32 << n).wrapping_sub(1);
            self.bits = w & !m;
            let mut r = 8_322_816u32.wrapping_add((w & m) << 12);
            self.refill()?;
            r = r.wrapping_add(self.bits >> 20);
            self.bitpos += 12;
            self.bits <<= 12;
            rv = r;
        }
        self.refill()?;
        Ok(rv)
    }

    /// Read a length code (forward). Returns the decoded length.
    pub(crate) fn read_length(&mut self) -> Result<u32, Error> {
        if self.bits == 0 {
            return Err(Error::Corrupt("kraken: length code zero accumulator"));
        }
        let mut n = 31 - bsr(self.bits);
        if n > 12 {
            return Err(Error::Corrupt("kraken: length code too long"));
        }
        self.bitpos += n as i32;
        self.bits <<= n;
        self.refill()?;
        n += 7;
        self.bitpos += n as i32;
        let rv = (self.bits >> (32 - n)).wrapping_sub(64);
        self.bits <<= n;
        self.refill()?;
        Ok(rv)
    }

    /// Signed logical byte position relative to the buffer start, used to
    /// reconcile the forward and backward readers after offset unpacking. The forward reader's
    /// cursor advances past the end while zero-filling, so this is just `pos`.
    #[inline]
    pub(crate) fn logical_pos(&self) -> isize {
        self.pos as isize
    }

    /// Bits consumed since the last refill; in `-7..=24`.
    #[inline]
    pub(crate) fn bitpos(&self) -> i32 {
        self.bitpos
    }

    /// End-of-stream reconciliation for offset unpacking: after rewinding each reader's
    /// unconsumed bytes, the forward and backward cursors must point at the same byte.
    pub(crate) fn meets(a: &BitReader, b: &BitReaderRev) -> bool {
        let pa = a.logical_pos() - (((24 - a.bitpos()) >> 3) as isize);
        let pb = b.logical_pos() + (((24 - b.bitpos()) >> 3) as isize);
        pa == pb
    }

    /// Read an Elias-gamma-like value. The accumulator must already hold
    /// at least 23 bits (the caller refills); the unary length prefix is the leading-zero
    /// run of `bits`. Returns the decoded value (`r - 2`).
    pub(crate) fn read_gamma(&mut self) -> Result<u32, Error> {
        self.refill()?;
        let n = if self.bits != 0 {
            31 - bsr(self.bits)
        } else {
            32
        };
        let n = 2 * n + 2;
        debug_assert!(n < 24, "gamma length {n} out of range");
        self.bitpos += n as i32;
        let r = self.bits >> (32 - n);
        self.bits <<= n;
        Ok(r - 2)
    }
}

/// Reverse, MSB-first bit reader: walks `data` high→low, loading bytes from the end.
///
/// The reader is initialised at the end of the buffer and each refill steps `p` backwards,
/// loading `*--p` into the accumulator. Reads still take the top `n` bits.
pub(crate) struct BitReaderRev<'a> {
    data: &'a [u8],
    /// Index one past the next byte to load (load happens at `pos - 1`, then `pos` decrements).
    pos: usize,
    /// How many bytes before index 0 have been zero-filled so far.
    overrun: usize,
    bits: u32,
    bitpos: i32,
}

impl<'a> BitReaderRev<'a> {
    /// Create a reverse reader positioned at the end of `data`, MSB-first.
    pub(crate) fn new(data: &'a [u8]) -> Self {
        let mut br = BitReaderRev {
            data,
            pos: data.len(),
            overrun: 0,
            bits: 0,
            bitpos: 24,
        };
        let _ = br.refill();
        br
    }

    fn prev_byte(&mut self) -> Result<u32, Error> {
        if self.pos > 0 {
            self.pos -= 1;
            Ok(self.data[self.pos] as u32)
        } else {
            if self.overrun >= OVERRUN_LIMIT {
                return Err(Error::Truncated);
            }
            self.overrun += 1;
            Ok(0)
        }
    }

    /// Top up the accumulator to at least 24 bits, loading bytes backwards.
    fn refill(&mut self) -> Result<(), Error> {
        while self.bitpos > 0 {
            let b = self.prev_byte()?;
            self.bits |= b << self.bitpos;
            self.bitpos -= 8;
        }
        Ok(())
    }

    /// Read `n` bits MSB-first (`n` in `0..=24`) from the backward stream.
    pub(crate) fn read_bits(&mut self, n: u32) -> Result<u32, Error> {
        debug_assert!(n <= 24, "read_bits supports up to 24 bits, got {n}");
        if n == 0 {
            return Ok(0);
        }
        self.refill()?;
        let r = (self.bits >> 1) >> (31 - n);
        self.bits <<= n;
        self.bitpos += n as i32;
        Ok(r)
    }

    /// Current MSB-aligned accumulator without refilling.
    #[inline]
    pub(crate) fn peek_bits(&self) -> u32 {
        self.bits
    }

    /// Consume `n` bits from the top without refilling.
    #[inline]
    pub(crate) fn skip_bits(&mut self, n: u32) {
        self.bits <<= n;
        self.bitpos += n as i32;
    }

    /// Read `n` bits from the top without refilling, then refill backwards. `n` in `1..=24`.
    pub(crate) fn read_top(&mut self, n: u32) -> Result<u32, Error> {
        let r = self.bits >> (32 - n);
        self.bits <<= n;
        self.bitpos += n as i32;
        self.refill()?;
        Ok(r)
    }

    /// Read an `n`-bit value (backward), handling `n > 24` in two refilled halves.
    pub(crate) fn read_more_than_24_bits(&mut self, n: u32) -> Result<u32, Error> {
        let rv = if n <= 24 {
            let r = (self.bits >> 1) >> (31 - n);
            self.bits <<= n;
            self.bitpos += n as i32;
            r
        } else {
            let hi = self.bits >> (32 - 24);
            self.bits <<= 24;
            self.bitpos += 24;
            let mut rv = hi << (n - 24);
            self.refill()?;
            let lo = self.bits >> (32 - (n - 24));
            self.bits <<= n - 24;
            self.bitpos += (n - 24) as i32;
            rv += lo;
            rv
        };
        self.refill()?;
        Ok(rv)
    }

    /// Read an offset code (backward), parametrized by `v`.
    pub(crate) fn read_distance(&mut self, v: u32) -> Result<u32, Error> {
        let rv;
        if v < 0xF0 {
            let n = (v >> 4) + 4;
            let w = (self.bits | 1).rotate_left(n);
            self.bitpos += n as i32;
            let m = (2u32 << n).wrapping_sub(1);
            self.bits = w & !m;
            rv = ((w & m) << 4).wrapping_add(v & 0xF).wrapping_sub(248);
        } else {
            let n = v - 0xF0 + 4;
            let w = (self.bits | 1).rotate_left(n);
            self.bitpos += n as i32;
            let m = (2u32 << n).wrapping_sub(1);
            self.bits = w & !m;
            let mut r = 8_322_816u32.wrapping_add((w & m) << 12);
            self.refill()?;
            r = r.wrapping_add(self.bits >> (32 - 12));
            self.bitpos += 12;
            self.bits <<= 12;
            rv = r;
        }
        self.refill()?;
        Ok(rv)
    }

    /// Read a length code (backward). Returns the decoded length.
    pub(crate) fn read_length(&mut self) -> Result<u32, Error> {
        if self.bits == 0 {
            return Err(Error::Corrupt("kraken: length code zero accumulator"));
        }
        let mut n = 31 - bsr(self.bits);
        if n > 12 {
            return Err(Error::Corrupt("kraken: length code too long"));
        }
        self.bitpos += n as i32;
        self.bits <<= n;
        self.refill()?;
        n += 7;
        self.bitpos += n as i32;
        let rv = (self.bits >> (32 - n)).wrapping_sub(64);
        self.bits <<= n;
        self.refill()?;
        Ok(rv)
    }

    /// Signed logical byte position relative to the buffer start. The backward
    /// reader steps `p` down each refilled byte and may pass below `0` while zero-filling; we
    /// model that as `pos - overrun` (once `pos` hits `0`, further loads only grow `overrun`).
    #[inline]
    pub(crate) fn logical_pos(&self) -> isize {
        self.pos as isize - self.overrun as isize
    }

    /// Bits consumed since the last refill.
    #[inline]
    pub(crate) fn bitpos(&self) -> i32 {
        self.bitpos
    }
}

/// Bit-scan-reverse: index of the most-significant set bit. `x` must be non-zero.
#[inline]
fn bsr(x: u32) -> u32 {
    debug_assert!(x != 0);
    31 - x.leading_zeros()
}

/// Minimal forward, MSB-first bit writer used by the encoder. Bits are packed into bytes
/// most-significant-first so a [`BitReader`] reads them back in the same order.
pub(crate) struct BitWriter {
    out: Vec<u8>,
    /// Accumulator holding pending bits in its high end.
    acc: u64,
    /// Number of valid bits currently in `acc`.
    nbits: u32,
}

impl BitWriter {
    pub(crate) fn new() -> Self {
        BitWriter {
            out: Vec::new(),
            acc: 0,
            nbits: 0,
        }
    }

    /// Append the low `n` bits of `v` (`n` in `0..=32`), most-significant-bit first.
    pub(crate) fn write_bits(&mut self, v: u32, n: u32) {
        debug_assert!(n <= 32, "write_bits supports up to 32 bits, got {n}");
        if n == 0 {
            return;
        }
        let masked = if n == 32 {
            v as u64
        } else {
            (v as u64) & ((1u64 << n) - 1)
        };
        // Place the new bits just below the bits already pending (MSB-first packing).
        self.nbits += n;
        self.acc |= masked << (64 - self.nbits);
        // Flush whole bytes from the top.
        while self.nbits >= 8 {
            let byte = (self.acc >> 56) as u8;
            self.out.push(byte);
            self.acc <<= 8;
            self.nbits -= 8;
        }
    }

    /// Finish writing, flushing any trailing partial byte (zero-padded on the low side), and
    /// return the packed bytes.
    pub(crate) fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            let byte = (self.acc >> 56) as u8;
            self.out.push(byte);
        }
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_bits_msb_first() {
        // The reader fills `bits` from bytes MSB-first and reads from the top. 0xB2,0xC1 =
        // 1011_0010 1100_0001; first 3 bits = 101, next 5 = 10010.
        let mut br = BitReader::new(&[0b1011_0010, 0b1100_0001]);
        assert_eq!(br.read_bits(3).unwrap(), 0b101);
        assert_eq!(br.read_bits(5).unwrap(), 0b10010);
    }

    #[test]
    fn read_bit_matches_read_bits() {
        // Reading 8 single bits must reproduce the byte 0b1011_0010 MSB-first.
        let mut br = BitReader::new(&[0b1011_0010, 0x00]);
        let expected = [1u32, 0, 1, 1, 0, 0, 1, 0];
        for (i, e) in expected.iter().enumerate() {
            assert_eq!(br.read_bit().unwrap(), *e, "bit {i}");
        }
    }

    #[test]
    fn reverse_reader_reads_from_high_end() {
        // The reverse reader loads from the end: top byte is the last byte, 0b1010_0000,
        // so the first 3 bits read MSB-first are 101.
        let mut br = BitReaderRev::new(&[0x00, 0x00, 0b1010_0000]);
        assert_eq!(br.read_bits(3).unwrap(), 0b101);
    }

    #[test]
    fn reverse_reader_reads_consecutive_bits() {
        // Last byte 0b1100_1011 read MSB-first across two reads: 110 then 01011.
        let mut br = BitReaderRev::new(&[0x00, 0b1100_1011]);
        assert_eq!(br.read_bits(3).unwrap(), 0b110);
        assert_eq!(br.read_bits(5).unwrap(), 0b01011);
    }

    #[test]
    fn read_bits_zero_returns_zero_without_consuming() {
        let mut br = BitReader::new(&[0xFF, 0xFF]);
        assert_eq!(br.read_bits(0).unwrap(), 0);
        assert_eq!(br.read_bits(4).unwrap(), 0b1111);
    }

    #[test]
    fn writer_then_reader_roundtrips_widths_1_to_24() {
        // A sequence of (value, width) pairs, widths 1..=24, written MSB-first then read back.
        let pairs: &[(u32, u32)] = &[
            (0b1, 1),
            (0b10, 2),
            (0b101, 3),
            (0b1_0010, 5),
            (0xAB, 8),
            (0x1FF, 9),
            (0xBEEF, 16),
            (0x12_3456, 21),
            (0xAB_CDEF, 24),
            (0, 24),
            (0xFF_FFFF, 24),
            (0b1011, 4),
        ];
        let mut w = BitWriter::new();
        for &(v, n) in pairs {
            w.write_bits(v, n);
        }
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        for &(v, n) in pairs {
            let mask = if n == 32 { u32::MAX } else { (1u32 << n) - 1 };
            assert_eq!(r.read_bits(n).unwrap(), v & mask, "value {v:#x} width {n}");
        }
    }

    #[test]
    fn over_read_past_end_is_truncated_eventually() {
        // Zero-fill slack is bounded: reading far past the end must error rather than
        // returning zeros forever or panicking.
        let mut br = BitReader::new(&[0xFF]);
        let mut saw_truncated = false;
        for _ in 0..64 {
            match br.read_bits(8) {
                Ok(_) => {}
                Err(Error::Truncated) => {
                    saw_truncated = true;
                    break;
                }
                Err(e) => panic!("unexpected error {e:?}"),
            }
        }
        assert!(
            saw_truncated,
            "expected Truncated after exhausting overrun budget"
        );
    }
}
