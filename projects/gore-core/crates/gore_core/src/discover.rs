//! Locate the Gothic 1 Remake install and its localization `.lcache`.
//!
//! Auto-detects via Steam (registry → `libraryfolders.vdf` → the game folder),
//! and also resolves an explicit hint the user picked (the game root, the
//! `Story/Cache` directory, or the `.lcache` file itself). Returns `None` when
//! nothing is found, leaving the caller to fall back to a manual picker.

use std::path::{Path, PathBuf};

/// Steam appid for Gothic 1 Remake (folder `steamapps/common/Gothic 1 Remake`).
const GAME_FOLDER: &str = "Gothic 1 Remake";
/// Relative path from the game install root to the localization cache directory.
const CACHE_SUBDIR: &[&str] = &["G1R", "Story", "Cache"];

/// Resolve a user-provided hint to an `.lcache` path. The hint may be the
/// `.lcache` file, the `Story/Cache` directory, or any ancestor up to the game
/// root / a Steam library / `steamapps/common`.
pub fn lcache_from_hint(hint: &Path) -> Option<PathBuf> {
    if hint.is_file() && is_lcache(hint) {
        return Some(hint.to_path_buf());
    }
    if hint.is_dir() {
        // Direct: hint is a Cache dir (or any dir) holding the file.
        if let Some(f) = lcache_in_dir(hint) {
            return Some(f);
        }
        // hint is the game root -> root/G1R/Story/Cache
        if let Some(f) = lcache_in_dir(&join(hint, CACHE_SUBDIR)) {
            return Some(f);
        }
        // hint is a library root or steamapps/common -> scan for the game folder
        for candidate in [
            hint.join(GAME_FOLDER),
            hint.join("steamapps").join("common").join(GAME_FOLDER),
            hint.join("common").join(GAME_FOLDER),
        ] {
            if let Some(f) = lcache_in_dir(&join(&candidate, CACHE_SUBDIR)) {
                return Some(f);
            }
        }
    }
    None
}

/// Full auto-detect through Steam. `None` if Steam or the game isn't found.
pub fn find_lcache() -> Option<PathBuf> {
    for lib in steam_libraries() {
        let cache = join(
            &lib.join("steamapps").join("common").join(GAME_FOLDER),
            CACHE_SUBDIR,
        );
        if let Some(f) = lcache_in_dir(&cache) {
            return Some(f);
        }
    }
    None
}

fn is_lcache(p: &Path) -> bool {
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.starts_with("AlkimiaLocalization") && name.ends_with(".lcache")
}

fn lcache_in_dir(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_lcache(p))
        .collect();
    found.sort();
    found.into_iter().next()
}

fn join(base: &Path, parts: &[&str]) -> PathBuf {
    let mut p = base.to_path_buf();
    for part in parts {
        p.push(part);
    }
    p
}

/// Steam library roots (each contains `steamapps/common/...`). Empty if Steam
/// can't be located.
pub fn steam_libraries() -> Vec<PathBuf> {
    let Some(steam) = steam_root() else {
        return Vec::new();
    };
    let mut libs = vec![steam.clone()];
    let vdf = steam.join("steamapps").join("libraryfolders.vdf");
    if let Ok(text) = std::fs::read_to_string(&vdf) {
        for path in vdf_paths(&text) {
            let p = PathBuf::from(path);
            if !libs.contains(&p) {
                libs.push(p);
            }
        }
    }
    libs
}

/// Extract every `"path"  "<value>"` value from a `libraryfolders.vdf`,
/// un-escaping the doubled backslashes Steam writes.
fn vdf_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let rest = match line.strip_prefix("\"path\"") {
            Some(r) => r.trim(),
            None => continue,
        };
        // rest looks like: "X:\\SteamLibrary"
        if let Some(start) = rest.find('"') {
            if let Some(end_rel) = rest[start + 1..].find('"') {
                let raw = &rest[start + 1..start + 1 + end_rel];
                out.push(raw.replace("\\\\", "\\"));
            }
        }
    }
    out
}

#[cfg(windows)]
fn steam_root() -> Option<PathBuf> {
    // HKCU\Software\Valve\Steam\SteamPath is set by every Steam install.
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey("Software\\Valve\\Steam").ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    let p = PathBuf::from(path);
    if p.is_dir() {
        Some(p)
    } else {
        None
    }
}

#[cfg(not(windows))]
fn steam_root() -> Option<PathBuf> {
    // Common Steam locations on macOS / Linux.
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    for rel in [
        ".steam/steam",
        ".local/share/Steam",
        "Library/Application Support/Steam",
    ] {
        let p = home.join(rel);
        if p.is_dir() {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_paths_from_vdf() {
        let vdf = r#"
"libraryfolders"
{
    "0"
    {
        "path"		"C:\\Program Files (x86)\\Steam"
    }
    "1"
    {
        "path"		"D:\\SteamLibrary"
    }
}
"#;
        let paths = vdf_paths(vdf);
        assert_eq!(paths, vec![r"C:\Program Files (x86)\Steam", r"D:\SteamLibrary"]);
    }

    #[test]
    fn is_lcache_matches_only_the_localization_cache() {
        assert!(is_lcache(Path::new("AlkimiaLocalization_00000000.lcache")));
        assert!(is_lcache(Path::new("/x/AlkimiaLocalization_00000000.lcache")));
        assert!(!is_lcache(Path::new("Other.lcache")));
        assert!(!is_lcache(Path::new("AlkimiaLocalization_00000000.txt")));
    }

    #[test]
    fn hint_resolves_the_lcache_file_directly(/* and rejects non-lcache */) {
        // A non-existent path can't be a file; resolution returns None rather
        // than panicking.
        assert_eq!(lcache_from_hint(Path::new("/no/such/file.lcache")), None);
    }
}
