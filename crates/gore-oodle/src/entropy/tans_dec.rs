//! tANS array decode.
//!
//! A tANS array is: a 1-bit reserved flag (must be 0), 2 bits of `L_bits-8` (table size
//! `L = 1<<L_bits`, `L_bits` in 8..=11), a normalized-count table, then five interleaved
//! arithmetic states decoded from a *forward* and a *backward* bit stream (3 states off the
//! forward stream, 2 off the backward stream per the round structure). The final five states
//! are the last five output bytes.

#![allow(dead_code)]

use super::bitio::{bsr32, BitReader};
use super::hufftable::{convert_to_ranges, read_fluff, HuffRange};
use super::rice::{decode_golomb_rice_lengths, Rice2};
use crate::{Error, Result};

/// The decoded normalized symbol/weight lists.
struct TansData {
    a: Vec<u8>,        // weight == 1 symbols
    b: Vec<u32>,       // (symbol<<16) | weight, weight >= 2
}

/// One decode LUT entry: state transition plus its output symbol.
#[derive(Clone, Copy, Default)]
struct LutEnt {
    x: u32,
    bits_x: u8,
    symbol: u8,
    w: u16,
}

use alloc::vec;
use alloc::vec::Vec;

/// Decode one tANS array payload into `dst`. Returns payload bytes consumed.
pub(crate) fn decode_tans(src: &[u8], dst: &mut [u8]) -> Result<usize> {
    let src_size = src.len();
    if src_size < 8 || dst.len() < 5 {
        return Err(Error::Corrupt("tans: too small"));
    }

    let mut br = BitReader::new(src);
    // reserved bit
    if br.read_bit_no_refill() != 0 {
        return Err(Error::Corrupt("tans: reserved bit"));
    }
    let l_bits = br.read_bits_no_refill(2) + 8;

    let tans = decode_table(&mut br, l_bits)?;

    // Recover byte position after the table.
    let p = br.cursor_after();
    if p >= src_size {
        return Err(Error::Corrupt("tans: no data"));
    }

    let l = 1u32 << l_bits;
    let lut = init_lut(&tans, l_bits, l);

    let dst_len = dst.len();
    let dst_end = dst_len - 5; // states are the last 5 bytes

    // Read out the initial states. Forward stream at `p`, backward stream at end.
    let l_mask = (1u32 << l_bits) - 1;
    let mut f = p;
    let mut bits_f = le32(src, f);
    f += 4;
    let mut bits_b = be32(src, src_size - 4);
    let mut bend = src_size - 4;
    let mut bitpos_f: i32 = 32;
    let mut bitpos_b: i32 = 32;

    let mut state = [0u32; 5];
    state[0] = bits_f & l_mask;
    state[1] = bits_b & l_mask;
    bits_f >>= l_bits;
    bitpos_f -= l_bits as i32;
    bits_b >>= l_bits;
    bitpos_b -= l_bits as i32;

    state[2] = bits_f & l_mask;
    state[3] = bits_b & l_mask;
    bits_f >>= l_bits;
    bitpos_f -= l_bits as i32;
    bits_b >>= l_bits;
    bitpos_b -= l_bits as i32;

    // Refill forward.
    bits_f |= le32(src, f) << bitpos_f;
    f += ((31 - bitpos_f) >> 3) as usize;
    bitpos_f |= 24;

    state[4] = bits_f & l_mask;
    bits_f >>= l_bits;
    bitpos_f -= l_bits as i32;

    // ptr_f = src + (f) - (bitpos_f>>3); ptr_b = src_end + (bitpos_b>>3).
    let ptr_f = f as i64 - (bitpos_f >> 3) as i64;
    let bitpos_f = bitpos_f & 7;
    let ptr_b = bend as i64 + (bitpos_b >> 3) as i64;
    let bitpos_b = bitpos_b & 7;
    let _ = &mut bend;

    run_decode(
        src, &lut, dst, dst_end, ptr_f, ptr_b, bits_f, bits_b, bitpos_f, bitpos_b, &mut state,
    )?;

    Ok(src_size)
}

