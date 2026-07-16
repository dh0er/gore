//! Deterministic, exact-snapshot export for one currently published managed revision-3 project.
//!
//! Export is deliberately not an importer, clone, Save As, build, or backup/restore operation.
//! It copies the exact current Store closure into a portable, reviewable ZIP without changing the
//! Store head or adopting the output path. Historical Quest basis snapshots are traversed
//! transitively; unrelated immutable or staging objects are excluded.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use cap_std::ambient_authority;
use cap_std::fs::Dir;
#[cfg(windows)]
use cap_std::fs::OpenOptions as CapOpenOptions;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

use super::*;
use crate::{
    MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1,
    MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1,
};

pub const REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V1: &str = "managed_revision3_exact_snapshot_v1";
pub const REVISION3_EXACT_SNAPSHOT_MANIFEST_MARKER_V1: &str = "gore.managed-project-snapshot.v1";
pub const REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V1: &str = "portable_snapshot_review_copy";
pub const REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V1: &str = "not_supported";
pub const REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V1: &str = "gore-export.json";

const REVIEW_PROJECT_FILE: &str = "project.json";
const STORE_HEAD_MEMBER: &str = "store/gore-project.json";
const MAX_EXPORT_MANIFEST_BYTES_V1: usize = 128 * 1024 * 1024;
const MAX_EXPORT_ARCHIVE_ENTRIES_V1: u64 = 300_003;
const ZIP_FILE_MODE: u32 = 0o644;
const ZIP_VERSION_45: u16 = 45;
const ZIP_UNIX_VERSION_45: u16 = (3 << 8) | ZIP_VERSION_45;
const ZIP_DOS_EPOCH_TIME: u16 = 0;
const ZIP_DOS_EPOCH_DATE: u16 = 33;
const ZIP_EXTERNAL_FILE_ATTRIBUTES: u32 = (0o100000 | ZIP_FILE_MODE) << 16;

/// Complete exact Store closure represented by a managed snapshot export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ExactSnapshotClosureV1 {
    pub snapshot_objects: u64,
    pub entity_objects: u64,
    pub asset_objects: u64,
    pub archive_entries: u64,
    pub uncompressed_bytes: u64,
}

/// Path-independent receipt prepared and bounded before publication begins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ExactSnapshotExportV1 {
    pub head: WorkingHead,
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub archive: ContentSeal,
    pub manifest: ContentSeal,
    pub closure: Revision3ExactSnapshotClosureV1,
}

/// Stable warning class for the two successful-call terminals that need user attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3ExactSnapshotExportWarningV1 {
    CleanupIncomplete,
    PublicationUncertain,
}

/// Publication result. Every variant means the atomic publish boundary may have been crossed.
/// Callers must never turn either warning terminal into an automatic retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3ExactSnapshotExportPublicationV1 {
    Exported(Revision3ExactSnapshotExportV1),
    ExportedWithCleanupWarning(Revision3ExactSnapshotExportV1),
    PublicationUncertain(Revision3ExactSnapshotExportV1),
}

impl Revision3ExactSnapshotExportPublicationV1 {
    pub const fn receipt(&self) -> &Revision3ExactSnapshotExportV1 {
        match self {
            Self::Exported(receipt)
            | Self::ExportedWithCleanupWarning(receipt)
            | Self::PublicationUncertain(receipt) => receipt,
        }
    }

    pub const fn warning(&self) -> Option<Revision3ExactSnapshotExportWarningV1> {
        match self {
            Self::Exported(_) => None,
            Self::ExportedWithCleanupWarning(_) => {
                Some(Revision3ExactSnapshotExportWarningV1::CleanupIncomplete)
            }
            Self::PublicationUncertain(_) => {
                Some(Revision3ExactSnapshotExportWarningV1::PublicationUncertain)
            }
        }
    }
}

