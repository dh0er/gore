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
    /// The requested decode output buffer could not be sized/allocated (the
    /// declared uncompressed size overflowed or exceeded available memory).
    OutputTooLarge(usize),
}

impl fmt::Display for OodleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OodleError::Decompress { expected, got } => {
                write!(f, "oodle decompress returned {got}, expected {expected} bytes")
            }
            OodleError::Compress { got } => write!(f, "oodle compress returned {got}"),
            OodleError::InputTooLarge(n) => write!(f, "oodle input too large: {n} bytes"),
            OodleError::OutputTooLarge(n) => {
                write!(f, "oodle output size unallocatable: {n} bytes")
            }
        }
    }
}

impl std::error::Error for OodleError {}

/// Decode an Oodle block whose decompressed length is exactly `expected_size`.
///
/// `expected_size` comes from the (untrusted) save container, so the output
/// buffer is sized with a checked add and a fallible allocation: a corrupt or
/// hostile size returns an error instead of overflowing into a tiny buffer (and
/// then letting the C++ decoder write `expected_size` bytes into it) or aborting
/// the process on a giant allocation.
pub fn kraken_decompress(src: &[u8], expected_size: usize) -> Result<Vec<u8>, OodleError> {
    let capacity = expected_size
        .checked_add(DECODE_SAFE_PADDING)
        .ok_or(OodleError::OutputTooLarge(expected_size))?;
    let mut dst: Vec<u8> = Vec::new();
    dst.try_reserve_exact(capacity)
        .map_err(|_| OodleError::OutputTooLarge(expected_size))?;
    dst.resize(capacity, 0);
    let got = unsafe {
        goresave_ooz_decompress(src.as_ptr(), src.len(), dst.as_mut_ptr(), expected_size)
    };
    if got < 0 || got as usize != expected_size {
        return Err(OodleError::Decompress { expected: expected_size, got });
    }
    dst.truncate(expected_size);
    Ok(dst)
}

/// Oodle block length (256 KiB) and the per-block worst-case expansion ooz
/// itself budgets for. Mirrors the vendored `GetCompressedBufferSizeNeeded`
/// (`compress.cpp`): `raw + 274 * ceil(raw / 0x40000)`.
const OOZ_BLOCK_LEN: usize = 0x40000;
const OOZ_BLOCK_OVERHEAD: usize = 274;

/// Worst-case compressed size the vendored encoder may write for `raw_len`.
///
/// The C++ writes into the output buffer before Rust can check the returned
/// length, so the buffer MUST be at least this large or a large input would
/// corrupt memory. A fixed headroom underflows above ~62 MiB (239 blocks).
fn compressed_capacity(raw_len: usize) -> usize {
    raw_len + OOZ_BLOCK_OVERHEAD * raw_len.div_ceil(OOZ_BLOCK_LEN).max(1)
}

/// Kraken-encode `src`. `level` is clamped to [`MAX_SAFE_COMPRESS_LEVEL`].
pub fn kraken_compress(src: &[u8], level: u8) -> Result<Vec<u8>, OodleError> {
    let src_len = i32::try_from(src.len()).map_err(|_| OodleError::InputTooLarge(src.len()))?;
    let capacity = compressed_capacity(src.len());
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
    fn decompress_rejects_unallocatable_expected_size() {
        // A hostile/corrupt declared size must not overflow the padding add or
        // attempt a giant allocation; it returns a clean error.
        let err = kraken_decompress(&[0u8; 8], usize::MAX).unwrap_err();
        assert_eq!(err, OodleError::OutputTooLarge(usize::MAX));
        let err = kraken_decompress(&[0u8; 8], usize::MAX - 8).unwrap_err();
        assert_eq!(err, OodleError::OutputTooLarge(usize::MAX - 8));
    }

    #[test]
    fn compressed_capacity_matches_ooz_worst_case_formula() {
        // Mirrors GetCompressedBufferSizeNeeded: raw + 274 * ceil(raw/0x40000),
        // with a one-block floor so empty input still has headroom.
        assert_eq!(compressed_capacity(0), 274);
        assert_eq!(compressed_capacity(0x40000), 0x40000 + 274);
        assert_eq!(compressed_capacity(0x40000 + 1), (0x40000 + 1) + 274 * 2);
        // A fixed 64 KiB headroom underflows here; the per-block formula must not.
        let huge = 240 * 0x40000;
        assert!(compressed_capacity(huge) - huge > 0x10000);
    }

    #[test]
    fn kraken_roundtrips_multi_block_input() {
        // >256 KiB spans multiple Oodle blocks, exercising the scaled capacity.
        let input: Vec<u8> = (0..700_000u32).map(|i| (i.wrapping_mul(7) >> 2) as u8).collect();
        let comp = kraken_compress(&input, 5).unwrap();
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
