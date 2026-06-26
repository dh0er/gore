# Texture Tab + Mod-Bundle Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Textures" tab to mod-studio that browses/previews game textures via a cached index and stages `{asset→PNG}` replacements, flowing through the existing unified `gore-mod` Build/Deploy pipeline as a first-class `TexturePatch` bundle component (one Zen triplet → `~mods`).

**Architecture:** A cached texture index (`{asset_path: package_id}`) makes browse/preview fast (direct chunk read by package_id, no scan). The bundle carries source PNGs; deploy cooks (`replace_texture`) + packs (`repack_to_zen`) one triplet into `~mods`, tracked additively in the deploy record (not `*.gore-bak`). The Flutter tab mirrors the Audio vertical slice; `ModProject.toBuildSpec()` is the single integration chokepoint.

**Tech Stack:** Rust (`gore-tex`, `gore-mod`, `gore-ffi`, vendored `retoc`), Flutter/Riverpod (`apps/mod-studio`), real game at `D:\SteamLibrary\steamapps\common\Gothic 1 Remake`.

**Spec:** `docs/superpowers/specs/2026-06-26-texture-tab-bundle-integration-design.md`

**Spec deviation (intentional):** the index stores `asset_path → package_id` only. Width/height/format are NOT indexed (they'd require unpacking all 13k textures). Dimensions/format are surfaced on preview (from the extracted `TexInfo`).

---

## File Structure

**Rust:**
- `crates/gore-tex/src/index.rs` — NEW: `TextureIndex`, `build_index`, `extract_by_package_id`, cache load/save.
- `crates/gore-tex/src/container.rs` — MODIFY: add `unpack_asset_by_id` (scan-free unpack by package_id).
- `crates/gore-tex/src/lib.rs` — MODIFY: `pub mod index;`.
- `crates/gore-tex/src/paths.rs` — MODIFY: add `texture_index_path()` (shared gore-tools dir).
- `crates/gore/src/cmd/texture.rs` — MODIFY: add `Index` subcommand.
- `crates/gore/src/main.rs` — MODIFY: add `TextureAction::Index` arm.
- `crates/gore-mod/src/lib.rs` — MODIFY: `TextureReplacement`, `BuildSpec.texture`, `Component::TexturePatch`, build arm, `DeployPlan.texture_triplets`, `DeployRecord.texture_triplets`, prepare/apply/undeploy arms. Add `gore-tex` dep.
- `crates/gore-mod/Cargo.toml`, `crates/gore-ffi/Cargo.toml` — MODIFY: add `gore-tex`.
- `crates/gore-ffi/src/lib.rs` — MODIFY: `texture_index`, `texture_extract` commands.

**Flutter (`apps/mod-studio/lib/`):**
- `textures/domain/texture_replacements_notifier.dart` — NEW.
- `textures/domain/texture_index_provider.dart` — NEW.
- `textures/ui/texture_tab.dart` — NEW.
- `core/mod_ffi.dart` — MODIFY: `textureIndex`, `textureExtract`.
- `home_page.dart` — MODIFY: 6th tab + dirty flag.
- `project/project_model.dart` — MODIFY: `textures` field.
- `project/project_io.dart` — MODIFY: embed/extract PNGs.
- `project/project_controller.dart` — MODIFY: gather/apply/new/dirty.
- `export/build_deploy_dialog.dart` — MODIFY: content count.

---

## Phase 1 — gore-tex index + fast extract

### Task 1: Index types + cache path

**Files:** Create `crates/gore-tex/src/index.rs`; modify `crates/gore-tex/src/lib.rs`, `crates/gore-tex/src/paths.rs`.

- [ ] **Step 1: Add the cache-path helper.** In `crates/gore-tex/src/paths.rs`, the crate currently has no shared-dir helper. Add a function that returns the index cache path next to the loc catalog. The shared dir is owned by `gore-loc`; reference it. Add to `crates/gore-tex/Cargo.toml` under `[dependencies]`: `gore-loc = { path = "../gore-loc" }`. Then add to `paths.rs`:
```rust
/// The shared gore-tools cache path for the texture index (next to loc_catalog.json).
pub fn texture_index_path() -> PathBuf {
    gore_loc::paths::shared_data_dir().join("texture_index.json")
}
```
(Verify `gore_loc::paths::shared_data_dir() -> PathBuf` exists — it's used by `gore-ffi` as `paths::shared_data_dir()`. If the path is `gore_loc::paths::shared_data_dir`, use that; confirm the exact module path by checking `crates/gore-loc/src/paths.rs`.)

- [ ] **Step 2: Write the index module with types + a round-trip test.** Create `crates/gore-tex/src/index.rs`:
```rust
//! Cached texture index: asset_path -> package_id, for instant search + scan-free extract.

use std::collections::BTreeMap;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::error::Result;

/// Maps each Texture2D asset path to its IoStore package id (u64). Built once per game
/// build (a full container scan); cached to the shared gore-tools dir.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextureIndex {
    /// Identifies the game build the index was built against (the .usmap filename), so a
    /// game update invalidates a stale cache.
    pub build_id: String,
    /// asset_path -> package_id
    pub entries: BTreeMap<String, u64>,
}

impl TextureIndex {
    pub fn to_json(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self).map_err(|e| crate::error::TexError::Io(
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())))?)
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| crate::error::TexError::Io(
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())))
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self> {
        Self::from_json(&std::fs::read(path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn index_json_roundtrips() {
        let mut idx = TextureIndex { build_id: "G1R-5.4.3".into(), entries: BTreeMap::new() };
        idx.entries.insert("/Game/UI/T_X".into(), 0x1122334455667788);
        let back = TextureIndex::from_json(&idx.to_json().unwrap()).unwrap();
        assert_eq!(idx, back);
    }
}
```
(Check `crate::error::TexError` has an `Io(std::io::Error)` variant — it does, per `error.rs`. The `serde` mapping above avoids needing a new `Json` variant; if `TexError` already has a serde-friendly variant, use it.)

- [ ] **Step 3: Register the module.** In `crates/gore-tex/src/lib.rs` add `pub mod index;`.

- [ ] **Step 4: Build + test.** Run: `cargo test -p gore-tex index::`
Expected: PASS (`index_json_roundtrips`).

- [ ] **Step 5: Commit.**
```bash
git add crates/gore-tex/src/index.rs crates/gore-tex/src/lib.rs crates/gore-tex/src/paths.rs crates/gore-tex/Cargo.toml Cargo.lock
git commit -m "feat(gore-tex): texture index types + cache path"
```

### Task 2: `build_index` (one scan → path→package_id)

**Files:** Modify `crates/gore-tex/src/index.rs`.

- [ ] **Step 1: Add `build_index` mirroring `list_textures`' scan.** `list_textures` (container.rs:52) already iterates `store.packages()`, reads each ExportBundleData chunk, deserializes the zen header, checks the Texture2D class, and gets `header.package_name()`. `build_index` is the same loop but captures `pkg.id()` (a `FPackageId`) → `u64` alongside the path. The `FPackageId` raw u64: per the retoc note, `FPackageId` has a `pub` tuple field — use `pkg.id().0` (verify the field; the swap note says `FPackageId(pub u64)`). Add to `index.rs`:
```rust
use std::io::Cursor;
use std::sync::Arc;
use retoc::iostore;
use retoc::script_objects::FPackageObjectIndex;
use retoc::zen::FZenPackageHeader;
use retoc::{Config, EIoChunkType, FIoChunkId};

/// Build the index by scanning the container once (same walk as `list_textures`,
/// additionally capturing each Texture2D package's id). `build_id` identifies the
/// game build (pass the .usmap filename).
pub fn build_index(utoc: &Path, build_id: &str) -> Result<TextureIndex> {
    let store = iostore::open(utoc, Arc::new(Config::default()))?;
    let texture2d = FPackageObjectIndex::create_script_import("/Script/Engine.Texture2D");
    let cv = store.container_file_version().ok_or_else(|| anyhow::anyhow!("no TOC version"))?;
    let hv = store.container_header_version().ok_or_else(|| anyhow::anyhow!("no header version"))?;
    let mut entries = BTreeMap::new();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for pkg in store.packages() {
        let pkg_id = pkg.id();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let cid = FIoChunkId::from_package_id(pkg_id, 0, EIoChunkType::ExportBundleData);
            let data = store.read(cid).ok()?;
            let header = FZenPackageHeader::deserialize(
                &mut Cursor::new(&data), store.package_store_entry(pkg_id), cv, hv, None).ok()?;
            if !header.export_map.iter().any(|e| e.class_index == texture2d) { return None; }
            Some(header.package_name())
        }));
        if let Ok(Some(path)) = result {
            entries.insert(path, pkg_id.0);
        }
    }
    std::panic::set_hook(prev_hook);
    Ok(TextureIndex { build_id: build_id.to_string(), entries })
}
```
Add `anyhow` to `gore-tex/Cargo.toml` if not already a dep (it is — `container.rs` uses `anyhow::anyhow!`).

- [ ] **Step 2: Add a gated real-container test.** Append to `index.rs` tests:
```rust
    fn game_dir() -> Option<std::path::PathBuf> {
        let p = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        p.exists().then_some(p)
    }
    #[test]
    #[ignore = "slow: full container scan"]
    fn builds_index_from_real_container() {
        let Some(g) = game_dir() else { eprintln!("skip: game not installed"); return; };
        let utoc = crate::paths::main_container(&g).unwrap();
        let idx = build_index(&utoc, "test-build").unwrap();
        assert!(idx.entries.len() > 10000, "expected ~13k textures, got {}", idx.entries.len());
        // A known asset resolves to a non-zero package id.
        let pid = idx.entries.get("/Game/UI/Textures/Common/T_HardwareCursor");
        assert!(pid.is_some() && *pid.unwrap() != 0);
    }
```

- [ ] **Step 3: Run the gated test once.** Run: `cargo test -p gore-tex index::tests::builds_index -- --ignored --nocapture`
Expected: PASS (>10000 entries; cursor resolves). ~5-8 min.

- [ ] **Step 4: Commit.**
```bash
git add crates/gore-tex/src/index.rs
git commit -m "feat(gore-tex): build_index scans container to asset->package_id map"
```

### Task 3: `unpack_asset_by_id` (scan-free) + `extract_by_package_id`

**Files:** Modify `crates/gore-tex/src/container.rs`, `crates/gore-tex/src/index.rs`.

- [ ] **Step 1: Add scan-free unpack in container.rs.** `unpack_asset` (container.rs:157) finds the package by scanning + matching `package_name`, then calls `build_legacy`. Refactor so the post-lookup body is reusable, and add an id-based entry. Read the existing `unpack_asset` body to find where it has the `FPackageId` after the scan, then add:
```rust
/// Like `unpack_asset` but takes the package id directly (from the texture index),
/// skipping the full-container name scan. `utoc` must be the main container path; the
/// parent Paks dir is opened so global script objects resolve (same as `unpack_asset`).
pub fn unpack_asset_by_id(
    utoc: &Path,
    _usmap: &Path,
    package_id: u64,
    leaf: &str,
    out_dir: &Path,
) -> Result<PathBuf> {
    // Mirror unpack_asset's store-open (parent dir for global script objects) and its
    // build_legacy call, but use FPackageId(package_id) directly instead of scanning.
    // Reuse the exact build_legacy wiring from unpack_asset (FZenPackageContext::create,
    // build_legacy, FSFileWriter) — extract it into a shared helper
    // `fn legacy_from_package(store, package_id, leaf, out_dir) -> Result<PathBuf>` and
    // call it from BOTH unpack_asset and unpack_asset_by_id.
    todo!("wire: open parent store, FPackageId(package_id), shared legacy_from_package helper")
}
```
Then implement: extract the part of `unpack_asset` from "got the `FPackageId`" through `build_legacy`+return into a private `fn legacy_from_package(store: &dyn IoStoreTrait, package_id: FPackageId, leaf: &str, out_dir: &Path) -> Result<PathBuf>`, and have `unpack_asset` (after its name scan) and `unpack_asset_by_id` (constructing `FPackageId(package_id)`) both call it. `leaf` is the output filename stem (e.g. `T_HardwareCursor`); `unpack_asset` derives it from the asset path's last segment — pass the same.

- [ ] **Step 2: Add `extract_by_package_id` to index.rs** (unpack + parse + decode, for preview):
```rust
/// Extract a texture to RGBA by package id (fast: no scan). Returns (TexInfo, rgba u32 px).
pub fn extract_by_package_id(
    utoc: &Path, usmap: &Path, package_id: u64, leaf: &str,
) -> Result<(crate::decode::TexInfo, Vec<u32>)> {
    let tmp = std::env::temp_dir().join("gore-tex-idx-extract");
    std::fs::create_dir_all(&tmp)?;
    let uasset = crate::container::unpack_asset_by_id(utoc, usmap, package_id, leaf, &tmp)?;
    let uexp = uasset.with_extension("uexp");
    let ubulk = uasset.with_extension("ubulk");
    let info = crate::decode::parse(
        &std::fs::read(&uasset)?, &std::fs::read(&uexp)?,
        &std::fs::read(&ubulk).unwrap_or_default(), &std::fs::read(usmap)?)?;
    let px = crate::decode::to_rgba8(&info)?;
    Ok((info, px))
}
```

- [ ] **Step 3: Gated test — id-extract == path-extract pixels.** Append to index.rs tests:
```rust
    #[test]
    #[ignore = "slow: unpack from real container"]
    fn id_extract_matches_path_extract() {
        let Some(g) = game_dir() else { eprintln!("skip"); return; };
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let asset = "/Game/UI/Textures/Common/T_HardwareCursor";
        // path-based (scan) extract for reference
        let tmp = std::env::temp_dir().join("gore-tex-ref");
        let _ = std::fs::remove_dir_all(&tmp); std::fs::create_dir_all(&tmp).unwrap();
        let ua = crate::container::unpack_asset(&utoc, &usmap, asset, &tmp).unwrap();
        let ref_info = crate::decode::parse(
            &std::fs::read(&ua).unwrap(), &std::fs::read(ua.with_extension("uexp")).unwrap(),
            &std::fs::read(ua.with_extension("ubulk")).unwrap_or_default(),
            &std::fs::read(&usmap).unwrap()).unwrap();
        let ref_px = crate::decode::to_rgba8(&ref_info).unwrap();
        // index-based fast extract
        let idx = build_index(&utoc, "t").unwrap();
        let pid = *idx.entries.get(asset).unwrap();
        let (info, px) = extract_by_package_id(&utoc, &usmap, pid, "T_HardwareCursor").unwrap();
        assert_eq!(info.width, ref_info.width);
        assert_eq!(info.format, ref_info.format);
        assert_eq!(px, ref_px);
    }
```

- [ ] **Step 4: Run gated test once.** Run: `cargo test -p gore-tex index::tests::id_extract -- --ignored --nocapture`
Expected: PASS (pixels identical). Also run fast suite: `cargo test -p gore-tex` (green).

- [ ] **Step 5: Commit.**
```bash
git add crates/gore-tex/src/container.rs crates/gore-tex/src/index.rs
git commit -m "feat(gore-tex): scan-free unpack + extract by package id"
```

### Task 4: CLI `gore texture index`

**Files:** Modify `crates/gore/src/cmd/texture.rs`, `crates/gore/src/main.rs`.

- [ ] **Step 1: Add the `Index` subcommand variant.** In `crates/gore/src/cmd/texture.rs`, add to `enum TextureAction`:
```rust
    /// Build the texture index (asset->package_id) and cache it to the shared dir
    Index {
        #[arg(long)] game: PathBuf,
        /// Output path (defaults to the shared gore-tools texture_index.json)
        #[arg(short = 'o', long)] out: Option<PathBuf>,
    },
```
And in `run`:
```rust
        TextureAction::Index { game, out } => {
            let utoc = gore_tex::paths::main_container(&game)?;
            let usmap = gore_tex::paths::usmap(&game)?;
            let build_id = usmap.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
            eprintln!("scanning container to build the texture index (a few minutes)...");
            let idx = gore_tex::index::build_index(&utoc, &build_id)?;
            let path = out.unwrap_or_else(gore_tex::paths::texture_index_path);
            idx.save(&path)?;
            println!("wrote {} ({} textures)", path.display(), idx.entries.len());
            Ok(())
        }
```

- [ ] **Step 2: Build + help.** Run: `cargo build -p gore` then `cargo run -q -p gore -- texture index --help`
Expected: shows `--game` and `--out`.

- [ ] **Step 3: Commit.**
```bash
git add crates/gore/src/cmd/texture.rs crates/gore/src/main.rs
git commit -m "feat(gore): 'texture index' CLI builds + caches the texture index"
```

---

## Phase 2 — gore-mod TexturePatch component + FFI

### Task 5: BuildSpec.texture + Component::TexturePatch + build arm

**Files:** Modify `crates/gore-mod/src/lib.rs`, `crates/gore-mod/Cargo.toml`.

- [ ] **Step 1: Add the dep + spec/manifest types.** In `crates/gore-mod/Cargo.toml` `[dependencies]` add `gore-tex = { path = "../gore-tex" }`. In `crates/gore-mod/src/lib.rs` after `AudioReplacement` (line 54) add:
```rust
/// One texture replacement: put `image_path` (a PNG) in place of cooked `asset` (in-game path).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureReplacement {
    pub asset: String,      // e.g. "/Game/UI/Textures/Common/T_HardwareCursor"
    pub image_path: String, // a PNG on disk
}
```
Add to `BuildSpec` (after `audio`, line 68):
```rust
    #[serde(default)]
    pub texture: Vec<TextureReplacement>,
```
Add a `Component` variant (after `AudioPatch`, line 79):
```rust
    /// Texture patch dir at `path` (manifest.json + pngs); deploy cooks + packs a Zen triplet
    /// into `~mods` for `assets`. Additive — no in-place game-file patch, no `*.gore-bak`.
    TexturePatch { path: String, assets: Vec<String> },
```

- [ ] **Step 2: Build arm.** In `build_bundle`, after the audio block (line 140) and before the manifest assembly (line 142):
```rust
    // textures → manifest + pngs (source images; cooked+packed at deploy)
    if !spec.texture.is_empty() {
        let mut map: BTreeMap<String, String> = BTreeMap::new();
        for (i, t) in spec.texture.iter().enumerate() {
            let png = std::fs::read(&t.image_path)
                .map_err(io(&format!("reading png {}", t.image_path)))?;
            let fname = format!("{i}_{}.png", sanitize(&t.asset));
            files.insert(format!("texture/{fname}"), png);
            map.insert(t.asset.clone(), format!("texture/{fname}"));
        }
        let assets: Vec<String> = map.keys().cloned().collect();
        files.insert("texture/manifest.json".into(), serde_json::to_vec_pretty(&map)?);
        components.push(Component::TexturePatch { path: "texture".into(), assets });
    }
```

- [ ] **Step 3: Test the build arm (no game).** Add a test in `crates/gore-mod/src/lib.rs` `#[cfg(test)] mod tests` (create the module if absent):
```rust
    #[test]
    fn build_emits_texture_patch() {
        let dir = std::env::temp_dir().join("gore-mod-tex-build");
        let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("img.png");
        std::fs::write(&png, b"\x89PNG\r\n\x1a\nfake").unwrap();
        let spec = BuildSpec {
            meta: ModMeta { name: "TestMod".into(), version: String::new(), author: String::new() },
            delay_ms: 0, overrides: vec![], loc_edits: Default::default(), audio: vec![],
            texture: vec![TextureReplacement { asset: "/Game/UI/T_X".into(), image_path: png.display().to_string() }],
        };
        let bundle = build_bundle(&spec).unwrap();
        assert!(bundle.files.contains_key("texture/manifest.json"));
        assert!(bundle.files.keys().any(|k| k.starts_with("texture/") && k.ends_with(".png")));
        assert!(matches!(bundle.manifest.components.last(), Some(Component::TexturePatch { assets, .. }) if assets == &vec!["/Game/UI/T_X".to_string()]));
    }
```
Note: every existing `BuildSpec { .. }` literal in the crate's tests now needs `texture: vec![]` added — search the crate for `BuildSpec {` and add the field. (Or change to `..Default::default()` only if `BuildSpec` derives `Default`; it does not — add the field explicitly.)

- [ ] **Step 4: Build + test.** Run: `cargo test -p gore-mod build_emits_texture_patch` then `cargo test -p gore-mod` (all green; fix any `BuildSpec {` literals missing `texture`).
Expected: PASS.

- [ ] **Step 5: Commit.**
```bash
git add crates/gore-mod/src/lib.rs crates/gore-mod/Cargo.toml Cargo.lock
git commit -m "feat(gore-mod): TexturePatch component + build arm (source pngs)"
```

### Task 6: Deploy/undeploy lifecycle for TexturePatch

**Files:** Modify `crates/gore-mod/src/lib.rs`.

This is the divergence: textures install an additive Zen triplet into `~mods`, NOT a backed-up in-place write. Add a separate plan/record channel.

- [ ] **Step 1: Extend `DeployPlan` + `DeployRecord`.** In `DeployPlan` (line 349) add:
```rust
    /// Zen triplet files to copy into the game's ~mods (src in a temp dir, dst in ~mods).
    texture_triplets: Vec<(PathBuf, PathBuf)>,
```
Update the `DeployPlan { .. }` initializer in `prepare` (line 587) to `texture_triplets: Vec::new()`.
In `DeployRecord` (line 248) add:
```rust
    /// Zen triplet files (absolute) this deploy dropped into ~mods; deleted on undeploy.
    /// Additive override paks — no backup needed (removing them fully reverts). serde(default)
    /// keeps old records loadable.
    #[serde(default)]
    pub texture_triplets: Vec<String>,
```

- [ ] **Step 2: prepare arm — cook + pack one triplet.** In `prepare` (line 588 match), add after the `AudioPatch` arm:
```rust
            Component::TexturePatch { path, assets: _ } => {
                if !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!("unsafe texture patch path: {path:?}")));
                }
                let map: BTreeMap<String, String> = serde_json::from_slice(
                    &std::fs::read(bundle_dir.join(path).join("manifest.json"))
                        .map_err(io("reading texture manifest"))?)?;
                // gp.root: derive the game install dir from gp (the parent of the G1R dirs).
                // ue4ss_mods == <root>/G1R/Binaries/Win64/ue4ss/Mods -> 5 parents up = <root>.
                let game_dir = gp.ue4ss_mods.ancestors().nth(5)
                    .ok_or_else(|| ModError::Other("cannot derive game dir from paths".into()))?
                    .to_path_buf();
                let utoc = gore_tex::paths::main_container(&game_dir)
                    .map_err(|e| ModError::Other(format!("container: {e}")))?;
                let usmap = gore_tex::paths::usmap(&game_dir)
                    .map_err(|e| ModError::Other(format!("usmap: {e}")))?;
                // Optional fast index (falls back to scanning unpack if absent).
                let index = gore_tex::index::TextureIndex::load(&gore_tex::paths::texture_index_path()).ok();
                // Cook each replacement into a single cooked tree under the mount path.
                let cook_dir = std::env::temp_dir().join(format!("gore-mod-tex-cook-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&cook_dir);
                for (asset, png_rel) in &map {
                    if !is_safe_rel_path(png_rel) {
                        return Err(ModError::Other(format!("unsafe png path: {png_rel:?}")));
                    }
                    let leaf = asset.rsplit('/').next().unwrap_or(asset);
                    // mount path: /Game/X -> G1R/Content/X ; strip leading slash for others.
                    let rel = asset.strip_prefix("/Game/").map(|r| format!("G1R/Content/{r}"))
                        .unwrap_or_else(|| format!("G1R/Content/{}", asset.trim_start_matches('/')));
                    let dest_dir = cook_dir.join(std::path::Path::new(&rel).parent().unwrap());
                    std::fs::create_dir_all(&dest_dir).map_err(io("mkdir cook dir"))?;
                    // unpack original (index-fast if available, else scan)
                    let tmp_orig = std::env::temp_dir().join("gore-mod-tex-orig");
                    let _ = std::fs::remove_dir_all(&tmp_orig); std::fs::create_dir_all(&tmp_orig).map_err(io("mkdir"))?;
                    let orig_uasset = match index.as_ref().and_then(|i| i.entries.get(asset)) {
                        Some(&pid) => gore_tex::container::unpack_asset_by_id(&utoc, &usmap, pid, leaf, &tmp_orig),
                        None => gore_tex::container::unpack_asset(&utoc, &usmap, asset, &tmp_orig),
                    }.map_err(|e| ModError::Other(format!("unpack {asset}: {e}")))?;
                    let ua = std::fs::read(&orig_uasset).map_err(io("read uasset"))?;
                    let ue = std::fs::read(orig_uasset.with_extension("uexp")).map_err(io("read uexp"))?;
                    let ub = std::fs::read(orig_uasset.with_extension("ubulk")).unwrap_or_default();
                    // load PNG -> rgba; encode mips; replace_texture
                    let img = image::open(bundle_dir.join(png_rel)).map_err(|e| ModError::Other(format!("png {png_rel}: {e}")))?.to_rgba8();
                    let (w, h) = (img.width(), img.height());
                    let info = gore_tex::decode::parse(&ua, &ue, &ub, &std::fs::read(&usmap).map_err(io("usmap"))?)
                        .map_err(|e| ModError::Other(format!("parse {asset}: {e}")))?;
                    let mips = gore_tex::encode::encode_mips(img.as_raw(), w, h, &info.format)
                        .map_err(|e| ModError::Other(format!("encode {asset}: {e}")))?;
                    let (na, ne, nb) = gore_tex::texdata::replace_texture(&ua, &ue, &ub, w, h, mips)
                        .map_err(|e| ModError::Other(format!("replace {asset}: {e}")))?;
                    std::fs::write(dest_dir.join(format!("{leaf}.uasset")), &na).map_err(io("write uasset"))?;
                    std::fs::write(dest_dir.join(format!("{leaf}.uexp")), &ne).map_err(io("write uexp"))?;
                    if !nb.is_empty() { std::fs::write(dest_dir.join(format!("{leaf}.ubulk")), &nb).map_err(io("write ubulk"))?; }
                }
                // pack ONE triplet for all textures (uncompressed = proven), into a temp out dir.
                let triplet_name = format!("zzz_{}_tex_P", sanitize(&manifest.mod_meta.name));
                let pack_out = std::env::temp_dir().join(format!("gore-mod-tex-pack-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&pack_out); std::fs::create_dir_all(&pack_out).map_err(io("mkdir pack"))?;
                let triplet = gore_tex::container::repack_to_zen(&cook_dir, &triplet_name, &pack_out, &game_dir, false)
                    .map_err(|e| ModError::Other(format!("pack: {e}")))?;
                // queue copy into ~mods (G1R/Content/Paks/~mods).
                let mods_dir = game_dir.join("G1R").join("Content").join("Paks").join("~mods");
                for src in triplet {
                    let dst = mods_dir.join(src.file_name().ok_or_else(|| ModError::Other("triplet file".into()))?);
                    plan.texture_triplets.push((src, dst));
                }
            }
```
Add `use gore_tex;` is implicit via the crate dep. Add `image = "0.25"` to `gore-mod/Cargo.toml` (for `image::open`). **Verify** the exact signature of `gore_tex::container::repack_to_zen` (it returns `Result<[PathBuf;3]>` and takes `(cooked_dir, name, out_dir, game_dir, compress: bool)`) and `gore_tex::texdata::replace_texture` (returns `(Vec<u8>,Vec<u8>,Vec<u8>)`) against the current source before relying on them.

- [ ] **Step 3: apply — copy triplets + record.** In `apply_writes` (line 715), after the ue4ss swap + before/after the file writes, add the triplet copy:
```rust
    for (src, dst) in &plan.texture_triplets {
        if let Some(p) = dst.parent() { std::fs::create_dir_all(p).map_err(io("mkdir ~mods"))?; }
        std::fs::copy(src, dst).map_err(io(&format!("copy triplet to {}", dst.display())))?;
    }
```
And record the dsts: in `stage` (line 670) add at the end (so the record knows them before live writes):
```rust
    for (_, dst) in &plan.texture_triplets {
        record.texture_triplets.push(dst.display().to_string());
    }
```
(`stage` receives `record: &mut DeployRecord` — it already sets `record.ue4ss_mod_dir`.) For rollback safety: triplets are additive; on rollback the copied files should be removed. Add their removal to the `Undo` path — find `Undo` (line ~388 / its struct) and add a `texture_files: Vec<PathBuf>` field whose `rollback()` deletes them; push to it in `apply_writes` right after each successful `std::fs::copy`. (Mirror how `created_baks` are tracked/cleaned.)

- [ ] **Step 4: undeploy arm — delete recorded triplets.** Find `restore_record` (line ~928) / the `undeploy` body. After restoring backups + removing ue4ss dirs, add:
```rust
    for f in &record.texture_triplets {
        let p = std::path::Path::new(f);
        if p.exists() { let _ = std::fs::remove_file(p); }
    }
```
(Best-effort delete; missing files are fine — additive revert.)

- [ ] **Step 5: Unit test deploy-record triplet cleanup (no game, no cook).** This tests only the record/undeploy file lifecycle, not the cook+pack (that's the gated E2E). Add to `gore-mod` tests:
```rust
    #[test]
    fn undeploy_removes_recorded_texture_triplets() {
        let game = std::env::temp_dir().join("gore-mod-undeploy-tex");
        let _ = std::fs::remove_dir_all(&game);
        let mods = game.join("G1R/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        let files: Vec<String> = ["zzz_M_tex_P.utoc","zzz_M_tex_P.ucas","zzz_M_tex_P.pak"].iter().map(|n| {
            let p = mods.join(n); std::fs::write(&p, b"x").unwrap(); p.display().to_string()
        }).collect();
        // Hand-write a record with only the texture_triplets populated + persist it.
        let rec = DeployRecord { mod_name: "M".into(), texture_triplets: files.clone(), ..Default::default() };
        std::fs::write(record_path(&game), serde_json::to_vec(&rec).unwrap()).unwrap();
        undeploy(&game).unwrap();
        for f in &files { assert!(!std::path::Path::new(f).exists(), "triplet not removed: {f}"); }
    }
```
(Confirm `record_path` + `undeploy` are reachable from the test module; `record_path` is private — the test is in the same crate so it's accessible.)

- [ ] **Step 6: Build + test.** Run: `cargo build -p gore-mod` then `cargo test -p gore-mod undeploy_removes_recorded_texture` and `cargo test -p gore-mod` (green).
Expected: PASS.

- [ ] **Step 7: Commit.**
```bash
git add crates/gore-mod/src/lib.rs crates/gore-mod/Cargo.toml Cargo.lock
git commit -m "feat(gore-mod): TexturePatch deploy/undeploy (cook+pack triplet to ~mods)"
```

### Task 7: gore-ffi texture commands

**Files:** Modify `crates/gore-ffi/src/lib.rs`, `crates/gore-ffi/Cargo.toml`.

- [ ] **Step 1: Add the dep.** In `crates/gore-ffi/Cargo.toml` `[dependencies]` add `gore-tex = { path = "../gore-tex" }` and `image = "0.25"`.

- [ ] **Step 2: Add `texture_index` + `texture_extract` + dispatch arms.** In `crates/gore-ffi/src/lib.rs`, add to the `match command` (line 81-93):
```rust
        "texture_index" => texture_index(payload),
        "texture_extract" => texture_extract(payload),
```
Add the functions (mirror `audio_list`/`audio_extract` shape — JSON in, `json!({...})` out, `err(code, msg)` on failure):
```rust
/// `{ok, build_id, count, entries:{path:package_id_str}}` — load the cached index, building it
/// if absent or if `payload.rebuild` is true. `payload.game` = install dir.
fn texture_index(payload: Value) -> Value {
    let game = match payload.get("game").and_then(Value::as_str) {
        Some(g) => std::path::PathBuf::from(g),
        None => return err("BAD_REQUEST", "missing game"),
    };
    let rebuild = payload.get("rebuild").and_then(Value::as_bool).unwrap_or(false);
    let cache = gore_tex::paths::texture_index_path();
    let idx = if !rebuild && cache.exists() {
        match gore_tex::index::TextureIndex::load(&cache) { Ok(i) => i, Err(e) => return err("INDEX_LOAD", e.to_string()) }
    } else {
        let utoc = match gore_tex::paths::main_container(&game) { Ok(p) => p, Err(e) => return err("CONTAINER", e.to_string()) };
        let usmap = match gore_tex::paths::usmap(&game) { Ok(p) => p, Err(e) => return err("USMAP", e.to_string()) };
        let build_id = usmap.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
        let i = match gore_tex::index::build_index(&utoc, &build_id) { Ok(i) => i, Err(e) => return err("INDEX_BUILD", e.to_string()) };
        let _ = i.save(&cache);
        i
    };
    // package_id as string (JSON numbers lose u64 precision in some clients).
    let entries: serde_json::Map<String, Value> = idx.entries.iter()
        .map(|(k, v)| (k.clone(), Value::String(v.to_string()))).collect();
    json!({ "ok": true, "build_id": idx.build_id, "count": idx.entries.len(), "entries": entries })
}

/// `{ok, png_path, width, height, format}` — extract a texture to a temp PNG. `payload.game`,
/// and either `payload.package_id` (string) or `payload.asset` (path).
fn texture_extract(payload: Value) -> Value {
    let game = match payload.get("game").and_then(Value::as_str) {
        Some(g) => std::path::PathBuf::from(g), None => return err("BAD_REQUEST", "missing game") };
    let utoc = match gore_tex::paths::main_container(&game) { Ok(p) => p, Err(e) => return err("CONTAINER", e.to_string()) };
    let usmap = match gore_tex::paths::usmap(&game) { Ok(p) => p, Err(e) => return err("USMAP", e.to_string()) };
    let asset = payload.get("asset").and_then(Value::as_str).unwrap_or("");
    let leaf = asset.rsplit('/').next().unwrap_or("texture").to_string();
    let (info, px) = if let Some(pid) = payload.get("package_id").and_then(Value::as_str).and_then(|s| s.parse::<u64>().ok()) {
        match gore_tex::index::extract_by_package_id(&utoc, &usmap, pid, &leaf) { Ok(x) => x, Err(e) => return err("EXTRACT", e.to_string()) }
    } else if !asset.is_empty() {
        // fallback scan extract
        let tmp = std::env::temp_dir().join("gore-tex-ffi-extract");
        if std::fs::create_dir_all(&tmp).is_err() { return err("IO", "tmp"); }
        let ua = match gore_tex::container::unpack_asset(&utoc, &usmap, asset, &tmp) { Ok(p) => p, Err(e) => return err("UNPACK", e.to_string()) };
        let info = match gore_tex::decode::parse(
            &std::fs::read(&ua).unwrap_or_default(), &std::fs::read(ua.with_extension("uexp")).unwrap_or_default(),
            &std::fs::read(ua.with_extension("ubulk")).unwrap_or_default(), &std::fs::read(&usmap).unwrap_or_default()) {
            Ok(i) => i, Err(e) => return err("PARSE", e.to_string()) };
        let px = match gore_tex::decode::to_rgba8(&info) { Ok(p) => p, Err(e) => return err("DECODE", e.to_string()) };
        (info, px)
    } else { return err("BAD_REQUEST", "need package_id or asset"); };
    // pack u32 ARGB px -> RGBA8 bytes (0xAARRGGBB -> [R,G,B,A]) and write a PNG.
    let mut buf = Vec::with_capacity(px.len() * 4);
    for p in px { buf.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8]); }
    let out = std::env::temp_dir().join(format!("gore-tex-preview-{leaf}.png"));
    if image::save_buffer(&out, &buf, info.width, info.height, image::ColorType::Rgba8).is_err() {
        return err("PNG", "save failed");
    }
    json!({ "ok": true, "png_path": out.display().to_string(), "width": info.width, "height": info.height, "format": info.format })
}
```

- [ ] **Step 3: Build the dll.** Run: `cargo build -p gore-ffi`
Expected: clean.

- [ ] **Step 4: Commit.**
```bash
git add crates/gore-ffi/src/lib.rs crates/gore-ffi/Cargo.toml Cargo.lock
git commit -m "feat(gore-ffi): texture_index + texture_extract commands"
```

---

## Phase 3 — Flutter Textures tab + project/build-deploy wiring

### Task 8: Texture replacements notifier

**Files:** Create `apps/mod-studio/lib/textures/domain/texture_replacements_notifier.dart`.

- [ ] **Step 1: Write the notifier (exact mirror of `audio_replacements_notifier.dart`).**
```dart
import 'package:flutter_riverpod/legacy.dart';

/// One staged texture replacement: put [imagePath] (a PNG) in place of cooked [asset].
class TextureReplacement {
  const TextureReplacement({required this.asset, required this.imagePath});
  final String asset;
  final String imagePath;

  String get key => asset;

  Map<String, Object?> toJson() => {'asset': asset, 'image_path': imagePath};

  factory TextureReplacement.fromJson(Map<String, Object?> j) => TextureReplacement(
        asset: j['asset'] as String,
        imagePath: j['image_path'] as String,
      );

  TextureReplacement withImagePath(String path) =>
      TextureReplacement(asset: asset, imagePath: path);
}

class TextureReplacementsState {
  const TextureReplacementsState({this.items = const {}});
  final Map<String, TextureReplacement> items;
  int get count => items.length;
  List<TextureReplacement> get entries => items.values.toList()
    ..sort((a, b) => a.asset.compareTo(b.asset));
  TextureReplacementsState copyWith({Map<String, TextureReplacement>? items}) =>
      TextureReplacementsState(items: items ?? this.items);
}

class TextureReplacementsNotifier extends StateNotifier<TextureReplacementsState> {
  TextureReplacementsNotifier() : super(const TextureReplacementsState());
  void setReplacement(TextureReplacement r) {
    final items = Map<String, TextureReplacement>.from(state.items);
    items[r.key] = r;
    state = state.copyWith(items: items);
  }
  void remove(String key) {
    if (!state.items.containsKey(key)) return;
    final items = Map<String, TextureReplacement>.from(state.items)..remove(key);
    state = state.copyWith(items: items);
  }
  void clearAll() {
    if (state.items.isEmpty) return;
    state = const TextureReplacementsState();
  }
  void loadAll(List<TextureReplacement> list) {
    state = TextureReplacementsState(items: {for (final r in list) r.key: r});
  }
}

final textureReplacementsProvider =
    StateNotifierProvider<TextureReplacementsNotifier, TextureReplacementsState>(
        (ref) => TextureReplacementsNotifier());
```

- [ ] **Step 2: Write a notifier test.** Create `apps/mod-studio/test/textures/texture_replacements_notifier_test.dart`:
```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/textures/domain/texture_replacements_notifier.dart';

void main() {
  test('set/remove/clear/loadAll', () {
    final n = TextureReplacementsNotifier();
    n.setReplacement(const TextureReplacement(asset: '/Game/T_A', imagePath: 'a.png'));
    n.setReplacement(const TextureReplacement(asset: '/Game/T_B', imagePath: 'b.png'));
    expect(n.state.count, 2);
    n.remove('/Game/T_A');
    expect(n.state.count, 1);
    n.loadAll([const TextureReplacement(asset: '/Game/T_C', imagePath: 'c.png')]);
    expect(n.state.items.keys.single, '/Game/T_C');
    n.clearAll();
    expect(n.state.count, 0);
  });
}
```
(Confirm the package name in imports is `gore_mod` — per the app's pubspec `name:`. The exploration notes the Flutter pkg name is `gore_mod`. Verify in `apps/mod-studio/pubspec.yaml`.)

- [ ] **Step 3: Run the test.** Run (from `apps/mod-studio`): `flutter test test/textures/texture_replacements_notifier_test.dart`
Expected: PASS.

- [ ] **Step 4: Commit.**
```bash
git add apps/mod-studio/lib/textures/domain/texture_replacements_notifier.dart apps/mod-studio/test/textures/texture_replacements_notifier_test.dart
git commit -m "feat(mod-studio): texture replacements notifier"
```

### Task 9: ModFfi texture wrappers + index provider

**Files:** Modify `apps/mod-studio/lib/core/mod_ffi.dart`; create `apps/mod-studio/lib/textures/domain/texture_index_provider.dart`.

- [ ] **Step 1: Add FFI wrappers.** In `mod_ffi.dart`, mirroring `audioList`/`audioExtract` (lines 18-35), add methods to the `ModFfi` class:
```dart
  /// Load (or build, if absent/`rebuild`) the texture index. Returns {path: packageIdString}.
  Future<Map<String, String>> textureIndex(String game, {bool rebuild = false}) async {
    final r = await _core.execute('texture_index', payload: {'game': game, 'rebuild': rebuild});
    final entries = (r['entries'] as Map).cast<String, Object?>();
    return entries.map((k, v) => MapEntry(k, v as String));
  }

  /// Extract a texture to a temp PNG; returns its path (+ dims/format in the map).
  Future<Map<String, Object?>> textureExtract(String game, {String? asset, String? packageId}) async {
    return _core.execute('texture_extract', payload: {
      'game': game, if (asset != null) 'asset': asset, if (packageId != null) 'package_id': packageId,
    });
  }
```
(Match the existing `_core`/field name + `execute` signature in `mod_ffi.dart` — confirm whether it's `_core.execute(cmd, payload: {...})` returning `Map`. The exploration shows `GoreCoreFfiService.execute(command, {payload})`.)

- [ ] **Step 2: Write the index provider.** Create `apps/mod-studio/lib/textures/domain/texture_index_provider.dart`:
```dart
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/providers.dart';
import '../../core/mod_ffi.dart';
import '../../app/game_paths.dart';

/// Loads (building on first use) the texture index for the configured game path.
/// Returns asset_path -> packageIdString. Long on first build; runs off-isolate in the DLL call.
final textureIndexProvider = FutureProvider<Map<String, String>>((ref) async {
  final game = ref.watch(gameExePathProvider); // the configured game exe/dir path
  if (game == null || game.isEmpty) return {};
  final gameDir = gameRootFromExe(game); // derive install dir from the exe path
  final ffi = ModFfi(ref.read(coreServiceProvider));
  return ffi.textureIndex(gameDir);
});
```
(VERIFY the exact provider for the game path + the helper to get the install dir — the Audio tab uses `fmodDesktopDir(gameExePath)` from `lib/app/game_paths.dart`. Use the same source-of-truth provider the Audio tab reads for the game path, and the same game-root derivation. Adjust the two `// VERIFY` lines to the real names found in `lib/app/game_paths.dart` + the settings provider.)

- [ ] **Step 3: Analyze.** Run (from `apps/mod-studio`): `flutter analyze lib/core/mod_ffi.dart lib/textures/`
Expected: no errors (fix the two verified names).

- [ ] **Step 4: Commit.**
```bash
git add apps/mod-studio/lib/core/mod_ffi.dart apps/mod-studio/lib/textures/domain/texture_index_provider.dart
git commit -m "feat(mod-studio): ModFfi texture wrappers + index provider"
```

### Task 10: Textures tab UI

**Files:** Create `apps/mod-studio/lib/textures/ui/texture_tab.dart`.

- [ ] **Step 1: Write the tab.** Mirror `lib/audio/ui/audio_tab.dart` structure (ConsumerStatefulWidget; left = search + filtered list; right = preview + Replace; bottom = staged panel). Create `texture_tab.dart`:
```dart
import 'dart:io';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/providers.dart';
import '../../core/mod_ffi.dart';
import '../../app/game_paths.dart';
import '../domain/texture_index_provider.dart';
import '../domain/texture_replacements_notifier.dart';

class TextureTab extends ConsumerStatefulWidget {
  const TextureTab({super.key});
  @override
  ConsumerState<TextureTab> createState() => _TextureTabState();
}

class _TextureTabState extends ConsumerState<TextureTab> {
  String _query = '';
  String? _selected; // selected asset path
  String? _previewPng;

  @override
  Widget build(BuildContext context) {
    final game = ref.watch(gameExePathProvider);
    if (game == null || game.isEmpty) {
      return const Center(child: Text('Set the game path in Settings to browse textures.'));
    }
    final indexAsync = ref.watch(textureIndexProvider);
    final staged = ref.watch(textureReplacementsProvider);
    return indexAsync.when(
      loading: () => const Center(child: Column(mainAxisSize: MainAxisSize.min, children: [
        CircularProgressIndicator(), SizedBox(height: 12),
        Text('Building texture index (first run, ~few minutes)...'),
      ])),
      error: (e, _) => Center(child: Text('Index error: $e')),
      data: (entries) {
        final matches = entries.keys
            .where((p) => _query.isEmpty || p.toLowerCase().contains(_query.toLowerCase()))
            .take(500).toList()..sort();
        return Row(children: [
          // left: search + list
          Expanded(flex: 2, child: Column(children: [
            Padding(padding: const EdgeInsets.all(8), child: TextField(
              decoration: const InputDecoration(prefixIcon: Icon(Icons.search), hintText: 'Search textures'),
              onChanged: (v) => setState(() => _query = v))),
            Expanded(child: ListView.builder(itemCount: matches.length, itemBuilder: (c, i) {
              final p = matches[i];
              final isReplaced = staged.items.containsKey(p);
              return ListTile(
                dense: true, selected: p == _selected,
                title: Text(p, maxLines: 1, overflow: TextOverflow.ellipsis),
                trailing: isReplaced ? const Icon(Icons.check, size: 16) : null,
                onTap: () => setState(() { _selected = p; _previewPng = null; }));
            })),
            Text('${matches.length} shown / ${entries.length} total', style: Theme.of(context).textTheme.bodySmall),
          ])),
          const VerticalDivider(width: 1),
          // right: detail
          Expanded(flex: 3, child: _detail(entries, staged)),
        ]);
      },
    );
  }

  Widget _detail(Map<String, String> entries, TextureReplacementsState staged) {
    final sel = _selected;
    if (sel == null) return const Center(child: Text('Select a texture'));
    final game = ref.read(gameExePathProvider)!;
    final gameDir = gameRootFromExe(game);
    final replaced = staged.items[sel];
    return Padding(padding: const EdgeInsets.all(12), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Text(sel, style: Theme.of(context).textTheme.titleSmall),
      const SizedBox(height: 8),
      Row(children: [
        OutlinedButton.icon(icon: const Icon(Icons.visibility), label: const Text('Preview'),
          onPressed: () async {
            final ffi = ModFfi(ref.read(coreServiceProvider));
            final r = await ffi.textureExtract(gameDir, asset: sel, packageId: entries[sel]);
            if (r['ok'] == true && mounted) setState(() => _previewPng = r['png_path'] as String?);
          }),
        const SizedBox(width: 8),
        FilledButton.icon(icon: const Icon(Icons.image), label: const Text('Replace...'),
          onPressed: () async {
            final f = await openFile(acceptedTypeGroups: [const XTypeGroup(label: 'PNG', extensions: ['png'])]);
            if (f != null) ref.read(textureReplacementsProvider.notifier)
              .setReplacement(TextureReplacement(asset: sel, imagePath: f.path));
          }),
      ]),
      const SizedBox(height: 12),
      if (_previewPng != null) Expanded(child: Image.file(File(_previewPng!), fit: BoxFit.contain))
      else const Expanded(child: Center(child: Text('Preview to see the current texture'))),
      if (replaced != null) Padding(padding: const EdgeInsets.only(top: 8), child: Row(children: [
        const Icon(Icons.swap_horiz, size: 16), const SizedBox(width: 4),
        Expanded(child: Text('→ ${replaced.imagePath}', maxLines: 1, overflow: TextOverflow.ellipsis)),
        IconButton(icon: const Icon(Icons.close, size: 16),
          onPressed: () => ref.read(textureReplacementsProvider.notifier).remove(sel)),
      ])),
    ]));
  }
}
```
(VERIFY: `gameExePathProvider` + `gameRootFromExe` — use the SAME game-path provider/helper the Audio tab uses; `file_selector` is already a dep, `Image.file` is core Flutter. Adjust names to the real ones in `lib/app/game_paths.dart` + settings.)

- [ ] **Step 2: Analyze.** Run (from `apps/mod-studio`): `flutter analyze lib/textures/ui/texture_tab.dart`
Expected: no errors.

- [ ] **Step 3: Commit.**
```bash
git add apps/mod-studio/lib/textures/ui/texture_tab.dart
git commit -m "feat(mod-studio): Textures tab UI (browse/preview/replace)"
```

### Task 11: Wire tab into home_page + project + build-deploy

**Files:** Modify `home_page.dart`, `project/project_model.dart`, `project/project_io.dart`, `project/project_controller.dart`, `export/build_deploy_dialog.dart`.

- [ ] **Step 1: Add the 6th tab.** In `apps/mod-studio/lib/home_page.dart`: change `DefaultTabController(length: 5` → `length: 6`. Add a `Tab(icon: Icon(Icons.texture), text: 'Textures')` to the `tabs:` list after the Audio tab. Add `const TextureTab()` to `TabBarView.children` at the SAME index (after `const AudioTab()`). Add `import 'textures/ui/texture_tab.dart';`. Extend the Build/Deploy dirty/enabled flag (the `overridesState.count > 0 || locEdits.isDirty || audioReplacements.count > 0` expression) with `|| ref.watch(textureReplacementsProvider).count > 0` and import the notifier. (Keep the two child lists index-aligned — that's the one easy bug here.)

- [ ] **Step 2: Add `textures` to ModProject.** In `project/project_model.dart`: add `import '../textures/domain/texture_replacements_notifier.dart';`. Add field `final List<TextureReplacement> textures;` to `ModProject`; add it to the constructor (default `const []`), `copyWith`, `toJson` (key `'textures'` → `textures.map((t) => t.toJson()).toList()`), `fromJson` (`textures: ((j['textures'] as List?) ?? const []).map((e) => TextureReplacement.fromJson((e as Map).cast())).toList()`), and **`toBuildSpec()`** (add `'texture': textures.map((t) => t.toJson()).toList()` — matching the Rust `BuildSpec.texture` field name `texture` and `TextureReplacement{asset, image_path}` shape; note `toJson` already emits `asset`/`image_path`).

- [ ] **Step 3: Embed/extract PNGs in project_io.dart.** Mirror the WAV embedding (lines 17-25): after the audio loop in `saveProject`, embed each texture PNG under `assets/textures/<idx>_<basename>.png` and rewrite `imagePath` to the relative path. In `loadProject` (lines 61-84), extract `assets/textures/`-prefixed entries with the IDENTICAL path-traversal guard (only `assets/` prefix, no `..`, resolves inside temp) and rewrite to absolute temp paths. Use the same `goremod_loaded` temp dir.

- [ ] **Step 4: Wire project_controller.dart.** In `gatherProject` add `textures: ref.read(textureReplacementsProvider).entries`; in `applyProject` add `ref.read(textureReplacementsProvider.notifier).loadAll(project.textures)`; in `newProject` add `ref.read(textureReplacementsProvider.notifier).clearAll()`. (The dirty signature `_projectSignature` = `jsonEncode(gatherProject(ref).toJson())` now includes textures automatically.)

- [ ] **Step 5: Build-deploy dialog content count.** In `export/build_deploy_dialog.dart`: add `final textures = ref.watch(textureReplacementsProvider).count;` to the contents summary; include `+ textures` in the `hasContent` total (line ~98); add a `Text('• $textures texture replacement(s)')` Contents line (~line 131). No other change — `gatherProject(ref).toBuildSpec()` already carries textures.

- [ ] **Step 6: Analyze + project round-trip test.** Add `apps/mod-studio/test/project/texture_project_roundtrip_test.dart`:
```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/textures/domain/texture_replacements_notifier.dart';

void main() {
  test('ModProject round-trips textures through json', () {
    final p = ModProject(name: 'M', textures: const [
      TextureReplacement(asset: '/Game/UI/T_X', imagePath: 'x.png'),
    ]);
    final back = ModProject.fromJson(p.toJson());
    expect(back.textures.single.asset, '/Game/UI/T_X');
    expect(back.toBuildSpec()['texture'], isA<List>());
  });
}
```
(Adjust the `ModProject(...)` ctor call to its real required params — `name` plus any non-defaulted fields. Check `project_model.dart`.)
Run (from `apps/mod-studio`): `flutter analyze` (whole app, clean) then `flutter test test/project/texture_project_roundtrip_test.dart`.
Expected: analyze clean, test PASS.

- [ ] **Step 7: Commit.**
```bash
git add apps/mod-studio/lib/home_page.dart apps/mod-studio/lib/project apps/mod-studio/lib/export/build_deploy_dialog.dart apps/mod-studio/test/project/texture_project_roundtrip_test.dart
git commit -m "feat(mod-studio): wire Textures tab into home, project, build/deploy"
```

---

## Phase 4 — End-to-end verification

### Task 12: E2E — build + deploy a real texture bundle, self-launch

**Files:** none committed (scratch); record in `docs/superpowers/notes/2026-06-26-texture-tab-e2e.md`.

- [ ] **Step 1: Build the dll + a CLI-driven bundle.** `cargo build` (workspace). Author a `BuildSpec` JSON with one texture replacement (cyan-X `cursor256.png` from `work/spike/up/`, asset `/Game/UI/Textures/Common/T_HardwareCursor`) and run `gore mod build --spec <json> -o <bundle>` (PowerShell). Confirm the bundle has `texture/manifest.json` + the PNG + a `texture_patch` component in `gore-mod.json`.

- [ ] **Step 2: Deploy + verify triplet.** `gore mod deploy --bundle <bundle> --game "D:\SteamLibrary\steamapps\common\Gothic 1 Remake"`. Confirm a `zzz_<mod>_tex_P.{utoc,ucas,pak}` triplet appears in `~mods` and `gore-mod.deployed.json` lists `texture_triplets`.

- [ ] **Step 3: Self-launch + confirm.** Build the texture index first (`gore texture index --game ...`) so deploy is fast. Launch the game (`Start-Process` the shipping exe `-windowed -ResX=640 -ResY=360`, Steam running), wait ~90s, read `ue4ss/UE4SS.log` (booted, no crash) + the newest crash dump (none expected). For the VISIBLE cursor check, use a computer-use screenshot of the menu (the 256² cyan-X cursor is large+colored). `Stop-Process` to close.

- [ ] **Step 4: Undeploy + verify clean.** `gore mod undeploy --game ...`. Confirm `~mods` is empty (triplet removed) and the base game is untouched.

- [ ] **Step 5: Record + commit the note.**
```bash
git add docs/superpowers/notes/2026-06-26-texture-tab-e2e.md
git commit -m "docs(mod-studio): texture-tab bundle e2e verified (build/deploy/undeploy)"
```

---

## Self-Review

- **Spec coverage:** index `{path→package_id}` + fast extract ✔ (T1-3, dims/format-on-preview deviation stated); CLI `texture index` ✔ (T4); `BuildSpec.texture` + `Component::TexturePatch` + build arm ✔ (T5); deploy cook+pack→`~mods` triplet, additive `DeployRecord.texture_triplets`, undeploy ✔ (T6); FFI `texture_index`/`texture_extract` ✔ (T7); notifier ✔ (T8); ModFfi + index provider ✔ (T9); tab UI browse/preview/replace ✔ (T10); home/project/build-deploy wiring + `toBuildSpec` chokepoint ✔ (T11); E2E self-launch ✔ (T12); compression off (T6 passes `false`). Error handling: index-absent build, unsupported/VT formats (gore-tex errors surfaced), no-game-path guard (tab). VT/unsupported textures: surfaced as deploy/extract errors (gore-tex already rejects) — acceptable for v1.
- **Placeholder scan:** one `todo!()` in T3 Step 1 is the signature-then-impl scaffold replaced in the same step's prose (extract shared `legacy_from_package` helper). All other code is concrete. Several explicit `VERIFY`/`adjust to real name` notes target genuinely environment-specific identifiers (Flutter game-path provider name, `FPackageId.0`, exact `repack_to_zen`/`replace_texture` signatures) — these are verification instructions against current source, not unfilled logic.
- **Type consistency:** `TextureIndex{build_id, entries: BTreeMap<String,u64>}` (T1) used by `build_index` (T2), `extract_by_package_id` (T3), FFI (T7), prepare (T6). `TextureReplacement{asset, image_path}` (Rust T5) ⇄ Dart `TextureReplacement{asset, imagePath}` with `toJson` emitting `asset`/`image_path` (T8) ⇄ `BuildSpec.texture` (T5) via `toBuildSpec()['texture']` (T11). `Component::TexturePatch{path, assets}` (T5) matched in prepare (T6). `DeployRecord.texture_triplets: Vec<String>` (T6) read by undeploy (T6) + E2E (T12). `unpack_asset_by_id(utoc,usmap,package_id,leaf,out_dir)` (T3) called in prepare (T6) + extract (T3/T7).
