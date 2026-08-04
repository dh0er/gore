//! Import mods into the manager library — plus [`list`]/[`remove`] over the imported entries.
//!
//! [`import`] materializes any supported source (dir, `.zip`, single game file) into a staging
//! dir, detects what it is (a goremod bundle via `gore-mod.json`, else a foreign-mod scan),
//! extracts each component's game-side **targets** (for later conflict analysis), and activates
//! the staged dir as `<library>/<id>/` with a [`META_FILE`] sidecar.

use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use super::model::{
    metadata_is_link, open_directory_nofollow, open_file_nofollow, ComponentInfo, LibraryRoot,
    ModEntryMeta, ModKind, RawTarget, SecureDirectory, SecureFile, SecureNode, META_FILE,
};
use crate::{Component, ModError, ModManifest, ScriptEntry, VoicePatchManifest};

/// Default resource envelope for one manager import. These limits are deliberately high enough
/// for multi-gigabyte IoStore mods, but finite so a malformed or hostile ZIP/manifest cannot grow
/// without bound:
///
/// - source ZIP: 16 GiB compressed
/// - ZIP/folder entries / voice-manifest edits: 100,000, with paths up to 4 KiB
/// - one copied/extracted entry: 8 GiB; all copied/extracted entries: 16 GiB
/// - maximum ZIP compression ratio: 1,000:1
/// - folder nesting: [`MAX_SCAN_DEPTH`] directories below the source root
/// - one JSON manifest: 16 MiB
/// - one voice Ogg: 64 MiB; all referenced voice Oggs in one component: 4 GiB
#[derive(Debug, Clone, Copy)]
struct ImportLimits {
    max_zip_bytes: u64,
    max_zip_entries: usize,
    max_zip_path_bytes: usize,
    max_zip_entry_uncompressed_bytes: u64,
    max_zip_total_uncompressed_bytes: u64,
    max_zip_compression_ratio: u64,
    max_directory_depth: usize,
    max_manifest_bytes: u64,
    max_voice_ogg_bytes: u64,
    max_voice_ogg_total_bytes: u64,
}

const DEFAULT_IMPORT_LIMITS: ImportLimits = ImportLimits {
    max_zip_bytes: 16 * 1024 * 1024 * 1024,
    max_zip_entries: 100_000,
    max_zip_path_bytes: 4 * 1024,
    max_zip_entry_uncompressed_bytes: 8 * 1024 * 1024 * 1024,
    max_zip_total_uncompressed_bytes: 16 * 1024 * 1024 * 1024,
    max_zip_compression_ratio: 1_000,
    max_directory_depth: MAX_SCAN_DEPTH,
    max_manifest_bytes: 16 * 1024 * 1024,
    max_voice_ogg_bytes: 64 * 1024 * 1024,
    max_voice_ogg_total_bytes: 4 * 1024 * 1024 * 1024,
};

const REPLACEMENT_PREFIX: &str = ".replacing-";
const REPLACEMENT_STATE_FILE: &str = "replacement.json";
const REPLACEMENT_BACKUP_DIR: &str = "previous";
const REPLACEMENT_STATE_MAX_BYTES: u64 = 4 * 1024;

static REPLACEMENT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Import `source` (a folder, `.zip` archive, or single recognized game file) into the library
/// at `library_dir`, returning the entry metadata that was also written as its sidecar.
///
/// Pipeline: materialize into a `.staging-*` dir under the library (same volume, so activation
/// is a rename) → detect components + extract targets → write the sidecar → swap into place.
/// Re-importing the SAME source (same name AND same source file/dir name) replaces its entry —
/// a mod update — because the id folds both into its hash. Two DIFFERENT mods that happen to
/// share a display name but come from different sources get DISTINCT ids and coexist, rather than
/// one silently clobbering the other.
pub fn import(library_dir: &Path, source: &Path) -> crate::Result<ModEntryMeta> {
    import_with_limits(library_dir, source, DEFAULT_IMPORT_LIMITS)
}

fn import_with_limits(
    library_dir: &Path,
    source: &Path,
    limits: ImportLimits,
) -> crate::Result<ModEntryMeta> {
    if !source.exists() {
        return Err(ModError::Other(format!(
            "import source not found: {}",
            source.display()
        )));
    }
    std::fs::create_dir_all(library_dir).map_err(crate::io("creating library dir"))?;
    {
        let _replacement_lock = replacement_lock();
        recover_interrupted_replacements(library_dir)?;
    }

    // Canonical view so `.`/trailing-separator sources still yield a usable name.
    let canon = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let source_name = canon
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_string)
        .ok_or_else(|| {
            ModError::Other(format!("cannot derive a name from {}", source.display()))
        })?;
    let fallback_name = if canon.is_dir() {
        source_name.clone()
    } else {
        canon
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or(&source_name)
            .to_string()
    };

    // A folder import that points AT the library dir itself — or any parent that contains it —
    // would place the staging dir (created under `library_dir` below) INSIDE the source tree, and
    // the recursive `copy_dir` in `materialize` would then copy staging into itself, growing the
    // path/disk until the filesystem errors. Reject such sources up front. Only directory sources
    // are affected: file/zip imports don't walk the source tree.
    if canon.is_dir() {
        let lib_canon =
            std::fs::canonicalize(library_dir).unwrap_or_else(|_| library_dir.to_path_buf());
        if lib_canon.starts_with(&canon) {
            return Err(ModError::Other(format!(
                "refusing to import {}: it is or contains the manager library directory ({})",
                source.display(),
                library_dir.display()
            )));
        }
    }

    // Claim an actually unique create-new directory. A guessed/colliding staging name must never
    // let a concurrent import share payloads or make the cleanup guard remove somebody else's dir.
    let staging = tempfile::Builder::new()
        .prefix(".staging-")
        .tempdir_in(library_dir)
        .map_err(crate::io("creating staging dir"))?
        .keep();
    // Cleans the staging dir on EVERY early-return path; defused only after activation.
    let mut guard = StagingGuard(Some(staging.clone()));

    // Walk the caller's actual path rather than the canonicalized naming/id view above. This lets
    // materialization reject a root symbolic link or junction instead of silently following it.
    materialize(source, &staging, limits)?;
    wrap_root_ue4ss(&staging, &fallback_name)?;
    // A goremod bundle shipped BELOW a wrapper dir (`Wrap/Sub/gore-mod.json`) is re-rooted so the
    // staging (→ entry) root IS the bundle root. This keeps every stored `ComponentInfo.rel`
    // bundle-root-relative (`audio`, not `Wrap/Sub/audio`), which matters because the manifests
    // INSIDE (audio/scripts/manifest.json, texture PNGs) hold bundle-root-relative payload paths;
    // apply then reads `<entry>/audio/0.wav` as authored instead of a nonexistent nested path.
    reroot_nested_bundle(&staging)?;

    let (manifest, components) = detect(&staging, limits)?;
    if components.is_empty() {
        return Err(ModError::Other(format!(
            "nothing importable recognized in {}",
            source.display()
        )));
    }
    let kind = if manifest.is_some() {
        ModKind::Goremod
    } else {
        foreign_kind(&components)
    };
    let (name, version, author) = match &manifest {
        Some(m) => (
            m.mod_meta.name.clone(),
            m.mod_meta.version.clone(),
            m.mod_meta.author.clone(),
        ),
        None => (fallback_name, String::new(), String::new()),
    };
    let since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let now = since_epoch.as_secs() as i64;
    let now_micros = since_epoch.subsec_micros();
    // Fold the display name and the FULL canonical source path into the disambiguating hash: a
    // re-import of the same source path resolves to the same id and replaces the entry (update),
    // while two different mods that share a display name AND a bare filename but live in different
    // directories (e.g. two `mod.zip` in different folders) still get different ids and coexist
    // instead of one silently clobbering the other. The `slug(name)` prefix keeps the dir
    // human-readable.
    let id = format!(
        "{}-{}",
        slug(&name),
        crate::name_hash(&format!("{name}\0{}", canon.display()))
    );
    let meta = ModEntryMeta {
        id: id.clone(),
        kind,
        name,
        version,
        author,
        imported_at: format_utc(now, now_micros),
        source: source_name,
        components,
    };

    // Sidecar goes into staging BEFORE the swap so the entry appears fully formed — a
    // concurrent `list()` never sees a half-imported dir it would have to skip.
    std::fs::write(staging.join(META_FILE), serde_json::to_vec_pretty(&meta)?)
        .map_err(crate::io("writing entry sidecar"))?;
    let entry_dir = library_dir.join(&id);
    // Activate atomically. Same source (name + source name) ⇒ same id, so a re-import replaces the
    // previous copy (an update); a different source with the same display name hashes to a different
    // id and lands in its own dir. When an entry already exists, move it ASIDE first, promote the
    // staged copy, and only then delete the old one — if promotion fails (crash, transient
    // FS/permission/AV), restore the old entry so a failed update never leaves the library (and the
    // loadout that references it) pointing at a now-missing mod. The backup is dot-prefixed so
    // `list()` skips it during the brief window it exists.
    {
        let _replacement_lock = replacement_lock();
        recover_interrupted_replacements(library_dir)?;
        activate_staged_entry(library_dir, &staging, &entry_dir, &id)?;
    }
    guard.0 = None; // staging IS the entry now — nothing to clean
    Ok(meta)
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReplacementPhase {
    Prepared,
    PreviousMoved,
    Promoted,
    Restored,
}

