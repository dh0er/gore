//! tANS array encode.
//!
//! Produces a chunk-type-1 array. The table is written MSB-first (read back by the spine
//! `BitReader`); the five interleaved arithmetic streams are written so the decoder
//! (`super::tans_dec`) reconstructs them. The streams use the same forward-LSB / backward-
//! byteswapped layout as the huffman data path, via `BitWriterLsb` plus a byte reversal.
//!
//! The encoder is validated by `encode → decode == identity` roundtrip tests in `array.rs`
//! and here; that is what pins the (very fiddly) stream bit-order to the decoder.

#![allow(dead_code)]

use super::bitio::{bsr32, BitWriterFwd};
use crate::Level;
use alloc::vec;
use alloc::vec::Vec;

const ALPHABET: usize = 256;

/// Encode `symbols` as a complete tANS array, or `None` if tANS does not apply / help.
pub(crate) fn encode_tans_array(symbols: &[u8], _level: Level) -> Option<Vec<u8>> {
    let src_size = symbols.len();
    if src_size < 32 {
        return None;
    }

    // Histogram excludes the trailing 5 bytes (which become the initial states).
    let mut histo = [0u32; ALPHABET];
    for &b in symbols {
        histo[b as usize] += 1;
    }
    let tail = &symbols[src_size - 5..];
    for &b in tail {
        histo[b as usize] -= 1;
    }

    let l_bits = ilog2round((src_size - 5) as u32)
        .saturating_sub(2)
        .clamp(8, 11);
    let l = 1u32 << l_bits;

    let mut weights_size = ALPHABET;
    while weights_size > 0 && histo[weights_size - 1] == 0 {
        weights_size -= 1;
    }

    let mut weights = [0u32; ALPHABET];
    let used_symbols = normalize_counts(&mut weights, l, &histo, (src_size - 5) as u32, weights_size);
    if used_symbols <= 1 {
        return None;
    }
    // Every symbol present in the encoded body (src[0..len-5]) must receive weight >= 1, else
    // it cannot be tANS-encoded. Near-uniform inputs can normalize a rare symbol to 0; in that
    // case decline tANS and let the mode selector fall back to huffman.
    for i in 0..weights_size {
        if histo[i] != 0 && weights[i] == 0 {
            return None;
        }
    }

    // --- table ---
    let mut tbits = BitWriterFwd::new();
    tbits.write(l_bits - 8, 3);
    encode_table(&mut tbits, l_bits, &weights, weights_size, used_symbols);
    let table = tbits.finish();

    // --- transition table ---
    let te = init_table(&weights, weights_size, l_bits);

    // --- data ---
    let data = encode_bytes(&te, symbols, l_bits)?;

    let mut payload = Vec::with_capacity(table.len() + data.len());
    payload.extend_from_slice(&table);
    payload.extend_from_slice(&data);

    let out = wrap_chunk(1, src_size, &payload)?;
    if out.len() >= src_size + 3 {
        return None;
    }
    Some(out)
}

/// Rounded integer log2 of `v`.
fn ilog2round(v: u32) -> u32 {
    let f = v as f32;
    let u = f.to_bits();
    ((u.wrapping_add(0x257D86) >> 23) as i32 - 127).max(0) as u32
}

