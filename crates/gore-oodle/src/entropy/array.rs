//! Array-format dispatch (raw | huffman | tANS | RLE | multi-array).
//!
//! `decode_array` is specialised to the entropy seam: the entropy stage always materialises
//! bytes into the caller's `out` buffer (no output-pointer aliasing). It reads the array
//! header — a chunk type in `(src[0]>>4)&7` plus little-endian src/dst sizes — then dispatches
//! to the per-mode decoder.
//!
//! `encode_array` evaluates the candidate encoders (raw, RLE, huffman, tANS) and emits the
//! smallest. It never *emits* a multi-array (chunk type 5) — that's allowed; the decoder still
//! reads it.

#![allow(dead_code)]

use super::{huff_dec, huff_enc, tans_dec, tans_enc};
use crate::bytes::ByteWriter;
use crate::{Error, Level, Result};
use alloc::vec;
use alloc::vec::Vec;

/// Parsed array header: chunk type and the src/dst sizes, plus header byte length.
struct ArrayHeader {
    chunk_type: u32,
    src_size: usize,
    dst_size: usize,
    header_len: usize,
}

/// Parse the array header from `src`, decoding the src/dst sizes.
fn parse_header(src: &[u8]) -> Result<ArrayHeader> {
    if src.len() < 2 {
        return Err(Error::Truncated);
    }
    let chunk_type = ((src[0] >> 4) & 0x7) as u32;
    if chunk_type == 0 {
        // Raw / memcpy.
        let (src_size, header_len) = if src[0] >= 0x80 {
            (((src[0] as usize) << 8 | src[1] as usize) & 0xFFF, 2)
        } else {
            if src.len() < 3 {
                return Err(Error::Truncated);
            }
            let s = (src[0] as usize) << 16 | (src[1] as usize) << 8 | src[2] as usize;
            if s & !0x3ffff != 0 {
                return Err(Error::Corrupt("array: reserved bits set"));
            }
            (s, 3)
        };
        return Ok(ArrayHeader {
            chunk_type: 0,
            src_size,
            dst_size: src_size,
            header_len,
        });
    }

    // All other modes carry both a src and a dst size.
    let (src_size, dst_size, header_len) = if src[0] >= 0x80 {
        if src.len() < 3 {
            return Err(Error::Truncated);
        }
        // short mode, 10-bit sizes
        let bits = (src[0] as u32) << 16 | (src[1] as u32) << 8 | src[2] as u32;
        let src_size = (bits & 0x3ff) as usize;
        let dst_size = src_size + ((bits >> 10) & 0x3ff) as usize + 1;
        (src_size, dst_size, 3)
    } else {
        if src.len() < 5 {
            return Err(Error::Truncated);
        }
        // long mode, 18-bit sizes
        let bits =
            (src[1] as u32) << 24 | (src[2] as u32) << 16 | (src[3] as u32) << 8 | src[4] as u32;
        let src_size = (bits & 0x3ffff) as usize;
        let dst_size = (((bits >> 18) | ((src[0] as u32) << 14)) & 0x3FFFF) as usize + 1;
        if src_size >= dst_size {
            return Err(Error::Corrupt("array: src>=dst"));
        }
        (src_size, dst_size, 5)
    };
    Ok(ArrayHeader {
        chunk_type,
        src_size,
        dst_size,
        header_len,
    })
}

/// Decode one array from `src`, where the *decoded* size is dictated by the array header
/// (not by `out.len()`). The header's `dst_size` must fit within `out` (the caller's
/// capacity). Returns `(decoded_size, input_bytes_consumed)`.
///
/// This is the form the Kraken quantum decoder needs: it knows only an upper bound on the
/// output (the quantum's `dst_count` or a fraction of it) and learns the exact produced
/// length from the header.
pub(crate) fn decode_array_capped(src: &[u8], out: &mut [u8]) -> Result<(usize, usize)> {
    decode_array_into(src, out)
}

