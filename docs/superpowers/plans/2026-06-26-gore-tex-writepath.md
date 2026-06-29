# gore-tex Write Path Implementation Plan (replace → pack → deploy, upscale-capable)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Replace a game texture with a user image of arbitrary (multiple-of-4, power-of-two) dimensions — including upscales (4K retextures) — keeping the original pixel format, and deploy it as a mountable `~mods` Zen triplet using our own toolchain (gore-oodle encode, no oo2core).

**Architecture:** Reuse the proven read path (`gore-tex` unpack + decode). Add: BCn encode + mip-pyramid generation; a byte-faithful `FTexturePlatformData` parse↔serialize codec (the safety net: re-serialize the original byte-identically, then drive it with new dims/mips); a splice that rewrites the cooked `UTexture2D` (platform data + `.uasset` summary/export-size + `.ubulk`); a repack to Zen via the vendored retoc `to-zen` lib path with gore-oodle Oodle **encode**; deploy/undeploy to `~mods` with a JSON record. CLI: `gore texture replace|pack|deploy|undeploy`.

**Tech Stack:** Rust; `gore-tex` crate; vendored `retoc` (lib, `to-zen` + gore-oodle encode); `gore-oodle` (ooz Kraken encode/decode); `intel_tex_2` (BCn encode); `image` (load PNG); real game at `D:\SteamLibrary\steamapps\common\Gothic 1 Remake`.

**Why this shape:** The magenta spike (2026-06-26) proved a same-size swap mounts and renders. The new risk is whether the game accepts a *rewritten/upscaled* platform data — gated by an in-game test (Task 9, user-run). The byte-faithful round-trip (Task 3) ensures our serializer reproduces retoc's legacy format exactly before we change anything.

**Spec:** `docs/superpowers/specs/2026-06-26-gore-texture-replacement-design.md`
**Builds on:** `docs/superpowers/plans/2026-06-26-gore-tex-phase0-readpath.md` (read path, done), notes `2026-06-26-retoc-oodle-swap.md`, `2026-06-26-utexture2d-layout.md`, `2026-06-26-magenta-spike-result.md`.

---

## File Structure

- `crates/gore-tex/src/encode.rs` — NEW: PNG/RGBA → BCn + mip-pyramid generation.
- `crates/gore-tex/src/texdata.rs` — NEW: `FTexturePlatformData` parse↔serialize codec (byte-faithful) + the splice that rewrites cooked files. (Refactors the parse logic currently in `decode.rs` into a shared struct.)
- `crates/gore-tex/src/decode.rs` — MODIFY: keep `to_rgba8`; have `parse` delegate to `texdata` (or re-export) to avoid duplicate parsing.
- `crates/gore-tex/src/container.rs` — MODIFY: add `repack_to_zen` (legacy folder → Zen triplet) + `deploy`/`undeploy`.
- `crates/gore-tex/src/error.rs` — MODIFY: add write-path error variants.
- `vendor/retoc/src/compression.rs` — MODIFY: route the Oodle ENCODE arm to `gore_oodle::kraken_compress` (currently `bail!`s).
- `crates/gore/src/cmd/texture.rs` — MODIFY: add `Replace`, `Pack`, `Deploy`, `Undeploy` subcommands.
- `docs/superpowers/notes/2026-06-26-tozen-encode.md` — NEW: investigation output (Task 1/5: vendored to-zen lib API + encode validation).

---

## Task 1: Wire gore-oodle ENCODE into vendored retoc

**Files:** `vendor/retoc/src/compression.rs`, `crates/gore-tex/Cargo.toml` (if a dep is needed), `docs/superpowers/notes/2026-06-26-tozen-encode.md`

- [ ] **Step 1: Inspect the encode arm.** Read `vendor/retoc/src/compression.rs`. The DECODE arm calls `gore_oodle::kraken_decompress(input, output.len())`; the ENCODE arm currently `bail!`s ("not supported … decode only"). Identify the exact compress signature retoc expects (input slice → compressed Vec or into a buffer; what compressor/level enum it passes).

