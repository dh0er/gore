//! Kraken encode: raw bytes → a valid Kraken stream the in-tree decoder (and real Oodle)
//! accepts.
//!
//! This is the inverse of [`super::decode`].
//!
//! ## What this encoder emits
//!
//! The output is a sequence of quanta, each producing up to [`QUANTUM_LEN`] = `0x40000`
//! decompressed bytes. Every quantum is prefixed by the 2-byte stream header (see
//! [`write_stream_header`]); the first quantum sets the keyframe bit.
//!
//! For each quantum we pick the smallest of three encodings:
//!
//! * **stream-level uncompressed** — used for a *full* `0x40000` quantum that does not
//!   shrink. A full quantum's compressed size cannot fit the 18-bit quantum-header size
//!   field (`MAX_QUANTUM_PAYLOAD` = `0x3FFFF`), so the encoder falls back to the stream
//!   header's `uncompressed` bit and copies the raw bytes with no quantum header.
//! * **stored** — a *partial* final quantum (`out_len < QUANTUM_LEN`) that does not
//!   shrink: a quantum header with `compressed_size == out_len` followed by the raw bytes.
//! * **compressed** — a quantum header ([`write_compressed_header`]) followed by a payload of
//!   one-or-more inner chunks. The quantum is split into chunks of at most [`STEP_LEN`] =
//!   `0x20000` bytes, matching the decoder's chunk loop. Each chunk is either:
//!   * an **LZ chunk** (3-byte big-endian chunk header with bit 23 *set*): the full
//!     literal/match LZ form, the inverse of the decoder's `read_lz_table` path (built by
//!     [`lz::encode_lz_chunk`]); or
//!   * an **entropy chunk** (bit 23 *clear*): a single entropy array — exactly what
//!     [`crate::entropy::encode_array`] produces (raw / huffman / tANS / RLE).
//!
//!   A *single-chunk* quantum picks the smaller of {LZ chunk, entropy chunk}. A *multi-chunk*
//!   quantum must be **homogeneous** — all chunks LZ — or it falls through to stored framing;
//!   real Oodle mis-decodes a multi-chunk quantum that mixes LZ and entropy chunks (see
//!   [`try_compress_quantum`]). The compressed form is used only when its payload is both ≤
//!   `MAX_QUANTUM_PAYLOAD` and strictly smaller than storing.
//!
//! ## Staging
//!
//! Stage 1+2 is entropy-only (raw / huffman / tANS / RLE arrays, no LZ); Stage 3 adds the LZ
//! chunk form ([`lz`]) for better ratios on data with matches. Both stages share the
//! framing above; the decoder validates every output.

#![allow(dead_code)]

use crate::bytes::ByteWriter;
use crate::{Error, Level, Result};
use alloc::vec::Vec;

use super::block::{
    write_compressed_header, write_stored_header, write_stream_header, MAX_QUANTUM_PAYLOAD,
    QUANTUM_LEN,
};

/// Kraken's stream-header codec id.
const CODEC_KRAKEN: u8 = 6;

/// Largest chunk the decoder's inner loop consumes at once (128 KiB). A 256 KiB quantum is
/// split into chunks of at most this size; the encoder mirrors that split exactly so the
/// decoder's `dst_count = min(remaining, STEP_LEN)` walk lines up with our chunk boundaries.
const STEP_LEN: usize = 0x20000;

/// Upper bound on input we will compress in one call. Kraken offsets are 32-bit and the
/// worst-case output formula below must not overflow `usize`; 4 GiB is far beyond any real
/// Oodle payload and keeps the size math comfortably in range on 64-bit targets.
const MAX_INPUT: usize = 0xFFFF_FFFF;

/// Compress `src` into a Kraken stream that decompresses back to exactly `src`.
///
/// `level` currently only tunes the entropy/LZ match effort (the framing is identical at
/// every level). Returns [`Error::InputTooLarge`] if `src` exceeds [`MAX_INPUT`], or
/// [`Error::OutputTooLarge`] if the worst-case output buffer cannot be allocated.
pub(crate) fn compress(src: &[u8], level: Level) -> Result<Vec<u8>> {
    if src.len() > MAX_INPUT {
        return Err(Error::InputTooLarge(src.len()));
    }

    // Worst-case output size: raw + 274 bytes of framing per 256 KiB block, with a one-block
    // floor so empty input still has headroom for a (degenerate) quantum.
    let blocks = src.len().div_ceil(QUANTUM_LEN).max(1);
    let worst = src
        .len()
        .checked_add(274usize.checked_mul(blocks).ok_or(Error::OutputTooLarge(src.len()))?)
        .ok_or(Error::OutputTooLarge(src.len()))?;

    // Pre-reserve the worst case with a *fallible* allocation (the size is bounded by
    // MAX_INPUT but can still be large), so a single growable buffer never reallocates
    // mid-stream and a hostile size errors instead of aborting.
    let mut backing = Vec::new();
    backing
        .try_reserve(worst)
        .map_err(|_| Error::OutputTooLarge(worst))?;
    let mut dst = ByteWriter::from_vec(backing);

    // Parse the whole input into LZ tokens *once* (offsets are whole-output relative, exactly
    // what the decoder's per-chunk loop expects); each chunk later re-segments this single
    // list to its output window rather than re-parsing, keeping the encoder near-linear.
    let tokens = crate::lz::find_tokens(src, level);
    let index = lz::TokenIndex::new(&tokens);

    // The empty input still produces a valid (zero-quantum) stream: the decoder's loop
    // `while produced < len` never executes, so no bytes are needed. Emit nothing.
    let mut produced = 0usize;
    let mut first = true;
    while produced < src.len() {
        let out_len = (src.len() - produced).min(QUANTUM_LEN);
        encode_quantum(&mut dst, src, &index, produced, out_len, first, level)?;
        produced += out_len;
        first = false;
    }

    Ok(dst.into_vec())
}

