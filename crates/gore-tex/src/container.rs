//! List texture assets in a cooked UE5 IoStore container.
//!
//! The container holds *cooked* zen packages; an asset's class (e.g. `Texture2D`)
//! is not stored in chunk metadata but inside each package's zen header. For every
//! package we parse the zen header, walk its export map, and compare each export's
//! `class_index` (a `FPackageObjectIndex`) against a single precomputed
//! script-import index for `Texture2D` -- an exact integer/hash match, with no name
//! lookup or table resolution involved. A package is reported as a texture if any
//! of its exports' `class_index` equals that precomputed Texture2D index
//! (`LightMapTexture2D`/`ShadowMapTexture2D` have their own distinct classes, so
//! this is an exact match on `Texture2D` only).
//!
//! This is the "per-package class resolution" route from the task note. The
//! global script-object *table* lives in the engine's `global.utoc`, not in the
//! game's `G1R-Windows.utoc`, so `load_script_objects()` is unavailable here.
//! Instead we reproduce the exact import-hash UE assigns to a script object:
//! `FPackageObjectIndex::create_script_import("/Script/Engine.Texture2D")`
//! (cityhash64 of the lower-cased, slash-normalised path) and compare each
//! export's `class_index` against it. This is an exact, table-free match.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use retoc::asset_conversion::{FZenPackageContext, build_legacy};
use retoc::iostore;
use retoc::logging::Log;
use retoc::script_objects::FPackageObjectIndex;
use retoc::zen::FZenPackageHeader;
use retoc::{Config, EIoChunkType, FIoChunkId, FPackageId, FSFileWriter, UEPath, UEPathBuf};

use crate::error::{Result, TexError};

/// A texture asset discovered in a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureEntry {
    /// Cooked package path, e.g. `/Game/Characters/Hero/T_Hero_BaseColor`.
    pub asset_path: String,
}

/// Full script-object path of the cooked Texture2D class.
const TEXTURE2D_CLASS_PATH: &str = "/Script/Engine.Texture2D";

/// List texture assets in an IoStore container, using `usmap` to resolve types.
///
/// Filters to `UTexture2D`-class exports. `filter` keeps only paths containing the
/// substring.
///
/// Note: `usmap` is accepted for API symmetry with the rest of `gore-tex`; class
/// resolution here is driven by the container's own script-object table (which is
/// exact for the cooked class name) and does not require usmap property parsing.
pub fn list_textures(utoc: &Path, _usmap: &Path, filter: Option<&str>) -> Result<Vec<TextureEntry>> {
    // The script-import index UE assigns to the Texture2D class. Computed the same
    // way the cooker does (cityhash of the normalised path) so we can match it
    // without the engine's global script-object table.
    let texture2d_class = FPackageObjectIndex::create_script_import(TEXTURE2D_CLASS_PATH);

    let paths = collect_package_paths(utoc, Some(texture2d_class))?;
    Ok(paths
        .into_iter()
        .filter(|p| filter.is_none_or(|f| p.contains(f)))
        .map(|asset_path| TextureEntry { asset_path })
        .collect())
}

/// List every package asset path in the container at `utoc` (standalone foreign
/// triplets OK: `iostore::open` dispatches a file path to a single-container
/// store). Returns sorted, deduped cooked package paths, e.g.
/// `/Game/Characters/Hero/T_Hero_BaseColor` -- the mod-manager uses this to
/// detect asset overlaps between mods.
pub fn list_packages(utoc: &Path) -> Result<Vec<String>> {
    collect_package_paths(utoc, None)
}

/// Shared per-package scan behind `list_textures`/`list_packages`: parse every
/// package's zen header and collect its asset path, keeping only packages with an
/// export of class `class_filter` when one is given (every package when `None`).
/// Returns sorted, deduped paths.
fn collect_package_paths(
    utoc: &Path,
    class_filter: Option<FPackageObjectIndex>,
) -> Result<Vec<String>> {
    let store = iostore::open(utoc, Arc::new(Config::default()))?;

    let container_version = store
        .container_file_version()
        .ok_or_else(|| anyhow::anyhow!("container has no TOC version"))?;
    let header_version = store
        .container_header_version()
        .ok_or_else(|| anyhow::anyhow!("container has no header version"))?;

    let mut out = Vec::new();

    // Silence the default panic hook for the duration of the loop so the panics we
    // intentionally catch below (one per malformed package) don't spam stderr with
    // backtraces. Restored before returning.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    for pkg in store.packages() {
        let pkg_id = pkg.id();

        // The per-package work below is panic-safe: a malformed package can not only
        // return `Err` from `read`/`deserialize` but also *panic* deeper in retoc --
        // e.g. `header.package_name()` -> `FNameMap::get` asserts on name kind and
        // indexes `self.names` unchecked, so an out-of-range name index aborts. We
        // wrap the whole body in `catch_unwind` so one bad package is skipped, not
        // fatal to the entire listing. The closure returns `Some(path)` for a
        // matching package, `None` for a non-match or any handled failure; a caught
        // panic is treated exactly like the previous `Err(_) => continue` path.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let chunk_id =
                FIoChunkId::from_package_id(pkg_id, 0, EIoChunkType::ExportBundleData);

            // Some package entries may not have a readable export-bundle chunk; skip
            // them rather than failing the whole listing.
            let data = match store.read(chunk_id) {
                Ok(d) => d,
                Err(_) => return None,
            };

            let header = match FZenPackageHeader::deserialize(
                &mut Cursor::new(&data),
                store.package_store_entry(pkg_id),
                container_version,
                header_version,
                None,
            ) {
                Ok(h) => h,
                Err(_) => return None,
            };

            // Does any export match the requested class? Compare each export's class
            // import index to the precomputed script-import index (no filter == keep).
            let keep = class_filter.is_none_or(|class| {
                header
                    .export_map
                    .iter()
                    .any(|export| export.class_index == class)
            });

            if !keep {
                return None;
            }

            Some(header.package_name())
        }));

        // Caught panic == skip this package (same as the `Err(_) => continue` arms).
        if let Ok(Some(path)) = result {
            out.push(path);
        }
    }

    std::panic::set_hook(prev_hook);

    out.sort();
    out.dedup();
    Ok(out)
}