impl ReplacementPhase {
    fn marker(self) -> Option<&'static str> {
        match self {
            Self::Prepared => None,
            Self::PreviousMoved => Some("phase-previous-moved"),
            Self::Promoted => Some("phase-promoted"),
            Self::Restored => Some("phase-restored"),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ReplacementState {
    format: u32,
    id: String,
    phase: ReplacementPhase,
}

#[derive(Debug)]
struct ReplacementTransaction {
    root: PathBuf,
    state: ReplacementState,
}

impl ReplacementTransaction {
    fn begin(library_dir: &Path, id: &str) -> crate::Result<Self> {
        if !crate::is_safe_mod_name(id) {
            return Err(ModError::Other(format!(
                "invalid replacement entry id {id:?}"
            )));
        }
        // `tempdir_in` claims a random create-new name. Unlike the old PID-only backup path, two
        // imports can never truncate or delete each other's recovery data.
        let root = tempfile::Builder::new()
            .prefix(REPLACEMENT_PREFIX)
            .tempdir_in(library_dir)
            .map_err(crate::io("creating replacement transaction"))?
            .keep();
        let state = ReplacementState {
            format: 1,
            id: id.to_owned(),
            phase: ReplacementPhase::Prepared,
        };
        let transaction = Self { root, state };
        if let Err(error) = transaction.write_initial_state() {
            let cleanup = transaction.cleanup();
            return Err(combine_replacement_errors(
                error,
                cleanup.err(),
                &transaction.root,
            ));
        }
        Ok(transaction)
    }

    fn from_state(root: PathBuf, state: ReplacementState) -> Self {
        Self { root, state }
    }

    fn backup(&self) -> PathBuf {
        self.root.join(REPLACEMENT_BACKUP_DIR)
    }

    fn write_initial_state(&self) -> crate::Result<()> {
        let path = self.root.join(REPLACEMENT_STATE_FILE);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(crate::io("creating replacement state"))?;
        let bytes = serde_json::to_vec(&self.state)?;
        file.write_all(&bytes)
            .map_err(crate::io("writing replacement state"))?;
        file.sync_all()
            .map_err(crate::io("syncing replacement state"))?;
        sync_replacement_directory(&self.root)?;
        sync_replacement_directory(
            self.root
                .parent()
                .ok_or_else(|| ModError::Other("replacement root has no parent".into()))?,
        )
    }

    /// Phase transitions are append-only marker files. A crash can therefore leave an older
    /// marker, but can never tear/truncate the sole copy of the entry id needed for recovery.
    fn mark(&self, phase: ReplacementPhase) -> crate::Result<()> {
        let Some(marker) = phase.marker() else {
            return Ok(());
        };
        let path = self.root.join(marker);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                let bytes = serde_json::to_vec(&phase)?;
                file.write_all(&bytes)
                    .map_err(crate::io("writing replacement phase"))?;
                file.sync_all()
                    .map_err(crate::io("syncing replacement phase"))?;
                sync_replacement_directory(&self.root)
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
            Err(error) => Err(crate::io("creating replacement phase")(error)),
        }
    }

    fn phase(&self) -> crate::Result<ReplacementPhase> {
        for phase in [
            ReplacementPhase::Restored,
            ReplacementPhase::Promoted,
            ReplacementPhase::PreviousMoved,
        ] {
            if path_present(&self.root.join(phase.marker().expect("non-prepared phase")))? {
                return Ok(phase);
            }
        }
        Ok(self.state.phase)
    }

    fn cleanup(&self) -> crate::Result<()> {
        let Some(metadata) = metadata_if_present(&self.root)? else {
            return Ok(());
        };
        if import_metadata_is_link(&metadata) || !metadata.is_dir() {
            return Err(ModError::Other(format!(
                "replacement transaction is not a real directory: {}",
                self.root.display()
            )));
        }
        let phase_markers = [
            ReplacementPhase::PreviousMoved
                .marker()
                .expect("phase marker"),
            ReplacementPhase::Promoted.marker().expect("phase marker"),
            ReplacementPhase::Restored.marker().expect("phase marker"),
        ];
        for entry in std::fs::read_dir(&self.root)
            .map_err(crate::io("reading replacement transaction for cleanup"))?
        {
            let entry = entry.map_err(crate::io("reading replacement cleanup entry"))?;
            let name = entry.file_name();
            let known = name == REPLACEMENT_STATE_FILE
                || name == REPLACEMENT_BACKUP_DIR
                || phase_markers.iter().any(|marker| name == *marker);
            if !known {
                return Err(ModError::Other(format!(
                    "replacement transaction contains an unexpected path: {}",
                    entry.path().display()
                )));
            }
        }

        // Delete the old payload while the durable state file is still present. If removal is
        // interrupted, startup sees the state and safely retries instead of mistaking a partially
        // emptied transaction for an unidentifiable dot-directory.
        let backup = self.backup();
        if let Some(metadata) = metadata_if_present(&backup)? {
            if import_metadata_is_link(&metadata) || !metadata.is_dir() {
                return Err(ModError::Other(format!(
                    "replacement backup is not a real directory: {}",
                    backup.display()
                )));
            }
            std::fs::remove_dir_all(&backup)
                .map_err(crate::io("removing previous replacement entry"))?;
            sync_replacement_directory(&self.root)?;
        }
        for marker in phase_markers {
            remove_replacement_file_if_present(&self.root.join(marker), "replacement phase")?;
        }
        // State is deliberately removed last. A crash after this point can leave only an empty
        // transaction directory, which legacy/partial-startup recovery removes safely.
        remove_replacement_file_if_present(
            &self.root.join(REPLACEMENT_STATE_FILE),
            "replacement state",
        )?;
        sync_replacement_directory(&self.root)?;
        std::fs::remove_dir(&self.root).map_err(crate::io(&format!(
            "removing replacement transaction {}",
            self.root.display()
        )))?;
        let parent = self
            .root
            .parent()
            .ok_or_else(|| ModError::Other("replacement root has no parent".into()))?;
        sync_replacement_directory(parent)
    }
}

fn remove_replacement_file_if_present(path: &Path, label: &str) -> crate::Result<()> {
    let Some(metadata) = metadata_if_present(path)? else {
        return Ok(());
    };
    if import_metadata_is_link(&metadata) || !metadata.is_file() {
        return Err(ModError::Other(format!(
            "{label} is not a regular file: {}",
            path.display()
        )));
    }
    std::fs::remove_file(path).map_err(crate::io(&format!("removing {label}")))
}

fn replacement_lock() -> std::sync::MutexGuard<'static, ()> {
    REPLACEMENT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn activate_staged_entry(
    library_dir: &Path,
    staging: &Path,
    entry_dir: &Path,
    id: &str,
) -> crate::Result<()> {
    let mut rename = rename_replacement_path;
    activate_staged_entry_with(library_dir, staging, entry_dir, id, &mut rename)
}

fn activate_staged_entry_with<F>(
    library_dir: &Path,
    staging: &Path,
    entry_dir: &Path,
    id: &str,
    rename: &mut F,
) -> crate::Result<()>
where
    F: FnMut(&Path, &Path) -> std::io::Result<()>,
{
    let mut sync = sync_staged_tree;
    activate_staged_entry_with_sync(library_dir, staging, entry_dir, id, rename, &mut sync)
}

fn activate_staged_entry_with_sync<F, S>(
    library_dir: &Path,
    staging: &Path,
    entry_dir: &Path,
    id: &str,
    rename: &mut F,
    sync_staged: &mut S,
) -> crate::Result<()>
where
    F: FnMut(&Path, &Path) -> std::io::Result<()>,
    S: FnMut(&Path) -> crate::Result<()>,
{
    // Content durability comes first. In particular, do not move the previous live entry into a
    // recovery transaction until every staged regular file and directory has reached its platform
    // durability barrier. A sync failure therefore leaves the old entry completely untouched.
    sync_staged(staging)?;

    let Some(previous_metadata) = metadata_if_present(entry_dir)? else {
        rename(staging, entry_dir).map_err(crate::io("activating library entry"))?;
        return sync_replacement_directory(library_dir);
    };
    if import_metadata_is_link(&previous_metadata) || !previous_metadata.is_dir() {
        return Err(ModError::Other(format!(
            "existing library entry is not a real directory: {}",
            entry_dir.display()
        )));
    }

    let transaction = ReplacementTransaction::begin(library_dir, id)?;
    if let Err(error) = rename(entry_dir, &transaction.backup()) {
        let original = crate::io("moving the previous entry aside")(error);
        let cleanup = transaction.cleanup();
        return Err(combine_replacement_errors(
            original,
            cleanup.err(),
            &transaction.root,
        ));
    }
    if let Err(error) = sync_replacement_directory(library_dir) {
        return Err(rollback_previous(error, &transaction, entry_dir, rename));
    }
    if let Err(error) = transaction.mark(ReplacementPhase::PreviousMoved) {
        return Err(rollback_previous(error, &transaction, entry_dir, rename));
    }

    if let Err(error) = rename(staging, entry_dir) {
        return Err(rollback_previous(
            crate::io("activating library entry")(error),
            &transaction,
            entry_dir,
            rename,
        ));
    }
    if let Err(error) = sync_replacement_directory(library_dir) {
        let phase_error = transaction.mark(ReplacementPhase::Promoted).err();
        return Err(promoted_replacement_error(
            error,
            phase_error,
            &transaction.root,
        ));
    }
    if let Err(error) = transaction.mark(ReplacementPhase::Promoted) {
        return Err(promoted_replacement_error(error, None, &transaction.root));
    }

    // Once `promoted` is durable, recovery will always retain the new live entry and finish
    // deleting the old copy if this cleanup is interrupted.
    transaction.cleanup()
}

#[cfg(not(windows))]
fn rename_replacement_path(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}

#[cfg(windows)]
fn rename_replacement_path(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

    let from: Vec<u16> = from
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let to: Vec<u16> = to
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: both buffers are stable, NUL-terminated UTF-16 paths for the duration of the call.
    let moved = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Flush every staged file, then its containing directories from deepest to root. Directory fsync
/// is available on Unix. Windows flushes each regular file here and uses `MoveFileExW` with
/// `MOVEFILE_WRITE_THROUGH` in [`rename_replacement_path`] as the directory-entry publication
/// barrier.
fn sync_staged_tree(root: &Path) -> crate::Result<()> {
    let metadata = std::fs::symlink_metadata(root)
        .map_err(crate::io("reading staged tree root metadata before sync"))?;
    if import_metadata_is_link(&metadata) || !metadata.is_dir() {
        return Err(ModError::Other(format!(
            "staged import root is not a real directory: {}",
            root.display()
        )));
    }

    let mut pending = vec![root.to_path_buf()];
    let mut directories = Vec::new();
    while let Some(directory) = pending.pop() {
        directories.push(directory.clone());
        let mut entries = std::fs::read_dir(&directory)
            .map_err(crate::io(&format!(
                "reading staged directory before sync {}",
                directory.display()
            )))?
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(crate::io("reading staged entry before sync"))?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        // Reverse the sorted order before pushing so the stack visits paths deterministically.
        for entry in entries.into_iter().rev() {
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).map_err(crate::io(&format!(
                "reading staged payload metadata before sync {}",
                path.display()
            )))?;
            if import_metadata_is_link(&metadata) {
                return Err(ModError::Other(format!(
                    "staged import contains a symbolic link or reparse point: {}",
                    path.display()
                )));
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                sync_staged_regular_file(&path)?;
            } else {
                return Err(ModError::Other(format!(
                    "staged import contains a non-regular filesystem entry: {}",
                    path.display()
                )));
            }
        }
    }

    for directory in directories.into_iter().rev() {
        sync_replacement_directory(&directory)?;
    }
    Ok(())
}

#[cfg(windows)]
fn sync_staged_regular_file(path: &Path) -> crate::Result<()> {
    // FlushFileBuffers (used by File::sync_all) requires a handle opened for writing on Windows.
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(crate::io(&format!(
            "opening staged file for durable sync {}",
            path.display()
        )))?
        .sync_all()
        .map_err(crate::io("syncing staged regular file"))
}

#[cfg(not(windows))]
fn sync_staged_regular_file(path: &Path) -> crate::Result<()> {
    std::fs::File::open(path)
        .map_err(crate::io(&format!(
            "opening staged file for durable sync {}",
            path.display()
        )))?
        .sync_all()
        .map_err(crate::io("syncing staged regular file"))
}

fn rollback_previous<F>(
    original: ModError,
    transaction: &ReplacementTransaction,
    entry_dir: &Path,
    rename: &mut F,
) -> ModError
where
    F: FnMut(&Path, &Path) -> std::io::Result<()>,
{
    let rollback = (|| -> crate::Result<()> {
        rename(&transaction.backup(), entry_dir)
            .map_err(crate::io("restoring previous library entry"))?;
        let library_dir = transaction
            .root
            .parent()
            .ok_or_else(|| ModError::Other("replacement root has no parent".into()))?;
        sync_replacement_directory(library_dir)?;
        transaction.mark(ReplacementPhase::Restored)?;
        transaction.cleanup()
    })();
    combine_replacement_errors(original, rollback.err(), &transaction.root)
}

fn combine_replacement_errors(
    original: ModError,
    recovery: Option<ModError>,
    transaction_root: &Path,
) -> ModError {
    match recovery {
        None => original,
        Some(recovery) => ModError::Other(format!(
            "{original}; restoring/cleaning the previous entry also failed: {recovery}; recovery data retained at {}",
            transaction_root.display()
        )),
    }
}

fn promoted_replacement_error(
    original: ModError,
    phase_error: Option<ModError>,
    transaction_root: &Path,
) -> ModError {
    let phase_detail = phase_error
        .map(|error| format!("; recording the promoted phase also failed: {error}"))
        .unwrap_or_default();
    ModError::Other(format!(
        "{original}{phase_detail}; the new entry was already promoted and remains active; any recovery data is retained at {}",
        transaction_root.display()
    ))
}

fn recover_interrupted_replacements(library_dir: &Path) -> crate::Result<()> {
    let read_dir = match std::fs::read_dir(library_dir) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(crate::io("reading replacement transactions")(error)),
    };
    let mut roots = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(crate::io("reading replacement transaction entry"))?;
        if !entry
            .file_name()
            .to_string_lossy()
            .starts_with(REPLACEMENT_PREFIX)
        {
            continue;
        }
        let metadata = std::fs::symlink_metadata(entry.path())
            .map_err(crate::io("reading replacement transaction metadata"))?;
        if import_metadata_is_link(&metadata) {
            return Err(ModError::Other(format!(
                "replacement transaction is a symbolic link or reparse point: {}",
                entry.path().display()
            )));
        }
        if metadata.is_dir() {
            roots.push(entry.path());
        }
    }
    roots.sort();
    for root in roots {
        recover_replacement(library_dir, root)?;
    }
    Ok(())
}

fn recover_replacement(library_dir: &Path, root: PathBuf) -> crate::Result<()> {
    let state_path = root.join(REPLACEMENT_STATE_FILE);
    if !path_present(&state_path)? {
        return recover_legacy_replacement(library_dir, &root);
    }
    let state = read_replacement_state(&state_path)?;
    if state.format != 1 || !crate::is_safe_mod_name(&state.id) {
        return Err(ModError::Other(format!(
            "invalid replacement state in {}",
            state_path.display()
        )));
    }
    let transaction = ReplacementTransaction::from_state(root, state);
    let phase = transaction.phase()?;
    let entry_dir = library_dir.join(&transaction.state.id);
    let live = metadata_if_present(&entry_dir)?;
    let backup_path = transaction.backup();
    let backup = metadata_if_present(&backup_path)?;
    validate_replacement_entry_metadata(live.as_ref(), &entry_dir, "live")?;
    validate_replacement_entry_metadata(backup.as_ref(), &backup_path, "backup")?;

    match (live.is_some(), backup.is_some()) {
        // Both paths means promotion completed atomically but cleanup (and possibly its final phase
        // marker) did not. The visible entry is the promoted copy; discard the previous one.
        (true, true) | (true, false) => transaction.cleanup(),
        (false, true) => {
            rename_replacement_path(&backup_path, &entry_dir).map_err(|error| {
                ModError::Other(format!(
                    "restoring interrupted replacement {:?} for {} failed: {error}",
                    phase,
                    entry_dir.display()
                ))
            })?;
            sync_replacement_directory(library_dir)?;
            transaction.mark(ReplacementPhase::Restored)?;
            transaction.cleanup()
        }
        (false, false) => Err(ModError::Other(format!(
            "cannot recover interrupted replacement {:?} for {:?}: both live and backup entries are missing (state at {})",
            phase,
            transaction.state.id,
            transaction.root.display()
        ))),
    }
}

/// Recover PID-named backups written by the pre-transaction implementation. The entry's own
/// bounded sidecar supplies the id; no path component is inferred from the dot-directory name.
fn recover_legacy_replacement(library_dir: &Path, root: &Path) -> crate::Result<()> {
    let meta_path = root.join(META_FILE);
    if !path_present(&meta_path)? {
        if std::fs::read_dir(root)
            .map_err(crate::io("reading incomplete replacement transaction"))?
            .next()
            .is_none()
        {
            std::fs::remove_dir(root)
                .map_err(crate::io("removing empty replacement transaction"))?;
            return sync_replacement_directory(library_dir);
        }
        return Err(ModError::Other(format!(
            "replacement transaction has no recoverable state: {}",
            root.display()
        )));
    }
    let metadata = std::fs::symlink_metadata(&meta_path)
        .map_err(crate::io("reading legacy replacement sidecar metadata"))?;
    if import_metadata_is_link(&metadata)
        || !metadata.is_file()
        || metadata.len() > DEFAULT_IMPORT_LIMITS.max_manifest_bytes
    {
        return Err(ModError::Other(format!(
            "legacy replacement sidecar is unsafe or oversized: {}",
            meta_path.display()
        )));
    }
    let meta: ModEntryMeta = serde_json::from_slice(&read_nofollow_bounded(
        &meta_path,
        "legacy replacement sidecar",
        DEFAULT_IMPORT_LIMITS.max_manifest_bytes,
    )?)?;
    if !crate::is_safe_mod_name(&meta.id) {
        return Err(ModError::Other(format!(
            "legacy replacement contains invalid entry id {:?}",
            meta.id
        )));
    }
    let entry_dir = library_dir.join(&meta.id);
    if let Some(live) = metadata_if_present(&entry_dir)? {
        validate_replacement_entry_metadata(Some(&live), &entry_dir, "live")?;
        std::fs::remove_dir_all(root).map_err(crate::io("removing legacy replacement backup"))?;
    } else {
        rename_replacement_path(root, &entry_dir)
            .map_err(crate::io("restoring legacy replacement backup"))?;
    }
    sync_replacement_directory(library_dir)
}

fn read_replacement_state(path: &Path) -> crate::Result<ReplacementState> {
    let bytes = read_nofollow_bounded(path, "replacement state", REPLACEMENT_STATE_MAX_BYTES)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn read_nofollow_bounded(path: &Path, label: &str, limit: u64) -> crate::Result<Vec<u8>> {
    let mut file = open_file_nofollow(path, label)?;
    if file.len() > limit {
        return Err(ModError::Other(format!(
            "{label} exceeds the {limit} byte limit: {}",
            file.path().display()
        )));
    }
    let expected = file.len();
    let capacity = usize::try_from(expected)
        .map_err(|_| ModError::Other(format!("{label} exceeds process address space")))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| ModError::Other(format!("could not reserve memory for {label}")))?;
    std::io::Read::by_ref(&mut file.file)
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(crate::io(&format!("reading opened {label}")))?;
    if bytes.len() as u64 != expected {
        return Err(ModError::Other(format!(
            "{label} changed while being read: {}",
            file.path().display()
        )));
    }
    file.verify_len(expected, label)?;
    Ok(bytes)
}