/// Encode one quantum into `dst`, choosing the smallest of compressed / stored / uncompressed.
///
/// `whole` is the entire input and `origin` is this quantum's start offset within it (matches
/// in an LZ chunk reference `whole` by absolute origin); `out_len` is the quantum's length.
fn encode_quantum(
    dst: &mut ByteWriter,
    whole: &[u8],
    index: &lz::TokenIndex,
    origin: usize,
    out_len: usize,
    keyframe: bool,
    level: Level,
) -> Result<()> {
    debug_assert!((1..=QUANTUM_LEN).contains(&out_len));
    let quantum = &whole[origin..origin + out_len];

    // Try the compressed form: a payload of inner chunks (entropy and/or LZ).
    if let Some(payload) = try_compress_quantum(whole, index, origin, level)? {
        // Only worth it if the framed compressed quantum beats storing raw.
        // Compressed framing = 3-byte quantum header + payload.
        // Stored framing = (partial) 3-byte header + out_len, or (full) 0-byte header.
        let compressed_total = 3 + payload.len();
        let stored_total = if out_len == QUANTUM_LEN { out_len } else { 3 + out_len };
        if payload.len() <= MAX_QUANTUM_PAYLOAD && compressed_total < stored_total {
            write_stream_header(dst, CODEC_KRAKEN, false, keyframe, false);
            write_compressed_header(dst, payload.len());
            dst.extend(&payload);
            return Ok(());
        }
    }

    // Fall back to a verbatim copy.
    if out_len == QUANTUM_LEN {
        // A full quantum cannot use a quantum header (size field maxes at 0x3FFFF). Use the
        // stream-level uncompressed bit and copy the raw bytes directly.
        write_stream_header(dst, CODEC_KRAKEN, false, keyframe, true);
        dst.extend(quantum);
    } else {
        // A partial final quantum stores via a normal quantum header whose compressed_size
        // equals out_len.
        write_stream_header(dst, CODEC_KRAKEN, false, keyframe, false);
        write_stored_header(dst, out_len);
        dst.extend(quantum);
    }
    Ok(())
}

/// Build the compressed payload for one quantum, or `None` if it does not shrink / does not fit
/// the quantum-header size budget.
///
/// Mirrors the decoder's chunk loop: the quantum is cut into pieces of at most [`STEP_LEN`]
/// bytes, each becoming one inner chunk. `whole` is the entire input (so an LZ chunk's matches
/// can reach back across chunk/quantum boundaries); `quantum_origin` is this quantum's start.
///
/// ## Why a multi-chunk quantum is homogeneous (all-LZ or all-raw)
///
/// Empirically (see the `lz_multichunk_*` tests),
/// real Oodle only decodes a multi-chunk quantum correctly when **every** chunk is the same
/// "size-unambiguous" kind: all LZ chunks (header bit 23 set, explicit `inner_size`) or all raw
/// arrays (chunk type 0, a plain memcpy). A multi-chunk quantum that mixes kinds — or that uses
/// a huffman/tANS/RLE array (type 1/2/3) for any chunk that isn't the sole chunk — corrupts at a
/// chunk boundary in real Oodle. (A *single*-chunk quantum may use any encoding freely.)
///
/// So: a single-chunk quantum picks the smaller of {LZ chunk, full entropy chunk}; a multi-chunk
/// quantum uses the all-LZ payload when every chunk is LZ-able, and otherwise falls through to
/// the caller's stored/uncompressed framing (an all-raw payload can never beat storing, since it
/// adds a 3-byte header per chunk on top of the raw bytes).
fn try_compress_quantum(
    whole: &[u8],
    index: &lz::TokenIndex,
    quantum_origin: usize,
    level: Level,
) -> Result<Option<Vec<u8>>> {
    let out_len = (whole.len() - quantum_origin).min(QUANTUM_LEN);
    let single_chunk = out_len <= STEP_LEN;

    let mut candidates: Vec<Vec<u8>> = Vec::new();

    // All-LZ payload (also covers the single-chunk LZ case).
    if let Some(p) = build_lz_payload(whole, index, quantum_origin, out_len, level) {
        candidates.push(p);
    }

    // The full entropy array (huffman/tANS/RLE/raw) is only oracle-safe as the *sole* chunk of
    // a quantum; a multi-chunk quantum that can't be all-LZ falls through to stored framing.
    if single_chunk {
        candidates
            .push(encode_entropy_chunk(&whole[quantum_origin..quantum_origin + out_len], level)?);
    }

    // Keep the smallest candidate that fits the size budget and actually shrinks.
    let mut best: Option<Vec<u8>> = None;
    for cand in candidates {
        if cand.len() <= MAX_QUANTUM_PAYLOAD
            && cand.len() < out_len
            && best.as_ref().is_none_or(|b| cand.len() < b.len())
        {
            best = Some(cand);
        }
    }
    Ok(best)
}