/// List the file entry paths of a plain (non-IoStore) V11 `.pak`, sorted and
/// deduped. Paths are as recorded in the pak index (relative to its mount
/// point), e.g. `G1R/Content/UI/Textures/Common/T_HardwareCursor.uasset` -- the
/// mod-manager uses this to inspect foreign pak-only mods.
pub fn list_pak_files(pak: &Path) -> Result<Vec<String>> {
    let mut file = std::io::BufReader::new(std::fs::File::open(pak)?);
    let reader = repak::PakBuilder::new()
        .reader(&mut file)
        .map_err(|e| anyhow::anyhow!("failed to read pak index of {}: {e}", pak.display()))?;
    let mut files = reader.files();
    files.sort();
    files.dedup();
    Ok(files)
}

/// Unpack a single asset's cooked files (.uasset/.uexp/.ubulk) from the
/// container into `out_dir`. Returns the path to the written `.uasset`.
///
/// The asset is converted from its on-disk *zen* (IoStore) form back to the
/// legacy cooked `.uasset`/`.uexp`/`.ubulk` layout via retoc's
/// `asset_conversion::build_legacy`. That conversion resolves the package's
/// script imports against the engine's *global* script-object table, which for
/// G1R lives in `global.utoc` -- a sibling of the main container, **not** inside
/// `G1R-Windows.utoc`. So we open the whole Paks *directory* (the parent of
/// `utoc`) as a composite store: `IoStoreBackend` then exposes the global
/// script objects through the default `load_script_objects()` while still
/// serving the asset's chunks. (Opening the single `.utoc` file would make
/// `build_legacy` fail to resolve script imports.)
///
/// `usmap` is accepted for API symmetry; the zen->legacy conversion is driven
/// entirely by the package's own header + the global script-object table and
/// does not need property mappings.
pub fn unpack_asset(
    utoc: &Path,
    _usmap: &Path,
    asset_path: &str,
    out_dir: &Path,
) -> Result<PathBuf> {
    // Open the directory holding the .utoc so the composite store also picks up
    // `global.utoc` (script objects) -- required for build_legacy to resolve
    // script imports. Fall back to the file itself if it has no parent.
    let store_path = utoc.parent().unwrap_or(utoc);
    let store = iostore::open(store_path, Arc::new(Config::default()))?;

    let container_version = store
        .container_file_version()
        .ok_or_else(|| anyhow::anyhow!("container has no TOC version"))?;
    let header_version = store
        .container_header_version()
        .ok_or_else(|| anyhow::anyhow!("container has no header version"))?;

    // Locate the package whose name == asset_path. Reuses the per-package
    // header-parsing route from `list_textures` (panic-safe: a malformed package
    // can panic deep in retoc; one bad package must not abort the whole search).
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let mut found: Option<FPackageId> = None;
    for pkg in store.packages() {
        let pkg_id = pkg.id();
        let matches = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let chunk_id =
                FIoChunkId::from_package_id(pkg_id, 0, EIoChunkType::ExportBundleData);
            let data = match store.read(chunk_id) {
                Ok(d) => d,
                Err(_) => return false,
            };
            let header = match FZenPackageHeader::deserialize(
                &mut Cursor::new(&data),
                store.package_store_entry(pkg_id),
                container_version,
                header_version,
                None,
            ) {
                Ok(h) => h,
                Err(_) => return false,
            };
            header.package_name() == asset_path
        }));
        if let Ok(true) = matches {
            found = Some(pkg_id);
            break;
        }
    }

    std::panic::set_hook(prev_hook);

    let package_id = found.ok_or_else(|| TexError::AssetNotFound(asset_path.into()))?;

    std::fs::create_dir_all(out_dir)?;
    let leaf = asset_path.rsplit('/').next().unwrap_or(asset_path);
    legacy_from_package(store.as_ref(), package_id, leaf, out_dir)
}

/// Like `unpack_asset` but takes the package id directly (from the texture index),
/// skipping the full-container name scan. Opens the parent Paks dir so global script
/// objects resolve (same as `unpack_asset`). `leaf` is the output filename stem.
pub fn unpack_asset_by_id(
    utoc: &Path,
    _usmap: &Path,
    package_id: u64,
    leaf: &str,
    out_dir: &Path,
) -> Result<PathBuf> {
    let store_path = utoc.parent().unwrap_or(utoc);
    let store = iostore::open(store_path, Arc::new(Config::default()))?;
    std::fs::create_dir_all(out_dir)?;
    legacy_from_package(store.as_ref(), FPackageId(package_id), leaf, out_dir)
}

/// Shared zen->legacy conversion tail. Given an already-resolved `FPackageId` and an
/// open (composite) store, builds the legacy cooked `.uasset`/`.uexp`/`.ubulk` into
/// `out_dir` (named after `leaf`) and returns the `.uasset` path.
///
/// `build_legacy` writes paths *relative* to the FSFileWriter's root dir, so we name
/// the output after `leaf` and root the writer at `out_dir`: the
/// .uasset/.uexp/.ubulk land directly in out_dir sharing the same stem (so
/// `with_extension(..)` finds siblings).
fn legacy_from_package(
    store: &dyn iostore::IoStoreTrait,
    package_id: FPackageId,
    leaf: &str,
    out_dir: &Path,
) -> Result<PathBuf> {
    let out_rel = format!("{leaf}.uasset");

    let log = Log::no_log();
    // No verse script cells store: G1R textures are plain UTexture2D, and script
    // cells are only needed to resolve Verse cell imports (none here).
    let context = FZenPackageContext::create(store, None, &log, None);
    let writer = FSFileWriter::new(out_dir);

    build_legacy(&context, package_id, UEPath::new(&out_rel), &writer)?;

    let uasset = out_dir.join(format!("{leaf}.uasset"));
    Ok(uasset)
}

