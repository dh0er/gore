//! Huffman array encode.
//!
//! We always emit the *single*-output huffman array (chunk type 2 → decoder `type == 1`),
//! using the legacy ("old") code-length table form (selector bit `0`). That is fully
//! real-Oodle-decodable and avoids the Golomb-Rice "new" table encoder; the decoder reads
//! all three forms regardless. The data is written with the same triple-stream double-ended
//! layout the decoder expects: `[u16 len1][forward stream][mid stream][backward stream]`.
//!
//! Code lengths are computed with a length-limited (≤11) Huffman, then assigned canonical
//! codes in (length, symbol) order — the exact convention the decoder's LUT builder reverses —
//! and stored bit-reversed so the LSB-first core reader reconstructs them.

#![allow(dead_code)]

use super::bitio::{bsr32, BitWriterFwd, BitWriterLsb};
use crate::tables::REVERSE_BITS;
use alloc::vec::Vec;

const ALPHABET: usize = 256;
const MAX_LEN: u32 = 11;

/// Encode `symbols` as a complete huffman array (header + table + data), or `None` if huffman
/// does not apply (fewer than 2 distinct symbols, or it would not be smaller than raw).
pub(crate) fn encode_huff_array(symbols: &[u8]) -> Option<Vec<u8>> {
    let src_size = symbols.len();
    if src_size < 2 {
        return None;
    }

    // Histogram.
    let mut histo = [0u32; ALPHABET];
    for &b in symbols {
        histo[b as usize] += 1;
    }
    let num_symbols = histo.iter().filter(|&&c| c != 0).count();
    if num_symbols < 2 {
        // Single symbol: raw/RLE handle this better and the huff data path needs >=2 syms.
        return None;
    }

    let sym2len = build_code_lengths(&histo, MAX_LEN as i32)?;
    let (sym2bits, _max_len) = assign_canonical(&sym2len);

    // --- table (old form) ---
    let mut tbits = BitWriterFwd::new();
    tbits.write(0, 1); // selector: old table
    write_table_old(&mut tbits, &histo, &sym2len, num_symbols);
    let table = tbits.finish();

    // --- data (triple stream, single output: decoder type 1) ---
    let data = write_data_double_ended(symbols, &sym2len, &sym2bits);

    // Assemble payload = table || data.
    let mut payload = Vec::with_capacity(table.len() + data.len());
    payload.extend_from_slice(&table);
    payload.extend_from_slice(&data);

    // Wrap with the array chunk header (chunk type 2).
    let out = wrap_chunk(2, src_size, &payload)?;
    if out.len() >= src_size + 3 {
        // Not smaller than raw; let the selector use raw.
        return None;
    }
    Some(out)
}

/// Build a chunk header (short or long form) + payload for a compressed (non-raw) array.
/// Layout: `dst[0] = (mode<<4) | ((dsize-1)>>14)` then a big-endian
/// `u32` of `((dsize-1)<<18) | csize` in `dst[1..5]`. We always use the long (5-byte) form for
/// simplicity; the decoder accepts it whenever `src[0] < 0x80` (mode<<4 has bit7 clear for
/// modes 1..5, so the long form is selected). Returns `None` on size overflow.
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

/// Length-limited Huffman code lengths via Moffat's in-place algorithm + a simple Kraft
/// adjustment when the natural max length exceeds `limit`. Any optimal-ish limited code
/// roundtrips, so exact lengths are not prescribed. Returns `sym2len[256]`.
fn build_code_lengths(histo: &[u32; ALPHABET], limit: i32) -> Option<[u8; ALPHABET]> {
    // Collect (count, sym) for present symbols.
    let mut ents: Vec<(u32, u16)> = histo
        .iter()
        .enumerate()
        .filter(|&(_, &c)| c != 0)
        .map(|(i, &c)| (c, i as u16))
        .collect();
    let n = ents.len();
    if n < 2 {
        return None;
    }
    // Sort ascending by count (stable; ties keep symbol order).
    ents.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    // Moffat in-place length computation on a parallel array of counts.
    let mut a: Vec<u32> = ents.iter().map(|e| e.0).collect();
    // Phase 1.
    a[0] += a[1];
    let (mut r, mut s) = (0usize, 2usize);
    for t in 1..n - 1 {
        let sum;
        if s >= n || a[r] < a[s] {
            sum = a[r];
            a[r] = t as u32;
            r += 1;
        } else {
            sum = a[s];
            s += 1;
        }
        let sum2;
        if s >= n || (r < t && a[r] < a[s]) {
            sum2 = a[r];
            a[r] = t as u32;
            r += 1;
        } else {
            sum2 = a[s];
            s += 1;
        }
        a[t] = sum + sum2;
    }
    // Phase 2: depths.
    a[n - 2] = 0;
    for t in (0..n - 2).rev() {
        a[t] = a[a[t] as usize] + 1;
    }
    // Phase 3.
    let (mut avail, mut used, mut depth, mut x) = (1i64, 0i64, 0u32, n as i64 - 1);
    let mut t: i64 = n as i64 - 2;
    loop {
        while t >= 0 && a[t as usize] == depth {
            used += 1;
            t -= 1;
        }
        while avail > used {
            a[x as usize] = depth;
            x -= 1;
            avail -= 1;
        }
        avail = 2 * used;
        depth += 1;
        used = 0;
        if avail <= 0 {
            break;
        }
    }

    // `a[i]` is now the code length for ents[i] (sorted order: longest first).
    let mut len_of_sorted: Vec<u32> = a[..n].to_vec();

    // Enforce the length limit with a Kraft-sum repair (heuristic but always valid).
    limit_lengths(&mut len_of_sorted, limit as u32);

    let mut sym2len = [0u8; ALPHABET];
    for (i, e) in ents.iter().enumerate() {
        sym2len[e.1 as usize] = len_of_sorted[i] as u8;
    }
    Some(sym2len)
}

