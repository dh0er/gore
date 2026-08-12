//! Import mods into the manager library — plus [`list`]/[`remove`] over the imported entries.
//!
//! [`import`] materializes any supported source (dir, `.zip`, single game file) into a staging
//! dir, detects what it is (a goremod bundle via `gore-mod.json`, else a foreign-mod scan),
//! extracts each component's game-side **targets** (for later conflict analysis), and activates
//! the staged dir as `<library>/<id>/` with a [`META_FILE`] sidecar.

use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

#[cfg(not(unix))]
use super::model::metadata_is_link;
use super::model::{
    open_directory_nofollow, open_file_nofollow, ComponentInfo, FileIdentity, FileRevision,
    ImportIdentityMeta, LibraryEntry, LibraryMutationFileGuard, LibraryRoot, LibrarySidecar,
    ManagerPrivateMeta, MetaReadFailure, ModEntryMeta, ModKind, RawTarget, SecureDirectory,
    SecureFile, SecureNode, META_FILE,
};
use crate::{Component, ModError, ModManifest, ScriptEntry, VoicePatchManifest};

/// Result of one identity-aware import.  The entry remains the same public metadata shape used by
/// the compatibility [`import`] API; disposition and match provenance are additive native facts.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportOutcome {
    pub entry: ModEntryMeta,
    pub disposition: ImportDisposition,
    pub matched_by: ImportMatchedBy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportDisposition {
    Created,
    Updated,
    Unchanged,
}

impl ImportDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Unchanged => "unchanged",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportMatchedBy {
    None,
    Source,
    Content,
    EntryId,
}

/// One bounded, role-truthful witness for an identity conflict. The list in an error is capped for
/// wire safety, but the decision itself always considers the complete bounded candidate set.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ImportConflictCandidate {
    pub id: String,
    pub matched_by: Vec<ImportMatchedBy>,
}

impl ImportMatchedBy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Source => "source",
            Self::Content => "content",
            Self::EntryId => "entry_id",
        }
    }
}

/// Identity refusals stay distinguishable at the FFI boundary.  Everything else remains the
/// established `IMPORT_FAILED` class and is wrapped in `Failed`.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("multiple existing mods have the same verified content: {candidate_ids:?}")]
    DuplicateAmbiguous { candidate_ids: Vec<String> },
    #[error("verified import identities conflict: {candidates:?}")]
    IdentityConflict {
        candidates: Vec<ImportConflictCandidate>,
    },
    #[error(transparent)]
    Failed(#[from] ModError),
}

impl ImportError {
    pub fn candidate_ids(&self) -> &[String] {
        match self {
            Self::DuplicateAmbiguous { candidate_ids } => candidate_ids,
            _ => &[],
        }
    }

    pub fn conflict_candidates(&self) -> Option<&[ImportConflictCandidate]> {
        match self {
            Self::IdentityConflict { candidates } => Some(candidates),
            _ => None,
        }
    }

    fn into_mod_error(self) -> ModError {
        match self {
            Self::Failed(error) => error,
            refusal => ModError::Other(refusal.to_string()),
        }
    }
}

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
const REPLACEMENT_QUARANTINE_DIR: &str = "quarantine";
#[cfg(unix)]
const REPLACEMENT_ATOMIC_TEMP_SUFFIX: &str = ".pending";
const REPLACEMENT_STATE_MAX_BYTES: u64 = 4 * 1024;
const IMPORT_IDENTITY_FORMAT: u32 = 1;
const IMPORT_SOURCE_HASH_DOMAIN: &[u8] = b"gore-manager-import-source-v1\0";
const IMPORT_TREE_HASH_DOMAIN: &[u8] = b"gore-manager-import-tree-v1\0";
const MAX_IDENTITY_LIBRARY_ENTRIES: usize = 4_096;
const MAX_IDENTITY_SIDECAR_BYTES: u64 = 64 * 1024 * 1024;
const MAX_IDENTITY_REHASH_CANDIDATES: usize = 64;
const MAX_IDENTITY_REFUSAL_IDS: usize = 2;
#[cfg(test)]
const LIBRARY_LOCK_MARKER_ENV: &str = "GORE_TEST_LIBRARY_LOCK_MARKER";
#[cfg(test)]
const LIBRARY_LOCK_HOLD_MS_ENV: &str = "GORE_TEST_LIBRARY_LOCK_HOLD_MS";

/// The local mutex establishes one acquisition order inside this process. Windows then owns a
/// persistent lock-file byte range; Unix flocks the retained canonical library-directory inode.
/// Together they protect cooperative GORE recovery, identity decisions, sidecar writes,
/// publication, list recovery, and removal. Untrusted source materialization stays outside the
/// lane.
static LIBRARY_MUTATION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Import `source` (a folder, `.zip` archive, or single recognized game file) into the library
/// at `library_dir`, returning the entry metadata that was also written as its sidecar.
///
/// Pipeline: materialize into a `.staging-*` dir under the library (same volume, so activation
/// is a rename) → detect components + extract targets → hash/decide under the library lock →
/// write the private sidecar hints → publish through the existing replacement transaction.
/// A selected source/content hint is authority only after a bounded no-follow re-hash of the
/// current entry. New unmatched content keeps the legacy path-derived proposed-id shape.
pub fn import(library_dir: &Path, source: &Path) -> crate::Result<ModEntryMeta> {
    import_detailed(library_dir, source)
        .map(|outcome| outcome.entry)
        .map_err(ImportError::into_mod_error)
}

/// Identity-aware import with additive disposition and match provenance.
pub fn import_detailed(library_dir: &Path, source: &Path) -> Result<ImportOutcome, ImportError> {
    import_detailed_with_limits(library_dir, source, DEFAULT_IMPORT_LIMITS)
}

#[cfg(test)]
fn import_with_limits(
    library_dir: &Path,
    source: &Path,
    limits: ImportLimits,
) -> crate::Result<ModEntryMeta> {
    import_detailed_with_limits(library_dir, source, limits)
        .map(|outcome| outcome.entry)
        .map_err(ImportError::into_mod_error)
}

fn import_detailed_with_limits(
    library_dir: &Path,
    source: &Path,
    limits: ImportLimits,
) -> Result<ImportOutcome, ImportError> {
    if !source.exists() {
        return Err(
            ModError::Other(format!("import source not found: {}", source.display())).into(),
        );
    }
    std::fs::create_dir_all(library_dir).map_err(crate::io("creating library dir"))?;

    // Canonical view so `.`/trailing-separator sources still yield a usable name.
    let canon = std::fs::canonicalize(source).map_err(crate::io(&format!(
        "resolving import source {}",
        source.display()
    )))?;
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
            ))
            .into());
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
        ))
        .into());
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
    // Preserve the legacy proposed-id algorithm for create and legacy same-path binding. Stable
    // private source/content identity may choose a different existing id below, but never changes
    // the public id shape or ModEntryMeta fingerprint algorithm.
    let proposed_id = proposed_import_id(&name, &canon);
    let source_sha256 = hash_normalized_source_path(&canon);
    // Retain the exact materialized staging inode before entering the library lane. Unix later
    // resolves its direct-child name through the locked root fd and requires this identity;
    // Windows reopens and revalidates it immediately before path-based publication.
    let staged_directory = open_directory_nofollow(&staging, "materialized import staging tree")?;

    let library_lock = library_mutation_lock(library_dir)?;
    #[cfg(unix)]
    let mut locked_staging_guard = {
        // From this point on cleanup must use the retained, locked library inode. Disarm the
        // configured-path guard even if binding fails: deleting through that path after the lock
        // is released could target a replacement root. A failed bind therefore leaves, at worst,
        // an internal dot-directory in the retained inode rather than deleting an unverified path.
        let bound = LockedStagingGuard::bind(&library_lock, &staging, staged_directory.identity());
        guard.0 = None;
        bound?
    };
    let canonical_library_dir = library_lock.path().to_path_buf();
    recover_interrupted_replacements_locked(&library_lock)?;
    let staged_tree_sha256 = hash_secure_import_tree(&staged_directory, limits, false)?;
    let decision = {
        let library = library_lock.open_library()?;
        decide_import_identity(
            &library,
            &proposed_id,
            &source_sha256,
            &staged_tree_sha256,
            limits,
        )?
    };
    let (id, matched_by, previous) = match decision {
        ImportDecision::Create => (proposed_id, ImportMatchedBy::None, None),
        ImportDecision::Reuse {
            id,
            matched_by,
            sidecar,
            seal,
        } => (id, matched_by, Some((*sidecar, seal))),
    };
    let same_tree = previous
        .as_ref()
        .is_some_and(|(_, seal)| seal.tree_sha256 == staged_tree_sha256);
    let imported_at = match &previous {
        Some((sidecar, _)) if same_tree => sidecar.entry.imported_at.clone(),
        Some((sidecar, _)) => changed_import_timestamp(&sidecar.entry.imported_at)?,
        None => import_timestamp_now(),
    };
    let meta = ModEntryMeta {
        id: id.clone(),
        kind,
        name,
        version,
        author,
        imported_at,
        source: source_name,
        components,
    };
    let sidecar = LibrarySidecar {
        entry: meta.clone(),
        manager: Some(ManagerPrivateMeta {
            import_identity: Some(ImportIdentityMeta {
                format: IMPORT_IDENTITY_FORMAT,
                source_sha256,
                tree_sha256: staged_tree_sha256.clone(),
            }),
        }),
    };

    let unchanged = previous
        .as_ref()
        .is_some_and(|(previous_sidecar, _)| same_tree && previous_sidecar == &sidecar);

    // Sidecar goes into staging BEFORE the swap so the entry appears fully formed — a
    // concurrent `list()` never sees a half-imported dir it would have to skip.
    let staged_sidecar_sha256 =
        write_manager_sidecar(&staged_directory, &sidecar, limits.max_manifest_bytes)?;
    let expectation = PublishExpectation {
        staged: EntryPublishSeal {
            root_identity: staged_directory.identity(),
            tree_sha256: staged_tree_sha256,
            sidecar_sha256: staged_sidecar_sha256,
        },
        current: previous.as_ref().map(|(_, seal)| seal.clone()),
        limits,
    };
    let entry_dir = canonical_library_dir.join(&id);
    if unchanged {
        run_prepublish_race_hook(&staging, &entry_dir);
        let library = library_lock.open_library()?;
        verify_publish_expectation(&library, &staged_directory, &entry_dir, &expectation)?;
        return Ok(ImportOutcome {
            entry: previous
                .as_ref()
                .expect("unchanged imports have previous metadata")
                .0
                .entry
                .clone(),
            disposition: ImportDisposition::Unchanged,
            matched_by,
        });
    }
    // Activate atomically. When identity chooses an existing entry, move it ASIDE first, promote the
    // staged copy, and only then delete the old one — if promotion fails (crash, transient
    // FS/permission/AV), restore the old entry so a failed update never leaves the library (and the
    // loadout that references it) pointing at a now-missing mod. The backup is dot-prefixed so
    // `list()` skips it during the brief window it exists.
    run_prepublish_failure_hook()?;
    #[cfg(not(unix))]
    {
        drop(staged_directory);
    }
    activate_staged_entry(&library_lock, &staging, &entry_dir, &id, &expectation)?;
    #[cfg(unix)]
    locked_staging_guard.disarm();
    #[cfg(not(unix))]
    {
        guard.0 = None; // staging IS the entry now — nothing to clean
    }
    Ok(ImportOutcome {
        entry: meta,
        disposition: if previous.is_some() {
            ImportDisposition::Updated
        } else {
            ImportDisposition::Created
        },
        matched_by,
    })
}

#[derive(Debug)]
enum ImportDecision {
    Create,
    Reuse {
        id: String,
        matched_by: ImportMatchedBy,
        sidecar: Box<LibrarySidecar>,
        seal: EntryPublishSeal,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct EntryPublishSeal {
    root_identity: FileIdentity,
    tree_sha256: String,
    sidecar_sha256: String,
}

#[derive(Debug, Clone)]
struct PublishExpectation {
    staged: EntryPublishSeal,
    current: Option<EntryPublishSeal>,
    limits: ImportLimits,
}

#[derive(Debug)]
struct VerifiedIdentityCandidate {
    id: String,
    sidecar: LibrarySidecar,
    current_tree_sha256: String,
    sidecar_sha256: String,
    root_identity: FileIdentity,
    identity_managed: bool,
    source_match: bool,
    entry_id_match: bool,
    content_match: bool,
}

fn decide_import_identity(
    library: &LibraryRoot,
    proposed_id: &str,
    source_sha256: &str,
    staged_tree_sha256: &str,
    limits: ImportLimits,
) -> Result<ImportDecision, ImportError> {
    let mut names = Vec::new();
    for entry in library.read_dir()? {
        let entry = entry.map_err(crate::io("reading manager library identity entry"))?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        if names.len() >= MAX_IDENTITY_LIBRARY_ENTRIES {
            return Err(ModError::InspectionBound(format!(
                "manager import identity entry limit exceeded: more than {MAX_IDENTITY_LIBRARY_ENTRIES}"
            ))
            .into());
        }
        names.push(name);
    }
    names.sort();

    let mut remaining_sidecar_bytes = MAX_IDENTITY_SIDECAR_BYTES;
    let mut candidates = Vec::new();
    let mut rehash_budget = IdentityRehashBudget::new(limits);
    for name in names {
        let directory_id = name.to_str().ok_or_else(|| {
            ModError::Other("manager-library entry id is not valid Unicode".into())
        })?;
        let entry_id_match = library_ids_equal(directory_id, proposed_id);
        let entry = library.entry(directory_id).map_err(|error| {
            ModError::Other(format!(
                "manager-library entry {directory_id:?} is unsafe or unreadable: {error}"
            ))
        })?;
        let inspection = match entry
            .read_identity_sidecar_bounded_classified(&mut remaining_sidecar_bytes)
        {
            Ok(inspection) => inspection,
            Err(MetaReadFailure::AggregateBudgetExhausted { .. }) => {
                return Err(ModError::InspectionBound(format!(
                    "manager import identity sidecar budget exceeded: {MAX_IDENTITY_SIDECAR_BYTES} bytes"
                ))
                .into())
            }
            Err(MetaReadFailure::Other(error)) => {
                return Err(ModError::Other(format!(
                    "manager-library entry {directory_id:?} has an unreadable public sidecar: {error}"
                ))
                .into())
            }
        };
        let manager = inspection.manager.map_err(|error| {
            ModError::Other(format!(
                "manager-library entry {directory_id:?} has invalid private identity metadata: {error}"
            ))
        })?;
        let identity = manager
            .as_ref()
            .and_then(|manager| manager.import_identity.as_ref());
        if let Some(identity) = identity {
            validate_import_identity(identity).map_err(|error| {
                ModError::Other(format!(
                    "manager-library entry {:?} has invalid private identity metadata: {error}",
                    inspection.entry.id
                ))
            })?;
        }
        let source_match = identity
            .is_some_and(|identity| constant_time_text_eq(&identity.source_sha256, source_sha256));
        let content_hint_match = identity.is_some_and(|identity| {
            constant_time_text_eq(&identity.tree_sha256, staged_tree_sha256)
        });
        if !entry_id_match && !source_match && !content_hint_match {
            // Deliberate V1 boundary: no global re-hash of hint-less legacy entries or stale
            // negative content hints. Only a proposed-id/source/content-hint candidate is opened.
            continue;
        }
        let current_tree_sha256 =
            hash_library_entry_tree_for_identity(&entry, limits, &mut rehash_budget)?;
        if let Some(identity) = identity {
            if !constant_time_text_eq(&identity.tree_sha256, &current_tree_sha256) {
                return Err(ModError::Other(format!(
                    "manager-library tampering detected for selected entry {:?}: persisted tree identity does not match its current bounded no-follow tree",
                    inspection.entry.id
                ))
                .into());
            }
        }
        let identity_managed = identity.is_some();
        let sidecar = LibrarySidecar {
            entry: inspection.entry,
            manager,
        };
        candidates.push(VerifiedIdentityCandidate {
            id: sidecar.entry.id.clone(),
            sidecar,
            content_match: content_hint_match
                && constant_time_text_eq(&current_tree_sha256, staged_tree_sha256),
            current_tree_sha256,
            sidecar_sha256: inspection.sidecar_sha256,
            root_identity: entry.secure_directory().identity(),
            identity_managed,
            source_match,
            entry_id_match,
        });
    }

    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    let managed_id_collision = candidates.iter().find(|candidate| {
        candidate.identity_managed
            && candidate.entry_id_match
            && !candidate.source_match
            && !candidate.content_match
    });
    if let Some(collision) = managed_id_collision {
        return Err(ImportError::IdentityConflict {
            // This collision is independently sufficient to refuse. Restrict the bounded wire to
            // the causal witness so lexically earlier content candidates cannot crowd it out.
            candidates: conflict_details(std::iter::once(collision)),
        });
    }
    let primary: Vec<_> = candidates
        .iter()
        .filter(|candidate| {
            candidate.source_match || (candidate.entry_id_match && !candidate.identity_managed)
        })
        .collect();
    if primary.len() > 1 {
        return Err(ImportError::IdentityConflict {
            candidates: conflict_details(primary.iter().copied()),
        });
    }
    let content: Vec<_> = candidates
        .iter()
        .filter(|candidate| candidate.content_match)
        .collect();
    if let Some(primary) = primary.first().copied() {
        if content.iter().any(|candidate| candidate.id != primary.id) {
            let distinct_content = content
                .iter()
                .copied()
                .find(|candidate| candidate.id != primary.id)
                .expect("conflict predicate found a distinct content candidate");
            return Err(ImportError::IdentityConflict {
                // Select the source/legacy-id witness and one distinct content witness before
                // sorting/capping the diagnostic wire. Both causal roles are therefore retained.
                candidates: conflict_details([primary, distinct_content]),
            });
        }
        return Ok(reuse_decision(
            primary,
            if primary.source_match {
                ImportMatchedBy::Source
            } else {
                ImportMatchedBy::EntryId
            },
        ));
    }
    if content.len() > 1 {
        return Err(ImportError::DuplicateAmbiguous {
            candidate_ids: content
                .iter()
                .take(MAX_IDENTITY_REFUSAL_IDS)
                .map(|candidate| candidate.id.clone())
                .collect(),
        });
    }
    if let Some(content) = content.first().copied() {
        return Ok(reuse_decision(content, ImportMatchedBy::Content));
    }
    Ok(ImportDecision::Create)
}

fn conflict_details<'a>(
    candidates: impl IntoIterator<Item = &'a VerifiedIdentityCandidate>,
) -> Vec<ImportConflictCandidate> {
    let mut reasons = BTreeMap::<String, [bool; 3]>::new();
    for candidate in candidates {
        let matched = reasons.entry(candidate.id.clone()).or_default();
        matched[0] |= candidate.entry_id_match;
        matched[1] |= candidate.source_match;
        matched[2] |= candidate.content_match;
    }
    reasons
        .into_iter()
        .take(MAX_IDENTITY_REFUSAL_IDS)
        .map(|(id, matched)| {
            let mut matched_by = Vec::with_capacity(3);
            if matched[0] {
                matched_by.push(ImportMatchedBy::EntryId);
            }
            if matched[1] {
                matched_by.push(ImportMatchedBy::Source);
            }
            if matched[2] {
                matched_by.push(ImportMatchedBy::Content);
            }
            ImportConflictCandidate { id, matched_by }
        })
        .collect()
}

fn reuse_decision(
    candidate: &VerifiedIdentityCandidate,
    matched_by: ImportMatchedBy,
) -> ImportDecision {
    ImportDecision::Reuse {
        id: candidate.id.clone(),
        matched_by,
        sidecar: Box::new(candidate.sidecar.clone()),
        seal: EntryPublishSeal {
            root_identity: candidate.root_identity,
            tree_sha256: candidate.current_tree_sha256.clone(),
            sidecar_sha256: candidate.sidecar_sha256.clone(),
        },
    }
}

fn validate_import_identity(identity: &ImportIdentityMeta) -> crate::Result<()> {
    if identity.format != IMPORT_IDENTITY_FORMAT
        || !is_lower_sha256(&identity.source_sha256)
        || !is_lower_sha256(&identity.tree_sha256)
    {
        return Err(ModError::Other(format!(
            "expected format {IMPORT_IDENTITY_FORMAT} and two lowercase SHA-256 digests"
        )));
    }
    Ok(())
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn constant_time_text_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

#[cfg(windows)]
fn library_ids_equal(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

#[cfg(not(windows))]
fn library_ids_equal(left: &str, right: &str) -> bool {
    left == right
}

fn proposed_import_id(name: &str, canonical_source: &Path) -> String {
    let proposed = format!(
        "{}-{}",
        slug(name),
        crate::name_hash(&format!("{name}\0{}", canonical_source.display()))
    );
    proposed_import_id_override(proposed)
}

#[cfg(test)]
type ImportPathHook = Option<Box<dyn FnOnce(&Path)>>;

#[cfg(test)]
type ImportPathPairHook = Option<Box<dyn FnOnce(&Path, &Path)>>;

#[cfg(test)]
thread_local! {
    static PROPOSED_IMPORT_ID_OVERRIDE: std::cell::RefCell<Option<String>> = const {
        std::cell::RefCell::new(None)
    };
    static IMPORT_TIMESTAMP_OVERRIDE: std::cell::RefCell<Option<String>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(test)]
fn inject_proposed_import_id(id: impl Into<String>) {
    PROPOSED_IMPORT_ID_OVERRIDE.with(|slot| *slot.borrow_mut() = Some(id.into()));
}

#[cfg(test)]
fn proposed_import_id_override(fallback: String) -> String {
    PROPOSED_IMPORT_ID_OVERRIDE.with(|slot| slot.borrow_mut().take().unwrap_or(fallback))
}

#[cfg(not(test))]
fn proposed_import_id_override(fallback: String) -> String {
    fallback
}

#[cfg(test)]
fn inject_import_timestamp(timestamp: impl Into<String>) {
    IMPORT_TIMESTAMP_OVERRIDE.with(|slot| *slot.borrow_mut() = Some(timestamp.into()));
}

fn import_timestamp_now() -> String {
    #[cfg(test)]
    if let Some(timestamp) = IMPORT_TIMESTAMP_OVERRIDE.with(|slot| slot.borrow_mut().take()) {
        return timestamp;
    }
    let since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format_utc(since_epoch.as_secs() as i64, since_epoch.subsec_micros())
}

fn changed_import_timestamp(previous: &str) -> crate::Result<String> {
    let current = import_timestamp_now();
    if let (Some((current_seconds, current_micros)), Some((previous_seconds, previous_micros))) =
        (parse_utc_timestamp(&current), parse_utc_timestamp(previous))
    {
        let maximum = parse_utc_timestamp("9999-12-31T23:59:59.999999Z")
            .expect("maximum manager timestamp is canonical");
        if (previous_seconds, previous_micros) >= maximum {
            return Err(ModError::Other(
                "manager import timestamp exhausted the canonical four-digit RFC 3339 range".into(),
            ));
        }
        let (next_seconds, next_micros) = if previous_micros == 999_999 {
            (
                previous_seconds
                    .checked_add(1)
                    .ok_or_else(|| ModError::Other("manager import timestamp overflowed".into()))?,
                0,
            )
        } else {
            (previous_seconds, previous_micros + 1)
        };
        let next = (next_seconds, next_micros);
        if next > maximum {
            return Err(ModError::Other(
                "manager import timestamp exhausted the canonical four-digit RFC 3339 range".into(),
            ));
        }
        return Ok(if (current_seconds, current_micros) >= next {
            format_utc(current_seconds, current_micros)
        } else {
            format_utc(next_seconds, next_micros)
        });
    }
    if current != previous {
        return Ok(current);
    }
    // Legacy sidecars did not validate `imported_at`. Preserve compatibility while still ensuring
    // changed bytes cannot reuse the exact fingerprint when either value is non-canonical.
    let Some(prefix) = current.strip_suffix('Z') else {
        return Ok(format!("{current}.000001Z"));
    };
    if prefix
        .rsplit_once('T')
        .is_some_and(|(_, time)| time.contains('.'))
    {
        Ok(format!("{prefix}1Z"))
    } else {
        Ok(format!("{prefix}.000001Z"))
    }
}

fn parse_utc_timestamp(value: &str) -> Option<(i64, u32)> {
    let bytes = value.as_bytes();
    if bytes.len() != 27
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'.'
        || bytes[26] != b'Z'
    {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        bytes[range].iter().try_fold(0i64, |value, byte| {
            byte.is_ascii_digit()
                .then_some(value * 10 + i64::from(byte - b'0'))
        })
    };
    let year = number(0..4)?;
    let month = u32::try_from(number(5..7)?).ok()?;
    let day = u32::try_from(number(8..10)?).ok()?;
    let hour = number(11..13)?;
    let minute = number(14..16)?;
    let second = number(17..19)?;
    let micros = u32::try_from(number(20..26)?).ok()?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let seconds = days
        .checked_mul(86_400)?
        .checked_add(hour * 3_600 + minute * 60 + second)?;
    (format_utc(seconds, micros) == value).then_some((seconds, micros))
}

fn hash_normalized_source_path(path: &Path) -> String {
    let normalized = normalized_source_path_bytes(path);
    let mut hash = Sha256::new();
    hash.update(IMPORT_SOURCE_HASH_DOMAIN);
    hash.update((normalized.len() as u64).to_le_bytes());
    hash.update(&normalized);
    digest_hex(hash.finalize().into())
}

#[cfg(windows)]
fn normalized_source_path_bytes(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt as _;

    let mut units: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .map(|unit| match unit {
            0x005c => 0x002f,
            0x0041..=0x005a => unit + 0x0020,
            _ => unit,
        })
        .collect();
    const VERBATIM: &[u16] = &[b'/' as u16, b'/' as u16, b'?' as u16, b'/' as u16];
    const VERBATIM_UNC: &[u16] = &[
        b'/' as u16,
        b'/' as u16,
        b'?' as u16,
        b'/' as u16,
        b'u' as u16,
        b'n' as u16,
        b'c' as u16,
        b'/' as u16,
    ];
    if units.starts_with(VERBATIM_UNC) {
        units.splice(..VERBATIM_UNC.len(), [b'/' as u16, b'/' as u16]);
    } else if units.starts_with(VERBATIM) {
        units.drain(..VERBATIM.len());
    }
    units.into_iter().flat_map(u16::to_le_bytes).collect()
}

#[cfg(unix)]
fn normalized_source_path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt as _;

    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(any(windows, unix)))]
fn normalized_source_path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().replace('\\', "/").into_bytes()
}

