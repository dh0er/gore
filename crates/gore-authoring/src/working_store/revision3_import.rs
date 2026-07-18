//! Read-only inspection of untrusted restorable managed revision-3 snapshot archives.
//!
//! This module is deliberately only the authority-free first half of import. It never extracts
//! a member, creates a destination, installs an immutable object, publishes a head, or adopts a
//! project. A later importer must re-run this inspection (or retain and consume the exact open
//! handle) before it may materialize an archive into a separately proven empty Store.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

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

const REVIEW_PROJECT_FILE: &str = "project.json";
const STORE_HEAD_MEMBER: &str = "store/gore-project.json";
const MAX_IMPORT_MANIFEST_BYTES_V2: usize = 128 * 1024 * 1024;
const MAX_IMPORT_ARCHIVE_ENTRIES_V2: u64 = 300_003;
// The closed format can contain at most 64 GiB of assets, 512 MiB each of snapshot and entity
// objects, the bounded manifest/review/head, and deterministic ZIP metadata. This early hard cap
// rejects an absurd source before the central directory is parsed; tighter caller limits and the
// exact member sums are enforced immediately after the first manifest is read.
const MAX_IMPORT_ARCHIVE_BYTES_V2: u64 = 70 * 1024 * 1024 * 1024;
const ZIP_FILE_MODE: u32 = 0o644;
const ZIP_VERSION_45: u16 = 45;
const ZIP_UNIX_VERSION_45: u16 = (3 << 8) | ZIP_VERSION_45;
const ZIP_DOS_EPOCH_TIME: u16 = 0;
const ZIP_DOS_EPOCH_DATE: u16 = 33;
const ZIP_EXTERNAL_FILE_ATTRIBUTES: u32 = (0o100000 | ZIP_FILE_MODE) << 16;

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

/// Stable failure vocabulary for read-only inspection of an untrusted source archive.
#[derive(Debug, thiserror::Error)]
pub enum Revision3ExactSnapshotInspectionErrorV2 {
    #[error(
        "managed snapshot declares the V1 review-copy format and is not accepted as restorable V2"
    )]
    UnsupportedReviewCopyV1,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum ReviewManifestFormatV1 {
    #[serde(rename = "gore.managed-project-snapshot.v1")]
    ExactSnapshotV1,
}

impl<'de> Deserialize<'de> for ReviewManifestFormatV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value == "gore.managed-project-snapshot.v1" {
            Ok(Self::ExactSnapshotV1)
        } else {
            Err(serde::de::Error::custom("not a managed V1 review copy"))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReviewManifestSchemaV1;

impl<'de> Deserialize<'de> for ReviewManifestSchemaV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if u32::deserialize(deserializer)? == 1 {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("not schema 1"))
        }
    }
}

impl Serialize for ReviewManifestSchemaV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ReviewArtifactKindV1 {
    #[serde(rename = "portable_snapshot_review_copy")]
    PortableSnapshotReviewCopy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ReviewRestoreStatusV1 {
    #[serde(rename = "not_supported")]
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactSnapshotReviewManifestV1 {
    format: ReviewManifestFormatV1,
    schema: ReviewManifestSchemaV1,
    artifact_kind: ReviewArtifactKindV1,
    restore_status: ReviewRestoreStatusV1,
    basis: ImportBasisV2,
    members: Vec<ImportMemberSealV2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportMemberKind {
    ReviewProject,
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
    let source = source.as_ref();
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
    let manifest: ExactSnapshotManifestV2 = match parse_canonical_json(
        &manifest_bytes,
        "managed restorable exact snapshot import manifest",
    ) {
        Ok(manifest) => manifest,
        Err(v2_error) => {
            // This recognizes only the canonical V1 authority declaration. It intentionally does
            // not claim that the remaining review-copy closure is complete or otherwise valid.
            if parse_canonical_json::<ExactSnapshotReviewManifestV1>(
                &manifest_bytes,
                "managed exact snapshot V1 review manifest",
            )
            .is_ok()
            {
                return Err(Revision3ExactSnapshotInspectionErrorV2::UnsupportedReviewCopyV1);
            }
            return Err(manifest_model_error(v2_error));
        }
    };
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

    Ok(Revision3ExactSnapshotInspectionV2 {
        head,
        project_id: project.project_id,
        project_revision: project.revision,
        archive: ContentSeal {
            byte_len: final_len,
            sha256: archive_after,
        },
        manifest: manifest_seal,
        closure: plan.closure,
    })
}

fn open_untrusted_source(source: &Path) -> Result<File, Revision3ExactSnapshotInspectionErrorV2> {
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
            ImportMemberKind::ReviewProject => {
                if project_seen || member.byte_len > MAX_PROJECT_JSON_BYTES as u64 {
                    return Err(member_limit_or_duplicate(
                        "review project bytes",
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
            "manifest must seal exactly one review project and one fixed Store head".to_owned(),
        ));
    }

    let mut archive_order = Vec::with_capacity(manifest.members.len() + 1);
    archive_order.push(REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2.to_owned());
    archive_order.push(REVIEW_PROJECT_FILE.to_owned());
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
    if name == REVIEW_PROJECT_FILE {
        return Ok(ImportMemberKind::ReviewProject);
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
        BTreeSet::from([REVIEW_PROJECT_FILE.to_owned(), STORE_HEAD_MEMBER.to_owned()]);
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
    let review_member = required_member(plan, REVIEW_PROJECT_FILE)?;
    let review_bytes = read_sealed_member(
        archive,
        review_member,
        MAX_PROJECT_JSON_BYTES as u64,
        "review project bytes",
    )?;
    let review_text = std::str::from_utf8(&review_bytes).map_err(|_| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
            "review project is not UTF-8".to_owned(),
        )
    })?;
    let review = ProjectRevision3::from_json(review_text).map_err(|error| {
        Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(format!(
            "review project is invalid: {error}"
        ))
    })?;
    if review != current_project {
        return Err(Revision3ExactSnapshotInspectionErrorV2::InvalidClosure(
            "review project differs from the fully reopened current Store project".to_owned(),
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
        QuestCollisionArtifactRef, Revision3EntityKind, Revision3OriginRef, Revision3QuestDraft,
        Revision3QuestDraftInput, Revision3QuestGiverInput, Revision3QuestParentInput,
        Revision3ScriptModule, Revision3TypedRef, ScriptModuleStatus,
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
            (REVIEW_PROJECT_FILE.to_owned(), project_bytes.clone()),
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
            (REVIEW_PROJECT_FILE.to_owned(), project_bytes),
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
    fn manifest_rejects_v1_authority_and_unknown_fields() {
        let mut members = exact_archive_members();
        let v2: ExactSnapshotManifestV2 =
            parse_canonical_json(&members[0].1, "test V2 manifest").unwrap();
        let v1 = ExactSnapshotReviewManifestV1 {
            format: ReviewManifestFormatV1::ExactSnapshotV1,
            schema: ReviewManifestSchemaV1,
            artifact_kind: ReviewArtifactKindV1::PortableSnapshotReviewCopy,
            restore_status: ReviewRestoreStatusV1::NotSupported,
            basis: v2.basis,
            members: v2.members,
        };
        members[0].1 = canonical_json(&v1).unwrap();
        let area = TestArea::new("v1");
        let path = area.archive();
        write_exact_archive(&path, &members);
        assert!(matches!(
            inspect_revision3_exact_snapshot_v2(&path),
            Err(Revision3ExactSnapshotInspectionErrorV2::UnsupportedReviewCopyV1)
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
}