fn validate_replacement_entry_metadata(
    metadata: Option<&std::fs::Metadata>,
    path: &Path,
    label: &str,
) -> crate::Result<()> {
    if let Some(metadata) = metadata {
        if import_metadata_is_link(metadata) || !metadata.is_dir() {
            return Err(ModError::Other(format!(
                "replacement {label} entry is not a real directory: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn metadata_if_present(path: &Path) -> crate::Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(crate::io(&format!(
            "reading metadata for {}",
            path.display()
        ))(error)),
    }
}

fn path_present(path: &Path) -> crate::Result<bool> {
    metadata_if_present(path).map(|metadata| metadata.is_some())
}

/// Persist directory-entry changes on platforms that expose a portable directory fsync.
#[cfg(unix)]
fn sync_replacement_directory(path: &Path) -> crate::Result<()> {
    std::fs::File::open(path)
        .map_err(crate::io("opening replacement directory for sync"))?
        .sync_all()
        .map_err(crate::io("syncing replacement directory"))
}

#[cfg(windows)]
fn sync_replacement_directory(_path: &Path) -> crate::Result<()> {
    // std exposes no portable Windows directory fsync. All replacement/recovery renames use
    // `rename_replacement_path`, whose MoveFileExW call requests MOVEFILE_WRITE_THROUGH.
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn sync_replacement_directory(_path: &Path) -> crate::Result<()> {
    Ok(())
}

/// Delete library entry `id` (the dir `<library_dir>/<id>`); `Ok(false)` if it doesn't exist.
pub fn remove(library_dir: &Path, id: &str) -> crate::Result<bool> {
    // `id` becomes a path component — refuse anything that could climb out of the library.
    if !crate::is_safe_mod_name(id) {
        return Err(ModError::Other(format!("invalid library entry id {id:?}")));
    }
    let _replacement_lock = replacement_lock();
    recover_interrupted_replacements(library_dir)?;
    let dir = library_dir.join(id);
    let metadata = match std::fs::symlink_metadata(&dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(crate::io(&format!(
                "reading library entry metadata {}",
                dir.display()
            ))(error))
        }
    };
    if metadata_is_link(&metadata) || !metadata.is_dir() {
        return Err(ModError::Other(format!(
            "refusing to remove unsafe library entry: {}",
            dir.display()
        )));
    }
    let library = LibraryRoot::open(library_dir)?;
    let entry = library.entry(id)?;
    let entry_path = entry.path().to_path_buf();
    // The entry handle intentionally denies FILE_SHARE_DELETE and must be released before the
    // authorized removal. Keep the validated library-root anchor alive across the path operation:
    // on Windows it prevents parent replacement, and on Unix it is the exact directory handle that
    // receives the post-delete durability barrier below.
    drop(entry);
    std::fs::remove_dir_all(&entry_path).map_err(crate::io(&format!(
        "removing entry {}",
        entry_path.display()
    )))?;
    library.sync_after_mutation()?;
    Ok(true)
}

/// All library entries, sorted by name. Entries with an unreadable/corrupt sidecar are skipped
/// (with a note on stderr), a missing library dir is an empty library.
pub fn list(library_dir: &Path) -> crate::Result<Vec<ModEntryMeta>> {
    let _replacement_lock = replacement_lock();
    recover_interrupted_replacements(library_dir)?;
    let rd = match std::fs::read_dir(library_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(crate::io(&format!(
                "reading library {}",
                library_dir.display()
            ))(e))
        }
    };
    let library = LibraryRoot::open(library_dir)?;
    let mut out = Vec::new();
    for entry in rd.filter_map(|e| e.ok()) {
        let path = entry.path();
        // Dot-dirs are transient staging areas (possibly a concurrent import), not entries.
        let file_name = entry.file_name();
        if file_name.to_string_lossy().starts_with('.') {
            continue;
        }
        let parsed = file_name
            .to_str()
            .ok_or_else(|| "library entry id is not valid Unicode".to_string())
            .and_then(|id| {
                library
                    .entry(id)
                    .and_then(|entry| entry.read_meta())
                    .map_err(|error| error.to_string())
            });
        match parsed {
            Ok(meta) => out.push(meta),
            Err(e) => {
                eprintln!(
                    "gore-mod: skipping unreadable library entry {}: {e}",
                    path.display()
                );
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    Ok(out)
}

// ── Materialization ─────────────────────────────────────────────────────────

/// Copy/extract `source` into the empty `staging` dir.
fn materialize(source: &Path, staging: &Path, limits: ImportLimits) -> crate::Result<()> {
    let source_metadata = std::fs::symlink_metadata(source).map_err(crate::io(&format!(
        "reading import source metadata {}",
        source.display()
    )))?;
    if import_metadata_is_link(&source_metadata) {
        return Err(ModError::Other(format!(
            "import source is a symbolic link or reparse point (folder import root is not a real directory): {}",
            source.display()
        )));
    }
    if source_metadata.is_dir() {
        return copy_import_directory(source, staging, limits);
    }
    if !source_metadata.is_file() {
        return Err(ModError::Other(format!(
            "import source is neither a regular file nor a directory: {}",
            source.display()
        )));
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "zip" => extract_zip(source, staging, limits),
        "7z" | "rar" => Err(ModError::Other(format!(
            "archive format .{ext} not supported — extract manually and import the folder"
        ))),
        "utoc" | "ucas" | "pak" | "lcache" | "bank" | "cache" => {
            let mut sources = vec![source.to_path_buf()];
            // A container file only works as a set: discover the same-stem siblings first. This runs
            // for `.pak` too — importing the `.pak` member of an IoStore triplet (the common
            // file-picker pick) must still materialize the `.utoc`/`.ucas`. Every sibling is
            // preflighted before the selected file is written, so a bad sibling leaves no partial
            // payload behind. A lone loose `_P.pak` simply has no siblings to pull.
            if ext == "utoc" || ext == "ucas" || ext == "pak" {
                for sib_ext in ["utoc", "ucas", "pak"] {
                    if sib_ext == ext {
                        continue;
                    }
                    let sib = source.with_extension(sib_ext);
                    match std::fs::symlink_metadata(&sib) {
                        Ok(_) => sources.push(sib),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => {
                            return Err(crate::io(&format!(
                                "reading import sibling metadata {}",
                                sib.display()
                            ))(error))
                        }
                    }
                }
            }
            materialize_single_file_set(&sources, staging, limits)
        }
        _ => Err(ModError::Other(format!(
            "unrecognized import source {}: expected a folder, .zip, a pak/utoc container, \
             or a known game file (.lcache/.bank/PrecompiledScript*.Cache)",
            source.display()
        ))),
    }
}

#[derive(Debug)]
struct SingleFileCandidate {
    file_name: String,
    file: SecureFile,
}

/// Preflight a selected game file and every discovered IoStore sibling before creating any staged
/// file, then copy each through the same opened-handle, size-stable path used for folder imports.
fn materialize_single_file_set(
    sources: &[PathBuf],
    staging: &Path,
    limits: ImportLimits,
) -> crate::Result<()> {
    check_import_limit(
        "single-file import entry count",
        sources.len() as u64,
        limits.max_zip_entries as u64,
    )?;
    let mut candidates = Vec::with_capacity(sources.len());
    let mut total_bytes = 0u64;
    let mut destinations = BTreeMap::<String, String>::new();
    for source in sources {
        let file = open_file_nofollow(source, "single-file import member")?;
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                ModError::Other(format!(
                    "single-file import name is not valid Unicode: {}",
                    source.display()
                ))
            })?
            .to_owned();
        check_import_limit(
            "single-file import path bytes",
            file_name.len() as u64,
            limits.max_zip_path_bytes as u64,
        )?;
        if !crate::is_safe_rel_path(&file_name) || Path::new(&file_name).components().count() != 1 {
            return Err(ModError::Other(format!(
                "single-file import has an unsafe file name: {file_name:?}"
            )));
        }
        let folded = file_name.to_lowercase();
        if let Some(first) = destinations.insert(folded, file_name.clone()) {
            return Err(ModError::Other(format!(
                "single-file import members {first:?} and {file_name:?} have the same portable destination"
            )));
        }
        check_import_limit(
            "single-file import entry bytes",
            file.len(),
            limits.max_zip_entry_uncompressed_bytes,
        )?;
        total_bytes = total_bytes
            .checked_add(file.len())
            .ok_or_else(|| ModError::Other("single-file import byte count overflowed".into()))?;
        check_import_limit(
            "single-file import total bytes",
            total_bytes,
            limits.max_zip_total_uncompressed_bytes,
        )?;
        candidates.push(SingleFileCandidate { file_name, file });
    }

    let mut copied_total = 0u64;
    for candidate in candidates {
        let expected_bytes = candidate.file.len();
        copy_opened_import_file(
            candidate.file,
            &staging.join(&candidate.file_name),
            limits.max_zip_entry_uncompressed_bytes,
            limits
                .max_zip_total_uncompressed_bytes
                .saturating_sub(copied_total),
        )?;
        copied_total = copied_total
            .checked_add(expected_bytes)
            .ok_or_else(|| ModError::Other("single-file copied byte count overflowed".into()))?;
    }
    Ok(())
}

/// Copy an unpacked import through the same finite resource envelope used for ZIP extraction.
/// Unlike the old generic `copy_dir`, every read-dir/type error is surfaced, links/reparse points
/// and special files are rejected, and no byte beyond either cap is ever written into staging.
fn copy_import_directory(source: &Path, staging: &Path, limits: ImportLimits) -> crate::Result<()> {
    let source = open_directory_nofollow(source, "folder import root")?;
    let mut budget = DirectoryCopyBudget {
        entries: 0,
        total_bytes: 0,
    };
    copy_import_directory_at(&source, Path::new(""), staging, 0, limits, &mut budget)
}

#[derive(Debug, Default)]
struct DirectoryCopyBudget {
    entries: usize,
    total_bytes: u64,
}

fn copy_import_directory_at(
    source: &SecureDirectory,
    relative_dir: &Path,
    staging: &Path,
    depth: usize,
    limits: ImportLimits,
    budget: &mut DirectoryCopyBudget,
) -> crate::Result<()> {
    let entries = source.read_dir("folder import")?;
    // Do not collect/sort the whole directory before checking the budget: a directory with more
    // than the allowed number of entries must stop at limit+1 without first allocating for all of
    // them. Detection sorts the already-bounded staged tree later where determinism matters.
    for entry in entries {
        let entry = entry.map_err(crate::io("reading folder import entry"))?;
        budget.entries = budget
            .entries
            .checked_add(1)
            .ok_or_else(|| ModError::Other("folder import entry count overflowed".into()))?;
        check_import_limit(
            "folder entry count",
            budget.entries as u64,
            limits.max_zip_entries as u64,
        )?;

        let name = entry.file_name();
        let from = source.path().join(&name);
        let rel = relative_dir.join(&name);
        let rel_text = portable_import_rel_path(&rel, &from)?;
        check_import_limit(
            "folder entry path bytes",
            rel_text.len() as u64,
            limits.max_zip_path_bytes as u64,
        )?;
        if !crate::is_safe_rel_path(&rel_text) {
            return Err(ModError::Other(format!(
                "folder import entry has an unsafe relative path: {rel_text:?}"
            )));
        }

        let to = staging.join(&name);
        match source.open_child(&name, "folder import entry")? {
            SecureNode::Directory(directory) => {
                if depth >= limits.max_directory_depth {
                    return Err(ModError::Other(format!(
                        "folder import nesting depth limit exceeded at {}: {} > {}",
                        from.display(),
                        depth + 1,
                        limits.max_directory_depth
                    )));
                }
                std::fs::create_dir(&to).map_err(crate::io(&format!(
                    "creating staged folder import directory {}",
                    to.display()
                )))?;
                copy_import_directory_at(&directory, &rel, &to, depth + 1, limits, budget)?;
            }
            SecureNode::File(file) => {
                check_import_limit(
                    "folder entry bytes",
                    file.len(),
                    limits.max_zip_entry_uncompressed_bytes,
                )?;
                let next_total = budget
                    .total_bytes
                    .checked_add(file.len())
                    .ok_or_else(|| ModError::Other("folder import byte count overflowed".into()))?;
                check_import_limit(
                    "folder total bytes",
                    next_total,
                    limits.max_zip_total_uncompressed_bytes,
                )?;
                // Charge before copying so the aggregate cap cannot be exceeded by one full file.
                budget.total_bytes = next_total;
                copy_opened_import_file(
                    file,
                    &to,
                    limits.max_zip_entry_uncompressed_bytes,
                    next_total,
                )?;
            }
        }
    }
    Ok(())
}

fn portable_import_rel_path(rel: &Path, source_path: &Path) -> crate::Result<String> {
    let mut components = Vec::new();
    for component in rel.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(ModError::Other(format!(
                "folder import path is not a plain relative path: {}",
                source_path.display()
            )));
        };
        components.push(
            component
                .to_str()
                .ok_or_else(|| {
                    ModError::Other(format!(
                        "folder import path is not valid Unicode: {}",
                        source_path.display()
                    ))
                })?
                .to_owned(),
        );
    }
    if components.is_empty() {
        return Err(ModError::Other(format!(
            "folder import entry has an empty relative path: {}",
            source_path.display()
        )));
    }
    Ok(components.join("/"))
}

fn copy_opened_import_file(
    source: SecureFile,
    destination: &Path,
    max_file_bytes: u64,
    remaining_total_bytes: u64,
) -> crate::Result<()> {
    copy_opened_import_file_with(
        source,
        destination,
        max_file_bytes,
        remaining_total_bytes,
        || {},
    )
}

fn copy_opened_import_file_with<F>(
    mut source: SecureFile,
    destination: &Path,
    max_file_bytes: u64,
    remaining_total_bytes: u64,
    after_open: F,
) -> crate::Result<()>
where
    F: FnOnce(),
{
    let expected_bytes = source.len();
    let effective_limit = max_file_bytes.min(remaining_total_bytes);
    if expected_bytes > effective_limit {
        return Err(ModError::Other(format!(
            "import file exceeds its bounded remaining byte limit: {expected_bytes} > {effective_limit}: {}",
            source.path().display()
        )));
    }
    after_open();

    // Never write more than the size already charged to the budget. If the source grows after its
    // metadata snapshot, the one-byte probe below detects it without copying that byte to staging.
    let max_copy = expected_bytes.min(effective_limit);
    let mut destination_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(crate::io(&format!(
            "creating staged import file {}",
            destination.display()
        )))?;
    let copy_result = (|| -> crate::Result<()> {
        let copied = std::io::copy(
            &mut std::io::Read::by_ref(&mut source.file).take(max_copy),
            &mut destination_file,
        )
        .map_err(crate::io(&format!(
            "copying import file {}",
            source.path().display()
        )))?;
        let mut probe = [0u8; 1];
        let has_more = source
            .file
            .read(&mut probe)
            .map_err(crate::io("probing import file size"))?
            != 0;
        source.verify_len(expected_bytes, "import file")?;
        if has_more || copied != expected_bytes {
            return Err(ModError::Other(format!(
                "import file changed or exceeded its byte limit while being copied: {}",
                source.path().display()
            )));
        }
        destination_file
            .flush()
            .map_err(crate::io("flushing staged import file"))?;
        Ok(())
    })();
    drop(destination_file);
    if copy_result.is_err() {
        let _ = std::fs::remove_file(destination);
    }
    copy_result
}

#[cfg(test)]
fn copy_import_regular_file_with<F>(
    source: &Path,
    destination: &Path,
    expected_bytes: u64,
    max_file_bytes: u64,
    remaining_total_bytes: u64,
    after_open: F,
) -> crate::Result<()>
where
    F: FnOnce(),
{
    let file = open_file_nofollow(source, "test import file")?;
    if file.len() != expected_bytes {
        return Err(ModError::Other(format!(
            "test import file changed before opened-handle copy: {} != {expected_bytes}",
            file.len()
        )));
    }
    copy_opened_import_file_with(
        file,
        destination,
        max_file_bytes,
        remaining_total_bytes,
        after_open,
    )
}

/// Extract a zip into `staging`, refusing any entry whose name could escape it.
fn extract_zip(zip_path: &Path, staging: &Path, limits: ImportLimits) -> crate::Result<()> {
    let zip_source = open_file_nofollow(zip_path, "ZIP import source")?;
    check_import_limit(
        "ZIP compressed bytes",
        zip_source.len(),
        limits.max_zip_bytes,
    )?;
    let file = zip_source
        .file
        .try_clone()
        .map_err(crate::io("cloning opened ZIP handle"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ModError::Other(format!("reading zip {}: {e}", zip_path.display())))?;
    preflight_zip(&mut archive, limits)?;
    let mut copied_total = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ModError::Other(format!("reading zip entry {i}: {e}")))?;
        let raw_name = entry.name().to_string();
        let Some(rel) = safe_zip_entry(&raw_name, limits.max_zip_path_bytes) else {
            return Err(ModError::Other(format!(
                "zip entry {raw_name:?} has an unsafe path (absolute, drive letter, or '..') — \
                 refusing to extract"
            )));
        };
        let dest = staging.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest).map_err(crate::io("creating zip dir"))?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(crate::io("creating zip parent dir"))?;
        }
        let mut out = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dest)
            .map_err(crate::io(&format!("creating {}", dest.display())))?;
        let declared = entry.size();
        let max_read = declared.saturating_add(1);
        let copy_result = std::io::copy(&mut (&mut entry).take(max_read), &mut out)
            .map_err(crate::io(&format!("extracting {raw_name}")));
        let copied = match copy_result {
            Ok(copied) => copied,
            Err(error) => {
                drop(out);
                let _ = std::fs::remove_file(&dest);
                return Err(error);
            }
        };
        if copied != declared {
            drop(out);
            let _ = std::fs::remove_file(&dest);
            return Err(ModError::Other(format!(
                "ZIP entry {raw_name:?} extracted {copied} bytes, expected {declared}"
            )));
        }
        copied_total = copied_total
            .checked_add(copied)
            .ok_or_else(|| ModError::Other("ZIP extracted byte count overflowed".into()))?;
        check_import_limit(
            "ZIP total extracted bytes",
            copied_total,
            limits.max_zip_total_uncompressed_bytes,
        )?;
    }
    drop(archive);
    zip_source.verify_len(zip_source.len(), "ZIP import source")?;
    Ok(())
}

/// Validate every central-directory entry before extraction starts. Limit failures therefore leave
/// the staging directory empty; the import guard removes it before returning the error.
fn preflight_zip(
    archive: &mut zip::ZipArchive<std::fs::File>,
    limits: ImportLimits,
) -> crate::Result<()> {
    if archive.len() > limits.max_zip_entries {
        return Err(ModError::Other(format!(
            "ZIP entry count limit exceeded: {} > {}",
            archive.len(),
            limits.max_zip_entries
        )));
    }

    let mut total_uncompressed = 0u64;
    let mut targets = BTreeMap::<String, String>::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| ModError::Other(format!("reading zip entry {index}: {e}")))?;
        let raw_name = entry.name().to_string();
        check_import_limit(
            "ZIP entry path bytes",
            entry.name_raw().len() as u64,
            limits.max_zip_path_bytes as u64,
        )?;
        let Some(rel) = safe_zip_entry(&raw_name, limits.max_zip_path_bytes) else {
            return Err(ModError::Other(format!(
                "zip entry {raw_name:?} has an unsafe path; refusing to extract"
            )));
        };
        if entry.is_symlink() {
            return Err(ModError::Other(format!(
                "ZIP entry {raw_name:?} is a symbolic link; refusing to extract"
            )));
        }
        if entry.encrypted() {
            return Err(ModError::Other(format!(
                "ZIP entry {raw_name:?} is encrypted; refusing to extract"
            )));
        }

        let uncompressed = entry.size();
        let compressed = entry.compressed_size();
        if entry.is_dir() && uncompressed != 0 {
            return Err(ModError::Other(format!(
                "ZIP directory entry {raw_name:?} declares {uncompressed} data bytes"
            )));
        }
        check_import_limit(
            "ZIP entry uncompressed bytes",
            uncompressed,
            limits.max_zip_entry_uncompressed_bytes,
        )?;
        check_zip_ratio(
            &raw_name,
            uncompressed,
            compressed,
            limits.max_zip_compression_ratio,
        )?;
        total_uncompressed = total_uncompressed
            .checked_add(uncompressed)
            .ok_or_else(|| {
                ModError::Other("ZIP total uncompressed byte count overflowed".into())
            })?;
        check_import_limit(
            "ZIP total uncompressed bytes",
            total_uncompressed,
            limits.max_zip_total_uncompressed_bytes,
        )?;

        let key = rel.replace('\\', "/").to_lowercase();
        if let Some(first) = targets.insert(key, raw_name.clone()) {
            return Err(ModError::Other(format!(
                "ZIP entries {first:?} and {raw_name:?} have the same portable extraction path"
            )));
        }
    }
    Ok(())
}

