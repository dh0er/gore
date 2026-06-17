# Design: ooz Kraken codec replaces the binary codec host

Date: 2026-06-16
Branch: `feat/ooz-kraken-codec`

## Goal

Replace the out-of-process G1R binary codec host with an in-process Oodle Kraken
codec, so that reading and writing private save payloads never depends on a
locally installed `G1R-Win64-Shipping.exe`. The codec host fails for many users
because it must locate and call Oodle functions inside an ever-changing game
executable (version drift, missing sibling DLLs, anti-cheat, etc.). An
in-process codec removes that entire failure class.

The codec is provided by [zao/ooz](https://github.com/zao/ooz), an open-source
reimplementation of the Oodle codecs. License has been cleared by the project
owner for vendoring.

### Validated facts (local testing, June 2026)

- ooz builds standalone (no Oodle DLL) and round-trips Kraken/Mermaid/Selkie/
  Leviathan.
- **Decode-compatible with real Oodle:** ooz decodes the codec host's real-Oodle
  calibration block to the exact expected sha1 `ac98ade8…`.
- **Encode-compatible with real Oodle:** ooz Kraken output for the calibration
  input is **byte-identical** to the real-Oodle block, so the game decodes it.
- **ooz Kraken encoder crashes at compression level >= 6** (levels 6/7/8/9 →
  0xC0000005). Levels -4..5 work; level 5 gives the best safe ratio.

## Scope

- Kraken only (G1R uses compressor id 8 = Kraken). Decode + encode.
- Decode dispatches all Oodle codecs anyway (`Kraken_Decompress`), so reads of
  any codec a save might contain keep working; only encode is Kraken-specific.

## Non-goals

- No pure-Rust port of the codec (decided: vendor C++, build via `cc`).
- No Mermaid/Selkie/Leviathan/LZNA/Bitknit encode API surface.
- No change to the save container/format logic, the typed property system, or
  the private-edit system in `goresave_core` (all operate on the decoded payload
  and are codec-agnostic).

## Architecture

### New crate: `crates/goresave_oodle`

- Vendors the ooz codec sources under `crates/goresave_oodle/vendor/ooz/`
  (codec `.cpp`/`.h` + the `simde` headers it needs; not the PoE/bun/validate
  parts, which pull libsodium/libpoe).
- `build.rs` compiles the vendored sources into a static library with the `cc`
  crate (`cc::Build`, C++17, `OOZ_DYNAMIC=0`). simde keeps it portable across
  MSVC/clang/gcc. The end-user binary links it statically — no runtime C++ dep.
- Exposes a small, safe Rust API and nothing else:
  ```rust
  pub fn kraken_decompress(src: &[u8], expected_size: usize) -> Result<Vec<u8>, OodleError>;
  pub fn kraken_compress(src: &[u8], level: u8) -> Result<Vec<u8>, OodleError>;
  pub const MAX_SAFE_COMPRESS_LEVEL: u8 = 5;
  ```
  - `kraken_decompress` calls `extern "C" Kraken_Decompress(src, src_len, dst, dst_len)`
    into a `dst` buffer of `expected_size` (+ ooz's safety padding). Errors if the
    return value != `expected_size`.
  - `kraken_compress` clamps `level` to `MAX_SAFE_COMPRESS_LEVEL`, allocates a
    worst-case output buffer, calls `extern "C" CompressBlock(kKraken, ...)`, and
    truncates to the returned size. Errors on non-positive return.
  - All `unsafe` FFI is confined to this crate behind these two functions.

### `goresave_core::codec_backend`

- Remove `PureRustKrakenBackend` (stub) and `G1rBinaryHostBackend` (+ the stdio
  invoker, `ProcessCodecHostInvoker`, response parsing, base64 plumbing, and all
  host-specific tests).
- Add `OozKrakenBackend` implementing the existing `CodecBackend` trait via
  `goresave_oodle`. The trait (`probe`/`decompress`/`compress` + `_many`) is
  unchanged, so all `goresave_core` call sites that take
  `&dyn CodecBackend` keep working.
- `OozKrakenBackend::probe()` runs an **in-process self-test** (see Validation)
  and returns a `CodecBackendProbe { backend: "ooz_kraken", available, can_decompress,
  can_compress, status, details }`. No exe/helper path, no profile, no
  resolution mode.

### `goresave_core::lib.rs`

- The codec backend is now always available. Construct `OozKrakenBackend`
  unconditionally where a backend is needed instead of building it from the
  `binaryHost` FFI payload field.
- Remove `ensured_binary_host_from_config` and the `binaryHost` payload parsing.
- `inspect_private_payload`'s `None`-backend fallback currently calls
  `kraken::inspect_private_payload`. Replace that call with an inline metadata
  `json!` stub (the same 12-line no-decode response) so the `kraken` module can
  be removed. (In production the backend is always `Some`, so this branch is only
  hit by default/test wrappers that pass `None`.)
- Rewrite `codec_status_from_probes` (which combined a pure-Rust probe and a
  binary-host probe) into a single ooz probe → codec status.

### Remove the codec host

- Delete `crates/goresave_g1r_codec_host/` and its workspace member entry in the
  root `Cargo.toml`.
- Remove any build/packaging step that bundles the helper exe (CMake/Flutter).

### Move WIP pure-Rust Kraken out of the build

- Move `crates/goresave_core/src/kraken.rs` (WIP encoder, ~5.4k LOC) to `work/`
  alongside its reversing notes. It has only two external touchpoints, both
  removed by this change (`codec_status` via the deleted `PureRustKrakenBackend`,
  and the inlined `inspect_private_payload` fallback). Drop `mod kraken;`.

### Flutter app (`apps/goresave`)

- Remove the game-exe-path and codec-host-helper-path settings and the code that
  sends `binaryHost` in FFI payloads (`editor_settings_store.dart`,
  `editor_notifier.dart`, `editor_page.dart`, `default_paths.dart` + tests).
- Keep the codec status panel, repointed to the ooz self-check result (now
  effectively always "ready", independent of any game install).

## Data flow

Read: Flutter inspect/search → FFI `goresave_execute` → core decodes private
payload via `OozKrakenBackend::decompress` → `goresave_oodle::kraken_decompress`
→ ooz C++ → decoded bytes → typed/property browsing.

Write: edited payload → `OozKrakenBackend::compress(level<=5)` →
`goresave_oodle::kraken_compress` → ooz C++ → Kraken stream → re-wrapped into the
save container by existing core logic.

## Error handling

- FFI failures (non-matching output size, non-positive compress result) become
  `OodleError`, mapped to `CoreError::Codec` at the backend boundary — same error
  type the rest of core already handles.
- ooz is "not fuzz safe", but malformed input is contained: decode runs into a
  fixed `expected_size` buffer with ooz's safety padding and the result size is
  checked. (Loss of process isolation vs. the old out-of-process host is
  accepted; the host is being removed precisely because that isolation cost more
  reliability than it bought.)
- Compress level is clamped to `MAX_SAFE_COMPRESS_LEVEL = 5` to avoid the ooz
  encoder crash at level >= 6.

## Validation (runtime self-test)

An embedded calibration sample (reuse the codec host's
`CALIBRATION_SAMPLE_COMPRESSED_B64` real-Oodle block, its expected sha1/size, and
a deterministic compress input) drives `OozKrakenBackend::probe()`:

1. `kraken_decompress(real_oodle_sample)` → output size == 4096 and sha1 ==
   `ac98ade8…` → `can_decompress = true`.
2. `kraken_compress(calibration_input, 5)` then `kraken_decompress` of that →
   round-trip equals the input → `can_compress = true`.

The probe result feeds the codec status the UI already shows, satisfying the
requirement that the app still validates that compress/decompress work — now as a
self-contained check that needs no game executable.

## Testing

- `goresave_oodle` unit tests:
  - Kraken self round-trip on real data (sha256 match).
  - **Decode of the real-Oodle calibration block → exact sha1** (regression
    lock on real-Oodle decode compatibility).
  - Compress level clamp (level 6 in → encodes at 5, no crash) + round-trip.
- `goresave_core` tests: repoint codec-dependent tests from the binary host to
  `OozKrakenBackend`; delete host-specific tests; update `codec_status_from_probes`
  tests for the single-backend shape.
- Flutter: update `codec_status_panel_test.dart` and the settings/default-paths
  tests for the removed exe/helper configuration.

## Risks

- `cc` build needs a C++ compiler at build time (MSVC present locally; CI/other
  platforms need clang/gcc). End-user runtime needs nothing.
- Minimal vendored source set must compile cleanly; if codec interdependencies
  make trimming fragile, vendor the full codec source set and expose only the
  Kraken API (decode still dispatches all codecs).
- ooz encoder level >= 6 crash is worked around by clamping, not fixed. A future
  task could fix the ooz high-level encoder if better ratios are ever needed.

## Out of scope / follow-ups

- Fixing ooz's level >= 6 encoder crash.
- Pure-Rust port (the moved `work/kraken.rs` remains the research seed).
