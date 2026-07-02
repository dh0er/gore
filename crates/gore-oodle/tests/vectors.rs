//! Decode every frozen real-Oodle vector back to its `.raw` golden output.
//!
//! Each `.krk` is a real Oodle Kraken stream and the `.raw` is its exact decompressed
//! bytes. A passing decode here proves the Kraken decode loop *and* the entropy stage are
//! bit-exact against the reference encoder.

use gore_oodle::decompress;

macro_rules! v {
    ($n:ident) => {
        #[test]
        fn $n() {
            let raw = include_bytes!(concat!("vectors/", stringify!($n), ".raw"));
            let krk = include_bytes!(concat!("vectors/", stringify!($n), ".krk"));
            assert_eq!(decompress(krk, raw.len()).unwrap(), raw);
        }
    };
}

v!(one_byte);
v!(zeros_64k);
v!(repetitive);
v!(text);
v!(counter);
v!(multiblock);
v!(random);
// Huffman "new" Golomb-Rice code-length path with a byte 0x20 in the length stream — the
// real-save decode that regressed as "huff: lut not full" before the RICE_VALUE[0x20] fix.
v!(rice_new);