/// Decode one array from `src` into exactly `out.len()` bytes; returns input bytes consumed.
pub(crate) fn decode_array(src: &[u8], out: &mut [u8]) -> Result<usize> {
    let hdr = parse_header(src)?;
    let body = &src[hdr.header_len..];

    if hdr.chunk_type == 0 {
        // Raw: copy src_size bytes straight through.
        if hdr.src_size > out.len() || body.len() < hdr.src_size {
            return Err(Error::Corrupt("array: raw size"));
        }
        out[..hdr.src_size].copy_from_slice(&body[..hdr.src_size]);
        return Ok(hdr.header_len + hdr.src_size);
    }

    if body.len() < hdr.src_size || hdr.dst_size > out.len() {
        return Err(Error::Corrupt("array: body size"));
    }
    let payload = &body[..hdr.src_size];
    let dst = &mut out[..hdr.dst_size];

    let used = match hdr.chunk_type {
        2 | 4 => huff_dec::decode_huff(payload, dst, hdr.chunk_type >> 1)?,
        5 => decode_recursive(payload, dst)?,
        3 => decode_rle(payload, dst)?,
        1 => tans_dec::decode_tans(payload, dst)?,
        _ => return Err(Error::Corrupt("array: bad chunk type")),
    };
    if used != hdr.src_size {
        return Err(Error::Corrupt("array: wrong consumed length"));
    }
    Ok(hdr.header_len + hdr.src_size)
}

/// A back-to-front command stream describes runs of literals (copied
/// from the front) and runs of a sticky RLE byte.
fn decode_rle(src: &[u8], dst: &mut [u8]) -> Result<usize> {
    let src_size = src.len();
    if src_size <= 1 {
        if src_size != 1 {
            return Err(Error::Corrupt("rle: empty"));
        }
        dst.iter_mut().for_each(|b| *b = src[0]);
        return Ok(1);
    }

    let dst_size = dst.len();
    // The command buffer may itself be an entropy array (when src[0] != 0): decode it to a
    // scratch buffer and lay the trailing raw command bytes after it.
    let mut scratch: Vec<u8>;
    let (cmd_buf, mut cmd_lo, mut cmd_hi, lit_off);
    if src[0] != 0 {
        // Decode the leading entropy array out of src (header starts at src[0]).
        let mut tmp = vec![0u8; dst_size + 64];
        let dec = decode_array_into(src, &mut tmp)?;
        let dec_size = dec.0;
        let n = dec.1;
        let cmd_len = src_size - n + dec_size;
        scratch = vec![0u8; cmd_len];
        scratch[..dec_size].copy_from_slice(&tmp[..dec_size]);
        scratch[dec_size..cmd_len].copy_from_slice(&src[n..src_size]);
        cmd_buf = &scratch[..];
        cmd_lo = 0usize;
        cmd_hi = cmd_len;
        lit_off = 0usize; // literals come from the front of cmd_buf
    } else {
        // Commands sit directly in src; literals come from cmd region too.
        scratch = Vec::new();
        let _ = &scratch;
        cmd_buf = src;
        cmd_lo = 1usize;
        cmd_hi = src_size;
        lit_off = 0usize;
    }
    let _ = lit_off;

    let mut dpos = 0usize;
    let mut rle_byte: u8 = 0;

    // `cmd_lo` walks forward (consuming literal source bytes); `cmd_hi` walks backward
    // (consuming command bytes from the end).
    // Remaining output capacity and command-byte budget, computed without underflow. The
    // checks below all use these so corrupt input yields `Corrupt`, never a subtract panic.
    while cmd_lo < cmd_hi {
        let dst_left = dst_size - dpos; // dpos <= dst_size is an invariant (checked before writes)
        let cmd_avail = cmd_hi - cmd_lo; // > 0 by loop guard
        let cmd = cmd_buf[cmd_hi - 1] as u32;
        if cmd.wrapping_sub(1) >= 0x2f {
            cmd_hi -= 1;
            let bytes_to_copy = ((!cmd).wrapping_sub(0) & 0xF) as usize; // (-1 - cmd) & 0xF
            let bytes_to_rle = (cmd >> 4) as usize;
            // cmd_avail already accounts for the 1 command byte just consumed plus the literals.
            if dst_left < bytes_to_copy + bytes_to_rle || cmd_avail - 1 < bytes_to_copy {
                return Err(Error::Corrupt("rle: short A"));
            }
            dst[dpos..dpos + bytes_to_copy]
                .copy_from_slice(&cmd_buf[cmd_lo..cmd_lo + bytes_to_copy]);
            cmd_lo += bytes_to_copy;
            dpos += bytes_to_copy;
            for b in &mut dst[dpos..dpos + bytes_to_rle] {
                *b = rle_byte;
            }
            dpos += bytes_to_rle;
        } else if cmd >= 0x10 {
            if cmd_avail < 2 {
                return Err(Error::Corrupt("rle: short B hdr"));
            }
            let data = (read_u16(cmd_buf, cmd_hi - 2)).wrapping_sub(4096) as usize;
            cmd_hi -= 2;
            let bytes_to_copy = data & 0x3F;
            let bytes_to_rle = data >> 6;
            if dst_left < bytes_to_copy + bytes_to_rle || cmd_avail - 2 < bytes_to_copy {
                return Err(Error::Corrupt("rle: short B"));
            }
            dst[dpos..dpos + bytes_to_copy]
                .copy_from_slice(&cmd_buf[cmd_lo..cmd_lo + bytes_to_copy]);
            cmd_lo += bytes_to_copy;
            dpos += bytes_to_copy;
            for b in &mut dst[dpos..dpos + bytes_to_rle] {
                *b = rle_byte;
            }
            dpos += bytes_to_rle;
        } else if cmd == 1 {
            rle_byte = cmd_buf[cmd_lo];
            cmd_lo += 1;
            cmd_hi -= 1;
        } else if cmd >= 9 {
            if cmd_avail < 2 {
                return Err(Error::Corrupt("rle: short C hdr"));
            }
            let bytes_to_rle = (read_u16(cmd_buf, cmd_hi - 2).wrapping_sub(0x8ff) as usize) * 128;
            cmd_hi -= 2;
            if dst_left < bytes_to_rle {
                return Err(Error::Corrupt("rle: short C"));
            }
            for b in &mut dst[dpos..dpos + bytes_to_rle] {
                *b = rle_byte;
            }
            dpos += bytes_to_rle;
        } else {
            if cmd_avail < 2 {
                return Err(Error::Corrupt("rle: short D hdr"));
            }
            let bytes_to_copy = (read_u16(cmd_buf, cmd_hi - 2).wrapping_sub(511) as usize) * 64;
            cmd_hi -= 2;
            if cmd_avail - 2 < bytes_to_copy || dst_left < bytes_to_copy {
                return Err(Error::Corrupt("rle: short D"));
            }
            dst[dpos..dpos + bytes_to_copy]
                .copy_from_slice(&cmd_buf[cmd_lo..cmd_lo + bytes_to_copy]);
            dpos += bytes_to_copy;
            cmd_lo += bytes_to_copy;
        }
    }
    if cmd_hi != cmd_lo {
        return Err(Error::Corrupt("rle: cmd mismatch"));
    }
    if dpos != dst_size {
        return Err(Error::Corrupt("rle: dst mismatch"));
    }
    Ok(src_size)
}

