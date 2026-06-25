//! Auto-resolve the game container + .usmap from an install dir.

use std::path::{Path, PathBuf};
use crate::error::{Result, TexError};

/// Given a game install dir, return the main IoStore container `.utoc`.
pub fn main_container(game_dir: &Path) -> Result<PathBuf> {
    let p = game_dir.join("G1R/Content/Paks/G1R-Windows.utoc");
    if p.exists() { Ok(p) } else { Err(TexError::ContainerNotFound(p)) }
}

/// Given a game install dir, return the `.usmap` mappings file (first match).
pub fn usmap(game_dir: &Path) -> Result<PathBuf> {
    let dir = game_dir.join("G1R/Binaries/Win64/ue4ss");
    let found = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "usmap"));
    found.ok_or_else(|| TexError::UsmapNotFound(dir))
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
    fn usmap_missing_dir_errors() {
        let err = usmap(Path::new("/no/such/game")).unwrap_err();
        // read_dir on a missing dir yields an io error -> mapped via From.
        assert!(matches!(err, TexError::Io(_)));
    }
}