- [ ] **Step 2: Route encode to gore-oodle.** Replace the `bail!` with a call to `gore_oodle::kraken_compress(input, level)` where `level` maps from retoc's requested level, CLAMPED to `gore_oodle::MAX_SAFE_COMPRESS_LEVEL` (5). Return the compressed bytes in the shape retoc's caller expects. Keep the method tag = Oodle/Kraken (matches what the `.utoc` records as the compression method).

- [ ] **Step 3: Round-trip test.** Add a unit test in `compression.rs` (or a gore-tex test): compress a 256 KiB buffer of varied bytes via the wired path, decompress via the decode path, assert equal. Run `cargo test -p gore-tex` (or `-p retoc` if the vendored crate runs its own tests). Expected: PASS.

- [ ] **Step 4: Confirm to-zen lib API + write the note.** Read `vendor/retoc/src/` to confirm the `to-zen` (legacy→zen) conversion is reachable as a LIBRARY call (the upstream CLI action wraps a lib fn). Document in `docs/superpowers/notes/2026-06-26-tozen-encode.md`: the exact fn(s) + signature to convert a legacy cooked dir/pak → `.utoc/.ucas/.pak` triplet for `--version UE5_4`, what `Config`/version args to pass, and that it now uses gore-oodle encode. If `to-zen` is NOT reachable from the trimmed vendor (CLI-only logic was dropped), note exactly which modules/functions must be un-trimmed from upstream `d7b6350` and do so (vendor them), keeping the gore-oodle compression seam.

- [ ] **Step 5: Commit.** `git add vendor/retoc crates/gore-tex/Cargo.toml docs/superpowers/notes/2026-06-26-tozen-encode.md && git commit -m "feat(gore-tex): gore-oodle Oodle encode in vendored retoc (to-zen)"`

---

## Task 2: BCn encode + mip-pyramid (`encode.rs`)

**Files:** `crates/gore-tex/src/encode.rs` (new), `crates/gore-tex/src/lib.rs` (add `pub mod encode;`), `crates/gore-tex/Cargo.toml` (add `intel_tex_2`)

- [ ] **Step 1: Add the dep.** In `crates/gore-tex/Cargo.toml`, add `intel_tex_2 = "0.4"` (verify the version resolves; use the latest 0.x that does, note it). `image = "0.25"` is in the `gore` bin already; add it to `gore-tex` too if `encode` loads PNGs directly (or accept raw RGBA and let the CLI load PNG — prefer: `encode` takes raw RGBA8 + dims, CLI loads PNG → keeps `encode` pure/testable).

- [ ] **Step 2: Define the API + a failing test.** In `encode.rs`:
```rust
use crate::error::{Result, TexError};

/// Generate the full mip pyramid for `rgba` (row-major RGBA8, width*height px),
/// BCn-encoded to `format` (one of PF_DXT1/PF_DXT5/PF_BC5/PF_BC7). Returns one
/// Vec<u8> per mip, largest first (mip0 = full res), down to 1x1.
pub fn encode_mips(rgba: &[u8], width: u32, height: u32, format: &str) -> Result<Vec<Vec<u8>>>;
```
Test (CI-runnable): make a 8x8 solid-red RGBA buffer, `encode_mips(.., "PF_DXT1")`, assert: mip count == 4 (8,4,2,1), mip0.len() == block_math(8,8,DXT1)=2*2*8=32, last mip = block_math(1,1)=8, and decoding mip0 back via `texture2ddecoder::decode_bc1` yields ~red. Assert dims are validated: `encode_mips(.., 6, 6, ..)` (not power-of-two) → `TexError` (BCn needs mult-of-4; PoT required when mipping). Run: FAIL (not implemented).