/// Build an all-LZ payload for the quantum, or `None` if any chunk cannot be an LZ chunk.
fn build_lz_payload(
    whole: &[u8],
    index: &lz::TokenIndex,
    quantum_origin: usize,
    out_len: usize,
    level: Level,
) -> Option<Vec<u8>> {
    let mut payload = Vec::new();
    let mut pos = 0usize;
    while pos < out_len {
        let chunk_len = (out_len - pos).min(STEP_LEN);
        let chunk_origin = quantum_origin + pos;
        let lz_chunk = lz::encode_lz_chunk(whole, index, chunk_origin, chunk_len, level)?;
        if payload.len() + lz_chunk.len() > MAX_QUANTUM_PAYLOAD {
            return None;
        }
        payload.extend_from_slice(&lz_chunk);
        pos += chunk_len;
    }
    Some(payload)
}

/// Encode `chunk` as a bit-23-clear entropy array (the decoder's no-LZ path). This is exactly
/// `encode_array`'s output: its 3 leading header bytes are the array's own size header, and
/// every encoder `encode_array` emits keeps byte0's bit 7 clear (raw: size>>16 ≤ 3;
/// huff/tANS/RLE: (mode<<4) with mode ≤ 5), so the chunk header's bit 23 is clear.
fn encode_entropy_chunk(chunk: &[u8], level: Level) -> Result<Vec<u8>> {
    let mut w = ByteWriter::new();
    let n = crate::entropy::encode_array(chunk, &mut w, level)?;
    debug_assert_eq!(n, w.len());
    Ok(w.into_vec())
}

/// LZ chunk encoding (bit-23-set inner chunk): the optional high-ratio path.
///
/// This is the inverse of the decoder's [`super::decode`] LZ path (`read_lz_table` +
/// `unpack_offsets` + `process_lz_runs`).
///
/// To keep the encoder simple while staying bit-exact, it pins the easy decoder-legal
/// choices: **mode 1** (raw, verbatim literals — no sub-predictor), **offs_encode_type 0**
/// (single-table distance coding), and **every match emits a fresh offset** (token offset
/// index 3), so the recent-offset cache never needs encoder bookkeeping.
mod lz {
    use super::STEP_LEN;
    use crate::bytes::ByteWriter;
    use crate::lz::Token;
    use crate::Level;
    use alloc::vec::Vec;

    /// A per-chunk token after re-segmentation to the chunk's output window. `lit_len`
    /// literals are taken from the chunk's literal source, then a match of `match_len` at
    /// `distance` back (in the *whole* output). A final token has `match_len == 0`.
    struct ChunkTok {
        lit_len: usize,
        distance: usize,
        match_len: usize,
    }

