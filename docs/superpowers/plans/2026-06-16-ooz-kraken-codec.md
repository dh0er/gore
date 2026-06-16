# ooz Kraken Codec Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the out-of-process G1R binary codec host with an in-process Oodle Kraken codec (vendored zao/ooz, built via `cc`), so reads/writes of private save payloads never need the game executable.

**Architecture:** A new isolated crate `goresave_oodle` vendors the ooz C++ codec, builds it into a static lib via a `cc` build script, and exposes two safe functions (`kraken_decompress`, `kraken_compress`) behind a clean `extern "C"` shim. `goresave_core` gets a single `OozKrakenBackend` implementing the existing `CodecBackend` trait; the binary host crate, its wiring, and the WIP `kraken.rs` are removed. The Flutter app drops the game-exe/helper settings and repoints its codec status panel to the always-available ooz self-test.

**Tech Stack:** Rust (workspace), C++17 (vendored ooz, simde), `cc` crate, Flutter/Dart FFI.

**Reference spec:** `docs/superpowers/specs/2026-06-16-ooz-kraken-codec-design.md`

**Validated facts (do not re-derive):**
- ooz decode is bit-compatible with real Oodle; ooz Kraken encode of the calibration input is byte-identical to the real-Oodle block.
- ooz Kraken encoder **crashes at level >= 6**; levels -4..5 are safe (level 5 = best safe ratio). Clamp to 5.
- Codec source set (11 `.cpp`): `bitknit compr_entropy compr_kraken compr_leviathan compr_match_finder compr_mermaid compr_multiarray compr_tans compress kraken lzna`.
- `main` in `kraken.cpp` is guarded by `#if !OOZ_BUILD_DLL` → define `OOZ_BUILD_DLL=1` to drop it. Do NOT define `OOZ_DYNAMIC` (keeps symbols non-dllexport for static linking).
- Entry points: `int Kraken_Decompress(const byte* src, size_t src_len, byte* dst, size_t dst_len)` (kraken.cpp, not header-declared) and `int CompressBlock(int codec_id, uint8* src, uint8* dst, int src_size, int level, const CompressOptions*, uint8*, LRMCascade*)` (compress.h:102). Kraken codec id = 8.

---

## Phase A — New `goresave_oodle` crate (isolated, no core changes)

### Task A1: Vendor ooz sources

**Files:**
- Create: `crates/goresave_oodle/vendor/ooz/` (copied tree)

- [ ] **Step 1: Copy the codec sources + headers + simde into the crate**

The validated ooz checkout is at `work/reversing/ooz`. Copy only the codec files (no bun/ggpk/poe/validate/libsodium parts).

```bash
mkdir -p crates/goresave_oodle/vendor/ooz
cd work/reversing/ooz
cp bitknit.cpp compr_entropy.cpp compr_kraken.cpp compr_leviathan.cpp \
   compr_match_finder.cpp compr_mermaid.cpp compr_multiarray.cpp compr_tans.cpp \
   compress.cpp kraken.cpp lzna.cpp \
   ../../../crates/goresave_oodle/vendor/ooz/
cp bits_rev_table.h compr_entropy.h compr_kraken.h compr_leviathan.h \
   compr_match_finder.h compr_mermaid.h compr_util.h compress.h \
   log_lookup.h match_hasher.h qsort.h targetver.h \
   ../../../crates/goresave_oodle/vendor/ooz/
cp -r simde ../../../crates/goresave_oodle/vendor/ooz/simde
cd ../../..
```

- [ ] **Step 2: Add the upstream license/attribution**

```bash
cp work/reversing/ooz/README.md crates/goresave_oodle/vendor/ooz/README.md
```

Create `crates/goresave_oodle/vendor/ooz/UPSTREAM.md`:

```markdown
# Vendored from zao/ooz

Source: https://github.com/zao/ooz (commit pinned at vendor time)
Only the codec sources are vendored (Kraken/Mermaid/Selkie/Leviathan/LZNA/Bitknit);
the PoE bundle / validation tooling is excluded.

License cleared for vendoring by the project owner.
```

- [ ] **Step 3: Commit**

```bash
git add crates/goresave_oodle/vendor
git commit -m "vendor: add zao/ooz codec sources for goresave_oodle"
```

---

### Task A2: C ABI shim

**Files:**
- Create: `crates/goresave_oodle/csrc/ooz_shim.cpp`

- [ ] **Step 1: Write the shim**

