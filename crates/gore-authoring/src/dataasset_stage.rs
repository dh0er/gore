//! Managed, deployment-independent staging of receipt-bound fixed-leaf DataAsset edits.
//!
//! A stage is represented only by an immutable canonical manifest plus ordinary AssetStore CAS
//! blobs. It deliberately adds no revision-3 entity kind and grants no build, runtime, deploy,
//! reinspection, or fixed-head publication authority.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use gore_asset::dataasset_workflow::{
    validate_generation_receipt, validate_sidecar_generation_mapping, AssetGenerationReceipt,
    SidecarReceipt, SidecarRole, VerifiedFixedLeafStageInput, MAX_OPTIONAL_SIDECAR_BYTES,
    MAX_USMAP_BYTES,
};
use gore_asset::{
    FixedLeafRole, FixedLeafSelector, FixedWireKind, PackageComponent, FIXED_LEAF_SELECTOR_FORMAT,
    FIXED_LEAF_SELECTOR_PROFILE,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    AssetMeta, AssetVerification, ContentSeal, GameGenerationAnchor, ProjectId, ProjectRevision3,
    Revision3CheckpointPreparation, Sha256Digest, WorkingHead, WorkingProjectStore,
    WorkingStoreError, WorkingStoreLimits, MAX_REVISION3_ASSETS,
    MAX_REVISION3_REFERENCED_ASSET_BYTES,
};

const DATAASSET_FIXED_LEAF_STAGE_FORMAT_V1: &str = "gore.dataasset.fixed-leaf-stage.v1";
pub const DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1: &str =
    "application/vnd.gore.dataasset-fixed-leaf-stage+json;version=1";
pub const DATAASSET_FIXED_LEAF_COMPONENT_MEDIA_TYPE_V1: &str =
    "application/vnd.gore.dataasset-fixed-leaf-component;version=1";
pub const MAX_DATAASSET_FIXED_LEAF_STAGE_MANIFEST_BYTES_V1: usize = 8 * 1024 * 1024;
pub const MAX_DATAASSET_FIXED_LEAF_STAGES_V1: usize = 1024;
pub const MAX_DATAASSET_STAGE_MANIFEST_BATCH_BYTES_V1: u64 = 64 * 1024 * 1024;
pub const MAX_DATAASSET_STAGE_HISTORICAL_BASES_V1: usize = 256;
pub const MAX_DATAASSET_STAGE_HISTORICAL_BASIS_BYTES_V1: u64 = 64 * 1024 * 1024;
pub const MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1: u64 = 262_144;
pub const MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1: u64 = 128 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataAssetStageBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataAssetStageRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataAssetStageArtifactAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataAssetStagePublicationStatusV1 {
    NotSupported,
}

/// Closed manifest stored as an ordinary content-addressed asset.
///
/// It contains no local filesystem path, patch offset, receipt bytes, receipt seal, or
/// Ogg-oriented `AssetRef`. Its semantic selector is offset-free; historical generation filenames
/// are content anchors, never local paths. All byte-bearing members are exact raw SHA-256/length
/// seals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3DataAssetStageManifestV1 {
    format: String,
    project_id: ProjectId,
    project_target: GameGenerationAnchor,
    basis_head: WorkingHead,
    basis_project_revision: u64,
    staged_project_revision: u64,
    target_path: String,
    generation: AssetGenerationReceipt,
    selector: FixedLeafSelector,
    replacement_hex: String,
    patched_uasset: ContentSeal,
    patched_uexp: ContentSeal,
    usmap: ContentSeal,
    sidecars: BTreeMap<SidecarRole, ContentSeal>,
    build_status: DataAssetStageBuildStatusV1,
    runtime_status: DataAssetStageRuntimeStatusV1,
    artifact_authority: DataAssetStageArtifactAuthorityV1,
    publication_status: DataAssetStagePublicationStatusV1,
}

impl Revision3DataAssetStageManifestV1 {
    pub fn from_json(json: &str) -> Result<Self, DataAssetStageManifestErrorV1> {
        if json.len() > MAX_DATAASSET_FIXED_LEAF_STAGE_MANIFEST_BYTES_V1 {
            return Err(DataAssetStageManifestErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_DATAASSET_FIXED_LEAF_STAGE_MANIFEST_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json).map_err(DataAssetStageManifestErrorV1::InvalidJson)?;
        let manifest: Self =
            serde_json::from_str(json).map_err(DataAssetStageManifestErrorV1::InvalidJson)?;
        manifest.validate()?;
        if manifest.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(DataAssetStageManifestErrorV1::NonCanonicalJson);
        }
        Ok(manifest)
    }

    pub fn to_canonical_json(&self) -> Result<String, DataAssetStageManifestErrorV1> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(DataAssetStageManifestErrorV1::Serialize)?;
        if bytes.len() > MAX_DATAASSET_FIXED_LEAF_STAGE_MANIFEST_BYTES_V1 {
            return Err(DataAssetStageManifestErrorV1::InputTooLarge {
                actual: bytes.len(),
                limit: MAX_DATAASSET_FIXED_LEAF_STAGE_MANIFEST_BYTES_V1,
            });
        }
        String::from_utf8(bytes).map_err(|_| DataAssetStageManifestErrorV1::NonUtf8Serialization)
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn project_target(&self) -> &GameGenerationAnchor {
        &self.project_target
    }

    pub fn basis_head(&self) -> &WorkingHead {
        &self.basis_head
    }

    pub const fn basis_project_revision(&self) -> u64 {
        self.basis_project_revision
    }

    pub const fn staged_project_revision(&self) -> u64 {
        self.staged_project_revision
    }

    pub fn target_path(&self) -> &str {
        &self.target_path
    }

    pub fn generation(&self) -> &AssetGenerationReceipt {
        &self.generation
    }

    pub fn selector(&self) -> &FixedLeafSelector {
        &self.selector
    }

    pub fn replacement_hex(&self) -> &str {
        &self.replacement_hex
    }

    pub fn patched_uasset(&self) -> &ContentSeal {
        &self.patched_uasset
    }

    pub fn patched_uexp(&self) -> &ContentSeal {
        &self.patched_uexp
    }

    pub fn usmap(&self) -> &ContentSeal {
        &self.usmap
    }

    pub fn sidecars(&self) -> &BTreeMap<SidecarRole, ContentSeal> {
        &self.sidecars
    }

    pub const fn build_status(&self) -> DataAssetStageBuildStatusV1 {
        self.build_status
    }

    pub const fn runtime_status(&self) -> DataAssetStageRuntimeStatusV1 {
        self.runtime_status
    }

    pub const fn artifact_authority(&self) -> DataAssetStageArtifactAuthorityV1 {
        self.artifact_authority
    }

    pub const fn publication_status(&self) -> DataAssetStagePublicationStatusV1 {
        self.publication_status
    }