/// The five-state interleaved decode loop.
#[allow(clippy::too_many_arguments)]
fn run_decode(
    src: &[u8],
    lut: &[LutEnt],
    dst: &mut [u8],
    dst_end: usize,
    mut ptr_f: i64,
    mut ptr_b: i64,
    mut bits_f: u32,
    mut bits_b: u32,
    mut bitpos_f: i32,
    mut bitpos_b: i32,
    state: &mut [u32; 5],
) -> Result<()> {
    let mut dpos = 0usize;

    #[inline]
    fn le32_at(src: &[u8], idx: i64) -> u32 {
        let mut v = 0u32;
        for k in 0..4 {
            let i = idx + k;
            let byte = if i >= 0 && (i as usize) < src.len() {
                src[i as usize]
            } else {
                0
            };
            v |= (byte as u32) << (8 * k);
        }
        v
    }
    #[inline]
    fn be32_before(src: &[u8], idx: i64) -> u32 {
        // The 4 bytes ending at idx, big-endian.
        let mut v = 0u32;
        for k in 0..4 {
            let i = idx - 4 + k;
            let byte = if i >= 0 && (i as usize) < src.len() {
                src[i as usize]
            } else {
                0
            };
            // big-endian: most significant first => byte at (idx-4) is the top.
            v = (v << 8) | byte as u32;
        }
        v
    }

    if ptr_f > ptr_b {
        return Err(Error::Corrupt("tans: f>b"));
    }

    // Macro-equivalent helpers.
    macro_rules! fwd_bits {
        () => {{
            bits_f |= le32_at(src, ptr_f) << bitpos_f;
            ptr_f += ((31 - bitpos_f) >> 3) as i64;
            bitpos_f |= 24;
        }};
    }
    macro_rules! bwd_bits {
        () => {{
            bits_b |= be32_before(src, ptr_b) << bitpos_b;
            ptr_b -= ((31 - bitpos_b) >> 3) as i64;
            bitpos_b |= 24;
        }};
    }

    // Each round emits one symbol off a given state, breaking when dst is full.
    let mut done = false;
    if dpos < dst_end {
        'outer: loop {
            // FORWARD x3 (states 0,1 then 2,3 then 4)
            fwd_bits!();
            for &s in &[0usize, 1] {
                let e = lut[state[s] as usize];
                dst[dpos] = e.symbol;
                dpos += 1;
                bitpos_f -= e.bits_x as i32;
                state[s] = (bits_f & e.x) + e.w as u32;
                bits_f >>= e.bits_x;
                if dpos >= dst_end {
                    done = true;
                    break 'outer;
                }
            }
            fwd_bits!();
            for &s in &[2usize, 3] {
                let e = lut[state[s] as usize];
                dst[dpos] = e.symbol;
                dpos += 1;
                bitpos_f -= e.bits_x as i32;
                state[s] = (bits_f & e.x) + e.w as u32;
                bits_f >>= e.bits_x;
                if dpos >= dst_end {
                    done = true;
                    break 'outer;
                }
            }
            fwd_bits!();
            {
                let e = lut[state[4] as usize];
                dst[dpos] = e.symbol;
                dpos += 1;
                bitpos_f -= e.bits_x as i32;
                state[4] = (bits_f & e.x) + e.w as u32;
                bits_f >>= e.bits_x;
                if dpos >= dst_end {
                    done = true;
                    break 'outer;
                }
            }
            // BACKWARD x5 (states 0,1 then 2,3 then 4)
            bwd_bits!();
            for &s in &[0usize, 1] {
                let e = lut[state[s] as usize];
                dst[dpos] = e.symbol;
                dpos += 1;
                bitpos_b -= e.bits_x as i32;
                state[s] = (bits_b & e.x) + e.w as u32;
                bits_b >>= e.bits_x;
                if dpos >= dst_end {
                    done = true;
                    break 'outer;
                }
            }
            bwd_bits!();
            for &s in &[2usize, 3] {
                let e = lut[state[s] as usize];
                dst[dpos] = e.symbol;
                dpos += 1;
                bitpos_b -= e.bits_x as i32;
                state[s] = (bits_b & e.x) + e.w as u32;
                bits_b >>= e.bits_x;
                if dpos >= dst_end {
                    done = true;
                    break 'outer;
                }
            }
            bwd_bits!();
            {
                let e = lut[state[4] as usize];
                dst[dpos] = e.symbol;
                dpos += 1;
                bitpos_b -= e.bits_x as i32;
                state[4] = (bits_b & e.x) + e.w as u32;
                bits_b >>= e.bits_x;
                if dpos >= dst_end {
                    done = true;
                    break 'outer;
                }
            }
        }
    }
    let _ = done;

    // Validate stream meeting and state ranges.
    if ptr_b - ptr_f + (bitpos_f >> 3) as i64 + (bitpos_b >> 3) as i64 != 0 {
        return Err(Error::Corrupt("tans: streams did not meet"));
    }
    let states_or = state[0] | state[1] | state[2] | state[3] | state[4];
    if states_or & !0xFF != 0 {
        return Err(Error::Corrupt("tans: state out of range"));
    }
    dst[dst_end] = state[0] as u8;
    dst[dst_end + 1] = state[1] as u8;
    dst[dst_end + 2] = state[2] as u8;
    dst[dst_end + 3] = state[3] as u8;
    dst[dst_end + 4] = state[4] as u8;
    Ok(())
}

