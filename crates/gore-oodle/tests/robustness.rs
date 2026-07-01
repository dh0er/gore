//! Hostile-input robustness: a malformed/short stream or an absurd `decompressed_len` must
//! return an `Err`, never panic, OOB-read, or attempt an unbounded allocation.

use gore_oodle::decompress;

/// A genuinely huge requested size must fail the fallible allocation, not abort the process.
#[test]
fn absurd_output_size_errors() {
    // ~ near usize::MAX: try_reserve_exact must reject this.
    let r = decompress(&[0xCC, 0x06, 0x00, 0x00], usize::MAX / 2);
    assert!(r.is_err(), "absurd size should error, got Ok");
}

/// Zero-length output is trivially the empty vector regardless of input.
#[test]
fn zero_len_is_empty() {
    assert_eq!(decompress(&[], 0).unwrap(), Vec::<u8>::new());
    assert_eq!(decompress(&[0xff, 0xff, 0xff], 0).unwrap(), Vec::<u8>::new());
}

/// Truncated stream header / quantum.
#[test]
fn truncated_inputs_error() {
    // Want output but no input at all.
    assert!(decompress(&[], 16).is_err());
    // Stream header only, no quantum header.
    assert!(decompress(&[0x8c, 0x06], 16).is_err());
    // Claims a memset but is cut off before the value byte.
    assert!(decompress(&[0x0c, 0x06, 0x07, 0xff], 16).is_err());
}

/// Fuzz: every truncation of each real vector must either decode-or-error without panicking.
#[test]
fn truncated_vectors_never_panic() {
    let vectors: &[(&[u8], usize)] = &[
        (include_bytes!("vectors/counter.krk"), 200_000),
        (include_bytes!("vectors/text.krk"), 185),
        (include_bytes!("vectors/repetitive.krk"), 40_000),
        (include_bytes!("vectors/multiblock.krk"), 600_000),
        (include_bytes!("vectors/random.krk"), 50_000),
    ];
    for (data, len) in vectors {
        for cut in 0..data.len() {
            // Must not panic; result value is irrelevant.
            let _ = decompress(&data[..cut], *len);
            // Also try with a smaller-than-real requested length.
            let _ = decompress(&data[..cut], len / 2);
        }
    }
}

/// Fuzz: single-byte corruptions of each vector must never panic (decode or error only).
#[test]
fn corrupted_vectors_never_panic() {
    let vectors: &[(&[u8], usize)] = &[
        (include_bytes!("vectors/counter.krk"), 200_000),
        (include_bytes!("vectors/text.krk"), 185),
        (include_bytes!("vectors/repetitive.krk"), 40_000),
        (include_bytes!("vectors/multiblock.krk"), 600_000),
    ];
    for (data, len) in vectors {
        let mut buf = data.to_vec();
        for i in 0..buf.len() {
            let orig = buf[i];
            for delta in [1u8, 0x80, 0xff] {
                buf[i] = orig ^ delta;
                let _ = decompress(&buf, *len);
            }
            buf[i] = orig;
        }
    }
}