/// Pack a directory of edited legacy cooked files (laid out under their mount
/// path, e.g. `cooked_dir/G1R/Content/UI/Textures/Common/T_HardwareCursor.uasset`)
/// into a Zen triplet `out_dir/<name>.{utoc,ucas,pak}`, UE5.4. Returns the 3 paths
/// `[utoc, ucas, pak]`.
///
/// `compress` is opt-in. With `compress == false` (the default at every call
/// site) chunks are written UNCOMPRESSED (method 0, `container_flags = 8`) --
/// valid and game-loadable (UE mounts uncompressed IoStore fine), and proven to
/// work in-game. With `compress == true` the writer Oodle-compresses `.ucas`
/// blocks (16-aligned, `container_flags = Indexed|Compressed = 9`); the
/// compression code is wired and framing-fixed but the game currently ignores
/// our compressed containers (unresolved Oodle framing/encoder issue), so it is
/// off by default.
///
/// This re-implements retoc's `to-zen` orchestration (`action_to_zen`, which was
/// CLI-only) on top of the vendored lib's `pub` building blocks:
///   1. open the game's Paks *directory* as a composite source store so its
///      *global* script objects (in `global.utoc`) resolve -- exactly mirroring
///      `unpack_asset`; `build_zen_asset` needs them to resolve each package's
///      script imports;
///   2. for every `.uasset` (with a sibling `.uexp`) found under `cooked_dir`,
///      read the legacy cooked bytes into an `FSerializedAssetBundle`;
///   3. `build_zen_asset(...)` (UE5_4: `NoExportInfo` header, `OnDemandMetaData`
///      toc, `PropertyTagCompleteTypeName` pkg version) with mount point
///      `../../../` and asset path `../../../<relative-cooked-path>`;
///   4. `ConvertedZenAssetBundle::write` into the `IoStoreWriter`, then `finalize`
///      (which serialises the TOC + container-header chunk);
///   5. emit the empty `.pak` stub the game needs to detect/mount the container.
///
/// `game_dir` *is* required: a plain texture's package still references the
/// Texture2D script class, whose `FPackageObjectIndex` must resolve against the
/// global script-object table -- which lives in the game's `global.utoc`, not in
/// the cooked input. Passing `None` for `script_objects` produces a container the
/// game rejects (unresolved script imports). Verified: script objects ARE needed.
pub fn repack_to_zen(
    cooked_dir: &Path,
    name: &str,
    out_dir: &Path,
    game_dir: &Path,
    compress: bool,
) -> Result<[PathBuf; 3]> {
    use retoc::iostore_writer::IoStoreWriter;
    use retoc::legacy_asset::FSerializedAssetBundle;
    use retoc::version::EngineVersion;
    use retoc::zen_asset_conversion::build_zen_asset;
    use retoc::{UEPath, UEPathBuf, build_verse_cell_store};

    let ver = EngineVersion::UE5_4;
    let toc_version = ver.toc_version();
    let header_version = ver.container_header_version();
    let pkg_file_version = ver.package_file_version();
    let mount_point = UEPath::new("../../../");

    // 1. Open the game's Paks directory as a composite source store so the global
    //    script objects (in `global.utoc`) are available -- same rationale as
    //    `unpack_asset`. `build_zen_asset` resolves each package's script imports
    //    against these; without them the container's imports are unresolved and the
    //    game refuses to load it.
    let paks_dir = game_dir.join("G1R/Content/Paks");
    let store = iostore::open(&paks_dir, Arc::new(Config::default()))?;
    let script_objects = Some(Arc::new(store.load_script_objects()?));

    // No Verse cells in plain cooked textures; an empty store mirrors the CLI's
    // `Some(script_cell_store)` arg (the CLI always passes a constructed store).
    let script_cells = Some(build_verse_cell_store(&Vec::new()));

    // 2. Collect every `.uasset` (with a sibling `.uexp`) under `cooked_dir`, as a
    //    path relative to `cooked_dir` (becomes the cooked/pak path inside the
    //    mount, e.g. `G1R/Content/UI/Textures/Common/T_HardwareCursor.uasset`).
    let mut asset_rels: Vec<PathBuf> = Vec::new();
    collect_uassets(cooked_dir, cooked_dir, &mut asset_rels)?;
    if asset_rels.is_empty() {
        return Err(TexError::AssetNotFound(format!(
            "no .uasset (with sibling .uexp) found under {}",
            cooked_dir.display()
        )));
    }

    // 3-4. Open the writer and convert+write each asset.
    std::fs::create_dir_all(out_dir)?;
    let utoc_path = out_dir.join(format!("{name}.utoc"));
    let writer = IoStoreWriter::new(
        &utoc_path,
        toc_version,
        Some(header_version),
        UEPathBuf::from(mount_point),
    )?;
    // Compression is opt-in. Default (`compress == false`) writes raw blocks
    // (`container_flags = 8`) -- the proven-in-game uncompressed path. When
    // `compress == true` the writer Oodle-compresses blocks and the container is
    // flagged `Indexed|Compressed` (9).
    let mut writer = writer.set_compress(compress);

    let log = Log::no_log();
    for rel in &asset_rels {
        let abs = cooked_dir.join(rel);
        // The path handed to `build_zen_asset` is the mount-relative cooked path
        // (forward-slash, UE-style), prefixed with the `../../../` mount point.
        let rel_ue = path_to_ue(rel);
        let asset_ue_path = mount_point.join(&rel_ue);

        let bundle = FSerializedAssetBundle {
            asset_file_buffer: std::fs::read(&abs)?,
            exports_file_buffer: std::fs::read(abs.with_extension("uexp"))?,
            bulk_data_buffer: read_opt(&abs.with_extension("ubulk"))?,
            optional_bulk_data_buffer: read_opt(&abs.with_extension("uptnl"))?,
            // `.m.ubulk` -> the leaf gains a `.m` before `.ubulk`.
            memory_mapped_bulk_data_buffer: read_opt(&with_double_ext(&abs, "m.ubulk"))?,
        };

        let mut converted = build_zen_asset(
            bundle,
            &std::collections::HashMap::new(), // no referenced shader maps for a plain texture
            &asset_ue_path,
            Some(pkg_file_version),
            header_version,
            false, // allow_fixup: UE4-only external-arc fixup; false for UE5_4 (NoExportInfo)
            script_objects.clone(),
            script_cells.clone(),
            &log,
        )?;

        // NoExportInfo > Initial, so no import fix-up pass is needed: write directly.
        converted.write(&mut writer)?;
    }

    // 5. Serialise the TOC + container-header chunk.
    writer.finalize()?;

    // The game needs an (even empty) `.pak` sidecar to detect and mount the
    // IoStore container -- mirrors retoc's `action_to_zen`.
    let pak_path = out_dir.join(format!("{name}.pak"));
    {
        use std::io::BufWriter;
        let mut pak_file = BufWriter::new(std::fs::File::create(&pak_path)?);
        repak::PakBuilder::new()
            .writer(
                &mut pak_file,
                repak::Version::V11,
                mount_point.to_string(),
                None,
            )
            .write_index()
            .map_err(|e| anyhow::anyhow!("failed to write empty .pak index: {e}"))?;
    }

    let ucas_path = utoc_path.with_extension("ucas");
    Ok([utoc_path, ucas_path, pak_path])
}

