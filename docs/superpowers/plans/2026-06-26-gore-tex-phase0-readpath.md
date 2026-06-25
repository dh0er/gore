# gore-tex Phase 0 + Read Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the `gore-tex` crate, prove a `~mods` Zen-container override actually mounts in Gothic 1 Remake (de-risk spike), and ship a working read path (`gore texture list` + `gore texture extract`).

**Architecture:** New Rust crate `gore-tex` reuses a forked `retoc` for IoStore container I/O, with retoc's Oodle backend swapped to the in-repo `gore-oodle` (ooz Kraken) so no `oo2core` DLL is needed. The read path decodes cooked `UTexture2D` BCn data to PNG via `texture2ddecoder`. The write path (replace/pack/deploy) is deliberately deferred to a follow-up plan, because its exact byte-surgery code depends on real cooked-texture layouts and retoc internals discovered here.

**Tech Stack:** Rust, `retoc` (git dep, trumank), `gore-oodle` (in-repo ooz Kraken), `texture2ddecoder` (pure-Rust BCn decode), `clap` (CLI), real game at `D:\SteamLibrary\steamapps\common\Gothic 1 Remake`.

**Scope boundary:** This plan covers Phase 0 (foundation + de-risk spike) and Phase 1 (read path). It does NOT cover `replace`/`pack`/`deploy`/`undeploy` or the GUI — those get their own plan once the facts below land.

**Spec:** `docs/superpowers/specs/2026-06-26-gore-texture-replacement-design.md`

---

## File Structure

- `crates/gore-tex/Cargo.toml` — new crate manifest (deps: retoc fork, gore-oodle, texture2ddecoder, thiserror, anyhow).
- `crates/gore-tex/src/lib.rs` — crate root; re-exports `error`, `container`, `decode`.
- `crates/gore-tex/src/error.rs` — `TexError` (thiserror).
- `crates/gore-tex/src/container.rs` — retoc-fork glue: `list_textures`, `unpack_asset`.
- `crates/gore-tex/src/decode.rs` — cooked BCn → RGBA → PNG.
- `crates/gore-tex/src/paths.rs` — auto-resolve game container + `.usmap` from install dir.
- `crates/gore/src/main.rs` — add `Texture` subcommand group (modify).
- `crates/gore/src/cmd/texture.rs` — CLI handlers `list`/`extract` (new).
- `crates/gore/src/cmd/mod.rs` — register `texture` module (modify).
- `work/tex-fixtures/` — committed real cooked-texture fixture + expected PNG (created in Task 6).
- `docs/superpowers/notes/2026-06-26-retoc-oodle-swap.md` — investigation output (Task 2).
- `docs/superpowers/notes/2026-06-26-utexture2d-layout.md` — investigation output (Task 7, feeds the write-path re-plan).

---

## Task 1: Scaffold the `gore-tex` crate

**Files:**
- Create: `crates/gore-tex/Cargo.toml`
- Create: `crates/gore-tex/src/lib.rs`
- Create: `crates/gore-tex/src/error.rs`

- [ ] **Step 1: Write the crate manifest**

Create `crates/gore-tex/Cargo.toml`:

```toml
[package]
name = "gore-tex"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
gore-oodle = { path = "../gore-oodle" }
thiserror = "1"
anyhow = "1"
texture2ddecoder = "0.1"

[dev-dependencies]
sha2 = "0.10"
```

(retoc is added in Task 2 once its dependency form is confirmed — keep this task buildable standalone.)

- [ ] **Step 2: Write the error type**

Create `crates/gore-tex/src/error.rs`:

```rust
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum TexError {
    #[error("container not found: {0}")]
    ContainerNotFound(PathBuf),
    #[error("usmap mappings not found: {0}")]
    UsmapNotFound(PathBuf),
    #[error("asset not found in container: {0}")]
    AssetNotFound(String),
    #[error("unsupported pixel format: {0}")]
    UnsupportedFormat(String),
    #[error("virtual textures are not supported in v1: {0}")]
    VirtualTexture(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TexError>;
```

- [ ] **Step 3: Write the crate root**

Create `crates/gore-tex/src/lib.rs`:

```rust
//! Texture extraction/replacement for Gothic 1 Remake (UE5 IoStore).
pub mod decode;
pub mod error;
pub mod paths;

pub use error::{Result, TexError};
```

