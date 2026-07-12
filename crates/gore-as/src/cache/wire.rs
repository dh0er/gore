//! UE `FArchive` wire primitives for the precompiled-cache format.
//!
//! Critical rule (see `work/reversing/gore-as/findings/container-splice.md` §0):
//! UE serializes a `bool` as a **4-byte int32**, not 1 byte. Two string encodings:
//! - `FStringInArchive` (SIA): `int32 Length`(chars); if `Length>0` read `Length+1`
//!   bytes (incl trailing NUL). Empty = just the 4-byte `0`.
//! - UE `FString`: `int32 Len`(= chars+1, incl NUL); then `Len` bytes. Used only for
//!   the `Modules` TMap key.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WireError {
    #[error("unexpected end of data at pos {pos}: needed {need} more bytes, have {have}")]
    Eof {
        pos: usize,
        need: usize,
        have: usize,
    },
    #[error("implausible length {len} at pos {pos} (field {field})")]
    BadLen {
        pos: usize,
        len: i64,
        field: &'static str,
    },
}

/// A little-endian byte cursor over a cache buffer.
pub struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Cursor { buf, pos: 0 }
    }

    pub fn at(buf: &'a [u8], pos: usize) -> Self {
        Cursor { buf, pos }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    /// Prove that a serialized count has enough backing bytes for even its smallest possible
    /// elements before a caller loops or allocates from it. Variable-width records pass their
    /// format minimum; fixed-width records pass their exact width.
    pub fn ensure_minimum_remaining(
        &self,
        count: usize,
        minimum_element_bytes: usize,
        field: &'static str,
    ) -> Result<(), WireError> {
        let need = count
            .checked_mul(minimum_element_bytes)
            .ok_or(WireError::BadLen {
                pos: self.pos.saturating_sub(4),
                len: count as i64,
                field,
            })?;
        if need > self.remaining() {
            return Err(WireError::Eof {
                pos: self.pos,
                need,
                have: self.remaining(),
            });
        }
        Ok(())
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        if self.pos + n > self.buf.len() {
            return Err(WireError::Eof {
                pos: self.pos,
                need: n,
                have: self.remaining(),
            });
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    /// Advance `n` bytes without reading them.
    pub fn skip(&mut self, n: usize) -> Result<(), WireError> {
        self.take(n).map(|_| ())
    }

    pub fn read_i32(&mut self) -> Result<i32, WireError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn read_u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn read_i64(&mut self) -> Result<i64, WireError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    pub fn read_u64(&mut self) -> Result<u64, WireError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    /// A UE `bool` = 4-byte int32; any nonzero is `true`.
    pub fn read_bool4(&mut self) -> Result<bool, WireError> {
        Ok(self.read_i32()? != 0)
    }

    /// `FStringInArchive`: int32 Length(chars); if Length>0 read Length+1 bytes (incl NUL).
    /// Negative length = UTF-16 (`-Length` code units, 2 bytes each).
    pub fn read_sia(&mut self) -> Result<String, WireError> {
        let len = self.read_i32()?;
        match len.cmp(&0) {
            std::cmp::Ordering::Equal => Ok(String::new()),
            std::cmp::Ordering::Greater => {
                let n = len as usize + 1; // +1 trailing NUL
                let raw = self.take(n)?;
                Ok(String::from_utf8_lossy(&raw[..raw.len().saturating_sub(1)]).into_owned())
            }
            std::cmp::Ordering::Less => self.read_utf16(-(len as i64)),
        }
    }

    /// UE `FString`: int32 Len(= chars+1, incl NUL); then Len bytes.
    /// Negative = UTF-16 (`-Len` code units incl NUL terminator).
    pub fn read_fstring(&mut self) -> Result<String, WireError> {
        let len = self.read_i32()?;
        match len.cmp(&0) {
            std::cmp::Ordering::Equal => Ok(String::new()),
            std::cmp::Ordering::Greater => {
                let raw = self.take(len as usize)?;
                Ok(String::from_utf8_lossy(&raw[..raw.len().saturating_sub(1)]).into_owned())
            }
            std::cmp::Ordering::Less => self.read_utf16((-(len as i64)).saturating_sub(1)),
        }
    }

    fn read_utf16(&mut self, code_units: i64) -> Result<String, WireError> {
        if !(0..=1_000_000).contains(&code_units) {
            return Err(WireError::BadLen {
                pos: self.pos,
                len: code_units,
                field: "utf16",
            });
        }
        let n = code_units as usize * 2;
        let raw = self.take(n + 2)?; // +2 for the wide NUL terminator
        let units: Vec<u16> = raw[..n]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        Ok(String::from_utf16_lossy(&units))
    }

    /// `TArray<T>` header: read int32 count, sanity-check against `field`.
    pub fn read_count(&mut self, field: &'static str) -> Result<usize, WireError> {
        let n = self.read_i32()?;
        // A count this large is a desync, not real data.
        if !(0..=50_000_000).contains(&n) {
            return Err(WireError::BadLen {
                pos: self.pos - 4,
                len: n as i64,
                field,
            });
        }
        Ok(n as usize)
    }

    /// Skip a `TArray<T>` whose element is a fixed `elem` bytes wide.
    pub fn skip_tarray_fixed(&mut self, elem: usize, field: &'static str) -> Result<(), WireError> {
        let n = self.read_count(field)?;
        self.skip(n * elem)
    }

    /// Skip a `TArray<FStringInArchive>`.
    pub fn skip_tarray_sia(&mut self, field: &'static str) -> Result<(), WireError> {
        let n = self.read_count(field)?;
        for _ in 0..n {
            self.read_sia()?;
        }
        Ok(())
    }

    /// Read a `TArray<int32>` into a Vec (used to capture function bytecode).
    pub fn read_tarray_i32(&mut self, field: &'static str) -> Result<Vec<i32>, WireError> {
        let n = self.read_count(field)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.read_i32()?);
        }
        Ok(v)
    }

    /// Read a `TArray<FStringInArchive>` into a Vec (used to capture parameter names).
    pub fn read_tarray_sia(&mut self, field: &'static str) -> Result<Vec<String>, WireError> {
        let n = self.read_count(field)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.read_sia()?);
        }
        Ok(v)
    }
}
