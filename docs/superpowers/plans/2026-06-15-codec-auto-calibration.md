# Codec Auto-Calibration & User-Facing Errors — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** New/unknown G1R game builds are adopted automatically and silently when the codec proves it works on the user's machine, and genuinely-unusable builds surface a plain-language error instead of techy codec jargon.

**Architecture:** The host gains a `calibrate` command that runs the existing runtime selftest against an embedded known-answer sample and, on success, writes the SHA-256-keyed derived profile cache. The core triggers `calibrate` automatically inside `check_codec` when the binary probe is `pattern_profile`, then re-probes. The core also maps codec state to user-facing title/message/hint/severity fields, which the Flutter status panel renders, with technical fields moved behind a `Details` disclosure.

**Tech Stack:** Rust (`goresave_g1r_codec_host`, `goresave_core`), Flutter/Dart (`apps/goresave`).

**Reference spec:** `docs/superpowers/specs/2026-06-15-codec-auto-calibration-design.md`

**Working branch:** `feat/codec-auto-calibration` (already created).

---

## Pre-generated calibration sample (verified 2026-06-15)

These values were produced by compressing a deterministic 4096-byte synthetic
plaintext through the live host against the real 1.0.1 build, then verifying the
compressed blob decompresses back to the same plaintext (SHA-1 matched). They
are used verbatim in Task 1. Oodle Kraken decode format is version-stable, so
this blob decompresses correctly on future builds.