/// The game's IoStore override folder: containers dropped here are mounted on top
/// of the base game (additive override; later-mounting wins).
fn mods_dir(game_dir: &Path) -> PathBuf {
    game_dir.join("G1R/Content/Paks/~mods")
}

/// On-disk record of a deployed container, written next to the triplet as
/// `<name>.gore-deploy.json`. Lists the absolute paths of every file we copied in
/// so `undeploy` can remove exactly what it added (and nothing else).
#[derive(serde::Serialize, serde::Deserialize)]
struct DeployRecord {
    name: String,
    files: Vec<PathBuf>,
}

/// Copy a Zen triplet (`[utoc, ucas, pak]`) into the game's `~mods` override folder
/// and write a JSON deploy record listing the copied file paths. Returns the path to
/// the record (`<mods>/<name>.gore-deploy.json`).
///
/// Non-destructive: this is an *additive* override -- the game mounts the `~mods`
/// container on top of the base game, so nothing in the base install is modified or
/// backed up. `undeploy` reverses it by deleting exactly the files this recorded.
pub fn deploy(triplet: &[PathBuf; 3], game_dir: &Path, name: &str) -> Result<PathBuf> {
    let mods = mods_dir(game_dir);
    std::fs::create_dir_all(&mods)?;
    // Canonicalize the mods dir so the deploy record holds ABSOLUTE paths even when
    // `game_dir` is relative (e.g. `--game .`). Otherwise a later `undeploy --game
    // <absolute>` run from a different cwd would resolve the recorded relative paths
    // against the wrong directory and fail to remove the mounted triplet. Falls back
    // to the un-canonicalized path if canonicalize fails (dir was just created, so it
    // should succeed).
    let mods = std::fs::canonicalize(&mods).unwrap_or(mods);

    // Crash-safety + rollback: write the deploy RECORD first, then copy the triplet
    // files. The record journals the intended destinations, so if the process is
    // killed or power is lost mid-copy, a record always exists for `undeploy` (or a
    // later redeploy) to remove the partial triplet — the copied .utoc/.ucas/.pak
    // never linger mounted with nothing to find them. Each on-disk mutation (the
    // record, then each triplet file) snapshots its PRIOR bytes before being
    // overwritten; on any RETURNED error `cleanup` restores those bytes (`Some`) or
    // removes a genuinely-new file (`None`), so a failed (re)deploy leaves the
    // previous state intact. An existing-but-unreadable file aborts before any
    // write rather than risk a rollback deleting it as if it were fresh.
    let mut written: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::with_capacity(4);
    let cleanup = |written: &[(PathBuf, Option<Vec<u8>>)]| {
        for (f, prior) in written.iter().rev() {
            match prior {
                Some(bytes) => {
                    let _ = std::fs::write(f, bytes);
                }
                None => {
                    let _ = std::fs::remove_file(f);
                }
            }
        }
    };
    // Snapshot prior bytes of a path we're about to overwrite. NotFound -> None
    // (fresh add). Any other error -> abort (we can't safely roll it back).
    let snapshot = |path: &Path| -> Result<Option<Vec<u8>>> {
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    };

    // Resolve destination paths up front — the record must list them before any
    // file is copied.
    let mut dsts: Vec<PathBuf> = Vec::with_capacity(3);
    for src in triplet {
        let leaf = src.file_name().ok_or_else(|| {
            TexError::AssetNotFound(format!("triplet path has no file name: {}", src.display()))
        })?;
        dsts.push(mods.join(leaf));
    }

    let record_path = mods.join(format!("{name}.gore-deploy.json"));
    let record = DeployRecord {
        name: name.to_string(),
        files: dsts.clone(),
    };
    let json = serde_json::to_string_pretty(&record)
        .map_err(|e| TexError::Retoc(anyhow::anyhow!("serialising deploy record: {e}")))?;

    // 1. Write the record FIRST (atomically: temp sibling + rename, so an existing
    //    record is never left truncated). Register its prior bytes for rollback.
    let prior_record = snapshot(&record_path)?;
    written.push((record_path.clone(), prior_record));
    let tmp_record = mods.join(format!("{name}.gore-deploy.json.tmp"));
    if let Err(e) = std::fs::write(&tmp_record, &json) {
        let _ = std::fs::remove_file(&tmp_record);
        cleanup(&written);
        return Err(e.into());
    }
    if let Err(e) = std::fs::rename(&tmp_record, &record_path) {
        let _ = std::fs::remove_file(&tmp_record);
        cleanup(&written);
        return Err(e.into());
    }

    // 2. Copy each triplet file, snapshotting its prior bytes before the copy
    //    (std::fs::copy creates/truncates the dst first, so a mid-copy failure
    //    leaves a partial file the rollback must restore or remove).
    for (src, dst) in triplet.iter().zip(dsts.iter()) {
        let prior = match snapshot(dst) {
            Ok(p) => p,
            Err(e) => {
                cleanup(&written);
                return Err(e);
            }
        };
        written.push((dst.clone(), prior));
        if let Err(e) = std::fs::copy(src, dst) {
            cleanup(&written);
            return Err(e.into());
        }
    }

    Ok(record_path)
}

