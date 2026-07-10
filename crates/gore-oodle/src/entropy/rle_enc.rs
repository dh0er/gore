//! Advanced-RLE array encode (chunk type 3), raw-command-buffer path only (no optional
//! entropy-recompression of the literal/command buffers). Also handles the all-same "memset"
//! special case the decoder reads from a 1-byte payload.
//!
//! Layout produced (chunk type 3, header byte 0): `[0x00][literals…][commands…]`. The decoder
//! walks literals forward from offset 1 and commands backward from the end; they meet in the
//! middle. Commands are therefore emitted back-to-front into a high cursor and spliced after
//! the literals at the end.

#![allow(dead_code)]

use alloc::vec::Vec;

/// Try to RLE-encode `src`. Returns a complete array (with chunk header) on success.
pub(crate) fn encode_rle(src: &[u8]) -> Option<Vec<u8>> {
    let n = src.len();
    if n < 6 {
        return None;
    }

    // All-same → memset array: chunk type 3 with a single payload byte.
    if src.iter().all(|&b| b == src[0]) {
        return wrap_chunk(3, n, &[src[0]]);
    }

    let payload = build_rle_payload(src)?;
    // payload must be smaller than raw (size+3) to be worth it; also must be < dst_size for the
    // chunk header to be valid.
    if payload.len() >= n {
        return None;
    }
    wrap_chunk(3, n, &payload)
}

/// Build the raw RLE payload `[0x00][literals][commands]`, or `None` if it cannot be expressed
/// compactly, via a scan + command emission.
fn build_rle_payload(src: &[u8]) -> Option<Vec<u8>> {
    let n = src.len();
    let src_end = n;
    let safe_end = if n >= 18 { n - 18 } else { 0 };

    // Forward literal buffer and backward command buffer. We size both generously and splice.
    let mut lits: Vec<u8> = Vec::with_capacity(n);
    let mut cmds_rev: Vec<u8> = Vec::with_capacity(n); // commands in REVERSE emission order

    // Commands are written to a descending pointer; pushing to `cmds_rev` in the same order
    // yields the bytes high→low, i.e. exactly what `[..].reverse()` turns into the forward
    // command block. We collect "decode-first-last" then reverse at the end.
    let mut start = 0usize; // start of the pending literal run
    let mut src_pos = 0usize;
    let mut last_rle_byte: u8 = 0;

    while src_pos < safe_end {
        let first_rle = match scan_for_rle3(src, src_pos, safe_end) {
            Some(p) => p,
            None => break,
        };
        if first_rle >= safe_end {
            break;
        }
        src_pos = rle_length(src, first_rle, src_end);
        let mut lrl = first_rle - start;
        let mut rlel = src_pos - first_rle;

        if src[first_rle] != last_rle_byte {
            if rlel < 8 {
                // Not worth switching the sticky byte for a short run. `src` stays at the
                // run end (already advanced) so the run is absorbed into the next literal;
                // `start` is unchanged.
                continue;
            }
            last_rle_byte = src[first_rle];
            cmds_rev.push(1);
            lits.push(last_rle_byte);
        }
        lits.extend_from_slice(&src[start..start + lrl]);
        start = src_pos;

        if (lrl <= 30 && rlel <= 15) || (lrl <= 15 && rlel <= 30) {
            write_short_lrl_rle(&mut cmds_rev, lrl, rlel);
            continue;
        }

        // Very long literal lengths: emit "long literal" commands until lrl < 0x40.
        if lrl >= 0x40 {
            if lrl < 0x4f {
                cmds_rev.push(0);
                lrl -= 15;
            }
            while lrl >= 0x40 {
                let nn = core::cmp::min(0x700usize, lrl >> 6);
                cmds_rev.push((((nn - 1) >> 8) + 2) as u8);
                cmds_rev.push((nn - 1) as u8);
                lrl -= nn << 6;
            }
        }

        let mut rle_big = rlel >> 7;
        rlel &= 0x7f;

        if rlel >= 3 && ((lrl <= 30 && rlel <= 15) || (lrl <= 15 && rlel <= 30)) {
            write_short_lrl_rle(&mut cmds_rev, lrl, rlel);
        } else if (lrl | rlel) != 0 {
            let nn = lrl | (rlel << 6);
            cmds_rev.push(((nn >> 8) + 16) as u8);
            cmds_rev.push(nn as u8);
        }

        while rle_big != 0 {
            let nn = core::cmp::min(0x700usize, rle_big);
            cmds_rev.push((((nn - 1) >> 8) + 9) as u8);
            cmds_rev.push((nn - 1) as u8);
            rle_big -= nn;
        }
    }

    // Trailing literal run.
    let mut lrl = src_end - start;
    if lrl != 0 {
        lits.extend_from_slice(&src[start..start + lrl]);
        if lrl >= 0x40 {
            if lrl < 0x4f {
                cmds_rev.push(0);
                lrl -= 15;
            }
            while lrl >= 0x40 {
                let nn = core::cmp::min(0x700usize, lrl >> 6);
                cmds_rev.push((((nn - 1) >> 8) + 2) as u8);
                cmds_rev.push((nn - 1) as u8);
                lrl -= nn << 6;
            }
        }
        if lrl != 0 {
            cmds_rev.push(((lrl >> 8) + 16) as u8);
            cmds_rev.push(lrl as u8);
        }
    }

    // Assemble: [0x00][literals][commands forward].
    // `cmds_rev` holds commands in the order they were written descending (first-written = the
    // one decoded last). The decoder reads from the end backward, so the forward command block
    // is `cmds_rev` reversed.
    let mut cmds = cmds_rev;
    cmds.reverse();

    let mut out = Vec::with_capacity(1 + lits.len() + cmds.len());
    out.push(0u8);
    out.extend_from_slice(&lits);
    out.extend_from_slice(&cmds);
    Some(out)
}

