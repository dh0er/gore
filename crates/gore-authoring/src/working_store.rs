use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::model_revision2::{
    Entity as Revision2Entity, EntityPayload as Revision2EntityPayload,
    OggCodec as Revision2OggCodec, OggMetadata as Revision2OggMetadata,
};
use crate::model_revision3::{
    is_quest_collision_artifact_media_type, Entity as Revision3Entity,
    EntityPayload as Revision3EntityPayload,
};
use crate::{
    AssetMeta, AssetRef, AssetStoreIndex, ContentSeal, Diagnostic, DiagnosticCode, Entity,
    EntityId, EntityPayload, FormatV2, GameGenerationAnchor, LocaleCode, OggCodec, OggMetadata,
    PreparedRevision3QuestCollisionInspectionSourceV2, PreparedRevision3QuestCollisionSourceV2,
    ProjectDocument, ProjectId, ProjectMeta, ProjectRevision2, ProjectRevision3, ProjectV2,
    Revision3QuestCollisionSourceErrorV2, SchemaRevisionV1, SchemaRevisionV2, SchemaRevisionV3,
    Sha256Digest, ValidationProfile, MAX_QUEST_COLLISION_ARTIFACT_BYTES,
    MAX_REVISION3_BASE_SNAPSHOT_BYTES, MAX_REVISION3_SNAPSHOT_BYTES,
    QUEST_COLLISION_ARTIFACT_MEDIA_TYPE, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
};

mod revision3_export;
mod revision3_history;
mod revision3_import;

pub use revision3_export::{
    Revision3ExactSnapshotClosureV1, Revision3ExactSnapshotClosureV2,
    Revision3ExactSnapshotExportErrorV1, Revision3ExactSnapshotExportErrorV2,
    Revision3ExactSnapshotExportPublicationV1, Revision3ExactSnapshotExportPublicationV2,
    Revision3ExactSnapshotExportV1, Revision3ExactSnapshotExportV2,
    Revision3ExactSnapshotExportWarningV1, Revision3ExactSnapshotExportWarningV2,
    REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V1, REVISION3_EXACT_SNAPSHOT_ARTIFACT_KIND_V2,
    REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V1, REVISION3_EXACT_SNAPSHOT_EXPORT_FORMAT_V2,
    REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V1, REVISION3_EXACT_SNAPSHOT_MANIFEST_FILE_V2,
    REVISION3_EXACT_SNAPSHOT_MANIFEST_MARKER_V1, REVISION3_EXACT_SNAPSHOT_MANIFEST_MARKER_V2,
    REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V1, REVISION3_EXACT_SNAPSHOT_RESTORE_STATUS_V2,
};
pub use revision3_history::{
    PreparedRevision3HistoryRestoreV1, Revision3CheckpointHistoryV1, Revision3CheckpointParentV1,
    Revision3HistoryEntryV1, Revision3HistoryErrorV1, Revision3HistoryV1,
    MAX_REVISION3_HISTORY_ENTRIES_V1, MAX_REVISION3_HISTORY_MANIFEST_BYTES_V1,
    MAX_REVISION3_HISTORY_PARENT_RECORDS_V1, REVISION3_HISTORY_AUTHORITY_V1,
};
pub use revision3_import::{
    import_revision3_exact_snapshot_v2, import_revision3_exact_snapshot_v2_with_limits,
    inspect_revision3_exact_snapshot_v2, inspect_revision3_exact_snapshot_v2_with_limits,
    Revision3ExactSnapshotImportErrorV2, Revision3ExactSnapshotImportPublicationV2,
    Revision3ExactSnapshotImportV2, Revision3ExactSnapshotImportWarningV2,
    Revision3ExactSnapshotInspectionClosureV2, Revision3ExactSnapshotInspectionErrorV2,
    Revision3ExactSnapshotInspectionV2, REVISION3_EXACT_SNAPSHOT_IMPORT_ARTIFACT_KIND_V2,
    REVISION3_EXACT_SNAPSHOT_IMPORT_FORMAT_V2, REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_FILE_V2,
    REVISION3_EXACT_SNAPSHOT_IMPORT_MANIFEST_MARKER_V2,
    REVISION3_EXACT_SNAPSHOT_IMPORT_RESTORE_STATUS_V2,
};

const HEAD_FILE_NAME: &str = "gore-project.json";
const STORE_FORMAT: u32 = 1;
const MAX_HEAD_BYTES_HARD: usize = 64 * 1024;
const MAX_SNAPSHOT_BYTES_HARD: usize = MAX_REVISION3_BASE_SNAPSHOT_BYTES as usize;
/// Default format reserve between the history-free and final R3 Store snapshot ceilings.
pub const REVISION3_HISTORY_SNAPSHOT_RESERVE_BYTES_V1: usize =
    (MAX_REVISION3_SNAPSHOT_BYTES - MAX_REVISION3_BASE_SNAPSHOT_BYTES) as usize;
const MAX_ENTITY_BYTES_HARD: usize = 1024 * 1024;
const MAX_REFERENCED_ENTITY_BYTES_HARD: u64 = 512 * 1024 * 1024;
const MAX_ENTITIES_HARD: usize = 100_000;
const MAX_ASSETS_HARD: usize = 100_000;
const MAX_ASSET_BYTES_HARD: u64 = 64 * 1024 * 1024 * 1024;
const MAX_OGG_BYTES_HARD: usize = 64 * 1024 * 1024;
const MAX_LOGICAL_NAME_BYTES_HARD: usize = 1024;
const COPY_BUFFER_BYTES: usize = 64 * 1024;
const WINDOWS_REPARSE_POINT_ATTRIBUTE: u32 = 0x400;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Finite resource ceilings applied by a [`WorkingProjectStore`].
///
/// Values may be made stricter than the format ceilings, but never looser. This makes every
/// store operation bounded without allowing one caller to create objects another conforming
/// reader cannot safely open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkingStoreLimits {
    pub max_head_bytes: usize,
    /// Legacy snapshot and history-free R3 manifest ceiling. Final R3 snapshots receive the fixed
    /// bounded-history reserve in addition; making this value stricter still lowers both caps.
    pub max_snapshot_bytes: usize,
    pub max_entity_bytes: usize,
    pub max_referenced_entity_bytes: u64,
    pub max_entities: usize,
    pub max_assets: usize,
    pub max_referenced_asset_bytes: u64,
    pub max_ogg_bytes: usize,
    pub max_logical_name_bytes: usize,
}

impl Default for WorkingStoreLimits {
    fn default() -> Self {
        Self {
            max_head_bytes: MAX_HEAD_BYTES_HARD,
            max_snapshot_bytes: MAX_SNAPSHOT_BYTES_HARD,
            max_entity_bytes: MAX_ENTITY_BYTES_HARD,
            max_referenced_entity_bytes: MAX_REFERENCED_ENTITY_BYTES_HARD,
            max_entities: MAX_ENTITIES_HARD,
            max_assets: MAX_ASSETS_HARD,
            max_referenced_asset_bytes: MAX_ASSET_BYTES_HARD,
            max_ogg_bytes: MAX_OGG_BYTES_HARD,
            max_logical_name_bytes: MAX_LOGICAL_NAME_BYTES_HARD,
        }
    }
}

impl WorkingStoreLimits {
    fn validate(self) -> Result<Self, WorkingStoreError> {
        let checks = [
            ("max_head_bytes", self.max_head_bytes, MAX_HEAD_BYTES_HARD),
            (
                "max_snapshot_bytes",
                self.max_snapshot_bytes,
                MAX_SNAPSHOT_BYTES_HARD,
            ),
            (
                "max_entity_bytes",
                self.max_entity_bytes,
                MAX_ENTITY_BYTES_HARD,
            ),
            ("max_entities", self.max_entities, MAX_ENTITIES_HARD),
            ("max_assets", self.max_assets, MAX_ASSETS_HARD),
            ("max_ogg_bytes", self.max_ogg_bytes, MAX_OGG_BYTES_HARD),
            (
                "max_logical_name_bytes",
                self.max_logical_name_bytes,
                MAX_LOGICAL_NAME_BYTES_HARD,
            ),
        ];
        for (name, value, hard_limit) in checks {
            if value == 0 || value > hard_limit {
                return Err(WorkingStoreError::InvalidLimits(format!(
                    "{name} must be in 1..={hard_limit}, got {value}"
                )));
            }
        }
        if self.max_referenced_asset_bytes == 0
            || self.max_referenced_asset_bytes > MAX_ASSET_BYTES_HARD
        {
            return Err(WorkingStoreError::InvalidLimits(format!(
                "max_referenced_asset_bytes must be in 1..={MAX_ASSET_BYTES_HARD}, got {}",
                self.max_referenced_asset_bytes
            )));
        }
        if self.max_referenced_entity_bytes == 0
            || self.max_referenced_entity_bytes > MAX_REFERENCED_ENTITY_BYTES_HARD
        {
            return Err(WorkingStoreError::InvalidLimits(format!(
                "max_referenced_entity_bytes must be in 1..={MAX_REFERENCED_ENTITY_BYTES_HARD}, got {}",
                self.max_referenced_entity_bytes
            )));
        }
        Ok(self)
    }
}

fn revision3_total_snapshot_limit(limits: &WorkingStoreLimits) -> usize {
    limits
        .max_snapshot_bytes
        .saturating_add(REVISION3_HISTORY_SNAPSHOT_RESERVE_BYTES_V1)
        .min(MAX_REVISION3_SNAPSHOT_BYTES as usize)
}

/// Exact immutable-store format marker. Only integer `1` is accepted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorkingStoreFormat;

impl<'de> Deserialize<'de> for WorkingStoreFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == STORE_FORMAT {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported working-store format {value}; expected {STORE_FORMAT}"
            )))
        }
    }
}

impl Serialize for WorkingStoreFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(STORE_FORMAT)
    }
}

/// Small fixed-head document. Publishing these bytes is intentionally the caller's CAS step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkingHead {
    pub store_format: WorkingStoreFormat,
    pub snapshot: ContentSeal,
}

/// Result of preparing a checkpoint without replacing the fixed head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointPreparation {
    pub head_bytes: Vec<u8>,
    pub head: WorkingHead,
    pub diagnostics: Vec<Diagnostic>,
    pub blocks_build: bool,
}

/// Fully reconstituted project and its semantic validation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenedCheckpoint {
    pub head: WorkingHead,
    pub project: ProjectV2,
    pub diagnostics: Vec<Diagnostic>,
    pub blocks_build: bool,
}

/// Fully reconstituted project document and its revision-specific validation result.
///
/// Revision 1 callers may keep using [`OpenedCheckpoint`]. This additive result is used by the
/// document APIs that dispatch between the two closed schema revisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenedDocumentCheckpoint {
    pub head: WorkingHead,
    pub project: ProjectDocument,
    pub diagnostics: Vec<Diagnostic>,
    pub blocks_build: bool,
}

/// Result of preparing one schema-revision-3 checkpoint without publishing the fixed head.
///
/// This type deliberately has no diagnostics or readiness boolean: this store slice proves only
/// bounded structural identity and durable content-addressed storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3CheckpointPreparation {
    pub head_bytes: Vec<u8>,
    pub head: WorkingHead,
}

/// A schema-revision-3 project reconstituted from one exact immutable snapshot.
///
/// `head` identifies the opened snapshot. It does not assert that the fixed head currently points
/// to that snapshot; this distinction keeps old basis snapshots independently reopenable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenedRevision3Checkpoint {
    pub head: WorkingHead,
    pub project: ProjectRevision3,
}

/// Manifest-only first phase for a budgeted historical DataAsset basis reopen.
///
/// The snapshot itself has already been fully hashed and canonical-parsed. `verification_*`
/// conservatively covers the later full reopen: snapshot, entity reads, every asset twice (the
/// second pass bounds VoiceTake metadata), and one maximum-size nested basis per entity.
pub(crate) struct Revision3DataAssetBasisPreflight {
    pub(crate) head: WorkingHead,
    pub(crate) project_id: ProjectId,
    pub(crate) target: GameGenerationAnchor,
    pub(crate) revision: u64,
    pub(crate) manifest: Revision3SnapshotManifest,
    pub(crate) verification_objects: u64,
    pub(crate) verification_bytes: u64,
}

/// Physical blob verification performed while opening or checking an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetVerification {
    /// Validate derived path, object type, link policy, and byte length.
    /// This fast mode is for browsing only and is never production/build-readiness proof.
    Structural,
    /// Perform structural checks and stream the complete SHA-256 digest. VoiceTake Ogg blobs are
    /// additionally parsed and their derived metadata is matched against the persisted entity.
    Full,
}

/// Content-addressed result of importing one validated Ogg stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedOgg {
    pub asset: AssetRef,
    pub ogg: OggMetadata,
    pub deduplicated: bool,
}

/// Bounded, fully parsed Ogg source bytes that have not been installed in immutable Store CAS.
///
/// Fields stay private so callers cannot forge a source preparation. A preparation may be
/// previewed for filesystem-free semantic/capacity evaluation, compared with a second source
/// read, and finally consumed by [`WorkingProjectStore::install_prepared_ogg`]. Preparing a source
/// never creates a Store staging file or asset object.
#[derive(PartialEq, Eq)]
pub struct PreparedOggImport {
    bytes: Vec<u8>,
    asset: AssetRef,
    ogg: OggMetadata,
}

impl std::fmt::Debug for PreparedOggImport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedOggImport")
            .field("verified_bytes_len", &self.bytes.len())
            .field("asset", &self.asset)
            .field("ogg", &self.ogg)
            .finish_non_exhaustive()
    }
}

impl PreparedOggImport {
    /// Immutable receipt preview for filesystem-free authoring evaluation.
    ///
    /// `deduplicated` is necessarily `false` until Store installation determines whether the
    /// exact immutable object already exists. Callers must replace this preview with the actual
    /// receipt returned by [`WorkingProjectStore::install_prepared_ogg`] before exposing it.
    pub fn preview(&self) -> ImportedOgg {
        ImportedOgg {
            asset: self.asset.clone(),
            ogg: self.ogg.clone(),
            deduplicated: false,
        }
    }

    /// Compare every accepted source byte and all derived metadata with a second preparation.
    pub fn has_same_content(&self, other: &Self) -> bool {
        self == other
    }
}

/// Stable context for failures produced while importing an external Ogg.
///
/// Store/CAS failures remain distinct from correctable source-file failures,
/// while [`WorkingProjectStore::import_ogg`] preserves its legacy flattened
/// [`WorkingStoreError`] API for existing callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OggImportFailureContext {
    Store,
    SourceMissing,
    SourceUnavailable,
    SourceUnsafe,
    SourceLimit,
    SourceInvalid,
    SourceChanged,
}

#[derive(Debug, thiserror::Error)]
#[error("{source}")]
pub struct OggImportError {
    context: OggImportFailureContext,
    #[source]
    source: WorkingStoreError,
}

impl OggImportError {
    pub fn context(&self) -> OggImportFailureContext {
        self.context
    }

    pub fn into_store_error(self) -> WorkingStoreError {
        self.source
    }

    fn source(context: OggImportFailureContext, source: WorkingStoreError) -> Self {
        Self { context, source }
    }
}

impl From<WorkingStoreError> for OggImportError {
    fn from(source: WorkingStoreError) -> Self {
        Self::source(OggImportFailureContext::Store, source)
    }
}

fn classify_ogg_source_error(error: WorkingStoreError, missing_hint: bool) -> OggImportError {
    let context = match &error {
        WorkingStoreError::Io(source)
            if missing_hint && source.kind() == io::ErrorKind::NotFound =>
        {
            OggImportFailureContext::SourceMissing
        }
        WorkingStoreError::Io(_) => OggImportFailureContext::SourceUnavailable,
        WorkingStoreError::UnsafePath { .. } => OggImportFailureContext::SourceUnsafe,
        WorkingStoreError::LimitExceeded {
            kind: "Ogg bytes", ..
        } => OggImportFailureContext::SourceLimit,
        WorkingStoreError::InvalidOgg(_) => OggImportFailureContext::SourceInvalid,
        WorkingStoreError::Invariant(message)
            if message.starts_with("Ogg source length changed while reading") =>
        {
            OggImportFailureContext::SourceChanged
        }
        _ => OggImportFailureContext::Store,
    };
    OggImportError::source(context, error)
}

/// Content-addressed result of installing one upstream-verified canonical Quest artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedQuestCollisionArtifactV1 {
    pub artifact: ContentSeal,
    pub asset_meta: AssetMeta,
    pub deduplicated: bool,
}

/// Content-addressed result of installing one upstream-verified canonical version-2 Quest
/// artifact against one exact currently published basis head.
///
/// This receipt is structural storage metadata only. It does not authenticate the semantic source
/// seal and grants no artifact, build, runtime, source-inspection, publication, or head-publishing
/// authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedQuestCollisionArtifactV2 {
    pub artifact: ContentSeal,
    pub asset_meta: AssetMeta,
    pub basis_head: WorkingHead,
    pub deduplicated: bool,
}

