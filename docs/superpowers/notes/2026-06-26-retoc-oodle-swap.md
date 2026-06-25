# retoc + gore-oodle: swapping Epic's Oodle DLL for the in-repo codec

Task 2 of the gore-tex plan. Goal: make `retoc` (pure-Rust IoStore↔legacy tool)
usable as a dependency of `gore-tex` while decompressing Oodle chunks with the
in-repo `gore-oodle` (ooz Kraken) codec instead of Epic's proprietary
`oo2core_9_win64.dll`. Gothic 1 Remake (UE 5.4.3) ships no such DLL.

Upstream pinned: `trumank/retoc` @ `d7b635039c3db60942efabcd29d49679f42ab089`.

## 1. retoc's Oodle decompress call sites

retoc loads Oodle **at runtime** via `oodle_loader::oodle()` (from
`trumank/repak`), which `fetch`es + `libloading::Library::new`s
`oo2core_9_win64.dll`. The backend is **NOT swappable** upstream — no trait, no
feature flag, no callback. There are exactly two call sites, both in the `retoc`
library crate, file `retoc/src/compression.rs`:

- **`compression.rs:34` (ENCODE)** — `compress()` Oodle arm:
  `oodle_loader::oodle()?.compress(input, Compressor::Mermaid, CompressionLevel::Normal)`
- **`compression.rs:53` (DECODE)** — `decompress()` Oodle arm:
  `let status = oodle_loader::oodle()?.decompress(input, output);`
  where `decompress(&self, input: &[u8], output: &mut [u8]) -> isize`. `output`
  is pre-sized by the caller to the exact uncompressed block length (computed
  from the TOC compression-block entry). Success ⇔ `status == output.len()`.

`compression::decompress` is the single choke point for **all** IoStore reads:
`Toc::read` (`retoc/src/lib.rs:702`) calls it per compression block, and every
high-level read (`IoStoreTrait::read`, `ChunkInfo::read`, `packages()`,
`to-legacy`, `unpack`, `verify`) funnels through `Toc::read`. So redirecting
this one arm covers the entire decode path.

The only other Oodle reference is `repak::Compression::Oodle` in
`retoc_cli/src/main.rs` (the `to-legacy → .pak` writer) — **not** in the library,
and not needed by gore-tex.

## 2. retoc's library API (it is a real lib, not CLI-only)

`retoc` is a Cargo workspace: lib crate `retoc/` + binary `retoc_cli/` +
`load_logger/`. The lib exposes everything gore-tex needs for T4/T5:

Entry point — open a container or a directory of containers
(`iostore::open`, `vendor/retoc/src/iostore.rs:58`, signature
`open<P: AsRef<Path>>(path: P, config: Arc<Config>) -> Result<Box<dyn IoStoreTrait>>`):
```rust
use std::sync::Arc;
use retoc::{Config, iostore};
let store: Box<dyn iostore::IoStoreTrait> =
    iostore::open("…/G1R-Windows.utoc", Arc::new(Config::default()))?;
```
`Config` is `pub struct Config` (`lib.rs:184`) re-exported at the crate root, and
`Config::default()` is fine for G1R (unencrypted + unsigned; no AES key needed).

`IoStoreTrait` (`retoc/src/iostore.rs:81`) key methods:
- `chunks() -> impl Iterator<Item = ChunkInfo>` — unique chunks (deduped across
  patch containers); `chunks_all()` includes patch-overridden ones.
- `packages() -> impl Iterator<Item = PackageInfo>` — one per asset package.
- `read(chunk_id: FIoChunkId) -> Result<Vec<u8>>` — fully decoded chunk bytes.
- `read_raw(chunk_id_raw: FIoChunkIdRaw) -> Result<Vec<u8>>`.
- `has_chunk_id(id)`, `chunk_path(id) -> Option<String>` (absolute mount path),
  `package_store_entry(pkg_id)`, `container_file_version()`,
  `container_header_version()`, `load_script_objects()`.

`ChunkInfo` (`iostore.rs:121`): `.id() -> FIoChunkId`, `.size() -> u64`,
`.path() -> Option<String>`, `.hash() -> &FIoChunkHash`, `.read() -> Result<Vec<u8>>`.