- [ ] **Step 3: Implement.** Validate dims (mult-of-4; power-of-two — required for a clean mip chain). For each mip level: box-downsample the RGBA (simple 2x2 average; mip0 = original), then BCn-encode via intel_tex_2 (map PF_DXT1→BC1, PF_DXT5→BC3, PF_BC5→BC5, PF_BC7→BC7 — verify intel_tex_2's exact entry points/structs first). BC5 is 2-channel: feed R,G (for normal maps); document. Stop at 1x1. Honor sRGB? intel_tex_2 encodes raw bytes; sRGB is a flag on the asset, not the encoder — no action needed here (we keep the original format/flags).

- [ ] **Step 4: Run test.** `cargo test -p gore-tex encode:: -- --nocapture` → PASS.

- [ ] **Step 5: Commit.** `git add crates/gore-tex/src/encode.rs crates/gore-tex/src/lib.rs crates/gore-tex/Cargo.toml Cargo.lock && git commit -m "feat(gore-tex): BCn encode + mip pyramid generation"`

---

## Task 3: Byte-faithful FTexturePlatformData codec (`texdata.rs`)

This is the safety net. Build a parse↔serialize that reproduces the ORIGINAL bytes exactly. Only after round-trip passes do we change dims (Task 4).

**Files:** `crates/gore-tex/src/texdata.rs` (new), `crates/gore-tex/src/decode.rs` (refactor parse to use it), `crates/gore-tex/src/lib.rs`

- [ ] **Step 1: Define the data model.** In `texdata.rs`:
```rust
/// One mip's location + bytes as serialized in legacy cooked form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MipEntry {
    pub width: u32,
    pub height: u32,
    /// true = payload inline in .uexp right after the mip's u32 flags;
    /// false = streamed (bytes live in .ubulk, this mip carries no .uexp payload).
    pub inline: bool,
    pub flags: u32,
    pub data: Vec<u8>, // the BCn bytes (from .uexp inline region or the .ubulk slice)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformData {
    pub size_x: u32,
    pub size_y: u32,
    pub packed_data: u32,   // bit31 = bIsVirtual
    pub format: String,     // "PF_DXT5" etc.
    pub first_mip: u32,
    pub mips: Vec<MipEntry>,
    /// trailing FTexturePlatformData bytes after the mip array we don't model
    /// individually (kept verbatim for faithful re-serialization).
    pub trailer: Vec<u8>,
    /// byte offset in .uexp where the platform-data region begins (after the
    /// property block + StripFlags + bCooked), and where it ends.
    pub region: std::ops::Range<usize>,
}
```

- [ ] **Step 2: Failing round-trip test (the oracle).** In `texdata.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn fx(n:&str)->Option<Vec<u8>>{ std::fs::read(format!("../../work/tex-fixtures/{n}")).ok() }
    #[test]
    fn roundtrip_inline_fixture_byte_identical() {
        let (Some(ua),Some(ue)) = (fx("sample.uasset"), fx("sample.uexp")) else {
            eprintln!("skip: fixture absent"); return; };
        let ub = fx("sample.ubulk").unwrap_or_default();
        let pd = PlatformData::parse(&ua, &ue, &ub).unwrap();
        // Re-serialize the platform-data region and assert the .uexp region is identical.
        let mut uexp2 = ue.clone();
        pd.serialize_into_uexp(&mut uexp2, &ua).unwrap();
        assert_eq!(uexp2, ue, "re-serialized .uexp must be byte-identical to original");
    }
}
```
Run: FAIL.

- [ ] **Step 3: Implement parse.** Port the working anchor-locate from `decode.rs` (anchor on `PF_*` FString, validate preceding SizeX/SizeY/PackedData, read FirstMip/NumMips, then walk mips). For each mip: read `u32 flags`; decide inline vs streamed using the read-path rule (streamed when a `.ubulk` exists and the computed mip is one of the largest written there; inline otherwise) — but to be byte-faithful you must determine inline-vs-stream from the SERIALIZED form, not a heuristic: per the layout note, an inline mip has its payload bytes between `flags` and the next `SizeX,SizeY,SizeZ`, a streamed mip has `SizeX,SizeY,SizeZ` immediately after `flags`. Discriminate by checking whether the 12 bytes after `flags` equal the computed `(mipW, mipH, 1)` (streamed) or not (inline, payload follows). Validate each against block math. Record `region` and `trailer`. For streamed mips, pull `data` from `.ubulk` at the running offset (mip0 at 0, then concatenated).

- [ ] **Step 4: Implement serialize.** `serialize_into_uexp(&self, uexp: &mut Vec<u8>, uasset: &[u8])` rewrites `uexp[self.region]` from the model: SizeX,SizeY,PackedData, format FString, FirstMip, NumMips, then each mip (`flags`, then inline payload if `inline` else nothing, then `SizeX,SizeY,SizeZ`), then `trailer`. Also expose `serialize_ubulk(&self) -> Vec<u8>` concatenating streamed mips in order. For the round-trip test the region length is unchanged so no summary fixup is needed yet.

- [ ] **Step 5: Run round-trip.** `cargo test -p gore-tex texdata::roundtrip -- --nocapture` → PASS (byte-identical). Also add a gated round-trip test against the streamed real texture `T_Water_N` (mark `#[ignore]`, like the other slow tests) asserting both the `.uexp` region AND `serialize_ubulk()` reproduce the originals.

- [ ] **Step 6: Refactor `decode::parse`** to build a `PlatformData` and map to `TexInfo` (single source of truth; no duplicated parsing). Keep `TexInfo`/`to_rgba8` API stable; run `cargo test -p gore-tex` (fast) → all green.

- [ ] **Step 7: Commit.** `git add crates/gore-tex/src/texdata.rs crates/gore-tex/src/decode.rs crates/gore-tex/src/lib.rs && git commit -m "feat(gore-tex): byte-faithful FTexturePlatformData codec"`

---

## Task 4: Splice — rewrite cooked files with new dims/mips (upscale)

Now drive the Task-3 serializer with NEW content. Changing dims changes region length → must fix the `.uasset` summary (export serial size + bulk data offsets/total sizes).

**Files:** `crates/gore-tex/src/texdata.rs`, `crates/gore-tex/src/error.rs`

- [ ] **Step 1: Define the splice API + failing test.**
```rust
/// Rewrite the cooked files to carry `new_mips` (largest first) at `new_w`x`new_h`,
/// keeping the original pixel format. Returns (new_uasset, new_uexp, new_ubulk).
pub fn replace_texture(
    uasset: &[u8], uexp: &[u8], ubulk: &[u8],
    new_w: u32, new_h: u32, new_mips: Vec<Vec<u8>>,
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)>;
```
Test (gated/ignored, uses fixture): take the inline fixture (128x128 DXT5), build `new_mips` by re-encoding a solid-color image at 128x128 via `encode::encode_mips`, `replace_texture` with same dims, then `PlatformData::parse` the result and assert dims=128x128, format unchanged, mip0 decodes to the solid color. (Same-dims through the new code path first.) Then a second assertion at 256x256 (upscale): parse result has size_x==256, NumMips==9, mip0.len()==block_math(256,256,DXT5).

- [ ] **Step 2: Implement.** Parse original `PlatformData`. Decide inline/stream policy for the new mips: replicate UE's convention — stream the largest mips to `.ubulk`, inline the small ones. SIMPLEST CORRECT v1 policy: keep the SAME split point semantics as the original where possible; if upscaling, stream mips whose dimension ≥ a threshold (e.g. ≥ the original's largest streamed mip dimension, or simply: stream all mips with width≥256, inline the rest) — document the chosen rule and validate the game accepts it (Task 9). Build new `MipEntry`s (flags: inline mips use the same flag value the original inline mips used; streamed use the streamed value — both were 0x0 in observed data, so use 0x0 and rely on the inline-vs-stream serialized shape). Set `size_x/size_y/first_mip(0)`. Re-serialize `.uexp` region (new length) and rebuild `.ubulk`.
- [ ] **Step 3: Fix `.uasset` summary + export size.** Changing the `.uexp` length changes the export's serial size and the package's `BulkDataStartOffset`/total header+exp size fields in the `.uasset` summary. Determine the exact summary fields retoc's legacy writer emits (read `vendor/retoc/src/` legacy summary serialization) and patch them: the export map entry's `serial_size` (= new uexp export length), `Summary.bulk_data_start_offset` (= total_header_size + new uexp length), and any total-size fields. This must match what `to-zen` expects when re-ingesting. If retoc's to-zen recomputes these from the files (rather than trusting the summary), document that and only fix what's actually read. VERIFY by round-tripping through `repack_to_zen` (Task 5) + reading back.