/// Hard failures are store corruption, unsafe paths, resource-limit violations, or I/O errors.
/// Semantic authoring diagnostics are returned by successful checkpoint operations instead.
#[derive(Debug, thiserror::Error)]
pub enum WorkingStoreError {
    #[error("invalid working-store limits: {0}")]
    InvalidLimits(String),
    #[error("unsafe working-store path {path:?}: {reason}")]
    UnsafePath { path: PathBuf, reason: String },
    #[error("working-store resource limit exceeded for {kind}: {actual} > {limit}")]
    LimitExceeded {
        kind: &'static str,
        actual: u64,
        limit: u64,
    },
    #[error("working-store head conflict: expected {expected:?}, current is {actual:?}")]
    HeadConflict {
        expected: Option<WorkingHead>,
        actual: Option<WorkingHead>,
    },
    #[error("working-store head does not exist at {0:?}")]
    MissingHead(PathBuf),
    #[error("working-store root does not exist at {0:?}")]
    MissingRoot(PathBuf),
    #[error("immutable object is missing at {0:?}")]
    MissingObject(PathBuf),
    #[error("immutable object at {path:?} has an invalid seal: {reason}")]
    SealMismatch { path: PathBuf, reason: String },
    #[error("immutable object collision at {path:?}: {reason}")]
    Collision { path: PathBuf, reason: String },
    #[error("invalid {kind} JSON: {source}")]
    InvalidJson {
        kind: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("{kind} JSON is not in canonical store encoding")]
    NonCanonicalJson { kind: &'static str },
    #[error("invalid working-store invariant: {0}")]
    Invariant(String),
    #[error("invalid Ogg asset: {0}")]
    InvalidOgg(String),
    #[error(
        "voice take {entity} Ogg metadata for asset {asset} does not match the validated blob: declared {declared:?}, actual {actual:?}"
    )]
    OggMetadataMismatch {
        entity: EntityId,
        asset: Sha256Digest,
        declared: OggMetadata,
        actual: OggMetadata,
    },
    #[error("failed to remove working-store staging file {path:?}: {source}")]
    StagingCleanup {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotManifest {
    store_format: WorkingStoreFormat,
    format: FormatV2,
    schema_revision: SchemaRevisionV1,
    project_id: ProjectId,
    #[serde(default)]
    revision: u64,
    meta: ProjectMeta,
    target: GameGenerationAnchor,
    #[serde(default, deserialize_with = "deserialize_unique_set")]
    authoring_locales: BTreeSet<LocaleCode>,
    #[serde(default, deserialize_with = "deserialize_unique_map")]
    entities: BTreeMap<EntityId, ContentSeal>,
    asset_store: AssetStoreIndex,
}

/// Revision-2 snapshots deliberately have their own closed parser. Keeping this separate from
/// [`SnapshotManifest`] freezes revision-1 serialization and rejects cross-revision field drift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotManifestRevision2 {
    store_format: WorkingStoreFormat,
    format: FormatV2,
    schema_revision: SchemaRevisionV2,
    project_id: ProjectId,
    #[serde(default)]
    revision: u64,
    meta: ProjectMeta,
    target: GameGenerationAnchor,
    #[serde(default, deserialize_with = "deserialize_unique_set")]
    authoring_locales: BTreeSet<LocaleCode>,
    #[serde(default, deserialize_with = "deserialize_unique_map")]
    entities: BTreeMap<EntityId, ContentSeal>,
    asset_store: AssetStoreIndex,
}

/// Closed immutable snapshot manifest for schema revision 3.
///
/// It remains separate from the frozen revision-1 and revision-2 manifests so neither older wire
/// parser nor their canonical bytes need to change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3SnapshotManifest {
    pub store_format: WorkingStoreFormat,
    pub format: FormatV2,
    pub schema_revision: SchemaRevisionV3,
    pub project_id: ProjectId,
    #[serde(default)]
    pub revision: u64,
    pub meta: ProjectMeta,
    pub target: GameGenerationAnchor,
    #[serde(default, deserialize_with = "deserialize_unique_set")]
    pub authoring_locales: BTreeSet<LocaleCode>,
    #[serde(default, deserialize_with = "deserialize_unique_map")]
    pub entities: BTreeMap<EntityId, ContentSeal>,
    pub asset_store: AssetStoreIndex,
    /// Complete bounded retained timeline, when this snapshot was prepared as a successor by a
    /// history-aware revision-3 Store. Legacy/root snapshots omit this field byte-for-byte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<Revision3CheckpointHistoryV1>,
}

impl SnapshotManifestRevision2 {
    fn from_project(project: &ProjectRevision2, entities: BTreeMap<EntityId, ContentSeal>) -> Self {
        Self {
            store_format: WorkingStoreFormat,
            format: project.format,
            schema_revision: project.schema_revision,
            project_id: project.project_id,
            revision: project.revision,
            meta: project.meta.clone(),
            target: project.target.clone(),
            authoring_locales: project.authoring_locales.clone(),
            entities,
            asset_store: project.asset_store.clone(),
        }
    }

    fn into_project(self, entities: BTreeMap<EntityId, Revision2Entity>) -> ProjectRevision2 {
        ProjectRevision2 {
            format: self.format,
            schema_revision: self.schema_revision,
            project_id: self.project_id,
            revision: self.revision,
            meta: self.meta,
            target: self.target,
            authoring_locales: self.authoring_locales,
            entities,
            asset_store: self.asset_store,
        }
    }
}

impl Revision3SnapshotManifest {
    fn from_project_with_history(
        project: &ProjectRevision3,
        entities: BTreeMap<EntityId, ContentSeal>,
        history: Option<Revision3CheckpointHistoryV1>,
    ) -> Self {
        Self {
            store_format: WorkingStoreFormat,
            format: project.format,
            schema_revision: project.schema_revision,
            project_id: project.project_id,
            revision: project.revision,
            meta: project.meta.clone(),
            target: project.target.clone(),
            authoring_locales: project.authoring_locales.clone(),
            entities,
            asset_store: project.asset_store.clone(),
            history,
        }
    }

    fn into_project(self, entities: BTreeMap<EntityId, Revision3Entity>) -> ProjectRevision3 {
        ProjectRevision3 {
            format: self.format,
            schema_revision: self.schema_revision,
            project_id: self.project_id,
            revision: self.revision,
            meta: self.meta,
            target: self.target,
            authoring_locales: self.authoring_locales,
            entities,
            asset_store: self.asset_store,
        }
    }
}

fn encode_revision3_snapshot(
    project: &ProjectRevision3,
    entities: BTreeMap<EntityId, ContentSeal>,
    history: Option<Revision3CheckpointHistoryV1>,
    limits: &WorkingStoreLimits,
) -> Result<Vec<u8>, WorkingStoreError> {
    let mut snapshot =
        Revision3SnapshotManifest::from_project_with_history(project, entities, None);
    let base_bytes = canonical_json(&snapshot)?;
    enforce_limit(
        "revision-3 base snapshot bytes",
        base_bytes.len(),
        limits.max_snapshot_bytes,
    )?;

    let Some(history) = history else {
        return Ok(base_bytes);
    };
    snapshot.history = Some(history);
    revision3_history::validate_revision3_checkpoint_history_v1(&snapshot, limits)?;
    let snapshot_bytes = canonical_json(&snapshot)?;
    enforce_limit(
        "revision-3 snapshot bytes",
        snapshot_bytes.len(),
        revision3_total_snapshot_limit(limits),
    )?;
    Ok(snapshot_bytes)
}

impl SnapshotManifest {
    fn from_project(project: &ProjectV2, entities: BTreeMap<EntityId, ContentSeal>) -> Self {
        Self {
            store_format: WorkingStoreFormat,
            format: project.format,
            schema_revision: project.schema_revision,
            project_id: project.project_id,
            revision: project.revision,
            meta: project.meta.clone(),
            target: project.target.clone(),
            authoring_locales: project.authoring_locales.clone(),
            entities,
            asset_store: project.asset_store.clone(),
        }
    }

    fn into_project(self, entities: BTreeMap<EntityId, Entity>) -> ProjectV2 {
        ProjectV2 {
            format: self.format,
            schema_revision: self.schema_revision,
            project_id: self.project_id,
            revision: self.revision,
            meta: self.meta,
            target: self.target,
            authoring_locales: self.authoring_locales,
            entities,
            asset_store: self.asset_store,
        }
    }
}

/// Rooted immutable working-object store for format-2 authoring projects.
#[derive(Debug, Clone)]
pub struct WorkingProjectStore {
    root: PathBuf,
    limits: WorkingStoreLimits,
}

impl WorkingProjectStore {
    /// Open or create a store root after rejecting links/reparse points in its prefix chain.
    pub fn at(
        root: impl AsRef<Path>,
        limits: WorkingStoreLimits,
    ) -> Result<Self, WorkingStoreError> {
        let limits = limits.validate()?;
        let root = absolute_path(root.as_ref())?;
        create_directory_chain(&root)?;
        ensure_safe_directory_chain(&root)?;
        Ok(Self { root, limits })
    }

    /// Open an existing store root without creating the root or any missing parent directory.
    ///
    /// This is the side-effect-free counterpart to [`Self::at`] for probes and read paths.
    pub fn open_existing(
        root: impl AsRef<Path>,
        limits: WorkingStoreLimits,
    ) -> Result<Self, WorkingStoreError> {
        let limits = limits.validate()?;
        let root = absolute_path(root.as_ref())?;
        match fs::symlink_metadata(&root) {
            Ok(_) => ensure_safe_directory_chain(&root)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(WorkingStoreError::MissingRoot(root));
            }
            Err(error) => return Err(error.into()),
        }
        Ok(Self { root, limits })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub const fn limits(&self) -> WorkingStoreLimits {
        self.limits
    }

    /// Parse the current fixed head, if present, without opening its snapshot.
    pub fn current_head(&self) -> Result<Option<WorkingHead>, WorkingStoreError> {
        self.ensure_root_safe()?;
        let path = self.head_path();
        let Some(bytes) =
            read_optional_regular_bounded(&path, self.limits.max_head_bytes, "head bytes")?
        else {
            return Ok(None);
        };
        let head: WorkingHead = parse_canonical_json(&bytes, "head")?;
        validate_nonzero_seal(
            &head.snapshot,
            revision3_total_snapshot_limit(&self.limits),
            "snapshot",
        )?;
        Ok(Some(head))
    }

    /// Write no-clobber immutable entity and snapshot objects, then fully reopen the candidate.
    /// The fixed `gore-project.json` head is never created or replaced by this method.
    /// `expected_head` is a strict CAS token: `None` requires the fixed head to be absent.
    pub fn prepare_checkpoint(
        &self,
        expected_head: Option<&WorkingHead>,
        project: &ProjectV2,
        profile: ValidationProfile,
    ) -> Result<CheckpointPreparation, WorkingStoreError> {
        self.ensure_root_safe()?;
        self.check_expected_head(expected_head)?;
        self.validate_project_limits(project)?;
        self.verify_asset_index(&project.asset_store, AssetVerification::Full)?;
        self.verify_voice_take_ogg_metadata(project, AssetVerification::Full)?;

        let mut entity_seals = BTreeMap::new();
        let mut total_entity_bytes = 0u64;
        for (id, entity) in &project.entities {
            if id != &entity.id {
                return Err(WorkingStoreError::Invariant(format!(
                    "entity map key {id} does not match embedded id {}",
                    entity.id
                )));
            }
            let bytes = canonical_json(entity)?;
            enforce_limit("entity bytes", bytes.len(), self.limits.max_entity_bytes)?;
            total_entity_bytes = checked_bounded_sum(
                "aggregate referenced entity bytes",
                total_entity_bytes,
                bytes.len() as u64,
                self.limits.max_referenced_entity_bytes,
            )?;
            let seal = seal_bytes(&bytes);
            let path = self.entity_path(*id, seal.sha256);
            self.install_immutable_bytes(&path, &bytes, &seal)?;
            entity_seals.insert(*id, seal);
        }

        let snapshot = SnapshotManifest::from_project(project, entity_seals);
        let snapshot_bytes = canonical_json(&snapshot)?;
        enforce_limit(
            "snapshot bytes",
            snapshot_bytes.len(),
            self.limits.max_snapshot_bytes,
        )?;
        let snapshot_seal = seal_bytes(&snapshot_bytes);
        let snapshot_path = self.snapshot_path(snapshot_seal.sha256);
        self.install_immutable_bytes(&snapshot_path, &snapshot_bytes, &snapshot_seal)?;

        let head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: snapshot_seal,
        };
        let head_bytes = canonical_json(&head)?;
        enforce_limit("head bytes", head_bytes.len(), self.limits.max_head_bytes)?;

