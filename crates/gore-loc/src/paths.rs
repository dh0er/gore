//! Shared on-disk locations for the gore tool suite.
//!
//! All three tools (gore-cli, gore-save, gore-mod) read and write the extracted
//! localization catalog from ONE shared per-user directory named `gore`,
//! so a single extraction serves every tool. The directory lives under the
//! platform's local-app-data root and is never part of the repo.

use std::path::PathBuf;

/// The shared `gore` data directory:
/// - Windows: `%LOCALAPPDATA%\gore` (falls back to `%APPDATA%`)
/// - macOS:   `~/Library/Application Support/gore`
/// - Linux:   `$XDG_DATA_HOME/gore` or `~/.local/share/gore`
pub fn shared_data_dir() -> PathBuf {
    let base = local_app_data_root();
    base.join("gore")
}

/// The shared localized-text catalog (`{id:{language:value}}`).
pub fn loc_catalog_path() -> PathBuf {
    shared_data_dir().join("loc_catalog.json")
}

/// Sidecar metadata about the last extraction (source path, sizes, timestamp).
pub fn loc_meta_path() -> PathBuf {
    shared_data_dir().join("loc_meta.json")
}

/// The shared, extensible tool configuration (`{ "game_path": ... }`).
pub fn config_path() -> PathBuf {
    shared_data_dir().join("config.json")
}

#[cfg(windows)]
fn local_app_data_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(target_os = "macos")]
fn local_app_data_root() -> PathBuf {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn local_app_data_root() -> PathBuf {
    if let Some(x) = std::env::var_os("XDG_DATA_HOME") {
        if !x.is_empty() {
            return PathBuf::from(x);
        }
    }
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".local").join("share"))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_meta_config_live_in_the_shared_gore_dir() {
        let dir = shared_data_dir();
        assert!(dir.ends_with("gore"));
        assert_eq!(loc_catalog_path().parent().unwrap(), dir);
        assert_eq!(loc_meta_path().parent().unwrap(), dir);
        assert_eq!(config_path().parent().unwrap(), dir);
        assert_eq!(loc_catalog_path().file_name().unwrap(), "loc_catalog.json");
        assert_eq!(config_path().file_name().unwrap(), "config.json");
    }
}