/// Decode the normalized-count table into symbol/weight lists.
fn decode_table(br: &mut BitReader, l_bits: u32) -> Result<TansData> {
    br.refill();
    let l = 1u32 << l_bits;
    if br.read_bit_no_refill() != 0 {
        // Rice-coded weights.
        let q = br.read_bits_no_refill(3) as i32;
        let num_symbols = br.read_bits_no_refill(8) as i32 + 1;
        if num_symbols < 2 {
            return Err(Error::Corrupt("tans: num_symbols<2"));
        }
        let fluff = read_fluff(br, num_symbols);
        let total_rice_values = (fluff + num_symbols) as usize;

        let mut rice = vec![0u8; 512 + 16];
        let mut br2 = Rice2 {
            p: br.p - (((24 - br.bitpos + 7) >> 3) as usize),
            p_end: br.p_end,
            bitpos: ((br.bitpos - 24) & 7) as u32,
        };
        decode_golomb_rice_lengths(&mut rice[..total_rice_values], br.data, &mut br2)?;
        for b in &mut rice[total_rice_values..total_rice_values + 16] {
            *b = 0;
        }

        // switch back
        br.bitpos = 24;
        br.p = br2.p;
        br.bits = 0;
        br.refill();
        br.bits <<= br2.bitpos;
        br.bitpos += br2.bitpos as i32;

        let mut range = [HuffRange::default(); 133];
        let ranges = convert_to_ranges(
            &mut range,
            num_symbols,
            fluff,
            &rice[num_symbols as usize..],
            br,
        )?;

        br.refill();

        let mut a: Vec<u8> = Vec::new();
        let mut b: Vec<u32> = Vec::new();
        let mut cur = 0usize; // rice ptr
        let mut average = 6i32;
        let mut somesum = 0i32;

        for ri in 0..ranges {
            let mut symbol = range[ri].symbol as i32;
            let mut num = range[ri].num as i32;
            while num > 0 {
                br.refill();
                let nextra = q + rice[cur] as i32;
                cur += 1;
                if nextra > 15 {
                    return Err(Error::Corrupt("tans: nextra>15"));
                }
                let mut v = br.read_bits_no_refill_zero(nextra as u32) as i32 + (1 << nextra)
                    - (1 << q);
                let average_div4 = average >> 2;
                let mut limit = 2 * average_div4;
                if v <= limit {
                    v = average_div4 + (-(v & 1) ^ (v >> 1));
                }
                if limit > v {
                    limit = v;
                }
                v += 1;
                average += limit - average_div4;
                if v == 1 {
                    a.push(symbol as u8);
                } else {
                    b.push(((symbol as u32) << 16) + v as u32);
                }
                somesum += v;
                symbol += 1;
                num -= 1;
            }
        }
        if somesum as u32 != l {
            return Err(Error::Corrupt("tans: weight sum != L"));
        }
        Ok(TansData { a, b })
    } else {
        // Direct delta-coded weights.
        let mut seen = [false; 256];
        let count = br.read_bits_no_refill(3) as i32 + 1;
        let bits_per_sym = bsr32(l_bits) + 1;
        let max_delta_bits = br.read_bits_no_refill(bits_per_sym) as i32;
        if max_delta_bits == 0 || max_delta_bits > l_bits as i32 {
            return Err(Error::Corrupt("tans: max_delta_bits"));
        }
        let mut a: Vec<u8> = Vec::new();
        let mut b: Vec<u32> = Vec::new();
        let mut weight = 0i32;
        let mut total_weights = 0i32;

        for _ in 0..count {
            br.refill();
            let sym = br.read_bits_no_refill(8) as usize;
            if seen[sym] {
                return Err(Error::Corrupt("tans: dup sym"));
            }
            let delta = br.read_bits_no_refill(max_delta_bits as u32) as i32;
            weight += delta;
            if weight == 0 {
                return Err(Error::Corrupt("tans: zero weight"));
            }
            seen[sym] = true;
            if weight == 1 {
                a.push(sym as u8);
            } else {
                b.push(((sym as u32) << 16) + weight as u32);
            }
            total_weights += weight;
        }

        br.refill();
        let sym = br.read_bits_no_refill(8) as usize;
        if seen[sym] {
            return Err(Error::Corrupt("tans: dup last sym"));
        }
        if (l as i32 - total_weights) < weight || (l as i32 - total_weights) <= 1 {
            return Err(Error::Corrupt("tans: last weight"));
        }
        b.push(((sym as u32) << 16) + (l - total_weights as u32));

        a.sort_unstable();
        b.sort_unstable();
        Ok(TansData { a, b })
    }
}