/// Failures returned only while native code still knows that it has not published an output.
#[derive(Debug, thiserror::Error)]
pub enum Revision3ExactSnapshotExportErrorV1 {
    #[error(transparent)]
    Store(#[from] WorkingStoreError),
    #[error("invalid managed snapshot export output: {0}")]
    InvalidOutput(String),
    #[error("managed snapshot export output already exists")]
    OutputAlreadyExists,
    #[error("managed snapshot export closure is invalid: {0}")]
    InvalidClosure(String),
    #[error("managed snapshot export closure limit exceeded for {kind}: {actual} > {limit}")]
    ClosureLimit {
        kind: &'static str,
        actual: u64,
        limit: u64,
    },
    #[error("managed snapshot ZIP composition failed: {0}")]
    Archive(String),
    #[error("managed snapshot staged reopen failed: {0}")]
    Verification(String),
    #[error("managed snapshot export could not be published without clobbering: {0}")]
    Publication(String),
    #[error("{primary}; managed snapshot private staging cleanup also failed: {cleanup}")]
    StagingCleanup {
        primary: Box<Revision3ExactSnapshotExportErrorV1>,
        cleanup: String,
    },
}

impl From<io::Error> for Revision3ExactSnapshotExportErrorV1 {
    fn from(error: io::Error) -> Self {
        Self::Store(WorkingStoreError::Io(error))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum ExportManifestFormatV1 {
    #[serde(rename = "gore.managed-project-snapshot.v1")]
    ExactSnapshotV1,
}

impl<'de> Deserialize<'de> for ExportManifestFormatV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value == REVISION3_EXACT_SNAPSHOT_MANIFEST_MARKER_V1 {
            Ok(Self::ExactSnapshotV1)
        } else {
            Err(serde::de::Error::custom(
                "unsupported managed project snapshot format",
            ))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExportManifestSchemaV1;

impl<'de> Deserialize<'de> for ExportManifestSchemaV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == 1 {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(
                "unsupported managed project snapshot schema",
            ))
        }
    }
}

impl Serialize for ExportManifestSchemaV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ExportArtifactKindV1 {
    #[serde(rename = "portable_snapshot_review_copy")]
    PortableSnapshotReviewCopy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ExportRestoreStatusV1 {
    #[serde(rename = "not_supported")]
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportBasisV1 {
    head: WorkingHead,
    project_id: ProjectId,
    project_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportMemberSealV1 {
    relative_name: String,
    byte_len: u64,
    sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactSnapshotManifestV1 {
    format: ExportManifestFormatV1,
    schema: ExportManifestSchemaV1,
    artifact_kind: ExportArtifactKindV1,
    restore_status: ExportRestoreStatusV1,
    basis: ExportBasisV1,
    members: Vec<ExportMemberSealV1>,
}

#[derive(Debug)]
enum MemberSource {
    Bytes(Vec<u8>),
    StoreObject(PathBuf),
}

#[derive(Debug)]
struct PlannedMember {
    relative_name: String,
    seal: ContentSeal,
    source: MemberSource,
}

#[derive(Debug, Default)]
struct ClosureObjects {
    snapshots: BTreeMap<String, PlannedMember>,
    entities: BTreeMap<String, PlannedMember>,
    assets: BTreeMap<String, PlannedMember>,
    snapshot_bytes: u64,
    entity_bytes: u64,
    asset_bytes: u64,
}

struct ExportPlan {
    receipt: Revision3ExactSnapshotExportV1,
    members: Vec<PlannedMember>,
    manifest: ExactSnapshotManifestV1,
    project: ProjectRevision3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawZipMemberExpectation {
    name: Vec<u8>,
    byte_len: u64,
    crc32: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportGuardPhase {
    BeforeStagingCreate,
    BeforeArchiveWrite,
    BeforePublication,
    BeforeInstall,
    AfterPublication,
    BeforeStagingCleanup,
}

struct OutputGuard {
    target: PathBuf,
    parent_path: PathBuf,
    filename: OsString,
    parent: Dir,
    parent_identity: DirectoryIdentity,
    store_root_identity: DirectoryIdentity,
    #[cfg(target_os = "linux")]
    publication_parent: File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirectoryIdentity {
    device: u64,
    inode: u64,
}

struct StagedArchive {
    file: File,
    display_path: PathBuf,
    published: bool,
}

impl WorkingProjectStore {
    /// Export the exact currently published managed revision-3 snapshot without adopting or
    /// mutating either the Store head or output path.
    pub fn export_current_revision3_exact_snapshot_v1(
        &self,
        expected_head: &WorkingHead,
        output: impl AsRef<Path>,
    ) -> Result<Revision3ExactSnapshotExportPublicationV1, Revision3ExactSnapshotExportErrorV1>
    {
        self.export_current_revision3_exact_snapshot_v1_guarded(
            expected_head,
            output.as_ref(),
            |_, _| Ok(()),
        )
    }

    fn export_current_revision3_exact_snapshot_v1_guarded<F>(
        &self,
        expected_head: &WorkingHead,
        output: &Path,
        mut external_guard: F,
    ) -> Result<Revision3ExactSnapshotExportPublicationV1, Revision3ExactSnapshotExportErrorV1>
    where
        F: FnMut(ExportGuardPhase, &Path) -> Result<(), WorkingStoreError>,
    {
        self.ensure_root_safe()?;
        self.check_expected_head(Some(expected_head))?;
        let output_guard = self.prepare_export_output(output)?;
        let mut plan = self.plan_revision3_exact_snapshot_export(expected_head)?;
        external_guard(ExportGuardPhase::BeforeStagingCreate, &output_guard.target)?;
        revalidate_export_output(&output_guard)?;
        let mut staged_archive = create_staged_archive(&output_guard)?;

        let staged_result = (|| {
            external_guard(
                ExportGuardPhase::BeforeArchiveWrite,
                &staged_archive.display_path,
            )?;
            write_deterministic_archive(&mut staged_archive.file, &plan.members)?;
            let archive = verify_staged_archive(
                &mut staged_archive.file,
                &plan.members,
                &plan.manifest,
                &plan.project,
            )?;
            plan.receipt.archive = archive;
            Ok::<(), Revision3ExactSnapshotExportErrorV1>(())
        })();
        if let Err(primary) = staged_result {
            return Err(abort_staged_archive(&mut staged_archive, primary));
        }

        // Constructing the plan and receipt, strict staged reopen, and every response-bound hash
        // are complete before the only publication boundary.
        if let Err(error) = external_guard(
            ExportGuardPhase::BeforePublication,
            &staged_archive.display_path,
        ) {
            return Err(abort_staged_archive(&mut staged_archive, error.into()));
        }
        if let Err(error) = self.check_expected_head(Some(expected_head)) {
            return Err(abort_staged_archive(&mut staged_archive, error.into()));
        }
        if let Err(error) = external_guard(ExportGuardPhase::BeforeInstall, &output_guard.target) {
            return Err(abort_staged_archive(&mut staged_archive, error.into()));
        }
        if let Err(error) = revalidate_export_output(&output_guard) {
            return Err(abort_staged_archive(&mut staged_archive, error));
        }

        match publish_staged_archive_no_clobber(&mut staged_archive, &output_guard) {
            Err(NoClobberInstallError::AlreadyExists) => {
                let primary = Revision3ExactSnapshotExportErrorV1::OutputAlreadyExists;
                Err(abort_staged_archive(&mut staged_archive, primary))
            }
            Err(NoClobberInstallError::Failed(error)) if !staged_archive.published => {
                let primary = Revision3ExactSnapshotExportErrorV1::Publication(error.to_string());
                Err(abort_staged_archive(&mut staged_archive, primary))
            }
            Err(NoClobberInstallError::Failed(_)) => {
                // The platform primitive may have made the target visible before returning its
                // durability/cleanup error. From this point onward, never report a normal error.
                let _ = cleanup_staged_archive(&mut staged_archive);
                Ok(Revision3ExactSnapshotExportPublicationV1::PublicationUncertain(plan.receipt))
            }
            Ok(()) => {
                // All remaining fallible work is folded into typed terminals. The caller must not
                // retry either terminal automatically because publication has definitely or may
                // have completed.
                if external_guard(ExportGuardPhase::AfterPublication, &output_guard.target).is_err()
                {
                    let _ = cleanup_staged_archive(&mut staged_archive);
                    return Ok(
                        Revision3ExactSnapshotExportPublicationV1::PublicationUncertain(
                            plan.receipt,
                        ),
                    );
                }
                let guard_cleanup_failed = external_guard(
                    ExportGuardPhase::BeforeStagingCleanup,
                    &staged_archive.display_path,
                )
                .is_err();
                let cleanup_warning =
                    guard_cleanup_failed || cleanup_staged_archive(&mut staged_archive).is_err();
                let final_verified = verify_published_archive(
                    &mut staged_archive.file,
                    &output_guard,
                    &plan.receipt.archive,
                )
                .is_ok();
                if !final_verified {
                    Ok(
                        Revision3ExactSnapshotExportPublicationV1::PublicationUncertain(
                            plan.receipt,
                        ),
                    )
                } else if cleanup_warning {
                    Ok(
                        Revision3ExactSnapshotExportPublicationV1::ExportedWithCleanupWarning(
                            plan.receipt,
                        ),
                    )
                } else {
                    Ok(Revision3ExactSnapshotExportPublicationV1::Exported(
                        plan.receipt,
                    ))
                }
            }
        }
    }

    fn prepare_export_output(
        &self,
        output: &Path,
    ) -> Result<OutputGuard, Revision3ExactSnapshotExportErrorV1> {
        if !output.is_absolute() {
            return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
                "output must be an absolute path".to_owned(),
            ));
        }
        if output
            .extension()
            .and_then(|value| value.to_str())
            .is_none_or(|value| !value.eq_ignore_ascii_case("goremod"))
        {
            return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
                "output must use the .goremod extension".to_owned(),
            ));
        }
        let normalized = normalize_absolute(output).map_err(|error| {
            Revision3ExactSnapshotExportErrorV1::InvalidOutput(error.to_string())
        })?;
        let parent = normalized.parent().ok_or_else(|| {
            Revision3ExactSnapshotExportErrorV1::InvalidOutput(
                "output has no parent directory".to_owned(),
            )
        })?;
        ensure_safe_directory_chain(parent).map_err(|error| {
            Revision3ExactSnapshotExportErrorV1::InvalidOutput(error.to_string())
        })?;
        let parent = fs::canonicalize(parent).map_err(|error| {
            Revision3ExactSnapshotExportErrorV1::InvalidOutput(error.to_string())
        })?;
        let filename = normalized
            .file_name()
            .ok_or_else(|| {
                Revision3ExactSnapshotExportErrorV1::InvalidOutput(
                    "output has no filename".to_owned(),
                )
            })?
            .to_owned();
        let target = parent.join(&filename);
        let store_root_dir = Dir::open_ambient_dir(&self.root, ambient_authority())?;
        let store_root_identity = directory_identity(&store_root_dir)?;
        if !ambient_directory_matches(&self.root, store_root_identity) {
            return Err(WorkingStoreError::UnsafePath {
                path: self.root.clone(),
                reason: "managed Store root changed while export output was validated".to_owned(),
            }
            .into());
        }
        let parent_dir = Dir::open_ambient_dir(&parent, ambient_authority()).map_err(|error| {
            Revision3ExactSnapshotExportErrorV1::InvalidOutput(error.to_string())
        })?;
        let parent_identity = directory_identity(&parent_dir).map_err(|error| {
            Revision3ExactSnapshotExportErrorV1::InvalidOutput(error.to_string())
        })?;
        if !ambient_directory_matches(&parent, parent_identity) {
            return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
                "output parent changed while its capability was pinned".to_owned(),
            ));
        }
        #[cfg(target_os = "linux")]
        let publication_parent = {
            let publication_parent = open_syncable_directory(&parent_dir).map_err(|error| {
                Revision3ExactSnapshotExportErrorV1::InvalidOutput(format!(
                    "output parent cannot be opened for durable publication: {error}"
                ))
            })?;
            let publication_identity = file_identity(&publication_parent).map_err(|error| {
                Revision3ExactSnapshotExportErrorV1::InvalidOutput(error.to_string())
            })?;
            if publication_identity != parent_identity {
                return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
                    "output parent changed while its durable publication handle was opened"
                        .to_owned(),
                ));
            }
            publication_parent
        };
        if directory_is_at_or_below(&parent_dir, store_root_identity).map_err(|error| {
            Revision3ExactSnapshotExportErrorV1::InvalidOutput(format!(
                "output ancestry could not be validated from its pinned directory: {error}"
            ))
        })? {
            return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
                "output must be outside the managed Store root".to_owned(),
            ));
        }
        match parent_dir.symlink_metadata(&filename) {
            Ok(_) => return Err(Revision3ExactSnapshotExportErrorV1::OutputAlreadyExists),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
                    error.to_string(),
                ));
            }
        }
        Ok(OutputGuard {
            target,
            parent_path: parent,
            filename,
            parent: parent_dir,
            parent_identity,
            store_root_identity,
            #[cfg(target_os = "linux")]
            publication_parent,
        })
    }

    fn plan_revision3_exact_snapshot_export(
        &self,
        expected_head: &WorkingHead,
    ) -> Result<ExportPlan, Revision3ExactSnapshotExportErrorV1> {
        let head_bytes = read_required_regular_bounded(
            &self.head_path(),
            self.limits.max_head_bytes,
            "managed export head bytes",
        )?;
        let disk_head: WorkingHead = parse_canonical_json(&head_bytes, "managed export head")?;
        if &disk_head != expected_head {
            return Err(WorkingStoreError::HeadConflict {
                expected: Some(expected_head.clone()),
                actual: Some(disk_head),
            }
            .into());
        }

        let mut closure = ClosureObjects::default();
        let mut pending = BTreeMap::<Sha256Digest, ContentSeal>::new();
        let mut known_snapshot_lengths = BTreeMap::<Sha256Digest, u64>::new();
        let mut known_snapshot_bytes = 0u64;
        let mut verification_objects = 0u64;
        let mut verification_bytes = 0u64;
        let mut current_project = None;
        enqueue_snapshot(
            &mut pending,
            &mut known_snapshot_lengths,
            &mut known_snapshot_bytes,
            expected_head.snapshot.clone(),
            self.limits.max_entities as u64,
            self.limits.max_referenced_entity_bytes,
        )?;

        while let Some((digest, seal)) = pending.pop_first() {
            // Hash and parse the unique manifest first, then conservatively charge every object
            // and byte the later Full reopen may touch. Shared assets repeated across historical
            // bases are deliberately charged repeatedly, so work is bounded even without a
            // cross-open verification cache.
            let preflight = self.inspect_revision3_dataasset_basis(&seal)?;
            charge_full_verification_work(
                &mut verification_objects,
                &mut verification_bytes,
                preflight.verification_objects,
                preflight.verification_bytes,
            )?;
            let opened = self.open_revision3_snapshot(&seal, AssetVerification::Full)?;
            if opened.head != preflight.head
                || opened.project.project_id != preflight.project_id
                || opened.project.target != preflight.target
                || opened.project.revision != preflight.revision
            {
                return Err(Revision3ExactSnapshotExportErrorV1::InvalidClosure(
                    "snapshot identity changed between bounded preflight and full reopen"
                        .to_owned(),
                ));
            }
            let snapshot_path = self.snapshot_path(digest);
            let manifest = preflight.manifest;
            let snapshot_relative = snapshot_member_name(digest);
            insert_closure_member(
                &mut closure.snapshots,
                PlannedMember {
                    relative_name: snapshot_relative,
                    seal: seal.clone(),
                    source: MemberSource::StoreObject(snapshot_path),
                },
                "snapshot object",
            )?;
            closure.snapshot_bytes = checked_export_sum(
                "aggregate snapshot bytes",
                closure.snapshot_bytes,
                seal.byte_len,
                self.limits.max_referenced_entity_bytes,
            )?;

            for (id, entity_seal) in &manifest.entities {
                let relative_name = entity_member_name(*id, entity_seal.sha256);
                if !closure.entities.contains_key(&relative_name)
                    && closure.entities.len() as u64 >= self.limits.max_entities as u64
                {
                    return Err(Revision3ExactSnapshotExportErrorV1::ClosureLimit {
                        kind: "entity object count",
                        actual: closure.entities.len() as u64 + 1,
                        limit: self.limits.max_entities as u64,
                    });
                }
                let inserted = insert_closure_member(
                    &mut closure.entities,
                    PlannedMember {
                        relative_name,
                        seal: entity_seal.clone(),
                        source: MemberSource::StoreObject(
                            self.entity_path(*id, entity_seal.sha256),
                        ),
                    },
                    "entity object",
                )?;
                if inserted {
                    closure.entity_bytes = checked_export_sum(
                        "aggregate entity object bytes",
                        closure.entity_bytes,
                        entity_seal.byte_len,
                        self.limits.max_referenced_entity_bytes,
                    )?;
                }
            }

            for (digest, meta) in &manifest.asset_store.assets {
                let relative_name = asset_member_name(*digest);
                if !closure.assets.contains_key(&relative_name)
                    && closure.assets.len() as u64 >= self.limits.max_assets as u64
                {
                    return Err(Revision3ExactSnapshotExportErrorV1::ClosureLimit {
                        kind: "asset object count",
                        actual: closure.assets.len() as u64 + 1,
                        limit: self.limits.max_assets as u64,
                    });
                }
                let asset_seal = ContentSeal {
                    byte_len: meta.byte_len,
                    sha256: *digest,
                };
                let inserted = insert_closure_member(
                    &mut closure.assets,
                    PlannedMember {
                        relative_name,
                        seal: asset_seal,
                        source: MemberSource::StoreObject(self.asset_path(*digest)),
                    },
                    "asset object",
                )?;
                if inserted {
                    closure.asset_bytes = checked_export_sum(
                        "aggregate asset object bytes",
                        closure.asset_bytes,
                        meta.byte_len,
                        self.limits.max_referenced_asset_bytes,
                    )?;
                }
            }

            let basis_snapshots = opened
                .project
                .entities
                .values()
                .filter_map(|entity| match &entity.payload {
                    Revision3EntityPayload::QuestDraft(quest) => {
                        Some(quest.input.collision_catalog.basis_snapshot.clone())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            for basis in basis_snapshots {
                enqueue_snapshot(
                    &mut pending,
                    &mut known_snapshot_lengths,
                    &mut known_snapshot_bytes,
                    basis,
                    self.limits.max_entities as u64,
                    self.limits.max_referenced_entity_bytes,
                )?;
            }
            if digest == expected_head.snapshot.sha256
                && current_project.replace(opened.project).is_some()
            {
                return Err(Revision3ExactSnapshotExportErrorV1::InvalidClosure(
                    "current snapshot appeared twice in the export closure".to_owned(),
                ));
            }
        }

        debug_assert_eq!(closure.snapshot_bytes, known_snapshot_bytes);
        let project = current_project.ok_or_else(|| {
            Revision3ExactSnapshotExportErrorV1::InvalidClosure(
                "current snapshot was absent from its own export closure".to_owned(),
            )
        })?;
        let project_json = project.to_canonical_json().map_err(|error| {
            Revision3ExactSnapshotExportErrorV1::InvalidClosure(format!(
                "current project could not be serialized canonically: {error}"
            ))
        })?;
        if project.revision > i64::MAX as u64 {
            return Err(Revision3ExactSnapshotExportErrorV1::InvalidClosure(
                "project revision exceeds the closed signed response range".to_owned(),
            ));
        }
        let project_bytes = project_json.into_bytes();

        let project_member = PlannedMember {
            relative_name: REVIEW_PROJECT_FILE.to_owned(),
            seal: seal_bytes(&project_bytes),
            source: MemberSource::Bytes(project_bytes),
        };
        let head_member = PlannedMember {
            relative_name: STORE_HEAD_MEMBER.to_owned(),
            seal: seal_bytes(&head_bytes),
            source: MemberSource::Bytes(head_bytes),
        };

        let mut sorted_member_seals = BTreeMap::<String, ExportMemberSealV1>::new();
        for member in std::iter::once(&project_member)
            .chain(std::iter::once(&head_member))
            .chain(closure.snapshots.values())
            .chain(closure.entities.values())
            .chain(closure.assets.values())
        {
            let seal = ExportMemberSealV1 {
                relative_name: member.relative_name.clone(),
                byte_len: member.seal.byte_len,
                sha256: member.seal.sha256,
            };
            if sorted_member_seals
                .insert(member.relative_name.clone(), seal)
                .is_some()
            {
                return Err(Revision3ExactSnapshotExportErrorV1::InvalidClosure(
                    "two export members have the same relative name".to_owned(),
                ));
            }
        }

        let manifest = ExactSnapshotManifestV1 {
            format: ExportManifestFormatV1::ExactSnapshotV1,
            schema: ExportManifestSchemaV1,
            artifact_kind: ExportArtifactKindV1::PortableSnapshotReviewCopy,
            restore_status: ExportRestoreStatusV1::NotSupported,
            basis: ExportBasisV1 {
                head: expected_head.clone(),
                project_id: project.project_id,
                project_revision: project.revision,
            },
            members: sorted_member_seals.into_values().collect(),
        };
        validate_manifest_members(&manifest)?;
        let manifest_bytes = canonical_json(&manifest)?;
        if manifest_bytes.len() > MAX_EXPORT_MANIFEST_BYTES_V1 {
            return Err(Revision3ExactSnapshotExportErrorV1::ClosureLimit {
                kind: "export manifest bytes",
                actual: manifest_bytes.len() as u64,
                limit: MAX_EXPORT_MANIFEST_BYTES_V1 as u64,
            });
        }
        let manifest_seal = seal_bytes(&manifest_bytes);
        let manifest_member = PlannedMember {
            relative_name: REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V1.to_owned(),
            seal: manifest_seal.clone(),
            source: MemberSource::Bytes(manifest_bytes),
        };

        let snapshot_objects = closure.snapshots.len() as u64;
        let entity_objects = closure.entities.len() as u64;
        let asset_objects = closure.assets.len() as u64;
        let archive_entries = 3u64
            .checked_add(snapshot_objects)
            .and_then(|value| value.checked_add(entity_objects))
            .and_then(|value| value.checked_add(asset_objects))
            .ok_or(Revision3ExactSnapshotExportErrorV1::ClosureLimit {
                kind: "archive entry count",
                actual: u64::MAX,
                limit: MAX_EXPORT_ARCHIVE_ENTRIES_V1,
            })?;
        if archive_entries > MAX_EXPORT_ARCHIVE_ENTRIES_V1 {
            return Err(Revision3ExactSnapshotExportErrorV1::ClosureLimit {
                kind: "archive entry count",
                actual: archive_entries,
                limit: MAX_EXPORT_ARCHIVE_ENTRIES_V1,
            });
        }
        let mut uncompressed_bytes = manifest_seal.byte_len;
        for member in std::iter::once(&project_member)
            .chain(std::iter::once(&head_member))
            .chain(closure.snapshots.values())
            .chain(closure.entities.values())
            .chain(closure.assets.values())
        {
            uncompressed_bytes = uncompressed_bytes.checked_add(member.seal.byte_len).ok_or(
                Revision3ExactSnapshotExportErrorV1::ClosureLimit {
                    kind: "archive uncompressed bytes",
                    actual: u64::MAX,
                    limit: u64::MAX - 1,
                },
            )?;
        }

        let mut members = Vec::with_capacity(usize::try_from(archive_entries).map_err(|_| {
            Revision3ExactSnapshotExportErrorV1::ClosureLimit {
                kind: "archive entry count",
                actual: archive_entries,
                limit: usize::MAX as u64,
            }
        })?);
        members.push(manifest_member);
        members.push(project_member);
        members.push(head_member);
        members.extend(closure.snapshots.into_values());
        members.extend(closure.entities.into_values());
        members.extend(closure.assets.into_values());

        Ok(ExportPlan {
            receipt: Revision3ExactSnapshotExportV1 {
                head: expected_head.clone(),
                project_id: project.project_id,
                project_revision: project.revision,
                // Filled after strict staged reopen, still before publication.
                archive: ContentSeal {
                    byte_len: 0,
                    sha256: Sha256Digest::from_bytes([0; 32]),
                },
                manifest: manifest_seal,
                closure: Revision3ExactSnapshotClosureV1 {
                    snapshot_objects,
                    entity_objects,
                    asset_objects,
                    archive_entries,
                    uncompressed_bytes,
                },
            },
            members,
            manifest,
            project,
        })
    }
}

fn enqueue_snapshot(
    pending: &mut BTreeMap<Sha256Digest, ContentSeal>,
    known_lengths: &mut BTreeMap<Sha256Digest, u64>,
    known_bytes: &mut u64,
    seal: ContentSeal,
    max_count: u64,
    max_bytes: u64,
) -> Result<(), Revision3ExactSnapshotExportErrorV1> {
    if let Some(existing) = known_lengths.get(&seal.sha256) {
        if *existing != seal.byte_len {
            return Err(Revision3ExactSnapshotExportErrorV1::InvalidClosure(
                format!(
                    "snapshot {} has conflicting lengths {existing} and {}",
                    seal.sha256, seal.byte_len
                ),
            ));
        }
        return Ok(());
    }
    let actual_count = known_lengths.len() as u64 + 1;
    if actual_count > max_count {
        return Err(Revision3ExactSnapshotExportErrorV1::ClosureLimit {
            kind: "snapshot object count",
            actual: actual_count,
            limit: max_count,
        });
    }
    let next_bytes = checked_export_sum(
        "aggregate snapshot bytes",
        *known_bytes,
        seal.byte_len,
        max_bytes,
    )?;
    known_lengths.insert(seal.sha256, seal.byte_len);
    pending.insert(seal.sha256, seal);
    *known_bytes = next_bytes;
    Ok(())
}

fn insert_closure_member(
    members: &mut BTreeMap<String, PlannedMember>,
    member: PlannedMember,
    kind: &'static str,
) -> Result<bool, Revision3ExactSnapshotExportErrorV1> {
    if let Some(existing) = members.get(&member.relative_name) {
        if existing.seal != member.seal {
            return Err(Revision3ExactSnapshotExportErrorV1::InvalidClosure(
                format!("{kind} {} has conflicting seals", member.relative_name),
            ));
        }
        return Ok(false);
    }
    members.insert(member.relative_name.clone(), member);
    Ok(true)
}

fn checked_export_sum(
    kind: &'static str,
    current: u64,
    addition: u64,
    limit: u64,
) -> Result<u64, Revision3ExactSnapshotExportErrorV1> {
    let actual = current.checked_add(addition).unwrap_or(u64::MAX);
    if actual > limit {
        Err(Revision3ExactSnapshotExportErrorV1::ClosureLimit {
            kind,
            actual,
            limit,
        })
    } else {
        Ok(actual)
    }
}

fn charge_full_verification_work(
    objects: &mut u64,
    bytes: &mut u64,
    additional_objects: u64,
    additional_bytes: u64,
) -> Result<(), Revision3ExactSnapshotExportErrorV1> {
    let next_objects = checked_export_sum(
        "full-verification objects",
        *objects,
        additional_objects,
        MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1,
    )?;
    let next_bytes = checked_export_sum(
        "full-verification bytes",
        *bytes,
        additional_bytes,
        MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1,
    )?;
    *objects = next_objects;
    *bytes = next_bytes;
    Ok(())
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

fn validate_manifest_members(
    manifest: &ExactSnapshotManifestV1,
) -> Result<(), Revision3ExactSnapshotExportErrorV1> {
    if manifest.format != ExportManifestFormatV1::ExactSnapshotV1
        || manifest.schema != ExportManifestSchemaV1
        || manifest.artifact_kind != ExportArtifactKindV1::PortableSnapshotReviewCopy
        || manifest.restore_status != ExportRestoreStatusV1::NotSupported
        || manifest.basis.head.snapshot.byte_len == 0
    {
        return Err(Revision3ExactSnapshotExportErrorV1::Verification(
            "export manifest marker or basis is invalid".to_owned(),
        ));
    }
    let mut previous: Option<&str> = None;
    let mut folded = BTreeSet::new();
    for member in &manifest.members {
        if member.relative_name == REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V1
            || member.relative_name.is_empty()
            || member.byte_len == 0
        {
            return Err(Revision3ExactSnapshotExportErrorV1::Verification(
                "export manifest contains an invalid or self member".to_owned(),
            ));
        }
        if previous.is_some_and(|value| value >= member.relative_name.as_str()) {
            return Err(Revision3ExactSnapshotExportErrorV1::Verification(
                "export manifest members are not strictly sorted".to_owned(),
            ));
        }
        if !folded.insert(member.relative_name.to_ascii_lowercase()) {
            return Err(Revision3ExactSnapshotExportErrorV1::Verification(
                "export manifest members collide case-insensitively".to_owned(),
            ));
        }
        previous = Some(&member.relative_name);
    }
    Ok(())
}

fn write_deterministic_archive(
    file: &mut File,
    members: &[PlannedMember],
) -> Result<(), Revision3ExactSnapshotExportErrorV1> {
    let timestamp = DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).map_err(|error| {
        Revision3ExactSnapshotExportErrorV1::Archive(format!(
            "fixed ZIP timestamp is invalid: {error:?}"
        ))
    })?;
    file.set_len(0)
        .map_err(|error| Revision3ExactSnapshotExportErrorV1::Archive(error.to_string()))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| Revision3ExactSnapshotExportErrorV1::Archive(error.to_string()))?;
    let writer_file = file
        .try_clone()
        .map_err(|error| Revision3ExactSnapshotExportErrorV1::Archive(error.to_string()))?;
    let mut writer = ZipWriter::new(writer_file);
    // Force the closed v1 artifact to ZIP64 even for small projects. This keeps the container
    // dialect invariant instead of making it depend on one project's object sizes/counts.
    writer.set_zip64_comment(Some(""));
    for member in members {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(timestamp)
            .unix_permissions(ZIP_FILE_MODE)
            .large_file(true);
        writer
            .start_file(&member.relative_name, options)
            .map_err(|error| Revision3ExactSnapshotExportErrorV1::Archive(error.to_string()))?;
        match &member.source {
            MemberSource::Bytes(bytes) => {
                if seal_bytes(bytes) != member.seal {
                    return Err(Revision3ExactSnapshotExportErrorV1::InvalidClosure(
                        format!("in-memory member {} changed", member.relative_name),
                    ));
                }
                writer.write_all(bytes).map_err(|error| {
                    Revision3ExactSnapshotExportErrorV1::Archive(error.to_string())
                })?;
            }
            MemberSource::StoreObject(path) => {
                copy_sealed_store_object(path, &member.seal, &mut writer)?;
            }
        }
    }
    let mut file = writer
        .finish()
        .map_err(|error| Revision3ExactSnapshotExportErrorV1::Archive(error.to_string()))?;
    file.flush()
        .map_err(|error| Revision3ExactSnapshotExportErrorV1::Archive(error.to_string()))?;
    file.sync_all()
        .map_err(|error| Revision3ExactSnapshotExportErrorV1::Archive(error.to_string()))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| Revision3ExactSnapshotExportErrorV1::Archive(error.to_string()))?;
    Ok(())
}