This exposes a stable `extern "C"` ABI so Rust never depends on C++ name mangling or ooz's incomplete export surface. `Kraken_Decompress` is forward-declared with `unsigned char` (matches the `byte`==`unsigned char` definition's mangling); `CompressBlock` comes from `compress.h`.

```cpp
// Clean C ABI over the vendored ooz codec. Kraken only on the encode side;
// decode dispatches every Oodle codec.
#include <cstddef>
#include <cstdint>
#include "compress.h"

// Defined in kraken.cpp, not declared in any header.
int Kraken_Decompress(const unsigned char* src, size_t src_len,
                      unsigned char* dst, size_t dst_len);

extern "C" {

// Returns decoded byte count, or < 0 on failure.
int goresave_ooz_decompress(const unsigned char* src, size_t src_len,
                            unsigned char* dst, size_t dst_len) {
    return Kraken_Decompress(src, src_len, dst, dst_len);
}

// Kraken encode (codec id 8). Returns compressed byte count, or <= 0 on failure.
int goresave_ooz_compress_kraken(const unsigned char* src, int src_len,
                                 unsigned char* dst, int level) {
    return CompressBlock(8, const_cast<uint8*>(reinterpret_cast<const uint8*>(src)),
                         reinterpret_cast<uint8*>(dst), src_len, level,
                         nullptr, nullptr, nullptr);
}

} // extern "C"
```

- [ ] **Step 2: Commit**

```bash
git add crates/goresave_oodle/csrc/ooz_shim.cpp
git commit -m "feat(oodle): add C ABI shim over vendored ooz"
```

---

### Task A3: Crate manifest + build script

**Files:**
- Create: `crates/goresave_oodle/Cargo.toml`
- Create: `crates/goresave_oodle/build.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Write `crates/goresave_oodle/Cargo.toml`**

```toml
[package]
name = "goresave_oodle"
version = "0.1.0"
edition = "2021"

[build-dependencies]
cc = "1"
```

- [ ] **Step 2: Write `crates/goresave_oodle/build.rs`**

```rust
use std::path::Path;

fn main() {
    let vendor = Path::new("vendor/ooz");
    let sources = [
        "bitknit.cpp",
        "compr_entropy.cpp",
        "compr_kraken.cpp",
        "compr_leviathan.cpp",
        "compr_match_finder.cpp",
        "compr_mermaid.cpp",
        "compr_multiarray.cpp",
        "compr_tans.cpp",
        "compress.cpp",
        "kraken.cpp",
        "lzna.cpp",
    ];

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .define("OOZ_BUILD_DLL", "1") // drops the CLI main() in kraken.cpp
        .include(vendor)
        .include(vendor.join("simde"))
        .file("csrc/ooz_shim.cpp");
    for src in sources {
        build.file(vendor.join(src));
    }
    // ooz triggers many narrowing/shift warnings; they are upstream noise.
    if build.get_compiler().is_like_msvc() {
        build.flag("/wd4267").flag("/wd4334").flag("/wd4244");
    } else {
        build.flag_if_supported("-w");
    }
    build.compile("goresave_ooz");

    println!("cargo:rerun-if-changed=csrc/ooz_shim.cpp");
    println!("cargo:rerun-if-changed=vendor/ooz");
}
```

- [ ] **Step 3: Register the crate in the workspace**

In the root `Cargo.toml`, add `crates/goresave_oodle` to `members` (keep `goresave_g1r_codec_host` for now; it is removed in Phase D):

```toml
members = ["crates/goresave_core", "crates/goresave_g1r_codec_host", "crates/goresave_oodle"]
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p goresave_oodle`
Expected: builds (cc compiles the C++ into `libgoresave_ooz`), no errors.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_oodle/Cargo.toml crates/goresave_oodle/build.rs Cargo.toml
git commit -m "feat(oodle): cc build of vendored ooz codec"
```

---

### Task A4: Safe Rust API (decompress)

**Files:**
- Create: `crates/goresave_oodle/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Put this in `crates/goresave_oodle/src/lib.rs` (it references items defined in Step 3):

```rust
//! Safe in-process Oodle codec backed by the vendored ooz C++ sources.

use std::fmt;

unsafe extern "C" {
    fn goresave_ooz_decompress(
        src: *const u8,
        src_len: usize,
        dst: *mut u8,
        dst_len: usize,
    ) -> i32;
    fn goresave_ooz_compress_kraken(
        src: *const u8,
        src_len: i32,
        dst: *mut u8,
        level: i32,
    ) -> i32;
}

