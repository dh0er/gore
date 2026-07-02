//! Kraken decode: stream/quantum framing → entropy sub-streams → LZ copy loop.
//!
//! [`decompress`] drives the quantum loop, [`decode_quantum`] decodes one 256 KiB quantum,
//! [`read_lz_table`] expands the entropy sub-streams into the per-quantum int arrays, and
//! [`process_lz_runs`] runs the literal/match copy loop.
//!
//! ## Sub-stream layout of a compressed Kraken quantum (so the encoder can mirror it)
//!
//! After the 2-byte stream header and the (3/6-byte) quantum header comes the quantum
//! payload. Its first 3 bytes are an inner **chunk header** (big-endian
//! `chunkhdr = src[0]<<16 | src[1]<<8 | src[2]`):
//!
//! * If bit 23 (`0x800000`) is **clear**: the whole quantum is one entropy array with *no*
//!   LZ matching. The 3 bytes are the array's own size header (not consumed separately); the
//!   array decodes to exactly `dst_count` bytes.
//! * If bit 23 is **set**: `src += 3`, then `inner_size = chunkhdr & 0x7FFFF` (19-bit
//!   compressed size of the LZ block) and `mode = (chunkhdr >> 19) & 0xF` (0 = sub-literals,
//!   1 = raw literals). Then:
//!   * `inner_size < dst_count` → an LZ-compressed block: parse the LZ table and run the copy
//!     loop.
//!   * `inner_size == dst_count` and `mode == 0` → stored verbatim (`memmove`).
//!   * otherwise → corrupt.
//!
//! The LZ block (consumed by [`read_lz_table`]) lays its sub-streams out as, in order:
//!   1. an optional 8-byte literal seed copied straight to `dst` when this is the first
//!      quantum (`offset == 0`);
//!   2. `lit_stream`  — the literals array (bounded by `dst_size`);
//!   3. `cmd_stream`  — the command/token array (bounded by `dst_size`);
//!   4. a 1-byte offset-mode flag, then `packed_offs_stream` (and, for the 2-table offset
//!      mode, an `offs_scaling` byte + a second extra array);
//!   5. `packed_len_stream` — the packed literal-run/extra-length array (bounded by
//!      `dst_size/4`);
//!   6. a trailing bit-packed region (read forwards *and* backwards) holding the actual
//!      distance codes, the extra u32 lengths, and a small length-table size prefix.
//!
//! Kraken proper never uses the multi-array literal/command modes (those are Leviathan), so
//! the offset-mode flag's high bit selects only the 1-table vs 2-table distance coding.

#![allow(dead_code)]
// The LZ table reader and the literal copier take wide argument lists; splitting them into
// structs would obscure the sub-stream layout they thread through.
#![allow(clippy::too_many_arguments)]

use crate::bits::{BitReader, BitReaderRev};
use crate::{Error, Result};
use alloc::vec::Vec;

use super::block::{
    classify, parse_quantum_header, parse_stream_header, Quantum, QUANTUM_LEN,
};

/// Largest quantum the Kraken decoder produces in one inner step (128 KiB). A 256 KiB quantum
/// is internally split into two of these, sharing history.
const STEP_LEN: usize = 0x20000;

/// Decode a full Kraken stream to exactly `decompressed_len` bytes.
pub(crate) fn decompress(src: &[u8], decompressed_len: usize) -> Result<Vec<u8>> {
    decompress_observed(src, decompressed_len, &mut |_| ())
}

/// One event reported by the instrumented decode walk ([`decompress_observed`]).
///
/// Match positions are relative to the enclosing inner chunk's output window
/// (`0..chunk_len`) — the frame real Oodle's end-of-chunk match guard zone is stated in
/// (see `encode`'s `segment_tokens`).
pub(crate) enum LzEvent {
    /// An LZ-coded inner chunk begins; it decodes to `chunk_len` output bytes. Entropy-only
    /// and stored chunks are not reported (they carry no match commands).
    LzChunk { chunk_len: usize },
    /// The current LZ chunk's copy loop materialized a match covering the chunk-relative
    /// output range `[match_start, match_end)`.
    Match { match_start: usize, match_end: usize },
}

