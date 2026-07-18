//! Inspection and native materialization of untrusted managed revision-3 snapshot archives.
//!
//! Inspection is read-only and authority-free. Native Windows materialization retains and consumes
//! that exact verified source handle, constructs a complete private Store, publishes its fixed head
//! last, and atomically promotes the whole directory into an absent destination. Neither operation
//! adopts a project session or makes any game-runtime readiness claim.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use zip::{CompressionMethod, DateTime, ZipArchive};

use super::revision3_export::revision3_exact_snapshot_v2_full_reopen_work;
use super::*;
use crate::{
    MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1,
    MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1, MAX_PROJECT_JSON_BYTES,
};

pub const REVISION3_EXACT_SNAPSHOT_IMPORT_FORMAT_V2: &str = "managed_revision3_exact_snapshot_v2";
pub const REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_MARKER_V2: &str =
    "gore.managed-project-snapshot.v2";
pub const REVISION3_EXACT_SNAPSHOT_IMPORT_ARTIFACT_KIND_V2: &str =
    "portable_snapshot_restorable_copy";
pub const REVISION3_EXACT_SNAPSHOT_IMPORT_RESTORE_STATUS_V2: &str = "supported";
pub const REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2: &str = "gore-export.json";

const PROJECT_FILE: &str = "project.json";
const STORE_HEAD_MEMBER: &str = "store/gore-project.json";
const MAX_IMPORT_MANIFEST_BYTES_V2: usize = 128 * 1024 * 1024;
const MAX_IMPORT_ARCHIVE_ENTRIES_V2: u64 = 300_003;
// The closed format can contain at most 64 GiB of assets, 512 MiB each of snapshot and entity
// objects, the bounded manifest/project/head, and deterministic ZIP metadata. This early hard cap
// rejects an absurd source before the central directory is parsed; tighter caller limits and the
// exact member sums are enforced immediately after the first manifest is read.
const MAX_IMPORT_ARCHIVE_BYTES_V2: u64 = 70 * 1024 * 1024 * 1024;
const ZIP_FILE_MODE: u32 = 0o644;
const ZIP_VERSION_45: u16 = 45;
const ZIP_UNIX_VERSION_45: u16 = (3 << 8) | ZIP_VERSION_45;
const ZIP_DOS_EPOCH_TIME: u16 = 0;
const ZIP_DOS_EPOCH_DATE: u16 = 33;
const ZIP_EXTERNAL_FILE_ATTRIBUTES: u32 = (0o100000 | ZIP_FILE_MODE) << 16;
const MAX_IMPORT_PATH_UTF8_BYTES_V2: usize = 32 * 1024;

static IMPORT_STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Bounded closure proved by a successful read-only V2 inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ExactSnapshotInspectionClosureV2 {
    pub snapshot_objects: u64,
    pub entity_objects: u64,
    pub asset_objects: u64,
    pub archive_entries: u64,
    pub uncompressed_bytes: u64,
}

/// Path-independent proof returned after the complete archive and Store closure reopen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ExactSnapshotInspectionV2 {
    pub head: WorkingHead,
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub archive: ContentSeal,
    pub manifest: ContentSeal,
    pub closure: Revision3ExactSnapshotInspectionClosureV2,
}

/// Path-independent receipt returned only after the destination directory is atomically visible,
/// identity-pinned, and fully reopened as the exact inspected project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ExactSnapshotImportV2 {
    pub head: WorkingHead,
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub archive: ContentSeal,
    pub manifest: ContentSeal,
    pub closure: Revision3ExactSnapshotInspectionClosureV2,
}

/// Stable warning class for confirmed publication and the non-adoptable uncertain terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3ExactSnapshotImportWarningV2 {
    CleanupIncomplete,
    PublicationUncertain,
}

/// Destination publication terminal. Publication uncertainty intentionally carries no receipt:
/// callers must not adopt the requested path without a separate recovery operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3ExactSnapshotImportPublicationV2 {
    Imported(Revision3ExactSnapshotImportV2),
    ImportedWithCleanupWarning(Revision3ExactSnapshotImportV2),
    PublicationUncertain,
}

impl Revision3ExactSnapshotImportPublicationV2 {
    pub const fn receipt(&self) -> Option<&Revision3ExactSnapshotImportV2> {
        match self {
            Self::Imported(receipt) | Self::ImportedWithCleanupWarning(receipt) => Some(receipt),
            Self::PublicationUncertain => None,
        }
    }

    pub const fn warning(&self) -> Option<Revision3ExactSnapshotImportWarningV2> {
        match self {
            Self::Imported(_) => None,
            Self::ImportedWithCleanupWarning(_) => {
                Some(Revision3ExactSnapshotImportWarningV2::CleanupIncomplete)
            }
            Self::PublicationUncertain => {
                Some(Revision3ExactSnapshotImportWarningV2::PublicationUncertain)
            }
        }
    }

    /// Import publication is never safe to retry automatically, including confirmed success.
    pub const fn retry_safe(&self) -> bool {
        false
    }
}

/// Failures returned only while native code can still prove the final destination was not
/// published. Once that boundary may have been crossed, the API returns `PublicationUncertain`.
#[derive(Debug, thiserror::Error)]
pub enum Revision3ExactSnapshotImportErrorV2 {
    #[error(transparent)]
    Inspection(#[from] Revision3ExactSnapshotInspectionErrorV2),
    #[error("managed snapshot archive CAS differs from the required archive")]
    ArchiveCasMismatch {
        expected: ContentSeal,
        actual: ContentSeal,
    },
    #[error("invalid managed import destination: {0}")]
    InvalidDestination(String),
    #[error("managed import destination already exists")]
    DestinationAlreadyExists,
    #[error("managed snapshot destination materialization failed: {0}")]
    Materialization(String),
    #[error("managed snapshot staged Store verification failed: {0}")]
    CandidateVerification(String),
    #[error("managed snapshot destination publication failed: {0}")]
    Publication(String),
    #[error("{primary}; bounded private staging cleanup also failed: {cleanup}")]
    StagingCleanup {
        primary: Box<Revision3ExactSnapshotImportErrorV2>,
        cleanup: String,
    },
}

/// Stable failure vocabulary for read-only inspection of an untrusted source archive.
#[derive(Debug, thiserror::Error)]
pub enum Revision3ExactSnapshotInspectionErrorV2 {
    #[error("restorable snapshot inspection is not supported safely on this platform")]
    UnsupportedPlatform,
    #[error("invalid restorable snapshot source: {0}")]
    InvalidSource(String),
    #[error("restorable snapshot source limit exceeded for {kind}: {actual} > {limit}")]
    Limit {
        kind: &'static str,
        actual: u64,
        limit: u64,
    },
    #[error("invalid restorable snapshot ZIP: {0}")]
    InvalidArchive(String),
    #[error("invalid restorable snapshot manifest: {0}")]
    InvalidManifest(String),
    #[error("invalid restorable snapshot Store closure: {0}")]
    InvalidClosure(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum ImportManifestFormatV2 {
    #[serde(rename = "gore.managed-project-snapshot.v2")]
    ExactSnapshotV2,
}

impl<'de> Deserialize<'de> for ImportManifestFormatV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value == REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_MARKER_V2 {
            Ok(Self::ExactSnapshotV2)
        } else {
            Err(serde::de::Error::custom(format!(
                "unsupported managed project snapshot format {value:?}; expected {:?}",
                REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_MARKER_V2
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImportManifestSchemaV2;

impl<'de> Deserialize<'de> for ImportManifestSchemaV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == 2 {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "unsupported managed project snapshot schema {value}; expected 2"
            )))
        }
    }
}

impl Serialize for ImportManifestSchemaV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ImportArtifactKindV2 {
    #[serde(rename = "portable_snapshot_restorable_copy")]
    PortableSnapshotRestorableCopy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ImportRestoreStatusV2 {
    #[serde(rename = "supported")]
    Supported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportBasisV2 {
    head: WorkingHead,
    project_id: ProjectId,
    project_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportMemberSealV2 {
    relative_name: String,
    byte_len: u64,
    sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactSnapshotManifestV2 {
    format: ImportManifestFormatV2,
    schema: ImportManifestSchemaV2,
    artifact_kind: ImportArtifactKindV2,
    restore_status: ImportRestoreStatusV2,
    basis: ImportBasisV2,
    members: Vec<ImportMemberSealV2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportMemberKind {
    Project,
    StoreHead,
    Snapshot(Sha256Digest),
    Entity(EntityId, Sha256Digest),
    Asset(Sha256Digest),
}

#[derive(Debug, Clone)]
struct InspectedMember {
    index: usize,
    relative_name: String,
    seal: ContentSeal,
    kind: ImportMemberKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawZipMemberExpectation {
    name: Vec<u8>,
    byte_len: u64,
    crc32: u32,
}

#[derive(Debug, Clone)]
struct SnapshotNeed {
    seal: ContentSeal,
    expected_tuple: Option<SnapshotTuple>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotTuple {
    project_id: ProjectId,
    project_revision: u64,
    target: GameGenerationAnchor,
}

#[derive(Debug, Clone, Copy)]
struct FullReopenWorkBudget {
    objects: u64,
    bytes: u64,
    max_objects: u64,
    max_bytes: u64,
}

impl FullReopenWorkBudget {
    fn closed_v2() -> Self {
        Self {
            objects: 0,
            bytes: 0,
            max_objects: MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1,
            max_bytes: MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1,
        }
    }

    #[cfg(test)]
    fn with_limits(max_objects: u64, max_bytes: u64) -> Self {
        Self {
            objects: 0,
            bytes: 0,
            max_objects,
            max_bytes,
        }
    }

    fn charge_snapshot(
        &mut self,
        snapshot_seal: &ContentSeal,
        snapshot: &Revision3SnapshotManifest,
    ) -> Result<(), Revision3ExactSnapshotInspectionErrorV2> {
        let work = revision3_exact_snapshot_v2_full_reopen_work(snapshot_seal, snapshot).ok_or(
            Revision3ExactSnapshotInspectionErrorV2::Limit {
                kind: "full-reopen bytes",
                actual: u64::MAX,
                limit: self.max_bytes,
            },
        )?;
        let objects = checked_import_sum(
            "full-reopen objects",
            self.objects,
            work.objects,
            self.max_objects,
        )?;
        let bytes =
            checked_import_sum("full-reopen bytes", self.bytes, work.bytes, self.max_bytes)?;
        self.objects = objects;
        self.bytes = bytes;
        Ok(())
    }
}

struct ManifestPlan {
    members_by_name: BTreeMap<String, InspectedMember>,
    archive_order: Vec<String>,
    closure: Revision3ExactSnapshotInspectionClosureV2,
}

/// Private capability retaining the exact source handle used for every verification pass. A
/// public inspection is only a projection of this value; destination materialization consumes it
/// directly so a path can never be reopened after the proof boundary.
struct VerifiedRevision3ExactSnapshotArchiveV2 {
    source: File,
    plan: ManifestPlan,
    head: WorkingHead,
    project: ProjectRevision3,
    archive: ContentSeal,
    manifest: ContentSeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportGuardPhase {
    AfterStagingCreate,
    AfterStoreMembers,
    BeforeHeadPublication,
    AfterHeadPublication,
    BeforePromotion,
    BeforePublicationSyscall,
    AfterPromotion,
    BeforeCleanupAccounting,
    BeforeReceiptLinearization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportCreateGuardPhase {
    StagingRoot,
    StoreDirectory,
    StoreFile,
}

#[cfg(test)]
thread_local! {
    static IMPORT_CREATE_FAILPOINT: std::cell::Cell<Option<ImportCreateGuardPhase>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(all(test, windows))]
thread_local! {
    static IMPORT_PUBLISH_POST_RENAME_FAILURE: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

fn injected_import_create_failure(
    phase: ImportCreateGuardPhase,
) -> Option<Revision3ExactSnapshotImportErrorV2> {
    #[cfg(test)]
    {
        return IMPORT_CREATE_FAILPOINT.with(|failpoint| {
            if failpoint.get() == Some(phase) {
                failpoint.set(None);
                Some(Revision3ExactSnapshotImportErrorV2::Materialization(
                    format!("injected post-create failure at {phase:?}"),
                ))
            } else {
                None
            }
        });
    }
    #[cfg(not(test))]
    {
        let _ = phase;
        None
    }
}

#[cfg(windows)]
struct ImportDestinationGuard {
    target: PathBuf,
    parent_path: PathBuf,
    filename: OsString,
    parent: cap_std::fs::Dir,
    parent_identity: WindowsFileIdentity,
}

#[cfg(windows)]
struct CreatedImportFile {
    relative: PathBuf,
    file: Option<File>,
    identity: WindowsFileIdentity,
    seal: ContentSeal,
}

#[cfg(windows)]
struct CreatedImportDirectory {
    file: Option<File>,
    directory: Option<cap_std::fs::Dir>,
    identity: WindowsFileIdentity,
}

#[cfg(windows)]
struct ImportStagingDirectory {
    name: OsString,
    path: PathBuf,
    file: Option<File>,
    directory: Option<cap_std::fs::Dir>,
    identity: WindowsFileIdentity,
    files: Vec<CreatedImportFile>,
    file_indices: BTreeMap<PathBuf, usize>,
    directories: BTreeMap<PathBuf, CreatedImportDirectory>,
    descendant_handles_released: bool,
    root_handles_final_frozen: bool,
    promoted: bool,
}

#[cfg(windows)]
struct PublishedImportChangeWatch {
    file: File,
    buffer: Box<[u32; 16_384]>,
    overlapped: Box<windows_sys::Win32::System::IO::OVERLAPPED>,
    _event: std::os::windows::io::OwnedHandle,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
enum ExpectedImportEntry {
    File(usize),
    Directory(WindowsFileIdentity),
}

impl VerifiedRevision3ExactSnapshotArchiveV2 {
    fn inspection(&self) -> Revision3ExactSnapshotInspectionV2 {
        Revision3ExactSnapshotInspectionV2 {
            head: self.head.clone(),
            project_id: self.project.project_id,
            project_revision: self.project.revision,
            archive: self.archive.clone(),
            manifest: self.manifest.clone(),
            closure: self.plan.closure.clone(),
        }
    }

    fn import_receipt(&self) -> Revision3ExactSnapshotImportV2 {
        Revision3ExactSnapshotImportV2 {
            head: self.head.clone(),
            project_id: self.project.project_id,
            project_revision: self.project.revision,
            archive: self.archive.clone(),
            manifest: self.manifest.clone(),
            closure: self.plan.closure.clone(),
        }
    }
}

/// Inspect a V2 restorable snapshot under the format's default hard limits.
///
/// On Windows the parent is identity-pinned and the final name is opened handle-relative without
/// following a reparse point; all payload reads use that write/delete-exclusive final handle.
/// Platforms without an enforceable immutable-source primitive fail closed. The inspector issues
/// no application-level filesystem create, write, delete, extraction, or publication operation.
pub fn inspect_revision3_exact_snapshot_v2(
    source: impl AsRef<Path>,
) -> Result<Revision3ExactSnapshotInspectionV2, Revision3ExactSnapshotInspectionErrorV2> {
    inspect_revision3_exact_snapshot_v2_with_limits(source, WorkingStoreLimits::default())
}

/// Inspect a V2 restorable snapshot under stricter managed-Store limits.
pub fn inspect_revision3_exact_snapshot_v2_with_limits(
    source: impl AsRef<Path>,
    limits: WorkingStoreLimits,
) -> Result<Revision3ExactSnapshotInspectionV2, Revision3ExactSnapshotInspectionErrorV2> {
    let limits = limits.validate().map_err(|error| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidSource(error.to_string())
    })?;
    verify_revision3_exact_snapshot_archive_v2(source.as_ref(), &limits)
        .map(|verified| verified.inspection())
}

/// Materialize one exact V2 archive into an absent managed destination directory.
///
/// The expected archive seal is a strict CAS token. The source is inspected and then streamed
/// through that same retained Windows handle. This function does not adopt a session, mutate a
/// game or save, build, deploy, or make any runtime-readiness claim.
///
/// A confirmed receipt linearizes the exact in-root managed tree, recorded identities and bytes at
/// the final pending recursive-change-watch check. Both final passes and the Full reopen require
/// single-link files, but the receipt does not claim a globally atomic absence of aliases created
/// outside the managed root; every later session open must revalidate that Store invariant.
pub fn import_revision3_exact_snapshot_v2(
    source: impl AsRef<Path>,
    expected_archive: &ContentSeal,
    destination: impl AsRef<Path>,
) -> Result<Revision3ExactSnapshotImportPublicationV2, Revision3ExactSnapshotImportErrorV2> {
    import_revision3_exact_snapshot_v2_with_limits(
        source,
        expected_archive,
        destination,
        WorkingStoreLimits::default(),
    )
}

/// Materialize V2 under stricter managed-Store limits without changing the closed archive policy.
pub fn import_revision3_exact_snapshot_v2_with_limits(
    source: impl AsRef<Path>,
    expected_archive: &ContentSeal,
    destination: impl AsRef<Path>,
    limits: WorkingStoreLimits,
) -> Result<Revision3ExactSnapshotImportPublicationV2, Revision3ExactSnapshotImportErrorV2> {
    import_revision3_exact_snapshot_v2_guarded(
        source.as_ref(),
        expected_archive,
        destination.as_ref(),
        limits,
        |_, _, _| Ok(()),
    )
}

fn import_revision3_exact_snapshot_v2_guarded<F>(
    source: &Path,
    expected_archive: &ContentSeal,
    destination: &Path,
    limits: WorkingStoreLimits,
    mut guard_hook: F,
) -> Result<Revision3ExactSnapshotImportPublicationV2, Revision3ExactSnapshotImportErrorV2>
where
    F: FnMut(ImportGuardPhase, &Path, &Path) -> Result<(), Revision3ExactSnapshotImportErrorV2>,
{
    let limits = limits.validate().map_err(|error| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidSource(error.to_string())
    })?;
    // On unsupported platforms this fails before the destination spelling is inspected and before
    // any destination filesystem operation can occur.
    let verified = verify_revision3_exact_snapshot_archive_v2(source, &limits)?;
    if &verified.archive != expected_archive {
        return Err(Revision3ExactSnapshotImportErrorV2::ArchiveCasMismatch {
            expected: expected_archive.clone(),
            actual: verified.archive.clone(),
        });
    }
    materialize_verified_revision3_exact_snapshot_v2(verified, destination, limits, &mut guard_hook)
}

fn verify_revision3_exact_snapshot_archive_v2(
    source: &Path,
    limits: &WorkingStoreLimits,
) -> Result<VerifiedRevision3ExactSnapshotArchiveV2, Revision3ExactSnapshotInspectionErrorV2> {
    let file = open_untrusted_source(source)?;
    let initial_len = file.metadata().map_err(source_io_error)?.len();
    if initial_len == 0 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "archive is empty".to_owned(),
        ));
    }
    if initial_len > MAX_IMPORT_ARCHIVE_BYTES_V2 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind: "archive bytes",
            actual: initial_len,
            limit: MAX_IMPORT_ARCHIVE_BYTES_V2,
        });
    }

    // A before/after whole-file seal brackets all structured reads. Each structured read is also
    // independently checked against its member seal, so a source that changes during inspection
    // cannot combine unauthenticated bytes into an accepted result.
    let archive_before = hash_open_file(&file, initial_len)?;
    let reader = file.try_clone().map_err(source_io_error)?;
    let mut archive = ZipArchive::new(reader).map_err(archive_error)?;
    if archive.is_empty() {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(
            "archive contains no entries".to_owned(),
        ));
    }
    if archive.len() as u64 > MAX_IMPORT_ARCHIVE_ENTRIES_V2 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind: "archive entry count",
            actual: archive.len() as u64,
            limit: MAX_IMPORT_ARCHIVE_ENTRIES_V2,
        });
    }
    if !archive.comment().is_empty() {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(
            "archive comment is not permitted".to_owned(),
        ));
    }

    let timestamp = fixed_zip_timestamp()?;
    let manifest_bytes = read_first_manifest(&mut archive, timestamp)?;
    let manifest_seal = seal_bytes(&manifest_bytes);
    let manifest: ExactSnapshotManifestV2 = parse_canonical_json(
        &manifest_bytes,
        "managed restorable exact snapshot import manifest",
    )
    .map_err(manifest_model_error)?;
    let plan = validate_manifest_plan(&manifest, &limits, manifest_seal.byte_len)?;
    if archive.len() != plan.archive_order.len() {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(
            "archive entry count differs from the sealed manifest".to_owned(),
        ));
    }

    let raw_members =
        verify_and_hash_archive_members(&mut archive, &plan, &manifest_seal, timestamp)?;
    drop(archive);
    verify_exact_zip_layout(&file, initial_len, &raw_members)?;

    let reader = file.try_clone().map_err(source_io_error)?;
    let mut archive = ZipArchive::new(reader).map_err(archive_error)?;
    let (head, project) = reopen_store_closure(&mut archive, &manifest, &plan, &limits)?;
    if project.project_id != manifest.basis.project_id
        || project.revision != manifest.basis.project_revision
    {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
            "reopened current project disagrees with the manifest basis tuple".to_owned(),
        ));
    }
    drop(archive);

    let final_len = file.metadata().map_err(source_io_error)?.len();
    if final_len != initial_len {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "archive length changed during inspection".to_owned(),
        ));
    }
    let archive_after = hash_open_file(&file, final_len)?;
    if archive_after != archive_before {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "archive bytes changed during inspection".to_owned(),
        ));
    }

    Ok(VerifiedRevision3ExactSnapshotArchiveV2 {
        source: file,
        plan,
        head,
        project,
        archive: ContentSeal {
            byte_len: final_len,
            sha256: archive_after,
        },
        manifest: manifest_seal,
    })
}

#[cfg(not(windows))]
fn materialize_verified_revision3_exact_snapshot_v2<F>(
    _verified: VerifiedRevision3ExactSnapshotArchiveV2,
    _destination: &Path,
    _limits: WorkingStoreLimits,
    _guard_hook: &mut F,
) -> Result<Revision3ExactSnapshotImportPublicationV2, Revision3ExactSnapshotImportErrorV2>
where
    F: FnMut(ImportGuardPhase, &Path, &Path) -> Result<(), Revision3ExactSnapshotImportErrorV2>,
{
    // Verification returns UnsupportedPlatform before this function can be reached. Keep this
    // branch independently fail-closed if platform dispatch is ever rearranged.
    Err(Revision3ExactSnapshotInspectionErrorV2::UnsupportedPlatform.into())
}

