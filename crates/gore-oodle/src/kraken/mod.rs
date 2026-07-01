//! Kraken quantum stage: framing (`block`), decode (`decode`), encode (`encode`).

#![allow(dead_code)]

mod block;
mod decode;
mod encode;

use alloc::vec::Vec;

pub(crate) fn decompress(src: &[u8], len: usize) -> crate::Result<Vec<u8>> {
    decode::decompress(src, len)
}

pub(crate) fn compress(src: &[u8], level: crate::Level) -> crate::Result<Vec<u8>> {
    encode::compress(src, level)
}