`PackageInfo` (`iostore.rs:160`): `.id() -> FPackageId`, `.container()`.

### How T4 (list textures) will use it
The container holds *cooked* assets; Texture2D class info lives inside the zen
package header, not in chunk metadata. Two routes — **both use only `pub` API
verified reachable from an external crate:**

**Package route (authoritative, recommended).** `PackageInfo` exposes only
`.id() -> FPackageId` and `.container()` (`iostore.rs:161-168`) — there is **no**
`PackageInfo::path()`/`::name()`. The package name lives *inside* the zen header,
so to map a package to a name you read its header chunk and call the public
`zen::get_package_name`:
```rust
use retoc::{EIoChunkType, FIoChunkId};
use retoc::iostore::IoStoreTrait;

let hdr_ver = store.container_header_version().expect("container has header"); // EIoContainerHeaderVersion
for pkg in store.packages() {                       // pkg: PackageInfo
    let id = FIoChunkId::from_package_id(pkg.id(), 0, EIoChunkType::ExportBundleData);
    let data = store.read(id)?;                      // fully decoded zen package bytes
    let name = retoc::zen::get_package_name(&data, hdr_ver)?; // pub fn, zen.rs:21
    // For the class filter, parse the full header instead of just the name:
    //   let header = retoc::zen::FZenPackageHeader::deserialize(
    //       &mut std::io::Cursor::new(&data),
    //       store.package_store_entry(pkg.id()),     // Option<StoreEntry>
    //       store.container_file_version().unwrap(), // EIoStoreTocVersion
    //       hdr_ver,                                 // EIoContainerHeaderVersion
    //       None,                                    // package_version_override: Option<FPackageFileVersion>
    //   )?;
    // then inspect header.exports / export class names for Texture2D/TextureCube/etc.
}
```
Verified `pub`/reachable: `IoStoreTrait::{packages, read, container_header_version,
container_file_version, package_store_entry}`, `PackageInfo::id`,
`FIoChunkId::from_package_id` (`lib.rs:882`), `EIoChunkType` (`lib.rs:1239`),
`zen::get_package_name` (`zen.rs:21`), `FZenPackageHeader::deserialize` (`zen.rs:871`).
The export-class introspection on the parsed `FZenPackageHeader` is **not yet
spelled out as verified copy-paste** — the exact public accessors for export
class names still need to be confirmed against `zen.rs` when T4 is implemented;
treat the `deserialize` call above as verified and the per-export class extraction
as TODO-verify.

**Asset-registry route (cheap, if `AssetRegistry.bin` is present).**
`asset_registry::AssetRegistry::deserialize(stream)` (`asset_registry.rs:648`, pub)
yields `AssetRegistry` whose `asset_data[].asset_class` (`asset_registry.rs:243/308`)
gives the class (e.g. `Texture2D`) per asset path. You still locate the registry
chunk via the package/chunk API above, then `Cursor::new(bytes)` into `deserialize`.

### How T5 (unpack one asset) will use it
**Do not use `FPackageId::from_name`** — it is **private** (`lib.rs:731`, no `pub`)
and so is the `lower_utf16_cityhash` helper it wraps, so neither is callable from
gore-tex. Resolve the target `FPackageId` instead by **scanning `store.packages()`
and matching the package name** (same loop as T4), which uses only public API:
```rust
use retoc::{EIoChunkType, FIoChunkId, FPackageId};
use retoc::iostore::IoStoreTrait;

let hdr_ver = store.container_header_version().expect("container has header");
let target = "/Game/Path/To/Asset"; // package path WITHOUT extension

// 1. Find the FPackageId for the target path.
let mut pkg_id: Option<FPackageId> = None;
for pkg in store.packages() {
    let head = FIoChunkId::from_package_id(pkg.id(), 0, EIoChunkType::ExportBundleData);
    let name = retoc::zen::get_package_name(&store.read(head)?, hdr_ver)?;
    if name == target { pkg_id = Some(pkg.id()); break; }
}
let pkg = pkg_id.expect("package not found");

// 2. Read its cooked parts by chunk type (header+exports, then optional bulk).
let uasset = store.read(FIoChunkId::from_package_id(pkg, 0, EIoChunkType::ExportBundleData))?;
let ubulk_id = FIoChunkId::from_package_id(pkg, 0, EIoChunkType::BulkData);
let ubulk = store.has_chunk_id(ubulk_id).then(|| store.read(ubulk_id)).transpose()?;
// OptionalBulkData / MemoryMappedBulkData are pulled the same way if has_chunk_id.
```
`FPackageId` is `pub struct FPackageId(pub u64)` (`lib.rs:713`) — its `u64` field is
public, so if you already hold a raw id you can build one with `FPackageId(raw)`,
and you can recover one from any chunk via `FIoChunkId::get_package_id()`
(`lib.rs:911`, pub). What you **cannot** do externally is hash a *path string* into
an id (that path is the private `from_name`); hence the name-scan above.