#[cfg(windows)]
fn materialize_verified_revision3_exact_snapshot_v2<F>(
    verified: VerifiedRevision3ExactSnapshotArchiveV2,
    destination: &Path,
    limits: WorkingStoreLimits,
    guard_hook: &mut F,
) -> Result<Revision3ExactSnapshotImportPublicationV2, Revision3ExactSnapshotImportErrorV2>
where
    F: FnMut(ImportGuardPhase, &Path, &Path) -> Result<(), Revision3ExactSnapshotImportErrorV2>,
{
    let destination_guard = prepare_import_destination(destination)?;
    let mut staging = create_import_staging_directory(&destination_guard)?;

    let staged_result = (|| {
        guard_hook(
            ImportGuardPhase::AfterStagingCreate,
            &staging.path,
            &destination_guard.target,
        )?;
        revalidate_import_staging(&destination_guard, &staging)?;

        let reader = verified
            .source
            .try_clone()
            .map_err(import_materialization_io)?;
        let mut archive = ZipArchive::new(reader).map_err(|error| {
            Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string())
        })?;
        for relative_name in &verified.plan.archive_order {
            let Some(member) = verified.plan.members_by_name.get(relative_name) else {
                continue;
            };
            let relative = match member.kind {
                ImportMemberKind::Snapshot(digest) => {
                    import_store_relative(&snapshot_member_name(digest))?
                }
                ImportMemberKind::Entity(id, digest) => {
                    import_store_relative(&entity_member_name(id, digest))?
                }
                ImportMemberKind::Asset(digest) => {
                    import_store_relative(&asset_member_name(digest))?
                }
                ImportMemberKind::Project | ImportMemberKind::StoreHead => continue,
            };
            materialize_archive_member(&mut archive, member, &relative, &mut staging)?;
        }
        drop(archive);

        guard_hook(
            ImportGuardPhase::AfterStoreMembers,
            &staging.path,
            &destination_guard.target,
        )?;
        revalidate_import_staging(&destination_guard, &staging)?;
        verify_import_staging_shape_and_seals(&staging, false)?;

        let head_bytes = canonical_json(&verified.head).map_err(|error| {
            Revision3ExactSnapshotImportErrorV2::CandidateVerification(error.to_string())
        })?;
        let head_member = required_member(&verified.plan, STORE_HEAD_MEMBER)?;
        if seal_bytes(&head_bytes) != head_member.seal {
            return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                "canonical head bytes differ from the authenticated archive head".to_owned(),
            ));
        }

        // The candidate can be fully opened from explicit canonical head bytes while the fixed
        // head remains absent. This proves all current-project Store members before publication.
        let staged_store =
            WorkingProjectStore::open_existing(&staging.path, limits).map_err(|error| {
                Revision3ExactSnapshotImportErrorV2::CandidateVerification(error.to_string())
            })?;
        let explicit = staged_store
            .open_revision3_head_bytes(&head_bytes, AssetVerification::Full)
            .map_err(|error| {
                Revision3ExactSnapshotImportErrorV2::CandidateVerification(error.to_string())
            })?;
        if explicit.head != verified.head || explicit.project != verified.project {
            return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                "staged Store did not reopen to the exact inspected project".to_owned(),
            ));
        }

        guard_hook(
            ImportGuardPhase::BeforeHeadPublication,
            &staging.path,
            &destination_guard.target,
        )?;
        revalidate_import_staging(&destination_guard, &staging)?;
        materialize_bytes_member(
            &head_bytes,
            &head_member.seal,
            Path::new(HEAD_FILE_NAME),
            &mut staging,
        )?;
        guard_hook(
            ImportGuardPhase::AfterHeadPublication,
            &staging.path,
            &destination_guard.target,
        )?;

        // The fixed head is the last staged file. Reopen through the ordinary current-head API,
        // then authenticate the exact planned tree once more before the only publish boundary.
        revalidate_import_staging(&destination_guard, &staging)?;
        verify_import_staging_shape_and_seals(&staging, true)?;
        let current = staged_store
            .open_current_revision3(AssetVerification::Full)
            .map_err(|error| {
                Revision3ExactSnapshotImportErrorV2::CandidateVerification(error.to_string())
            })?;
        if current.head != verified.head || current.project != verified.project {
            return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                "published staged head did not reopen to the exact inspected project".to_owned(),
            ));
        }
        revalidate_verified_source_for_import(&verified)?;
        Ok::<(), Revision3ExactSnapshotImportErrorV2>(())
    })();
    if let Err(primary) = staged_result {
        return Err(abort_import_staging(&mut staging, primary));
    }

    if let Err(primary) = guard_hook(
        ImportGuardPhase::BeforePromotion,
        &staging.path,
        &destination_guard.target,
    ) {
        return Err(abort_import_staging(&mut staging, primary));
    }
    if let Err(primary) = revalidate_import_destination_absent(&destination_guard) {
        return Err(abort_import_staging(&mut staging, primary));
    }
    if let Err(primary) = revalidate_import_staging(&destination_guard, &staging) {
        return Err(abort_import_staging(&mut staging, primary));
    }
    if let Err(primary) = verify_import_staging_shape_and_seals(&staging, true) {
        return Err(abort_import_staging(&mut staging, primary));
    }
    if let Err(primary) = release_import_descendant_handles(&mut staging) {
        return Err(abort_import_staging(&mut staging, primary));
    }
    if let Err(primary) = revalidate_import_destination_absent(&destination_guard) {
        return Err(abort_import_staging(&mut staging, primary));
    }
    if let Err(primary) = revalidate_import_staging(&destination_guard, &staging) {
        return Err(abort_import_staging(&mut staging, primary));
    }
    if let Err(primary) = verify_import_staging_shape_and_seals(&staging, true) {
        return Err(abort_import_staging(&mut staging, primary));
    }
    if let Err(primary) = guard_hook(
        ImportGuardPhase::BeforePublicationSyscall,
        &staging.path,
        &destination_guard.target,
    ) {
        return Err(abort_import_staging(&mut staging, primary));
    }

    match publish_import_staging_no_clobber(&destination_guard, &mut staging) {
        Err(ImportPublishError::AlreadyExists) => {
            let primary = Revision3ExactSnapshotImportErrorV2::DestinationAlreadyExists;
            return Err(abort_import_staging(&mut staging, primary));
        }
        Err(ImportPublishError::BeforeSyscall(error)) => {
            let primary = Revision3ExactSnapshotImportErrorV2::Publication(error);
            return Err(abort_import_staging(&mut staging, primary));
        }
        Err(ImportPublishError::PublicationUncertain) => {
            return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
        }
        Ok(()) => {}
    }

    // No ordinary error may escape after the atomic directory rename. Drop the writable root
    // capabilities, then immediately reacquire the final root and every descendant under
    // read-only sharing. Those freeze handles remain live until the receipt leaves this function.
    if freeze_published_import_root(&destination_guard, &mut staging).is_err() {
        return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
    }
    let mut change_watch =
        match PublishedImportChangeWatch::start(&destination_guard, staging.identity) {
            Ok(watch) => watch,
            Err(_) => {
                return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
            }
        };
    if reacquire_published_import_descendant_handles(&mut staging).is_err() {
        return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
    }
    if guard_hook(
        ImportGuardPhase::AfterPromotion,
        &staging.path,
        &destination_guard.target,
    )
    .is_err()
    {
        return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
    }
    if verify_published_import_destination(&destination_guard, &staging).is_err() {
        return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
    }
    if verify_import_staging_shape_and_seals(&staging, true).is_err() {
        return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
    }
    let final_store = match WorkingProjectStore::open_existing(&destination_guard.target, limits) {
        Ok(store) => store,
        Err(_) => {
            return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
        }
    };
    let final_current = match final_store.open_current_revision3(AssetVerification::Full) {
        Ok(opened) => opened,
        Err(_) => {
            return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
        }
    };
    if final_current.head != verified.head
        || final_current.project != verified.project
        || revalidate_verified_source_for_import(&verified).is_err()
    {
        return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
    }

    let cleanup_warning = guard_hook(
        ImportGuardPhase::BeforeCleanupAccounting,
        &staging.path,
        &destination_guard.target,
    )
    .is_err();
    // The hook is deliberately inside the final freeze interval. Re-run every exact proof after
    // it so even a privileged test or same-process race cannot turn a warning into a false receipt.
    let final_after_hook = WorkingProjectStore::open_existing(&destination_guard.target, limits)
        .and_then(|store| store.open_current_revision3(AssetVerification::Full));
    if verify_published_import_destination(&destination_guard, &staging).is_err()
        || verify_import_staging_shape_and_seals(&staging, true).is_err()
        || final_after_hook.is_err()
        || final_after_hook
            .as_ref()
            .is_ok_and(|opened| opened.head != verified.head || opened.project != verified.project)
        || revalidate_verified_source_for_import(&verified).is_err()
    {
        return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
    }
    if guard_hook(
        ImportGuardPhase::BeforeReceiptLinearization,
        &staging.path,
        &destination_guard.target,
    )
    .is_err()
        || change_watch.quiet_at_linearization().is_err()
    {
        return Ok(Revision3ExactSnapshotImportPublicationV2::PublicationUncertain);
    }
    let receipt = verified.import_receipt();
    if cleanup_warning {
        Ok(Revision3ExactSnapshotImportPublicationV2::ImportedWithCleanupWarning(receipt))
    } else {
        Ok(Revision3ExactSnapshotImportPublicationV2::Imported(receipt))
    }
}

#[cfg(windows)]
fn prepare_import_destination(
    destination: &Path,
) -> Result<ImportDestinationGuard, Revision3ExactSnapshotImportErrorV2> {
    use cap_std::ambient_authority;
    use cap_std::fs::Dir;

    if !destination.is_absolute() {
        return Err(Revision3ExactSnapshotImportErrorV2::InvalidDestination(
            "destination must be an absolute path".to_owned(),
        ));
    }
    validate_windows_import_destination_spelling(destination)?;
    let normalized = normalize_absolute(destination).map_err(|error| {
        Revision3ExactSnapshotImportErrorV2::InvalidDestination(error.to_string())
    })?;
    let filename = normalized
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::InvalidDestination(
                "destination must have one nonempty final directory name".to_owned(),
            )
        })?
        .to_owned();
    let parent_path = normalized.parent().ok_or_else(|| {
        Revision3ExactSnapshotImportErrorV2::InvalidDestination(
            "destination has no parent directory".to_owned(),
        )
    })?;
    ensure_safe_directory_chain(parent_path).map_err(|error| {
        Revision3ExactSnapshotImportErrorV2::InvalidDestination(error.to_string())
    })?;
    let parent_path = fs::canonicalize(parent_path).map_err(|error| {
        Revision3ExactSnapshotImportErrorV2::InvalidDestination(error.to_string())
    })?;
    let target = parent_path.join(&filename);
    let parent = Dir::open_ambient_dir(&parent_path, ambient_authority()).map_err(|error| {
        Revision3ExactSnapshotImportErrorV2::InvalidDestination(error.to_string())
    })?;
    let parent_identity = windows_directory_identity(&parent).map_err(|error| {
        Revision3ExactSnapshotImportErrorV2::InvalidDestination(error.to_string())
    })?;
    let guard = ImportDestinationGuard {
        target,
        parent_path,
        filename,
        parent,
        parent_identity,
    };
    revalidate_import_destination_absent(&guard)?;
    Ok(guard)
}

#[cfg(windows)]
fn validate_windows_import_destination_spelling(
    destination: &Path,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    use std::path::{Component, Prefix};

    let invalid =
        |reason: &str| Revision3ExactSnapshotImportErrorV2::InvalidDestination(reason.to_owned());
    let spelling = destination
        .to_str()
        .ok_or_else(|| invalid("destination contains non-Unicode Windows path spelling"))?;
    if spelling.len() > MAX_IMPORT_PATH_UTF8_BYTES_V2 {
        return Err(invalid(
            "destination UTF-8 spelling exceeds the managed 32-KiB limit",
        ));
    }
    if spelling
        .split(['\\', '/'])
        .any(|component| matches!(component, "." | ".."))
    {
        return Err(invalid(
            "destination must not contain explicit dot or parent components",
        ));
    }
    let raw_components = spelling.split(['\\', '/']).collect::<Vec<_>>();
    let component_start = if spelling.starts_with("\\\\") || spelling.starts_with("//") {
        2
    } else {
        0
    };
    if raw_components
        .get(component_start..)
        .is_none_or(|components| components.iter().any(|component| component.is_empty()))
    {
        return Err(invalid(
            "destination must not contain duplicate or trailing separators",
        ));
    }
    let mut components = destination.components();
    match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(_) => {}
            Prefix::UNC(server, share) => {
                validate_windows_import_component(server, true)?;
                validate_windows_import_component(share, true)?;
            }
            Prefix::Verbatim(_)
            | Prefix::VerbatimUNC(_, _)
            | Prefix::VerbatimDisk(_)
            | Prefix::DeviceNS(_) => {
                return Err(invalid(
                    "verbatim and device namespace destinations are not supported",
                ));
            }
        },
        _ => {
            return Err(invalid(
                "destination has no ordinary Windows drive or UNC prefix",
            ))
        }
    }
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(invalid("destination prefix is not filesystem-rooted"));
    }
    let mut normal_components = 0usize;
    for component in components {
        let Component::Normal(name) = component else {
            return Err(invalid(
                "destination must not contain dot, parent, or repeated prefix components",
            ));
        };
        validate_windows_import_component(name, true)?;
        normal_components = normal_components
            .checked_add(1)
            .ok_or_else(|| invalid("destination component count overflowed the native range"))?;
    }
    if normal_components == 0 {
        return Err(invalid(
            "destination must name a child below its filesystem root",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn validate_windows_import_component(
    component: &std::ffi::OsStr,
    reject_reserved_devices: bool,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    let invalid =
        |reason: &str| Revision3ExactSnapshotImportErrorV2::InvalidDestination(reason.to_owned());
    let value = component
        .to_str()
        .ok_or_else(|| invalid("destination contains a non-Unicode Windows component"))?;
    if value.is_empty() {
        return Err(invalid("destination contains an empty Windows component"));
    }
    if value.ends_with(' ') || value.ends_with('.') {
        return Err(invalid(
            "destination components must not end in a space or dot",
        ));
    }
    if value.chars().any(|character| {
        character <= '\u{1f}'
            || ('\u{7f}'..='\u{9f}').contains(&character)
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
    }) {
        return Err(invalid(
            "destination contains a Win32-reserved character or alternate-stream separator",
        ));
    }
    if reject_reserved_devices {
        let stem = value
            .split('.')
            .next()
            .unwrap_or(value)
            .to_ascii_uppercase();
        let numbered_device = stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                (suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9'))
                    || matches!(suffix, "\u{00b9}" | "\u{00b2}" | "\u{00b3}")
            });
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") || numbered_device {
            return Err(invalid(
                "destination contains a reserved Windows device component",
            ));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn revalidate_import_parent(
    guard: &ImportDestinationGuard,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    revalidate_windows_source_parent(&guard.parent, &guard.parent_path, guard.parent_identity)
        .map_err(|error| Revision3ExactSnapshotImportErrorV2::InvalidDestination(error.to_string()))
}

#[cfg(windows)]
fn revalidate_import_destination_absent(
    guard: &ImportDestinationGuard,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    revalidate_import_parent(guard)?;
    match guard.parent.symlink_metadata(&guard.filename) {
        Ok(_) => Err(Revision3ExactSnapshotImportErrorV2::DestinationAlreadyExists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Revision3ExactSnapshotImportErrorV2::InvalidDestination(
            error.to_string(),
        )),
    }
}

#[cfg(windows)]
fn create_import_staging_directory(
    guard: &ImportDestinationGuard,
) -> Result<ImportStagingDirectory, Revision3ExactSnapshotImportErrorV2> {
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt as _;
    use std::os::windows::fs::MetadataExt as _;
    use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _};
    use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
    use windows_sys::Wdk::Storage::FileSystem::{
        NtCreateFile, FILE_CREATE, FILE_DIRECTORY_FILE, FILE_SYNCHRONOUS_IO_NONALERT,
    };
    use windows_sys::Win32::Foundation::{
        RtlNtStatusToDosError, HANDLE, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE,
        STATUS_OBJECT_NAME_COLLISION, UNICODE_STRING,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_DELETE_CHILD,
        FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE,
        FILE_TRAVERSE, SYNCHRONIZE,
    };
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

    revalidate_import_destination_absent(guard)?;
    let parent = guard
        .parent
        .try_clone()
        .map_err(import_materialization_io)?
        .into_std_file();
    for _ in 0..128 {
        let sequence = IMPORT_STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = OsString::from(format!(
            ".gore-import-{}-{sequence:016x}.staging",
            std::process::id()
        ));
        if name == guard.filename {
            continue;
        }
        let mut wide = name.encode_wide().collect::<Vec<_>>();
        let name_bytes = wide
            .len()
            .checked_mul(size_of::<u16>())
            .and_then(|length| u16::try_from(length).ok())
            .ok_or_else(|| {
                Revision3ExactSnapshotImportErrorV2::Materialization(
                    "private staging name exceeds the native range".to_owned(),
                )
            })?;
        let unicode_name = UNICODE_STRING {
            Length: name_bytes,
            MaximumLength: name_bytes,
            Buffer: wide.as_mut_ptr(),
        };
        let object_attributes = OBJECT_ATTRIBUTES {
            Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
            RootDirectory: parent.as_raw_handle() as HANDLE,
            ObjectName: &unicode_name,
            Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
            SecurityDescriptor: std::ptr::null(),
            SecurityQualityOfService: std::ptr::null(),
        };
        let mut raw_handle: HANDLE = std::ptr::null_mut();
        let mut io_status = IO_STATUS_BLOCK::default();
        // SAFETY: the pinned parent and UTF-16 child component remain live. FILE_CREATE is an
        // atomic create-new operation and OBJ_DONT_REPARSE forbids redirection through a reparse
        // point. DELETE access and no delete sharing retain exact root identity through publish.
        let status = unsafe {
            NtCreateFile(
                &mut raw_handle,
                FILE_LIST_DIRECTORY
                    | FILE_ADD_FILE
                    | FILE_ADD_SUBDIRECTORY
                    | FILE_DELETE_CHILD
                    | FILE_TRAVERSE
                    | FILE_READ_ATTRIBUTES
                    | SYNCHRONIZE
                    | DELETE,
                &object_attributes,
                &mut io_status,
                std::ptr::null(),
                FILE_ATTRIBUTE_NORMAL,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                FILE_CREATE,
                FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT,
                std::ptr::null(),
                0,
            )
        };
        if status == STATUS_OBJECT_NAME_COLLISION {
            continue;
        }
        if status < 0 {
            return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                format!("private staging root could not be created ({})", unsafe {
                    RtlNtStatusToDosError(status)
                }),
            ));
        }
        if raw_handle.is_null() {
            return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                "native staging creation returned no directory handle".to_owned(),
            ));
        }
        // SAFETY: successful NtCreateFile returned one newly owned handle.
        let file = unsafe { File::from_raw_handle(raw_handle) };
        if let Some(primary) = injected_import_create_failure(ImportCreateGuardPhase::StagingRoot) {
            return Err(rollback_created_import_object(&file, primary));
        }
        let metadata = match file.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                let primary = import_materialization_io(error);
                return Err(rollback_created_import_object(&file, primary));
            }
        };
        if !metadata.is_dir()
            || metadata.file_attributes()
                & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
                != 0
        {
            let primary = Revision3ExactSnapshotImportErrorV2::Materialization(
                "native staging root is not a regular non-reparse directory".to_owned(),
            );
            return Err(rollback_created_import_object(&file, primary));
        }
        let identity = match windows_file_identity(&file) {
            Ok(identity) => identity,
            Err(error) => {
                let primary =
                    Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string());
                return Err(rollback_created_import_object(&file, primary));
            }
        };
        let directory_file = match file.try_clone() {
            Ok(file) => file,
            Err(error) => {
                let primary = import_materialization_io(error);
                return Err(rollback_created_import_object(&file, primary));
            }
        };
        let directory = cap_std::fs::Dir::from_std_file(directory_file);
        let mut staging = ImportStagingDirectory {
            path: guard.parent_path.join(&name),
            name,
            file: Some(file),
            directory: Some(directory),
            identity,
            files: Vec::new(),
            file_indices: BTreeMap::new(),
            directories: BTreeMap::new(),
            descendant_handles_released: false,
            root_handles_final_frozen: false,
            promoted: false,
        };
        if let Err(primary) = revalidate_import_staging(guard, &staging) {
            return Err(abort_import_staging(&mut staging, primary));
        }
        return Ok(staging);
    }
    Err(Revision3ExactSnapshotImportErrorV2::Materialization(
        "could not allocate one unique private staging directory".to_owned(),
    ))
}

#[cfg(windows)]
fn revalidate_import_staging(
    guard: &ImportDestinationGuard,
    staging: &ImportStagingDirectory,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    if staging.promoted {
        return Err(Revision3ExactSnapshotImportErrorV2::Publication(
            "private staging root was already promoted".to_owned(),
        ));
    }
    revalidate_import_destination_absent(guard)?;
    let root_file = staging.file.as_ref().ok_or_else(|| {
        Revision3ExactSnapshotImportErrorV2::Materialization(
            "retained staging root handle is absent".to_owned(),
        )
    })?;
    if windows_file_identity(root_file)
        .map_err(|error| Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string()))?
        != staging.identity
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "retained staging root identity changed".to_owned(),
        ));
    }
    let named = open_import_named_child(&guard.parent, &staging.name, true)?;
    if windows_file_identity(&named)
        .map_err(|error| Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string()))?
        != staging.identity
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "private staging name no longer identifies the retained directory".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn open_import_named_child(
    parent: &cap_std::fs::Dir,
    name: &std::ffi::OsStr,
    directory: bool,
) -> Result<File, Revision3ExactSnapshotImportErrorV2> {
    use cap_std::fs::{OpenOptions as CapOpenOptions, OpenOptionsExt as _};
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Foundation::GENERIC_READ;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, FILE_TRAVERSE,
    };

    let mut options = CapOpenOptions::new();
    let flags = FILE_FLAG_OPEN_REPARSE_POINT
        | if directory {
            FILE_FLAG_BACKUP_SEMANTICS
        } else {
            0
        };
    options
        .access_mode(if directory {
            FILE_LIST_DIRECTORY | FILE_TRAVERSE | FILE_READ_ATTRIBUTES
        } else {
            GENERIC_READ
        })
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(flags);
    let file = parent
        .open_with(name, &options)
        .map_err(import_materialization_io)?
        .into_std();
    let metadata = file.metadata().map_err(import_materialization_io)?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || metadata.is_dir() != directory
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "planned staging name is not the retained non-reparse object type".to_owned(),
        ));
    }
    Ok(file)
}

#[cfg(windows)]
fn open_verified_staging_directory(
    staging: &ImportStagingDirectory,
    relative: &Path,
) -> Result<cap_std::fs::Dir, Revision3ExactSnapshotImportErrorV2> {
    if relative.as_os_str().is_empty() {
        let directory = staging
            .directory
            .as_ref()
            .ok_or_else(|| {
                Revision3ExactSnapshotImportErrorV2::Materialization(
                    "retained staging root directory handle is absent".to_owned(),
                )
            })?
            .try_clone()
            .map_err(import_materialization_io)?;
        if windows_directory_identity(&directory).map_err(|error| {
            Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string())
        })? != staging.identity
        {
            return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                "staging root identity changed".to_owned(),
            ));
        }
        return Ok(directory);
    }
    let expected = staging.directories.get(relative).ok_or_else(|| {
        Revision3ExactSnapshotImportErrorV2::Materialization(format!(
            "unplanned staging directory {:?}",
            relative
        ))
    })?;
    if let Some(retained) = &expected.directory {
        let directory = retained.try_clone().map_err(import_materialization_io)?;
        if windows_directory_identity(&directory).map_err(|error| {
            Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string())
        })? != expected.identity
        {
            return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                format!("staging directory {:?} changed identity", relative),
            ));
        }
        return Ok(directory);
    }
    if !staging.descendant_handles_released {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "staging directory handle disappeared before the release boundary".to_owned(),
        ));
    }

    let mut directory = staging
        .directory
        .as_ref()
        .ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::Materialization(
                "retained staging root directory handle is absent".to_owned(),
            )
        })?
        .try_clone()
        .map_err(import_materialization_io)?;
    let mut current = PathBuf::new();
    for component in relative.components() {
        let std::path::Component::Normal(name) = component else {
            return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                "recorded staging directory path is not component-normal".to_owned(),
            ));
        };
        current.push(name);
        let recorded = staging.directories.get(&current).ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::Materialization(format!(
                "recorded staging directory {:?} has no ownership entry",
                current
            ))
        })?;
        let child = open_import_named_child(&directory, name, true)?;
        if windows_file_identity(&child).map_err(|error| {
            Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string())
        })? != recorded.identity
        {
            return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                format!("staging directory {:?} changed identity", current),
            ));
        }
        directory = cap_std::fs::Dir::from_std_file(child);
    }
    Ok(directory)
}