/// Find the next position with three equal bytes in a row.
fn scan_for_rle3(src: &[u8], mut pos: usize, safe_end: usize) -> Option<usize> {
    while pos < safe_end {
        if pos + 2 < src.len() && src[pos] == src[pos + 1] && src[pos] == src[pos + 2] {
            return Some(pos);
        }
        pos += 1;
    }
    Some(safe_end)
}

/// Extend a run of equal bytes from `first_rle`.
fn rle_length(src: &[u8], first_rle: usize, src_end: usize) -> usize {
    let v = src[first_rle];
    let mut p = first_rle;
    while p < src_end && src[p] == v {
        p += 1;
    }
    p
}

/// Emit the compact single/double-byte literal+rle command(s).
fn write_short_lrl_rle(cmds_rev: &mut Vec<u8>, lrl: usize, rlel: usize) {
    if lrl > 15 {
        // two bytes: [0][ 16*rlel | (15-(lrl-15)) ]
        cmds_rev.push(0);
        let lrl2 = lrl - 15;
        cmds_rev.push((16 * rlel | (15 - lrl2)) as u8);
    } else if rlel > 15 {
        cmds_rev.push((16 * (rlel >> 1) | (15 - lrl)) as u8);
        cmds_rev.push((16 * (rlel - (rlel >> 1)) | 0xF) as u8);
    } else {
        cmds_rev.push((16 * rlel | (15 - lrl)) as u8);
    }
}

/// Build a chunk header (long form) + payload. `None` on
/// overflow. (Shared shape with the huffman encoder; kept local to avoid cross-module deps.)
fn wrap_chunk(mode: u32, dsize: usize, payload: &[u8]) -> Option<Vec<u8>> {
    let csize = payload.len();
    if dsize == 0 || dsize > 0x3FFFF || csize > 0x3FFFF || csize >= dsize {
        return None;
    }
    let mut out = Vec::with_capacity(5 + csize);
    let d = (dsize - 1) as u32;
    out.push(((mode << 4) as u8) | ((d >> 14) as u8));
    let word = (d << 18) | csize as u32;
    out.push((word >> 24) as u8);
    out.push((word >> 16) as u8);
    out.push((word >> 8) as u8);
    out.push(word as u8);
    out.extend_from_slice(payload);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memset_array_is_tiny() {
        let arr = encode_rle(&[0x42u8; 1000]).unwrap();
        // 5-byte header + 1 payload byte.
        assert_eq!(arr.len(), 6);
        assert_eq!((arr[0] >> 4) & 7, 3);
    }
}