#[cfg(test)]
fn hash_import_tree(root: &Path, limits: ImportLimits) -> crate::Result<String> {
    let root = open_directory_nofollow(root, "normalized import tree")?;
    hash_secure_import_tree(&root, limits, false)
}

fn hash_library_entry_tree(entry: &LibraryEntry, limits: ImportLimits) -> crate::Result<String> {
    hash_secure_import_tree(entry.secure_directory(), limits, true)
}

fn hash_library_entry_tree_for_identity(
    entry: &LibraryEntry,
    limits: ImportLimits,
    budget: &mut IdentityRehashBudget,
) -> crate::Result<String> {
    hash_secure_import_tree_inner(entry.secure_directory(), limits, true, Some(budget))
}

#[derive(Debug, Default)]
struct TreeHashBudget {
    entries: usize,
    bytes: u64,
}

#[derive(Debug)]
struct IdentityRehashBudget {
    candidate_hashes: usize,
    descriptor_work: usize,
    bytes: u64,
    max_descriptor_work: usize,
    max_bytes: u64,
}

impl IdentityRehashBudget {
    fn new(limits: ImportLimits) -> Self {
        Self {
            candidate_hashes: 0,
            descriptor_work: 0,
            bytes: 0,
            // Every hash performs a before/after membership collection. One maximally sized
            // candidate must fit; subsequent candidates share the remaining envelope.
            max_descriptor_work: limits.max_zip_entries.saturating_mul(2),
            max_bytes: limits.max_zip_total_uncompressed_bytes,
        }
    }

