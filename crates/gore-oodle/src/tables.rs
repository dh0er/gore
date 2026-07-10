//! Static lookup tables for the Kraken codec.

#![allow(dead_code)]

/// 11-bit bit-reversal table. `REVERSE_BITS[i]` is the low 11 bits of `i` reversed.
///
/// This is pure data: a bit-reversal has exactly one correct value per index, so a
/// const-evaluated table is guaranteed bit-identical to the vendored literal array with
/// zero transcription risk. (Spot-checked against the header in the unit test below.)
pub(crate) static REVERSE_BITS: [u16; 2048] = build_reverse_bits();

const fn build_reverse_bits() -> [u16; 2048] {
    let mut t = [0u16; 2048];
    let mut i = 0;
    while i < 2048 {
        let mut r = 0u16;
        let mut b = 0;
        while b < 11 {
            r |= (((i as u16) >> b) & 1) << (10 - b);
            b += 1;
        }
        t[i] = r;
        i += 1;
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_bits_is_a_bit_reversal() {
        // REVERSE_BITS[i] reverses the low 11 bits of i.
        for i in [0usize, 1, 2, 1023, 2047] {
            let r = (0..11).fold(0u16, |acc, b| acc | (((i as u16 >> b) & 1) << (10 - b)));
            assert_eq!(REVERSE_BITS[i], r, "i={i}");
        }
    }

    #[test]
    fn reverse_bits_matches_vendored_literals() {
        // Spot-check against known bit-reversal literals so the
        // const-fn can never silently drift.
        assert_eq!(REVERSE_BITS[0], 0x000);
        assert_eq!(REVERSE_BITS[1], 0x400);
        assert_eq!(REVERSE_BITS[2], 0x200);
        assert_eq!(REVERSE_BITS[3], 0x600);
        assert_eq!(REVERSE_BITS[2032], 0x07f);
        assert_eq!(REVERSE_BITS[2047], 0x7ff);
        assert_eq!(REVERSE_BITS.len(), 2048);
    }
}
