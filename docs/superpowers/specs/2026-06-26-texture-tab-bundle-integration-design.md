# mod-studio Textures Tab + Mod-Bundle Integration — Design

**Date:** 2026-06-26
**Status:** Approved (brainstorm), pending implementation plan
**Builds on:** the completed `gore-tex` texture read/write engine (CLI `gore texture list/extract/replace/pack/deploy/undeploy`, proven in-game, uncompressed default).

## Goal

Add a "Textures" tab to the mod-studio Flutter app that lets the user browse/search the game's textures, preview them, and stage `{asset → PNG}` replacements — and have those replacements flow through the **existing unified mod-bundle Build/Deploy pipeline** (`gore-mod`) alongside overrides / loc / audio, as a first-class bundle component. Integrating cleanly with the bundle export is the primary constraint.

## Background (verified by exploration, 2026-06-26)

- **`gore-mod`** (`crates/gore-mod/src/lib.rs`) is the unified bundle backend: `build_bundle(spec: &BuildSpec) -> Bundle{files, manifest}`; `deploy(bundle_dir, game_root) -> DeployRecord`; `undeploy(game_root)`. The manifest (`ModManifest{format, mod_meta, components}`) holds a `Vec<Component>` where `Component` is an internally-tagged enum (`#[serde(tag="type", rename_all="snake_case")]`) with variants `Ue4ssLua{name,path}`, `LocPatch{path}`, `AudioPatch{path,banks}`. `BuildSpec{meta, delay_ms, overrides, loc_edits, audio}`. Deploy is two-phase: `prepare` (in-memory) → `stage`/`apply_writes` (with `*.gore-bak` backups for in-place patches). The unified deploy record is `<game_root>/gore-mod.deployed.json` (`RECORD_NAME`).
- **Build = no game; Deploy = game present.** `build_bundle` takes no game path (assembles payloads); `deploy` has `game_root` and decodes/patches against the live install. Audio bundles WAVs at build, patches banks at deploy.
- **mod-studio app** (`apps/mod-studio`): 5 tabs via `DefaultTabController` in `lib/home_page.dart` (Items / Dialoge / Audio / Overrides / Settings). The Audio feature is the vertical-slice template: `lib/audio/domain/audio_replacements_notifier.dart` (`AudioReplacement{bank,sample,wavPath}` + `audioReplacementsProvider`), `lib/audio/ui/audio_tab.dart` (search list + preview + replace + staged panel). Project file `.goremod` = a zip: `lib/project/project_model.dart` (`ModProject` with `toJson`/`fromJson`/`toBuildSpec`), `project_io.dart` (embeds WAVs under `assets/audio/` with path-traversal guard), `project_controller.dart` (`gatherProject`/`applyProject`/`newProject`/dirty). Build/Deploy hub `lib/export/build_deploy_dialog.dart`: `gatherProject(ref).toBuildSpec()` → `ModFfi.modBuild(spec, dir)` then `modDeploy(bundle, gameRoot)`. FFI via `lib/core/core_service.dart` (`execute(command, payload)`, runs in `Isolate.run`) + typed `lib/core/mod_ffi.dart`. **No texture FFI/component exists yet.**
- **`gore-ffi`** (`crates/gore-ffi/src/lib.rs`): JSON-in/JSON-out, dispatch `match command`. Mod commands: `mod_build`, `mod_deploy`, `mod_undeploy`, `audio_list`, `audio_extract`. `gore-tex` is not yet a dependency.
- **Performance reality:** both `gore_tex::container::list_textures` and `unpack_asset` do a full ~25GB container scan (minutes). 13,480 Texture2D assets. Naive listing/preview is unusably slow in a GUI → requires a cached index.

See memory: `gothic-remake-texture-modding`, `mod-studio-unified-plan`.

## Decisions (locked in brainstorm)

1. **Bundle carries SOURCE PNGs (option A), cook+pack at deploy.** Mirrors audio (bundle = WAVs, patch at deploy) and the build=no-game / deploy=game contract. Bundle stays small + portable. Slower deploy (unpacks per asset) accepted; mitigated by the index.
2. **Cached texture index (option A).** A one-time container scan builds `{asset_path: {package_id, width, height, format}}`, cached to the shared gore-tools dir. The GUI searches it locally (instant) and previews via an index-aware fast extract (direct chunk read by `package_id`, no scan).
3. **Texture deploy is additive (Zen triplet → `~mods`), NOT a `*.gore-bak` patch.** Tracked separately in the deploy record.
4. **Compression OFF** (the proven-loading path).

## Architecture

```
Flutter "Textures" tab ──> textureReplacementsProvider (stage {asset, png})
        │                          │
        │ texture_index / texture_extract (FFI)        gatherProject → ModProject.textures
        ▼                          │                          │ toBuildSpec()
   cached index JSON               ▼                          ▼
   (shared gore-tools dir)   preview PNG          BuildSpec.texture ──> mod_build (assemble PNGs)
                                                                              │
                                                                   bundle: assets/texture/*.png + texture/manifest.json + TexturePatch
                                                                              │ mod_deploy(game)
                                                                              ▼
                                              per {asset,png}: unpack(index-fast) → replace_texture → cooked tree
                                                       → repack_to_zen(whole tree, game) → ONE triplet → ~mods
                                                       → record triplet in gore-mod DeployRecord
```