/// [`decompress`], instrumented: identical decoding (the plain wrapper passes a no-op
/// observer that compiles away), with every LZ chunk and match command reported to
/// `observe`. Tests use this to validate the *encoder's* output structurally — e.g. the
/// end-of-chunk match guard zone, which real Oodle enforces but this decoder deliberately
/// tolerates, so a compress→decompress roundtrip alone cannot catch a violation.
pub(crate) fn decompress_observed<F: FnMut(LzEvent)>(
    src: &[u8],
    decompressed_len: usize,
    observe: &mut F,
) -> Result<Vec<u8>> {
    // Size the output with a fallible allocation: `decompressed_len` is untrusted.
    let mut out: Vec<u8> = Vec::new();
    out.try_reserve_exact(decompressed_len)
        .map_err(|_| Error::OutputTooLarge(decompressed_len))?;

    // Scratch for entropy sub-streams + unpacked int arrays, reused across quanta.
    let mut scratch = Scratch::new(decompressed_len.min(QUANTUM_LEN))?;

    let mut r = crate::bytes::ByteReader::new(src);
    let mut produced = 0usize;

    while produced < decompressed_len {
        let out_len = (decompressed_len - produced).min(QUANTUM_LEN);

        let sh = parse_stream_header(&mut r)?;
        if sh.codec_id != 6 {
            return Err(Error::Corrupt("only Kraken (codec id 6) is supported"));
        }

        if sh.uncompressed {
            // Stream-level uncompressed: out_len raw bytes follow the stream header.
            let raw = r.take(out_len)?;
            out.extend_from_slice(raw);
            produced += out_len;
            continue;
        }

        let qh = parse_quantum_header(&mut r, out_len, sh.use_checksum)?;
        let body = &src[r.pos()..];
        match classify(&qh, body, out_len)? {
            Quantum::Memset { value, out_len } => {
                out.resize(out.len() + out_len, value);
                produced += out_len;
            }
            Quantum::Stored(bytes) => {
                out.extend_from_slice(bytes);
                let _ = r.take(bytes.len())?;
                produced += out_len;
            }
            Quantum::Compressed { payload, out_len } => {
                let base = out.len();
                // Materialize the quantum's output region, then decode in place.
                out.resize(base + out_len, 0);
                decode_quantum(&mut out[..], base, payload, &mut scratch, observe)?;
                let _ = r.take(payload.len())?;
                produced += out_len;
            }
        }
    }

    debug_assert_eq!(out.len(), decompressed_len);
    Ok(out)
}

/// Decode one compressed quantum's `payload` into `out[base..base+out_len]`.
///
/// `out` is the *whole* output so far (back-references reach into earlier quanta);
/// `base` is the start of this quantum and `out.len() - base` is `out_len`. This splits the
/// 256 KiB quantum into ≤128 KiB chunks.
fn decode_quantum<F: FnMut(LzEvent)>(
    out: &mut [u8],
    base: usize,
    payload: &[u8],
    scratch: &mut Scratch,
    observe: &mut F,
) -> Result<()> {
    let out_len = out.len() - base;
    let mut src_pos = 0usize; // offset into payload
    let mut dpos = base; // absolute write cursor into out

    let dst_end = base + out_len;
    while dpos < dst_end {
        let dst_count = (dst_end - dpos).min(STEP_LEN);
        let chunk = payload.get(src_pos..).ok_or(Error::Truncated)?;
        if chunk.len() < 4 {
            return Err(Error::Truncated);
        }
        let chunkhdr =
            (chunk[0] as u32) << 16 | (chunk[1] as u32) << 8 | chunk[2] as u32;

        if chunkhdr & 0x80_0000 == 0 {
            // Stored as entropy without any match copying: the chunk header *is* the array
            // size header. Decodes to exactly dst_count bytes.
            let (produced, used) =
                crate::entropy::decode_array_capped(chunk, &mut out[dpos..dpos + dst_count])?;
            if produced != dst_count {
                return Err(Error::Corrupt("kraken: entropy chunk size != dst_count"));
            }
            src_pos += used;
            dpos += dst_count;
        } else {
            let inner_size = (chunkhdr & 0x7_FFFF) as usize;
            let mode = ((chunkhdr >> 19) & 0xF) as u8;
            let lz = &chunk[3..];
            if lz.len() < inner_size {
                return Err(Error::Truncated);
            }
            let lz = &lz[..inner_size];

            if inner_size < dst_count {
                // `offset` is the distance of `dpos` from the *whole output* start (index 0):
                // `dst_start` is the global origin, so matches and the keyframe seed are
                // relative to the entire stream, not the quantum.
                let offset = dpos;
                observe(LzEvent::LzChunk { chunk_len: dst_count });
                read_lz_table(mode, lz, out, dpos, dst_count, offset, scratch)?;
                process_lz_runs(mode, out, dpos, dst_count, 0, scratch, observe)?;
                src_pos += 3 + inner_size;
                dpos += dst_count;
            } else if inner_size > dst_count || mode != 0 {
                return Err(Error::Corrupt("kraken: bad stored chunk"));
            } else {
                // inner_size == dst_count, mode 0: stored verbatim.
                out[dpos..dpos + dst_count].copy_from_slice(lz);
                src_pos += 3 + inner_size;
                dpos += dst_count;
            }
        }
    }
    Ok(())
}