- `expectedSize` = `4096`
- `expectedDecompressedSha1` = `ac98ade89e3d7417584bc0aa8036a56d31d4e285`
- `expectedDecompressedHeadHex` = `676f7265736176652d636f6465632d63616c6962726174696f6e2d73616d706c642c77302c454e2c4f4e552c524948512c46404c442c454055402c2c2c2c2c2c`
- `compressedBase64` (3622 compressed bytes) =
  `jAYADiBAP/wOHIfwBX////////////////////4DBwBWAvPpbTwdL4f780Gj0VSTmkw0oNEcxvqZfndY6/czUyZTKId0dZ1ufR+u59PZYTVdnINpRSYUiCgUw/FhOp7u58vrTaVSlfOqVDimUr2W0o108hpKpxtHLNZqJiRdiWT63E+X68Vvsp5981FZKJNLyWTr5Wu/3E+34/9HIBA0o4JAMiMQfPbilXj0mYvHK18g0OnnRG2R6D++/s/H0+R3/MyrfU+jVil4PNf75Hg/vs/f4cjn88Xbvl695PPNrs6D8zH7Op8HSasVi1YcQYfjOT0/r/fL6HF/DctNV69Qqrlc/+/u/X0fn/f1wmKxZIuWSrdhsSze9p/9tjjb7z9VpZJIt2x5m+28f57/39/i9D2sm2VDpdXoGAzP9+L7/t7/x+3OZDKl66ZSu2cylYLYCDZTSmKzEcJudzoOMFcMJluMZ4PhQC0TzlXnW9xqMhvhcOFgKR6MZ6PpegWBQBy3iMVwgkBU4ugQOlXJo9Mh2mJxuc9QZxQq3s7G08lUJ5aM9LdzyGC3WUAg2XwnmU+G09FmCwaD/cew2X4Fg/WywAQw0IsCgwnOaPT7bgBPACDYTQez+UwrkA8110PQbLHagUD1aqxcLTeL/XCAw+FMn5zP9cDhBMr8Gr8XaPP7NdDnMxifeHMerx0vtuvVWqBVbYSPb8bndLgwGMVyoFqu9uvNaIzFYo3/rNf5xmKlmtQOtZIqUqsdxO22Wj4oWwqlmuxXm+1GrtKuZf9X2unze9Fo7Waq3+xXu+V8hkAgLK+Ew/NDIGT65Ba5lKmTyy3c4bDZ/0hrEgmnoXai3ZyW2u3EkMuVigFNVaPZHufb4Xhw24x313hWl4rEQjrdeHiaD+fb6fp+USgUxawiEYwoFJe5eqReXfbq9ciWSFTqMVVZpZq/t/P1cvWZLSf/bFwSyGUSEsl2/1nul+P19PmSyWT9sCyWT8lkv61wIRz8psLhwhMK9boZQVMgGH7Xw+1+8xrsR890UBRLpHIi0f06O1/Pz+N/PPB4PNGmp1MteDyDs//m/w3e/v9N1OkEwiVf3Od7z4/v+/U2eF0f42Lb0SkVKg7H8Ty4nq//+3M6c7lc4b6rVa65XKun9WO9rI7W60dRq6WSDUvWYrku/9fn+7G7vG/bftVW6vRaNtv7ufo//9fveb8xGAzJqqHQ7BgMm7/5ZT5t7ubzS1coZPI9U9pkypej+XAwVMkFM/XpHrOYTQYYTDReCcaD6XC2WMLhcPc1bjcf4XC1KDKATNSyyGSAslrdrhPEEYFIVsPJaDxSSsRTxfEStRuMZihUPttLZ9PBZLzdgEAg3yFksl1AII00PAePNcLweI41mTzeK9gfBgv3k+F8NtcIZQPt5RgwWS02AEAy3cims/F8sNsDgUDvOWi03oFAoSK3wu2EmtxuBfD7jYYHzpTD6Qbr3WK5EOuUe9H7l/e63E48XrkYqhfr3Wo7HmEwGMMv43G8MBiROrvEbkX67HYJ9nhM5jfWmMWqp7v1drOVqTUr+e+dcvh9HhRKt59p9pvldjWZotFo+zPt9n/RaLkusUEs5KrEYgNzOu22H8KSQChm28Vuv5Mq9EvJ95F0e7x+JBIJEj/enSd9aRIkc2Yv3rd23RO2aECHx68uEzz9IW6lEvbv3LN85WjT20X6xZcuoe9emUS16Pem0SUKD75cpHtlChQTp6zYu27tc3fqxY3Th36yfngreo1CV61ZuWjNGDIYwvTZr15ALNmXWI2YFmVWLx4n7v3m/m8ePEMG7l2xesUj1qnDgcu3HnP9fidivSKWLl+1cP1Ag3oHf8+DBvFOHBxOocCnB5OgABuWDAU54AAAO/cNmj5k+VsnSwYK3ZtGwp+7rHSReidMmj5m0SBC+QFz17NmMM0aEFaRoGaElaoUM8atR7zqFChrVk8fNHDRC4YIgQTBozYjXDyiaKkKxo8dMHrpcOO7hHnDmzaRj58aQrnwx8YQpgwDpmzEuuYMGAuXzBk4aOFrR4pkz1aNapou2Wn/h1Z//fj77ccMGkhjtlylSmh8eNXdh4w+Nfdtmy361bZfLZs2Vy6+/PP/zzf++WLDTr0KW63fZee3Nr5+/vf191y7Cua8SI0ayT7938mv3J93cmOHFYo0ddgjhw4nz317/eP3Rx8758lalVr671pv859mHzx7/eHfPHqJ4LBQvXoIvfvT0Z+8XvR0aJE50oUXHaiRIn9+H7529dsfrlggRkSpIgsMneDtox7u37xy/+Nky+slO0aRIrnfPljHs+TvlrHkiQmyRJQbpIkSH7+cuHrt639v2qVMUIFyqk4Y6vmLTk+dOXjryxQqKKI1WqlSWBy5VtuLlA5VNu6dIeLl1p6unTpPHt49cPnDL86Zo0BIsRJrzR7m4bsujh4/dPP7VgJyqU0sFSuFerNhs9lQkRsMFLDZGMR72V5nUO/1shMGQyCDdvTdTm0XquVSmUElVZiB6AUGBIAsFkKxQSqW6uVKq221WpHxKxQYtlq5lFPNVOISTqWaBwzGYiIm3MlE6tRLlWqFT6KWefHQGQiDQ81maqVLr9RLtWK/ZzAYMKEDABIzGHj0Y9VY5JGPxaofAMDh40bs0cgvtvrNRpPEZ/TIqfwPg0YhfD5Wu8RoN7rNXqHo9/vB2T8enfT7yaxPw9Mh8z6dhgmLBYNSHsDHwyk1O612i8hhdwnJzBePQKK9Xn6vzu11G512teJyuWCJFwqXcbko3Hff3aYw3+2+FYWCQLNu+NvNrHea/V6fwuQ1qJnkA4XF4BwOTrfC6/ba/Uat7nQ6oeknEpt3OpGAWUgWQ0JmsZCCTmcyCjLWTAZLhGOBYAANA8ZR5dqcSiIT5XJgIAkOhGOhaDolkUgYtQmFUJJIUOBpUBpFwafRoJpCYbHLUuZUCs7GwtFIFAeGhPC18ohAp1FEIlg8B4lHgtFQJisWi/nFMZleFYvxsEFEEMCDBoGIjkjk82oCzkAAyEUDsXgMC4AHMdXCkEyh0oVCdCqMTCUziXwwoNPpSJ0dj9XQ6QDIfVqfB2D3+bSQxyMQm3ryXo8NJ7LpVBqARWWAje6Gx2SwNBpEMoBKpvLpTCis1WqJ/S2X2dZqoZhVTpWCIlapnITNplI6KtpKhYrkU5lsBo7CpmH91prJ43PVamwmis/kU7lkPKZQKCitBYPTUyhg+GVWmYShl8msnMGg0ftK6lIJJuFWstWYlFut5IDDkYiADXWz0RrlWqFYYNOIdVY4doeCwEC7nVhokgvlWqnablksFkTsAgGELBYW+Vq0Vln0a7XohkBQ6LAVebWSu7VytVLlkSklfix8AsBhEJOJVu9R6pVitdTpms1mfPAMhkfNZj7tUDEU+KRDoeIDAvG4mAFzMBB61UKtXuMS6EVONHAEQ6Bwo5HdKjNbzU6jXyz4fD5Q5odDJXw+AvPf9vcJ3H+/bcThAMCkH/z3c8uNbrvVJnBZHWIi+8EhESiPh9EssJqtfrtTKnu9XmD+i0WmvV4q59VztaiMV6tnQaOhkIwL9nKxKv1Wp9uhs7htWj71RuLwWLeb26nyO/1Wr1mvORwOSOqBwOQcDhr/2XU2aexns2tHIGDwvBP6dMKToXgwEETBATF0qT6jkEkEmQwUTgHCgWgwlkjK5XJ2dU4nF+VyNGgSkETQsEkkoKJS2ayShDGRQFLBSCgcQkLAUUSxMqUTiGSpFB7LQ2PRQCSczYhEIl5hRKJVRCIMdBwXhzHAcTiuJZE43KqYPxYD85FgPBbHAGEBbKU4IFEpNIEAEs3AorFwPJDLC4VCbnlIpNaFQiBil9LlgJhdLiXg84mEho600+EC6VwimQDjkHlQu7fnsthMvR6ZCKIT6VwqGw5pNBpCb8NhtDQaEHqb1GZB+G02KeZwSOS2lrjVoqO5dDaThaExKXivvWLweRyVCpePYfKZZDYViarVanpzzeZ31Wo4bpFRJOCoRSIjYzLptJ6CslAgYtlELp+DIvBJSLexZHO4fKUSCVK+3JkvbUlSpXFiJ86XNlwRsmBQp4cuLBc07QEuJBDy58yTPGRq19NV2oUWLKXnTrkENOrzrsEFSo68WKxrRUqVEaOkyJsubHFz4tWtkwZ8tnZoCzoMQFOlSZkoTBo2ENLUmS5cRCTJtxgMmhJtRg9epuz57u5PXr1BAuZNkTpEIdKh06GLNhx39W4HIjwgkiZPlTB8QodyF1+PAgT1Rgw8DiDCow+DgECalAgNKcBAgTnzBYoeJHhZI0MWKp0aRMaXq4w0EHgjRIoeJlAiRvERU5cyZDTFCjAWEKJijIWKVLLEqU+sylSpaVJHDxQwUAmCAJEkgSI0J1QsgigoCMKHDRA6aHLns5RZgxo0lYePOgI48sPOAIZMgqRozKrGTJkJk8QJGChgaUOCdO8WDGieJslJvwZUe/Xo261GDh5Ac5YcJEhsdGj1nQaOOj3nTdus+NG+Tw3bt1UqtuzTv041+vFy4w48CF+lz0WnNjS6du7XtXZevwL2nAgMGM027f+JLt6bf2JDx5SIMH3IA8ePJct1a92jdkUbM/fpGhRY/udKT7MeZBs0a92hXj5+gfCQEDx4DLXr85Eevlr8ZEjRuNCBHw2I0aN9eheuVrVaHapQMGYEKCAPBI3ArSIc6tesUr9iZs/jNRsGECC91y54hzLm656hxMmIsECcC4TJkx27lKharWp9a9K1bBAAcK5GCMq5CkxLlSlYq0oWLiCyFRooUFwUqXabCpYKXSbO3aDg4d6Ojt27TRrWLVC5Qi3KkbNgCDAQb8UOxqE6LIoWL1Szeg==`