## Components

### Backend — `gore-tex` (index + fast extract)
- **`index` module** (new): `TextureIndex { entries: BTreeMap<String, TextureIndexEntry> }`, `TextureIndexEntry { package_id: u64, width: u32, height: u32, format: String }`. `build_index(utoc, usmap, game_dir) -> TextureIndex` — one scan (extend the existing `list_textures` per-package walk to also capture `package_id` (from `PackageInfo::id()`) + dims/format (already parsed)). Serialize/deserialize JSON.
- **Index-aware extract**: `extract_by_package_id(utoc, usmap, package_id) -> TexInfo` (or thread it through a fast path in `unpack_asset`/`decode`): build `FIoChunkId::from_package_id(package_id, 0, ExportBundleData)`, `store.read`, parse — **no package scan**. Existing `extract` (by path, scanning) stays for the non-indexed CLI path.
- **Cache location**: shared gore-tools dir (the same dir `loc_catalog.json` uses, resolved via the existing paths helper). File e.g. `texture_index.json`, with a header recording the game build/usmap name so a game update invalidates it.
- **CLI**: `gore texture index --game <dir> [-o <cache>]` builds + writes the index.

### Backend — `gore-mod` (TexturePatch component)
- `BuildSpec.texture: Vec<TextureReplacement>` where `TextureReplacement { asset: String, image_path: String }`.
- `Component::TexturePatch { path: String, assets: Vec<String> }` (internally-tagged → `{"type":"texture_patch", "path":"texture", "assets":[...]}`).
- `build_bundle` new arm (when `spec.texture` non-empty): copy each PNG into `texture/<idx>_<basename>.png`, write `texture/manifest.json` = `{asset: png_rel}`, push `TexturePatch{path:"texture", assets}`. **No game access** (assembly only).
- Add `gore-tex` to `gore-mod/Cargo.toml`.

### Backend — deploy lifecycle (the divergence)
- New `prepare` arm for `TexturePatch` (it does NOT go through `plan.writes`/`stage`/backup):
  1. Read `texture/manifest.json` from the bundle.
  2. For each `{asset, png}`: index-fast unpack the original cooked files (load the cached index for `package_id`; fall back to a scan-unpack if the index is absent), `replace_texture(orig, png)`, write the resulting cooked `.uasset/.uexp/.ubulk` into a temp cooked tree under the asset's mount path (`/Game/` → `G1R/Content/`).
  3. **One** `repack_to_zen(temp_cooked_tree, name=<mod-derived triplet name>, out, game_dir, compress=false)` → one triplet for all textures.
  4. `gore_tex::container::deploy(triplet, game_root, name)` → copies into `~mods`.
- **Record**: add `#[serde(default)] texture_triplet: Vec<String>` to `gore-mod`'s `DeployRecord` (the deployed `~mods` file paths). `undeploy`/`restore_record` deletes exactly those files (analogous to the `stale_ue4ss_dirs` additive cleanup), in addition to restoring the bak-based components.
- Triplet naming: derived from the mod name, `_P` suffix + a high-sorting prefix so it out-sorts the base container (e.g. `zzz_<modname>_tex_P`).

### Backend — `gore-ffi`
- `texture_index` (`{game, [rebuild]}` → builds if absent/forced, returns the index JSON or a "built" status), `texture_extract` (`{game, asset | package_id}` → writes a preview PNG to a temp path, returns the path; uses the index-fast path). Wire `texture` into `mod_build`/`mod_deploy` (they already take the whole `BuildSpec`/bundle, so once `BuildSpec.texture` exists the existing commands carry it — only the Rust `build_bundle`/`prepare` arms change). Add `gore-tex` to `gore-ffi/Cargo.toml`. Add the dispatch arms.

### Frontend — `apps/mod-studio/lib/textures/`
- `domain/texture_replacements_notifier.dart`: `TextureReplacement{asset, imagePath}` (+`key`, `toJson`/`fromJson`), `TextureReplacementsState{Map<String,TextureReplacement> items}`, `TextureReplacementsNotifier` (`setReplacement`/`remove`/`clearAll`/`loadAll`), `textureReplacementsProvider` (mirror `audio_replacements_notifier.dart`).
- `domain/texture_index_provider.dart`: `textureIndexProvider` (`FutureProvider`) — loads the cached index via `ModFfi.textureIndex(game)`; triggers a build if absent. A `TextureIndexEntry` Dart model.
- `ui/texture_tab.dart`: `TextureTab` (resolves game path from Settings; shows "Set the game path" if absent). Left = search `TextField` filtering the index entries (local, instant) + `ListView` of matching asset paths (with dims/format subtitle). Right = detail: **Preview** (calls `ModFfi.textureExtract` → display the PNG), **Replace…** (`file_selector` `openFile(extensions:['png'])` → `notifier.setReplacement(asset, pngPath)`). A staged-replacements panel listing `{asset → png}` with remove buttons. First-run: if the index is absent, a "Build texture index (~few min)" button → `textureIndexProvider` build (runs in `Isolate.run`, non-blocking) + spinner.

