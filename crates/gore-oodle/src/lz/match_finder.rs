//! Hash-chain match finder.
//!
//! A 4-byte-hash chain finder. For each position we hash four bytes, walk a
//! bounded chain of earlier positions sharing that bucket, and keep the longest
//! match (minimum length [`MIN_MATCH`]).
//!
//! The emitted tokens cover `src` exactly: walking them (copy `lit_len`
//! literals, then copy `match_len` bytes from `offset` back in the *output*)
//! reproduces `src` byte-for-byte. See [`crate::lz`].

#![allow(dead_code)]

use crate::lz::Token;
use crate::Level;
use alloc::vec::Vec;

/// Kraken's minimum match length. Matches shorter than this are emitted as literals.
pub(crate) const MIN_MATCH: usize = 4;

/// Smallest legal match offset. Kraken's decoder copies in 64-bit chunks, so a
/// match whose offset is below 8 and whose length exceeds the offset would read
/// output bytes that are not written yet. We work around this by expanding a
/// sub-8 offset to at least 8 by repeated addition, so every emitted match is
/// decoder-legal.
pub(crate) const MIN_OFFSET: usize = 8;

/// Number of hash-bucket slots, as a power-of-two exponent. 2^18 = 262144
/// buckets keeps collisions low for the buffer sizes we compress while staying a
/// small, fixed allocation. A fixed table is simpler and adequate for v1.
const HASH_BITS: u32 = 18;
const HASH_SIZE: usize = 1 << HASH_BITS;

/// How far down a bucket's collision chain we walk before giving up. Bounds the
/// worst case to keep the finder near-linear. Higher effort levels walk deeper.
fn chain_depth(level: Level) -> u32 {
    match level {
        Level::Fastest => 16,
        Level::Fast => 32,
        Level::Default => 64,
        Level::Max => 128,
    }
}

/// Whether to attempt a one-step lazy match (defer the current match if the next
/// position has a strictly longer one). Off for [`Level::Fastest`].
fn use_lazy(level: Level) -> bool {
    !matches!(level, Level::Fastest)
}

/// Hash the four bytes at `src[pos..]` into a bucket index. Multiplicative hash
/// specialised to a 4-byte key.
#[inline]
fn hash4(src: &[u8], pos: usize) -> usize {
    let v = u32::from_le_bytes([src[pos], src[pos + 1], src[pos + 2], src[pos + 3]]);
    // 0x9E3779B1 is the 32-bit golden-ratio constant. The high bits hold the best entropy.
    ((v as u64).wrapping_mul(0x9E37_79B1) >> (32 - HASH_BITS)) as usize & (HASH_SIZE - 1)
}

/// Count how many bytes match starting at `a` and `b` in `src`, up to `limit`,
/// as a safe slice walk. `a > b` is assumed (we only ever extend a *back*-reference).
#[inline]
fn match_len_at(src: &[u8], a: usize, b: usize, limit: usize) -> usize {
    let mut n = 0;
    while n < limit && src[b + n] == src[a + n] {
        n += 1;
    }
    n
}

/// Expand `offset` to at least [`MIN_OFFSET`] by repeated addition. Returns
/// `None` if no legal multiple fits before the window start (`cur_pos`), in which
/// case the candidate must be dropped.
#[inline]
fn legalize_offset(offset: usize, cur_pos: usize) -> Option<usize> {
    if offset >= MIN_OFFSET {
        return Some(offset);
    }
    let base = offset;
    let mut o = offset;
    while o < MIN_OFFSET {
        o += base;
    }
    // The expanded offset must still point inside the already-emitted output.
    if o > cur_pos {
        None
    } else {
        Some(o)
    }
}

/// A single best match candidate at a position: its length and (legalised) offset.
#[derive(Clone, Copy)]
struct Match {
    len: usize,
    offset: usize,
}