(`decode` and `paths` modules are filled in later tasks; create empty stubs so the crate compiles.)

Create `crates/gore-tex/src/decode.rs`:

```rust
//! Cooked BCn -> RGBA -> PNG decoding.
```

Create `crates/gore-tex/src/paths.rs`:

```rust
//! Auto-resolve the game container + .usmap from an install dir.
```

- [ ] **Step 4: Verify it builds**

Run: `cargo build -p gore-tex`
Expected: compiles clean (warnings about unused modules are OK).

- [ ] **Step 5: Commit**

```bash
git add crates/gore-tex
git commit -m "feat(gore-tex): scaffold crate with error type"
```

---

## Task 2: Investigate + wire retoc with the gore-oodle backend

This is an **investigation task** — its product is a working retoc dependency that uses `gore-oodle` instead of `oo2core`, plus a written note. retoc (https://github.com/trumank/retoc) loads Oodle at runtime via its `oodle_loader` module; we must redirect that to `gore-oodle::{kraken_compress, kraken_decompress}`.

**Files:**
- Modify: `crates/gore-tex/Cargo.toml` (add retoc dep)
- Create: `docs/superpowers/notes/2026-06-26-retoc-oodle-swap.md`

- [ ] **Step 1: Clone and read retoc's Oodle integration**

Run:
```bash
git clone https://github.com/trumank/retoc "$TEMP/retoc-src"
```
Read `$TEMP/retoc-src/src/oodle*.rs` (and `Cargo.toml`). Identify: (a) the public fn(s) retoc calls to compress/decompress a chunk, (b) whether the Oodle backend is swappable via a trait/feature or is a hard `oodle_loader` call, (c) the exact CLI/lib entry for "unpack one asset" and "to-zen".

- [ ] **Step 2: Decide the integration form and record it**

Write `docs/superpowers/notes/2026-06-26-retoc-oodle-swap.md` capturing:
- retoc's compress/decompress call sites (file:line).
- Chosen swap mechanism, one of:
  - **(a) Fork** retoc → replace `oodle_loader` calls with `gore-oodle`; depend on the fork via `git = "https://github.com/dh0er/retoc", branch = "gore-oodle"`.
  - **(b) Cargo patch** — if retoc factors Oodle behind a crate, `[patch]` that crate with a gore-oodle shim.
- The exact retoc lib API (or CLI invocation) for `unpack one asset` and `to-zen`.
- Confirmation that ooz Kraken at the default level produces chunks retoc/the game can decompress (compress a 256 KiB buffer with `gore_oodle::kraken_compress`, decompress with retoc's path, assert equal).

- [ ] **Step 3: Add the retoc dependency**

Modify `crates/gore-tex/Cargo.toml` `[dependencies]` per the decision (example for fork form):

```toml
retoc = { git = "https://github.com/dh0er/retoc", branch = "gore-oodle" }
```

- [ ] **Step 4: Verify the dependency builds and links gore-oodle**

Run: `cargo build -p gore-tex`
Expected: compiles; retoc present in `cargo tree -p gore-tex` and the build does NOT reference `oo2core`.

- [ ] **Step 5: Commit**

```bash
git add crates/gore-tex/Cargo.toml docs/superpowers/notes/2026-06-26-retoc-oodle-swap.md
git commit -m "feat(gore-tex): depend on retoc with gore-oodle backend"
```

---

## Task 3: Path auto-resolution

**Files:**
- Modify: `crates/gore-tex/src/paths.rs`
- Test: inline `#[cfg(test)]` in `paths.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/gore-tex/src/paths.rs`:

```rust
use std::path::{Path, PathBuf};
use crate::error::{Result, TexError};

/// Given a game install dir, return the main IoStore container `.utoc`.
pub fn main_container(game_dir: &Path) -> Result<PathBuf> {
    let p = game_dir.join("G1R/Content/Paks/G1R-Windows.utoc");
    if p.exists() { Ok(p) } else { Err(TexError::ContainerNotFound(p)) }
}

/// Given a game install dir, return the `.usmap` mappings file (first match).
pub fn usmap(game_dir: &Path) -> Result<PathBuf> {
    let dir = game_dir.join("G1R/Binaries/Win64/ue4ss");
    let found = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "usmap"));
    found.ok_or_else(|| TexError::UsmapNotFound(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_container_missing_dir_errors() {
        let err = main_container(Path::new("/no/such/game")).unwrap_err();
        assert!(matches!(err, TexError::ContainerNotFound(_)));
    }
}
```

- [ ] **Step 2: Run the test to verify it passes (pure logic, no fixture)**

Run: `cargo test -p gore-tex paths::`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/gore-tex/src/paths.rs
git commit -m "feat(gore-tex): game container + usmap path resolution"
```

---

## Task 4: `container::list_textures` over the real container

**Files:**
- Modify: `crates/gore-tex/src/container.rs` (create)
- Modify: `crates/gore-tex/src/lib.rs` (add `pub mod container;`)

- [ ] **Step 1: Define the listing type + function signature**

Create `crates/gore-tex/src/container.rs`:

```rust
use std::path::Path;
use crate::error::Result;

/// One texture asset found in a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureEntry {
    /// Full in-container asset path, e.g. "/Game/Characters/Hero/T_Hero_D".
    pub asset_path: String,
}

/// List texture assets in an IoStore container, using `usmap` to resolve types.
/// Filters to `UTexture2D`-class exports.
pub fn list_textures(utoc: &Path, usmap: &Path, filter: Option<&str>) -> Result<Vec<TextureEntry>> {
    // Implemented via the retoc lib API recorded in the Task 2 note:
    // open container -> iterate package store -> keep exports whose class is
    // Texture2D -> apply `filter` substring. See note for the exact retoc calls.
    let _ = (utoc, usmap, filter);
    todo!("wire retoc package iteration per task-2 note")
}
```

- [ ] **Step 2: Implement using the retoc API from the Task 2 note**

Replace the `todo!` body with the concrete retoc package-store iteration (exact calls recorded in `2026-06-26-retoc-oodle-swap.md` Step 1). Add `pub mod container;` to `lib.rs`.

- [ ] **Step 3: Write an integration test gated on the real game install**

Add to `container.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn game_dir() -> Option<PathBuf> {
        let p = PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        p.exists().then_some(p)
    }

    #[test]
    fn lists_textures_from_real_container() {
        let Some(g) = game_dir() else { eprintln!("skip: game not installed"); return; };
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let all = list_textures(&utoc, &usmap, None).unwrap();
        assert!(all.len() > 100, "expected many textures, got {}", all.len());
        let filtered = list_textures(&utoc, &usmap, Some("Hero")).unwrap();
        assert!(filtered.len() < all.len());
        assert!(filtered.iter().all(|e| e.asset_path.contains("Hero")));
    }
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p gore-tex container:: -- --nocapture`
Expected: PASS (or "skip: game not installed" on machines without the game — that's acceptable; CI does not have the game).

- [ ] **Step 5: Commit**

```bash
git add crates/gore-tex/src/container.rs crates/gore-tex/src/lib.rs
git commit -m "feat(gore-tex): list Texture2D assets from IoStore container"
```

---

## Task 5: `container::unpack_asset` — extract one cooked texture

**Files:**
- Modify: `crates/gore-tex/src/container.rs`

- [ ] **Step 1: Add the unpack function signature**

Add to `crates/gore-tex/src/container.rs`:

```rust
use std::path::PathBuf;

/// Unpack a single asset's cooked files (.uasset/.uexp/.ubulk) from the
/// container into `out_dir`. Returns the path to the `.uasset`.
pub fn unpack_asset(utoc: &Path, usmap: &Path, asset_path: &str, out_dir: &Path) -> Result<PathBuf> {
    // Implemented via the retoc unpack API recorded in the Task 2 note.
    let _ = (utoc, usmap, asset_path, out_dir);
    todo!("wire retoc single-asset unpack per task-2 note")
}
```

- [ ] **Step 2: Implement using the retoc unpack API**

Replace the `todo!` with retoc's single-asset extraction (exact calls from the Task 2 note). Decompresses chunks through `gore-oodle`.

- [ ] **Step 3: Integration test against the real container**

Add to the `tests` mod in `container.rs`:

```rust
#[test]
fn unpacks_one_texture_asset() {
    let Some(g) = game_dir() else { eprintln!("skip: game not installed"); return; };
    let utoc = crate::paths::main_container(&g).unwrap();
    let usmap = crate::paths::usmap(&g).unwrap();
    let first = list_textures(&utoc, &usmap, None).unwrap().remove(0);
    let tmp = std::env::temp_dir().join("gore-tex-unpack-test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let uasset = unpack_asset(&utoc, &usmap, &first.asset_path, &tmp).unwrap();
    assert!(uasset.exists());
    assert!(std::fs::metadata(&uasset).unwrap().len() > 0);
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p gore-tex container::tests::unpacks -- --nocapture`
Expected: PASS (or skip without the game).

- [ ] **Step 5: Commit**

```bash
git add crates/gore-tex/src/container.rs
git commit -m "feat(gore-tex): unpack a single cooked texture asset"
```

---

## Task 6: Create the committed test fixture

The read-path decode tests need a small, stable cooked-texture fixture in-repo (CI has no game). This task uses Task 5's `unpack_asset` to capture one real small texture.

**Files:**
- Create: `work/tex-fixtures/sample.uasset`, `sample.uexp`, `sample.ubulk` (real bytes)
- Create: `work/tex-fixtures/README.md` (provenance: which asset path, dims, format)

- [ ] **Step 1: Pick and unpack a small texture**

Run a throwaway binary or test that lists textures, picks a small 2D non-virtual one (e.g. a UI icon < 256x256), and unpacks it to `work/tex-fixtures/sample.*`. Record the asset path, SizeX/SizeY, and pixel format in `work/tex-fixtures/README.md`.

- [ ] **Step 2: Verify the fixture is self-contained and small**

Run: `ls -la work/tex-fixtures/`
Expected: three files present, total < ~1 MB. If the chosen texture's `.ubulk` is large, pick a smaller texture and redo.

- [ ] **Step 3: Commit the fixture**

```bash
git add work/tex-fixtures
git commit -m "test(gore-tex): add real cooked-texture fixture"
```

---

## Task 7: Investigate UTexture2D layout (feeds decode + the write-path re-plan)

**Investigation task.** Parse the fixture to extract the platform-data needed for decoding: pixel format, dims, top-mip byte range. Record the byte layout in a note — this is the foundation the write-path splicer plan will build on.

**Files:**
- Modify: `crates/gore-tex/src/decode.rs` (add a `PlatformData` parser)
- Create: `docs/superpowers/notes/2026-06-26-utexture2d-layout.md`

- [ ] **Step 1: Define the parsed type**

Add to `crates/gore-tex/src/decode.rs`:

```rust
use crate::error::{Result, TexError};

/// Minimal decoded platform data needed to turn a cooked texture into pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexInfo {
    pub width: u32,
    pub height: u32,
    /// UE pixel format name, e.g. "PF_DXT1", "PF_DXT5", "PF_BC5", "PF_BC7".
    pub format: String,
    /// Raw BCn bytes of mip 0 (largest), as stored on disk.
    pub mip0: Vec<u8>,
    pub is_virtual: bool,
}
```

- [ ] **Step 2: Implement the parser against the fixture**

Implement `pub fn parse(uasset: &[u8], uexp: &[u8], ubulk: &[u8], usmap: &[u8]) -> Result<TexInfo>`:
- Skip the unversioned property block using `usmap` to reach `DeserializeCookedPlatformData` (reference: CUE4Parse `UTexture2D.Deserialize` / `FTexturePlatformData`; field order documented in the design spec §Background).
- Read `SizeX`, `SizeY`, the `PixelFormat` FString, and the `Mips` array; for mip 0 resolve its `FByteBulkData` header (Flags/ElementCount/SizeOnDisk/OffsetInFile) to slice the bytes from `uexp` or `ubulk`.
- If `bIsVirtual`, return `TexError::VirtualTexture(format)`.
- If the format is not one of PF_DXT1/PF_DXT5/PF_BC5/PF_BC7, return `TexError::UnsupportedFormat`.

Record the exact observed offsets/field order (and any usmap-skip mechanism) in `docs/superpowers/notes/2026-06-26-utexture2d-layout.md`.

- [ ] **Step 3: Test against the fixture**

Add to `decode.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Vec<u8> {
        std::fs::read(format!("../../work/tex-fixtures/{name}")).unwrap()
    }

    #[test]
    fn parses_fixture_platform_data() {
        let usmap = std::fs::read(
            // committed copy of the usmap, or skip if absent
            "../../work/tex-fixtures/mappings.usmap",
        ).unwrap_or_default();
        if usmap.is_empty() { eprintln!("skip: no usmap fixture"); return; }
        let info = parse(&load("sample.uasset"), &load("sample.uexp"), &load("sample.ubulk"), &usmap).unwrap();
        // Values asserted against work/tex-fixtures/README.md provenance.
        assert!(info.width >= 4 && info.width % 4 == 0);
        assert!(info.format.starts_with("PF_"));
        assert!(!info.mip0.is_empty());
        assert!(!info.is_virtual);
    }
}
```

Copy the game's `.usmap` into `work/tex-fixtures/mappings.usmap` and commit it with the fixture if licensing allows (it is a generated mappings file, not game content). If it must not be committed, mark the test skip-on-absent as above.

- [ ] **Step 4: Run the test**

Run: `cargo test -p gore-tex decode::tests::parses -- --nocapture`
Expected: PASS (or skip if usmap fixture absent).

- [ ] **Step 5: Commit**

```bash
git add crates/gore-tex/src/decode.rs docs/superpowers/notes/2026-06-26-utexture2d-layout.md work/tex-fixtures/mappings.usmap
git commit -m "feat(gore-tex): parse cooked UTexture2D platform data"
```

---

## Task 8: Decode BCn → PNG

**Files:**
- Modify: `crates/gore-tex/src/decode.rs`

- [ ] **Step 1: Add the decode-to-RGBA function**

Add to `crates/gore-tex/src/decode.rs`:

```rust
/// Decode `TexInfo.mip0` to RGBA8 (width*height*4 bytes).
pub fn to_rgba8(info: &TexInfo) -> Result<Vec<u32>> {
    let (w, h) = (info.width as usize, info.height as usize);
    let mut out = vec![0u32; w * h];
    let r = match info.format.as_str() {
        "PF_DXT1" => texture2ddecoder::decode_bc1(&info.mip0, w, h, &mut out),
        "PF_DXT5" => texture2ddecoder::decode_bc3(&info.mip0, w, h, &mut out),
        "PF_BC5"  => texture2ddecoder::decode_bc5(&info.mip0, w, h, &mut out),
        "PF_BC7"  => texture2ddecoder::decode_bc7(&info.mip0, w, h, &mut out),
        other => return Err(TexError::UnsupportedFormat(other.to_string())),
    };
    r.map_err(|_| TexError::UnsupportedFormat(info.format.clone()))?;
    Ok(out)
}
```

(Confirm exact `texture2ddecoder` fn names/signatures against the crate docs; adjust the ARGB/RGBA channel order when writing PNG in Task 9.)

- [ ] **Step 2: Test decode produces the right pixel count**

Add to the `tests` mod in `decode.rs`:

```rust
#[test]
fn decode_fixture_to_rgba() {
    let usmap = std::fs::read("../../work/tex-fixtures/mappings.usmap").unwrap_or_default();
    if usmap.is_empty() { eprintln!("skip: no usmap fixture"); return; }
    let info = parse(&load("sample.uasset"), &load("sample.uexp"), &load("sample.ubulk"), &usmap).unwrap();
    let px = to_rgba8(&info).unwrap();
    assert_eq!(px.len(), (info.width * info.height) as usize);
}
```

- [ ] **Step 3: Run the test**

Run: `cargo test -p gore-tex decode::tests::decode_fixture -- --nocapture`
Expected: PASS (or skip).

- [ ] **Step 4: Commit**

```bash
git add crates/gore-tex/src/decode.rs
git commit -m "feat(gore-tex): decode cooked BCn mip to RGBA"
```

---

## Task 9: CLI `gore texture list` + `gore texture extract`

**Files:**
- Modify: `crates/gore/src/main.rs`
- Create: `crates/gore/src/cmd/texture.rs`
- Modify: `crates/gore/src/cmd/mod.rs`
- Modify: `crates/gore/Cargo.toml` (add `gore-tex`, `image` deps)

- [ ] **Step 1: Add deps to the `gore` binary**

Modify `crates/gore/Cargo.toml` `[dependencies]`:

```toml
gore-tex = { path = "../gore-tex" }
image = "0.25"
```

- [ ] **Step 2: Add the subcommand to the CLI enum**

In `crates/gore/src/main.rs`, add a variant to `enum Commands` (after `Package`):

```rust
    /// Extract/replace game textures (Gothic 1 Remake, UE5 IoStore)
    Texture {
        #[command(subcommand)]
        action: cmd::texture::TextureAction,
    },
```

And add the dispatch arm in `main()`:

```rust
        Commands::Texture { action } => cmd::texture::run(action),
```

- [ ] **Step 3: Write the command module**

Create `crates/gore/src/cmd/texture.rs`:

```rust
use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum TextureAction {
    /// List Texture2D assets in the game container
    List {
        /// Game install dir (auto-resolves container + usmap)
        #[arg(long)]
        game: PathBuf,
        /// Only show asset paths containing SUBSTR
        #[arg(long)]
        filter: Option<String>,
    },
    /// Extract a texture's top mip to a PNG
    Extract {
        /// Game install dir
        #[arg(long)]
        game: PathBuf,
        /// In-container asset path
        asset: String,
        /// Output PNG path
        #[arg(short = 'o', long)]
        out: PathBuf,
    },
}

pub fn run(action: TextureAction) -> Result<()> {
    match action {
        TextureAction::List { game, filter } => {
            let utoc = gore_tex::paths::main_container(&game)?;
            let usmap = gore_tex::paths::usmap(&game)?;
            for e in gore_tex::container::list_textures(&utoc, &usmap, filter.as_deref())? {
                println!("{}", e.asset_path);
            }
            Ok(())
        }
        TextureAction::Extract { game, asset, out } => {
            let utoc = gore_tex::paths::main_container(&game)?;
            let usmap = gore_tex::paths::usmap(&game)?;
            let tmp = std::env::temp_dir().join("gore-tex-extract");
            std::fs::create_dir_all(&tmp)?;
            let uasset = gore_tex::container::unpack_asset(&utoc, &usmap, &asset, &tmp)?;
            let uexp = uasset.with_extension("uexp");
            let ubulk = uasset.with_extension("ubulk");
            let info = gore_tex::decode::parse(
                &std::fs::read(&uasset)?,
                &std::fs::read(&uexp)?,
                &std::fs::read(&ubulk).unwrap_or_default(),
                &std::fs::read(&usmap)?,
            )?;
            let px = gore_tex::decode::to_rgba8(&info)?;
            // Convert u32 ARGB -> RGBA8 bytes (channel order per texture2ddecoder).
            let mut buf = Vec::with_capacity(px.len() * 4);
            for p in px {
                buf.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8]);
            }
            image::save_buffer(&out, &buf, info.width, info.height, image::ColorType::Rgba8)
                .with_context(|| format!("writing {}", out.display()))?;
            // Sidecar for round-trip (format/dims) — consumed by the future `replace`.
            let sidecar = out.with_extension("png.json");
            std::fs::write(&sidecar, format!(
                "{{\"asset\":\"{}\",\"width\":{},\"height\":{},\"format\":\"{}\"}}",
                asset, info.width, info.height, info.format))?;
            println!("wrote {} ({}x{} {})", out.display(), info.width, info.height, info.format);
            Ok(())
        }
    }
}
```

- [ ] **Step 4: Register the module**

In `crates/gore/src/cmd/mod.rs`, add:

```rust
pub mod texture;
```

- [ ] **Step 5: Build the whole workspace**

Run: `cargo build`
Expected: clean build; `gore texture --help` lists `list` and `extract`.

Run: `cargo run -p gore -- texture --help`
Expected: shows the two subcommands.

- [ ] **Step 6: Manual smoke test against the real game**

Run:
```bash
cargo run -p gore -- texture list --game "D:/SteamLibrary/steamapps/common/Gothic 1 Remake" --filter Hero
cargo run -p gore -- texture extract --game "D:/SteamLibrary/steamapps/common/Gothic 1 Remake" "<one listed path>" -o hero.png
```
Expected: list prints asset paths; extract writes a viewable `hero.png` + `hero.png.json`. Open the PNG to confirm it looks like a real game texture.

- [ ] **Step 7: Commit**

```bash
git add crates/gore/Cargo.toml crates/gore/src/main.rs crates/gore/src/cmd/texture.rs crates/gore/src/cmd/mod.rs
git commit -m "feat(gore): add 'texture list' and 'texture extract' CLI"
```

---

## Task 10: Phase 0 de-risk spike — magenta-swatch in-game (USER-GATED)

**This is the gate for the entire write path.** It proves a `~mods` Zen container actually overrides a base-container asset in this game. It uses retoc round-trip + a hand-edited mip; it does NOT need our splicer. The user must launch the game — it cannot run headless.

**Files:** none committed (throwaway), but record the outcome in `docs/superpowers/notes/2026-06-26-magenta-spike-result.md`.

- [ ] **Step 1: Pick an obvious always-visible UI texture**

Use `gore texture list --filter UI` (or `--filter Menu`/`HUD`) to find a main-menu/HUD texture. Extract it to confirm it's the right one visually.

- [ ] **Step 2: Produce a solid-color override of identical format + dims**

Unpack the chosen asset (`unpack_asset`). Replace its mip-0 `.ubulk`/`.uexp` BCn bytes with a solid magenta block pattern (for BC1: repeat the 8-byte block encoding RGB(255,0,255); for BC7: a solid-color block). Keep all sizes identical (in-place byte overwrite, same length) so no offset bookkeeping is needed for the spike.

- [ ] **Step 3: Repack to a Zen triplet via retoc to-zen**

Using retoc (CLI or the lib path from the Task 2 note), build a legacy pak from the single edited asset folder, then `to-zen` it with `--version UE5_4`, naming the output `zzz_MagentaTest_P.{utoc,ucas,pak}`.

- [ ] **Step 4: Deploy to ~mods (USER confirms before copying into the game)**

Copy the triplet into `D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\Paks\~mods\`. (This writes into the game install — get explicit user OK first; it is fully reversible by deleting the three files.)

- [ ] **Step 5: USER launches the game and looks**

Ask the user to launch the deep exe `G1R\Binaries\Win64\G1R-Win64-Shipping.exe` and check whether the chosen UI element is magenta. We cannot do this headless.

- [ ] **Step 6: Record the result + clean up**

Write `docs/superpowers/notes/2026-06-26-magenta-spike-result.md` with: PASS/FAIL, the asset used, and any extra steps needed (e.g. SimpleModLoader required, load-order prefix). Delete the test triplet from `~mods`.

```bash
git add docs/superpowers/notes/2026-06-26-magenta-spike-result.md
git commit -m "docs(gore-tex): record magenta-swatch de-risk spike result"
```

- [ ] **Step 7: Decision gate**

- **PASS** → the write-path re-plan is unblocked (replace/pack/deploy).
- **FAIL** → STOP. Re-plan delivery (investigate SimpleModLoader plugin requirement, load order, or a UE4SS runtime-override fallback) before building the splicer.

---

## Next: write-path re-plan

After Tasks 1–10 land, write a follow-up plan
`docs/superpowers/plans/YYYY-MM-DD-gore-tex-writepath.md` covering Phase 1 (write) +
Phase 2 (deploy) using the now-known facts:
- `2026-06-26-utexture2d-layout.md` (exact splice offsets + bulk-header rewrite).
- `2026-06-26-retoc-oodle-swap.md` (to-zen lib/CLI form).
- `2026-06-26-magenta-spike-result.md` (delivery requirements).
Tasks there: `encode` (intel_tex_2 BCn + mip regen), `texasset::splice` with a
**byte-faithful round-trip test** (parse→serialize unchanged == original sha256),
`container::pack`/`to-zen`, and `deploy`/`undeploy` with a JSON record. GUI = a
separate spec/plan (Phase 3).

---

## Self-Review

- **Spec coverage (this plan's scope):** crate scaffold ✔ (T1); retoc+gore-oodle, no oo2core ✔ (T2); path auto-resolve ✔ (T3); `list` ✔ (T4,T9); `extract`+sidecar ✔ (T5,T8,T9); VT reject + unknown-format hard error ✔ (T7 error arms); Phase 0 spike gate ✔ (T10). Out-of-scope-by-design: `replace`/`pack`/`deploy`/GUI → deferred re-plan (stated).
- **Placeholder scan:** the two `todo!()`s (T4 Step 1, T5 Step 1) are *immediately* replaced in the next step of the same task using the Task 2 investigation output — they are scaffolding for a two-step (signature-then-impl) task, not unfilled work. All test code is concrete.
- **Type consistency:** `TexError`/`Result` (T1) used throughout; `TextureEntry.asset_path` (T4) consumed in T9; `TexInfo{width,height,format,mip0,is_virtual}` (T7) consumed by `to_rgba8` (T8) and the CLI (T9); `paths::main_container`/`paths::usmap` (T3) used in T4,T5,T9.