        // A concurrent publisher may make these newly written objects orphaned. That is safe;
        // immutable objects are never overwritten and callers still own the fixed-head CAS.
        self.check_expected_head(expected_head)?;
        let reopened = self.open_head_bytes(&head_bytes, AssetVerification::Full, profile)?;
        if reopened.project != *project || reopened.head != head {
            return Err(WorkingStoreError::Invariant(
                "candidate checkpoint did not reconstitute exactly".to_owned(),
            ));
        }
        Ok(CheckpointPreparation {
            head_bytes,
            head,
            diagnostics: reopened.diagnostics,
            blocks_build: reopened.blocks_build,
        })
    }

    /// Prepare an immutable checkpoint through the frozen revision-1/2 document paths.
    ///
    /// Revision 1 dispatches directly to [`Self::prepare_checkpoint`], preserving its exact bytes
    /// and behavior. Revision 2 uses its own snapshot and entity parsers. Revision 3 is rejected:
    /// document parsing alone does not authorize its dedicated Store path. Neither accepted branch
    /// publishes the fixed head; callers retain the same strict head CAS contract.
    pub fn prepare_document_checkpoint(
        &self,
        expected_head: Option<&WorkingHead>,
        document: &ProjectDocument,
        profile: ValidationProfile,
    ) -> Result<CheckpointPreparation, WorkingStoreError> {
        match document {
            ProjectDocument::Revision1(project) => {
                self.prepare_checkpoint(expected_head, project, profile)
            }
            ProjectDocument::Revision2(project) => {
                self.prepare_revision2_checkpoint(expected_head, project, profile)
            }
            ProjectDocument::Revision3(_) => Err(WorkingStoreError::Invariant(
                "generic document checkpoints do not authorize schema revision 3; use the dedicated revision-3 checkpoint API"
                    .to_owned(),
            )),
        }
    }

    /// Prepare immutable entity and snapshot objects for one closed schema-revision-3 project.
    ///
    /// The fixed `gore-project.json` head is never created or replaced. The project is validated
    /// structurally, every referenced asset is fully length/hash verified, and Quest source is
    /// never regenerated. With `Some(expected_head)`, an exact unchanged project deterministically
    /// reproduces the same head and parent; every changed candidate must retain the exact project
    /// identity and target and advance its project revision by one, sealing that expected head as
    /// the immediate history parent. `None` creates a legacy-compatible lineage root.
    /// `expected_head` uses the same strict two-check CAS contract as the frozen revision-1 and
    /// revision-2 preparation paths.
    pub fn prepare_revision3_checkpoint(
        &self,
        expected_head: Option<&WorkingHead>,
        project: &ProjectRevision3,
    ) -> Result<Revision3CheckpointPreparation, WorkingStoreError> {
        self.prepare_revision3_checkpoint_with_write_guard(expected_head, project, || Ok(()))
    }

    /// Revision-3 checkpoint preparation with a caller-supplied guard immediately before every
    /// immutable Store installation. Managed DataAsset staging uses this to revalidate its opaque
    /// live executable binding at the actual write boundary.
    pub(crate) fn prepare_revision3_checkpoint_with_write_guard<F>(
        &self,
        expected_head: Option<&WorkingHead>,
        project: &ProjectRevision3,
        mut before_write: F,
    ) -> Result<Revision3CheckpointPreparation, WorkingStoreError>
    where
        F: FnMut() -> Result<(), WorkingStoreError>,
    {
        self.ensure_root_safe()?;
        self.check_expected_head(expected_head)?;
        validate_revision3_persistability(project, &self.limits)?;
        let history = match self.prepare_revision3_checkpoint_history_v1(expected_head, project)? {
            revision3_history::Revision3CheckpointHistoryPlanV1::Root => None,
            revision3_history::Revision3CheckpointHistoryPlanV1::Successor { history } => {
                Some(history)
            }
            revision3_history::Revision3CheckpointHistoryPlanV1::ExactNoOp { head } => {
                self.check_expected_head(expected_head)?;
                let head_bytes = canonical_json(&head)?;
                enforce_limit("head bytes", head_bytes.len(), self.limits.max_head_bytes)?;
                return Ok(Revision3CheckpointPreparation { head_bytes, head });
            }
        };

        // Complete all cheap, filesystem-free entity work before hashing external assets or
        // installing immutable objects.
        let mut prepared_entities = Vec::with_capacity(project.entities.len());
        for (id, entity) in &project.entities {
            let bytes = canonical_json(entity)?;
            let seal = seal_bytes(&bytes);
            prepared_entities.push((*id, bytes, seal));
        }

        self.verify_asset_index(&project.asset_store, AssetVerification::Full)?;
        self.verify_revision3_voice_take_ogg_metadata(project, AssetVerification::Full)?;
        self.verify_revision3_basis_snapshots(project)?;

        let mut entity_seals = BTreeMap::new();
        for (id, bytes, seal) in prepared_entities {
            let path = self.entity_path(id, seal.sha256);
            self.install_checkpoint_bytes_with_write_guard(
                &path,
                &bytes,
                &seal,
                &mut before_write,
            )?;
            entity_seals.insert(id, seal);
        }

        let snapshot_bytes =
            encode_revision3_snapshot(project, entity_seals, history, &self.limits)?;
        let snapshot_seal = seal_bytes(&snapshot_bytes);
        let snapshot_path = self.snapshot_path(snapshot_seal.sha256);
        self.install_checkpoint_bytes_with_write_guard(
            &snapshot_path,
            &snapshot_bytes,
            &snapshot_seal,
            &mut before_write,
        )?;

        let head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: snapshot_seal,
        };
        let head_bytes = canonical_json(&head)?;
        enforce_limit("head bytes", head_bytes.len(), self.limits.max_head_bytes)?;

        // Objects installed after a concurrent head advance are safe immutable orphans. The
        // second check prevents returning a preparation based on a stale CAS token.
        self.check_expected_head(expected_head)?;
        let reopened = self.open_revision3_head_bytes(&head_bytes, AssetVerification::Full)?;
        if reopened.project != *project || reopened.head != head {
            return Err(WorkingStoreError::Invariant(
                "revision-3 candidate checkpoint did not reconstitute exactly".to_owned(),
            ));
        }
        Ok(Revision3CheckpointPreparation { head_bytes, head })
    }

    /// Open the currently published fixed head and exactly reconstitute its [`ProjectV2`].
    pub fn open_current(
        &self,
        verification: AssetVerification,
        profile: ValidationProfile,
    ) -> Result<OpenedCheckpoint, WorkingStoreError> {
        self.ensure_root_safe()?;
        let path = self.head_path();
        let bytes = read_required_regular_bounded(&path, self.limits.max_head_bytes, "head bytes")?;
        self.open_head_bytes(&bytes, verification, profile)
    }

    /// Open the published fixed head and dispatch its immutable snapshot to one closed revision.
    pub fn open_current_document(
        &self,
        verification: AssetVerification,
        profile: ValidationProfile,
    ) -> Result<OpenedDocumentCheckpoint, WorkingStoreError> {
        self.ensure_root_safe()?;
        let path = self.head_path();
        let bytes = read_required_regular_bounded(&path, self.limits.max_head_bytes, "head bytes")?;
        self.open_head_bytes_document(&bytes, verification, profile)
    }

    /// Open the currently published fixed head as schema revision 3 only.
    pub fn open_current_revision3(
        &self,
        verification: AssetVerification,
    ) -> Result<OpenedRevision3Checkpoint, WorkingStoreError> {
        self.ensure_root_safe()?;
        let path = self.head_path();
        let bytes = read_required_regular_bounded(&path, self.limits.max_head_bytes, "head bytes")?;
        self.open_revision3_head_bytes(&bytes, verification)
    }

    /// Prepare exact-current-project collision source evidence for additive revision-3 Quests.
    ///
    /// Only the fixed head's exact current snapshot and entity shards are source inputs. Prior
    /// Quest artifact blobs and every historical `basis_snapshot` are intentionally neither read
    /// nor trusted. All remaining non-Quest assets are fully verified before an opaque capsule is
    /// returned. `expected_head` is checked before and after the complete operation.
    pub fn prepare_current_revision3_quest_collision_source_v2(
        &self,
        expected_head: &WorkingHead,
    ) -> Result<PreparedRevision3QuestCollisionSourceV2, Revision3QuestCollisionSourceErrorV2> {
        self.prepare_current_revision3_quest_collision_source_v2_with_final_head_hook(
            expected_head,
            || Ok(()),
        )
    }

    fn prepare_current_revision3_quest_collision_source_v2_with_final_head_hook<F>(
        &self,
        expected_head: &WorkingHead,
        before_final_head_check: F,
    ) -> Result<PreparedRevision3QuestCollisionSourceV2, Revision3QuestCollisionSourceErrorV2>
    where
        F: FnOnce() -> Result<(), WorkingStoreError>,
    {
        self.ensure_root_safe()?;
        self.check_expected_head(Some(expected_head))?;
        let prepared = self.prepare_revision3_quest_collision_source_from_sealed_head_v2(
            expected_head,
            "current revision-3",
        )?;

        before_final_head_check()?;
        self.check_expected_head(Some(expected_head))?;
        Ok(prepared)
    }

    /// Reconstruct inspection-only collision evidence for one immutable historical head.
    ///
    /// Unlike [`Self::prepare_current_revision3_quest_collision_source_v2`], this path never
    /// compares with or returns authority over the fixed head. The distinct return type cannot be
    /// converted into authoring capability, cannot resolve catalog selections, and exists only so
    /// a caller can linearly verify the exact version-2 artifact that names this historical head.
    pub fn prepare_revision3_quest_collision_inspection_source_v2(
        &self,
        historical_head: &WorkingHead,
    ) -> Result<
        PreparedRevision3QuestCollisionInspectionSourceV2,
        Revision3QuestCollisionSourceErrorV2,
    > {
        self.ensure_root_safe()?;
        self.prepare_revision3_quest_collision_source_from_sealed_head_v2(
            historical_head,
            "historical revision-3 Quest inspection",
        )
        .map(PreparedRevision3QuestCollisionInspectionSourceV2::new)
    }

    fn prepare_revision3_quest_collision_source_from_sealed_head_v2(
        &self,
        source_head: &WorkingHead,
        source_kind: &'static str,
    ) -> Result<PreparedRevision3QuestCollisionSourceV2, Revision3QuestCollisionSourceErrorV2> {
        validate_nonzero_seal(
            &source_head.snapshot,
            revision3_total_snapshot_limit(&self.limits),
            "revision-3 Quest collision source snapshot",
        )?;

        let snapshot_path = self.snapshot_path(source_head.snapshot.sha256);
        let snapshot_bytes = self.read_sealed_object(
            &snapshot_path,
            &source_head.snapshot,
            revision3_total_snapshot_limit(&self.limits),
            "revision-3 Quest collision source snapshot",
            AssetVerification::Full,
        )?;
        let snapshot: Revision3SnapshotManifest = parse_canonical_json(
            &snapshot_bytes,
            "revision-3 Quest collision source snapshot",
        )?;
        self.validate_revision3_manifest_limits(&snapshot)?;

        // A monolithic current project is capped at 16 MiB. Entity JSON is a strict lower bound
        // on that spelling, so reject an impossible candidate before allocating/reading as much
        // as the store's broader 512 MiB shard aggregate allowance.
        let entity_bytes_lower_bound =
            snapshot.entities.values().try_fold(0u64, |total, seal| {
                total.checked_add(seal.byte_len).ok_or(
                    Revision3QuestCollisionSourceErrorV2::Limit {
                        kind: "canonical current project bytes",
                        actual: usize::MAX,
                        limit: crate::MAX_PROJECT_JSON_BYTES,
                    },
                )
            })?;
        if entity_bytes_lower_bound > crate::MAX_PROJECT_JSON_BYTES as u64 {
            return Err(Revision3QuestCollisionSourceErrorV2::Limit {
                kind: "canonical current project bytes",
                actual: usize::try_from(entity_bytes_lower_bound).unwrap_or(usize::MAX),
                limit: crate::MAX_PROJECT_JSON_BYTES,
            });
        }

        let mut entities = BTreeMap::new();
        let mut entity_seals = BTreeMap::new();
        for (id, seal) in &snapshot.entities {
            let entity_path = self.entity_path(*id, seal.sha256);
            let entity_bytes = self.read_sealed_object(
                &entity_path,
                seal,
                self.limits.max_entity_bytes,
                "revision-3 Quest collision source entity",
                AssetVerification::Full,
            )?;
            let entity: Revision3Entity =
                parse_canonical_json(&entity_bytes, "revision-3 Quest collision source entity")?;
            if entity.id != *id {
                return Err(
                    Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject {
                        reason: format!(
                            "{source_kind} entity shard {id} contains embedded id {}",
                            entity.id,
                        ),
                    },
                );
            }
            entities.insert(*id, entity);
            entity_seals.insert(*id, seal.clone());
        }
        let original_manifest = snapshot.clone();
        let original_history = snapshot.history.clone();
        let project = snapshot.into_project(entities);
        if Revision3SnapshotManifest::from_project_with_history(
            &project,
            entity_seals,
            original_history,
        ) != original_manifest
        {
            return Err(Revision3QuestCollisionSourceErrorV2::CurrentSnapshotDrift);
        }

        let prepared =
            crate::revision3_quest_source_v2::prepare_exact_revision3_quest_collision_source_v2(
                &project,
                source_head.clone(),
            )?;

        // Verify only the exact non-Quest projection. Historical Quest artifacts removed by the
        // splitter and their historical basis snapshots are deliberately absent from this I/O.
        let nonquest = prepared.nonquest_basis().project();
        validate_revision2_persistability(nonquest, &self.limits)?;
        self.verify_asset_index(&nonquest.asset_store, AssetVerification::Full)?;
        self.verify_revision2_voice_take_ogg_metadata(nonquest, AssetVerification::Full)?;

        Ok(prepared)
    }

    /// Open canonical head bytes and require their sealed snapshot to be schema revision 3.
    pub fn open_revision3_head_bytes(
        &self,
        bytes: &[u8],
        verification: AssetVerification,
    ) -> Result<OpenedRevision3Checkpoint, WorkingStoreError> {
        self.ensure_root_safe()?;
        enforce_limit("head bytes", bytes.len(), self.limits.max_head_bytes)?;
        let head: WorkingHead = parse_canonical_json(bytes, "head")?;
        self.open_revision3_snapshot_with_head(head, verification)
    }

    /// Reopen one exact immutable schema-revision-3 snapshot independently of the fixed head.
    ///
    /// This is the basis-snapshot path: advancing `gore-project.json` never invalidates an older
    /// content-addressed snapshot. The returned `head` is only the canonical envelope for the
    /// requested snapshot and makes no claim about current publication state.
    pub fn open_revision3_snapshot(
        &self,
        snapshot: &ContentSeal,
        verification: AssetVerification,
    ) -> Result<OpenedRevision3Checkpoint, WorkingStoreError> {
        self.ensure_root_safe()?;
        self.open_revision3_snapshot_with_head(
            WorkingHead {
                store_format: WorkingStoreFormat,
                snapshot: snapshot.clone(),
            },
            verification,
        )
    }

    /// Validate canonical head bytes, all immutable manifests, and optionally complete assets.
    pub fn open_head_bytes(
        &self,
        bytes: &[u8],
        verification: AssetVerification,
        profile: ValidationProfile,
    ) -> Result<OpenedCheckpoint, WorkingStoreError> {
        self.ensure_root_safe()?;
        enforce_limit("head bytes", bytes.len(), self.limits.max_head_bytes)?;
        let head: WorkingHead = parse_canonical_json(bytes, "head")?;
        validate_nonzero_seal(&head.snapshot, self.limits.max_snapshot_bytes, "snapshot")?;

        let snapshot_path = self.snapshot_path(head.snapshot.sha256);
        let snapshot_bytes = self.read_sealed_object(
            &snapshot_path,
            &head.snapshot,
            self.limits.max_snapshot_bytes,
            "snapshot",
            AssetVerification::Full,
        )?;
        let snapshot: SnapshotManifest = parse_canonical_json(&snapshot_bytes, "snapshot")?;
        self.validate_manifest_limits(&snapshot)?;

        let mut total_entity_bytes = 0u64;
        for seal in snapshot.entities.values() {
            validate_nonzero_seal(seal, self.limits.max_entity_bytes, "entity")?;
            total_entity_bytes = checked_bounded_sum(
                "aggregate referenced entity bytes",
                total_entity_bytes,
                seal.byte_len,
                self.limits.max_referenced_entity_bytes,
            )?;
        }
        // Finish every cheap manifest and entity-seal rejection before any attacker-amplifiable
        // asset stat/hash work (up to 100k entries / 64 GiB by format limits).
        self.verify_asset_index(&snapshot.asset_store, verification)?;

        let mut entities = BTreeMap::new();
        for (id, seal) in &snapshot.entities {
            let entity_path = self.entity_path(*id, seal.sha256);
            let entity_bytes = self.read_sealed_object(
                &entity_path,
                seal,
                self.limits.max_entity_bytes,
                "entity",
                AssetVerification::Full,
            )?;
            let entity: Entity = parse_canonical_json(&entity_bytes, "entity")?;
            if entity.id != *id {
                return Err(WorkingStoreError::Invariant(format!(
                    "entity shard {} contains embedded id {}",
                    id, entity.id
                )));
            }
            entities.insert(*id, entity);
        }

        let project = snapshot.into_project(entities);
        self.validate_project_limits(&project)?;
        self.verify_voice_take_ogg_metadata(&project, verification)?;
        let diagnostics = project.validate_with_profile(profile);
        let blocks_build = diagnostics.iter().any(|item| item.blocks_build);
        Ok(OpenedCheckpoint {
            head,
            project,
            diagnostics,
            blocks_build,
        })
    }

    /// Validate canonical head bytes and dispatch the sealed snapshot by schema revision.
    ///
    /// The revision probe only selects a parser. Each selected manifest and every entity shard is
    /// then parsed canonically through its revision-specific closed model.
    pub fn open_head_bytes_document(
        &self,
        bytes: &[u8],
        verification: AssetVerification,
        profile: ValidationProfile,
    ) -> Result<OpenedDocumentCheckpoint, WorkingStoreError> {
        self.ensure_root_safe()?;
        enforce_limit("head bytes", bytes.len(), self.limits.max_head_bytes)?;
        let head: WorkingHead = parse_canonical_json(bytes, "head")?;
        validate_nonzero_seal(&head.snapshot, self.limits.max_snapshot_bytes, "snapshot")?;

        let snapshot_path = self.snapshot_path(head.snapshot.sha256);
        let snapshot_bytes = self.read_sealed_object(
            &snapshot_path,
            &head.snapshot,
            self.limits.max_snapshot_bytes,
            "snapshot",
            AssetVerification::Full,
        )?;
        let probe = parse_snapshot_revision(&snapshot_bytes)?;
        match probe {
            1 => {
                // Keep revision-1 behavior centralized in the frozen API, including validation
                // ordering and every byte-level invariant.
                let opened = self.open_head_bytes(bytes, verification, profile)?;
                Ok(OpenedDocumentCheckpoint {
                    head: opened.head,
                    project: ProjectDocument::Revision1(opened.project),
                    diagnostics: opened.diagnostics,
                    blocks_build: opened.blocks_build,
                })
            }
            2 => self.open_revision2_snapshot(head, &snapshot_bytes, verification, profile),
            found => Err(WorkingStoreError::Invariant(format!(
                "unsupported working-store snapshot schema revision {found}; expected 1 or 2"
            ))),
        }
    }

    fn prepare_revision2_checkpoint(
        &self,
        expected_head: Option<&WorkingHead>,
        project: &ProjectRevision2,
        profile: ValidationProfile,
    ) -> Result<CheckpointPreparation, WorkingStoreError> {
        self.ensure_root_safe()?;
        self.check_expected_head(expected_head)?;
        validate_revision2_persistability(project, &self.limits)?;

        // Finish every cheap, read-only entity rejection before hashing potentially large asset
        // files. Besides deterministic error ordering, this guarantees malformed/oversized
        // in-memory documents cannot leave immutable entity or snapshot objects behind.
        let mut prepared_entities = Vec::with_capacity(project.entities.len());
        for (id, entity) in &project.entities {
            let bytes = canonical_json(entity)?;
            let seal = seal_bytes(&bytes);
            prepared_entities.push((*id, bytes, seal));
        }

        self.verify_asset_index(&project.asset_store, AssetVerification::Full)?;
        self.verify_revision2_voice_take_ogg_metadata(project, AssetVerification::Full)?;

        let mut entity_seals = BTreeMap::new();
        for (id, bytes, seal) in prepared_entities {
            let path = self.entity_path(id, seal.sha256);
            self.install_immutable_bytes(&path, &bytes, &seal)?;
            entity_seals.insert(id, seal);
        }

        let snapshot = SnapshotManifestRevision2::from_project(project, entity_seals);
        let snapshot_bytes = canonical_json(&snapshot)?;
        enforce_limit(
            "snapshot bytes",
            snapshot_bytes.len(),
            self.limits.max_snapshot_bytes,
        )?;
        let snapshot_seal = seal_bytes(&snapshot_bytes);
        let snapshot_path = self.snapshot_path(snapshot_seal.sha256);
        self.install_immutable_bytes(&snapshot_path, &snapshot_bytes, &snapshot_seal)?;

        let head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: snapshot_seal,
        };
        let head_bytes = canonical_json(&head)?;
        enforce_limit("head bytes", head_bytes.len(), self.limits.max_head_bytes)?;

        self.check_expected_head(expected_head)?;
        let reopened =
            self.open_head_bytes_document(&head_bytes, AssetVerification::Full, profile)?;
        if reopened.project != ProjectDocument::Revision2(project.clone()) || reopened.head != head
        {
            return Err(WorkingStoreError::Invariant(
                "revision-2 candidate checkpoint did not reconstitute exactly".to_owned(),
            ));
        }
        Ok(CheckpointPreparation {
            head_bytes,
            head,
            diagnostics: reopened.diagnostics,
            blocks_build: reopened.blocks_build,
        })
    }

    fn open_revision2_snapshot(
        &self,
        head: WorkingHead,
        snapshot_bytes: &[u8],
        verification: AssetVerification,
        profile: ValidationProfile,
    ) -> Result<OpenedDocumentCheckpoint, WorkingStoreError> {
        let snapshot: SnapshotManifestRevision2 =
            parse_canonical_json(snapshot_bytes, "revision-2 snapshot")?;
        self.validate_revision2_manifest_limits(&snapshot)?;

        let mut total_entity_bytes = 0u64;
        for seal in snapshot.entities.values() {
            validate_nonzero_seal(seal, self.limits.max_entity_bytes, "entity")?;
            total_entity_bytes = checked_bounded_sum(
                "aggregate referenced entity bytes",
                total_entity_bytes,
                seal.byte_len,
                self.limits.max_referenced_entity_bytes,
            )?;
        }
        self.verify_asset_index(&snapshot.asset_store, verification)?;

        let mut entities = BTreeMap::new();
        for (id, seal) in &snapshot.entities {
            let entity_path = self.entity_path(*id, seal.sha256);
            let entity_bytes = self.read_sealed_object(
                &entity_path,
                seal,
                self.limits.max_entity_bytes,
                "revision-2 entity",
                AssetVerification::Full,
            )?;
            let entity: Revision2Entity = parse_canonical_json(&entity_bytes, "revision-2 entity")?;
            if entity.id != *id {
                return Err(WorkingStoreError::Invariant(format!(
                    "revision-2 entity shard {id} contains embedded id {}",
                    entity.id
                )));
            }
            entities.insert(*id, entity);
        }

        let project = snapshot.into_project(entities);
        validate_revision2_persistability(&project, &self.limits)?;
        self.verify_revision2_voice_take_ogg_metadata(&project, verification)?;
        let diagnostics = revision2_checkpoint_diagnostics(&project, profile);
        let blocks_build = diagnostics.iter().any(|item| item.blocks_build);
        debug_assert!(blocks_build, "revision-2 checkpoints must stay fail-closed");
        Ok(OpenedDocumentCheckpoint {
            head,
            project: ProjectDocument::Revision2(project),
            diagnostics,
            blocks_build,
        })
    }

    fn open_revision3_snapshot_with_head(
        &self,
        head: WorkingHead,
        verification: AssetVerification,
    ) -> Result<OpenedRevision3Checkpoint, WorkingStoreError> {
        validate_nonzero_seal(
            &head.snapshot,
            revision3_total_snapshot_limit(&self.limits),
            "revision-3 snapshot",
        )?;
        let snapshot_path = self.snapshot_path(head.snapshot.sha256);
        let snapshot_bytes = self.read_sealed_object(
            &snapshot_path,
            &head.snapshot,
            revision3_total_snapshot_limit(&self.limits),
            "revision-3 snapshot",
            AssetVerification::Full,
        )?;
        let snapshot: Revision3SnapshotManifest =
            parse_canonical_json(&snapshot_bytes, "revision-3 snapshot")?;
        self.validate_revision3_manifest_limits(&snapshot)?;
        self.verify_asset_index(&snapshot.asset_store, verification)?;

        let mut entities = BTreeMap::new();
        for (id, seal) in &snapshot.entities {
            let entity_path = self.entity_path(*id, seal.sha256);
            let entity_bytes = self.read_sealed_object(
                &entity_path,
                seal,
                self.limits.max_entity_bytes,
                "revision-3 entity",
                AssetVerification::Full,
            )?;
            let entity: Revision3Entity = parse_canonical_json(&entity_bytes, "revision-3 entity")?;
            if entity.id != *id {
                return Err(WorkingStoreError::Invariant(format!(
                    "revision-3 entity shard {id} contains embedded id {}",
                    entity.id
                )));
            }
            entities.insert(*id, entity);
        }

        let project = snapshot.into_project(entities);
        validate_revision3_persistability(&project, &self.limits)?;
        self.verify_revision3_voice_take_ogg_metadata(&project, verification)?;
        self.verify_revision3_basis_snapshots(&project)?;
        Ok(OpenedRevision3Checkpoint { head, project })
    }

    /// Install upstream-verified canonical Quest collision artifact bytes in the ordinary asset
    /// CAS without parsing their semantic seal or making readiness claims.
    ///
    /// The caller must supply the exact canonical bytes returned by the version-1 artifact
    /// capability. Semantic reopen remains in `gore-story-inventory`; this lower store layer only
    /// enforces the exact 24 MiB boundary, raw SHA-256 identity, no-clobber installation, and head
    /// CAS. `None` requires the fixed head to remain absent through installation and final full
    /// verification; a raced install remains only an immutable orphan and returns a conflict.
    pub fn import_quest_collision_artifact_v1(
        &self,
        canonical_bytes: &[u8],
        expected_head: Option<&WorkingHead>,
    ) -> Result<ImportedQuestCollisionArtifactV1, WorkingStoreError> {
        self.import_quest_collision_artifact_v1_with_final_head_hook(
            canonical_bytes,
            expected_head,
            || Ok(()),
        )
    }

    /// Install upstream-verified canonical version-2 Quest collision artifact bytes in the
    /// ordinary asset CAS while the exact current basis head remains fixed.
    ///
    /// This storage-only boundary validates the unchanged 24 MiB cap, computes the raw content
    /// identity, installs without clobbering, fully re-reads the stored blob, and checks the basis
    /// head before staging, before installation, and after full verification. A late head race can
    /// therefore leave only the immutable content-addressed blob as an orphan. The semantic source
    /// seal is deliberately not accepted here and the fixed head is never published.
    pub fn import_quest_collision_artifact_v2(
        &self,
        canonical_bytes: &[u8],
        basis_head: &WorkingHead,
    ) -> Result<ImportedQuestCollisionArtifactV2, WorkingStoreError> {
        self.import_quest_collision_artifact_v2_with_final_head_hook(
            canonical_bytes,
            basis_head,
            || Ok(()),
        )
    }

    fn import_quest_collision_artifact_v2_with_final_head_hook<F>(
        &self,
        canonical_bytes: &[u8],
        basis_head: &WorkingHead,
        before_final_head_check: F,
    ) -> Result<ImportedQuestCollisionArtifactV2, WorkingStoreError>
    where
        F: FnOnce() -> Result<(), WorkingStoreError>,
    {
        validate_quest_collision_artifact_length(canonical_bytes.len() as u64)?;
        let artifact = seal_bytes(canonical_bytes);

        self.ensure_root_safe()?;
        self.check_expected_head(Some(basis_head))?;
        let (temp_path, mut temp) = self.create_temp_file()?;
        let result = (|| {
            temp.write_all(canonical_bytes)?;
            temp.flush()?;
            temp.sync_all()?;
            self.check_expected_head(Some(basis_head))?;
            let destination = self.asset_path(artifact.sha256);
            drop(temp);
            let deduplicated = self.install_staged_file(&temp_path, &destination, &artifact)?;
            self.verify_seal_at(&destination, &artifact, AssetVerification::Full, false)?;
            before_final_head_check()?;
            self.check_expected_head(Some(basis_head))?;
            Ok(ImportedQuestCollisionArtifactV2 {
                asset_meta: AssetMeta {
                    byte_len: artifact.byte_len,
                    media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
                },
                basis_head: basis_head.clone(),
                artifact,
                deduplicated,
            })
        })();
        let cleanup = cleanup_staged_file(&temp_path);
        match (result, cleanup) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(_), Err(source)) => Err(WorkingStoreError::StagingCleanup {
                path: temp_path,
                source,
            }),
            (Err(error), _) => Err(error),
        }
    }

    fn import_quest_collision_artifact_v1_with_final_head_hook<F>(
        &self,
        canonical_bytes: &[u8],
        expected_head: Option<&WorkingHead>,
        before_final_head_check: F,
    ) -> Result<ImportedQuestCollisionArtifactV1, WorkingStoreError>
    where
        F: FnOnce() -> Result<(), WorkingStoreError>,
    {
        validate_quest_collision_artifact_length(canonical_bytes.len() as u64)?;
        let artifact = seal_bytes(canonical_bytes);

        self.ensure_root_safe()?;
        self.check_expected_head(expected_head)?;
        let (temp_path, mut temp) = self.create_temp_file()?;
        let result = (|| {
            temp.write_all(canonical_bytes)?;
            temp.flush()?;
            temp.sync_all()?;
            self.check_expected_head(expected_head)?;
            let destination = self.asset_path(artifact.sha256);
            drop(temp);
            let deduplicated = self.install_staged_file(&temp_path, &destination, &artifact)?;
            self.verify_seal_at(&destination, &artifact, AssetVerification::Full, false)?;
            before_final_head_check()?;
            self.check_expected_head(expected_head)?;
            Ok(ImportedQuestCollisionArtifactV1 {
                asset_meta: AssetMeta {
                    byte_len: artifact.byte_len,
                    media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE.to_owned(),
                },
                artifact,
                deduplicated,
            })
        })();
        let cleanup = cleanup_staged_file(&temp_path);
        match (result, cleanup) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(_), Err(source)) => Err(WorkingStoreError::StagingCleanup {
                path: temp_path,
                source,
            }),
            (Err(error), _) => Err(error),
        }
    }

    /// Read one version-1 Quest collision artifact only through its exact indexed raw identity.
    ///
    /// Seal bounds and index digest/length/media are checked before any filesystem access. The
    /// subsequent CAS read always streams and verifies the complete SHA-256 regardless of browse
    /// verification policy. The independent semantic source seal is intentionally not accepted by
    /// this storage-only API.
    pub fn read_indexed_quest_collision_artifact_v1(
        &self,
        index: &AssetStoreIndex,
        artifact: &ContentSeal,
    ) -> Result<Vec<u8>, WorkingStoreError> {
        validate_quest_collision_artifact_length(artifact.byte_len)?;
        let meta = index.assets.get(&artifact.sha256).ok_or_else(|| {
            WorkingStoreError::Invariant(format!(
                "Quest collision artifact {} is absent from the supplied asset index",
                artifact.sha256
            ))
        })?;
        if meta.byte_len != artifact.byte_len {
            return Err(WorkingStoreError::Invariant(format!(
                "Quest collision artifact {} index declares {} bytes, raw seal declares {}",
                artifact.sha256, meta.byte_len, artifact.byte_len
            )));
        }
        if meta.media_type != QUEST_COLLISION_ARTIFACT_MEDIA_TYPE {
            return Err(WorkingStoreError::Invariant(format!(
                "Quest collision artifact {} media type is {:?}, expected {:?}",
                artifact.sha256, meta.media_type, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE
            )));
        }

        self.ensure_root_safe()?;
        self.read_sealed_object(
            &self.asset_path(artifact.sha256),
            artifact,
            quest_collision_artifact_limit(),
            "Quest collision artifact bytes",
            AssetVerification::Full,
        )
    }

    /// Read one version-2 Quest collision artifact only through its exact indexed raw identity.
    ///
    /// This is storage-only inspection: the asset index, length, media type, and complete CAS
    /// digest are verified, but the bytes grant no collision, head, build, or publication
    /// authority. Semantic reopening remains the inventory crate's responsibility.
    pub fn read_indexed_quest_collision_artifact_v2(
        &self,
        index: &AssetStoreIndex,
        artifact: &ContentSeal,
    ) -> Result<Vec<u8>, WorkingStoreError> {
        validate_quest_collision_artifact_length(artifact.byte_len)?;
        let meta = index.assets.get(&artifact.sha256).ok_or_else(|| {
            WorkingStoreError::Invariant(format!(
                "Quest collision artifact {} is absent from the supplied asset index",
                artifact.sha256
            ))
        })?;
        if meta.byte_len != artifact.byte_len {
            return Err(WorkingStoreError::Invariant(format!(
                "Quest collision artifact {} index declares {} bytes, raw seal declares {}",
                artifact.sha256, meta.byte_len, artifact.byte_len
            )));
        }
        if meta.media_type != QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2 {
            return Err(WorkingStoreError::Invariant(format!(
                "Quest collision artifact {} media type is {:?}, expected {:?}",
                artifact.sha256, meta.media_type, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2
            )));
        }

        self.ensure_root_safe()?;
        self.read_sealed_object(
            &self.asset_path(artifact.sha256),
            artifact,
            quest_collision_artifact_limit(),
            "Quest collision artifact V2 bytes",
            AssetVerification::Full,
        )
    }

    /// Internal exact-head guard shared by authority-free managed DataAsset staging.
    pub(crate) fn require_exact_head_for_dataasset(
        &self,
        expected_head: Option<&WorkingHead>,
    ) -> Result<(), WorkingStoreError> {
        self.ensure_root_safe()?;
        self.check_expected_head(expected_head)
    }

    /// Complete the deterministic revision-3 Store sizing/encoding preflight used before a
    /// DataAsset transaction installs any new CAS object.
    pub(crate) fn preflight_revision3_dataasset_candidate(
        &self,
        project: &ProjectRevision3,
    ) -> Result<(), WorkingStoreError> {
        self.ensure_root_safe()?;
        validate_revision3_persistability(project, &self.limits)?;
        let expected_head = self.current_head()?;
        let history = match self
            .prepare_revision3_checkpoint_history_v1(expected_head.as_ref(), project)?
        {
            revision3_history::Revision3CheckpointHistoryPlanV1::Root => None,
            revision3_history::Revision3CheckpointHistoryPlanV1::Successor { history } => {
                Some(history)
            }
            revision3_history::Revision3CheckpointHistoryPlanV1::ExactNoOp { .. } => return Ok(()),
        };
        let mut entity_seals = BTreeMap::new();
        for (id, entity) in &project.entities {
            let bytes = canonical_json(entity)?;
            entity_seals.insert(*id, seal_bytes(&bytes));
        }
        let snapshot_bytes =
            encode_revision3_snapshot(project, entity_seals, history, &self.limits)?;
        let head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: seal_bytes(&snapshot_bytes),
        };
        let head_bytes = canonical_json(&head)?;
        enforce_limit("head bytes", head_bytes.len(), self.limits.max_head_bytes)
    }

    /// Hash and parse one historical manifest, returning a conservative full-reopen work charge.
    /// Callers must aggregate and cap these charges before opening any basis with `Full` asset
    /// verification; the returned identity alone is not full basis authority.
    pub(crate) fn inspect_revision3_dataasset_basis(
        &self,
        snapshot: &ContentSeal,
    ) -> Result<Revision3DataAssetBasisPreflight, WorkingStoreError> {
        self.ensure_root_safe()?;
        validate_nonzero_seal(
            snapshot,
            revision3_total_snapshot_limit(&self.limits),
            "DataAsset historical basis snapshot",
        )?;
        let bytes = self.read_sealed_object(
            &self.snapshot_path(snapshot.sha256),
            snapshot,
            revision3_total_snapshot_limit(&self.limits),
            "DataAsset historical basis snapshot",
            AssetVerification::Full,
        )?;
        let manifest: Revision3SnapshotManifest =
            parse_canonical_json(&bytes, "DataAsset historical basis snapshot")?;
        self.validate_revision3_manifest_limits(&manifest)?;
        let entity_count = u64::try_from(manifest.entities.len()).map_err(|_| {
            WorkingStoreError::Invariant(
                "DataAsset historical entity count does not fit u64".to_owned(),
            )
        })?;
        let asset_count = u64::try_from(manifest.asset_store.assets.len()).map_err(|_| {
            WorkingStoreError::Invariant(
                "DataAsset historical asset count does not fit u64".to_owned(),
            )
        })?;
        let entity_bytes = manifest.entities.values().try_fold(0u64, |total, seal| {
            total.checked_add(seal.byte_len).ok_or_else(|| {
                WorkingStoreError::Invariant(
                    "DataAsset historical entity work overflowed".to_owned(),
                )
            })
        })?;
        let asset_bytes = manifest
            .asset_store
            .assets
            .values()
            .try_fold(0u64, |total, meta| {
                total.checked_add(meta.byte_len).ok_or_else(|| {
                    WorkingStoreError::Invariant(
                        "DataAsset historical asset work overflowed".to_owned(),
                    )
                })
            })?;
        let verification_objects = 1u64
            .checked_add(entity_count.checked_mul(2).ok_or_else(|| {
                WorkingStoreError::Invariant(
                    "DataAsset historical object work overflowed".to_owned(),
                )
            })?)
            .and_then(|total| total.checked_add(asset_count.checked_mul(2)?))
            .ok_or_else(|| {
                WorkingStoreError::Invariant(
                    "DataAsset historical object work overflowed".to_owned(),
                )
            })?;
        let nested_basis_bytes = entity_count
            .checked_mul(revision3_total_snapshot_limit(&self.limits) as u64)
            .ok_or_else(|| {
                WorkingStoreError::Invariant(
                    "DataAsset historical nested-basis work overflowed".to_owned(),
                )
            })?;
        let verification_bytes = snapshot
            .byte_len
            .checked_add(entity_bytes)
            .and_then(|total| total.checked_add(asset_bytes.checked_mul(2)?))
            .and_then(|total| total.checked_add(nested_basis_bytes))
            .ok_or_else(|| {
                WorkingStoreError::Invariant("DataAsset historical byte work overflowed".to_owned())
            })?;
        Ok(Revision3DataAssetBasisPreflight {
            head: WorkingHead {
                store_format: WorkingStoreFormat,
                snapshot: snapshot.clone(),
            },
            project_id: manifest.project_id,
            target: manifest.target.clone(),
            revision: manifest.revision,
            manifest,
            verification_objects,
            verification_bytes,
        })
    }

    /// Install already-owned, upstream-verified DataAsset bytes under an exact expected seal.
    ///
    /// The fixed head is checked before and after no-clobber installation. A late race can leave
    /// only the immutable CAS blob as an orphan; it can never publish a partial project.
    pub(crate) fn import_exact_dataasset_bytes_with_write_guard<F>(
        &self,
        bytes: &[u8],
        expected: &ContentSeal,
        max_bytes: u64,
        kind: &'static str,
        basis_head: &WorkingHead,
        before_write: F,
    ) -> Result<bool, WorkingStoreError>
    where
        F: FnOnce() -> Result<(), WorkingStoreError>,
    {
        self.import_exact_dataasset_bytes_with_hooks(
            bytes,
            expected,
            max_bytes,
            kind,
            basis_head,
            before_write,
            || Ok(()),
        )
    }

    #[cfg(test)]
    fn import_exact_dataasset_bytes_with_final_head_hook<F>(
        &self,
        bytes: &[u8],
        expected: &ContentSeal,
        max_bytes: u64,
        kind: &'static str,
        basis_head: &WorkingHead,
        before_final_head_check: F,
    ) -> Result<bool, WorkingStoreError>
    where
        F: FnOnce() -> Result<(), WorkingStoreError>,
    {
        self.import_exact_dataasset_bytes_with_hooks(
            bytes,
            expected,
            max_bytes,
            kind,
            basis_head,
            || Ok(()),
            before_final_head_check,
        )
    }

    // Keeping the two independent write/final race hooks beside the complete seal/head contract
    // makes this security boundary explicit; bundling them into an ambient context would obscure
    // which point each hook protects.
    #[allow(clippy::too_many_arguments)]
    fn import_exact_dataasset_bytes_with_hooks<F, G>(
        &self,
        bytes: &[u8],
        expected: &ContentSeal,
        max_bytes: u64,
        kind: &'static str,
        basis_head: &WorkingHead,
        before_write: F,
        before_final_head_check: G,
    ) -> Result<bool, WorkingStoreError>
    where
        F: FnOnce() -> Result<(), WorkingStoreError>,
        G: FnOnce() -> Result<(), WorkingStoreError>,
    {
        if expected.byte_len == 0 || expected.byte_len > max_bytes {
            return Err(WorkingStoreError::LimitExceeded {
                kind,
                actual: expected.byte_len,
                limit: max_bytes,
            });
        }
        if bytes.len() as u64 != expected.byte_len || seal_bytes(bytes) != *expected {
            return Err(WorkingStoreError::Invariant(format!(
                "{kind} bytes differ from their verified seal"
            )));
        }
        self.ensure_root_safe()?;
        self.check_expected_head(Some(basis_head))?;
        let destination = self.asset_path(expected.sha256);
        // Existing CAS objects are fully verified without consuming the caller's first-write
        // generation guard. Only a missing destination reaches this closure, after all long byte
        // hashing and immediately before creation of the staging file.
        let mut before_write = Some(before_write);
        let mut guarded_write = || {
            before_write
                .take()
                .expect("immutable install invokes its write guard at most once")()?;
            self.ensure_root_safe()?;
            self.check_expected_head(Some(basis_head))
        };
        let deduplicated = self.install_checkpoint_bytes_with_write_guard(
            &destination,
            bytes,
            expected,
            &mut guarded_write,
        )?;
        self.verify_seal_at(&destination, expected, AssetVerification::Full, false)?;
        before_final_head_check()?;
        self.check_expected_head(Some(basis_head))?;
        Ok(deduplicated)
    }

    pub(crate) fn read_exact_dataasset_blob(
        &self,
        seal: &ContentSeal,
        max_bytes: usize,
        kind: &'static str,
    ) -> Result<Vec<u8>, WorkingStoreError> {
        self.ensure_root_safe()?;
        self.read_sealed_object(
            &self.asset_path(seal.sha256),
            seal,
            max_bytes,
            kind,
            AssetVerification::Full,
        )
    }

    /// Stream, bound, hash, validate, and no-clobber install one Ogg asset.
    ///
    /// The source is opened without following a final symlink/reparse point. Deleting or changing
    /// the source after success cannot affect the stored blob.
    /// `expected_head` is a strict CAS token: `None` requires the fixed head to be absent.
    pub fn import_ogg(
        &self,
        source: impl AsRef<Path>,
        logical_name: impl Into<String>,
        expected_head: Option<&WorkingHead>,
    ) -> Result<ImportedOgg, WorkingStoreError> {
        self.import_ogg_classified(source, logical_name, expected_head)
            .map_err(OggImportError::into_store_error)
    }

    /// Import an Ogg while preserving whether a failure belongs to the user
    /// source or to the managed Store/CAS boundary.
    pub fn import_ogg_classified(
        &self,
        source: impl AsRef<Path>,
        logical_name: impl Into<String>,
        expected_head: Option<&WorkingHead>,
    ) -> Result<ImportedOgg, OggImportError> {
        self.ensure_root_safe()?;
        self.check_expected_head(expected_head)?;

        let prepared = self.prepare_ogg_import_classified(source, logical_name)?;
        self.install_prepared_ogg(prepared, expected_head)
            .map_err(OggImportError::from)
    }

    /// Read, bound, hash, and parse one external Ogg without creating Store staging or CAS state.
    ///
    /// Source failures retain their classified context. Store-root and logical-name failures are
    /// Store failures. The returned value is opaque and owns the exact verified bytes so a later
    /// install never has to trust or reopen the external source.
    pub fn prepare_ogg_import_classified(
        &self,
        source: impl AsRef<Path>,
        logical_name: impl Into<String>,
    ) -> Result<PreparedOggImport, OggImportError> {
        self.ensure_root_safe()?;
        let logical_name = logical_name.into();
        self.validate_logical_name(&logical_name)?;

        let source = absolute_path(source.as_ref())
            .map_err(|error| classify_ogg_source_error(error, false))?;
        ensure_safe_existing_chain(&source)
            .map_err(|error| classify_ogg_source_error(error, false))?;
        let source_meta = fs::symlink_metadata(&source)
            .map_err(|error| classify_ogg_source_error(WorkingStoreError::Io(error), true))?;
        ensure_regular_no_link(&source, &source_meta)
            .map_err(|error| classify_ogg_source_error(error, false))?;
        if source_meta.len() > self.limits.max_ogg_bytes as u64 {
            return Err(OggImportError::source(
                OggImportFailureContext::SourceLimit,
                WorkingStoreError::LimitExceeded {
                    kind: "Ogg bytes",
                    actual: source_meta.len(),
                    limit: self.limits.max_ogg_bytes as u64,
                },
            ));
        }

        let mut input = open_regular_read_no_follow(&source)
            .map_err(|error| classify_ogg_source_error(error, true))?;
        let mut bytes = Vec::with_capacity(source_meta.len() as usize);
        let mut hasher = Sha256::new();
        let mut total = 0usize;
        let mut buffer = [0u8; COPY_BUFFER_BYTES];
        loop {
            let count = input.read(&mut buffer).map_err(|error| {
                OggImportError::source(
                    OggImportFailureContext::SourceUnavailable,
                    WorkingStoreError::Io(error),
                )
            })?;
            if count == 0 {
                break;
            }
            total = total.checked_add(count).ok_or_else(|| {
                OggImportError::source(
                    OggImportFailureContext::SourceLimit,
                    WorkingStoreError::LimitExceeded {
                        kind: "Ogg bytes",
                        actual: u64::MAX,
                        limit: self.limits.max_ogg_bytes as u64,
                    },
                )
            })?;
            if total > self.limits.max_ogg_bytes {
                return Err(OggImportError::source(
                    OggImportFailureContext::SourceLimit,
                    WorkingStoreError::LimitExceeded {
                        kind: "Ogg bytes",
                        actual: total as u64,
                        limit: self.limits.max_ogg_bytes as u64,
                    },
                ));
            }
            hasher.update(&buffer[..count]);
            bytes.extend_from_slice(&buffer[..count]);
        }
        if total as u64 != source_meta.len() {
            return Err(OggImportError::source(
                OggImportFailureContext::SourceChanged,
                WorkingStoreError::Invariant(format!(
                    "Ogg source length changed while reading: expected {}, read {total}",
                    source_meta.len()
                )),
            ));
        }

        let ogg = self
            .derive_ogg_metadata(&bytes)
            .map_err(|error| match error {
                WorkingStoreError::InvalidOgg(_) => {
                    OggImportError::source(OggImportFailureContext::SourceInvalid, error)
                }
                _ => error.into(),
            })?;
        let digest = digest_from_hasher(hasher);
        Ok(PreparedOggImport {
            bytes,
            asset: AssetRef {
                sha256: digest,
                byte_len: total as u64,
                logical_name,
            },
            ogg,
        })
    }

    /// Validate already-owned source bytes without reopening an ambient path.
    ///
    /// This is the capability-oriented peer of [`Self::prepare_ogg_import_classified`]. Native
    /// folder workflows first obtain bytes through a retained no-follow directory/file handle and
    /// then hand ownership to this method. No Store staging or CAS object is created.
    pub fn prepare_ogg_bytes_classified(
        &self,
        bytes: Vec<u8>,
        logical_name: impl Into<String>,
    ) -> Result<PreparedOggImport, OggImportError> {
        self.ensure_root_safe()?;
        let logical_name = logical_name.into();
        self.validate_logical_name(&logical_name)?;
        enforce_limit("Ogg bytes", bytes.len(), self.limits.max_ogg_bytes)
            .map_err(|error| OggImportError::source(OggImportFailureContext::SourceLimit, error))?;
        let ogg = self
            .derive_ogg_metadata(&bytes)
            .map_err(|error| match error {
                WorkingStoreError::InvalidOgg(_) => {
                    OggImportError::source(OggImportFailureContext::SourceInvalid, error)
                }
                _ => error.into(),
            })?;
        let seal = seal_bytes(&bytes);
        Ok(PreparedOggImport {
            bytes,
            asset: AssetRef {
                sha256: seal.sha256,
                byte_len: seal.byte_len,
                logical_name,
            },
            ogg,
        })
    }

    /// Consume one verified source preparation and install its exact bytes under fixed-head CAS.
    ///
    /// No external source is reopened. Installation and cleanup failures are Store failures. A
    /// concurrent head advance may leave only a fully verified immutable orphan and returns a
    /// head conflict rather than an accepted receipt.
    pub fn install_prepared_ogg(
        &self,
        prepared: PreparedOggImport,
        expected_head: Option<&WorkingHead>,
    ) -> Result<ImportedOgg, WorkingStoreError> {
        self.ensure_root_safe()?;
        self.check_expected_head(expected_head)?;
        self.validate_logical_name(&prepared.asset.logical_name)?;
        enforce_limit("Ogg bytes", prepared.bytes.len(), self.limits.max_ogg_bytes)?;

        let seal = ContentSeal {
            byte_len: prepared.asset.byte_len,
            sha256: prepared.asset.sha256,
        };
        if seal_bytes(&prepared.bytes) != seal
            || self.derive_ogg_metadata(&prepared.bytes)? != prepared.ogg
        {
            return Err(WorkingStoreError::Invariant(
                "prepared Ogg bytes differ from their verified receipt".to_owned(),
            ));
        }

        let destination = self.asset_path(prepared.asset.sha256);
        self.check_expected_head(expected_head)?;
        let deduplicated = self.install_immutable_bytes(&destination, &prepared.bytes, &seal)?;
        self.verify_seal_at(&destination, &seal, AssetVerification::Full, false)?;
        self.check_expected_head(expected_head)?;
        Ok(ImportedOgg {
            asset: prepared.asset,
            ogg: prepared.ogg,
            deduplicated,
        })
    }

    /// Verify one logical asset reference against its content-addressed object.
    pub fn verify_asset(
        &self,
        asset: &AssetRef,
        verification: AssetVerification,
    ) -> Result<(), WorkingStoreError> {
        self.ensure_root_safe()?;
        self.validate_logical_name(&asset.logical_name)?;
        if asset.byte_len > self.limits.max_ogg_bytes as u64 {
            return Err(WorkingStoreError::LimitExceeded {
                kind: "Ogg bytes",
                actual: asset.byte_len,
                limit: self.limits.max_ogg_bytes as u64,
            });
        }
        let seal = ContentSeal {
            byte_len: asset.byte_len,
            sha256: asset.sha256,
        };
        self.verify_seal_at(&self.asset_path(asset.sha256), &seal, verification, false)
    }

    /// Read one content-addressed Ogg asset only after a complete bounded seal verification.
    ///
    /// This is the byte-owning counterpart to [`Self::verify_asset`]. It deliberately does not
    /// expose the Store's private object path: build integrations receive an immutable byte view
    /// whose length and SHA-256 were checked through the same no-follow Store boundary. The
    /// logical name and configured Ogg limit are enforced before allocating the result.
    pub fn read_verified_ogg_asset(&self, asset: &AssetRef) -> Result<Vec<u8>, WorkingStoreError> {
        self.ensure_root_safe()?;
        self.validate_logical_name(&asset.logical_name)?;
        if asset.byte_len > self.limits.max_ogg_bytes as u64 {
            return Err(WorkingStoreError::LimitExceeded {
                kind: "Ogg bytes",
                actual: asset.byte_len,
                limit: self.limits.max_ogg_bytes as u64,
            });
        }
        let seal = ContentSeal {
            byte_len: asset.byte_len,
            sha256: asset.sha256,
        };
        self.read_sealed_object(
            &self.asset_path(asset.sha256),
            &seal,
            self.limits.max_ogg_bytes,
            "Ogg asset",
            AssetVerification::Full,
        )
    }

    fn head_path(&self) -> PathBuf {
        self.root.join(HEAD_FILE_NAME)
    }

    fn snapshot_path(&self, digest: Sha256Digest) -> PathBuf {
        let hex = digest.to_string();
        self.root
            .join("snapshots")
            .join("sha256")
            .join(&hex[..2])
            .join(format!("{}.json", &hex[2..]))
    }

    fn entity_path(&self, id: EntityId, digest: Sha256Digest) -> PathBuf {
        let id = id.to_string();
        self.root
            .join("entities")
            .join(&id[..2])
            .join(&id[2..])
            .join(format!("{digest}.json"))
    }

    fn asset_path(&self, digest: Sha256Digest) -> PathBuf {
        let hex = digest.to_string();
        self.root
            .join("assets")
            .join("sha256")
            .join(&hex[..2])
            .join(&hex[2..])
    }

    fn ensure_root_safe(&self) -> Result<(), WorkingStoreError> {
        ensure_safe_directory_chain(&self.root)
    }

    fn check_expected_head(&self, expected: Option<&WorkingHead>) -> Result<(), WorkingStoreError> {
        let actual = self.current_head()?;
        if actual.as_ref() != expected {
            return Err(WorkingStoreError::HeadConflict {
                expected: expected.cloned(),
                actual,
            });
        }
        Ok(())
    }

    fn validate_project_limits(&self, project: &ProjectV2) -> Result<(), WorkingStoreError> {
        enforce_limit(
            "entity count",
            project.entities.len(),
            self.limits.max_entities,
        )?;
        self.validate_asset_index_limits(&project.asset_store)?;
        for entity in project.entities.values() {
            if let EntityPayload::VoiceTake(take) = &entity.payload {
                self.validate_logical_name(&take.asset.logical_name)?;
            }
        }
        Ok(())
    }

    fn validate_manifest_limits(
        &self,
        snapshot: &SnapshotManifest,
    ) -> Result<(), WorkingStoreError> {
        enforce_limit(
            "entity count",
            snapshot.entities.len(),
            self.limits.max_entities,
        )?;
        self.validate_asset_index_limits(&snapshot.asset_store)
    }

    fn validate_revision2_manifest_limits(
        &self,
        snapshot: &SnapshotManifestRevision2,
    ) -> Result<(), WorkingStoreError> {
        enforce_limit(
            "entity count",
            snapshot.entities.len(),
            self.limits.max_entities,
        )?;
        self.validate_asset_index_limits(&snapshot.asset_store)
    }

    fn validate_revision3_manifest_limits(
        &self,
        snapshot: &Revision3SnapshotManifest,
    ) -> Result<(), WorkingStoreError> {
        let mut base_snapshot = snapshot.clone();
        base_snapshot.history = None;
        let base_snapshot_bytes = canonical_json(&base_snapshot)?;
        enforce_limit(
            "revision-3 base snapshot bytes",
            base_snapshot_bytes.len(),
            self.limits.max_snapshot_bytes,
        )?;
        revision3_history::validate_revision3_checkpoint_history_v1(snapshot, &self.limits)?;
        enforce_limit(
            "entity count",
            snapshot.entities.len(),
            self.limits.max_entities,
        )?;
        validate_revision3_asset_index_persistability(&snapshot.asset_store, &self.limits)?;
        let mut total_entity_bytes = 0u64;
        for seal in snapshot.entities.values() {
            validate_nonzero_seal(seal, self.limits.max_entity_bytes, "revision-3 entity")?;
            total_entity_bytes = checked_bounded_sum(
                "aggregate referenced entity bytes",
                total_entity_bytes,
                seal.byte_len,
                self.limits.max_referenced_entity_bytes,
            )?;
        }
        Ok(())
    }

    fn validate_asset_index_limits(
        &self,
        index: &AssetStoreIndex,
    ) -> Result<(), WorkingStoreError> {
        validate_asset_index_persistability(index, &self.limits)
    }

    fn validate_logical_name(&self, name: &str) -> Result<(), WorkingStoreError> {
        validate_logical_name_persistability(name, &self.limits)
    }

    fn verify_asset_index(
        &self,
        index: &AssetStoreIndex,
        verification: AssetVerification,
    ) -> Result<(), WorkingStoreError> {
        self.validate_asset_index_limits(index)?;
        for (digest, meta) in &index.assets {
            let seal = ContentSeal {
                byte_len: meta.byte_len,
                sha256: *digest,
            };
            self.verify_seal_at(&self.asset_path(*digest), &seal, verification, false)?;
        }
        Ok(())
    }

    fn verify_voice_take_ogg_metadata(
        &self,
        project: &ProjectV2,
        verification: AssetVerification,
    ) -> Result<(), WorkingStoreError> {
        if verification != AssetVerification::Full {
            return Ok(());
        }

        let mut validated = BTreeMap::<Sha256Digest, OggMetadata>::new();
        for (entity_id, entity) in &project.entities {
            let EntityPayload::VoiceTake(take) = &entity.payload else {
                continue;
            };
            let Some(indexed) = project.asset_store.assets.get(&take.asset.sha256) else {
                continue;
            };
            if indexed.media_type != "audio/ogg" {
                continue;
            }

            let actual = if let Some(actual) = validated.get(&take.asset.sha256) {
                actual.clone()
            } else {
                let seal = ContentSeal {
                    byte_len: indexed.byte_len,
                    sha256: take.asset.sha256,
                };
                let bytes = self.read_sealed_object(
                    &self.asset_path(take.asset.sha256),
                    &seal,
                    self.limits.max_ogg_bytes,
                    "Ogg bytes",
                    AssetVerification::Full,
                )?;
                let actual = self.derive_ogg_metadata(&bytes)?;
                validated.insert(take.asset.sha256, actual.clone());
                actual
            };

            if take.ogg != actual {
                return Err(WorkingStoreError::OggMetadataMismatch {
                    entity: *entity_id,
                    asset: take.asset.sha256,
                    declared: take.ogg.clone(),
                    actual,
                });
            }
        }
        Ok(())
    }

    fn verify_revision2_voice_take_ogg_metadata(
        &self,
        project: &ProjectRevision2,
        verification: AssetVerification,
    ) -> Result<(), WorkingStoreError> {
        if verification != AssetVerification::Full {
            return Ok(());
        }

        let mut validated = BTreeMap::<Sha256Digest, OggMetadata>::new();
        for (entity_id, entity) in &project.entities {
            let Revision2EntityPayload::VoiceTake(take) = &entity.payload else {
                continue;
            };
            let indexed = project
                .asset_store
                .assets
                .get(&take.asset.sha256)
                .ok_or_else(|| {
                    WorkingStoreError::Invariant(format!(
                    "revision-2 voice take {entity_id} references asset {} absent from asset_store",
                    take.asset.sha256
                ))
                })?;
            if indexed.byte_len != take.asset.byte_len {
                return Err(WorkingStoreError::Invariant(format!(
                    "revision-2 voice take {entity_id} asset {} declares {} bytes but asset_store declares {}",
                    take.asset.sha256, take.asset.byte_len, indexed.byte_len
                )));
            }
            if indexed.media_type != "audio/ogg" {
                return Err(WorkingStoreError::Invariant(format!(
                    "revision-2 voice take {entity_id} asset {} has media type {:?}, expected \"audio/ogg\"",
                    take.asset.sha256, indexed.media_type
                )));
            }

            let actual = if let Some(actual) = validated.get(&take.asset.sha256) {
                actual.clone()
            } else {
                let seal = ContentSeal {
                    byte_len: indexed.byte_len,
                    sha256: take.asset.sha256,
                };
                let bytes = self.read_sealed_object(
                    &self.asset_path(take.asset.sha256),
                    &seal,
                    self.limits.max_ogg_bytes,
                    "revision-2 Ogg bytes",
                    AssetVerification::Full,
                )?;
                let actual = self.derive_ogg_metadata(&bytes)?;
                validated.insert(take.asset.sha256, actual.clone());
                actual
            };

            let declared = revision2_ogg_metadata_as_revision1(&take.ogg);
            if declared != actual {
                return Err(WorkingStoreError::OggMetadataMismatch {
                    entity: *entity_id,
                    asset: take.asset.sha256,
                    declared,
                    actual,
                });
            }
        }
        Ok(())
    }

    fn verify_revision3_voice_take_ogg_metadata(
        &self,
        project: &ProjectRevision3,
        verification: AssetVerification,
    ) -> Result<(), WorkingStoreError> {
        if verification != AssetVerification::Full {
            return Ok(());
        }

        let mut validated = BTreeMap::<Sha256Digest, OggMetadata>::new();
        for (entity_id, entity) in &project.entities {
            let Revision3EntityPayload::VoiceTake(take) = &entity.payload else {
                continue;
            };
            let indexed = project
                .asset_store
                .assets
                .get(&take.asset.sha256)
                .ok_or_else(|| {
                    WorkingStoreError::Invariant(format!(
                        "revision-3 voice take {entity_id} references asset {} absent from asset_store",
                        take.asset.sha256
                    ))
                })?;
            if indexed.byte_len != take.asset.byte_len {
                return Err(WorkingStoreError::Invariant(format!(
                    "revision-3 voice take {entity_id} asset {} declares {} bytes but asset_store declares {}",
                    take.asset.sha256, take.asset.byte_len, indexed.byte_len
                )));
            }
            if indexed.media_type != "audio/ogg" {
                return Err(WorkingStoreError::Invariant(format!(
                    "revision-3 voice take {entity_id} asset {} has media type {:?}, expected \"audio/ogg\"",
                    take.asset.sha256, indexed.media_type
                )));
            }

            let actual = if let Some(actual) = validated.get(&take.asset.sha256) {
                actual.clone()
            } else {
                let seal = ContentSeal {
                    byte_len: indexed.byte_len,
                    sha256: take.asset.sha256,
                };
                let bytes = self.read_sealed_object(
                    &self.asset_path(take.asset.sha256),
                    &seal,
                    self.limits.max_ogg_bytes,
                    "revision-3 Ogg bytes",
                    AssetVerification::Full,
                )?;
                let actual = self.derive_ogg_metadata(&bytes)?;
                validated.insert(take.asset.sha256, actual.clone());
                actual
            };

            let declared = revision2_ogg_metadata_as_revision1(&take.ogg);
            if declared != actual {
                return Err(WorkingStoreError::OggMetadataMismatch {
                    entity: *entity_id,
                    asset: take.asset.sha256,
                    declared,
                    actual,
                });
            }
        }
        Ok(())
    }

    fn verify_revision3_basis_snapshots(
        &self,
        project: &ProjectRevision3,
    ) -> Result<(), WorkingStoreError> {
        let mut snapshots = BTreeMap::<Sha256Digest, u64>::new();
        for entity in project.entities.values() {
            let Revision3EntityPayload::QuestDraft(quest) = &entity.payload else {
                continue;
            };
            let basis = &quest.input.collision_catalog.basis_snapshot;
            if let Some(existing) = snapshots.insert(basis.sha256, basis.byte_len) {
                if existing != basis.byte_len {
                    return Err(WorkingStoreError::Invariant(format!(
                        "revision-3 basis snapshot {} has conflicting lengths {existing} and {}",
                        basis.sha256, basis.byte_len
                    )));
                }
            }
        }
        for (sha256, byte_len) in snapshots {
            let seal = ContentSeal { byte_len, sha256 };
            let bytes = self.read_sealed_object(
                &self.snapshot_path(sha256),
                &seal,
                revision3_total_snapshot_limit(&self.limits),
                "revision-3 basis snapshot",
                AssetVerification::Full,
            )?;
            let manifest: Revision3SnapshotManifest =
                parse_canonical_json(&bytes, "revision-3 basis snapshot")?;
            self.validate_revision3_manifest_limits(&manifest)?;
        }
        Ok(())
    }

    fn derive_ogg_metadata(&self, bytes: &[u8]) -> Result<OggMetadata, WorkingStoreError> {
        let info = gore_vo::validate_ogg(bytes, &self.ogg_limits())
            .map_err(|error| WorkingStoreError::InvalidOgg(error.to_string()))?;
        let (codec, channels, sample_rate) = match info.codec {
            gore_vo::OggCodec::Vorbis {
                channels,
                sample_rate,
            } => (OggCodec::Vorbis, channels, sample_rate),
            gore_vo::OggCodec::Opus { channels, .. } => {
                // Opus always decodes at 48 kHz. OpusHead's input rate is informational and may
                // legitimately be zero, while the authoring model stores the decode rate.
                (OggCodec::Opus, channels, 48_000)
            }
            gore_vo::OggCodec::Unknown => {
                return Err(WorkingStoreError::InvalidOgg(
                    "Ogg codec is not Vorbis or Opus".to_owned(),
                ));
            }
        };
        Ok(OggMetadata {
            codec,
            channels,
            sample_rate,
            pages: u32::try_from(info.pages).map_err(|_| {
                WorkingStoreError::Invariant("Ogg page count does not fit u32".to_owned())
            })?,
            logical_streams: u32::try_from(info.logical_streams).map_err(|_| {
                WorkingStoreError::Invariant("Ogg stream count does not fit u32".to_owned())
            })?,
        })
    }

    fn install_immutable_bytes(
        &self,
        destination: &Path,
        bytes: &[u8],
        seal: &ContentSeal,
    ) -> Result<bool, WorkingStoreError> {
        let (temp_path, mut temp) = self.create_temp_file()?;
        let result = (|| {
            temp.write_all(bytes)?;
            temp.flush()?;
            temp.sync_all()?;
            // Never expose an immutable object while a writable staging handle aliases it.
            drop(temp);
            self.install_staged_file(&temp_path, destination, seal)
        })();
        let cleanup = cleanup_staged_file(&temp_path);
        match (result, cleanup) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(_), Err(source)) => Err(WorkingStoreError::StagingCleanup {
                path: temp_path,
                source,
            }),
            (Err(error), _) => Err(error),
        }
    }

    fn install_checkpoint_bytes_with_write_guard<F>(
        &self,
        destination: &Path,
        bytes: &[u8],
        seal: &ContentSeal,
        before_write: &mut F,
    ) -> Result<bool, WorkingStoreError>
    where
        F: FnMut() -> Result<(), WorkingStoreError>,
    {
        match fs::symlink_metadata(destination) {
            Ok(_) => {
                self.verify_existing_collision(destination, seal)?;
                Ok(true)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                before_write()?;
                self.install_immutable_bytes(destination, bytes, seal)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn install_staged_file(
        &self,
        staged: &Path,
        destination: &Path,
        seal: &ContentSeal,
    ) -> Result<bool, WorkingStoreError> {
        let parent = destination
            .parent()
            .ok_or_else(|| WorkingStoreError::UnsafePath {
                path: destination.to_path_buf(),
                reason: "object path has no parent".to_owned(),
            })?;
        create_directory_chain(parent)?;
        ensure_safe_directory_chain(parent)?;

        match fs::symlink_metadata(destination) {
            Ok(_) => {
                self.verify_existing_collision(destination, seal)?;
                cleanup_staged_file(staged).map_err(|source| {
                    WorkingStoreError::StagingCleanup {
                        path: staged.to_path_buf(),
                        source,
                    }
                })?;
                return Ok(true);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        match install_no_clobber(staged, destination) {
            Ok(()) => {
                cleanup_staged_file(staged).map_err(|source| {
                    WorkingStoreError::StagingCleanup {
                        path: staged.to_path_buf(),
                        source,
                    }
                })?;
                self.verify_seal_at(destination, seal, AssetVerification::Full, false)?;
                Ok(false)
            }
            Err(error) => {
                require_preinstall_collision(error)?;
                self.verify_existing_collision(destination, seal)?;
                cleanup_staged_file(staged).map_err(|source| {
                    WorkingStoreError::StagingCleanup {
                        path: staged.to_path_buf(),
                        source,
                    }
                })?;
                Ok(true)
            }
        }
    }

    fn verify_existing_collision(
        &self,
        path: &Path,
        seal: &ContentSeal,
    ) -> Result<(), WorkingStoreError> {
        self.verify_seal_at(path, seal, AssetVerification::Full, true)
            .map_err(|error| match error {
                WorkingStoreError::SealMismatch { reason, .. }
                | WorkingStoreError::UnsafePath { reason, .. } => WorkingStoreError::Collision {
                    path: path.to_path_buf(),
                    reason,
                },
                other => other,
            })
    }

    fn read_sealed_object(
        &self,
        path: &Path,
        seal: &ContentSeal,
        max_bytes: usize,
        kind: &'static str,
        verification: AssetVerification,
    ) -> Result<Vec<u8>, WorkingStoreError> {
        if seal.byte_len > max_bytes as u64 {
            return Err(WorkingStoreError::LimitExceeded {
                kind,
                actual: seal.byte_len,
                limit: max_bytes as u64,
            });
        }
        let bytes = read_required_regular_bounded(path, max_bytes, kind)?;
        if bytes.len() as u64 != seal.byte_len {
            return Err(WorkingStoreError::SealMismatch {
                path: path.to_path_buf(),
                reason: format!("expected {} bytes, found {}", seal.byte_len, bytes.len()),
            });
        }
        if verification == AssetVerification::Full && seal_bytes(&bytes).sha256 != seal.sha256 {
            return Err(WorkingStoreError::SealMismatch {
                path: path.to_path_buf(),
                reason: format!("content does not match expected SHA-256 {}", seal.sha256),
            });
        }
        Ok(bytes)
    }

    fn verify_seal_at(
        &self,
        path: &Path,
        seal: &ContentSeal,
        verification: AssetVerification,
        collision: bool,
    ) -> Result<(), WorkingStoreError> {
        ensure_safe_existing_chain(path)?;
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(WorkingStoreError::MissingObject(path.to_path_buf()));
            }
            Err(error) => return Err(error.into()),
        };
        ensure_regular_no_link(path, &metadata)?;
        ensure_single_link(path, &metadata)?;
        if metadata.len() != seal.byte_len {
            let reason = format!("expected {} bytes, found {}", seal.byte_len, metadata.len());
            return if collision {
                Err(WorkingStoreError::Collision {
                    path: path.to_path_buf(),
                    reason,
                })
            } else {
                Err(WorkingStoreError::SealMismatch {
                    path: path.to_path_buf(),
                    reason,
                })
            };
        }
        if verification == AssetVerification::Full {
            let actual = hash_file(path, seal.byte_len)?;
            if actual != seal.sha256 {
                let reason = format!("expected SHA-256 {}, found {actual}", seal.sha256);
                return if collision {
                    Err(WorkingStoreError::Collision {
                        path: path.to_path_buf(),
                        reason,
                    })
                } else {
                    Err(WorkingStoreError::SealMismatch {
                        path: path.to_path_buf(),
                        reason,
                    })
                };
            }
        }
        Ok(())
    }

    fn create_temp_file(&self) -> Result<(PathBuf, File), WorkingStoreError> {
        let directory = self.root.join(".gore").join("staging");
        create_directory_chain(&directory)?;
        ensure_safe_directory_chain(&directory)?;
        for _ in 0..128 {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = directory.join(format!("object-{}-{sequence:016x}.tmp", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(WorkingStoreError::Invariant(
            "could not allocate a unique staging file".to_owned(),
        ))
    }

    fn ogg_limits(&self) -> gore_vo::Limits {
        gore_vo::Limits {
            max_ogg_bytes: self.limits.max_ogg_bytes,
            ..gore_vo::Limits::default()
        }
    }
}

fn parse_snapshot_revision(bytes: &[u8]) -> Result<u32, WorkingStoreError> {
    #[derive(Debug)]
    struct SnapshotRevisionProbe(u32);

    impl<'de> Deserialize<'de> for SnapshotRevisionProbe {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct ProbeVisitor;

            impl<'de> Visitor<'de> for ProbeVisitor {
                type Value = SnapshotRevisionProbe;

                fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    formatter.write_str("a working-store snapshot object")
                }

                fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let mut seen = BTreeSet::new();
                    let mut schema_revision = None;
                    while let Some(key) = access.next_key::<String>()? {
                        if !seen.insert(key.clone()) {
                            return Err(de::Error::custom(format!(
                                "duplicate snapshot field {key:?}"
                            )));
                        }
                        if key == "schema_revision" {
                            schema_revision = Some(access.next_value::<u32>()?);
                        } else {
                            access.next_value::<de::IgnoredAny>()?;
                        }
                    }
                    Ok(SnapshotRevisionProbe(schema_revision.ok_or_else(|| {
                        de::Error::missing_field("schema_revision")
                    })?))
                }
            }

            deserializer.deserialize_map(ProbeVisitor)
        }
    }

    serde_json::from_slice::<SnapshotRevisionProbe>(bytes)
        .map(|probe| probe.0)
        .map_err(|source| WorkingStoreError::InvalidJson {
            kind: "snapshot revision probe",
            source,
        })
}

fn revision2_checkpoint_diagnostics(
    project: &ProjectRevision2,
    profile: ValidationProfile,
) -> Vec<Diagnostic> {
    let mut diagnostics = project.validate_story_entities_with_profile(profile);
    diagnostics.push(Diagnostic::project_error(
        DiagnosticCode::Revision2CombinedValidationUnavailable,
        "schema_revision",
        "schema revision 2 is not build-ready until combined story, voice, localization, and asset validation is implemented",
    ));
    diagnostics.sort_by(|left, right| {
        (
            left.severity,
            left.entity,
            left.property_path.as_deref(),
            left.code,
            left.message.as_str(),
            left.related_entities.as_slice(),
        )
            .cmp(&(
                right.severity,
                right.entity,
                right.property_path.as_deref(),
                right.code,
                right.message.as_str(),
                right.related_entities.as_slice(),
            ))
    });
    diagnostics
}

fn revision2_ogg_metadata_as_revision1(value: &Revision2OggMetadata) -> OggMetadata {
    OggMetadata {
        codec: match value.codec {
            Revision2OggCodec::Vorbis => OggCodec::Vorbis,
            Revision2OggCodec::Opus => OggCodec::Opus,
        },
        channels: value.channels,
        sample_rate: value.sample_rate,
        pages: value.pages,
        logical_streams: value.logical_streams,
    }
}

fn deserialize_unique_set<'de, D, T>(deserializer: D) -> Result<BTreeSet<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Ord + Clone + std::fmt::Display,
{
    let values = Vec::<T>::deserialize(deserializer)?;
    let mut set = BTreeSet::new();
    for value in values {
        if !set.insert(value.clone()) {
            return Err(de::Error::custom(format!("duplicate set value {value}")));
        }
    }
    Ok(set)
}

fn deserialize_unique_map<'de, D, K, V>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Ord + Clone + std::fmt::Display,
    V: Deserialize<'de>,
{
    struct UniqueMapVisitor<K, V>(std::marker::PhantomData<(K, V)>);

    impl<'de, K, V> Visitor<'de> for UniqueMapVisitor<K, V>
    where
        K: Deserialize<'de> + Ord + Clone + std::fmt::Display,
        V: Deserialize<'de>,
    {
        type Value = BTreeMap<K, V>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a map with unique keys")
        }

        fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut map = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<K, V>()? {
                if map.insert(key.clone(), value).is_some() {
                    return Err(de::Error::custom(format!("duplicate map key {key}")));
                }
            }
            Ok(map)
        }
    }

    deserializer.deserialize_map(UniqueMapVisitor(std::marker::PhantomData))
}