/// Scale histogram to sum exactly `L`, heap-adjusting the residue.
fn normalize_counts(
    lookup: &mut [u32; ALPHABET],
    l: u32,
    histo: &[u32; ALPHABET],
    histo_sum: u32,
    num_syms: usize,
) -> usize {
    let mut syms_used = 0usize;
    let multiplier = l as f64 / histo_sum as f64;
    let mut weight_sum = 0u32;
    for i in 0..num_syms {
        let h = histo[i];
        let mut u = 0u32;
        if h != 0 {
            u = double_to_uint_round_pow2(h as f64 * multiplier);
            weight_sum += u;
            syms_used += 1;
        }
        lookup[i] = u;
    }
    for v in lookup.iter_mut().take(ALPHABET).skip(num_syms) {
        *v = 0;
    }
    if weight_sum == l {
        return syms_used;
    }

    // Heap of (index, score). diff>0: need to add weight; diff<0: remove.
    let mut heap: Vec<(f32, usize)> = Vec::new();
    let diff = l as i64 - weight_sum as i64;
    if diff < 0 {
        for i in 0..num_syms {
            if lookup[i] > 1 {
                heap.push((histo[i] as f32 * log_factor_down(lookup[i]), i));
            }
        }
    } else {
        for i in 0..num_syms {
            if histo[i] != 0 {
                heap.push((histo[i] as f32 * log_factor_up(lookup[i]), i));
            }
        }
    }
    // Min-heap by score: pop the *smallest* score first.
    make_min_heap(&mut heap);

    let mut d = diff;
    if d < 0 {
        while d != 0 {
            let (_s, index) = pop_min_heap(&mut heap);
            lookup[index] -= 1;
            if lookup[index] > 1 {
                push_min_heap(&mut heap, (histo[index] as f32 * log_factor_down(lookup[index]), index));
            }
            d += 1;
        }
    } else {
        while d != 0 {
            let (_s, index) = pop_min_heap(&mut heap);
            lookup[index] += 1;
            push_min_heap(&mut heap, (histo[index] as f32 * log_factor_up(lookup[index]), index));
            d -= 1;
        }
    }
    syms_used
}

fn double_to_uint_round_pow2(v: f64) -> u32 {
    let u = v as u32;
    u + if v * v > (u as f64) * (u as f64 + 1.0) { 1 } else { 0 }
}

fn log_factor_up(value: u32) -> f32 {
    const T: [f32; 32] = [
        0.000000, 0.693147, 0.405465, 0.287682, 0.223144, 0.182322, 0.154151, 0.133531, 0.117783,
        0.105361, 0.095310, 0.087011, 0.080043, 0.074108, 0.068993, 0.064539, 0.060625, 0.057158,
        0.054067, 0.051293, 0.048790, 0.046520, 0.044452, 0.042560, 0.040822, 0.039221, 0.037740,
        0.036368, 0.035091, 0.033902, 0.032790, 0.031749,
    ];
    if value >= 32 {
        (1.0 / value as f32) - (1.0 / value as f32) * (1.0 / value as f32) * 0.5
    } else {
        T[value as usize]
    }
}

fn log_factor_down(value: u32) -> f32 {
    const T: [f32; 32] = [
        0.000000, 0.000000, -0.693147, -0.405465, -0.287682, -0.223144, -0.182322, -0.154151,
        -0.133531, -0.117783, -0.105361, -0.095310, -0.087011, -0.080043, -0.074108, -0.068993,
        -0.064539, -0.060625, -0.057158, -0.054067, -0.051293, -0.048790, -0.046520, -0.044452,
        -0.042560, -0.040822, -0.039221, -0.037740, -0.036368, -0.035091, -0.033902, -0.032790,
    ];
    if value >= 32 {
        -(1.0 / value as f32) - (1.0 / value as f32) * (1.0 / value as f32) * 0.5
    } else {
        T[value as usize]
    }
}