Verified `pub`/reachable here: `FIoChunkId::{from_package_id, get_package_id}`,
`EIoChunkType`, `FPackageId(pub u64)`, `IoStoreTrait::{packages, read, has_chunk_id,
container_header_version}`, `PackageInfo::id`, `zen::get_package_name`.

Note: in zen/IoStore form the `.uasset` header and `.uexp` exports are a single
`ExportBundleData` chunk (the bytes above are the *zen* cooked form, not legacy
split). Producing legacy split `.uasset`/`.uexp` requires the zen→legacy conversion
`asset_conversion::build_legacy(package_context, package_id, out_path, file_writer)`
(`asset_conversion.rs:1485`, pub). That call needs a `FZenPackageContext`, built via
the pub `FZenPackageContext::create(store_access, fallback_package_file_version,
log, script_cells)` (`asset_conversion.rs:48`); its `&Log` and
`Option<Arc<ZenScriptCellsStore>>` arguments still need their construction verified
against `vendor/retoc/src/` when T5's legacy-split path is implemented — treat
`build_legacy`/`FZenPackageContext::create` as **reachable but not yet
copy-paste-verified end to end.**

### CLI equivalents (for reference / shelling out, NOT how we integrate)
These are the *upstream* CLI verbs. **The CLI (`retoc_cli` / `src/main.rs`) was
dropped from our vendor** (see §3), so these commands do not exist in our tree and
none of the recipes above depend on them. They are listed only to describe the
equivalent operations; the canonical source is upstream
`https://github.com/trumank/retoc` @ `d7b6350` `src/main.rs` (the `action_*`
handlers, e.g. `action_dump_test`), which is **upstream-only, not in this repo.**
- List chunks w/ paths + sizes: `retoc list <utoc> --path --size --package`
- Unpack all files via directory index: `retoc unpack <utoc> <out_dir> [-v]`
- Dump one package's cooked parts: `retoc dump-test <utoc> <out_dir> <package_id>`
- Get one chunk raw: `retoc get <utoc> <chunk_id_hex> [out|-]`
- Zen→legacy (all): `retoc to-legacy <utoc-or-dir> <out_dir|out.pak> [--filter NAME]`
- **Legacy→zen repack:** `retoc to-zen <input_dir|in.pak> <out.utoc> --version UE5_4`
  (the `--version UE5_4` selects UE 5.4 TOC + container-header versions; expects a
  cooked dir of `.uasset`+`.uexp`(+`.ubulk`) and emits `.utoc`/`.ucas` and an
  empty `.pak`.)

## 3. Chosen integration form: vendored fork as a path library dep

**Decision: vendor a trimmed fork of the `retoc` *library* crate under
`vendor/retoc/` and depend on it from `crates/gore-tex/Cargo.toml` by path.**

Why this over the alternatives:
- **Not a Cargo `[patch]`:** the swap isn't a version bump of a published crate;
  it changes source (the Oodle arm) and drops a dependency (`oodle_loader`) +
  a cargo feature (`repak/oodle`). `[patch]` can't drop a feature another crate
  turned on, and retoc isn't on crates.io.
