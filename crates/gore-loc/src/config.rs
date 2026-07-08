//! Shared, extensible per-user configuration for the gore tools.
//!
//! Stored as JSON at `<shared>/config.json` (see [`crate::paths::config_path`])
//! so the CLI and every app read the same file. Currently holds the game
//! install path; the struct is designed so new keys are additive.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::paths;

/// Persisted per-user settings. All fields optional and additive so old files
/// keep loading as new keys appear. Serialized as snake_case JSON.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Game install path exactly as the user set it (an install root or the
    /// `.exe`). Consumers normalize to the root via [`game_root`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_path: Option<String>,
}

/// Absolute path of the shared `config.json`.
pub fn config_path() -> PathBuf {
    paths::config_path()
}

/// Load the shared config. A missing, unreadable, or corrupt file yields
/// [`Config::default`] — config is best-effort, never a hard error on read.
pub fn load() -> Config {
    load_from(&config_path())
}

/// Persist the shared config (creates the shared dir; atomic write).
pub fn save(cfg: &Config) -> std::io::Result<()> {
    save_to(&config_path(), cfg)
}

/// [`load`] against an explicit path (test seam).
pub fn load_from(path: &Path) -> Config {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

/// [`save`] against an explicit path (test seam).
pub fn save_to(path: &Path, cfg: &Config) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(cfg)?;
    crate::loc_store::write_atomic(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_file_loads_default() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = load_from(&dir.path().join("config.json"));
        assert_eq!(cfg.game_path, None);
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cfg = Config { game_path: Some("D:/Games/G1R".to_string()) };
        save_to(&path, &cfg).unwrap();
        let read = load_from(&path);
        assert_eq!(read.game_path.as_deref(), Some("D:/Games/G1R"));
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, br#"{"game_path":"x","future_key":42}"#).unwrap();
        let cfg = load_from(&path);
        assert_eq!(cfg.game_path.as_deref(), Some("x"));
    }

    #[test]
    fn corrupt_file_loads_default_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, b"not json at all").unwrap();
        assert_eq!(load_from(&path).game_path, None);
    }
}