fn copy_sealed_store_object<W: Write>(
    path: &Path,
    seal: &ContentSeal,
    output: &mut W,
) -> Result<(), Revision3ExactSnapshotExportErrorV1> {
    ensure_safe_existing_chain(path)?;
    let metadata = fs::symlink_metadata(path)?;
    ensure_regular_no_link(path, &metadata)?;
    ensure_single_link(path, &metadata)?;
    if metadata.len() != seal.byte_len {
        return Err(WorkingStoreError::SealMismatch {
            path: path.to_path_buf(),
            reason: format!("expected {} bytes, found {}", seal.byte_len, metadata.len()),
        }
        .into());
    }
    let mut source = open_regular_read_no_follow(path)?;
    let opened = source.metadata()?;
    ensure_regular_no_link(path, &opened)?;
    ensure_single_link(path, &opened)?;
    if opened.len() != seal.byte_len {
        return Err(WorkingStoreError::SealMismatch {
            path: path.to_path_buf(),
            reason: "object length changed before export streaming".to_owned(),
        }
        .into());
    }

    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or_else(|| {
            Revision3ExactSnapshotExportErrorV1::InvalidClosure(
                "object length overflow while streaming export".to_owned(),
            )
        })?;
        if total > seal.byte_len {
            return Err(WorkingStoreError::SealMismatch {
                path: path.to_path_buf(),
                reason: "object grew while streaming export".to_owned(),
            }
            .into());
        }
        hasher.update(&buffer[..count]);
        output
            .write_all(&buffer[..count])
            .map_err(|error| Revision3ExactSnapshotExportErrorV1::Archive(error.to_string()))?;
    }
    let actual = Sha256Digest::from_bytes(hasher.finalize().into());
    if total != seal.byte_len || actual != seal.sha256 {
        return Err(WorkingStoreError::SealMismatch {
            path: path.to_path_buf(),
            reason: format!(
                "streamed object differs from expected {} bytes / SHA-256 {}",
                seal.byte_len, seal.sha256
            ),
        }
        .into());
    }
    Ok(())
}