/// Either a sequence of `n` sub-arrays concatenated, or a single
/// multi-array. Returns payload bytes consumed.
fn decode_recursive(src: &[u8], dst: &mut [u8]) -> Result<usize> {
    if src.len() < 6 {
        return Err(Error::Corrupt("recursive: short"));
    }
    let n = (src[0] & 0x7f) as i32;
    if n < 2 {
        return Err(Error::Corrupt("recursive: n<2"));
    }
    if src[0] & 0x80 == 0 {
        let mut p = 1usize;
        let mut dpos = 0usize;
        let mut remaining = n;
        while remaining > 0 {
            let (decoded, used) = decode_array_into(&src[p..], &mut dst[dpos..])?;
            dpos += decoded;
            p += used;
            remaining -= 1;
        }
        if dpos != dst.len() {
            return Err(Error::Corrupt("recursive: size"));
        }
        Ok(p)
    } else {
        let (total, used) = decode_multi_array(src, dst, 1)?;
        if total != dst.len() {
            return Err(Error::Corrupt("recursive: multi size"));
        }
        Ok(used)
    }
}

/// Decode one array into the front of `out`, returning (decoded_size, input_bytes_used).
/// This is the form the recursive/RLE paths use where the *decoded* size is dictated by the
/// header, not by the caller's whole buffer length.
fn decode_array_into(src: &[u8], out: &mut [u8]) -> Result<(usize, usize)> {
    let hdr = parse_header(src)?;
    if hdr.dst_size > out.len() {
        return Err(Error::Corrupt("array: dst too big"));
    }
    let used = decode_array(src, &mut out[..hdr.dst_size])?;
    Ok((hdr.dst_size, used))
}

