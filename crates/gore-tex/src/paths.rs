//! Auto-resolve the game container + .usmap from an install dir.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use crate::error::{Result, TexError};

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Create and return a FRESH, UNIQUE temp subdir for one unpack/extract call, so concurrent
/// callers (e.g. overlapping previews, or a preview during a deploy) don't clobber each other's
/// cooked `.uasset`/`.uexp`/`.ubulk` files in a shared fixed directory. Keyed by process id + an
/// atomic counter, so it's unique within and across processes.
pub fn unique_temp_dir(prefix: &str) -> std::io::Result<PathBuf> {
    let n = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("{prefix}-{}-{n}", std::process::id()));
    // The pid + per-process counter can collide with a directory a PRIOR process
    // (same pid, since the counter restarts at 0) left behind. `create_dir_all`
    // succeeds on an existing path, so it would reopen that stale directory and
    // leave unrelated siblings in place (e.g. an old `.ubulk` next to freshly
    // unpacked `.uasset`/`.uexp`, corrupting an inline-texture parse). Remove any
    // leftover first so the returned directory is guaranteed empty.
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p)?;
    Ok(p)
}

/// A FRESH, UNIQUE temp file path (not created) for one output, e.g. a preview
/// PNG. Keyed by pid + the same atomic counter as [`unique_temp_dir`], so two
/// extractions never collide on the same path — the caller owns and deletes
/// exactly the file it was given.
pub fn unique_temp_file(prefix: &str, ext: &str) -> PathBuf {
    let n = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefix}-{}-{n}.{ext}", std::process::id()))
}

/// Read an OPTIONAL cooked sidecar (e.g. a texture's `.ubulk`, which inline-mip
/// textures legitimately lack). A missing file yields empty bytes; any OTHER I/O
/// error (permissions, partial read) is propagated rather than silently masked as
/// "no bulk data", which would let a streamed texture decode/cook with the wrong
/// (empty) bulk and produce a misleading result.
pub fn read_optional(path: &Path) -> std::io::Result<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

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

/// Map a UE virtual asset path to its physical content-relative path inside a
/// cooked container / mod tree, per UE mount rules: `/Game/X` → `G1R/Content/X`
/// (the project mount) and `/Engine/X` → `Engine/Content/X`. Returns `None` for
/// any other mount root (e.g. a plugin's `/PluginName/...`), which this tool
/// can't place — callers must reject those rather than mis-mount them under the
/// project content (where they would never override the intended asset).
pub fn content_mount_rel(asset: &str) -> Option<String> {
    if let Some(rest) = asset.strip_prefix("/Game/") {
        Some(format!("G1R/Content/{rest}"))
    } else if let Some(rest) = asset.strip_prefix("/Engine/") {
        Some(format!("Engine/Content/{rest}"))
    } else {
        None
    }
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
    fn content_mount_rel_maps_known_roots() {
        assert_eq!(
            content_mount_rel("/Game/UI/Textures/T_X").as_deref(),
            Some("G1R/Content/UI/Textures/T_X")
        );
        assert_eq!(
            content_mount_rel("/Engine/EngineMaterials/Black").as_deref(),
            Some("Engine/Content/EngineMaterials/Black")
        );
        // Plugin / unknown roots are not placeable -> None (caller rejects).
        assert_eq!(content_mount_rel("/MyPlugin/Foo/T_Y"), None);
        assert_eq!(content_mount_rel("NoLeadingSlash"), None);
    }

    #[test]
    fn usmap_missing_dir_errors() {
        let err = usmap(Path::new("/no/such/game")).unwrap_err();
        // read_dir on a missing dir yields an io error -> mapped via From.
        assert!(matches!(err, TexError::Io(_)));
    }
}