fn verify_staged_archive(
    file: &mut File,
    expected: &[PlannedMember],
    expected_manifest: &ExactSnapshotManifestV1,
    expected_project: &ProjectRevision3,
) -> Result<ContentSeal, Revision3ExactSnapshotExportErrorV1> {
    let metadata = file.metadata().map_err(staged_verification_error)?;
    if !metadata.is_file() {
        return Err(Revision3ExactSnapshotExportErrorV1::Verification(
            "staged ZIP handle is not a regular file".to_owned(),
        ));
    }
    if metadata.len() == 0 {
        return Err(Revision3ExactSnapshotExportErrorV1::Verification(
            "staged ZIP is empty".to_owned(),
        ));
    }
    let mut reader = file.try_clone().map_err(staged_verification_error)?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(staged_verification_error)?;
    let mut archive = ZipArchive::new(reader)
        .map_err(|error| Revision3ExactSnapshotExportErrorV1::Verification(error.to_string()))?;
    if archive.len() != expected.len() || !archive.comment().is_empty() {
        return Err(Revision3ExactSnapshotExportErrorV1::Verification(
            "staged ZIP entry count or comment differs".to_owned(),
        ));
    }
    let timestamp = DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).map_err(|error| {
        Revision3ExactSnapshotExportErrorV1::Verification(format!(
            "fixed ZIP timestamp is invalid: {error:?}"
        ))
    })?;
    let mut manifest_bytes = None;
    let mut project_bytes = None;
    let mut head_bytes = None;
    let mut raw_members = Vec::with_capacity(expected.len());
    for (index, member) in expected.iter().enumerate() {
        {
            let entry = archive.by_index_raw(index).map_err(|error| {
                Revision3ExactSnapshotExportErrorV1::Verification(error.to_string())
            })?;
            if entry.name() != member.relative_name
                || entry.name_raw() != member.relative_name.as_bytes()
                || !entry.comment().is_empty()
                || entry.is_dir()
                || entry.encrypted()
                || entry.compression() != CompressionMethod::Stored
                || entry.size() != member.seal.byte_len
                || entry.compressed_size() != member.seal.byte_len
                || entry.last_modified() != Some(timestamp)
                || entry.unix_mode().map(|mode| mode & 0o777) != Some(ZIP_FILE_MODE)
            {
                return Err(Revision3ExactSnapshotExportErrorV1::Verification(format!(
                    "staged ZIP metadata differs at entry {index}"
                )));
            }
            raw_members.push(RawZipMemberExpectation {
                name: member.relative_name.as_bytes().to_vec(),
                byte_len: member.seal.byte_len,
                crc32: entry.crc32(),
            });
        }
        let mut entry = archive.by_index(index).map_err(|error| {
            Revision3ExactSnapshotExportErrorV1::Verification(error.to_string())
        })?;
        let collect = index <= 2;
        let mut bytes = collect.then(|| Vec::with_capacity(member.seal.byte_len as usize));
        let mut hasher = Sha256::new();
        let mut total = 0u64;
        let mut buffer = [0u8; COPY_BUFFER_BYTES];
        loop {
            let count = entry.read(&mut buffer).map_err(|error| {
                Revision3ExactSnapshotExportErrorV1::Verification(error.to_string())
            })?;
            if count == 0 {
                break;
            }
            total = total.checked_add(count as u64).ok_or_else(|| {
                Revision3ExactSnapshotExportErrorV1::Verification(
                    "staged ZIP payload length overflow".to_owned(),
                )
            })?;
            if total > member.seal.byte_len {
                return Err(Revision3ExactSnapshotExportErrorV1::Verification(format!(
                    "staged ZIP payload grew at entry {index}"
                )));
            }
            hasher.update(&buffer[..count]);
            if let Some(bytes) = &mut bytes {
                bytes.extend_from_slice(&buffer[..count]);
            }
        }
        if total != member.seal.byte_len
            || Sha256Digest::from_bytes(hasher.finalize().into()) != member.seal.sha256
        {
            return Err(Revision3ExactSnapshotExportErrorV1::Verification(format!(
                "staged ZIP payload seal differs at entry {index}"
            )));
        }
        match index {
            0 => manifest_bytes = bytes,
            1 => project_bytes = bytes,
            2 => head_bytes = bytes,
            _ => {}
        }
    }
    drop(archive);
    verify_exact_zip_layout(file, metadata.len(), &raw_members)?;

    let manifest: ExactSnapshotManifestV1 = parse_canonical_json(
        manifest_bytes.as_deref().ok_or_else(|| {
            Revision3ExactSnapshotExportErrorV1::Verification(
                "staged ZIP has no manifest payload".to_owned(),
            )
        })?,
        "managed exact snapshot export manifest",
    )
    .map_err(staged_verification_error)?;
    validate_manifest_members(&manifest)?;
    if &manifest != expected_manifest {
        return Err(Revision3ExactSnapshotExportErrorV1::Verification(
            "staged ZIP manifest differs from its prepared marker".to_owned(),
        ));
    }
    let project_text = std::str::from_utf8(project_bytes.as_deref().ok_or_else(|| {
        Revision3ExactSnapshotExportErrorV1::Verification(
            "staged ZIP has no review project payload".to_owned(),
        )
    })?)
    .map_err(|_| {
        Revision3ExactSnapshotExportErrorV1::Verification(
            "staged review project is not UTF-8".to_owned(),
        )
    })?;
    let project = ProjectRevision3::from_json(project_text).map_err(|error| {
        Revision3ExactSnapshotExportErrorV1::Verification(format!(
            "staged review project is invalid: {error}"
        ))
    })?;
    if &project != expected_project
        || project.project_id != manifest.basis.project_id
        || project.revision != manifest.basis.project_revision
    {
        return Err(Revision3ExactSnapshotExportErrorV1::Verification(
            "staged review project differs from the exact manifest basis".to_owned(),
        ));
    }
    let head: WorkingHead = parse_canonical_json(
        head_bytes.as_deref().ok_or_else(|| {
            Revision3ExactSnapshotExportErrorV1::Verification(
                "staged ZIP has no Store head payload".to_owned(),
            )
        })?,
        "managed exact snapshot export Store head",
    )
    .map_err(staged_verification_error)?;
    if head != manifest.basis.head {
        return Err(Revision3ExactSnapshotExportErrorV1::Verification(
            "staged Store head differs from the exact manifest basis".to_owned(),
        ));
    }

    let byte_len = file.metadata().map_err(staged_verification_error)?.len();
    let sha256 = hash_open_file(file, byte_len).map_err(staged_verification_error)?;
    Ok(ContentSeal { byte_len, sha256 })
}

fn verify_exact_zip_layout(
    file: &File,
    byte_len: u64,
    members: &[RawZipMemberExpectation],
) -> Result<(), Revision3ExactSnapshotExportErrorV1> {
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
        read_exact_file_at(file, &mut header, local_offset).map_err(staged_verification_error)?;
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
            .map_err(staged_verification_error)?;
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
        read_exact_file_at(file, &mut header, central_cursor).map_err(staged_verification_error)?;
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
            .map_err(staged_verification_error)?;
        if variable[..member.name.len()] != member.name || variable[member.name.len()..] != extra {
            return Err(layout_error(format!(
                "member {index} central raw name or ZIP64 extra differs"
            )));
        }
        central_cursor = checked_layout_add(central_cursor, CENTRAL_FIXED_BYTES, "central header")?;
        central_cursor =
            checked_layout_add(central_cursor, u64::from(name_len), "central raw name")?;
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
) -> Result<(), Revision3ExactSnapshotExportErrorV1> {
    const ZIP64_EOCD_BYTES: usize = 56;
    const ZIP64_LOCATOR_BYTES: usize = 20;
    const LEGACY_EOCD_BYTES: usize = 22;
    let mut footer = [0u8; ZIP64_EOCD_BYTES + ZIP64_LOCATOR_BYTES + LEGACY_EOCD_BYTES];
    read_exact_file_at(file, &mut footer, footer_offset).map_err(staged_verification_error)?;
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
) -> Result<u64, Revision3ExactSnapshotExportErrorV1> {
    current
        .checked_add(addition)
        .ok_or_else(|| layout_error(format!("{kind} offset overflowed")))
}

fn layout_error(reason: String) -> Revision3ExactSnapshotExportErrorV1 {
    Revision3ExactSnapshotExportErrorV1::Verification(reason)
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

fn staged_verification_error(error: impl std::fmt::Display) -> Revision3ExactSnapshotExportErrorV1 {
    Revision3ExactSnapshotExportErrorV1::Verification(error.to_string())
}

fn hash_open_file(file: &File, expected_len: u64) -> io::Result<Sha256Digest> {
    let mut reader = file.try_clone()?;
    reader.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "open file length overflow")
        })?;
        if total > expected_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "open file grew while hashing",
            ));
        }
        hasher.update(&buffer[..count]);
    }
    if total != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "open file length differs while hashing",
        ));
    }
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}

#[cfg(unix)]
fn directory_identity(dir: &Dir) -> io::Result<DirectoryIdentity> {
    let metadata = dir.dir_metadata()?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "pinned output parent is not a directory",
        ));
    }
    cap_metadata_identity(&metadata)
}

#[cfg(windows)]
fn directory_identity(dir: &Dir) -> io::Result<DirectoryIdentity> {
    let metadata = dir.dir_metadata()?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "pinned output parent is not a directory",
        ));
    }
    let file = dir.try_clone()?.into_std_file();
    file_identity(&file)
}

#[cfg(unix)]
fn cap_metadata_identity(metadata: &cap_std::fs::Metadata) -> io::Result<DirectoryIdentity> {
    use cap_std::fs::MetadataExt as _;

    Ok(DirectoryIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(unix)]
fn file_identity(file: &File) -> io::Result<DirectoryIdentity> {
    use std::os::unix::fs::MetadataExt as _;

    let metadata = file.metadata()?;
    Ok(DirectoryIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(target_os = "linux")]
fn open_syncable_directory(dir: &Dir) -> io::Result<File> {
    use std::os::fd::{AsRawFd, FromRawFd};

    let parent = dir.try_clone()?.into_std_file();
    // SAFETY: parent is live and the relative C string is terminated. Opening "." from the
    // already-pinned descriptor cannot redirect through an ambient path.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            c".".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: fd is newly returned and uniquely transferred into File.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

#[cfg(windows)]
fn file_identity(file: &File) -> io::Result<DirectoryIdentity> {
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: info points to writable storage for the exact ABI structure and file is live.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as HANDLE, info.as_mut_ptr()) } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a successful call initializes the entire structure.
    let info = unsafe { info.assume_init() };
    Ok(DirectoryIdentity {
        device: u64::from(info.dwVolumeSerialNumber),
        inode: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    })
}

fn directory_is_at_or_below(
    directory: &Dir,
    possible_ancestor: DirectoryIdentity,
) -> io::Result<bool> {
    let mut current = directory.try_clone()?;
    // Native path component limits are far below this. The bound also fails closed if a platform
    // ever exposes a cyclic parent relationship that does not terminate at a self-parent root.
    for _ in 0..4_096 {
        let current_identity = directory_identity(&current)?;
        if current_identity == possible_ancestor {
            return Ok(true);
        }
        let parent = current.open_parent_dir(ambient_authority())?;
        let parent_identity = directory_identity(&parent)?;
        if parent_identity == current_identity {
            return Ok(false);
        }
        current = parent;
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "directory ancestry exceeded the closed traversal bound",
    ))
}

fn ambient_directory_matches(path: &Path, expected: DirectoryIdentity) -> bool {
    Dir::open_ambient_dir(path, ambient_authority())
        .and_then(|dir| directory_identity(&dir))
        .is_ok_and(|actual| actual == expected)
}

fn revalidate_export_output(
    guard: &OutputGuard,
) -> Result<(), Revision3ExactSnapshotExportErrorV1> {
    if directory_is_at_or_below(&guard.parent, guard.store_root_identity).map_err(|error| {
        Revision3ExactSnapshotExportErrorV1::InvalidOutput(format!(
            "output ancestry could not be revalidated from its pinned directory: {error}"
        ))
    })? {
        return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
            "output parent moved inside the managed Store root during export".to_owned(),
        ));
    }
    ensure_safe_directory_chain(&guard.parent_path)
        .map_err(|error| Revision3ExactSnapshotExportErrorV1::InvalidOutput(error.to_string()))?;
    if !ambient_directory_matches(&guard.parent_path, guard.parent_identity) {
        return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
            "output parent identity changed during export".to_owned(),
        ));
    }
    match guard.parent.symlink_metadata(&guard.filename) {
        Ok(_) => Err(Revision3ExactSnapshotExportErrorV1::OutputAlreadyExists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
            error.to_string(),
        )),
    }
}

