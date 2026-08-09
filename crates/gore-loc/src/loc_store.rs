//! Extract the game's localization into the shared `gore` directory.
//!
//! Ties together [`crate::discover`] (find the `.lcache`), [`crate::loc`] (decrypt
//! + flatten), and [`crate::paths`] (where the shared catalog lives). All three
//! tools call this so one extraction serves every tool. The extracted text is
//! user-local and never committed.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{discover, loc::Lcache, paths};

#[derive(Debug, thiserror::Error)]
pub enum LocStoreError {
    #[error("no AlkimiaLocalization .lcache found (auto-detect failed; pick it manually)")]
    NotFound,
    #[error("reading '{path}': {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("writing '{path}': {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Lcache(#[from] crate::loc::LcacheError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Metadata about the last extraction, written next to the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocMeta {
    /// Absolute path of the `.lcache` the catalog was extracted from.
    pub source_path: String,
    /// Size of that source file in bytes (cheap change-detection signal).
    pub source_bytes: u64,
    pub id_count: usize,
    pub languages: Vec<String>,
    /// Unix seconds when the extraction ran.
    pub extracted_at: u64,
    /// SHA-256 of the source `.lcache` bytes this catalog was built from.
    ///
    /// `extracted_at` cannot answer "is this catalog built from the bytes that are installed now":
    /// it is stamped after the read, the decode and the catalog write, so a `gore loc import` that
    /// lands inside that window leaves the cache with an mtime EARLIER than the extraction and a
    /// catalog built from the previous bytes.
    ///
    /// A timestamp cannot answer it either, whatever unit it is stored in. Nanoseconds do not make
    /// a filesystem's clock finer, and on one with a two-second tick — FAT32, and the removable
    /// media game files live on — a same-length `gore loc import` inside one tick leaves the mtime
    /// identical. Content is the only identity that holds, and the bytes are already in memory
    /// when the catalog is built.
    ///
    /// `None` on catalogs written before this field existed; readers fall back to `extracted_at`.
    #[serde(default)]
    pub source_sha256: Option<String>,
    /// Absolute path of the written catalog.
    pub catalog_path: String,
}

/// Resolve the `.lcache`. A caller-supplied hint is authoritative: it is
/// resolved on its own and, if it doesn't point at an `.lcache`, the result is
/// `None` (no silent Steam fallback to a possibly-different install). Steam
/// auto-detect runs only when no hint is given.
pub fn resolve_lcache(hint: Option<&Path>) -> Option<PathBuf> {
    match hint {
        Some(h) => discover::lcache_from_hint(h),
        None => discover::find_lcache(),
    }
}