/// Per-quantum scratch: decoded literal/command bytes plus the unpacked int offset/length
/// arrays. Owned buffers (no aliasing into `src`), reused across quanta.
struct Scratch {
    lit: Vec<u8>,
    cmd: Vec<u8>,
    packed_offs: Vec<u8>,
    packed_offs_extra: Vec<u8>,
    packed_len: Vec<u8>,
    offs: Vec<i32>,
    len: Vec<i32>,
}

impl Scratch {
    fn new(quantum_cap: usize) -> Result<Self> {
        // Generous fixed caps sized to one 128 KiB inner chunk's worth of streams. We size to
        // the quantum cap so tiny outputs don't over-allocate, but never below the inner step.
        let cap = quantum_cap.max(1);
        let try_vec_u8 = |n: usize| -> Result<Vec<u8>> {
            let mut v = Vec::new();
            v.try_reserve_exact(n).map_err(|_| Error::OutputTooLarge(n))?;
            Ok(v)
        };
        let try_vec_i32 = |n: usize| -> Result<Vec<i32>> {
            let mut v = Vec::new();
            v.try_reserve_exact(n).map_err(|_| Error::OutputTooLarge(n))?;
            Ok(v)
        };
        Ok(Scratch {
            lit: try_vec_u8(cap)?,
            cmd: try_vec_u8(cap)?,
            packed_offs: try_vec_u8(cap)?,
            packed_offs_extra: try_vec_u8(cap)?,
            packed_len: try_vec_u8(cap)?,
            offs: try_vec_i32(cap)?,
            len: try_vec_i32(cap)?,
        })
    }
}