// --- a tiny binary min-heap with pop-smallest behavior ---
fn make_min_heap(h: &mut [(f32, usize)]) {
    let n = h.len();
    if n < 2 {
        return;
    }
    for start in (0..n / 2).rev() {
        sift_down(h, start, n);
    }
}
fn sift_down(h: &mut [(f32, usize)], mut root: usize, n: usize) {
    loop {
        let mut smallest = root;
        let l = 2 * root + 1;
        let r = 2 * root + 2;
        if l < n && h[l].0 < h[smallest].0 {
            smallest = l;
        }
        if r < n && h[r].0 < h[smallest].0 {
            smallest = r;
        }
        if smallest == root {
            break;
        }
        h.swap(root, smallest);
        root = smallest;
    }
}
fn pop_min_heap(h: &mut Vec<(f32, usize)>) -> (f32, usize) {
    let n = h.len();
    h.swap(0, n - 1);
    let top = h.pop().unwrap();
    let m = h.len();
    if m > 0 {
        sift_down(h, 0, m);
    }
    top
}
fn push_min_heap(h: &mut Vec<(f32, usize)>, v: (f32, usize)) {
    h.push(v);
    let mut i = h.len() - 1;
    while i > 0 {
        let parent = (i - 1) / 2;
        if h[i].0 < h[parent].0 {
            h.swap(i, parent);
            i = parent;
        } else {
            break;
        }
    }
}

/// Encode the normalized-count table: the inverse of the decoder's table reader.
fn encode_table(
    bits: &mut BitWriterFwd,
    l_bits: u32,
    lookup: &[u32; ALPHABET],
    histo_size: usize,
    used_symbols: usize,
) {
    if used_symbols > 7 {
        bits.write(1, 1);

        let mut arr_z = [0u32; 128];
        let mut ranges: Vec<i32> = Vec::new();
        let mut arr_x: Vec<i32> = Vec::new();
        let mut arr_y: Vec<i32> = Vec::new();

        let mut pos = 0usize;
        while pos < histo_size && lookup[pos] == 0 {
            pos += 1;
        }
        ranges.push(pos as i32);

        let mut average = 6i32;
        let mut used_syms = 0i32;
        while pos < histo_size {
            let pos_start = pos;
            while pos < histo_size && lookup[pos] != 0 {
                let v = lookup[pos] as i32 - 1;
                let average_div4 = average >> 2;
                let limit = 2 * average_div4;
                let u = if v > limit {
                    v
                } else {
                    (2 * (v - average_div4)) ^ ((v - average_div4) >> 31)
                };
                arr_x.push(u);
                if u >= 0x80 {
                    arr_y.push(u);
                } else {
                    arr_z[u as usize] += 1;
                }
                let limit2 = if v < limit { v } else { limit };
                pos += 1;
                used_syms += 1;
                average += limit2 - average_div4;
            }
            ranges.push((pos - pos_start) as i32);

            let pos_start = pos;
            while pos < histo_size && lookup[pos] == 0 {
                pos += 1;
            }
            ranges.push((pos - pos_start) as i32);
        }
        // Final range gets the trailing gap: `ranges[last] += 256 - pos`.
        let last = ranges.len() - 1;
        ranges[last] += 256 - pos as i32;

        // Choose Q.
        let mut best_score = i32::MAX;
        let mut q = 0i32;
        for tq in 0..8 {
            let mut score = bits_for_rice(&arr_z, 128, tq);
            for &y in &arr_y {
                score += tq + 2 * bsr32(((y >> tq) + 1) as u32) as i32 + 1;
            }
            if score < best_score {
                best_score = score;
                q = tq;
            }
        }

        let (sr_rice, sr_bits, sr_bitcount, num_symrange) = encode_sym_range(used_syms, &ranges);
        bits.write(((used_syms - 1) as u32) + ((q as u32) << 8), 11);

        write_num_sym_range(bits, num_symrange, used_syms);

        // arr_w / arr_x low bits.
        let mut arr_w = vec![0i32; arr_x.len()];
        for i in 0..arr_x.len() {
            let x = arr_x[i] + (1 << q);
            let nb = bsr32((x >> q) as u32) as i32;
            arr_w[i] = nb;
            arr_x[i] = x & ((1 << (q + nb)) - 1);
        }

        write_many_rice(bits, &arr_w);
        write_many_rice_u8(bits, &sr_rice[..num_symrange]);
        write_sym_range_low(bits, &sr_bits[..num_symrange], &sr_bitcount[..num_symrange]);

        for i in 0..arr_x.len() {
            let nb = q + arr_w[i];
            if nb != 0 {
                bits.write(arr_x[i] as u32, nb as u32);
            }
        }
    } else {
        bits.write(0, 1);
        bits.write((used_symbols - 2) as u32, 3);

        let mut sympos: Vec<u32> = Vec::new();
        for i in 0..histo_size {
            if lookup[i] != 0 {
                sympos.push(i as u32 | (lookup[i] << 16));
            }
        }
        sympos.sort_unstable();

        let mut delta_bits = 1i32;
        let mut posv = 0i32;
        for i in 0..used_symbols - 1 {
            let v = (sympos[i] >> 16) as i32;
            let nb = if v - posv != 0 {
                bsr32((v - posv) as u32) as i32 + 1
            } else {
                0
            };
            delta_bits = delta_bits.max(nb);
            posv = v;
        }
        bits.write(delta_bits as u32, bsr32(l_bits) + 1);

        let mut posv = 0i32;
        for i in 0..used_symbols - 1 {
            let v = (sympos[i] >> 16) as i32;
            bits.write(
                ((v - posv) as u32) + (((sympos[i] as u8) as u32) << delta_bits),
                (delta_bits + 8) as u32,
            );
            posv = v;
        }
        bits.write((sympos[used_symbols - 1] as u8) as u32, 8);
    }
}

