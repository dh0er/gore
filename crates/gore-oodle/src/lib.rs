#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
//! Pure-Rust Oodle **Kraken** codec (decode + encode), `no_std` + `alloc`.
//!
//! The crate is `no_std` by default. It links `std` only when the `std` feature is on
//! (which enables the `std::error::Error` impl) or under `cargo test`.
//!
//! The crate compiles zero C/C++ and has no build script. (A local-only, gitignored
//! `local-oracle` crate links a reference C++ decoder to cross-check the encoder
//! against genuine Oodle during development; it is not part of the tracked workspace.)

extern crate alloc;

mod bits;
mod bytes;
mod entropy;
mod error;
mod kraken;
mod lz;
mod tables;

pub use error::{Error, Level, Result};

/// Decompress a full Kraken stream whose decompressed length is `decompressed_len`
/// (read from the surrounding container — i.e. untrusted).
pub fn decompress(src: &[u8], decompressed_len: usize) -> Result<alloc::vec::Vec<u8>> {
    kraken::decompress(src, decompressed_len)
}

/// Compress `src` as a Kraken stream at the given effort `level`.
pub fn compress(src: &[u8], level: Level) -> Result<alloc::vec::Vec<u8>> {
    kraken::compress(src, level)
}
