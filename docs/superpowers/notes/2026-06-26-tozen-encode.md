# to-zen (legacy→zen) encode path in the vendored retoc lib

Task 1 of the gore-tex write-path plan. Goal: make the vendored retoc lib PRODUCE
Zen containers (`.utoc`/`.ucas`) using `gore-oodle` for Oodle compression — so the
`to-zen` repack needs NO Epic `oo2core` DLL — and confirm `to-zen` is reachable as
a LIBRARY call (the orchestration that wraps these fns).

Upstream pinned: `trumank/retoc` @ `d7b635039c3db60942efabcd29d49679f42ab089` (MIT).

## 1. Oodle ENCODE seam (now wired to gore-oodle)

Single encode choke point: `vendor/retoc/src/compression.rs`,
`compress<S: Write>(compression: CompressionMethod, input: &[u8], mut output: S) -> Result<()>`,
the `CompressionMethod::Oodle` arm.

- **Shape:** retoc's `compress()` takes the uncompressed bytes (`input: &[u8]`) and a
  `Write` SINK (`output`); it does NOT return a `Vec`. The other arms (Zlib/Zstd/LZ4)
  build a `Vec` then `output.write_all(&buf)`. We match that: compress to a `Vec`, then
  `output.write_all(&compressed)?`.
- **No level argument:** the vendored `compress()` signature carries no level/compressor
  param (upstream hardcoded `Compressor::Mermaid` + `CompressionLevel::Normal` inside the
  Oodle arm). So we pick a safe level internally.
- **Wiring (replaces the old `bail!`):**
  ```rust
  let level = OODLE_ENCODE_LEVEL.min(gore_oodle::MAX_SAFE_COMPRESS_LEVEL);
  let compressed = gore_oodle::kraken_compress(input, level)
      .map_err(|e| anyhow::anyhow!("Oodle (gore-oodle) compression failed: {e}"))?;
  output.write_all(&compressed)?;
  ```
  where `const OODLE_ENCODE_LEVEL: u8 = gore_oodle::MAX_SAFE_COMPRESS_LEVEL;` (= 5).
- **Level mapping / clamp:** `gore_oodle::kraken_compress` already clamps internally, but we
  ALSO clamp at the call site (`.min(MAX_SAFE_COMPRESS_LEVEL)`) so the cap is explicit and
  obvious. Levels > 5 crash the ooz encoder; the default level (<= cap) is the one at which
  gore-oodle is byte-identical to Epic's Oodle.
- **Method tag preserved:** the block is still tagged `CompressionMethod::Oodle`, so the
  game/our `decompress()` decode it via Oodle/Kraken. Method tag is unchanged from upstream.

The symmetric DECODE arm was already wired (Task 2): `gore_oodle::kraken_decompress(input, output.len())`.

### Round-trip test
`vendor/retoc/src/compression.rs` → `mod test::oodle_encode_decode_round_trip`: builds a
256 KiB varied buffer (xorshift PRNG xor'd with a low-entropy `i % 251` pattern), runs it
through `compress(Oodle, …)` then `decompress(Oodle, …)`, asserts identity. PASSES
(`cargo test -p retoc oodle_encode_decode_round_trip` → ok).

## 2. to-zen reachable as a LIBRARY call — building blocks (un-trim NOT needed)

The upstream `to-zen` CLI verb's top-level orchestration (`action_to_zen`, which walks a
cooked dir / `.pak`, converts each asset, assembles the container) lived in the upstream
**CLI `src/main.rs`**, which was dropped from the vendor (lib-only). Per the task's
escalation guidance, re-vendoring that orchestration would drag in `clap` arg parsing,
the CLI logging harness, dir-walking, and `.pak` reading via `repak` — disproportionate
to copy wholesale. We did **NOT un-trim** it.

We did NOT need to: every LIB building block the orchestration calls is already `pub` and
reachable from an external crate (verified by a temporary compile-only check in
`crates/gore-tex/tests/`, which type-checked then was removed). Task 5 re-implements the
thin orchestration loop in gore-tex (or a small `gore-tex` writer module) on top of these:

Pipeline (all symbols below confirmed `pub` + linkable from gore-tex):

