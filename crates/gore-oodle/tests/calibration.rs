//! Locks decode compatibility against a real-Oodle-produced Kraken block.
//!
//! `calibration_block.b64` is a 4096-byte block produced by the real Oodle Kraken encoder.
//! Decoding it to its known SHA-1 proves the pure-Rust decode path is bit-exact.
use gore_oodle::decompress;
use sha1::{Digest, Sha1};

#[test]
fn decodes_real_oodle_block_to_expected_sha1() {
    use base64::Engine;
    let comp = base64::engine::general_purpose::STANDARD
        .decode(include_str!("calibration_block.b64").trim())
        .unwrap();
    let out = decompress(&comp, 4096).unwrap();
    assert_eq!(
        hex::encode(Sha1::digest(&out)),
        "ac98ade89e3d7417584bc0aa8036a56d31d4e285"
    );
}
