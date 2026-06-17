//! Embedded codec self-test vectors. The compressed sample is a real-Oodle
//! Kraken block; decoding it (and a compress->decode round-trip) proves the
//! in-process codec works without any game install.
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sha1::{Digest, Sha1};

const SAMPLE_B64: &str = include_str!("codec_calibration_block.b64");
const SAMPLE_SHA1: &str = "ac98ade89e3d7417584bc0aa8036a56d31d4e285";
const SAMPLE_SIZE: usize = 4096;

/// Deterministic 4 KiB buffer used for the compress round-trip self-test.
pub fn compress_input() -> Vec<u8> {
    (0..SAMPLE_SIZE as u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 13) as u8)
        .collect()
}

pub fn decode_self_test() -> bool {
    let Ok(comp) = BASE64.decode(SAMPLE_B64.trim()) else {
        return false;
    };
    let Ok(out) = goresave_oodle::kraken_decompress(&comp, SAMPLE_SIZE) else {
        return false;
    };
    out.len() == SAMPLE_SIZE && hex::encode(Sha1::digest(&out)) == SAMPLE_SHA1
}

pub fn compress_roundtrip_self_test() -> bool {
    let input = compress_input();
    let Ok(comp) = goresave_oodle::kraken_compress(&input, 5) else {
        return false;
    };
    let Ok(back) = goresave_oodle::kraken_decompress(&comp, input.len()) else {
        return false;
    };
    back == input
}