/// Cheap, filesystem-free proof that one programmatically constructed revision-2 document fits
/// every byte/count boundary required by the working store. Both mutation and store publication
/// use this exact helper, so a successful in-memory transaction cannot defer a store-format limit
/// failure until checkpoint preparation. Eager wire/API limits remain caller policy and are not
/// imposed on the general store's larger lazy-document contract here.
pub(crate) fn validate_revision2_persistability(
    project: &ProjectRevision2,
    limits: &WorkingStoreLimits,
) -> Result<(), WorkingStoreError> {
    let limits = (*limits).validate()?;
    enforce_limit("entity count", project.entities.len(), limits.max_entities)?;
    validate_asset_index_persistability(&project.asset_store, &limits)?;

    for entity in project.entities.values() {
        if let Revision2EntityPayload::VoiceTake(take) = &entity.payload {
            validate_logical_name_persistability(&take.asset.logical_name, &limits)?;
            if take.asset.byte_len > limits.max_ogg_bytes as u64 {
                return Err(WorkingStoreError::LimitExceeded {
                    kind: "Ogg bytes",
                    actual: take.asset.byte_len,
                    limit: limits.max_ogg_bytes as u64,
                });
            }
        }
    }

    let mut total_entity_bytes = 0u64;
    for (id, entity) in &project.entities {
        if id != &entity.id {
            return Err(WorkingStoreError::Invariant(format!(
                "revision-2 entity map key {id} does not match embedded id {}",
                entity.id
            )));
        }
        let bytes = canonical_json(entity)?;
        enforce_limit("entity bytes", bytes.len(), limits.max_entity_bytes)?;
        total_entity_bytes = checked_bounded_sum(
            "aggregate referenced entity bytes",
            total_entity_bytes,
            bytes.len() as u64,
            limits.max_referenced_entity_bytes,
        )?;
    }
    Ok(())
}