#[cfg(windows)]
fn ensure_import_member_parent_directories(
    staging: &mut ImportStagingDirectory,
    relative_file: &Path,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    use std::path::Component;

    if relative_file.is_absolute() {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "planned Store member path is absolute".to_owned(),
        ));
    }
    let parent = relative_file.parent().unwrap_or_else(|| Path::new(""));
    let mut current = PathBuf::new();
    for component in parent.components() {
        let Component::Normal(name) = component else {
            return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                "planned Store member path contains a non-normal component".to_owned(),
            ));
        };
        let parent_relative = current.clone();
        current.push(name);
        if staging.directories.contains_key(&current) {
            open_verified_staging_directory(staging, &current)?;
            continue;
        }
        let parent_directory = open_verified_staging_directory(staging, &parent_relative)?;
        let created = create_import_directory_child(&parent_directory, name)?;
        if staging
            .directories
            .insert(current.clone(), created)
            .is_some()
        {
            return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                "staging directory identity was recorded twice".to_owned(),
            ));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn create_import_directory_child(
    parent: &cap_std::fs::Dir,
    name: &std::ffi::OsStr,
) -> Result<CreatedImportDirectory, Revision3ExactSnapshotImportErrorV2> {
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt as _;
    use std::os::windows::fs::MetadataExt as _;
    use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _};
    use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
    use windows_sys::Wdk::Storage::FileSystem::{
        NtCreateFile, FILE_CREATE, FILE_DIRECTORY_FILE, FILE_SYNCHRONOUS_IO_NONALERT,
    };
    use windows_sys::Win32::Foundation::{
        RtlNtStatusToDosError, HANDLE, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE, UNICODE_STRING,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_DELETE_CHILD,
        FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE,
        FILE_TRAVERSE, SYNCHRONIZE,
    };
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

    let parent = parent
        .try_clone()
        .map_err(import_materialization_io)?
        .into_std_file();
    let mut wide = name.encode_wide().collect::<Vec<_>>();
    let name_bytes = wide
        .len()
        .checked_mul(size_of::<u16>())
        .and_then(|length| u16::try_from(length).ok())
        .ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::Materialization(
                "Store directory component exceeds the native range".to_owned(),
            )
        })?;
    let unicode_name = UNICODE_STRING {
        Length: name_bytes,
        MaximumLength: name_bytes,
        Buffer: wide.as_mut_ptr(),
    };
    let object_attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent.as_raw_handle() as HANDLE,
        ObjectName: &unicode_name,
        Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let mut raw_handle: HANDLE = std::ptr::null_mut();
    let mut io_status = IO_STATUS_BLOCK::default();
    // SAFETY: parent is the exact verified directory handle, the child is one normal component,
    // and FILE_CREATE refuses any preexisting (including attacker-created) name.
    let status = unsafe {
        NtCreateFile(
            &mut raw_handle,
            FILE_LIST_DIRECTORY
                | FILE_ADD_FILE
                | FILE_ADD_SUBDIRECTORY
                | FILE_DELETE_CHILD
                | FILE_TRAVERSE
                | FILE_READ_ATTRIBUTES
                | SYNCHRONIZE
                | DELETE,
            &object_attributes,
            &mut io_status,
            std::ptr::null(),
            FILE_ATTRIBUTE_NORMAL,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            FILE_CREATE,
            FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT,
            std::ptr::null(),
            0,
        )
    };
    if status < 0 {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            format!("Store directory could not be created ({})", unsafe {
                RtlNtStatusToDosError(status)
            }),
        ));
    }
    if raw_handle.is_null() {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "native Store directory creation returned no handle".to_owned(),
        ));
    }
    // SAFETY: successful NtCreateFile returned one newly owned handle.
    let file = unsafe { File::from_raw_handle(raw_handle) };
    if let Some(primary) = injected_import_create_failure(ImportCreateGuardPhase::StoreDirectory) {
        return Err(rollback_created_import_object(&file, primary));
    }
    let metadata = match file.metadata() {
        Ok(metadata) => metadata,
        Err(error) => {
            let primary = import_materialization_io(error);
            return Err(rollback_created_import_object(&file, primary));
        }
    };
    if !metadata.is_dir()
        || metadata.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
            != 0
    {
        let primary = Revision3ExactSnapshotImportErrorV2::Materialization(
            "created Store directory is not a non-reparse directory".to_owned(),
        );
        return Err(rollback_created_import_object(&file, primary));
    }
    let identity = match windows_file_identity(&file) {
        Ok(identity) => identity,
        Err(error) => {
            let primary = Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string());
            return Err(rollback_created_import_object(&file, primary));
        }
    };
    let directory_file = match file.try_clone() {
        Ok(file) => file,
        Err(error) => {
            let primary = import_materialization_io(error);
            return Err(rollback_created_import_object(&file, primary));
        }
    };
    Ok(CreatedImportDirectory {
        file: Some(file),
        directory: Some(cap_std::fs::Dir::from_std_file(directory_file)),
        identity,
    })
}

fn import_store_relative(
    archive_name: &str,
) -> Result<PathBuf, Revision3ExactSnapshotImportErrorV2> {
    let relative = archive_name.strip_prefix("store/").ok_or_else(|| {
        Revision3ExactSnapshotImportErrorV2::Materialization(
            "planned Store object is outside the sealed store/ namespace".to_owned(),
        )
    })?;
    if relative.is_empty() || relative.contains('\\') {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "planned Store object has no canonical relative path".to_owned(),
        ));
    }
    let path = PathBuf::from(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "planned Store object path is not one closed relative path".to_owned(),
        ));
    }
    Ok(path)
}

#[cfg(windows)]
fn materialize_archive_member(
    archive: &mut ZipArchive<File>,
    member: &InspectedMember,
    relative: &Path,
    staging: &mut ImportStagingDirectory,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    let mut entry = archive
        .by_index(member.index)
        .map_err(|error| Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string()))?;
    materialize_reader_member(&mut entry, &member.seal, relative, staging)
}

#[cfg(windows)]
fn materialize_bytes_member(
    bytes: &[u8],
    seal: &ContentSeal,
    relative: &Path,
    staging: &mut ImportStagingDirectory,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    materialize_reader_member(&mut io::Cursor::new(bytes), seal, relative, staging)
}

#[cfg(windows)]
fn materialize_reader_member<R: Read>(
    reader: &mut R,
    seal: &ContentSeal,
    relative: &Path,
    staging: &mut ImportStagingDirectory,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    if staging.files.len() >= MAX_IMPORT_ARCHIVE_ENTRIES_V2 as usize {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "planned Store file inventory exceeded the closed archive bound".to_owned(),
        ));
    }
    ensure_import_member_parent_directories(staging, relative)?;
    if staging.file_indices.contains_key(relative) {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            format!("Store member {:?} was materialized twice", relative),
        ));
    }
    let parent_relative = relative.parent().unwrap_or_else(|| Path::new(""));
    let leaf = relative.file_name().ok_or_else(|| {
        Revision3ExactSnapshotImportErrorV2::Materialization(
            "Store member has no final component".to_owned(),
        )
    })?;
    let parent = open_verified_staging_directory(staging, parent_relative)?;
    let (file, identity) = create_import_file_child(&parent, leaf)?;
    // Record ownership immediately after atomic create-new so partial writes are still cleaned by
    // the exact bounded inventory on every pre-publication failure.
    staging.files.push(CreatedImportFile {
        relative: relative.to_path_buf(),
        file: Some(file),
        identity,
        seal: seal.clone(),
    });
    let file_index = staging.files.len() - 1;
    if staging
        .file_indices
        .insert(relative.to_path_buf(), file_index)
        .is_some()
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "Store member ownership index was inserted twice".to_owned(),
        ));
    }
    let file = staging.files[file_index]
        .file
        .as_mut()
        .expect("newly retained Store member handle");

    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(import_materialization_io)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::Materialization(
                "Store member length overflowed while streaming".to_owned(),
            )
        })?;
        if total > seal.byte_len {
            return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                format!("Store member {:?} exceeds its authenticated seal", relative),
            ));
        }
        file.write_all(&buffer[..count])
            .map_err(import_materialization_io)?;
        hasher.update(&buffer[..count]);
    }
    let digest = Sha256Digest::from_bytes(hasher.finalize().into());
    if total != seal.byte_len || digest != seal.sha256 {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            format!(
                "Store member {:?} differs from its authenticated seal",
                relative
            ),
        ));
    }
    file.flush().map_err(import_materialization_io)?;
    file.sync_all().map_err(import_materialization_io)?;
    file.seek(SeekFrom::Start(0))
        .map_err(import_materialization_io)?;
    let reopened = hash_import_file_handle(&mut *file, seal.byte_len)?;
    if reopened != seal.sha256
        || windows_file_identity(file).map_err(|error| {
            Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string())
        })? != identity
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            format!(
                "Store member {:?} changed after its durable write",
                relative
            ),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn create_import_file_child(
    parent: &cap_std::fs::Dir,
    name: &std::ffi::OsStr,
) -> Result<(File, WindowsFileIdentity), Revision3ExactSnapshotImportErrorV2> {
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt as _;
    use std::os::windows::fs::MetadataExt as _;
    use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _};
    use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
    use windows_sys::Wdk::Storage::FileSystem::{
        NtCreateFile, FILE_CREATE, FILE_NON_DIRECTORY_FILE, FILE_SYNCHRONOUS_IO_NONALERT,
    };
    use windows_sys::Win32::Foundation::{
        RtlNtStatusToDosError, GENERIC_READ, GENERIC_WRITE, HANDLE, OBJ_CASE_INSENSITIVE,
        OBJ_DONT_REPARSE, UNICODE_STRING,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT, FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ, SYNCHRONIZE,
    };
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

    let parent = parent
        .try_clone()
        .map_err(import_materialization_io)?
        .into_std_file();
    let mut wide = name.encode_wide().collect::<Vec<_>>();
    let name_bytes = wide
        .len()
        .checked_mul(size_of::<u16>())
        .and_then(|length| u16::try_from(length).ok())
        .ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::Materialization(
                "Store filename exceeds the native range".to_owned(),
            )
        })?;
    let unicode_name = UNICODE_STRING {
        Length: name_bytes,
        MaximumLength: name_bytes,
        Buffer: wide.as_mut_ptr(),
    };
    let object_attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent.as_raw_handle() as HANDLE,
        ObjectName: &unicode_name,
        Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let mut raw_handle: HANDLE = std::ptr::null_mut();
    let mut io_status = IO_STATUS_BLOCK::default();
    // SAFETY: parent is the exact retained immediate directory, name is one normal component,
    // and FILE_CREATE atomically refuses any existing file/reparse object. Read sharing permits
    // ordinary Store readers while write/delete sharing stays denied. Every descendant handle is
    // intentionally closed only after the final private proof and immediately before promotion.
    let status = unsafe {
        NtCreateFile(
            &mut raw_handle,
            GENERIC_READ | GENERIC_WRITE | FILE_READ_ATTRIBUTES | DELETE | SYNCHRONIZE,
            &object_attributes,
            &mut io_status,
            std::ptr::null(),
            FILE_ATTRIBUTE_NORMAL,
            FILE_SHARE_READ,
            FILE_CREATE,
            FILE_NON_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT,
            std::ptr::null(),
            0,
        )
    };
    if status < 0 {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            format!("Store member could not be created ({})", unsafe {
                RtlNtStatusToDosError(status)
            }),
        ));
    }
    if raw_handle.is_null() {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "native Store member creation returned no handle".to_owned(),
        ));
    }
    // SAFETY: successful NtCreateFile returned one newly owned handle.
    let file = unsafe { File::from_raw_handle(raw_handle) };
    if let Some(primary) = injected_import_create_failure(ImportCreateGuardPhase::StoreFile) {
        return Err(rollback_created_import_object(&file, primary));
    }
    let metadata = match file.metadata() {
        Ok(metadata) => metadata,
        Err(error) => {
            let primary = import_materialization_io(error);
            return Err(rollback_created_import_object(&file, primary));
        }
    };
    let identity = match windows_file_identity(&file) {
        Ok(identity) => identity,
        Err(error) => {
            let primary = Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string());
            return Err(rollback_created_import_object(&file, primary));
        }
    };
    if !metadata.is_file()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || identity.links != 1
    {
        let primary = Revision3ExactSnapshotImportErrorV2::Materialization(
            "created Store member is not a single-link non-reparse regular file".to_owned(),
        );
        return Err(rollback_created_import_object(&file, primary));
    }
    Ok((file, identity))
}

#[cfg(windows)]
fn hash_import_file_handle(
    file: &mut File,
    expected_len: u64,
) -> Result<Sha256Digest, Revision3ExactSnapshotImportErrorV2> {
    file.seek(SeekFrom::Start(0))
        .map_err(import_materialization_io)?;
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        let count = file.read(&mut buffer).map_err(import_materialization_io)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::Materialization(
                "Store member length overflowed while rehashing".to_owned(),
            )
        })?;
        if total > expected_len {
            return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                "Store member grew while rehashing".to_owned(),
            ));
        }
        hasher.update(&buffer[..count]);
    }
    if total != expected_len {
        return Err(Revision3ExactSnapshotImportErrorV2::Materialization(
            "Store member length changed while rehashing".to_owned(),
        ));
    }
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}

#[cfg(windows)]
fn verify_import_staging_shape_and_seals(
    staging: &ImportStagingDirectory,
    expect_head: bool,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    let has_head = staging
        .files
        .iter()
        .any(|created| created.relative == Path::new(HEAD_FILE_NAME));
    if has_head != expect_head {
        return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
            "staged fixed-head publication order is invalid".to_owned(),
        ));
    }

    let mut expected = BTreeMap::<PathBuf, BTreeMap<OsString, ExpectedImportEntry>>::new();
    expected.entry(PathBuf::new()).or_default();
    for (relative, created) in &staging.directories {
        let parent = relative.parent().unwrap_or_else(|| Path::new(""));
        let name = relative.file_name().ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                "staged directory has no final component".to_owned(),
            )
        })?;
        if expected
            .entry(parent.to_path_buf())
            .or_default()
            .insert(
                name.to_owned(),
                ExpectedImportEntry::Directory(created.identity),
            )
            .is_some()
        {
            return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                "two staged directories share one name".to_owned(),
            ));
        }
        expected.entry(relative.clone()).or_default();
    }
    for (file_index, created) in staging.files.iter().enumerate() {
        let parent = created.relative.parent().unwrap_or_else(|| Path::new(""));
        let name = created.relative.file_name().ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                "staged Store member has no final component".to_owned(),
            )
        })?;
        if expected
            .entry(parent.to_path_buf())
            .or_default()
            .insert(name.to_owned(), ExpectedImportEntry::File(file_index))
            .is_some()
        {
            return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                "two staged Store entries share one name".to_owned(),
            ));
        }
    }

    let max_entries = staging
        .files
        .len()
        .checked_add(staging.directories.len())
        .ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                "staged Store shape bound overflowed".to_owned(),
            )
        })?;
    let mut visited = 0usize;
    for (relative, planned_children) in &expected {
        let directory = open_verified_staging_directory(staging, relative).map_err(|error| {
            Revision3ExactSnapshotImportErrorV2::CandidateVerification(error.to_string())
        })?;
        let mut remaining = planned_children.clone();
        let entries = directory.entries().map_err(|error| {
            Revision3ExactSnapshotImportErrorV2::CandidateVerification(error.to_string())
        })?;
        for entry in entries {
            visited = visited.checked_add(1).ok_or_else(|| {
                Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                    "staged Store traversal count overflowed".to_owned(),
                )
            })?;
            if visited > max_entries {
                return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                    "staged Store contains entries outside the bounded plan".to_owned(),
                ));
            }
            let entry = entry.map_err(|error| {
                Revision3ExactSnapshotImportErrorV2::CandidateVerification(error.to_string())
            })?;
            let name = entry.file_name();
            let planned = remaining.remove(&name).ok_or_else(|| {
                Revision3ExactSnapshotImportErrorV2::CandidateVerification(format!(
                    "staged Store contains unplanned entry {:?}",
                    relative.join(&name)
                ))
            })?;
            let child_relative = relative.join(&name);
            match planned {
                ExpectedImportEntry::Directory(identity) => {
                    let child =
                        open_import_named_child(&directory, &name, true).map_err(|error| {
                            Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                                error.to_string(),
                            )
                        })?;
                    if windows_file_identity(&child).map_err(|error| {
                        Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                            error.to_string(),
                        )
                    })? != identity
                    {
                        return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                            format!("staged directory {:?} changed identity", child_relative),
                        ));
                    }
                }
                ExpectedImportEntry::File(file_index) => {
                    let created = staging.files.get(file_index).ok_or_else(|| {
                        Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                            "staged file index is absent from its ownership inventory".to_owned(),
                        )
                    })?;
                    let named =
                        open_import_named_child(&directory, &name, false).map_err(|error| {
                            Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                                error.to_string(),
                            )
                        })?;
                    if windows_file_identity(&named).map_err(|error| {
                        Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                            error.to_string(),
                        )
                    })? != created.identity
                    {
                        return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                            format!("staged Store member {:?} changed identity", child_relative),
                        ));
                    }
                    if created.relative != child_relative {
                        return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                            "staged file index differs from its ownership path".to_owned(),
                        ));
                    }
                    verify_import_file(created, &named)?;
                }
            }
        }
        if !remaining.is_empty() {
            return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
                format!("staged directory {:?} is missing planned entries", relative),
            ));
        }
    }
    if visited != max_entries {
        return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
            "staged Store traversal did not cover the exact ownership plan".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn verify_import_file(
    created: &CreatedImportFile,
    file: &File,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

    let metadata = file.metadata().map_err(|error| {
        Revision3ExactSnapshotImportErrorV2::CandidateVerification(error.to_string())
    })?;
    let identity = windows_file_identity(file).map_err(|error| {
        Revision3ExactSnapshotImportErrorV2::CandidateVerification(error.to_string())
    })?;
    if !metadata.is_file()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || identity != created.identity
        || identity.links != 1
        || metadata.len() != created.seal.byte_len
    {
        return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
            format!(
                "staged Store member {:?} changed type, identity, or length",
                created.relative
            ),
        ));
    }
    let mut file = file.try_clone().map_err(|error| {
        Revision3ExactSnapshotImportErrorV2::CandidateVerification(error.to_string())
    })?;
    let digest = hash_import_file_handle(&mut file, created.seal.byte_len)?;
    if digest != created.seal.sha256 {
        return Err(Revision3ExactSnapshotImportErrorV2::CandidateVerification(
            format!("staged Store member {:?} changed content", created.relative),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn release_import_descendant_handles(
    staging: &mut ImportStagingDirectory,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    if staging.descendant_handles_released {
        return Err(Revision3ExactSnapshotImportErrorV2::Publication(
            "staging descendant handles were released twice".to_owned(),
        ));
    }
    if staging.files.iter().any(|created| created.file.is_none())
        || staging
            .directories
            .values()
            .any(|created| created.file.is_none() || created.directory.is_none())
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Publication(
            "staging ownership inventory was incomplete before handle release".to_owned(),
        ));
    }

    // Windows cannot rename a directory containing open descendant handles. The root handle stays
    // pinned; every descendant identity and seal remains in the bounded inventory and is reopened
    // component-relative immediately on both sides of the atomic no-clobber rename.
    for created in &mut staging.files {
        drop(created.file.take());
    }
    for created in staging.directories.values_mut() {
        drop(created.directory.take());
        drop(created.file.take());
    }
    staging.descendant_handles_released = true;
    Ok(())
}

#[cfg(windows)]
fn open_import_named_child_locked(
    parent: &cap_std::fs::Dir,
    name: &std::ffi::OsStr,
    directory: bool,
) -> Result<File, Revision3ExactSnapshotImportErrorV2> {
    use cap_std::fs::{OpenOptions as CapOpenOptions, OpenOptionsExt as _};
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Foundation::GENERIC_READ;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_TRAVERSE,
    };

    let mut options = CapOpenOptions::new();
    options
        .access_mode(if directory {
            FILE_LIST_DIRECTORY | FILE_TRAVERSE | FILE_READ_ATTRIBUTES
        } else {
            GENERIC_READ
        })
        .share_mode(FILE_SHARE_READ)
        .custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    0
                },
        );
    let file = parent
        .open_with(name, &options)
        .map_err(import_materialization_io)?
        .into_std();
    let metadata = file.metadata().map_err(import_materialization_io)?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || metadata.is_dir() != directory
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Publication(
            "published Store object changed type or became a reparse point".to_owned(),
        ));
    }
    Ok(file)
}

#[cfg(windows)]
fn freeze_published_import_root(
    guard: &ImportDestinationGuard,
    staging: &mut ImportStagingDirectory,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    if !staging.promoted
        || !staging.descendant_handles_released
        || staging.root_handles_final_frozen
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Publication(
            "published Store is not at its root-freeze boundary".to_owned(),
        ));
    }

    // A Windows share mode cannot be narrowed on a live handle. Close both writable root
    // capabilities, then reopen the final component through the still-pinned parent. Identity,
    // type and reparse checks make any transition race receipt-free uncertainty.
    drop(staging.directory.take());
    drop(staging.file.take());
    let file = open_import_named_child_locked(&guard.parent, &guard.filename, true)?;
    if windows_file_identity(&file)
        .map_err(|error| Revision3ExactSnapshotImportErrorV2::Publication(error.to_string()))?
        != staging.identity
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Publication(
            "published root changed identity during final freeze".to_owned(),
        ));
    }
    let directory_file = file
        .try_clone()
        .map_err(|error| Revision3ExactSnapshotImportErrorV2::Publication(error.to_string()))?;
    staging.file = Some(file);
    staging.directory = Some(cap_std::fs::Dir::from_std_file(directory_file));
    staging.root_handles_final_frozen = true;
    Ok(())
}

#[cfg(windows)]
impl PublishedImportChangeWatch {
    fn start(
        guard: &ImportDestinationGuard,
        expected_identity: WindowsFileIdentity,
    ) -> Result<Self, Revision3ExactSnapshotImportErrorV2> {
        use cap_std::fs::{OpenOptions as CapOpenOptions, OpenOptionsExt as _};
        use std::os::windows::fs::MetadataExt as _;
        use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _};
        use windows_sys::Win32::Storage::FileSystem::{
            ReadDirectoryChangesW, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
            FILE_NOTIFY_CHANGE_ATTRIBUTES, FILE_NOTIFY_CHANGE_CREATION,
            FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME,
            FILE_NOTIFY_CHANGE_LAST_ACCESS, FILE_NOTIFY_CHANGE_LAST_WRITE,
            FILE_NOTIFY_CHANGE_SECURITY, FILE_NOTIFY_CHANGE_SIZE, FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ,
        };
        use windows_sys::Win32::System::Threading::CreateEventW;

        let mut options = CapOpenOptions::new();
        options
            .access_mode(FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OVERLAPPED,
            );
        let file = guard
            .parent
            .open_with(&guard.filename, &options)
            .map_err(|error| Revision3ExactSnapshotImportErrorV2::Publication(error.to_string()))?
            .into_std();
        let metadata = file
            .metadata()
            .map_err(|error| Revision3ExactSnapshotImportErrorV2::Publication(error.to_string()))?;
        if !metadata.is_dir()
            || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || windows_file_identity(&file).map_err(|error| {
                Revision3ExactSnapshotImportErrorV2::Publication(error.to_string())
            })? != expected_identity
        {
            return Err(Revision3ExactSnapshotImportErrorV2::Publication(
                "published root changed before its recursive change watch was armed".to_owned(),
            ));
        }