fn check_import_limit(kind: &str, actual: u64, limit: u64) -> crate::Result<()> {
    if actual > limit {
        return Err(ModError::Other(format!(
            "{kind} limit exceeded: {actual} > {limit}"
        )));
    }
    Ok(())
}

fn check_zip_ratio(
    name: &str,
    uncompressed: u64,
    compressed: u64,
    max_ratio: u64,
) -> crate::Result<()> {
    if uncompressed == 0 {
        return Ok(());
    }
    let allowed = compressed.saturating_mul(max_ratio);
    if compressed == 0 || uncompressed > allowed {
        return Err(ModError::Other(format!(
            "ZIP entry {name:?} compression ratio limit exceeded: {uncompressed} bytes from \
             {compressed} compressed bytes (maximum {max_ratio}:1)"
        )));
    }
    Ok(())
}

/// Normalized safe relative path for a zip entry, or `None` if it must be rejected
/// (absolute, drive letter, `..`, control chars). Trailing `/` (dir markers) is dropped.
fn safe_zip_entry(name: &str, max_path_bytes: usize) -> Option<String> {
    let n = name.replace('\\', "/");
    let n = n.trim_end_matches('/');
    let path_limits = gore_vo::Limits {
        max_path_bytes,
        ..gore_vo::Limits::default()
    };
    if gore_vo::validate_archive_entry_path(n, &path_limits).is_err() {
        return None;
    }
    // Platform-aware second opinion (prefix/root components etc.).
    if !crate::is_safe_rel_path(n) {
        return None;
    }
    Some(n.to_string())
}

/// A source that IS a UE4SS mod (root holds `Scripts/main.lua`) gets nested into a `<name>/`
/// subdir, so entries are uniformly "mod dirs inside the entry" and a later deploy-copy of the
/// mod dir can never drag the sidecar along.
fn wrap_root_ue4ss(staging: &Path, name: &str) -> crate::Result<()> {
    if !staging.join("Scripts").join("main.lua").is_file() {
        return Ok(());
    }
    let tmp = staging.join(".gore-wrap");
    std::fs::create_dir(&tmp).map_err(crate::io("creating wrap dir"))?;
    let entries: Vec<_> = std::fs::read_dir(staging)
        .map_err(crate::io("reading staging"))?
        .filter_map(|e| e.ok())
        .collect();
    for e in entries {
        if e.file_name().to_string_lossy() == ".gore-wrap" {
            continue;
        }
        std::fs::rename(e.path(), tmp.join(e.file_name()))
            .map_err(crate::io("wrapping mod dir"))?;
    }
    // The wrapped dir becomes the UE4SS mod name — keep it a single safe component.
    let safe = if crate::is_safe_mod_name(name) {
        name.to_string()
    } else {
        slug(name)
    };
    std::fs::rename(&tmp, staging.join(&safe)).map_err(crate::io("naming mod dir"))?;
    Ok(())
}

/// If a goremod bundle sits BELOW `staging` (its `gore-mod.json` is in a nested wrapper dir like
/// `Wrap/Sub`), hoist that bundle subtree up so `staging` itself becomes the bundle root. After
/// this, [`find_manifest_dir`] finds `gore-mod.json` at the root and every component `rel` is
/// bundle-root-relative — which is what the payload manifests inside the bundle already assume.
///
/// No-op when there's no manifest, or the manifest is already at the root (the common flat case,
/// and every foreign import, which has no `gore-mod.json`).
fn reroot_nested_bundle(staging: &Path) -> crate::Result<()> {
    let Some(bundle_dir) = find_manifest_dir(staging) else {
        return Ok(());
    };
    if bundle_dir == staging {
        return Ok(()); // already rooted at the bundle
    }
    // Stash the nested bundle subtree at a fresh sibling under `staging` first (a valid rename:
    // `.gore-reroot` is NOT inside `bundle_dir`, so this doesn't move a dir into itself). Then clear
    // the old wrapper dirs and hoist the stashed bundle's children up to the root.
    let stash = staging.join(".gore-reroot");
    if stash.exists() {
        std::fs::remove_dir_all(&stash).map_err(crate::io("clearing reroot stash"))?;
    }
    std::fs::rename(&bundle_dir, &stash).map_err(crate::io("stashing nested bundle"))?;

    // Remove every remaining top-level entry (the emptied wrapper dirs and any stray sibling files
    // shipped alongside the bundle folder) so only the hoisted bundle content remains.
    for e in std::fs::read_dir(staging).map_err(crate::io("reading staging for reroot"))? {
        let e = e.map_err(crate::io("reading staging entry"))?;
        if e.file_name() == std::ffi::OsStr::new(".gore-reroot") {
            continue;
        }
        let p = e.path();
        let md = std::fs::symlink_metadata(&p).map_err(crate::io("stat reroot leftover"))?;
        if md.is_dir() {
            std::fs::remove_dir_all(&p).map_err(crate::io("removing wrapper dir"))?;
        } else {
            std::fs::remove_file(&p).map_err(crate::io("removing stray sibling"))?;
        }
    }

    // Hoist the stashed bundle's children up to the staging root, then drop the empty stash.
    for e in std::fs::read_dir(&stash).map_err(crate::io("reading reroot stash"))? {
        let e = e.map_err(crate::io("reading stash entry"))?;
        std::fs::rename(e.path(), staging.join(e.file_name()))
            .map_err(crate::io("hoisting bundle content"))?;
    }
    std::fs::remove_dir(&stash).map_err(crate::io("removing reroot stash"))?;
    Ok(())
}

// ── Detection ───────────────────────────────────────────────────────────────

/// Detect what the staged tree is: a goremod bundle (`gore-mod.json` at the root or nested at
/// most two folders deep — the usual "zip contains a folder" shipping shapes) or foreign files.
fn detect(
    staging: &Path,
    limits: ImportLimits,
) -> crate::Result<(Option<ModManifest>, Vec<ComponentInfo>)> {
    if let Some(bundle_dir) = find_manifest_dir(staging) {
        let bytes = read_bounded_bundle_file(
            &bundle_dir,
            Path::new("gore-mod.json"),
            "gore-mod.json",
            limits.max_manifest_bytes,
        )?;
        let manifest: ModManifest = serde_json::from_slice(&bytes)?;
        if manifest.format != 1 {
            return Err(ModError::Other(format!(
                "unsupported gore-mod.json format {} (expected 1)",
                manifest.format
            )));
        }
        let raw: serde_json::Value = serde_json::from_slice(&bytes)?;
        let prefix = rel_str(staging, &bundle_dir); // "" when the bundle is the staging root
        let comps = goremod_components(&bundle_dir, &prefix, &manifest, &raw, limits)?;
        Ok((Some(manifest), comps))
    } else {
        Ok((None, scan_foreign(staging)?))
    }
}

/// First dir at depth ≤2 (BFS, sorted — deterministic) containing `gore-mod.json`.
fn find_manifest_dir(root: &Path) -> Option<PathBuf> {
    if root.join("gore-mod.json").is_file() {
        return Some(root.to_path_buf());
    }
    let mut level = vec![root.to_path_buf()];
    for _ in 0..2 {
        let mut next = Vec::new();
        for dir in &level {
            let Ok(rd) = std::fs::read_dir(dir) else {
                continue;
            };
            let mut subs: Vec<PathBuf> = rd
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .collect();
            subs.sort();
            for sub in subs {
                if sub.join("gore-mod.json").is_file() {
                    return Some(sub);
                }
                next.push(sub);
            }
        }
        level = next;
    }
    None
}

/// Map a goremod manifest to library components, reading each payload to extract its targets.
/// `prefix` is the bundle dir's path relative to the entry root (rels must resolve from there);
/// `raw` is the manifest's raw JSON, used for fields the current [`Component`] doesn't carry.
fn goremod_components(
    bundle_dir: &Path,
    prefix: &str,
    manifest: &ModManifest,
    raw: &serde_json::Value,
    limits: ImportLimits,
) -> crate::Result<Vec<ComponentInfo>> {
    let raw_comps = raw.get("components").and_then(|v| v.as_array());
    let mut out = Vec::new();
    for (i, comp) in manifest.components.iter().enumerate() {
        // The manifest may come from an untrusted archive: no path may escape the entry dir.
        // (`..` patterns keep this compiling if variants grow extra fields.)
        let comp_path = match comp {
            Component::Ue4ssLua { path, .. }
            | Component::LocPatch { path, .. }
            | Component::AudioPatch { path, .. }
            | Component::TexturePatch { path, .. }
            | Component::AngelScriptPatch { path, .. }
            | Component::FilePatch { path, .. }
            | Component::PakFilePatch { path, .. }
            | Component::VoiceArchivePatch { path, .. } => path,
        };
        if !crate::is_safe_rel_path(comp_path) {
            return Err(ModError::Other(format!(
                "unsafe component path in gore-mod.json: {comp_path:?}"
            )));
        }
        out.push(match comp {
            Component::LocPatch { path, .. } => {
                let bytes = read_bounded_bundle_file(
                    bundle_dir,
                    Path::new(path),
                    "loc edits",
                    limits.max_manifest_bytes,
                )?;
                let edits: BTreeMap<String, BTreeMap<String, String>> =
                    serde_json::from_slice(&bytes)?;
                let mut targets: Vec<String> = edits
                    .iter()
                    .flat_map(|(id, sets)| sets.keys().map(move |set| format!("{id}|{set}")))
                    .collect();
                targets.sort();
                ComponentInfo::LocPatch {
                    rel: join_rel(prefix, path),
                    targets,
                }
            }
            Component::AudioPatch { path, .. } => {
                let manifest_path = Path::new(path).join("manifest.json");
                let bytes = read_bounded_bundle_file(
                    bundle_dir,
                    &manifest_path,
                    "audio manifest",
                    limits.max_manifest_bytes,
                )?;
                let map: BTreeMap<String, BTreeMap<String, String>> =
                    serde_json::from_slice(&bytes)?;
                let mut targets: Vec<String> = map
                    .iter()
                    .flat_map(|(bank, samples)| samples.keys().map(move |s| format!("{bank}|{s}")))
                    .collect();
                targets.sort();
                ComponentInfo::AudioPatch {
                    rel: join_rel(prefix, path),
                    targets,
                }
            }
            Component::TexturePatch { path, assets, .. } => {
                let mut targets = assets.clone();
                targets.sort();
                ComponentInfo::TexturePatch {
                    rel: join_rel(prefix, path),
                    targets,
                }
            }
            Component::AngelScriptPatch { path, .. } => {
                let manifest_path = Path::new(path).join("manifest.json");
                let bytes = read_bounded_bundle_file(
                    bundle_dir,
                    &manifest_path,
                    "script manifest",
                    limits.max_manifest_bytes,
                )?;
                let entries: Vec<ScriptEntry> = serde_json::from_slice(&bytes)?;
                let mut targets: Vec<String> = entries.iter().map(|e| e.module.clone()).collect();
                targets.sort();
                ComponentInfo::AngelScriptPatch {
                    rel: join_rel(prefix, path),
                    targets,
                }
            }
            // Both destinations come from the payload manifest deploy actually reads, checked
            // against the component's own declaration. The allowlist still runs on both, so an
            // archive cannot smuggle a destination past import through either door.
            Component::FilePatch { path, targets } => ComponentInfo::FilePatch {
                rel: join_rel(prefix, path),
                targets: loose_component_targets(
                    bundle_dir,
                    path,
                    targets,
                    "loose file manifest",
                    limits,
                )?,
            },
            Component::PakFilePatch { path, targets } => ComponentInfo::PakFilePatch {
                rel: join_rel(prefix, path),
                targets: loose_component_targets(
                    bundle_dir,
                    path,
                    targets,
                    "pak file manifest",
                    limits,
                )?,
            },
            Component::VoiceArchivePatch { path } => {
                let manifest_path = Path::new(path).join("manifest.json");
                let bytes = read_bounded_bundle_file(
                    bundle_dir,
                    &manifest_path,
                    "voice manifest",
                    limits.max_manifest_bytes,
                )?;
                let voice: VoicePatchManifest = serde_json::from_slice(&bytes)?;
                crate::validate_voice_manifest(&voice)?;
                if voice.edits.len() > limits.max_zip_entries {
                    return Err(ModError::Other(format!(
                        "voice manifest edit count limit exceeded: {} > {}",
                        voice.edits.len(),
                        limits.max_zip_entries
                    )));
                }
                let voice_limits = gore_vo::Limits::default();
                let mut targets = BTreeMap::<String, String>::new();
                let mut total_ogg_bytes = 0u64;
                for edit in &voice.edits {
                    gore_vo::validate_archive_entry_path(&edit.archive, &voice_limits).map_err(
                        |error| {
                            ModError::Voice(format!(
                                "unsafe voice archive name {:?}: {error}",
                                edit.archive
                            ))
                        },
                    )?;
                    gore_vo::validate_archive_entry_path(&edit.archive_path, &voice_limits)
                        .map_err(|error| {
                            ModError::Voice(format!(
                                "unsafe voice archive member {:?}: {error}",
                                edit.archive_path
                            ))
                        })?;
                    let ogg = read_bounded_bundle_file(
                        bundle_dir,
                        Path::new(&edit.ogg),
                        "voice Ogg payload",
                        limits.max_voice_ogg_bytes,
                    )?;
                    total_ogg_bytes =
                        total_ogg_bytes
                            .checked_add(ogg.len() as u64)
                            .ok_or_else(|| {
                                ModError::Other("voice Ogg payload byte count overflowed".into())
                            })?;
                    check_import_limit(
                        "voice Ogg payload total bytes",
                        total_ogg_bytes,
                        limits.max_voice_ogg_total_bytes,
                    )?;
                    gore_vo::validate_ogg(&ogg, &voice_limits)
                        .map_err(|e| ModError::Voice(format!("{}: {e}", edit.ogg)))?;
                    let target = format!("{}|{}", edit.archive, edit.archive_path);
                    targets.insert(target.replace('\\', "/").to_lowercase(), target);
                }
                ComponentInfo::VoiceArchivePatch {
                    rel: join_rel(prefix, path),
                    targets: targets.into_values().collect(),
                }
            }
            Component::Ue4ssLua {
                name,
                path,
                targets,
                opaque,
            } => {
                // Old manifests had no `opaque` field. Keep their empty target list conservative,
                // while an explicitly authored true/false value round-trips exactly.
                let mut targets = targets.clone();
                targets.sort();
                targets.dedup();
                let has_explicit_opaque = raw_comps
                    .and_then(|components| components.get(i))
                    .is_some_and(|component| component.get("opaque").is_some());
                let opaque = if has_explicit_opaque {
                    *opaque
                } else {
                    targets.is_empty()
                };
                ComponentInfo::Ue4ssLua {
                    name: name.clone(),
                    rel: join_rel(prefix, path),
                    targets,
                    opaque,
                }
            }
        });
    }
    Ok(out)
}

/// A loose-file component's destinations, read from the payload manifest inside the bundle rather
/// than believed from the component's own `targets` list.
///
/// Those two can disagree, and only one of them is what deploy acts on: `apply` reads
/// `<path>/manifest.json` and writes whatever it maps, while `mgr analyze` bucketed the declared
/// list. A bundle whose declaration is short — hand-edited, or written by a tool with a bug — was
/// therefore reported as claiming nothing at a path it then silently won at apply time, and the
/// user was told the loadout was conflict-free.
///
/// The declared list is still validated first, so a destination smuggled in there is refused with
/// the same allowlist error as before rather than being quietly ignored; then the two are required
/// to agree. Refusing the mismatch outright is the only honest option, because the disagreement
/// means the bundle does not describe what it does, and picking either side would be a guess about
/// which half is the mistake.
fn loose_component_targets(
    bundle_dir: &Path,
    path: &str,
    declared: &[String],
    label: &'static str,
    limits: ImportLimits,
) -> crate::Result<Vec<String>> {
    for target in declared {
        crate::validate_loose_game_path(target)?;
    }

    let manifest_path = Path::new(path).join("manifest.json");
    let bytes =
        read_bounded_bundle_file(bundle_dir, &manifest_path, label, limits.max_manifest_bytes)?;
    let map: BTreeMap<String, String> = serde_json::from_slice(&bytes)?;
    let mut actual: Vec<String> = map.keys().cloned().collect();
    for target in &actual {
        crate::validate_loose_game_path(target)?;
    }
    actual.sort();
    actual.dedup();

    let mut stated: Vec<String> = declared.to_vec();
    stated.sort();
    stated.dedup();
    if stated != actual {
        return Err(ModError::Other(format!(
            "the {label} and the component's declared targets disagree: the manifest maps {actual:?} \
             but the component claims {stated:?}"
        )));
    }

    Ok(actual)
}