/// ooz writes a little past the logical end during decode; mirror the upstream
/// CLI's SAFE_SPACE padding so the decode buffer can never overrun.
const DECODE_SAFE_PADDING: usize = 64;

/// ooz's Kraken encoder crashes at levels >= 6; cap to the best safe level.
pub const MAX_SAFE_COMPRESS_LEVEL: u8 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OodleError {
    Decompress { expected: usize, got: i32 },
    Compress { got: i32 },
    InputTooLarge(usize),
}

impl fmt::Display for OodleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OodleError::Decompress { expected, got } => {
                write!(f, "oodle decompress returned {got}, expected {expected} bytes")
            }
            OodleError::Compress { got } => write!(f, "oodle compress returned {got}"),
            OodleError::InputTooLarge(n) => write!(f, "oodle input too large: {n} bytes"),
        }
    }
}

impl std::error::Error for OodleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kraken_roundtrip_recovers_input() {
        let input: Vec<u8> = (0..8192u32).map(|i| (i.wrapping_mul(31) >> 3) as u8).collect();
        let comp = kraken_compress(&input, 5).unwrap();
        assert!(comp.len() < input.len());
        let back = kraken_decompress(&comp, input.len()).unwrap();
        assert_eq!(back, input);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_oodle kraken_roundtrip_recovers_input`
Expected: FAIL — `kraken_compress`/`kraken_decompress` not found.

- [ ] **Step 3: Implement `kraken_decompress` and `kraken_compress`**

Add to `crates/goresave_oodle/src/lib.rs` (above the tests module):

```rust
/// Decode an Oodle block whose decompressed length is exactly `expected_size`.
pub fn kraken_decompress(src: &[u8], expected_size: usize) -> Result<Vec<u8>, OodleError> {
    let mut dst = vec![0u8; expected_size + DECODE_SAFE_PADDING];
    let got = unsafe {
        goresave_ooz_decompress(src.as_ptr(), src.len(), dst.as_mut_ptr(), expected_size)
    };
    if got < 0 || got as usize != expected_size {
        return Err(OodleError::Decompress { expected: expected_size, got });
    }
    dst.truncate(expected_size);
    Ok(dst)
}

/// Kraken-encode `src`. `level` is clamped to [`MAX_SAFE_COMPRESS_LEVEL`].
pub fn kraken_compress(src: &[u8], level: u8) -> Result<Vec<u8>, OodleError> {
    let src_len = i32::try_from(src.len()).map_err(|_| OodleError::InputTooLarge(src.len()))?;
    let capacity = src.len() + 0x10000; // worst-case expansion headroom
    let mut dst = vec![0u8; capacity];
    let level = level.min(MAX_SAFE_COMPRESS_LEVEL) as i32;
    let got = unsafe {
        goresave_ooz_compress_kraken(src.as_ptr(), src_len, dst.as_mut_ptr(), level)
    };
    if got <= 0 || got as usize > capacity {
        return Err(OodleError::Compress { got });
    }
    dst.truncate(got as usize);
    Ok(dst)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_oodle kraken_roundtrip_recovers_input`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_oodle/src/lib.rs
git commit -m "feat(oodle): safe kraken_decompress/kraken_compress over ooz"
```

---

### Task A5: Level clamp + real-Oodle decode regression tests

**Files:**
- Modify: `crates/goresave_oodle/src/lib.rs` (tests)
- Create: `crates/goresave_oodle/tests/calibration.rs`

- [ ] **Step 1: Add the level-clamp unit test**

Add to the `tests` module in `crates/goresave_oodle/src/lib.rs`:

```rust
    #[test]
    fn compress_clamps_unsafe_level_instead_of_crashing() {
        let input: Vec<u8> = (0..4096u32).map(|i| (i * 7) as u8).collect();
        // Level 6 would crash the raw ooz encoder; the wrapper must clamp to 5.
        let comp = kraken_compress(&input, 6).unwrap();
        let back = kraken_decompress(&comp, input.len()).unwrap();
        assert_eq!(back, input);
    }

    #[test]
    fn decompress_rejects_wrong_expected_size() {
        let input = vec![7u8; 4096];
        let comp = kraken_compress(&input, 5).unwrap();
        let err = kraken_decompress(&comp, 1000).unwrap_err();
        assert!(matches!(err, OodleError::Decompress { .. }));
    }
```

- [ ] **Step 2: Run to verify pass**

Run: `cargo test -p goresave_oodle`
Expected: PASS (no crash at "level 6").

- [ ] **Step 3: Write the real-Oodle decode regression test**

This locks in decode compatibility with real Oodle using a known real-Oodle Kraken block. Create `crates/goresave_oodle/tests/calibration.rs`:

```rust
//! Locks ooz decode compatibility against a real-Oodle-produced Kraken block.
use goresave_oodle::kraken_decompress;
use sha1::{Digest, Sha1};

// A real-Oodle Kraken block (3622 bytes) decoding to 4096 bytes with the sha1
// below. Sourced from the former codec host's calibration sample.
const REAL_OODLE_BLOCK_B64: &str = include_str!("calibration_block.b64");
const EXPECTED_SHA1: &str = "ac98ade89e3d7417584bc0aa8036a56d31d4e285";
const EXPECTED_SIZE: usize = 4096;

#[test]
fn decodes_real_oodle_block_to_expected_sha1() {
    use base64::Engine;
    let b64 = REAL_OODLE_BLOCK_B64.trim();
    let comp = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
    let out = kraken_decompress(&comp, EXPECTED_SIZE).unwrap();
    assert_eq!(out.len(), EXPECTED_SIZE);
    let sha1 = hex::encode(Sha1::digest(&out));
    assert_eq!(sha1, EXPECTED_SHA1);
}
```

- [ ] **Step 4: Create the test fixture from the existing calibration constant**

Extract the `CALIBRATION_SAMPLE_COMPRESSED_B64` literal currently in
`crates/goresave_g1r_codec_host/src/lib.rs` into the fixture file:

```bash
python - <<'PY'
import re
lib = open(r"crates/goresave_g1r_codec_host/src/lib.rs", encoding="utf-8", errors="replace").read()
b64 = re.search(r'CALIBRATION_SAMPLE_COMPRESSED_B64: &str = "([^"]+)"', lib).group(1)
open(r"crates/goresave_oodle/tests/calibration_block.b64", "w").write(b64 + "\n")
print("wrote", len(b64), "b64 chars")
PY
```

- [ ] **Step 5: Add dev-deps for the test**

In `crates/goresave_oodle/Cargo.toml`:

```toml
[dev-dependencies]
base64 = "0.22"
sha1 = "0.10"
hex = "0.4"
```

(Match the versions already used in the workspace; check `crates/goresave_core/Cargo.toml` and reuse its exact versions if they differ.)

- [ ] **Step 6: Run the regression test**

Run: `cargo test -p goresave_oodle decodes_real_oodle_block_to_expected_sha1`
Expected: PASS (out is 4096 bytes, sha1 `ac98ade8…`).

- [ ] **Step 7: Commit**

```bash
git add crates/goresave_oodle
git commit -m "test(oodle): level clamp + real-Oodle decode sha1 regression"
```

---

## Phase B — `OozKrakenBackend` in core (additive)

### Task B1: Add `OozKrakenBackend`

**Files:**
- Modify: `crates/goresave_core/Cargo.toml` (add `goresave_oodle` dep)
- Modify: `crates/goresave_core/src/codec_backend.rs`

- [ ] **Step 1: Add the dependency**

In `crates/goresave_core/Cargo.toml` under `[dependencies]`:

```toml
goresave_oodle = { path = "../goresave_oodle" }
```

- [ ] **Step 2: Write the failing test**

Add to the `tests` module at the bottom of `crates/goresave_core/src/codec_backend.rs`:

```rust
    #[test]
    fn ooz_backend_roundtrips_and_reports_available() {
        let backend = OozKrakenBackend::default();

        let input: Vec<u8> = (0..4096u32).map(|i| (i * 5) as u8).collect();
        let comp = backend.compress(&input, 6).unwrap(); // level clamped internally
        let back = backend.decompress(&comp, input.len()).unwrap();
        assert_eq!(back, input);

        let probe = backend.probe().unwrap();
        assert_eq!(probe.backend, "ooz_kraken");
        assert!(probe.available);
        assert!(probe.can_decompress);
        assert!(probe.can_compress);
    }
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p goresave_core ooz_backend_roundtrips_and_reports_available`
Expected: FAIL — `OozKrakenBackend` not found.

- [ ] **Step 4: Implement `OozKrakenBackend`**

Add to `crates/goresave_core/src/codec_backend.rs` (after the trait, before/near the other backends). It uses the embedded calibration sample for `probe()`; copy the calibration constants into a new `crate::codec_calibration` module in Step 5.

```rust
use crate::codec_calibration;

/// In-process Oodle Kraken codec backed by the vendored ooz sources. Always
/// available; needs no game executable.
#[derive(Debug, Clone, Copy, Default)]
pub struct OozKrakenBackend;

impl OozKrakenBackend {
    fn self_test(&self) -> (bool, bool, String) {
        // Decode a real-Oodle block and verify the sha1.
        let can_decompress = codec_calibration::decode_self_test();
        // Compress a deterministic buffer at the safe level, then decode it back.
        let can_compress = can_decompress && codec_calibration::compress_roundtrip_self_test();
        let status = match (can_decompress, can_compress) {
            (true, true) => "ready",
            (true, false) => "decode_only",
            _ => "unavailable",
        };
        (can_decompress, can_compress, status.to_string())
    }
}

impl CodecBackend for OozKrakenBackend {
    fn probe(&self) -> Result<CodecBackendProbe, CoreError> {
        let (can_decompress, can_compress, status) = self.self_test();
        Ok(CodecBackendProbe {
            backend: "ooz_kraken".to_string(),
            available: can_decompress,
            can_decompress,
            can_compress,
            status,
            profile: None,
            resolution_mode: None,
            details: json!({ "adapter": "ooz_kraken" }),
        })
    }

    fn decompress(&self, input: &[u8], expected_size: usize) -> Result<Vec<u8>, CoreError> {
        goresave_oodle::kraken_decompress(input, expected_size)
            .map_err(|e| CoreError::Codec(e.to_string()))
    }

    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, CoreError> {
        goresave_oodle::kraken_compress(input, level).map_err(|e| CoreError::Codec(e.to_string()))
    }
}
```

- [ ] **Step 5: Create the calibration module**

Create `crates/goresave_core/src/codec_calibration.rs`:

```rust
//! Embedded codec self-test vectors. The compressed sample is a real-Oodle
//! Kraken block; decoding it (and a compress->decode round-trip) proves the
//! in-process codec works without any game install.
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sha1::{Digest, Sha1};

const SAMPLE_B64: &str = include_str!("codec_calibration_block.b64");
const SAMPLE_SHA1: &str = "ac98ade89e3d7417584bc0aa8036a56d31d4e285";
const SAMPLE_SIZE: usize = 4096;

/// Deterministic 4 KiB buffer used for the compress round-trip self-test.
pub fn compress_input() -> Vec<u8> {
    (0..SAMPLE_SIZE as u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect()
}

pub fn decode_self_test() -> bool {
    let Ok(comp) = BASE64.decode(SAMPLE_B64.trim()) else { return false; };
    let Ok(out) = goresave_oodle::kraken_decompress(&comp, SAMPLE_SIZE) else { return false; };
    out.len() == SAMPLE_SIZE && hex::encode(Sha1::digest(&out)) == SAMPLE_SHA1
}

pub fn compress_roundtrip_self_test() -> bool {
    let input = compress_input();
    let Ok(comp) = goresave_oodle::kraken_compress(&input, 5) else { return false; };
    let Ok(back) = goresave_oodle::kraken_decompress(&comp, input.len()) else { return false; };
    back == input
}
```

Register it in `crates/goresave_core/src/lib.rs` near the other `mod` declarations:

```rust
mod codec_calibration;
```

Create the fixture (same source as Task A5 Step 4):

```bash
python - <<'PY'
import re
lib = open(r"crates/goresave_g1r_codec_host/src/lib.rs", encoding="utf-8", errors="replace").read()
b64 = re.search(r'CALIBRATION_SAMPLE_COMPRESSED_B64: &str = "([^"]+)"', lib).group(1)
open(r"crates/goresave_core/src/codec_calibration_block.b64", "w").write(b64 + "\n")
print("wrote", len(b64), "b64 chars")
PY
```

Ensure `crates/goresave_core/Cargo.toml` has `base64`, `sha1`, `hex` deps (they are already used elsewhere in core — confirm with `grep -E 'base64|sha1|hex' crates/goresave_core/Cargo.toml`).

- [ ] **Step 6: Run to verify pass**

Run: `cargo test -p goresave_core ooz_backend_roundtrips_and_reports_available`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/goresave_core
git commit -m "feat(core): add OozKrakenBackend with embedded codec self-test"
```

---

## Phase C — Switch core to ooz, remove host backend + WIP kraken.rs

### Task C1: Make the backend always-on; drop the `binaryHost` payload path

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

- [ ] **Step 1: Replace each `payload.get("binaryHost")` backend construction**

There are several blocks of this shape (around lines 376, 401, 418, 429, 490):

```rust
let codec_backend = payload
    .get("binaryHost")
    .map(ensured_binary_host_from_config)
    .transpose()?;
let codec_backend = codec_backend
    .as_ref()
    .map(|backend| backend as &dyn codec_backend::CodecBackend);
```

Replace every occurrence with the always-on backend:

```rust
let ooz_backend = codec_backend::OozKrakenBackend::default();
let codec_backend = Some(&ooz_backend as &dyn codec_backend::CodecBackend);
```

- [ ] **Step 2: Remove `ensured_binary_host_from_config`**

Delete the `ensured_binary_host_from_config` function and any now-unused imports it pulled in. (Find it: `grep -n 'fn ensured_binary_host_from_config' crates/goresave_core/src/lib.rs`.)

- [ ] **Step 3: Build to find remaining references**

Run: `cargo build -p goresave_core`
Expected: errors only for the codec-status function and the `kraken::` fallback (handled in C2/C3). Fix any other stragglers (unused imports) inline.

- [ ] **Step 4: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "refactor(core): always use in-process codec, drop binaryHost config"
```

---

### Task C2: Rewrite codec status for the single backend

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`

- [ ] **Step 1: Find the codec-status command and `codec_status_from_probes`**

Run: `grep -n 'codec_status_from_probes\|"codec_status"\|fn .*codec_status' crates/goresave_core/src/lib.rs`

- [ ] **Step 2: Replace the two-probe status with a single ooz probe**

Replace the body that combined `pure_probe` + `binary_probe` so it returns the `OozKrakenBackend` probe directly. Example shape (adapt to the existing command handler and its JSON keys):

```rust
fn codec_status() -> Result<Value, CoreError> {
    let probe = codec_backend::OozKrakenBackend::default().probe()?;
    Ok(json!({
        "backend": probe.backend,
        "available": probe.available,
        "canDecompress": probe.can_decompress,
        "canCompress": probe.can_compress,
        "status": probe.status,
        "details": probe.details,
    }))
}
```

Delete `codec_status_from_probes` and update its callers to call `codec_status()`.

- [ ] **Step 3: Update/remove the old codec-status tests**

Delete `codec_status_prefers_configured_binary_host_when_available` and
`codec_status_unsupported_binary_build_shows_plain_message` (binary-host
specific). Add a replacement test:

```rust
    #[test]
    fn codec_status_reports_ooz_backend_ready() {
        let value = codec_status().unwrap();
        assert_eq!(value["backend"], "ooz_kraken");
        assert_eq!(value["available"], true);
        assert_eq!(value["canDecompress"], true);
        assert_eq!(value["canCompress"], true);
    }
```

- [ ] **Step 4: Run**

Run: `cargo test -p goresave_core codec_status`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "refactor(core): codec status reports the single ooz backend"
```

---

### Task C3: Remove old backends; inline the inspect fallback; drop `kraken.rs`

**Files:**
- Modify: `crates/goresave_core/src/codec_backend.rs`
- Modify: `crates/goresave_core/src/lib.rs`
- Move: `crates/goresave_core/src/kraken.rs` → `work/kraken.rs.wip`

- [ ] **Step 1: Replace the `kraken::inspect_private_payload` fallback**

At `crates/goresave_core/src/lib.rs` (~line 2771), replace:

```rust
let Some(backend) = codec_backend else {
    return kraken::inspect_private_payload(data, stream);
};
```

with an inline metadata stub (no decode, no `kraken` dependency):

```rust
let Some(backend) = codec_backend else {
    return Ok(json!({
        "status": "no_codec_backend",
        "message": "No codec backend was supplied for private payload inspection.",
        "method": stream.method,
        "algorithmId": stream.algorithm_id,
        "chunkCount": stream.chunk_count,
        "compressedSize": stream.summary_compressed_size,
        "uncompressedSize": stream.summary_uncompressed_size,
        "writable": [],
    }));
};
```

(Confirm the `CompressedStream` field names against the struct definition: `grep -n 'struct CompressedStream' crates/goresave_core/src/lib.rs` then check field names `method`, `algorithm_id`, `chunk_count`, `summary_compressed_size`, `summary_uncompressed_size`.)

- [ ] **Step 2: Remove `PureRustKrakenBackend` and `G1rBinaryHostBackend`**

In `crates/goresave_core/src/codec_backend.rs` delete: `PureRustKrakenBackend` (+ its impl + the `use crate::{CoreError, kraken};` → change to `use crate::CoreError;`), `G1rBinaryHostBackend` (+ impl + `calibrate` + `probe_from_response` + `request`), `CodecHostInvoker`, `ProcessCodecHostInvoker`, `DispatchingCodecHostInvoker`, `hide_console_window`, `invoke_codec_host_stdio`, `parse_codec_host_response`, `decode_output_base64`, `decode_outputs_base64`, and all their tests (`binary_host_*`, `codec_host_error_response_*`, `pure_rust_backend_reports_current_status`). Keep the `CodecBackend` trait, `CodecDecodeChunk`, `CodecEncodeChunk`, `CodecBackendProbe`, and `OozKrakenBackend`.

- [ ] **Step 3: Drop the `kraken` module and move the file**

In `crates/goresave_core/src/lib.rs` remove the line `mod kraken;`.

```bash
git mv crates/goresave_core/src/kraken.rs work/kraken.rs.wip
```

- [ ] **Step 4: Build the whole core**

Run: `cargo build -p goresave_core`
Expected: builds. Fix any remaining unused-import warnings.

- [ ] **Step 5: Run the full core test suite**

Run: `cargo test -p goresave_core`
Expected: PASS. (Some tests that constructed a binary-host backend may need to switch to `OozKrakenBackend::default()` — update them inline; do not delete coverage of real decode/edit paths.)

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(core): remove host/pure-rust backends, drop WIP kraken.rs"
```

---

### Task C4: Repoint core examples

**Files:**
- Modify: `crates/goresave_core/examples/dump_typed.rs`
- Modify: `crates/goresave_core/examples/try_add_item.rs`
- Modify: `crates/goresave_core/examples/try_typed_edit.rs`

- [ ] **Step 1: Replace binary-host construction in each example**

Each example builds a `G1rBinaryHostBackend` (find with `grep -rn 'G1rBinaryHostBackend\|binaryHost\|codec_host' crates/goresave_core/examples`). Replace with:

```rust
let backend = goresave_core::codec_backend::OozKrakenBackend::default();
```

and drop any CLI args / paths for the helper exe and game exe that are now unused.

- [ ] **Step 2: Build examples**

Run: `cargo build -p goresave_core --examples`
Expected: builds.

- [ ] **Step 3: Commit**

```bash
git add crates/goresave_core/examples
git commit -m "refactor(core): examples use OozKrakenBackend"
```

---

## Phase D — Delete the codec host crate

### Task D1: Remove the crate and fixtures

**Files:**
- Delete: `crates/goresave_g1r_codec_host/`
- Modify: root `Cargo.toml`

- [ ] **Step 1: Remove the workspace member**

In the root `Cargo.toml`:

```toml
members = ["crates/goresave_core", "crates/goresave_oodle"]
```

- [ ] **Step 2: Delete the crate**

```bash
git rm -r crates/goresave_g1r_codec_host
```

- [ ] **Step 3: Remove any helper-exe packaging**

Search the Flutter/native build for references that copy the helper exe:

```bash
grep -rn 'goresave_g1r_codec_host\|g1r_codec_host' apps/ --include='*.cmake' --include='CMakeLists.txt' --include='*.dart' --include='*.yaml' --include='*.ps1' --include='*.sh'
```

Remove each packaging/copy step found (e.g. CMake `add_custom_command`/install rules and any Dart asset reference).

- [ ] **Step 4: Build the workspace**

Run: `cargo build --workspace`
Expected: builds with two crates only.

- [ ] **Step 5: Run the workspace tests**

Run: `cargo test --workspace`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: remove the G1R binary codec host crate"
```

---

## Phase E — Flutter: drop exe/helper config, repoint codec status

### Task E1: Remove exe/helper settings + binaryHost payloads

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/editor_settings_store.dart`
- Modify: `apps/goresave/lib/features/editor/domain/editor_notifier.dart`
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart`
- Modify: `apps/goresave/lib/utils/default_paths.dart`

- [ ] **Step 1: Find every codec-host reference in the app**

Run: `grep -rn 'binaryHost\|codecHost\|codec_host\|g1r_codec_host\|gameExe\|helperPath' apps/goresave/lib`

- [ ] **Step 2: Remove the game-exe and helper-path settings**

In `editor_settings_store.dart` delete the stored keys/fields for the game exe path and codec-host helper path (and their getters/setters/defaults).

- [ ] **Step 3: Stop sending `binaryHost` in FFI payloads**

In `editor_notifier.dart` remove the code that adds `binaryHost` (and exe/helper paths) to the JSON payloads sent through the FFI bridge. Inspect/search/edit calls now send no codec config.

- [ ] **Step 4: Remove the settings UI**

In `editor_page.dart` remove the game-exe / helper-path pickers and any "configure codec host" UI. In `default_paths.dart` remove the helper-exe default path logic.

- [ ] **Step 5: Update the affected Dart tests**

Update `editor_settings_store_test.dart`, `editor_notifier_test.dart`, and
`default_paths_test.dart` to drop the removed keys/paths. Remove assertions about
`binaryHost` payloads.

- [ ] **Step 6: Analyze + test**

Run: `cd apps/goresave && flutter analyze`
Expected: no errors.
Run: `cd apps/goresave && flutter test test/editor_notifier_test.dart test/editor_settings_store_test.dart test/default_paths_test.dart`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/goresave
git commit -m "feat(app): remove game-exe/codec-host settings; codec is in-process"
```

---

### Task E2: Repoint the codec status panel

**Files:**
- Modify: codec status panel widget (find with grep)
- Modify: `apps/goresave/test/codec_status_panel_test.dart`

- [ ] **Step 1: Find the panel and its model**

Run: `grep -rln 'CodecStatus\|codecStatus\|codec_status' apps/goresave/lib`

- [ ] **Step 2: Simplify the panel to the ooz status**

The panel previously distinguished pure-rust vs binary-host, "configure game exe", profiles, resolution modes. Replace with the single in-process status from the `codec_status` command: show `available` / `canDecompress` / `canCompress` / `status` (`ready`/`decode_only`/`unavailable`). Remove "select game executable" calls to action.

- [ ] **Step 3: Update the panel test**

Rewrite `codec_status_panel_test.dart` to assert the simplified states (e.g. a "ready" status renders the ready indicator; no game-exe configuration prompt is shown).

- [ ] **Step 4: Analyze + test**

Run: `cd apps/goresave && flutter analyze`
Expected: no errors.
Run: `cd apps/goresave && flutter test test/codec_status_panel_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/goresave
git commit -m "feat(app): codec status panel shows in-process ooz status"
```

---

### Task E3: Full verification

- [ ] **Step 1: Workspace build + test**

Run: `cargo build --workspace && cargo test --workspace`
Expected: PASS.

- [ ] **Step 2: Flutter analyze + test**

Run: `cd apps/goresave && flutter analyze && flutter test`
Expected: PASS.

- [ ] **Step 3: Manual real-save smoke check (if a save is available)**

Use an example to decode + re-encode a real save's private payload and confirm
no errors and a byte-stable round-trip:

Run: `cargo run -p goresave_core --example dump_typed -- <path-to-save>`
Expected: typed properties dump without codec errors.

- [ ] **Step 4: Final commit (if any cleanup)**

```bash
git add -A
git commit -m "chore: finalize ooz codec migration"
```

---

## Self-review notes

- **Spec coverage:** new crate + cc build (A1–A3), safe API + clamp (A4–A5), OozKrakenBackend + self-test validation (B1), always-on backend + removed binaryHost (C1), single-backend status (C2), removed host/pure-rust backends + kraken.rs move (C3), examples (C4), host crate deletion (D1), Flutter settings + status (E1–E2), verification (E3). All spec sections mapped.
- **Level clamp** is enforced in `kraken_compress` (A4) and exercised in A5/B1.
- **Validation requirement** satisfied by `codec_calibration` self-test feeding `OozKrakenBackend::probe()` and the codec status (B1/C2/E2).
- **kraken.rs** has exactly two external callers, both removed in C2/C3 before the move.
- **Naming consistency:** `goresave_ooz_decompress`/`goresave_ooz_compress_kraken` (shim) ↔ `kraken_decompress`/`kraken_compress` (Rust) ↔ `OozKrakenBackend` (core) used consistently.