/// Find the longest legal match for `pos`, walking up to `max_chain` chain links.
/// `head`/`prev` form the hash chain (head bucket -> most recent position; `prev[p]`
/// -> the previous position in `p`'s bucket, or `NIL`). Returns `None` if no match
/// of at least [`MIN_MATCH`] exists.
#[inline]
fn best_match(
    src: &[u8],
    pos: usize,
    head: &[u32],
    prev: &[u32],
    max_chain: u32,
    nil: u32,
) -> Option<Match> {
    let n = src.len();
    // Need at least MIN_MATCH bytes of lookahead to form a match.
    if pos + MIN_MATCH > n {
        return None;
    }
    let limit = n - pos;
    let bucket = hash4(src, pos);
    let mut cand = head[bucket];
    let mut best: Option<Match> = None;
    let mut best_len = MIN_MATCH - 1; // require strictly longer than this to accept
    let mut steps = 0;

    while cand != nil && steps < max_chain {
        let cpos = cand as usize;
        // Chain is strictly decreasing in position; this is a back-reference.
        debug_assert!(cpos < pos);
        let raw_offset = pos - cpos;

        // Cheap rejection: the byte just past our current best must match, else
        // this candidate cannot beat it.
        if best_len < limit && src[cpos + best_len] == src[pos + best_len] {
            let ml = match_len_at(src, cpos, pos, limit);
            if ml >= MIN_MATCH && ml > best_len {
                if let Some(offset) = legalize_offset(raw_offset, pos) {
                    // After legalising a sub-8 offset the match may shorten,
                    // because copying from `pos - offset` reads different bytes.
                    let eff = if offset == raw_offset {
                        ml
                    } else {
                        match_len_at(src, pos - offset, pos, limit)
                    };
                    if eff >= MIN_MATCH && eff > best_len {
                        best_len = eff;
                        best = Some(Match { len: eff, offset });
                    }
                }
            }
        }
        cand = prev[cpos];
        steps += 1;
    }
    best
}

/// Push a token for a literal run of `lit_len` followed by a match, splitting the
/// literal run if it exceeds what a single token can carry. (`lit_len` is a `u32`;
/// our buffers are far smaller, so one token always suffices — kept explicit for
/// clarity.)
#[inline]
fn push_match(out: &mut Vec<Token>, lit_len: usize, offset: usize, match_len: usize) {
    out.push(Token {
        lit_len: lit_len as u32,
        offset: offset as u32,
        match_len: match_len as u32,
    });
}

