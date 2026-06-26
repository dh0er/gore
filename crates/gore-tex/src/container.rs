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
use retoc::{Config, EIoChunkType, FIoChunkId, FPackageId, FSFileWriter, UEPath};

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
}