        // ReadDirectoryChangesW requires DWORD alignment and rejects buffers larger than 64 KiB
        // for remote filesystems. The exact portable bound supports ordinary local and UNC roots
        // while keeping overflow/error handling deterministic and fail closed.
        let mut buffer = Box::new([0u32; 16_384]);
        let mut overlapped = Box::new(windows_sys::Win32::System::IO::OVERLAPPED::default());
        // SAFETY: null security/name pointers request one private unnamed event. The returned
        // handle is immediately transferred into OwnedHandle and retained beyond I/O drainage.
        let event_handle = unsafe {
            CreateEventW(
                std::ptr::null(),
                true.into(),
                false.into(),
                std::ptr::null(),
            )
        };
        if event_handle.is_null() {
            return Err(Revision3ExactSnapshotImportErrorV2::Publication(
                io::Error::last_os_error().to_string(),
            ));
        }
        // SAFETY: CreateEventW returned one newly owned non-null kernel handle.
        let event = unsafe { std::os::windows::io::OwnedHandle::from_raw_handle(event_handle) };
        overlapped.hEvent = event_handle;
        let filter = FILE_NOTIFY_CHANGE_FILE_NAME
            | FILE_NOTIFY_CHANGE_DIR_NAME
            | FILE_NOTIFY_CHANGE_ATTRIBUTES
            | FILE_NOTIFY_CHANGE_SIZE
            | FILE_NOTIFY_CHANGE_LAST_WRITE
            | FILE_NOTIFY_CHANGE_LAST_ACCESS
            | FILE_NOTIFY_CHANGE_CREATION
            | FILE_NOTIFY_CHANGE_SECURITY;
        // SAFETY: the overlapped file handle, aligned 64-KiB buffer and OVERLAPPED allocation all
        // remain owned by the returned guard until cancellation has synchronously completed.
        if unsafe {
            ReadDirectoryChangesW(
                file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE,
                buffer.as_mut_ptr().cast(),
                std::mem::size_of_val(buffer.as_ref()) as u32,
                true.into(),
                filter,
                std::ptr::null_mut(),
                overlapped.as_mut(),
                None,
            )
        } == 0
        {
            return Err(Revision3ExactSnapshotImportErrorV2::Publication(
                io::Error::last_os_error().to_string(),
            ));
        }
        Ok(Self {
            file,
            buffer,
            overlapped,
            _event: event,
        })
    }

    /// Linearize the confirmed receipt only while the recursive watch is still pending. Any
    /// completion means a change, buffer overflow, or asynchronous failure and is uncertainty.
    fn quiet_at_linearization(&mut self) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
        use std::os::windows::io::AsRawHandle as _;
        use windows_sys::Win32::Foundation::ERROR_IO_INCOMPLETE;
        use windows_sys::Win32::System::IO::GetOverlappedResult;

        let mut transferred = 0u32;
        // SAFETY: the watch handle and its OVERLAPPED allocation remain live. `false` makes this
        // the required nonblocking receipt-linearization check.
        if unsafe {
            GetOverlappedResult(
                self.file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE,
                self.overlapped.as_ref(),
                &mut transferred,
                false.into(),
            )
        } != 0
        {
            return Err(Revision3ExactSnapshotImportErrorV2::Publication(format!(
                "published Store changed during receipt verification ({transferred} notification bytes)"
            )));
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_IO_INCOMPLETE as i32) {
            Ok(())
        } else {
            Err(Revision3ExactSnapshotImportErrorV2::Publication(format!(
                "published Store change watch did not remain pending ({error})"
            )))
        }
    }
}

#[cfg(windows)]
impl Drop for PublishedImportChangeWatch {
    fn drop(&mut self) {
        use std::os::windows::io::AsRawHandle as _;
        use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult};

        let handle = self.file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
        // SAFETY: cancellation targets this guard's exact outstanding OVERLAPPED. Waiting for its
        // terminal result before fields drop prevents the kernel from retaining either pointer.
        unsafe {
            CancelIoEx(handle, self.overlapped.as_ref());
            let mut transferred = 0u32;
            GetOverlappedResult(
                handle,
                self.overlapped.as_ref(),
                &mut transferred,
                true.into(),
            );
        }
        // Keep the buffer observably owned until after the wait above.
        let _ = self.buffer.len();
    }
}

#[cfg(windows)]
fn reacquire_published_import_descendant_handles(
    staging: &mut ImportStagingDirectory,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    if !staging.descendant_handles_released
        || !staging.promoted
        || !staging.root_handles_final_frozen
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Publication(
            "published Store is not at its descendant-handle reacquisition boundary".to_owned(),
        ));
    }

    let mut directory_paths = staging.directories.keys().cloned().collect::<Vec<_>>();
    directory_paths.sort_by(|left, right| {
        left.components()
            .count()
            .cmp(&right.components().count())
            .then_with(|| left.cmp(right))
    });
    let mut directories = BTreeMap::<PathBuf, (File, cap_std::fs::Dir)>::new();
    for relative in directory_paths {
        let parent_relative = relative.parent().unwrap_or_else(|| Path::new(""));
        let parent = if parent_relative.as_os_str().is_empty() {
            staging
                .directory
                .as_ref()
                .ok_or_else(|| {
                    Revision3ExactSnapshotImportErrorV2::Publication(
                        "published root freeze handle is absent".to_owned(),
                    )
                })?
                .try_clone()
                .map_err(import_materialization_io)?
        } else {
            directories
                .get(parent_relative)
                .ok_or_else(|| {
                    Revision3ExactSnapshotImportErrorV2::Publication(format!(
                        "published directory {:?} has no reacquired parent",
                        relative
                    ))
                })?
                .1
                .try_clone()
                .map_err(import_materialization_io)?
        };
        let name = relative.file_name().ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::Publication(
                "published directory has no final component".to_owned(),
            )
        })?;
        let file = open_import_named_child_locked(&parent, name, true)?;
        let expected = staging.directories.get(&relative).ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::Publication(
                "published directory is absent from its ownership inventory".to_owned(),
            )
        })?;
        if windows_file_identity(&file)
            .map_err(|error| Revision3ExactSnapshotImportErrorV2::Publication(error.to_string()))?
            != expected.identity
        {
            return Err(Revision3ExactSnapshotImportErrorV2::Publication(format!(
                "published directory {:?} changed identity",
                relative
            )));
        }
        let directory =
            cap_std::fs::Dir::from_std_file(file.try_clone().map_err(import_materialization_io)?);
        directories.insert(relative, (file, directory));
    }

    let mut files = Vec::<Option<File>>::with_capacity(staging.files.len());
    files.resize_with(staging.files.len(), || None);
    for (index, created) in staging.files.iter().enumerate() {
        let parent_relative = created.relative.parent().unwrap_or_else(|| Path::new(""));
        let parent = if parent_relative.as_os_str().is_empty() {
            staging
                .directory
                .as_ref()
                .ok_or_else(|| {
                    Revision3ExactSnapshotImportErrorV2::Publication(
                        "published root freeze handle is absent".to_owned(),
                    )
                })?
                .try_clone()
                .map_err(import_materialization_io)?
        } else {
            directories
                .get(parent_relative)
                .ok_or_else(|| {
                    Revision3ExactSnapshotImportErrorV2::Publication(format!(
                        "published file {:?} has no reacquired parent",
                        created.relative
                    ))
                })?
                .1
                .try_clone()
                .map_err(import_materialization_io)?
        };
        let name = created.relative.file_name().ok_or_else(|| {
            Revision3ExactSnapshotImportErrorV2::Publication(
                "published Store member has no final component".to_owned(),
            )
        })?;
        let file = open_import_named_child_locked(&parent, name, false)?;
        if windows_file_identity(&file)
            .map_err(|error| Revision3ExactSnapshotImportErrorV2::Publication(error.to_string()))?
            != created.identity
        {
            return Err(Revision3ExactSnapshotImportErrorV2::Publication(format!(
                "published Store member {:?} changed identity",
                created.relative
            )));
        }
        files[index] = Some(file);
    }

    for (relative, (file, directory)) in directories {
        let created = staging
            .directories
            .get_mut(&relative)
            .expect("reacquired recorded directory");
        created.file = Some(file);
        created.directory = Some(directory);
    }
    for (created, file) in staging.files.iter_mut().zip(files) {
        created.file = file;
    }
    staging.descendant_handles_released = false;
    Ok(())
}

#[cfg(windows)]
fn revalidate_verified_source_for_import(
    verified: &VerifiedRevision3ExactSnapshotArchiveV2,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    let metadata = verified.source.metadata().map_err(source_io_error)?;
    if metadata.len() != verified.archive.byte_len {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "retained source length changed during destination materialization".to_owned(),
        )
        .into());
    }
    let digest = hash_open_file(&verified.source, verified.archive.byte_len)?;
    if digest != verified.archive.sha256 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "retained source bytes changed during destination materialization".to_owned(),
        )
        .into());
    }
    Ok(())
}

#[cfg(windows)]
enum ImportPublishError {
    AlreadyExists,
    /// Publication has definitely not reached the native rename syscall.
    BeforeSyscall(String),
    /// The native rename syscall was entered, so a failed response cannot prove that a remote
    /// filesystem did not commit the rename before losing the response.
    PublicationUncertain,
}

#[cfg(windows)]
fn publish_import_staging_no_clobber(
    guard: &ImportDestinationGuard,
    staging: &mut ImportStagingDirectory,
) -> Result<(), ImportPublishError> {
    use std::mem::{offset_of, size_of};
    use std::os::windows::ffi::OsStrExt as _;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Wdk::Storage::FileSystem::{
        FileRenameInformation, NtSetInformationFile, FILE_RENAME_INFORMATION,
    };
    use windows_sys::Win32::Foundation::{RtlNtStatusToDosError, HANDLE};
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

    let name = guard.filename.encode_wide().collect::<Vec<_>>();
    let header_bytes = offset_of!(FILE_RENAME_INFORMATION, FileName);
    let byte_len = header_bytes
        .checked_add(name.len().checked_mul(size_of::<u16>()).ok_or_else(|| {
            ImportPublishError::BeforeSyscall("destination name length overflowed".to_owned())
        })?)
        .ok_or_else(|| {
            ImportPublishError::BeforeSyscall("destination rename buffer overflowed".to_owned())
        })?;
    let mut storage = vec![0u64; byte_len.div_ceil(size_of::<u64>())];
    let info = storage.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    let parent = guard
        .parent
        .try_clone()
        .map_err(|error| ImportPublishError::BeforeSyscall(error.to_string()))?
        .into_std_file();
    let root_file = staging.file.as_ref().ok_or_else(|| {
        ImportPublishError::BeforeSyscall(
            "staging root handle is absent before publication".to_owned(),
        )
    })?;
    let mut io_status = IO_STATUS_BLOCK::default();
    // SAFETY: storage is aligned and exactly sized for the fixed header and one final UTF-16
    // component. Both the retained staged-directory handle and pinned parent remain live.
    unsafe {
        (*info).Anonymous.ReplaceIfExists = false;
        (*info).RootDirectory = parent.as_raw_handle() as HANDLE;
        (*info).FileNameLength = (name.len() * size_of::<u16>()) as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            storage
                .as_mut_ptr()
                .cast::<u8>()
                .add(header_bytes)
                .cast::<u16>(),
            name.len(),
        );
        let status = NtSetInformationFile(
            root_file.as_raw_handle() as HANDLE,
            &mut io_status,
            info.cast_const().cast(),
            byte_len as u32,
            FileRenameInformation,
        );
        if status < 0 {
            let error = io::Error::from_raw_os_error(RtlNtStatusToDosError(status) as i32);
            if error.kind() == io::ErrorKind::AlreadyExists
                || matches!(error.raw_os_error(), Some(80 | 183))
            {
                return Err(ImportPublishError::AlreadyExists);
            }
            // Once the syscall was entered, even an error response is not proof of non-
            // publication on SMB: the server may have committed the rename before the response
            // or connection was lost. Never turn this state into an ordinary retry-safe error.
            return Err(ImportPublishError::PublicationUncertain);
        }
    }

    #[cfg(test)]
    if IMPORT_PUBLISH_POST_RENAME_FAILURE.with(|failpoint| failpoint.replace(false)) {
        // Model a remote server committing the rename before its response is lost. Deliberately
        // leave `promoted` false so the caller must classify the syscall boundary itself.
        return Err(ImportPublishError::PublicationUncertain);
    }
    staging.promoted = true;
    // Every child file was individually synced. Flushing the retained root after the handle-
    // relative rename is the final available Windows durability step; failure is post-boundary
    // uncertainty and can never be reported as a normal error.
    root_file
        .sync_all()
        .map_err(|_| ImportPublishError::PublicationUncertain)?;
    Ok(())
}

#[cfg(all(windows, test))]
fn import_final_identity(guard: &ImportDestinationGuard) -> Option<WindowsFileIdentity> {
    let file = open_import_named_child(&guard.parent, &guard.filename, true).ok()?;
    windows_file_identity(&file).ok()
}

#[cfg(windows)]
fn verify_published_import_destination(
    guard: &ImportDestinationGuard,
    staging: &ImportStagingDirectory,
) -> Result<(), Revision3ExactSnapshotImportErrorV2> {
    if !staging.promoted || !staging.root_handles_final_frozen {
        return Err(Revision3ExactSnapshotImportErrorV2::Publication(
            "staging root was not marked promoted".to_owned(),
        ));
    }
    revalidate_import_parent(guard)?;
    let retained_root = staging.file.as_ref().ok_or_else(|| {
        Revision3ExactSnapshotImportErrorV2::Publication(
            "final freeze root handle is absent".to_owned(),
        )
    })?;
    let final_file = open_import_named_child(&guard.parent, &guard.filename, true)?;
    if windows_file_identity(&final_file)
        .map_err(|error| Revision3ExactSnapshotImportErrorV2::Publication(error.to_string()))?
        != staging.identity
        || windows_file_identity(retained_root)
            .map_err(|error| Revision3ExactSnapshotImportErrorV2::Publication(error.to_string()))?
            != staging.identity
    {
        return Err(Revision3ExactSnapshotImportErrorV2::Publication(
            "final destination does not identify the retained staged Store".to_owned(),
        ));
    }
    match guard.parent.symlink_metadata(&staging.name) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Ok(_) => {
            return Err(Revision3ExactSnapshotImportErrorV2::Publication(
                "private staging name still exists after directory promotion".to_owned(),
            ));
        }
        Err(error) => {
            return Err(Revision3ExactSnapshotImportErrorV2::Publication(
                error.to_string(),
            ));
        }
    }
    ensure_safe_directory_chain(&guard.target)
        .map_err(|error| Revision3ExactSnapshotImportErrorV2::Publication(error.to_string()))?;
    Ok(())
}

#[cfg(windows)]
fn abort_import_staging(
    staging: &mut ImportStagingDirectory,
    primary: Revision3ExactSnapshotImportErrorV2,
) -> Revision3ExactSnapshotImportErrorV2 {
    match cleanup_import_staging(staging) {
        Ok(()) => primary,
        Err(error) => Revision3ExactSnapshotImportErrorV2::StagingCleanup {
            primary: Box::new(primary),
            cleanup: error.to_string(),
        },
    }
}

#[cfg(windows)]
fn rollback_created_import_object(
    file: &File,
    primary: Revision3ExactSnapshotImportErrorV2,
) -> Revision3ExactSnapshotImportErrorV2 {
    match delete_import_object_by_exact_handle(file) {
        Ok(()) => primary,
        Err(error) => Revision3ExactSnapshotImportErrorV2::StagingCleanup {
            primary: Box::new(primary),
            cleanup: error.to_string(),
        },
    }
}

#[cfg(windows)]
fn open_import_named_child_for_cleanup(
    parent: &cap_std::fs::Dir,
    name: &std::ffi::OsStr,
    directory: bool,
) -> io::Result<File> {
    use cap_std::fs::{OpenOptions as CapOpenOptions, OpenOptionsExt as _};
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE,
    };

    let mut options = CapOpenOptions::new();
    options
        .access_mode(
            DELETE
                | FILE_READ_ATTRIBUTES
                | if directory {
                    FILE_LIST_DIRECTORY | FILE_TRAVERSE
                } else {
                    0
                },
        )
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    0
                },
        );
    let file = parent.open_with(name, &options)?.into_std();
    let metadata = file.metadata()?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || metadata.is_dir() != directory
    {
        return Err(io::Error::other(
            "recorded staging object changed type or became a reparse point",
        ));
    }
    Ok(file)
}

#[cfg(windows)]
fn reopen_recorded_import_object_for_cleanup(
    staging: &ImportStagingDirectory,
    relative: &Path,
    directory: bool,
    expected_identity: WindowsFileIdentity,
) -> io::Result<File> {
    let parent_relative = relative.parent().unwrap_or_else(|| Path::new(""));
    let name = relative
        .file_name()
        .ok_or_else(|| io::Error::other("recorded staging object has no final component"))?;
    let parent = open_verified_staging_directory(staging, parent_relative)
        .map_err(|error| io::Error::other(error.to_string()))?;
    let file = open_import_named_child_for_cleanup(&parent, name, directory)?;
    if windows_file_identity(&file).map_err(|error| io::Error::other(error.to_string()))?
        != expected_identity
    {
        return Err(io::Error::other(
            "recorded staging object changed identity before cleanup",
        ));
    }
    Ok(file)
}

#[cfg(windows)]
fn cleanup_import_staging(staging: &mut ImportStagingDirectory) -> io::Result<()> {
    if staging.promoted {
        return Ok(());
    }
    let mut first_error = None;

    if staging.descendant_handles_released {
        for created in staging.files.iter().rev() {
            match reopen_recorded_import_object_for_cleanup(
                staging,
                &created.relative,
                false,
                created.identity,
            ) {
                Ok(file) => {
                    if let Err(error) = delete_import_object_by_exact_handle(&file) {
                        first_error.get_or_insert(error);
                    }
                }
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
    } else {
        for created in staging.files.drain(..).rev() {
            let Some(file) = created.file else {
                first_error.get_or_insert_with(|| {
                    io::Error::other("owned staging file handle disappeared before cleanup")
                });
                continue;
            };
            match windows_file_identity(&file) {
                Ok(identity) if identity == created.identity => {
                    if let Err(error) = delete_import_object_by_exact_handle(&file) {
                        first_error.get_or_insert(error);
                    }
                }
                Ok(_) => {
                    first_error.get_or_insert_with(|| {
                        io::Error::other("owned staging file changed identity before cleanup")
                    });
                }
                Err(error) => {
                    first_error.get_or_insert_with(|| io::Error::other(error.to_string()));
                }
            }
        }
        staging.file_indices.clear();
    }

    let mut directory_paths = staging.directories.keys().cloned().collect::<Vec<_>>();
    directory_paths.sort_by(|left, right| {
        right
            .components()
            .count()
            .cmp(&left.components().count())
            .then_with(|| right.cmp(left))
    });
    if staging.descendant_handles_released {
        for relative in &directory_paths {
            let created = &staging.directories[relative];
            match reopen_recorded_import_object_for_cleanup(
                staging,
                relative,
                true,
                created.identity,
            ) {
                Ok(file) => {
                    if let Err(error) = delete_import_object_by_exact_handle(&file) {
                        first_error.get_or_insert(error);
                    }
                }
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
    } else {
        for relative in directory_paths {
            let Some(mut created) = staging.directories.remove(&relative) else {
                first_error.get_or_insert_with(|| {
                    io::Error::other("owned staging directory inventory disappeared")
                });
                continue;
            };
            drop(created.directory.take());
            let Some(file) = created.file.take() else {
                first_error.get_or_insert_with(|| {
                    io::Error::other("owned staging directory handle disappeared before cleanup")
                });
                continue;
            };
            match windows_file_identity(&file) {
                Ok(identity) if identity == created.identity => {
                    if let Err(error) = delete_import_object_by_exact_handle(&file) {
                        first_error.get_or_insert(error);
                    }
                }
                Ok(_) => {
                    first_error.get_or_insert_with(|| {
                        io::Error::other("owned staging directory changed identity before cleanup")
                    });
                }
                Err(error) => {
                    first_error.get_or_insert_with(|| io::Error::other(error.to_string()));
                }
            }
        }
    }

    let Some(root_file) = staging.file.as_ref() else {
        return Err(io::Error::other(
            "owned staging root handle disappeared before cleanup",
        ));
    };
    if windows_file_identity(root_file).map_err(|error| io::Error::other(error.to_string()))?
        != staging.identity
    {
        first_error.get_or_insert_with(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "owned staging root changed identity before cleanup",
            )
        });
    } else if let Err(error) = delete_import_object_by_exact_handle(root_file) {
        first_error.get_or_insert(error);
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(windows)]
fn delete_import_object_by_exact_handle(file: &File) -> io::Result<()> {
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
    };

    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: the retained handle has DELETE access and names the exact owned object; no ambient
    // path or attacker-controlled traversal participates in cleanup.
    let deleted = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle() as HANDLE,
            FileDispositionInfo,
            (&disposition as *const FILE_DISPOSITION_INFO).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    };
    if deleted == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn import_materialization_io(error: io::Error) -> Revision3ExactSnapshotImportErrorV2 {
    Revision3ExactSnapshotImportErrorV2::Materialization(error.to_string())
}

fn open_untrusted_source(source: &Path) -> Result<File, Revision3ExactSnapshotInspectionErrorV2> {
    let source_spelling = source.to_str().ok_or_else(|| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "source contains non-Unicode path spelling".to_owned(),
        )
    })?;
    if source_spelling.len() > MAX_IMPORT_PATH_UTF8_BYTES_V2 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "source UTF-8 spelling exceeds the managed 32-KiB limit".to_owned(),
        ));
    }
    if !source.is_absolute() {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "source must be an absolute path".to_owned(),
        ));
    }
    if source
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("goremod"))
    {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "source must use the .goremod extension".to_owned(),
        ));
    }
    open_pinned_source_read_only(source)
}

#[cfg(windows)]
fn open_pinned_source_read_only(
    source: &Path,
) -> Result<File, Revision3ExactSnapshotInspectionErrorV2> {
    use cap_std::ambient_authority;
    use cap_std::fs::{Dir, OpenOptions as CapOpenOptions, OpenOptionsExt as _};
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
    };

    let parent_path = source.parent().ok_or_else(|| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "source has no parent directory".to_owned(),
        )
    })?;
    let filename = source.file_name().ok_or_else(|| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidSource("source has no filename".to_owned())
    })?;
    let parent =
        Dir::open_ambient_dir(parent_path, ambient_authority()).map_err(source_io_error)?;
    let parent_identity = windows_directory_identity(&parent)?;
    revalidate_windows_source_parent(&parent, parent_path, parent_identity)?;

    let mut options = CapOpenOptions::new();
    options
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .attributes(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = parent
        .open_with(filename, &options)
        .map_err(source_io_error)?
        .into_std();
    let metadata = file.metadata().map_err(source_io_error)?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "source is not a regular non-reparse file".to_owned(),
        ));
    }
    let identity = windows_file_identity(&file)?;
    if identity.links != 1 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            format!(
                "source has {} hard links; expected exactly one",
                identity.links
            ),
        ));
    }

    // `file` denies write/delete sharing. A second handle-relative open must therefore resolve to
    // the same final object, binding the pinned parent, final name, and returned handle without
    // another ambient file lookup.
    let confirm = parent
        .open_with(filename, &options)
        .map_err(source_io_error)?
        .into_std();
    if windows_file_identity(&confirm)? != identity
        || confirm.metadata().map_err(source_io_error)?.len() != metadata.len()
    {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "source identity changed while its pinned handle was opened".to_owned(),
        ));
    }
    revalidate_windows_source_parent(&parent, parent_path, parent_identity)?;
    Ok(file)
}

#[cfg(unix)]
fn open_pinned_source_read_only(
    _source: &Path,
) -> Result<File, Revision3ExactSnapshotInspectionErrorV2> {
    // O_NOFOLLOW pins an inode but does not exclude an untrusted existing writer. Hash bracketing
    // cannot prove all structured reads came from one immutable image if same-length contents can
    // be swapped and restored. Fail closed until inspection owns a sealed private snapshot.
    Err(Revision3ExactSnapshotInspectionErrorV2::UnsupportedPlatform)
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowsFileIdentity {
    volume: u64,
    index: u64,
    links: u64,
}

#[cfg(windows)]
fn windows_file_identity(
    file: &File,
) -> Result<WindowsFileIdentity, Revision3ExactSnapshotInspectionErrorV2> {
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: `file` is live and `info` is aligned writable storage for the exact ABI structure.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as HANDLE, info.as_mut_ptr()) } == 0
    {
        return Err(source_io_error(io::Error::last_os_error()));
    }
    // SAFETY: success above initialized the complete structure.
    let info = unsafe { info.assume_init() };
    Ok(WindowsFileIdentity {
        volume: u64::from(info.dwVolumeSerialNumber),
        index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        links: u64::from(info.nNumberOfLinks),
    })
}

#[cfg(windows)]
fn windows_directory_identity(
    directory: &cap_std::fs::Dir,
) -> Result<WindowsFileIdentity, Revision3ExactSnapshotInspectionErrorV2> {
    let metadata = directory.dir_metadata().map_err(source_io_error)?;
    if !metadata.is_dir() {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "source parent is not a directory".to_owned(),
        ));
    }
    let file = directory
        .try_clone()
        .map_err(source_io_error)?
        .into_std_file();
    windows_file_identity(&file)
}

