//! LZ parse: raw matches -> final token stream.
//!
//! For v1 this is a thin wrapper over the hash-chain match finder
//! ([`super::match_finder::find_raw`]). The finder already emits a stream whose
//! cover reconstruction equals `src` exactly (every match has length >=
//! [`super::match_finder::MIN_MATCH`] and a decoder-legal offset, terminated by a
//! final literal-run token). The Kraken encoder (Wave 2) decides per-token costs
//! and recent-offset coding, so the parser deliberately does *not* second-guess
//! match choices here.

#![allow(dead_code)]

use crate::lz::Token;
use crate::Level;
use alloc::vec::Vec;

/// Produce the final token stream covering `src` at the given effort `level`.
pub(crate) fn parse(src: &[u8], level: Level) -> Vec<Token> {
    super::match_finder::find_raw(src, level)
}

#[cfg(test)]
mod tests {
    use crate::lz::{find_tokens, Token};
    use crate::Level;
    use alloc::vec::Vec;

    /// Reconstruct the source from a token stream (the contract's cover function).
    fn cover(src: &[u8], toks: &[Token]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut p = 0usize;
        for t in toks {
            out.extend_from_slice(&src[p..p + t.lit_len as usize]);
            p += t.lit_len as usize;
            for _ in 0..t.match_len {
                let b = out[out.len() - t.offset as usize];
                out.push(b);
            }
            p += t.match_len as usize;
        }
        out
    }

    fn pseudo_random(len: usize, seed: u64) -> Vec<u8> {
        // SplitMix64 — deterministic, no external deps.
        let mut x = seed;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            x = x.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^= z >> 31;
            v.push((z & 0xFF) as u8);
        }
        v
    }

    /// Walk `find_tokens` output and confirm it covers `src` byte-for-byte.
    fn check(src: &[u8], level: Level) {
        let toks = find_tokens(src, level);
        assert_eq!(
            cover(src, &toks),
            src,
            "cover != src (len {}, {:?})",
            src.len(),
            level
        );
    }

    #[test]
    fn tokens_reconstruct_source() {
        let repeat = {
            let mut v = b"abcdefghij0123456789".to_vec(); // 20 bytes
            let x = v.clone();
            v.extend_from_slice(&x);
            v
        };
        let corpus: &[&[u8]] = &[
            b"",
            b"a",
            b"abcabcabcabc",
            &[0u8; 5000],
            &pseudo_random(64 * 1024, 0xC0FFEE),
            &repeat,
        ];
        for src in corpus {
            check(src, Level::Fastest);
        }
    }

    #[test]
    fn finds_a_long_repeat() {
        // buffer = X ++ X (X ~20+ bytes) must yield a token with match_len >= 16.
        let x = b"abcdefghij0123456789KLMNOP"; // 26 bytes
        let mut buf = x.to_vec();
        buf.extend_from_slice(x);
        let toks = find_tokens(&buf, Level::Fastest);
        assert!(
            toks.iter().any(|t| t.match_len >= 16),
            "expected a match_len >= 16, got {:?}",
            toks.iter().map(|t| t.match_len).collect::<Vec<_>>()
        );
    }

    #[test]
    fn tokens_reconstruct_source_default() {
        check(b"", Level::Default);
        check(b"abcabcabcabc", Level::Default);
        check(&[0u8; 5000], Level::Default);
        check(&pseudo_random(64 * 1024, 0xD00D), Level::Default);
        let mut rep = pseudo_random(4096, 99);
        let tail = rep.clone();
        rep.extend_from_slice(&tail);
        check(&rep, Level::Default);
    }

    #[test]
    fn final_token_is_a_literal_run() {
        // A buffer that ends in unmatchable bytes must terminate with offset==0.
        let src = pseudo_random(1000, 5);
        for level in [Level::Fastest, Level::Default] {
            let toks = find_tokens(&src, level);
            let last = toks.last().expect("non-empty input yields tokens");
            assert_eq!(last.offset, 0, "final token must be a literal run");
            assert_eq!(last.match_len, 0, "final literal run has no match");
        }
    }
}