/// Hash-chain match finder over the whole of `src`.
///
/// Returns a token stream whose cover reconstruction equals `src` exactly,
/// terminated by a final literal-run token (`offset == 0, match_len == 0`) for
/// the trailing unmatched bytes. Greedy for [`Level::Fastest`]; one-step lazy
/// for higher levels.
pub(crate) fn find_raw(src: &[u8], level: Level) -> Vec<Token> {
    let n = src.len();
    let mut out: Vec<Token> = Vec::new();

    // Too short to ever form a 4-byte match: one literal run covering everything.
    if n < MIN_MATCH {
        if n > 0 {
            push_match(&mut out, n, 0, 0);
        }
        return out;
    }

    let nil = u32::MAX;
    let mut head = alloc::vec![nil; HASH_SIZE];
    // `prev[p]` is the previous position sharing p's hash bucket.
    let mut prev = alloc::vec![nil; n];

    let max_chain = chain_depth(level);
    let lazy = use_lazy(level);

    // Start of the current pending literal run.
    let mut lit_start = 0usize;
    let mut pos = 0usize;
    // Last position whose 4-byte hash we inserted into the chain, exclusive upper
    // bound; positions in `lit_start..pos` outside matches are inserted lazily.
    let last_hash_pos = n - (MIN_MATCH - 1); // positions [0, last_hash_pos) are hashable

    // Insert position `p`'s hash into the chain (head of its bucket).
    macro_rules! insert {
        ($p:expr) => {{
            let p = $p;
            if p < last_hash_pos {
                let b = hash4(src, p);
                prev[p] = head[b];
                head[b] = p as u32;
            }
        }};
    }

    while pos < last_hash_pos {
        let m = best_match(src, pos, &head, &prev, max_chain, nil);
        insert!(pos);

        let chosen = match m {
            None => {
                pos += 1;
                continue;
            }
            Some(m) => m,
        };

        // One-step lazy: if position pos+1 has a strictly longer match, defer.
        let mut start = pos;
        let mut chosen = chosen;
        if lazy && start + 1 < last_hash_pos {
            if let Some(m1) = best_match(src, start + 1, &head, &prev, max_chain, nil) {
                if m1.len > chosen.len {
                    // Emit pos as a literal, advance, take the better match.
                    insert!(start + 1);
                    start += 1;
                    chosen = m1;
                }
            }
        }

        // Emit: literals from lit_start..start, then the match.
        let lit_len = start - lit_start;
        push_match(&mut out, lit_len, chosen.offset, chosen.len);

        // Insert hashes for every position the match covers (so later matches can
        // reference into it), then skip past it. We insert every position for chain quality.
        let match_end = start + chosen.len;
        let mut q = start + 1;
        while q < match_end && q < last_hash_pos {
            insert!(q);
            q += 1;
        }
        pos = match_end;
        lit_start = match_end;
    }

    // Trailing literals (everything from lit_start to end, including the final
    // MIN_MATCH-1 unhashed tail).
    if lit_start < n {
        push_match(&mut out, n - lit_start, 0, 0);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lz::Token;

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

    fn assert_roundtrip(src: &[u8], level: Level) {
        let toks = find_raw(src, level);
        let rebuilt = cover(src, &toks);
        assert_eq!(
            rebuilt,
            src,
            "cover != src (len {}, {:?})",
            src.len(),
            level
        );
        // Every match must be decoder-legal: offset >= MIN_OFFSET and within window.
        let mut p = 0usize;
        for t in &toks {
            p += t.lit_len as usize;
            if t.match_len > 0 {
                assert!(
                    t.offset as usize >= MIN_OFFSET,
                    "offset {} < MIN_OFFSET",
                    t.offset
                );
                assert!(
                    t.offset as usize <= p,
                    "offset {} past output {}",
                    t.offset,
                    p
                );
            }
            p += t.match_len as usize;
        }
    }

    #[test]
    fn raw_reconstructs_corpus_fastest() {
        assert_roundtrip(b"", Level::Fastest);
        assert_roundtrip(b"a", Level::Fastest);
        assert_roundtrip(b"abcabcabcabc", Level::Fastest);
        assert_roundtrip(&[0u8; 5000], Level::Fastest);
        assert_roundtrip(&pseudo_random(64 * 1024, 0x1234), Level::Fastest);
        let mut rep = pseudo_random(2048, 7);
        let tail = rep.clone();
        rep.extend_from_slice(&tail); // long exact repeat
        assert_roundtrip(&rep, Level::Fastest);
    }

    #[test]
    fn raw_reconstructs_corpus_default() {
        assert_roundtrip(b"", Level::Default);
        assert_roundtrip(b"abcabcabcabc", Level::Default);
        assert_roundtrip(&[0u8; 5000], Level::Default);
        assert_roundtrip(&pseudo_random(64 * 1024, 0xBEEF), Level::Default);
    }

    #[test]
    fn raw_actually_compresses_repetitive_data() {
        // A 4 KiB block of a repeating 37-byte pattern should be covered almost
        // entirely by matches, not literals — confirms the finder is effective,
        // not merely correct. Count literal bytes across all tokens.
        let pat = b"Lorem ipsum dolor sit amet, consect!"; // 37 bytes
        let mut buf = Vec::new();
        while buf.len() < 4096 {
            buf.extend_from_slice(pat);
        }
        for level in [Level::Fastest, Level::Default] {
            let toks = find_raw(&buf, level);
            let lit_bytes: usize = toks.iter().map(|t| t.lit_len as usize).sum();
            assert!(
                lit_bytes < buf.len() / 4,
                "{:?}: {} literal bytes of {} — finder ineffective",
                level,
                lit_bytes,
                buf.len()
            );
        }
    }

    #[test]
    fn raw_finds_a_long_repeat() {
        // X ++ X with X ~20+ bytes must yield a token with match_len >= 16.
        let x = b"the quick brown fox jumps over!"; // 31 bytes
        let mut buf = x.to_vec();
        buf.extend_from_slice(x);
        let toks = find_raw(&buf, Level::Fastest);
        assert!(
            toks.iter().any(|t| t.match_len >= 16),
            "expected a match_len >= 16, got {:?}",
            toks.iter().map(|t| t.match_len).collect::<Vec<_>>()
        );
    }
}