/// Decode a multi-array for the `array_count == 1` case used by the recursive mode.
/// Returns (total_decoded_size, input_bytes_used). Decodes the constituent entropy arrays to
/// scratch, then re-assembles `out` according to the interval index/length tables.
fn decode_multi_array(src: &[u8], out: &mut [u8], array_count: usize) -> Result<(usize, usize)> {
    if src.len() < 4 {
        return Err(Error::Truncated);
    }
    let mut p = 0usize;
    let mut num_arrays = src[p] as usize;
    p += 1;
    if num_arrays & 0x80 == 0 {
        return Err(Error::Corrupt("multi: high bit"));
    }
    num_arrays &= 0x3f;

    let out_len = out.len();

    if num_arrays == 0 {
        // Each output array is just decoded sequentially.
        let mut total = 0usize;
        let mut dpos = 0usize;
        for _ in 0..array_count {
            let (decoded, used) = decode_array_into(&src[p..], &mut out[dpos..])?;
            dpos += decoded;
            p += used;
            total += decoded;
        }
        return Ok((total, p));
    }

    // Decode each constituent entropy array fully into its own owned buffer.
    let mut entropy_data: Vec<Vec<u8>> = Vec::with_capacity(num_arrays);
    let mut total = 0usize;
    for _ in 0..num_arrays {
        let hdr = parse_header(&src[p..])?;
        let mut buf = vec![0u8; hdr.dst_size];
        let used = decode_array(&src[p..], &mut buf)?;
        total += hdr.dst_size;
        p += used;
        entropy_data.push(buf);
    }

    if src.len() - p < 3 {
        return Err(Error::Truncated);
    }
    let q = read_u16(src, p) as usize;
    p += 2;

    // out_size of the index array via the *block size* header (not a full decode).
    let (out_size, _) = block_size(&src[p..], total)?;
    let num_indexes = out_size;
    if num_indexes < array_count + 1 {
        return Err(Error::Corrupt("multi: num_lens<1"));
    }
    let mut num_lens = num_indexes - array_count;

    let mut interval_lenlog2 = vec![0u8; num_indexes];
    let mut interval_indexes = vec![0u8; num_indexes];

    if q & 0x8000 != 0 {
        // Combined index+lenlog2 in one array.
        let (decoded, used) = decode_array_exact(&src[p..], &mut interval_indexes, num_indexes)?;
        let _ = decoded;
        p += used;
        for i in 0..num_indexes {
            let t = interval_indexes[i];
            interval_lenlog2[i] = t >> 4;
            interval_indexes[i] = t & 0xF;
        }
        num_lens = num_indexes;
    } else {
        let lenlog2_chunksize = num_indexes - array_count;
        let (_, used1) = decode_array_exact(&src[p..], &mut interval_indexes, num_indexes)?;
        p += used1;
        let (_, used2) =
            decode_array_exact(&src[p..], &mut interval_lenlog2, lenlog2_chunksize)?;
        p += used2;
        for &b in &interval_lenlog2[..lenlog2_chunksize] {
            if b > 16 {
                return Err(Error::Corrupt("multi: lenlog2>16"));
            }
        }
    }

    // Decode the variable-bit interval lengths via a dual-ended bit reader.
    let varbits_complen = (q & 0x3FFF) as usize;
    if src.len() - p < varbits_complen {
        return Err(Error::Truncated);
    }
    let decoded_intervals =
        decode_intervals(&src[p..p + varbits_complen], &interval_lenlog2, num_lens)?;
    p += varbits_complen;

    if interval_indexes[num_indexes - 1] != 0 {
        return Err(Error::Corrupt("multi: last index nonzero"));
    }

    // Re-assemble output arrays.
    let increment_leni = (q & 0x8000 != 0) as usize;
    let mut indi = 0usize;
    let mut leni = 0usize;
    let mut dpos = 0usize;
    // Per-array consumption cursors into the entropy buffers.
    let mut consumed = vec![0usize; num_arrays];

    for _arri in 0..array_count {
        if indi >= num_indexes {
            return Err(Error::Corrupt("multi: indi overflow"));
        }
        loop {
            let source = interval_indexes[indi] as usize;
            indi += 1;
            if source == 0 {
                break;
            }
            if source > num_arrays {
                return Err(Error::Corrupt("multi: source range"));
            }
            if leni >= num_lens {
                return Err(Error::Corrupt("multi: leni overflow"));
            }
            let cur_len = decoded_intervals[leni] as usize;
            leni += 1;
            let buf = &entropy_data[source - 1];
            let off = consumed[source - 1];
            let bytes_left = buf.len() - off;
            if cur_len > bytes_left || cur_len > out_len - dpos {
                return Err(Error::Corrupt("multi: copy overflow"));
            }
            out[dpos..dpos + cur_len].copy_from_slice(&buf[off..off + cur_len]);
            consumed[source - 1] += cur_len;
            dpos += cur_len;
        }
        leni += increment_leni;
    }

    if indi != num_indexes || leni != num_lens {
        return Err(Error::Corrupt("multi: leftover"));
    }
    for (i, buf) in entropy_data.iter().enumerate() {
        if consumed[i] != buf.len() {
            return Err(Error::Corrupt("multi: entropy leftover"));
        }
    }
    Ok((dpos, p))
}