fn validate_revision3_persistability(
    project: &ProjectRevision3,
    limits: &WorkingStoreLimits,
) -> Result<(), WorkingStoreError> {
    let limits = (*limits).validate()?;
    project.validate_closed_model().map_err(|error| {
        WorkingStoreError::Invariant(format!("invalid schema-revision-3 project: {error}"))
    })?;
    enforce_limit("entity count", project.entities.len(), limits.max_entities)?;
    validate_revision3_asset_index_persistability(&project.asset_store, &limits)?;
    for entity in project.entities.values() {
        if let Revision3EntityPayload::VoiceTake(take) = &entity.payload {
            validate_logical_name_persistability(&take.asset.logical_name, &limits)?;
            if take.asset.byte_len > limits.max_ogg_bytes as u64 {
                return Err(WorkingStoreError::LimitExceeded {
                    kind: "Ogg bytes",
                    actual: take.asset.byte_len,
                    limit: limits.max_ogg_bytes as u64,
                });
            }
        }
    }

    let mut total_entity_bytes = 0u64;
    for (id, entity) in &project.entities {
        if id != &entity.id {
            return Err(WorkingStoreError::Invariant(format!(
                "revision-3 entity map key {id} does not match embedded id {}",
                entity.id
            )));
        }
        let bytes = canonical_json(entity)?;
        enforce_limit("entity bytes", bytes.len(), limits.max_entity_bytes)?;
        total_entity_bytes = checked_bounded_sum(
            "aggregate referenced entity bytes",
            total_entity_bytes,
            bytes.len() as u64,
            limits.max_referenced_entity_bytes,
        )?;
    }
    Ok(())
}

