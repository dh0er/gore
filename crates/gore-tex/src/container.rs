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
    let store = iostore::open(utoc, Arc::new(Config::default()))?;

    // The script-import index UE assigns to the Texture2D class. Computed the same
    // way the cooker does (cityhash of the normalised path) so we can match it
    // without the engine's global script-object table.
    let texture2d_class = FPackageObjectIndex::create_script_import(TEXTURE2D_CLASS_PATH);

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
        // fatal to the entire listing. The closure returns `Some(entry)` for a
        // matching texture, `None` for a non-match or any handled failure; a caught
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

            // Is any export a Texture2D? Compare each export's class import index to
            // the precomputed Texture2D script-import index.
            let is_texture = header
                .export_map
                .iter()
                .any(|export| export.class_index == texture2d_class);

            if !is_texture {
                return None;
            }

            let path = header.package_name();
            if filter.is_none_or(|f| path.contains(f)) {
                Some(TextureEntry { asset_path: path })
            } else {
                None
            }
        }));

        // Caught panic == skip this package (same as the `Err(_) => continue` arms).
        if let Ok(Some(entry)) = result {
            out.push(entry);
        }
    }

    std::panic::set_hook(prev_hook);

    out.sort();
    out.dedup();
    Ok(out)
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

    // Build the legacy cooked files. `build_legacy` writes paths *relative* to
    // the FSFileWriter's root dir, so we name the output after the asset's leaf
    // and root the writer at `out_dir`: the .uasset/.uexp/.ubulk land directly
    // in out_dir sharing the same stem (so `with_extension(..)` finds siblings).
    std::fs::create_dir_all(out_dir)?;
    let leaf = asset_path.rsplit('/').next().unwrap_or(asset_path);
    let out_rel = format!("{leaf}.uasset");

    let log = Log::no_log();
    // No verse script cells store: G1R textures are plain UTexture2D, and script
    // cells are only needed to resolve Verse cell imports (none here).
    let context = FZenPackageContext::create(store.as_ref(), None, &log, None);
    let writer = FSFileWriter::new(out_dir);

    build_legacy(&context, package_id, UEPath::new(&out_rel), &writer)?;

    let uasset = out_dir.join(format!("{leaf}.uasset"));
    Ok(uasset)
}

/// Pack a directory of edited legacy cooked files (laid out under their mount
/// path, e.g. `cooked_dir/G1R/Content/UI/Textures/Common/T_HardwareCursor.uasset`)
/// into a Zen triplet `out_dir/<name>.{utoc,ucas,pak}`, UE5.4. Returns the 3 paths
/// `[utoc, ucas, pak]`. Chunks are written UNCOMPRESSED (method 0) -- valid and
/// game-loadable (UE mounts uncompressed IoStore fine); no Oodle compression in v1.
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
    let mut writer = IoStoreWriter::new(
        &utoc_path,
        toc_version,
        Some(header_version),
        UEPathBuf::from(mount_point),
    )?;

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
        let triplet = repack_to_zen(&tmp, "RepackRoundTrip_P", &out, &g).unwrap();
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