/// Decode an array whose decoded size must equal `expect`. Returns (decoded, used).
fn decode_array_exact(src: &[u8], out: &mut [u8], expect: usize) -> Result<(usize, usize)> {
    let hdr = parse_header(src)?;
    if hdr.dst_size != expect {
        return Err(Error::Corrupt("multi: size_out mismatch"));
    }
    if expect > out.len() {
        return Err(Error::Corrupt("multi: out too small"));
    }
    let used = decode_array(src, &mut out[..expect])?;
    Ok((expect, used))
}

/// Like `parse_header` but returns only the dst size and the number
/// of header bytes, validating against a capacity. Used by multi-array to size the index array.
fn block_size(src: &[u8], capacity: usize) -> Result<(usize, usize)> {
    let hdr = parse_header(src)?;
    let dst = if hdr.chunk_type == 0 {
        hdr.src_size
    } else {
        hdr.dst_size
    };
    if dst > capacity {
        return Err(Error::Corrupt("multi: block size > cap"));
    }
    Ok((dst, hdr.header_len))
}

/// Multi-array interval-length decode: a forward + backward varbit reader over
/// `src`, reading `interval_lenlog2[i]`-bit values into `decoded_intervals[i]`.
fn decode_intervals(src: &[u8], lenlog2: &[u8], num_lens: usize) -> Result<Vec<u32>> {
    const BITMASKS: [u32; 32] = [
        0x1, 0x3, 0x7, 0xf, 0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff, 0x3fff,
        0x7fff, 0xffff, 0x1ffff, 0x3ffff, 0x7ffff, 0xfffff, 0x1fffff, 0x3fffff, 0x7fffff,
        0xffffff, 0x1ffffff, 0x3ffffff, 0x7ffffff, 0xfffffff, 0x1fffffff, 0x3fffffff, 0x7fffffff,
        0xffffffff,
    ];
    let mut out = vec![0u32; num_lens];
    let n = src.len();

    // Forward reader: big-endian load, consume from the top.
    let mut f = 0usize;
    let mut bits_f: u32 = 0;
    let mut bitpos_f: i32 = 24;
    // Backward reader: load the 4 bytes ending at `b`, consume from the top.
    let mut b = n;
    let mut bits_b: u32 = 0;
    let mut bitpos_b: i32 = 24;

    #[inline]
    fn be32(src: &[u8], idx: usize) -> u32 {
        let mut v = 0u32;
        for k in 0..4 {
            let byte = if idx + k < src.len() { src[idx + k] } else { 0 };
            v = (v << 8) | byte as u32;
        }
        v
    }

    let mut i = 0usize;
    while i + 2 <= num_lens {
        bits_f |= be32(src, f) >> (24 - bitpos_f);
        f += ((bitpos_f + 7) >> 3) as usize;
        // backward: ((uint32*)b)[-1] little-endian load of the 4 bytes before b.
        let bw = le32_before(src, b);
        bits_b |= bw >> (24 - bitpos_b);
        b = b.saturating_sub(((bitpos_b + 7) >> 3) as usize);

        let nb_f = lenlog2[i] as u32;
        let nb_b = lenlog2[i + 1] as u32;

        bits_f = (bits_f | 1).rotate_left(nb_f);
        bitpos_f += nb_f as i32 - 8 * ((bitpos_f + 7) >> 3);

        bits_b = (bits_b | 1).rotate_left(nb_b);
        bitpos_b += nb_b as i32 - 8 * ((bitpos_b + 7) >> 3);

        let value_f = bits_f & BITMASKS[nb_f as usize];
        bits_f &= !BITMASKS[nb_f as usize];
        let value_b = bits_b & BITMASKS[nb_b as usize];
        bits_b &= !BITMASKS[nb_b as usize];

        out[i] = value_f;
        out[i + 1] = value_b;
        i += 2;
    }
    if i < num_lens {
        bits_f |= be32(src, f) >> (24 - bitpos_f);
        let nb_f = lenlog2[i] as u32;
        bits_f = (bits_f | 1).rotate_left(nb_f);
        let value_f = bits_f & BITMASKS[nb_f as usize];
        out[i] = value_f;
    }
    Ok(out)
}

