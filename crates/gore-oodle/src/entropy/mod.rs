//! Entropy stage: the Oodle "array" format (raw | huffman | tANS | RLE | multi-array).
//!
//! `mod.rs` owns only the seam between this stage and the Kraken stage; the algorithm
//! leaves (`huff_dec`, `huff_enc`, `tans_dec`, `tans_enc`, `array`) are each implemented
//! by a single later task so they never share a file.

#![allow(dead_code)]

mod array;
mod bitio;
mod huff_dec;
mod huff_enc;
mod hufftable;
mod rice;
mod rle_enc;
mod tans_dec;
mod tans_enc;

/// Decode one "byte array" (raw | huffman | rle | tans | multi-array) from `src`, writing
/// exactly `out.len()` bytes. Returns the number of compressed input bytes consumed.
pub(crate) fn decode_array(src: &[u8], out: &mut [u8]) -> crate::Result<usize> {
    array::decode_array(src, out)
}

/// Decode one "byte array" from `src` where the decoded size comes from the array header
/// rather than the caller. `out` is the maximum capacity (the header's decoded size must fit
/// inside it). Returns `(decoded_size, input_bytes_consumed)` with a bounded `output_size`,
/// which the Kraken quantum decoder relies on.
pub(crate) fn decode_array_capped(src: &[u8], out: &mut [u8]) -> crate::Result<(usize, usize)> {
    array::decode_array_capped(src, out)
}

/// Encode `symbols` into `dst`, choosing raw/huffman/tans by smallest size. Returns bytes written.
pub(crate) fn encode_array(
    symbols: &[u8],
    dst: &mut crate::bytes::ByteWriter,
    level: crate::Level,
) -> crate::Result<usize> {
    array::encode_array(symbols, dst, level)
}