fn bits_for_rice(arr: &[u32], size: usize, k: i32) -> i32 {
    let mut result = 0i32;
    for (i, &c) in arr.iter().enumerate().take(size) {
        if c != 0 {
            result += c as i32 * (k + 1 + 2 * bsr32(((i as i32 >> k) + 1) as u32) as i32);
        }
    }
    result
}

/// Encode the symbol ranges. Returns (rice, bits, bitcount, count).
fn encode_sym_range(used_syms: i32, range: &[i32]) -> (Vec<u8>, Vec<u8>, Vec<u8>, usize) {
    let numrange = range.len();
    let mut rice = vec![0u8; 256];
    let mut bits = vec![0u8; 256];
    let mut bitcount = vec![0u8; 256];
    if used_syms >= 256 {
        return (rice, bits, bitcount, 0);
    }
    let which0 = (range[0] == 0) as i32;
    let num = ((range[0] != 0) as i32) + 2 * ((numrange as i32 - 3) / 2);
    let base = (range[0] == 0) as usize;
    let mut which = which0;
    for i in 0..num as usize {
        let mut v = range[base + i];
        let ebit = (!which) & 1;
        which += 1;
        v += (1 << ebit) - 1;
        let nb0 = bsr32((v >> ebit) as u32) as i32;
        rice[i] = nb0 as u8;
        let nb = nb0 + ebit;
        bits[i] = (v & ((1 << nb) - 1)) as u8;
        bitcount[i] = nb as u8;
    }
    (rice, bits, bitcount, num as usize)
}

/// Write the encoded symbol-range count.
fn write_num_sym_range(bits: &mut BitWriterFwd, num_symrange: usize, used_syms: i32) {
    if used_syms == 256 {
        return;
    }
    let x = used_syms.min(257 - used_syms);
    let nb = bsr32((2 * x - 1) as u32) as i32 + 1;
    let base = (1 << nb) - 2 * x;
    if num_symrange as i32 >= base {
        bits.write((num_symrange as i32 + base) as u32, nb as u32);
    } else {
        bits.write(num_symrange as u32, (nb - 1) as u32);
    }
}

/// Write many Rice codes for an i32 slice (values are small unary counts).
fn write_many_rice(bits: &mut BitWriterFwd, data: &[i32]) {
    for &val in data {
        let mut v = val;
        while v >= 24 {
            bits.write(0, 24);
            v -= 24;
        }
        bits.write(1, (v + 1) as u32);
    }
}
fn write_many_rice_u8(bits: &mut BitWriterFwd, data: &[u8]) {
    for &val in data {
        let mut v = val as i32;
        while v >= 24 {
            bits.write(0, 24);
            v -= 24;
        }
        bits.write(1, (v + 1) as u32);
    }
}