/// Build the decode LUT from the normalized symbol/weight lists.
fn init_lut(tans: &TansData, l_bits: u32, l: u32) -> Vec<LutEnt> {
    let mut lut = vec![LutEnt::default(); l as usize];
    let a_used = tans.a.len() as u32;
    let slots_left = l - a_used;

    let sa = slots_left >> 2;
    let mut pointers = [0usize; 4];
    pointers[0] = 0;
    let mut sb = sa + ((slots_left & 3) > 0) as u32;
    pointers[1] = sb as usize;
    sb += sa + ((slots_left & 3) > 1) as u32;
    pointers[2] = sb as usize;
    sb += sa + ((slots_left & 3) > 2) as u32;
    pointers[3] = sb as usize;

    // weight==1 singles at the end.
    {
        let base = slots_left as usize;
        for (i, &sym) in tans.a.iter().enumerate() {
            lut[base + i] = LutEnt {
                w: 0,
                bits_x: l_bits as u8,
                x: (1u32 << l_bits) - 1,
                symbol: sym,
            };
        }
    }

    let mut weights_sum = 0i32;
    for &packed in &tans.b {
        let weight = (packed & 0xffff) as i32;
        let symbol = (packed >> 16) as u8;
        if weight > 4 {
            let sym_bits = bsr32(weight as u32);
            let mut z = l_bits as i32 - sym_bits as i32;
            let mut le = LutEnt {
                symbol,
                bits_x: z as u8,
                x: (1u32 << z) - 1,
                w: ((l - 1) & ((weight as u32) << z)) as u16,
            };
            let mut what_to_add = 1i32 << z;
            let mut xx = (1i32 << (sym_bits + 1)) - weight;

            for j in 0..4usize {
                let mut dst = pointers[j];
                let y = (weight + ((weights_sum - j as i32 - 1) & 3)) >> 2;
                if xx >= y {
                    for _ in 0..y {
                        lut[dst] = le;
                        dst += 1;
                        le.w = le.w.wrapping_add(what_to_add as u16);
                    }
                    xx -= y;
                } else {
                    for _ in 0..xx {
                        lut[dst] = le;
                        dst += 1;
                        le.w = le.w.wrapping_add(what_to_add as u16);
                    }
                    z -= 1;
                    what_to_add >>= 1;
                    le.bits_x = z as u8;
                    le.w = 0;
                    le.x >>= 1;
                    for _ in 0..(y - xx) {
                        lut[dst] = le;
                        dst += 1;
                        le.w = le.w.wrapping_add(what_to_add as u16);
                    }
                    xx = weight;
                }
                pointers[j] = dst;
            }
        } else {
            let mut bits = ((1u32 << weight) - 1) << (weights_sum & 3);
            bits |= bits >> 4;
            let mut n = weight;
            let mut ww = weight as u32;
            while n > 0 {
                let idx = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                let dst = pointers[idx];
                pointers[idx] += 1;
                let weight_bits = bsr32(ww);
                lut[dst] = LutEnt {
                    symbol,
                    bits_x: (l_bits - weight_bits) as u8,
                    x: (1u32 << (l_bits - weight_bits)) - 1,
                    w: ((l - 1) & (ww << (l_bits - weight_bits))) as u16,
                };
                ww += 1;
                n -= 1;
            }
        }
        weights_sum += weight;
    }
    lut
}

#[inline]
fn le32(src: &[u8], idx: usize) -> u32 {
    let mut v = 0u32;
    for k in 0..4 {
        let byte = src.get(idx + k).copied().unwrap_or(0);
        v |= (byte as u32) << (8 * k);
    }
    v
}

#[inline]
fn be32(src: &[u8], idx: usize) -> u32 {
    let mut v = 0u32;
    for k in 0..4 {
        let byte = src.get(idx + k).copied().unwrap_or(0);
        v = (v << 8) | byte as u32;
    }
    v
}