- **Not a feature/callback PR:** upstream has no Oodle-backend seam; adding one
  is out of scope for this task.
- **Not a pushed fork branch:** this environment can't reliably push to a remote;
  a path-vendored copy builds deterministically here and pins the exact source.
- **Lib, not CLI shell-out:** the `retoc` lib API (above) is rich and public, so
  gore-tex links it directly — no subprocess, no temp files, no oo2core anywhere.

What was modified in the vendor (see `vendor/retoc/Cargo.toml` header comment):
1. Copied only the `retoc` lib crate (dropped `retoc_cli`, `load_logger`, and the
   92 MB `tests/` fixture dir — gore-tex doesn't run retoc's internal tests).
2. Rewrote `Cargo.toml` to be standalone (no `*.workspace = true` inheritance,
   since it sits outside the gore workspace's `members = ["crates/*"]`):
   - **removed** `oodle_loader` dependency entirely;
   - **removed** the `oodle` feature from `repak` (that feature is what made repak
     fetch/link oo2core; the IoStore read path doesn't need it);
   - **added** `gore-oodle = { path = "../../crates/gore-oodle" }`.
3. `src/compression.rs`:
   - **DECODE** Oodle arm now calls
     `gore_oodle::kraken_decompress(input, output.len())` and
     `output.copy_from_slice(&decoded)`. `output.len()` is the exact uncompressed
     block size, which is precisely `gore-oodle`'s `expected_size` contract.
   - **ENCODE** Oodle arm `bail!`s "not supported (decode only)" — see §4.

The remaining git deps (`repak` w/o oodle, `ser-hex`, `jmap`) are pure-Rust and
pull no Oodle DLL. `cargo tree -p gore-tex` confirms `retoc` present and
**zero** `oo2core` / `oodle_loader` nodes.

## 4. How the ENCODE / `to-zen` backend swap will be done later (OUT OF SCOPE now)

The write path (`to-zen` repack) is not needed for list/unpack and is not wired
here. When it's implemented:

- The single encode choke point is the same file/arm: `compression.rs:34`
  (`compress()` Oodle arm). Repointing it to `gore_oodle::kraken_compress(input, level)`
  is the symmetric change to what we did for decode.
- **Caveat (documented, not yet solved):** `gore-oodle`'s ooz Kraken encoder is
  byte-identical to Epic's Oodle **only at the default level**; it is capped at
  `MAX_SAFE_COMPRESS_LEVEL = 5` and cannot reproduce Epic's higher-compression
  output. UE itself does not require byte-identical re-compression to *load* a
  container (it just needs valid Oodle-decodable blocks at the advertised sizes),
  so a gore-oodle-encoded container should be loadable — but this must be
  validated against the game before relying on it. The encode arm therefore
  `bail!`s today rather than silently emitting blocks we haven't verified the
  game accepts.
- Also note the CLI-only `repak::Compression::Oodle` path
  (`retoc_cli` `to-legacy → .pak`) — irrelevant to gore-tex (we don't vendor the
  CLI and don't write `.pak`s), but if a future task needs legacy `.pak` output
  with Oodle, repak's own Oodle hook would need the same gore-oodle redirection.

## 5. Proof of decode (against the real shipped container)

Scratch example `crates/gore-tex/examples/decode_real_container.rs` opens the
real `D:/SteamLibrary/.../G1R/Content/Paks/G1R-Windows.utoc`
(115,510 chunks, toc version `OnDemandMetaData`, header `NoExportInfo`),
reads a 65-chunk sample spanning the container, decodes each through the
gore-oodle-backed path, and checks the decoded bytes against the blake3 chunk
hash stored in the TOC meta (same check as retoc's `verify`). Result:

```
verified 65 sampled chunks (blake3 matched TOC meta)
decoded 24861541 bytes total, largest chunk 12005348 bytes
PROOF OK: gore-oodle decode path is byte-identical for this container
```

A blake3 match on the decoded output proves the gore-oodle decode is
byte-identical to whatever Epic's Oodle produced at cook time. Run it with:
```
cargo run -p gore-tex --example decode_real_container
```