---

## File Structure

- `crates/goresave_g1r_codec_host/src/lib.rs` — calibration sample consts + synthetic compress-input fn (Task 1), `calibrate` command handler + self_test-arm extraction (Task 2).
- `crates/goresave_g1r_codec_host/tests/probe_ipc.rs` — host tests (Tasks 1–2).
- `crates/goresave_core/src/codec_backend.rs` — `calibrate()` backend method (Task 3).
- `crates/goresave_core/src/lib.rs` — `check_codec` auto-calibrate trigger + user-facing field mapping (Tasks 4–5).
- `apps/goresave/lib/features/editor/domain/editor_models.dart` — `CodecStatus` user-facing fields (Task 6).
- `apps/goresave/lib/features/editor/ui/editor_page.dart` — status panel rewrite (Task 7).
- `apps/goresave/test/...` — Dart widget test (Task 7).

---

## Task 1: Host — embedded calibration sample

**Files:**
- Modify: `crates/goresave_g1r_codec_host/src/lib.rs` (add consts + fn near the other `const G1R_*` blocks, ~line 588 after `G1R_544_PROFILE_JSON`)
- Test: `crates/goresave_g1r_codec_host/tests/probe_ipc.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/goresave_g1r_codec_host/tests/probe_ipc.rs` (import the new symbols in the existing `use goresave_g1r_codec_host::{...}` block: `calibration_sample`, `calibration_compress_input`):

```rust
#[test]
fn calibration_sample_is_well_formed() {
    let sample = calibration_sample();
    // compressedBase64 decodes
    let compressed = base64_decode(&sample.compressed_base64);
    assert_eq!(compressed.len(), 3622);
    assert_eq!(sample.expected_size, 4096);
    assert_eq!(sample.expected_decompressed_sha1.len(), 40);
    // head hex is 64 bytes
    assert_eq!(sample.expected_decompressed_head_hex.len(), 128);
}

#[test]
fn calibration_compress_input_is_deterministic_4kib() {
    let a = calibration_compress_input();
    let b = calibration_compress_input();
    assert_eq!(a.len(), 4096);
    assert_eq!(a, b);
}
```

Add this helper near the top of the test file (after the imports) if no base64 decoder is already in scope — reuse the crate's base64 if exported, else use the `base64` dev-dependency:

```rust
fn base64_decode(s: &str) -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s).unwrap()
}
```

