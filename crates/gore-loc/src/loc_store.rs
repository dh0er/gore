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
    let lc = Lcache::decode(&enc)?;
    let catalog = lc.export(false);

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