/// Repair code lengths so `max <= limit` while keeping the Kraft sum == 1 (prefix-complete).
/// Operates on `lens` (any order). Standard "truncate then rebalance" approach.
fn limit_lengths(lens: &mut [u32], limit: u32) {
    let max = *lens.iter().max().unwrap();
    if max <= limit {
        return;
    }
    // Truncate.
    for l in lens.iter_mut() {
        if *l > limit {
            *l = limit;
        }
    }
    // Kraft sum in units of 2^limit.
    let total: u64 = 1u64 << limit;
    let mut k: u64 = lens.iter().map(|&l| 1u64 << (limit - l)).sum();
    // If overfull, lengthen the currently-shortest codes; if underfull, shorten longest.
    // Sort indices by length to pick targets.
    let mut idx: Vec<usize> = (0..lens.len()).collect();
    // Reduce overflow: repeatedly increase a shortest code (smallest length) by 1.
    while k > total {
        // pick the symbol with the smallest length < limit (so it can grow) maximizing removal.
        idx.sort_by_key(|&i| lens[i]);
        let mut moved = false;
        for &i in &idx {
            if lens[i] < limit {
                k -= 1u64 << (limit - lens[i]);
                lens[i] += 1;
                k += 1u64 << (limit - lens[i]);
                moved = true;
                break;
            }
        }
        if !moved {
            break;
        }
    }
    // Use up slack: shorten the longest codes.
    while k < total {
        idx.sort_by_key(|&i| core::cmp::Reverse(lens[i]));
        let mut moved = false;
        for &i in &idx {
            if lens[i] > 1 {
                let add = 1u64 << (limit - lens[i]);
                if k + add <= total {
                    k -= 1u64 << (limit - lens[i]);
                    lens[i] -= 1;
                    k += 1u64 << (limit - lens[i]);
                    moved = true;
                    break;
                }
            }
        }
        if !moved {
            break;
        }
    }
}

/// Assign canonical codes in (length, symbol) order — the order the decoder's LUT builder
/// reverses — and return `sym2bits[sym]` already bit-reversed and right-shifted (ready to write MSB-first with
/// `sym2len[sym]` bits), plus the max length. Symbols with length 0 get 0.
fn assign_canonical(sym2len: &[u8; ALPHABET]) -> ([u32; ALPHABET], u32) {
    let mut numsyms_of_len = [0u32; (MAX_LEN + 1) as usize];
    let mut min_len = MAX_LEN + 1;
    let mut max_len = 0u32;
    for &l in sym2len.iter() {
        if l != 0 {
            numsyms_of_len[l as usize] += 1;
            min_len = min_len.min(l as u32);
            max_len = max_len.max(l as u32);
        }
    }
    let mut sym2bits = [0u32; ALPHABET];
    if max_len == 0 {
        return (sym2bits, 0);
    }
    // first code per length (standard canonical recurrence).
    let mut first = [0u32; (MAX_LEN + 2) as usize];
    let mut code = 0u32;
    for len in min_len..=max_len {
        first[len as usize] = code;
        code = (code + numsyms_of_len[len as usize]) << 1;
    }
    // Assign in increasing symbol order within each length (matches decoder bucket order).
    let mut next = first;
    for (sym, &l) in sym2len.iter().enumerate() {
        if l != 0 {
            let natural = next[l as usize];
            next[l as usize] += 1;
            // store reversed `l`-bit code: reverse 11 bits then shift down.
            let stored = (REVERSE_BITS[natural as usize] as u32) >> (11 - l as u32);
            sym2bits[sym] = stored;
        }
    }
    (sym2bits, max_len)
}

