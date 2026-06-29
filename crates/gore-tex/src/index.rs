//! Cached texture index: asset_path -> package_id, for instant search + scan-free extract.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use retoc::iostore;
use retoc::script_objects::FPackageObjectIndex;
use retoc::zen::FZenPackageHeader;
use retoc::{Config, EIoChunkType, FIoChunkId};

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
        serde_json::to_vec(self).map_err(|e| crate::error::TexError::Io(
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())))
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
    /// Load the cached index only if it is still current for this game build.
    /// Returns `None` if the cache is absent, unreadable, or its `build_id` does not
    /// match `expected_build_id` (e.g. a game patch changed the .usmap) — so a stale
    /// cache mapping asset paths to outdated package ids is never trusted.
    pub fn load_current(path: &Path, expected_build_id: &str) -> Option<Self> {
        Self::load(path).ok().filter(|i| i.build_id == expected_build_id)
    }
}

/// The build id for a game install, used to invalidate a stale cached index. Keyed on the
/// `.usmap` filename PLUS the IoStore container's identity (`.utoc` length + mtime) — the
/// container is the actual source of the package ids the index maps to, so any game patch that
/// rewrites it invalidates the cache even when the `.usmap` keeps the same name. (`fs::metadata`
/// is a cheap stat — no file read.)
pub fn build_id_for(utoc: &Path, usmap: &Path) -> String {
    let name = usmap.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
    let (len, mtime) = std::fs::metadata(utoc)
        .ok()
        .map(|m| {
            let secs = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (m.len(), secs)
        })
        .unwrap_or((0, 0));
    format!("{name}|utoc:{len}:{mtime}")
}

/// Build the index by scanning the container once (same walk as `list_textures`,
/// additionally capturing each Texture2D package's id). `build_id` identifies the
/// game build (pass the .usmap filename).
pub fn build_index(utoc: &Path, build_id: &str) -> Result<TextureIndex> {
    let store = iostore::open(utoc, Arc::new(Config::default()))?;
    let texture2d = FPackageObjectIndex::create_script_import("/Script/Engine.Texture2D");
    let cv = store
        .container_file_version()
        .ok_or_else(|| anyhow::anyhow!("container has no TOC version"))?;
    let hv = store
        .container_header_version()
        .ok_or_else(|| anyhow::anyhow!("container has no header version"))?;

    let mut entries = BTreeMap::new();

    // Silence the default panic hook for the duration of the loop so the panics we
    // intentionally catch below (one per malformed package) don't spam stderr.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    for pkg in store.packages() {
        let pkg_id = pkg.id();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let cid = FIoChunkId::from_package_id(pkg_id, 0, EIoChunkType::ExportBundleData);
            let data = store.read(cid).ok()?;
            let header = FZenPackageHeader::deserialize(
                &mut Cursor::new(&data),
                store.package_store_entry(pkg_id),
                cv,
                hv,
                None,
            )
            .ok()?;
            if !header.export_map.iter().any(|e| e.class_index == texture2d) {
                return None;
            }
            Some(header.package_name())
        }));
        if let Ok(Some(path)) = result {
            entries.insert(path, pkg_id.0);
        }
    }

    std::panic::set_hook(prev_hook);

    Ok(TextureIndex {
        build_id: build_id.to_string(),
        entries,
    })
}

/// Extract a texture to RGBA by package id (fast: no scan). Returns (TexInfo, rgba u32 px).
pub fn extract_by_package_id(
    utoc: &Path, usmap: &Path, package_id: u64, leaf: &str,
) -> Result<(crate::decode::TexInfo, Vec<u32>)> {
    // Unique per-call temp dir so overlapping extracts don't clobber each other's cooked files.
    let tmp = crate::paths::unique_temp_dir("gore-tex-idx-extract")?;
    // Run the fallible work in a closure so the temp dir is removed on EVERY path (incl. an
    // unpack/parse/decode error), not just success.
    let result = (|| -> Result<(crate::decode::TexInfo, Vec<u32>)> {
        let uasset = crate::container::unpack_asset_by_id(utoc, usmap, package_id, leaf, &tmp)?;
        let uexp = uasset.with_extension("uexp");
        let ubulk = uasset.with_extension("ubulk");
        let info = crate::decode::parse(
            &std::fs::read(&uasset)?, &std::fs::read(&uexp)?,
            &crate::paths::read_optional(&ubulk)?, &std::fs::read(usmap)?)?;
        let px = crate::decode::to_rgba8(&info)?;
        Ok((info, px))
    })();
    let _ = std::fs::remove_dir_all(&tmp); // transient cooked files; pixels are in memory now
    result
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

    #[test]
    fn load_current_rejects_stale_build_id() {
        let dir = std::env::temp_dir().join("gore-tex-idx-stale");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("texture_index.json");
        let idx = TextureIndex { build_id: "G1R-5.4.3-old.usmap".into(), entries: BTreeMap::new() };
        idx.save(&path).unwrap();
        // Matching build id -> Some; mismatched (game patched) -> None; absent -> None.
        assert!(TextureIndex::load_current(&path, "G1R-5.4.3-old.usmap").is_some());
        assert!(TextureIndex::load_current(&path, "G1R-5.4.4-new.usmap").is_none());
        assert!(TextureIndex::load_current(&dir.join("missing.json"), "x").is_none());
    }

    fn game_dir() -> Option<std::path::PathBuf> {
        let p = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        p.exists().then_some(p)
    }

    #[test]
    #[ignore = "slow: full container scan"]
    fn builds_index_from_real_container() {
        let Some(g) = game_dir() else {
            eprintln!("skip: game not installed");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();
        let idx = build_index(&utoc, "test-build").unwrap();
        assert!(
            idx.entries.len() > 10000,
            "expected ~13k textures, got {}",
            idx.entries.len()
        );
        let pid = idx.entries.get("/Game/UI/Textures/Common/T_HardwareCursor");
        assert!(pid.is_some() && *pid.unwrap() != 0);
    }

    #[test]
    #[ignore = "slow: unpack from real container"]
    fn id_extract_matches_path_extract() {
        let Some(g) = game_dir() else { eprintln!("skip"); return; };
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let asset = "/Game/UI/Textures/Common/T_HardwareCursor";
        let tmp = std::env::temp_dir().join("gore-tex-ref");
        let _ = std::fs::remove_dir_all(&tmp); std::fs::create_dir_all(&tmp).unwrap();
        let ua = crate::container::unpack_asset(&utoc, &usmap, asset, &tmp).unwrap();
        let ref_info = crate::decode::parse(
            &std::fs::read(&ua).unwrap(), &std::fs::read(ua.with_extension("uexp")).unwrap(),
            &std::fs::read(ua.with_extension("ubulk")).unwrap_or_default(),
            &std::fs::read(&usmap).unwrap()).unwrap();
        let ref_px = crate::decode::to_rgba8(&ref_info).unwrap();
        let idx = build_index(&utoc, "t").unwrap();
        let pid = *idx.entries.get(asset).unwrap();
        let (info, px) = extract_by_package_id(&utoc, &usmap, pid, "T_HardwareCursor").unwrap();
        assert_eq!(info.width, ref_info.width);
        assert_eq!(info.format, ref_info.format);
        assert_eq!(px, ref_px);
    }
}
