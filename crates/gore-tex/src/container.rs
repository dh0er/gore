//! List texture assets in a cooked UE5 IoStore container.
//!
//! The container holds *cooked* zen packages; an asset's class (e.g. `Texture2D`)
//! is not stored in chunk metadata but inside each package's zen header. For every
//! package we parse the zen header, walk its export map, and resolve each export's
//! `class_index` (a `FPackageObjectIndex`) against the container's global script
//! objects to recover the class *name*. A package is reported as a texture if any
//! of its exports has class `Texture2D` (the cooked Texture2D class, which also
//! covers `LightMapTexture2D`/`ShadowMapTexture2D` only if they report that exact
//! name -- they do not; they have their own classes, so this is an exact match on
//! `Texture2D`).
//!
//! This is the "per-package class resolution" route from the task note. The
//! global script-object *table* lives in the engine's `global.utoc`, not in the
//! game's `G1R-Windows.utoc`, so `load_script_objects()` is unavailable here.
//! Instead we reproduce the exact import-hash UE assigns to a script object:
//! `FPackageObjectIndex::create_script_import("/Script/Engine.Texture2D")`
//! (cityhash64 of the lower-cased, slash-normalised path) and compare each
//! export's `class_index` against it. This is an exact, table-free match.

use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use retoc::iostore;
use retoc::script_objects::FPackageObjectIndex;
use retoc::zen::FZenPackageHeader;
use retoc::{Config, EIoChunkType, FIoChunkId};

use crate::error::Result;

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

    for pkg in store.packages() {
        let pkg_id = pkg.id();
        let chunk_id =
            FIoChunkId::from_package_id(pkg_id, 0, EIoChunkType::ExportBundleData);

        // Some package entries may not have a readable export-bundle chunk; skip them
        // rather than failing the whole listing.
        let data = match store.read(chunk_id) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let header = match FZenPackageHeader::deserialize(
            &mut Cursor::new(&data),
            store.package_store_entry(pkg_id),
            container_version,
            header_version,
            None,
        ) {
            Ok(h) => h,
            Err(_) => continue,
        };

        // Is any export a Texture2D? Compare each export's class import index to the
        // precomputed Texture2D script-import index.
        let is_texture = header
            .export_map
            .iter()
            .any(|export| export.class_index == texture2d_class);

        if is_texture {
            let path = header.package_name();
            if filter.is_none_or(|f| path.contains(f)) {
                out.push(TextureEntry { asset_path: path });
            }
        }
    }

    out.sort();
    out.dedup();
    Ok(out)
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
}