- [ ] **Step 4: Run tests.** `cargo test -p gore-tex texdata::replace -- --nocapture` → PASS.
- [ ] **Step 5: Commit.** `git add crates/gore-tex/src/texdata.rs crates/gore-tex/src/error.rs && git commit -m "feat(gore-tex): rewrite cooked UTexture2D with new dims/mips"`

---

## Task 5: repack_to_zen + deploy/undeploy (`container.rs`)

**Files:** `crates/gore-tex/src/container.rs`, `crates/gore-tex/src/error.rs`

- [ ] **Step 1: `repack_to_zen`.** Add:
```rust
/// Pack a folder of edited legacy cooked files into a Zen triplet
/// (.utoc/.ucas/.pak) at out_dir/<name>.{utoc,ucas,pak}, UE5.4, gore-oodle encode.
pub fn repack_to_zen(cooked_dir: &Path, name: &str, out_dir: &Path) -> Result<[PathBuf;3]>;
```
Implement via the vendored retoc to-zen lib fn documented in `2026-06-26-tozen-encode.md`. The cooked files must sit under the correct mount path inside `cooked_dir` (e.g. `cooked_dir/G1R/Content/UI/Textures/Common/T_HardwareCursor.uasset`) — the magenta spike used `G1R/Content/UI/Textures/Common/T_HardwareCursor`; mirror that mapping (`/Game/` → `G1R/Content/`).