#[cfg(target_os = "linux")]
fn create_staged_archive(
    guard: &OutputGuard,
) -> Result<StagedArchive, Revision3ExactSnapshotExportErrorV1> {
    use std::os::fd::{AsRawFd, FromRawFd};

    if !ambient_directory_matches(&guard.parent_path, guard.parent_identity) {
        return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
            "output parent identity changed before staging creation".to_owned(),
        ));
    }
    let parent = guard
        .parent
        .try_clone()
        .map_err(|error| Revision3ExactSnapshotExportErrorV1::InvalidOutput(error.to_string()))?
        .into_std_file();
    // SAFETY: parent is a live directory descriptor, the C string is terminated, and a successful
    // descriptor is immediately owned by File. O_TMPFILE creates no directory entry to clean up.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            c".".as_ptr(),
            libc::O_TMPFILE | libc::O_RDWR | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(format!(
            "output parent cannot host an anonymous private staging file: {}",
            io::Error::last_os_error()
        )));
    }
    // SAFETY: fd is newly returned and uniquely transferred into File.
    let file = unsafe { File::from_raw_fd(fd) };
    Ok(StagedArchive {
        file,
        display_path: guard.parent_path.join(".gore-export-anonymous.staging"),
        published: false,
    })
}

#[cfg(windows)]
fn create_staged_archive(
    guard: &OutputGuard,
) -> Result<StagedArchive, Revision3ExactSnapshotExportErrorV1> {
    use cap_std::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_TEMPORARY,
    };

    if !ambient_directory_matches(&guard.parent_path, guard.parent_identity) {
        return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
            "output parent identity changed before staging creation".to_owned(),
        ));
    }
    for _ in 0..128 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let staging_name = OsString::from(format!(
            ".gore-export-{}-{sequence:016x}.staging",
            std::process::id()
        ));
        let mut options = CapOpenOptions::new();
        options
            .read(true)
            .write(true)
            .create_new(true)
            .access_mode(GENERIC_READ | GENERIC_WRITE | DELETE)
            .share_mode(0)
            .attributes(FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_TEMPORARY);
        match guard.parent.open_with(&staging_name, &options) {
            Ok(file) => {
                return Ok(StagedArchive {
                    file: file.into_std(),
                    display_path: guard.parent_path.join(&staging_name),
                    published: false,
                });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(format!(
                    "output parent cannot host a private staging file: {error}"
                )));
            }
        }
    }
    Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(
        "could not allocate a unique handle-locked staging file".to_owned(),
    ))
}

#[cfg(all(unix, not(target_os = "linux")))]
fn create_staged_archive(
    _guard: &OutputGuard,
) -> Result<StagedArchive, Revision3ExactSnapshotExportErrorV1> {
    Err(Revision3ExactSnapshotExportErrorV1::Archive(
        "this Unix target lacks the required anonymous handle-relative publication primitive"
            .to_owned(),
    ))
}

#[cfg(target_os = "linux")]
fn publish_staged_archive_no_clobber(
    staged: &mut StagedArchive,
    guard: &OutputGuard,
) -> Result<(), NoClobberInstallError> {
    use std::os::fd::AsRawFd;
    use std::os::unix::ffi::OsStrExt;

    let filename = std::ffi::CString::new(guard.filename.as_bytes()).map_err(|_| {
        NoClobberInstallError::Failed(WorkingStoreError::Invariant(
            "export filename contains NUL".to_owned(),
        ))
    })?;
    // SAFETY: both descriptors are live, both names are terminated, AT_EMPTY_PATH binds the
    // exact verified anonymous inode, and linkat without replacement flags is no-clobber.
    let linked = unsafe {
        libc::linkat(
            staged.file.as_raw_fd(),
            c"".as_ptr(),
            guard.publication_parent.as_raw_fd(),
            filename.as_ptr(),
            libc::AT_EMPTY_PATH,
        )
    };
    if linked != 0 {
        return Err(classify_no_clobber_io(io::Error::last_os_error()));
    }
    staged.published = true;
    guard.publication_parent.sync_all()?;
    Ok(())
}

#[cfg(windows)]
fn publish_staged_archive_no_clobber(
    staged: &mut StagedArchive,
    guard: &OutputGuard,
) -> Result<(), NoClobberInstallError> {
    use std::mem::{offset_of, size_of};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Wdk::Storage::FileSystem::{
        FileRenameInformation, NtSetInformationFile, FILE_RENAME_INFORMATION,
    };
    use windows_sys::Win32::Foundation::{RtlNtStatusToDosError, HANDLE};
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

    let name = guard.filename.encode_wide().collect::<Vec<_>>();
    let header_bytes = offset_of!(FILE_RENAME_INFORMATION, FileName);
    let byte_len = header_bytes + name.len() * size_of::<u16>();
    let mut storage = vec![0u64; byte_len.div_ceil(size_of::<u64>())];
    let info = storage.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    let mut io_status = IO_STATUS_BLOCK::default();
    // SAFETY: storage is u64-aligned and large enough for the fixed header plus the exact UTF-16
    // filename. NtSetInformationFile honors RootDirectory, so publication stays relative to the
    // pinned output directory. Both file and parent handles remain live through the call.
    let parent = guard.parent.try_clone()?.into_std_file();
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
            staged.file.as_raw_handle() as HANDLE,
            &mut io_status,
            info.cast_const().cast(),
            byte_len as u32,
            FileRenameInformation,
        );
        if status < 0 {
            let error = io::Error::from_raw_os_error(RtlNtStatusToDosError(status) as i32);
            return Err(classify_no_clobber_io(error));
        }
    }
    staged.published = true;
    staged.file.sync_all()?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "linux")))]
fn publish_staged_archive_no_clobber(
    _staged: &mut StagedArchive,
    _guard: &OutputGuard,
) -> Result<(), NoClobberInstallError> {
    Err(NoClobberInstallError::Failed(WorkingStoreError::Invariant(
        "unsupported Unix handle-relative publication".to_owned(),
    )))
}

#[cfg(target_os = "linux")]
fn cleanup_staged_archive(_staged: &mut StagedArchive) -> io::Result<()> {
    // O_TMPFILE has no staging directory entry. Dropping the final handle is the cleanup.
    Ok(())
}