fn write_sym_range_low(bits: &mut BitWriterFwd, data: &[u8], bitcount: &[u8]) {
    for i in 0..data.len() {
        bits.write(data[i] as u32, bitcount[i] as u32);
    }
}

/// Encoder transition table: `next` maps `state >> nb` to the new state.
struct TransTable {
    te: Vec<TeRec>,
    next: Vec<u16>, // transition state table
}

#[derive(Clone, Copy, Default)]
struct TeRec {
    // next_state pointer expressed as an index offset into `next`: new_state = next[idx + (state>>nb)]
    next_off: i64,
    thres: u32,
    bits: u8,
    present: bool,
}

/// Build the encoder transition table from the normalized weights.
fn init_table(weights: &[u32; ALPHABET], weights_size: usize, l_bits: u32) -> TransTable {
    let l = 1u32 << l_bits;
    let mut ones = 0u32;
    for &w in weights.iter().take(weights_size) {
        if w == 1 {
            ones += 1;
        }
    }
    let slots_left = l - ones;
    let sa = slots_left >> 2;
    let mut pointers = [0i64; 4];
    pointers[0] = 0;
    let mut sb = sa as i64 + ((slots_left & 3) > 0) as i64;
    pointers[1] = sb;
    sb += sa as i64 + ((slots_left & 3) > 1) as i64;
    pointers[2] = sb;
    sb += sa as i64 + ((slots_left & 3) > 2) as i64;
    pointers[3] = sb;

    let mut next = vec![0u16; l as usize];
    let mut te = vec![TeRec::default(); weights_size];

    let mut ones_ptr = slots_left as i64; // index into next
    let mut weights_sum = 0i64;

    for i in 0..weights_size {
        let w = weights[i];
        if w == 0 {
            te[i].present = false;
            continue;
        }
        te[i].present = true;
        if w == 1 {
            te[i].bits = l_bits as u8;
            te[i].thres = 2 * l;
            // next_state = ones_ptr - 1 (index); new_state = next[(ones_ptr-1) + (state>>nb)].
            te[i].next_off = ones_ptr - 1;
            next[ones_ptr as usize] = (l + ones_ptr as u32) as u16;
            ones_ptr += 1;
        } else {
            let nb = bsr32(w - 1) as i32 + 1;
            te[i].bits = (l_bits as i32 - nb) as u8;
            te[i].thres = (2 * w) << (l_bits as i32 - nb);
            let other_base = weights_sum; // index into next
            te[i].next_off = other_base - w as i64;
            let mut other_ptr = other_base;
            for j in 0..4usize {
                let p0 = pointers[j];
                let mut p = p0;
                let y = (w as i64 + ((weights_sum - j as i64 - 1) & 3)) >> 2;
                let mut cnt = y;
                while cnt > 0 {
                    next[other_ptr as usize] = (p as u32 + l) as u16;
                    other_ptr += 1;
                    p += 1;
                    cnt -= 1;
                }
                pointers[j] = p;
            }
            weights_sum += w as i64;
        }
    }
    TransTable { te, next }
}

