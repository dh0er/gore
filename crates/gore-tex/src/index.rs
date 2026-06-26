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
