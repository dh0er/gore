//! Auto-resolve the game container + .usmap from an install dir.

use std::path::{Path, PathBuf};
use crate::error::{Result, TexError};

/// Given a game install dir, return the main IoStore container `.utoc`.
pub fn main_container(game_dir: &Path) -> Result<PathBuf> {
    let p = game_dir.join("G1R/Content/Paks/G1R-Windows.utoc");
    if p.exists() { Ok(p) } else { Err(TexError::ContainerNotFound(p)) }
}

/// Given a game install dir, return the `.usmap` mappings file. When several exist, the
/// pick is DETERMINISTIC (alphabetically first) rather than `read_dir` order — so `build_id`,
/// cached-index reuse, and mapping-dependent work stay stable across runs/filesystems.
pub fn usmap(game_dir: &Path) -> Result<PathBuf> {
    let dir = game_dir.join("G1R/Binaries/Win64/ue4ss");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "usmap"))
        .collect();
    found.sort();
    found.into_iter().next().ok_or_else(|| TexError::UsmapNotFound(dir))
}

/// The shared gore-tools cache path for the texture index (next to loc_catalog.json).
pub fn texture_index_path() -> PathBuf {
    gore_loc::paths::shared_data_dir().join("texture_index.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_container_missing_dir_errors() {
        let err = main_container(Path::new("/no/such/game")).unwrap_err();
        assert!(matches!(err, TexError::ContainerNotFound(_)));
    }

    #[test]
    fn usmap_pick_is_deterministic() {
        let base = std::env::temp_dir().join("gore-tex-usmap-pick");
        let dir = base.join("G1R/Binaries/Win64/ue4ss");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("b_second.usmap"), b"x").unwrap();
        std::fs::write(dir.join("a_first.usmap"), b"x").unwrap();
        std::fs::write(dir.join("notmap.txt"), b"x").unwrap();
        let got = usmap(&base).unwrap();
        assert_eq!(got.file_name().unwrap().to_str().unwrap(), "a_first.usmap");
    }

    #[test]
    fn usmap_missing_dir_errors() {
        let err = usmap(Path::new("/no/such/game")).unwrap_err();
        // read_dir on a missing dir yields an io error -> mapped via From.
        assert!(matches!(err, TexError::Io(_)));
    }
}
