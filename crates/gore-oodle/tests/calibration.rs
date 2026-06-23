//! Locks ooz decode compatibility against a real-Oodle-produced Kraken block.
use gore_oodle::kraken_decompress;
use sha1::{Digest, Sha1};

const REAL_OODLE_BLOCK_B64: &str = include_str!("calibration_block.b64");
const EXPECTED_SHA1: &str = "ac98ade89e3d7417584bc0aa8036a56d31d4e285";
const EXPECTED_SIZE: usize = 4096;

#[test]
fn decodes_real_oodle_block_to_expected_sha1() {
    use base64::Engine;
    let b64 = REAL_OODLE_BLOCK_B64.trim();
    let comp = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
    let out = kraken_decompress(&comp, EXPECTED_SIZE).unwrap();
    assert_eq!(out.len(), EXPECTED_SIZE);
    let sha1 = hex::encode(Sha1::digest(&out));
    assert_eq!(sha1, EXPECTED_SHA1);
}
