//! Loadout state: which library mods are enabled, in mount order (later wins on conflicts).

use serde::{Deserialize, Serialize};
use std::path::Path;

/// The persisted loadout. `entries` is ordered — position IS the mount order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Loadout {
    #[serde(default = "default_format")]
    pub format: u32,
    #[serde(default)]
    pub entries: Vec<LoadoutEntry>,
}

fn default_format() -> u32 {
    1
}

/// A fresh loadout is the current on-disk format with no entries. Hand-written rather than
/// `#[derive(Default)]` so `format` defaults to the real schema version (1), not `u32`'s 0 —
/// otherwise `Loadout::default()` would look like a pre-format-1 file to `load`'s version check.
impl Default for Loadout {
    fn default() -> Self {
        Loadout {
            format: 1,
            entries: Vec::new(),
        }
    }
}

impl Loadout {
    /// Validate every persisted id, including disabled slots. Disabled entries are still saved and
    /// may later be enabled, so they must never carry an absolute/traversing library path.
    pub(crate) fn validate(&self) -> crate::Result<()> {
        for (index, entry) in self.entries.iter().enumerate() {
            super::model::validate_library_id(&entry.id).map_err(|error| {
                crate::ModError::Other(format!(
                    "invalid loadout entry {index} ({:?}): {error}",
                    entry.id
                ))
            })?;
        }
        Ok(())
    }
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
        Ok(bytes) => {
            let parsed: Loadout = serde_json::from_slice(&bytes)?;
            // A newer tool may have written a schema this build can't interpret; refuse rather
            // than silently mis-reading (serde tolerates unknown fields, so a bumped format is
            // the only signal that fields we can't see were dropped).
            if parsed.format > 1 {
                return Err(crate::ModError::Other(format!(
                    "loadout format {} is newer than this tool supports",
                    parsed.format
                )));
            }
            parsed.validate()?;
            Ok(parsed)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Loadout::default()),
        Err(e) => Err(crate::io(&format!("reading loadout {}", path.display()))(e)),
    }
}

/// Persist `loadout` at `path`, creating parent dirs; atomic so a crash mid-save can't
/// truncate the previous loadout.
pub fn save(path: &Path, loadout: &Loadout) -> crate::Result<()> {
    loadout.validate()?;
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
        // `Loadout::default()` must carry the real schema version, not `u32`'s 0.
        assert_eq!(Loadout::default().format, 1);
        assert!(Loadout::default().entries.is_empty());
        // `{}` must parse to the current format with no entries (both fields defaulted).
        let empty: Loadout = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.format, 1);
        assert!(empty.entries.is_empty());

        let full = Loadout {
            format: 1,
            entries: vec![
                LoadoutEntry {
                    id: "mod-a".into(),
                    enabled: true,
                },
                LoadoutEntry {
                    id: "mod-b".into(),
                    enabled: false,
                },
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
        assert_eq!(
            l,
            Loadout {
                format: 1,
                entries: Vec::new()
            }
        );
    }

    #[test]
    fn loadout_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        // save() must create the missing parent dirs itself.
        let path = dir.path().join("mod-manager").join("loadout.json");
        let l = Loadout {
            format: 1,
            entries: vec![
                LoadoutEntry {
                    id: "mod-a".into(),
                    enabled: true,
                },
                LoadoutEntry {
                    id: "mod-b".into(),
                    enabled: false,
                },
            ],
        };
        save(&path, &l).unwrap();
        assert_eq!(load(&path).unwrap(), l);
    }

    #[test]
    fn loadout_load_rejects_newer_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loadout.json");
        std::fs::write(&path, br#"{"format": 2, "entries": []}"#).unwrap();
        let err = load(&path).unwrap_err();
        assert!(
            err.to_string().contains("newer than this tool supports"),
            "got: {err}"
        );
    }

    #[test]
    fn loadout_load_rejects_traversal_and_absolute_ids() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loadout.json");
        for id in ["../outside", "/absolute", r"C:\absolute"] {
            std::fs::write(
                &path,
                serde_json::json!({
                    "format": 1,
                    "entries": [{ "id": id, "enabled": false }]
                })
                .to_string(),
            )
            .unwrap();
            let error = load(&path).unwrap_err().to_string();
            assert!(error.contains("invalid loadout entry"), "{id:?}: {error}");
        }
    }

    #[test]
    fn loadout_save_refuses_unsafe_id_before_creating_parent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new").join("loadout.json");
        let loadout = Loadout {
            format: 1,
            entries: vec![LoadoutEntry {
                id: "../outside".into(),
                enabled: true,
            }],
        };
        assert!(save(&path, &loadout).is_err());
        assert!(!path.parent().unwrap().exists());
    }
}
