//! Shared, extensible per-user configuration for the gore tools.
//!
//! Stored as JSON at `<shared>/config.json` (see [`crate::paths::config_path`])
//! so the CLI and every app read the same file. Currently holds the game
//! install path; the struct is designed so new keys are additive.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::discover;
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

    /// Unknown/future keys written by a newer tool version. Preserved verbatim
    /// on save so an older tool never clobbers a newer one's settings.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
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

/// Error resolving the game path.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no game path set — run 'gore config set game-path <path>' or pass --game")]
    Unresolved,
}

/// Resolve the game install ROOT (the folder containing `G1R/`).
///
/// Precedence: explicit CLI arg > configured `game_path` > Steam auto-detect.
/// The winning path is normalized (an `.exe` or a descendant walks up to the
/// `G1R/` parent). Returns [`ConfigError::Unresolved`] when nothing resolves.
pub fn game_root(explicit: Option<PathBuf>) -> Result<PathBuf, ConfigError> {
    // Steam auto-detect is deferred (it probes the registry + filesystem): it
    // runs only when neither an explicit arg nor a configured path resolves.
    resolve_root(explicit, configured_game_path(&load()), || {
        if autodetect_disabled() {
            None
        } else {
            discover::find_game_root()
        }
    })
    .map(|p| normalize_root(&p))
    .ok_or(ConfigError::Unresolved)
}

/// The configured `game_path` as an `Option`, treating an empty/whitespace-only
/// string as **unset** — parity with the Dart apps' `SharedConfig.gamePath()`,
/// so the same `config.json` never leaves the CLI stuck on a blank value (and
/// blocking auto-detect) while the GUI behaves as if nothing is configured.
fn configured_game_path(cfg: &Config) -> Option<PathBuf> {
    cfg.game_path
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
}

/// Test / power-user seam: when `GORE_DISABLE_GAME_AUTODETECT` is set to a
/// non-empty value, skip Steam auto-detection so resolution relies solely on an
/// explicit arg or the configured `game_path`. Public so other Steam-scanning
/// entry points (e.g. `loc extract`'s cache fallback) honor the same switch.
pub fn autodetect_disabled() -> bool {
    std::env::var_os("GORE_DISABLE_GAME_AUTODETECT")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

/// Precedence selection: explicit > configured > detected. `detect` is a closure
/// so Steam auto-detection is invoked only when neither an explicit arg nor a
/// configured path is present (it does a registry + filesystem probe).
fn resolve_root(
    explicit: Option<PathBuf>,
    configured: Option<PathBuf>,
    detect: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    explicit.or(configured).or_else(detect)
}

/// Normalize any game path to the install root: the nearest ancestor (including
/// `p` itself) that holds a `G1R/` child. Best-effort — returns `p` unchanged
/// when no such ancestor exists (unusual layout / path not on disk).
fn normalize_root(p: &Path) -> PathBuf {
    for anc in p.ancestors() {
        if anc.join("G1R").is_dir() {
            return anc.to_path_buf();
        }
    }
    p.to_path_buf()
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
        let cfg = Config {
            game_path: Some("D:/Games/G1R".to_string()),
            ..Default::default()
        };
        save_to(&path, &cfg).unwrap();
        let read = load_from(&path);
        assert_eq!(read.game_path.as_deref(), Some("D:/Games/G1R"));
    }

    #[test]
    fn save_preserves_unknown_keys_and_creates_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        // Parent "nested/" does NOT exist -> save_to must create it.
        let path = dir.path().join("nested").join("config.json");

        // Seed a config file that carries an unmodeled key.
        let mut seed = Config::default();
        seed.extra
            .insert("future_key".to_string(), serde_json::json!(42));
        seed.game_path = Some("x".to_string());
        save_to(&path, &seed).unwrap(); // also proves parent-dir creation

        let mut cfg = load_from(&path);
        cfg.game_path = Some("y".to_string());
        save_to(&path, &cfg).unwrap();

        let reread = load_from(&path);
        assert_eq!(reread.game_path.as_deref(), Some("y"));
        assert_eq!(reread.extra.get("future_key"), Some(&serde_json::json!(42)));
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

    #[test]
    fn resolve_precedence_explicit_wins() {
        let got = resolve_root(
            Some(PathBuf::from("/explicit")),
            Some(PathBuf::from("/configured")),
            || Some(PathBuf::from("/detected")),
        );
        assert_eq!(got, Some(PathBuf::from("/explicit")));
    }

    #[test]
    fn resolve_precedence_config_over_detected() {
        let got = resolve_root(None, Some(PathBuf::from("/configured")), || {
            Some(PathBuf::from("/detected"))
        });
        assert_eq!(got, Some(PathBuf::from("/configured")));
    }

    #[test]
    fn resolve_precedence_detected_last() {
        let got = resolve_root(None, None, || Some(PathBuf::from("/detected")));
        assert_eq!(got, Some(PathBuf::from("/detected")));
    }

    #[test]
    fn resolve_none_when_all_absent() {
        assert_eq!(resolve_root(None, None, || None), None);
    }

    #[test]
    fn resolve_does_not_detect_when_a_path_is_present() {
        // Steam auto-detect must be deferred: the closure is not called when an
        // explicit (or configured) path already resolves.
        let mut detected = false;
        let got = resolve_root(Some(PathBuf::from("/x")), None, || {
            detected = true;
            Some(PathBuf::from("/steam"))
        });
        assert_eq!(got, Some(PathBuf::from("/x")));
        assert!(!detected, "detect closure ran despite an explicit path");
    }

    #[test]
    fn empty_configured_game_path_is_unset() {
        // Parity with Dart's SharedConfig.gamePath(): "" / whitespace = unset,
        // so it never wins over Steam auto-detect in resolve_root.
        let mut cfg = Config::default();
        cfg.game_path = Some(String::new());
        assert_eq!(configured_game_path(&cfg), None);
        cfg.game_path = Some("   ".to_string());
        assert_eq!(configured_game_path(&cfg), None);
        cfg.game_path = Some("D:/Games/G1R".to_string());
        assert_eq!(
            configured_game_path(&cfg),
            Some(PathBuf::from("D:/Games/G1R"))
        );
    }

    #[test]
    fn normalize_walks_exe_up_to_g1r_parent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("G1R")).unwrap();
        let exe = root
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("G1R-Win64-Shipping.exe");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, b"x").unwrap();
        assert_eq!(normalize_root(&exe), root.to_path_buf());
    }

    #[test]
    fn normalize_returns_root_unchanged_when_it_holds_g1r() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("G1R")).unwrap();
        assert_eq!(normalize_root(root), root.to_path_buf());
    }

    #[test]
    fn normalize_best_effort_when_no_g1r() {
        assert_eq!(
            normalize_root(Path::new("/no/such/place")),
            PathBuf::from("/no/such/place")
        );
    }
}
