//! Little-endian byte cursor: a bounds-checked reader and a `Vec`-backed writer.
//!
//! Oodle stores sizes little-endian, including 24-bit ones, so the reader exposes
//! `u8/u16/u24/u32` little-endian accessors and the writer the matching pushes. Reads
//! past the end return [`Error::Truncated`] rather than panicking.

#![allow(dead_code)]

use crate::Error;
use alloc::vec::Vec;

/// A forward, bounds-checked little-endian reader over a borrowed byte slice.
pub(crate) struct ByteReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        ByteReader { buf, pos: 0 }
    }

    /// Bytes not yet consumed.
    pub(crate) fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Current read offset from the start of the buffer.
    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    /// Borrow the next `n` bytes and advance. [`Error::Truncated`] if fewer than `n` remain.
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated)?;
        if end > self.buf.len() {
            return Err(Error::Truncated);
        }
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u16_le(&mut self) -> Result<u16, Error> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }

    /// Read a 24-bit little-endian value into the low three bytes of a `u32`.
    pub(crate) fn u24_le(&mut self) -> Result<u32, Error> {
        let s = self.take(3)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], 0]))
    }

    pub(crate) fn u32_le(&mut self) -> Result<u32, Error> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
}

/// A growable little-endian byte writer backed by `alloc::vec::Vec`.
pub(crate) struct ByteWriter {
    buf: Vec<u8>,
}

impl ByteWriter {
    pub(crate) fn new() -> Self {
        ByteWriter { buf: Vec::new() }
    }

    pub(crate) fn with_capacity(n: usize) -> Self {
        ByteWriter {
            buf: Vec::with_capacity(n),
        }
    }

    /// Wrap an existing `Vec` (typically one the caller pre-reserved with a fallible
    /// allocation) as the writer's backing buffer. The encoder uses this so the worst-case
    /// output capacity is reserved via `try_reserve` rather than the infallible
    /// `with_capacity`, keeping a hostile size from aborting the process.
    pub(crate) fn from_vec(buf: Vec<u8>) -> Self {
        ByteWriter { buf }
    }

    pub(crate) fn push_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub(crate) fn push_u16_le(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Push the low three bytes of `v` little-endian.
    pub(crate) fn push_u24_le(&mut self, v: u32) {
        let b = v.to_le_bytes();
        self.buf.extend_from_slice(&b[..3]);
    }

    pub(crate) fn push_u32_le(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn extend(&mut self, s: &[u8]) {
        self.buf.extend_from_slice(s);
    }

    pub(crate) fn len(&self) -> usize {
        self.buf.len()
    }

    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_le_widths_and_advances() {
        let mut r = ByteReader::new(&[0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(r.u8().unwrap(), 0x01);
        assert_eq!(r.u16_le().unwrap(), 0x0302);
        assert_eq!(r.u16_le().unwrap(), 0x0504);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn u24_le_reads_three_bytes() {
        let mut r = ByteReader::new(&[0xAA, 0xBB, 0xCC]);
        assert_eq!(r.u24_le().unwrap(), 0x00CC_BBAA);
    }

    #[test]
    fn short_read_is_truncated() {
        let mut r = ByteReader::new(&[0x01]);
        assert_eq!(r.u32_le(), Err(crate::Error::Truncated));
    }

    #[test]
    fn take_tracks_pos_and_remaining() {
        let mut r = ByteReader::new(&[0, 1, 2, 3]);
        assert_eq!(r.pos(), 0);
        assert_eq!(r.take(3).unwrap(), &[0, 1, 2]);
        assert_eq!(r.pos(), 3);
        assert_eq!(r.remaining(), 1);
        assert_eq!(r.take(2), Err(crate::Error::Truncated));
    }

    #[test]
    fn writer_roundtrips_widths() {
        let mut w = ByteWriter::new();
        w.push_u8(0x01);
        w.push_u16_le(0x0302);
        w.push_u24_le(0x0C0B0A);
        w.push_u32_le(0x04030201);
        assert_eq!(w.len(), 1 + 2 + 3 + 4);
        let v = w.into_vec();
        let mut r = ByteReader::new(&v);
        assert_eq!(r.u8().unwrap(), 0x01);
        assert_eq!(r.u16_le().unwrap(), 0x0302);
        assert_eq!(r.u24_le().unwrap(), 0x0C0B0A);
        assert_eq!(r.u32_le().unwrap(), 0x04030201);
        assert_eq!(r.remaining(), 0);
    }
}