/// Decode the LZ sub-streams for one chunk into `scratch`.
///
/// `out`/`dpos`/`dst_count` describe the chunk's output window; `offset` is the distance of
/// `dpos` from the quantum start (when `0`, the chunk begins with an 8-byte literal seed).
fn read_lz_table(
    mode: u8,
    mut src: &[u8],
    out: &mut [u8],
    dpos: usize,
    dst_count: usize,
    offset: usize,
    scratch: &mut Scratch,
) -> Result<()> {
    if mode > 1 {
        return Err(Error::Corrupt("kraken: bad mode"));
    }
    if src.len() < 13 {
        return Err(Error::Truncated);
    }

    let dst_size = dst_count;
    let mut dst_off = dpos; // where the next literal seed / output begins

    if offset == 0 {
        // 8-byte literal seed copied straight to the output start.
        if src.len() < 8 || dst_count < 8 {
            return Err(Error::Truncated);
        }
        out[dst_off..dst_off + 8].copy_from_slice(&src[..8]);
        dst_off += 8;
        src = &src[8..];
    }
    let _ = dst_off; // the run loop recomputes the seed skip itself.

    if src[0] & 0x80 != 0 {
        // `(flag & 0xc0) != 0x80` is reserved, and even the valid case is "excess bytes not
        // supported". Kraken streams in practice never set this for the lit/cmd preamble.
        return Err(Error::Corrupt("kraken: lz table flag unsupported"));
    }

    // 1) literal stream, bounded by dst_size.
    scratch.lit.clear();
    scratch.lit.resize(dst_size, 0);
    let (lit_len, n) = crate::entropy::decode_array_capped(src, &mut scratch.lit)?;
    scratch.lit.truncate(lit_len);
    src = &src[n..];

    // 2) command stream, bounded by dst_size.
    scratch.cmd.clear();
    scratch.cmd.resize(dst_size, 0);
    let (cmd_len, n) = crate::entropy::decode_array_capped(src, &mut scratch.cmd)?;
    scratch.cmd.truncate(cmd_len);
    src = &src[n..];

    if src.len() < 3 {
        return Err(Error::Truncated);
    }

    // 3) offset-mode flag, then packed offset stream (bounded by cmd_stream_size).
    let mut offs_scaling: i32 = 0;
    let mut have_extra = false;
    if src[0] & 0x80 != 0 {
        offs_scaling = (src[0] as i32) - 127;
        src = &src[1..];

        scratch.packed_offs.clear();
        scratch.packed_offs.resize(cmd_len, 0);
        let (po_len, n) = crate::entropy::decode_array_capped(src, &mut scratch.packed_offs)?;
        scratch.packed_offs.truncate(po_len);
        src = &src[n..];

        if offs_scaling != 1 {
            scratch.packed_offs_extra.clear();
            scratch.packed_offs_extra.resize(po_len, 0);
            let (pe_len, n) =
                crate::entropy::decode_array_capped(src, &mut scratch.packed_offs_extra)?;
            if pe_len != po_len {
                return Err(Error::Corrupt("kraken: offs extra size mismatch"));
            }
            scratch.packed_offs_extra.truncate(pe_len);
            src = &src[n..];
            have_extra = true;
        }
    } else {
        scratch.packed_offs.clear();
        scratch.packed_offs.resize(cmd_len.max(1), 0);
        let (po_len, n) = crate::entropy::decode_array_capped(src, &mut scratch.packed_offs)?;
        scratch.packed_offs.truncate(po_len);
        src = &src[n..];
    }

    // 4) packed litlen stream, bounded by dst_size/4.
    scratch.packed_len.clear();
    scratch.packed_len.resize((dst_size >> 2).max(1), 0);
    let (pl_len, n) = crate::entropy::decode_array_capped(src, &mut scratch.packed_len)?;
    scratch.packed_len.truncate(pl_len);
    src = &src[n..];

    // 5) the trailing bit-packed region drives the final offset + length int arrays.
    scratch.offs.clear();
    scratch.offs.resize(scratch.packed_offs.len(), 0);
    scratch.len.clear();
    scratch.len.resize(scratch.packed_len.len(), 0);

    unpack_offsets(
        src,
        offs_scaling,
        if have_extra {
            Some(&scratch.packed_offs_extra)
        } else {
            None
        },
        &scratch.packed_offs,
        &scratch.packed_len,
        &mut scratch.offs,
        &mut scratch.len,
    )?;

    Ok(())
}