/// A 64-bit bit writer over a scratch byte buffer. `Dir = +1` grows up
/// (byteswapped 8-byte stores), `Dir = -1` grows down (plain 8-byte stores). The produced
/// bytes match exactly what the decoder consumes.
struct Bw64 {
    bits: u64,
    pos: u32, // bit position, starts at 63
    ptr: i64, // byte index of the next 8-byte store base
    dir: i32,
}
impl Bw64 {
    fn new(ptr: i64, dir: i32) -> Self {
        Bw64 {
            bits: 0,
            pos: 63,
            ptr,
            dir,
        }
    }
    #[inline]
    fn write_no_flush(&mut self, bits: u32, n: u32) {
        self.pos = self.pos.wrapping_sub(n);
        self.bits = (self.bits << n) | bits as u64;
    }
    #[inline]
    fn flush(&mut self, buf: &mut [u8]) {
        let t = (63 - self.pos) >> 3;
        // `bits << (pos+1)` is only meaningful when a whole byte is ready (it would be UB at
        // pos==63, but there t==0 so the value is discarded).
        if t == 0 {
            return;
        }
        let v = self.bits << (self.pos + 1);
        self.pos += 8 * t;
        if self.dir < 0 {
            // *(u64*)(ptr - 8) = v
            let base = (self.ptr - 8) as usize;
            buf[base..base + 8].copy_from_slice(&v.to_le_bytes());
            self.ptr -= t as i64;
        } else {
            // *(u64*)ptr = byteswap(v)
            let base = self.ptr as usize;
            buf[base..base + 8].copy_from_slice(&v.swap_bytes().to_le_bytes());
            self.ptr += t as i64;
        }
    }
    #[inline]
    fn write(&mut self, bits: u32, n: u32, buf: &mut [u8]) {
        self.write_no_flush(bits, n);
        self.flush(buf);
    }
    /// Return the final cursor after all writes.
    fn final_ptr(&self) -> i64 {
        if self.dir >= 0 {
            self.ptr + (self.pos != 63) as i64
        } else {
            self.ptr - (self.pos != 63) as i64
        }
    }
}

/// Total forward/backward bit counts (for padding alignment).
fn get_encoded_bit_count(tt: &TransTable, src: &[u8], l_bits: u32) -> (u32, u32) {
    let l = 1u32 << l_bits;
    let src_size = src.len();
    let se = src_size - 5;
    let mut state = [
        src[se] as u32 | l,
        src[se + 1] as u32 | l,
        src[se + 2] as u32 | l,
        src[se + 3] as u32 | l,
        src[se + 4] as u32 | l,
    ];
    let mut fwd = 0u32;
    let mut bwd = 0u32;
    let rounds = (src_size - 5) / 10;
    let mut idx = se as i64 - 1;

    macro_rules! count {
        ($s:expr, $ctr:expr) => {{
            let sym = src[idx as usize] as usize;
            idx -= 1;
            let rec = tt.te[sym];
            let nb = rec.bits as u32 + (state[$s] >= rec.thres) as u32;
            $ctr += nb;
            let ns = rec.next_off + (state[$s] >> nb) as i64;
            state[$s] = tt.next[ns as usize] as u32;
        }};
    }
    let rem = (src_size - 5) % 10;
    let tail: &[(usize, bool)] = &[
        (3, true),
        (2, true),
        (1, true),
        (0, true),
        (4, false),
        (3, false),
        (2, false),
        (1, false),
        (0, false),
    ];
    for &(s, f) in &tail[9 - rem..] {
        if f {
            count!(s, fwd);
        } else {
            count!(s, bwd);
        }
    }
    for _ in 0..rounds {
        count!(4, fwd);
        count!(3, fwd);
        count!(2, fwd);
        count!(1, fwd);
        count!(0, fwd);
        count!(4, bwd);
        count!(3, bwd);
        count!(2, bwd);
        count!(1, bwd);
        count!(0, bwd);
    }
    (fwd + 2 * l_bits, bwd + 3 * l_bits)
}

