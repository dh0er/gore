//! Shared on-disk layout for the mod manager, under the per-user `gore-tools` data dir
//! (see `gore_loc::paths::shared_data_dir`) so every tool sees the same library/loadout.

use std::path::PathBuf;

/// Root of all manager state: `<gore-tools>/mod-manager`.
pub fn manager_root() -> PathBuf {
    gore_loc::paths::shared_data_dir().join("mod-manager")
}

/// The imported-mod library: one subdir per mod id, each with a
/// [`super::model::META_FILE`] sidecar.
pub fn library_dir() -> PathBuf {
    manager_root().join("library")
}

/// The loadout file (which mods are enabled, in order).
pub fn loadout_path() -> PathBuf {
    manager_root().join("loadout.json")
}