#[cfg(windows)]
fn revalidate_windows_source_parent(
    pinned: &cap_std::fs::Dir,
    parent_path: &Path,
    expected: WindowsFileIdentity,
) -> Result<(), Revision3ExactSnapshotInspectionErrorV2> {
    use cap_std::ambient_authority;
    use cap_std::fs::Dir;

    ensure_safe_directory_chain(parent_path).map_err(|error| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidSource(error.to_string())
    })?;
    if windows_directory_identity(pinned)? != expected {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "pinned source parent identity changed".to_owned(),
        ));
    }
    let ambient =
        Dir::open_ambient_dir(parent_path, ambient_authority()).map_err(source_io_error)?;
    if windows_directory_identity(&ambient)? != expected {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "ambient source parent no longer names the pinned directory".to_owned(),
        ));
    }
    Ok(())
}

fn fixed_zip_timestamp() -> Result<DateTime, Revision3ExactSnapshotInspectionErrorV2> {
    DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).map_err(|error| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(format!(
            "fixed ZIP timestamp is invalid: {error:?}"
        ))
    })
}

fn read_first_manifest(
    archive: &mut ZipArchive<File>,
    timestamp: DateTime,
) -> Result<Vec<u8>, Revision3ExactSnapshotInspectionErrorV2> {
    let byte_len = {
        let entry = archive.by_index_raw(0).map_err(archive_error)?;
        validate_zip_entry_metadata(
            &entry,
            0,
            REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2,
            entry.size(),
            timestamp,
        )?;
        if entry.size() == 0 {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(
                "import manifest is empty".to_owned(),
            ));
        }
        if entry.size() > MAX_IMPORT_MANIFEST_BYTES_V2 as u64 {
            return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
                kind: "import manifest bytes",
                actual: entry.size(),
                limit: MAX_IMPORT_MANIFEST_BYTES_V2 as u64,
            });
        }
        entry.size()
    };
    let mut entry = archive.by_index(0).map_err(archive_error)?;
    let bytes = read_entry_bounded(&mut entry, byte_len, "import manifest bytes")?;
    if bytes.len() as u64 != byte_len {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(
            "manifest payload length differs from its ZIP metadata".to_owned(),
        ));
    }
    Ok(bytes)
}

fn validate_manifest_plan(
    manifest: &ExactSnapshotManifestV2,
    limits: &WorkingStoreLimits,
    manifest_bytes: u64,
) -> Result<ManifestPlan, Revision3ExactSnapshotInspectionErrorV2> {
    if manifest.format != ImportManifestFormatV2::ExactSnapshotV2
        || manifest.schema != ImportManifestSchemaV2
        || manifest.artifact_kind != ImportArtifactKindV2::PortableSnapshotRestorableCopy
        || manifest.restore_status != ImportRestoreStatusV2::Supported
    {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(
            "manifest authority marker is not the closed restorable V2 tuple".to_owned(),
        ));
    }
    if manifest.basis.project_revision > i64::MAX as u64 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(
            "project revision exceeds the closed signed response range".to_owned(),
        ));
    }
    validate_nonzero_seal(
        &manifest.basis.head.snapshot,
        revision3_total_snapshot_limit(limits),
        "restorable current snapshot",
    )
    .map_err(manifest_store_error)?;

    let declared_entry_count = (manifest.members.len() as u64).checked_add(1).ok_or(
        Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind: "archive entry count",
            actual: u64::MAX,
            limit: MAX_IMPORT_ARCHIVE_ENTRIES_V2,
        },
    )?;
    if declared_entry_count > MAX_IMPORT_ARCHIVE_ENTRIES_V2 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind: "archive entry count",
            actual: declared_entry_count,
            limit: MAX_IMPORT_ARCHIVE_ENTRIES_V2,
        });
    }

    let mut previous: Option<&str> = None;
    let mut folded = BTreeSet::new();
    let mut members_by_name = BTreeMap::new();
    let mut snapshots = BTreeMap::new();
    let mut entities = BTreeMap::new();
    let mut assets = BTreeMap::new();
    let mut snapshot_bytes = 0u64;
    let mut entity_bytes = 0u64;
    let mut asset_bytes = 0u64;
    let mut project_seen = false;
    let mut head_seen = false;

    for member in &manifest.members {
        if member.relative_name.is_empty()
            || member.relative_name == REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2
            || member.byte_len == 0
        {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(
                "manifest contains an empty, zero-length, or self member".to_owned(),
            ));
        }
        if previous.is_some_and(|value| value >= member.relative_name.as_str()) {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(
                "manifest members are not strictly sorted".to_owned(),
            ));
        }
        if !folded.insert(member.relative_name.to_ascii_lowercase()) {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(
                "manifest member names collide case-insensitively".to_owned(),
            ));
        }
        previous = Some(&member.relative_name);
        let kind = parse_member_name(&member.relative_name)?;
        match kind {
            ImportMemberKind::Project => {
                if project_seen || member.byte_len > MAX_PROJECT_JSON_BYTES as u64 {
                    return Err(member_limit_or_duplicate(
                        "project bytes",
                        member.byte_len,
                        MAX_PROJECT_JSON_BYTES as u64,
                        project_seen,
                    ));
                }
                project_seen = true;
            }
            ImportMemberKind::StoreHead => {
                if head_seen || member.byte_len > limits.max_head_bytes as u64 {
                    return Err(member_limit_or_duplicate(
                        "Store head bytes",
                        member.byte_len,
                        limits.max_head_bytes as u64,
                        head_seen,
                    ));
                }
                head_seen = true;
            }
            ImportMemberKind::Snapshot(digest) => {
                if digest != member.sha256 {
                    return Err(path_seal_mismatch(&member.relative_name));
                }
                if member.byte_len > revision3_total_snapshot_limit(limits) as u64 {
                    return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
                        kind: "snapshot object bytes",
                        actual: member.byte_len,
                        limit: revision3_total_snapshot_limit(limits) as u64,
                    });
                }
                if snapshots.len() >= limits.max_entities {
                    return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
                        kind: "snapshot object count",
                        actual: snapshots.len() as u64 + 1,
                        limit: limits.max_entities as u64,
                    });
                }
                snapshot_bytes = checked_import_sum(
                    "aggregate snapshot bytes",
                    snapshot_bytes,
                    member.byte_len,
                    limits.max_referenced_entity_bytes,
                )?;
                snapshots.insert(member.relative_name.clone(), ());
            }
            ImportMemberKind::Entity(_, digest) => {
                if digest != member.sha256 {
                    return Err(path_seal_mismatch(&member.relative_name));
                }
                if member.byte_len > limits.max_entity_bytes as u64 {
                    return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
                        kind: "entity object bytes",
                        actual: member.byte_len,
                        limit: limits.max_entity_bytes as u64,
                    });
                }
                if entities.len() >= limits.max_entities {
                    return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
                        kind: "entity object count",
                        actual: entities.len() as u64 + 1,
                        limit: limits.max_entities as u64,
                    });
                }
                entity_bytes = checked_import_sum(
                    "aggregate entity bytes",
                    entity_bytes,
                    member.byte_len,
                    limits.max_referenced_entity_bytes,
                )?;
                entities.insert(member.relative_name.clone(), ());
            }
            ImportMemberKind::Asset(digest) => {
                if digest != member.sha256 {
                    return Err(path_seal_mismatch(&member.relative_name));
                }
                if assets.len() >= limits.max_assets {
                    return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
                        kind: "asset object count",
                        actual: assets.len() as u64 + 1,
                        limit: limits.max_assets as u64,
                    });
                }
                asset_bytes = checked_import_sum(
                    "aggregate asset bytes",
                    asset_bytes,
                    member.byte_len,
                    limits.max_referenced_asset_bytes,
                )?;
                assets.insert(member.relative_name.clone(), ());
            }
        }
        let inspected = InspectedMember {
            index: 0,
            relative_name: member.relative_name.clone(),
            seal: ContentSeal {
                byte_len: member.byte_len,
                sha256: member.sha256,
            },
            kind,
        };
        if members_by_name
            .insert(member.relative_name.clone(), inspected)
            .is_some()
        {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(
                "manifest repeats a member".to_owned(),
            ));
        }
    }
    if !project_seen || !head_seen {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(
            "manifest must seal exactly one project and one fixed Store head".to_owned(),
        ));
    }

    let mut archive_order = Vec::with_capacity(manifest.members.len() + 1);
    archive_order.push(REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2.to_owned());
    archive_order.push(PROJECT_FILE.to_owned());
    archive_order.push(STORE_HEAD_MEMBER.to_owned());
    archive_order.extend(snapshots.keys().cloned());
    archive_order.extend(entities.keys().cloned());
    archive_order.extend(assets.keys().cloned());
    if archive_order.len() != manifest.members.len() + 1 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(
            "manifest member classification is not closed".to_owned(),
        ));
    }
    for (index, name) in archive_order.iter().enumerate().skip(1) {
        members_by_name
            .get_mut(name)
            .expect("classified manifest member")
            .index = index;
    }

    let mut uncompressed_bytes = manifest_bytes;
    for member in &manifest.members {
        uncompressed_bytes = uncompressed_bytes.checked_add(member.byte_len).ok_or(
            Revision3ExactSnapshotInspectionErrorV2::Limit {
                kind: "archive uncompressed bytes",
                actual: u64::MAX,
                limit: MAX_IMPORT_ARCHIVE_BYTES_V2,
            },
        )?;
    }
    if uncompressed_bytes > MAX_IMPORT_ARCHIVE_BYTES_V2 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind: "archive uncompressed bytes",
            actual: uncompressed_bytes,
            limit: MAX_IMPORT_ARCHIVE_BYTES_V2,
        });
    }

    Ok(ManifestPlan {
        members_by_name,
        archive_order,
        closure: Revision3ExactSnapshotInspectionClosureV2 {
            snapshot_objects: snapshots.len() as u64,
            entity_objects: entities.len() as u64,
            asset_objects: assets.len() as u64,
            archive_entries: declared_entry_count,
            uncompressed_bytes,
        },
    })
}

fn member_limit_or_duplicate(
    kind: &'static str,
    actual: u64,
    limit: u64,
    duplicate: bool,
) -> Revision3ExactSnapshotInspectionErrorV2 {
    if duplicate {
        Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(format!(
            "manifest repeats the {kind} member"
        ))
    } else {
        Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind,
            actual,
            limit,
        }
    }
}

fn path_seal_mismatch(name: &str) -> Revision3ExactSnapshotInspectionErrorV2 {
    Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(format!(
        "content-addressed member {name:?} disagrees with its declared SHA-256"
    ))
}

fn checked_import_sum(
    kind: &'static str,
    current: u64,
    addition: u64,
    limit: u64,
) -> Result<u64, Revision3ExactSnapshotInspectionErrorV2> {
    let actual = current.saturating_add(addition);
    if actual > limit {
        Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind,
            actual,
            limit,
        })
    } else {
        Ok(actual)
    }
}

fn parse_member_name(
    name: &str,
) -> Result<ImportMemberKind, Revision3ExactSnapshotInspectionErrorV2> {
    if name == PROJECT_FILE {
        return Ok(ImportMemberKind::Project);
    }
    if name == STORE_HEAD_MEMBER {
        return Ok(ImportMemberKind::StoreHead);
    }
    if let Some(rest) = name.strip_prefix("store/snapshots/sha256/") {
        let (shard, file) = rest
            .split_once('/')
            .ok_or_else(|| invalid_member_path(name))?;
        if shard.len() != 2 || file.len() != 67 || !file.ends_with(".json") {
            return Err(invalid_member_path(name));
        }
        let digest_text = format!("{shard}{}", &file[..62]);
        let digest = digest_text
            .parse::<Sha256Digest>()
            .map_err(|_| invalid_member_path(name))?;
        if snapshot_member_name(digest) != name {
            return Err(invalid_member_path(name));
        }
        return Ok(ImportMemberKind::Snapshot(digest));
    }
    if let Some(rest) = name.strip_prefix("store/entities/") {
        let mut parts = rest.split('/');
        let id_shard = parts.next().ok_or_else(|| invalid_member_path(name))?;
        let id_tail = parts.next().ok_or_else(|| invalid_member_path(name))?;
        let file = parts.next().ok_or_else(|| invalid_member_path(name))?;
        if parts.next().is_some()
            || id_shard.len() != 2
            || id_tail.len() != 30
            || file.len() != 69
            || !file.ends_with(".json")
        {
            return Err(invalid_member_path(name));
        }
        let id = format!("{id_shard}{id_tail}")
            .parse::<EntityId>()
            .map_err(|_| invalid_member_path(name))?;
        let digest = file[..64]
            .parse::<Sha256Digest>()
            .map_err(|_| invalid_member_path(name))?;
        if entity_member_name(id, digest) != name {
            return Err(invalid_member_path(name));
        }
        return Ok(ImportMemberKind::Entity(id, digest));
    }
    if let Some(rest) = name.strip_prefix("store/assets/sha256/") {
        let (shard, tail) = rest
            .split_once('/')
            .ok_or_else(|| invalid_member_path(name))?;
        if shard.len() != 2 || tail.len() != 62 || tail.contains('/') {
            return Err(invalid_member_path(name));
        }
        let digest = format!("{shard}{tail}")
            .parse::<Sha256Digest>()
            .map_err(|_| invalid_member_path(name))?;
        if asset_member_name(digest) != name {
            return Err(invalid_member_path(name));
        }
        return Ok(ImportMemberKind::Asset(digest));
    }
    Err(invalid_member_path(name))
}

fn invalid_member_path(name: &str) -> Revision3ExactSnapshotInspectionErrorV2 {
    Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(format!(
        "member path {name:?} is outside the closed portable Store grammar"
    ))
}

fn snapshot_member_name(digest: Sha256Digest) -> String {
    let hex = digest.to_string();
    format!("store/snapshots/sha256/{}/{}.json", &hex[..2], &hex[2..])
}

fn entity_member_name(id: EntityId, digest: Sha256Digest) -> String {
    let id = id.to_string();
    format!("store/entities/{}/{}/{}.json", &id[..2], &id[2..], digest)
}

fn asset_member_name(digest: Sha256Digest) -> String {
    let hex = digest.to_string();
    format!("store/assets/sha256/{}/{}", &hex[..2], &hex[2..])
}

fn verify_and_hash_archive_members(
    archive: &mut ZipArchive<File>,
    plan: &ManifestPlan,
    manifest_seal: &ContentSeal,
    timestamp: DateTime,
) -> Result<Vec<RawZipMemberExpectation>, Revision3ExactSnapshotInspectionErrorV2> {
    let mut raw_members = Vec::with_capacity(plan.archive_order.len());
    for (index, expected_name) in plan.archive_order.iter().enumerate() {
        let expected_seal = if index == 0 {
            manifest_seal
        } else {
            &plan
                .members_by_name
                .get(expected_name)
                .expect("planned manifest member")
                .seal
        };
        let crc32 = {
            let entry = archive.by_index_raw(index).map_err(archive_error)?;
            validate_zip_entry_metadata(
                &entry,
                index,
                expected_name,
                expected_seal.byte_len,
                timestamp,
            )?;
            entry.crc32()
        };
        let mut entry = archive.by_index(index).map_err(archive_error)?;
        stream_verify_entry(&mut entry, expected_seal, index)?;
        raw_members.push(RawZipMemberExpectation {
            name: expected_name.as_bytes().to_vec(),
            byte_len: expected_seal.byte_len,
            crc32,
        });
    }
    Ok(raw_members)
}

fn validate_zip_entry_metadata(
    entry: &zip::read::ZipFile<'_>,
    index: usize,
    expected_name: &str,
    expected_len: u64,
    timestamp: DateTime,
) -> Result<(), Revision3ExactSnapshotInspectionErrorV2> {
    if entry.name_raw() != expected_name.as_bytes()
        || entry.name() != expected_name
        || !entry.comment().is_empty()
        || entry.is_dir()
        || entry.encrypted()
        || entry.compression() != CompressionMethod::Stored
        || entry.size() != expected_len
        || entry.compressed_size() != expected_len
        || entry.last_modified() != Some(timestamp)
        || entry.unix_mode().map(|mode| mode & 0o777) != Some(ZIP_FILE_MODE)
    {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(
            format!("entry {index} metadata, raw name, or deterministic order differs"),
        ));
    }
    Ok(())
}

fn read_entry_bounded<R: Read>(
    entry: &mut R,
    limit: u64,
    kind: &'static str,
) -> Result<Vec<u8>, Revision3ExactSnapshotInspectionErrorV2> {
    let capacity = usize::try_from(limit.min(16 * 1024 * 1024)).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    let mut total = 0u64;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        let count = entry.read(&mut buffer).map_err(archive_io_error)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or(
            Revision3ExactSnapshotInspectionErrorV2::Limit {
                kind,
                actual: u64::MAX,
                limit,
            },
        )?;
        if total > limit {
            return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
                kind,
                actual: total,
                limit,
            });
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    Ok(bytes)
}

fn stream_verify_entry<R: Read>(
    entry: &mut R,
    seal: &ContentSeal,
    index: usize,
) -> Result<(), Revision3ExactSnapshotInspectionErrorV2> {
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        let count = entry.read(&mut buffer).map_err(archive_io_error)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or_else(|| {
            Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(format!(
                "entry {index} payload length overflowed"
            ))
        })?;
        if total > seal.byte_len {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(
                format!("entry {index} payload exceeds its declared seal"),
            ));
        }
        hasher.update(&buffer[..count]);
    }
    let digest = Sha256Digest::from_bytes(hasher.finalize().into());
    if total != seal.byte_len || digest != seal.sha256 {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(
            format!("entry {index} payload differs from its declared seal"),
        ));
    }
    Ok(())
}

fn read_sealed_member(
    archive: &mut ZipArchive<File>,
    member: &InspectedMember,
    limit: u64,
    kind: &'static str,
) -> Result<Vec<u8>, Revision3ExactSnapshotInspectionErrorV2> {
    if member.seal.byte_len > limit {
        return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind,
            actual: member.seal.byte_len,
            limit,
        });
    }
    let mut entry = archive.by_index(member.index).map_err(archive_error)?;
    let bytes = read_entry_bounded(&mut entry, member.seal.byte_len, kind)?;
    if seal_bytes(&bytes) != member.seal {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(
            format!(
                "member {:?} changed between authenticated reads",
                member.relative_name
            ),
        ));
    }
    Ok(bytes)
}

fn reopen_store_closure(
    archive: &mut ZipArchive<File>,
    manifest: &ExactSnapshotManifestV2,
    plan: &ManifestPlan,
    limits: &WorkingStoreLimits,
) -> Result<(WorkingHead, ProjectRevision3), Revision3ExactSnapshotInspectionErrorV2> {
    let head_member = required_member(plan, STORE_HEAD_MEMBER)?;
    let head_bytes = read_sealed_member(
        archive,
        head_member,
        limits.max_head_bytes as u64,
        "Store head bytes",
    )?;
    let head: WorkingHead =
        parse_canonical_json(&head_bytes, "restorable exact snapshot Store head")
            .map_err(closure_model_error)?;
    if head != manifest.basis.head {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
            "fixed Store head differs from the manifest basis head".to_owned(),
        ));
    }
    validate_nonzero_seal(
        &head.snapshot,
        revision3_total_snapshot_limit(limits),
        "restorable current snapshot",
    )
    .map_err(closure_store_error)?;

    let mut pending = BTreeSet::new();
    let mut needs = BTreeMap::<Sha256Digest, SnapshotNeed>::new();
    enqueue_snapshot_need(
        &mut pending,
        &mut needs,
        head.snapshot.clone(),
        None,
        limits,
    )?;

    let mut referenced_names =
        BTreeSet::from([PROJECT_FILE.to_owned(), STORE_HEAD_MEMBER.to_owned()]);
    // Historical Projects can each contain a maximal entity map. Retaining every materialized
    // Project until the end would multiply heap use by the snapshot count, so only the current
    // Project survives its iteration; historical proof state is one compact identity tuple.
    let mut opened_tuples = BTreeMap::<Sha256Digest, SnapshotTuple>::new();
    let mut current_project = None;
    let mut voice_metadata = BTreeMap::<Sha256Digest, OggMetadata>::new();
    let mut reopen_work = FullReopenWorkBudget::closed_v2();

    while let Some(digest) = pending.pop_first() {
        let need = needs.get(&digest).cloned().expect("pending snapshot need");
        let snapshot_name = snapshot_member_name(digest);
        let snapshot_member = required_member(plan, &snapshot_name)?;
        if snapshot_member.kind != ImportMemberKind::Snapshot(digest)
            || snapshot_member.seal != need.seal
        {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                format!("snapshot {digest} is absent or has a conflicting seal"),
            ));
        }
        referenced_names.insert(snapshot_name);
        let snapshot_bytes = read_sealed_member(
            archive,
            snapshot_member,
            revision3_total_snapshot_limit(limits) as u64,
            "revision-3 snapshot bytes",
        )?;
        let snapshot: Revision3SnapshotManifest =
            parse_canonical_json(&snapshot_bytes, "restorable revision-3 snapshot")
                .map_err(closure_model_error)?;
        validate_archive_snapshot_manifest(&snapshot, limits)?;
        if let Some(expected) = &need.expected_tuple {
            if snapshot.project_id != expected.project_id
                || snapshot.revision != expected.project_revision
                || snapshot.target != expected.target
            {
                return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                    format!("snapshot {digest} disagrees with its retained-history tuple"),
                ));
            }
        }

        // Mirror the exporter's exact Full-reopen preflight before the first entity or asset loop.
        // Shared members are intentionally charged once per snapshot reopen: the implementation
        // reparses entities and reiterates asset indexes, and this prevents a small unique member
        // set from amplifying into unbounded CPU, I/O, or transient allocation. Matching the
        // writer's formula and caps also preserves V2 writer-to-reader compatibility.
        reopen_work.charge_snapshot(&need.seal, &snapshot)?;
        let mut entities = BTreeMap::new();
        for (id, seal) in &snapshot.entities {
            let name = entity_member_name(*id, seal.sha256);
            let member = required_member(plan, &name)?;
            if member.kind != ImportMemberKind::Entity(*id, seal.sha256) || &member.seal != seal {
                return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                    format!("entity shard {id} is absent or has a conflicting seal"),
                ));
            }
            referenced_names.insert(name);
            let bytes = read_sealed_member(
                archive,
                member,
                limits.max_entity_bytes as u64,
                "revision-3 entity bytes",
            )?;
            let entity: Revision3Entity =
                parse_canonical_json(&bytes, "restorable revision-3 entity")
                    .map_err(closure_model_error)?;
            if entity.id != *id {
                return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                    format!(
                        "entity shard {id} contains the different embedded id {}",
                        entity.id
                    ),
                ));
            }
            entities.insert(*id, entity);
        }

        for (asset_digest, meta) in &snapshot.asset_store.assets {
            let name = asset_member_name(*asset_digest);
            let member = required_member(plan, &name)?;
            let seal = ContentSeal {
                byte_len: meta.byte_len,
                sha256: *asset_digest,
            };
            if member.kind != ImportMemberKind::Asset(*asset_digest) || member.seal != seal {
                return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                    format!("asset {asset_digest} is absent or has a conflicting seal"),
                ));
            }
            referenced_names.insert(name);
        }

        let snapshot_tuple = SnapshotTuple {
            project_id: snapshot.project_id,
            project_revision: snapshot.revision,
            target: snapshot.target.clone(),
        };
        let is_current = digest == head.snapshot.sha256;
        let current_history = if is_current {
            snapshot.history.clone()
        } else {
            None
        };
        let project = snapshot.into_project(entities);
        validate_revision3_persistability(&project, limits).map_err(closure_store_error)?;
        verify_archive_voice_metadata(archive, plan, &project, limits, &mut voice_metadata)?;

        for entity in project.entities.values() {
            if let Revision3EntityPayload::QuestDraft(quest) = &entity.payload {
                enqueue_snapshot_need(
                    &mut pending,
                    &mut needs,
                    quest.input.collision_catalog.basis_snapshot.clone(),
                    None,
                    limits,
                )?;
            }
        }

        if is_current {
            if project.project_id != manifest.basis.project_id
                || project.revision != manifest.basis.project_revision
            {
                return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                    "current snapshot disagrees with the manifest basis tuple".to_owned(),
                ));
            }
            if let Some(history) = &current_history {
                for retained in &history.prior_checkpoints {
                    enqueue_snapshot_need(
                        &mut pending,
                        &mut needs,
                        retained.head.snapshot.clone(),
                        Some(SnapshotTuple {
                            project_id: retained.project_id,
                            project_revision: retained.project_revision,
                            target: retained.target.clone(),
                        }),
                        limits,
                    )?;
                }
            }
            if current_project.replace(project).is_some() {
                return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                    "current snapshot was reopened more than once".to_owned(),
                ));
            }
        }
        if opened_tuples.insert(digest, snapshot_tuple).is_some() {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                "snapshot was fully reopened more than once".to_owned(),
            ));
        }
    }

    let current_project = current_project.ok_or_else(|| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
            "current snapshot is absent from its own closure".to_owned(),
        )
    })?;
    let project_member = required_member(plan, PROJECT_FILE)?;
    let project_bytes = read_sealed_member(
        archive,
        project_member,
        MAX_PROJECT_JSON_BYTES as u64,
        "project bytes",
    )?;
    let project_text = std::str::from_utf8(&project_bytes).map_err(|_| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidClosure("project is not UTF-8".to_owned())
    })?;
    let project_copy = ProjectRevision3::from_json(project_text).map_err(|error| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(format!(
            "project is invalid: {error}"
        ))
    })?;
    if project_copy != current_project {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
            "project differs from the fully reopened current Store project".to_owned(),
        ));
    }

    let declared_names = plan
        .members_by_name
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if referenced_names != declared_names {
        let unreferenced = declared_names.difference(&referenced_names).count();
        let missing = referenced_names.difference(&declared_names).count();
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
            format!(
                "sealed member set is not the exact reachable Store closure ({unreferenced} unreferenced, {missing} missing)"
            ),
        ));
    }

    // Every retained-history tuple was checked while its snapshot was reopened. Compact tuples,
    // rather than complete historical Projects, prove all queued digests reached full validation.
    if opened_tuples.len() != needs.len() {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
            "not every required snapshot reached a complete reopen".to_owned(),
        ));
    }
    for (digest, need) in &needs {
        let Some(expected) = &need.expected_tuple else {
            continue;
        };
        let opened = opened_tuples.get(digest).expect("count and keys proved");
        if opened.project_id != expected.project_id
            || opened.project_revision != expected.project_revision
            || opened.target != expected.target
        {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                format!("snapshot {digest} disagrees with its retained-history tuple"),
            ));
        }
    }
    Ok((head, current_project))
}