fn validate_revision3_asset_index_persistability(
    index: &AssetStoreIndex,
    limits: &WorkingStoreLimits,
) -> Result<(), WorkingStoreError> {
    validate_asset_index_persistability(index, limits)?;
    for meta in index.assets.values() {
        if is_quest_collision_artifact_media_type(&meta.media_type) {
            validate_quest_collision_artifact_length(meta.byte_len)?;
        }
    }
    Ok(())
}

fn quest_collision_artifact_limit() -> usize {
    usize::try_from(MAX_QUEST_COLLISION_ARTIFACT_BYTES)
        .expect("the fixed Quest collision artifact limit fits usize")
}

fn validate_quest_collision_artifact_length(byte_len: u64) -> Result<(), WorkingStoreError> {
    if byte_len == 0 {
        return Err(WorkingStoreError::Invariant(
            "Quest collision artifact byte length must be non-zero".to_owned(),
        ));
    }
    if byte_len > MAX_QUEST_COLLISION_ARTIFACT_BYTES {
        return Err(WorkingStoreError::LimitExceeded {
            kind: "Quest collision artifact bytes",
            actual: byte_len,
            limit: MAX_QUEST_COLLISION_ARTIFACT_BYTES,
        });
    }
    Ok(())
}

fn validate_asset_index_persistability(
    index: &AssetStoreIndex,
    limits: &WorkingStoreLimits,
) -> Result<(), WorkingStoreError> {
    enforce_limit("asset count", index.assets.len(), limits.max_assets)?;
    let mut total = 0u64;
    for meta in index.assets.values() {
        total = checked_bounded_sum(
            "aggregate referenced asset bytes",
            total,
            meta.byte_len,
            limits.max_referenced_asset_bytes,
        )?;
        if meta.media_type == "audio/ogg" && meta.byte_len > limits.max_ogg_bytes as u64 {
            return Err(WorkingStoreError::LimitExceeded {
                kind: "Ogg bytes",
                actual: meta.byte_len,
                limit: limits.max_ogg_bytes as u64,
            });
        }
    }
    Ok(())
}

