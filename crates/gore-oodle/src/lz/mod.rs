//! LZ stage: turn raw bytes into the token stream the Kraken encoder serialises.

#![allow(dead_code)]

mod match_finder;
mod parse;

use alloc::vec::Vec;

/// One LZ token. `offset == 0` marks a final literal run (no match copy).
#[derive(Clone, Copy)]
pub(crate) struct Token {
    pub lit_len: u32,
    pub offset: u32,
    pub match_len: u32,
}

/// Produce the final token stream covering `src` at the given effort `level`.
pub(crate) fn find_tokens(src: &[u8], level: crate::Level) -> Vec<Token> {
    parse::parse(src, level)
}