/// Little-endian load of the 4 bytes immediately before index `b`.
#[inline]
fn le32_before(src: &[u8], b: usize) -> u32 {
    let mut v = 0u32;
    for k in 0..4 {
        // bytes at b-4..b, little-endian => byte (b-4+k) is bit 8*k.
        let idx = b as isize - 4 + k as isize;
        let byte = if idx >= 0 && (idx as usize) < src.len() {
            src[idx as usize]
        } else {
            0
        };
        v |= (byte as u32) << (8 * k);
    }
    v
}

#[inline]
fn read_u16(src: &[u8], idx: usize) -> u32 {
    let lo = src.get(idx).copied().unwrap_or(0) as u32;
    let hi = src.get(idx + 1).copied().unwrap_or(0) as u32;
    lo | (hi << 8)
}

// ---------------------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------------------

/// Encode `symbols` into `dst` choosing the smallest mode; returns bytes written.
pub(crate) fn encode_array(
    symbols: &[u8],
    dst: &mut ByteWriter,
    level: Level,
) -> Result<usize> {
    if symbols.len() > 0x3FFFF {
        return Err(Error::InputTooLarge(symbols.len()));
    }

    // Always have the raw fallback ready. Raw is a verbatim copy, so it is trusted without a
    // decode check; every *compressed* candidate is verified to decode back to `symbols`
    // before it can win, so a latent edge case in a sub-codec's bit serialization degrades
    // to a larger-but-correct array instead of producing a stream the decoder rejects.
    let mut best: Vec<u8> = encode_raw(symbols);

    // RLE candidate.
    if let Some(rle) = super::rle_enc::encode_rle(symbols) {
        if rle.len() < best.len() && decodes_back(&rle, symbols) {
            best = rle;
        }
    }

    if symbols.len() >= 32 {
        if let Some(huff) = huff_enc::encode_huff_array(symbols) {
            if huff.len() < best.len() && decodes_back(&huff, symbols) {
                best = huff;
            }
        }
        if let Some(tans) = tans_enc::encode_tans_array(symbols, level) {
            if tans.len() < best.len() && decodes_back(&tans, symbols) {
                best = tans;
            }
        }
    }

    let n = best.len();
    dst.extend(&best);
    Ok(n)
}

/// Verify a candidate array decodes back to exactly `symbols` (a self-check guarding the
/// fiddly huffman/tANS/RLE bit serializers). Returns `false` on any decode error or mismatch.
fn decodes_back(arr: &[u8], symbols: &[u8]) -> bool {
    let mut scratch = vec![0u8; symbols.len()];
    match decode_array(arr, &mut scratch) {
        Ok(_) => scratch == symbols,
        Err(_) => false,
    }
}