/// A forward + backward bit reader over `src` produces the
/// distance ints (`offs`, negative) and expands the packed length bytes into `len` ints.
fn unpack_offsets(
    src: &[u8],
    multi_dist_scale: i32,
    packed_offs_extra: Option<&[u8]>,
    packed_offs: &[u8],
    packed_litlen: &[u8],
    offs: &mut [i32],
    len: &mut [i32],
) -> Result<()> {
    let mut a = BitReader::new(src);
    let mut b = BitReaderRev::new(src);

    // Read the count of explicit u32 lengths (a unary-prefixed value at the tail).
    let bits_b = b.peek_bits();
    if bits_b < 0x2000 {
        return Err(Error::Corrupt("kraken: unpack offsets prefix"));
    }
    let n = 31 - bsr(bits_b);
    b.skip_bits(n);
    let n = n + 1;
    let u32_len_stream_size = b.read_top(n)?.wrapping_sub(1);
    let u32_len_stream_size = u32_len_stream_size as usize;

    // Distances.
    if multi_dist_scale == 0 {
        // Traditional coding: alternate forward/backward ReadDistance per packed byte.
        let mut i = 0usize;
        while i < packed_offs.len() {
            offs[i] = -(a.read_distance(packed_offs[i] as u32)? as i32);
            i += 1;
            if i >= packed_offs.len() {
                break;
            }
            offs[i] = -(b.read_distance(packed_offs[i] as u32)? as i32);
            i += 1;
        }
    } else {
        // 2-table coding: offs = ((8 + (cmd&7)) << (cmd>>3)) | extra_bits.
        let mut i = 0usize;
        while i < packed_offs.len() {
            let cmd = packed_offs[i] as u32;
            if (cmd >> 3) > 26 {
                return Err(Error::Corrupt("kraken: offs cmd range"));
            }
            let extra = a.read_more_than_24_bits(cmd >> 3)?;
            let o = ((8 + (cmd & 7)) << (cmd >> 3)) | extra;
            offs[i] = 8 - (o as i32);
            i += 1;
            if i >= packed_offs.len() {
                break;
            }
            let cmd = packed_offs[i] as u32;
            if (cmd >> 3) > 26 {
                return Err(Error::Corrupt("kraken: offs cmd range"));
            }
            let extra = b.read_more_than_24_bits(cmd >> 3)?;
            let o = ((8 + (cmd & 7)) << (cmd >> 3)) | extra;
            offs[i] = 8 - (o as i32);
            i += 1;
        }
        if multi_dist_scale != 1 {
            let extra = packed_offs_extra.ok_or(Error::Corrupt("kraken: missing offs extra"))?;
            for (k, o) in offs.iter_mut().enumerate() {
                let low = *extra.get(k).ok_or(Error::Corrupt("kraken: offs extra short"))? as i32;
                *o = multi_dist_scale * *o - low;
            }
        }
    }

    // Explicit u32 lengths read alternately forwards/backwards.
    if u32_len_stream_size > 512 {
        return Err(Error::Corrupt("kraken: too many u32 lengths"));
    }
    let mut u32_len = [0u32; 512];
    let mut i = 0usize;
    while i + 1 < u32_len_stream_size {
        u32_len[i] = a.read_length()?;
        u32_len[i + 1] = b.read_length()?;
        i += 2;
    }
    if i < u32_len_stream_size {
        u32_len[i] = a.read_length()?;
    }

    // Both readers must meet at the same byte after rewinding their unconsumed slack.
    if !BitReader::meets(&a, &b) {
        return Err(Error::Corrupt("kraken: unpack offsets readers diverge"));
    }

    // Expand the packed litlen bytes into final lengths (+3), substituting explicit u32s for 255.
    let mut u = 0usize;
    for (i, &v) in packed_litlen.iter().enumerate() {
        let mut v = v as u32;
        if v == 255 {
            let extra = *u32_len.get(u).ok_or(Error::Corrupt("kraken: u32 len underflow"))?;
            v = extra.wrapping_add(255);
            u += 1;
        }
        len[i] = v.wrapping_add(3) as i32;
    }
    if u != u32_len_stream_size {
        return Err(Error::Corrupt("kraken: u32 len leftover"));
    }

    Ok(())
}