    fn validate(&self) -> Result<(), DataAssetStageManifestErrorV1> {
        if self.format != DATAASSET_FIXED_LEAF_STAGE_FORMAT_V1 {
            return Err(DataAssetStageManifestErrorV1::Invalid(
                "unsupported manifest format".to_owned(),
            ));
        }
        if self.project_id.as_bytes().iter().all(|byte| *byte == 0)
            || self.project_target.executable.byte_len == 0
            || self.basis_head.snapshot.byte_len == 0
            || self
                .basis_project_revision
                .checked_add(1)
                .is_none_or(|revision| revision != self.staged_project_revision)
        {
            return Err(DataAssetStageManifestErrorV1::Invalid(
                "invalid project or exact-basis binding".to_owned(),
            ));
        }
        validate_generation_receipt(&self.generation, "DATAASSET_STAGE_MANIFEST")
            .map_err(|error| DataAssetStageManifestErrorV1::Invalid(error.to_string()))?;
        if self.target_path != self.generation.asset {
            return Err(DataAssetStageManifestErrorV1::Invalid(
                "target path differs from generation receipt".to_owned(),
            ));
        }
        validate_stage_intent(&self.selector, &self.replacement_hex, &self.generation)?;
        let package_limits = gore_asset::dataasset_workflow::asset_package_limits();
        validate_seal(
            &self.patched_uasset,
            package_limits.max_uasset_bytes,
            "patched uasset",
        )?;
        validate_seal(
            &self.patched_uexp,
            package_limits.max_uexp_bytes,
            "patched uexp",
        )?;
        if self
            .patched_uasset
            .byte_len
            .checked_add(self.patched_uexp.byte_len)
            .is_none_or(|bytes| bytes > package_limits.max_total_bytes)
        {
            return Err(DataAssetStageManifestErrorV1::Invalid(
                "patched pair exceeds aggregate limit".to_owned(),
            ));
        }
        validate_seal(&self.usmap, MAX_USMAP_BYTES, "USMAP")?;
        let expected_usmap = digest_from_canonical_hex(&self.generation.usmap.sha256)?;
        if self.usmap.byte_len != self.generation.usmap.length
            || self.usmap.sha256 != expected_usmap
        {
            return Err(DataAssetStageManifestErrorV1::Invalid(
                "USMAP seal differs from exact generation".to_owned(),
            ));
        }
        if self.sidecars.len() > SidecarRole::ALL.len() {
            return Err(DataAssetStageManifestErrorV1::Invalid(
                "too many optional sidecars".to_owned(),
            ));
        }
        let sidecar_receipts: Vec<_> = self
            .sidecars
            .iter()
            .map(|(role, seal)| SidecarReceipt {
                role: *role,
                file_name: format!("stage.{}", role.suffix()),
                length: seal.byte_len,
                sha256: seal.sha256.to_string(),
            })
            .collect();
        validate_sidecar_generation_mapping(
            &sidecar_receipts,
            &self.generation,
            "DATAASSET_STAGE_MANIFEST",
        )
        .map_err(|error| DataAssetStageManifestErrorV1::Invalid(error.to_string()))?;
        let mut sidecar_bytes = 0u64;
        for seal in self.sidecars.values() {
            validate_seal(seal, MAX_OPTIONAL_SIDECAR_BYTES, "sidecar")?;
            sidecar_bytes = sidecar_bytes.checked_add(seal.byte_len).ok_or_else(|| {
                DataAssetStageManifestErrorV1::Invalid("sidecar size overflow".to_owned())
            })?;
        }
        if self
            .patched_uasset
            .byte_len
            .checked_add(self.patched_uexp.byte_len)
            .and_then(|pair| pair.checked_add(sidecar_bytes))
            .is_none_or(|total| total > gore_asset::dataasset_workflow::MAX_COOKED_PACKAGE_BYTES)
        {
            return Err(DataAssetStageManifestErrorV1::Invalid(
                "cooked package plus sidecars exceeds aggregate limit".to_owned(),
            ));
        }
        Ok(())
    }