fn validate_logical_name_persistability(
    name: &str,
    limits: &WorkingStoreLimits,
) -> Result<(), WorkingStoreError> {
    if name.is_empty() {
        return Err(WorkingStoreError::Invariant(
            "asset logical_name must not be empty".to_owned(),
        ));
    }
    enforce_limit(
        "logical_name UTF-8 bytes",
        name.len(),
        limits.max_logical_name_bytes,
    )?;
    if name.chars().any(char::is_control) {
        return Err(WorkingStoreError::Invariant(
            "asset logical_name must not contain control characters".to_owned(),
        ));
    }
    Ok(())
}

fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, WorkingStoreError> {
    serde_json::to_vec(value).map_err(|source| WorkingStoreError::InvalidJson {
        kind: "canonical",
        source,
    })
}

fn parse_canonical_json<T>(bytes: &[u8], kind: &'static str) -> Result<T, WorkingStoreError>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let value = serde_json::from_slice(bytes)
        .map_err(|source| WorkingStoreError::InvalidJson { kind, source })?;
    let canonical = canonical_json(&value)?;
    if canonical != bytes {
        return Err(WorkingStoreError::NonCanonicalJson { kind });
    }
    Ok(value)
}

fn seal_bytes(bytes: &[u8]) -> ContentSeal {
    let digest = Sha256::digest(bytes);
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(digest.into()),
    }
}

fn digest_from_hasher(hasher: Sha256) -> Sha256Digest {
    Sha256Digest::from_bytes(hasher.finalize().into())
}

fn validate_nonzero_seal(
    seal: &ContentSeal,
    max: usize,
    kind: &'static str,
) -> Result<(), WorkingStoreError> {
    if seal.byte_len == 0 {
        return Err(WorkingStoreError::Invariant(format!(
            "{kind} seal byte_len must be non-zero"
        )));
    }
    if seal.byte_len > max as u64 {
        return Err(WorkingStoreError::LimitExceeded {
            kind,
            actual: seal.byte_len,
            limit: max as u64,
        });
    }
    Ok(())
}

fn enforce_limit(kind: &'static str, actual: usize, limit: usize) -> Result<(), WorkingStoreError> {
    if actual > limit {
        Err(WorkingStoreError::LimitExceeded {
            kind,
            actual: actual as u64,
            limit: limit as u64,
        })
    } else {
        Ok(())
    }
}

fn checked_bounded_sum(
    kind: &'static str,
    current: u64,
    addition: u64,
    limit: u64,
) -> Result<u64, WorkingStoreError> {
    let total = current
        .checked_add(addition)
        .ok_or(WorkingStoreError::LimitExceeded {
            kind,
            actual: u64::MAX,
            limit,
        })?;
    if total > limit {
        Err(WorkingStoreError::LimitExceeded {
            kind,
            actual: total,
            limit,
        })
    } else {
        Ok(total)
    }
}

fn cleanup_staged_file(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

enum NoClobberInstallError {
    AlreadyExists,
    Failed(WorkingStoreError),
}

impl From<WorkingStoreError> for NoClobberInstallError {
    fn from(error: WorkingStoreError) -> Self {
        Self::Failed(error)
    }
}

impl From<io::Error> for NoClobberInstallError {
    fn from(error: io::Error) -> Self {
        Self::Failed(error.into())
    }
}

fn classify_no_clobber_io(error: io::Error) -> NoClobberInstallError {
    if error.kind() == io::ErrorKind::AlreadyExists
        || matches!(error.raw_os_error(), Some(80 | 183))
    {
        NoClobberInstallError::AlreadyExists
    } else {
        NoClobberInstallError::Failed(error.into())
    }
}

fn require_preinstall_collision(error: NoClobberInstallError) -> Result<(), WorkingStoreError> {
    match error {
        NoClobberInstallError::AlreadyExists => Ok(()),
        // Cleanup, directory-sync, and write-through failures remain fatal even when the object
        // entry is already visible; otherwise a non-durable object could be published by head.
        NoClobberInstallError::Failed(error) => Err(error),
    }
}

#[cfg(windows)]
fn install_no_clobber(staged: &Path, destination: &Path) -> Result<(), NoClobberInstallError> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

    #[link(name = "Kernel32")]
    extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }

    let staged_meta = fs::symlink_metadata(staged)?;
    ensure_regular_no_link(staged, &staged_meta)?;
    ensure_single_link(staged, &staged_meta)?;

    // canonicalize gives Win32's verbatim absolute spelling, avoiding MAX_PATH truncation.
    let staged = fs::canonicalize(staged)?;
    let destination_parent =
        fs::canonicalize(
            destination
                .parent()
                .ok_or_else(|| WorkingStoreError::UnsafePath {
                    path: destination.to_path_buf(),
                    reason: "object path has no parent".to_owned(),
                })?,
        )?;
    let destination = destination_parent.join(destination.file_name().ok_or_else(|| {
        WorkingStoreError::UnsafePath {
            path: destination.to_path_buf(),
            reason: "object path has no filename".to_owned(),
        }
    })?);
    let staged_wide = staged
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: both pointers reference NUL-terminated buffers that live through the call. No
    // replace flag is supplied, so an existing immutable destination is never overwritten.
    let moved = unsafe {
        MoveFileExW(
            staged_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(classify_no_clobber_io(io::Error::last_os_error()))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn install_no_clobber(staged: &Path, destination: &Path) -> Result<(), NoClobberInstallError> {
    let staged_meta = fs::symlink_metadata(staged)?;
    ensure_regular_no_link(staged, &staged_meta)?;
    ensure_single_link(staged, &staged_meta)?;
    fs::hard_link(staged, destination).map_err(classify_no_clobber_io)?;
    cleanup_staged_file(staged).map_err(|source| WorkingStoreError::StagingCleanup {
        path: staged.to_path_buf(),
        source,
    })?;
    sync_directory(destination.parent().expect("derived object has a parent"))?;
    sync_directory(staged.parent().expect("staging object has a parent"))?;
    Ok(())
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> Result<(), WorkingStoreError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn read_optional_regular_bounded(
    path: &Path,
    max: usize,
    kind: &'static str,
) -> Result<Option<Vec<u8>>, WorkingStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure_safe_existing_chain(path)?;
            ensure_regular_no_link(path, &metadata)?;
            if metadata.len() > max as u64 {
                return Err(WorkingStoreError::LimitExceeded {
                    kind,
                    actual: metadata.len(),
                    limit: max as u64,
                });
            }
            let mut file = open_regular_read_no_follow(path)?;
            let opened_metadata = file.metadata()?;
            ensure_regular_no_link(path, &opened_metadata)?;
            ensure_single_link(path, &opened_metadata)?;
            if opened_metadata.len() > max as u64 {
                return Err(WorkingStoreError::LimitExceeded {
                    kind,
                    actual: opened_metadata.len(),
                    limit: max as u64,
                });
            }
            let mut bytes = Vec::with_capacity(opened_metadata.len() as usize);
            Read::by_ref(&mut file)
                .take(max as u64 + 1)
                .read_to_end(&mut bytes)?;
            enforce_limit(kind, bytes.len(), max)?;
            if bytes.len() as u64 != opened_metadata.len() {
                return Err(WorkingStoreError::Invariant(format!(
                    "{kind} length changed while reading {}: expected {}, read {}",
                    path.display(),
                    opened_metadata.len(),
                    bytes.len()
                )));
            }
            Ok(Some(bytes))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn read_required_regular_bounded(
    path: &Path,
    max: usize,
    kind: &'static str,
) -> Result<Vec<u8>, WorkingStoreError> {
    read_optional_regular_bounded(path, max, kind)?
        .ok_or_else(|| WorkingStoreError::MissingObject(path.to_path_buf()))
}

fn hash_file(path: &Path, expected_len: u64) -> Result<Sha256Digest, WorkingStoreError> {
    let mut file = open_regular_read_no_follow(path)?;
    let metadata = file.metadata()?;
    ensure_regular_no_link(path, &metadata)?;
    ensure_single_link(path, &metadata)?;
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or_else(|| {
            WorkingStoreError::Invariant("file length overflow while hashing".to_owned())
        })?;
        if total > expected_len {
            return Err(WorkingStoreError::SealMismatch {
                path: path.to_path_buf(),
                reason: format!("file grew beyond expected {expected_len} bytes while hashing"),
            });
        }
        hasher.update(&buffer[..count]);
    }
    if total != expected_len {
        return Err(WorkingStoreError::SealMismatch {
            path: path.to_path_buf(),
            reason: format!("expected {expected_len} bytes, read {total}"),
        });
    }
    Ok(digest_from_hasher(hasher))
}

fn open_regular_read_no_follow(path: &Path) -> Result<File, WorkingStoreError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Open the reparse point itself so handle metadata can reject it rather than following it.
        options.custom_flags(0x0020_0000);
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Linux O_NOFOLLOW. Other Unix targets retain prefix and post-open handle checks.
        options.custom_flags(0x0002_0000);
    }
    let file = options.open(path)?;
    ensure_regular_no_link(path, &file.metadata()?)?;
    Ok(file)
}