fn required_member<'a>(
    plan: &'a ManifestPlan,
    name: &str,
) -> Result<&'a InspectedMember, Revision3ExactSnapshotInspectionErrorV2> {
    plan.members_by_name.get(name).ok_or_else(|| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(format!(
            "required member {name:?} is absent"
        ))
    })
}

fn enqueue_snapshot_need(
    pending: &mut BTreeSet<Sha256Digest>,
    needs: &mut BTreeMap<Sha256Digest, SnapshotNeed>,
    seal: ContentSeal,
    expected_tuple: Option<SnapshotTuple>,
    limits: &WorkingStoreLimits,
) -> Result<(), Revision3ExactSnapshotInspectionErrorV2> {
    validate_nonzero_seal(
        &seal,
        revision3_total_snapshot_limit(limits),
        "restorable snapshot",
    )
    .map_err(closure_store_error)?;
    if let Some(existing) = needs.get_mut(&seal.sha256) {
        if existing.seal.byte_len != seal.byte_len {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                format!(
                    "snapshot {} has conflicting lengths {} and {}",
                    seal.sha256, existing.seal.byte_len, seal.byte_len
                ),
            ));
        }
        match (&existing.expected_tuple, expected_tuple) {
            (Some(left), Some(right)) if left != &right => {
                return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                    format!(
                        "snapshot {} has conflicting retained-history tuples",
                        seal.sha256
                    ),
                ));
            }
            (None, Some(tuple)) => existing.expected_tuple = Some(tuple),
            _ => {}
        }
        return Ok(());
    }
    if needs.len() >= limits.max_entities {
        return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind: "reachable snapshot count",
            actual: needs.len() as u64 + 1,
            limit: limits.max_entities as u64,
        });
    }
    pending.insert(seal.sha256);
    needs.insert(
        seal.sha256,
        SnapshotNeed {
            seal,
            expected_tuple,
        },
    );
    Ok(())
}

fn validate_archive_snapshot_manifest(
    snapshot: &Revision3SnapshotManifest,
    limits: &WorkingStoreLimits,
) -> Result<(), Revision3ExactSnapshotInspectionErrorV2> {
    let mut base = snapshot.clone();
    base.history = None;
    let base_bytes = canonical_json(&base).map_err(closure_store_error)?;
    if base_bytes.len() > limits.max_snapshot_bytes {
        return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind: "revision-3 base snapshot bytes",
            actual: base_bytes.len() as u64,
            limit: limits.max_snapshot_bytes as u64,
        });
    }
    revision3_history::validate_revision3_checkpoint_history_v1(snapshot, limits)
        .map_err(closure_store_error)?;
    if snapshot.entities.len() > limits.max_entities {
        return Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
            kind: "snapshot entity count",
            actual: snapshot.entities.len() as u64,
            limit: limits.max_entities as u64,
        });
    }
    validate_revision3_asset_index_persistability(&snapshot.asset_store, limits)
        .map_err(closure_store_error)?;
    let mut total_entity_bytes = 0u64;
    for seal in snapshot.entities.values() {
        validate_nonzero_seal(seal, limits.max_entity_bytes, "revision-3 entity")
            .map_err(closure_store_error)?;
        total_entity_bytes = checked_import_sum(
            "aggregate referenced entity bytes",
            total_entity_bytes,
            seal.byte_len,
            limits.max_referenced_entity_bytes,
        )?;
    }
    Ok(())
}

fn verify_archive_voice_metadata(
    archive: &mut ZipArchive<File>,
    plan: &ManifestPlan,
    project: &ProjectRevision3,
    limits: &WorkingStoreLimits,
    validated: &mut BTreeMap<Sha256Digest, OggMetadata>,
) -> Result<(), Revision3ExactSnapshotInspectionErrorV2> {
    for (entity_id, entity) in &project.entities {
        let Revision3EntityPayload::VoiceTake(take) = &entity.payload else {
            continue;
        };
        let indexed = project
            .asset_store
            .assets
            .get(&take.asset.sha256)
            .ok_or_else(|| {
                Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(format!(
                    "voice take {entity_id} references absent asset {}",
                    take.asset.sha256
                ))
            })?;
        if indexed.byte_len != take.asset.byte_len {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                format!(
                    "voice take {entity_id} asset {} declares {} bytes but the index declares {}",
                    take.asset.sha256, take.asset.byte_len, indexed.byte_len
                ),
            ));
        }
        if indexed.media_type != "audio/ogg" {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                format!(
                    "voice take {entity_id} asset {} is not indexed as audio/ogg",
                    take.asset.sha256
                ),
            ));
        }
        let actual = if let Some(actual) = validated.get(&take.asset.sha256) {
            actual.clone()
        } else {
            let member = required_member(plan, &asset_member_name(take.asset.sha256))?;
            let bytes =
                read_sealed_member(archive, member, limits.max_ogg_bytes as u64, "Ogg bytes")?;
            let actual = derive_archive_ogg_metadata(&bytes, limits)?;
            validated.insert(take.asset.sha256, actual.clone());
            actual
        };
        let declared = revision2_ogg_metadata_as_revision1(&take.ogg);
        if declared != actual {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                format!(
                    "voice take {entity_id} Ogg metadata for asset {} differs from the authenticated blob",
                    take.asset.sha256
                ),
            ));
        }
    }
    Ok(())
}

fn derive_archive_ogg_metadata(
    bytes: &[u8],
    limits: &WorkingStoreLimits,
) -> Result<OggMetadata, Revision3ExactSnapshotInspectionErrorV2> {
    let info = gore_vo::validate_ogg(
        bytes,
        &gore_vo::Limits {
            max_ogg_bytes: limits.max_ogg_bytes,
            ..gore_vo::Limits::default()
        },
    )
    .map_err(|error| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(format!(
            "invalid Ogg asset: {error}"
        ))
    })?;
    let (codec, channels, sample_rate) = match info.codec {
        gore_vo::OggCodec::Vorbis {
            channels,
            sample_rate,
        } => (OggCodec::Vorbis, channels, sample_rate),
        gore_vo::OggCodec::Opus { channels, .. } => (OggCodec::Opus, channels, 48_000),
        gore_vo::OggCodec::Unknown => {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                "Ogg codec is not Vorbis or Opus".to_owned(),
            ));
        }
    };
    Ok(OggMetadata {
        codec,
        channels,
        sample_rate,
        pages: u32::try_from(info.pages).map_err(|_| {
            Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                "Ogg page count does not fit u32".to_owned(),
            )
        })?,
        logical_streams: u32::try_from(info.logical_streams).map_err(|_| {
            Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
                "Ogg stream count does not fit u32".to_owned(),
            )
        })?,
    })
}

fn verify_exact_zip_layout(
    file: &File,
    byte_len: u64,
    members: &[RawZipMemberExpectation],
) -> Result<(), Revision3ExactSnapshotInspectionErrorV2> {
    const LOCAL_FIXED_BYTES: u64 = 30;
    const CENTRAL_FIXED_BYTES: u64 = 46;
    const FOOTER_BYTES: u64 = 98;
    const ZIP64_THRESHOLD: u64 = u32::MAX as u64;

    let mut local_offset = 0u64;
    let mut local_offsets = Vec::with_capacity(members.len());
    for (index, member) in members.iter().enumerate() {
        let name_len = u16::try_from(member.name.len()).map_err(|_| {
            layout_error(format!("member {index} name exceeds the closed ZIP range"))
        })?;
        let extra = exact_zip64_extra(member.byte_len, local_offset);
        let extra_len = u16::try_from(extra.len()).expect("closed ZIP64 extra is tiny");
        let mut header = [0u8; LOCAL_FIXED_BYTES as usize];
        read_exact_file_at(file, &mut header, local_offset).map_err(archive_io_error)?;
        if le_u32(&header, 0) != 0x0403_4b50
            || le_u16(&header, 4) != ZIP_VERSION_45
            || le_u16(&header, 6) != 0
            || le_u16(&header, 8) != 0
            || le_u16(&header, 10) != ZIP_DOS_EPOCH_TIME
            || le_u16(&header, 12) != ZIP_DOS_EPOCH_DATE
            || le_u32(&header, 14) != member.crc32
            || le_u32(&header, 18) != u32::MAX
            || le_u32(&header, 22) != u32::MAX
            || le_u16(&header, 26) != name_len
            || le_u16(&header, 28) != extra_len
        {
            return Err(layout_error(format!(
                "member {index} local header is outside the exact ZIP64 dialect"
            )));
        }
        let mut variable = vec![0u8; member.name.len() + extra.len()];
        read_exact_file_at(file, &mut variable, local_offset + LOCAL_FIXED_BYTES)
            .map_err(archive_io_error)?;
        if variable[..member.name.len()] != member.name || variable[member.name.len()..] != extra {
            return Err(layout_error(format!(
                "member {index} local raw name or ZIP64 extra differs"
            )));
        }
        local_offsets.push(local_offset);
        local_offset = checked_layout_add(local_offset, LOCAL_FIXED_BYTES, "local header")?;
        local_offset = checked_layout_add(local_offset, u64::from(name_len), "local name")?;
        local_offset = checked_layout_add(local_offset, u64::from(extra_len), "local extra")?;
        local_offset = checked_layout_add(local_offset, member.byte_len, "member payload")?;
    }

    let central_offset = local_offset;
    let mut central_cursor = central_offset;
    for (index, (member, local_offset)) in members.iter().zip(local_offsets).enumerate() {
        let name_len = u16::try_from(member.name.len()).expect("validated above");
        let extra = exact_zip64_extra(member.byte_len, local_offset);
        let extra_len = u16::try_from(extra.len()).expect("closed ZIP64 extra is tiny");
        let mut header = [0u8; CENTRAL_FIXED_BYTES as usize];
        read_exact_file_at(file, &mut header, central_cursor).map_err(archive_io_error)?;
        if le_u32(&header, 0) != 0x0201_4b50
            || le_u16(&header, 4) != ZIP_UNIX_VERSION_45
            || le_u16(&header, 6) != ZIP_VERSION_45
            || le_u16(&header, 8) != 0
            || le_u16(&header, 10) != 0
            || le_u16(&header, 12) != ZIP_DOS_EPOCH_TIME
            || le_u16(&header, 14) != ZIP_DOS_EPOCH_DATE
            || le_u32(&header, 16) != member.crc32
            || le_u32(&header, 20) != u32::MAX
            || le_u32(&header, 24) != u32::MAX
            || le_u16(&header, 28) != name_len
            || le_u16(&header, 30) != extra_len
            || le_u16(&header, 32) != 0
            || le_u16(&header, 34) != 0
            || le_u16(&header, 36) != 0
            || le_u32(&header, 38) != ZIP_EXTERNAL_FILE_ATTRIBUTES
            || le_u32(&header, 42) != local_offset.min(ZIP64_THRESHOLD) as u32
        {
            return Err(layout_error(format!(
                "member {index} central header is outside the exact ZIP64 dialect"
            )));
        }
        let mut variable = vec![0u8; member.name.len() + extra.len()];
        read_exact_file_at(file, &mut variable, central_cursor + CENTRAL_FIXED_BYTES)
            .map_err(archive_io_error)?;
        if variable[..member.name.len()] != member.name || variable[member.name.len()..] != extra {
            return Err(layout_error(format!(
                "member {index} central raw name or ZIP64 extra differs"
            )));
        }
        central_cursor = checked_layout_add(central_cursor, CENTRAL_FIXED_BYTES, "central header")?;
        central_cursor = checked_layout_add(central_cursor, u64::from(name_len), "central name")?;
        central_cursor = checked_layout_add(central_cursor, u64::from(extra_len), "central extra")?;
    }
    let central_size = central_cursor - central_offset;
    let expected_len = checked_layout_add(central_cursor, FOOTER_BYTES, "ZIP64 footer")?;
    if byte_len != expected_len {
        return Err(layout_error(
            "archive has gaps, trailing bytes, or a non-exact footer".to_owned(),
        ));
    }
    verify_exact_zip64_footer(
        file,
        central_cursor,
        central_offset,
        central_size,
        members.len() as u64,
    )
}

fn verify_exact_zip64_footer(
    file: &File,
    footer_offset: u64,
    central_offset: u64,
    central_size: u64,
    entry_count: u64,
) -> Result<(), Revision3ExactSnapshotInspectionErrorV2> {
    const ZIP64_EOCD_BYTES: usize = 56;
    const ZIP64_LOCATOR_BYTES: usize = 20;
    const LEGACY_EOCD_BYTES: usize = 22;
    let mut footer = [0u8; ZIP64_EOCD_BYTES + ZIP64_LOCATOR_BYTES + LEGACY_EOCD_BYTES];
    read_exact_file_at(file, &mut footer, footer_offset).map_err(archive_io_error)?;
    let locator = ZIP64_EOCD_BYTES;
    let legacy = locator + ZIP64_LOCATOR_BYTES;
    if le_u32(&footer, 0) != 0x0606_4b50
        || le_u64(&footer, 4) != 44
        || le_u16(&footer, 12) != ZIP_VERSION_45
        || le_u16(&footer, 14) != ZIP_VERSION_45
        || le_u32(&footer, 16) != 0
        || le_u32(&footer, 20) != 0
        || le_u64(&footer, 24) != entry_count
        || le_u64(&footer, 32) != entry_count
        || le_u64(&footer, 40) != central_size
        || le_u64(&footer, 48) != central_offset
        || le_u32(&footer, locator) != 0x0706_4b50
        || le_u32(&footer, locator + 4) != 0
        || le_u64(&footer, locator + 8) != footer_offset
        || le_u32(&footer, locator + 16) != 1
        || le_u32(&footer, legacy) != 0x0605_4b50
        || le_u16(&footer, legacy + 4) != 0
        || le_u16(&footer, legacy + 6) != 0
        || le_u16(&footer, legacy + 8) != entry_count.min(u16::MAX as u64) as u16
        || le_u16(&footer, legacy + 10) != entry_count.min(u16::MAX as u64) as u16
        || le_u32(&footer, legacy + 12) != central_size.min(u32::MAX as u64) as u32
        || le_u32(&footer, legacy + 16) != central_offset.min(u32::MAX as u64) as u32
        || le_u16(&footer, legacy + 20) != 0
    {
        return Err(layout_error(
            "ZIP64 EOCD, locator, or legacy EOCD differs from the exact dialect".to_owned(),
        ));
    }
    Ok(())
}

fn exact_zip64_extra(byte_len: u64, header_start: u64) -> Vec<u8> {
    let includes_offset = header_start >= u32::MAX as u64;
    let data_len = if includes_offset { 24u16 } else { 16u16 };
    let mut extra = Vec::with_capacity(usize::from(data_len) + 4);
    extra.extend_from_slice(&1u16.to_le_bytes());
    extra.extend_from_slice(&data_len.to_le_bytes());
    extra.extend_from_slice(&byte_len.to_le_bytes());
    extra.extend_from_slice(&byte_len.to_le_bytes());
    if includes_offset {
        extra.extend_from_slice(&header_start.to_le_bytes());
    }
    extra
}

fn checked_layout_add(
    current: u64,
    addition: u64,
    kind: &str,
) -> Result<u64, Revision3ExactSnapshotInspectionErrorV2> {
    current
        .checked_add(addition)
        .ok_or_else(|| layout_error(format!("{kind} offset overflowed")))
}

fn layout_error(reason: String) -> Revision3ExactSnapshotInspectionErrorV2 {
    Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(reason)
}

fn le_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().expect("fixed slice"))
}

fn le_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("fixed slice"))
}

fn le_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("fixed slice"))
}

#[cfg(unix)]
fn read_exact_file_at(file: &File, mut buffer: &mut [u8], mut offset: u64) -> io::Result<()> {
    use std::os::unix::fs::FileExt;

    while !buffer.is_empty() {
        let count = file.read_at(buffer, offset)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "short positioned file read",
            ));
        }
        offset += count as u64;
        buffer = &mut buffer[count..];
    }
    Ok(())
}

#[cfg(windows)]
fn read_exact_file_at(file: &File, mut buffer: &mut [u8], mut offset: u64) -> io::Result<()> {
    use std::os::windows::fs::FileExt;

    while !buffer.is_empty() {
        let count = file.seek_read(buffer, offset)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "short positioned file read",
            ));
        }
        offset += count as u64;
        buffer = &mut buffer[count..];
    }
    Ok(())
}

fn hash_open_file(
    file: &File,
    expected_len: u64,
) -> Result<Sha256Digest, Revision3ExactSnapshotInspectionErrorV2> {
    let mut reader = file.try_clone().map_err(source_io_error)?;
    reader.seek(SeekFrom::Start(0)).map_err(source_io_error)?;
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        let count = reader.read(&mut buffer).map_err(source_io_error)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or_else(|| {
            Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
                "source length overflowed while hashing".to_owned(),
            )
        })?;
        if total > expected_len {
            return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
                "source grew while hashing".to_owned(),
            ));
        }
        hasher.update(&buffer[..count]);
    }
    if total != expected_len {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(
            "source length changed while hashing".to_owned(),
        ));
    }
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}

fn source_io_error(error: io::Error) -> Revision3ExactSnapshotInspectionErrorV2 {
    Revision3ExactSnapshotInspectionErrorV2::InvalidSource(error.to_string())
}

fn archive_io_error(error: io::Error) -> Revision3ExactSnapshotInspectionErrorV2 {
    Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(error.to_string())
}

fn archive_error(error: zip::result::ZipError) -> Revision3ExactSnapshotInspectionErrorV2 {
    Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(error.to_string())
}

fn manifest_model_error(error: WorkingStoreError) -> Revision3ExactSnapshotInspectionErrorV2 {
    Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(error.to_string())
}

fn manifest_store_error(error: WorkingStoreError) -> Revision3ExactSnapshotInspectionErrorV2 {
    Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(error.to_string())
}

fn closure_model_error(error: WorkingStoreError) -> Revision3ExactSnapshotInspectionErrorV2 {
    Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(error.to_string())
}