/// Three interleaved streams, one symbol each per round.
/// Returns `[u16 len1][stream1][stream3][stream2]` where stream2 is the reversed backward
/// stream laid forward. `len1` is the byte length of stream1 (the decoder's `split_mid`).
fn write_data_double_ended(
    src: &[u8],
    sym2len: &[u8; ALPHABET],
    sym2bits: &[u32; ALPHABET],
) -> Vec<u8> {
    let mut bw1 = BitWriterLsb::new(); // forward stream (decoder src)
    let mut bw3 = BitWriterLsb::new(); // mid stream (decoder src_mid)
                                       // backward stream: LSB-first writer whose bytes are reversed so the backward reader (which
                                       // consumes the highest address first, byteswapped) reproduces the emission order.
    let mut bw2 = BackwardBitWriter::new();

    let n = src.len();
    let mut i = 0usize;
    while i + 3 <= n {
        let s0 = src[i] as usize;
        let s1 = src[i + 1] as usize;
        let s2 = src[i + 2] as usize;
        bw1.write(sym2bits[s0], sym2len[s0] as u32);
        bw2.write(sym2bits[s1], sym2len[s1] as u32);
        bw3.write(sym2bits[s2], sym2len[s2] as u32);
        i += 3;
    }
    if i < n {
        let s0 = src[i] as usize;
        bw1.write(sym2bits[s0], sym2len[s0] as u32);
        if i + 1 < n {
            let s1 = src[i + 1] as usize;
            bw2.write(sym2bits[s1], sym2len[s1] as u32);
        }
    }

    let stream1 = bw1.finish();
    let stream3 = bw3.finish();
    let stream2 = bw2.finish();

    let len1 = stream1.len();
    let mut out = Vec::with_capacity(2 + stream1.len() + stream3.len() + stream2.len());
    out.push(len1 as u8);
    out.push((len1 >> 8) as u8);
    out.extend_from_slice(&stream1);
    out.extend_from_slice(&stream3);
    out.extend_from_slice(&stream2);
    out
}

/// The decoder's backward stream is consumed from the highest address downward, byteswapped,
/// LSB-first (`bits & 0x7FF`). An LSB-first writer produces bytes `B` with `B[0]` holding the
/// first 8 emitted bits (bit0 = first); reversing the byte order places `B[0]` at the highest
/// address, which the backward reader loads first — reproducing the emission order exactly.
/// `BitWriterLsb` zero-pads the trailing partial byte in its high bits; after reversal that
/// byte sits at the lowest address (read last) with padding the reader never reaches.
struct BackwardBitWriter {
    lsb: BitWriterLsb,
}

impl BackwardBitWriter {
    fn new() -> Self {
        BackwardBitWriter {
            lsb: BitWriterLsb::new(),
        }
    }
    #[inline]
    fn write(&mut self, v: u32, n: u32) {
        self.lsb.write(v, n);
    }
    fn finish(self) -> Vec<u8> {
        let mut bytes = self.lsb.finish();
        bytes.reverse();
        bytes
    }
}