/// Read one regular bundle file through a hard byte cap. Metadata is checked before opening, and
/// `take(limit + 1)` keeps a concurrent growth race from allocating beyond the configured limit.
fn read_bounded_bundle_file(
    bundle_root: &Path,
    rel: &Path,
    label: &str,
    max_bytes: u64,
) -> crate::Result<Vec<u8>> {
    // `rel` may have been assembled internally with `Path::join`, which uses backslashes on
    // Windows. Validate its portable representation; untrusted manifest strings themselves were
    // already validated before conversion to `Path`, so authored backslashes remain forbidden.
    let rel_text = portable_import_rel_path(rel, &bundle_root.join(rel))?;
    if !crate::is_safe_rel_path(&rel_text) {
        return Err(ModError::Other(format!("unsafe {label} path {rel_text:?}")));
    }
    let root = open_directory_nofollow(bundle_root, "bundle root")?;
    let mut file = root.open_relative_file(rel, label)?;
    check_import_limit(label, file.len(), max_bytes)?;
    let expected = file.len();
    let capacity = usize::try_from(expected)
        .map_err(|_| ModError::Other(format!("{label} exceeds process address space")))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| ModError::Other(format!("could not reserve memory for {label}")))?;
    std::io::Read::by_ref(&mut file.file)
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(crate::io(&format!(
            "reading {label} {}",
            file.path().display()
        )))?;
    check_import_limit(label, bytes.len() as u64, max_bytes)?;
    if bytes.len() as u64 != expected {
        return Err(ModError::Other(format!(
            "{label} changed while being read through its opened handle: {}",
            file.path().display()
        )));
    }
    file.verify_len(expected, label)?;
    Ok(bytes)
}

fn import_metadata_is_link(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;

        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// Walk the staged tree and collect foreign components (deterministic: sorted per dir).
fn scan_foreign(root: &Path) -> crate::Result<Vec<ComponentInfo>> {
    let mut out = Vec::new();
    scan_dir(root, root, 0, &mut out)?;
    Ok(out)
}

/// Deepest directory nesting `scan_dir` will descend into. Real mods are shallow; a cap here
/// bounds the recursion so a symlink loop (or a maliciously deep archive) can't recurse forever
/// / overflow the stack — past the cap we just stop descending (files already at that depth are
/// still classified).
const MAX_SCAN_DEPTH: usize = 16;

fn scan_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    out: &mut Vec<ComponentInfo>,
) -> crate::Result<()> {
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)
        .map_err(crate::io(&format!("scanning {}", dir.display())))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if path.join("Scripts").join("main.lua").is_file() {
                // A UE4SS Lua mod dir is one opaque component; don't scan inside it.
                let name = e.file_name().to_string_lossy().into_owned();
                out.push(ComponentInfo::Ue4ssLua {
                    name,
                    rel: rel_str(root, &path),
                    targets: Vec::new(),
                    opaque: true,
                });
            } else if depth < MAX_SCAN_DEPTH {
                // Stop descending past the cap — a symlink loop would otherwise recurse forever.
                scan_dir(root, &path, depth + 1, out)?;
            }
        } else if ft.is_file() {
            classify_file(root, &path, out);
        }
    }
    Ok(())
}

/// Classify one foreign file into a component (or nothing). Target extraction is best-effort:
/// an unparsable container still imports, just with an empty (unknown) footprint.
fn classify_file(root: &Path, path: &Path, out: &mut Vec<ComponentInfo>) {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let lower = name.to_ascii_lowercase();
    let rel = rel_str(root, path);
    if lower.starts_with("precompiledscript") && lower.ends_with(".cache") {
        out.push(ComponentInfo::RawFile {
            rel,
            target_file: RawTarget::ScriptCache,
        });
    } else if lower.ends_with(".lcache") {
        out.push(ComponentInfo::RawFile {
            rel,
            target_file: RawTarget::Lcache,
        });
    } else if lower.ends_with(".bank") {
        out.push(ComponentInfo::RawFile {
            rel,
            target_file: RawTarget::Bank {
                name: name.to_string(),
            },
        });
    } else if lower.ends_with(".utoc") {
        // Only a complete pair is mountable; a lone .utoc is not importable on its own.
        if path.with_extension("ucas").is_file() {
            let targets = gore_tex::container::list_packages(path).unwrap_or_default();
            out.push(ComponentInfo::Triplet {
                rel_base: rel_str(root, &path.with_extension("")),
                targets,
            });
        }
    } else if lower.ends_with("_p.pak") && !path.with_extension("utoc").is_file() {
        // A pak WITH a sibling .utoc belongs to that triplet, not to a loose-pak component.
        //
        // Read through the mount point, because the conflict namespace these targets land in is
        // game-root-relative and a pak index is not: UnrealPak folds a common leading directory
        // into the mount point, so a cursor pak names its entry `Normal.PNG` while it claims
        // `G1R/Content/Slate/Cursors/Normal/Normal.PNG`. Comparing the raw index against real
        // destinations misses the overlap that matters and invents ones between two paks that
        // merely share a leaf name.
        let targets =
            gore_tex::container::list_pak_files_from_game_root(path).unwrap_or_default();
        out.push(ComponentInfo::LoosePak { rel, targets });
    }
}

/// Kind for a foreign import: the single component class, or Mixed for ≥2 classes.
fn foreign_kind(components: &[ComponentInfo]) -> ModKind {
    let mut classes = std::collections::BTreeSet::new();
    for c in components {
        classes.insert(match c {
            ComponentInfo::Triplet { .. } => 0u8,
            ComponentInfo::LoosePak { .. } => 1,
            ComponentInfo::Ue4ssLua { .. } => 2,
            ComponentInfo::RawFile { .. } => 3,
            _ => 4, // goremod-only shapes never come from the foreign scan
        });
    }
    if classes.len() != 1 {
        return ModKind::ForeignMixed;
    }
    match classes.into_iter().next().unwrap() {
        0 => ModKind::ForeignTriplet,
        1 => ModKind::ForeignPak,
        2 => ModKind::ForeignUe4ss,
        3 => ModKind::ForeignRawfile,
        _ => ModKind::ForeignMixed,
    }
}

// ── Small helpers ───────────────────────────────────────────────────────────

/// `p` relative to `root` as a '/'-separated string (entry-relative component paths).
fn rel_str(root: &Path, p: &Path) -> String {
    match p.strip_prefix(root) {
        Ok(rel) => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => p.display().to_string(),
    }
}

/// Join a bundle-dir prefix (may be "") and a manifest-relative path with '/'.
fn join_rel(prefix: &str, path: &str) -> String {
    let norm = path.replace('\\', "/");
    if prefix.is_empty() {
        norm
    } else {
        format!("{prefix}/{norm}")
    }
}

/// Lowercase alnum+`-` slug of a mod name for the library id (never empty).
fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "mod".into()
    } else {
        out
    }
}

/// `secs` since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ` (UTC, RFC 3339) — std-only.
fn format_utc(secs: i64, micros: u32) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days);
    // Microsecond precision matters: `imported_at` folds into the entry fingerprint, so a re-import
    // within the same SECOND (identical component descriptors, only changed payload bytes) must
    // still get a distinct timestamp — otherwise mgr_status could report InSync over changed bytes.
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{micros:06}Z")
}

/// Days since 1970-01-01 → (year, month, day). Howard Hinnant's `civil_from_days`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