fn closure_store_error(error: WorkingStoreError) -> Revision3ExactSnapshotInspectionErrorV2 {
    Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    use super::*;
    use crate::{
        LocaleCode, QuestCollisionArtifactRef, Revision2OggCodec, Revision2OggMetadata,
        Revision2VoiceTake, Revision2VoiceTakeStatus, Revision3EntityKind, Revision3OriginRef,
        Revision3QuestDraft, Revision3QuestDraftInput, Revision3QuestGiverInput,
        Revision3QuestParentInput, Revision3ScriptModule, Revision3TypedRef, ScriptModuleStatus,
        QUEST_COLLISION_CATALOG_LAYER, REVISION3_QUEST_GENERATOR_ID,
        REVISION3_QUEST_GENERATOR_VERSION,
    };

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TestArea(PathBuf);

    impl TestArea {
        fn new(label: &str) -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gore-r3-import-v2-{label}-{}-{sequence:016x}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn archive(&self) -> PathBuf {
            self.0.join("snapshot.goremod")
        }
    }

    impl Drop for TestArea {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn seal(value: u8, byte_len: u64) -> ContentSeal {
        ContentSeal {
            byte_len,
            sha256: Sha256Digest::from_bytes([value; 32]),
        }
    }

    fn empty_project() -> ProjectRevision3 {
        empty_project_at_revision(7)
    }

    fn empty_project_at_revision(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x31; 16]),
            revision,
            meta: ProjectMeta {
                name: "Restorable inspection".to_owned(),
                version: "2.0.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: seal(0x41, 171_698_176),
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn entity_id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
    }

    fn quest_project(
        revision: u64,
        basis_snapshot: ContentSeal,
        artifact: ContentSeal,
        asset_meta: AssetMeta,
    ) -> ProjectRevision3 {
        let mut project = empty_project_at_revision(revision);
        let artifact_byte_len = artifact.byte_len;
        project
            .asset_store
            .assets
            .insert(artifact.sha256, asset_meta);
        let quest_id = entity_id(10);
        let module_id = entity_id(11);
        let quest = Revision3QuestDraft {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            input: Revision3QuestDraftInput {
                target: project.target.clone(),
                quest_id,
                module_namespace: "GoreMods.Quests.ImportTrial".to_owned(),
                technical_id: "GORE_IMPORT_TRIAL".to_owned(),
                text_helper: "GoreQuestText".to_owned(),
                parent_quest: Revision3QuestParentInput {
                    generation: project.target.clone(),
                    source_seal: seal(2, 20_000),
                    catalog_layer: "base-game.g1r.quests".to_owned(),
                    canonical_selector: "CatalogQuest_Parent".to_owned(),
                    runtime_class: "UQuest_Parent".to_owned(),
                },
                giver: Revision3QuestGiverInput {
                    generation: project.target.clone(),
                    source_seal: seal(3, 30_000),
                    catalog_layer: "base-game.g1r.characters".to_owned(),
                    canonical_selector: "CatalogCharacter_Asghan".to_owned(),
                    runtime_unique_name: "OM_GRD_Asghan_263".to_owned(),
                },
                title: "Restorable Quest".to_owned(),
                description: "Exercise the exact import closure.".to_owned(),
                objective_title: "Reopen everything".to_owned(),
                additional_objective_titles: Vec::new(),
                transition_plan: None,
                collision_catalog: QuestCollisionArtifactRef {
                    generation: project.target.clone(),
                    catalog_layer: QUEST_COLLISION_CATALOG_LAYER.to_owned(),
                    artifact,
                    source_seal: seal(5, artifact_byte_len),
                    basis_snapshot,
                },
            },
            script_module: Revision3TypedRef::new(
                project.project_id,
                module_id,
                Revision3EntityKind::ScriptModule,
            ),
            transcript: Vec::new(),
        };
        let source = "// persisted import-inspection Quest module\n".to_owned();
        let owner = Revision3TypedRef::new(
            project.project_id,
            quest_id,
            Revision3EntityKind::QuestDraft,
        );
        let module = Revision3ScriptModule {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            owner: owner.clone(),
            module_namespace: quest.input.module_namespace.clone(),
            module_relative_path: "GoreMods/Quests/ImportTrial.as".to_owned(),
            source_sha256: Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into()),
            source,
            input_fingerprint: Sha256Digest::from_bytes([7; 32]),
            status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
        };
        project.entities.insert(
            quest_id,
            Revision3Entity {
                id: quest_id,
                display_name: "Restorable Quest".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: quest.input.technical_id.clone(),
                },
                revision: 0,
                payload: Revision3EntityPayload::QuestDraft(quest),
            },
        );
        project.entities.insert(
            module_id,
            Revision3Entity {
                id: module_id,
                display_name: "Restorable Quest source".to_owned(),
                origin: Revision3OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner,
                },
                revision: 0,
                payload: Revision3EntityPayload::ScriptModule(module),
            },
        );
        project
    }

    fn add_voice_take(project: &mut ProjectRevision3, imported: &ImportedOgg) {
        let take_id = entity_id(12);
        let locale = "en".parse::<LocaleCode>().unwrap();
        project.authoring_locales.insert(locale.clone());
        project.asset_store.assets.insert(
            imported.asset.sha256,
            AssetMeta {
                byte_len: imported.asset.byte_len,
                media_type: "audio/ogg".to_owned(),
            },
        );
        project.entities.insert(
            take_id,
            Revision3Entity {
                id: take_id,
                display_name: "Restorable tiny Vorbis take".to_owned(),
                origin: Revision3OriginRef::New {
                    authored_runtime_id: "VOICE_IMPORT_TRIAL_EN".to_owned(),
                },
                revision: 0,
                payload: Revision3EntityPayload::VoiceTake(Revision2VoiceTake {
                    locale,
                    asset: imported.asset.clone(),
                    ogg: Revision2OggMetadata {
                        codec: match imported.ogg.codec {
                            OggCodec::Vorbis => Revision2OggCodec::Vorbis,
                            OggCodec::Opus => Revision2OggCodec::Opus,
                        },
                        channels: imported.ogg.channels,
                        sample_rate: imported.ogg.sample_rate,
                        pages: imported.ogg.pages,
                        logical_streams: imported.ogg.logical_streams,
                    },
                    status: Revision2VoiceTakeStatus::Recorded,
                }),
            },
        );
    }

    fn exact_archive_members() -> Vec<(String, Vec<u8>)> {
        let project = empty_project();
        let project_bytes = project.to_canonical_json().unwrap().into_bytes();
        let snapshot = Revision3SnapshotManifest {
            store_format: WorkingStoreFormat,
            format: project.format,
            schema_revision: project.schema_revision,
            project_id: project.project_id,
            revision: project.revision,
            meta: project.meta.clone(),
            target: project.target.clone(),
            authoring_locales: project.authoring_locales.clone(),
            entities: BTreeMap::new(),
            asset_store: project.asset_store.clone(),
            history: None,
        };
        let snapshot_bytes = canonical_json(&snapshot).unwrap();
        let snapshot_seal = seal_bytes(&snapshot_bytes);
        let head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: snapshot_seal.clone(),
        };
        let head_bytes = canonical_json(&head).unwrap();
        let snapshot_name = snapshot_member_name(snapshot_seal.sha256);
        let mut member_seals = BTreeMap::new();
        for (name, bytes) in [
            (PROJECT_FILE.to_owned(), project_bytes.clone()),
            (STORE_HEAD_MEMBER.to_owned(), head_bytes.clone()),
            (snapshot_name.clone(), snapshot_bytes.clone()),
        ] {
            let seal = seal_bytes(&bytes);
            member_seals.insert(
                name.clone(),
                ImportMemberSealV2 {
                    relative_name: name,
                    byte_len: seal.byte_len,
                    sha256: seal.sha256,
                },
            );
        }
        let manifest = ExactSnapshotManifestV2 {
            format: ImportManifestFormatV2::ExactSnapshotV2,
            schema: ImportManifestSchemaV2,
            artifact_kind: ImportArtifactKindV2::PortableSnapshotRestorableCopy,
            restore_status: ImportRestoreStatusV2::Supported,
            basis: ImportBasisV2 {
                head,
                project_id: project.project_id,
                project_revision: project.revision,
            },
            members: member_seals.into_values().collect(),
        };
        vec![
            (
                REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2.to_owned(),
                canonical_json(&manifest).unwrap(),
            ),
            (PROJECT_FILE.to_owned(), project_bytes),
            (STORE_HEAD_MEMBER.to_owned(), head_bytes),
            (snapshot_name, snapshot_bytes),
        ]
    }

    fn write_exact_archive(path: &Path, members: &[(String, Vec<u8>)]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        writer.set_zip64_comment(Some(""));
        let timestamp = DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap();
        for (name, bytes) in members {
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .last_modified_time(timestamp)
                .unix_permissions(ZIP_FILE_MODE)
                .large_file(true);
            writer.start_file(name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        let mut file = writer.finish().unwrap();
        file.flush().unwrap();
        file.sync_all().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn exact_empty_archive_reopens_without_writing() {
        let area = TestArea::new("valid");
        let path = area.archive();
        write_exact_archive(&path, &exact_archive_members());
        let before = fs::read(&path).unwrap();
        let before_entries = fs::read_dir(&area.0).unwrap().count();

        let inspected = inspect_revision3_exact_snapshot_v2(&path).unwrap();

        // Importer-owned byte fixture: this is assembled independently of the production exporter
        // and pins the exact V2 dialect already accepted by the reader.
        assert_eq!(
            (
                inspected.archive.byte_len,
                inspected.archive.sha256.to_string()
            ),
            (
                2_523,
                "c074d1587f1fc3de6b3a1b6099cea78205393b50174437c187c566e94cdfb6a8".to_owned()
            )
        );
        assert_eq!(inspected.project_id, ProjectId::from_bytes([0x31; 16]));
        assert_eq!(inspected.project_revision, 7);
        assert_eq!(inspected.closure.snapshot_objects, 1);
        assert_eq!(inspected.closure.entity_objects, 0);
        assert_eq!(inspected.closure.asset_objects, 0);
        assert_eq!(inspected.closure.archive_entries, 4);
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read_dir(&area.0).unwrap().count(), before_entries);
    }

    #[cfg(windows)]
    #[test]
    fn native_import_materializes_an_exact_arbitrary_named_store() {
        let area = TestArea::new("native-success");
        let source = area.archive();
        write_exact_archive(&source, &exact_archive_members());
        let source_before = fs::read(&source).unwrap();
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let destination = area.0.join("Restored project without required suffix");

        let publication =
            import_revision3_exact_snapshot_v2(&source, &inspected.archive, &destination)
                .unwrap_or_else(|error| panic!("native import failed: {error:?}"));

        assert!(!publication.retry_safe());
        assert_eq!(publication.warning(), None);
        let receipt = publication.receipt().expect("confirmed import receipt");
        assert_eq!(receipt.head, inspected.head);
        assert_eq!(receipt.project_id, inspected.project_id);
        assert_eq!(receipt.project_revision, inspected.project_revision);
        assert_eq!(receipt.archive, inspected.archive);
        assert_eq!(receipt.manifest, inspected.manifest);
        assert_eq!(receipt.closure, inspected.closure);
        let store = WorkingProjectStore::open_existing(&destination, WorkingStoreLimits::default())
            .unwrap();
        let current = store
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        assert_eq!(current.head, inspected.head);
        assert_eq!(current.project, empty_project());
        assert_eq!(fs::read(&source).unwrap(), source_before);
        assert_eq!(
            fs::read_dir(&area.0)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .contains(".gore-import-"))
                .count(),
            0
        );
    }

    #[cfg(windows)]
    #[test]
    fn native_publish_primitive_renames_an_empty_retained_directory() {
        let area = TestArea::new("native-empty-publish");
        let destination = area.0.join("arbitrary final name");
        let guard = prepare_import_destination(&destination).unwrap();
        let mut staging = create_import_staging_directory(&guard).unwrap();

        publish_import_staging_no_clobber(&guard, &mut staging).unwrap_or_else(
            |error| match error {
                ImportPublishError::AlreadyExists => panic!("unexpected target collision"),
                ImportPublishError::BeforeSyscall(error) => {
                    panic!("empty publish failed before syscall: {error}")
                }
                ImportPublishError::PublicationUncertain => {
                    panic!("empty publish result was uncertain")
                }
            },
        );

        assert!(destination.is_dir());
        assert_eq!(import_final_identity(&guard), Some(staging.identity));
    }

    #[cfg(windows)]
    fn private_staging_paths(parent: &Path) -> Vec<PathBuf> {
        fs::read_dir(parent)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains(".gore-import-")
            })
            .map(|entry| entry.path())
            .collect()
    }

    #[cfg(windows)]
    #[test]
    fn every_prepublication_guard_failure_cleans_bounded_staging_and_preserves_head_last() {
        for phase in [
            ImportGuardPhase::AfterStagingCreate,
            ImportGuardPhase::AfterStoreMembers,
            ImportGuardPhase::BeforeHeadPublication,
            ImportGuardPhase::AfterHeadPublication,
            ImportGuardPhase::BeforePromotion,
            ImportGuardPhase::BeforePublicationSyscall,
        ] {
            let area = TestArea::new(&format!("guard-{phase:?}"));
            let source = area.archive();
            write_exact_archive(&source, &exact_archive_members());
            let source_before = fs::read(&source).unwrap();
            let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
            let destination = area.0.join("guarded destination");
            let mut observed = Vec::new();

            let result = import_revision3_exact_snapshot_v2_guarded(
                &source,
                &inspected.archive,
                &destination,
                WorkingStoreLimits::default(),
                |actual, staging, _| {
                    observed.push(actual);
                    if actual != phase {
                        return Ok(());
                    }
                    let head_exists = staging.join(HEAD_FILE_NAME).exists();
                    assert_eq!(
                        head_exists,
                        matches!(
                            actual,
                            ImportGuardPhase::AfterHeadPublication
                                | ImportGuardPhase::BeforePromotion
                                | ImportGuardPhase::BeforePublicationSyscall
                        ),
                        "fixed head publication order at {actual:?}"
                    );
                    if actual == ImportGuardPhase::AfterStagingCreate {
                        assert_eq!(fs::read_dir(staging).unwrap().count(), 0);
                    }
                    Err(Revision3ExactSnapshotImportErrorV2::Materialization(
                        format!("injected {actual:?}"),
                    ))
                },
            );

            assert!(matches!(
                result,
                Err(Revision3ExactSnapshotImportErrorV2::Materialization(message))
                    if message == format!("injected {phase:?}")
            ));
            assert_eq!(observed.last(), Some(&phase));
            assert!(!destination.exists());
            assert!(private_staging_paths(&area.0).is_empty());
            assert_eq!(fs::read(&source).unwrap(), source_before);
        }
    }

    #[cfg(windows)]
    #[test]
    fn every_post_native_create_failure_rolls_back_before_or_through_inventory() {
        for phase in [
            ImportCreateGuardPhase::StagingRoot,
            ImportCreateGuardPhase::StoreDirectory,
            ImportCreateGuardPhase::StoreFile,
        ] {
            let area = TestArea::new(&format!("post-create-{phase:?}"));
            let source = area.archive();
            write_exact_archive(&source, &exact_archive_members());
            let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
            let destination = area.0.join("post-create destination");
            IMPORT_CREATE_FAILPOINT.with(|failpoint| failpoint.set(Some(phase)));

            let result =
                import_revision3_exact_snapshot_v2(&source, &inspected.archive, &destination);

            assert!(matches!(
                result,
                Err(Revision3ExactSnapshotImportErrorV2::Materialization(message))
                    if message == format!("injected post-create failure at {phase:?}")
            ));
            assert!(!destination.exists());
            assert!(private_staging_paths(&area.0).is_empty());
            IMPORT_CREATE_FAILPOINT.with(|failpoint| assert_eq!(failpoint.get(), None));
        }
    }

    #[cfg(windows)]
    #[test]
    fn postpublication_failure_is_receipt_free_uncertainty() {
        let area = TestArea::new("postpublication-uncertain");
        let source = area.archive();
        write_exact_archive(&source, &exact_archive_members());
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let destination = area.0.join("published but uncertain");

        let publication = import_revision3_exact_snapshot_v2_guarded(
            &source,
            &inspected.archive,
            &destination,
            WorkingStoreLimits::default(),
            |phase, _, _| {
                if phase == ImportGuardPhase::AfterPromotion {
                    Err(Revision3ExactSnapshotImportErrorV2::Publication(
                        "injected after rename".to_owned(),
                    ))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap();

        assert_eq!(
            publication,
            Revision3ExactSnapshotImportPublicationV2::PublicationUncertain
        );
        assert!(publication.receipt().is_none());
        assert_eq!(
            publication.warning(),
            Some(Revision3ExactSnapshotImportWarningV2::PublicationUncertain)
        );
        assert!(!publication.retry_safe());
        assert!(destination.is_dir());
        assert!(private_staging_paths(&area.0).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn committed_rename_with_lost_response_is_receipt_free_uncertainty() {
        let area = TestArea::new("rename-response-lost");
        let source = area.archive();
        write_exact_archive(&source, &exact_archive_members());
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let destination = area.0.join("renamed before response loss");
        IMPORT_PUBLISH_POST_RENAME_FAILURE.with(|failpoint| failpoint.set(true));

        let publication =
            import_revision3_exact_snapshot_v2(&source, &inspected.archive, &destination).unwrap();

        assert_eq!(
            publication,
            Revision3ExactSnapshotImportPublicationV2::PublicationUncertain
        );
        assert!(publication.receipt().is_none());
        assert_eq!(
            publication.warning(),
            Some(Revision3ExactSnapshotImportWarningV2::PublicationUncertain)
        );
        assert!(!publication.retry_safe());
        assert!(destination.is_dir());
        assert!(private_staging_paths(&area.0).is_empty());
        IMPORT_PUBLISH_POST_RENAME_FAILURE.with(|failpoint| assert!(!failpoint.get()));
    }

    #[cfg(windows)]
    #[test]
    fn cleanup_accounting_warning_keeps_one_exact_receipt_and_source_lock() {
        let area = TestArea::new("cleanup-warning");
        let source = area.archive();
        write_exact_archive(&source, &exact_archive_members());
        let source_before = fs::read(&source).unwrap();
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let destination = area.0.join("warning destination");
        let mut observed = Vec::new();

        let publication = import_revision3_exact_snapshot_v2_guarded(
            &source,
            &inspected.archive,
            &destination,
            WorkingStoreLimits::default(),
            |phase, _, _| {
                observed.push(phase);
                assert!(OpenOptions::new().write(true).open(&source).is_err());
                assert!(fs::remove_file(&source).is_err());
                if phase == ImportGuardPhase::BeforeCleanupAccounting {
                    Err(Revision3ExactSnapshotImportErrorV2::Publication(
                        "injected cleanup accounting warning".to_owned(),
                    ))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap();

        assert!(matches!(
            publication,
            Revision3ExactSnapshotImportPublicationV2::ImportedWithCleanupWarning(_)
        ));
        assert_eq!(publication.receipt().unwrap().archive, inspected.archive);
        assert_eq!(
            publication.warning(),
            Some(Revision3ExactSnapshotImportWarningV2::CleanupIncomplete)
        );
        assert_eq!(
            observed,
            vec![
                ImportGuardPhase::AfterStagingCreate,
                ImportGuardPhase::AfterStoreMembers,
                ImportGuardPhase::BeforeHeadPublication,
                ImportGuardPhase::AfterHeadPublication,
                ImportGuardPhase::BeforePromotion,
                ImportGuardPhase::BeforePublicationSyscall,
                ImportGuardPhase::AfterPromotion,
                ImportGuardPhase::BeforeCleanupAccounting,
                ImportGuardPhase::BeforeReceiptLinearization,
            ]
        );
        assert_eq!(fs::read(&source).unwrap(), source_before);
        assert!(OpenOptions::new().write(true).open(&source).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn archive_cas_and_existing_or_racing_targets_never_clobber() {
        let area = TestArea::new("cas-no-clobber");
        let source = area.archive();
        write_exact_archive(&source, &exact_archive_members());
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let missing_parent_destination = area.0.join("missing-parent").join("destination");
        let wrong = ContentSeal {
            byte_len: inspected.archive.byte_len,
            sha256: Sha256Digest::from_bytes([0x99; 32]),
        };
        assert!(matches!(
            import_revision3_exact_snapshot_v2(&source, &wrong, &missing_parent_destination),
            Err(Revision3ExactSnapshotImportErrorV2::ArchiveCasMismatch { expected, actual })
                if expected == wrong && actual == inspected.archive
        ));
        assert!(!area.0.join("missing-parent").exists());

        let existing = area.0.join("existing destination");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("foreign.marker"), b"foreign").unwrap();
        assert!(matches!(
            import_revision3_exact_snapshot_v2(&source, &inspected.archive, &existing),
            Err(Revision3ExactSnapshotImportErrorV2::DestinationAlreadyExists)
        ));
        assert_eq!(
            fs::read(existing.join("foreign.marker")).unwrap(),
            b"foreign"
        );

        let racing = area.0.join("racing destination");
        let raced = import_revision3_exact_snapshot_v2_guarded(
            &source,
            &inspected.archive,
            &racing,
            WorkingStoreLimits::default(),
            |phase, _, target| {
                if phase == ImportGuardPhase::BeforePublicationSyscall {
                    fs::create_dir(target).unwrap();
                    fs::write(target.join("foreign.marker"), b"race winner").unwrap();
                }
                Ok(())
            },
        );
        assert!(matches!(
            raced,
            Err(Revision3ExactSnapshotImportErrorV2::DestinationAlreadyExists)
        ));
        assert_eq!(
            fs::read(racing.join("foreign.marker")).unwrap(),
            b"race winner"
        );
        assert!(private_staging_paths(&area.0).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn source_and_destination_parents_remain_pinned_through_materialization() {
        let area = TestArea::new("parent-pins");
        let source_parent = area.0.join("source-parent");
        let destination_parent = area.0.join("destination-parent");
        fs::create_dir(&source_parent).unwrap();
        fs::create_dir(&destination_parent).unwrap();
        let source = source_parent.join("snapshot.goremod");
        write_exact_archive(&source, &exact_archive_members());
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let destination = destination_parent.join("restored project");
        let moved_source_parent = area.0.join("moved-source-parent");
        let moved_destination_parent = area.0.join("moved-destination-parent");
        let mut attempted = false;

        let publication = import_revision3_exact_snapshot_v2_guarded(
            &source,
            &inspected.archive,
            &destination,
            WorkingStoreLimits::default(),
            |phase, _, _| {
                if phase == ImportGuardPhase::AfterStagingCreate {
                    attempted = true;
                    assert!(fs::rename(&source_parent, &moved_source_parent).is_err());
                    assert!(fs::rename(&destination_parent, &moved_destination_parent).is_err());
                }
                Ok(())
            },
        )
        .unwrap();

        assert!(attempted);
        assert!(matches!(
            publication,
            Revision3ExactSnapshotImportPublicationV2::Imported(_)
        ));
        assert!(source_parent.is_dir());
        assert!(destination.is_dir());
    }

    #[cfg(windows)]
    #[test]
    fn destination_reparse_parents_and_preexisting_link_names_fail_closed() {
        use std::os::windows::fs::{symlink_dir, symlink_file};

        let area = TestArea::new("destination-reparse");
        let source = area.archive();
        write_exact_archive(&source, &exact_archive_members());
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let real_parent = area.0.join("real-parent");
        fs::create_dir(&real_parent).unwrap();
        let linked_parent = area.0.join("linked-parent");
        match symlink_dir(&real_parent, &linked_parent) {
            Ok(()) => {
                let through_link = linked_parent.join("must-not-exist");
                assert!(matches!(
                    import_revision3_exact_snapshot_v2(&source, &inspected.archive, &through_link,),
                    Err(Revision3ExactSnapshotImportErrorV2::InvalidDestination(_))
                ));
                assert!(!real_parent.join("must-not-exist").exists());
            }
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {}
            Err(error) => panic!("failed to create destination-parent symlink: {error}"),
        }

        let external_file = area.0.join("external-sentinel.bin");
        fs::write(&external_file, b"external sentinel").unwrap();
        let hardlinked_final = area.0.join("hardlinked-final-name");
        fs::hard_link(&external_file, &hardlinked_final).unwrap();
        assert!(matches!(
            import_revision3_exact_snapshot_v2(&source, &inspected.archive, &hardlinked_final,),
            Err(Revision3ExactSnapshotImportErrorV2::DestinationAlreadyExists)
        ));
        assert_eq!(fs::read(&external_file).unwrap(), b"external sentinel");

        let symlinked_final = area.0.join("symlinked-final-name");
        match symlink_file(&external_file, &symlinked_final) {
            Ok(()) => {
                assert!(matches!(
                    import_revision3_exact_snapshot_v2(
                        &source,
                        &inspected.archive,
                        &symlinked_final,
                    ),
                    Err(Revision3ExactSnapshotImportErrorV2::DestinationAlreadyExists)
                ));
                assert_eq!(fs::read(&external_file).unwrap(), b"external sentinel");
            }
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {}
            Err(error) => panic!("failed to create destination symlink: {error}"),
        }
    }

    #[cfg(windows)]
    #[test]
    fn native_destination_rejects_device_ads_reserved_and_ambiguous_spellings() {
        let area = TestArea::new("destination-spelling");
        let source = area.archive();
        write_exact_archive(&source, &exact_archive_members());
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let area_text = area.0.to_string_lossy();
        let verbatim_target = area.0.join("verbatim-target");
        let device_target = area.0.join("device-target");
        let rejected = [
            PathBuf::from(format!(r"\\?\{}\verbatim-target", area_text)),
            PathBuf::from(format!(r"\\.\{}\device-target", area_text)),
            area.0.join("stream:name"),
            area.0.join("NUL"),
            area.0.join("COM1.txt"),
            area.0.join("COM\u{00b9}.txt"),
            area.0.join("LPT\u{00b2}"),
            area.0.join("AUX").join("child"),
            area.0.join("trailing."),
            area.0.join("trailing "),
            PathBuf::from(format!(r"{}\duplicate\\component", area_text)),
            PathBuf::from(format!("{}\\trailing-separator\\", area_text)),
            PathBuf::from(format!("{}\\c1-\u{0085}-component", area_text)),
            PathBuf::from(format!(r"{}\.\dot-component", area_text)),
            PathBuf::from(format!(r"{}\nested\..\parent-component", area_text)),
            PathBuf::from(r"\\NUL\share\target"),
            PathBuf::from(r"\\server\COM3\target"),
            area.0.join("x".repeat(MAX_IMPORT_PATH_UTF8_BYTES_V2 + 1)),
        ];

        for destination in rejected {
            assert!(
                matches!(
                    import_revision3_exact_snapshot_v2(&source, &inspected.archive, &destination,),
                    Err(Revision3ExactSnapshotImportErrorV2::InvalidDestination(_))
                ),
                "unsafe destination spelling was accepted: {destination:?}"
            );
        }
        assert!(!verbatim_target.exists());
        assert!(!device_target.exists());
        assert!(private_staging_paths(&area.0).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn native_source_rejects_non_unicode_and_overlong_spellings_before_io() {
        use std::os::windows::ffi::OsStringExt as _;

        let oversized = PathBuf::from(format!(
            r"C:\{}.goremod",
            "x".repeat(MAX_IMPORT_PATH_UTF8_BYTES_V2 + 1)
        ));
        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&oversized),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(message))
                if message.contains("32-KiB")
        ));

        let mut invalid_wide = "C:\\invalid-".encode_utf16().collect::<Vec<_>>();
        invalid_wide.push(0xd800);
        invalid_wide.extend(".goremod".encode_utf16());
        let invalid_unicode = PathBuf::from(OsString::from_wide(&invalid_wide));
        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&invalid_unicode),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(message))
                if message.contains("non-Unicode")
        ));
    }

    #[cfg(windows)]
    #[test]
    fn bounded_cleanup_never_deletes_an_unplanned_foreign_entry() {
        let area = TestArea::new("bounded-foreign-cleanup");
        let source = area.archive();
        write_exact_archive(&source, &exact_archive_members());
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let destination = area.0.join("must remain absent");
        let mut private = None;

        let result = import_revision3_exact_snapshot_v2_guarded(
            &source,
            &inspected.archive,
            &destination,
            WorkingStoreLimits::default(),
            |phase, staging, _| {
                if phase == ImportGuardPhase::AfterStoreMembers {
                    fs::write(staging.join("foreign.marker"), b"do not delete").unwrap();
                    private = Some(staging.to_path_buf());
                }
                Ok(())
            },
        );

        assert!(matches!(
            result,
            Err(Revision3ExactSnapshotImportErrorV2::StagingCleanup { .. })
        ));
        assert!(!destination.exists());
        let private = private.unwrap();
        assert_eq!(
            fs::read(private.join("foreign.marker")).unwrap(),
            b"do not delete"
        );
        assert_eq!(fs::read_dir(&private).unwrap().count(), 1);
        fs::remove_dir_all(private).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn recursive_change_watch_makes_final_extra_entries_receipt_free() {
        let area = TestArea::new("watch-extra-entry");
        let source = area.archive();
        write_exact_archive(&source, &exact_archive_members());
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let destination = area.0.join("watch destination");

        let publication = import_revision3_exact_snapshot_v2_guarded(
            &source,
            &inspected.archive,
            &destination,
            WorkingStoreLimits::default(),
            |phase, _, target| {
                if phase == ImportGuardPhase::BeforeReceiptLinearization {
                    fs::write(target.join("late-extra.file"), b"late").unwrap();
                    fs::create_dir(target.join("late-extra-directory")).unwrap();
                }
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(
            publication,
            Revision3ExactSnapshotImportPublicationV2::PublicationUncertain
        );
        assert!(publication.receipt().is_none());
        assert!(!publication.retry_safe());
    }

    #[cfg(windows)]
    #[test]
    fn final_external_hardlink_is_revalidated_but_not_a_global_receipt_claim() {
        let area = TestArea::new("final-hardlink-recheck");
        let source = area.archive();
        write_exact_archive(&source, &exact_archive_members());
        let inspected = inspect_revision3_exact_snapshot_v2(&source).unwrap();
        let destination = area.0.join("hardlink destination");
        let alias = area.0.join("outside-tree-head-alias.json");

        let publication = import_revision3_exact_snapshot_v2_guarded(
            &source,
            &inspected.archive,
            &destination,
            WorkingStoreLimits::default(),
            |phase, _, target| {
                if phase == ImportGuardPhase::BeforeCleanupAccounting {
                    fs::hard_link(target.join(HEAD_FILE_NAME), &alias).unwrap();
                }
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(
            publication,
            Revision3ExactSnapshotImportPublicationV2::PublicationUncertain
        );
        assert!(publication.receipt().is_none());
        assert!(alias.is_file());
    }

    #[cfg(windows)]
    #[test]
    fn real_v2_export_reopens_to_the_same_exact_receipt() {
        let area = TestArea::new("export-compatibility");
        let store_root = area.0.join("managed.goreproj");
        let store = WorkingProjectStore::at(&store_root, WorkingStoreLimits::default()).unwrap();
        let prepared = store
            .prepare_revision3_checkpoint(None, &empty_project())
            .unwrap();
        fs::write(store_root.join(HEAD_FILE_NAME), &prepared.head_bytes).unwrap();
        let path = area.archive();
        let exported = store
            .export_current_revision3_exact_snapshot_v2(&prepared.head, &path)
            .unwrap();

        let inspected = inspect_revision3_exact_snapshot_v2(&path).unwrap();

        assert_eq!(inspected.head, exported.receipt().head);
        assert_eq!(inspected.project_id, exported.receipt().project_id);
        assert_eq!(
            inspected.project_revision,
            exported.receipt().project_revision
        );
        assert_eq!(inspected.archive, exported.receipt().archive);
        assert_eq!(inspected.manifest, exported.receipt().manifest);
        assert_eq!(
            inspected.closure.snapshot_objects,
            exported.receipt().closure.snapshot_objects
        );
        assert_eq!(
            inspected.closure.archive_entries,
            exported.receipt().closure.archive_entries
        );
        assert_eq!(
            inspected.closure.uncompressed_bytes,
            exported.receipt().closure.uncompressed_bytes
        );
    }

    #[cfg(windows)]
    #[test]
    fn strict_producer_v2_reopens_with_default_reader_across_shared_closure() {
        let area = TestArea::new("nonempty-export-compatibility");
        let store_root = area.0.join("managed.goreproj");
        let producer_limits = WorkingStoreLimits {
            max_snapshot_bytes: 64 * 1024,
            max_entity_bytes: 64 * 1024,
            ..WorkingStoreLimits::default()
        };
        let store = WorkingProjectStore::at(&store_root, producer_limits).unwrap();
        // Materialize a separately reachable Quest basis while the fixed head is still absent.
        let quest_basis = store
            .prepare_revision3_checkpoint(None, &empty_project_at_revision(3))
            .unwrap();
        let published_basis = store
            .prepare_revision3_checkpoint(None, &empty_project_at_revision(6))
            .unwrap();
        fs::write(store_root.join(HEAD_FILE_NAME), &published_basis.head_bytes).unwrap();
        let artifact_bytes = br#"{"padding":"import-closure"}"#;
        let imported = store
            .import_quest_collision_artifact_v1(artifact_bytes, Some(&published_basis.head))
            .unwrap();
        let prior_project = quest_project(
            7,
            quest_basis.head.snapshot.clone(),
            imported.artifact.clone(),
            imported.asset_meta.clone(),
        );
        let prior = store
            .prepare_revision3_checkpoint(Some(&published_basis.head), &prior_project)
            .unwrap();
        fs::write(store_root.join(HEAD_FILE_NAME), &prior.head_bytes).unwrap();
        let project = quest_project(
            8,
            quest_basis.head.snapshot.clone(),
            imported.artifact,
            imported.asset_meta,
        );
        let current = store
            .prepare_revision3_checkpoint(Some(&prior.head), &project)
            .unwrap();
        fs::write(store_root.join(HEAD_FILE_NAME), &current.head_bytes).unwrap();
        let preflight = store
            .inspect_revision3_dataasset_basis(&current.head.snapshot)
            .unwrap();
        let mut mirrored_budget = FullReopenWorkBudget::with_limits(u64::MAX, u64::MAX);
        mirrored_budget
            .charge_snapshot(&current.head.snapshot, &preflight.manifest)
            .unwrap();
        let closed_work = revision3_exact_snapshot_v2_full_reopen_work(
            &current.head.snapshot,
            &preflight.manifest,
        )
        .unwrap();
        assert_eq!(mirrored_budget.objects, closed_work.objects);
        assert_eq!(mirrored_budget.bytes, closed_work.bytes);
        assert_eq!(preflight.verification_objects, closed_work.objects);
        assert!(
            preflight.verification_bytes < closed_work.bytes,
            "strict producer-local legacy work must differ from the closed V2 format charge"
        );
        let path = area.archive();
        let exported = store
            .export_current_revision3_exact_snapshot_v2(&current.head, &path)
            .unwrap();

        // The public reader uses default limits. Its V2 work policy must nevertheless be exactly
        // the same as the strict producer's V2 policy, not either side's local Store limit.
        let inspected = inspect_revision3_exact_snapshot_v2(&path).unwrap();

        assert_eq!(inspected.head, current.head);
        assert_eq!(inspected.project_revision, 8);
        // Current and prior share both entity shards and the asset. This is the normal-path
        // writer-to-reader compatibility regression for the aggregate Full-reopen work budget.
        assert_eq!(inspected.closure.snapshot_objects, 4);
        assert_eq!(inspected.closure.entity_objects, 2);
        assert_eq!(inspected.closure.asset_objects, 1);
        assert_eq!(inspected.closure.archive_entries, 10);
        assert_eq!(inspected.archive, exported.receipt().archive);
        assert_eq!(inspected.manifest, exported.receipt().manifest);
    }

    #[cfg(windows)]
    #[test]
    fn rich_history_entities_assets_and_real_ogg_materialize_exactly() {
        let area = TestArea::new("rich-native-import");
        let store_root = area.0.join("producer.goreproj");
        let store = WorkingProjectStore::at(&store_root, WorkingStoreLimits::default()).unwrap();
        let quest_basis = store
            .prepare_revision3_checkpoint(None, &empty_project_at_revision(3))
            .unwrap();
        let published_basis = store
            .prepare_revision3_checkpoint(None, &empty_project_at_revision(6))
            .unwrap();
        fs::write(store_root.join(HEAD_FILE_NAME), &published_basis.head_bytes).unwrap();
        let collision = store
            .import_quest_collision_artifact_v1(
                br#"{"padding":"rich-import-closure"}"#,
                Some(&published_basis.head),
            )
            .unwrap();
        let ogg_source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../gore-vo/testdata/tiny-vorbis.ogg");
        let ogg_source_before = fs::read(&ogg_source).unwrap();
        let ogg = store
            .import_ogg(&ogg_source, "tiny-vorbis.ogg", Some(&published_basis.head))
            .unwrap();
        assert_eq!(ogg.asset.byte_len, 3_661);

        let mut prior_project = quest_project(
            7,
            quest_basis.head.snapshot.clone(),
            collision.artifact.clone(),
            collision.asset_meta.clone(),
        );
        add_voice_take(&mut prior_project, &ogg);
        let prior = store
            .prepare_revision3_checkpoint(Some(&published_basis.head), &prior_project)
            .unwrap();
        fs::write(store_root.join(HEAD_FILE_NAME), &prior.head_bytes).unwrap();
        let mut project = quest_project(
            8,
            quest_basis.head.snapshot.clone(),
            collision.artifact,
            collision.asset_meta,
        );
        add_voice_take(&mut project, &ogg);
        let current = store
            .prepare_revision3_checkpoint(Some(&prior.head), &project)
            .unwrap();
        fs::write(store_root.join(HEAD_FILE_NAME), &current.head_bytes).unwrap();
        let source = area.archive();
        let exported = store
            .export_current_revision3_exact_snapshot_v2(&current.head, &source)
            .unwrap();
        let source_before = fs::read(&source).unwrap();
        let destination = area.0.join("rich restored project");

        let publication =
            import_revision3_exact_snapshot_v2(&source, &exported.receipt().archive, &destination)
                .unwrap();

        assert!(matches!(
            publication,
            Revision3ExactSnapshotImportPublicationV2::Imported(_)
        ));
        let receipt = publication.receipt().unwrap();
        assert_eq!(receipt.head, current.head);
        assert_eq!(receipt.project_id, project.project_id);
        assert_eq!(receipt.project_revision, 8);
        assert_eq!(receipt.archive, exported.receipt().archive);
        assert_eq!(receipt.manifest, exported.receipt().manifest);
        assert_eq!(receipt.closure.snapshot_objects, 4);
        assert_eq!(receipt.closure.entity_objects, 3);
        assert_eq!(receipt.closure.asset_objects, 2);
        assert_eq!(receipt.closure.archive_entries, 12);
        let restored =
            WorkingProjectStore::open_existing(&destination, WorkingStoreLimits::default())
                .unwrap();
        let reopened = restored
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        assert_eq!(reopened.head, current.head);
        assert_eq!(reopened.project, project);
        assert_eq!(
            restored.read_verified_ogg_asset(&ogg.asset).unwrap(),
            ogg_source_before
        );
        assert_eq!(fs::read(&source).unwrap(), source_before);
        assert_eq!(fs::read(&ogg_source).unwrap(), ogg_source_before);
    }

    #[test]
    fn repeated_shared_entity_references_fail_before_payload_read_amplification() {
        let project = empty_project();
        let entity = Revision3Entity {
            id: entity_id(20),
            display_name: "shared shard".to_owned(),
            origin: Revision3OriginRef::New {
                authored_runtime_id: "SHARED_SHARD".to_owned(),
            },
            revision: 0,
            payload: Revision3EntityPayload::ScriptModule(Revision3ScriptModule {
                generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                owner: Revision3TypedRef::new(
                    project.project_id,
                    entity_id(21),
                    Revision3EntityKind::QuestDraft,
                ),
                module_namespace: "GoreMods.Shared".to_owned(),
                module_relative_path: "GoreMods/Shared.as".to_owned(),
                source: "// shared\n".to_owned(),
                source_sha256: Sha256Digest::from_bytes(Sha256::digest(b"// shared\n").into()),
                input_fingerprint: Sha256Digest::from_bytes([8; 32]),
                status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
            }),
        };
        let entity_bytes = canonical_json(&entity).unwrap();
        let entity_seal = seal_bytes(&entity_bytes);
        let mut snapshot = Revision3SnapshotManifest {
            store_format: WorkingStoreFormat,
            format: project.format,
            schema_revision: project.schema_revision,
            project_id: project.project_id,
            revision: project.revision,
            meta: project.meta,
            target: project.target,
            authoring_locales: project.authoring_locales,
            entities: BTreeMap::from([(entity.id, entity_seal)]),
            asset_store: project.asset_store,
            history: None,
        };
        // One shared reference costs three Full-reopen objects (snapshot + two entity passes).
        // A second snapshot is rejected by the aggregate charge before its payload loop can run.
        let mut budget = FullReopenWorkBudget::with_limits(5, u64::MAX);
        let mut simulated_payload_reads = 0;
        let mut rejected = None;
        for revision in 0..1_000 {
            snapshot.revision = revision;
            let snapshot_seal = seal_bytes(&canonical_json(&snapshot).unwrap());
            match budget.charge_snapshot(&snapshot_seal, &snapshot) {
                Ok(()) => simulated_payload_reads += snapshot.entities.len(),
                Err(error) => {
                    rejected = Some(error);
                    break;
                }
            }
        }
        assert_eq!(simulated_payload_reads, 1);
        assert!(matches!(
            rejected,
            Some(Revision3ExactSnapshotInspectionErrorV2::Limit {
                kind: "full-reopen objects",
                actual: 6,
                limit: 5,
            })
        ));
    }

    #[cfg(windows)]
    #[test]
    fn pinned_windows_source_handle_denies_write_and_delete() {
        use std::os::windows::ffi::OsStrExt as _;
        use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING};

        let area = TestArea::new("windows-pin");
        let path = area.archive();
        write_exact_archive(&path, &exact_archive_members());
        let replacement = area.0.join("same-length-replacement.goremod");
        fs::copy(&path, &replacement).unwrap();
        let wide = |path: &Path| {
            path.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>()
        };
        let path_wide = wide(&path);
        let replacement_wide = wide(&replacement);

        let pinned = open_untrusted_source(&path).unwrap();
        assert!(OpenOptions::new().write(true).open(&path).is_err());
        assert!(fs::remove_file(&path).is_err());
        // SAFETY: both slices are live, NUL-terminated UTF-16 paths for the duration of the call.
        assert_eq!(
            unsafe {
                MoveFileExW(
                    replacement_wide.as_ptr(),
                    path_wide.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING,
                )
            },
            0
        );
        drop(pinned);
        assert!(OpenOptions::new().write(true).open(&path).is_ok());
        // SAFETY: same live, NUL-terminated paths as above; the pinned handle has been released.
        assert_ne!(
            unsafe {
                MoveFileExW(
                    replacement_wide.as_ptr(),
                    path_wide.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING,
                )
            },
            0
        );
    }

    #[cfg(windows)]
    #[test]
    fn final_hard_link_source_is_rejected() {
        let area = TestArea::new("final-hard-link");
        let path = area.archive();
        write_exact_archive(&path, &exact_archive_members());
        fs::hard_link(&path, area.0.join("second-name.goremod")).unwrap();

        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn final_symlink_or_reparse_source_is_rejected() {
        use std::os::windows::fs::symlink_file;

        let area = TestArea::new("final-symlink");
        let target = area.0.join("target.goremod");
        write_exact_archive(&target, &exact_archive_members());
        let path = area.archive();
        match symlink_file(&target, &path) {
            Ok(()) => {}
            // Older Windows installations may disable unprivileged symlink creation. The opener
            // is still covered by the hard-link and pinned-handle tests on those hosts.
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => return,
            Err(error) => panic!("failed to create final symlink fixture: {error}"),
        }

        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidSource(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn manifest_must_be_the_exact_first_member() {
        let area = TestArea::new("first");
        let path = area.archive();
        let mut members = exact_archive_members();
        members.swap(0, 1);
        write_exact_archive(&path, &members);

        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn empty_zip_is_an_invalid_archive_not_a_limit() {
        let area = TestArea::new("empty-zip");
        let path = area.archive();
        let file = File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        writer.set_zip64_comment(Some(""));
        writer.finish().unwrap();

        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn zero_byte_first_manifest_is_an_invalid_manifest_not_a_limit() {
        let area = TestArea::new("zero-manifest");
        let path = area.archive();
        write_exact_archive(
            &path,
            &[(
                REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2.to_owned(),
                Vec::new(),
            )],
        );

        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn exact_layout_rejects_trailing_bytes() {
        let area = TestArea::new("trailing");
        let path = area.archive();
        write_exact_archive(&path, &exact_archive_members());
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"trailing")
            .unwrap();

        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn deterministic_metadata_is_mandatory() {
        let area = TestArea::new("metadata");
        let path = area.archive();
        let members = exact_archive_members();
        let file = File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        writer.set_zip64_comment(Some(""));
        for (index, (name, bytes)) in members.iter().enumerate() {
            let timestamp = if index == 1 {
                DateTime::from_date_and_time(1981, 1, 1, 0, 0, 0).unwrap()
            } else {
                DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap()
            };
            writer
                .start_file(
                    name,
                    SimpleFileOptions::default()
                        .compression_method(CompressionMethod::Stored)
                        .last_modified_time(timestamp)
                        .unix_permissions(ZIP_FILE_MODE)
                        .large_file(true),
                )
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();

        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn every_member_payload_must_match_its_manifest_seal() {
        let area = TestArea::new("member-seal");
        let path = area.archive();
        let mut members = exact_archive_members();
        members[3].1.push(b' ');
        write_exact_archive(&path, &members);

        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidArchive(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn missing_referenced_current_snapshot_is_rejected_as_closure_corruption() {
        let area = TestArea::new("missing-current");
        let path = area.archive();
        let mut members = exact_archive_members();
        let mut manifest: ExactSnapshotManifestV2 =
            parse_canonical_json(&members[0].1, "test V2 manifest").unwrap();
        manifest.members.retain(|member| {
            !matches!(
                parse_member_name(&member.relative_name),
                Ok(ImportMemberKind::Snapshot(_))
            )
        });
        members[0].1 = canonical_json(&manifest).unwrap();
        members.pop();
        write_exact_archive(&path, &members);

        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn sealed_but_unreachable_store_object_is_rejected() {
        let area = TestArea::new("orphan");
        let path = area.archive();
        let mut members = exact_archive_members();
        let asset_bytes = b"unreachable".to_vec();
        let asset_seal = seal_bytes(&asset_bytes);
        let asset_name = asset_member_name(asset_seal.sha256);
        let mut manifest: ExactSnapshotManifestV2 =
            parse_canonical_json(&members[0].1, "test V2 manifest").unwrap();
        manifest.members.push(ImportMemberSealV2 {
            relative_name: asset_name.clone(),
            byte_len: asset_seal.byte_len,
            sha256: asset_seal.sha256,
        });
        manifest
            .members
            .sort_by(|left, right| left.relative_name.cmp(&right.relative_name));
        members[0].1 = canonical_json(&manifest).unwrap();
        members.push((asset_name, asset_bytes));
        write_exact_archive(&path, &members);

        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(_))
        ));
    }

    #[test]
    fn member_path_grammar_rejects_traversal_case_and_wrong_shards() {
        for name in [
            "../project.json",
            "store\\gore-project.json",
            "STORE/gore-project.json",
            "store/snapshots/sha256/aa/../evil.json",
            "store/assets/sha256/AA/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "store/entities/00/00000000000000000000000000000/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.json",
        ] {
            assert!(matches!(
                parse_member_name(name),
                Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(_))
            ));
        }
    }

    #[test]
    fn manifest_category_count_and_byte_limits_fail_before_payload_reads() {
        let base = exact_archive_members();
        let mut manifest: ExactSnapshotManifestV2 =
            parse_canonical_json(&base[0].1, "test V2 manifest").unwrap();
        for bytes in [b"asset-a".as_slice(), b"asset-b".as_slice()] {
            let seal = seal_bytes(bytes);
            manifest.members.push(ImportMemberSealV2 {
                relative_name: asset_member_name(seal.sha256),
                byte_len: seal.byte_len,
                sha256: seal.sha256,
            });
        }
        manifest
            .members
            .sort_by(|left, right| left.relative_name.cmp(&right.relative_name));
        let count_limits = WorkingStoreLimits {
            max_assets: 1,
            ..WorkingStoreLimits::default()
        };
        assert!(matches!(
            validate_manifest_plan(
                &manifest,
                &count_limits,
                canonical_json(&manifest).unwrap().len() as u64,
            ),
            Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
                kind: "asset object count",
                ..
            })
        ));

        let mut byte_manifest: ExactSnapshotManifestV2 =
            parse_canonical_json(&base[0].1, "test V2 manifest").unwrap();
        let asset = seal_bytes(b"asset-a");
        byte_manifest.members.push(ImportMemberSealV2 {
            relative_name: asset_member_name(asset.sha256),
            byte_len: asset.byte_len,
            sha256: asset.sha256,
        });
        byte_manifest
            .members
            .sort_by(|left, right| left.relative_name.cmp(&right.relative_name));
        let byte_limits = WorkingStoreLimits {
            max_referenced_asset_bytes: 1,
            ..WorkingStoreLimits::default()
        };
        assert!(matches!(
            validate_manifest_plan(
                &byte_manifest,
                &byte_limits,
                canonical_json(&byte_manifest).unwrap().len() as u64,
            ),
            Err(Revision3ExactSnapshotInspectionErrorV2::Limit {
                kind: "aggregate asset bytes",
                ..
            })
        ));
    }

    #[cfg(windows)]
    #[test]
    fn manifest_rejects_non_v2_authority_and_unknown_fields_generically() {
        let mut members = exact_archive_members();
        let mut unsupported: serde_json::Value = serde_json::from_slice(&members[0].1).unwrap();
        unsupported["format"] =
            serde_json::Value::String("gore.managed-project-snapshot.future".to_owned());
        unsupported["schema"] = serde_json::Value::Number(999.into());
        unsupported["artifact_kind"] =
            serde_json::Value::String("unsupported_snapshot_kind".to_owned());
        unsupported["restore_status"] = serde_json::Value::String("unsupported".to_owned());
        members[0].1 = serde_json::to_vec(&unsupported).unwrap();
        let area = TestArea::new("unsupported-authority");
        let path = area.archive();
        write_exact_archive(&path, &members);
        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(_))
        ));

        let mut members = exact_archive_members();
        let mut unknown: serde_json::Value = serde_json::from_slice(&members[0].1).unwrap();
        unknown["unexpected"] = serde_json::Value::Bool(true);
        members[0].1 = serde_json::to_vec(&unknown).unwrap();
        let area = TestArea::new("unknown");
        let path = area.archive();
        write_exact_archive(&path, &members);
        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::InvalidManifest(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn unix_inspection_fails_closed_before_source_io() {
        let source = std::env::temp_dir().join("untrusted.goremod");
        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&source),
            Err(Revision3ExactSnapshotInspectionErrorV2::UnsupportedPlatform)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn unix_import_fails_before_any_destination_mutation() {
        let area = TestArea::new("unix-import");
        let source = area.archive();
        let destination = area.0.join("must-not-be-created");
        let expected = seal(0x44, 123);
        assert!(matches!(
            import_revision3_exact_snapshot_v2(&source, &expected, &destination),
            Err(Revision3ExactSnapshotImportErrorV2::Inspection(
                Revision3ExactSnapshotInspectionErrorV2::UnsupportedPlatform
            ))
        ));
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(&area.0).unwrap().count(), 0);
    }
}