- [ ] **Step 2: Gated round-trip test.** `#[ignore]` test: unpack `T_HardwareCursor`, lay it under the mount path, `repack_to_zen(.., "RoundTripTest", ..)`, then open the produced triplet with `iostore::open` and read+decode the asset back — assert it decodes to the SAME pixels as the original (proves to-zen + encode + FBulkDataMapEntry regen produce a valid, game-readable container). Run with `--ignored`.

- [ ] **Step 3: deploy/undeploy.**
```rust
pub fn deploy(triplet: &[PathBuf;3], game_dir: &Path, name: &str) -> Result<PathBuf>; // copies to ~mods, writes <mods>/<name>.gore-deploy.json record; returns record path
pub fn undeploy(game_dir: &Path, name: &str) -> Result<()>; // reads record, deletes listed files + record
```
`~mods` = `game_dir/G1R/Content/Paks/~mods`. Record = JSON list of the deployed file paths. undeploy deletes exactly those + the record. Non-destructive (no game-file backup needed — additive override).

- [ ] **Step 4: Unit test deploy/undeploy** against a temp fake game dir with dummy triplet files (no real game needed): deploy copies 3 files + record into `<tmp>/G1R/Content/Paks/~mods`; undeploy removes them; assert the `~mods` dir is empty after. Run (fast).

- [ ] **Step 5: Commit.** `git add crates/gore-tex/src/container.rs crates/gore-tex/src/error.rs && git commit -m "feat(gore-tex): repack to Zen triplet + deploy/undeploy to ~mods"`

---

## Task 6: CLI `replace` / `pack` / `deploy` / `undeploy`

**Files:** `crates/gore/src/cmd/texture.rs`