/// Removes the staging dir on drop unless defused (`.0 = None`) — covers every failure path.
struct StagingGuard(Option<PathBuf>);

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if let Some(p) = self.0.take() {
            let _ = std::fs::remove_dir_all(&p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build_bundle, write_bundle, BuildSpec, LooseFileReplacement, ModMeta, ScriptModule,
        VoiceArchiveEdit, VoicePatchOp,
    };
    use gore_modgen::gen::{OverrideValue, SingleOverride};
    use std::fs;

    #[cfg(unix)]
    fn make_file_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn make_file_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("creating test file symlink failed: {error}"),
        }
    }

    #[cfg(unix)]
    fn make_dir_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn make_dir_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_dir(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("creating test directory symlink failed: {error}"),
        }
    }

    /// Build + write a real goremod bundle (override + loc + script + voice) and
    /// return its dir. The name deliberately has a space: id slugging must handle it.
    fn mk_goremod_bundle(root: &Path) -> PathBuf {
        let mini = root.join("TestModule.mini.cache");
        fs::write(&mini, b"FAKE-MINI-CACHE-BYTES").unwrap();
        let ogg = root.join("hello.ogg");
        fs::write(&ogg, crate::tests::test_ogg(44_100)).unwrap();
        let mut loc = BTreeMap::new();
        loc.insert(
            "itfo_cheese".to_string(),
            BTreeMap::from([("german".to_string(), "X".to_string())]),
        );
        let spec = BuildSpec {
            meta: ModMeta {
                name: "Target Probe".into(),
                version: "0.9".into(),
                author: "tester".into(),
            },
            delay_ms: 0,
            overrides: vec![SingleOverride {
                class: "ItFo_Apple".into(),
                field: "m_Value".into(),
                module: "Angelscript".into(),
                value: OverrideValue::Int(500),
            }],
            loc_edits: loc,
            audio: vec![],
            texture: vec![],
            files: vec![],
            pak_files: vec![],
            scripts: vec![ScriptModule {
                op: "add".into(),
                module_name: "TestModule".into(),
                mini_cache: mini.display().to_string(),
            }],
            dialog_topics: vec![],
            voice: vec![VoiceArchiveEdit {
                archive: "German.zip".into(),
                op: VoicePatchOp::Replace,
                archive_path: "NPC/Hero/hello.ogg".into(),
                ogg_path: ogg.display().to_string(),
                observation: None,
            }],
        };
        let bundle = build_bundle(&spec).unwrap();
        let bdir = root.join("Target Probe");
        write_bundle(&bdir, &bundle).unwrap();
        bdir
    }

    /// The library sidecar has to carry a loose-file component's DESTINATIONS, because that is the
    /// only thing conflict analysis and apply can key off. The second half is the point of the
    /// test: the manifest is authored data, so an archive that names a destination the deploy
    /// record would refuse must be caught at import, not at apply — by then a user has already
    /// built a loadout around it.
    #[test]
    fn import_goremod_file_patch_keeps_targets_and_refuses_a_forbidden_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let cursor = tmp.path().join("Normal.PNG");
        fs::write(&cursor, b"CURSOR-BYTES").unwrap();
        let spec = BuildSpec {
            meta: ModMeta {
                name: "LooseProbe".into(),
                version: "1".into(),
                author: "tester".into(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![LooseFileReplacement {
                game_path: "G1R/Content/Slate/Cursors/Normal/Normal.PNG".into(),
                source_path: cursor.display().to_string(),
            }],
            pak_files: vec![],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        };
        let bdir = tmp.path().join("LooseProbe");
        write_bundle(&bdir, &build_bundle(&spec).unwrap()).unwrap();

        let meta = import(&lib, &bdir).unwrap();
        assert!(
            meta.components.iter().any(|c| matches!(
                c,
                ComponentInfo::FilePatch { rel, targets }
                    if rel == "files"
                        && targets == &vec!["G1R/Content/Slate/Cursors/Normal/Normal.PNG".to_string()]
            )),
            "components: {:?}",
            meta.components
        );

        let manifest_path = bdir.join("gore-mod.json");
        let tampered = String::from_utf8(fs::read(&manifest_path).unwrap())
            .unwrap()
            .replace(
                "G1R/Content/Slate/Cursors/Normal/Normal.PNG",
                "G1R/Binaries/Win64/G1R-Win64-Shipping.exe",
            );
        fs::write(&manifest_path, tampered).unwrap();
        // A fresh library so the refusal cannot be confused with an update-path failure.
        let error = import(&tmp.path().join("lib-tampered"), &bdir)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("not a replaceable game file"),
            "unexpected error: {error}"
        );
    }

    /// A folder import that IS the library dir — or a parent that contains it — must be rejected
    /// up front. Otherwise the staging dir (created under the library) lands inside the source and
    /// the recursive copy would copy staging into itself until the filesystem errors.
    #[test]
    fn rejects_importing_the_library_or_a_containing_parent() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        fs::create_dir_all(&lib).unwrap();

        // Source == the library dir itself.
        let err = import(&lib, &lib).unwrap_err().to_string();
        assert!(
            err.contains("manager library directory"),
            "unexpected error: {err}"
        );

        // Source == a parent that contains the library dir.
        let err = import(&lib, tmp.path()).unwrap_err().to_string();
        assert!(
            err.contains("manager library directory"),
            "unexpected error: {err}"
        );

        // Sanity: a normal sibling folder next to the library still imports fine.
        let bdir = mk_goremod_bundle(tmp.path());
        assert!(
            import(&lib, &bdir).is_ok(),
            "a sibling source must still import"
        );
    }

    /// Zip every file under `dir` (names relative to `dir`, '/'-separated), each entry name
    /// prefixed with `prefix` (empty = zip root).
    fn zip_dir_with_prefix(dir: &Path, prefix: &str, zip_path: &Path) {
        fn add(zw: &mut zip::ZipWriter<fs::File>, root: &Path, dir: &Path, prefix: &str) {
            let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap()).collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                let p = e.path();
                if p.is_dir() {
                    add(zw, root, &p, prefix);
                } else {
                    let rel = p
                        .strip_prefix(root)
                        .unwrap()
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/");
                    let name = if prefix.is_empty() {
                        rel
                    } else {
                        format!("{prefix}/{rel}")
                    };
                    zw.start_file(name, zip::write::SimpleFileOptions::default())
                        .unwrap();
                    zw.write_all(&fs::read(&p).unwrap()).unwrap();
                }
            }
        }
        let mut zw = zip::ZipWriter::new(fs::File::create(zip_path).unwrap());
        add(&mut zw, dir, dir, prefix);
        zw.finish().unwrap();
    }

    fn zip_entries(zip_path: &Path, entries: &[(&str, &[u8], zip::CompressionMethod)]) {
        let mut writer = zip::ZipWriter::new(fs::File::create(zip_path).unwrap());
        for (name, bytes, method) in entries {
            let options = zip::write::SimpleFileOptions::default().compression_method(*method);
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    fn assert_failed_import_left_nothing(library: &Path) {
        assert!(list(library).unwrap().is_empty());
        let leftovers = fs::read_dir(library)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert!(
            leftovers.is_empty(),
            "failed import left staging/library artifacts: {leftovers:?}"
        );
    }

    #[test]
    fn zip_resource_limits_preflight_all_entries_and_leave_no_partial_import() {
        let temp = tempfile::tempdir().unwrap();
        let stored = zip::CompressionMethod::Stored;

        let count_zip = temp.path().join("too-many.zip");
        zip_entries(
            &count_zip,
            &[("a.bin", b"a", stored), ("b.bin", b"b", stored)],
        );
        let count_lib = temp.path().join("count-lib");
        let error = import_with_limits(
            &count_lib,
            &count_zip,
            ImportLimits {
                max_zip_entries: 1,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("entry count limit"),
            "unexpected error: {error}"
        );
        assert_failed_import_left_nothing(&count_lib);

        // The first member is valid and the second exceeds the cap. Whole-ZIP preflight must reject
        // before extracting the first member, and StagingGuard must remove the staging directory.
        let entry_zip = temp.path().join("entry-too-large.zip");
        zip_entries(
            &entry_zip,
            &[
                ("first.bin", b"ok", stored),
                ("later.bin", b"12345", stored),
            ],
        );
        let entry_lib = temp.path().join("entry-lib");
        let error = import_with_limits(
            &entry_lib,
            &entry_zip,
            ImportLimits {
                max_zip_entry_uncompressed_bytes: 4,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("entry uncompressed bytes limit"),
            "unexpected error: {error}"
        );
        assert_failed_import_left_nothing(&entry_lib);

        let total_zip = temp.path().join("total-too-large.zip");
        zip_entries(
            &total_zip,
            &[("one.bin", b"1234", stored), ("two.bin", b"5678", stored)],
        );
        let total_lib = temp.path().join("total-lib");
        let error = import_with_limits(
            &total_lib,
            &total_zip,
            ImportLimits {
                max_zip_entry_uncompressed_bytes: 4,
                max_zip_total_uncompressed_bytes: 7,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("total uncompressed bytes limit"),
            "unexpected error: {error}"
        );
        assert_failed_import_left_nothing(&total_lib);
    }

    #[test]
    fn folder_resource_limits_fail_closed_and_leave_no_partial_import() {
        let temp = tempfile::tempdir().unwrap();

        let count_source = temp.path().join("count-source");
        fs::create_dir(&count_source).unwrap();
        fs::write(count_source.join("a.lcache"), b"a").unwrap();
        fs::write(count_source.join("b.lcache"), b"b").unwrap();
        let count_library = temp.path().join("count-library");
        let error = import_with_limits(
            &count_library,
            &count_source,
            ImportLimits {
                max_zip_entries: 1,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("folder entry count limit"), "{error}");
        assert_failed_import_left_nothing(&count_library);

        let file_source = temp.path().join("file-source");
        fs::create_dir(&file_source).unwrap();
        fs::write(file_source.join("large.lcache"), b"12345").unwrap();
        let file_library = temp.path().join("file-library");
        let error = import_with_limits(
            &file_library,
            &file_source,
            ImportLimits {
                max_zip_entry_uncompressed_bytes: 4,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("folder entry bytes limit"), "{error}");
        assert_failed_import_left_nothing(&file_library);

        let total_source = temp.path().join("total-source");
        fs::create_dir(&total_source).unwrap();
        fs::write(total_source.join("one.lcache"), b"1234").unwrap();
        fs::write(total_source.join("two.lcache"), b"5678").unwrap();
        let total_library = temp.path().join("total-library");
        let error = import_with_limits(
            &total_library,
            &total_source,
            ImportLimits {
                max_zip_entry_uncompressed_bytes: 4,
                max_zip_total_uncompressed_bytes: 7,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("folder total bytes limit"), "{error}");
        assert_failed_import_left_nothing(&total_library);
    }

    #[test]
    fn single_file_and_triplet_limits_preflight_before_staging_writes() {
        let temp = tempfile::tempdir().unwrap();

        let oversized = temp.path().join("oversized_P.pak");
        fs::write(&oversized, b"12345").unwrap();
        let oversized_library = temp.path().join("oversized-library");
        let error = import_with_limits(
            &oversized_library,
            &oversized,
            ImportLimits {
                max_zip_entry_uncompressed_bytes: 4,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("single-file import entry bytes limit"),
            "{error}"
        );
        assert_failed_import_left_nothing(&oversized_library);

        let triplet = temp.path().join("pair_P.pak");
        fs::write(&triplet, b"123").unwrap();
        fs::write(triplet.with_extension("utoc"), b"456").unwrap();
        fs::write(triplet.with_extension("ucas"), b"789").unwrap();

        let count_library = temp.path().join("triplet-count-library");
        let error = import_with_limits(
            &count_library,
            &triplet,
            ImportLimits {
                max_zip_entries: 2,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("entry count limit"), "{error}");
        assert_failed_import_left_nothing(&count_library);

        // All three members pass the per-file cap; their 9-byte sum exceeds the 8-byte total.
        // Whole-set preflight rejects before even the selected `.pak` is copied.
        let total_library = temp.path().join("triplet-total-library");
        let error = import_with_limits(
            &total_library,
            &triplet,
            ImportLimits {
                max_zip_entry_uncompressed_bytes: 3,
                max_zip_total_uncompressed_bytes: 8,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("single-file import total bytes limit"),
            "{error}"
        );
        assert_failed_import_left_nothing(&total_library);

        let path_library = temp.path().join("single-path-library");
        let error = import_with_limits(
            &path_library,
            &oversized,
            ImportLimits {
                max_zip_path_bytes: 4,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("path bytes limit"), "{error}");
        assert_failed_import_left_nothing(&path_library);
    }

    #[cfg(any(unix, windows))]
    #[cfg_attr(
        windows,
        ignore = "requires Windows symbolic-link privilege; run explicitly on a privileged worker"
    )]
    #[test]
    fn single_file_import_rejects_root_and_sibling_links_without_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real_P.pak");
        fs::write(&real, b"pak").unwrap();
        let linked_root = temp.path().join("linked_P.pak");
        assert!(
            make_file_link(&real, &linked_root),
            "test requires symbolic-link creation support"
        );
        let root_library = temp.path().join("root-link-library");
        let error = import(&root_library, &linked_root).unwrap_err().to_string();
        assert!(
            error.contains("symbolic link") || error.contains("reparse point"),
            "{error}"
        );
        assert_failed_import_left_nothing(&root_library);

        let selected = temp.path().join("siblings_P.pak");
        fs::write(&selected, b"pak").unwrap();
        let sibling_target = temp.path().join("outside.utoc");
        fs::write(&sibling_target, b"utoc").unwrap();
        let linked_sibling = selected.with_extension("utoc");
        assert!(
            make_file_link(&sibling_target, &linked_sibling),
            "test requires symbolic-link creation support"
        );
        fs::write(selected.with_extension("ucas"), b"ucas").unwrap();
        let sibling_library = temp.path().join("sibling-link-library");
        let error = import(&sibling_library, &selected).unwrap_err().to_string();
        assert!(
            error.contains("regular non-link file")
                || error.contains("symbolic link")
                || error.contains("reparse point"),
            "{error}"
        );
        assert_failed_import_left_nothing(&sibling_library);
    }

    #[test]
    fn opened_handle_copy_detects_growth_or_denies_writer() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.pak");
        let destination = temp.path().join("staged.pak");
        fs::write(&source, b"1234").unwrap();

        let mut writer_was_denied = None;
        let result = copy_import_regular_file_with(&source, &destination, 4, 8, 8, || {
            match fs::OpenOptions::new().append(true).open(&source) {
                Ok(mut writer) => {
                    writer.write_all(b"5").unwrap();
                    writer.sync_all().unwrap();
                    writer_was_denied = Some(false);
                }
                Err(_) => writer_was_denied = Some(true),
            }
        });
        assert!(writer_was_denied.is_some(), "the growth hook must run");
        if writer_was_denied == Some(true) {
            #[cfg(not(windows))]
            panic!("Unix must permit and then detect the write");
            result.unwrap();
            assert_eq!(fs::read(&destination).unwrap(), b"1234");
        } else {
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("changed or exceeded")
                    || error.contains("changed identity/size/content revision"),
                "{error}"
            );
            assert!(!destination.exists(), "partial staged copy must be removed");
        }
    }

    #[test]
    fn opened_handle_copy_detects_same_size_mutation_or_denies_writer() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.pak");
        let destination = temp.path().join("staged.pak");
        fs::write(&source, b"1234").unwrap();

        let mut writer_was_denied = None;
        let result = copy_import_regular_file_with(&source, &destination, 4, 8, 8, || {
            match fs::OpenOptions::new().write(true).open(&source) {
                Ok(mut writer) => {
                    writer.write_all(b"abcd").unwrap();
                    writer.sync_all().unwrap();
                    writer_was_denied = Some(false);
                }
                Err(_) => writer_was_denied = Some(true),
            }
        });
        assert!(writer_was_denied.is_some(), "the mutation hook must run");
        if writer_was_denied == Some(true) {
            #[cfg(not(windows))]
            panic!("Unix must permit and then detect the write");
            result.unwrap();
            assert_eq!(fs::read(&destination).unwrap(), b"1234");
        } else {
            let error = result.unwrap_err().to_string();
            assert!(error.contains("content revision"), "{error}");
            assert!(!destination.exists(), "partial staged copy must be removed");
        }
    }

    #[cfg(any(unix, windows))]
    #[cfg_attr(
        windows,
        ignore = "requires Windows symbolic-link privilege; run explicitly on a privileged worker"
    )]
    #[test]
    fn folder_import_rejects_file_swapped_to_link_between_enumeration_and_open() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("race-source");
        let library = temp.path().join("race-library");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("safe.lcache"), b"safe").unwrap();
        let outside = temp.path().join("outside.lcache");
        fs::write(&outside, b"escaped").unwrap();
        let staged_link = temp.path().join("race-link");
        assert!(
            make_file_link(&outside, &staged_link),
            "test requires symbolic-link creation support"
        );

        crate::mgr::model::inject_open_child_race(move |enumerated_path| {
            assert_eq!(
                enumerated_path.file_name(),
                Some(std::ffi::OsStr::new("safe.lcache"))
            );
            fs::remove_file(enumerated_path).unwrap();
            fs::rename(&staged_link, enumerated_path).unwrap();
        });
        let error = import(&library, &source).unwrap_err().to_string();
        assert!(
            error.contains("symbolic link")
                || error.contains("reparse point")
                || error.contains("without following"),
            "{error}"
        );
        assert_failed_import_left_nothing(&library);
    }

    #[cfg(any(unix, windows))]
    #[cfg_attr(
        windows,
        ignore = "requires Windows symbolic-link privilege; run explicitly on a privileged worker"
    )]
    #[test]
    fn folder_import_rejects_symbolic_link_or_reparse_descendants() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("linked-source");
        let outside = temp.path().join("outside");
        let library = temp.path().join("library");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(source.join("top.lcache"), b"recognized").unwrap();
        fs::write(outside.join("escaped.lcache"), b"must not copy").unwrap();
        assert!(
            make_dir_link(&outside, &source.join("linked")),
            "test requires symbolic-link creation support"
        );

        let error = import(&library, &source).unwrap_err().to_string();
        assert!(
            error.contains("symbolic link or reparse point")
                || error.contains("Too many levels of symbolic links")
                || error.contains("without following"),
            "unexpected error: {error}"
        );
        assert_failed_import_left_nothing(&library);

        let linked_root = temp.path().join("linked-root");
        assert!(
            make_dir_link(&outside, &linked_root),
            "test requires symbolic-link creation support"
        );
        let root_library = temp.path().join("root-library");
        let error = import(&root_library, &linked_root).unwrap_err().to_string();
        assert!(
            error.contains("root is not a real directory"),
            "unexpected root-link error: {error}"
        );
        assert_failed_import_left_nothing(&root_library);
    }

    #[test]
    fn zip_bomb_ratio_is_rejected_before_import_activation() {
        let temp = tempfile::tempdir().unwrap();
        let zip_path = temp.path().join("bomb.zip");
        let bomb = vec![0u8; 16 * 1024];
        zip_entries(
            &zip_path,
            &[
                ("safe.bin", b"safe", zip::CompressionMethod::Stored),
                ("bomb.bin", &bomb, zip::CompressionMethod::Deflated),
            ],
        );
        let library = temp.path().join("lib");
        let error = import_with_limits(
            &library,
            &zip_path,
            ImportLimits {
                max_zip_compression_ratio: 2,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("compression ratio limit"),
            "unexpected error: {error}"
        );
        assert_failed_import_left_nothing(&library);
    }

    #[test]
    fn manifest_and_voice_ogg_reads_obey_hard_limits_without_activation() {
        let temp = tempfile::tempdir().unwrap();
        let bundle = mk_goremod_bundle(temp.path());

        let manifest_library = temp.path().join("manifest-lib");
        let error = import_with_limits(
            &manifest_library,
            &bundle,
            ImportLimits {
                max_manifest_bytes: 8,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("gore-mod.json limit"),
            "unexpected error: {error}"
        );
        assert_failed_import_left_nothing(&manifest_library);

        let ogg_library = temp.path().join("ogg-lib");
        let error = import_with_limits(
            &ogg_library,
            &bundle,
            ImportLimits {
                max_voice_ogg_bytes: 8,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("voice Ogg payload limit"),
            "unexpected error: {error}"
        );
        assert_failed_import_left_nothing(&ogg_library);
    }

    #[test]
    fn rejects_unsupported_goremod_manifest_format_before_activation() {
        let temp = tempfile::tempdir().unwrap();
        let bundle = mk_goremod_bundle(temp.path());
        let manifest_path = bundle.join("gore-mod.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["format"] = serde_json::json!(2);
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let library = temp.path().join("lib");
        let error = import(&library, &bundle).unwrap_err().to_string();
        assert!(
            error.contains("unsupported gore-mod.json format 2"),
            "unexpected error: {error}"
        );
        assert_failed_import_left_nothing(&library);
    }

    fn assert_goremod_components(meta: &ModEntryMeta, want_prefix: &str) {
        let pre = |s: &str| {
            if want_prefix.is_empty() {
                s.to_string()
            } else {
                format!("{want_prefix}/{s}")
            }
        };
        let (mut saw_loc, mut saw_as, mut saw_lua, mut saw_voice) = (false, false, false, false);
        for c in &meta.components {
            match c {
                ComponentInfo::LocPatch { rel, targets } => {
                    saw_loc = true;
                    assert_eq!(rel, &pre("loc/edits.json"));
                    assert_eq!(targets, &vec!["itfo_cheese|german".to_string()]);
                }
                ComponentInfo::AngelScriptPatch { rel, targets } => {
                    saw_as = true;
                    assert_eq!(rel, &pre("scripts"));
                    assert_eq!(targets, &vec!["TestModule".to_string()]);
                }
                ComponentInfo::Ue4ssLua {
                    name,
                    rel,
                    targets,
                    opaque,
                } => {
                    saw_lua = true;
                    assert_eq!(name, "Target Probe");
                    assert_eq!(rel, &pre("ue4ss/Target Probe"));
                    assert_eq!(targets, &["ItFo_Apple.m_Value"]);
                    assert!(!*opaque, "ordinary generated override metadata is precise");
                }
                ComponentInfo::VoiceArchivePatch { rel, targets } => {
                    saw_voice = true;
                    assert_eq!(rel, &pre("voice"));
                    assert_eq!(targets, &vec!["German.zip|NPC/Hero/hello.ogg".to_string()]);
                }
                other => panic!("unexpected component in goremod import: {other:?}"),
            }
        }
        assert!(
            saw_loc && saw_as && saw_lua && saw_voice,
            "missing components: {:?}",
            meta.components
        );
    }

    /// [import 1] A goremod bundle DIR imports as kind Goremod with manifest meta and
    /// per-component targets extracted from the payload files.
    #[test]
    fn import_goremod_bundle_dir_extracts_targets() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bdir = mk_goremod_bundle(tmp.path());

        let meta = import(&lib, &bdir).unwrap();
        assert_eq!(meta.kind, ModKind::Goremod);
        assert_eq!(meta.name, "Target Probe");
        assert_eq!(meta.version, "0.9");
        assert_eq!(meta.author, "tester");
        assert_eq!(meta.source, "Target Probe");
        assert!(
            meta.id.starts_with("target-probe-") && meta.id.len() == "target-probe-".len() + 8,
            "id: {}",
            meta.id
        );
        assert_goremod_components(&meta, "");

        // The entry dir holds the payload + sidecar; list() round-trips the same meta.
        let entry = lib.join(&meta.id);
        assert!(entry.join(META_FILE).is_file());
        assert!(entry.join("gore-mod.json").is_file());
        assert!(entry.join("loc").join("edits.json").is_file());
        assert_eq!(list(&lib).unwrap(), vec![meta]);
    }

    #[test]
    fn import_roundtrips_explicit_opaque_with_known_targets() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bundle = mk_goremod_bundle(tmp.path());
        let manifest_path = bundle.join("gore-mod.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        let lua = manifest["components"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|component| component["type"] == "ue4ss_lua")
            .unwrap();
        lua["opaque"] = serde_json::Value::Bool(true);
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let meta = import(&lib, &bundle).unwrap();
        assert!(matches!(
            meta.components.iter().find(|component| matches!(
                component,
                ComponentInfo::Ue4ssLua { .. }
            )),
            Some(ComponentInfo::Ue4ssLua {
                targets,
                opaque: true,
                ..
            }) if targets == &["ItFo_Apple.m_Value"]
        ));
        let persisted: ModEntryMeta =
            serde_json::from_slice(&fs::read(lib.join(&meta.id).join(META_FILE)).unwrap()).unwrap();
        assert_eq!(persisted, meta);
    }

    #[test]
    fn import_preserves_explicit_precise_targetless_lua() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bundle = mk_goremod_bundle(tmp.path());
        let manifest_path = bundle.join("gore-mod.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        let lua = manifest["components"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|component| component["type"] == "ue4ss_lua")
            .unwrap();
        lua["targets"] = serde_json::json!([]);
        lua["opaque"] = serde_json::Value::Bool(false);
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let meta = import(&lib, &bundle).unwrap();
        assert!(matches!(
            meta.components.iter().find(|component| matches!(
                component,
                ComponentInfo::Ue4ssLua { .. }
            )),
            Some(ComponentInfo::Ue4ssLua {
                targets,
                opaque: false,
                ..
            }) if targets.is_empty()
        ));
    }

    #[test]
    fn import_legacy_targetless_lua_stays_conservatively_opaque() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bundle = mk_goremod_bundle(tmp.path());
        let manifest_path = bundle.join("gore-mod.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        let lua = manifest["components"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|component| component["type"] == "ue4ss_lua")
            .and_then(serde_json::Value::as_object_mut)
            .unwrap();
        lua.remove("targets");
        lua.remove("opaque");
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let meta = import(&lib, &bundle).unwrap();
        assert!(matches!(
            meta.components.iter().find(|component| matches!(
                component,
                ComponentInfo::Ue4ssLua { .. }
            )),
            Some(ComponentInfo::Ue4ssLua {
                targets,
                opaque: true,
                ..
            }) if targets.is_empty()
        ));
    }

    #[test]
    fn import_rejects_bad_voice_manifest_and_payload_before_activation() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bdir = mk_goremod_bundle(tmp.path());
        let manifest_path = bdir.join("voice/manifest.json");
        let mut manifest: crate::VoicePatchManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest.format = 2;
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = import(&lib, &bdir).unwrap_err().to_string();
        assert!(error.contains("format 2"), "unexpected error: {error}");
        assert!(list(&lib).unwrap().is_empty());

        let bdir = mk_goremod_bundle(tmp.path());
        let manifest: crate::VoicePatchManifest =
            serde_json::from_slice(&fs::read(bdir.join("voice/manifest.json")).unwrap()).unwrap();
        fs::write(bdir.join(&manifest.edits[0].ogg), b"not an Ogg stream").unwrap();
        let error = import(&lib, &bdir).unwrap_err().to_string();
        assert!(error.contains("voice archive"), "unexpected error: {error}");
        assert!(list(&lib).unwrap().is_empty());
    }

    #[test]
    fn import_reuses_portable_voice_archive_path_validation() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bundle = mk_goremod_bundle(tmp.path());
        let manifest_path = bundle.join("voice/manifest.json");
        let original: crate::VoicePatchManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        let valid_archive = original.edits[0].archive.clone();
        let valid_member = original.edits[0].archive_path.clone();
        let overlong_member = format!("{}.ogg", "a".repeat(1_021));
        let cases = [
            ("COM¹.zip".to_string(), valid_member.clone()),
            ("LPT³.zip".to_string(), valid_member.clone()),
            (valid_archive.clone(), "NPC/COM¹.ogg".to_string()),
            (valid_archive.clone(), "CLOCK$/line.ogg".to_string()),
            (valid_archive.clone(), "CONIN$/line.ogg".to_string()),
            (valid_archive.clone(), "CONOUT$/line.ogg".to_string()),
            (valid_archive.clone(), "NPC/name?.ogg".to_string()),
            (valid_archive, overlong_member),
        ];

        for (index, (archive, archive_path)) in cases.into_iter().enumerate() {
            let mut manifest = original.clone();
            manifest.edits[0].archive = archive.clone();
            manifest.edits[0].archive_path = archive_path.clone();
            fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

            let error = import(&lib, &bundle).unwrap_err().to_string();
            assert!(
                error.contains("unsafe voice archive"),
                "case {index} ({archive:?}, {archive_path:?}) returned {error}"
            );
            assert_failed_import_left_nothing(&lib);
        }
    }

    /// [import 2] The SAME bundle zipped (manifest at zip root) imports to the same CONTENT as the
    /// dir (kind, name, components). The id now differs because it folds in the source name (dir
    /// "Target Probe" vs "Target Probe.zip") — so a dir and its zip are treated as two distinct
    /// sources, which lets both coexist rather than clobbering each other.
    #[test]
    fn import_zip_bundle_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let bdir = mk_goremod_bundle(tmp.path());
        let from_dir = import(&tmp.path().join("lib-a"), &bdir).unwrap();

        let zp = tmp.path().join("Target Probe.zip");
        zip_dir_with_prefix(&bdir, "", &zp);
        let from_zip = import(&tmp.path().join("lib-b"), &zp).unwrap();

        assert_eq!(from_zip.kind, ModKind::Goremod);
        assert_eq!(from_zip.name, from_dir.name);
        // Same slug prefix (same display name), different hash suffix (different source name).
        assert!(from_zip.id.starts_with("target-probe-"));
        assert_ne!(
            from_zip.id, from_dir.id,
            "dir vs zip are distinct sources → distinct ids"
        );
        assert_eq!(from_zip.components, from_dir.components);
        assert_eq!(from_zip.source, "Target Probe.zip");
    }

    /// [import 3] A zip whose bundle sits BELOW the root (nested folders, the usual way mods
    /// are shipped) is RE-ROOTED at import: the stored entry's top level IS the bundle root, so
    /// every component `rel` is bundle-root-relative (no `Wrap/Sub` prefix) — matching the payload
    /// manifests inside, which hold bundle-root-relative paths. The wrapper dirs are dropped.
    #[test]
    fn import_zip_nested_bundle_reroots() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bdir = mk_goremod_bundle(tmp.path());
        let zp = tmp.path().join("nested.zip");
        zip_dir_with_prefix(&bdir, "Wrap/Sub", &zp);

        let meta = import(&lib, &zp).unwrap();
        assert_eq!(meta.kind, ModKind::Goremod);
        assert_eq!(meta.name, "Target Probe");
        // Re-rooted: rels are canonical (`loc/edits.json`, `scripts`, …), NOT `Wrap/Sub/...`.
        assert_goremod_components(&meta, "");
        let entry = lib.join(&meta.id);
        assert!(
            entry.join("gore-mod.json").is_file(),
            "manifest hoisted to the entry root"
        );
        assert!(
            entry.join("loc").join("edits.json").is_file(),
            "payload hoisted to the root"
        );
        // The wrapper prefix is gone entirely.
        assert!(
            !entry.join("Wrap").exists(),
            "wrapper dir must be dropped after re-root"
        );
    }

    /// [import 3b] BUG 1 focus: a nested bundle carrying an AUDIO component re-roots so the stored
    /// `AudioPatch.rel` is `audio` (bundle-root-relative) and apply can read the payload at
    /// `<entry>/audio/manifest.json` + `<entry>/audio/0.wav` — the exact files the audio manifest
    /// references by bundle-root path. Before the re-root fix the rel was `Wrap/Sub/audio` while the
    /// manifest still said `audio/0.wav`, so apply read a nonexistent nested path.
    #[test]
    fn import_nested_bundle_with_audio_reroots_rel() {
        use crate::{Component, ModManifest};
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");

        // Hand-build a minimal audio bundle: gore-mod.json + audio/manifest.json + audio/0.wav,
        // all shipped under a `Wrap/Sub` wrapper (the nested shape find_manifest_dir supports).
        let bundle_root = tmp.path().join("src/Wrap/Sub");
        let audio = bundle_root.join("audio");
        fs::create_dir_all(&audio).unwrap();
        // Manifest maps bank→sample→wav_rel, where wav_rel is BUNDLE-ROOT-relative ("audio/0.wav").
        let mut manifest: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        manifest
            .entry("Voice.bank".into())
            .or_default()
            .insert("shout".into(), "audio/0.wav".into());
        fs::write(
            audio.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(audio.join("0.wav"), b"FAKE-WAV").unwrap();
        // A gore-mod.json whose single component is an AudioPatch at `audio`. Built through the
        // real `ModManifest` (its `mod` rename + the component's `type` tag) so it deserializes
        // exactly like a shipped bundle's manifest.
        let comp = Component::AudioPatch {
            path: "audio".into(),
            banks: vec!["Voice.bank".into()],
        };
        let mm = ModMeta {
            name: "Nested Audio".into(),
            version: "1".into(),
            author: "t".into(),
        };
        let manifest = ModManifest {
            format: 1,
            mod_meta: mm,
            components: vec![comp],
        };
        fs::write(
            bundle_root.join("gore-mod.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        // Import the wrapper root (`src`) so the bundle is nested two dirs deep.
        let meta = import(&lib, &tmp.path().join("src")).unwrap();
        assert_eq!(meta.kind, ModKind::Goremod);
        assert_eq!(meta.name, "Nested Audio");

        // The stored AudioPatch rel is bundle-root-relative (`audio`), not `Wrap/Sub/audio`.
        let rels: Vec<&str> = meta
            .components
            .iter()
            .filter_map(|c| match c {
                ComponentInfo::AudioPatch { rel, .. } => Some(rel.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            rels,
            vec!["audio"],
            "audio rel must be re-rooted: {:?}",
            meta.components
        );

        // And apply's read path resolves: <entry>/<rel>/manifest.json and the referenced wav exist.
        let entry = lib.join(&meta.id);
        assert!(entry.join("audio").join("manifest.json").is_file());
        assert!(
            entry.join("audio").join("0.wav").is_file(),
            "payload readable at bundle-root rel"
        );
        assert!(!entry.join("Wrap").exists(), "wrapper dropped");
    }

    /// [import 4] Zip entries that would escape the staging dir (`..`) abort the import,
    /// nothing is extracted outside, and the staging dir is cleaned up.
    #[test]
    fn import_zip_rejects_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let zp = tmp.path().join("evil.zip");
        let mut zw = zip::ZipWriter::new(fs::File::create(&zp).unwrap());
        zw.start_file("../evil.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zw.write_all(b"boo").unwrap();
        zw.finish().unwrap();

        let err = import(&lib, &zp).unwrap_err().to_string();
        assert!(err.contains("evil.txt"), "err: {err}");
        // `..` relative to a staging dir directly under the library would land here:
        assert!(!lib.join("evil.txt").exists());
        assert!(!tmp.path().join("evil.txt").exists());
        // ...and no staging leftovers survive the failed import.
        let leftovers: Vec<_> = fs::read_dir(&lib)
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name()).collect())
            .unwrap_or_default();
        assert!(leftovers.is_empty(), "staging leftovers: {leftovers:?}");
    }

    /// [import 5] A single foreign `*_P.pak` file imports as ForeignPak; a dummy (unparsable)
    /// pak yields empty targets rather than failing the import.
    #[test]
    fn import_foreign_pak_lists_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let pak = tmp.path().join("foo_P.pak");
        fs::write(&pak, b"definitely not a real pak").unwrap();

        let meta = import(&lib, &pak).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignPak);
        assert_eq!(meta.name, "foo_P");
        assert_eq!(meta.source, "foo_P.pak");
        assert_eq!(
            meta.components,
            vec![ComponentInfo::LoosePak {
                rel: "foo_P.pak".into(),
                targets: vec![]
            }]
        );
        assert!(lib.join(&meta.id).join("foo_P.pak").is_file());
    }

    /// [import 5b] Importing the `.pak` MEMBER of an IoStore triplet (the common file-picker pick)
    /// must pull its `.utoc`/`.ucas` siblings so the staged entry is the full triplet — otherwise
    /// apply would deploy an incomplete, un-mountable container.
    #[test]
    fn import_pak_member_of_triplet_pulls_siblings() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let dir = tmp.path().join("TripletSrc");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("zzz_foo_P.utoc"), b"junk").unwrap();
        fs::write(dir.join("zzz_foo_P.ucas"), b"junk").unwrap();
        fs::write(dir.join("zzz_foo_P.pak"), b"junk").unwrap();

        // Pick the .pak, not the .utoc.
        let meta = import(&lib, &dir.join("zzz_foo_P.pak")).unwrap();
        assert_eq!(
            meta.kind,
            ModKind::ForeignTriplet,
            "must detect the full triplet, not a loose pak: {:?}",
            meta.components
        );
        assert_eq!(
            meta.components,
            vec![ComponentInfo::Triplet {
                rel_base: "zzz_foo_P".into(),
                targets: vec![]
            }]
        );
        // All three members were staged into the entry.
        let entry = lib.join(&meta.id);
        assert!(entry.join("zzz_foo_P.utoc").is_file());
        assert!(entry.join("zzz_foo_P.ucas").is_file());
        assert!(entry.join("zzz_foo_P.pak").is_file());
    }

    /// [import 6] A `.utoc` + sibling `.ucas` pair is ONE Triplet component (unparsable dummy
    /// container → empty targets, import still succeeds).
    #[test]
    fn import_triplet_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("BarMod");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("bar.utoc"), b"junk").unwrap();
        fs::write(src.join("bar.ucas"), b"junk").unwrap();

        let meta = import(&lib, &src).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignTriplet);
        assert_eq!(meta.name, "BarMod");
        assert_eq!(
            meta.components,
            vec![ComponentInfo::Triplet {
                rel_base: "bar".into(),
                targets: vec![]
            }]
        );
    }

    /// [import 6b] Importing the `.utoc` FILE directly pulls its same-stem siblings along.
    #[test]
    fn import_utoc_file_copies_siblings() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        fs::write(tmp.path().join("bar.utoc"), b"junk").unwrap();
        fs::write(tmp.path().join("bar.ucas"), b"junk").unwrap();
        fs::write(tmp.path().join("bar.pak"), b"junk").unwrap();

        let meta = import(&lib, &tmp.path().join("bar.utoc")).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignTriplet);
        assert_eq!(
            meta.components,
            vec![ComponentInfo::Triplet {
                rel_base: "bar".into(),
                targets: vec![]
            }]
        );
        let entry = lib.join(&meta.id);
        assert!(entry.join("bar.ucas").is_file());
        assert!(entry.join("bar.pak").is_file());
    }

    /// [import 7] All-raw-files dir → ForeignRawfile with one RawFile component per file and
    /// the right live-target mapping; adding a pak to the mix → ForeignMixed.
    #[test]
    fn import_rawfiles_and_mixed() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let raw = tmp.path().join("RawStuff");
        fs::create_dir_all(&raw).unwrap();
        fs::write(raw.join("AlkimiaLocalization_0.lcache"), b"x").unwrap();
        fs::write(raw.join("SFX.bank"), b"x").unwrap();
        fs::write(raw.join("PrecompiledScript_Shipping.Cache"), b"x").unwrap();

        let meta = import(&lib, &raw).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignRawfile);
        assert_eq!(
            meta.components,
            vec![
                ComponentInfo::RawFile {
                    rel: "AlkimiaLocalization_0.lcache".into(),
                    target_file: RawTarget::Lcache,
                },
                ComponentInfo::RawFile {
                    rel: "PrecompiledScript_Shipping.Cache".into(),
                    target_file: RawTarget::ScriptCache,
                },
                ComponentInfo::RawFile {
                    rel: "SFX.bank".into(),
                    target_file: RawTarget::Bank {
                        name: "SFX.bank".into()
                    },
                },
            ]
        );

        let mixed = tmp.path().join("MixedStuff");
        fs::create_dir_all(&mixed).unwrap();
        fs::write(mixed.join("Music.bank"), b"x").unwrap();
        fs::write(mixed.join("extra_P.pak"), b"x").unwrap();
        let meta = import(&lib, &mixed).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignMixed);
        assert_eq!(meta.components.len(), 2);
    }

    /// [import 8] `.7z`/`.rar` are rejected with a "extract manually" pointer.
    #[test]
    fn import_rejects_7z() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let p = tmp.path().join("a.7z");
        fs::write(&p, b"7z\xbc\xaf\x27\x1c").unwrap();
        let err = import(&lib, &p).unwrap_err().to_string();
        assert!(err.contains("extract manually"), "err: {err}");
    }

    /// [import 9] A dir that IS a UE4SS mod (root `Scripts/main.lua`) is wrapped into a named
    /// subdir so the entry stays uniform and the deployable dir excludes the sidecar.
    #[test]
    fn import_ue4ss_mod_dir_wraps_root() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("MyLuaMod");
        fs::create_dir_all(src.join("Scripts")).unwrap();
        fs::write(src.join("Scripts").join("main.lua"), b"-- lua").unwrap();
        fs::write(src.join("enabled.txt"), b"").unwrap();

        let meta = import(&lib, &src).unwrap();
        assert_eq!(meta.kind, ModKind::ForeignUe4ss);
        assert_eq!(
            meta.components,
            vec![ComponentInfo::Ue4ssLua {
                name: "MyLuaMod".into(),
                rel: "MyLuaMod".into(),
                targets: vec![],
                opaque: true,
            }]
        );
        let entry = lib.join(&meta.id);
        assert!(entry
            .join("MyLuaMod")
            .join("Scripts")
            .join("main.lua")
            .is_file());
        assert!(entry.join("MyLuaMod").join("enabled.txt").is_file());
    }

    /// [import 10] A source with nothing recognizable in it is an error, not an empty entry.
    #[test]
    fn import_empty_dir_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("Nothing");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("readme.txt"), b"hi").unwrap();
        let err = import(&lib, &src).unwrap_err().to_string();
        assert!(err.contains("nothing importable"), "err: {err}");
        // failed import leaves no staging dir behind
        let leftovers: Vec<_> = fs::read_dir(&lib)
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name()).collect())
            .unwrap_or_default();
        assert!(leftovers.is_empty(), "staging leftovers: {leftovers:?}");
    }

    /// [import 11] Re-importing the SAME source (same name + same source dir/file name) REPLACES
    /// its entry (same id, one copy) — a mod update.
    #[test]
    fn reimport_same_source_replaces_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("BarMod");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("bar.utoc"), b"junk").unwrap();
        fs::write(src.join("bar.ucas"), b"junk").unwrap();

        let a = import(&lib, &src).unwrap();
        let b = import(&lib, &src).unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(list(&lib).unwrap().len(), 1);
        // The move-aside backup used for the atomic replace must be cleaned up after a successful
        // update — no `.replacing-*` dir may linger in the library.
        let leftovers: Vec<_> = fs::read_dir(&lib)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(".replacing-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "stale backup dir(s) after replace: {leftovers:?}"
        );
    }

    #[test]
    fn startup_recovery_restores_an_entry_interrupted_after_move_aside() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("old_P.pak");
        fs::write(&source, b"old payload").unwrap();
        let meta = import(&lib, &source).unwrap();
        let entry = lib.join(&meta.id);

        let transaction = ReplacementTransaction::begin(&lib, &meta.id).unwrap();
        fs::rename(&entry, transaction.backup()).unwrap();
        sync_replacement_directory(&lib).unwrap();
        transaction.mark(ReplacementPhase::PreviousMoved).unwrap();
        assert!(
            !entry.exists(),
            "simulated crash window requires a missing live entry"
        );
        assert!(transaction.backup().is_dir());

        // `list` is the normal manager startup/read path and performs recovery before observing
        // entries. It must restore the old entry rather than silently reporting an empty library.
        assert_eq!(list(&lib).unwrap(), vec![meta]);
        assert_eq!(fs::read(entry.join("old_P.pak")).unwrap(), b"old payload");
        assert!(!transaction.root.exists());
    }

    #[test]
    fn startup_recovery_keeps_promoted_entry_if_cleanup_was_interrupted() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("old_P.pak");
        fs::write(&source, b"old payload").unwrap();
        let old_meta = import(&lib, &source).unwrap();
        let entry = lib.join(&old_meta.id);

        let staging = lib.join(".staging-simulated-crash");
        fs::create_dir(&staging).unwrap();
        let mut new_meta = old_meta.clone();
        new_meta.version = "promoted".into();
        fs::write(
            staging.join(META_FILE),
            serde_json::to_vec(&new_meta).unwrap(),
        )
        .unwrap();
        fs::write(staging.join("new_P.pak"), b"new payload").unwrap();

        let transaction = ReplacementTransaction::begin(&lib, &old_meta.id).unwrap();
        fs::rename(&entry, transaction.backup()).unwrap();
        transaction.mark(ReplacementPhase::PreviousMoved).unwrap();
        fs::rename(&staging, &entry).unwrap();
        sync_replacement_directory(&lib).unwrap();
        // Deliberately omit the `Promoted` marker: this is the narrowest post-promotion crash.
        assert!(entry.is_dir() && transaction.backup().is_dir());

        assert_eq!(list(&lib).unwrap(), vec![new_meta]);
        assert_eq!(fs::read(entry.join("new_P.pak")).unwrap(), b"new payload");
        assert!(!transaction.root.exists());
    }

    #[test]
    fn replacement_names_are_unique_and_never_clear_an_existing_transaction() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        fs::create_dir(&lib).unwrap();
        let first = ReplacementTransaction::begin(&lib, "entry-a").unwrap();
        let second = ReplacementTransaction::begin(&lib, "entry-a").unwrap();
        assert_ne!(first.root, second.root);
        assert!(first.root.is_dir() && second.root.is_dir());
        first.cleanup().unwrap();
        second.cleanup().unwrap();
    }

    #[test]
    fn staged_sync_failure_happens_before_first_rename_and_preserves_old_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let entry = lib.join("entry-a");
        let staging = lib.join(".staging-new");
        fs::create_dir_all(&entry).unwrap();
        fs::create_dir(&staging).unwrap();
        fs::write(entry.join("payload.bin"), b"old").unwrap();
        fs::write(staging.join("payload.bin"), b"new").unwrap();

        let rename_calls = std::cell::Cell::new(0usize);
        let mut rename = |from: &Path, to: &Path| {
            rename_calls.set(rename_calls.get() + 1);
            fs::rename(from, to)
        };
        let mut fail_sync =
            |_root: &Path| Err(ModError::Other("injected staged-tree sync failure".into()));
        let error = activate_staged_entry_with_sync(
            &lib,
            &staging,
            &entry,
            "entry-a",
            &mut rename,
            &mut fail_sync,
        )
        .unwrap_err()
        .to_string();

        assert!(
            error.contains("injected staged-tree sync failure"),
            "{error}"
        );
        assert_eq!(rename_calls.get(), 0, "sync must precede every rename");
        assert_eq!(fs::read(entry.join("payload.bin")).unwrap(), b"old");
        assert_eq!(fs::read(staging.join("payload.bin")).unwrap(), b"new");
        assert!(fs::read_dir(&lib)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .all(|entry| !entry
                .file_name()
                .to_string_lossy()
                .starts_with(REPLACEMENT_PREFIX)));
        recover_interrupted_replacements(&lib).unwrap();
        assert_eq!(fs::read(entry.join("payload.bin")).unwrap(), b"old");
    }

    #[test]
    fn staged_sync_failure_does_not_activate_a_new_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let entry = lib.join("entry-a");
        let staging = lib.join(".staging-new");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("payload.bin"), b"new").unwrap();

        let mut rename =
            |_from: &Path, _to: &Path| panic!("rename must not run after a staged sync failure");
        let mut fail_sync =
            |_root: &Path| Err(ModError::Other("injected staged-tree sync failure".into()));
        activate_staged_entry_with_sync(
            &lib,
            &staging,
            &entry,
            "entry-a",
            &mut rename,
            &mut fail_sync,
        )
        .unwrap_err();

        assert!(!entry.exists());
        assert_eq!(fs::read(staging.join("payload.bin")).unwrap(), b"new");
        assert!(fs::read_dir(&lib)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .all(|entry| !entry
                .file_name()
                .to_string_lossy()
                .starts_with(REPLACEMENT_PREFIX)));
    }

    #[test]
    fn failed_promotion_reports_restore_failure_and_retains_recovery_data() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let entry = lib.join("entry-a");
        let staging = lib.join(".staging-new");
        fs::create_dir_all(&entry).unwrap();
        fs::create_dir(&staging).unwrap();
        fs::write(entry.join("payload.bin"), b"old").unwrap();
        fs::write(staging.join("payload.bin"), b"new").unwrap();

        let calls = std::cell::Cell::new(0usize);
        let mut injected_rename = |from: &Path, to: &Path| {
            let call = calls.get();
            calls.set(call + 1);
            if call == 0 {
                fs::rename(from, to)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected rename failure",
                ))
            }
        };
        let error =
            activate_staged_entry_with(&lib, &staging, &entry, "entry-a", &mut injected_rename)
                .unwrap_err()
                .to_string();
        assert!(error.contains("activating library entry"), "{error}");
        assert!(
            error.contains("restoring/cleaning the previous entry also failed"),
            "restore failure was swallowed: {error}"
        );
        assert!(!entry.exists());
        assert!(
            staging.is_dir(),
            "failed promotion must leave staging to its guard"
        );
        assert_eq!(
            fs::read_dir(&lib)
                .unwrap()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(REPLACEMENT_PREFIX))
                .count(),
            1,
            "the old entry must remain recoverable"
        );

        recover_interrupted_replacements(&lib).unwrap();
        assert_eq!(fs::read(entry.join("payload.bin")).unwrap(), b"old");
        assert!(fs::read_dir(&lib)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .all(|entry| !entry
                .file_name()
                .to_string_lossy()
                .starts_with(REPLACEMENT_PREFIX)));
    }

    #[test]
    fn concurrent_cleanup_after_promotion_never_rolls_back_the_only_live_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let entry = lib.join("entry-a");
        let staging = lib.join(".staging-new");
        fs::create_dir_all(&entry).unwrap();
        fs::create_dir(&staging).unwrap();
        fs::write(entry.join("payload.bin"), b"old").unwrap();
        fs::write(staging.join("payload.bin"), b"new").unwrap();

        let calls = std::cell::Cell::new(0usize);
        let mut concurrent_cleanup = |from: &Path, to: &Path| {
            fs::rename(from, to)?;
            let call = calls.get();
            calls.set(call + 1);
            if call == 1 {
                // Simulate another process observing live+backup immediately after promotion and
                // completing the transaction cleanup before this process can write `promoted`.
                let transaction = fs::read_dir(&lib)?
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .find(|path| {
                        path.file_name().is_some_and(|name| {
                            name.to_string_lossy().starts_with(REPLACEMENT_PREFIX)
                        })
                    })
                    .expect("replacement transaction");
                fs::remove_dir_all(transaction)?;
            }
            Ok(())
        };

        let error =
            activate_staged_entry_with(&lib, &staging, &entry, "entry-a", &mut concurrent_cleanup)
                .unwrap_err()
                .to_string();
        assert!(
            error.contains("already promoted and remains active"),
            "{error}"
        );
        assert_eq!(fs::read(entry.join("payload.bin")).unwrap(), b"new");
        assert!(!staging.exists());
    }

    /// [import 11b] Two mods that share a display NAME but come from DIFFERENT sources must get
    /// distinct ids and coexist — otherwise the old name-only id let one silently clobber the
    /// other (data loss). A goremod bundle's name comes from its manifest, so importing the SAME
    /// manifest-name bundle once as a dir ("Target Probe") and once as a differently-named zip
    /// ("other.zip") yields identical display names but different `source`s → different ids.
    #[test]
    fn different_source_same_name_coexist() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bdir = mk_goremod_bundle(tmp.path()); // manifest name "Target Probe"

        let from_dir = import(&lib, &bdir).unwrap();
        let zp = tmp.path().join("other.zip");
        zip_dir_with_prefix(&bdir, "", &zp);
        let from_zip = import(&lib, &zp).unwrap();

        assert_eq!(
            from_dir.name, from_zip.name,
            "precondition: same display name"
        );
        assert_ne!(
            from_dir.source, from_zip.source,
            "precondition: different source"
        );
        assert_ne!(
            from_dir.id, from_zip.id,
            "distinct sources must not collide into one id"
        );
        assert_eq!(list(&lib).unwrap().len(), 2, "both must coexist");
    }

    /// [import 11c] The nastier collision the name-only id missed: two DIFFERENT mods that share
    /// both a display name AND a bare filename but live in different directories (`a/mod` vs
    /// `b/mod`). Only the FULL source path disambiguates them; a filename-only hash would give
    /// both the same id and silently clobber the first. Must yield distinct ids and coexist.
    #[test]
    fn same_filename_different_dir_coexist() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let a = tmp.path().join("a").join("mod");
        let b = tmp.path().join("b").join("mod");
        for d in [&a, &b] {
            fs::create_dir_all(d).unwrap();
            fs::write(d.join("bar.utoc"), b"junk").unwrap();
            fs::write(d.join("bar.ucas"), b"junk").unwrap();
        }
        let from_a = import(&lib, &a).unwrap();
        let from_b = import(&lib, &b).unwrap();
        assert_eq!(from_a.name, from_b.name, "precondition: same display name");
        assert_eq!(
            from_a.source, from_b.source,
            "precondition: same bare filename"
        );
        assert_ne!(
            from_a.id, from_b.id,
            "same-name+filename in different dirs must not collide"
        );
        assert_eq!(list(&lib).unwrap().len(), 2, "both must coexist");
    }

    /// [remove] Deletes exactly the entry dir; absent id → Ok(false); ids that could climb
    /// out of the library are refused.
    #[test]
    fn remove_deletes_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let pak = tmp.path().join("foo_P.pak");
        fs::write(&pak, b"x").unwrap();
        let meta = import(&lib, &pak).unwrap();
        assert!(lib.join(&meta.id).is_dir());

        assert!(remove(&lib, &meta.id).unwrap());
        assert!(!lib.join(&meta.id).exists());
        assert!(
            !remove(&lib, &meta.id).unwrap(),
            "second remove must be false"
        );
        assert!(!remove(&lib, "never-existed").unwrap());
        assert!(
            remove(&lib, "..").is_err(),
            "path-escaping id must be refused"
        );
    }

    /// [list] Corrupt/unreadable sidecars are skipped (not fatal), non-entries are ignored,
    /// missing library dir is an empty list.
    #[test]
    fn list_skips_corrupt_meta() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        assert_eq!(list(&lib).unwrap(), vec![], "missing library dir");

        let src = tmp.path().join("GoodMod");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("stuff.lcache"), b"x").unwrap();
        let good = import(&lib, &src).unwrap();

        let broken = lib.join("zz-broken");
        fs::create_dir_all(&broken).unwrap();
        fs::write(broken.join(META_FILE), b"{ this is not json").unwrap();
        let no_meta = lib.join("zz-no-meta");
        fs::create_dir_all(&no_meta).unwrap();
        fs::write(lib.join("stray.txt"), b"x").unwrap();

        let all = list(&lib).unwrap();
        assert_eq!(all.len(), 1, "only the good entry: {all:?}");
        assert_eq!(all[0], good);
    }

    #[test]
    fn list_skips_sidecar_whose_id_does_not_match_its_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("GoodMod");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("stuff.lcache"), b"x").unwrap();
        let mut meta = import(&lib, &src).unwrap();
        let sidecar = lib.join(&meta.id).join(META_FILE);
        meta.id = "different-entry".into();
        fs::write(&sidecar, serde_json::to_vec(&meta).unwrap()).unwrap();

        assert!(list(&lib).unwrap().is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn list_accepts_sidecar_id_with_same_windows_path_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("GoodMod");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("stuff.lcache"), b"x").unwrap();
        let mut meta = import(&lib, &src).unwrap();
        let directory_id = meta.id.clone();
        let sidecar = lib.join(&directory_id).join(META_FILE);
        meta.id = meta.id.to_ascii_uppercase();
        assert_ne!(meta.id, directory_id);
        fs::write(&sidecar, serde_json::to_vec(&meta).unwrap()).unwrap();

        assert_eq!(list(&lib).unwrap(), vec![meta]);
    }

    #[cfg(any(unix, windows))]
    #[cfg_attr(
        windows,
        ignore = "requires Windows symbolic-link privilege; run explicitly on a privileged worker"
    )]
    #[test]
    fn list_skips_symbolic_link_or_reparse_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&lib).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let meta = ModEntryMeta {
            id: "linked-entry".into(),
            kind: ModKind::Goremod,
            name: "Linked".into(),
            version: String::new(),
            author: String::new(),
            imported_at: "2026-07-03T00:00:00Z".into(),
            source: String::new(),
            components: Vec::new(),
        };
        fs::write(outside.join(META_FILE), serde_json::to_vec(&meta).unwrap()).unwrap();
        assert!(
            make_dir_link(&outside, &lib.join("linked-entry")),
            "test requires symbolic-link creation support"
        );

        assert!(list(&lib).unwrap().is_empty());
    }

    /// [import 12] A pathologically deep source tree fails during bounded materialization instead
    /// of being copied and then silently omitted by classification below `MAX_SCAN_DEPTH`.
    #[test]
    fn folder_import_depth_is_capped_without_silent_omission() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("DeepMod");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("top.lcache"), b"x").unwrap();
        // Build a 20-deep nested chain (deeper than the depth cap of 16).
        let mut d = src.clone();
        for i in 0..20 {
            d = d.join(format!("d{i}"));
            fs::create_dir_all(&d).unwrap();
        }
        fs::write(d.join("buried.lcache"), b"x").unwrap();

        let error = import(&lib, &src).unwrap_err().to_string();
        assert!(error.contains("nesting depth limit exceeded"), "{error}");
        assert_failed_import_left_nothing(&lib);
    }

    /// The epoch→RFC3339 formatter, incl. a leap day and a modern date.
    #[test]
    fn utc_timestamp_formats_correctly() {
        assert_eq!(format_utc(0, 0), "1970-01-01T00:00:00.000000Z");
        assert_eq!(format_utc(1_000_000_000, 0), "2001-09-09T01:46:40.000000Z");
        assert_eq!(format_utc(951_782_400, 0), "2000-02-29T00:00:00.000000Z");
        assert_eq!(
            format_utc(1_767_225_600, 123_456),
            "2026-01-01T00:00:00.123456Z"
        );
        // Same second, different microseconds → distinct timestamps, so an entry re-imported within
        // the same second still gets a fingerprint-distinguishing `imported_at`.
        assert_ne!(
            format_utc(1_767_225_600, 100),
            format_utc(1_767_225_600, 200)
        );
    }
}