    fn component_seals(&self) -> impl Iterator<Item = &ContentSeal> {
        [&self.patched_uasset, &self.patched_uexp, &self.usmap]
            .into_iter()
            .chain(self.sidecars.values())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DataAssetStageManifestErrorV1 {
    #[error("DataAsset stage manifest exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid DataAsset stage manifest JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize DataAsset stage manifest: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("DataAsset stage manifest is not in exact canonical JSON form")]
    NonCanonicalJson,
    #[error("DataAsset stage manifest serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
    #[error("invalid DataAsset stage manifest: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DataAssetStageConflictV1 {
    #[error("a fixed-leaf DataAsset stage already exists for target {target:?}")]
    TargetAlreadyStaged { target: String },
    #[error("no fixed-leaf DataAsset stage exists for target {target:?}")]
    TargetNotStaged { target: String },
    #[error("the revision-3 project revision cannot be incremented")]
    ProjectRevisionOverflow,
    #[error("the revision-3 project cannot index all stage blobs")]
    AssetCapacityExceeded,
    #[error("the revision-3 project's referenced asset-byte limit would be exceeded")]
    AssetBytesExceeded,
    #[error("asset {asset} already has incompatible metadata")]
    AssetMetadataCollision { asset: Sha256Digest },
    #[error("the current project contains duplicate stage manifests for target {target:?}")]
    DuplicateTarget { target: String },
    #[error(
        "DataAsset stage batch exceeds its {resource} budget: {actual} is greater than {limit}"
    )]
    StageBatchBudgetExceeded {
        resource: &'static str,
        actual: u64,
        limit: u64,
    },
    #[error("the verified live executable does not match the revision-3 project target")]
    ExecutableTargetMismatch,
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3DataAssetStagingErrorV1 {
    #[error(transparent)]
    Store(#[from] WorkingStoreError),
    #[error(transparent)]
    Manifest(#[from] DataAssetStageManifestErrorV1),
    #[error("verified DataAsset input became invalid: {0}")]
    VerifiedInput(String),
    #[error(transparent)]
    Conflict(#[from] DataAssetStageConflictV1),
    #[error("DataAsset stage manifest is not bound to the exact project: {0}")]
    ProjectBinding(String),
    #[error("prepared DataAsset stage candidate did not reopen exactly")]
    CandidateReopenMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3DataAssetStageViewV1 {
    manifest_asset: ContentSeal,
    manifest: Revision3DataAssetStageManifestV1,
}

impl Revision3DataAssetStageViewV1 {
    pub fn manifest_asset(&self) -> &ContentSeal {
        &self.manifest_asset
    }

    pub fn manifest(&self) -> &Revision3DataAssetStageManifestV1 {
        &self.manifest
    }

    pub fn target_path(&self) -> &str {
        self.manifest.target_path()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PreparedRevision3DataAssetStageV1 {
    basis_head: WorkingHead,
    project: ProjectRevision3,
    canonical_project_json: String,
    checkpoint: Revision3CheckpointPreparation,
    stage: Revision3DataAssetStageViewV1,
    deduplicated_blobs: usize,
}

impl PreparedRevision3DataAssetStageV1 {
    pub fn basis_head(&self) -> &WorkingHead {
        &self.basis_head
    }

    pub fn project(&self) -> &ProjectRevision3 {
        &self.project
    }

    pub fn canonical_project_json(&self) -> &str {
        &self.canonical_project_json
    }

    pub fn checkpoint(&self) -> &Revision3CheckpointPreparation {
        &self.checkpoint
    }

    pub fn stage(&self) -> &Revision3DataAssetStageViewV1 {
        &self.stage
    }

    pub const fn deduplicated_blobs(&self) -> usize {
        self.deduplicated_blobs
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PreparedRevision3DataAssetStageRemovalV1 {
    basis_head: WorkingHead,
    project: ProjectRevision3,
    canonical_project_json: String,
    checkpoint: Revision3CheckpointPreparation,
    removed: Revision3DataAssetStageViewV1,
}

impl PreparedRevision3DataAssetStageRemovalV1 {
    pub fn basis_head(&self) -> &WorkingHead {
        &self.basis_head
    }

    pub fn project(&self) -> &ProjectRevision3 {
        &self.project
    }

    pub fn canonical_project_json(&self) -> &str {
        &self.canonical_project_json
    }

    pub fn checkpoint(&self) -> &Revision3CheckpointPreparation {
        &self.checkpoint
    }

    pub fn removed(&self) -> &Revision3DataAssetStageViewV1 {
        &self.removed
    }
}

struct DataAssetStageProjectionV1<'a> {
    target_path: &'a str,
    generation: &'a AssetGenerationReceipt,
    selector: &'a FixedLeafSelector,
    replacement_hex: &'a str,
    patched_uasset: &'a [u8],
    patched_uexp: &'a [u8],
    usmap: &'a [u8],
    sidecars: Vec<(SidecarRole, &'a [u8])>,
    executable: ContentSeal,
}

impl WorkingProjectStore {
    /// Import a fully verified fixed-leaf result and prepare, but never publish, a revision-3
    /// candidate head. Every imported object can remain as a safe immutable orphan on a race.
    pub fn prepare_revision3_dataasset_stage_v1(
        &self,
        basis_head: &WorkingHead,
        input: VerifiedFixedLeafStageInput,
    ) -> Result<PreparedRevision3DataAssetStageV1, Revision3DataAssetStagingErrorV1> {
        let projection = DataAssetStageProjectionV1 {
            target_path: input.target_path(),
            generation: input.generation(),
            selector: input.selector(),
            replacement_hex: input.replacement_hex(),
            patched_uasset: input.patched_component_bytes(PackageComponent::Uasset),
            patched_uexp: input.patched_component_bytes(PackageComponent::Uexp),
            usmap: input.usmap_bytes(),
            sidecars: input
                .sidecars()
                .iter()
                .map(|sidecar| (sidecar.role(), sidecar.bytes()))
                .collect(),
            executable: ContentSeal {
                byte_len: input.executable_anchor().length(),
                sha256: Sha256Digest::from_bytes(*input.executable_anchor().sha256()),
            },
        };
        self.prepare_revision3_dataasset_stage_projection_v1(
            basis_head,
            projection,
            || {
                input
                    .require_store_root_disjoint(self.root())
                    .map_err(|error| {
                        Revision3DataAssetStagingErrorV1::VerifiedInput(error.to_string())
                    })?;
                input.reverify_executable_anchor().map_err(|error| {
                    Revision3DataAssetStagingErrorV1::VerifiedInput(error.to_string())
                })
            },
            || {
                input
                    .require_store_root_disjoint(self.root())
                    .map_err(|error| {
                        Revision3DataAssetStagingErrorV1::VerifiedInput(error.to_string())
                    })?;
                input.reverify_executable_path_identity().map_err(|error| {
                    Revision3DataAssetStagingErrorV1::VerifiedInput(error.to_string())
                })
            },
            || {
                input.reverify_live_generation().map_err(|error| {
                    Revision3DataAssetStagingErrorV1::VerifiedInput(error.to_string())
                })
            },
        )
    }

    fn prepare_revision3_dataasset_stage_projection_v1<F, G, H>(
        &self,
        basis_head: &WorkingHead,
        input: DataAssetStageProjectionV1<'_>,
        verify_executable: F,
        verify_write_boundary: G,
        verify_generation: H,
    ) -> Result<PreparedRevision3DataAssetStageV1, Revision3DataAssetStagingErrorV1>
    where
        F: Fn() -> Result<(), Revision3DataAssetStagingErrorV1>,
        G: Fn() -> Result<(), Revision3DataAssetStagingErrorV1>,
        H: Fn() -> Result<(), Revision3DataAssetStagingErrorV1>,
    {
        self.require_exact_head_for_dataasset(Some(basis_head))?;
        let opened = self.open_current_revision3(AssetVerification::Full)?;
        if &opened.head != basis_head {
            return Err(WorkingStoreError::HeadConflict {
                expected: Some(basis_head.clone()),
                actual: Some(opened.head),
            }
            .into());
        }
        require_executable_binding(&input, &opened.project.target, &verify_executable)?;
        verify_generation()?;
        let existing = self.load_revision3_dataasset_stages(&opened.project)?;
        if existing
            .iter()
            .any(|stage| stage.target_path().eq_ignore_ascii_case(input.target_path))
        {
            return Err(DataAssetStageConflictV1::TargetAlreadyStaged {
                target: input.target_path.to_owned(),
            }
            .into());
        }
        let staged_project_revision = opened
            .project
            .revision
            .checked_add(1)
            .ok_or(DataAssetStageConflictV1::ProjectRevisionOverflow)?;

        let package_limits = gore_asset::dataasset_workflow::asset_package_limits();
        let patched_uasset = seal_bytes(input.patched_uasset);
        let patched_uexp = seal_bytes(input.patched_uexp);
        let usmap = seal_bytes(input.usmap);
        let mut sidecars = BTreeMap::new();
        for (role, source) in &input.sidecars {
            let seal = seal_bytes(source);
            if sidecars.insert(*role, seal).is_some() {
                return Err(Revision3DataAssetStagingErrorV1::VerifiedInput(
                    "duplicate verified sidecar role".to_owned(),
                ));
            }
        }
        let manifest = Revision3DataAssetStageManifestV1 {
            format: DATAASSET_FIXED_LEAF_STAGE_FORMAT_V1.to_owned(),
            project_id: opened.project.project_id,
            project_target: opened.project.target.clone(),
            basis_head: basis_head.clone(),
            basis_project_revision: opened.project.revision,
            staged_project_revision,
            target_path: input.target_path.to_owned(),
            generation: input.generation.clone(),
            selector: input.selector.clone(),
            replacement_hex: input.replacement_hex.to_owned(),
            patched_uasset,
            patched_uexp,
            usmap,
            sidecars,
            build_status: DataAssetStageBuildStatusV1::Blocked,
            runtime_status: DataAssetStageRuntimeStatusV1::RuntimeUnqualified,
            artifact_authority: DataAssetStageArtifactAuthorityV1::NotGranted,
            publication_status: DataAssetStagePublicationStatusV1::NotSupported,
        };
        let manifest_json = manifest.to_canonical_json()?;
        let manifest_asset = seal_bytes(manifest_json.as_bytes());
        preflight_candidate_stage_budgets(self, &existing, &manifest_asset, basis_head)?;

        // Complete every deterministic closed-model/count/metadata preflight before installing
        // any immutable blob. Only an I/O failure or concurrent head race may leave an orphan.
        let mut candidate = opened.project;
        for seal in manifest.component_seals() {
            insert_asset_meta(
                &mut candidate,
                seal,
                DATAASSET_FIXED_LEAF_COMPONENT_MEDIA_TYPE_V1,
            )?;
        }
        insert_asset_meta(
            &mut candidate,
            &manifest_asset,
            DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1,
        )?;
        candidate.revision = staged_project_revision;
        validate_candidate_asset_limits(&candidate, self.limits())?;
        let canonical_project_json = candidate
            .to_canonical_json()
            .map_err(|error| Revision3DataAssetStagingErrorV1::ProjectBinding(error.to_string()))?;
        self.preflight_revision3_dataasset_candidate(&candidate)?;

        // Close live drift that may have occurred during stage/history loading and deterministic
        // candidate validation before the first CAS staging file can be created.
        require_executable_binding(&input, &candidate.target, &verify_executable)?;
        verify_generation()?;

        let first_cas_write = Cell::new(true);
        let cas_write_guard = || -> Result<(), WorkingStoreError> {
            require_executable_binding(&input, &candidate.target, &verify_write_boundary).map_err(
                |error| {
                    WorkingStoreError::Invariant(format!(
                        "DataAsset executable CAS write guard failed: {error}"
                    ))
                },
            )?;
            if first_cas_write.replace(false) {
                verify_generation().map_err(|error| {
                    WorkingStoreError::Invariant(format!(
                        "DataAsset generation CAS write guard failed: {error}"
                    ))
                })?;
            }
            Ok(())
        };
        let mut deduplicated_blobs = 0usize;
        for (bytes, seal, limit, kind) in [
            (
                input.patched_uasset,
                manifest.patched_uasset(),
                package_limits.max_uasset_bytes,
                "DataAsset patched uasset",
            ),
            (
                input.patched_uexp,
                manifest.patched_uexp(),
                package_limits.max_uexp_bytes,
                "DataAsset patched uexp",
            ),
            (
                input.usmap,
                manifest.usmap(),
                MAX_USMAP_BYTES,
                "DataAsset USMAP",
            ),
        ] {
            deduplicated_blobs += usize::from(self.import_exact_dataasset_bytes_with_write_guard(
                bytes,
                seal,
                limit,
                kind,
                basis_head,
                cas_write_guard,
            )?);
        }
        for (role, source) in &input.sidecars {
            let seal = manifest.sidecars().get(role).ok_or_else(|| {
                Revision3DataAssetStagingErrorV1::VerifiedInput(
                    "verified sidecar disappeared from manifest".to_owned(),
                )
            })?;
            deduplicated_blobs += usize::from(self.import_exact_dataasset_bytes_with_write_guard(
                source,
                seal,
                MAX_OPTIONAL_SIDECAR_BYTES,
                "DataAsset sidecar",
                basis_head,
                cas_write_guard,
            )?);
        }
        deduplicated_blobs += usize::from(self.import_exact_dataasset_bytes_with_write_guard(
            manifest_json.as_bytes(),
            &manifest_asset,
            MAX_DATAASSET_FIXED_LEAF_STAGE_MANIFEST_BYTES_V1 as u64,
            "DataAsset stage manifest",
            basis_head,
            cas_write_guard,
        )?);

        require_executable_binding(&input, &candidate.target, &verify_executable)?;
        verify_generation()?;
        self.require_exact_head_for_dataasset(Some(basis_head))?;
        let checkpoint = self.prepare_revision3_checkpoint_with_write_guard(
            Some(basis_head),
            &candidate,
            || {
                require_executable_binding(&input, &candidate.target, &verify_write_boundary)
                    .map_err(|error| {
                        WorkingStoreError::Invariant(format!(
                            "DataAsset executable write guard failed: {error}"
                        ))
                    })
            },
        )?;
        let reopened =
            self.open_revision3_head_bytes(&checkpoint.head_bytes, AssetVerification::Full)?;
        if reopened.head != checkpoint.head || reopened.project != candidate {
            return Err(Revision3DataAssetStagingErrorV1::CandidateReopenMismatch);
        }
        let reopened_stages = self.load_revision3_dataasset_stages(&reopened.project)?;
        if !reopened_stages
            .iter()
            .any(|stage| stage.manifest_asset == manifest_asset && stage.manifest == manifest)
        {
            return Err(Revision3DataAssetStagingErrorV1::CandidateReopenMismatch);
        }
        require_executable_binding(&input, &candidate.target, &verify_executable)?;
        verify_generation()?;
        self.require_exact_head_for_dataasset(Some(basis_head))?;

        Ok(PreparedRevision3DataAssetStageV1 {
            basis_head: basis_head.clone(),
            project: candidate,
            canonical_project_json,
            checkpoint,
            stage: Revision3DataAssetStageViewV1 {
                manifest_asset,
                manifest,
            },
            deduplicated_blobs,
        })
    }

    /// List all fully verified managed stages against one exact currently published head.
    pub fn list_revision3_dataasset_stages_v1(
        &self,
        expected_head: &WorkingHead,
    ) -> Result<Vec<Revision3DataAssetStageViewV1>, Revision3DataAssetStagingErrorV1> {
        self.require_exact_head_for_dataasset(Some(expected_head))?;
        let opened = self.open_current_revision3(AssetVerification::Full)?;
        if &opened.head != expected_head {
            return Err(WorkingStoreError::HeadConflict {
                expected: Some(expected_head.clone()),
                actual: Some(opened.head),
            }
            .into());
        }
        let stages = self.load_revision3_dataasset_stages(&opened.project)?;
        self.require_exact_head_for_dataasset(Some(expected_head))?;
        Ok(stages)
    }

    /// Remove one managed-stage registry entry from a revision-3 candidate without deleting CAS
    /// objects and without publishing the prepared head.
    pub fn prepare_remove_revision3_dataasset_stage_v1(
        &self,
        basis_head: &WorkingHead,
        target_path: &str,
    ) -> Result<PreparedRevision3DataAssetStageRemovalV1, Revision3DataAssetStagingErrorV1> {
        gore_asset::dataasset_workflow::validate_game_asset_path(
            target_path,
            "DATAASSET_STAGE_REMOVE_TARGET",
        )
        .map_err(|error| Revision3DataAssetStagingErrorV1::ProjectBinding(error.to_string()))?;
        self.require_exact_head_for_dataasset(Some(basis_head))?;
        let opened = self.open_current_revision3(AssetVerification::Full)?;
        if &opened.head != basis_head {
            return Err(WorkingStoreError::HeadConflict {
                expected: Some(basis_head.clone()),
                actual: Some(opened.head),
            }
            .into());
        }
        let stages = self.load_revision3_dataasset_stages(&opened.project)?;
        let position = stages
            .iter()
            .position(|stage| stage.target_path().eq_ignore_ascii_case(target_path))
            .ok_or_else(|| DataAssetStageConflictV1::TargetNotStaged {
                target: target_path.to_owned(),
            })?;
        let removed = stages[position].clone();
        let mut candidate = opened.project;
        candidate
            .asset_store
            .assets
            .remove(&removed.manifest_asset.sha256);
        // Component AssetMeta may predate the stage or be shared by non-stage consumers. Without
        // persistent ownership evidence, removal is deliberately registry-only and keeps all
        // component metadata/CAS objects as verified, reusable orphans.
        candidate.revision = candidate
            .revision
            .checked_add(1)
            .ok_or(DataAssetStageConflictV1::ProjectRevisionOverflow)?;
        let canonical_project_json = candidate
            .to_canonical_json()
            .map_err(|error| Revision3DataAssetStagingErrorV1::ProjectBinding(error.to_string()))?;
        self.preflight_revision3_dataasset_candidate(&candidate)?;
        let checkpoint = self.prepare_revision3_checkpoint(Some(basis_head), &candidate)?;
        let reopened =
            self.open_revision3_head_bytes(&checkpoint.head_bytes, AssetVerification::Full)?;
        if reopened.head != checkpoint.head || reopened.project != candidate {
            return Err(Revision3DataAssetStagingErrorV1::CandidateReopenMismatch);
        }
        let reopened_stages = self.load_revision3_dataasset_stages(&reopened.project)?;
        if reopened_stages
            .iter()
            .any(|stage| stage.target_path().eq_ignore_ascii_case(target_path))
        {
            return Err(Revision3DataAssetStagingErrorV1::CandidateReopenMismatch);
        }
        self.require_exact_head_for_dataasset(Some(basis_head))?;
        Ok(PreparedRevision3DataAssetStageRemovalV1 {
            basis_head: basis_head.clone(),
            project: candidate,
            canonical_project_json,
            checkpoint,
            removed,
        })
    }

    fn load_revision3_dataasset_stages(
        &self,
        project: &ProjectRevision3,
    ) -> Result<Vec<Revision3DataAssetStageViewV1>, Revision3DataAssetStagingErrorV1> {
        let mut manifest_seals = Vec::new();
        let mut manifest_bytes = 0u64;
        for (digest, meta) in &project.asset_store.assets {
            if meta.media_type != DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1 {
                continue;
            }
            if manifest_seals.len() == MAX_DATAASSET_FIXED_LEAF_STAGES_V1 {
                return Err(stage_budget_error(
                    "stage count",
                    (manifest_seals.len() as u64).saturating_add(1),
                    MAX_DATAASSET_FIXED_LEAF_STAGES_V1 as u64,
                ));
            }
            manifest_bytes = manifest_bytes.checked_add(meta.byte_len).ok_or_else(|| {
                stage_budget_error(
                    "cumulative manifest bytes",
                    u64::MAX,
                    MAX_DATAASSET_STAGE_MANIFEST_BATCH_BYTES_V1,
                )
            })?;
            if manifest_bytes > MAX_DATAASSET_STAGE_MANIFEST_BATCH_BYTES_V1 {
                return Err(stage_budget_error(
                    "cumulative manifest bytes",
                    manifest_bytes,
                    MAX_DATAASSET_STAGE_MANIFEST_BATCH_BYTES_V1,
                ));
            }
            manifest_seals.push(ContentSeal {
                byte_len: meta.byte_len,
                sha256: *digest,
            });
        }

        let mut stages = Vec::with_capacity(manifest_seals.len());
        let mut targets = BTreeSet::new();
        let mut historical_bases = BTreeMap::<Sha256Digest, u64>::new();
        for seal in manifest_seals {
            let bytes = self.read_exact_dataasset_blob(
                &seal,
                MAX_DATAASSET_FIXED_LEAF_STAGE_MANIFEST_BYTES_V1,
                "DataAsset stage manifest",
            )?;
            let json = std::str::from_utf8(&bytes)
                .map_err(|_| DataAssetStageManifestErrorV1::NonUtf8Serialization)?;
            let manifest = Revision3DataAssetStageManifestV1::from_json(json)?;
            self.validate_manifest_index_for_project(&seal, &manifest, project)?;
            let folded = manifest.target_path.to_ascii_lowercase();
            if !targets.insert(folded) {
                return Err(DataAssetStageConflictV1::DuplicateTarget {
                    target: manifest.target_path.clone(),
                }
                .into());
            }
            match historical_bases.insert(
                manifest.basis_head.snapshot.sha256,
                manifest.basis_head.snapshot.byte_len,
            ) {
                Some(existing) if existing != manifest.basis_head.snapshot.byte_len => {
                    return Err(Revision3DataAssetStagingErrorV1::ProjectBinding(
                        "historical basis digest has conflicting byte lengths".to_owned(),
                    ));
                }
                _ => {}
            }
            stages.push(Revision3DataAssetStageViewV1 {
                manifest_asset: seal,
                manifest,
            });
        }

        if historical_bases.len() > MAX_DATAASSET_STAGE_HISTORICAL_BASES_V1 {
            return Err(stage_budget_error(
                "historical basis count",
                historical_bases.len() as u64,
                MAX_DATAASSET_STAGE_HISTORICAL_BASES_V1 as u64,
            ));
        }
        let historical_bytes = historical_bases.values().try_fold(0u64, |total, bytes| {
            total.checked_add(*bytes).ok_or_else(|| {
                stage_budget_error(
                    "cumulative historical basis bytes",
                    u64::MAX,
                    MAX_DATAASSET_STAGE_HISTORICAL_BASIS_BYTES_V1,
                )
            })
        })?;
        if historical_bytes > MAX_DATAASSET_STAGE_HISTORICAL_BASIS_BYTES_V1 {
            return Err(stage_budget_error(
                "cumulative historical basis bytes",
                historical_bytes,
                MAX_DATAASSET_STAGE_HISTORICAL_BASIS_BYTES_V1,
            ));
        }

        let mut basis_preflights = BTreeMap::new();
        let mut verification_objects = 0u64;
        let mut verification_bytes = 0u64;
        for (sha256, byte_len) in historical_bases {
            let seal = ContentSeal { byte_len, sha256 };
            let preflight = self.inspect_revision3_dataasset_basis(&seal)?;
            verification_objects = verification_objects
                .checked_add(preflight.verification_objects)
                .ok_or_else(|| {
                    stage_budget_error(
                        "historical full-verification objects",
                        u64::MAX,
                        MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1,
                    )
                })?;
            verification_bytes = verification_bytes
                .checked_add(preflight.verification_bytes)
                .ok_or_else(|| {
                    stage_budget_error(
                        "historical full-verification bytes",
                        u64::MAX,
                        MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1,
                    )
                })?;
            basis_preflights.insert(sha256, preflight);
        }
        if verification_objects > MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1 {
            return Err(stage_budget_error(
                "historical full-verification objects",
                verification_objects,
                MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1,
            ));
        }
        if verification_bytes > MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1 {
            return Err(stage_budget_error(
                "historical full-verification bytes",
                verification_bytes,
                MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1,
            ));
        }

        // Only after every unique basis has been charged against one aggregate budget do we allow
        // full entity/asset/voice/nested-basis I/O.
        let mut basis_identities = BTreeMap::new();
        for (sha256, preflight) in basis_preflights {
            let opened =
                self.open_revision3_snapshot(&preflight.head.snapshot, AssetVerification::Full)?;
            if opened.head != preflight.head
                || opened.project.project_id != preflight.project_id
                || opened.project.target != preflight.target
                || opened.project.revision != preflight.revision
            {
                return Err(Revision3DataAssetStagingErrorV1::ProjectBinding(
                    "historical basis changed between preflight and full reopen".to_owned(),
                ));
            }
            basis_identities.insert(
                sha256,
                (
                    opened.head,
                    opened.project.project_id,
                    opened.project.target,
                    opened.project.revision,
                ),
            );
        }
        for stage in &stages {
            let basis = basis_identities
                .get(&stage.manifest.basis_head.snapshot.sha256)
                .ok_or_else(|| {
                    Revision3DataAssetStagingErrorV1::ProjectBinding(
                        "historical basis identity disappeared".to_owned(),
                    )
                })?;
            if basis.0 != stage.manifest.basis_head
                || basis.1 != stage.manifest.project_id
                || basis.2 != stage.manifest.project_target
                || basis.3 != stage.manifest.basis_project_revision
            {
                return Err(Revision3DataAssetStagingErrorV1::ProjectBinding(
                    "basis snapshot does not match the bound project identity".to_owned(),
                ));
            }
        }
        stages.sort_by(|left, right| left.target_path().cmp(right.target_path()));
        Ok(stages)
    }

    fn validate_manifest_index_for_project(
        &self,
        manifest_asset: &ContentSeal,
        manifest: &Revision3DataAssetStageManifestV1,
        project: &ProjectRevision3,
    ) -> Result<(), Revision3DataAssetStagingErrorV1> {
        if manifest.project_id != project.project_id
            || manifest.project_target != project.target
            || manifest.staged_project_revision > project.revision
        {
            return Err(Revision3DataAssetStagingErrorV1::ProjectBinding(
                "project id, target, or revision disagrees".to_owned(),
            ));
        }
        let manifest_meta = project
            .asset_store
            .assets
            .get(&manifest_asset.sha256)
            .ok_or_else(|| {
                Revision3DataAssetStagingErrorV1::ProjectBinding(
                    "manifest is absent from AssetStore".to_owned(),
                )
            })?;
        if manifest_meta.byte_len != manifest_asset.byte_len
            || manifest_meta.media_type != DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1
        {
            return Err(Revision3DataAssetStagingErrorV1::ProjectBinding(
                "manifest AssetStore metadata differs".to_owned(),
            ));
        }
        for component in manifest.component_seals() {
            let meta = project
                .asset_store
                .assets
                .get(&component.sha256)
                .ok_or_else(|| {
                    Revision3DataAssetStagingErrorV1::ProjectBinding(format!(
                        "component {} is absent from AssetStore",
                        component.sha256
                    ))
                })?;
            if meta.byte_len != component.byte_len
                || meta.media_type != DATAASSET_FIXED_LEAF_COMPONENT_MEDIA_TYPE_V1
            {
                return Err(Revision3DataAssetStagingErrorV1::ProjectBinding(format!(
                    "component {} AssetStore metadata differs",
                    component.sha256
                )));
            }
        }
        Ok(())
    }
}

fn require_executable_binding<F>(
    input: &DataAssetStageProjectionV1<'_>,
    target: &GameGenerationAnchor,
    verify: &F,
) -> Result<(), Revision3DataAssetStagingErrorV1>
where
    F: Fn() -> Result<(), Revision3DataAssetStagingErrorV1>,
{
    if input.executable != target.executable {
        return Err(DataAssetStageConflictV1::ExecutableTargetMismatch.into());
    }
    verify()
}

fn stage_budget_error(
    resource: &'static str,
    actual: u64,
    limit: u64,
) -> Revision3DataAssetStagingErrorV1 {
    DataAssetStageConflictV1::StageBatchBudgetExceeded {
        resource,
        actual,
        limit,
    }
    .into()
}

fn preflight_candidate_stage_budgets(
    store: &WorkingProjectStore,
    existing: &[Revision3DataAssetStageViewV1],
    candidate_manifest: &ContentSeal,
    candidate_basis: &WorkingHead,
) -> Result<(), Revision3DataAssetStagingErrorV1> {
    let stage_count = existing.len().checked_add(1).ok_or_else(|| {
        stage_budget_error(
            "stage count",
            u64::MAX,
            MAX_DATAASSET_FIXED_LEAF_STAGES_V1 as u64,
        )
    })?;
    if stage_count > MAX_DATAASSET_FIXED_LEAF_STAGES_V1 {
        return Err(stage_budget_error(
            "stage count",
            stage_count as u64,
            MAX_DATAASSET_FIXED_LEAF_STAGES_V1 as u64,
        ));
    }

    let manifest_bytes =
        existing
            .iter()
            .try_fold(candidate_manifest.byte_len, |total, stage| {
                total
                    .checked_add(stage.manifest_asset.byte_len)
                    .ok_or_else(|| {
                        stage_budget_error(
                            "cumulative manifest bytes",
                            u64::MAX,
                            MAX_DATAASSET_STAGE_MANIFEST_BATCH_BYTES_V1,
                        )
                    })
            })?;
    if manifest_bytes > MAX_DATAASSET_STAGE_MANIFEST_BATCH_BYTES_V1 {
        return Err(stage_budget_error(
            "cumulative manifest bytes",
            manifest_bytes,
            MAX_DATAASSET_STAGE_MANIFEST_BATCH_BYTES_V1,
        ));
    }

    let mut historical_bases = BTreeMap::<Sha256Digest, u64>::new();
    for head in existing
        .iter()
        .map(|stage| &stage.manifest.basis_head)
        .chain(std::iter::once(candidate_basis))
    {
        match historical_bases.insert(head.snapshot.sha256, head.snapshot.byte_len) {
            Some(existing) if existing != head.snapshot.byte_len => {
                return Err(Revision3DataAssetStagingErrorV1::ProjectBinding(
                    "historical basis digest has conflicting byte lengths".to_owned(),
                ));
            }
            _ => {}
        }
    }
    if historical_bases.len() > MAX_DATAASSET_STAGE_HISTORICAL_BASES_V1 {
        return Err(stage_budget_error(
            "historical basis count",
            historical_bases.len() as u64,
            MAX_DATAASSET_STAGE_HISTORICAL_BASES_V1 as u64,
        ));
    }
    let historical_bytes = historical_bases.values().try_fold(0u64, |total, bytes| {
        total.checked_add(*bytes).ok_or_else(|| {
            stage_budget_error(
                "cumulative historical basis bytes",
                u64::MAX,
                MAX_DATAASSET_STAGE_HISTORICAL_BASIS_BYTES_V1,
            )
        })
    })?;
    if historical_bytes > MAX_DATAASSET_STAGE_HISTORICAL_BASIS_BYTES_V1 {
        return Err(stage_budget_error(
            "cumulative historical basis bytes",
            historical_bytes,
            MAX_DATAASSET_STAGE_HISTORICAL_BASIS_BYTES_V1,
        ));
    }

    let mut verification_objects = 0u64;
    let mut verification_bytes = 0u64;
    for (sha256, byte_len) in historical_bases {
        let preflight =
            store.inspect_revision3_dataasset_basis(&ContentSeal { byte_len, sha256 })?;
        verification_objects = verification_objects
            .checked_add(preflight.verification_objects)
            .ok_or_else(|| {
                stage_budget_error(
                    "historical full-verification objects",
                    u64::MAX,
                    MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1,
                )
            })?;
        verification_bytes = verification_bytes
            .checked_add(preflight.verification_bytes)
            .ok_or_else(|| {
                stage_budget_error(
                    "historical full-verification bytes",
                    u64::MAX,
                    MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1,
                )
            })?;
    }
    if verification_objects > MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1 {
        return Err(stage_budget_error(
            "historical full-verification objects",
            verification_objects,
            MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1,
        ));
    }
    if verification_bytes > MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1 {
        return Err(stage_budget_error(
            "historical full-verification bytes",
            verification_bytes,
            MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1,
        ));
    }
    Ok(())
}

fn validate_stage_intent(
    selector: &FixedLeafSelector,
    replacement_hex: &str,
    generation: &AssetGenerationReceipt,
) -> Result<(), DataAssetStageManifestErrorV1> {
    if selector.format != FIXED_LEAF_SELECTOR_FORMAT
        || selector.profile != FIXED_LEAF_SELECTOR_PROFILE
        || selector.path.is_empty()
        || selector.path.len() > 128
        || selector.object_name.is_empty()
        || selector.class_path.is_empty()
        || selector.role != FixedLeafRole::PropertyValue
        || selector.usmap_sha256 != generation.usmap.sha256
    {
        return Err(DataAssetStageManifestErrorV1::Invalid(
            "invalid or generation-unbound semantic selector".to_owned(),
        ));
    }
    digest_from_canonical_hex(&selector.usmap_sha256)?;
    digest_from_canonical_hex(&selector.export_sha256)?;
    let expected = selector.expected_bytes().map_err(|error| {
        DataAssetStageManifestErrorV1::Invalid(format!("invalid selector expected bytes: {error}"))
    })?;
    let replacement =
        decode_canonical_hex_bytes(replacement_hex, selector.kind.width(), "replacement_hex")?;
    if expected == replacement {
        return Err(DataAssetStageManifestErrorV1::Invalid(
            "semantic replacement makes no change".to_owned(),
        ));
    }
    match selector.kind {
        FixedWireKind::PackageIndex | FixedWireKind::FName => {
            return Err(DataAssetStageManifestErrorV1::Invalid(
                "referential fixed leaves are not editable".to_owned(),
            ));
        }
        FixedWireKind::Bool if expected[0] > 1 || replacement[0] > 1 => {
            return Err(DataAssetStageManifestErrorV1::Invalid(
                "Bool edits require canonical 0/1 bytes".to_owned(),
            ));
        }
        FixedWireKind::Bool
        | FixedWireKind::Byte
        | FixedWireKind::Int32
        | FixedWireKind::Float32
        | FixedWireKind::Float64
        | FixedWireKind::UInt64
        | FixedWireKind::UInt32
        | FixedWireKind::UInt16
        | FixedWireKind::Int64
        | FixedWireKind::Int16
        | FixedWireKind::Int8
        | FixedWireKind::LinearColorF32x4
        | FixedWireKind::Vector4F64x4 => {}
    }
    Ok(())
}

fn decode_canonical_hex_bytes(
    value: &str,
    expected_bytes: usize,
    field: &'static str,
) -> Result<Vec<u8>, DataAssetStageManifestErrorV1> {
    if value.len() != expected_bytes.saturating_mul(2)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DataAssetStageManifestErrorV1::Invalid(format!(
            "{field} is not canonical lowercase fixed-width hex"
        )));
    }
    Ok(value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect())
}

fn validate_seal(
    seal: &ContentSeal,
    max_bytes: u64,
    role: &'static str,
) -> Result<(), DataAssetStageManifestErrorV1> {
    if seal.byte_len == 0
        || seal.byte_len > max_bytes
        || seal.sha256.as_bytes().iter().all(|byte| *byte == 0)
    {
        return Err(DataAssetStageManifestErrorV1::Invalid(format!(
            "{role} seal has invalid byte length"
        )));
    }
    Ok(())
}

fn digest_from_canonical_hex(value: &str) -> Result<Sha256Digest, DataAssetStageManifestErrorV1> {
    if value.len() != 64
        || value != value.to_ascii_lowercase()
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(DataAssetStageManifestErrorV1::Invalid(
            "noncanonical SHA-256 hex".to_owned(),
        ));
    }
    let mut bytes = [0u8; 32];
    for (output, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        *output = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
    }
    Ok(Sha256Digest::from_bytes(bytes))
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("hex was validated before decoding"),
    }
}

fn seal_bytes(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn insert_asset_meta(
    project: &mut ProjectRevision3,
    seal: &ContentSeal,
    media_type: &str,
) -> Result<(), DataAssetStageConflictV1> {
    let expected = AssetMeta {
        byte_len: seal.byte_len,
        media_type: media_type.to_owned(),
    };
    match project.asset_store.assets.get(&seal.sha256) {
        Some(actual) if actual == &expected => Ok(()),
        Some(_) => Err(DataAssetStageConflictV1::AssetMetadataCollision { asset: seal.sha256 }),
        None => {
            project.asset_store.assets.insert(seal.sha256, expected);
            Ok(())
        }
    }
}

fn validate_candidate_asset_limits(
    project: &ProjectRevision3,
    store_limits: WorkingStoreLimits,
) -> Result<(), DataAssetStageConflictV1> {
    if project.asset_store.assets.len() > MAX_REVISION3_ASSETS
        || project.asset_store.assets.len() > store_limits.max_assets
    {
        return Err(DataAssetStageConflictV1::AssetCapacityExceeded);
    }
    let bytes = project
        .asset_store
        .assets
        .values()
        .try_fold(0u64, |sum, meta| sum.checked_add(meta.byte_len))
        .ok_or(DataAssetStageConflictV1::AssetBytesExceeded)?;
    if bytes > MAX_REVISION3_REFERENCED_ASSET_BYTES
        || bytes > store_limits.max_referenced_asset_bytes
    {
        return Err(DataAssetStageConflictV1::AssetBytesExceeded);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use gore_asset::dataasset_workflow::{GenerationChunkAnchor, GenerationFileAnchor};
    use gore_asset::{FixedLeafSelectorStep, FixedLeafWireType, PackagePairSeal};

    use crate::{AssetStoreIndex, FormatV2, ProjectMeta, SchemaRevisionV3};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gore-authoring-dataasset-{label}-{}-{sequence}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn publish(&self, bytes: &[u8]) {
            fs::write(self.0.join("gore-project.json"), bytes).unwrap();
        }

        fn asset_path(&self, digest: Sha256Digest) -> PathBuf {
            let digest = digest.to_string();
            self.0
                .join("assets/sha256")
                .join(&digest[..2])
                .join(&digest[2..])
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct ProjectionFixture {
        generation: AssetGenerationReceipt,
        selector: FixedLeafSelector,
        uasset: Vec<u8>,
        uexp: Vec<u8>,
        usmap: Vec<u8>,
        sidecar: Vec<u8>,
        executable: ContentSeal,
    }

    impl ProjectionFixture {
        fn new() -> Self {
            let uasset = b"managed patched uasset".to_vec();
            let uexp = b"managed patched uexp".to_vec();
            let usmap = b"managed exact usmap".to_vec();
            let sidecar = b"managed bulk sidecar".to_vec();
            let main = GenerationFileAnchor {
                file_name: "G1R-Windows.utoc".to_owned(),
                length: 64,
                sha256: "11".repeat(32),
            };
            let global_utoc = GenerationFileAnchor {
                file_name: "global.utoc".to_owned(),
                length: 32,
                sha256: "22".repeat(32),
            };
            let global_ucas = GenerationFileAnchor {
                file_name: "global.ucas".to_owned(),
                length: 96,
                sha256: "33".repeat(32),
            };
            let usmap_anchor = GenerationFileAnchor {
                file_name: "Mappings.usmap".to_owned(),
                length: usmap.len() as u64,
                sha256: hex_sha256(&usmap),
            };
            let chunk = |chunk_id: &str, chunk_type: &str| GenerationChunkAnchor {
                chunk_id: chunk_id.to_owned(),
                chunk_type: chunk_type.to_owned(),
                winner_utoc: main.clone(),
                length: 1,
                blake3: "a1".repeat(32),
                toc_hash: "b2".repeat(20),
                toc_hash_bytes: 20,
            };
            let generation = AssetGenerationReceipt {
                format: "gore.asset.generation.v1".to_owned(),
                asset: "/Game/TestAsset".to_owned(),
                usmap: usmap_anchor.clone(),
                main_utoc: main.clone(),
                global_utoc: global_utoc.clone(),
                global_ucas,
                container_set: vec![main.clone(), global_utoc],
                target_chunks: vec![
                    chunk("58becc37c6ec7b2000000001", "ContainerHeader"),
                    chunk("58becc37c6ec7b2000000002", "ExportBundleData"),
                    chunk("58becc37c6ec7b2000000003", "BulkData"),
                ],
            };
            validate_generation_receipt(&generation, "TEST_GENERATION").unwrap();
            let selector = FixedLeafSelector {
                format: FIXED_LEAF_SELECTOR_FORMAT,
                profile: FIXED_LEAF_SELECTOR_PROFILE.to_owned(),
                package_seal: PackagePairSeal {
                    uasset_sha256: [0x41; 32],
                    uexp_sha256: [0x42; 32],
                },
                usmap_sha256: usmap_anchor.sha256,
                export_index: 0,
                object_name: "TestAsset".to_owned(),
                class_path: "/Script/Test.Fixture".to_owned(),
                component: PackageComponent::Uexp,
                export_sha256: "55".repeat(32),
                role: FixedLeafRole::PropertyValue,
                kind: FixedWireKind::Bool,
                path: vec![FixedLeafSelectorStep::Property {
                    schema_index: 0,
                    property_name: "Enabled".to_owned(),
                    array_index: 0,
                    array_dimension: 1,
                    declaring_schema_name: "Fixture".to_owned(),
                    declaring_module_path: Some("/Script/Test".to_owned()),
                    property_type: FixedLeafWireType::Bool {},
                }],
                expected_hex: "01".to_owned(),
            };
            Self {
                generation,
                selector,
                uasset,
                uexp,
                usmap,
                sidecar,
                executable: ContentSeal {
                    byte_len: 17,
                    sha256: Sha256Digest::from_bytes([9; 32]),
                },
            }
        }

        fn projection(&self) -> DataAssetStageProjectionV1<'_> {
            DataAssetStageProjectionV1 {
                target_path: &self.generation.asset,
                generation: &self.generation,
                selector: &self.selector,
                replacement_hex: "00",
                patched_uasset: &self.uasset,
                patched_uexp: &self.uexp,
                usmap: &self.usmap,
                sidecars: vec![(SidecarRole::Bulk, &self.sidecar)],
                executable: self.executable.clone(),
            }
        }
    }

    fn hex_sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn project(executable: &ContentSeal) -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: ProjectId::from_bytes([7; 16]),
            revision: 4,
            meta: ProjectMeta {
                name: "DataAsset stage test".to_owned(),
                version: "0.1.0".to_owned(),
                author: "tests".to_owned(),
            },
            target: GameGenerationAnchor {
                executable: executable.clone(),
            },
            authoring_locales: BTreeSet::new(),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    fn prepare_projection(
        store: &WorkingProjectStore,
        basis: &WorkingHead,
        fixture: &ProjectionFixture,
    ) -> Result<PreparedRevision3DataAssetStageV1, Revision3DataAssetStagingErrorV1> {
        store.prepare_revision3_dataasset_stage_projection_v1(
            basis,
            fixture.projection(),
            || Ok(()),
            || Ok(()),
            || Ok(()),
        )
    }

    #[test]
    fn projection_prepare_list_dedupe_duplicate_and_registry_only_remove() {
        let root = TestRoot::new("roundtrip");
        let store = WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap();
        let fixture = ProjectionFixture::new();
        let basis_project = project(&fixture.executable);
        let basis = store
            .prepare_revision3_checkpoint(None, &basis_project)
            .unwrap();
        root.publish(&basis.head_bytes);

        let first = prepare_projection(&store, &basis.head, &fixture).unwrap();
        assert_eq!(store.current_head().unwrap(), Some(basis.head.clone()));
        assert_eq!(first.project().revision, basis_project.revision + 1);
        assert_eq!(first.stage().target_path(), "/Game/TestAsset");
        assert_eq!(
            first.stage().manifest().build_status(),
            DataAssetStageBuildStatusV1::Blocked
        );
        assert_eq!(
            first.stage().manifest().runtime_status(),
            DataAssetStageRuntimeStatusV1::RuntimeUnqualified
        );
        assert_eq!(
            first.stage().manifest().artifact_authority(),
            DataAssetStageArtifactAuthorityV1::NotGranted
        );
        assert_eq!(
            first.stage().manifest().publication_status(),
            DataAssetStagePublicationStatusV1::NotSupported
        );
        assert_eq!(first.stage().manifest().sidecars().len(), 1);

        let manifest_json = first.stage().manifest().to_canonical_json().unwrap();
        for forbidden in [
            "absolute_offset",
            "patch_receipt",
            "extract_receipt",
            "receipt_path",
            "source_path",
        ] {
            assert!(!manifest_json.contains(forbidden), "found {forbidden}");
        }
        assert_eq!(
            Revision3DataAssetStageManifestV1::from_json(&manifest_json).unwrap(),
            *first.stage().manifest()
        );
        for digest in first.project().asset_store.assets.keys() {
            let bytes = fs::read(root.asset_path(*digest)).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            assert!(!text.contains("patch_receipt"));
            assert!(!text.contains("extract_receipt"));
            assert!(!text.contains("absolute_offset"));
        }

        let second = prepare_projection(&store, &basis.head, &fixture).unwrap();
        assert_eq!(first.checkpoint(), second.checkpoint());
        assert_eq!(first.project(), second.project());
        assert_eq!(
            second.deduplicated_blobs(),
            first.project().asset_store.assets.len()
        );

        root.publish(&first.checkpoint().head_bytes);
        let listed = store
            .list_revision3_dataasset_stages_v1(&first.checkpoint().head)
            .unwrap();
        assert_eq!(listed, vec![first.stage().clone()]);
        assert!(matches!(
            prepare_projection(&store, &first.checkpoint().head, &fixture),
            Err(Revision3DataAssetStagingErrorV1::Conflict(
                DataAssetStageConflictV1::TargetAlreadyStaged { .. }
            ))
        ));

        let component_seals: Vec<_> = first.stage().manifest().component_seals().collect();
        let removal = store
            .prepare_remove_revision3_dataasset_stage_v1(
                &first.checkpoint().head,
                "/Game/testasset",
            )
            .unwrap();
        assert!(!removal
            .project()
            .asset_store
            .assets
            .contains_key(&first.stage().manifest_asset().sha256));
        for component in component_seals {
            assert!(removal
                .project()
                .asset_store
                .assets
                .contains_key(&component.sha256));
            assert!(root.asset_path(component.sha256).exists());
        }
        assert!(root
            .asset_path(first.stage().manifest_asset().sha256)
            .exists());
        root.publish(&removal.checkpoint().head_bytes);
        assert!(store
            .list_revision3_dataasset_stages_v1(&removal.checkpoint().head)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn executable_mismatch_and_invalid_remove_target_fail_before_writes() {
        let root = TestRoot::new("executable-mismatch");
        let store = WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap();
        let fixture = ProjectionFixture::new();
        let mut basis_project = project(&fixture.executable);
        basis_project.target.executable.sha256 = Sha256Digest::from_bytes([0x77; 32]);
        let basis = store
            .prepare_revision3_checkpoint(None, &basis_project)
            .unwrap();
        root.publish(&basis.head_bytes);

        assert!(matches!(
            prepare_projection(&store, &basis.head, &fixture),
            Err(Revision3DataAssetStagingErrorV1::Conflict(
                DataAssetStageConflictV1::ExecutableTargetMismatch
            ))
        ));
        assert_eq!(store.current_head().unwrap(), Some(basis.head.clone()));
        assert!(!root.path().join("assets").exists());
        assert!(store
            .prepare_remove_revision3_dataasset_stage_v1(&basis.head, "x".repeat(4096).as_str())
            .is_err());
    }

    #[test]
    fn full_historical_basis_reopen_detects_an_unreferenced_current_asset_loss() {
        let root = TestRoot::new("historical-full");
        let store = WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap();
        let fixture = ProjectionFixture::new();
        let historical_bytes = b"historical basis-only asset";
        let historical = seal_bytes(historical_bytes);
        let historical_path = root.asset_path(historical.sha256);
        fs::create_dir_all(historical_path.parent().unwrap()).unwrap();
        fs::write(&historical_path, historical_bytes).unwrap();
        let mut basis_project = project(&fixture.executable);
        basis_project.asset_store.assets.insert(
            historical.sha256,
            AssetMeta {
                byte_len: historical.byte_len,
                media_type: "application/octet-stream".to_owned(),
            },
        );
        let basis = store
            .prepare_revision3_checkpoint(None, &basis_project)
            .unwrap();
        root.publish(&basis.head_bytes);
        let staged = prepare_projection(&store, &basis.head, &fixture).unwrap();
        root.publish(&staged.checkpoint().head_bytes);

        let mut current_project = staged.project().clone();
        current_project.revision += 1;
        current_project
            .asset_store
            .assets
            .remove(&historical.sha256);
        let current = store
            .prepare_revision3_checkpoint(Some(&staged.checkpoint().head), &current_project)
            .unwrap();
        root.publish(&current.head_bytes);
        fs::remove_file(&historical_path).unwrap();
        store
            .open_current_revision3(AssetVerification::Full)
            .unwrap();
        assert!(store
            .list_revision3_dataasset_stages_v1(&current.head)
            .is_err());
    }

    #[test]
    fn candidate_registry_budgets_reject_count_batch_and_unique_bases_without_io() {
        let root = TestRoot::new("candidate-budgets");
        let store = WorkingProjectStore::at(root.path(), WorkingStoreLimits::default()).unwrap();
        let fixture = ProjectionFixture::new();
        let basis_project = project(&fixture.executable);
        let basis = store
            .prepare_revision3_checkpoint(None, &basis_project)
            .unwrap();
        root.publish(&basis.head_bytes);
        let staged = prepare_projection(&store, &basis.head, &fixture).unwrap();
        let view = staged.stage().clone();

        let count = vec![view.clone(); MAX_DATAASSET_FIXED_LEAF_STAGES_V1];
        assert!(preflight_candidate_stage_budgets(
            &store,
            &count,
            view.manifest_asset(),
            &basis.head,
        )
        .is_err());

        let mut large = view.clone();
        large.manifest_asset.byte_len = MAX_DATAASSET_STAGE_MANIFEST_BATCH_BYTES_V1;
        assert!(preflight_candidate_stage_budgets(
            &store,
            &[large],
            view.manifest_asset(),
            &basis.head,
        )
        .is_err());

        let mut unique = Vec::new();
        for index in 0..MAX_DATAASSET_STAGE_HISTORICAL_BASES_V1 {
            let mut distinct = view.clone();
            let mut digest = [0u8; 32];
            digest[..8].copy_from_slice(&(index as u64 + 1).to_le_bytes());
            distinct.manifest.basis_head.snapshot.sha256 = Sha256Digest::from_bytes(digest);
            unique.push(distinct);
        }
        assert!(preflight_candidate_stage_budgets(
            &store,
            &unique,
            view.manifest_asset(),
            &basis.head,
        )
        .is_err());
    }
}