```rust
use std::sync::Arc;                                   // for script objects/cells if used
use retoc::iostore_writer::IoStoreWriter;
use retoc::legacy_asset::FSerializedAssetBundle;
use retoc::logging::Log;
use retoc::version::EngineVersion;
use retoc::zen_asset_conversion::{build_zen_asset, ConvertedZenAssetBundle};
use retoc::{UEPath, UEPathBuf};

// versions for UE 5.4 (Gothic 1 Remake):
let ver = EngineVersion::UE5_4;
let toc_ver = ver.toc_version();              // -> EIoStoreTocVersion::OnDemandMetaData
let hdr_ver = ver.container_header_version(); // -> EIoContainerHeaderVersion::NoExportInfo
let pkg_file_ver = ver.package_file_version();// -> FPackageFileVersion (UE5: PropertyTagCompleteTypeName)

// 1. Open the output container writer (.utoc + .ucas created here).
let mut writer = IoStoreWriter::new(
    "out/Mod.utoc",
    toc_ver,
    Some(hdr_ver),
    UEPathBuf::from("../../../"),   // mount point
)?;

// 2. Per asset: read legacy cooked bytes into FSerializedAssetBundle (pub struct, pub fields):
let bundle = FSerializedAssetBundle {
    asset_file_buffer: uasset_bytes,            // .uasset
    exports_file_buffer: uexp_bytes,            // .uexp
    bulk_data_buffer: ubulk_bytes,              // Option<Vec<u8>> .ubulk
    optional_bulk_data_buffer: None,            // .uptnl
    memory_mapped_bulk_data_buffer: None,       // .m.ubulk
};

// 3. Convert legacy -> zen.
let mut converted: ConvertedZenAssetBundle = build_zen_asset(
    bundle,
    &shader_maps,            // &HashMap<String, Vec<FSHAHash>>  (empty HashMap is fine)
    path,                    // &UEPath, e.g. "../../../Game/.../Asset.uasset"
    Some(pkg_file_ver),      // Option<FPackageFileVersion>
    hdr_ver,                 // EIoContainerHeaderVersion
    false,                   // allow_fixup (UE4-only external-arc fixup; false for UE5_4)
    None,                    // script_objects: Option<Arc<ZenScriptObjects>>
    None,                    // script_cells:   Option<Arc<ZenScriptCellsStore>>
    &Log::no_log(),          // &Log
)?;

// 4. Write the converted package + bulk chunks into the container, then finalize.
converted.write(&mut writer)?;   // pub fn write(&mut self, &mut IoStoreWriter)
writer.finalize()?;              // writes container header chunk + serializes the TOC
```

### Exact fn names + signatures Task 5 will call
- `IoStoreWriter::new<P: AsRef<Path>>(toc_path: P, toc_version: EIoStoreTocVersion, container_header_version: Option<EIoContainerHeaderVersion>, mount_point: UEPathBuf) -> Result<Self>`
  — `vendor/retoc/src/iostore_writer.rs:25`
- `zen_asset_conversion::build_zen_asset(legacy_asset: FSerializedAssetBundle, package_name_to_referenced_shader_maps: &HashMap<String, Vec<FSHAHash>>, path: &UEPath, package_version_fallback: Option<FPackageFileVersion>, container_header_version: EIoContainerHeaderVersion, allow_fixup: bool, script_objects: Option<Arc<ZenScriptObjects>>, script_cells: Option<Arc<ZenScriptCellsStore>>, log: &Log) -> Result<ConvertedZenAssetBundle>`
  — `vendor/retoc/src/zen_asset_conversion.rs:1318`
- `ConvertedZenAssetBundle::write(&mut self, writer: &mut IoStoreWriter) -> Result<()>`
  — `vendor/retoc/src/zen_asset_conversion.rs:1234` (calls `write_package_data` +
  `write_and_release_bulk_data`, both also `pub`)
- `IoStoreWriter::finalize(self) -> Result<()>` — `vendor/retoc/src/iostore_writer.rs:106`
- `EngineVersion::{toc_version, container_header_version, package_file_version}` —
  `vendor/retoc/src/version.rs:25/41/80`
- `FSerializedAssetBundle` (pub struct, all fields pub) — `vendor/retoc/src/legacy_asset.rs:1350`
- `Log::no_log() -> Log` — `vendor/retoc/src/logging.rs:101`

(`build_legacy` / `FZenPackageContext::create` in `asset_conversion.rs` are the zen→LEGACY
direction — the `to-legacy`/unpack path — NOT to-zen; listed in the read-path note.)

## 3. IMPORTANT caveat for Task 5: the writer does not compress yet

`IoStoreWriter::write_chunk` (`iostore_writer.rs:60-71`) currently writes every block
UNCOMPRESSED — it hardcodes `compression_method_index = 0` ("None") and copies the raw
block into the `.ucas`. So **the gore-oodle ENCODE arm wired in step 1 is correct and
tested but is NOT yet invoked by the current writer.** A to-zen run today produces a valid,
loadable but uncompressed container (UE loads uncompressed IoStore fine).

To actually exercise gore-oodle compression in the container, Task 5 (or a follow-up) must
extend `IoStoreWriter::write_chunk` to optionally route blocks through
`compression::compress(CompressionMethod::Oodle, block, &mut buf)`, register `Oodle` in the
TOC's `compression_methods` table, set the block's `compression_method_index` to its index,
and record the compressed size. The encode seam is ready; only the writer opt-in is missing.

## 4. No oo2core / oodle_loader anywhere in the vendor

`grep -rniE "oo2core|oodle_loader" vendor/retoc/src` → ZERO. The encode arm routes through
`gore_oodle::kraken_compress`; nothing fetches or links Epic's DLL. Nothing was un-trimmed,
so no new modules were added that could reintroduce it.

## 5. Build / test results
- `cargo build -p retoc` — clean (one pre-existing unrelated `private_interfaces` warning in
  `compact_binary.rs`).
- `cargo build -p gore-tex` — clean.
- `cargo test -p retoc oodle_encode_decode_round_trip` — `ok. 1 passed`.
- to-zen reachability: temporary compile-only check in `crates/gore-tex/tests/` compiled
  green from the external crate, then was removed.