/// Run the literal/match copy loop for one chunk.
///
/// `mode 0` (Type0) reconstructs literals as `lit + out[dpos+last_offset]` (a delta from the
/// most-recent-offset byte); `mode 1` (Type1) copies literals verbatim. Matches use a 7-entry
/// recent-offset cache.
fn process_lz_runs<F: FnMut(LzEvent)>(
    mode: u8,
    out: &mut [u8],
    dpos: usize,
    dst_count: usize,
    base: usize,
    scratch: &Scratch,
    observe: &mut F,
) -> Result<()> {
    // The 8-byte literal seed at quantum start is already in `out`; the run loop begins after it.
    let start = if dpos == base { dpos + 8 } else { dpos };
    let dst_end = dpos + dst_count;
    let sub = mode == 0;

    let cmd = &scratch.cmd[..];
    let lit = &scratch.lit[..];
    let offs = &scratch.offs[..];
    let len = &scratch.len[..];

    let mut d = start;
    let mut ci = 0usize; // cmd index
    let mut li = 0usize; // literal index
    let mut oi = 0usize; // offs index
    let mut leni = 0usize; // len index

    // recent_offs[3..=5] seeded to -8; index `offs_index+3` selects, with 6 holding the fresh
    // value pulled from the offs stream.
    let mut recent: [i32; 7] = [0, 0, 0, -8, -8, -8, 0];
    let mut last_offset: i32 = -8;

    while ci < cmd.len() {
        let f = cmd[ci] as u32;
        ci += 1;
        let mut litlen = (f & 3) as usize;
        let offs_index = (f >> 6) as usize;
        let matchlen = ((f >> 2) & 0xF) as usize;

        // litlen == 3 → pull a long length from the len stream.
        if litlen == 3 {
            litlen = *len.get(leni).ok_or(Error::Corrupt("kraken: len underflow"))? as usize;
            leni += 1;
        }
        // Fresh offset candidate from the offs stream into slot 6.
        recent[6] = offs.get(oi).copied().unwrap_or(0);

        // Copy literals.
        copy_literals(out, &mut d, lit, &mut li, litlen, last_offset, sub, dst_end)?;

        // Resolve the offset via the recent-offset cache.
        let offset = recent[offs_index + 3];
        recent[offs_index + 3] = recent[offs_index + 2];
        recent[offs_index + 2] = recent[offs_index + 1];
        recent[offs_index + 1] = recent[offs_index];
        recent[3] = offset;
        last_offset = offset;
        // Advance offs stream only when a non-recent (index 0) offset was used:
        // `offs_stream += ((offs_index + 1) & 4) / 4`.
        oi += ((offs_index + 1) & 4) >> 2;

        // Bounds: copyfrom = d + offset must be >= base (history floor) and within bounds.
        let copyfrom_signed = d as isize + offset as isize;
        if copyfrom_signed < base as isize {
            return Err(Error::Corrupt("kraken: match offset out of bounds"));
        }
        let copyfrom = copyfrom_signed as usize;

        let m = if matchlen != 15 {
            matchlen + 2
        } else {
            let extra = *len.get(leni).ok_or(Error::Corrupt("kraken: len underflow"))? as usize;
            leni += 1;
            14 + extra
        };
        if m > dst_end - d {
            return Err(Error::Corrupt("kraken: match length out of bounds"));
        }
        observe(LzEvent::Match { match_start: d - dpos, match_end: d + m - dpos });
        copy_match(out, d, copyfrom, m);
        d += m;
    }

    // The remaining output is a final literal run.
    if oi != offs.len() || leni != len.len() {
        return Err(Error::Corrupt("kraken: stream not fully consumed"));
    }
    let final_len = dst_end - d;
    if li > lit.len() || final_len != lit.len() - li {
        return Err(Error::Corrupt("kraken: final literal length mismatch"));
    }
    if sub {
        for _ in 0..final_len {
            let pi = d as isize + last_offset as isize;
            if pi < 0 {
                return Err(Error::Corrupt("kraken: predictor out of bounds"));
            }
            let pred = out[pi as usize];
            out[d] = lit[li].wrapping_add(pred);
            d += 1;
            li += 1;
        }
    } else {
        out[d..d + final_len].copy_from_slice(&lit[li..li + final_len]);
    }

    Ok(())
}

