//! Outer header of `PrecompiledScript_Shipping.Cache`.
//!
//! Layout (little-endian):
//! - `0x00..0x10`  16-byte validation hash
//! - `0x10..0x14`  u32 magic = `0x9e377abe`
//! - `0x14..0x18`  u32 type/entry count
//! - `0x18..`      per-type records (see `cache::scan` / follow-up decode)

use thiserror::Error;

/// Magic at offset `0x10` of the precompiled cache.
pub const CACHE_MAGIC: u32 = 0x9e37_7abe;

/// Parsed outer header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheHeader {
    /// 16-byte validation hash (`0x00..0x10`).
    pub hash: [u8; 16],
    /// Magic word; must equal [`CACHE_MAGIC`].
    pub magic: u32,
    /// Number of type/entry records that follow the header.
    pub type_count: u32,
}

/// Errors from [`CacheHeader::parse`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HeaderError {
    #[error("cache too short: need {need} bytes, got {got}")]
    TooShort { need: usize, got: usize },
    #[error("bad cache magic: got {got:#010x}, expected {expected:#010x}")]
    BadMagic { got: u32, expected: u32 },
}

impl CacheHeader {
    /// Byte length of the outer header.
    pub const SIZE: usize = 24;

    /// Parse the outer header from the start of the cache bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, HeaderError> {
        if bytes.len() < Self::SIZE {
            return Err(HeaderError::TooShort {
                need: Self::SIZE,
                got: bytes.len(),
            });
        }
        let mut hash = [0u8; 16];
        hash.copy_from_slice(&bytes[0..16]);
        let magic = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let type_count = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
        if magic != CACHE_MAGIC {
            return Err(HeaderError::BadMagic {
                got: magic,
                expected: CACHE_MAGIC,
            });
        }
        Ok(CacheHeader {
            hash,
            magic,
            type_count,
        })
    }
}