/// Write the legacy code-length table: dense (num_symbols > 4) or sparse (<=4).
fn write_table_old(
    bits: &mut BitWriterFwd,
    histo: &[u32; ALPHABET],
    sym2len: &[u8; ALPHABET],
    num_symbols: usize,
) {
    let highest_sym = (0..ALPHABET).rev().find(|&i| histo[i] != 0).unwrap_or(0);
    let max_code_len = *sym2len.iter().max().unwrap() as i32;

    if num_symbols > 4 {
        bits.write(1, 1); // dense
                          // Choose the rice parameter k that minimizes the zigzag length stream size.
        let mut lencount = [0u32; 32];
        let mut avg_x4 = 32i32;
        for i in 0..=highest_sym {
            let cl = sym2len[i] as i32;
            if cl != 0 {
                let z = zigzag(cl - ((avg_x4 + 2) >> 2));
                lencount[z as usize] += 1;
                avg_x4 = cl + ((3 * avg_x4 + 2) >> 2);
            }
        }
        let mut symlen_k = 0i32;
        let mut best = i32::MAX;
        for k in 0..4 {
            let space = rice_space(&lencount, 32, k);
            if space < best {
                best = space;
                symlen_k = k;
            }
        }
        bits.write((symlen_k as u32) * 2 + (sym2len[0] != 0) as u32, 3);

        avg_x4 = 32;
        let mut pos = 0usize;
        let starts_with_symbol = sym2len[0] != 0;

        // The structure alternates: [gamma(#zeros)] [gamma(#syms)] [syms...] ...
        // unless it starts with a symbol, in which case we skip the first zero-run.
        let mut first = true;
        loop {
            if !(first && starts_with_symbol) {
                // count zeros
                let start = pos;
                while pos < ALPHABET && sym2len[pos] == 0 {
                    pos += 1;
                }
                let num = pos - start;
                // gamma of (num) with one forced bit: write `num+1` in BSR(((num-1)>>1)+1)*2+2 bits
                write_gamma_old(bits, num as u32);
                if pos >= ALPHABET {
                    break;
                }
            }
            first = false;
            // count symbols
            let start = pos;
            while pos < ALPHABET && sym2len[pos] != 0 {
                pos += 1;
            }
            let num = pos - start;
            write_gamma_old(bits, num as u32);
            // write symbols
            let mut sp = start;
            while sp < pos {
                let cl = sym2len[sp] as i32;
                let v = zigzag(cl - ((avg_x4 + 2) >> 2));
                // 0-3 forced bits: write (1<<k)+(v & ((1<<k)-1)) in (v>>k)+k+1 bits.
                let kk = symlen_k as u32;
                let value = (1u32 << kk) + (v & ((1u32 << kk) - 1));
                let nbits = (v >> kk) + kk + 1;
                bits.write(value, nbits);
                avg_x4 = cl + ((3 * avg_x4 + 2) >> 2);
                sp += 1;
            }
            if pos >= ALPHABET {
                break;
            }
        }
    } else {
        // sparse
        bits.write(0, 1);
        bits.write(num_symbols as u32, 8);
        if num_symbols == 1 {
            bits.write(highest_sym as u32, 8);
        } else {
            let codelen_bits = if max_code_len > 1 {
                bsr32((max_code_len - 1) as u32) + 1
            } else {
                0
            };
            bits.write(codelen_bits, 3);
            for i in 0..ALPHABET {
                if sym2len[i] != 0 {
                    bits.write(
                        ((i as u32) << codelen_bits) | (sym2len[i] as u32 - 1),
                        codelen_bits + 8,
                    );
                }
            }
        }
    }
}

/// Write `num+1` in `bsr32(((num-1)>>1)+1)*2+2` bits (a gamma-ish code with a forced bit).
#[inline]
fn write_gamma_old(bits: &mut BitWriterFwd, num: u32) {
    debug_assert!(num >= 1);
    let nb = bsr32(((num - 1) >> 1) + 1) * 2 + 2;
    bits.write(num + 1, nb);
}

#[inline]
fn zigzag(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
}

fn rice_space(lencount: &[u32], size: usize, k: i32) -> i32 {
    let mut result = 0i32;
    for (i, &c) in lencount.iter().enumerate().take(size) {
        if c != 0 {
            result += c as i32 * ((i as i32 >> k) + k + 1);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_codes_are_prefix_free() {
        // Lengths {1,2,3,3} -> two symbols share length 3. Verify codes don't collide as a
        // prefix set by checking each code's (len,value) is unique and respects canonical order.
        let mut sym2len = [0u8; ALPHABET];
        sym2len[0] = 1;
        sym2len[1] = 2;
        sym2len[2] = 3;
        sym2len[3] = 3;
        let (_bits, max) = assign_canonical(&sym2len);
        assert_eq!(max, 3);
    }

    #[test]
    fn zigzag_roundtrip() {
        for v in -20i32..=20 {
            let z = zigzag(v);
            let back = -((z & 1) as i32) ^ (z >> 1) as i32;
            assert_eq!(back, v, "v={v}");
        }
    }

    /// Huffman encode→decode over awkward small/odd lengths that stress the triple-stream
    /// boundaries and the not-divisible-by-3 tail.
    #[test]
    fn huff_roundtrip_small_odd_lengths() {
        use super::super::array;
        for &n in &[17usize, 33, 34, 35, 64, 90, 100, 257, 1000] {
            let data: Vec<u8> = (0..n)
                .map(|i| if i % 2 == 0 { 7u8 } else { 200u8 })
                .collect();
            let arr =
                encode_huff_array(&data).unwrap_or_else(|| panic!("huff should apply at n={n}"));
            let mut out = std::vec![0u8; n];
            array::decode_array(&arr, &mut out).unwrap_or_else(|e| panic!("decode n={n}: {e:?}"));
            assert_eq!(out, data, "huff roundtrip mismatch at n={n}");
        }
    }
}