/// Encode symbols high→low into a forward stream
/// (offset 0, growing up) and a backward stream (high end, growing down) within a scratch
/// buffer, then swap the two so the final data is `BACKWARD…FORWARD`.
fn encode_bytes(tt: &TransTable, src: &[u8], l_bits: u32) -> Option<Vec<u8>> {
    let l = 1u32 << l_bits;
    let src_size = src.len();
    let se = src_size - 5;

    let (fwd_pad, bwd_pad) = get_encoded_bit_count(tt, src, l_bits);

    // Scratch large enough for both streams plus the 8-byte store slack.
    let cap = src_size + 64;
    let mut buf = vec![0u8; cap];
    let dst_lo: i64 = 8; // leave 8 bytes of low slack so dir<0 stores (ptr-8) stay in range
    let dst_hi: i64 = (cap - 8) as i64;

    let mut fb = Bw64::new(dst_lo, 1);
    let mut bb = Bw64::new(dst_hi, -1);

    if fwd_pad & 7 != 0 {
        fb.write_no_flush(0, 8 - (fwd_pad & 7));
    }
    if bwd_pad & 7 != 0 {
        bb.write_no_flush(0, 8 - (bwd_pad & 7));
    }

    let mut state = [
        src[se] as u32 | l,
        src[se + 1] as u32 | l,
        src[se + 2] as u32 | l,
        src[se + 3] as u32 | l,
        src[se + 4] as u32 | l,
    ];
    let mut idx = se as i64 - 1;

    macro_rules! enc {
        ($s:expr, $w:expr) => {{
            let sym = src[idx as usize] as usize;
            idx -= 1;
            let rec = tt.te[sym];
            let nb = rec.bits as u32 + (state[$s] >= rec.thres) as u32;
            $w.write_no_flush(state[$s] & ((1u32 << nb) - 1), nb);
            let ns = rec.next_off + (state[$s] >> nb) as i64;
            state[$s] = tt.next[ns as usize] as u32;
        }};
    }

    let rounds = (src_size - 5) / 10;
    let rem = (src_size - 5) % 10;
    // Tail switch (with flush after case 1).
    let tail: &[(usize, bool)] = &[
        (3, true),
        (2, true),
        (1, true),
        (0, true),
        (4, false),
        (3, false),
        (2, false),
        (1, false),
        (0, false),
    ];
    let tstart = 9 - rem;
    for (k, &(s, f)) in tail[tstart..].iter().enumerate() {
        if f {
            enc!(s, fb);
        } else {
            enc!(s, bb);
        }
        // Flush after the case-1 body (the last tail entry).
        let is_last = tstart + k == tail.len() - 1;
        if is_last {
            bb.flush(&mut buf);
            fb.flush(&mut buf);
        }
    }

    for _ in 0..rounds {
        enc!(4, fb);
        enc!(3, fb);
        enc!(2, fb);
        enc!(1, fb);
        enc!(0, fb);
        enc!(4, bb);
        enc!(3, bb);
        enc!(2, bb);
        enc!(1, bb);
        enc!(0, bb);
        bb.flush(&mut buf);
        fb.flush(&mut buf);
    }

    // Final state bits.
    bb.write_no_flush(state[4] & (l - 1), l_bits);
    bb.write_no_flush(state[2] & (l - 1), l_bits);
    bb.write_no_flush(state[0] & (l - 1), l_bits);
    fb.write_no_flush(state[3] & (l - 1), l_bits);
    fb.write_no_flush(state[1] & (l - 1), l_bits);
    bb.flush(&mut buf);
    fb.flush(&mut buf);

    if idx != -1 {
        return None;
    }

    // forward bytes occupy [dst_lo, fb.final_ptr()); backward bytes occupy [bb.final_ptr(), dst_hi).
    let f_end = fb.final_ptr();
    let b_start = bb.final_ptr();
    let forward_bytes = (f_end - dst_lo) as usize;
    let backward_bytes = (dst_hi - b_start) as usize;

    // Assemble BACKWARD..FORWARD (swapped so the decoder reads backward from the very end).
    let mut out = Vec::with_capacity(forward_bytes + backward_bytes);
    out.extend_from_slice(&buf[b_start as usize..b_start as usize + backward_bytes]);
    out.extend_from_slice(&buf[dst_lo as usize..dst_lo as usize + forward_bytes]);
    Some(out)
}

/// Build a chunk header (long form) + payload.
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