    fn charge_candidate(&mut self, descriptors: &[TreeDescriptor]) -> crate::Result<()> {
        self.candidate_hashes = self.candidate_hashes.checked_add(1).ok_or_else(|| {
            ModError::InspectionBound("manager import identity hash work overflowed".into())
        })?;
        if self.candidate_hashes > MAX_IDENTITY_REHASH_CANDIDATES {
            return Err(ModError::InspectionBound(format!(
                "manager import identity candidate rehash limit exceeded: {} > {MAX_IDENTITY_REHASH_CANDIDATES}",
                self.candidate_hashes
            )));
        }
        let work = descriptors.len().checked_mul(2).ok_or_else(|| {
            ModError::InspectionBound("manager import identity entry work overflowed".into())
        })?;
        self.descriptor_work = self.descriptor_work.checked_add(work).ok_or_else(|| {
            ModError::InspectionBound(
                "manager import identity aggregate entry work overflowed".into(),
            )
        })?;
        if self.descriptor_work > self.max_descriptor_work {
            return Err(ModError::InspectionBound(format!(
                "manager import identity aggregate entry work limit exceeded: {} > {}",
                self.descriptor_work, self.max_descriptor_work
            )));
        }
        let candidate_bytes = descriptors.iter().try_fold(0u64, |total, descriptor| {
            let length = match descriptor.kind {
                TreeDescriptorKind::File { length, .. } => length,
                TreeDescriptorKind::Directory { .. } => 0,
            };
            total.checked_add(length).ok_or_else(|| {
                ModError::InspectionBound(
                    "manager import identity candidate bytes overflowed".into(),
                )
            })
        })?;
        self.bytes = self.bytes.checked_add(candidate_bytes).ok_or_else(|| {
            ModError::InspectionBound("manager import identity aggregate bytes overflowed".into())
        })?;
        if self.bytes > self.max_bytes {
            return Err(ModError::InspectionBound(format!(
                "manager import identity aggregate byte limit exceeded: {} > {}",
                self.bytes, self.max_bytes
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TreeDescriptorKind {
    File {
        identity: FileIdentity,
        revision: FileRevision,
        length: u64,
    },
    Directory {
        identity: FileIdentity,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeDescriptor {
    relative: PathBuf,
    portable: String,
    kind: TreeDescriptorKind,
}

fn hash_secure_import_tree(
    root: &SecureDirectory,
    limits: ImportLimits,
    allow_exact_manager_sidecar: bool,
) -> crate::Result<String> {
    hash_secure_import_tree_inner(root, limits, allow_exact_manager_sidecar, None)
}

fn hash_secure_import_tree_inner(
    root: &SecureDirectory,
    limits: ImportLimits,
    allow_exact_manager_sidecar: bool,
    identity_budget: Option<&mut IdentityRehashBudget>,
) -> crate::Result<String> {
    let descriptors = collect_secure_import_tree(root, limits, allow_exact_manager_sidecar)?;
    if let Some(budget) = identity_budget {
        // Debit the complete planned work before the first payload byte is opened/read. The second
        // membership pass is charged from the same descriptor count and must match it exactly.
        budget.charge_candidate(&descriptors)?;
    }
    let mut hash = Sha256::new();
    hash.update(IMPORT_TREE_HASH_DOMAIN);
    for descriptor in &descriptors {
        let node = root.open_relative_node(
            &descriptor.relative,
            "normalized import tree entry selected for hashing",
        )?;
        match (descriptor.kind, node) {
            (TreeDescriptorKind::Directory { identity }, SecureNode::Directory(directory))
                if directory.identity() == identity =>
            {
                hash_tree_record(&mut hash, b'd', &descriptor.portable, None);
            }
            (
                TreeDescriptorKind::File {
                    identity,
                    revision,
                    length,
                },
                SecureNode::File(mut file),
            ) if file.identity() == identity
                && file.revision() == revision
                && file.len() == length =>
            {
                hash_tree_record(&mut hash, b'f', &descriptor.portable, Some(length));
                hash_open_import_file(&mut file, length, &mut hash)?;
            }
            _ => {
                return Err(ModError::Other(format!(
                    "normalized import tree changed type, identity, size, or content revision while hashing: {}",
                    root.path().join(&descriptor.relative).display()
                )));
            }
        }
    }
    let observed = collect_secure_import_tree(root, limits, allow_exact_manager_sidecar)?;
    if observed != descriptors {
        return Err(ModError::Other(format!(
            "normalized import tree changed identity or membership while hashing: {}",
            root.path().display()
        )));
    }
    Ok(digest_hex(hash.finalize().into()))
}

fn collect_secure_import_tree(
    root: &SecureDirectory,
    limits: ImportLimits,
    allow_exact_manager_sidecar: bool,
) -> crate::Result<Vec<TreeDescriptor>> {
    let mut budget = TreeHashBudget::default();
    let mut descriptors = BTreeMap::new();
    collect_secure_import_directory(
        root,
        Path::new(""),
        0,
        limits,
        allow_exact_manager_sidecar,
        &mut budget,
        &mut descriptors,
    )?;
    Ok(descriptors.into_values().collect())
}

fn collect_secure_import_directory(
    directory: &SecureDirectory,
    relative_dir: &Path,
    depth: usize,
    limits: ImportLimits,
    allow_exact_manager_sidecar: bool,
    budget: &mut TreeHashBudget,
    descriptors: &mut BTreeMap<String, TreeDescriptor>,
) -> crate::Result<()> {
    for child in directory.read_dir("normalized import tree")? {
        let child = child.map_err(crate::io("reading normalized import tree entry"))?;
        let name = child.file_name();
        let name_text = name.to_str().ok_or_else(|| {
            ModError::Other(format!(
                "normalized import tree path is not valid Unicode: {}",
                directory.path().join(&name).display()
            ))
        })?;
        if relative_dir.as_os_str().is_empty() && portable_windows_names_equal(name_text, META_FILE)
        {
            if allow_exact_manager_sidecar && name_text == META_FILE {
                continue;
            }
            return Err(ModError::Other(format!(
                "normalized import tree contains reserved manager-sidecar name {name_text:?}: {}",
                directory.path().join(&name).display()
            )));
        }
        budget.entries = budget
            .entries
            .checked_add(1)
            .ok_or_else(|| ModError::Other("normalized tree entry count overflowed".into()))?;
        check_import_limit(
            "normalized tree entry count",
            budget.entries as u64,
            limits.max_zip_entries as u64,
        )?;
        let relative = relative_dir.join(&name);
        let portable = portable_import_rel_path(&relative, &directory.path().join(&name))?;
        check_import_limit(
            "normalized tree path bytes",
            portable.len() as u64,
            limits.max_zip_path_bytes as u64,
        )?;
        let key = portable_windows_key(&portable);
        let node = directory.open_child(&name, "normalized import tree entry")?;
        match node {
            SecureNode::Directory(child) => {
                if depth >= limits.max_directory_depth {
                    return Err(ModError::Other(format!(
                        "normalized tree nesting depth limit exceeded at {}: {} > {}",
                        child.path().display(),
                        depth + 1,
                        limits.max_directory_depth
                    )));
                }
                let descriptor = TreeDescriptor {
                    relative: relative.clone(),
                    portable,
                    kind: TreeDescriptorKind::Directory {
                        identity: child.identity(),
                    },
                };
                insert_tree_descriptor(descriptors, key, descriptor.clone())?;
                collect_secure_import_directory(
                    &child,
                    &relative,
                    depth + 1,
                    limits,
                    allow_exact_manager_sidecar,
                    budget,
                    descriptors,
                )?;
            }
            SecureNode::File(file) => {
                let expected = file.len();
                check_import_limit(
                    "normalized tree file bytes",
                    expected,
                    limits.max_zip_entry_uncompressed_bytes,
                )?;
                let next_total = budget.bytes.checked_add(expected).ok_or_else(|| {
                    ModError::Other("normalized tree total byte count overflowed".into())
                })?;
                check_import_limit(
                    "normalized tree total bytes",
                    next_total,
                    limits.max_zip_total_uncompressed_bytes,
                )?;
                budget.bytes = next_total;
                let descriptor = TreeDescriptor {
                    relative,
                    portable,
                    kind: TreeDescriptorKind::File {
                        identity: file.identity(),
                        revision: file.revision(),
                        length: expected,
                    },
                };
                insert_tree_descriptor(descriptors, key, descriptor)?;
            }
        }
    }
    Ok(())
}

fn insert_tree_descriptor(
    descriptors: &mut BTreeMap<String, TreeDescriptor>,
    key: String,
    descriptor: TreeDescriptor,
) -> crate::Result<()> {
    if let Some(first) = descriptors.insert(key, descriptor.clone()) {
        return Err(ModError::Other(format!(
            "normalized import tree contains portable path collision between {:?} and {:?}",
            first.portable, descriptor.portable
        )));
    }
    Ok(())
}

fn hash_tree_record(hash: &mut Sha256, marker: u8, path: &str, length: Option<u64>) {
    hash.update([marker]);
    hash.update((path.len() as u64).to_le_bytes());
    hash.update(path.as_bytes());
    if let Some(length) = length {
        hash.update(length.to_le_bytes());
    }
}

fn hash_open_import_file(
    file: &mut SecureFile,
    expected: u64,
    hash: &mut Sha256,
) -> crate::Result<()> {
    let mut remaining = expected;
    let mut buffer = [0u8; 64 * 1024];
    while remaining != 0 {
        let wanted = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = file
            .file
            .read(&mut buffer[..wanted])
            .map_err(crate::io("hashing normalized import tree file"))?;
        if read == 0 {
            return Err(ModError::Other(format!(
                "normalized import tree file changed while hashing: {}",
                file.path().display()
            )));
        }
        hash.update(&buffer[..read]);
        remaining -= read as u64;
    }
    let mut probe = [0u8; 1];
    if file
        .file
        .read(&mut probe)
        .map_err(crate::io("probing normalized import tree file"))?
        != 0
    {
        return Err(ModError::Other(format!(
            "normalized import tree file grew while hashing: {}",
            file.path().display()
        )));
    }
    file.verify_len(expected, "normalized import tree file")
}

fn digest_hex(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

struct BoundedSidecarWriter {
    bytes: Vec<u8>,
    limit: u64,
    exceeded: bool,
}

impl std::io::Write for BoundedSidecarWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let next = (self.bytes.len() as u64)
            .checked_add(buffer.len() as u64)
            .ok_or_else(|| std::io::Error::other("manager sidecar length overflowed"))?;
        if next > self.limit {
            self.exceeded = true;
            return Err(std::io::Error::other("manager sidecar limit exceeded"));
        }
        self.bytes
            .try_reserve(buffer.len())
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn write_manager_sidecar(
    root: &SecureDirectory,
    sidecar: &LibrarySidecar,
    max_bytes: u64,
) -> crate::Result<String> {
    let mut writer = BoundedSidecarWriter {
        bytes: Vec::new(),
        limit: max_bytes,
        exceeded: false,
    };
    let serialized = serde_json::to_writer_pretty(&mut writer, sidecar);
    if writer.exceeded {
        return Err(ModError::InspectionBound(format!(
            "manager sidecar serialization exceeds the {max_bytes} byte limit"
        )));
    }
    serialized?;
    let bytes = writer.bytes;
    let sidecar_sha256 = digest_hex(Sha256::digest(&bytes).into());
    let name = std::ffi::OsStr::new(META_FILE);
    if root.contains_child(name, "staged manager sidecar")? {
        match root.open_child(name, "existing staged manager sidecar")? {
            SecureNode::File(file) => {
                let identity = file.identity();
                drop(file);
                root.remove_child_file_if_identity(
                    name,
                    identity,
                    "existing staged manager sidecar",
                )?;
            }
            SecureNode::Directory(directory) => {
                return Err(ModError::Other(format!(
                    "staged manager sidecar path is a directory: {}",
                    directory.path().display()
                )));
            }
        }
    }
    let (mut file, _) = root.create_child_file_new(name, "staged manager sidecar")?;
    file.write_all(&bytes)
        .map_err(crate::io("writing staged manager sidecar"))?;
    file.sync_all()
        .map_err(crate::io("syncing staged manager sidecar"))?;
    drop(file);
    root.sync_after_mutation("staged manager sidecar")?;
    Ok(sidecar_sha256)
}

#[cfg(test)]
thread_local! {
    static PREPUBLISH_FAILURE: std::cell::RefCell<Option<ModError>> = const {
        std::cell::RefCell::new(None)
    };
    static PREPUBLISH_RACE_HOOK: std::cell::RefCell<ImportPathPairHook> =
        const { std::cell::RefCell::new(None) };
    static POST_CREATE_RENAME_HOOK: std::cell::RefCell<ImportPathHook> =
        const { std::cell::RefCell::new(None) };
    static POST_PROMOTE_RENAME_HOOK: std::cell::RefCell<ImportPathHook> =
        const { std::cell::RefCell::new(None) };
    static REPLACEMENT_MARK_FAILURE: std::cell::RefCell<Option<ReplacementPhase>> =
        const { std::cell::RefCell::new(None) };
    static LIBRARY_ROOT_SWAP_HOOK: std::cell::RefCell<ImportPathHook> =
        const { std::cell::RefCell::new(None) };
    static RECOVERY_SEAL_FAILURE: std::cell::Cell<bool> = const {
        std::cell::Cell::new(false)
    };
}

#[cfg(test)]
fn inject_prepublish_failure(error: ModError) {
    PREPUBLISH_FAILURE.with(|slot| *slot.borrow_mut() = Some(error));
}

#[cfg(test)]
fn inject_recovery_seal_failure() {
    RECOVERY_SEAL_FAILURE.with(|slot| slot.set(true));
}

#[cfg(test)]
fn run_recovery_seal_failure_hook() -> crate::Result<()> {
    RECOVERY_SEAL_FAILURE.with(|slot| {
        if slot.replace(false) {
            Err(ModError::Other(
                "injected transient recovery seal read failure".into(),
            ))
        } else {
            Ok(())
        }
    })
}

#[cfg(not(test))]
fn run_recovery_seal_failure_hook() -> crate::Result<()> {
    Ok(())
}

#[cfg(test)]
fn run_prepublish_failure_hook() -> crate::Result<()> {
    PREPUBLISH_FAILURE.with(|slot| match slot.borrow_mut().take() {
        Some(error) => Err(error),
        None => Ok(()),
    })
}

#[cfg(not(test))]
fn run_prepublish_failure_hook() -> crate::Result<()> {
    Ok(())
}

#[cfg(test)]
fn inject_prepublish_race(hook: impl FnOnce(&Path, &Path) + 'static) {
    PREPUBLISH_RACE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_prepublish_race_hook(staging: &Path, entry: &Path) {
    PREPUBLISH_RACE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(staging, entry);
        }
    });
}

#[cfg(not(test))]
fn run_prepublish_race_hook(_staging: &Path, _entry: &Path) {}

#[cfg(test)]
fn inject_post_create_rename(hook: impl FnOnce(&Path) + 'static) {
    POST_CREATE_RENAME_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_post_create_rename_hook(entry: &Path) {
    POST_CREATE_RENAME_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(entry);
        }
    });
}

#[cfg(not(test))]
fn run_post_create_rename_hook(_entry: &Path) {}

#[cfg(test)]
fn inject_post_promote_rename(hook: impl FnOnce(&Path) + 'static) {
    POST_PROMOTE_RENAME_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_post_promote_rename_hook(entry: &Path) {
    POST_PROMOTE_RENAME_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(entry);
        }
    });
}

#[cfg(not(test))]
fn run_post_promote_rename_hook(_entry: &Path) {}

#[cfg(test)]
fn inject_replacement_mark_failure(phase: ReplacementPhase) {
    REPLACEMENT_MARK_FAILURE.with(|slot| *slot.borrow_mut() = Some(phase));
}

#[cfg(all(test, unix))]
fn inject_library_root_swap(hook: impl FnOnce(&Path) + 'static) {
    LIBRARY_ROOT_SWAP_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_library_root_swap_hook(path: &Path) {
    LIBRARY_ROOT_SWAP_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(path);
        }
    });
}

#[cfg(not(test))]
fn run_library_root_swap_hook(_path: &Path) {}

#[cfg(test)]
fn run_replacement_mark_failure(phase: ReplacementPhase) -> crate::Result<()> {
    REPLACEMENT_MARK_FAILURE.with(|slot| {
        if slot.borrow().as_ref() == Some(&phase) {
            slot.borrow_mut().take();
            Err(ModError::Other(format!(
                "injected {phase:?} replacement-marker failure"
            )))
        } else {
            Ok(())
        }
    })
}

#[cfg(not(test))]
fn run_replacement_mark_failure(_phase: ReplacementPhase) -> crate::Result<()> {
    Ok(())
}

#[cfg(any(not(unix), test))]
fn seal_import_path(root: &Path, limits: ImportLimits) -> crate::Result<EntryPublishSeal> {
    let root = open_directory_nofollow(root, "sealed manager-library tree")?;
    seal_secure_import_directory(&root, limits)
}

fn read_secure_child_bounded(
    directory: &SecureDirectory,
    name: &std::ffi::OsStr,
    label: &str,
    limit: u64,
) -> crate::Result<Vec<u8>> {
    let mut file = match directory.open_child(name, label)? {
        SecureNode::File(file) => file,
        SecureNode::Directory(child) => {
            return Err(ModError::Other(format!(
                "{label} must be a regular file: {}",
                child.path().display()
            )))
        }
    };
    if file.len() > limit {
        return Err(ModError::Other(format!(
            "{label} exceeds the {limit} byte limit: {}",
            file.path().display()
        )));
    }
    let expected = file.len();
    let mut bytes = Vec::with_capacity(usize::try_from(expected).map_err(|_| {
        ModError::Other(format!(
            "{label} length does not fit memory on this platform"
        ))
    })?);
    (&mut file.file)
        .take(expected + 1)
        .read_to_end(&mut bytes)
        .map_err(crate::io(&format!("reading {label}")))?;
    if bytes.len() as u64 != expected {
        return Err(ModError::Other(format!(
            "{label} changed length while reading: {}",
            file.path().display()
        )));
    }
    file.verify_len(expected, label)?;
    Ok(bytes)
}

fn seal_secure_import_directory(
    root: &SecureDirectory,
    limits: ImportLimits,
) -> crate::Result<EntryPublishSeal> {
    let sidecar = read_secure_child_bounded(
        root,
        std::ffi::OsStr::new(META_FILE),
        "manager-library publish sidecar",
        limits.max_manifest_bytes,
    )?;
    Ok(EntryPublishSeal {
        root_identity: root.identity(),
        tree_sha256: hash_secure_import_tree(root, limits, true)?,
        sidecar_sha256: digest_hex(Sha256::digest(&sidecar).into()),
    })
}

#[cfg(unix)]
fn sync_secure_tree(root: &SecureDirectory) -> crate::Result<()> {
    let mut names = root
        .read_dir("staged import durability tree")?
        .map(|entry| {
            entry
                .map(|entry| entry.file_name())
                .map_err(crate::io("reading staged import durability entry"))
        })
        .collect::<crate::Result<Vec<_>>>()?;
    names.sort();
    for name in names {
        match root.open_child(&name, "staged import durability entry")? {
            SecureNode::File(file) => file
                .file
                .sync_all()
                .map_err(crate::io("syncing staged import regular file"))?,
            SecureNode::Directory(directory) => sync_secure_tree(&directory)?,
        }
    }
    root.sync_after_mutation("staged import durability tree")
}

fn seal_library_entry(
    entry: &LibraryEntry,
    limits: ImportLimits,
) -> crate::Result<EntryPublishSeal> {
    let mut remaining = MAX_IDENTITY_SIDECAR_BYTES;
    let inspection = entry
        .read_identity_sidecar_bounded_classified(&mut remaining)
        .map_err(MetaReadFailure::into_mod_error)?;
    let manager = inspection.manager?;
    if let Some(identity) = manager
        .as_ref()
        .and_then(|manager| manager.import_identity.as_ref())
    {
        validate_import_identity(identity)?;
    }
    Ok(EntryPublishSeal {
        root_identity: entry.secure_directory().identity(),
        tree_sha256: hash_library_entry_tree(entry, limits)?,
        sidecar_sha256: inspection.sidecar_sha256,
    })
}

fn verify_publish_expectation(
    library: &LibraryRoot,
    staging: &SecureDirectory,
    entry_dir: &Path,
    expectation: &PublishExpectation,
) -> crate::Result<()> {
    let staged = seal_secure_import_directory(staging, expectation.limits)?;
    if staged != expectation.staged {
        return Err(ModError::Other(format!(
            "staged import changed after identity decision: {}",
            staging.path().display()
        )));
    }
    match &expectation.current {
        Some(expected) => {
            let id = entry_dir
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    ModError::Other(format!(
                        "selected manager-library entry id is not valid Unicode: {}",
                        entry_dir.display()
                    ))
                })?;
            let current = seal_library_entry(&library.entry(id)?, expectation.limits)?;
            if &current != expected {
                return Err(ModError::Other(format!(
                    "selected manager-library entry changed after identity decision: {}",
                    entry_dir.display()
                )));
            }
        }
        None => {
            if metadata_if_present(entry_dir)?.is_some() {
                return Err(ModError::Other(format!(
                    "new manager-library entry appeared after identity decision: {}",
                    entry_dir.display()
                )));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReplacementPhase {
    Prepared,
    PreviousMoved,
    Promoted,
    Restored,
    Quarantined,
}

#[cfg(unix)]
fn replacement_atomic_temp_name(final_name: &str) -> String {
    format!("{final_name}{REPLACEMENT_ATOMIC_TEMP_SUFFIX}")
}

#[cfg(unix)]
fn replacement_atomic_temp_names() -> Vec<String> {
    let mut names = vec![replacement_atomic_temp_name(REPLACEMENT_STATE_FILE)];
    names.extend(
        [
            ReplacementPhase::PreviousMoved,
            ReplacementPhase::Promoted,
            ReplacementPhase::Restored,
            ReplacementPhase::Quarantined,
        ]
        .into_iter()
        .map(|phase| replacement_atomic_temp_name(phase.marker().expect("phase marker"))),
    );
    names
}

impl ReplacementPhase {
    fn marker(self) -> Option<&'static str> {
        match self {
            Self::Prepared => None,
            Self::PreviousMoved => Some("phase-previous-moved"),
            Self::Promoted => Some("phase-promoted"),
            Self::Restored => Some("phase-restored"),
            Self::Quarantined => Some("phase-quarantined"),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ReplacementState {
    format: u32,
    id: String,
    phase: ReplacementPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expected_previous: Option<EntryPublishSeal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expected_staged: Option<EntryPublishSeal>,
    #[serde(default)]
    verification_pending: bool,
}

fn validate_replacement_state(state: &ReplacementState, location: &Path) -> crate::Result<()> {
    let valid = crate::is_safe_mod_name(&state.id)
        && match state.format {
            1 => {
                !state.verification_pending
                    && state.expected_previous.is_none()
                    && state.expected_staged.is_none()
            }
            2 => {
                state.phase == ReplacementPhase::Prepared
                    && state.verification_pending
                    && state.expected_staged.is_some()
            }
            _ => false,
        };
    if !valid {
        return Err(ModError::Other(format!(
            "invalid replacement state in {}",
            location.display()
        )));
    }
    Ok(())
}

fn validate_replacement_phase_consistency(
    state: &ReplacementState,
    phase: ReplacementPhase,
    location: &Path,
) -> crate::Result<()> {
    if state.format == 2
        && state.expected_previous.is_none()
        && matches!(
            phase,
            ReplacementPhase::PreviousMoved | ReplacementPhase::Restored
        )
    {
        return Err(ModError::Other(format!(
            "replacement phase {phase:?} requires a sealed previous entry in {}",
            location.display()
        )));
    }
    Ok(())
}

#[cfg(any(not(unix), test))]
#[derive(Debug)]
struct ReplacementTransaction {
    root: PathBuf,
    state: ReplacementState,
}

#[cfg(any(not(unix), test))]
impl ReplacementTransaction {
    fn begin(
        library_dir: &Path,
        id: &str,
        expectation: Option<&PublishExpectation>,
    ) -> crate::Result<Self> {
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
            format: u32::from(expectation.is_some()) + 1,
            id: id.to_owned(),
            phase: ReplacementPhase::Prepared,
            expected_previous: expectation.and_then(|value| value.current.clone()),
            expected_staged: expectation.map(|value| value.staged.clone()),
            verification_pending: expectation.is_some(),
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

    #[cfg(not(unix))]
    fn from_state(root: PathBuf, state: ReplacementState) -> Self {
        Self { root, state }
    }

    fn backup(&self) -> PathBuf {
        self.root.join(REPLACEMENT_BACKUP_DIR)
    }

    fn quarantine(&self) -> PathBuf {
        self.root.join(REPLACEMENT_QUARANTINE_DIR)
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
        run_replacement_mark_failure(phase)?;
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

    #[cfg(not(unix))]
    fn phase(&self) -> crate::Result<ReplacementPhase> {
        for phase in [
            ReplacementPhase::Quarantined,
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
            ReplacementPhase::Quarantined
                .marker()
                .expect("phase marker"),
        ];
        for entry in std::fs::read_dir(&self.root)
            .map_err(crate::io("reading replacement transaction for cleanup"))?
        {
            let entry = entry.map_err(crate::io("reading replacement cleanup entry"))?;
            let name = entry.file_name();
            let known = name == REPLACEMENT_STATE_FILE
                || name == REPLACEMENT_BACKUP_DIR
                || name == REPLACEMENT_QUARANTINE_DIR
                || phase_markers.iter().any(|marker| name == *marker);
            if !known {
                return Err(ModError::Other(format!(
                    "replacement transaction contains an unexpected path: {}",
                    entry.path().display()
                )));
            }
        }

        if metadata_if_present(&self.root.join(REPLACEMENT_QUARANTINE_DIR))?.is_some() {
            return Err(ModError::Other(format!(
                "refusing to clean a quarantined replacement transaction: {}",
                self.root.display()
            )));
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

#[cfg(unix)]
static UNIX_REPLACEMENT_SEQUENCE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

/// Unix transaction whose namespace operations are all relative to retained directory
/// descriptors. `display_root` is diagnostics only and is never reopened.
#[cfg(unix)]
#[derive(Debug)]
struct UnixReplacementTransaction {
    library: LibraryRoot,
    root: SecureDirectory,
    root_name: std::ffi::OsString,
    state: ReplacementState,
}

#[cfg(unix)]
impl UnixReplacementTransaction {
    fn begin(
        library: &LibraryRoot,
        id: &str,
        expectation: &PublishExpectation,
    ) -> crate::Result<Self> {
        if !crate::is_safe_mod_name(id) {
            return Err(ModError::Other(format!(
                "invalid replacement entry id {id:?}"
            )));
        }
        let mut claimed = None;
        for _ in 0..128 {
            let sequence =
                UNIX_REPLACEMENT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let name = std::ffi::OsString::from(format!(
                "{REPLACEMENT_PREFIX}{}-{sequence:016x}",
                std::process::id()
            ));
            if let Some(root) = library
                .secure_directory()
                .try_create_child_directory_new(&name, "manager replacement transaction")?
            {
                claimed = Some((name, root));
                break;
            }
        }
        let (root_name, root) = claimed.ok_or_else(|| {
            ModError::Other("could not claim a unique manager replacement transaction".into())
        })?;
        let transaction = Self {
            library: library.clone(),
            root,
            root_name,
            state: ReplacementState {
                format: 2,
                id: id.to_owned(),
                phase: ReplacementPhase::Prepared,
                expected_previous: expectation.current.clone(),
                expected_staged: Some(expectation.staged.clone()),
                verification_pending: true,
            },
        };
        if let Err(error) = transaction.write_initial_state() {
            let cleanup = transaction.remove_empty_root().err();
            return Err(combine_replacement_errors(
                error,
                cleanup,
                transaction.root.path(),
            ));
        }
        Ok(transaction)
    }

    fn from_state(
        library: &LibraryRoot,
        root_name: std::ffi::OsString,
        root: SecureDirectory,
        state: ReplacementState,
    ) -> Self {
        Self {
            library: library.clone(),
            root,
            root_name,
            state,
        }
    }

    fn write_initial_state(&self) -> crate::Result<()> {
        self.write_new_json_file(REPLACEMENT_STATE_FILE, &self.state, "replacement state")?;
        self.root
            .sync_after_mutation("manager replacement transaction")?;
        self.library.sync_after_mutation()
    }

    fn write_new_json_file<T: serde::Serialize>(
        &self,
        name: &str,
        value: &T,
        label: &str,
    ) -> crate::Result<()> {
        let final_name = std::ffi::OsStr::new(name);
        if self.root.contains_child(final_name, label)? {
            return Err(ModError::Other(format!(
                "refusing to replace existing {label} {name:?}"
            )));
        }
        let temp_name = replacement_atomic_temp_name(name);
        // A process may have stopped after syncing the private temporary file but before rename.
        // Under the library inode lock it is safe to discard only this exact, known temp name and
        // retry; the final journal/phase name was never partially visible.
        self.remove_file_if_present(&temp_name, "incomplete replacement JSON")?;
        let bytes = serde_json::to_vec(value)?;
        let (mut file, _) = self
            .root
            .create_child_file_new(std::ffi::OsStr::new(&temp_name), label)?;
        file.write_all(&bytes)
            .map_err(crate::io(&format!("writing {label}")))?;
        file.sync_all()
            .map_err(crate::io(&format!("syncing {label}")))?;
        drop(file);
        self.root.rename_child_to(
            std::ffi::OsStr::new(&temp_name),
            &self.root,
            final_name,
            label,
        )?;
        self.root.sync_after_mutation(label)
    }

    fn mark(&self, phase: ReplacementPhase) -> crate::Result<()> {
        run_replacement_mark_failure(phase)?;
        let Some(marker) = phase.marker() else {
            return Ok(());
        };
        let name = std::ffi::OsStr::new(marker);
        if self.root.contains_child(name, "replacement phase")? {
            let bytes = read_secure_child_bounded(
                &self.root,
                name,
                "replacement phase",
                REPLACEMENT_STATE_MAX_BYTES,
            )?;
            let existing: ReplacementPhase = serde_json::from_slice(&bytes)?;
            if existing != phase {
                return Err(ModError::Other(format!(
                    "replacement phase marker {marker:?} has the wrong value"
                )));
            }
            return Ok(());
        }
        self.write_new_json_file(marker, &phase, "replacement phase")?;
        self.root.sync_after_mutation("replacement phase")
    }

    fn phase(&self) -> crate::Result<ReplacementPhase> {
        for phase in [
            ReplacementPhase::Quarantined,
            ReplacementPhase::Restored,
            ReplacementPhase::Promoted,
            ReplacementPhase::PreviousMoved,
        ] {
            let marker = phase.marker().expect("non-prepared phase");
            if self
                .root
                .contains_child(std::ffi::OsStr::new(marker), "replacement phase")?
            {
                let bytes = read_secure_child_bounded(
                    &self.root,
                    std::ffi::OsStr::new(marker),
                    "replacement phase",
                    REPLACEMENT_STATE_MAX_BYTES,
                )?;
                let recorded: ReplacementPhase = serde_json::from_slice(&bytes)?;
                if recorded != phase {
                    return Err(ModError::Other(format!(
                        "replacement phase marker {marker:?} has the wrong value"
                    )));
                }
                return Ok(phase);
            }
        }
        Ok(self.state.phase)
    }

    fn backup(&self) -> crate::Result<Option<SecureDirectory>> {
        self.root.open_optional_child_directory(
            std::ffi::OsStr::new(REPLACEMENT_BACKUP_DIR),
            "previous replacement entry",
        )
    }

    fn quarantine(&self) -> crate::Result<Option<SecureDirectory>> {
        self.root.open_optional_child_directory(
            std::ffi::OsStr::new(REPLACEMENT_QUARANTINE_DIR),
            "quarantined replacement entry",
        )
    }

    fn remove_file_if_present(&self, name: &str, label: &str) -> crate::Result<()> {
        let name = std::ffi::OsStr::new(name);
        let Some(node) = self.root.open_optional_child(name, label)? else {
            return Ok(());
        };
        let file = match node {
            SecureNode::File(file) => file,
            SecureNode::Directory(directory) => {
                return Err(ModError::Other(format!(
                    "{label} is not a regular file: {}",
                    directory.path().display()
                )))
            }
        };
        let identity = file.identity();
        drop(file);
        self.root
            .remove_child_file_if_identity(name, identity, label)
    }

    fn cleanup(&self) -> crate::Result<()> {
        if self.quarantine()?.is_some() {
            return Err(ModError::Other(format!(
                "refusing to clean a quarantined replacement transaction: {}",
                self.root.path().display()
            )));
        }
        let mut names = self
            .root
            .read_dir("replacement transaction for cleanup")?
            .map(|entry| {
                entry
                    .map(|entry| entry.file_name())
                    .map_err(crate::io("reading replacement cleanup entry"))
            })
            .collect::<crate::Result<Vec<_>>>()?;
        names.sort();
        let markers = [
            ReplacementPhase::PreviousMoved.marker().expect("marker"),
            ReplacementPhase::Promoted.marker().expect("marker"),
            ReplacementPhase::Restored.marker().expect("marker"),
            ReplacementPhase::Quarantined.marker().expect("marker"),
        ];
        let atomic_temps = replacement_atomic_temp_names();
        for name in &names {
            let known = name == REPLACEMENT_STATE_FILE
                || name == REPLACEMENT_BACKUP_DIR
                || markers.iter().any(|marker| name == marker)
                || atomic_temps
                    .iter()
                    .any(|temp| name == std::ffi::OsStr::new(temp));
            if !known {
                return Err(ModError::Other(format!(
                    "replacement transaction contains an unexpected path: {}",
                    self.root.path().join(name).display()
                )));
            }
        }
        if self.backup()?.is_some() {
            self.root.remove_child_tree(
                std::ffi::OsStr::new(REPLACEMENT_BACKUP_DIR),
                "previous replacement entry",
            )?;
        }
        for marker in markers {
            self.remove_file_if_present(marker, "replacement phase")?;
        }
        for temp in atomic_temps {
            self.remove_file_if_present(&temp, "incomplete replacement JSON")?;
        }
        self.remove_file_if_present(REPLACEMENT_STATE_FILE, "replacement state")?;
        self.root
            .sync_after_mutation("manager replacement transaction")?;
        self.remove_empty_root()
    }

    fn remove_empty_root(&self) -> crate::Result<()> {
        let identity = self.root.identity();
        self.library
            .secure_directory()
            .remove_child_directory_if_identity(
                &self.root_name,
                identity,
                "manager replacement transaction",
            )
    }
}

#[cfg(any(not(unix), test))]
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

struct LibraryMutationGuard {
    os: LibraryMutationFileGuard,
    _process: std::sync::MutexGuard<'static, ()>,
}

impl LibraryMutationGuard {
    fn path(&self) -> &Path {
        self.os.path()
    }

    fn open_library(&self) -> crate::Result<LibraryRoot> {
        #[cfg(unix)]
        {
            Ok(self.os.retained_library())
        }

        #[cfg(not(unix))]
        {
            let library = LibraryRoot::open(self.os.path())?;
            if library.identity() != self.os.identity() {
                return Err(ModError::Other(format!(
                "manager library changed filesystem identity while its mutation lock was held: {}",
                self.os.path().display()
            )));
            }
            Ok(library)
        }
    }
}

fn library_mutation_lock(library_dir: &Path) -> crate::Result<LibraryMutationGuard> {
    let process = LIBRARY_MUTATION_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let library = LibraryRoot::open(library_dir)?;
    let os = library.acquire_mutation_lock()?;
    run_library_root_swap_hook(os.path());
    run_library_lock_acquired_hook()?;
    Ok(LibraryMutationGuard {
        os,
        _process: process,
    })
}

#[cfg(test)]
fn run_library_lock_acquired_hook() -> crate::Result<()> {
    if let Some(marker) = std::env::var_os(LIBRARY_LOCK_MARKER_ENV) {
        std::fs::write(&marker, b"locked")
            .map_err(crate::io("writing manager-library lock test marker"))?;
    }
    if let Ok(raw) = std::env::var(LIBRARY_LOCK_HOLD_MS_ENV) {
        let milliseconds = raw.parse::<u64>().map_err(|error| {
            ModError::Other(format!(
                "invalid {LIBRARY_LOCK_HOLD_MS_ENV} test value {raw:?}: {error}"
            ))
        })?;
        std::thread::sleep(std::time::Duration::from_millis(milliseconds));
    }
    Ok(())
}

#[cfg(not(test))]
fn run_library_lock_acquired_hook() -> crate::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn activate_staged_entry(
    library_lock: &LibraryMutationGuard,
    staging: &Path,
    entry_dir: &Path,
    id: &str,
    expectation: &PublishExpectation,
) -> crate::Result<()> {
    run_prepublish_race_hook(staging, entry_dir);
    let library = library_lock.open_library()?;
    let root = library.secure_directory();
    let staging_name = staging.file_name().ok_or_else(|| {
        ModError::Other(format!(
            "staged import has no direct-child name: {}",
            staging.display()
        ))
    })?;
    let staged = root
        .open_optional_child_directory(staging_name, "staged import selected for publication")?
        .ok_or_else(|| {
            ModError::Other(format!(
                "staged import is not a child of the locked manager-library inode: {}",
                staging.display()
            ))
        })?;
    if seal_secure_import_directory(&staged, expectation.limits)? != expectation.staged {
        return Err(ModError::Other(format!(
            "staged import changed or moved to another library inode before publication: {}",
            staging.display()
        )));
    }

    let id_name = std::ffi::OsStr::new(id);
    let current = root.open_optional_child_directory(id_name, "current manager-library entry")?;
    match (&current, &expectation.current) {
        (Some(current), Some(expected)) => {
            if seal_secure_import_directory(current, expectation.limits)? != *expected {
                return Err(ModError::Other(format!(
                    "selected manager-library entry changed after identity decision: {}",
                    current.path().display()
                )));
            }
        }
        (None, None) => {}
        (None, Some(_)) => {
            return Err(ModError::Other(format!(
                "selected manager-library entry disappeared before publication: {id:?}"
            )))
        }
        (Some(current), None) => {
            return Err(ModError::Other(format!(
                "new manager-library entry appeared before publication: {}",
                current.path().display()
            )))
        }
    }
    sync_secure_tree(&staged)?;
    drop(staged);
    drop(current);

    let transaction = UnixReplacementTransaction::begin(&library, id, expectation)?;
    if expectation.current.is_none() {
        if let Err(error) = root.rename_child_to(
            staging_name,
            root,
            id_name,
            "publishing new manager-library entry",
        ) {
            let cleanup = transaction.cleanup().err();
            return Err(combine_replacement_errors(
                error,
                cleanup,
                transaction.root.path(),
            ));
        }
        run_post_create_rename_hook(entry_dir);
        let observed = library
            .entry(id)
            .and_then(|entry| seal_library_entry(&entry, expectation.limits));
        if !matches!(&observed, Ok(seal) if seal == &expectation.staged) {
            let quarantine = root
                .rename_child_to(
                    id_name,
                    &transaction.root,
                    std::ffi::OsStr::new(REPLACEMENT_QUARANTINE_DIR),
                    "quarantining failed new manager-library entry",
                )
                .and_then(|()| transaction.mark(ReplacementPhase::Quarantined))
                .err();
            return Err(quarantined_replacement_error(
                observed
                    .err()
                    .unwrap_or_else(|| ModError::Other("new entry seal mismatch".into())),
                quarantine,
                transaction.root.path(),
                "new manager-library entry failed its post-rename seal",
            ));
        }
        transaction.mark(ReplacementPhase::Promoted)?;
        return transaction.cleanup();
    }

    if let Err(error) = root.rename_child_to(
        id_name,
        &transaction.root,
        std::ffi::OsStr::new(REPLACEMENT_BACKUP_DIR),
        "moving previous manager-library entry into recovery transaction",
    ) {
        let cleanup = transaction.cleanup().err();
        return Err(combine_replacement_errors(
            error,
            cleanup,
            transaction.root.path(),
        ));
    }
    let expected_previous = expectation
        .current
        .as_ref()
        .expect("replacement branch has previous seal");
    let backup = transaction.backup()?.ok_or_else(|| {
        ModError::Other("previous entry disappeared after anchored rename".into())
    })?;
    if seal_secure_import_directory(&backup, expectation.limits)? != *expected_previous {
        let marker_error = transaction.mark(ReplacementPhase::Quarantined).err();
        return Err(quarantined_replacement_error(
            ModError::Other("previous entry failed its post-rename seal".into()),
            marker_error,
            transaction.root.path(),
            "previous entry failed its post-rename seal before promotion",
        ));
    }
    drop(backup);
    if let Err(error) = transaction.mark(ReplacementPhase::PreviousMoved) {
        return Err(rollback_previous_unix(error, &transaction, root, id_name));
    }

    if let Err(error) = root.rename_child_to(
        staging_name,
        root,
        id_name,
        "promoting staged manager-library entry",
    ) {
        return Err(rollback_previous_unix(error, &transaction, root, id_name));
    }
    run_post_promote_rename_hook(entry_dir);
    let live = library.entry(id)?;
    let promoted_seal = seal_library_entry(&live, expectation.limits);
    drop(live);
    if !matches!(&promoted_seal, Ok(seal) if seal == &expectation.staged) {
        let quarantine = (|| -> crate::Result<()> {
            root.rename_child_to(
                id_name,
                &transaction.root,
                std::ffi::OsStr::new(REPLACEMENT_QUARANTINE_DIR),
                "quarantining failed promoted manager-library entry",
            )?;
            let backup = transaction.backup()?.ok_or_else(|| {
                ModError::Other("verified previous entry disappeared during quarantine".into())
            })?;
            if seal_secure_import_directory(&backup, expectation.limits)? != *expected_previous {
                return Err(ModError::Other(
                    "previous entry changed before quarantine restore".into(),
                ));
            }
            drop(backup);
            transaction.root.rename_child_to(
                std::ffi::OsStr::new(REPLACEMENT_BACKUP_DIR),
                root,
                id_name,
                "restoring previous entry after failed promotion",
            )?;
            transaction.mark(ReplacementPhase::Quarantined)
        })()
        .err();
        return Err(quarantined_replacement_error(
            promoted_seal
                .err()
                .unwrap_or_else(|| ModError::Other("promoted entry seal mismatch".into())),
            quarantine,
            transaction.root.path(),
            "promoted entry failed its post-rename seal",
        ));
    }
    transaction.mark(ReplacementPhase::Promoted)?;
    transaction.cleanup()
}

#[cfg(unix)]
fn rollback_previous_unix(
    original: ModError,
    transaction: &UnixReplacementTransaction,
    library: &SecureDirectory,
    id: &std::ffi::OsStr,
) -> ModError {
    let rollback = (|| -> crate::Result<()> {
        transaction.root.rename_child_to(
            std::ffi::OsStr::new(REPLACEMENT_BACKUP_DIR),
            library,
            id,
            "restoring previous manager-library entry",
        )?;
        transaction.mark(ReplacementPhase::Restored)?;
        transaction.cleanup()
    })();
    combine_replacement_errors(original, rollback.err(), transaction.root.path())
}

#[cfg(not(unix))]
fn activate_staged_entry(
    library_lock: &LibraryMutationGuard,
    staging: &Path,
    entry_dir: &Path,
    id: &str,
    expectation: &PublishExpectation,
) -> crate::Result<()> {
    run_prepublish_race_hook(staging, entry_dir);
    let library = library_lock.open_library()?;
    let staged_directory = open_directory_nofollow(staging, "staged import before publication")?;
    verify_publish_expectation(&library, &staged_directory, entry_dir, expectation)?;
    drop(staged_directory);
    drop(library);
    let mut rename = rename_replacement_path;
    let mut sync = sync_staged_tree;
    activate_staged_entry_with_sync_inner(
        library_lock.path(),
        staging,
        entry_dir,
        id,
        &mut rename,
        &mut sync,
        Some(expectation),
    )
}

#[cfg(test)]
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
    activate_staged_entry_with_sync_inner(
        library_dir,
        staging,
        entry_dir,
        id,
        rename,
        &mut sync,
        None,
    )
}

#[cfg(test)]
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
    activate_staged_entry_with_sync_inner(
        library_dir,
        staging,
        entry_dir,
        id,
        rename,
        sync_staged,
        None,
    )
}

#[cfg(any(not(unix), test))]
fn activate_staged_entry_with_sync_inner<F, S>(
    library_dir: &Path,
    staging: &Path,
    entry_dir: &Path,
    id: &str,
    rename: &mut F,
    sync_staged: &mut S,
    expectation: Option<&PublishExpectation>,
) -> crate::Result<()>
where
    F: FnMut(&Path, &Path) -> std::io::Result<()>,
    S: FnMut(&Path) -> crate::Result<()>,
{
    // Content durability comes first. In particular, do not move the previous live entry into a
    // recovery transaction until every staged regular file and directory has reached its platform
    // durability barrier. A sync failure therefore leaves the old entry completely untouched.
    sync_staged(staging)?;

    let previous_metadata = metadata_if_present(entry_dir)?;
    if let Some(expectation) = expectation {
        match (previous_metadata.is_some(), expectation.current.is_some()) {
            (false, true) => {
                return Err(ModError::Other(format!(
                    "selected manager-library entry disappeared before publication: {}",
                    entry_dir.display()
                )))
            }
            (true, false) => {
                return Err(ModError::Other(format!(
                    "new manager-library entry appeared before publication: {}",
                    entry_dir.display()
                )))
            }
            _ => {}
        }
    }
    let Some(previous_metadata) = previous_metadata else {
        // Create publication also receives a durable verification-pending journal before its
        // first rename. Recovery can therefore distinguish a verified create from an unsealed
        // object even if the quarantine marker itself cannot be written.
        let transaction = ReplacementTransaction::begin(library_dir, id, expectation)?;
        if let Err(error) = rename(staging, entry_dir) {
            let original = crate::io("activating library entry")(error);
            let cleanup = transaction.cleanup();
            return Err(combine_replacement_errors(
                original,
                cleanup.err(),
                &transaction.root,
            ));
        }
        run_post_create_rename_hook(entry_dir);
        if let Err(error) = sync_replacement_directory(library_dir) {
            let quarantine = quarantine_created_entry(&transaction, entry_dir, rename).err();
            return Err(quarantined_replacement_error(
                error,
                quarantine,
                &transaction.root,
                "new manager-library entry could not be durably verified",
            ));
        }
        if let Some(expectation) = expectation {
            if let Err(error) =
                verify_post_rename_seal(entry_dir, &expectation.staged, expectation.limits)
            {
                let quarantine = quarantine_created_entry(&transaction, entry_dir, rename).err();
                return Err(quarantined_replacement_error(
                    error,
                    quarantine,
                    &transaction.root,
                    "new manager-library entry failed its post-rename seal",
                ));
            }
        }
        transaction.mark(ReplacementPhase::Promoted)?;
        return transaction.cleanup();
    };
    if import_metadata_is_link(&previous_metadata) || !previous_metadata.is_dir() {
        return Err(ModError::Other(format!(
            "existing library entry is not a real directory: {}",
            entry_dir.display()
        )));
    }

    let transaction = ReplacementTransaction::begin(library_dir, id, expectation)?;
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
    if let Some(expectation) = expectation {
        if let Some(expected) = expectation.current.as_ref() {
            if let Err(error) =
                verify_post_rename_seal(&transaction.backup(), expected, expectation.limits)
            {
                let quarantine = transaction.mark(ReplacementPhase::Quarantined).err();
                return Err(quarantined_replacement_error(
                    error,
                    quarantine,
                    &transaction.root,
                    "previous entry failed its post-rename seal before promotion",
                ));
            }
        }
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
    run_post_promote_rename_hook(entry_dir);
    if let Err(error) = sync_replacement_directory(library_dir) {
        if let Some(expectation) = expectation {
            let quarantine = quarantine_promoted_entry(
                &transaction,
                entry_dir,
                rename,
                expectation.current.as_ref(),
                expectation.limits,
            )
            .err();
            return Err(quarantined_replacement_error(
                error,
                quarantine,
                &transaction.root,
                "promoted entry could not be durably verified",
            ));
        }
        let phase_error = transaction.mark(ReplacementPhase::Promoted).err();
        return Err(promoted_replacement_error(
            error,
            phase_error,
            &transaction.root,
        ));
    }
    if let Some(expectation) = expectation {
        if let Err(error) =
            verify_post_rename_seal(entry_dir, &expectation.staged, expectation.limits)
        {
            let quarantine = quarantine_promoted_entry(
                &transaction,
                entry_dir,
                rename,
                expectation.current.as_ref(),
                expectation.limits,
            )
            .err();
            return Err(quarantined_replacement_error(
                error,
                quarantine,
                &transaction.root,
                "promoted entry failed its post-rename seal",
            ));
        }
    }
    if let Err(error) = transaction.mark(ReplacementPhase::Promoted) {
        return Err(promoted_replacement_error(error, None, &transaction.root));
    }

    // Once `promoted` is durable, recovery will always retain the new live entry and finish
    // deleting the old copy if this cleanup is interrupted.
    transaction.cleanup()
}

#[cfg(not(any(windows, unix)))]
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
#[cfg(any(not(unix), test))]
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

#[cfg(any(all(unix, test), not(any(unix, windows))))]
fn sync_staged_regular_file(path: &Path) -> crate::Result<()> {
    std::fs::File::open(path)
        .map_err(crate::io(&format!(
            "opening staged file for durable sync {}",
            path.display()
        )))?
        .sync_all()
        .map_err(crate::io("syncing staged regular file"))
}

#[cfg(any(not(unix), test))]
fn quarantine_created_entry<F>(
    transaction: &ReplacementTransaction,
    entry_dir: &Path,
    rename: &mut F,
) -> crate::Result<()>
where
    F: FnMut(&Path, &Path) -> std::io::Result<()>,
{
    rename(entry_dir, &transaction.quarantine())
        .map_err(crate::io("moving failed new entry into durable quarantine"))?;
    let library_dir = transaction
        .root
        .parent()
        .ok_or_else(|| ModError::Other("replacement root has no parent".into()))?;
    sync_replacement_directory(library_dir)?;
    sync_replacement_directory(&transaction.root)?;
    transaction.mark(ReplacementPhase::Quarantined)
}

#[cfg(any(not(unix), test))]
fn quarantine_promoted_entry<F>(
    transaction: &ReplacementTransaction,
    entry_dir: &Path,
    rename: &mut F,
    expected_previous: Option<&EntryPublishSeal>,
    limits: ImportLimits,
) -> crate::Result<()>
where
    F: FnMut(&Path, &Path) -> std::io::Result<()>,
{
    rename(entry_dir, &transaction.quarantine()).map_err(crate::io(
        "moving failed promoted entry into durable quarantine",
    ))?;
    let library_dir = transaction
        .root
        .parent()
        .ok_or_else(|| ModError::Other("replacement root has no parent".into()))?;
    sync_replacement_directory(library_dir)?;
    sync_replacement_directory(&transaction.root)?;
    if let Some(expected) = expected_previous {
        verify_post_rename_seal(&transaction.backup(), expected, limits)?;
        rename(&transaction.backup(), entry_dir).map_err(crate::io(
            "restoring verified previous entry after failed promotion",
        ))?;
        sync_replacement_directory(library_dir)?;
    }
    transaction.mark(ReplacementPhase::Quarantined)
}

#[cfg(any(not(unix), test))]
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

#[cfg(any(not(unix), test))]
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

fn quarantined_replacement_error(
    original: ModError,
    marker_error: Option<ModError>,
    transaction_root: &Path,
    context: &str,
) -> ModError {
    let marker_detail = marker_error
        .map(|error| format!("; recording quarantine also failed: {error}"))
        .unwrap_or_default();
    ModError::Other(format!(
        "{context}: {original}{marker_detail}; the public entry is fail-closed and recovery evidence was retained at {}",
        transaction_root.display()
    ))
}

#[cfg(any(not(unix), test))]
fn verify_post_rename_seal(
    path: &Path,
    expected: &EntryPublishSeal,
    limits: ImportLimits,
) -> crate::Result<()> {
    let observed = seal_import_path(path, limits)?;
    if &observed != expected {
        return Err(ModError::Other(format!(
            "manager-library object changed across rename: {}",
            path.display()
        )));
    }
    Ok(())
}

fn recover_interrupted_replacements_locked(
    library_lock: &LibraryMutationGuard,
) -> crate::Result<()> {
    let library = library_lock.open_library()?;
    #[cfg(unix)]
    {
        recover_interrupted_replacements_unix(&library)
    }
    #[cfg(not(unix))]
    {
        // Cooperative Windows writers share the kernel lock. Reopening and comparing FileIdInfo
        // immediately before pathname recovery detects an ambient root substitution; the durable
        // transaction seals below remain the authority for every payload decision.
        drop(library);
        recover_interrupted_replacements(library_lock.path())
    }
}

#[cfg(unix)]
fn recover_interrupted_replacements_unix(library: &LibraryRoot) -> crate::Result<()> {
    let mut names = Vec::new();
    for entry in library.read_dir()? {
        let entry = entry.map_err(crate::io("reading replacement transaction entry"))?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(REPLACEMENT_PREFIX) {
            names.push(name);
        }
    }
    names.sort();
    for name in names {
        let root = library
            .secure_directory()
            .open_optional_child_directory(&name, "replacement transaction")?
            .ok_or_else(|| {
                ModError::Other(format!(
                    "replacement transaction disappeared while locked: {name:?}"
                ))
            })?;
        recover_replacement_unix(library, name, root)?;
    }
    Ok(())
}

#[cfg(unix)]
fn recover_replacement_unix(
    library: &LibraryRoot,
    root_name: std::ffi::OsString,
    root: SecureDirectory,
) -> crate::Result<()> {
    let state_name = std::ffi::OsStr::new(REPLACEMENT_STATE_FILE);
    if !root.contains_child(state_name, "replacement state")? {
        let pending_name = replacement_atomic_temp_name(REPLACEMENT_STATE_FILE);
        if root.contains_child(
            std::ffi::OsStr::new(&pending_name),
            "incomplete replacement state",
        )? {
            let names = root
                .read_dir("unpublished replacement state")?
                .map(|entry| {
                    entry
                        .map(|entry| entry.file_name())
                        .map_err(crate::io("reading unpublished replacement state child"))
                })
                .collect::<crate::Result<Vec<_>>>()?;
            if names.len() != 1 || names[0] != std::ffi::OsStr::new(&pending_name) {
                return Err(ModError::Other(format!(
                    "unpublished replacement state has unexpected recovery objects: {}",
                    root.path().display()
                )));
            }
            let pending = match root.open_child(
                std::ffi::OsStr::new(&pending_name),
                "incomplete replacement state",
            )? {
                SecureNode::File(file) => file,
                SecureNode::Directory(directory) => {
                    return Err(ModError::Other(format!(
                        "incomplete replacement state is not a regular file: {}",
                        directory.path().display()
                    )))
                }
            };
            let pending_identity = pending.identity();
            drop(pending);
            root.remove_child_file_if_identity(
                std::ffi::OsStr::new(&pending_name),
                pending_identity,
                "incomplete replacement state",
            )?;
            let root_identity = root.identity();
            drop(root);
            return library
                .secure_directory()
                .remove_child_directory_if_identity(
                    &root_name,
                    root_identity,
                    "unpublished replacement transaction",
                );
        }
        return recover_legacy_replacement_unix(library, root_name, root);
    }
    let state: ReplacementState = serde_json::from_slice(&read_secure_child_bounded(
        &root,
        state_name,
        "replacement state",
        REPLACEMENT_STATE_MAX_BYTES,
    )?)?;
    validate_replacement_state(&state, root.path())?;
    let transaction = UnixReplacementTransaction::from_state(library, root_name, root, state);
    let phase = transaction.phase()?;
    validate_replacement_phase_consistency(&transaction.state, phase, transaction.root.path())?;
    if phase == ReplacementPhase::Quarantined || transaction.quarantine()?.is_some() {
        return Err(ModError::Other(format!(
            "replacement transaction is quarantined; recovery evidence was retained at {}",
            transaction.root.path().display()
        )));
    }
    let live = library.secure_directory().open_optional_child_directory(
        std::ffi::OsStr::new(&transaction.state.id),
        "live manager-library entry during recovery",
    )?;
    let backup = transaction.backup()?;
    if transaction.state.verification_pending {
        return recover_verification_pending_replacement_unix(transaction, phase, live, backup);
    }

    match (live, backup) {
        (Some(_), Some(_)) | (Some(_), None) => transaction.cleanup(),
        (None, Some(backup)) => {
            drop(backup);
            transaction.root.rename_child_to(
                std::ffi::OsStr::new(REPLACEMENT_BACKUP_DIR),
                library.secure_directory(),
                std::ffi::OsStr::new(&transaction.state.id),
                "restoring interrupted replacement",
            )?;
            transaction.mark(ReplacementPhase::Restored)?;
            transaction.cleanup()
        }
        (None, None) => Err(ModError::Other(format!(
            "cannot recover interrupted replacement {phase:?} for {:?}: both live and backup entries are missing (state at {})",
            transaction.state.id,
            transaction.root.path().display()
        ))),
    }
}

#[cfg(unix)]
fn recover_verification_pending_replacement_unix(
    transaction: UnixReplacementTransaction,
    phase: ReplacementPhase,
    live: Option<SecureDirectory>,
    backup: Option<SecureDirectory>,
) -> crate::Result<()> {
    let expected_staged = transaction
        .state
        .expected_staged
        .as_ref()
        .expect("verification-pending state was validated");
    let live_seal = live
        .as_ref()
        .map(|entry| {
            run_recovery_seal_failure_hook()?;
            seal_secure_import_directory(entry, DEFAULT_IMPORT_LIMITS)
        })
        .transpose()?;
    let live_matches = live_seal
        .as_ref()
        .is_some_and(|observed| observed == expected_staged);
    let expected_previous = transaction.state.expected_previous.as_ref();
    let live_present = live.is_some();
    let backup_present = backup.is_some();
    if live_matches && (expected_previous.is_some() || !backup_present) {
        // The durable staged seal alone proves promotion completed. Cleanup may already have
        // removed any subset of the previous tree, including its sidecar, so do not inspect that
        // manager-owned remainder before recording and completing the verified promotion.
        drop(live);
        drop(backup);
        transaction.mark(ReplacementPhase::Promoted)?;
        return transaction.cleanup();
    }
    let live_previous_matches = match (live_seal.as_ref(), expected_previous) {
        (Some(observed), Some(expected)) => observed == expected,
        _ => false,
    };
    let backup_seal = backup
        .as_ref()
        .map(|entry| {
            run_recovery_seal_failure_hook()?;
            seal_secure_import_directory(entry, DEFAULT_IMPORT_LIMITS)
        })
        .transpose()?;
    let backup_matches = match (backup_seal.as_ref(), expected_previous) {
        (None, None) => true,
        (Some(observed), Some(expected)) => observed == expected,
        _ => false,
    };
    drop(live);
    drop(backup);

    match (live_present, backup_present, expected_previous) {
        (false, false, None) if phase == ReplacementPhase::Prepared => {
            // A create journals before its first rename. If the process stopped in that window,
            // there is no public or recovery object to classify and only the empty journal may be
            // removed. Any unpromoted `.staging-*` directory remains non-public.
            transaction.cleanup()
        }
        (true, false, Some(_)) if live_previous_matches => {
            // Rollback restores the verified previous object before recording `restored`. A crash
            // or marker error in that narrow interval is reconstructed from the durable seal.
            transaction.mark(ReplacementPhase::Restored)?;
            transaction.cleanup()
        }
        (false, true, Some(_)) if backup_matches => {
            transaction.root.rename_child_to(
                std::ffi::OsStr::new(REPLACEMENT_BACKUP_DIR),
                transaction.library.secure_directory(),
                std::ffi::OsStr::new(&transaction.state.id),
                "restoring verified interrupted replacement",
            )?;
            transaction.mark(ReplacementPhase::Restored)?;
            transaction.cleanup()
        }
        _ => quarantine_mismatched_recovery_unix(&transaction, phase, live_present, backup_matches),
    }
}

#[cfg(unix)]
fn quarantine_mismatched_recovery_unix(
    transaction: &UnixReplacementTransaction,
    phase: ReplacementPhase,
    live_present: bool,
    backup_matches: bool,
) -> crate::Result<()> {
    let original = ModError::Other(format!(
        "verification-pending replacement objects do not match their durable seals (phase {phase:?})"
    ));
    let safety = (|| -> crate::Result<()> {
        let id = std::ffi::OsStr::new(&transaction.state.id);
        if live_present {
            transaction.library.secure_directory().rename_child_to(
                id,
                &transaction.root,
                std::ffi::OsStr::new(REPLACEMENT_QUARANTINE_DIR),
                "quarantining mismatched live manager-library entry during recovery",
            )?;
        }
        if transaction.state.expected_previous.is_some() && backup_matches {
            transaction.root.rename_child_to(
                std::ffi::OsStr::new(REPLACEMENT_BACKUP_DIR),
                transaction.library.secure_directory(),
                id,
                "restoring verified previous manager-library entry during recovery",
            )?;
        }
        Ok(())
    })();
    if let Err(safety_error) = safety {
        return Err(ModError::Other(format!(
            "refusing ambiguous replacement recovery: {original}; fail-closed quarantine/restore failed: {safety_error}; recovery evidence retained at {}",
            transaction.root.path().display()
        )));
    }
    let marker_error = transaction.mark(ReplacementPhase::Quarantined).err();
    Err(quarantined_replacement_error(
        original,
        marker_error,
        transaction.root.path(),
        "refusing ambiguous replacement recovery",
    ))
}

#[cfg(unix)]
fn recover_legacy_replacement_unix(
    library: &LibraryRoot,
    root_name: std::ffi::OsString,
    root: SecureDirectory,
) -> crate::Result<()> {
    let meta_name = std::ffi::OsStr::new(META_FILE);
    if !root.contains_child(meta_name, "legacy replacement sidecar")? {
        if root
            .read_dir("incomplete replacement transaction")?
            .next()
            .is_none()
        {
            let identity = root.identity();
            drop(root);
            return library
                .secure_directory()
                .remove_child_directory_if_identity(
                    &root_name,
                    identity,
                    "empty replacement transaction",
                );
        }
        return Err(ModError::Other(format!(
            "replacement transaction has no recoverable state: {}",
            root.path().display()
        )));
    }
    let meta: ModEntryMeta = serde_json::from_slice(&read_secure_child_bounded(
        &root,
        meta_name,
        "legacy replacement sidecar",
        DEFAULT_IMPORT_LIMITS.max_manifest_bytes,
    )?)?;
    if !crate::is_safe_mod_name(&meta.id) {
        return Err(ModError::Other(format!(
            "legacy replacement contains invalid entry id {:?}",
            meta.id
        )));
    }
    let live = library.secure_directory().open_optional_child_directory(
        std::ffi::OsStr::new(&meta.id),
        "live legacy replacement entry",
    )?;
    drop(root);
    if live.is_some() {
        drop(live);
        library
            .secure_directory()
            .remove_child_tree(&root_name, "legacy replacement backup")
    } else {
        library.secure_directory().rename_child_to(
            &root_name,
            library.secure_directory(),
            std::ffi::OsStr::new(&meta.id),
            "restoring legacy replacement backup",
        )
    }
}

#[cfg(not(unix))]
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

#[cfg(test)]
fn recover_interrupted_replacements_for_test(library_dir: &Path) -> crate::Result<()> {
    let lock = library_mutation_lock(library_dir)?;
    recover_interrupted_replacements_locked(&lock)
}

#[cfg(not(unix))]
fn recover_replacement(library_dir: &Path, root: PathBuf) -> crate::Result<()> {
    let state_path = root.join(REPLACEMENT_STATE_FILE);
    if !path_present(&state_path)? {
        return recover_legacy_replacement(library_dir, &root);
    }
    let state = read_replacement_state(&state_path)?;
    validate_replacement_state(&state, &state_path)?;
    let transaction = ReplacementTransaction::from_state(root, state);
    let phase = transaction.phase()?;
    validate_replacement_phase_consistency(&transaction.state, phase, &transaction.root)?;
    if phase == ReplacementPhase::Quarantined {
        return Err(ModError::Other(format!(
            "replacement transaction is quarantined after a post-rename identity mismatch; recovery evidence was retained at {}",
            transaction.root.display()
        )));
    }
    let entry_dir = library_dir.join(&transaction.state.id);
    let live = metadata_if_present(&entry_dir)?;
    let backup_path = transaction.backup();
    let backup = metadata_if_present(&backup_path)?;
    validate_replacement_entry_metadata(live.as_ref(), &entry_dir, "live")?;
    validate_replacement_entry_metadata(backup.as_ref(), &backup_path, "backup")?;

    if transaction.state.verification_pending {
        return recover_verification_pending_replacement(
            library_dir,
            transaction,
            phase,
            live.is_some(),
            backup.is_some(),
        );
    }

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

#[cfg(not(unix))]
fn recover_verification_pending_replacement(
    library_dir: &Path,
    transaction: ReplacementTransaction,
    phase: ReplacementPhase,
    live_present: bool,
    backup_present: bool,
) -> crate::Result<()> {
    let entry_dir = library_dir.join(&transaction.state.id);
    let expected_staged = transaction
        .state
        .expected_staged
        .as_ref()
        .expect("verification-pending state was validated");
    let live_seal = live_present
        .then(|| {
            run_recovery_seal_failure_hook()?;
            seal_import_path(&entry_dir, DEFAULT_IMPORT_LIMITS)
        })
        .transpose()?;
    let live_matches = live_seal
        .as_ref()
        .is_some_and(|observed| observed == expected_staged);
    let expected_previous = transaction.state.expected_previous.as_ref();
    if live_matches && (expected_previous.is_some() || !backup_present) {
        // A partial previous tree cannot weaken the exact staged-live proof. In particular, its
        // sidecar may be the first cleanup child removed before a crash, so never seal it here.
        transaction.mark(ReplacementPhase::Promoted)?;
        return transaction.cleanup();
    }
    let live_previous_matches = match (live_seal.as_ref(), expected_previous) {
        (Some(observed), Some(expected)) => observed == expected,
        _ => false,
    };
    let backup_seal = backup_present
        .then(|| {
            run_recovery_seal_failure_hook()?;
            seal_import_path(&transaction.backup(), DEFAULT_IMPORT_LIMITS)
        })
        .transpose()?;
    let backup_matches = match (backup_seal.as_ref(), expected_previous) {
        (None, None) => true,
        (Some(observed), Some(expected)) => observed == expected,
        _ => false,
    };

    if phase == ReplacementPhase::Quarantined {
        return Err(ModError::Other(format!(
            "replacement transaction is quarantined after a verification mismatch; recovery evidence was retained at {}",
            transaction.root.display()
        )));
    }

    match (live_present, backup_present, expected_previous) {
        (false, false, None) if phase == ReplacementPhase::Prepared => {
            // The durable create journal preceded its first publication rename. With no live or
            // backup object present, cleanup removes only transaction metadata.
            transaction.cleanup()
        }
        (true, false, Some(_)) if live_previous_matches => {
            // The verified previous entry was restored, but the process stopped before the
            // restored marker and transaction cleanup became durable.
            transaction.mark(ReplacementPhase::Restored)?;
            transaction.cleanup()
        }
        (false, true, Some(_)) if backup_matches => {
            rename_replacement_path(&transaction.backup(), &entry_dir)
                .map_err(crate::io("restoring verified interrupted replacement"))?;
            sync_replacement_directory(library_dir)?;
            transaction.mark(ReplacementPhase::Restored)?;
            transaction.cleanup()
        }
        _ => quarantine_mismatched_recovery(
            library_dir,
            &transaction,
            phase,
            live_present,
            backup_matches,
        ),
    }
}

#[cfg(not(unix))]
fn quarantine_mismatched_recovery(
    library_dir: &Path,
    transaction: &ReplacementTransaction,
    phase: ReplacementPhase,
    live_present: bool,
    backup_matches: bool,
) -> crate::Result<()> {
    let original = ModError::Other(format!(
        "verification-pending replacement objects do not match their durable seals (phase {phase:?})"
    ));
    let entry_dir = library_dir.join(&transaction.state.id);
    let safety = (|| -> crate::Result<()> {
        if live_present {
            rename_replacement_path(&entry_dir, &transaction.quarantine()).map_err(crate::io(
                "quarantining mismatched live manager-library entry during recovery",
            ))?;
            sync_replacement_directory(library_dir)?;
            sync_replacement_directory(&transaction.root)?;
        }
        if transaction.state.expected_previous.is_some() && backup_matches {
            rename_replacement_path(&transaction.backup(), &entry_dir).map_err(crate::io(
                "restoring verified previous manager-library entry during recovery",
            ))?;
            sync_replacement_directory(library_dir)?;
        }
        Ok(())
    })();
    if let Err(safety_error) = safety {
        return Err(ModError::Other(format!(
            "refusing ambiguous replacement recovery: {original}; fail-closed quarantine/restore failed: {safety_error}; recovery evidence retained at {}",
            transaction.root.display()
        )));
    }
    let marker_error = transaction.mark(ReplacementPhase::Quarantined).err();
    Err(quarantined_replacement_error(
        original,
        marker_error,
        &transaction.root,
        "refusing ambiguous replacement recovery",
    ))
}

/// Recover PID-named backups written by the pre-transaction implementation. The entry's own
/// bounded sidecar supplies the id; no path component is inferred from the dot-directory name.
#[cfg(not(unix))]
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

#[cfg(not(unix))]
fn read_replacement_state(path: &Path) -> crate::Result<ReplacementState> {
    let bytes = read_nofollow_bounded(path, "replacement state", REPLACEMENT_STATE_MAX_BYTES)?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(not(unix))]
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

#[cfg(not(unix))]
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

#[cfg(not(unix))]
fn path_present(path: &Path) -> crate::Result<bool> {
    metadata_if_present(path).map(|metadata| metadata.is_some())
}

/// Persist directory-entry changes on platforms that expose a portable directory fsync.
#[cfg(all(unix, test))]
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
    if metadata_if_present(library_dir)?.is_none() {
        return Ok(false);
    }
    let library_lock = library_mutation_lock(library_dir)?;
    #[cfg(not(unix))]
    let canonical_library_dir = library_lock.path().to_path_buf();
    recover_interrupted_replacements_locked(&library_lock)?;

    #[cfg(unix)]
    {
        let library = library_lock.open_library()?;
        let name = std::ffi::OsStr::new(id);
        if library
            .secure_directory()
            .open_optional_child_directory(name, "library entry selected for removal")?
            .is_none()
        {
            return Ok(false);
        }
        library
            .secure_directory()
            .remove_child_tree(name, "manager-library entry")?;
        Ok(true)
    }

    #[cfg(not(unix))]
    {
        let dir = canonical_library_dir.join(id);
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
        let library = library_lock.open_library()?;
        let entry = library.entry(id)?;
        let entry_path = entry.path().to_path_buf();
        // The entry handle intentionally denies FILE_SHARE_DELETE and must be released before the
        // authorized Windows removal. The cooperative library lock remains held, and the root was
        // re-opened/FileId-checked immediately before this path operation. This does not claim a
        // hostile same-user namespace boundary.
        drop(entry);
        drop(library);
        std::fs::remove_dir_all(&entry_path).map_err(crate::io(&format!(
            "removing entry {}",
            entry_path.display()
        )))?;
        let library = library_lock.open_library()?;
        library.sync_after_mutation()?;
        Ok(true)
    }
}

/// Fail-closed recovery gate for consumers that read deployable library payloads directly. The
/// guard is intentionally released after recovery; this slice does not claim a joint library/game
/// transaction, but an interrupted or quarantined import can never be silently consumed by Apply.
pub(crate) fn recover_library_for_read(library_dir: &Path) -> crate::Result<()> {
    if metadata_if_present(library_dir)?.is_none() {
        return Ok(());
    }
    let library_lock = library_mutation_lock(library_dir)?;
    recover_interrupted_replacements_locked(&library_lock)
}

/// All library entries, sorted by name. Entries with an unreadable/corrupt sidecar are skipped
/// (with a note on stderr), a missing library dir is an empty library.
pub fn list(library_dir: &Path) -> crate::Result<Vec<ModEntryMeta>> {
    if metadata_if_present(library_dir)?.is_none() {
        return Ok(Vec::new());
    }
    let library_lock = library_mutation_lock(library_dir)?;
    recover_interrupted_replacements_locked(&library_lock)?;
    let library = library_lock.open_library()?;
    let rd = library.read_dir()?;
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
                sources = direct_container_members(source, limits)?;
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

/// Enumerate the selected container member's directory before staging any bytes. Besides finding
/// same-base siblings through the same portable Windows identity used by folder/ZIP imports, this
/// makes an otherwise-hidden split member (`.ucas.N`, `.utoc.N`, or `.pak.N`) a hard refusal.
fn direct_container_members(source: &Path, limits: ImportLimits) -> crate::Result<Vec<PathBuf>> {
    // Bind relative or aliased input to the opened file's final path. `read_dir("")` is invalid,
    // and comparing an enumerated absolute entry with the caller's relative spelling would add the
    // selected member twice.
    let selected_file = open_file_nofollow(source, "selected container import member")?;
    let selected = selected_file.path().to_path_buf();
    drop(selected_file);
    let selected_name = selected
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ModError::Other(format!(
                "single-file import name is not valid Unicode: {}",
                selected.display()
            ))
        })?;
    let (selected_base, _) = primary_iostore_member(selected_name).ok_or_else(|| {
        ModError::Other(format!(
            "selected container member has an unsupported name: {selected_name:?}"
        ))
    })?;
    let selected_base_key = portable_windows_key(selected_base);
    let parent = selected
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(parent).map_err(crate::io(&format!(
        "reading import siblings in {}",
        parent.display()
    )))? {
        let entry = entry.map_err(crate::io("reading import sibling entry"))?;
        check_import_limit(
            "single-file import entry count",
            entries.len() as u64 + 1,
            limits.max_zip_entries as u64,
        )?;
        entries.push(entry);
    }
    entries.sort_by_key(|entry| entry.file_name());

    let mut sources = vec![selected.clone()];
    for entry in entries {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let semantic_lower_name = name.to_ascii_lowercase();
        if partitioned_iostore_member_base(&semantic_lower_name)
            .is_some_and(|base| portable_windows_key(base) == selected_base_key)
        {
            return Err(ModError::Other(format!(
                "unsupported multipart IoStore member {:?}; the manager cannot deploy partitioned container payloads",
                entry.path().display().to_string()
            )));
        }
        if primary_iostore_member(&name)
            .is_some_and(|(base, _)| portable_windows_key(base) == selected_base_key)
        {
            let sibling = open_file_nofollow(&entry.path(), "container import sibling")?;
            if !same_opened_import_path(sibling.path(), &selected) {
                sources.push(sibling.path().to_path_buf());
            }
        }
    }
    Ok(sources)
}

fn same_opened_import_path(left: &Path, right: &Path) -> bool {
    // Both paths come from an opened handle. Keep this case-sensitive so distinct entries in a
    // case-sensitive directory are retained and then rejected as competing portable destinations.
    left == right
}

fn primary_iostore_member(name: &str) -> Option<(&str, &'static str)> {
    let (base, extension) = name.rsplit_once('.')?;
    if extension.eq_ignore_ascii_case("utoc") {
        Some((base, "utoc"))
    } else if extension.eq_ignore_ascii_case("ucas") {
        Some((base, "ucas"))
    } else if extension.eq_ignore_ascii_case("pak") {
        Some((base, "pak"))
    } else {
        None
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
        let folded = portable_windows_key(&file_name);
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
        let rel_path = Path::new(&rel);
        let directory_depth = if entry.is_dir() {
            rel_path.components().count()
        } else {
            rel_path
                .parent()
                .map(|parent| parent.components().count())
                .unwrap_or(0)
        };
        if directory_depth > limits.max_directory_depth {
            return Err(ModError::Other(format!(
                "ZIP entry nesting depth limit exceeded for {raw_name:?}: {directory_depth} > {}",
                limits.max_directory_depth
            )));
        }
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

        let key = portable_windows_key(&rel);
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
    if !staged_regular_file_exists(&staging.join("Scripts").join("main.lua"))? {
        return Ok(());
    }
    let tmp = staging.join(".gore-wrap");
    std::fs::create_dir(&tmp).map_err(crate::io("creating wrap dir"))?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(staging).map_err(crate::io("reading staging"))? {
        entries.push(entry.map_err(crate::io("reading staging entry for UE4SS wrap"))?);
    }
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
    let Some(bundle_dir) = find_manifest_dir(staging)? else {
        return Ok(());
    };
    if bundle_dir == staging {
        return Ok(()); // already rooted at the bundle
    }
    validate_reroot_chain(staging, &bundle_dir)?;
    // Stash the nested bundle subtree at a fresh sibling under `staging` first (a valid rename:
    // `.gore-reroot` is NOT inside `bundle_dir`, so this doesn't move a dir into itself). Then clear
    // the old wrapper dirs and hoist the stashed bundle's children up to the root.
    let stash = staging.join(".gore-reroot");
    if metadata_if_present(&stash)?.is_some() {
        return Err(ModError::Other(format!(
            "nested gore-mod import uses reserved reroot path: {}",
            stash.display()
        )));
    }
    std::fs::rename(&bundle_dir, &stash).map_err(crate::io("stashing nested bundle"))?;

    // Remove only wrapper directories that became empty. Benign README/license/thumbnail siblings
    // remain under their original wrapper path; deployable or reserved siblings were rejected by
    // the preflight above, so nothing actionable is silently dropped from the imported entry.
    let mut wrapper = bundle_dir.parent();
    while let Some(path) = wrapper {
        if path == staging {
            break;
        }
        let parent = path.parent();
        match std::fs::remove_dir(path) {
            Ok(()) => wrapper = parent,
            Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
            Err(error) => return Err(crate::io("removing empty gore-mod wrapper")(error)),
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

/// Check that re-rooting cannot hide or overwrite content. Benign wrapper extras are retained, but
/// anything the foreign scanner could deploy is refused rather than being left outside the
/// manifest contract. Root-name collisions are also refused before the first rename.
fn validate_reroot_chain(staging: &Path, bundle_dir: &Path) -> crate::Result<()> {
    let relative = bundle_dir.strip_prefix(staging).map_err(|_| {
        ModError::Other(format!(
            "nested gore-mod manifest root escapes staging: {}",
            bundle_dir.display()
        ))
    })?;
    let top_wrapper = relative
        .components()
        .next()
        .and_then(|component| match component {
            std::path::Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .ok_or_else(|| {
            ModError::Other(format!(
                "nested gore-mod manifest has an unsafe wrapper root: {}",
                bundle_dir.display()
            ))
        })?;
    let top_wrapper_key = portable_windows_key(top_wrapper);
    let mut top_wrapper_will_be_removed = true;
    let mut current = staging.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(expected) = component else {
            return Err(ModError::Other(format!(
                "nested gore-mod manifest has an unsafe wrapper path: {}",
                bundle_dir.display()
            )));
        };
        if expected == std::ffi::OsStr::new(".gore-reroot") {
            return Err(ModError::Other(format!(
                "nested gore-mod wrapper uses reserved reroot path: {}",
                bundle_dir.display()
            )));
        }
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&current).map_err(crate::io(&format!(
            "reading gore-mod wrapper {}",
            current.display()
        )))? {
            entries.push(entry.map_err(crate::io(&format!(
                "reading gore-mod wrapper entry in {}",
                current.display()
            )))?);
        }
        let expected_entry = entries
            .iter()
            .find(|entry| entry.file_name() == expected)
            .ok_or_else(|| {
                ModError::Other(format!(
                    "nested gore-mod wrapper chain changed while being inspected: {}",
                    current.display()
                ))
            })?;
        let entry_type = expected_entry.file_type().map_err(crate::io(&format!(
            "reading gore-mod wrapper entry type {}",
            expected_entry.path().display()
        )))?;
        let entry_metadata = std::fs::symlink_metadata(expected_entry.path())
            .map_err(crate::io("reading gore-mod wrapper entry metadata"))?;
        if import_metadata_is_link(&entry_metadata) {
            return Err(ModError::Other(format!(
                "nested gore-mod wrapper is a symbolic link or reparse point: {}",
                expected_entry.path().display()
            )));
        }
        if !entry_type.is_dir() || !entry_metadata.is_dir() {
            return Err(ModError::Other(format!(
                "nested gore-mod wrapper is not a directory: {}",
                expected_entry.path().display()
            )));
        }
        let below_staging_root = current != staging;
        for sibling in entries.iter().filter(|entry| entry.file_name() != expected) {
            if let Some(deployable) = find_deployable_reroot_sibling(&sibling.path(), 0)? {
                return Err(ModError::Other(format!(
                    "nested gore-mod manifest has deployable or reserved sibling content outside its contract: {}",
                    deployable.display()
                )));
            }
            // A benign sibling below the staging root keeps the top wrapper non-empty after the
            // bundle subtree moves to the stash. The root entry therefore remains collision-relevant.
            if below_staging_root {
                top_wrapper_will_be_removed = false;
            }
        }
        current = expected_entry.path();
    }

    // Hoisting the bundle's direct children must be create-new with respect to every retained root
    // sibling. `.gore-reroot` is reserved for the temporary stash itself.
    for entry in std::fs::read_dir(bundle_dir).map_err(crate::io(&format!(
        "reading nested gore-mod root {}",
        bundle_dir.display()
    )))? {
        let entry = entry.map_err(crate::io("reading nested gore-mod root entry"))?;
        let name = entry.file_name();
        let name_key = name.to_str().map(portable_windows_key).ok_or_else(|| {
            ModError::Other(format!(
                "nested gore-mod content name is not valid Unicode: {}",
                entry.path().display()
            ))
        })?;
        let occupied = metadata_if_present(&staging.join(&name))?.is_some();
        let occupied_only_by_removable_wrapper =
            occupied && top_wrapper_will_be_removed && name_key == top_wrapper_key;
        if name == std::ffi::OsStr::new(".gore-reroot")
            || (occupied && !occupied_only_by_removable_wrapper)
        {
            return Err(ModError::Other(format!(
                "nested gore-mod content would collide while re-rooting: {}",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

fn find_deployable_reroot_sibling(path: &Path, depth: usize) -> crate::Result<Option<PathBuf>> {
    let sibling_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ModError::Other(format!(
                "nested gore-mod sibling name is not valid Unicode: {}",
                path.display()
            ))
        })?;
    if portable_windows_names_equal(sibling_name, META_FILE)
        || portable_windows_names_equal(sibling_name, ".gore-reroot")
    {
        return Ok(Some(path.to_path_buf()));
    }
    let metadata = std::fs::symlink_metadata(path).map_err(crate::io(&format!(
        "reading nested gore-mod sibling metadata {}",
        path.display()
    )))?;
    if import_metadata_is_link(&metadata) {
        return Err(ModError::Other(format!(
            "nested gore-mod sibling is a symbolic link or reparse point: {}",
            path.display()
        )));
    }
    if metadata.is_file() {
        let lower = sibling_name.to_ascii_lowercase();
        let deployable = lower.ends_with(".utoc")
            || lower.ends_with(".ucas")
            || is_partitioned_iostore_member(&lower)
            || lower.ends_with(".pak")
            || lower.ends_with(".lcache")
            || lower.ends_with(".bank")
            || (lower.starts_with("precompiledscript") && lower.ends_with(".cache"));
        return Ok(deployable.then(|| path.to_path_buf()));
    }
    if !metadata.is_dir() {
        return Err(ModError::Other(format!(
            "nested gore-mod sibling is neither a regular file nor a directory: {}",
            path.display()
        )));
    }
    if staged_regular_file_exists(&path.join("Scripts").join("main.lua"))? {
        return Ok(Some(path.to_path_buf()));
    }
    if depth >= MAX_SCAN_DEPTH {
        return Err(ModError::Other(format!(
            "nested gore-mod sibling nesting depth limit exceeded at {}",
            path.display()
        )));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path).map_err(crate::io(&format!(
        "scanning nested gore-mod sibling {}",
        path.display()
    )))? {
        entries.push(entry.map_err(crate::io("reading nested gore-mod sibling entry"))?);
    }
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if let Some(deployable) = find_deployable_reroot_sibling(&entry.path(), depth + 1)? {
            return Ok(Some(deployable));
        }
    }
    Ok(None)
}

// ── Detection ───────────────────────────────────────────────────────────────

/// Detect what the staged tree is: a goremod bundle (`gore-mod.json` at the root or nested at
/// most two folders deep — the usual "zip contains a folder" shipping shapes) or foreign files.
fn detect(
    staging: &Path,
    limits: ImportLimits,
) -> crate::Result<(Option<ModManifest>, Vec<ComponentInfo>)> {
    if let Some(bundle_dir) = find_manifest_dir(staging)? {
        let bytes = read_bounded_bundle_file(
            &bundle_dir,
            Path::new("gore-mod.json"),
            "gore-mod.json",
            limits.max_manifest_bytes,
        )?;
        let manifest: ModManifest = serde_json::from_slice(&bytes)?;
        // The bundle format is a capability contract, not merely a serde shape. Validate it
        // before interpreting any component path or payload contract so a format/component
        // mismatch never reaches manager metadata or activation.
        crate::validate_mod_manifest_format(&manifest)?;
        let raw: serde_json::Value = serde_json::from_slice(&bytes)?;
        let prefix = rel_str(staging, &bundle_dir); // "" when the bundle is the staging root
        let comps = goremod_components(&bundle_dir, &prefix, &manifest, &raw, limits)?;
        Ok((Some(manifest), comps))
    } else {
        Ok((None, scan_foreign(staging)?))
    }
}

/// Find the sole `gore-mod.json` in the bounded staged tree. Only a manifest at depth ≤2 is a
/// supported bundle root, but deeper manifests are still detected and refused rather than letting
/// their payload fall through to the foreign scanner. Until two manifests have already proved
/// ambiguity, every directory-entry and file-type failure is authoritative: choosing a different
/// visible manifest after an incomplete scan could discard content during re-rooting.
const MANIFEST_AMBIGUITY_EVIDENCE_LIMIT: usize = 2;

fn find_manifest_dir(root: &Path) -> crate::Result<Option<PathBuf>> {
    let mut manifests = Vec::<(PathBuf, usize)>::with_capacity(MANIFEST_AMBIGUITY_EVIDENCE_LIMIT);
    find_manifest_dirs(root, 0, &mut manifests)?;
    manifests.sort_by(|(left, _), (right, _)| left.cmp(right));
    if manifests.len() > 1 {
        let paths = manifests
            .iter()
            .map(|(path, _)| path.join("gore-mod.json").display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ModError::Other(format!(
            "ambiguous gore-mod import: multiple gore-mod.json manifests found (first two; at least two exist): {paths}"
        )));
    }
    let Some((manifest_dir, depth)) = manifests.pop() else {
        return Ok(None);
    };
    if depth > 2 {
        return Err(ModError::Other(format!(
            "gore-mod.json is nested too deeply for a supported bundle layout (depth {depth} > 2): {}",
            manifest_dir.join("gore-mod.json").display()
        )));
    }
    Ok(Some(manifest_dir))
}

fn find_manifest_dirs(
    dir: &Path,
    depth: usize,
    manifests: &mut Vec<(PathBuf, usize)>,
) -> crate::Result<()> {
    if manifests.len() >= MANIFEST_AMBIGUITY_EVIDENCE_LIMIT {
        return Ok(());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(crate::io(&format!(
        "scanning for gore-mod.json in {}",
        dir.display()
    )))? {
        entries.push(entry.map_err(crate::io(&format!(
            "reading manifest-scan entry in {}",
            dir.display()
        )))?);
    }
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if manifests.len() >= MANIFEST_AMBIGUITY_EVIDENCE_LIMIT {
            break;
        }
        let path = entry.path();
        let file_type = entry.file_type().map_err(crate::io(&format!(
            "reading manifest-scan entry type {}",
            path.display()
        )))?;
        let metadata = std::fs::symlink_metadata(&path).map_err(crate::io(&format!(
            "reading manifest-scan metadata {}",
            path.display()
        )))?;
        if import_metadata_is_link(&metadata) {
            return Err(ModError::Other(format!(
                "manifest scan encountered a symbolic link or reparse point: {}",
                path.display()
            )));
        }
        if file_type.is_file() != metadata.is_file() || file_type.is_dir() != metadata.is_dir() {
            return Err(ModError::Other(format!(
                "manifest-scan entry changed type while being inspected: {}",
                path.display()
            )));
        }
        let is_manifest = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.eq_ignore_ascii_case("gore-mod.json"));
        if file_type.is_file() && is_manifest {
            manifests.push((dir.to_path_buf(), depth));
        } else if file_type.is_dir() {
            if depth >= MAX_SCAN_DEPTH {
                return Err(ModError::Other(format!(
                    "manifest scan nesting depth limit exceeded at {}: {} > {}",
                    path.display(),
                    depth + 1,
                    MAX_SCAN_DEPTH
                )));
            }
            find_manifest_dirs(&path, depth + 1, manifests)?;
        } else if !file_type.is_file() {
            return Err(ModError::Other(format!(
                "manifest scan encountered a non-file, non-directory entry: {}",
                path.display()
            )));
        }
    }
    Ok(())
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
                    targets.insert(portable_windows_key(&target), target);
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
    let mut scan = ForeignScan::default();
    scan_dir(root, root, 0, &mut scan)?;
    scan.finish(root)
}

#[derive(Debug, Default)]
struct ForeignScan {
    components: Vec<(String, String, ComponentInfo)>,
    iostore: BTreeMap<String, IoStoreMembers>,
    raw_targets: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
struct IoStoreMembers {
    rel_base: Option<String>,
    utoc: Option<PathBuf>,
    ucas: Option<PathBuf>,
    pak: Option<PathBuf>,
}

impl ForeignScan {
    fn push_component(&mut self, rel: String, component: ComponentInfo) {
        self.components
            .push((rel.to_ascii_lowercase(), rel, component));
    }

    fn push_raw(&mut self, rel: String, target_file: RawTarget) -> crate::Result<()> {
        let target_key = match &target_file {
            RawTarget::Lcache => "lcache".to_owned(),
            RawTarget::Bank { name } => {
                format!("bank:{}", portable_windows_key(name).to_lowercase())
            }
            RawTarget::ScriptCache => "script_cache".to_owned(),
        };
        if let Some(first) = self.raw_targets.insert(target_key.clone(), rel.clone()) {
            return Err(ModError::Other(format!(
                "foreign import contains duplicate raw deployment target {target_key:?}: {first:?} and {rel:?}"
            )));
        }
        self.push_component(rel.clone(), ComponentInfo::RawFile { rel, target_file });
        Ok(())
    }

    fn record_iostore_member(
        &mut self,
        root: &Path,
        path: &Path,
        extension: &str,
    ) -> crate::Result<()> {
        let actual_extension = path
            .extension()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                ModError::Other(format!(
                    "IoStore member extension is not valid Unicode: {}",
                    path.display()
                ))
            })?;
        if actual_extension != extension {
            return Err(ModError::Other(format!(
                "IoStore member name is not exactly reconstructable during Apply: expected lowercase .{extension}, found {actual_extension:?} in {}",
                path.display()
            )));
        }
        let rel_base = rel_str(root, &path.with_extension(""));
        let portable_base = portable_windows_key(&rel_base);
        let members = self.iostore.entry(portable_base.clone()).or_default();
        if let Some(first_base) = &members.rel_base {
            if first_base != &rel_base {
                return Err(ModError::Other(format!(
                    "IoStore member names are not exactly reconstructable during Apply: {first_base:?} and {rel_base:?} differ in spelling"
                )));
            }
        } else {
            members.rel_base = Some(rel_base);
        }
        let slot = match extension {
            "utoc" => &mut members.utoc,
            "ucas" => &mut members.ucas,
            "pak" => &mut members.pak,
            _ => unreachable!("record_iostore_member called for {extension}"),
        };
        if let Some(first) = slot.replace(path.to_path_buf()) {
            return Err(ModError::Other(format!(
                "foreign import contains duplicate .{extension} members for IoStore base {portable_base:?}: {} and {}",
                first.display(),
                path.display()
            )));
        }
        Ok(())
    }

    fn finish(mut self, root: &Path) -> crate::Result<Vec<ComponentInfo>> {
        for (_, members) in std::mem::take(&mut self.iostore) {
            let observed_base = members
                .rel_base
                .expect("IoStore member groups always record a relative base");
            if members.utoc.is_some() || members.ucas.is_some() {
                let mut missing = Vec::new();
                if members.utoc.is_none() {
                    missing.push(".utoc");
                }
                if members.ucas.is_none() {
                    missing.push(".ucas");
                }
                if !missing.is_empty() {
                    return Err(ModError::Other(format!(
                        "incomplete IoStore set {observed_base:?}: missing {}",
                        missing.join(" and ")
                    )));
                }
                // The manager's established deploy contract requires the mountable pair. A same-
                // stem `.pak` stub is supported and copied when present, but is not required.
                let utoc = members.utoc.expect("checked above");
                let rel_base = rel_str(root, &utoc.with_extension(""));
                let targets = gore_tex::container::list_packages(&utoc).unwrap_or_default();
                self.push_component(
                    rel_str(root, &utoc),
                    ComponentInfo::Triplet { rel_base, targets },
                );
            } else if let Some(pak) = members.pak {
                let is_loose_pak = pak
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.to_ascii_lowercase().ends_with("_p.pak"));
                if is_loose_pak {
                    let rel = rel_str(root, &pak);
                    // Read through the mount point, because this is the game-root-relative
                    // conflict namespace. Container parsing intentionally remains best-effort.
                    let targets = gore_tex::container::list_pak_files_from_game_root(&pak)
                        .unwrap_or_default();
                    self.push_component(rel.clone(), ComponentInfo::LoosePak { rel, targets });
                } else {
                    return Err(ModError::Other(format!(
                        "unsupported standalone .pak member {observed_base:?}: only a loose *_P.pak or the optional same-stem member of an IoStore pair can be deployed"
                    )));
                }
            }
        }
        self.components
            .sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        Ok(self
            .components
            .into_iter()
            .map(|(_, _, component)| component)
            .collect())
    }
}

/// Deepest directory nesting `scan_dir` will descend into. Real mods are shallow; a cap here
/// bounds the recursion so a maliciously deep tree cannot overflow the stack. Required descent
/// past the cap is rejected; content is never silently omitted.
const MAX_SCAN_DEPTH: usize = 16;

fn scan_dir(root: &Path, dir: &Path, depth: usize, scan: &mut ForeignScan) -> crate::Result<()> {
    let mut entries = Vec::new();
    for entry in
        std::fs::read_dir(dir).map_err(crate::io(&format!("scanning {}", dir.display())))?
    {
        entries.push(entry.map_err(crate::io(&format!(
            "reading foreign-scan entry in {}",
            dir.display()
        )))?);
    }
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        let ft = e.file_type().map_err(crate::io(&format!(
            "reading foreign-scan entry type {}",
            path.display()
        )))?;
        let metadata = std::fs::symlink_metadata(&path).map_err(crate::io(&format!(
            "reading foreign-scan metadata {}",
            path.display()
        )))?;
        if import_metadata_is_link(&metadata) {
            return Err(ModError::Other(format!(
                "foreign scan encountered a symbolic link or reparse point: {}",
                path.display()
            )));
        }
        if ft.is_file() != metadata.is_file() || ft.is_dir() != metadata.is_dir() {
            return Err(ModError::Other(format!(
                "foreign-scan entry changed type while being inspected: {}",
                path.display()
            )));
        }
        if ft.is_dir() {
            if staged_regular_file_exists(&path.join("Scripts").join("main.lua"))? {
                // A UE4SS Lua mod dir is one opaque component; don't scan inside it.
                let name = e.file_name().to_string_lossy().into_owned();
                let rel = rel_str(root, &path);
                scan.push_component(
                    rel.clone(),
                    ComponentInfo::Ue4ssLua {
                        name,
                        rel,
                        targets: Vec::new(),
                        opaque: true,
                    },
                );
            } else if depth < MAX_SCAN_DEPTH {
                scan_dir(root, &path, depth + 1, scan)?;
            } else {
                return Err(ModError::Other(format!(
                    "foreign scan nesting depth limit exceeded at {}: {} > {}",
                    path.display(),
                    depth + 1,
                    MAX_SCAN_DEPTH
                )));
            }
        } else if ft.is_file() {
            classify_file(root, &path, scan)?;
        } else {
            return Err(ModError::Other(format!(
                "foreign scan encountered a non-file, non-directory entry: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn staged_regular_file_exists(path: &Path) -> crate::Result<bool> {
    let Some(metadata) = metadata_if_present(path)? else {
        return Ok(false);
    };
    if import_metadata_is_link(&metadata) {
        return Err(ModError::Other(format!(
            "foreign scan encountered a symbolic link or reparse point: {}",
            path.display()
        )));
    }
    if metadata.is_file() {
        Ok(true)
    } else if metadata.is_dir() {
        Ok(false)
    } else {
        Err(ModError::Other(format!(
            "foreign scan encountered a non-file, non-directory entry: {}",
            path.display()
        )))
    }
}

/// Classify one foreign file into a component (or nothing). Target extraction is best-effort:
/// an unparsable container still imports, just with an empty (unknown) footprint.
fn classify_file(root: &Path, path: &Path, scan: &mut ForeignScan) -> crate::Result<()> {
    let name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
        ModError::Other(format!(
            "foreign scan file name is not valid Unicode: {}",
            path.display()
        ))
    })?;
    let lower = name.to_ascii_lowercase();
    let rel = rel_str(root, path);
    if is_partitioned_iostore_member(&lower) {
        return Err(ModError::Other(format!(
            "unsupported multipart IoStore member {rel:?}; the manager cannot deploy partitioned container payloads"
        )));
    } else if lower.starts_with("precompiledscript") && lower.ends_with(".cache") {
        scan.push_raw(rel, RawTarget::ScriptCache)?;
    } else if lower.ends_with(".lcache") {
        scan.push_raw(rel, RawTarget::Lcache)?;
    } else if lower.ends_with(".bank") {
        scan.push_raw(
            rel,
            RawTarget::Bank {
                name: name.to_string(),
            },
        )?;
    } else if let Some(extension) = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|extension| matches!(extension.as_str(), "utoc" | "ucas" | "pak"))
    {
        // Record all three possible members first. Finalization sees the complete staged tree and
        // can reject every orphan pair member even when another valid mixed component was found.
        scan.record_iostore_member(root, path, &extension)?;
    }
    Ok(())
}

fn partitioned_iostore_member_base(lower_name: &str) -> Option<&str> {
    for marker in [".ucas.", ".utoc."] {
        if let Some((base, suffix)) = lower_name.rsplit_once(marker) {
            if !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()) {
                return Some(base);
            }
        }
    }
    lower_name.rsplit_once(".pak.").and_then(|(base, suffix)| {
        (!suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())).then_some(base)
    })
}

fn is_partitioned_iostore_member(lower_name: &str) -> bool {
    partitioned_iostore_member_base(lower_name).is_some()
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

/// Conservative portable Windows identity for untrusted relative names. Windows compares
/// case-insensitive filesystem names through an uppercase table (not Unicode lowercase/casefold),
/// so uppercase also catches final-sigma/long-s aliases that lowercase misses. Full Unicode
/// expansions may reject additional pairs on non-Windows hosts; that fail-closed over-approximation
/// is preferable to publishing an archive/tree that aliases after extraction on Windows.
fn portable_windows_key(value: &str) -> String {
    value.replace('\\', "/").to_uppercase()
}

fn portable_windows_names_equal(left: &str, right: &str) -> bool {
    portable_windows_key(left) == portable_windows_key(right)
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

/// Inverse of [`civil_from_days`]: a validated civil date to days since 1970-01-01.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
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

/// Once the library lock is held, Unix cleanup is anchored to that retained inode instead of the
/// configured pathname. Declaring this guard after `LibraryMutationGuard` makes Rust drop it first
/// on every `?`/return path, so the inode lock remains held throughout identity-bound cleanup.
#[cfg(unix)]
struct LockedStagingGuard {
    root: SecureDirectory,
    name: std::ffi::OsString,
    identity: FileIdentity,
    armed: bool,
}

#[cfg(unix)]
impl LockedStagingGuard {
    fn bind(
        library_lock: &LibraryMutationGuard,
        staging: &Path,
        expected_identity: FileIdentity,
    ) -> crate::Result<Self> {
        let name = staging
            .file_name()
            .ok_or_else(|| {
                ModError::Other(format!(
                    "staged import has no direct-child name: {}",
                    staging.display()
                ))
            })?
            .to_os_string();
        let library = library_lock.open_library()?;
        let root = library.secure_directory().clone();
        let child = root
            .open_optional_child_directory(&name, "staged import cleanup binding")?
            .ok_or_else(|| {
                ModError::Other(format!(
                    "staged import disappeared from the locked manager-library inode: {}",
                    staging.display()
                ))
            })?;
        if child.identity() != expected_identity {
            return Err(ModError::Other(format!(
                "staged import changed filesystem identity while binding cleanup: {}",
                staging.display()
            )));
        }
        drop(child);
        Ok(Self {
            root,
            name,
            identity: expected_identity,
            armed: true,
        })
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

#[cfg(unix)]
impl Drop for LockedStagingGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.root.remove_optional_child_tree_if_identity(
                &self.name,
                self.identity,
                "failed staged import",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mgr::FootprintCoverage;
    use crate::{
        build_bundle, write_bundle, BuildSpec, LooseFileReplacement, ModMeta, ScriptModule,
        VoiceArchiveEdit, VoicePatchOp,
    };
    use gore_modgen::gen::{OverrideValue, SingleOverride};
    use std::fs;

    fn read_library_sidecar(library: &Path, id: &str) -> LibrarySidecar {
        serde_json::from_slice(&fs::read(library.join(id).join(META_FILE)).unwrap()).unwrap()
    }

    fn write_library_sidecar(library: &Path, id: &str, sidecar: &LibrarySidecar) {
        fs::write(
            library.join(id).join(META_FILE),
            serde_json::to_vec_pretty(sidecar).unwrap(),
        )
        .unwrap();
    }

    fn copy_test_tree(source: &Path, destination: &Path) {
        fs::create_dir(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let target = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_test_tree(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    fn prepare_format_2_promoted_update(
        temp_root: &Path,
        library: &Path,
        label: &str,
    ) -> (ModEntryMeta, ModEntryMeta, PathBuf, ReplacementTransaction) {
        let payload_name = format!("{label}_P.pak");
        let source = temp_root.join(&payload_name);
        fs::write(&source, b"verified previous payload").unwrap();
        let previous_meta = import(library, &source).unwrap();
        let entry = library.join(&previous_meta.id);
        let previous_seal = seal_import_path(&entry, DEFAULT_IMPORT_LIMITS).unwrap();

        let staging = library.join(format!(".staging-{label}-recovery"));
        copy_test_tree(&entry, &staging);
        fs::write(staging.join(&payload_name), b"verified promoted payload").unwrap();
        let mut staged_sidecar = read_library_sidecar(library, &previous_meta.id);
        staged_sidecar.entry.version = "promoted".into();
        fs::write(
            staging.join(META_FILE),
            serde_json::to_vec_pretty(&staged_sidecar).unwrap(),
        )
        .unwrap();
        let promoted_meta = staged_sidecar.entry;
        let staged_seal = seal_import_path(&staging, DEFAULT_IMPORT_LIMITS).unwrap();
        let expectation = PublishExpectation {
            staged: staged_seal,
            current: Some(previous_seal),
            limits: DEFAULT_IMPORT_LIMITS,
        };
        let transaction =
            ReplacementTransaction::begin(library, &previous_meta.id, Some(&expectation)).unwrap();
        fs::rename(&entry, transaction.backup()).unwrap();
        transaction.mark(ReplacementPhase::PreviousMoved).unwrap();
        fs::rename(&staging, &entry).unwrap();
        (previous_meta, promoted_meta, entry, transaction)
    }

    fn visible_library_snapshot(library: &Path) -> Vec<(String, Option<Vec<u8>>)> {
        fn walk(root: &Path, path: &Path, out: &mut Vec<(String, Option<Vec<u8>>)>) {
            let mut entries: Vec<_> = fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap())
                .collect();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                if path == root && entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                let relative = entry.path().strip_prefix(root).unwrap().to_path_buf();
                let relative = rel_str(root, &root.join(&relative));
                if entry.file_type().unwrap().is_dir() {
                    out.push((relative, None));
                    walk(root, &entry.path(), out);
                } else {
                    out.push((relative, Some(fs::read(entry.path()).unwrap())));
                }
            }
        }

        if !library.exists() {
            return Vec::new();
        }
        let mut snapshot = Vec::new();
        walk(library, library, &mut snapshot);
        snapshot
    }

    fn assert_no_import_residue(library: &Path) {
        let residue: Vec<_> = fs::read_dir(library)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                name.starts_with(".staging-") || name.starts_with(REPLACEMENT_PREFIX)
            })
            .map(|entry| entry.file_name())
            .collect();
        assert!(residue.is_empty(), "import residue: {residue:?}");
    }

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

    /// A real format-2 bundle carrying both loose replacement mechanisms. Keeping the two
    /// destinations distinct makes it possible to prove that import preserves each route's exact
    /// footprint instead of merging or inferring them.
    fn mk_mixed_file_bundle(root: &Path, name: &str) -> PathBuf {
        fs::create_dir_all(root).unwrap();
        let loose = root.join("intro.bk2");
        let packed = root.join("Normal.PNG");
        fs::write(&loose, b"LOOSE-INTRO").unwrap();
        fs::write(&packed, b"PACKED-CURSOR").unwrap();
        let spec = BuildSpec {
            meta: ModMeta {
                name: name.into(),
                version: "2.0".into(),
                author: "tester".into(),
            },
            delay_ms: 0,
            overrides: vec![],
            loc_edits: BTreeMap::new(),
            audio: vec![],
            texture: vec![],
            files: vec![LooseFileReplacement {
                game_path: "G1R/Content/Movies/Intro.bk2".into(),
                source_path: loose.display().to_string(),
            }],
            pak_files: vec![LooseFileReplacement {
                game_path: "G1R/Content/Slate/Cursors/Normal/Normal.PNG".into(),
                source_path: packed.display().to_string(),
            }],
            scripts: vec![],
            dialog_topics: vec![],
            voice: vec![],
        };
        let bdir = root.join(name);
        write_bundle(&bdir, &build_bundle(&spec).unwrap()).unwrap();
        bdir
    }

    #[test]
    fn import_format_2_mixed_file_routes_preserves_targets_and_refuses_tampering() {
        let temp = tempfile::tempdir().unwrap();
        let bundle = mk_mixed_file_bundle(&temp.path().join("valid-source"), "MixedRoutes");
        let meta = import(&temp.path().join("valid-library"), &bundle).unwrap();
        assert!(
            matches!(
                meta.components.as_slice(),
                [
                    ComponentInfo::FilePatch { rel: loose_rel, targets: loose_targets },
                    ComponentInfo::PakFilePatch { rel: pak_rel, targets: pak_targets },
                ] if loose_rel == "files"
                    && loose_targets == &["G1R/Content/Movies/Intro.bk2".to_string()]
                    && pak_rel == "pak_files"
                    && pak_targets
                        == &["G1R/Content/Slate/Cursors/Normal/Normal.PNG".to_string()]
            ),
            "components: {:?}",
            meta.components
        );

        // The sidecar footprint and the payload manifest are one contract for BOTH mechanisms.
        // Mutating either payload manifest while leaving gore-mod.json unchanged must fail before
        // the staged entry can be activated.
        for (route, original, replacement) in [
            (
                "files",
                "G1R/Content/Movies/Intro.bk2",
                "G1R/Content/Movies/Outro.bk2",
            ),
            (
                "pak_files",
                "G1R/Content/Slate/Cursors/Normal/Normal.PNG",
                "G1R/Content/Movies/Outro.bk2",
            ),
        ] {
            let source_root = temp.path().join(format!("tampered-{route}-source"));
            let tampered = mk_mixed_file_bundle(&source_root, &format!("Tampered-{route}"));
            let payload_manifest = tampered.join(route).join("manifest.json");
            let mut map: BTreeMap<String, String> =
                serde_json::from_slice(&fs::read(&payload_manifest).unwrap()).unwrap();
            let payload = map.remove(original).unwrap();
            map.insert(replacement.into(), payload);
            fs::write(&payload_manifest, serde_json::to_vec_pretty(&map).unwrap()).unwrap();

            let library = temp.path().join(format!("tampered-{route}-library"));
            let error = import(&library, &tampered).unwrap_err().to_string();
            assert!(
                error.contains("declared targets disagree"),
                "unexpected {route} error: {error}"
            );
            assert_failed_import_left_nothing(&library);
        }
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
            .filter(|name| name != ".gore-manager-library.lock")
            .collect::<Vec<_>>();
        assert!(
            leftovers.is_empty(),
            "failed import left staging/library artifacts: {leftovers:?}"
        );
    }

    struct RemoveFilesOnDrop(Vec<PathBuf>);

    impl Drop for RemoveFilesOnDrop {
        fn drop(&mut self) {
            for path in &self.0 {
                let _ = fs::remove_file(path);
            }
        }
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

        let depth_zip = temp.path().join("too-deep.zip");
        zip_entries(&depth_zip, &[("d0/d1/d2/file.lcache", b"x", stored)]);
        let depth_lib = temp.path().join("depth-lib");
        let error = import_with_limits(
            &depth_lib,
            &depth_zip,
            ImportLimits {
                max_directory_depth: 2,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("ZIP entry nesting depth limit exceeded"),
            "unexpected error: {error}"
        );
        assert_failed_import_left_nothing(&depth_lib);
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
                || error.contains("is a symbolic link")
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
    fn rejects_format_component_mismatches_and_unknown_formats_before_activation() {
        let temp = tempfile::tempdir().unwrap();
        let cases = [
            (
                "format-2-without-pak",
                2,
                false,
                "requires at least one pak_file_patch",
            ),
            (
                "format-1-with-pak",
                1,
                true,
                "format 1 does not support pak_file_patch",
            ),
            (
                "unknown-format",
                99,
                false,
                "unsupported gore-mod manifest format 99",
            ),
        ];
        for (case, format, with_pak, expected) in cases {
            let source_root = temp.path().join(format!("{case}-source"));
            fs::create_dir_all(&source_root).unwrap();
            let bundle = if with_pak {
                mk_mixed_file_bundle(&source_root, case)
            } else {
                mk_goremod_bundle(&source_root)
            };
            let manifest_path = bundle.join("gore-mod.json");
            let mut manifest: serde_json::Value =
                serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
            manifest["format"] = serde_json::json!(format);
            fs::write(
                &manifest_path,
                serde_json::to_vec_pretty(&manifest).unwrap(),
            )
            .unwrap();

            let library = temp.path().join(format!("{case}-library"));
            let error = import(&library, &bundle).unwrap_err().to_string();
            assert!(error.contains(expected), "unexpected {case} error: {error}");
            assert_failed_import_left_nothing(&library);
        }
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
        let lua = meta
            .components
            .iter()
            .find(|component| matches!(component, ComponentInfo::Ue4ssLua { .. }))
            .unwrap();
        assert_eq!(lua.footprint_coverage(), FootprintCoverage::Partial);
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
        let lua = meta
            .components
            .iter()
            .find(|component| matches!(component, ComponentInfo::Ue4ssLua { .. }))
            .unwrap();
        assert_eq!(lua.footprint_coverage(), FootprintCoverage::Exact);
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
        let lua = meta
            .components
            .iter()
            .find(|component| matches!(component, ComponentInfo::Ue4ssLua { .. }))
            .unwrap();
        assert_eq!(lua.footprint_coverage(), FootprintCoverage::Opaque);
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

    #[test]
    fn nested_bundle_reroot_preserves_benign_wrapper_siblings() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bundle = mk_goremod_bundle(tmp.path());
        let source = tmp.path().join("wrapped-source");
        let nested = source.join("Wrap/Sub");
        fs::create_dir_all(nested.parent().unwrap()).unwrap();
        fs::rename(bundle, &nested).unwrap();
        fs::write(source.join("README.txt"), b"read me").unwrap();
        fs::write(source.join("Wrap/LICENSE.txt"), b"license").unwrap();

        let meta = import(&lib, &source).unwrap();
        let entry = lib.join(meta.id);
        assert!(entry.join("gore-mod.json").is_file());
        assert_eq!(fs::read(entry.join("README.txt")).unwrap(), b"read me");
        assert_eq!(
            fs::read(entry.join("Wrap/LICENSE.txt")).unwrap(),
            b"license"
        );
    }

    #[test]
    fn nested_bundle_can_hoist_content_named_like_its_removed_wrapper() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bundle = mk_goremod_bundle(tmp.path());
        let source = tmp.path().join("same-name-wrapper");
        let nested = source.join("Wrap");
        fs::create_dir_all(&source).unwrap();
        fs::rename(bundle, &nested).unwrap();
        fs::create_dir(nested.join("Wrap")).unwrap();
        fs::write(nested.join("Wrap/README.txt"), b"inner payload").unwrap();

        let meta = import(&lib, &source).unwrap();
        let entry = lib.join(meta.id);
        assert!(entry.join("gore-mod.json").is_file());
        assert_eq!(
            fs::read(entry.join("Wrap/README.txt")).unwrap(),
            b"inner payload"
        );
    }

    #[test]
    fn retained_wrapper_sibling_still_blocks_a_same_name_hoist() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bundle = mk_goremod_bundle(tmp.path());
        let source = tmp.path().join("retained-wrapper");
        let nested = source.join("Wrap/Sub");
        fs::create_dir_all(nested.parent().unwrap()).unwrap();
        fs::rename(bundle, &nested).unwrap();
        fs::create_dir(nested.join("Wrap")).unwrap();
        fs::write(nested.join("Wrap/README.txt"), b"inner payload").unwrap();
        fs::write(source.join("Wrap/LICENSE.txt"), b"retains wrapper").unwrap();

        let error = import(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("would collide while re-rooting"), "{error}");
        assert_failed_import_left_nothing(&lib);
    }

    #[test]
    fn nested_bundle_reroot_rejects_deployable_siblings_without_activation() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bundle = mk_goremod_bundle(tmp.path());
        let source = tmp.path().join("wrapped-source");
        let nested = source.join("Wrap/Sub");
        fs::create_dir_all(nested.parent().unwrap()).unwrap();
        fs::rename(bundle, &nested).unwrap();
        fs::write(source.join("outside.pak"), b"must not be dropped").unwrap();

        let error = import(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("deployable or reserved sibling"), "{error}");
        assert_failed_import_left_nothing(&lib);
    }

    #[test]
    fn ambiguous_or_too_deep_gore_mod_manifests_never_fall_back_to_foreign() {
        let tmp = tempfile::tempdir().unwrap();
        let cases = ["multiple", "root-and-nested", "too-deep"];
        for case in cases {
            let source = tmp.path().join(format!("source-{case}"));
            fs::create_dir_all(&source).unwrap();
            fs::write(source.join("visible_P.pak"), b"otherwise importable").unwrap();
            match case {
                "multiple" => {
                    for child in ["A", "B"] {
                        let dir = source.join(child);
                        fs::create_dir_all(&dir).unwrap();
                        fs::write(dir.join("gore-mod.json"), b"{}").unwrap();
                    }
                }
                "root-and-nested" => {
                    fs::write(source.join("gore-mod.json"), b"{}").unwrap();
                    let nested = source.join("Nested");
                    fs::create_dir_all(&nested).unwrap();
                    fs::write(nested.join("gore-mod.json"), b"{}").unwrap();
                }
                "too-deep" => {
                    let nested = source.join("A/B/C");
                    fs::create_dir_all(&nested).unwrap();
                    fs::write(nested.join("gore-mod.json"), b"{}").unwrap();
                }
                _ => unreachable!(),
            }
            let lib = tmp.path().join(format!("lib-{case}"));
            let error = import(&lib, &source).unwrap_err().to_string();
            if case == "too-deep" {
                assert!(error.contains("nested too deeply"), "{case}: {error}");
            } else {
                assert!(error.contains("multiple gore-mod.json"), "{case}: {error}");
            }
            assert_failed_import_left_nothing(&lib);
        }
    }

    #[test]
    fn ambiguous_manifest_error_retains_only_bounded_first_two_evidence_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("many-manifests");
        for index in 0..64 {
            let dir = source.join(format!("manifest-{index:03}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("gore-mod.json"), b"{}").unwrap();
        }

        let error = find_manifest_dir(&source).unwrap_err().to_string();

        assert!(error.contains("first two"), "{error}");
        assert!(error.contains("manifest-000"), "{error}");
        assert!(error.contains("manifest-001"), "{error}");
        assert!(!error.contains("manifest-002"), "{error}");
        assert!(
            error.len() < 1_024,
            "ambiguity error grew unexpectedly: {} bytes",
            error.len()
        );
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
        assert_eq!(
            meta.components[0].footprint_coverage(),
            FootprintCoverage::Opaque,
            "an unreadable Pak keeps importing with an explicitly unknown footprint"
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
        assert_eq!(
            meta.components[0].footprint_coverage(),
            FootprintCoverage::Opaque,
            "an unreadable IoStore container keeps importing with an explicitly unknown footprint"
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

    #[test]
    fn case_distinct_iostore_members_are_refused_before_activation() {
        for (case, utoc, ucas, pak) in [
            ("ascii", "MixedCase.utoc", "mixedcase.ucas", None),
            ("unicode", "Ä.utoc", "ä.ucas", None),
            (
                "optional-pak",
                "Consistent.utoc",
                "Consistent.ucas",
                Some("consistent.pak"),
            ),
        ] {
            let tmp = tempfile::tempdir().unwrap();
            let lib = tmp.path().join("lib");
            let source = tmp.path().join(case);
            fs::create_dir_all(&source).unwrap();
            fs::write(source.join(utoc), b"utoc").unwrap();
            fs::write(source.join(ucas), b"ucas").unwrap();
            if let Some(pak) = pak {
                fs::write(source.join(pak), b"pak").unwrap();
            }

            let error = import(&lib, &source).unwrap_err().to_string();
            assert!(
                error.contains("not exactly reconstructable during Apply"),
                "unexpected {case} error: {error}"
            );
            assert!(
                !lib.exists() || fs::read_dir(&lib).unwrap().next().is_none(),
                "refused {case} import must not publish a library entry"
            );
        }
    }

    #[test]
    fn case_distinct_iostore_extension_is_refused_before_activation() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("UppercaseExtension");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("Exact.UTOC"), b"utoc").unwrap();
        fs::write(source.join("Exact.ucas"), b"ucas").unwrap();

        let error = import(&lib, &source).unwrap_err().to_string();
        assert!(
            error.contains("expected lowercase .utoc"),
            "unexpected extension error: {error}"
        );
        assert!(!lib.exists() || fs::read_dir(&lib).unwrap().next().is_none());
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
        assert_eq!(
            meta.components[0].footprint_coverage(),
            FootprintCoverage::Opaque
        );
        let entry = lib.join(&meta.id);
        assert!(entry.join("bar.ucas").is_file());
        assert!(entry.join("bar.pak").is_file());
    }

    #[test]
    fn relative_direct_container_selection_is_enumerated_once() {
        let current = std::env::current_dir().unwrap();
        let placeholder = tempfile::Builder::new()
            .prefix("gore-relative-container-")
            .suffix(".utoc")
            .tempfile_in(&current)
            .unwrap();
        let utoc = placeholder.path().to_path_buf();
        placeholder.close().unwrap();
        let ucas = utoc.with_extension("ucas");
        let _cleanup = RemoveFilesOnDrop(vec![utoc.clone(), ucas.clone()]);
        fs::write(&utoc, b"utoc").unwrap();
        fs::write(&ucas, b"ucas").unwrap();
        let selected = PathBuf::from(utoc.file_name().unwrap());
        assert!(selected
            .parent()
            .is_some_and(|parent| parent.as_os_str().is_empty()));

        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let meta = import(&lib, &selected).unwrap();

        assert_eq!(meta.kind, ModKind::ForeignTriplet);
        let entry = lib.join(meta.id);
        assert_eq!(
            fs::read(entry.join(utoc.file_name().unwrap())).unwrap(),
            b"utoc"
        );
        assert_eq!(
            fs::read(entry.join(ucas.file_name().unwrap())).unwrap(),
            b"ucas"
        );
    }

    #[test]
    fn incomplete_iostore_sets_are_rejected_even_beside_valid_foreign_content() {
        let tmp = tempfile::tempdir().unwrap();
        let cases: &[(&str, &[&str], &str)] = &[
            ("orphan-utoc", &["broken.utoc"], ".ucas"),
            ("orphan-ucas", &["broken.ucas"], ".utoc"),
            ("utoc-and-pak", &["broken_P.utoc", "broken_P.pak"], ".ucas"),
            ("ucas-and-pak", &["broken_P.ucas", "broken_P.pak"], ".utoc"),
        ];
        for &(case, members, missing) in cases {
            let source = tmp.path().join(case);
            fs::create_dir_all(&source).unwrap();
            fs::write(source.join("valid_P.pak"), b"valid loose pak").unwrap();
            fs::write(source.join("Music.bank"), b"valid raw file").unwrap();
            for member in members {
                fs::write(source.join(member), b"incomplete").unwrap();
            }
            let lib = tmp.path().join(format!("lib-{case}"));
            let error = import(&lib, &source).unwrap_err().to_string();
            assert!(error.contains("incomplete IoStore set"), "{case}: {error}");
            assert!(error.contains(missing), "{case}: {error}");
            assert_failed_import_left_nothing(&lib);
        }
    }

    #[test]
    fn directly_selected_incomplete_iostore_member_leaves_library_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("direct");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("broken_P.ucas"), b"ucas").unwrap();
        fs::write(source.join("broken_P.pak"), b"pak").unwrap();
        let lib = tmp.path().join("lib");

        let error = import(&lib, &source.join("broken_P.ucas"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("incomplete IoStore set"), "{error}");
        assert!(error.contains(".utoc"), "{error}");
        assert_failed_import_left_nothing(&lib);
    }

    #[test]
    fn directly_selected_multipart_reimport_preserves_previous_entry_byte_for_byte() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("direct-multipart");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("pair.utoc"), b"old utoc").unwrap();
        fs::write(source.join("pair.ucas"), b"old ucas").unwrap();
        let selected = source.join("pair.utoc");
        let lib = tmp.path().join("lib");
        let before = import(&lib, &selected).unwrap();
        let entry = lib.join(&before.id);
        let before_utoc = fs::read(entry.join("pair.utoc")).unwrap();
        let before_ucas = fs::read(entry.join("pair.ucas")).unwrap();
        let before_sidecar = fs::read(entry.join(META_FILE)).unwrap();

        fs::write(&selected, b"new utoc").unwrap();
        fs::write(source.join("pair.ucas"), b"new ucas").unwrap();
        fs::write(source.join("pair.ucas.1"), b"split payload").unwrap();
        let error = import(&lib, &selected).unwrap_err().to_string();
        assert!(error.contains("unsupported multipart IoStore"), "{error}");
        assert!(error.contains("pair.ucas.1"), "{error}");

        assert_eq!(fs::read(entry.join("pair.utoc")).unwrap(), before_utoc);
        assert_eq!(fs::read(entry.join("pair.ucas")).unwrap(), before_ucas);
        assert_eq!(fs::read(entry.join(META_FILE)).unwrap(), before_sidecar);
        assert_eq!(list(&lib).unwrap(), vec![before]);
        let mut library_names: Vec<_> = fs::read_dir(&lib)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name != ".gore-manager-library.lock")
            .collect();
        library_names.sort();
        assert_eq!(library_names, vec![entry.file_name().unwrap().to_owned()]);
    }

    #[test]
    fn zip_with_hidden_incomplete_iostore_set_is_rejected_before_activation() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("incomplete.zip");
        zip_entries(
            &archive,
            &[
                (
                    "valid_P.pak",
                    b"valid loose pak",
                    zip::CompressionMethod::Stored,
                ),
                ("broken_P.ucas", b"ucas", zip::CompressionMethod::Stored),
                ("broken_P.pak", b"pak", zip::CompressionMethod::Stored),
            ],
        );
        let lib = tmp.path().join("lib");

        let error = import(&lib, &archive).unwrap_err().to_string();
        assert!(error.contains("incomplete IoStore set"), "{error}");
        assert!(error.contains(".utoc"), "{error}");
        assert_failed_import_left_nothing(&lib);
    }

    #[test]
    fn multipart_iostore_leftovers_are_not_hidden_by_a_valid_pair() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("multipart");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("broken.utoc"), b"utoc").unwrap();
        fs::write(source.join("broken.ucas"), b"ucas").unwrap();
        fs::write(source.join("broken.ucas.1"), b"partition").unwrap();
        fs::write(source.join("valid_P.pak"), b"valid loose pak").unwrap();
        let lib = tmp.path().join("lib");

        let error = import(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("unsupported multipart IoStore"), "{error}");
        assert!(error.contains("broken.ucas.1"), "{error}");
        assert_failed_import_left_nothing(&lib);
    }

    #[test]
    fn split_pak_members_are_rejected_in_folders_and_zips() {
        let tmp = tempfile::tempdir().unwrap();

        let folder = tmp.path().join("split-pak-folder");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("valid_P.pak"), b"valid loose pak").unwrap();
        fs::write(folder.join("broken_P.pak.1"), b"split payload").unwrap();
        let folder_lib = tmp.path().join("folder-lib");
        let error = import(&folder_lib, &folder).unwrap_err().to_string();
        assert!(error.contains("unsupported multipart IoStore"), "{error}");
        assert!(error.contains("broken_P.pak.1"), "{error}");
        assert_failed_import_left_nothing(&folder_lib);

        let archive = tmp.path().join("split-pak.zip");
        zip_entries(
            &archive,
            &[
                (
                    "valid_P.pak",
                    b"valid loose pak",
                    zip::CompressionMethod::Stored,
                ),
                (
                    "broken_P.pak.7",
                    b"split payload",
                    zip::CompressionMethod::Stored,
                ),
            ],
        );
        let zip_lib = tmp.path().join("zip-lib");
        let error = import(&zip_lib, &archive).unwrap_err().to_string();
        assert!(error.contains("unsupported multipart IoStore"), "{error}");
        assert!(error.contains("broken_P.pak.7"), "{error}");
        assert_failed_import_left_nothing(&zip_lib);

        let paired_folder = tmp.path().join("split-optional-pair-pak");
        fs::create_dir_all(&paired_folder).unwrap();
        fs::write(paired_folder.join("bar.utoc"), b"utoc").unwrap();
        fs::write(paired_folder.join("bar.ucas"), b"ucas").unwrap();
        fs::write(paired_folder.join("bar.pak.1"), b"split payload").unwrap();
        let paired_lib = tmp.path().join("paired-lib");
        let error = import(&paired_lib, &paired_folder).unwrap_err().to_string();
        assert!(error.contains("unsupported multipart IoStore"), "{error}");
        assert!(error.contains("bar.pak.1"), "{error}");
        assert_failed_import_left_nothing(&paired_lib);
    }

    #[test]
    fn unsupported_standalone_pak_is_rejected_without_replacing_or_hiding_content() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("standalone-pak-folder");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("valid_P.pak"), b"old loose pak").unwrap();
        let lib = tmp.path().join("folder-lib");
        let before = import(&lib, &source).unwrap();
        let entry = lib.join(&before.id);
        let before_pak = fs::read(entry.join("valid_P.pak")).unwrap();
        let before_sidecar = fs::read(entry.join(META_FILE)).unwrap();

        fs::write(source.join("valid_P.pak"), b"new loose pak").unwrap();
        fs::write(source.join("hidden.pak"), b"unsupported payload").unwrap();
        let error = import(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("unsupported standalone .pak"), "{error}");
        assert!(error.contains("hidden"), "{error}");
        assert_eq!(fs::read(entry.join("valid_P.pak")).unwrap(), before_pak);
        assert_eq!(fs::read(entry.join(META_FILE)).unwrap(), before_sidecar);
        assert_eq!(list(&lib).unwrap(), vec![before]);
        let library_names = fs::read_dir(&lib)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name != ".gore-manager-library.lock")
            .collect::<Vec<_>>();
        assert_eq!(library_names, vec![entry.file_name().unwrap().to_owned()]);

        let archive = tmp.path().join("standalone-pak.zip");
        zip_entries(
            &archive,
            &[
                ("valid_P.pak", b"loose pak", zip::CompressionMethod::Stored),
                (
                    "hidden.pak",
                    b"unsupported payload",
                    zip::CompressionMethod::Stored,
                ),
            ],
        );
        let zip_lib = tmp.path().join("zip-lib");
        let error = import(&zip_lib, &archive).unwrap_err().to_string();
        assert!(error.contains("unsupported standalone .pak"), "{error}");
        assert!(error.contains("hidden"), "{error}");
        assert_failed_import_left_nothing(&zip_lib);
    }

    #[test]
    fn refused_iostore_reimport_preserves_the_previous_library_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("PairMod");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("pair.utoc"), b"old utoc").unwrap();
        fs::write(source.join("pair.ucas"), b"old ucas").unwrap();
        let lib = tmp.path().join("lib");
        let before = import(&lib, &source).unwrap();
        let entry = lib.join(&before.id);
        let before_sidecar = fs::read(entry.join(META_FILE)).unwrap();

        fs::remove_file(source.join("pair.ucas")).unwrap();
        fs::write(source.join("pair.utoc"), b"new but incomplete").unwrap();
        let error = import(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("incomplete IoStore set"), "{error}");
        assert_eq!(fs::read(entry.join("pair.utoc")).unwrap(), b"old utoc");
        assert_eq!(fs::read(entry.join("pair.ucas")).unwrap(), b"old ucas");
        assert_eq!(fs::read(entry.join(META_FILE)).unwrap(), before_sidecar);
        assert_eq!(list(&lib).unwrap(), vec![before]);
    }

    #[test]
    fn duplicate_raw_deployment_targets_are_rejected_case_insensitively() {
        let tmp = tempfile::tempdir().unwrap();
        let cases = [
            (
                "lcache",
                vec![("A/one.lcache", b"a".as_slice()), ("B/two.lcache", b"b")],
                "lcache",
            ),
            (
                "script-cache",
                vec![
                    ("A/PrecompiledScript_One.Cache", b"a".as_slice()),
                    ("B/PrecompiledScript_Two.Cache", b"b"),
                ],
                "script_cache",
            ),
            (
                "bank-case",
                vec![("A/SFX.bank", b"a".as_slice()), ("B/sfx.BANK", b"b")],
                "bank:sfx.bank",
            ),
        ];
        for (case, files, target) in cases {
            let source = tmp.path().join(case);
            for &(relative, bytes) in &files {
                let path = source.join(relative);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, bytes).unwrap();
            }
            let lib = tmp.path().join(format!("lib-{case}"));
            let error = import(&lib, &source).unwrap_err().to_string();
            assert!(error.contains("duplicate raw deployment target"), "{error}");
            assert!(error.contains(target), "{error}");
            for (relative, _) in files {
                assert!(error.contains(relative), "{case}: {error}");
            }
            assert_failed_import_left_nothing(&lib);
        }
    }

    #[test]
    fn duplicate_bank_targets_are_rejected_with_unicode_case_folding() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("unicode-bank-case");
        for (relative, bytes) in [("A/Ä.bank", b"a"), ("B/ä.BANK", b"b")] {
            let path = source.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        let lib = tmp.path().join("lib");

        let error = import(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("duplicate raw deployment target"), "{error}");
        assert!(error.contains("bank:ä.bank"), "{error}");
        assert!(error.contains("A/Ä.bank"), "{error}");
        assert!(error.contains("B/ä.BANK"), "{error}");
        assert_failed_import_left_nothing(&lib);
    }

    #[test]
    fn foreign_scan_rejects_links_instead_of_omitting_them() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("staged");
        fs::create_dir_all(&root).unwrap();
        let outside = tmp.path().join("outside_P.pak");
        fs::write(&outside, b"outside").unwrap();
        if !make_file_link(&outside, &root.join("linked_P.pak")) {
            return;
        }

        let error = scan_foreign(&root).unwrap_err().to_string();
        assert!(error.contains("symbolic link or reparse point"), "{error}");
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
        assert_eq!(
            meta.components[0].footprint_coverage(),
            FootprintCoverage::Opaque,
            "a foreign UE4SS tree has no complete declared script footprint"
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

    /// An unchanged same-source re-import keeps the stable id, timestamp, and fingerprint.
    #[test]
    fn reimport_same_source_unchanged_preserves_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = tmp.path().join("BarMod");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("bar.utoc"), b"junk").unwrap();
        fs::write(src.join("bar.ucas"), b"junk").unwrap();

        let a = import_detailed(&lib, &src).unwrap();
        let sidecar_before = fs::read(lib.join(&a.entry.id).join(META_FILE)).unwrap();
        let b = import_detailed(&lib, &src).unwrap();
        assert_eq!(a.entry.id, b.entry.id);
        assert_eq!(b.disposition, ImportDisposition::Unchanged);
        assert_eq!(b.matched_by, ImportMatchedBy::Source);
        assert_eq!(a.entry.imported_at, b.entry.imported_at);
        assert_eq!(a.entry.fingerprint(), b.entry.fingerprint());
        assert_eq!(
            fs::read(lib.join(&a.entry.id).join(META_FILE)).unwrap(),
            sidecar_before
        );
        assert_eq!(list(&lib).unwrap().len(), 1);
        // No no-op import may manufacture a replacement transaction.
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
    fn moved_folder_rebind_then_changed_same_source_updates_the_original_id() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let original = tmp.path().join("original-mod");
        let moved = tmp.path().join("moved-mod");
        fs::create_dir(&original).unwrap();
        fs::write(original.join("payload_P.pak"), b"opaque-v1").unwrap();

        let first = import_detailed(&lib, &original).unwrap();
        fs::rename(&original, &moved).unwrap();
        let rebound = import_detailed(&lib, &moved).unwrap();
        assert_eq!(rebound.entry.id, first.entry.id);
        assert_eq!(rebound.disposition, ImportDisposition::Updated);
        assert_eq!(rebound.matched_by, ImportMatchedBy::Content);
        assert_eq!(rebound.entry.imported_at, first.entry.imported_at);
        assert_eq!(rebound.entry.fingerprint(), first.entry.fingerprint());

        std::thread::sleep(std::time::Duration::from_millis(2));
        fs::write(moved.join("payload_P.pak"), b"opaque-v2").unwrap();
        let changed = import_detailed(&lib, &moved).unwrap();
        assert_eq!(changed.entry.id, first.entry.id);
        assert_eq!(changed.disposition, ImportDisposition::Updated);
        assert_eq!(changed.matched_by, ImportMatchedBy::Source);
        assert_ne!(changed.entry.imported_at, first.entry.imported_at);
        assert_ne!(changed.entry.fingerprint(), first.entry.fingerprint());
        assert_eq!(list(&lib).unwrap().len(), 1);
    }

