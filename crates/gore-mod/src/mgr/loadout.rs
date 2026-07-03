//! Loadout state: which library mods are enabled, in mount order (later wins on conflicts).

use serde::{Deserialize, Serialize};
use std::path::Path;

/// The persisted loadout. `entries` is ordered — position IS the mount order.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Loadout {
    #[serde(default = "default_format")]
    pub format: u32,
    #[serde(default)]
    pub entries: Vec<LoadoutEntry>,
}

fn default_format() -> u32 {
    1
}

/// One loadout slot: a library mod id and whether it deploys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoadoutEntry {
    pub id: String,
    pub enabled: bool,
}

/// Read the loadout at `path`. A missing file is an empty (not an error) loadout, so the
/// manager works before anything was ever saved.
pub fn load(path: &Path) -> crate::Result<Loadout> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(Loadout { format: 1, entries: Vec::new() })
        }
        Err(e) => Err(crate::io(&format!("reading loadout {}", path.display()))(e)),
    }
}

/// Persist `loadout` at `path`, creating parent dirs; atomic so a crash mid-save can't
/// truncate the previous loadout.
pub fn save(path: &Path, loadout: &Loadout) -> crate::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(crate::io("creating loadout dir"))?;
    }
    let bytes = serde_json::to_vec_pretty(loadout)?;
    crate::atomic_write(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadout_roundtrips_and_defaults() {
        // `{}` must parse to the current format with no entries (both fields defaulted).
        let empty: Loadout = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.format, 1);
        assert!(empty.entries.is_empty());

        let full = Loadout {
            format: 1,
            entries: vec![
                LoadoutEntry { id: "mod-a".into(), enabled: true },
                LoadoutEntry { id: "mod-b".into(), enabled: false },
            ],
        };
        let json = serde_json::to_string_pretty(&full).unwrap();
        let back: Loadout = serde_json::from_str(&json).unwrap();
        assert_eq!(full, back);
    }

    #[test]
    fn loadout_load_missing_file_gives_default() {
        let dir = tempfile::tempdir().unwrap();
        // Neither the file nor its parent dir exist yet.
        let l = load(&dir.path().join("mod-manager").join("loadout.json")).unwrap();
        assert_eq!(l, Loadout { format: 1, entries: Vec::new() });
    }

    #[test]
    fn loadout_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        // save() must create the missing parent dirs itself.
        let path = dir.path().join("mod-manager").join("loadout.json");
        let l = Loadout {
            format: 1,
            entries: vec![
                LoadoutEntry { id: "mod-a".into(), enabled: true },
                LoadoutEntry { id: "mod-b".into(), enabled: false },
            ],
        };
        save(&path, &l).unwrap();
        assert_eq!(load(&path).unwrap(), l);
    }
}