/// Read `<mods>/<name>.gore-deploy.json` and delete every file it lists plus the
/// record itself. Individually-missing files are tolerated (reported to stderr) so a
/// partially-cleaned deploy can still be finished. Errors if the record is absent.
pub fn undeploy(game_dir: &Path, name: &str) -> Result<()> {
    let mods = mods_dir(game_dir);
    let record_path = mods.join(format!("{name}.gore-deploy.json"));
    if !record_path.exists() {
        return Err(TexError::DeployRecordNotFound(record_path));
    }

    let json = std::fs::read_to_string(&record_path)?;
    let record: DeployRecord = serde_json::from_str(&json)
        .map_err(|e| TexError::Retoc(anyhow::anyhow!("parsing deploy record {}: {e}", record_path.display())))?;

    for f in &record.files {
        match std::fs::remove_file(f) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("warning: deployed file already gone: {}", f.display());
            }
            Err(e) => return Err(e.into()),
        }
    }

    std::fs::remove_file(&record_path)?;
    Ok(())
}

/// Recursively collect `.uasset` files (that have a sibling `.uexp`) under `dir`,
/// pushing each as a path relative to `root`.
fn collect_uassets(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_uassets(root, &path, out)?;
        } else if path.extension().is_some_and(|e| e == "uasset")
            && path.with_extension("uexp").exists()
        {
            if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_path_buf());
            }
        }
    }
    Ok(())
}

/// `std::fs::read` but `Ok(None)` when the file is absent.
fn read_opt(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Replace a path's final extension with a compound one (e.g. `T_X.uasset` ->
/// `T_X.m.ubulk`).
fn with_double_ext(path: &Path, compound_ext: &str) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_os_string()).unwrap_or_default();
    let mut name = stem;
    name.push(".");
    name.push(compound_ext);
    path.with_file_name(name)
}

/// Convert an OS relative path to a forward-slash UE path string.
fn path_to_ue(rel: &Path) -> UEPathBuf {
    let s = rel.to_string_lossy().replace('\\', "/");
    UEPathBuf::from(s)
}

