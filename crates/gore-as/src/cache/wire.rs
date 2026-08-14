//! UE `FArchive` wire primitives for the precompiled-cache format.
//!
//! Critical rule (see `work/reversing/gore-as/findings/container-splice.md` §0):
//! UE serializes a `bool` as a **4-byte int32**, not 1 byte. Two string encodings:
//! - `FStringInArchive` (SIA): `int32 Length`(chars); if `Length>0` read `Length+1`
//!   bytes (incl trailing NUL). Empty = just the 4-byte `0`.
//! - UE `FString`: `int32 Len`(= chars+1, incl NUL); then `Len` bytes. Used only for
//!   the `Modules` TMap key.

use thiserror::Error;

const MAX_SERIALIZED_STRING_UNITS: usize = 1_000_000;

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
    #[error("cyclic TypeReferences subtype graph at key {key:#x}")]
    CyclicTypeReference { key: i64 },
    #[error("TypeReferences subtype nesting exceeds {max} levels at key {key:#x}")]
    TypeReferenceDepth { key: i64, max: usize },
    #[error("portable symbol identity for key {key:#x} exceeds the {max}-byte limit")]
    IdentityTooLarge { key: i64, max: usize },
    #[error("portable symbol identities exceed the cache-wide {max}-byte construction budget")]
    IdentityBudgetExceeded { max: usize },
    #[error("portable symbol identity matching exceeds the cache-wide {max}-byte work budget")]
    IdentityComparisonBudgetExceeded { max: usize },
    #[error("DataType in symbol row {key:#x} is not a canonical runtime type ({detail})")]
    InvalidDataType { key: i64, detail: &'static str },
    #[error("TypeReference row {key:#x} is not canonical ({detail})")]
    InvalidTypeReference { key: i64, detail: &'static str },
    #[error("FunctionReference row {key:#x} is not canonical ({detail})")]
    InvalidFunctionReference { key: i64, detail: &'static str },
    #[error("invalid FStringInArchive at pos {pos} ({detail})")]
    InvalidSia { pos: usize, detail: &'static str },
    #[error("invalid FString at pos {pos} ({detail})")]
    InvalidFString { pos: usize, detail: &'static str },
}

/// Decode Unreal's ANSI payload without collapsing distinct input bytes. Windows builds use the
/// Windows-1252 code page for `TCHAR_TO_ANSI`; the five undefined CP-1252 slots deliberately map
/// to their matching C1 controls so all 256 byte values remain one-to-one.
fn decode_windows_1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| match byte {
            0x80 => '\u{20ac}',
            0x81 => '\u{0081}',
            0x82 => '\u{201a}',
            0x83 => '\u{0192}',
            0x84 => '\u{201e}',
            0x85 => '\u{2026}',
            0x86 => '\u{2020}',
            0x87 => '\u{2021}',
            0x88 => '\u{02c6}',
            0x89 => '\u{2030}',
            0x8a => '\u{0160}',
            0x8b => '\u{2039}',
            0x8c => '\u{0152}',
            0x8d => '\u{008d}',
            0x8e => '\u{017d}',
            0x8f => '\u{008f}',
            0x90 => '\u{0090}',
            0x91 => '\u{2018}',
            0x92 => '\u{2019}',
            0x93 => '\u{201c}',
            0x94 => '\u{201d}',
            0x95 => '\u{2022}',
            0x96 => '\u{2013}',
            0x97 => '\u{2014}',
            0x98 => '\u{02dc}',
            0x99 => '\u{2122}',
            0x9a => '\u{0161}',
            0x9b => '\u{203a}',
            0x9c => '\u{0153}',
            0x9d => '\u{009d}',
            0x9e => '\u{017e}',
            0x9f => '\u{0178}',
            other => char::from(other),
        })
        .collect()
}

/// Raw, validated FStringInArchive payload. Most fields are `TCHAR_TO_ANSI` and use the normal
/// Windows-1252 projection; script-literal GlobalReferences explicitly use `AssignAsUTF8` and
/// must decode the same bytes as UTF-8 after their `bIsString` discriminator is known.
pub struct SiaBytes<'a> {
    raw: &'a [u8],
}

impl SiaBytes<'_> {
    pub fn decode_ansi(&self) -> String {
        decode_windows_1252(self.raw)
    }

    pub fn decode_utf8(&self, pos: usize) -> Result<String, WireError> {
        std::str::from_utf8(self.raw)
            .map(str::to_owned)
            .map_err(|_| WireError::InvalidSia {
                pos,
                detail: "script string literal is not valid UTF-8",
            })
    }

    pub fn len(&self) -> usize {
        self.raw.len()
    }
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

    /// A serialized UE `bool` must be the canonical int32 0/1 representation. Accepting other
    /// nonzero values would let byte-distinct rows share the same runtime meaning.
    pub fn read_bool4(&mut self) -> Result<bool, WireError> {
        let pos = self.pos;
        match self.read_i32()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(WireError::BadLen {
                pos,
                len: value as i64,
                field: "bool",
            }),
        }
    }

    /// `FStringInArchive`: custom ANSI-only `int32 Length(chars)` followed by `Length+1` bytes,
    /// including exactly one trailing NUL. This is not UE's general UTF-16-capable FString.
    pub fn read_sia(&mut self) -> Result<String, WireError> {
        Ok(self.read_sia_bytes()?.decode_ansi())
    }

    /// Validate and return one SIA payload without assuming its contextual character encoding.
    pub fn read_sia_bytes(&mut self) -> Result<SiaBytes<'a>, WireError> {
        let start = self.pos;
        let len = self.read_i32()?;
        match len.cmp(&0) {
            std::cmp::Ordering::Equal => Ok(SiaBytes { raw: &[] }),
            std::cmp::Ordering::Greater => {
                if len as usize > MAX_SERIALIZED_STRING_UNITS {
                    return Err(WireError::BadLen {
                        pos: start,
                        len: len as i64,
                        field: "FStringInArchive",
                    });
                }
                let n = len as usize + 1; // +1 trailing NUL
                let raw = self.take(n)?;
                if raw.last() != Some(&0) {
                    return Err(WireError::InvalidSia {
                        pos: start,
                        detail: "missing trailing NUL",
                    });
                }
                let content = &raw[..raw.len() - 1];
                if content.contains(&0) {
                    return Err(WireError::InvalidSia {
                        pos: start,
                        detail: "embedded NUL",
                    });
                }
                Ok(SiaBytes { raw: content })
            }
            std::cmp::Ordering::Less => Err(WireError::InvalidSia {
                pos: start,
                detail: "negative/UTF-16 length is not supported by FStringInArchive",
            }),
        }
    }

    /// UE `FString`: int32 Len(= chars+1, incl NUL); then Len bytes.
    /// Negative = UTF-16 (`-Len` code units incl NUL terminator).
    pub fn read_fstring(&mut self) -> Result<String, WireError> {
        let start = self.pos;
        let len = self.read_i32()?;
        match len.cmp(&0) {
            std::cmp::Ordering::Equal => Ok(String::new()),
            std::cmp::Ordering::Greater => {
                if len as usize > MAX_SERIALIZED_STRING_UNITS + 1 {
                    return Err(WireError::BadLen {
                        pos: start,
                        len: len as i64,
                        field: "FString",
                    });
                }
                let raw = self.take(len as usize)?;
                if raw.last() != Some(&0) {
                    return Err(WireError::InvalidFString {
                        pos: start,
                        detail: "missing trailing NUL",
                    });
                }
                let content = &raw[..raw.len() - 1];
                if content.contains(&0) {
                    return Err(WireError::InvalidFString {
                        pos: start,
                        detail: "embedded NUL",
                    });
                }
                Ok(decode_windows_1252(content))
            }
            std::cmp::Ordering::Less => self.read_utf16_fstring(start, -(len as i64)),
        }
    }

    fn read_utf16_fstring(
        &mut self,
        start: usize,
        total_code_units: i64,
    ) -> Result<String, WireError> {
        if !(1..=MAX_SERIALIZED_STRING_UNITS as i64 + 1).contains(&total_code_units) {
            return Err(WireError::BadLen {
                pos: start,
                len: -total_code_units,
                field: "utf16",
            });
        }
        let raw = self.take(total_code_units as usize * 2)?;
        let units: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        if units.last() != Some(&0) {
            return Err(WireError::InvalidFString {
                pos: start,
                detail: "missing trailing UTF-16 NUL",
            });
        }
        let content = &units[..units.len() - 1];
        if content.contains(&0) {
            return Err(WireError::InvalidFString {
                pos: start,
                detail: "embedded UTF-16 NUL",
            });
        }
        String::from_utf16(content).map_err(|_| WireError::InvalidFString {
            pos: start,
            detail: "invalid UTF-16",
        })
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

#[cfg(test)]
mod tests {
    use super::{Cursor, WireError};

    #[test]
    fn ansi_strings_are_canonical_and_windows_1252_is_injective() {
        let sia = [1i32.to_le_bytes().as_slice(), &[0xe4, 0]].concat();
        assert_eq!(Cursor::new(&sia).read_sia().unwrap(), "ä");

        let ansi = [3i32.to_le_bytes().as_slice(), &[0x80, 0x81, 0]].concat();
        assert_eq!(Cursor::new(&ansi).read_fstring().unwrap(), "€\u{81}");

        for (raw, expected) in [
            (
                [1i32.to_le_bytes().as_slice(), b"x"].concat(),
                WireError::InvalidFString {
                    pos: 0,
                    detail: "missing trailing NUL",
                },
            ),
            (
                [2i32.to_le_bytes().as_slice(), &[0, 0]].concat(),
                WireError::InvalidFString {
                    pos: 0,
                    detail: "embedded NUL",
                },
            ),
        ] {
            assert_eq!(Cursor::new(&raw).read_fstring(), Err(expected));
        }
    }

    #[test]
    fn utf16_fstrings_require_one_terminal_nul_and_valid_units() {
        let valid = [
            (-2i32).to_le_bytes().as_slice(),
            &('ä' as u16).to_le_bytes(),
            &0u16.to_le_bytes(),
        ]
        .concat();
        assert_eq!(Cursor::new(&valid).read_fstring().unwrap(), "ä");

        let missing_nul = [
            (-2i32).to_le_bytes().as_slice(),
            &('a' as u16).to_le_bytes(),
            &('b' as u16).to_le_bytes(),
        ]
        .concat();
        assert!(matches!(
            Cursor::new(&missing_nul).read_fstring(),
            Err(WireError::InvalidFString {
                detail: "missing trailing UTF-16 NUL",
                ..
            })
        ));

        let embedded_nul = [
            (-3i32).to_le_bytes().as_slice(),
            &('a' as u16).to_le_bytes(),
            &0u16.to_le_bytes(),
            &0u16.to_le_bytes(),
        ]
        .concat();
        assert!(matches!(
            Cursor::new(&embedded_nul).read_fstring(),
            Err(WireError::InvalidFString {
                detail: "embedded UTF-16 NUL",
                ..
            })
        ));

        let invalid_surrogate = [
            (-2i32).to_le_bytes().as_slice(),
            &0xd800u16.to_le_bytes(),
            &0u16.to_le_bytes(),
        ]
        .concat();
        assert!(matches!(
            Cursor::new(&invalid_surrogate).read_fstring(),
            Err(WireError::InvalidFString {
                detail: "invalid UTF-16",
                ..
            })
        ));
    }
}