fn absolute_path(path: &Path) -> Result<PathBuf, WorkingStoreError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    normalize_absolute(&absolute)
}

fn normalize_absolute(path: &Path) -> Result<PathBuf, WorkingStoreError> {
    use std::path::Component;
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(WorkingStoreError::UnsafePath {
                        path: path.to_path_buf(),
                        reason: "path escapes its filesystem root".to_owned(),
                    });
                }
            }
        }
    }
    if !normalized.is_absolute() {
        return Err(WorkingStoreError::UnsafePath {
            path: path.to_path_buf(),
            reason: "path could not be made absolute".to_owned(),
        });
    }
    Ok(normalized)
}

fn create_directory_chain(path: &Path) -> Result<(), WorkingStoreError> {
    // Reject any existing link before creation, then re-walk after creation. This deliberately
    // avoids `create_dir_all` traversing an existing link/reparse prefix.
    ensure_existing_prefixes_safe(path)?;
    fs::create_dir_all(path)?;
    ensure_safe_directory_chain(path)
}

fn ensure_safe_directory_chain(path: &Path) -> Result<(), WorkingStoreError> {
    ensure_existing_prefixes_safe(path)?;
    let metadata = fs::symlink_metadata(path)?;
    if is_link_or_reparse(&metadata) || !metadata.file_type().is_dir() {
        return Err(WorkingStoreError::UnsafePath {
            path: path.to_path_buf(),
            reason: "expected a real directory, not a link/reparse point".to_owned(),
        });
    }
    Ok(())
}

fn ensure_safe_existing_chain(path: &Path) -> Result<(), WorkingStoreError> {
    ensure_existing_prefixes_safe(path)
}

fn ensure_existing_prefixes_safe(path: &Path) -> Result<(), WorkingStoreError> {
    let mut prefixes = path.ancestors().collect::<Vec<_>>();
    prefixes.reverse();
    for prefix in prefixes {
        if prefix.as_os_str().is_empty() {
            continue;
        }
        match fs::symlink_metadata(prefix) {
            Ok(metadata) => {
                if is_link_or_reparse(&metadata) {
                    return Err(WorkingStoreError::UnsafePath {
                        path: prefix.to_path_buf(),
                        reason: "link/reparse points are forbidden in store paths".to_owned(),
                    });
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn ensure_regular_no_link(path: &Path, metadata: &fs::Metadata) -> Result<(), WorkingStoreError> {
    if is_link_or_reparse(metadata) || !metadata.file_type().is_file() {
        return Err(WorkingStoreError::UnsafePath {
            path: path.to_path_buf(),
            reason: "expected a regular file, not a link/reparse point".to_owned(),
        });
    }
    Ok(())
}

fn ensure_single_link(path: &Path, _metadata: &fs::Metadata) -> Result<(), WorkingStoreError> {
    #[cfg(windows)]
    let count = windows_hard_link_count(path)?;
    #[cfg(unix)]
    let count = {
        use std::os::unix::fs::MetadataExt;
        _metadata.nlink()
    };
    #[cfg(not(any(windows, unix)))]
    let count = 1;

    if count != 1 {
        Err(WorkingStoreError::UnsafePath {
            path: path.to_path_buf(),
            reason: format!("immutable object has {count} hard links; expected exactly one"),
        })
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn windows_hard_link_count(path: &Path) -> Result<u64, WorkingStoreError> {
    use std::ffi::c_void;
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct FileTime {
        dwLowDateTime: u32,
        dwHighDateTime: u32,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct ByHandleFileInformation {
        dwFileAttributes: u32,
        ftCreationTime: FileTime,
        ftLastAccessTime: FileTime,
        ftLastWriteTime: FileTime,
        dwVolumeSerialNumber: u32,
        nFileSizeHigh: u32,
        nFileSizeLow: u32,
        nNumberOfLinks: u32,
        nFileIndexHigh: u32,
        nFileIndexLow: u32,
    }

    #[link(name = "Kernel32")]
    extern "system" {
        fn GetFileInformationByHandle(
            file: *mut c_void,
            information: *mut ByHandleFileInformation,
        ) -> i32;
    }

    let file = open_regular_read_no_follow(path)?;
    let mut information = MaybeUninit::<ByHandleFileInformation>::uninit();
    // SAFETY: `file` remains open, and `information` points to writable storage of the exact
    // Win32 BY_HANDLE_FILE_INFORMATION layout until the call returns.
    let succeeded =
        unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) };
    if succeeded == 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: a successful call initializes the complete output structure.
    let information = unsafe { information.assume_init() };
    Ok(u64::from(information.nNumberOfLinks))
}

fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & WINDOWS_REPARSE_POINT_ATTRIBUTE != 0
    }
    #[cfg(not(windows))]
    {
        let _ = WINDOWS_REPARSE_POINT_ATTRIBUTE;
        false
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn published_revision3_basis(
        store: &WorkingProjectStore,
        tag: u8,
    ) -> Revision3CheckpointPreparation {
        let project = ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([tag; 16]),
            revision: 4,
            meta: ProjectMeta {
                name: "Quest artifact V2 basis".into(),
                version: "0.1.0".into(),
                author: "tests".into(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 170_000_000,
                    sha256: Sha256Digest::from_bytes([tag.wrapping_add(1); 32]),
                },
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        };
        let preparation = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(store.head_path(), &preparation.head_bytes).unwrap();
        preparation
    }

    #[test]
    fn post_install_durability_failure_is_never_reclassified_as_dedupe() {
        let injected = WorkingStoreError::StagingCleanup {
            path: PathBuf::from("injected-staging-object"),
            source: io::Error::other("injected directory sync failure"),
        };
        assert!(matches!(
            require_preinstall_collision(NoClobberInstallError::Failed(injected)),
            Err(WorkingStoreError::StagingCleanup { .. })
        ));
        assert!(require_preinstall_collision(NoClobberInstallError::AlreadyExists).is_ok());
    }

    #[test]
    fn exact_revision3_no_op_preserves_head_without_entering_a_write_boundary() {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-authoring-revision3-history-no-op-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
        let basis = published_revision3_basis(&store, 0x71);
        let project = store
            .open_current_revision3(AssetVerification::Full)
            .unwrap()
            .project;
        let write_guards = Cell::new(0usize);

        let prepared = store
            .prepare_revision3_checkpoint_with_write_guard(Some(&basis.head), &project, || {
                write_guards.set(write_guards.get() + 1);
                Ok(())
            })
            .unwrap();

        assert_eq!(prepared, basis);
        assert_eq!(write_guards.get(), 0);
        assert_eq!(store.current_head().unwrap(), Some(basis.head));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn quest_artifact_head_race_after_install_returns_conflict_and_leaves_only_orphan() {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-authoring-artifact-final-head-race-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
        let raced_head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: ContentSeal {
                byte_len: 1,
                sha256: Sha256Digest::from_bytes([0x91; 32]),
            },
        };
        let raced_head_bytes = canonical_json(&raced_head).unwrap();
        let artifact_bytes = b"{}";
        let artifact = seal_bytes(artifact_bytes);

        let result = store.import_quest_collision_artifact_v1_with_final_head_hook(
            artifact_bytes,
            None,
            || {
                fs::write(store.head_path(), &raced_head_bytes)?;
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(WorkingStoreError::HeadConflict {
                expected: None,
                actual: Some(actual),
            }) if actual == raced_head
        ));
        store
            .verify_seal_at(
                &store.asset_path(artifact.sha256),
                &artifact,
                AssetVerification::Full,
                false,
            )
            .unwrap();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn quest_artifact_v2_import_is_exact_basis_bound_and_deduplicates() {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-authoring-artifact-v2-import-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
        let basis = published_revision3_basis(&store, 0x62);
        let bytes = br#"{"format":"quest_collision_capability","schema_revision":2}"#;
        let expected = seal_bytes(bytes);

        let first = store
            .import_quest_collision_artifact_v2(bytes, &basis.head)
            .unwrap();
        let second = store
            .import_quest_collision_artifact_v2(bytes, &basis.head)
            .unwrap();
        assert_eq!(first.artifact, expected);
        assert_eq!(first.basis_head, basis.head);
        assert_eq!(first.asset_meta.byte_len, expected.byte_len);
        assert_eq!(
            first.asset_meta.media_type,
            QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2
        );
        assert!(!first.deduplicated);
        assert!(second.deduplicated);
        store
            .verify_seal_at(
                &store.asset_path(expected.sha256),
                &expected,
                AssetVerification::Full,
                false,
            )
            .unwrap();
        assert_eq!(
            store
                .open_current_revision3(AssetVerification::Full)
                .unwrap()
                .head,
            basis.head
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn quest_artifact_v2_stale_basis_is_rejected_before_install() {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-authoring-artifact-v2-stale-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
        let basis = published_revision3_basis(&store, 0x63);
        let stale = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: ContentSeal {
                byte_len: 1,
                sha256: Sha256Digest::from_bytes([0x64; 32]),
            },
        };
        let bytes = b"stale V2 artifact";
        let expected = seal_bytes(bytes);

        assert!(matches!(
            store.import_quest_collision_artifact_v2(bytes, &stale),
            Err(WorkingStoreError::HeadConflict {
                expected: Some(expected),
                actual: Some(actual),
            }) if expected == stale && actual == basis.head
        ));
        assert!(!store.asset_path(expected.sha256).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn quest_artifact_v2_post_install_head_race_leaves_only_verified_orphan() {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-authoring-artifact-v2-final-head-race-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
        let basis = published_revision3_basis(&store, 0x65);
        let raced_head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: ContentSeal {
                byte_len: 1,
                sha256: Sha256Digest::from_bytes([0x66; 32]),
            },
        };
        let raced_head_bytes = canonical_json(&raced_head).unwrap();
        let bytes = b"racing V2 artifact";
        let artifact = seal_bytes(bytes);

        let result = store.import_quest_collision_artifact_v2_with_final_head_hook(
            bytes,
            &basis.head,
            || {
                fs::write(store.head_path(), &raced_head_bytes)?;
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(WorkingStoreError::HeadConflict {
                expected: Some(expected),
                actual: Some(actual),
            }) if expected == basis.head && actual == raced_head
        ));
        store
            .verify_seal_at(
                &store.asset_path(artifact.sha256),
                &artifact,
                AssetVerification::Full,
                false,
            )
            .unwrap();
        assert_eq!(fs::read(store.head_path()).unwrap(), raced_head_bytes);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dataasset_component_post_install_head_race_leaves_only_verified_orphan() {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-authoring-dataasset-component-race-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
        let basis = published_revision3_basis(&store, 0x71);
        let raced_head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: ContentSeal {
                byte_len: 1,
                sha256: Sha256Digest::from_bytes([0x72; 32]),
            },
        };
        let raced_head_bytes = canonical_json(&raced_head).unwrap();
        let bytes = b"verified DataAsset component";
        let component = seal_bytes(bytes);

        let result = store.import_exact_dataasset_bytes_with_final_head_hook(
            bytes,
            &component,
            1024,
            "DataAsset component test",
            &basis.head,
            || {
                fs::write(store.head_path(), &raced_head_bytes)?;
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(WorkingStoreError::HeadConflict {
                expected: Some(expected),
                actual: Some(actual),
            }) if expected == basis.head && actual == raced_head
        ));
        store
            .verify_seal_at(
                &store.asset_path(component.sha256),
                &component,
                AssetVerification::Full,
                false,
            )
            .unwrap();
        assert_eq!(fs::read(store.head_path()).unwrap(), raced_head_bytes);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dataasset_write_guard_skips_dedupe_and_runs_for_the_first_missing_blob() {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-authoring-dataasset-dedupe-guard-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
        let basis = published_revision3_basis(&store, 0x73);
        let existing_bytes = b"existing DataAsset component";
        let existing = seal_bytes(existing_bytes);
        store
            .import_exact_dataasset_bytes_with_write_guard(
                existing_bytes,
                &existing,
                1024,
                "DataAsset component test",
                &basis.head,
                || Ok(()),
            )
            .unwrap();

        let calls = std::cell::Cell::new(0usize);
        assert!(store
            .import_exact_dataasset_bytes_with_write_guard(
                existing_bytes,
                &existing,
                1024,
                "DataAsset component test",
                &basis.head,
                || {
                    calls.set(calls.get() + 1);
                    Ok(())
                },
            )
            .unwrap());
        assert_eq!(
            calls.get(),
            0,
            "dedupe must not consume the first-write guard"
        );

        let new_bytes = b"new DataAsset component";
        let new_seal = seal_bytes(new_bytes);
        assert!(!store
            .import_exact_dataasset_bytes_with_write_guard(
                new_bytes,
                &new_seal,
                1024,
                "DataAsset component test",
                &basis.head,
                || {
                    calls.set(calls.get() + 1);
                    Ok(())
                },
            )
            .unwrap());
        assert_eq!(calls.get(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn current_revision3_quest_source_rechecks_the_fixed_head_after_full_preparation() {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gore-authoring-current-quest-source-final-head-race-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = WorkingProjectStore::at(&root, WorkingStoreLimits::default()).unwrap();
        let project = ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([0x31; 16]),
            revision: 4,
            meta: ProjectMeta {
                name: "current Quest source race".into(),
                version: "0.1.0".into(),
                author: "tests".into(),
            },
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 170_000_000,
                    sha256: Sha256Digest::from_bytes([0x41; 32]),
                },
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        };
        let current = store.prepare_revision3_checkpoint(None, &project).unwrap();
        fs::write(store.head_path(), &current.head_bytes).unwrap();
        let raced_head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: ContentSeal {
                byte_len: 1,
                sha256: Sha256Digest::from_bytes([0x51; 32]),
            },
        };
        let raced_head_bytes = canonical_json(&raced_head).unwrap();

        let result = store
            .prepare_current_revision3_quest_collision_source_v2_with_final_head_hook(
                &current.head,
                || {
                    fs::write(store.head_path(), &raced_head_bytes)?;
                    Ok(())
                },
            );
        assert!(matches!(
            result,
            Err(Revision3QuestCollisionSourceErrorV2::Store(
                WorkingStoreError::HeadConflict {
                    expected: Some(expected),
                    actual: Some(actual),
                }
            )) if expected == current.head && actual == raced_head
        ));
        let _ = fs::remove_dir_all(root);
    }
}