(If `base64` is not already a dev-dependency, add `base64 = "0.22"` under `[dev-dependencies]` in `crates/goresave_g1r_codec_host/Cargo.toml`. Check first: `grep -n base64 crates/goresave_g1r_codec_host/Cargo.toml`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_g1r_codec_host calibration_sample_is_well_formed calibration_compress_input_is_deterministic_4kib`
Expected: FAIL — `cannot find function calibration_sample` / `calibration_compress_input`.

- [ ] **Step 3: Write minimal implementation**

In `crates/goresave_g1r_codec_host/src/lib.rs`, add after the `G1R_544_PROFILE_JSON` const block. Reuse the existing `RuntimeSelftestOracleSample` struct (fields: `compressed_base64`, `expected_size`, `expected_decompressed_sha1`, `expected_decompressed_head_hex`).

```rust
/// Compressed bytes of a 4 KiB deterministic synthetic plaintext we authored
/// (not game data). Generated once against a current build and verified to
/// round-trip. Oodle Kraken decode format is version-stable, so this blob
/// decompresses correctly on any build, giving calibration a known-answer
/// decompress oracle that needs no user save.
const CALIBRATION_SAMPLE_COMPRESSED_B64: &str = "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";
const CALIBRATION_SAMPLE_EXPECTED_SIZE: usize = 4096;
const CALIBRATION_SAMPLE_DECOMPRESSED_SHA1: &str = "ac98ade89e3d7417584bc0aa8036a56d31d4e285";
const CALIBRATION_SAMPLE_DECOMPRESSED_HEAD_HEX: &str = "676f7265736176652d636f6465632d63616c6962726174696f6e2d73616d706c642c77302c454e2c4f4e552c524948512c46404c442c454055402c2c2c2c2c2c";

/// The embedded known-answer decompress oracle for calibration.
pub fn calibration_sample() -> RuntimeSelftestOracleSample {
    RuntimeSelftestOracleSample {
        compressed_base64: CALIBRATION_SAMPLE_COMPRESSED_B64.to_string(),
        expected_size: CALIBRATION_SAMPLE_EXPECTED_SIZE,
        expected_decompressed_sha1: CALIBRATION_SAMPLE_DECOMPRESSED_SHA1.to_string(),
        expected_decompressed_head_hex: CALIBRATION_SAMPLE_DECOMPRESSED_HEAD_HEX.to_string(),
    }
}

/// A 4 KiB deterministic synthetic buffer used as the compress selftest input.
/// Independent of the decompress oracle: the worker compresses it then
/// decompresses the result to verify a real round-trip.
pub fn calibration_compress_input() -> Vec<u8> {
    (0..CALIBRATION_SAMPLE_EXPECTED_SIZE as u32)
        .map(|i| (i.wrapping_mul(73).wrapping_add(17) & 0xFF) as u8)
        .collect()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_g1r_codec_host calibration_sample_is_well_formed calibration_compress_input_is_deterministic_4kib`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_g1r_codec_host/src/lib.rs crates/goresave_g1r_codec_host/tests/probe_ipc.rs crates/goresave_g1r_codec_host/Cargo.toml
git commit -m "feat(host): embed known-answer calibration sample"
```

---

## Task 2: Host — `calibrate` command

The `calibrate` command probes the exe; if already supported (known profile or
derived cache) it returns the probe unchanged; otherwise it runs the existing
runtime selftest with the embedded sample + synthetic compress input, lets the
existing cache-recording path persist a derived profile, and re-probes so the
response reflects the promoted state.

**Files:**
- Modify: `crates/goresave_g1r_codec_host/src/lib.rs` — extract the `self_test` match-arm body into a reusable helper, add the `calibrate` arm (in `handle_ipc_line_inner`, the `match command` near line 4073).
- Test: `crates/goresave_g1r_codec_host/tests/probe_ipc.rs`

- [ ] **Step 1: Write the failing test**

`calibrate` on a synthetic exe that matches a **known profile** must be a no-op
that reports supported (no selftest needed). This mirrors
`ipc_self_test_uses_runtime_worker_when_configured`'s setup.

```rust
#[test]
fn ipc_calibrate_is_noop_for_known_profile() {
    let exe = minimal_pe64_with_imports_and_relocations();
    let profile = parse_profile_json(&profile_json(
        &sha256_hex(&exe),
        exe.len() as u64,
        "0x23A85CE7",
        "0x140000000",
        "0x1010",
        "0x1020",
        "0x1030",
    ))
    .unwrap();
    let temp = write_temp_exe(&exe);

    let response = handle_ipc_line_with_runtime_worker(
        &format!(
            r#"{{"id":"cal-1","command":"calibrate","exePath":"{}"}}"#,
            json_escape_path(temp.path())
        ),
        &[profile],
        &helper_binary_path(),
    );
    let value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(value["id"], "cal-1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["supported"], true);
    assert_eq!(value["data"]["resolutionMode"], "known_profile");
    assert_eq!(value["data"]["calibrationRan"], false);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_g1r_codec_host ipc_calibrate_is_noop_for_known_profile`
Expected: FAIL — the response is an `invalid_command` error (no `calibrate` arm yet).

- [ ] **Step 3: Write minimal implementation**

In `crates/goresave_g1r_codec_host/src/lib.rs`:

(a) Extract the existing `"self_test" => { ... }` arm body (lines ~4064–4171) into a free function so `calibrate` can reuse it. Replace the arm with a call:

```rust
        "self_test" => run_self_test_command(id, &value, profiles, runtime_worker_path),
```

and add (place above `handle_ipc_line_inner`):

```rust
fn run_self_test_command(
    id: Option<String>,
    value: &Value,
    profiles: &[VersionProfile],
    runtime_worker_path: Option<&Path>,
) -> (Option<String>, Result<Value, HostError>) {
    // ... the exact body previously inside the "self_test" match arm,
    // with `value` used by reference (it already was `&value` internally) ...
}
```

(Move the body verbatim. It already references `value`, `profiles`,
`runtime_worker_path`, `id`. The only change is taking them as params.)

(b) Add the `calibrate` arm in the `match command`:

```rust
        "calibrate" => {
            let Some(exe_path) = value.get("exePath").and_then(Value::as_str) else {
                return (
                    id,
                    Err(HostError::new(
                        ErrorCode::InvalidRequest,
                        "calibrate request exePath is required",
                    )),
                );
            };
            let exe_path = PathBuf::from(exe_path);
            let cache_path = parse_derived_profile_cache_path(&value);
            let response = run_calibrate_command(
                id.clone(),
                exe_path,
                cache_path,
                &value,
                profiles,
                runtime_worker_path,
            );
            (id, response)
        }
```

(c) Add the calibration orchestrator. It returns a `ProbeResponse`-shaped value
plus a `calibrationRan` flag:

```rust
fn run_calibrate_command(
    id: Option<String>,
    exe_path: PathBuf,
    cache_path: PathBuf,
    base_value: &Value,
    profiles: &[VersionProfile],
    runtime_worker_path: Option<&Path>,
) -> Result<Value, HostError> {
    // 1. Probe first. If already supported (known profile or derived cache),
    //    no calibration is needed.
    let probe = probe_exe_with_derived_cache(
        &ProbeRequest { exe_path: exe_path.clone() },
        profiles,
        Some(cache_path.as_path()),
    )?;
    if probe.supported {
        let mut value = serde_json::to_value(&probe)
            .map_err(|e| HostError::new(ErrorCode::InvalidRequest, e.to_string()))?;
        value["calibrationRan"] = json!(false);
        return Ok(value);
    }

    // 2. Build a self_test request carrying the embedded sample + synthetic
    //    compress input, force the runtime selftests on, and reuse the proven
    //    self_test command (it records the derived profile cache on success).
    let sample = calibration_sample();
    let compress_input = calibration_compress_input();
    let mut st = base_value.clone();
    let obj = st.as_object_mut().ok_or_else(|| {
        HostError::new(ErrorCode::InvalidRequest, "calibrate request must be an object")
    })?;
    obj.insert("exePath".into(), json!(exe_path.display().to_string()));
    obj.insert("derivedProfileCachePath".into(), json!(cache_path.display().to_string()));
    obj.insert("mapImage".into(), json!(true));
    obj.insert("resolveImports".into(), json!(true));
    obj.insert("runRuntimeSelftests".into(), json!(true));
    obj.insert("runtimeSelftestRunDecompress".into(), json!(true));
    obj.insert("runtimeSelftestRunCompress".into(), json!(true));
    obj.insert("runtimeSelftestSample".into(), serde_json::to_value(&sample)
        .map_err(|e| HostError::new(ErrorCode::InvalidRequest, e.to_string()))?);
    obj.insert("runtimeSelftestCompressSample".into(), json!({
        "inputBase64": BASE64_STANDARD.encode(&compress_input),
        "level": 4
    }));

    let (_id, st_result) = run_self_test_command(id, &st, profiles, runtime_worker_path);
    let _ = st_result?; // selftest + cache recording ran; failures propagate.

    // 3. Re-probe: the derived cache now (if calibration passed) promotes the
    //    exe to a supported derived profile.
    let promoted = probe_exe_with_derived_cache(
        &ProbeRequest { exe_path },
        profiles,
        Some(cache_path.as_path()),
    )?;
    if !promoted.supported {
        return Err(HostError::with_details(
            ErrorCode::UnsupportedExe,
            "calibration did not produce a usable codec profile",
            json!({ "sha256": promoted.exe_sha256, "peTimestamp": format!("0x{:08X}", promoted.pe_timestamp) }),
        ));
    }
    let mut value = serde_json::to_value(&promoted)
        .map_err(|e| HostError::new(ErrorCode::InvalidRequest, e.to_string()))?;
    value["calibrationRan"] = json!(true);
    Ok(value)
}
```

Confirm `BASE64_STANDARD` is already imported in lib.rs (the compress path uses
it). If not, add `use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_g1r_codec_host ipc_calibrate_is_noop_for_known_profile`
Expected: PASS.

- [ ] **Step 5: Run the full host suite (no regressions from the self_test extraction)**

Run: `cargo test -p goresave_g1r_codec_host`
Expected: all prior tests + the 3 new ones PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/goresave_g1r_codec_host/src/lib.rs crates/goresave_g1r_codec_host/tests/probe_ipc.rs
git commit -m "feat(host): add calibrate command (selftest + derived cache promotion)"
```

---

## Task 3: Core — `calibrate()` backend method

**Files:**
- Modify: `crates/goresave_core/src/codec_backend.rs` (add method on `G1rBinaryHostBackend`, mirroring `probe()` at line 151)
- Test: `crates/goresave_core/src/codec_backend.rs` (tests module at the bottom; see `binary_host_probe_maps_supported_response_to_backend_probe` ~line 485)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn binary_host_calibrate_maps_promoted_response() {
    let invoker = StubInvoker::new(json!({
        "supported": true,
        "profile": "g1r-derived-77f3d48c",
        "resolutionMode": "derived_profile_cache",
        "canCompress": true,
        "canDecompress": true,
        "calibrationRan": true
    }));
    let backend = G1rBinaryHostBackend::with_invoker(Box::new(invoker));

    let probe = backend.calibrate().unwrap();

    assert!(probe.available);
    assert!(probe.can_compress);
    assert_eq!(probe.resolution_mode.as_deref(), Some("derived_profile_cache"));
}
```

(Use whatever stub-invoker helper the existing backend tests use. Inspect
`binary_host_probe_maps_supported_response_to_backend_probe` to copy the exact
construction — method name `with_invoker` / `StubInvoker` may differ; match it.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core binary_host_calibrate_maps_promoted_response`
Expected: FAIL — `no method named calibrate`.

- [ ] **Step 3: Write minimal implementation**

In `crates/goresave_core/src/codec_backend.rs`, add to `impl G1rBinaryHostBackend` (not the trait — calibration is host-specific), mirroring `probe()`:

```rust
    pub fn calibrate(&self) -> Result<CodecBackendProbe, CoreError> {
        let data = self.invoker.invoke(self.request(json!({
            "command": "calibrate",
        })))?;
        Ok(CodecBackendProbe {
            backend: "g1r_binary_host".to_string(),
            available: data.get("supported").and_then(Value::as_bool).unwrap_or(false),
            can_decompress: data.get("canDecompress").and_then(Value::as_bool).unwrap_or(false),
            can_compress: data.get("canCompress").and_then(Value::as_bool).unwrap_or(false),
            status: if data.get("supported").and_then(Value::as_bool).unwrap_or(false) {
                "supported".to_string()
            } else {
                "unsupported".to_string()
            },
            profile: data.get("profile").and_then(Value::as_str).map(ToOwned::to_owned),
            resolution_mode: data.get("resolutionMode").and_then(Value::as_str).map(ToOwned::to_owned),
            details: data,
        })
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core binary_host_calibrate_maps_promoted_response`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/codec_backend.rs
git commit -m "feat(core): add G1rBinaryHostBackend::calibrate"
```

---

## Task 4: Core — auto-calibrate in `check_codec`

**Files:**
- Modify: `crates/goresave_core/src/lib.rs` — `check_codec` (line 4048) and `probe_binary_host_from_config` (line 4131).
- Test: `crates/goresave_core/src/lib.rs` tests module (near `check_codec_*` tests ~line 10378).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn check_codec_auto_calibrates_pattern_profile_then_reports_supported() {
    // A stub backend whose probe() returns pattern_profile (unsupported) and
    // whose calibrate() returns a promoted derived_profile_cache (supported).
    let backend = SequencedStubBackend::new(
        /* probe */ json!({
            "backend": "g1r_binary_host", "available": false,
            "canDecompress": false, "canCompress": false,
            "status": "unsupported", "profile": "g1r-23A85CE7",
            "resolutionMode": "pattern_profile", "details": {}
        }),
        /* calibrate */ json!({
            "backend": "g1r_binary_host", "available": true,
            "canDecompress": true, "canCompress": true,
            "status": "supported", "profile": "g1r-derived-77f3d48c",
            "resolutionMode": "derived_profile_cache", "details": {}
        }),
    );

    let probe = resolve_binary_host_probe(&backend).unwrap();

    assert!(probe.available);
    assert!(probe.can_compress);
    assert_eq!(probe.resolution_mode.as_deref(), Some("derived_profile_cache"));
}
```

This test introduces a small seam: a function `resolve_binary_host_probe` taking
something that can `probe()` + `calibrate()`. To keep it simple and avoid a new
trait, make `resolve_binary_host_probe` generic over a concrete helper, OR test
through `binary_host_backend_from_config` with a fake invoker that returns
different payloads per command. Prefer the fake-invoker route if the existing
tests already have a command-dispatching stub (check
`check_codec_reports_optional_binary_host_probe_errors_without_selecting_it`
~line 10450 for the pattern). Use whichever seam matches existing tests; the
assertion (pattern_profile → calibrate → supported) is the contract.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core check_codec_auto_calibrates_pattern_profile_then_reports_supported`
Expected: FAIL — `resolve_binary_host_probe` not defined (or assertion fails: still unsupported).

- [ ] **Step 3: Write minimal implementation**

Replace `probe_binary_host_from_config` usage in `check_codec` with a version
that auto-calibrates. In `crates/goresave_core/src/lib.rs`:

```rust
fn probe_binary_host_from_config(
    config: &Value,
) -> Result<codec_backend::CodecBackendProbe, CoreError> {
    let backend = binary_host_backend_from_config(config)?;
    let probe = codec_backend::CodecBackend::probe(&backend)?;
    // Auto-calibration: a pattern-resolved (untrusted) build is promoted to a
    // verified derived profile by running one runtime selftest with the host's
    // embedded sample. Best-effort: a calibration failure leaves the original
    // unsupported probe, which the UI surfaces as a plain "can't open yet"
    // message rather than an error.
    if probe.resolution_mode.as_deref() == Some("pattern_profile") {
        match backend.calibrate() {
            Ok(calibrated) => return Ok(calibrated),
            Err(_) => return Ok(probe),
        }
    }
    Ok(probe)
}
```

If the test uses a `resolve_binary_host_probe(&backend)` seam instead, factor the
post-probe logic into that function and call it from
`probe_binary_host_from_config`. Keep one implementation; no duplication.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core check_codec_auto_calibrates_pattern_profile_then_reports_supported`
Expected: PASS.

- [ ] **Step 5: Run core codec tests (no regressions)**

Run: `cargo test -p goresave_core check_codec`
Expected: existing `check_codec_*` tests + the new one PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): auto-calibrate pattern-resolved builds in check_codec"
```

---

## Task 5: Core — user-facing codec message mapping

**Files:**
- Modify: `crates/goresave_core/src/lib.rs` — `codec_status_from_probes` (line 4055) and replace `binary_host_message` usage.
- Test: `crates/goresave_core/src/lib.rs` tests module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn codec_user_message_ready_for_compress_capable_build() {
    let m = codec_user_message_for("g1r_binary_host", true, true, true, None);
    assert_eq!(m["userSeverity"], "ok");
    assert_eq!(m["userTitle"], "Game codec ready");
}

#[test]
fn codec_user_message_unsupported_for_unavailable_build() {
    let m = codec_user_message_for("g1r_binary_host", false, false, false, None);
    assert_eq!(m["userSeverity"], "error");
    assert_eq!(m["userTitle"], "This game version can't be opened yet");
    assert!(m["userHint"].as_str().unwrap().contains("editor update"));
}

#[test]
fn codec_user_message_exe_not_found() {
    let m = codec_user_message_for(
        "g1r_binary_host", false, false, false,
        Some("G1R executable not found: D:/x/G1R-Win64-Shipping.exe"),
    );
    assert_eq!(m["userTitle"], "Gothic 1 Remake not found");
    assert!(m["userHint"].as_str().unwrap().contains("game path"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core codec_user_message`
Expected: FAIL — `codec_user_message_for` not defined.

- [ ] **Step 3: Write minimal implementation**

Add to `crates/goresave_core/src/lib.rs`:

```rust
/// Build the user-facing codec fields. `error_text` is the raw host/probe error
/// string when the binary backend probe failed (used only to categorize, never
/// shown verbatim).
fn codec_user_message_for(
    selected_backend: &str,
    available: bool,
    can_compress: bool,
    can_decompress: bool,
    error_text: Option<&str>,
) -> Value {
    // Pure-Rust backend selected: leave user fields empty (no game codec needed
    // for read-only flows). Callers fall back to the existing message.
    if selected_backend != "g1r_binary_host" {
        return json!({});
    }
    if let Some(err) = error_text {
        let lower = err.to_ascii_lowercase();
        if lower.contains("not found") || lower.contains("missing") {
            return json!({
                "userSeverity": "error",
                "userTitle": "Gothic 1 Remake not found",
                "userMessage": "The game executable wasn't found at the saved path.",
                "userHint": "Set the game path in settings.",
            });
        }
        return json!({
            "userSeverity": "error",
            "userTitle": "This game version can't be opened yet",
            "userMessage": "Looks like a new game update the editor doesn't recognize yet.",
            "userHint": "Check for an editor update — a new version usually follows shortly.",
        });
    }
    if available || can_compress || can_decompress {
        return json!({
            "userSeverity": "ok",
            "userTitle": "Game codec ready",
            "userMessage": "The editor can read and write this game version.",
            "userHint": "",
        });
    }
    json!({
        "userSeverity": "error",
        "userTitle": "This game version can't be opened yet",
        "userMessage": "Looks like a new game update the editor doesn't recognize yet.",
        "userHint": "Check for an editor update — a new version usually follows shortly.",
    })
}
```

Wire it into `codec_status_from_probes`. After the existing `value[...]`
assignments (~line 4097) and before returning, merge the user fields. Thread the
binary probe error through: change `codec_status_from_probes` to capture the
error string in the `Err` branch into a local `binary_error: Option<String>`,
then:

```rust
    let user = codec_user_message_for(
        &selected_probe.backend,
        selected_probe.available,
        selected_probe.can_compress,
        selected_probe.can_decompress,
        binary_error.as_deref(),
    );
    if let Some(obj) = user.as_object() {
        for (k, v) in obj {
            value[k] = v.clone();
        }
    }
```

Leave the existing `value["message"]`/`status` (techy) in place for the Details
section. `binary_host_message` may stay as the `message` source.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core codec_user_message`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): map codec state to plain user-facing title/message/hint"
```

---

## Task 6: App — `CodecStatus` user-facing fields

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/editor_models.dart` (`CodecStatus`, line 786)
- Test: `apps/goresave/test/` (add `codec_status_test.dart` if none exists; check `ls apps/goresave/test`)

- [ ] **Step 1: Write the failing test**

Create/extend `apps/goresave/test/codec_status_test.dart`:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';

void main() {
  test('CodecStatus parses user-facing fields', () {
    final s = CodecStatus.fromJson(const {
      'available': false,
      'status': 'unsupported',
      'message': 'techy',
      'userSeverity': 'error',
      'userTitle': "This game version can't be opened yet",
      'userMessage': 'Looks like a new game update...',
      'userHint': 'Check for an editor update...',
    });
    expect(s.userTitle, "This game version can't be opened yet");
    expect(s.userSeverity, 'error');
    expect(s.userHint, isNotEmpty);
  });
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/goresave && flutter test test/codec_status_test.dart`
Expected: FAIL — `userTitle` is not a member of `CodecStatus`.

- [ ] **Step 3: Write minimal implementation**

In `editor_models.dart`, add fields to `CodecStatus` (constructor params,
`fromJson`, and final declarations):

```dart
    this.userTitle,
    this.userMessage,
    this.userHint,
    this.userSeverity,
```

In `fromJson`:

```dart
      userTitle: json['userTitle'] as String?,
      userMessage: json['userMessage'] as String?,
      userHint: json['userHint'] as String?,
      userSeverity: json['userSeverity'] as String?,
```

Field declarations:

```dart
  final String? userTitle;
  final String? userMessage;
  final String? userHint;
  final String? userSeverity;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd apps/goresave && flutter test test/codec_status_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/editor_models.dart apps/goresave/test/codec_status_test.dart
git commit -m "feat(app): parse user-facing codec fields in CodecStatus"
```

---

## Task 7: App — status panel rewrite (plain message + Details disclosure)

**Files:**
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart` (codec status block, lines 3556–3567)
- Test: `apps/goresave/test/` widget test (add `codec_status_panel_test.dart` if the panel is extractable; otherwise verify via existing editor widget test harness — check `ls apps/goresave/test`).

- [ ] **Step 1: Write the failing test**

If the status block is inline (not its own widget), first extract it into a
`_CodecStatusView extends StatelessWidget` taking `CodecStatus? codec` and
`String? codecError`, then test that widget:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/ui/editor_page.dart' show CodecStatusView;

void main() {
  testWidgets('shows plain message + hint, hides techy detail behind Details',
      (tester) async {
    const codec = CodecStatus(
      available: false,
      status: 'unsupported',
      message: 'G1R codec host is configured but not available.',
      userSeverity: 'error',
      userTitle: "This game version can't be opened yet",
      userMessage: 'Looks like a new game update the editor doesn\'t recognize yet.',
      userHint: 'Check for an editor update — a new version usually follows shortly.',
      profile: 'g1r-23A85CE7',
      resolutionMode: 'pattern_profile',
    );
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: CodecStatusView(codec: codec, codecError: null)),
    ));

    expect(find.text("This game version can't be opened yet"), findsOneWidget);
    expect(find.textContaining('editor update'), findsOneWidget);
    // Techy fields not shown until Details is expanded.
    expect(find.textContaining('pattern_profile'), findsNothing);

    await tester.tap(find.text('Details'));
    await tester.pumpAndSettle();
    expect(find.textContaining('pattern_profile'), findsOneWidget);
  });
}
```

(Rename the extracted widget `CodecStatusView` — public so the test can import
it. Adjust the `show` clause to the real export.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/goresave && flutter test test/codec_status_panel_test.dart`
Expected: FAIL — `CodecStatusView` undefined (not yet extracted).

- [ ] **Step 3: Write minimal implementation**

Extract lines 3556–3567 into a `CodecStatusView` widget and replace the techy
body. Primary content = `userTitle` (with a severity icon) + `userMessage` +
`userHint`; technical fields go inside an `ExpansionTile(title: Text('Details'))`:

```dart
class CodecStatusView extends StatelessWidget {
  const CodecStatusView({super.key, required this.codec, required this.codecError});

  final CodecStatus? codec;
  final String? codecError;

  @override
  Widget build(BuildContext context) {
    final codec = this.codec;
    final scheme = Theme.of(context).colorScheme;
    if (codec == null) {
      return Text(codecError ?? 'No codec status');
    }
    final severity = codec.userSeverity ?? (codec.available ? 'ok' : 'error');
    final isError = severity == 'error';
    final title = codec.userTitle ?? codec.message;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(
              isError ? Icons.error_outline : Icons.check_circle_outline,
              size: 18,
              color: isError ? scheme.error : scheme.primary,
            ),
            const SizedBox(width: 6),
            Expanded(
              child: Text(title,
                  style: TextStyle(color: isError ? scheme.error : null)),
            ),
          ],
        ),
        if ((codec.userMessage ?? '').isNotEmpty) ...[
          const SizedBox(height: 4),
          Text(codec.userMessage!),
        ],
        if ((codec.userHint ?? '').isNotEmpty) ...[
          const SizedBox(height: 4),
          Text(codec.userHint!, style: Theme.of(context).textTheme.bodySmall),
        ],
        const SizedBox(height: 8),
        ExpansionTile(
          tilePadding: EdgeInsets.zero,
          childrenPadding: const EdgeInsets.only(bottom: 8),
          title: const Text('Details'),
          children: [
            Align(
              alignment: Alignment.centerLeft,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(codec.message),
                  Text('Decompress: ${codec.canDecompress ? 'yes' : 'no'} | '
                      'Compress: ${codec.canCompress ? 'yes' : 'no'}'),
                  if (codec.selectedBackend != null)
                    Text('Backend: ${codec.selectedBackend}'),
                  if (codec.profile != null) Text('Profile: ${codec.profile}'),
                  if (codec.resolutionMode != null)
                    Text('Resolution: ${codec.resolutionMode}'),
                ],
              ),
            ),
          ],
        ),
      ],
    );
  }
}
```

Replace the old inline block (lines 3556–3567) with
`CodecStatusView(codec: codec, codecError: state.codecError)` — keep the existing
`state.codecError` error row above it as-is, or fold it into the view (the view
already falls back to `codecError` when `codec == null`). Remove the now-dead
inline `Text(codec?.message ...)` + flag lines.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd apps/goresave && flutter test test/codec_status_panel_test.dart`
Expected: PASS.

- [ ] **Step 5: Analyze + full app test**

Run: `cd apps/goresave && flutter analyze && flutter test`
Expected: no analyzer errors; all tests pass.

- [ ] **Step 6: Commit**

```bash
git add apps/goresave/lib/features/editor/ui/editor_page.dart apps/goresave/test/codec_status_panel_test.dart
git commit -m "feat(app): plain-language codec status with Details disclosure"
```

---

## Task 8: Manual verification gate (real game build)

Unit tests use synthetic PEs and never call real Oodle (matching the existing
suite). This task proves the end-to-end auto-calibration against the real
installed build, the way the codec host was originally validated.

**Files:** none (verification only).

- [ ] **Step 1: Build the host release binary**

Run: `cargo build -p goresave_g1r_codec_host --release`
Expected: `Finished release`.

- [ ] **Step 2: Calibrate the real 1.0.1 build from a clean cache**

```bash
BIN=target/release/goresave_g1r_codec_host.exe
EXE='D:/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Binaries/Win64/G1R-Win64-Shipping.exe'
CACHE="$TEMP/goresave_calib_test.json"
rm -f "$CACHE"
printf '{"id":"c1","command":"calibrate","exePath":"%s","derivedProfileCachePath":"%s"}\n' "$EXE" "$CACHE" | "$BIN" --stdio
```

Expected: `"ok":true`, `data.supported == true`, `data.canCompress == true`,
`data.resolutionMode == "derived_profile_cache"` (or `known_profile` if 1.0.1 is
still a built-in known profile — in that case `calibrationRan == false`, which is
also correct).

To exercise the real pattern→calibrate path, temporarily test against a build
that is NOT a known profile if one is available; otherwise the no-op known-profile
path plus the synthetic-PE unit test cover the logic, and the live Task-8 proof
from the original 1.0.1 reverse-engineering session stands.

- [ ] **Step 3: Confirm the cache persists a supported re-probe**

```bash
printf '{"id":"p1","command":"probe","exePath":"%s","derivedProfileCachePath":"%s"}\n' "$EXE" "$CACHE" | "$BIN" --stdio
```

Expected: `data.supported == true` resolved via `known_profile` or
`derived_profile_cache` without re-running calibration.

- [ ] **Step 4: Record the result**

Note the observed `resolutionMode`, `canCompress`, `canDecompress` in the PR
description. No commit.

---

## Final: Changelog, version, PR

- [ ] **Step 1: Update CHANGELOG + bump version**

Add a `## [0.1.3] - 2026-06-15` section to `CHANGELOG.md` describing automatic
codec calibration for new game builds and the friendlier unsupported-build
message. Bump `apps/goresave/pubspec.yaml` `version:` to `0.1.3`.

- [ ] **Step 2: Full workspace test**

Run: `cargo test --workspace` and `cd apps/goresave && flutter test`
Expected: green.

- [ ] **Step 3: Commit + push branch + open PR**

```bash
git add CHANGELOG.md apps/goresave/pubspec.yaml
git commit -m "chore(release): codec auto-calibration, bump to 0.1.3"
git push -u origin feat/codec-auto-calibration
```

Open a PR (do not auto-merge; this is a feature branch per the user's request).

---

## Notes for the implementer

- The host `calibrate` reuses the **already-proven** `self_test` + derived-cache
  path; the new code is orchestration only. Do not reimplement selftest logic.
- Keep user-facing strings in **English** (matches existing app UI).
- The big base64 const in Task 1 is opaque data — paste it verbatim, do not
  reflow or "clean" it.
- If a referenced helper name (`StubInvoker`, `minimal_pe64_with_imports_and_relocations`,
  `helper_binary_path`, `json_escape_path`, `sha256_hex`, `profile_json`,
  `write_temp_exe`) differs slightly, match the real name in the neighboring
  existing test — the surrounding tests are the source of truth.