impl PartialOrd for TextureEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TextureEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.asset_path.cmp(&other.asset_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn game_dir() -> Option<PathBuf> {
        let p = PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        p.exists().then_some(p)
    }

    /// A unique throwaway dir under the system temp dir (no `tempfile` dep).
    fn unique_tmp(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "gore-tex-test-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Build a fake triplet `[utoc, ucas, pak]` of small files in `dir`.
    fn fake_triplet(dir: &Path, stem: &str) -> [PathBuf; 3] {
        let exts = ["utoc", "ucas", "pak"];
        let mut out: Vec<PathBuf> = Vec::new();
        for ext in exts {
            let p = dir.join(format!("{stem}.{ext}"));
            std::fs::write(&p, format!("{stem}.{ext} contents").as_bytes()).unwrap();
            out.push(p);
        }
        [out[0].clone(), out[1].clone(), out[2].clone()]
    }

    /// [5] Deploying with a NON-canonical / relative-style `game_dir` must still
    /// record ABSOLUTE paths, so an `undeploy` invoked with a differently-spelled
    /// (absolute) `game_dir` from another cwd resolves them correctly. We pass a
    /// game dir containing a `.` component (the same non-canonical shape `--game .`
    /// produces) and assert every recorded file path is absolute, then undeploy via
    /// the plain absolute dir and confirm the recorded files are gone.
    #[test]
    fn deploy_records_absolute_paths_for_relative_game_dir() {
        let base = unique_tmp("relgame");
        let src_dir = base.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let triplet = fake_triplet(&src_dir, "zzz_mod_tex_P");

        // Non-canonical game dir: `<base>/./.` — `canonicalize` in `deploy` must
        // collapse this so the record holds absolute, canonical paths rather than a
        // path carrying the `.` components.
        let noncanon_game = base.join(".").join(".");
        let record_path = deploy(&triplet, &noncanon_game, "zzz_mod_tex_P").unwrap();
        let json = std::fs::read_to_string(&record_path).unwrap();
        let record: DeployRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.files.len(), 3);
        for f in &record.files {
            assert!(f.is_absolute(), "record path not absolute: {}", f.display());
            assert!(f.exists(), "record path missing: {}", f.display());
            assert!(
                !f.components().any(|c| c == std::path::Component::CurDir),
                "record path not canonical (has '.'): {}",
                f.display()
            );
        }

        // Undeploy via the plain absolute base (different spelling) still finds and
        // removes exactly the recorded files + the record.
        undeploy(&base, "zzz_mod_tex_P").unwrap();
        for f in &record.files {
            assert!(!f.exists(), "undeploy left file: {}", f.display());
        }
        assert!(!record_path.exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// [4] If a triplet copy fails partway, no files from this `deploy` call may be
    /// left in `~mods` (and no record is written) — otherwise a partial IoStore set
    /// mounts on next launch with nothing for undeploy to remove. We force failure
    /// by giving a triplet whose 2nd entry's source does not exist.
    #[test]
    fn deploy_rolls_back_partial_on_copy_failure() {
        let base = unique_tmp("partial");
        let src_dir = base.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        // First src exists; second does NOT -> copy of #2 fails after #1 copied.
        let s0 = src_dir.join("zzz_mod_tex_P.utoc");
        std::fs::write(&s0, b"utoc").unwrap();
        let s1 = src_dir.join("zzz_mod_tex_P.ucas"); // intentionally NOT created
        let s2 = src_dir.join("zzz_mod_tex_P.pak");
        std::fs::write(&s2, b"pak").unwrap();
        let triplet = [s0, s1, s2];

        let err = deploy(&triplet, &base, "zzz_mod_tex_P");
        assert!(err.is_err(), "expected deploy to fail on missing src");

        // The first file's copy succeeded then was rolled back: ~mods must hold
        // neither the copied file nor a deploy record.
        let mods = mods_dir(&base);
        if mods.exists() {
            let leftovers: Vec<_> = std::fs::read_dir(&mods)
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.file_name()))
                .collect();
            assert!(
                leftovers.is_empty(),
                "partial deploy left files in ~mods: {leftovers:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Redeploying an already-deployed name overwrites the live `~mods` triplet.
    /// If a later copy fails, rollback must RESTORE the previous triplet's bytes,
    /// not delete them (deletion would wipe the working deployment). Deploy v1,
    /// then attempt v2 (same leaf names, 2nd src missing) and assert the first
    /// destination still holds v1's bytes and the old record survives.
    #[test]
    fn deploy_redeploy_failure_restores_existing_triplet() {
        let base = unique_tmp("redeploy");
        let src1 = base.join("src1");
        std::fs::create_dir_all(&src1).unwrap();
        let v1 = fake_triplet(&src1, "zzz_mod_tex_P");
        deploy(&v1, &base, "zzz_mod_tex_P").unwrap();
        let mods = std::fs::canonicalize(mods_dir(&base)).unwrap();
        let dst_utoc = mods.join("zzz_mod_tex_P.utoc");
        let v1_utoc = std::fs::read(&dst_utoc).unwrap();

        // v2: same leaf names so it targets the same destinations; 2nd src missing
        // so the copy fails AFTER the first destination was overwritten.
        let src2 = base.join("src2");
        std::fs::create_dir_all(&src2).unwrap();
        let s0 = src2.join("zzz_mod_tex_P.utoc");
        std::fs::write(&s0, b"V2 NEW UTOC BYTES").unwrap();
        let s1 = src2.join("zzz_mod_tex_P.ucas"); // intentionally NOT created
        let s2 = src2.join("zzz_mod_tex_P.pak");
        std::fs::write(&s2, b"v2 pak").unwrap();
        let v2 = [s0, s1, s2];

        let err = deploy(&v2, &base, "zzz_mod_tex_P");
        assert!(err.is_err(), "expected redeploy to fail on missing src");

        assert!(dst_utoc.exists(), "redeploy failure deleted the existing triplet");
        assert_eq!(
            std::fs::read(&dst_utoc).unwrap(),
            v1_utoc,
            "existing triplet bytes were not restored on rollback"
        );
        assert!(
            mods.join("zzz_mod_tex_P.gore-deploy.json").exists(),
            "old deploy record was removed"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    #[ignore = "slow: full container scan; run with --ignored"]
    fn lists_textures_from_real_container() {
        let Some(g) = game_dir() else {
            eprintln!("skip: game not installed");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();

        let all = list_textures(&utoc, &usmap, None).unwrap();
        eprintln!("total textures: {}", all.len());
        for e in all.iter().take(20) {
            eprintln!("  {}", e.asset_path);
        }
        assert!(all.len() > 100, "expected many textures, got {}", all.len());

        let filtered = list_textures(&utoc, &usmap, Some("Hero")).unwrap();
        eprintln!("filtered (Hero): {}", filtered.len());
        for e in filtered.iter().take(20) {
            eprintln!("  {}", e.asset_path);
        }
        assert!(filtered.len() <= all.len());
        assert!(filtered.iter().all(|e| e.asset_path.contains("Hero")));
    }

    #[test]
    #[ignore = "slow: full container scan; run with --ignored"]
    fn unpacks_one_texture_asset() {
        let Some(g) = game_dir() else {
            eprintln!("skip: game not installed");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();

        // Take the first "T_" texture the container actually contains, so the
        // test stays valid even if a specific path is renamed by a game patch.
        let textures = list_textures(&utoc, &usmap, Some("T_")).unwrap();
        let asset = textures
            .first()
            .map(|e| e.asset_path.clone())
            .expect("expected at least one T_ texture in the container");
        eprintln!("unpacking asset: {asset}");

        let tmp = std::env::temp_dir().join("gore-tex-unpack-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let uasset = unpack_asset(&utoc, &usmap, &asset, &tmp).unwrap();
        assert!(uasset.exists());
        assert!(std::fs::metadata(&uasset).unwrap().len() > 0);

        let uexp = uasset.with_extension("uexp");
        let ubulk = uasset.with_extension("ubulk");
        eprintln!(
            "unpacked: {:?} ({} bytes); siblings: uexp={} ({} bytes) ubulk={} ({} bytes)",
            uasset,
            std::fs::metadata(&uasset).unwrap().len(),
            uexp.exists(),
            uexp.exists().then(|| std::fs::metadata(&uexp).unwrap().len()).unwrap_or(0),
            ubulk.exists(),
            ubulk.exists().then(|| std::fs::metadata(&ubulk).unwrap().len()).unwrap_or(0),
        );
    }

    /// `list_pak_files` over a tiny plain V11 pak built with the same repak writer
    /// API `repack_to_zen` uses. Entries are written out of order to prove the
    /// returned list is sorted.
    #[test]
    fn list_pak_files_reads_v11_pak() {
        let dir = unique_tmp("listpak");
        let pak_path = dir.join("tiny.pak");
        {
            use std::io::BufWriter;
            let mut pak_file = BufWriter::new(std::fs::File::create(&pak_path).unwrap());
            let mut w = repak::PakBuilder::new().writer(
                &mut pak_file,
                repak::Version::V11,
                "../../../".to_string(),
                None,
            );
            w.write_file("G1R/Content/B.txt", false, b"bee").unwrap();
            w.write_file("G1R/Content/A.txt", false, b"aye").unwrap();
            w.write_index().unwrap();
        }

        let files = list_pak_files(&pak_path).unwrap();
        assert_eq!(
            files,
            vec!["G1R/Content/A.txt".to_string(), "G1R/Content/B.txt".to_string()]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A nonexistent container path must surface as an error (io-ish TexError),
    /// not a panic or an empty listing.
    #[test]
    fn list_packages_missing_file_errors() {
        let dir = unique_tmp("nopkg");
        let missing = dir.join("does_not_exist.utoc");
        let err = list_packages(&missing).unwrap_err();
        assert!(
            matches!(err, TexError::Retoc(_) | TexError::Io(_)),
            "expected io-ish error, got: {err:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `list_packages` against the real main container: every package (not just
    /// textures) is listed, so the result must be much larger than the texture
    /// listing and include `/Game/` paths.
    #[test]
    #[ignore = "slow: full container scan; run with --ignored"]
    fn list_packages_main_container_nonempty() {
        let Some(g) = game_dir() else {
            eprintln!("skip: game not installed");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();

        let all = list_packages(&utoc).unwrap();
        eprintln!("total packages: {}", all.len());
        for p in all.iter().take(20) {
            eprintln!("  {p}");
        }
        assert!(all.len() > 1000, "expected many packages, got {}", all.len());
        assert!(
            all.iter().any(|p| p.starts_with("/Game/")),
            "expected at least one /Game/ package path"
        );
        // Sorted + deduped contract.
        assert!(all.windows(2).all(|w| w[0] < w[1]), "paths not sorted/deduped");
    }

    /// The real-container test above needs the game installed; this fast test pins
    /// the panic-safety contract our per-package loop relies on, with no I/O.
    ///
    /// A malformed package can panic deep in retoc (e.g. `FNameMap::get`'s
    /// `assert_eq!`/unchecked index, see module docs). Constructing such a package
    /// for a unit test would require crafting a full on-disk IoStore container with
    /// a deliberately corrupt zen header -- too expensive to be worthwhile. Instead
    /// we verify the exact mechanism the loop uses: a panic inside the per-package
    /// closure is caught and turned into "skip" (`None`), the surviving packages are
    /// still collected, and the panic hook is restored afterwards.
    #[test]
    fn panicking_package_is_skipped_not_fatal() {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let mut out: Vec<u32> = Vec::new();
        // Package 1 -> ok, package 2 -> panics (stand-in for FNameMap::get aborting),
        // package 3 -> ok. A non-panic-safe loop would die on package 2.
        for pkg in [1u32, 2, 3] {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if pkg == 2 {
                    let names: Vec<&str> = Vec::new();
                    // Unchecked out-of-range index, mirroring `self.names[idx]`.
                    return Some(names[5].len() as u32);
                }
                Some(pkg)
            }));
            if let Ok(Some(v)) = result {
                out.push(v);
            }
        }

        std::panic::set_hook(prev_hook);

        // Bad package skipped; good packages survived.
        assert_eq!(out, vec![1, 3]);
    }

    /// deploy/undeploy against a fake game dir (no real container needed): deploy
    /// copies the triplet + writes the record into `~mods`; undeploy removes all 4
    /// and leaves `~mods` empty.
    #[test]
    fn deploy_then_undeploy_roundtrip() {
        let base = std::env::temp_dir().join(format!("gore-tex-deploy-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let game = base.join("game");
        let src = base.join("src");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(&src).unwrap();

        let name = "zzz_X_P";
        let triplet = [
            src.join(format!("{name}.utoc")),
            src.join(format!("{name}.ucas")),
            src.join(format!("{name}.pak")),
        ];
        for p in &triplet {
            std::fs::write(p, b"dummy").unwrap();
        }

        let mods = game.join("G1R/Content/Paks/~mods");

        // deploy: 3 triplet files + the record exist under ~mods. `deploy`
        // canonicalizes the mods dir (so records hold absolute paths even for a
        // relative `--game`), so compare canonicalized paths rather than the exact
        // spelling.
        let record = deploy(&triplet, &game, name).unwrap();
        assert_eq!(
            std::fs::canonicalize(&record).unwrap(),
            std::fs::canonicalize(mods.join(format!("{name}.gore-deploy.json"))).unwrap()
        );
        for ext in ["utoc", "ucas", "pak"] {
            assert!(
                mods.join(format!("{name}.{ext}")).exists(),
                "missing deployed .{ext}"
            );
        }
        assert!(record.exists(), "missing deploy record");

        // undeploy: all 4 gone, ~mods is empty.
        undeploy(&game, name).unwrap();
        for ext in ["utoc", "ucas", "pak"] {
            assert!(
                !mods.join(format!("{name}.{ext}")).exists(),
                ".{ext} not removed"
            );
        }
        assert!(!record.exists(), "record not removed");
        assert_eq!(
            std::fs::read_dir(&mods).unwrap().count(),
            0,
            "~mods should be empty after undeploy"
        );

        // undeploy again -> record-missing error.
        let err = undeploy(&game, name).unwrap_err();
        assert!(matches!(err, TexError::DeployRecordNotFound(_)));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// The to-zen write-path oracle: unpack an UNCHANGED asset, repack the cooked
    /// files into a fresh Zen triplet, then read the asset back OUT of that triplet
    /// and confirm it decodes to the SAME pixels. Proves `repack_to_zen` (legacy ->
    /// zen conversion + FBulkDataMapEntry regeneration + the IoStore writer) yields
    /// a valid, game-readable container.
    #[test]
    #[ignore = "slow: unpack + repack against real container"]
    fn repack_unchanged_roundtrips_to_same_pixels() {
        let g = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        if !g.exists() {
            eprintln!("skip: game absent");
            return;
        }
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let asset = "/Game/UI/Textures/Common/T_HardwareCursor"; // small inline texture

        // 1. unpack original + record its decoded pixels
        let tmp = std::env::temp_dir().join("gore-tex-repack-rt");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let cooked = tmp.join("G1R/Content/UI/Textures/Common");
        std::fs::create_dir_all(&cooked).unwrap();
        let uasset = unpack_asset(&utoc, &usmap, asset, &cooked).unwrap();
        let orig = crate::decode::parse(
            &std::fs::read(&uasset).unwrap(),
            &std::fs::read(uasset.with_extension("uexp")).unwrap(),
            &std::fs::read(uasset.with_extension("ubulk")).unwrap_or_default(),
            &std::fs::read(&usmap).unwrap(),
        )
        .unwrap();
        let orig_px = crate::decode::to_rgba8(&orig).unwrap();

        // 2. repack the (unchanged) cooked dir -> triplet
        let out = tmp.join("out");
        std::fs::create_dir_all(&out).unwrap();
        // DEFAULT (compress = false): the proven-in-game uncompressed write path.
        let triplet = repack_to_zen(&tmp, "RepackRoundTrip_P", &out, &g, false).unwrap();
        for p in &triplet {
            assert!(
                p.exists() && std::fs::metadata(p).unwrap().len() > 0,
                "triplet member missing/empty: {}",
                p.display()
            );
        }
        eprintln!(
            "triplet sizes: utoc={} ucas={} pak={}",
            std::fs::metadata(&triplet[0]).unwrap().len(),
            std::fs::metadata(&triplet[1]).unwrap().len(),
            std::fs::metadata(&triplet[2]).unwrap().len(),
        );

        // 2b. Re-dump the regenerated TOC and prove the DEFAULT path is the
        //     uncompressed one: container_flags == 8 (Indexed only, no Compressed
        //     bit) and NO block carries a non-zero compression method. Reuses
        //     retoc's real Toc reader. (The compress=true path -- flags==9 +
        //     16-aligned compressed offsets -- is covered by the gated
        //     `upscale_streamed_water_2x_roundtrips_through_zen` test.)
        let (flags, comp_offsets) =
            retoc::iostore_writer::dump_compressed_layout(&triplet[0]).unwrap();
        eprintln!(
            "container_flags={flags} (expect 8); {} compressed blocks",
            comp_offsets.len()
        );
        assert_eq!(flags, 8, "container_flags must be Indexed only (8) for the uncompressed default");
        assert!(
            comp_offsets.is_empty(),
            "uncompressed default must have no compressed blocks (method != 0)"
        );
        eprintln!("OK: container_flags=8 (uncompressed), no compressed blocks");

        // 3. read the asset back out of the freshly-built triplet and decode it.
        //    Point `unpack_asset` at the produced `.utoc`: it opens the parent dir
        //    as a composite store, but the global script objects come from the
        //    game's `global.utoc` -- which our `out` dir lacks. So copy `global.*`
        //    next to our triplet first, giving the composite store the table it
        //    needs to convert zen->legacy on the way back out.
        let game_paks = g.join("G1R/Content/Paks");
        for ext in ["utoc", "ucas", "pak"] {
            let src = game_paks.join(format!("global.{ext}"));
            if src.exists() {
                std::fs::copy(&src, out.join(format!("global.{ext}"))).unwrap();
            }
        }

        let readback_dir = tmp.join("readback");
        let _ = std::fs::remove_dir_all(&readback_dir);
        std::fs::create_dir_all(&readback_dir).unwrap();
        let rb_uasset = unpack_asset(&triplet[0], &usmap, asset, &readback_dir).unwrap();
        let rb = crate::decode::parse(
            &std::fs::read(&rb_uasset).unwrap(),
            &std::fs::read(rb_uasset.with_extension("uexp")).unwrap(),
            &std::fs::read(rb_uasset.with_extension("ubulk")).unwrap_or_default(),
            &std::fs::read(&usmap).unwrap(),
        )
        .unwrap();
        let rb_px = crate::decode::to_rgba8(&rb).unwrap();

        // The essential assertion: same pixels in == same pixels out.
        assert_eq!(orig.width, rb.width, "width changed");
        assert_eq!(orig.height, rb.height, "height changed");
        assert_eq!(orig_px.len(), rb_px.len(), "pixel count changed");
        assert!(
            orig_px == rb_px,
            "decoded pixels differ after repack round-trip"
        );
        eprintln!(
            "OK: {}x{} px identical after repack round-trip",
            orig.width, orig.height
        );
    }
}
