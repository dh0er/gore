//! Safe in-process Oodle codec backed by the vendored ooz C++ sources.

use std::fmt;

unsafe extern "C" {
    fn goresave_ooz_decompress(
        src: *const u8,
        src_len: usize,
        dst: *mut u8,
        dst_len: usize,
    ) -> i32;
    fn goresave_ooz_compress_kraken(
        src: *const u8,
        src_len: i32,
        dst: *mut u8,
        level: i32,
    ) -> i32;
}

/// ooz writes a little past the logical end during decode; mirror the upstream
/// CLI's SAFE_SPACE padding so the decode buffer can never overrun.
const DECODE_SAFE_PADDING: usize = 64;

/// ooz's Kraken encoder crashes at levels >= 6; cap to the best safe level.
pub const MAX_SAFE_COMPRESS_LEVEL: u8 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OodleError {
    Decompress { expected: usize, got: i32 },
    Compress { got: i32 },
    InputTooLarge(usize),
}

impl fmt::Display for OodleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OodleError::Decompress { expected, got } => {
                write!(f, "oodle decompress returned {got}, expected {expected} bytes")
            }
            OodleError::Compress { got } => write!(f, "oodle compress returned {got}"),
            OodleError::InputTooLarge(n) => write!(f, "oodle input too large: {n} bytes"),
        }
    }
}

impl std::error::Error for OodleError {}

/// Decode an Oodle block whose decompressed length is exactly `expected_size`.
pub fn kraken_decompress(src: &[u8], expected_size: usize) -> Result<Vec<u8>, OodleError> {
    let mut dst = vec![0u8; expected_size + DECODE_SAFE_PADDING];
    let got = unsafe {
        goresave_ooz_decompress(src.as_ptr(), src.len(), dst.as_mut_ptr(), expected_size)
    };
    if got < 0 || got as usize != expected_size {
        return Err(OodleError::Decompress { expected: expected_size, got });
    }
    dst.truncate(expected_size);
    Ok(dst)
}

/// Kraken-encode `src`. `level` is clamped to [`MAX_SAFE_COMPRESS_LEVEL`].
pub fn kraken_compress(src: &[u8], level: u8) -> Result<Vec<u8>, OodleError> {
    let src_len = i32::try_from(src.len()).map_err(|_| OodleError::InputTooLarge(src.len()))?;
    let capacity = src.len() + 0x10000; // worst-case expansion headroom
    let mut dst = vec![0u8; capacity];
    let level = level.min(MAX_SAFE_COMPRESS_LEVEL) as i32;
    let got = unsafe {
        goresave_ooz_compress_kraken(src.as_ptr(), src_len, dst.as_mut_ptr(), level)
    };
    if got <= 0 || got as usize > capacity {
        return Err(OodleError::Compress { got });
    }
    dst.truncate(got as usize);
    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kraken_roundtrip_recovers_input() {
        let input: Vec<u8> = (0..8192u32).map(|i| (i.wrapping_mul(31) >> 3) as u8).collect();
        let comp = kraken_compress(&input, 5).unwrap();
        assert!(comp.len() < input.len());
        let back = kraken_decompress(&comp, input.len()).unwrap();
        assert_eq!(back, input);
    }

    #[test]
    fn compress_clamps_unsafe_level_instead_of_crashing() {
        let input: Vec<u8> = (0..4096u32).map(|i| (i * 7) as u8).collect();
        // Level 6 would crash the raw ooz encoder; the wrapper must clamp to 5.
        let comp = kraken_compress(&input, 6).unwrap();
        let back = kraken_decompress(&comp, input.len()).unwrap();
        assert_eq!(back, input);
    }

    #[test]
    fn decompress_rejects_wrong_expected_size() {
        let input: Vec<u8> = (0..4096u32).map(|i| (i * 7) as u8).collect();
        let comp = kraken_compress(&input, 5).unwrap();
        // Feed only the first 16 bytes of the compressed stream; the decoder
        // cannot produce the full 4096 bytes from a truncated block.
        let truncated = &comp[..16];
        let err = kraken_decompress(truncated, input.len()).unwrap_err();
        assert!(matches!(err, OodleError::Decompress { .. }));
    }
}
