//! Gate 1 (fast): the in-tree decoder must reproduce every input from `compress`'s output.
//!
//! `crate::decompress(&compress(x, lvl)?, x.len())? == x` for a spread of inputs that
//! exercise the empty, single-quantum, multi-quantum, all-same, skewed, repetitive and
//! incompressible paths — at both `Fastest` and `Default`. Ratios (compressed/raw) are
//! printed with `--nocapture` so the encoder's strength is visible.

use gore_oodle::{compress, decompress, Level};

/// One labelled corpus entry.
struct Case {
    name: &'static str,
    data: Vec<u8>,
}

/// The shared corpus used by both gates. Kept in a free function so the oracle cross-check
/// test can build the identical set.
fn corpus() -> Vec<Case> {
    let mut cases = Vec::new();
    let mut push = |name: &'static str, data: Vec<u8>| cases.push(Case { name, data });

    push("empty", Vec::new());
    push("one_byte", vec![0x5a]);
    push("all_same_100k", vec![7u8; 100_000]);
    push("zeros_70k", vec![0u8; 70_000]);

    // Skewed text: mostly 'a' and spaces with an occasional other letter (compresses via
    // huffman/tANS).
    {
        let mut seed = 0x1234_5678u32;
        let mut d = Vec::with_capacity(80_000);
        for _ in 0..80_000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let r = (seed >> 24) as u8;
            d.push(if r < 205 {
                b'a'
            } else if r < 245 {
                b' '
            } else {
                b'a' + (r % 26)
            });
        }
        push("skewed_text", d);
    }

    // Highly repetitive: a short motif tiled many times (entropy collapses it hard).
    {
        let motif = b"The quick brown fox jumps over the lazy dog. ";
        let mut d = Vec::with_capacity(120_000);
        while d.len() < 120_000 {
            d.extend_from_slice(motif);
        }
        d.truncate(120_000);
        push("repetitive", d);
    }

    // Incompressible LCG bytes (~50 KiB): should fall back to stored/uncompressed framing.
    {
        let mut seed = 0xDEAD_BEEFu32;
        let mut d = Vec::with_capacity(50_000);
        for _ in 0..50_000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            d.push((seed >> 24) as u8);
        }
        push("incompressible_lcg", d);
    }

    // >256 KiB multi-quantum (~600 KB): mildly structured so most quanta compress, forcing
    // the multi-quantum framing path.
    {
        let mut d = Vec::with_capacity(600_000);
        for i in 0..600_000u32 {
            d.push((i % 251) as u8);
        }
        push("multi_quantum_600k", d);
    }

    cases
}

#[test]
fn roundtrips_all_cases_both_levels() {
    for case in corpus() {
        for level in [Level::Fastest, Level::Default] {
            let comp = compress(&case.data, level)
                .unwrap_or_else(|e| panic!("compress {} {level:?}: {e:?}", case.name));
            let back = decompress(&comp, case.data.len())
                .unwrap_or_else(|e| panic!("decompress {} {level:?}: {e:?}", case.name));
            assert_eq!(
                back, case.data,
                "roundtrip mismatch for {} at {level:?}",
                case.name
            );
            let ratio = if case.data.is_empty() {
                0.0
            } else {
                comp.len() as f64 / case.data.len() as f64
            };
            std::eprintln!(
                "{:>20} {:?}: {} -> {} bytes (ratio {:.4})",
                case.name,
                level,
                case.data.len(),
                comp.len(),
                ratio
            );
        }
    }
}

/// The per-block worst-case formula must not underflow on a large many-quantum buffer:
/// 240 full 256 KiB quanta = ~60 MiB. (The buffer is all-zero so it compresses, but the
/// allocation sizing is what's under test.)
#[test]
fn huge_many_quantum_buffer_roundtrips() {
    let len = 240 * 0x40000; // 240 quanta
    let data = vec![0u8; len];
    let comp = compress(&data, Level::Default).expect("compress huge");
    let back = decompress(&comp, data.len()).expect("decompress huge");
    assert_eq!(back.len(), data.len());
    assert!(back == data, "huge buffer roundtrip mismatch");
    std::eprintln!(
        "huge_many_quantum: {} -> {} bytes (ratio {:.6})",
        len,
        comp.len(),
        comp.len() as f64 / len as f64
    );
}

/// Fuzz: many sizes × distributions, each compressed at both levels and round-tripped through
/// the decoder. Catches LZ edge cases the curated corpus misses (tiny/odd chunk lengths,
/// single-match chunks, boundary-straddling matches near 0x20000 / 0x40000, mixed runs).
#[test]
fn fuzz_sizes_and_distributions() {
    // A small deterministic PRNG.
    let mut state = 0x9E37_79B9u32;
    let mut next = move || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        state
    };

    // Sizes clustered around chunk/quantum boundaries plus a spread of odd values.
    let mut sizes: Vec<usize> = vec![
        2, 3, 7, 8, 9, 15, 16, 31, 127, 128, 129, 130, 255, 256, 257, 1000, 4096, 0x1FFFF, 0x20000,
        0x20001, 0x20002, 0x3FFFE, 0x3FFFF, 0x40000, 0x40001, 0x40008, 0x60000, 0x60001,
    ];
    for _ in 0..8 {
        sizes.push((next() as usize % 0x50000) + 1);
    }

    for &n in &sizes {
        // Three distributions: highly repetitive, skewed, and near-random — plus a few
        // structural seeds to vary match patterns.
        let datasets: [Vec<u8>; 3] = [
            // repetitive motif with a varying period
            {
                let period = 3 + (n % 61);
                (0..n).map(|i| (i % period) as u8).collect()
            },
            // skewed two-symbol-ish
            {
                let mut s = next();
                (0..n)
                    .map(|_| {
                        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                        let r = (s >> 24) as u8;
                        if r < 220 {
                            0u8
                        } else {
                            r
                        }
                    })
                    .collect()
            },
            // near-random
            {
                let mut s = next();
                (0..n)
                    .map(|_| {
                        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                        (s >> 24) as u8
                    })
                    .collect()
            },
        ];

        for (di, data) in datasets.into_iter().enumerate() {
            for level in [Level::Fastest, Level::Default] {
                let comp = compress(&data, level)
                    .unwrap_or_else(|e| panic!("compress n={n} ds={di} {level:?}: {e:?}"));
                let back = decompress(&comp, data.len())
                    .unwrap_or_else(|e| panic!("decompress n={n} ds={di} {level:?}: {e:?}"));
                assert_eq!(back, data, "fuzz mismatch n={n} ds={di} {level:?}");
            }
        }
    }
}

/// A genuinely incompressible full quantum (0x40000 random bytes) must round-trip via the
/// stream-level uncompressed path, and a partial incompressible tail via the stored path.
#[test]
fn incompressible_full_and_partial_quanta() {
    let mut seed = 0xABCD_1234u32;
    let mut gen = |n: usize| -> Vec<u8> {
        (0..n)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 24) as u8
            })
            .collect()
    };
    // Exactly one full incompressible quantum.
    let full = gen(0x40000);
    // One full + a partial incompressible tail.
    let mut mixed = gen(0x40000);
    mixed.extend(gen(12345));

    for (name, data) in [("full_quantum", full), ("full_plus_partial", mixed)] {
        for level in [Level::Fastest, Level::Default] {
            let comp = compress(&data, level).expect("compress");
            let back = decompress(&comp, data.len()).expect("decompress");
            assert_eq!(back, data, "{name} {level:?} mismatch");
        }
    }
}