### Frontend — integration points
- `home_page.dart`: `DefaultTabController(length: 5 → 6)`; add `Tab` + `const TextureTab()` at the matching index (after Audio); import; extend the dirty/Build-enabled flag to include `ref.watch(textureReplacementsProvider).count > 0`.
- `project_model.dart`: add `List<TextureReplacement> textures` to `ModProject` ctor/`copyWith`/`toJson` (key `'textures'`)/`fromJson`/**`toBuildSpec()`** (emits the `texture` array consumed by `mod_build`).
- `project_io.dart`: embed PNGs under `assets/textures/<idx>_<basename>.png` in `saveProject` (rewrite paths relative); extract them in `loadProject` with the identical path-traversal guard.
- `project_controller.dart`: include `textureReplacementsProvider` in `gatherProject`/`applyProject`/`newProject` + the dirty signature.
- `build_deploy_dialog.dart`: add `textures` to the `hasContent` count + a "• N texture replacement(s)" Contents line. No other dialog change — `toBuildSpec()` is the single chokepoint.
- `mod_ffi.dart`: `textureIndex(game, {rebuild})`, `textureExtract(game, {asset|packageId})` returning a PNG path, + a `TextureIndexEntry` model.

## Data flow (one texture replacement, end to end)

```
tab: search index → pick asset → Preview (texture_extract, index-fast) → Replace… (pick PNG)
   → textureReplacementsProvider stages {asset, pngPath}
Build/Deploy: gatherProject → ModProject.textures → toBuildSpec().texture
   → mod_build: PNGs → assets/texture/, texture/manifest.json, TexturePatch   (no game)
   → mod_deploy: per {asset,png} unpack(index)→replace_texture→cooked tree
                 → repack_to_zen(tree, game, compress=false) → triplet → ~mods
                 → DeployRecord.texture_triplet = [deployed paths]
undeploy: delete recorded triplet from ~mods (+ restore other components)
```

## Error handling
- Index absent → tab offers to build it; FFI build is long but non-blocking (`Isolate.run`) + a clear "~few min" message.
- Index stale (game build changed) → header mismatch → rebuild prompt.
- PNG dims not multiple-of-4 / not power-of-two → `replace_texture`/`encode_mips` already error clearly; surface the message in the staged panel / deploy result.
- Virtual / unsupported-format textures → `gore-tex` already rejects (`TexError::VirtualTexture`/`UnsupportedFormat`); the tab marks such assets non-replaceable (or surfaces the error on Replace).
- Deploy with no game path → blocked with "set game path" (mirrors audio).
- `repack_to_zen` needs `game_dir` for global script objects (lives in `global.utoc`) — deploy has it; build does not (build never packs).

## Testing
- **Rust `gore-tex`**: gated (real container) — `build_index` resolves `asset_path → package_id`; `extract_by_package_id` decodes to the SAME pixels as the scan-based extract (and is fast). Index JSON round-trips.
- **Rust `gore-mod`**: `build_bundle` with a `texture` spec emits a `TexturePatch` component + `texture/manifest.json` + copied PNGs (no game). Deploy arm: against a fake game dir + a stub triplet, the `~mods` triplet is installed + recorded; `undeploy` removes exactly the recorded files. (The full cook+pack path is exercised by a gated real-game E2E, not a unit test.)
- **Flutter**: `texture_replacements_notifier` set/remove/clear; `ModProject` round-trips a `textures` section through `.goremod` save/load; `build_deploy_dialog` content count includes textures; `flutter analyze` clean.
- **E2E (gated, self-launch)**: author a project with one texture replacement → Build → Deploy to the real game → triplet in `~mods` → launch (self) → confirm load (uncompressed). Undeploy cleans up.

## Phasing
1. **`gore-tex` index + fast extract** (+ CLI `gore texture index`). Foundation; unblocks usable browse/preview.
2. **`gore-mod` `TexturePatch`** component (build arm + deploy/undeploy lifecycle + DeployRecord field) + **`gore-ffi`** `texture_index`/`texture_extract` + `gore-tex` deps.
3. **Flutter Textures tab** + `lib/textures/` + project/build-deploy wiring.
4. **E2E verify** (self-launch a built+deployed bundle).

## Out of scope (v1)
- Compression in bundles (opt-in `--compress` exists but is broken in-game; default off).
- Upscale-specific UI (the engine handles any valid dims; the tab just passes the PNG — upscale "just works" via `replace_texture`).
- Mod stacking / conflict resolution (future separate manager app, per the unified plan).
- Streaming the index build progress (one-time spinner is enough for v1).
- BC7-streamed / `FirstMipToSerialize!=0` edge textures (already gated/rejected by `gore-tex`).