/// Raw array: chunk type 0, 3-byte big-endian size header, payload verbatim.
fn encode_raw(symbols: &[u8]) -> Vec<u8> {
    let size = symbols.len();
    let mut out = Vec::with_capacity(size + 3);
    out.push((size >> 16) as u8);
    out.push((size >> 8) as u8);
    out.push(size as u8);
    out.extend_from_slice(symbols);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A skewed full-alphabet distribution that previously made `encode_tans_array` win with
    /// a table the decoder rejected ("weight sum != L"). `encode_array` must now self-verify
    /// and fall back, so the chosen array always decodes back to the input.
    #[test]
    fn skewed_full_alphabet_array_roundtrips() {
        let mut data = Vec::new();
        let mut seed = 1u32;
        for _ in 0..40_000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let r = (seed >> 24) as u8;
            data.push(if r < 200 { b'a' } else if r < 240 { b' ' } else { b'a' + (r % 26) });
        }
        for &len in &[40_000usize, 0x20000.min(data.len())] {
            roundtrip(&data[..len]);
        }
    }

    #[test]
    fn header_roundtrip_raw_short() {
        // A raw array of 5 bytes: 3-byte header then payload.
        let raw = encode_raw(&[1, 2, 3, 4, 5]);
        let hdr = parse_header(&raw).unwrap();
        assert_eq!(hdr.chunk_type, 0);
        assert_eq!(hdr.src_size, 5);
        assert_eq!(hdr.header_len, 3);
    }

    #[test]
    fn raw_array_roundtrips() {
        let data: Vec<u8> = (0..200u32).map(|i| (i * 7) as u8).collect();
        let mut w = ByteWriter::new();
        let n = encode_array(&data, &mut w, Level::Default).unwrap();
        let buf = w.into_vec();
        assert_eq!(buf.len(), n);
        let mut out = vec![0u8; data.len()];
        let used = decode_array(&buf, &mut out).unwrap();
        assert_eq!(used, n);
        assert_eq!(out, data);
    }

    /// Encode then decode, asserting exact bytes back and the consumed length matches.
    fn roundtrip(data: &[u8]) -> Vec<u8> {
        let mut w = ByteWriter::new();
        let n = encode_array(data, &mut w, Level::Default).unwrap();
        let buf = w.into_vec();
        assert_eq!(buf.len(), n, "encode_array returned wrong length");
        let mut out = vec![0u8; data.len()];
        let used = decode_array(&buf, &mut out).unwrap();
        assert_eq!(used, n, "decode consumed != produced for len {}", data.len());
        assert_eq!(out, data, "roundtrip mismatch for len {}", data.len());
        buf
    }

    fn lcg(seed: &mut u32) -> u8 {
        *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (*seed >> 24) as u8
    }

    const SIZES: &[usize] = &[1, 17, 256, 4096, 70_000];

    #[test]
    fn roundtrip_all_same_byte() {
        for &n in SIZES {
            let data = vec![0xABu8; n];
            let buf = roundtrip(&data);
            if n >= 6 {
                assert!(buf.len() < n + 3, "all-same len {n} did not shrink: {}", buf.len());
            }
        }
    }

    #[test]
    fn roundtrip_two_symbols_5050() {
        for &n in SIZES {
            let data: Vec<u8> = (0..n).map(|i| if i % 2 == 0 { 7u8 } else { 200u8 }).collect();
            roundtrip(&data);
        }
    }

    #[test]
    fn roundtrip_skewed() {
        // Heavily skewed: mostly one byte, a few others — should compress well via huffman.
        for &n in SIZES {
            let mut seed = 12345u32;
            let data: Vec<u8> = (0..n)
                .map(|_| {
                    let r = lcg(&mut seed);
                    if r < 230 {
                        0u8
                    } else if r < 250 {
                        1u8
                    } else {
                        r
                    }
                })
                .collect();
            let buf = roundtrip(&data);
            if n >= 256 {
                assert!(
                    buf.len() < n + 3,
                    "skewed len {n} did not shrink: {}",
                    buf.len()
                );
            }
        }
    }

    #[test]
    fn roundtrip_near_uniform_random() {
        for &n in SIZES {
            let mut seed = 0xDEAD_BEEFu32;
            let data: Vec<u8> = (0..n).map(|_| lcg(&mut seed)).collect();
            roundtrip(&data);
        }
    }

    /// Force the general RLE path (literal runs interleaved with byte runs) and verify the
    /// RLE encoder's output decodes back exactly via `decode_array`/`decode_rle`.
    #[test]
    fn rle_general_roundtrips() {
        // Build run-heavy data: blocks of literals then long runs of a repeated byte.
        let mut data: Vec<u8> = Vec::new();
        let mut seed = 99u32;
        for blk in 0..40u32 {
            // a short literal stretch
            for _ in 0..(3 + (blk % 5)) {
                data.push(lcg(&mut seed));
            }
            // a long run
            let run_byte = (blk * 13) as u8;
            for _ in 0..(20 + blk * 7) {
                data.push(run_byte);
            }
        }
        let arr = super::super::rle_enc::encode_rle(&data).expect("rle should encode run-heavy");
        let mut out = vec![0u8; data.len()];
        let used = decode_array(&arr, &mut out).unwrap();
        assert_eq!(used, arr.len());
        assert_eq!(out, data, "general RLE roundtrip mismatch");
        assert!(
            arr.len() < data.len(),
            "RLE did not shrink run-heavy data: {} vs {}",
            arr.len(),
            data.len()
        );
    }

    /// tANS encode → decode roundtrip across histograms/sizes. Drives `encode_tans_array`
    /// directly (the mode selector may otherwise pick huffman) so the tANS codec is exercised.
    #[test]
    fn tans_roundtrips() {
        let cases: [(&str, fn(usize) -> Vec<u8>); 3] = [
            ("two_5050", |n| {
                (0..n).map(|i| if i % 2 == 0 { 3u8 } else { 9u8 }).collect()
            }),
            ("skewed", |n| {
                let mut seed = 7u32;
                (0..n)
                    .map(|_| {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        let r = (seed >> 24) as u8;
                        if r < 200 {
                            5u8
                        } else if r < 240 {
                            6u8
                        } else {
                            r
                        }
                    })
                    .collect()
            }),
            ("near_uniform", |n| {
                let mut seed = 0x1234_5678u32;
                (0..n)
                    .map(|_| {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        (seed >> 24) as u8
                    })
                    .collect()
            }),
        ];
        for (name, gen) in cases {
            for &n in &[32usize, 64, 256, 4096, 70_000] {
                let data = gen(n);
                if let Some(arr) = super::super::tans_enc::encode_tans_array(&data, Level::Default) {
                    let mut out = vec![0u8; n];
                    let used = decode_array(&arr, &mut out)
                        .unwrap_or_else(|e| panic!("tans decode {name} n={n}: {e:?}"));
                    assert_eq!(used, arr.len(), "tans {name} n={n} consumed");
                    assert_eq!(out, data, "tans {name} n={n} roundtrip mismatch");
                } else {
                    std::eprintln!("tans {name} n={n}: encoder declined");
                }
            }
        }
    }

    /// A focused memset (all-same) decode-direction test: a 1-byte chunk-type-3 payload must
    /// expand to the full output.
    #[test]
    fn rle_memset_decodes() {
        // chunk type 3, long header, dst=1000, csize=1, payload byte 0x5A.
        let arr = super::super::rle_enc::encode_rle(&[0x5Au8; 1000]).unwrap();
        let mut out = vec![0u8; 1000];
        let used = decode_array(&arr, &mut out).unwrap();
        assert_eq!(used, arr.len());
        assert!(out.iter().all(|&b| b == 0x5A));
    }

    /// Encode a single complete array for `data` (used to build recursive containers).
    fn one_array(data: &[u8]) -> Vec<u8> {
        let mut w = ByteWriter::new();
        encode_array(data, &mut w, Level::Default).unwrap();
        w.into_vec()
    }

    /// Recursive mode (chunk type 5, non-multi `src[0] & 0x80 == 0`): a count `n` followed by
    /// `n` concatenated sub-arrays whose decoded sizes sum to the output. We hand-assemble one
    /// from sub-arrays the encoder produces, then decode the whole thing.
    #[test]
    fn recursive_concat_decodes() {
        let parts: [Vec<u8>; 3] = [
            (0..500u32).map(|i| (i % 7) as u8).collect(),
            vec![0xAAu8; 300],
            {
                let mut s = 5u32;
                (0..400)
                    .map(|_| {
                        s = s.wrapping_mul(1103515245).wrapping_add(12345);
                        (s >> 16) as u8
                    })
                    .collect()
            },
        ];
        let total: usize = parts.iter().map(|p| p.len()).sum();
        let mut whole: Vec<u8> = Vec::new();
        for p in &parts {
            whole.extend_from_slice(p);
        }

        // Build the recursive payload: [n][subarray...].
        let mut payload: Vec<u8> = Vec::new();
        payload.push(parts.len() as u8); // n=3, high bit clear => non-multi path
        for p in &parts {
            payload.extend_from_slice(&one_array(p));
        }
        // Wrap as chunk type 5 (long header).
        let csize = payload.len();
        let d = (total - 1) as u32;
        let mut arr = Vec::new();
        arr.push(((5u32 << 4) as u8) | ((d >> 14) as u8));
        let word = (d << 18) | csize as u32;
        arr.push((word >> 24) as u8);
        arr.push((word >> 16) as u8);
        arr.push((word >> 8) as u8);
        arr.push(word as u8);
        arr.extend_from_slice(&payload);

        let mut out = vec![0u8; total];
        let used = decode_array(&arr, &mut out).unwrap();
        assert_eq!(used, arr.len());
        assert_eq!(out, whole, "recursive concat roundtrip mismatch");
    }

    /// Hand-traced tiny huffman: two symbols with code length 1 each. Build the array via the
    /// encoder and confirm a focused decode yields the exact symbols.
    #[test]
    fn huffman_two_symbol_exact() {
        let data: Vec<u8> = b"ABABABABABABABABABABABABABABABAB".to_vec(); // 32 bytes, A/B
        let arr = super::super::huff_enc::encode_huff_array(&data).expect("huff applies");
        assert_eq!((arr[0] >> 4) & 7, 2, "should be a huffman chunk");
        let mut out = vec![0u8; data.len()];
        decode_array(&arr, &mut out).unwrap();
        assert_eq!(out, data);
    }
}
