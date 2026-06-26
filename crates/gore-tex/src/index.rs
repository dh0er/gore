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
}