#[cfg(windows)]
fn cleanup_staged_archive(staged: &mut StagedArchive) -> io::Result<()> {
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
    };

    if staged.published {
        return Ok(());
    }
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: disposition has the exact ABI type and the stage handle was opened with DELETE.
    let deleted = unsafe {
        SetFileInformationByHandle(
            staged.file.as_raw_handle() as HANDLE,
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

#[cfg(all(unix, not(target_os = "linux")))]
fn cleanup_staged_archive(_staged: &mut StagedArchive) -> io::Result<()> {
    Ok(())
}

fn verify_published_archive(
    file: &mut File,
    guard: &OutputGuard,
    expected: &ContentSeal,
) -> Result<(), WorkingStoreError> {
    let staged_metadata = file.metadata()?;
    if !staged_metadata.is_file() || staged_metadata.len() != expected.byte_len {
        return Err(WorkingStoreError::SealMismatch {
            path: guard.target.clone(),
            reason: "published archive handle length/type differs from staged archive".to_owned(),
        });
    }
    let actual = hash_open_file(file, expected.byte_len)?;
    if actual != expected.sha256 {
        return Err(WorkingStoreError::SealMismatch {
            path: guard.target.clone(),
            reason: "published archive handle hash differs from staged archive".to_owned(),
        });
    }
    if !ambient_directory_matches(&guard.parent_path, guard.parent_identity) {
        return Err(WorkingStoreError::UnsafePath {
            path: guard.parent_path.clone(),
            reason: "ambient output parent no longer names the pinned publication directory"
                .to_owned(),
        });
    }
    verify_published_name(file, guard, expected)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn verify_published_name(
    staged: &File,
    guard: &OutputGuard,
    expected: &ContentSeal,
) -> Result<(), WorkingStoreError> {
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;

    let filename = std::ffi::CString::new(guard.filename.as_bytes()).map_err(|_| {
        WorkingStoreError::UnsafePath {
            path: guard.target.clone(),
            reason: "published export filename contains NUL".to_owned(),
        }
    })?;
    let parent = guard.parent.try_clone()?.into_std_file();
    // SAFETY: parent and filename are live for the call. O_NOFOLLOW prevents a replaced final
    // symlink from redirecting verification, and O_NONBLOCK prevents a foreign special node from
    // blocking before the regular-file and identity checks reject it.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            filename.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: fd is newly returned and uniquely transferred into File.
    let named = unsafe { File::from_raw_fd(fd) };
    verify_named_archive_handle(staged, &named, guard, expected)
}

#[cfg(windows)]
fn verify_published_name(
    staged: &File,
    guard: &OutputGuard,
    expected: &ContentSeal,
) -> Result<(), WorkingStoreError> {
    use cap_std::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    // The staged handle was opened with share_mode(0), so while it is live Windows forbids a
    // second readable handle and also forbids rename/delete replacement. A no-follow metadata
    // lookup requests no data access, yet still proves that the pinned parent name identifies the
    // exact published file.
    let mut options = CapOpenOptions::new();
    options
        .access_mode(0)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let named = guard
        .parent
        .open_with(&guard.filename, &options)?
        .into_std();
    let named_metadata = named.metadata()?;
    let named_identity = file_identity(&named)?;
    let staged_identity = file_identity(staged)?;
    if !named_metadata.is_file()
        || named_metadata.len() != expected.byte_len
        || named_identity != staged_identity
    {
        return Err(WorkingStoreError::SealMismatch {
            path: guard.target.clone(),
            reason: "pinned output name does not identify the verified staged archive".to_owned(),
        });
    }
    Ok(())
}

#[cfg(all(unix, not(target_os = "linux")))]
fn verify_published_name(
    _staged: &File,
    _guard: &OutputGuard,
    _expected: &ContentSeal,
) -> Result<(), WorkingStoreError> {
    Err(WorkingStoreError::Invariant(
        "unsupported Unix handle-relative publication verification".to_owned(),
    ))
}

#[cfg(target_os = "linux")]
fn verify_named_archive_handle(
    staged: &File,
    named: &File,
    guard: &OutputGuard,
    expected: &ContentSeal,
) -> Result<(), WorkingStoreError> {
    use std::os::unix::fs::MetadataExt as _;

    let metadata = named.metadata()?;
    if !metadata.is_file()
        || metadata.len() != expected.byte_len
        || metadata.nlink() != 1
        || file_identity(named)? != file_identity(staged)?
        || hash_open_file(named, expected.byte_len)? != expected.sha256
    {
        return Err(WorkingStoreError::SealMismatch {
            path: guard.target.clone(),
            reason: "pinned output name does not identify the exact verified archive".to_owned(),
        });
    }
    Ok(())
}

fn abort_staged_archive(
    staged: &mut StagedArchive,
    primary: Revision3ExactSnapshotExportErrorV1,
) -> Revision3ExactSnapshotExportErrorV1 {
    preserve_primary_through_cleanup(primary, cleanup_staged_archive(staged))
}

fn preserve_primary_through_cleanup(
    primary: Revision3ExactSnapshotExportErrorV1,
    cleanup: io::Result<()>,
) -> Revision3ExactSnapshotExportErrorV1 {
    match cleanup {
        Ok(()) => primary,
        Err(error) => Revision3ExactSnapshotExportErrorV1::StagingCleanup {
            primary: Box::new(primary),
            cleanup: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::Value;

    use super::*;
    use crate::{
        AssetMeta, AssetStoreIndex, FormatV2, GameGenerationAnchor, ProjectMeta,
        QuestCollisionArtifactRef, Revision3EntityKind, Revision3OriginRef, Revision3QuestDraft,
        Revision3QuestDraftInput, Revision3QuestGiverInput, Revision3QuestParentInput,
        Revision3ScriptModule, Revision3TypedRef, SchemaRevisionV3, ScriptModuleStatus,
        QUEST_COLLISION_CATALOG_LAYER, REVISION3_QUEST_GENERATOR_ID,
        REVISION3_QUEST_GENERATOR_VERSION,
    };

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TestArea(PathBuf);

    impl TestArea {
        fn new(label: &str) -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gore-authoring-exact-export-{label}-{}-{sequence}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn store_root(&self) -> PathBuf {
            self.0.join("managed.goreproj")
        }

        fn output(&self, name: &str) -> PathBuf {
            self.0.join(name)
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

    fn empty_project(revision: u64) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([3; 16]),
            revision,
            meta: ProjectMeta {
                name: "Exact snapshot export".to_owned(),
                version: "1.2.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: seal(1, 171_698_176),
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
        title: &str,
    ) -> ProjectRevision3 {
        let mut project = empty_project(revision);
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
                module_namespace: "GoreMods.Quests.ExportTrial".to_owned(),
                technical_id: "GORE_EXPORT_TRIAL".to_owned(),
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
                title: title.to_owned(),
                description: "Retain the complete historical export basis.".to_owned(),
                objective_title: "Verify the closure".to_owned(),
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
        };
        let source = "// persisted exact-export Quest module\n".to_owned();
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
            module_relative_path: "GoreMods/Quests/ExportTrial.as".to_owned(),
            source_sha256: Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into()),
            source,
            input_fingerprint: Sha256Digest::from_bytes([7; 32]),
            status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
        };
        project.entities.insert(
            quest_id,
            Revision3Entity {
                id: quest_id,
                display_name: title.to_owned(),
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
                display_name: "Exact export source".to_owned(),
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

    fn tree_bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(base: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
            let mut entries = fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap())
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                if entry.file_type().unwrap().is_dir() {
                    visit(base, &path, result);
                } else {
                    result.insert(
                        path.strip_prefix(base).unwrap().to_owned(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }

        let mut result = BTreeMap::new();
        visit(root, root, &mut result);
        result
    }

    #[cfg(unix)]
    fn symlink_directory(target: &Path, link: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn symlink_directory(target: &Path, link: &Path) -> io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(unix)]
    fn remove_directory_symlink(link: &Path) -> io::Result<()> {
        fs::remove_file(link)
    }

    #[cfg(windows)]
    fn remove_directory_symlink(link: &Path) -> io::Result<()> {
        fs::remove_dir(link)
    }

    fn published_empty_store(
        label: &str,
    ) -> (
        TestArea,
        WorkingProjectStore,
        Revision3CheckpointPreparation,
    ) {
        let area = TestArea::new(label);
        let store =
            WorkingProjectStore::at(area.store_root(), WorkingStoreLimits::default()).unwrap();
        let prepared = store
            .prepare_revision3_checkpoint(None, &empty_project(7))
            .unwrap();
        fs::write(area.store_root().join(HEAD_FILE_NAME), &prepared.head_bytes).unwrap();
        (area, store, prepared)
    }

    struct QuestFixture {
        area: TestArea,
        store: WorkingProjectStore,
        basis: Revision3CheckpointPreparation,
        current: Revision3CheckpointPreparation,
        project: ProjectRevision3,
        artifact: ContentSeal,
    }

    fn published_quest_store(label: &str) -> QuestFixture {
        let area = TestArea::new(label);
        let store =
            WorkingProjectStore::at(area.store_root(), WorkingStoreLimits::default()).unwrap();
        let basis = store
            .prepare_revision3_checkpoint(None, &empty_project(7))
            .unwrap();
        fs::write(area.store_root().join(HEAD_FILE_NAME), &basis.head_bytes).unwrap();
        let artifact_bytes =
            serde_json::to_vec(&serde_json::json!({"padding": "closure"})).unwrap();
        let imported = store
            .import_quest_collision_artifact_v1(&artifact_bytes, Some(&basis.head))
            .unwrap();
        let project = quest_project(
            8,
            basis.head.snapshot.clone(),
            imported.artifact.clone(),
            imported.asset_meta,
            "Current Quest",
        );
        let current = store
            .prepare_revision3_checkpoint(Some(&basis.head), &project)
            .unwrap();
        fs::write(area.store_root().join(HEAD_FILE_NAME), &current.head_bytes).unwrap();
        QuestFixture {
            area,
            store,
            basis,
            current,
            project,
            artifact: imported.artifact,
        }
    }

    fn archive_names(path: &Path) -> Vec<String> {
        let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
        (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().to_owned())
            .collect()
    }

    fn raw_zip_members(path: &Path) -> Vec<RawZipMemberExpectation> {
        let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
        (0..archive.len())
            .map(|index| {
                let entry = archive.by_index_raw(index).unwrap();
                RawZipMemberExpectation {
                    name: entry.name_raw().to_vec(),
                    byte_len: entry.size(),
                    crc32: entry.crc32(),
                }
            })
            .collect()
    }

    fn raw_zip_offsets(members: &[RawZipMemberExpectation]) -> (Vec<u64>, Vec<u64>, u64) {
        let mut local_cursor = 0u64;
        let mut locals = Vec::new();
        for member in members {
            locals.push(local_cursor);
            local_cursor += 30
                + member.name.len() as u64
                + exact_zip64_extra(member.byte_len, local_cursor).len() as u64
                + member.byte_len;
        }
        let mut central_cursor = local_cursor;
        let mut centrals = Vec::new();
        for (member, local) in members.iter().zip(&locals) {
            centrals.push(central_cursor);
            central_cursor += 46
                + member.name.len() as u64
                + exact_zip64_extra(member.byte_len, *local).len() as u64;
        }
        (locals, centrals, central_cursor)
    }

    #[test]
    fn deterministic_export_has_closed_order_manifest_and_receipt() {
        let (area, store, prepared) = published_empty_store("deterministic");
        let first_path = area.output("first.goremod");
        let second_path = area.output("second.goremod");

        let first = store
            .export_current_revision3_exact_snapshot_v1(&prepared.head, &first_path)
            .unwrap();
        let second = store
            .export_current_revision3_exact_snapshot_v1(&prepared.head, &second_path)
            .unwrap();
        assert!(matches!(
            first,
            Revision3ExactSnapshotExportPublicationV1::Exported(_)
        ));
        assert!(matches!(
            second,
            Revision3ExactSnapshotExportPublicationV1::Exported(_)
        ));
        assert_eq!(
            fs::read(&first_path).unwrap(),
            fs::read(&second_path).unwrap()
        );
        assert_eq!(first.receipt(), second.receipt());

        let snapshot_name = snapshot_member_name(prepared.head.snapshot.sha256);
        assert_eq!(
            archive_names(&first_path),
            vec![
                REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V1.to_owned(),
                REVIEW_PROJECT_FILE.to_owned(),
                STORE_HEAD_MEMBER.to_owned(),
                snapshot_name.clone(),
            ]
        );
        assert_eq!(first.receipt().closure.snapshot_objects, 1);
        assert_eq!(first.receipt().closure.entity_objects, 0);
        assert_eq!(first.receipt().closure.asset_objects, 0);
        assert_eq!(first.receipt().closure.archive_entries, 4);
        assert!(first.receipt().closure.uncompressed_bytes > first.receipt().manifest.byte_len);

        let mut archive = ZipArchive::new(File::open(&first_path).unwrap()).unwrap();
        let mut bytes = Vec::new();
        archive
            .by_index(0)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            manifest["format"],
            REVISION3_EXACT_SNAPSHOT_MANIFEST_MARKER_V1
        );
        assert_eq!(manifest["schema"], 1);
        assert_eq!(
            manifest["artifact_kind"],
            REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V1
        );
        assert_eq!(
            manifest["restore_status"],
            REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V1
        );
        assert_eq!(
            manifest["basis"]["head"],
            serde_json::to_value(&prepared.head).unwrap()
        );
        let members = manifest["members"].as_array().unwrap();
        assert_eq!(members.len(), 3);
        assert_eq!(
            members
                .iter()
                .map(|member| member["relative_name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec![
                REVIEW_PROJECT_FILE,
                STORE_HEAD_MEMBER,
                snapshot_name.as_str()
            ]
        );

        let fixed_timestamp = DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap();
        let manifest_seal = seal_bytes(&bytes);
        assert_eq!(manifest_seal, first.receipt().manifest);
        drop(archive);
        let archive_bytes = fs::read(&first_path).unwrap();
        assert_eq!(seal_bytes(&archive_bytes), first.receipt().archive);
        assert!(archive_bytes
            .windows(4)
            .any(|window| window == [0x50, 0x4b, 0x06, 0x06]));
        assert!(archive_bytes
            .windows(4)
            .any(|window| window == [0x50, 0x4b, 0x06, 0x07]));

        let mut archive = ZipArchive::new(File::open(&first_path).unwrap()).unwrap();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            assert_eq!(entry.compression(), CompressionMethod::Stored);
            assert_eq!(entry.last_modified(), Some(fixed_timestamp));
            assert_eq!(
                entry.unix_mode().map(|mode| mode & 0o777),
                Some(ZIP_FILE_MODE)
            );
            assert_eq!(entry.size(), entry.compressed_size());
            let name = entry.name().to_owned();
            let mut payload = Vec::new();
            entry.read_to_end(&mut payload).unwrap();
            let actual = seal_bytes(&payload);
            if index == 0 {
                assert_eq!(actual, first.receipt().manifest);
            } else {
                let expected = members
                    .iter()
                    .find(|member| member["relative_name"] == name)
                    .unwrap();
                assert_eq!(actual.byte_len, expected["byte_len"].as_u64().unwrap());
                assert_eq!(
                    actual.sha256.to_string(),
                    expected["sha256"].as_str().unwrap()
                );
            }
        }
    }

    #[test]
    fn exact_zip_dialect_rejects_local_central_and_footer_mutations() {
        let (area, store, prepared) = published_empty_store("raw-zip-mutations");
        let output = area.output("source.goremod");
        store
            .export_current_revision3_exact_snapshot_v1(&prepared.head, &output)
            .unwrap();
        let original = fs::read(&output).unwrap();
        let members = raw_zip_members(&output);
        let (locals, centrals, footer) = raw_zip_offsets(&members);
        let local = locals[0] as usize;
        let central = centrals[0] as usize;
        let local_extra = local + 30 + members[0].name.len();
        let central_name = central + 46;
        let central_extra = central_name + members[0].name.len();
        let footer = footer as usize;
        let locator = footer + 56;
        let legacy = locator + 20;
        let mutations = [
            ("local-version", local + 4),
            ("local-flags", local + 6),
            ("local-extra-length", local + 28),
            ("local-extra-value", local_extra + 4),
            ("central-made-version", central + 4),
            ("central-needed-version", central + 6),
            ("central-flags", central + 8),
            ("central-raw-name", central_name),
            ("central-extra-length", central + 30),
            ("central-comment-length", central + 32),
            ("central-extra-value", central_extra + 4),
            ("zip64-record-size", footer + 4),
            ("zip64-locator-disk", locator + 4),
            ("legacy-comment-length", legacy + 20),
        ];
        for (label, offset) in mutations {
            let mut changed = original.clone();
            changed[offset] ^= 1;
            let path = area.output(&format!("{label}.goremod"));
            fs::write(&path, &changed).unwrap();
            let file = File::open(&path).unwrap();
            assert!(
                matches!(
                    verify_exact_zip_layout(&file, changed.len() as u64, &members),
                    Err(Revision3ExactSnapshotExportErrorV1::Verification(_))
                ),
                "raw mutation {label} was accepted"
            );
        }
    }

    #[test]
    fn export_manifest_parser_is_closed_and_rejects_self_or_unsorted_members() {
        let (area, store, prepared) = published_empty_store("closed-manifest");
        let output = area.output("closed.goremod");
        store
            .export_current_revision3_exact_snapshot_v1(&prepared.head, &output)
            .unwrap();
        let mut archive = ZipArchive::new(File::open(output).unwrap()).unwrap();
        let mut bytes = Vec::new();
        archive
            .by_index(0)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        let canonical = String::from_utf8(bytes).unwrap();

        let unknown =
            canonical.replacen("\"artifact_kind\"", "\"future\":true,\"artifact_kind\"", 1);
        assert!(serde_json::from_str::<ExactSnapshotManifestV1>(&unknown).is_err());
        let wrong_marker = canonical.replacen(
            REVISION3_EXACT_SNAPSHOT_MANIFEST_MARKER_V1,
            "gore.managed-project-snapshot.v2",
            1,
        );
        assert!(serde_json::from_str::<ExactSnapshotManifestV1>(&wrong_marker).is_err());

        let mut manifest: ExactSnapshotManifestV1 = serde_json::from_str(&canonical).unwrap();
        manifest.members[0].relative_name = REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V1.to_owned();
        assert!(matches!(
            validate_manifest_members(&manifest),
            Err(Revision3ExactSnapshotExportErrorV1::Verification(_))
        ));
        manifest.members[0].relative_name = "z-last".to_owned();
        assert!(matches!(
            validate_manifest_members(&manifest),
            Err(Revision3ExactSnapshotExportErrorV1::Verification(_))
        ));
    }

    #[test]
    fn closure_dedupe_conflicts_and_aggregate_budgets_fail_closed() {
        let member = PlannedMember {
            relative_name: "store/entities/aa/id/seal.json".to_owned(),
            seal: seal(4, 10),
            source: MemberSource::Bytes(vec![0; 10]),
        };
        let mut members = BTreeMap::new();
        assert!(insert_closure_member(&mut members, member, "entity").unwrap());
        let duplicate = PlannedMember {
            relative_name: "store/entities/aa/id/seal.json".to_owned(),
            seal: seal(4, 10),
            source: MemberSource::Bytes(vec![0; 10]),
        };
        assert!(!insert_closure_member(&mut members, duplicate, "entity").unwrap());
        let conflict = PlannedMember {
            relative_name: "store/entities/aa/id/seal.json".to_owned(),
            seal: seal(5, 10),
            source: MemberSource::Bytes(vec![0; 10]),
        };
        assert!(matches!(
            insert_closure_member(&mut members, conflict, "entity"),
            Err(Revision3ExactSnapshotExportErrorV1::InvalidClosure(_))
        ));

        let mut pending = BTreeMap::new();
        let mut known = BTreeMap::new();
        let mut known_bytes = 0;
        enqueue_snapshot(
            &mut pending,
            &mut known,
            &mut known_bytes,
            seal(6, 10),
            1,
            10,
        )
        .unwrap();
        enqueue_snapshot(
            &mut pending,
            &mut known,
            &mut known_bytes,
            seal(6, 10),
            1,
            10,
        )
        .unwrap();
        assert_eq!(pending.len(), 1);
        assert!(matches!(
            enqueue_snapshot(
                &mut pending,
                &mut known,
                &mut known_bytes,
                seal(6, 11),
                1,
                10,
            ),
            Err(Revision3ExactSnapshotExportErrorV1::InvalidClosure(_))
        ));
        assert!(matches!(
            enqueue_snapshot(
                &mut pending,
                &mut known,
                &mut known_bytes,
                seal(7, 1),
                1,
                10,
            ),
            Err(Revision3ExactSnapshotExportErrorV1::ClosureLimit {
                kind: "snapshot object count",
                ..
            })
        ));
        assert_eq!(checked_export_sum("bytes", 7, 3, 10).unwrap(), 10);
        assert!(matches!(
            checked_export_sum("bytes", 7, 4, 10),
            Err(Revision3ExactSnapshotExportErrorV1::ClosureLimit { .. })
        ));
        assert!(matches!(
            checked_export_sum("bytes", u64::MAX, 1, u64::MAX - 1),
            Err(Revision3ExactSnapshotExportErrorV1::ClosureLimit { .. })
        ));

        // Reopening the same large shared asset through multiple bases is conservatively charged
        // every time. The third 64-GiB basis is rejected before its Full reopen can start.
        let mut verification_objects = 0;
        let mut verification_bytes = 0;
        charge_full_verification_work(
            &mut verification_objects,
            &mut verification_bytes,
            1,
            64 * 1024 * 1024 * 1024,
        )
        .unwrap();
        charge_full_verification_work(
            &mut verification_objects,
            &mut verification_bytes,
            1,
            64 * 1024 * 1024 * 1024,
        )
        .unwrap();
        assert!(matches!(
            charge_full_verification_work(
                &mut verification_objects,
                &mut verification_bytes,
                1,
                64 * 1024 * 1024 * 1024,
            ),
            Err(Revision3ExactSnapshotExportErrorV1::ClosureLimit {
                kind: "full-verification bytes",
                ..
            })
        ));
    }

    #[test]
    fn recursively_exports_quest_bases_dedupes_shared_objects_and_excludes_orphans() {
        let area = TestArea::new("recursive-closure");
        let store =
            WorkingProjectStore::at(area.store_root(), WorkingStoreLimits::default()).unwrap();
        let first = store
            .prepare_revision3_checkpoint(None, &empty_project(7))
            .unwrap();
        fs::write(area.store_root().join(HEAD_FILE_NAME), &first.head_bytes).unwrap();

        let artifact_bytes = serde_json::to_vec(&serde_json::json!({"padding": "basis"})).unwrap();
        let artifact = store
            .import_quest_collision_artifact_v1(&artifact_bytes, Some(&first.head))
            .unwrap();
        let second_project = quest_project(
            8,
            first.head.snapshot.clone(),
            artifact.artifact.clone(),
            artifact.asset_meta.clone(),
            "Historical Quest",
        );
        let second = store
            .prepare_revision3_checkpoint(Some(&first.head), &second_project)
            .unwrap();
        fs::write(area.store_root().join(HEAD_FILE_NAME), &second.head_bytes).unwrap();

        let current_project = quest_project(
            9,
            second.head.snapshot.clone(),
            artifact.artifact.clone(),
            artifact.asset_meta.clone(),
            "Current Quest",
        );
        let current = store
            .prepare_revision3_checkpoint(Some(&second.head), &current_project)
            .unwrap();
        fs::write(area.store_root().join(HEAD_FILE_NAME), &current.head_bytes).unwrap();

        let orphan_bytes = serde_json::to_vec(&serde_json::json!({"padding": "orphan"})).unwrap();
        let orphan_artifact = store
            .import_quest_collision_artifact_v1(&orphan_bytes, Some(&current.head))
            .unwrap();
        let orphan_project = quest_project(
            10,
            current.head.snapshot.clone(),
            orphan_artifact.artifact.clone(),
            orphan_artifact.asset_meta,
            "Unreachable Orphan",
        );
        let orphan = store
            .prepare_revision3_checkpoint(Some(&current.head), &orphan_project)
            .unwrap();

        let before = tree_bytes(&area.store_root());
        let output = area.output("recursive.goremod");
        let exported = store
            .export_current_revision3_exact_snapshot_v1(&current.head, &output)
            .unwrap();
        assert!(matches!(
            exported,
            Revision3ExactSnapshotExportPublicationV1::Exported(_)
        ));
        assert_eq!(tree_bytes(&area.store_root()), before);
        assert_eq!(exported.receipt().closure.snapshot_objects, 3);
        assert_eq!(exported.receipt().closure.entity_objects, 3);
        assert_eq!(exported.receipt().closure.asset_objects, 1);
        assert_eq!(exported.receipt().closure.archive_entries, 10);

        let names = archive_names(&output);
        for head in [&first.head, &second.head, &current.head] {
            assert!(names.contains(&snapshot_member_name(head.snapshot.sha256)));
        }
        assert!(!names.contains(&snapshot_member_name(orphan.head.snapshot.sha256)));
        assert!(!names.contains(&asset_member_name(orphan_artifact.artifact.sha256)));
        let orphan_quest = &orphan_project.entities[&entity_id(10)];
        let orphan_quest_seal = seal_bytes(&canonical_json(orphan_quest).unwrap());
        assert!(!names.contains(&entity_member_name(entity_id(10), orphan_quest_seal.sha256)));

        let mut sorted = names[3..].to_vec();
        let snapshots_end = 3 + exported.receipt().closure.snapshot_objects as usize;
        assert!(names[3..snapshots_end]
            .iter()
            .all(|name| name.starts_with("store/snapshots/")));
        let entities_end = snapshots_end + exported.receipt().closure.entity_objects as usize;
        assert!(names[snapshots_end..entities_end]
            .iter()
            .all(|name| name.starts_with("store/entities/")));
        assert!(names[entities_end..]
            .iter()
            .all(|name| name.starts_with("store/assets/")));
        sorted.sort();
        assert_ne!(
            names[3..],
            sorted,
            "type groups, not one global sort, define ZIP order"
        );
    }

    #[test]
    fn rejects_relative_store_local_stale_and_existing_outputs_without_clobber() {
        let (area, store, prepared) = published_empty_store("reject");
        assert!(matches!(
            store.export_current_revision3_exact_snapshot_v1(
                &prepared.head,
                Path::new("relative.goremod")
            ),
            Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(_))
        ));
        assert!(matches!(
            store.export_current_revision3_exact_snapshot_v1(
                &prepared.head,
                area.store_root().join("inside.goremod")
            ),
            Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(_))
        ));

        let mut stale = prepared.head.clone();
        stale.snapshot.sha256 = Sha256Digest::from_bytes([9; 32]);
        assert!(matches!(
            store.export_current_revision3_exact_snapshot_v1(&stale, area.output("stale.goremod")),
            Err(Revision3ExactSnapshotExportErrorV1::Store(
                WorkingStoreError::HeadConflict { .. }
            ))
        ));

        let existing = area.output("existing.goremod");
        fs::write(&existing, b"untouched").unwrap();
        assert!(matches!(
            store.export_current_revision3_exact_snapshot_v1(&prepared.head, &existing),
            Err(Revision3ExactSnapshotExportErrorV1::OutputAlreadyExists)
        ));
        assert_eq!(fs::read(existing).unwrap(), b"untouched");
    }

    #[test]
    fn rejects_wrong_extension_missing_parent_and_output_race_without_clobber() {
        let (area, store, prepared) = published_empty_store("output-boundaries");
        assert!(matches!(
            store.export_current_revision3_exact_snapshot_v1(
                &prepared.head,
                area.output("review.zip")
            ),
            Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(_))
        ));
        assert!(matches!(
            store.export_current_revision3_exact_snapshot_v1(
                &prepared.head,
                area.output("missing-parent").join("review.goremod")
            ),
            Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(_))
        ));

        let raced = area.output("raced.goremod");
        let result = store.export_current_revision3_exact_snapshot_v1_guarded(
            &prepared.head,
            &raced,
            |phase, target| {
                if phase == ExportGuardPhase::BeforeInstall {
                    fs::write(target, b"racing publisher")?;
                }
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(Revision3ExactSnapshotExportErrorV1::OutputAlreadyExists)
        ));
        assert_eq!(fs::read(raced).unwrap(), b"racing publisher");
    }

    #[test]
    fn source_snapshot_entity_asset_and_historical_basis_corruption_fail_before_publish() {
        {
            let fixture = published_quest_store("corrupt-snapshot");
            let output = fixture.area.output("snapshot.goremod");
            let path = fixture
                .store
                .snapshot_path(fixture.current.head.snapshot.sha256);
            let mut bytes = fs::read(&path).unwrap();
            bytes[0] ^= 1;
            fs::write(path, bytes).unwrap();
            assert!(matches!(
                fixture
                    .store
                    .export_current_revision3_exact_snapshot_v1(&fixture.current.head, &output),
                Err(Revision3ExactSnapshotExportErrorV1::Store(
                    WorkingStoreError::SealMismatch { .. }
                ))
            ));
            assert!(!output.exists());
        }

        {
            let fixture = published_quest_store("missing-entity");
            let output = fixture.area.output("entity.goremod");
            let entity = &fixture.project.entities[&entity_id(10)];
            let entity_seal = seal_bytes(&canonical_json(entity).unwrap());
            fs::remove_file(fixture.store.entity_path(entity_id(10), entity_seal.sha256)).unwrap();
            assert!(matches!(
                fixture
                    .store
                    .export_current_revision3_exact_snapshot_v1(&fixture.current.head, &output),
                Err(Revision3ExactSnapshotExportErrorV1::Store(
                    WorkingStoreError::MissingObject(_)
                ))
            ));
            assert!(!output.exists());
        }

        {
            let fixture = published_quest_store("corrupt-asset");
            let output = fixture.area.output("asset.goremod");
            let path = fixture.store.asset_path(fixture.artifact.sha256);
            let mut bytes = fs::read(&path).unwrap();
            bytes[0] ^= 1;
            fs::write(path, bytes).unwrap();
            assert!(matches!(
                fixture
                    .store
                    .export_current_revision3_exact_snapshot_v1(&fixture.current.head, &output),
                Err(Revision3ExactSnapshotExportErrorV1::Store(
                    WorkingStoreError::SealMismatch { .. }
                ))
            ));
            assert!(!output.exists());
        }

        {
            let fixture = published_quest_store("missing-basis");
            let output = fixture.area.output("basis.goremod");
            fs::remove_file(
                fixture
                    .store
                    .snapshot_path(fixture.basis.head.snapshot.sha256),
            )
            .unwrap();
            assert!(matches!(
                fixture
                    .store
                    .export_current_revision3_exact_snapshot_v1(&fixture.current.head, &output),
                Err(Revision3ExactSnapshotExportErrorV1::Store(
                    WorkingStoreError::MissingObject(_)
                ))
            ));
            assert!(!output.exists());
        }
    }

    #[test]
    fn hard_linked_store_entity_is_integrity_failure_not_an_output_failure() {
        let fixture = published_quest_store("linked-entity");
        let entity = &fixture.project.entities[&entity_id(10)];
        let entity_seal = seal_bytes(&canonical_json(entity).unwrap());
        let entity_path = fixture.store.entity_path(entity_id(10), entity_seal.sha256);
        let outside = fixture.area.output("linked-copy.json");
        fs::rename(&entity_path, &outside).unwrap();
        fs::hard_link(&outside, &entity_path).unwrap();
        let output = fixture.area.output("linked.goremod");
        assert!(matches!(
            fixture
                .store
                .export_current_revision3_exact_snapshot_v1(&fixture.current.head, &output),
            Err(Revision3ExactSnapshotExportErrorV1::Store(
                WorkingStoreError::UnsafePath { .. }
            ))
        ));
        assert!(!output.exists());
    }

    #[test]
    fn linked_output_parent_is_invalid_and_linked_store_root_is_integrity_failure() {
        let (area, store, prepared) = published_empty_store("linked-paths");
        let real_output = area.output("real-output");
        let linked_output = area.output("linked-output");
        fs::create_dir(&real_output).unwrap();
        if symlink_directory(&real_output, &linked_output).is_err() {
            // Windows can deny symlink creation when Developer Mode is disabled. The hard-link
            // object test above still exercises no-follow/link-count enforcement on that host.
            return;
        }
        let output = linked_output.join("review.goremod");
        assert!(matches!(
            store.export_current_revision3_exact_snapshot_v1(&prepared.head, &output),
            Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(_))
        ));
        assert!(!real_output.join("review.goremod").exists());
        remove_directory_symlink(&linked_output).unwrap();

        let root = area.store_root();
        let moved_root = area.output("real-store");
        fs::rename(&root, &moved_root).unwrap();
        if symlink_directory(&moved_root, &root).is_err() {
            fs::rename(&moved_root, &root).unwrap();
            return;
        }
        let linked_root_result = store.export_current_revision3_exact_snapshot_v1(
            &prepared.head,
            area.output("linked-root.goremod"),
        );
        assert!(matches!(
            linked_root_result,
            Err(Revision3ExactSnapshotExportErrorV1::Store(
                WorkingStoreError::UnsafePath { .. }
            ))
        ));
        remove_directory_symlink(&root).unwrap();
        fs::rename(&moved_root, &root).unwrap();
    }

    #[test]
    fn second_head_gate_prevents_stale_publication() {
        let (area, store, prepared) = published_empty_store("prepublish-head-gate");
        let candidate = store
            .prepare_revision3_checkpoint(Some(&prepared.head), &empty_project(8))
            .unwrap();
        let output = area.output("blocked.goremod");
        let fixed_head = area.store_root().join(HEAD_FILE_NAME);

        let result = store.export_current_revision3_exact_snapshot_v1_guarded(
            &prepared.head,
            &output,
            |phase, _| {
                if phase == ExportGuardPhase::BeforePublication {
                    fs::write(&fixed_head, &candidate.head_bytes)?;
                }
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(Revision3ExactSnapshotExportErrorV1::Store(
                WorkingStoreError::HeadConflict { .. }
            ))
        ));
        assert!(!output.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parent_drift_before_staging_is_invalid_output_and_never_redirects() {
        let (area, store, prepared) = published_empty_store("parent-drift-prestage");
        let original_parent = area.output("output-parent");
        let moved_parent = area.output("pinned-parent");
        fs::create_dir(&original_parent).unwrap();
        let output = original_parent.join("review.goremod");

        let result = store.export_current_revision3_exact_snapshot_v1_guarded(
            &prepared.head,
            &output,
            |phase, _| {
                if phase == ExportGuardPhase::BeforeStagingCreate {
                    fs::rename(&original_parent, &moved_parent)?;
                    fs::create_dir(&original_parent)?;
                }
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(_))
        ));
        assert!(!original_parent.join("review.goremod").exists());
        assert!(!moved_parent.join("review.goremod").exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn link_free_revalidation_rejects_symlink_to_same_pinned_parent() {
        let (area, store, prepared) = published_empty_store("parent-link-install");
        let original_parent = area.output("output-parent");
        let moved_parent = area.output("pinned-parent");
        fs::create_dir(&original_parent).unwrap();
        let output = original_parent.join("review.goremod");

        let result = store.export_current_revision3_exact_snapshot_v1_guarded(
            &prepared.head,
            &output,
            |phase, _| {
                if phase == ExportGuardPhase::BeforeInstall {
                    fs::rename(&original_parent, &moved_parent)?;
                    symlink_directory(&moved_parent, &original_parent)?;
                }
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(_))
        ));
        assert!(!original_parent.join("review.goremod").exists());
        assert!(!moved_parent.join("review.goremod").exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parent_moved_into_store_and_linked_back_is_rejected_before_install() {
        let (area, store, prepared) = published_empty_store("parent-store-link-install");
        let original_parent = area.output("output-parent");
        let moved_parent = area.store_root().join("relocated-output-parent");
        fs::create_dir(&original_parent).unwrap();
        let output = original_parent.join("review.goremod");

        let result = store.export_current_revision3_exact_snapshot_v1_guarded(
            &prepared.head,
            &output,
            |phase, _| {
                if phase == ExportGuardPhase::BeforeInstall {
                    fs::rename(&original_parent, &moved_parent)?;
                    symlink_directory(&moved_parent, &original_parent)?;
                }
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(Revision3ExactSnapshotExportErrorV1::InvalidOutput(_))
        ));
        assert!(!output.exists());
        assert!(!moved_parent.join("review.goremod").exists());
    }

    #[cfg(windows)]
    #[test]
    fn windows_pinned_parent_denies_install_time_directory_replacement() {
        let (area, store, prepared) = published_empty_store("parent-lock-install");
        let original_parent = area.output("output-parent");
        let moved_parent = area.store_root().join("relocated-output-parent");
        fs::create_dir(&original_parent).unwrap();
        let output = original_parent.join("review.goremod");
        let mut replacement_denied = false;

        let result = store
            .export_current_revision3_exact_snapshot_v1_guarded(
                &prepared.head,
                &output,
                |phase, _| {
                    if phase == ExportGuardPhase::BeforeInstall {
                        // The pinned Windows directory handle excludes FILE_SHARE_DELETE, so the
                        // parent cannot be moved under Store and replaced by a junction/reparse
                        // point while the export is live.
                        replacement_denied = fs::rename(&original_parent, &moved_parent).is_err();
                    }
                    Ok(())
                },
            )
            .unwrap();
        assert!(replacement_denied);
        assert!(matches!(
            result,
            Revision3ExactSnapshotExportPublicationV1::Exported(_)
        ));
        assert!(output.is_file());
        assert!(!moved_parent.exists());
    }

    #[test]
    fn post_publication_failure_is_typed_uncertain_and_never_an_error() {
        let (area, store, prepared) = published_empty_store("uncertain");
        let output = area.output("uncertain.goremod");
        let result = store
            .export_current_revision3_exact_snapshot_v1_guarded(
                &prepared.head,
                &output,
                |phase, _| {
                    if phase == ExportGuardPhase::AfterPublication {
                        return Err(WorkingStoreError::Invariant(
                            "injected post-publication failure".to_owned(),
                        ));
                    }
                    Ok(())
                },
            )
            .unwrap();
        assert!(matches!(
            result,
            Revision3ExactSnapshotExportPublicationV1::PublicationUncertain(_)
        ));
        assert!(output.is_file());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn final_name_replacement_is_detected_without_deleting_foreign_file() {
        let (area, store, prepared) = published_empty_store("final-name-race");
        let output = area.output("review.goremod");
        let displaced = area.output("verified-but-displaced.goremod");
        let result = store
            .export_current_revision3_exact_snapshot_v1_guarded(
                &prepared.head,
                &output,
                |phase, _| {
                    if phase == ExportGuardPhase::AfterPublication {
                        fs::rename(&output, &displaced)?;
                        fs::write(&output, b"foreign replacement")?;
                    }
                    Ok(())
                },
            )
            .unwrap();
        assert!(matches!(
            result,
            Revision3ExactSnapshotExportPublicationV1::PublicationUncertain(_)
        ));
        assert_eq!(fs::read(&output).unwrap(), b"foreign replacement");
        assert!(ZipArchive::new(File::open(displaced).unwrap()).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn windows_final_name_is_locked_through_final_verification() {
        let (area, store, prepared) = published_empty_store("final-name-lock");
        let output = area.output("review.goremod");
        let displaced = area.output("displaced.goremod");
        let mut replacement_denied = false;
        let result = store
            .export_current_revision3_exact_snapshot_v1_guarded(
                &prepared.head,
                &output,
                |phase, _| {
                    if phase == ExportGuardPhase::AfterPublication {
                        replacement_denied = fs::rename(&output, &displaced).is_err();
                    }
                    Ok(())
                },
            )
            .unwrap();
        assert!(replacement_denied);
        assert!(matches!(
            result,
            Revision3ExactSnapshotExportPublicationV1::Exported(_)
        ));
        assert!(output.is_file());
        assert!(!displaced.exists());
    }

    #[test]
    fn post_publication_head_drift_does_not_invalidate_a_verified_basis_export() {
        let (area, store, prepared) = published_empty_store("postpublish-head-drift");
        let candidate = store
            .prepare_revision3_checkpoint(Some(&prepared.head), &empty_project(8))
            .unwrap();
        let fixed_head = area.store_root().join(HEAD_FILE_NAME);
        let output = area.output("verified.goremod");
        let result = store
            .export_current_revision3_exact_snapshot_v1_guarded(
                &prepared.head,
                &output,
                |phase, _| {
                    if phase == ExportGuardPhase::AfterPublication {
                        fs::write(&fixed_head, &candidate.head_bytes)?;
                    }
                    Ok(())
                },
            )
            .unwrap();
        assert!(matches!(
            result,
            Revision3ExactSnapshotExportPublicationV1::Exported(_)
        ));
        assert!(output.is_file());
    }

    #[test]
    fn private_cleanup_failure_is_a_success_warning() {
        let (area, store, prepared) = published_empty_store("cleanup-warning");
        let output = area.output("cleanup-warning.goremod");
        let result = store
            .export_current_revision3_exact_snapshot_v1_guarded(
                &prepared.head,
                &output,
                |phase, _| {
                    if phase == ExportGuardPhase::BeforeStagingCleanup {
                        return Err(WorkingStoreError::Invariant(
                            "injected cleanup accounting failure".to_owned(),
                        ));
                    }
                    Ok(())
                },
            )
            .unwrap();
        assert!(matches!(
            result,
            Revision3ExactSnapshotExportPublicationV1::ExportedWithCleanupWarning(_)
        ));
        assert!(output.is_file());
    }

    #[test]
    fn prepublication_cleanup_failure_preserves_store_integrity_primary() {
        let primary = Revision3ExactSnapshotExportErrorV1::Store(WorkingStoreError::SealMismatch {
            path: PathBuf::from("source-object"),
            reason: "injected source drift".to_owned(),
        });
        let result = preserve_primary_through_cleanup(
            primary,
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "injected handle cleanup failure",
            )),
        );
        assert!(matches!(
            result,
            Revision3ExactSnapshotExportErrorV1::StagingCleanup {
                primary,
                ..
            } if matches!(*primary, Revision3ExactSnapshotExportErrorV1::Store(
                WorkingStoreError::SealMismatch { .. }
            ))
        ));
    }

    #[test]
    fn prepublication_abort_cannot_delete_a_foreign_path() {
        let (area, store, prepared) = published_empty_store("foreign-cleanup");
        let output = area.output("must-not-publish.goremod");
        let foreign = area.output("foreign-do-not-delete");
        fs::write(&foreign, b"foreign").unwrap();
        #[cfg(target_os = "linux")]
        let mut injected_display_path = None;

        let result = store.export_current_revision3_exact_snapshot_v1_guarded(
            &prepared.head,
            &output,
            |phase, path| {
                if phase == ExportGuardPhase::BeforeArchiveWrite {
                    #[cfg(target_os = "linux")]
                    {
                        // The anonymous stage has no name. A foreign actor may create the merely
                        // diagnostic display path, but handle cleanup must never remove it.
                        fs::write(path, b"foreign-display-path")?;
                        injected_display_path = Some(path.to_owned());
                    }
                    #[cfg(windows)]
                    {
                        // share_mode(0) prevents replacing the live stage with a foreign file.
                        assert!(fs::remove_file(path).is_err());
                    }
                    return Err(WorkingStoreError::Invariant(
                        "injected prepublication abort".to_owned(),
                    ));
                }
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(Revision3ExactSnapshotExportErrorV1::Store(
                WorkingStoreError::Invariant(_)
            ))
        ));
        assert!(!output.exists());
        assert_eq!(fs::read(&foreign).unwrap(), b"foreign");

        #[cfg(target_os = "linux")]
        {
            let injected = injected_display_path.unwrap();
            assert_eq!(fs::read(&injected).unwrap(), b"foreign-display-path");
        }
    }
}