    #[test]
    fn legacy_proposed_id_equal_tree_is_bound_without_churning_import_time() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("legacy_P.pak");
        fs::write(&source, b"legacy opaque bytes").unwrap();
        let first = import_detailed(&lib, &source).unwrap();
        let mut legacy = read_library_sidecar(&lib, &first.entry.id);
        legacy.manager = None;
        write_library_sidecar(&lib, &first.entry.id, &legacy);

        let rebound = import_detailed(&lib, &source).unwrap();
        assert_eq!(rebound.entry.id, first.entry.id);
        assert_eq!(rebound.matched_by, ImportMatchedBy::EntryId);
        assert_eq!(rebound.disposition, ImportDisposition::Updated);
        assert_eq!(rebound.entry.imported_at, first.entry.imported_at);
        assert_eq!(rebound.entry.fingerprint(), first.entry.fingerprint());
        assert!(read_library_sidecar(&lib, &first.entry.id)
            .manager
            .and_then(|manager| manager.import_identity)
            .is_some());
    }

    #[test]
    fn duplicate_verified_content_refuses_with_two_bounded_ids_and_no_mutation() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let original_dir = tmp.path().join("original");
        let moved_dir = tmp.path().join("moved");
        fs::create_dir(&original_dir).unwrap();
        fs::create_dir(&moved_dir).unwrap();
        let original = original_dir.join("same_P.pak");
        let moved = moved_dir.join("same_P.pak");
        fs::write(&original, b"same corrupt pak bytes").unwrap();
        fs::write(&moved, b"same corrupt pak bytes").unwrap();
        let first = import_detailed(&lib, &original).unwrap();

        let duplicate_id = "verified-duplicate";
        copy_test_tree(&lib.join(&first.entry.id), &lib.join(duplicate_id));
        let mut duplicate = read_library_sidecar(&lib, duplicate_id);
        duplicate.entry.id = duplicate_id.into();
        let identity = duplicate
            .manager
            .as_mut()
            .and_then(|manager| manager.import_identity.as_mut())
            .unwrap();
        identity.source_sha256 = "0".repeat(64);
        write_library_sidecar(&lib, duplicate_id, &duplicate);
        let before = visible_library_snapshot(&lib);

        let error = import_detailed(&lib, &moved).unwrap_err();
        let ImportError::DuplicateAmbiguous { candidate_ids } = error else {
            panic!("unexpected error: {error}");
        };
        assert_eq!(candidate_ids.len(), 2);
        assert!(candidate_ids.contains(&first.entry.id));
        assert!(candidate_ids.contains(&duplicate_id.to_owned()));
        assert_eq!(visible_library_snapshot(&lib), before);
        assert_no_import_residue(&lib);
    }

    #[test]
    fn source_candidate_and_distinct_content_candidate_refuse_without_mutation() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let a_dir = tmp.path().join("a");
        let b_dir = tmp.path().join("b");
        fs::create_dir(&a_dir).unwrap();
        fs::create_dir(&b_dir).unwrap();
        let a = a_dir.join("same_P.pak");
        let b = b_dir.join("same_P.pak");
        fs::write(&a, b"old source bytes").unwrap();
        fs::write(&b, b"new source bytes").unwrap();
        let a_entry = import_detailed(&lib, &a).unwrap();
        let b_entry = import_detailed(&lib, &b).unwrap();
        for duplicate_id in ["000-content-a", "001-content-b"] {
            copy_test_tree(&lib.join(&b_entry.entry.id), &lib.join(duplicate_id));
            let mut duplicate = read_library_sidecar(&lib, duplicate_id);
            duplicate.entry.id = duplicate_id.into();
            write_library_sidecar(&lib, duplicate_id, &duplicate);
        }
        fs::write(&a, b"new source bytes").unwrap();
        let before = visible_library_snapshot(&lib);

        let error = import_detailed(&lib, &a).unwrap_err();
        let ImportError::IdentityConflict { candidates } = error else {
            panic!("unexpected error: {error}");
        };
        let mut expected = vec![
            ImportConflictCandidate {
                id: a_entry.entry.id,
                matched_by: vec![ImportMatchedBy::EntryId, ImportMatchedBy::Source],
            },
            ImportConflictCandidate {
                id: "000-content-a".into(),
                matched_by: vec![ImportMatchedBy::Content],
            },
        ];
        expected.sort_by(|left, right| left.id.cmp(&right.id));
        assert_eq!(candidates, expected);
        assert_eq!(visible_library_snapshot(&lib), before);
        assert_no_import_residue(&lib);
    }

    #[test]
    fn managed_proposed_id_collision_refuses_without_overwriting() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let first_source = tmp.path().join("first_P.pak");
        let colliding_source = tmp.path().join("different_P.pak");
        fs::write(&first_source, b"first managed bytes").unwrap();
        fs::write(&colliding_source, b"different managed bytes").unwrap();
        let first = import_detailed(&lib, &first_source).unwrap();
        let before = visible_library_snapshot(&lib);

        inject_proposed_import_id(first.entry.id.clone());
        let error = import_detailed(&lib, &colliding_source).unwrap_err();
        let ImportError::IdentityConflict { candidates } = error else {
            panic!("unexpected error: {error}");
        };
        assert_eq!(
            candidates,
            vec![ImportConflictCandidate {
                id: first.entry.id,
                matched_by: vec![ImportMatchedBy::EntryId],
            }]
        );
        assert_eq!(visible_library_snapshot(&lib), before);
        assert_no_import_residue(&lib);
    }

    #[test]
    fn successive_changed_trees_get_monotonic_timestamps_under_a_fixed_or_backward_clock() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("clock_P.pak");
        let fixed = "2026-08-11T12:34:56.000000Z";
        fs::write(&source, b"version one").unwrap();
        inject_import_timestamp(fixed);
        let first = import_detailed(&lib, &source).unwrap();

        fs::write(&source, b"version two").unwrap();
        inject_import_timestamp(fixed);
        let changed = import_detailed(&lib, &source).unwrap();
        assert_eq!(changed.entry.id, first.entry.id);
        assert_eq!(changed.disposition, ImportDisposition::Updated);
        assert_eq!(changed.matched_by, ImportMatchedBy::Source);
        assert_eq!(first.entry.imported_at, fixed);
        assert_eq!(changed.entry.imported_at, "2026-08-11T12:34:56.000001Z");
        assert_ne!(changed.entry.fingerprint(), first.entry.fingerprint());

        fs::write(&source, b"version three").unwrap();
        inject_import_timestamp("2026-08-10T01:00:00.000000Z");
        let changed_again = import_detailed(&lib, &source).unwrap();
        assert_eq!(changed_again.entry.id, first.entry.id);
        assert_eq!(changed_again.disposition, ImportDisposition::Updated);
        assert_eq!(
            changed_again.entry.imported_at,
            "2026-08-11T12:34:56.000002Z"
        );
        assert_ne!(
            changed_again.entry.fingerprint(),
            changed.entry.fingerprint()
        );
        assert_ne!(changed_again.entry.fingerprint(), first.entry.fingerprint());
    }

    #[test]
    fn staged_and_current_drift_after_decision_abort_before_replacement() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("race_P.pak");
        fs::write(&source, b"old source bytes").unwrap();
        let first = import_detailed(&lib, &source).unwrap();
        fs::write(&source, b"new source bytes").unwrap();
        let before_staged_race = visible_library_snapshot(&lib);

        inject_prepublish_race(|staging, _entry| {
            fs::write(staging.join("race_P.pak"), b"raced staged bytes").unwrap();
        });
        let error = import_detailed(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("staged import changed"), "{error}");
        assert_eq!(visible_library_snapshot(&lib), before_staged_race);
        assert_no_import_residue(&lib);

        let original_sidecar = fs::read(lib.join(&first.entry.id).join(META_FILE)).unwrap();
        inject_prepublish_race(|_staging, entry| {
            fs::write(entry.join("race_P.pak"), b"external current edit").unwrap();
        });
        let error = import_detailed(&lib, &source).unwrap_err().to_string();
        assert!(
            error.contains("selected manager-library entry changed"),
            "{error}"
        );
        assert_eq!(
            fs::read(lib.join(&first.entry.id).join("race_P.pak")).unwrap(),
            b"external current edit"
        );
        assert_eq!(
            fs::read(lib.join(&first.entry.id).join(META_FILE)).unwrap(),
            original_sidecar
        );
        assert_no_import_residue(&lib);
    }

    #[test]
    fn equal_byte_current_directory_swap_fails_the_retained_root_identity_seal() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("identity-swap_P.pak");
        fs::write(&source, b"previous payload").unwrap();
        let first = import_detailed(&lib, &source).unwrap();
        fs::write(&source, b"intended update").unwrap();
        let before = visible_library_snapshot(&lib);
        let displaced = lib.join(".externally-displaced-entry");
        let displaced_for_hook = displaced.clone();
        inject_prepublish_race(move |_staging, entry| {
            fs::rename(entry, &displaced_for_hook).unwrap();
            copy_test_tree(&displaced_for_hook, entry);
        });

        let error = import_detailed(&lib, &source).unwrap_err().to_string();
        assert!(
            error.contains("selected manager-library entry changed"),
            "{error}"
        );
        assert_eq!(visible_library_snapshot(&lib), before);
        assert_eq!(
            fs::read(lib.join(&first.entry.id).join("identity-swap_P.pak")).unwrap(),
            b"previous payload"
        );
        fs::remove_dir_all(displaced).unwrap();
        assert_no_import_residue(&lib);
    }

    #[test]
    fn failed_create_post_rename_seal_never_leaves_a_public_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("library");
        let source = tmp.path().join("create_P.pak");
        fs::write(&source, b"sealed create bytes").unwrap();
        inject_post_create_rename(|entry| {
            fs::write(entry.join("create_P.pak"), b"changed after rename").unwrap();
        });

        let error = import_detailed(&library, &source).unwrap_err().to_string();
        assert!(error.contains("post-rename seal"), "{error}");
        let visible = fs::read_dir(&library)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
            .collect::<Vec<_>>();
        assert!(
            visible.is_empty(),
            "failed create became public: {visible:?}"
        );
        let transaction = fs::read_dir(&library)
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(REPLACEMENT_PREFIX)
            })
            .expect("failed create retains a durable transaction")
            .path();
        assert!(transaction.join(REPLACEMENT_STATE_FILE).is_file());
        assert!(transaction.join(REPLACEMENT_QUARANTINE_DIR).is_dir());
        assert!(list(&library)
            .unwrap_err()
            .to_string()
            .contains("quarantined"));
    }

    #[test]
    fn quarantine_marker_failure_cannot_make_recovery_delete_verified_previous_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("library");
        let source = tmp.path().join("replace_P.pak");
        fs::write(&source, b"verified previous bytes").unwrap();
        let first = import_detailed(&library, &source).unwrap();
        let before = visible_library_snapshot(&library);

        fs::write(&source, b"intended promoted bytes").unwrap();
        inject_post_promote_rename(|entry| {
            fs::write(entry.join("replace_P.pak"), b"unverified promoted bytes").unwrap();
        });
        inject_replacement_mark_failure(ReplacementPhase::Quarantined);
        let error = import_detailed(&library, &source).unwrap_err().to_string();
        assert!(error.contains("replacement-marker failure"), "{error}");
        assert_eq!(visible_library_snapshot(&library), before);
        assert_eq!(
            fs::read(library.join(&first.entry.id).join("replace_P.pak")).unwrap(),
            b"verified previous bytes"
        );

        let transaction = fs::read_dir(&library)
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(REPLACEMENT_PREFIX)
            })
            .expect("marker failure retains a durable transaction")
            .path();
        let state_path = transaction.join(REPLACEMENT_STATE_FILE);
        assert!(state_path.is_file());
        let state: ReplacementState =
            serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
        assert_eq!(state.format, 2);
        assert_eq!(state.id, first.entry.id);
        assert_eq!(state.phase, ReplacementPhase::Prepared);
        assert!(state.verification_pending);
        let previous = state
            .expected_previous
            .expect("format-2 update journal seals the previous object");
        let staged = state
            .expected_staged
            .expect("format-2 update journal seals the staged object");
        assert_ne!(previous.tree_sha256, staged.tree_sha256);
        assert!(transaction.join(REPLACEMENT_QUARANTINE_DIR).is_dir());
        assert!(!transaction
            .join(ReplacementPhase::Quarantined.marker().unwrap())
            .exists());
        let recovery = list(&library).unwrap_err().to_string();
        assert!(recovery.contains("verification") || recovery.contains("quarantined"));
        assert_eq!(visible_library_snapshot(&library), before);
    }

    #[test]
    fn public_metadata_ignores_private_shape_but_identity_import_is_strict() {
        let private_values = [
            serde_json::json!("future-private-v2"),
            serde_json::json!({
                "import_identity": {
                    "format": 1,
                    "source_sha256": "0".repeat(64),
                    "tree_sha256": "0".repeat(64)
                },
                "future_field": true
            }),
        ];
        for private in private_values {
            let tmp = tempfile::tempdir().unwrap();
            let lib = tmp.path().join("lib");
            let source = tmp.path().join("private_P.pak");
            fs::write(&source, b"private parse bytes").unwrap();
            let first = import_detailed(&lib, &source).unwrap();
            let sidecar_path = lib.join(&first.entry.id).join(META_FILE);
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&sidecar_path).unwrap()).unwrap();
            value["_manager"] = private;
            fs::write(&sidecar_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
            let before = visible_library_snapshot(&lib);

            assert_eq!(list(&lib).unwrap(), vec![first.entry.clone()]);
            let error = import_detailed(&lib, &source).unwrap_err().to_string();
            assert!(error.contains("private identity metadata"), "{error}");
            assert_eq!(visible_library_snapshot(&lib), before);
            assert_no_import_residue(&lib);
        }
    }

    #[test]
    fn unrelated_unsafe_or_corrupt_public_entry_makes_identity_import_fail_closed() {
        for case in ["plain-file", "missing-sidecar", "corrupt-sidecar"] {
            let tmp = tempfile::tempdir().unwrap();
            let lib = tmp.path().join("lib");
            fs::create_dir(&lib).unwrap();
            match case {
                "plain-file" => fs::write(lib.join("unrelated"), b"not a directory").unwrap(),
                "missing-sidecar" => fs::create_dir(lib.join("unrelated")).unwrap(),
                "corrupt-sidecar" => {
                    fs::create_dir(lib.join("unrelated")).unwrap();
                    fs::write(lib.join("unrelated").join(META_FILE), b"{broken").unwrap();
                }
                _ => unreachable!(),
            }
            let source = tmp.path().join("new_P.pak");
            fs::write(&source, b"new import bytes").unwrap();
            let before = visible_library_snapshot(&lib);

            let error = import_detailed(&lib, &source).unwrap_err().to_string();
            assert!(
                error.contains("unsafe or unreadable")
                    || error.contains("unreadable public sidecar"),
                "{case}: {error}"
            );
            assert_eq!(visible_library_snapshot(&lib), before);
            assert_no_import_residue(&lib);
        }
    }

    #[test]
    fn selected_hint_tree_tamper_and_prepublish_failure_both_preserve_visible_library() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("tamper_P.pak");
        fs::write(&source, b"original").unwrap();
        let first = import_detailed(&lib, &source).unwrap();
        fs::write(lib.join(&first.entry.id).join("tamper_P.pak"), b"tampered").unwrap();
        let tampered_before = visible_library_snapshot(&lib);
        let error = import_detailed(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("manager-library tampering"), "{error}");
        assert_eq!(visible_library_snapshot(&lib), tampered_before);
        assert_no_import_residue(&lib);

        fs::write(lib.join(&first.entry.id).join("tamper_P.pak"), b"original").unwrap();
        fs::write(&source, b"updated").unwrap();
        let before_publish = visible_library_snapshot(&lib);
        inject_prepublish_failure(ModError::Other("injected prepublish failure".into()));
        let error = import_detailed(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("injected prepublish failure"), "{error}");
        assert_eq!(visible_library_snapshot(&lib), before_publish);
        assert_no_import_residue(&lib);
    }

    #[test]
    fn identity_candidate_rehashes_share_one_aggregate_entry_budget() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let first_source = tmp.path().join("first_P.pak");
        let second_source = tmp.path().join("second_P.pak");
        let incoming = tmp.path().join("incoming");
        fs::write(&first_source, b"first candidate").unwrap();
        fs::write(&second_source, b"second candidate").unwrap();
        fs::create_dir(&incoming).unwrap();
        fs::write(incoming.join("incoming_P.pak"), b"incoming candidate").unwrap();
        let first = import_detailed(&lib, &first_source).unwrap();
        let second = import_detailed(&lib, &second_source).unwrap();
        let incoming_hash = hash_normalized_source_path(&fs::canonicalize(&incoming).unwrap());
        for id in [&first.entry.id, &second.entry.id] {
            let mut sidecar = read_library_sidecar(&lib, id);
            sidecar
                .manager
                .as_mut()
                .unwrap()
                .import_identity
                .as_mut()
                .unwrap()
                .source_sha256 = incoming_hash.clone();
            write_library_sidecar(&lib, id, &sidecar);
        }
        let before = visible_library_snapshot(&lib);

        let error = import_detailed_with_limits(
            &lib,
            &incoming,
            ImportLimits {
                max_zip_entries: 1,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ImportError::Failed(ModError::InspectionBound(_))),
            "{error}"
        );
        assert!(error.to_string().contains("aggregate entry work"));
        assert_eq!(visible_library_snapshot(&lib), before);
        assert_no_import_residue(&lib);
    }

    #[test]
    fn maximum_canonical_import_timestamp_refuses_changed_bytes_without_mutation() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("timestamp_P.pak");
        fs::write(&source, b"first timestamp bytes").unwrap();
        let first = import_detailed(&lib, &source).unwrap();
        let mut sidecar = read_library_sidecar(&lib, &first.entry.id);
        sidecar.entry.imported_at = "9999-12-31T23:59:59.999999Z".into();
        write_library_sidecar(&lib, &first.entry.id, &sidecar);
        fs::write(&source, b"changed timestamp bytes").unwrap();
        let before = visible_library_snapshot(&lib);

        let error = import_detailed(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("timestamp exhausted"), "{error}");
        assert_eq!(visible_library_snapshot(&lib), before);
        assert_no_import_residue(&lib);
    }

    #[test]
    fn manager_sidecar_serialization_is_capped_before_any_file_is_published() {
        let tmp = tempfile::tempdir().unwrap();
        let staging = tmp.path().join("staging");
        fs::create_dir(&staging).unwrap();
        let secured = open_directory_nofollow(&staging, "bounded sidecar test").unwrap();
        let sidecar = LibrarySidecar {
            entry: ModEntryMeta {
                id: "bounded".into(),
                kind: ModKind::ForeignPak,
                name: "x".repeat(1_024),
                version: String::new(),
                author: String::new(),
                imported_at: "2026-01-01T00:00:00.000000Z".into(),
                source: "bounded_P.pak".into(),
                components: Vec::new(),
            },
            manager: None,
        };

        let error = write_manager_sidecar(&secured, &sidecar, 128)
            .unwrap_err()
            .to_string();
        assert!(error.contains("serialization exceeds"), "{error}");
        assert!(!staging.join(META_FILE).exists());
    }

    #[test]
    fn equal_corrupt_pak_and_iostore_inputs_rebind_and_remain_opaque() {
        for kind in ["pak", "iostore"] {
            let tmp = tempfile::tempdir().unwrap();
            let lib = tmp.path().join("lib");
            let a = tmp.path().join("a");
            let b = tmp.path().join("b");
            fs::create_dir(&a).unwrap();
            fs::create_dir(&b).unwrap();
            if kind == "pak" {
                fs::write(a.join("broken_P.pak"), b"not a pak").unwrap();
                fs::write(b.join("broken_P.pak"), b"not a pak").unwrap();
            } else {
                for extension in ["utoc", "ucas", "pak"] {
                    fs::write(a.join(format!("broken.{extension}")), b"not iostore").unwrap();
                    fs::write(b.join(format!("broken.{extension}")), b"not iostore").unwrap();
                }
            }
            let first = import_detailed(&lib, &a).unwrap();
            assert!(first.entry.components.iter().all(|component| {
                component.footprint_coverage() == FootprintCoverage::Opaque
                    || matches!(component, ComponentInfo::Triplet { targets, .. } if targets.is_empty())
            }));
            let moved = import_detailed(&lib, &b).unwrap();
            assert_eq!(moved.entry.id, first.entry.id);
            assert_eq!(moved.matched_by, ImportMatchedBy::Content);
            assert_eq!(list(&lib).unwrap().len(), 1);
        }
    }

    #[test]
    fn process_shared_library_lock_serializes_equal_parallel_imports() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let a_dir = tmp.path().join("a");
        let b_dir = tmp.path().join("b");
        fs::create_dir(&a_dir).unwrap();
        fs::create_dir(&b_dir).unwrap();
        let a = a_dir.join("same_P.pak");
        let b = b_dir.join("same_P.pak");
        fs::write(&a, b"parallel identical").unwrap();
        fs::write(&b, b"parallel identical").unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let mut workers = Vec::new();
        for source in [a, b] {
            let library = lib.clone();
            let barrier = barrier.clone();
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                import_detailed(&library, &source).unwrap()
            }));
        }
        barrier.wait();
        let outcomes: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        assert_eq!(outcomes[0].entry.id, outcomes[1].entry.id);
        assert_eq!(list(&lib).unwrap().len(), 1);
        assert_no_import_residue(&lib);
    }

    #[test]
    #[ignore = "child-process worker; invoked explicitly by cross-process lock tests"]
    fn library_lock_child_worker() {
        let library = PathBuf::from(std::env::var_os("GORE_TEST_CHILD_LIBRARY").unwrap());
        match std::env::var("GORE_TEST_CHILD_MODE").unwrap().as_str() {
            "lock" => {
                fs::create_dir_all(&library).unwrap();
                let _guard = library_mutation_lock(&library).unwrap();
            }
            "import" => {
                let source = PathBuf::from(std::env::var_os("GORE_TEST_CHILD_SOURCE").unwrap());
                let result = PathBuf::from(std::env::var_os("GORE_TEST_CHILD_RESULT").unwrap());
                let outcome = import_detailed(&library, &source).unwrap();
                fs::write(result, outcome.entry.id).unwrap();
            }
            mode => panic!("unknown child worker mode {mode:?}"),
        }
    }

    fn spawn_library_child(
        library: &Path,
        mode: &str,
        marker: &Path,
        hold_ms: u64,
        source: Option<&Path>,
        result: Option<&Path>,
    ) -> std::process::Child {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .arg("library_lock_child_worker")
            .arg("--ignored")
            .arg("--nocapture")
            .env("GORE_TEST_CHILD_LIBRARY", library)
            .env("GORE_TEST_CHILD_MODE", mode)
            .env(LIBRARY_LOCK_MARKER_ENV, marker)
            .env(LIBRARY_LOCK_HOLD_MS_ENV, hold_ms.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        if let Some(source) = source {
            command.env("GORE_TEST_CHILD_SOURCE", source);
        }
        if let Some(result) = result {
            command.env("GORE_TEST_CHILD_RESULT", result);
        }
        command.spawn().unwrap()
    }

    fn wait_for_test_path(path: &Path) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !path.exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {}",
                path.display()
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn assert_child_success(child: std::process::Child) {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "child failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn kernel_library_lock_blocks_other_process_and_is_released_by_crash() {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("library");
        let marker_a = tmp.path().join("locked-a");
        let marker_b = tmp.path().join("locked-b");
        let mut first = spawn_library_child(&library, "lock", &marker_a, 60_000, None, None);
        wait_for_test_path(&marker_a);
        let mut second = spawn_library_child(&library, "lock", &marker_b, 0, None, None);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(
            !marker_b.exists(),
            "second process acquired a held library lock"
        );
        assert!(
            second.try_wait().unwrap().is_none(),
            "second process did not block"
        );

        first.kill().unwrap();
        first.wait().unwrap();
        assert_child_success(second);
        assert!(marker_b.exists());
        #[cfg(windows)]
        assert!(library.join(".gore-manager-library.lock").is_file());
        #[cfg(unix)]
        assert!(!library.join(".gore-manager-library.lock").exists());
    }

    #[cfg(windows)]
    #[test]
    fn kernel_library_lock_rejects_a_non_regular_persistent_path() {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("library");
        fs::create_dir(&library).unwrap();
        fs::create_dir(library.join(".gore-manager-library.lock")).unwrap();

        let error = match library_mutation_lock(&library) {
            Ok(_) => panic!("directory lock path was accepted"),
            Err(error) => error.to_string(),
        };
        assert!(
            error.contains("mutation lock") || error.contains("Access is denied"),
            "{error}"
        );
        assert!(library.join(".gore-manager-library.lock").is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn unix_lock_owns_the_directory_inode_and_creates_no_replaceable_lockfile() {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("library");
        fs::create_dir(&library).unwrap();
        let guard = library_mutation_lock(&library).unwrap();
        assert_eq!(
            guard.open_library().unwrap().identity(),
            guard.os.identity()
        );
        assert!(!library.join(".gore-manager-library.lock").exists());
    }

    #[cfg(unix)]
    #[test]
    fn unix_root_swap_never_redirects_import_into_the_unlocked_replacement_path() {
        let tmp = tempfile::tempdir().unwrap();
        let configured = tmp.path().join("library");
        let retained = tmp.path().join("retained-library");
        let source = tmp.path().join("root-swap_P.pak");
        fs::write(&source, b"root swap bytes").unwrap();
        let configured_for_hook = configured.clone();
        let retained_for_hook = retained.clone();
        inject_library_root_swap(move |_locked_path| {
            fs::rename(&configured_for_hook, &retained_for_hook).unwrap();
            fs::create_dir(&configured_for_hook).unwrap();
        });

        let outcome = import_detailed(&configured, &source).unwrap();
        assert!(retained.join(&outcome.entry.id).is_dir());
        assert_eq!(fs::read_dir(&configured).unwrap().count(), 0);
        assert!(!configured.join(&outcome.entry.id).exists());
    }

    #[cfg(unix)]
    #[test]
    fn unix_root_swap_failure_cleans_staging_through_the_retained_inode() {
        let tmp = tempfile::tempdir().unwrap();
        let configured = tmp.path().join("library");
        let retained = tmp.path().join("retained-library");
        let source = tmp.path().join("root-swap-failure_P.pak");
        fs::write(&source, b"root swap failure bytes").unwrap();
        let configured_for_hook = configured.clone();
        let retained_for_hook = retained.clone();
        inject_library_root_swap(move |_locked_path| {
            fs::rename(&configured_for_hook, &retained_for_hook).unwrap();
            fs::create_dir(&configured_for_hook).unwrap();
        });
        inject_prepublish_failure(ModError::Other(
            "injected failure after retained-root binding".into(),
        ));

        let error = import_detailed(&configured, &source)
            .unwrap_err()
            .to_string();
        assert!(error.contains("injected failure"), "{error}");
        assert_eq!(fs::read_dir(&configured).unwrap().count(), 0);
        assert!(
            fs::read_dir(&retained)
                .unwrap()
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().starts_with(".staging-")),
            "retained library inode kept failed staging residue"
        );
    }

    #[test]
    fn equal_content_imports_from_two_processes_publish_one_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("library");
        let source_a_dir = tmp.path().join("a");
        let source_b_dir = tmp.path().join("b");
        fs::create_dir(&source_a_dir).unwrap();
        fs::create_dir(&source_b_dir).unwrap();
        let source_a = source_a_dir.join("same_P.pak");
        let source_b = source_b_dir.join("same_P.pak");
        fs::write(&source_a, b"equal child-process bytes").unwrap();
        fs::write(&source_b, b"equal child-process bytes").unwrap();
        let marker_a = tmp.path().join("import-locked-a");
        let marker_b = tmp.path().join("import-locked-b");
        let result_a = tmp.path().join("result-a");
        let result_b = tmp.path().join("result-b");

        let first = spawn_library_child(
            &library,
            "import",
            &marker_a,
            1_500,
            Some(&source_a),
            Some(&result_a),
        );
        wait_for_test_path(&marker_a);
        let mut second = spawn_library_child(
            &library,
            "import",
            &marker_b,
            0,
            Some(&source_b),
            Some(&result_b),
        );
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(
            !marker_b.exists(),
            "second import bypassed the held kernel lock"
        );
        assert!(
            second.try_wait().unwrap().is_none(),
            "second import did not block"
        );

        assert_child_success(first);
        assert_child_success(second);
        assert_eq!(
            fs::read_to_string(&result_a).unwrap(),
            fs::read_to_string(&result_b).unwrap()
        );
        assert_eq!(list(&library).unwrap().len(), 1);
        assert_no_import_residue(&library);
    }

    #[test]
    fn normalized_tree_hash_enforces_depth_entry_and_byte_ceilings() {
        let tmp = tempfile::tempdir().unwrap();

        let entries = tmp.path().join("entries");
        fs::create_dir(&entries).unwrap();
        fs::write(entries.join("a.bin"), b"a").unwrap();
        fs::write(entries.join("b.bin"), b"b").unwrap();
        let error = hash_import_tree(
            &entries,
            ImportLimits {
                max_zip_entries: 1,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("entry count limit"), "{error}");

        let bytes = tmp.path().join("bytes");
        fs::create_dir(&bytes).unwrap();
        fs::write(bytes.join("large.bin"), b"ab").unwrap();
        let error = hash_import_tree(
            &bytes,
            ImportLimits {
                max_zip_entry_uncompressed_bytes: 1,
                max_zip_total_uncompressed_bytes: 1,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("file bytes limit"), "{error}");

        let depth = tmp.path().join("depth");
        fs::create_dir_all(depth.join("nested")).unwrap();
        fs::write(depth.join("nested").join("file.bin"), b"x").unwrap();
        let error = hash_import_tree(
            &depth,
            ImportLimits {
                max_directory_depth: 0,
                ..DEFAULT_IMPORT_LIMITS
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("nesting depth limit"), "{error}");
    }

    #[test]
    fn normalized_tree_hash_orders_globally_and_reserves_every_root_sidecar_spelling() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("tree");
        fs::create_dir_all(tree.join("a")).unwrap();
        fs::write(tree.join("a.txt"), b"top").unwrap();
        fs::write(tree.join("a").join("z.bin"), b"nested").unwrap();

        let mut expected = Sha256::new();
        expected.update(IMPORT_TREE_HASH_DOMAIN);
        hash_tree_record(&mut expected, b'd', "a", None);
        hash_tree_record(&mut expected, b'f', "a.txt", Some(3));
        expected.update(b"top");
        hash_tree_record(&mut expected, b'f', "a/z.bin", Some(6));
        expected.update(b"nested");
        let expected = digest_hex(expected.finalize().into());

        assert_eq!(
            hash_import_tree(&tree, DEFAULT_IMPORT_LIMITS).unwrap(),
            expected
        );
        fs::write(tree.join(META_FILE), b"private metadata is not payload").unwrap();
        let error = hash_import_tree(&tree, DEFAULT_IMPORT_LIMITS)
            .unwrap_err()
            .to_string();
        assert!(error.contains("reserved manager-sidecar"), "{error}");
        let secured = open_directory_nofollow(&tree, "test manager-library entry").unwrap();
        assert_eq!(
            hash_secure_import_tree(&secured, DEFAULT_IMPORT_LIMITS, true).unwrap(),
            expected
        );

        fs::remove_file(tree.join(META_FILE)).unwrap();
        fs::write(
            tree.join("GORE-MANAGER-META.JSON"),
            b"case-variant metadata",
        )
        .unwrap();
        for allow_exact in [false, true] {
            let secured = open_directory_nofollow(&tree, "test reserved-name tree").unwrap();
            let error = hash_secure_import_tree(&secured, DEFAULT_IMPORT_LIMITS, allow_exact)
                .unwrap_err()
                .to_string();
            assert!(error.contains("reserved manager-sidecar"), "{error}");
        }
    }

    #[test]
    fn external_import_rejects_exact_and_case_variant_manager_sidecar_names() {
        for reserved in [META_FILE, "GORE-MANAGER-META.JSON"] {
            let tmp = tempfile::tempdir().unwrap();
            let library = tmp.path().join("library");
            let source = tmp.path().join("source");
            fs::create_dir(&source).unwrap();
            fs::write(source.join("opaque_P.pak"), b"opaque bytes").unwrap();
            fs::write(source.join(reserved), b"caller-controlled sidecar").unwrap();

            let error = import_detailed(&library, &source).unwrap_err().to_string();
            assert!(error.contains("reserved manager-sidecar"), "{error}");
            assert!(list(&library).unwrap().is_empty());
            assert_no_import_residue(&library);
        }
    }

    #[test]
    fn zip_preflight_uses_windows_uppercase_identity_for_final_sigma() {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("library");
        let archive = tmp.path().join("sigma.zip");
        zip_entries(
            &archive,
            &[
                ("Σ_P.pak", b"first", zip::CompressionMethod::Stored),
                ("ς_P.pak", b"second", zip::CompressionMethod::Stored),
            ],
        );

        let error = import_detailed(&library, &archive).unwrap_err().to_string();
        assert!(error.contains("portable extraction path"), "{error}");
        assert!(list(&library).unwrap().is_empty());
        assert_no_import_residue(&library);
    }

    #[cfg(unix)]
    #[test]
    fn folder_tree_rejects_final_sigma_collision_and_long_s_sidecar_alias() {
        let tmp = tempfile::tempdir().unwrap();
        let sigma_library = tmp.path().join("sigma-library");
        let sigma_source = tmp.path().join("sigma-source");
        fs::create_dir(&sigma_source).unwrap();
        fs::write(sigma_source.join("Σ_P.pak"), b"first").unwrap();
        fs::write(sigma_source.join("ς_P.pak"), b"second").unwrap();
        import_detailed(&sigma_library, &sigma_source).unwrap_err();
        assert_no_import_residue(&sigma_library);

        let alias_library = tmp.path().join("alias-library");
        let alias_source = tmp.path().join("alias-source");
        fs::create_dir(&alias_source).unwrap();
        fs::write(alias_source.join("opaque_P.pak"), b"opaque").unwrap();
        fs::write(
            alias_source.join("gore-manager-meta.jſon"),
            b"caller-controlled alias",
        )
        .unwrap();
        let error = import_detailed(&alias_library, &alias_source)
            .unwrap_err()
            .to_string();
        assert!(error.contains("reserved manager-sidecar"), "{error}");
        assert_no_import_residue(&alias_library);
    }

    #[cfg(windows)]
    #[test]
    fn source_identity_normalizes_windows_case_separators_and_verbatim_prefix() {
        assert_eq!(
            hash_normalized_source_path(Path::new(r"\\?\C:\Mods\Example.ZIP")),
            hash_normalized_source_path(Path::new("c:/mods/example.zip"))
        );
        assert_eq!(
            hash_normalized_source_path(Path::new(r"\\?\UNC\Server\Share\Mod")),
            hash_normalized_source_path(Path::new(r"\\server\share\mod"))
        );
    }

    #[cfg(any(unix, windows))]
    #[cfg_attr(
        windows,
        ignore = "requires Windows symbolic-link privilege; run explicitly on a privileged worker"
    )]
    #[test]
    fn normalized_tree_hash_rejects_links_without_following_them() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("tree");
        fs::create_dir(&tree).unwrap();
        let outside = tmp.path().join("outside.bin");
        fs::write(&outside, b"outside").unwrap();
        assert!(make_file_link(&outside, &tree.join("linked.bin")));
        let error = hash_import_tree(&tree, DEFAULT_IMPORT_LIMITS)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("symbolic link")
                || error.contains("reparse point")
                || error.contains("without following"),
            "{error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn normalized_tree_hash_rejects_special_files_without_blocking() {
        use std::os::unix::ffi::OsStrExt as _;

        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("tree");
        fs::create_dir(&tree).unwrap();
        let fifo = tree.join("payload.pipe");
        let fifo_name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo_name.as_ptr(), 0o600) }, 0);
        let error = hash_import_tree(&tree, DEFAULT_IMPORT_LIMITS)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("neither a regular file nor directory"),
            "{error}"
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

        let transaction = ReplacementTransaction::begin(&lib, &meta.id, None).unwrap();
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
    fn startup_recovery_never_guesses_through_a_quarantined_seal_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("old_P.pak");
        fs::write(&source, b"old payload").unwrap();
        let meta = import(&lib, &source).unwrap();
        let entry = lib.join(&meta.id);
        let transaction = ReplacementTransaction::begin(&lib, &meta.id, None).unwrap();
        fs::rename(&entry, transaction.backup()).unwrap();
        transaction.mark(ReplacementPhase::Quarantined).unwrap();

        let error = list(&lib).unwrap_err().to_string();
        assert!(error.contains("quarantined"), "{error}");
        assert!(!entry.exists());
        assert!(transaction.backup().is_dir());
        assert!(transaction.root.is_dir());
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

        let transaction = ReplacementTransaction::begin(&lib, &old_meta.id, None).unwrap();
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
    fn format_2_recovery_reconstructs_promotion_after_partial_backup_or_marker_cleanup() {
        for partial_backup in [true, false] {
            let tmp = tempfile::tempdir().unwrap();
            let lib = tmp.path().join("lib");
            let (_previous, promoted, entry, transaction) = prepare_format_2_promoted_update(
                tmp.path(),
                &lib,
                if partial_backup {
                    "partial"
                } else {
                    "markerless"
                },
            );
            transaction.mark(ReplacementPhase::Promoted).unwrap();
            if partial_backup {
                let payload = fs::read_dir(transaction.backup())
                    .unwrap()
                    .map(Result::unwrap)
                    .find(|entry| entry.file_name() != META_FILE)
                    .unwrap();
                fs::remove_file(payload.path()).unwrap();
            } else {
                fs::remove_dir_all(transaction.backup()).unwrap();
                for phase in [ReplacementPhase::PreviousMoved, ReplacementPhase::Promoted] {
                    fs::remove_file(transaction.root.join(phase.marker().unwrap())).unwrap();
                }
            }

            assert_eq!(list(&lib).unwrap(), vec![promoted]);
            assert_eq!(
                fs::read(entry.join(if partial_backup {
                    "partial_P.pak"
                } else {
                    "markerless_P.pak"
                }))
                .unwrap(),
                b"verified promoted payload"
            );
            assert!(!transaction.root.exists());
        }
    }

    #[test]
    fn format_2_recovery_accepts_staged_live_after_previous_sidecar_cleanup() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let (_previous, promoted, entry, transaction) =
            prepare_format_2_promoted_update(tmp.path(), &lib, "sidecar-cleanup");
        transaction.mark(ReplacementPhase::Promoted).unwrap();
        fs::remove_file(transaction.backup().join(META_FILE)).unwrap();
        let live_sidecar = fs::read(entry.join(META_FILE)).unwrap();
        let live_payload = fs::read(entry.join("sidecar-cleanup_P.pak")).unwrap();

        assert_eq!(list(&lib).unwrap(), vec![promoted]);
        assert_eq!(fs::read(entry.join(META_FILE)).unwrap(), live_sidecar);
        assert_eq!(
            fs::read(entry.join("sidecar-cleanup_P.pak")).unwrap(),
            live_payload
        );
        assert!(!transaction.backup().exists());
        assert!(!transaction.root.exists());
    }

    #[test]
    fn transient_recovery_seal_error_never_becomes_a_persistent_quarantine() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let (_previous, promoted, entry, transaction) =
            prepare_format_2_promoted_update(tmp.path(), &lib, "transient");
        inject_recovery_seal_failure();

        let error = list(&lib).unwrap_err().to_string();
        assert!(error.contains("transient recovery seal"), "{error}");
        assert!(entry.is_dir());
        assert!(transaction.backup().is_dir());
        assert!(!transaction.quarantine().exists());
        assert!(!transaction
            .root
            .join(ReplacementPhase::Quarantined.marker().unwrap())
            .exists());

        assert_eq!(list(&lib).unwrap(), vec![promoted]);
        assert!(!transaction.root.exists());
    }

    #[test]
    fn apply_quarantines_a_false_live_recovery_object_before_deploying_any_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let (previous, _promoted, entry, transaction) =
            prepare_format_2_promoted_update(tmp.path(), &lib, "apply-recovery");
        fs::write(
            entry.join("apply-recovery_P.pak"),
            b"unverified false live bytes",
        )
        .unwrap();
        let game = tmp.path().join("game");
        let mods = game.join("G1R/Content/Paks/~mods");
        fs::create_dir_all(&mods).unwrap();
        let loadout = crate::mgr::Loadout {
            format: 1,
            entries: vec![crate::mgr::LoadoutEntry {
                id: previous.id.clone(),
                enabled: true,
            }],
        };

        let error = crate::mgr::apply::apply_loadout(&game, &lib, &loadout)
            .unwrap_err()
            .to_string();
        assert!(error.contains("ambiguous replacement recovery"), "{error}");
        assert_eq!(
            fs::read(entry.join("apply-recovery_P.pak")).unwrap(),
            b"verified previous payload"
        );
        assert_eq!(
            fs::read(transaction.quarantine().join("apply-recovery_P.pak")).unwrap(),
            b"unverified false live bytes"
        );
        assert!(fs::read_dir(&mods).unwrap().next().is_none());
        assert!(!crate::record_path(&game).exists());
    }

    #[test]
    fn malformed_format_2_state_cannot_downgrade_to_presence_based_recovery() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("state_P.pak");
        fs::write(&source, b"state payload").unwrap();
        let meta = import(&lib, &source).unwrap();
        let before = visible_library_snapshot(&lib);
        let transaction = ReplacementTransaction::begin(&lib, &meta.id, None).unwrap();
        assert_eq!(transaction.state.format, 1);
        let malformed = ReplacementState {
            format: 2,
            id: meta.id.clone(),
            phase: ReplacementPhase::Promoted,
            expected_previous: None,
            expected_staged: None,
            verification_pending: false,
        };
        fs::write(
            transaction.root.join(REPLACEMENT_STATE_FILE),
            serde_json::to_vec(&malformed).unwrap(),
        )
        .unwrap();

        let error = list(&lib).unwrap_err().to_string();
        assert!(error.contains("invalid replacement state"), "{error}");
        assert_eq!(visible_library_snapshot(&lib), before);
        assert!(transaction.root.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn unix_recovery_discards_only_an_unpublished_partial_state_temp() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        fs::create_dir(&lib).unwrap();
        let transaction = lib.join(format!("{REPLACEMENT_PREFIX}partial-state"));
        fs::create_dir(&transaction).unwrap();
        let pending = replacement_atomic_temp_name(REPLACEMENT_STATE_FILE);
        fs::write(transaction.join(&pending), b"{\"format\":2").unwrap();

        assert!(list(&lib).unwrap().is_empty());
        assert!(!transaction.exists());
    }

    #[test]
    fn format_2_recovery_recognizes_a_verified_previous_entry_before_or_after_restore_marker() {
        for restored_marker in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            let lib = tmp.path().join("lib");
            let source = tmp.path().join("old_P.pak");
            fs::write(&source, b"verified previous payload").unwrap();
            let meta = import(&lib, &source).unwrap();
            let entry = lib.join(&meta.id);
            let previous = seal_import_path(&entry, DEFAULT_IMPORT_LIMITS).unwrap();
            let mut staged = previous.clone();
            staged.tree_sha256 = "0".repeat(64);
            let expectation = PublishExpectation {
                staged,
                current: Some(previous),
                limits: DEFAULT_IMPORT_LIMITS,
            };
            let transaction =
                ReplacementTransaction::begin(&lib, &meta.id, Some(&expectation)).unwrap();
            if restored_marker {
                transaction.mark(ReplacementPhase::Restored).unwrap();
            }

            // This is either a crash immediately after the verification-pending journal became
            // durable, or immediately after rollback restored the old object. In both cases the
            // live seal proves that no promotion occurred and recovery may remove only the journal.
            assert_eq!(list(&lib).unwrap(), vec![meta.clone()]);
            assert_eq!(
                fs::read(entry.join("old_P.pak")).unwrap(),
                b"verified previous payload"
            );
            assert!(!transaction.root.exists());
        }
    }

    #[test]
    fn format_2_recovery_cleans_an_empty_pre_rename_create_journal() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        fs::create_dir(&lib).unwrap();
        let expectation = PublishExpectation {
            staged: EntryPublishSeal {
                root_identity: open_directory_nofollow(&lib, "test library")
                    .unwrap()
                    .identity(),
                tree_sha256: "1".repeat(64),
                sidecar_sha256: "2".repeat(64),
            },
            current: None,
            limits: DEFAULT_IMPORT_LIMITS,
        };
        let transaction =
            ReplacementTransaction::begin(&lib, "new-entry", Some(&expectation)).unwrap();

        // A create writes this journal before the first rename. There is no payload object to
        // recover or quarantine, so startup may remove only the empty transaction metadata.
        assert!(list(&lib).unwrap().is_empty());
        assert!(!transaction.root.exists());
    }

    #[test]
    fn failed_previous_moved_marker_rolls_back_and_cleans_format_2_journal() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let source = tmp.path().join("rollback_P.pak");
        fs::write(&source, b"verified previous payload").unwrap();
        let first = import_detailed(&lib, &source).unwrap();
        let before = visible_library_snapshot(&lib);
        fs::write(&source, b"new staged payload").unwrap();
        inject_replacement_mark_failure(ReplacementPhase::PreviousMoved);

        let error = import_detailed(&lib, &source).unwrap_err().to_string();
        assert!(error.contains("replacement-marker failure"), "{error}");
        assert_eq!(visible_library_snapshot(&lib), before);
        assert_eq!(
            fs::read(lib.join(&first.entry.id).join("rollback_P.pak")).unwrap(),
            b"verified previous payload"
        );
        assert_no_import_residue(&lib);
    }

    #[test]
    fn replacement_names_are_unique_and_never_clear_an_existing_transaction() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        fs::create_dir(&lib).unwrap();
        let first = ReplacementTransaction::begin(&lib, "entry-a", None).unwrap();
        let second = ReplacementTransaction::begin(&lib, "entry-a", None).unwrap();
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
        recover_interrupted_replacements_for_test(&lib).unwrap();
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

        recover_interrupted_replacements_for_test(&lib).unwrap();
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

    /// The same normalized bundle tree moved from a folder to ZIP keeps its entry identity.
    #[test]
    fn moved_folder_to_zip_rebinds_same_content_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let bdir = mk_goremod_bundle(tmp.path()); // manifest name "Target Probe"

        let from_dir = import_detailed(&lib, &bdir).unwrap();
        let zp = tmp.path().join("other.zip");
        zip_dir_with_prefix(&bdir, "", &zp);
        let from_zip = import_detailed(&lib, &zp).unwrap();

        assert_eq!(
            from_dir.entry.name, from_zip.entry.name,
            "precondition: same display name"
        );
        assert_ne!(
            from_dir.entry.source, from_zip.entry.source,
            "precondition: different source"
        );
        assert_eq!(
            from_dir.entry.id, from_zip.entry.id,
            "the same normalized tree moved from a folder to ZIP keeps its id"
        );
        assert_eq!(from_zip.disposition, ImportDisposition::Updated);
        assert_eq!(from_zip.matched_by, ImportMatchedBy::Content);
        assert_eq!(
            from_dir.entry.fingerprint(),
            from_zip.entry.fingerprint(),
            "pure source rebind preserves deployment identity"
        );
        assert_eq!(list(&lib).unwrap().len(), 1);
    }

    /// Same display/basename but different normalized bytes remain separate entries.
    #[test]
    fn same_filename_different_dir_coexist() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let a = tmp.path().join("a").join("mod");
        let b = tmp.path().join("b").join("mod");
        for (d, bytes) in [(&a, b"alpha".as_slice()), (&b, b"bravo".as_slice())] {
            fs::create_dir_all(d).unwrap();
            fs::write(d.join("bar.utoc"), bytes).unwrap();
            fs::write(d.join("bar.ucas"), bytes).unwrap();
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
        // Same second, different microseconds → distinct timestamps, so a changed-tree re-import
        // can still get a fingerprint-distinguishing `imported_at`.
        assert_ne!(
            format_utc(1_767_225_600, 100),
            format_utc(1_767_225_600, 200)
        );
    }
}