    /// The whole-input token list plus a prefix-sum of each token's *starting* output
    /// position, so a chunk window can be located by binary search instead of re-walking the
    /// whole list. Built once per `compress` call.
    pub(super) struct TokenIndex<'a> {
        toks: &'a [Token],
        /// `start[i]` = output position where token `i` begins (cumulative lit+match before it).
        start: Vec<usize>,
    }

    impl<'a> TokenIndex<'a> {
        pub(super) fn new(toks: &'a [Token]) -> Self {
            let mut start = Vec::with_capacity(toks.len() + 1);
            let mut pos = 0usize;
            for t in toks {
                start.push(pos);
                pos += t.lit_len as usize + t.match_len as usize;
            }
            start.push(pos); // total output length
            TokenIndex { toks, start }
        }

        /// Index of the first token whose covered output range can overlap `win_start`
        /// (i.e. the last token starting at or before `win_start`).
        fn first_overlapping(&self, win_start: usize) -> usize {
            // start is sorted ascending; find the rightmost i with start[i] <= win_start.
            match self.start.binary_search(&win_start) {
                Ok(i) => i,
                Err(i) => i.saturating_sub(1),
            }
        }
    }

    /// Build an LZ chunk for the output window `[chunk_origin, chunk_origin + chunk_len)` of
    /// `whole`, or `None` if LZ is not applicable / not worthwhile / would not be
    /// decoder-legal. The returned bytes are the complete inner chunk (3-byte header with bit
    /// 23 set, then the LZ sub-streams).
    pub(super) fn encode_lz_chunk(
        whole: &[u8],
        index: &TokenIndex,
        chunk_origin: usize,
        chunk_len: usize,
        level: Level,
    ) -> Option<Vec<u8>> {
        // Bail on tiny inputs (`chunk_len <= 128`): below that the framing overhead dominates
        // and the entropy chunk wins anyway.
        if chunk_len <= 128 {
            return None;
        }
        debug_assert!(chunk_len <= STEP_LEN);

        let toks = segment_tokens(index, chunk_origin, chunk_len)?;
        let chunk = &whole[chunk_origin..chunk_origin + chunk_len];

        // The whole-output offset of this chunk's first emitted byte. For the very first
        // chunk (origin 0) the decoder copies an 8-byte literal seed before the run loop.
        let seed = chunk_origin == 0;

        // --- Build the six LZ arrays from the tokens. ---
        let mut lits: Vec<u8> = Vec::new();
        let mut tokens: Vec<u8> = Vec::new();
        let mut u8_offs: Vec<u8> = Vec::new();
        let mut u32_offs: Vec<u32> = Vec::new();
        let mut lrl8: Vec<u8> = Vec::new();
        let mut len32: Vec<u32> = Vec::new();

        // The seed bytes are emitted to the output directly (not via the literal stream); the
        // run loop starts after them, so the first token's literals begin at chunk byte 8.
        let mut src_pos = 0usize; // byte offset within `chunk` consumed by literals
        let seed_skip = if seed { 8 } else { 0 };
        if seed && chunk_len < 8 {
            return None;
        }
        src_pos += seed_skip;

        let n = toks.len();
        for (i, t) in toks.iter().enumerate() {
            let is_final = i == n - 1;
            if is_final {
                debug_assert_eq!(t.match_len, 0);
                // Final literal run: emit verbatim into `lits`; the decoder fills the tail
                // from the literal stream. No token byte is written for the final run.
                let lit = &chunk[src_pos..src_pos + t.lit_len];
                lits.extend_from_slice(lit);
                break;
            }

            // A regular token: literals, then a fresh-offset match. Literals are read from the
            // chunk at their output position; the match then consumes `match_len` output bytes,
            // so the next token's literals start `lit_len + match_len` further into `chunk`.
            let lit = &chunk[src_pos..src_pos + t.lit_len];
            lits.extend_from_slice(lit);
            src_pos += t.lit_len + t.match_len;

            // The decoder reads litlen from `f & 3` (0..2) or, for value 3, from the lrl
            // stream. len>=3 pushes (len-3) to lrl8 (255-escaped to len32 for len>=258).
            let litlen_field = write_lits_len(&mut lrl8, &mut len32, t.lit_len);

            // Match length: ml_token = ml-2 capped at 15;
            // ml_token 15 escapes (ml-17, 255-escaped to len32 for ml>=255+17).
            let ml_field = write_match_len(&mut lrl8, &mut len32, t.match_len);

            // Always a fresh offset (token offset index 3 → high bits 3<<6).
            let offs = t.distance as u32;
            let (u8b, u32v) = pack_offset(offs);
            u8_offs.push(u8b);
            u32_offs.push(u32v);

            let token = litlen_field + ml_field + (3 << 6);
            if token > 255 {
                return None; // malformed; bail to entropy chunk
            }
            tokens.push(token as u8);
        }

        // --- Serialise the arrays into the chunk body. ---
        let mut body = ByteWriter::new();

        // 0) the 8-byte literal seed (only for the first chunk of the whole output): copied
        //    verbatim to output[0..8] by the decoder before the run loop.
        if seed {
            body.extend(&whole[0..8]);
        }

        // 1) literal stream.
        crate::entropy::encode_array(&lits, &mut body, level).ok()?;
        // 2) token (command) stream.
        crate::entropy::encode_array(&tokens, &mut body, level).ok()?;
        // 3) packed-offset (u8) stream. Its first header byte's bit 7 must be clear so the
        //    decoder selects single-table mode (offs_encode_type 0); `encode_array` always
        //    emits a long-form header, which satisfies that.
        crate::entropy::encode_array(&u8_offs, &mut body, level).ok()?;
        // 4) packed litlen (lrl8) stream.
        crate::entropy::encode_array(&lrl8, &mut body, level).ok()?;
        // 5) the trailing forward/backward bit-packed offset+length region.
        let bits = write_offset_bits(&u8_offs, &u32_offs, &len32)?;
        body.extend(&bits);

        let body = body.into_vec();
        let inner_size = body.len();
        if inner_size >= chunk_len || inner_size > 0x7_FFFF {
            return None;
        }

        // --- Wrap with the 3-byte big-endian chunk header (bit 23 set). ---
        // chunkhdr = 0x800000 | (mode << 19) | inner_size, mode = 1 (raw literals).
        let mode = 1u32;
        let hdr = 0x80_0000u32 | (mode << 19) | (inner_size as u32);
        let mut out = Vec::with_capacity(3 + inner_size);
        out.push((hdr >> 16) as u8);
        out.push((hdr >> 8) as u8);
        out.push(hdr as u8);
        out.extend_from_slice(&body);
        Some(out)
    }

    /// Re-segment the whole-stream token list to the chunk's output window. Matches that
    /// straddle a window edge are **clamped** to the window (a back-reference at distance `d`
    /// stays valid byte-for-byte under clamping, since output byte `k` always copies from
    /// `k - d`); a clamp shorter than the 2-byte minimum match becomes literals instead. The
    /// result is a list of [`ChunkTok`] whose cover is exactly the window, ending in a literal
    /// run. Returns `None` only if a clamped match would be decoder-illegal.
    fn segment_tokens(
        index: &TokenIndex,
        chunk_origin: usize,
        chunk_len: usize,
    ) -> Option<Vec<ChunkTok>> {
        // The chunk's run loop begins after the 8-byte literal seed for the first chunk.
        let win_start = chunk_origin + if chunk_origin == 0 { 8 } else { 0 };
        let win_end = chunk_origin + chunk_len;

        let mut out: Vec<ChunkTok> = Vec::new();
        let mut pending_lits = 0usize; // literals accumulated for the next emitted token
        let mut cursor = win_start; // next output position not yet accounted for

        let first = index.first_overlapping(win_start);
        for i in first..index.toks.len() {
            let t = &index.toks[i];
            let tok_start = index.start[i];
            let lit_len = t.lit_len as usize;
            let match_len = t.match_len as usize;

            let lit_hi = tok_start + lit_len;
            // Literal portion inside the window contributes to `pending_lits`.
            if lit_hi > win_start && tok_start < win_end {
                let lo = tok_start.max(cursor);
                let hi = lit_hi.min(win_end);
                if hi > lo {
                    pending_lits += hi - lo;
                    cursor = hi;
                }
            }

            let m_start = lit_hi;
            if m_start >= win_end {
                break; // past the window
            }
            if match_len == 0 {
                continue;
            }
            let m_end = m_start + match_len;
            let distance = t.offset as usize;

            if m_end <= win_start {
                continue; // match entirely before the window
            }

            // Clamp the match copy to the window.
            let lo = m_start.max(win_start).max(cursor);
            let hi = m_end.min(win_end);
            if hi <= lo {
                continue;
            }
            let clamped_len = hi - lo;

            // Decoder-legality: copyfrom = lo - distance must be >= 0 (>= base 0).
            if distance > lo {
                return None;
            }

            if clamped_len < 2 {
                // Too short to encode as a match; fold into the literal run (the match bytes
                // equal the source bytes at this position, so they decode the same as literals).
                pending_lits += clamped_len;
                cursor = hi;
                continue;
            }

            out.push(ChunkTok {
                lit_len: pending_lits,
                distance,
                match_len: clamped_len,
            });
            pending_lits = 0;
            cursor = hi;
        }

        // Everything after the last in-window match is the final literal run.
        if cursor < win_end {
            pending_lits += win_end - cursor;
        }
        out.push(ChunkTok {
            lit_len: pending_lits,
            distance: 0,
            match_len: 0,
        });

        // Validate the cover reproduces the window exactly.
        let covered: usize = out.iter().map(|t| t.lit_len + t.match_len).sum::<usize>();
        if covered != win_end - win_start {
            return None;
        }
        Some(out)
    }

    /// Litlen accounting. Returns the token's `litlen_field` (`min(len, 3)`); pushes to `lrl8`
    /// (255-escaped to `len32`) when len >= 3.
    fn write_lits_len(lrl8: &mut Vec<u8>, len32: &mut Vec<u32>, len: usize) -> u32 {
        if len == 0 {
            return 0;
        }
        if len >= 3 {
            if len >= 258 {
                lrl8.push(255);
                len32.push((len - 258) as u32);
            } else {
                lrl8.push((len - 3) as u8);
            }
            3
        } else {
            // len is 1 or 2: encoded directly in the token's low 2 bits, no lrl byte.
            len as u32
        }
    }

    /// Returns `ml_token << 2`. ml_token = ml-2 capped at 15;
    /// the cap escapes via lrl8 (`ml-17`, 255-escaped to len32 for ml >= 272).
    fn write_match_len(lrl8: &mut Vec<u8>, len32: &mut Vec<u32>, ml: usize) -> u32 {
        let mut ml_token = ml as i64 - 2;
        if ml_token >= 15 {
            ml_token = 15;
            if ml >= 255 + 17 {
                lrl8.push(255);
                len32.push((ml - 255 - 17) as u32);
            } else {
                lrl8.push((ml - 17) as u8);
            }
        }
        (ml_token as u32) << 2
    }

    /// The u8 packed-offset byte plus the raw u32 offset that the bit region carries.
    fn pack_offset(offs: u32) -> (u8, u32) {
        if offs >= 8_388_360 {
            let bsr = bsr32(offs - 8_322_816);
            ((bsr | 0xF0) as u8, offs)
        } else {
            let bsr = bsr32(offs + 248);
            ((((offs - 8) & 0xF) | (16 * (bsr - 8))) as u8, offs)
        }
    }

    /// Write the offset bits with `offs_encode_type == 0`, `flag_ignore_u32_length == 0`.
    /// Produces the trailing bit-packed region read back during offset unpacking (a forward and
    /// a backward MSB-first stream that meet in the middle).
    fn write_offset_bits(u8_offs: &[u8], u32_offs: &[u32], len32: &[u32]) -> Option<Vec<u8>> {
        let u32_len_count = len32.len();

        // Generous scratch with headroom for the 8-byte flush windows on both ends. The region
        // is far smaller than the chunk in practice (≈ a few bytes per offset/length).
        let cap = 128 + u8_offs.len() * 8 + u32_len_count * 8;
        let mut buf = alloc::vec![0u8; cap];

        let mut f = BitWriter64Fwd::new();
        let mut b = BitWriter64Bwd::new(cap);

        // The forward and backward 8-byte flush windows must never overlap. Conservatively
        // require a comfortable gap.
        macro_rules! guard {
            () => {
                if b.ptr <= f.ptr + 16 {
                    return None;
                }
            };
        }

        // u32-length-count prefix (backward): a unary-prefixed value of `u32_len_count + 1`.
        let nb = bsr32_z(u32_len_count as u32 + 1);
        b.write(1, nb + 1, &mut buf);
        if nb != 0 {
            b.write(u32_len_count as u32 + 1 - (1 << nb), nb, &mut buf);
        }

        // Distances: alternate forward/backward, single-table coding (offs_encode_type 0).
        for (i, (&u8b, &u32v)) in u8_offs.iter().zip(u32_offs.iter()).enumerate() {
            guard!();
            let (nb, bits) = if u8b < 0xf0 {
                let nb = (u8b >> 4) as u32 + 4;
                let bits = ((u32v + 248) >> 4).wrapping_sub(1 << nb);
                (nb, bits)
            } else {
                let nb = u8b as u32 - 0xe0;
                let bits = u32v.wrapping_sub(1 << nb).wrapping_sub(8_322_816);
                (nb, bits)
            };
            if i & 1 == 1 {
                b.write(bits, nb, &mut buf);
            } else {
                f.write(bits, nb, &mut buf);
            }
        }

        // Explicit u32 lengths: alternate forward/backward, gamma-ish + 6 low bits.
        for (i, &len) in len32.iter().enumerate() {
            guard!();
            let nb = bsr32_z((len >> 6) + 1);
            if i & 1 == 1 {
                b.write(1, nb + 1, &mut buf);
                if nb != 0 {
                    b.write((len >> 6) + 1 - (1 << nb), nb, &mut buf);
                }
                b.write(len & 0x3f, 6, &mut buf);
            } else {
                f.write(1, nb + 1, &mut buf);
                if nb != 0 {
                    f.write((len >> 6) + 1 - (1 << nb), nb, &mut buf);
                }
                f.write(len & 0x3f, 6, &mut buf);
            }
        }

        // Finalise both writers and compact the backward portion against the forward one.
        let fp = f.finish(&mut buf);
        let bp = b.finish(&mut buf);
        if bp <= fp + 8 {
            return None; // writers collided — chunk too dense; fall back to entropy
        }
        let tail = cap - bp;
        let mut out = Vec::with_capacity(fp + tail);
        out.extend_from_slice(&buf[..fp]);
        out.extend_from_slice(&buf[bp..cap]);
        Some(out)
    }

    /// Forward 64-bit bit writer: an MSB-first accumulator flushed as
    /// byteswapped 64-bit words into `buf`, advancing the cursor low→high. Each flush emits
    /// the whole bytes currently buffered; `finish` flushes the trailing partial byte and
    /// returns the final cursor.
    struct BitWriter64Fwd {
        bits: u64,
        pos: i32, // bit position, starts at 63
        ptr: usize,
    }
    impl BitWriter64Fwd {
        fn new() -> Self {
            BitWriter64Fwd { bits: 0, pos: 63, ptr: 0 }
        }
        fn write(&mut self, bits: u32, n: u32, buf: &mut [u8]) {
            debug_assert!(n as i32 <= self.pos);
            self.pos -= n as i32;
            self.bits = (self.bits << n) | bits as u64;
            self.flush(buf);
        }
        fn flush(&mut self, buf: &mut [u8]) {
            let t = ((63 - self.pos) >> 3) as u32;
            // `pos` is the post-`Write` value (Write decremented by n>=1), so `pos <= 62` here
            // and `pos + 1 <= 63` never overflows the u64 shift.
            let v = self.bits << (self.pos + 1);
            self.pos += 8 * t as i32;
            // Forward: store byteswap(v) as 8 bytes at ptr, advance by t. The trailing partial
            // byte lands at buf[ptr] (within the 8-byte store) and is picked up by `finish`.
            let be = v.swap_bytes().to_le_bytes(); // byte-swap then little-endian store
            buf[self.ptr..self.ptr + 8].copy_from_slice(&be);
            self.ptr += t as usize;
        }
        /// Return the final cursor: the last `flush` already stored any
        /// partial byte at `buf[ptr]`, so just account for it.
        fn finish(self, _buf: &mut [u8]) -> usize {
            self.ptr + if self.pos != 63 { 1 } else { 0 }
        }
    }

    /// Backward 64-bit bit writer: same accumulator, but words store at `ptr-8` and the
    /// cursor walks high→low. `finish` returns the final cursor.
    struct BitWriter64Bwd {
        bits: u64,
        pos: i32,
        ptr: usize,
    }
    impl BitWriter64Bwd {
        fn new(end: usize) -> Self {
            BitWriter64Bwd { bits: 0, pos: 63, ptr: end }
        }
        fn write(&mut self, bits: u32, n: u32, buf: &mut [u8]) {
            debug_assert!(n as i32 <= self.pos);
            self.pos -= n as i32;
            self.bits = (self.bits << n) | bits as u64;
            self.flush(buf);
        }
        fn flush(&mut self, buf: &mut [u8]) {
            let t = ((63 - self.pos) >> 3) as u32;
            // `pos <= 62` here (see forward writer), so `pos + 1 <= 63` is a safe u64 shift.
            let v = self.bits << (self.pos + 1);
            self.pos += 8 * t as i32;
            // Backward: store v (no byteswap) as 8 bytes ending at ptr, retreat by t. The
            // trailing partial byte lands at buf[ptr-1] and is accounted by `finish`.
            let le = v.to_le_bytes();
            buf[self.ptr - 8..self.ptr].copy_from_slice(&le);
            self.ptr -= t as usize;
        }
        fn finish(self, _buf: &mut [u8]) -> usize {
            self.ptr - if self.pos != 63 { 1 } else { 0 }
        }
    }

    /// Bit-scan-reverse for non-zero `x`.
    fn bsr32(x: u32) -> u32 {
        debug_assert!(x != 0);
        31 - x.leading_zeros()
    }
    /// BSR that treats 0 as 0 (only >=1 is used here, but guard anyway).
    fn bsr32_z(x: u32) -> u32 {
        if x == 0 { 0 } else { 31 - x.leading_zeros() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// Round-trip through the in-tree decoder: the encoder's output must decode to `src`.
    fn rt(src: &[u8], level: Level) {
        let comp = compress(src, level).expect("compress");
        let back = crate::decompress(&comp, src.len())
            .unwrap_or_else(|e| panic!("decode failed (len {}, {level:?}): {e:?}", src.len()));
        assert_eq!(back, src, "roundtrip mismatch (len {}, {level:?})", src.len());
    }

    fn both_levels(src: &[u8]) {
        rt(src, Level::Fastest);
        rt(src, Level::Default);
    }

    #[test]
    fn empty_input_roundtrips() {
        both_levels(&[]);
        // An empty stream is genuinely empty (zero quanta).
        assert_eq!(compress(&[], Level::Default).unwrap().len(), 0);
    }

    #[test]
    fn one_byte_roundtrips() {
        both_levels(&[0x5a]);
    }

    #[test]
    fn all_same_large_roundtrips() {
        both_levels(&vec![7u8; 100_000]);
    }

    #[test]
    fn zeros_70k_roundtrips() {
        both_levels(&vec![0u8; 70_000]);
    }

    #[test]
    fn skewed_text_roundtrips() {
        let mut data = Vec::new();
        let mut seed = 1u32;
        for _ in 0..40_000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let r = (seed >> 24) as u8;
            data.push(if r < 200 { b'a' } else if r < 240 { b' ' } else { b'a' + (r % 26) });
        }
        both_levels(&data);
    }

    /// Skewed data (long zero runs + sparse literals) over a multi-chunk quantum stresses the
    /// LZ encoder's run-of-zeros matches and literal/match interleaving. Regression for a
    /// first-bytes mismatch found by the integration fuzz at n=0x3FFFE.
    #[test]
    fn lz_skewed_zero_runs_multichunk() {
        for &n in &[0x10000usize, 0x1FFFE, 0x20002, 0x30000, 0x3FFFE] {
            let mut s = 0xABCD1234u32;
            let data: Vec<u8> = (0..n)
                .map(|_| {
                    s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                    let r = (s >> 24) as u8;
                    if r < 220 { 0u8 } else { r }
                })
                .collect();
            for level in [Level::Fastest, Level::Default] {
                let comp = compress(&data, level).expect("compress");
                let back = crate::decompress(&comp, n).expect("decode");
                let first = (0..n).find(|&i| back.get(i) != data.get(i));
                assert!(first.is_none(), "n={n:#x} {level:?} first_mismatch={first:?}");
            }
        }
    }

    /// Directly build a single LZ chunk for skewed data and decode it via a hand-wrapped
    /// single-quantum stream, isolating whether the LZ *block* itself is wrong (vs a
    /// multi-chunk interaction). Bypasses entropy selection entirely.
    #[test]
    fn lz_single_chunk_forced_skewed() {
        for &n in &[0x1000usize, 0x8000, 0x1FFFE, 0x20000] {
            let mut s = 0xABCD1234u32;
            let data: Vec<u8> = (0..n)
                .map(|_| {
                    s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                    let r = (s >> 24) as u8;
                    if r < 220 { 0u8 } else { r }
                })
                .collect();
            let toks = crate::lz::find_tokens(&data, Level::Fastest);
            let index = super::lz::TokenIndex::new(&toks);
            let Some(chunk) = super::lz::encode_lz_chunk(&data, &index, 0, n, Level::Fastest) else {
                continue; // LZ declined for this size; nothing to check here
            };
            // Wrap: stream header (keyframe, compressed) + compressed quantum header + chunk.
            let mut w = ByteWriter::new();
            super::write_stream_header(&mut w, CODEC_KRAKEN, false, true, false);
            super::write_compressed_header(&mut w, chunk.len());
            w.extend(&chunk);
            let stream = w.into_vec();
            let back = crate::decompress(&stream, n).expect("decode");
            let first = (0..n).find(|&i| back.get(i) != data.get(i));
            assert!(first.is_none(), "n={n:#x} LZ block first_mismatch={first:?}");
        }
    }

    /// Minimal forced-LZ probe: a 2-chunk quantum just over STEP_LEN whose chunk1 is forced to
    /// be LZ (skewed/zero-run). Isolates the multi-chunk LZ-block bug from entropy selection.
    #[test]
    fn lz_forced_multichunk_zero_runs() {
        // chunk1: 0x20000 bytes = mostly zeros with sparse single-byte literals (forces many
        // distance-8 matches with short literal runs); chunk2: a few more bytes.
        let mut s = 0xABCD1234u32;
        let mut data: Vec<u8> = (0..0x20000)
            .map(|_| {
                s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                let r = (s >> 24) as u8;
                if r < 220 { 0u8 } else { r }
            })
            .collect();
        data.extend((0..0x10000).map(|i| (i % 50) as u8)); // LZ-able chunk2
        let comp = compress(&data, Level::Fastest).expect("compress");
        let back = crate::decompress(&comp, data.len()).expect("decode");
        let first = (0..data.len()).find(|&i| back.get(i) != data.get(i));
        assert!(first.is_none(), "first_mismatch={first:?}");
    }

    /// LZ effectiveness: a long-match-rich buffer (single chunk, < 128 KiB) must compress to a
    /// small fraction only achievable with LZ matches — proves the LZ chunk path fires *and*
    /// round-trips through the decoder, not merely that it falls back to entropy.
    #[test]
    fn lz_chunk_fires_and_shrinks() {
        let motif = b"The quick brown fox jumps over the lazy dog 0123456789. ";
        let mut data = Vec::with_capacity(50_000);
        while data.len() < 50_000 {
            data.extend_from_slice(motif);
        }
        data.truncate(50_000);
        for level in [Level::Fastest, Level::Default] {
            let comp = compress(&data, level).expect("compress");
            let back = crate::decompress(&comp, data.len()).expect("decode");
            assert_eq!(back, data, "lz roundtrip mismatch at {level:?}");
            let ratio = comp.len() as f64 / data.len() as f64;
            // Entropy-only on a repeated motif lands ~0.5; LZ matches must beat 0.15.
            assert!(
                ratio < 0.15,
                "{level:?}: ratio {ratio:.4} ({} bytes) — LZ path not effective",
                comp.len()
            );
        }
    }

    /// A long exact self-repeat (X ++ X) forces a single very long match — exercises the
    /// match-length escape (`ml_token == 15` + len32) through the decoder.
    #[test]
    fn lz_long_match_roundtrips() {
        let mut x = Vec::with_capacity(20_000);
        let mut seed = 7u32;
        for _ in 0..20_000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            x.push((seed >> 24) as u8);
        }
        let mut data = x.clone();
        data.extend_from_slice(&x); // exact 20 KB repeat
        both_levels(&data);
    }

    #[test]
    fn incompressible_lcg_roundtrips() {
        let mut data = Vec::with_capacity(50_000);
        let mut seed = 0xDEAD_BEEFu32;
        for _ in 0..50_000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            data.push((seed >> 24) as u8);
        }
        both_levels(&data);
    }

    #[test]
    fn multi_quantum_roundtrips() {
        // > 256 KiB: spans multiple quanta. Mildly repetitive so most quanta compress.
        let mut data = Vec::with_capacity(600_000);
        for i in 0..600_000u32 {
            data.push((i % 251) as u8);
        }
        both_levels(&data);
    }

    #[test]
    fn full_quantum_exact_boundary() {
        // Exactly one full quantum, and one full + 1 partial.
        both_levels(&vec![0xABu8; QUANTUM_LEN]);
        let mut v = vec![0xABu8; QUANTUM_LEN];
        v.push(0x01);
        both_levels(&v);
    }
}