/// Copy `litlen` literal bytes into `out[d..]`. In `sub` mode each byte is added to the byte
/// `last_offset` back (the most-recent-offset prediction); otherwise it's copied verbatim.
#[inline]
fn copy_literals(
    out: &mut [u8],
    d: &mut usize,
    lit: &[u8],
    li: &mut usize,
    litlen: usize,
    last_offset: i32,
    sub: bool,
    dst_end: usize,
) -> Result<()> {
    if *d > dst_end || litlen > dst_end - *d {
        return Err(Error::Corrupt("kraken: literal run out of bounds"));
    }
    if *li + litlen > lit.len() {
        return Err(Error::Corrupt("kraken: literal stream underflow"));
    }
    if sub {
        // last_offset is negative; the predictor sits within already-written output. The
        // smallest predictor index across this run is at the first byte, so one check covers it.
        if (*d as isize) + (last_offset as isize) < 0 {
            return Err(Error::Corrupt("kraken: predictor out of bounds"));
        }
        for _ in 0..litlen {
            let pred = out[(*d as isize + last_offset as isize) as usize];
            out[*d] = lit[*li].wrapping_add(pred);
            *d += 1;
            *li += 1;
        }
    } else {
        out[*d..*d + litlen].copy_from_slice(&lit[*li..*li + litlen]);
        *d += litlen;
        *li += litlen;
    }
    Ok(())
}

/// Copy a match of `m` bytes from `copyfrom` to `dst` within `out`, byte-by-byte (so
/// overlapping copies with small offsets replicate correctly).
#[inline]
fn copy_match(out: &mut [u8], dst: usize, copyfrom: usize, m: usize) {
    for k in 0..m {
        out[dst + k] = out[copyfrom + k];
    }
}

/// Bit-scan-reverse: index of the most-significant set bit. `x` must be non-zero.
#[inline]
fn bsr(x: u32) -> u32 {
    debug_assert!(x != 0);
    31 - x.leading_zeros()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn read_vector(name: &str) -> Vec<u8> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/vectors/");
        std::fs::read(std::format!("{path}{name}")).expect("read test vector")
    }

    /// The framing fast-paths: a memset quantum and a stream-level uncompressed quantum decode
    /// without touching the LZ loop.
    #[test]
    fn memset_quantum_fills() {
        // `07 ff ff 5a` memset of value 0x5a; stream header `0c 06`, out_len small.
        let stream = [0x0c, 0x06, 0x07, 0xff, 0xff, 0x5a];
        let out = decompress(&stream, 10).unwrap();
        assert_eq!(out, vec![0x5a; 10]);
    }

    #[test]
    fn uncompressed_quantum_copies_raw() {
        // `cc 06` (uncompressed bit set), then raw bytes follow with no quantum header.
        let mut stream = vec![0xcc, 0x06];
        let payload: Vec<u8> = (0..40u8).collect();
        stream.extend_from_slice(&payload);
        let out = decompress(&stream, payload.len()).unwrap();
        assert_eq!(out, payload);
    }

    /// End-to-end: every in-tree real vector decodes to its `.raw` (mirrors the integration
    /// test, but exercised in-crate so a decode regression fails the unit suite too).
    #[test]
    fn all_vectors_decode_in_crate() {
        let cases: &[(&str, usize)] = &[
            ("one_byte", 1),
            ("zeros_64k", 65_536),
            ("repetitive", 40_000),
            ("text", 185),
            ("counter", 200_000),
            ("multiblock", 600_000),
            ("random", 50_000),
        ];
        for (name, len) in cases {
            let krk = read_vector(&std::format!("{name}.krk"));
            let raw = read_vector(&std::format!("{name}.raw"));
            assert_eq!(raw.len(), *len, "{name} raw len");
            let out = decompress(&krk, *len)
                .unwrap_or_else(|e| panic!("decode {name}: {e:?}"));
            assert_eq!(out, raw, "{name} roundtrip");
        }
    }

    /// A non-Kraken codec id in the stream header is rejected (we only implement Kraken).
    #[test]
    fn rejects_non_kraken_codec() {
        // `0c 0a` -> codec id 10 (Mermaid). Must error, not mis-decode.
        let stream = [0x0c, 0x0a, 0x07, 0xff, 0xff, 0x00];
        assert!(decompress(&stream, 4).is_err());
    }

    /// A hostile `decompressed_len` must fail the fallible allocation rather than abort.
    #[test]
    fn rejects_unallocatable_size() {
        let r = decompress(&[0xcc, 0x06], usize::MAX - 16);
        assert!(matches!(r, Err(Error::OutputTooLarge(_))));
    }
}