/// Read the sidecar metadata, if a previous extraction exists.
pub fn status() -> Option<LocMeta> {
    let bytes = fs::read(paths::loc_meta_path()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Lowercase hex SHA-256, the shape every other digest in this toolkit is written in.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Whether the path still holds the bytes this run read, asked by reading it again.
///
/// This used to compare the modification time from before and after the read. A timestamp cannot
/// answer it: FAT32 keeps whole seconds, and `gore loc import` publishes with a rename, so a cache
/// replaced inside one tick reports the very same mtime. Everything downstream — the catalog, the
/// digest, the doctor's freshness check — would then describe bytes the install no longer has,
/// and say so with confidence.
///
/// It costs one more pass over the file (37 MB in the shipped install) next to decrypting it and
/// writing a 28 MB catalog. What it cannot do is prove the file will still hold them a moment
/// later; it proves the read was not overtaken while this run was working, which is the case that
/// happens.
fn still_holds(path: &Path, digest: &str) -> Result<bool, std::io::Error> {
    Ok(sha256_hex(&fs::read(path)?) == digest)
}

/// True when the shared catalog file is present.
pub fn catalog_present() -> bool {
    paths::loc_catalog_path().is_file()
}

/// Decrypt the `.lcache` (resolved from `hint` or Steam), flatten it, and write
/// `loc_catalog.json` + `loc_meta.json` into the shared `gore` dir.
pub fn extract(hint: Option<&Path>) -> Result<LocMeta, LocStoreError> {
    let lcache = resolve_lcache(hint).ok_or(LocStoreError::NotFound)?;
    let enc = fs::read(&lcache).map_err(|source| LocStoreError::Read {
        path: lcache.display().to_string(),
        source,
    })?;
    let source_bytes = enc.len() as u64;
    // The digest of exactly what was read, so nothing about the file's own metadata has to be
    // trusted later.
    let digest = sha256_hex(&enc);
    let lc = Lcache::decode(&enc)?;
    let catalog = lc.export(false);

    // Asked here rather than right after the read: everything published below describes these
    // bytes, so the question is whether the install still has them at the moment of publishing,
    // and the decode and export are where the time goes.
    let unchanged = still_holds(&lcache, &digest).map_err(|source| LocStoreError::Read {
        path: lcache.display().to_string(),
        source,
    })?;
    if !unchanged {
        return Err(LocStoreError::Read {
            path: lcache.display().to_string(),
            source: std::io::Error::other(
                "the .lcache changed while it was being read — run 'gore loc extract' again",
            ),
        });
    }
    let source_sha256 = Some(digest);

    let dir = paths::shared_data_dir();
    fs::create_dir_all(&dir).map_err(|source| LocStoreError::Write {
        path: dir.display().to_string(),
        source,
    })?;

    // Write via a temp file + rename so a failed/partial write (disk full,
    // interruption) never truncates the previous usable catalog in place.
    let catalog_path = paths::loc_catalog_path();
    write_atomic(&catalog_path, &serde_json::to_vec(&catalog)?).map_err(|source| {
        LocStoreError::Write {
            path: catalog_path.display().to_string(),
            source,
        }
    })?;

    let meta = LocMeta {
        source_path: lcache.display().to_string(),
        source_bytes,
        id_count: catalog.len(),
        languages: lc.languages(),
        extracted_at: now_unix(),
        source_sha256,
        catalog_path: catalog_path.display().to_string(),
    };
    let meta_path = paths::loc_meta_path();
    if let Err(source) = write_atomic(&meta_path, &serde_json::to_vec_pretty(&meta)?) {
        // The fresh catalog is already in place; a leftover meta would describe
        // the *previous* extraction (stale source/counts/timestamp). Drop it so
        // status() reports no metadata rather than wrong provenance.
        let _ = fs::remove_file(&meta_path);
        return Err(LocStoreError::Write {
            path: meta_path.display().to_string(),
            source,
        });
    }
    Ok(meta)
}

/// Write `bytes` to `path` atomically: to a sibling temp file, then rename over
/// `path` (same directory, so the rename stays on one volume). A failed write
/// leaves the existing file untouched.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{sha256_hex, still_holds};

    #[test]
    fn a_same_length_rewrite_is_caught_where_a_timestamp_could_not_see_it() {
        // This check used to be "did the modification time change while we read". FAT32 keeps
        // whole seconds and `gore loc import` publishes with a rename, so a cache replaced inside
        // one tick reports the very same mtime — and the catalog, the recorded digest and the
        // doctor's freshness check would all have described bytes the install no longer has.
        //
        // Nothing here touches a timestamp, deliberately: that is the signal being replaced.
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("AlkimiaLocalization_00000000.lcache");
        std::fs::write(&cache, vec![0u8; 2048]).unwrap();
        let digest = sha256_hex(&std::fs::read(&cache).unwrap());

        assert!(still_holds(&cache, &digest).unwrap(), "unread and unchanged");

        std::fs::write(&cache, vec![1u8; 2048]).unwrap();
        assert_eq!(
            std::fs::metadata(&cache).unwrap().len(),
            2048,
            "the fixture is only interesting while the length still matches"
        );
        assert!(!still_holds(&cache, &digest).unwrap(), "different bytes, same length");

        // A file that cannot be read at all is not an answer of "unchanged".
        std::fs::remove_file(&cache).unwrap();
        assert!(still_holds(&cache, &digest).is_err());
    }
}