- [ ] **Step 1: Add subcommands.** Extend `TextureAction` with:
  - `Replace { game: PathBuf, asset: String, image: PathBuf, mod_dir: PathBuf }` — load `image` PNG → RGBA via `image`; unpack `asset` (reuse) to get original cooked files + format/dims; `encode::encode_mips(rgba, w, h, format)`; `texdata::replace_texture(...)`; write the new cooked files under `mod_dir/<mount-path>/<leaf>.{uasset,uexp,ubulk}`.
  - `Pack { mod_dir: PathBuf, name: String, out: PathBuf }` — `container::repack_to_zen(mod_dir, &name, &out)`; print the 3 triplet paths.
  - `Deploy { game: PathBuf, triplet_dir: PathBuf, name: String }` — locate the triplet by name, `container::deploy(...)`.
  - `Undeploy { game: PathBuf, name: String }` — `container::undeploy(...)`.
- [ ] **Step 2: Build + help.** `cargo build`; `cargo run -p gore -- texture --help` lists all 6 subcommands (list/extract/replace/pack/deploy/undeploy).
- [ ] **Step 3: Commit.** `git add crates/gore/src/cmd/texture.rs && git commit -m "feat(gore): texture replace/pack/deploy/undeploy CLI"`

---

## Task 7: End-to-end dry run (no game launch) — build a real upscaled mod

**Files:** none committed (scratch under gitignored `work/`); record outcome in `docs/superpowers/notes/2026-06-26-writepath-e2e.md` (commit the note).

- [ ] **Step 1.** Via the CLI (PowerShell — Git-Bash mangles `/Game/...`): `extract` a real texture (e.g. `T_HardwareCursor`) to PNG; edit it to an obvious test pattern at 2x resolution (e.g. 256x256 solid cyan) using any image lib or by generating it programmatically; `replace` → `pack` → producing a triplet. Confirm each step succeeds and the triplet files are non-empty and openable via `iostore::open` (decode the asset back, assert it's 256x256 cyan). Do NOT deploy yet.
- [ ] **Step 2.** Record results + the exact commands in the note. Commit the note.

---

## Task 8: In-game verification (USER-GATED) — upscaled texture renders

- [ ] **Step 1.** Build the cyan upscaled `T_HardwareCursor` triplet (Task 7) and `deploy` it to `~mods`.
- [ ] **Step 2. USER launches** `G1R\Binaries\Win64\G1R-Win64-Shipping.exe` and checks the cursor: a cyan square = the upscaled/rewritten platform data is accepted and renders → upscale write-path proven. (We cannot launch headless.)
- [ ] **Step 3.** Record PASS/FAIL in `docs/superpowers/notes/2026-06-26-writepath-e2e.md`. `undeploy` afterwards. If FAIL: the rewritten summary/mip layout is likely the cause — diff a known-good upscaled mod's container structure; iterate on Task 4's summary fixup / inline-stream policy.

---

## Self-Review

- **Spec coverage:** replace (T4/T6), allow-upscale (T4), keep-format + regen-mips (T2/T4), pack→Zen with our encode/no-oo2core (T1/T5), deploy/undeploy non-destructive record (T5/T6), CLI surface complete (T6), VT already rejected (read path), unknown-format hard error (encode/parse). Byte-faithful round-trip test (T3). In-game gate (T8).
- **Risks called out:** game acceptance of rewritten platform data (T8 gate); `.uasset` summary fixup correctness (T4 step 3, verified via T5 round-trip); inline/stream split policy for new mips (T4 step 2, validated T8); BC7/BC5 encode paths (T2 tests partial — full proof needs a real BC7 target in T7/T8).
- **Type consistency:** `PlatformData`/`MipEntry` (T3) consumed by `replace_texture` (T4); `encode_mips -> Vec<Vec<u8>>` (T2) feeds `replace_texture`'s `new_mips` (T4); `repack_to_zen -> [PathBuf;3]` (T5) consumed by `deploy` (T5) + CLI (T6).
- **No placeholders:** test code concrete; the one investigation point (vendored to-zen lib reachability, T1S4) has an explicit fallback (un-trim from upstream `d7b6350`).
