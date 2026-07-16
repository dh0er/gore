//! Canonical, path-free evidence for one managed reviewed DataAsset build.
//!
//! A receipt is deliberately not a capability. Parsing one never grants filesystem access,
//! publication, deployment, or runtime support; it only revalidates a closed evidence envelope.

use std::collections::BTreeSet;

use gore_asset::dataasset_workflow::{
    asset_package_limits, validate_generation_receipt, AssetGenerationReceipt,
    MAX_CONTAINER_COMPONENT_BYTES, MAX_GAME_EXECUTABLE_BYTES, MAX_USMAP_BYTES,
};
use gore_asset::{
    prepare_reviewed_footstep_preset_size_v1, reviewed_footstep_preset_target_from_ids_v1,
    FixedLeafSelector, PackagePairSeal, ReviewedFootstepPresetSizeV1,
    VerifiedManagedReviewedTripletPostPackV1, FIXED_LEAF_SELECTOR_FORMAT,
    FIXED_LEAF_SELECTOR_PROFILE, REVIEWED_DATAASSET_FORMAT_V1, REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID,
    REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID, REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    ContentSeal, GameGenerationAnchor, ProjectId, Sha256Digest,
    VerifiedCurrentReviewedDataAssetStageSourceV1, WorkingHead,
};

pub const MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FORMAT_V1: &str =
    "gore.authoring.managed-revision3-reviewed-dataasset-build-receipt.v1";
pub const MAX_MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_BYTES_V1: usize = 8 * 1024 * 1024;

const MAX_SELECTOR_JSON_BYTES: usize = 256 * 1024;
const MAX_PACK_NAME_BYTES: usize = 96;
const MAX_READBACK_SOURCES: usize = 256;
const MAX_READBACK_CHUNKS: usize = 4096;
// Mirrors gore-tex container's private per-chunk and aggregate snapshot bounds. Keeping the
// spelling local makes this path-free wire validator independent of a filesystem capability.
const MAX_VERIFIED_IOSTORE_SNAPSHOT_CHUNK_BYTES: u64 = 512 * 1024 * 1024;
const MAX_VERIFIED_IOSTORE_SNAPSHOT_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
// A generation probe and the complete primary readback each originate in one bounded snapshot.
const MAX_GENERATION_TARGET_CHUNK_BYTES: u64 = MAX_VERIFIED_IOSTORE_SNAPSHOT_TOTAL_BYTES;
const MAX_PRIMARY_READBACK_CHUNK_BYTES: u64 = MAX_VERIFIED_IOSTORE_SNAPSHOT_TOTAL_BYTES;
// The readback union merges at most complete-primary, fallback-metadata, and routed-conversion
// snapshots, each independently bounded by gore-tex before this projection is constructed.
const MAX_READBACK_CHUNK_BYTES: u64 = 3 * MAX_VERIFIED_IOSTORE_SNAPSHOT_TOTAL_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDataAssetBuildAuthorityV1 {
    ReviewedFixedLeafSinglePackageTriplet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDataAssetPublicationAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDataAssetRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReadbackSourceRoleProjectionV1 {
    Primary,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagedBuildBasisProjectionV1 {
    current_head: WorkingHead,
    project_id: ProjectId,
    project_revision: u64,
    executable: ContentSeal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectorProjectionV1 {
    canonical_json: String,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagedStageProjectionV1 {
    manifest_asset: ContentSeal,
    project_id: ProjectId,
    project_target: GameGenerationAnchor,
    basis_head: WorkingHead,
    basis_project_revision: u64,
    staged_project_revision: u64,
    target_path: String,
    generation: AssetGenerationReceipt,
    selector: SelectorProjectionV1,
    replacement_hex: String,
    patched_uasset: ContentSeal,
    patched_uexp: ContentSeal,
    usmap: ContentSeal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestedSizeProjectionV1 {
    x_f64_bits: u64,
    y_f64_bits: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewedIntentProjectionV1 {
    format: u32,
    schema_id: String,
    schema_revision: u32,
    field_id: String,
    target_id: String,
    target_path: String,
    requested: RequestedSizeProjectionV1,
    selector_sha256: String,
    replacement_hex: String,
    binding_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackagePairProjectionV1 {
    uasset_sha256: String,
    uexp_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadbackSourceSealProjectionV1 {
    role: ReadbackSourceRoleProjectionV1,
    utoc_blake3: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadbackChunkSealProjectionV1 {
    source_role: ReadbackSourceRoleProjectionV1,
    source_utoc_blake3: String,
    chunk_id: String,
    chunk_type: String,
    length: u64,
    blake3: String,
    toc_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewedPostPackProjectionV1 {
    target_id: String,
    requested: RequestedSizeProjectionV1,
    replacement_hex: String,
    reviewed_binding_sha256: String,
    package: PackagePairProjectionV1,
    usmap_sha256: String,
    fresh_selector: SelectorProjectionV1,
    source_seals: Vec<ReadbackSourceSealProjectionV1>,
    chunk_seals: Vec<ReadbackChunkSealProjectionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagedDataAssetTripletFileSealV1 {
    relative_name: String,
    byte_len: u64,
    sha256: Sha256Digest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagedDataAssetTripletFileSealWireV1 {
    relative_name: String,
    byte_len: u64,
    sha256: Sha256Digest,
}

impl<'de> Deserialize<'de> for ManagedDataAssetTripletFileSealV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ManagedDataAssetTripletFileSealWireV1::deserialize(deserializer)?;
        Self::try_new(wire.relative_name, wire.byte_len, wire.sha256)
            .map_err(serde::de::Error::custom)
    }
}

impl ManagedDataAssetTripletFileSealV1 {
    pub fn try_new(
        relative_name: impl Into<String>,
        byte_len: u64,
        sha256: Sha256Digest,
    ) -> Result<Self, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        let value = Self {
            relative_name: relative_name.into(),
            byte_len,
            sha256,
        };
        validate_relative_triplet_file(&value)?;
        Ok(value)
    }

    pub fn relative_name(&self) -> &str {
        &self.relative_name
    }

    pub const fn byte_len(&self) -> u64 {
        self.byte_len
    }

    pub const fn sha256(&self) -> Sha256Digest {
        self.sha256
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagedBuildOutputProjectionV1 {
    pack_name: String,
    files: Vec<ManagedDataAssetTripletFileSealV1>,
}

/// Path-free authoring basis for one exact-current reviewed revision-3 DataAsset build.
///
/// Construction consumes the Store-backed stage source. Only the small, already-verified
/// basis/stage/review projections survive; the source's package and USMAP byte buffers are dropped
/// before this value is returned. The value has no `Clone`, serialization, public fields, raw-parts
/// constructor, publication authority, or filesystem-path API.
#[derive(Debug)]
pub struct VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1 {
    basis: ManagedBuildBasisProjectionV1,
    stage: ManagedStageProjectionV1,
    reviewed: ReviewedIntentProjectionV1,
}

impl VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1 {
    pub fn from_current_source(
        source: VerifiedCurrentReviewedDataAssetStageSourceV1,
    ) -> Result<Self, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        let value = {
            let stage = source.stage();
            let manifest = stage.manifest();
            if !manifest.sidecars().is_empty() {
                return Err(invalid(
                    "reviewed managed build receipts refuse staged sidecars",
                ));
            }
            let reviewed = source.reviewed();
            let selector = SelectorProjectionV1::capture(reviewed.selector())?;

            Self {
                basis: ManagedBuildBasisProjectionV1 {
                    current_head: source.current_head().clone(),
                    project_id: source.project_id(),
                    project_revision: source.project_revision(),
                    executable: source.project_target().executable.clone(),
                },
                stage: ManagedStageProjectionV1 {
                    manifest_asset: stage.manifest_asset().clone(),
                    project_id: manifest.project_id(),
                    project_target: manifest.project_target().clone(),
                    basis_head: manifest.basis_head().clone(),
                    basis_project_revision: manifest.basis_project_revision(),
                    staged_project_revision: manifest.staged_project_revision(),
                    target_path: manifest.target_path().to_owned(),
                    generation: manifest.generation().clone(),
                    selector: selector.clone(),
                    replacement_hex: manifest.replacement_hex().to_owned(),
                    patched_uasset: manifest.patched_uasset().clone(),
                    patched_uexp: manifest.patched_uexp().clone(),
                    usmap: manifest.usmap().clone(),
                },
                reviewed: ReviewedIntentProjectionV1 {
                    format: reviewed.format(),
                    schema_id: reviewed.schema_id().to_owned(),
                    schema_revision: reviewed.schema_revision(),
                    field_id: reviewed.field_id().to_owned(),
                    target_id: reviewed.target().id().to_owned(),
                    target_path: reviewed.target().target_path().to_owned(),
                    requested: RequestedSizeProjectionV1::capture(reviewed.requested()),
                    selector_sha256: selector.sha256,
                    replacement_hex: encode_hex(reviewed.replacement_bytes()),
                    binding_sha256: encode_hex(reviewed.binding_sha256()),
                },
            }
        };

        // Make the lifetime boundary explicit: none of the large CAS buffers survives in `value`.
        drop(source);
        Ok(value)
    }
}

#[derive(Debug)]
struct ManagedVerifiedTripletPostPackProjectionV1 {
    target_path: String,
    generation: AssetGenerationReceipt,
    executable_length: u64,
    executable_sha256: [u8; 32],
    replay_seal: PackagePairSeal,
    post_pack: ReviewedPostPackProjectionV1,
    output: ManagedBuildOutputProjectionV1,
}

impl ManagedVerifiedTripletPostPackProjectionV1 {
    fn capture(
        proof: &VerifiedManagedReviewedTripletPostPackV1,
    ) -> Result<Self, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        let post_pack = proof.post_pack();
        let mut source_seals = Vec::with_capacity(post_pack.source_seals().len());
        for seal in post_pack.source_seals() {
            source_seals.push(ReadbackSourceSealProjectionV1 {
                role: project_source_role(seal.role())?,
                utoc_blake3: encode_hex(seal.utoc_blake3()),
            });
        }
        source_seals.sort();

        let mut chunk_seals = Vec::with_capacity(post_pack.chunk_seals().len());
        for seal in post_pack.chunk_seals() {
            chunk_seals.push(ReadbackChunkSealProjectionV1 {
                source_role: project_source_role(seal.source_role())?,
                source_utoc_blake3: encode_hex(seal.source_utoc_blake3()),
                chunk_id: encode_hex(seal.chunk_id()),
                chunk_type: seal.chunk_type().to_owned(),
                length: seal.length(),
                blake3: encode_hex(seal.blake3()),
                toc_hash: encode_hex(seal.toc_hash()),
            });
        }
        chunk_seals.sort();

        let files = proof
            .triplet_files()
            .iter()
            .map(|file| {
                ManagedDataAssetTripletFileSealV1::try_new(
                    file.relative_name(),
                    file.byte_len(),
                    Sha256Digest::from_bytes(*file.sha256()),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            target_path: proof.target_path().to_owned(),
            generation: proof.generation().clone(),
            executable_length: proof.executable_length(),
            executable_sha256: *proof.executable_sha256(),
            replay_seal: proof.replay_seal().clone(),
            post_pack: ReviewedPostPackProjectionV1 {
                target_id: post_pack.target().id().to_owned(),
                requested: RequestedSizeProjectionV1::capture(post_pack.requested()),
                replacement_hex: encode_hex(post_pack.replacement_bytes()),
                reviewed_binding_sha256: encode_hex(post_pack.reviewed_binding_sha256()),
                package: PackagePairProjectionV1::capture(post_pack.package_seal()),
                usmap_sha256: encode_hex(post_pack.usmap_sha256()),
                fresh_selector: SelectorProjectionV1::capture(post_pack.fresh_selector())?,
                source_seals,
                chunk_seals,
            },
            output: ManagedBuildOutputProjectionV1 {
                pack_name: proof.pack_name().to_owned(),
                files,
            },
        })
    }
}

fn verify_proof_binding(
    basis: &VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1,
    proof: &ManagedVerifiedTripletPostPackProjectionV1,
) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
    if proof.target_path != basis.stage.target_path
        || proof.target_path != basis.reviewed.target_path
        || proof.generation != basis.stage.generation
    {
        return Err(invalid(
            "managed build proof target or generation differs from the authoring basis",
        ));
    }
    if proof.executable_length != basis.basis.executable.byte_len
        || proof.executable_sha256 != *basis.basis.executable.sha256.as_bytes()
    {
        return Err(invalid(
            "managed build proof executable differs from the authoring basis",
        ));
    }

    let expected_package = PackagePairSeal {
        uasset_sha256: *basis.stage.patched_uasset.sha256.as_bytes(),
        uexp_sha256: *basis.stage.patched_uexp.sha256.as_bytes(),
    };
    if proof.replay_seal != expected_package {
        return Err(invalid(
            "managed build proof replay differs from the staged package",
        ));
    }

    if proof.post_pack.target_id != basis.reviewed.target_id
        || proof.post_pack.requested != basis.reviewed.requested
        || proof.post_pack.replacement_hex != basis.reviewed.replacement_hex
        || proof.post_pack.reviewed_binding_sha256 != basis.reviewed.binding_sha256
    {
        return Err(invalid(
            "managed post-pack review facts differ from the authoring basis",
        ));
    }
    if proof.post_pack.package.to_package_pair()? != proof.replay_seal
        || proof.replay_seal != expected_package
    {
        return Err(invalid(
            "managed post-pack package differs from the verified replay",
        ));
    }
    if proof.post_pack.usmap_sha256 != basis.stage.usmap.sha256.to_string()
        || basis.stage.usmap.byte_len != basis.stage.generation.usmap.length
        || basis.stage.usmap.sha256.to_string() != basis.stage.generation.usmap.sha256
    {
        return Err(invalid(
            "managed post-pack USMAP differs from the authoring basis",
        ));
    }

    let original = basis.stage.selector.open()?;
    let fresh = proof.post_pack.fresh_selector.open()?;
    if fresh.package_seal != expected_package
        || fresh.usmap_sha256 != basis.stage.usmap.sha256.to_string()
        || fresh.object_name != original.object_name
        || fresh.class_path != original.class_path
        || fresh.export_index != original.export_index
        || fresh.component != original.component
        || fresh.role != original.role
        || fresh.kind != original.kind
        || fresh.path != original.path
        || fresh.expected_hex != basis.reviewed.replacement_hex
    {
        return Err(invalid(
            "managed post-pack selector differs from the reviewed authoring basis",
        ));
    }

    validate_pack_name(&proof.output.pack_name)?;
    validate_output_triplet(&proof.output.pack_name, &proof.output.files)?;
    Ok(())
}

/// Canonical evidence for exactly one reviewed revision-3 DataAsset package triplet.
///
/// The private fields and closed enums prevent callers from widening the claim. `from_json`
/// authenticates no producer and grants no authority beyond the bounded claim recorded here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedRevision3ReviewedDataAssetBuildReceiptV1 {
    format: String,
    basis: ManagedBuildBasisProjectionV1,
    stage: ManagedStageProjectionV1,
    reviewed: ReviewedIntentProjectionV1,
    post_pack: ReviewedPostPackProjectionV1,
    output: ManagedBuildOutputProjectionV1,
    build_authority: ManagedDataAssetBuildAuthorityV1,
    publication_authority: ManagedDataAssetPublicationAuthorityV1,
    deployed: bool,
    runtime_status: ManagedDataAssetRuntimeStatusV1,
}

/// Deserialization is deliberately confined to the validating [`from_json`](
/// ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json) boundary.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManagedRevision3ReviewedDataAssetBuildReceiptWireV1 {
    format: String,
    basis: ManagedBuildBasisProjectionV1,
    stage: ManagedStageProjectionV1,
    reviewed: ReviewedIntentProjectionV1,
    post_pack: ReviewedPostPackProjectionV1,
    output: ManagedBuildOutputProjectionV1,
    build_authority: ManagedDataAssetBuildAuthorityV1,
    publication_authority: ManagedDataAssetPublicationAuthorityV1,
    deployed: bool,
    runtime_status: ManagedDataAssetRuntimeStatusV1,
}

impl ManagedRevision3ReviewedDataAssetBuildReceiptV1 {
    /// Seal one reviewed build receipt from the exact authoring basis and gore-asset's opaque
    /// triplet/post-pack proof.
    ///
    /// The output seals and post-pack evidence cannot be supplied independently. Consuming the
    /// basis also prevents it from being reused to bless more than one build proof.
    pub fn from_verified(
        basis: VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1,
        proof: &VerifiedManagedReviewedTripletPostPackV1,
    ) -> Result<Self, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        let proof = ManagedVerifiedTripletPostPackProjectionV1::capture(proof)?;
        verify_proof_binding(&basis, &proof)?;

        let receipt = Self {
            format: MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FORMAT_V1.to_owned(),
            basis: basis.basis,
            stage: basis.stage,
            reviewed: basis.reviewed,
            post_pack: proof.post_pack,
            output: proof.output,
            build_authority:
                ManagedDataAssetBuildAuthorityV1::ReviewedFixedLeafSinglePackageTriplet,
            publication_authority: ManagedDataAssetPublicationAuthorityV1::NotGranted,
            deployed: false,
            runtime_status: ManagedDataAssetRuntimeStatusV1::RuntimeUnqualified,
        };
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn from_json(
        json: &str,
    ) -> Result<Self, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        if json.len() > MAX_MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_BYTES_V1 {
            return Err(
                ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::InputTooLarge {
                    actual: json.len(),
                    limit: MAX_MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_BYTES_V1,
                },
            );
        }
        reject_duplicate_object_keys(json)
            .map_err(ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::InvalidJson)?;
        let wire: ManagedRevision3ReviewedDataAssetBuildReceiptWireV1 = serde_json::from_str(json)
            .map_err(ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::InvalidJson)?;
        let receipt = Self {
            format: wire.format,
            basis: wire.basis,
            stage: wire.stage,
            reviewed: wire.reviewed,
            post_pack: wire.post_pack,
            output: wire.output,
            build_authority: wire.build_authority,
            publication_authority: wire.publication_authority,
            deployed: wire.deployed,
            runtime_status: wire.runtime_status,
        };
        receipt.validate()?;
        if receipt.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::NonCanonicalJson);
        }
        Ok(receipt)
    }

    pub fn to_canonical_json(
        &self,
    ) -> Result<String, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Serialize)?;
        if bytes.len() > MAX_MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_BYTES_V1 {
            return Err(
                ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::InputTooLarge {
                    actual: bytes.len(),
                    limit: MAX_MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_BYTES_V1,
                },
            );
        }
        String::from_utf8(bytes)
            .map_err(|_| ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::NonUtf8Serialization)
    }

    pub fn current_head(&self) -> &WorkingHead {
        &self.basis.current_head
    }

    pub const fn project_id(&self) -> ProjectId {
        self.basis.project_id
    }

    pub const fn project_revision(&self) -> u64 {
        self.basis.project_revision
    }

    pub fn target_path(&self) -> &str {
        &self.stage.target_path
    }

    pub fn pack_name(&self) -> &str {
        &self.output.pack_name
    }

    pub fn files(&self) -> &[ManagedDataAssetTripletFileSealV1] {
        &self.output.files
    }

    pub const fn build_authority(&self) -> ManagedDataAssetBuildAuthorityV1 {
        self.build_authority
    }

    pub const fn publication_authority(&self) -> ManagedDataAssetPublicationAuthorityV1 {
        self.publication_authority
    }

    pub const fn deployed(&self) -> bool {
        self.deployed
    }

    pub const fn runtime_status(&self) -> ManagedDataAssetRuntimeStatusV1 {
        self.runtime_status
    }

    fn validate(&self) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        if self.format != MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FORMAT_V1 {
            return Err(invalid("unsupported receipt format"));
        }
        validate_content_seal(&self.basis.current_head.snapshot, "current head snapshot")?;
        validate_content_seal(&self.basis.executable, "executable")?;
        if self
            .basis
            .project_id
            .as_bytes()
            .iter()
            .all(|byte| *byte == 0)
        {
            return Err(invalid("zero project id"));
        }
        validate_content_seal(&self.stage.manifest_asset, "stage manifest asset")?;
        validate_content_seal(&self.stage.basis_head.snapshot, "stage basis snapshot")?;
        validate_content_seal(&self.stage.project_target.executable, "stage executable")?;
        validate_content_seal(&self.stage.patched_uasset, "staged uasset")?;
        validate_content_seal(&self.stage.patched_uexp, "staged uexp")?;
        validate_content_seal(&self.stage.usmap, "stage USMAP")?;
        if self.basis.current_head.snapshot.byte_len > crate::MAX_REVISION3_SNAPSHOT_BYTES
            || self.stage.basis_head.snapshot.byte_len > crate::MAX_REVISION3_SNAPSHOT_BYTES
        {
            return Err(invalid(
                "revision-3 snapshot seal exceeds its closed byte limit",
            ));
        }
        if self.basis.executable.byte_len > MAX_GAME_EXECUTABLE_BYTES
            || self.stage.project_target.executable.byte_len > MAX_GAME_EXECUTABLE_BYTES
        {
            return Err(invalid(
                "game executable seal exceeds its closed byte limit",
            ));
        }
        let package_limits = asset_package_limits();
        if self.stage.manifest_asset.byte_len
            > crate::MAX_DATAASSET_FIXED_LEAF_STAGE_MANIFEST_BYTES_V1 as u64
            || self.stage.patched_uasset.byte_len > package_limits.max_uasset_bytes
            || self.stage.patched_uexp.byte_len > package_limits.max_uexp_bytes
            || self
                .stage
                .patched_uasset
                .byte_len
                .checked_add(self.stage.patched_uexp.byte_len)
                .is_none_or(|total| total > package_limits.max_total_bytes)
            || self.stage.usmap.byte_len > MAX_USMAP_BYTES
        {
            return Err(invalid("staged content seal exceeds its closed byte limit"));
        }
        if self.stage.project_id != self.basis.project_id
            || self.stage.project_target.executable != self.basis.executable
            || self.stage.staged_project_revision != self.basis.project_revision
            || self
                .stage
                .basis_project_revision
                .checked_add(1)
                .is_none_or(|revision| revision != self.stage.staged_project_revision)
        {
            return Err(invalid("stage differs from exact current project basis"));
        }
        validate_generation_anchor_lengths(&self.stage.generation)?;
        validate_generation_receipt(&self.stage.generation, "MANAGED_BUILD_RECEIPT")
            .map_err(|error| invalid(error.to_string()))?;
        if self.stage.target_path != self.stage.generation.asset
            || self.stage.target_path != self.reviewed.target_path
        {
            return Err(invalid("stage, generation, and reviewed targets differ"));
        }
        if self.stage.usmap.byte_len != self.stage.generation.usmap.length
            || self.stage.usmap.sha256.to_string() != self.stage.generation.usmap.sha256
        {
            return Err(invalid("stage USMAP differs from generation"));
        }

        let selector = self.stage.selector.open()?;
        if self.reviewed.selector_sha256 != self.stage.selector.sha256 {
            return Err(invalid("review binding names a different selector"));
        }
        if self.reviewed.format != REVIEWED_DATAASSET_FORMAT_V1
            || self.reviewed.schema_id != REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID
            || self.reviewed.schema_revision != REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION
            || self.reviewed.field_id != REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID
        {
            return Err(invalid(
                "reviewed schema identity is not the closed V1 identity",
            ));
        }
        let target = reviewed_footstep_preset_target_from_ids_v1(
            self.reviewed.format,
            &self.reviewed.schema_id,
            self.reviewed.schema_revision,
            &self.reviewed.field_id,
            &self.reviewed.target_id,
        )
        .map_err(|error| invalid(error.to_string()))?;
        if target.target_path() != self.reviewed.target_path {
            return Err(invalid("reviewed target id and path differ"));
        }
        let requested = self.reviewed.requested.open()?;
        let prepared = prepare_reviewed_footstep_preset_size_v1(
            &self.reviewed.target_path,
            &selector,
            requested,
        )
        .map_err(|error| invalid(error.to_string()))?;
        if self.reviewed.replacement_hex != encode_hex(prepared.replacement_bytes())
            || self.reviewed.binding_sha256 != encode_hex(prepared.binding_sha256())
            || self.stage.replacement_hex != self.reviewed.replacement_hex
        {
            return Err(invalid(
                "reviewed replacement or binding differs from selector",
            ));
        }

        if self.post_pack.target_id != self.reviewed.target_id
            || self.post_pack.requested != self.reviewed.requested
            || self.post_pack.replacement_hex != self.reviewed.replacement_hex
            || self.post_pack.reviewed_binding_sha256 != self.reviewed.binding_sha256
        {
            return Err(invalid("post-pack proof differs from reviewed intent"));
        }
        validate_hex(&self.post_pack.usmap_sha256, 32, "post-pack USMAP")?;
        if self.post_pack.usmap_sha256 != self.stage.usmap.sha256.to_string() {
            return Err(invalid("post-pack USMAP differs from staged USMAP"));
        }
        self.post_pack.package.validate()?;
        if self.post_pack.package.uasset_sha256 != self.stage.patched_uasset.sha256.to_string()
            || self.post_pack.package.uexp_sha256 != self.stage.patched_uexp.sha256.to_string()
        {
            return Err(invalid("post-pack package differs from staged package"));
        }
        let fresh = self.post_pack.fresh_selector.open()?;
        if fresh.package_seal != self.post_pack.package.to_package_pair()?
            || fresh.usmap_sha256 != self.post_pack.usmap_sha256
            || fresh.object_name != selector.object_name
            || fresh.class_path != selector.class_path
            || fresh.export_index != selector.export_index
            || fresh.component != selector.component
            || fresh.role != selector.role
            || fresh.kind != selector.kind
            || fresh.path != selector.path
            || fresh.expected_hex != self.reviewed.replacement_hex
        {
            return Err(invalid(
                "fresh selector facts do not bind the reviewed rebuilt package",
            ));
        }
        validate_readback_evidence(
            &self.post_pack.source_seals,
            &self.post_pack.chunk_seals,
            &self.stage.generation,
        )?;
        validate_pack_name(&self.output.pack_name)?;
        validate_output_triplet(&self.output.pack_name, &self.output.files)?;
        if self.deployed {
            return Err(invalid("receipt must remain undeployed"));
        }
        Ok(())
    }
}

impl RequestedSizeProjectionV1 {
    fn capture(value: ReviewedFootstepPresetSizeV1) -> Self {
        Self {
            x_f64_bits: value.x().to_bits(),
            y_f64_bits: value.y().to_bits(),
        }
    }

    fn open(
        &self,
    ) -> Result<ReviewedFootstepPresetSizeV1, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1>
    {
        ReviewedFootstepPresetSizeV1::try_new(
            f64::from_bits(self.x_f64_bits),
            f64::from_bits(self.y_f64_bits),
        )
        .map_err(|error| invalid(error.to_string()))
    }
}

impl SelectorProjectionV1 {
    fn capture(
        selector: &FixedLeafSelector,
    ) -> Result<Self, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        let canonical_json = serde_json::to_string(selector)
            .map_err(ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Serialize)?;
        if canonical_json.len() > MAX_SELECTOR_JSON_BYTES {
            return Err(invalid("selector exceeds receipt selector limit"));
        }
        Ok(Self {
            sha256: sha256_hex(canonical_json.as_bytes()),
            canonical_json,
        })
    }

    fn open(
        &self,
    ) -> Result<FixedLeafSelector, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        if self.canonical_json.len() > MAX_SELECTOR_JSON_BYTES {
            return Err(invalid("selector exceeds receipt selector limit"));
        }
        validate_hex(&self.sha256, 32, "selector seal")?;
        if sha256_hex(self.canonical_json.as_bytes()) != self.sha256 {
            return Err(invalid("selector seal mismatch"));
        }
        reject_duplicate_object_keys(&self.canonical_json)
            .map_err(ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::InvalidJson)?;
        let selector: FixedLeafSelector = serde_json::from_str(&self.canonical_json)
            .map_err(ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::InvalidJson)?;
        let canonical = serde_json::to_string(&selector)
            .map_err(ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Serialize)?;
        if canonical != self.canonical_json
            || selector.format != FIXED_LEAF_SELECTOR_FORMAT
            || selector.profile != FIXED_LEAF_SELECTOR_PROFILE
            || selector.path.is_empty()
        {
            return Err(invalid("selector is unsupported or noncanonical"));
        }
        validate_hex(&selector.usmap_sha256, 32, "selector USMAP")?;
        validate_hex(&selector.export_sha256, 32, "selector export")?;
        selector
            .expected_bytes()
            .map_err(|error| invalid(error.to_string()))?;
        Ok(selector)
    }
}

impl PackagePairProjectionV1 {
    fn capture(value: &PackagePairSeal) -> Self {
        Self {
            uasset_sha256: encode_hex(&value.uasset_sha256),
            uexp_sha256: encode_hex(&value.uexp_sha256),
        }
    }

    fn validate(&self) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        validate_hex(&self.uasset_sha256, 32, "post-pack uasset")?;
        validate_hex(&self.uexp_sha256, 32, "post-pack uexp")
    }

    fn to_package_pair(
        &self,
    ) -> Result<PackagePairSeal, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        self.validate()?;
        Ok(PackagePairSeal {
            uasset_sha256: decode_hex_32(&self.uasset_sha256)?,
            uexp_sha256: decode_hex_32(&self.uexp_sha256)?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1 {
    #[error("managed DataAsset build receipt exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid managed DataAsset build receipt JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize managed DataAsset build receipt: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("managed DataAsset build receipt is not exact canonical JSON")]
    NonCanonicalJson,
    #[error("managed DataAsset build receipt serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
    #[error("invalid managed DataAsset build receipt: {0}")]
    Invalid(String),
}

fn invalid(message: impl Into<String>) -> ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1 {
    ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message.into())
}

fn validate_content_seal(
    seal: &ContentSeal,
    label: &'static str,
) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
    if seal.byte_len == 0 {
        return Err(invalid(format!("{label} is empty")));
    }
    Ok(())
}

fn validate_pack_name(
    value: &str,
) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
    if value.is_empty()
        || value.len() > MAX_PACK_NAME_BYTES
        || !value.is_ascii()
        || !value.as_bytes()[0].is_ascii_alphanumeric()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(invalid("pack name is not a closed safe output component"));
    }
    let upper = value.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && matches!(upper.as_bytes()[3], b'1'..=b'9'))
    {
        return Err(invalid("pack name is a reserved Windows device name"));
    }
    Ok(())
}

fn validate_relative_triplet_file(
    value: &ManagedDataAssetTripletFileSealV1,
) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
    if value.relative_name.is_empty()
        || value.relative_name.len() > MAX_PACK_NAME_BYTES + 5
        || value.relative_name.contains(['/', '\\'])
        || value.byte_len == 0
        || value.byte_len > MAX_CONTAINER_COMPONENT_BYTES
    {
        return Err(invalid("malformed path-free triplet file seal"));
    }
    Ok(())
}

fn validate_output_triplet(
    pack_name: &str,
    files: &[ManagedDataAssetTripletFileSealV1],
) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
    let expected = [
        format!("{pack_name}.pak"),
        format!("{pack_name}.ucas"),
        format!("{pack_name}.utoc"),
    ];
    if files.len() != expected.len() {
        return Err(invalid("output must contain exactly three triplet seals"));
    }
    for (file, expected_name) in files.iter().zip(expected) {
        validate_relative_triplet_file(file)?;
        if file.relative_name != expected_name {
            return Err(invalid("output triplet names are not exact and sorted"));
        }
    }
    Ok(())
}

fn validate_generation_anchor_lengths(
    generation: &AssetGenerationReceipt,
) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
    fn require_at_most(
        length: u64,
        limit: u64,
        label: &'static str,
    ) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
        if length > limit {
            return Err(invalid(format!(
                "{label} exceeds its closed {limit}-byte limit"
            )));
        }
        Ok(())
    }

    require_at_most(generation.usmap.length, MAX_USMAP_BYTES, "generation USMAP")?;
    require_at_most(
        generation.main_utoc.length,
        MAX_CONTAINER_COMPONENT_BYTES,
        "generation main UTOC",
    )?;
    require_at_most(
        generation.global_utoc.length,
        MAX_CONTAINER_COMPONENT_BYTES,
        "generation global UTOC",
    )?;
    require_at_most(
        generation.global_ucas.length,
        MAX_CONTAINER_COMPONENT_BYTES,
        "generation global UCAS",
    )?;
    for anchor in &generation.container_set {
        require_at_most(
            anchor.length,
            MAX_CONTAINER_COMPONENT_BYTES,
            "generation source-container UTOC",
        )?;
    }
    let mut target_chunk_bytes = 0u64;
    for chunk in &generation.target_chunks {
        require_at_most(
            chunk.winner_utoc.length,
            MAX_CONTAINER_COMPONENT_BYTES,
            "generation chunk-winner UTOC",
        )?;
        target_chunk_bytes = target_chunk_bytes
            .checked_add(chunk.length)
            .ok_or_else(|| invalid("generation target chunk byte total overflowed"))?;
        if target_chunk_bytes > MAX_GENERATION_TARGET_CHUNK_BYTES {
            return Err(invalid(
                "generation target chunks exceed their closed aggregate byte limit",
            ));
        }
    }
    Ok(())
}

fn validate_readback_evidence(
    sources: &[ReadbackSourceSealProjectionV1],
    chunks: &[ReadbackChunkSealProjectionV1],
    generation: &AssetGenerationReceipt,
) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
    if sources.is_empty()
        || sources.len() > MAX_READBACK_SOURCES
        || chunks.is_empty()
        || chunks.len() > MAX_READBACK_CHUNKS
        || sources.windows(2).any(|pair| pair[0] >= pair[1])
        || chunks.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(invalid(
            "readback evidence is empty, excessive, duplicate, or unsorted",
        ));
    }
    let mut source_keys = BTreeSet::new();
    let mut primary_sources = 0usize;
    for source in sources {
        validate_hex(&source.utoc_blake3, 32, "readback source BLAKE3")?;
        source_keys.insert((source.role, source.utoc_blake3.as_str()));
        primary_sources += usize::from(source.role == ReadbackSourceRoleProjectionV1::Primary);
    }
    if primary_sources != 1 {
        return Err(invalid(
            "readback evidence must contain exactly one primary source",
        ));
    }
    let mut target_exports = generation.target_chunks.iter().filter(|chunk| {
        gore_tex::container::chunk_id_matches_asset_path(&chunk.chunk_id, &generation.asset)
            && chunk.chunk_type == "ExportBundleData"
    });
    let Some(target_export) = target_exports.next() else {
        return Err(invalid(
            "generation must contain exactly one target-package export",
        ));
    };
    if target_exports.next().is_some() {
        return Err(invalid(
            "generation must contain exactly one target-package export",
        ));
    }
    if generation.target_chunks.iter().any(|chunk| {
        gore_tex::container::chunk_id_matches_asset_path(&chunk.chunk_id, &generation.asset)
            && matches!(
                chunk.chunk_type.as_str(),
                "BulkData" | "OptionalBulkData" | "MemoryMappedBulkData"
            )
    }) {
        return Err(invalid(
            "reviewed generation cannot contain target-package sidecar chunks",
        ));
    }
    let target_export_prefix = &target_export.chunk_id[..16];
    let mut used_sources = BTreeSet::new();
    let mut unique_chunk_keys = BTreeSet::new();
    let mut primary_chunk_ids = BTreeSet::new();
    let mut fallback_chunk_ids = BTreeSet::new();
    let mut primary_exports = 0usize;
    let mut primary_headers = 0usize;
    let mut readback_chunk_bytes = 0u64;
    let mut primary_chunk_bytes = 0u64;
    for chunk in chunks {
        validate_hex(&chunk.source_utoc_blake3, 32, "chunk source BLAKE3")?;
        validate_hex(&chunk.chunk_id, 12, "chunk id")?;
        validate_hex(&chunk.blake3, 32, "chunk BLAKE3")?;
        if !matches!(chunk.toc_hash.len(), 40 | 64) {
            return Err(invalid("chunk TOC hash has unsupported width"));
        }
        validate_hex(&chunk.toc_hash, chunk.toc_hash.len() / 2, "chunk TOC hash")?;
        if !matches!(
            chunk.chunk_type.as_str(),
            "ContainerHeader"
                | "ExportBundleData"
                | "BulkData"
                | "OptionalBulkData"
                | "MemoryMappedBulkData"
                | "ScriptObjects"
        ) || chunk.length > MAX_VERIFIED_IOSTORE_SNAPSHOT_CHUNK_BYTES
            || (!matches!(
                chunk.chunk_type.as_str(),
                "BulkData" | "OptionalBulkData" | "MemoryMappedBulkData"
            ) && chunk.length == 0)
            || !source_keys.contains(&(chunk.source_role, chunk.source_utoc_blake3.as_str()))
        {
            return Err(invalid("malformed or unbound readback chunk evidence"));
        }
        readback_chunk_bytes = readback_chunk_bytes
            .checked_add(chunk.length)
            .ok_or_else(|| invalid("readback chunk byte total overflowed"))?;
        if readback_chunk_bytes > MAX_READBACK_CHUNK_BYTES {
            return Err(invalid(
                "readback chunks exceed their closed aggregate byte limit",
            ));
        }
        if !unique_chunk_keys.insert((
            chunk.source_role,
            chunk.source_utoc_blake3.as_str(),
            chunk.chunk_id.as_str(),
        )) {
            return Err(invalid("duplicate readback source/chunk identity"));
        }
        used_sources.insert((chunk.source_role, chunk.source_utoc_blake3.as_str()));
        match chunk.source_role {
            ReadbackSourceRoleProjectionV1::Primary => {
                primary_chunk_bytes = primary_chunk_bytes
                    .checked_add(chunk.length)
                    .ok_or_else(|| invalid("primary readback chunk byte total overflowed"))?;
                if primary_chunk_bytes > MAX_PRIMARY_READBACK_CHUNK_BYTES {
                    return Err(invalid(
                        "primary readback chunks exceed their closed aggregate byte limit",
                    ));
                }
                primary_chunk_ids.insert(chunk.chunk_id.as_str());
                match chunk.chunk_type.as_str() {
                    "ContainerHeader" => primary_headers += 1,
                    "ExportBundleData" => primary_exports += 1,
                    "BulkData" | "OptionalBulkData" | "MemoryMappedBulkData" => {
                        return Err(invalid(
                            "reviewed primary readback cannot contain sidecar chunks",
                        ));
                    }
                    "ScriptObjects" => {
                        return Err(invalid(
                            "primary readback source cannot contain ScriptObjects",
                        ));
                    }
                    _ => unreachable!("closed readback chunk type was validated above"),
                }
                if matches!(
                    chunk.chunk_type.as_str(),
                    "ExportBundleData" | "BulkData" | "OptionalBulkData" | "MemoryMappedBulkData"
                ) && &chunk.chunk_id[..16] != target_export_prefix
                {
                    return Err(invalid(
                        "primary target chunk differs from staged target package id",
                    ));
                }
            }
            ReadbackSourceRoleProjectionV1::Fallback => {
                fallback_chunk_ids.insert(chunk.chunk_id.as_str());
            }
        }
    }
    if primary_chunk_ids
        .iter()
        .any(|chunk_id| fallback_chunk_ids.contains(chunk_id))
    {
        return Err(invalid(
            "primary and fallback readback sources claim the same chunk id",
        ));
    }
    if primary_exports != 1 || primary_headers != 1 {
        return Err(invalid(
            "readback evidence has invalid primary single-package chunk cardinality",
        ));
    }
    if used_sources != source_keys {
        return Err(invalid(
            "readback evidence contains a source seal unused by every chunk",
        ));
    }
    Ok(())
}

fn project_source_role<T: Serialize>(
    role: T,
) -> Result<ReadbackSourceRoleProjectionV1, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
    match serde_json::to_string(&role)
        .map_err(ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Serialize)?
        .as_str()
    {
        "\"primary\"" => Ok(ReadbackSourceRoleProjectionV1::Primary),
        "\"fallback\"" => Ok(ReadbackSourceRoleProjectionV1::Fallback),
        _ => Err(invalid("unsupported readback source role")),
    }
}

fn validate_hex(
    value: &str,
    bytes: usize,
    label: &'static str,
) -> Result<(), ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
    if value.len() != bytes.saturating_mul(2)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(invalid(format!("{label} is not canonical lowercase hex")));
    }
    Ok(())
}

fn decode_hex_32(
    value: &str,
) -> Result<[u8; 32], ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1> {
    validate_hex(value, 32, "SHA-256")?;
    let mut output = [0u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        output[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
    }
    Ok(output)
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("canonical lowercase hex was validated"),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn sha256_hex(bytes: &[u8]) -> String {
    encode_hex(&Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use gore_asset::dataasset_workflow::{
        AssetGenerationReceipt, GenerationChunkAnchor, GenerationFileAnchor,
    };
    use gore_asset::{
        FixedLeafRole, FixedLeafSelectorStep, FixedLeafWireType, FixedWireKind, PackageComponent,
    };
    use serde_json::{json, Value};

    use super::*;
    use crate::working_store::WorkingStoreFormat;

    const TARGET: &str = "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps";
    const TARGET_CHUNK_PREFIX: &str = "01e173a19ea374c9";

    fn digest(byte: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([byte; 32])
    }

    fn seal(byte_len: u64, byte: u8) -> ContentSeal {
        ContentSeal {
            byte_len,
            sha256: digest(byte),
        }
    }

    fn anchor(name: &str, length: u64, byte: u8) -> GenerationFileAnchor {
        GenerationFileAnchor {
            file_name: name.to_owned(),
            length,
            sha256: format!("{byte:02x}").repeat(32),
        }
    }

    fn generation(usmap_sha256: &str) -> AssetGenerationReceipt {
        let main = anchor("G1R-Windows.utoc", 64, 0x11);
        let global_utoc = anchor("global.utoc", 32, 0x22);
        let chunk = |ordinal: u32, chunk_type: &str| GenerationChunkAnchor {
            chunk_id: format!("{TARGET_CHUNK_PREFIX}{ordinal:08x}"),
            chunk_type: chunk_type.to_owned(),
            winner_utoc: main.clone(),
            length: 1,
            blake3: "a1".repeat(32),
            toc_hash: "b2".repeat(20),
            toc_hash_bytes: 20,
        };
        AssetGenerationReceipt {
            format: "gore.asset.generation.v1".to_owned(),
            asset: TARGET.to_owned(),
            usmap: GenerationFileAnchor {
                file_name: "Mappings.usmap".to_owned(),
                length: 128,
                sha256: usmap_sha256.to_owned(),
            },
            main_utoc: main.clone(),
            global_utoc: global_utoc.clone(),
            global_ucas: anchor("global.ucas", 96, 0x33),
            container_set: vec![main.clone(), global_utoc],
            target_chunks: vec![chunk(1, "ContainerHeader"), chunk(2, "ExportBundleData")],
        }
    }

    fn vector4_hex(components: [f64; 4]) -> String {
        components
            .into_iter()
            .flat_map(f64::to_le_bytes)
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn reviewed_selector() -> FixedLeafSelector {
        FixedLeafSelector {
            format: FIXED_LEAF_SELECTOR_FORMAT,
            profile: FIXED_LEAF_SELECTOR_PROFILE.to_owned(),
            package_seal: PackagePairSeal {
                uasset_sha256: [0x41; 32],
                uexp_sha256: [0x42; 32],
            },
            usmap_sha256: "73".repeat(32),
            export_index: 0,
            object_name: "DA_WolfFootsteps".to_owned(),
            class_path: "/Script/G1R.FootstepTag".to_owned(),
            component: PackageComponent::Uexp,
            export_sha256: "51".repeat(32),
            role: FixedLeafRole::PropertyValue,
            kind: FixedWireKind::Vector4F64x4,
            path: vec![
                FixedLeafSelectorStep::Property {
                    schema_index: 0,
                    property_name: "BoneData".to_owned(),
                    array_index: 0,
                    array_dimension: 1,
                    declaring_schema_name: "FootstepTag".to_owned(),
                    declaring_module_path: Some("/Script/G1R".to_owned()),
                    property_type: FixedLeafWireType::Struct {
                        name: "BoneFeetData".to_owned(),
                    },
                },
                FixedLeafSelectorStep::Struct {
                    name: "BoneFeetData".to_owned(),
                    schema_name: "/Script/G1R.BoneFeetData".to_owned(),
                },
                FixedLeafSelectorStep::Property {
                    schema_index: 0,
                    property_name: "FeetTextureSize".to_owned(),
                    array_index: 0,
                    array_dimension: 1,
                    declaring_schema_name: "BoneFeetData".to_owned(),
                    declaring_module_path: Some("/Script/G1R".to_owned()),
                    property_type: FixedLeafWireType::Struct {
                        name: "Vector4".to_owned(),
                    },
                },
            ],
            expected_hex: vector4_hex([10.0, 10.0, 0.0, 1.0]),
        }
    }

    fn fixture() -> ManagedRevision3ReviewedDataAssetBuildReceiptV1 {
        let selector = reviewed_selector();
        let requested = ReviewedFootstepPresetSizeV1::try_new(11.0, 12.0).unwrap();
        let reviewed = prepare_reviewed_footstep_preset_size_v1(TARGET, &selector, requested)
            .expect("reviewed fixture");
        let replacement_hex = encode_hex(reviewed.replacement_bytes());
        let binding_sha256 = encode_hex(reviewed.binding_sha256());
        let selector_projection = SelectorProjectionV1::capture(&selector).unwrap();
        let patched_uasset = seal(101, 0x31);
        let patched_uexp = seal(202, 0x32);
        let mut fresh = selector.clone();
        fresh.package_seal = PackagePairSeal {
            uasset_sha256: [0x31; 32],
            uexp_sha256: [0x32; 32],
        };
        fresh.export_sha256 = "33".repeat(32);
        fresh.expected_hex = replacement_hex.clone();
        let current_head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: seal(777, 0x77),
        };
        let stage_basis_head = WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: seal(666, 0x66),
        };
        let files = vec![
            ManagedDataAssetTripletFileSealV1::try_new("WolfReview.pak", 1, digest(0x81)).unwrap(),
            ManagedDataAssetTripletFileSealV1::try_new("WolfReview.ucas", 2, digest(0x82)).unwrap(),
            ManagedDataAssetTripletFileSealV1::try_new("WolfReview.utoc", 3, digest(0x83)).unwrap(),
        ];
        let source = ReadbackSourceSealProjectionV1 {
            role: ReadbackSourceRoleProjectionV1::Primary,
            utoc_blake3: "91".repeat(32),
        };
        let chunk = ReadbackChunkSealProjectionV1 {
            source_role: ReadbackSourceRoleProjectionV1::Primary,
            source_utoc_blake3: source.utoc_blake3.clone(),
            chunk_id: format!("{TARGET_CHUNK_PREFIX}00000002"),
            chunk_type: "ExportBundleData".to_owned(),
            length: 303,
            blake3: "92".repeat(32),
            toc_hash: "93".repeat(20),
        };
        let header = ReadbackChunkSealProjectionV1 {
            chunk_id: format!("{TARGET_CHUNK_PREFIX}00000001"),
            chunk_type: "ContainerHeader".to_owned(),
            length: 111,
            blake3: "94".repeat(32),
            toc_hash: "95".repeat(20),
            ..chunk.clone()
        };
        let executable = seal(4096, 0xe1);
        let generation = generation(&selector.usmap_sha256);
        let receipt = ManagedRevision3ReviewedDataAssetBuildReceiptV1 {
            format: MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FORMAT_V1.to_owned(),
            basis: ManagedBuildBasisProjectionV1 {
                current_head,
                project_id: ProjectId::from_bytes([7; 16]),
                project_revision: 9,
                executable: executable.clone(),
            },
            stage: ManagedStageProjectionV1 {
                manifest_asset: seal(909, 0x90),
                project_id: ProjectId::from_bytes([7; 16]),
                project_target: GameGenerationAnchor { executable },
                basis_head: stage_basis_head,
                basis_project_revision: 8,
                staged_project_revision: 9,
                target_path: TARGET.to_owned(),
                generation,
                selector: selector_projection.clone(),
                replacement_hex: replacement_hex.clone(),
                patched_uasset,
                patched_uexp,
                usmap: ContentSeal {
                    byte_len: 128,
                    sha256: selector.usmap_sha256.parse().unwrap(),
                },
            },
            reviewed: ReviewedIntentProjectionV1 {
                format: REVIEWED_DATAASSET_FORMAT_V1,
                schema_id: REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID.to_owned(),
                schema_revision: REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION,
                field_id: REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID.to_owned(),
                target_id: "g1r:dataasset:footstep-preset:wolf".to_owned(),
                target_path: TARGET.to_owned(),
                requested: RequestedSizeProjectionV1::capture(requested),
                selector_sha256: selector_projection.sha256,
                replacement_hex: replacement_hex.clone(),
                binding_sha256: binding_sha256.clone(),
            },
            post_pack: ReviewedPostPackProjectionV1 {
                target_id: "g1r:dataasset:footstep-preset:wolf".to_owned(),
                requested: RequestedSizeProjectionV1::capture(requested),
                replacement_hex,
                reviewed_binding_sha256: binding_sha256,
                package: PackagePairProjectionV1 {
                    uasset_sha256: "31".repeat(32),
                    uexp_sha256: "32".repeat(32),
                },
                usmap_sha256: selector.usmap_sha256,
                fresh_selector: SelectorProjectionV1::capture(&fresh).unwrap(),
                source_seals: vec![source],
                chunk_seals: vec![header, chunk],
            },
            output: ManagedBuildOutputProjectionV1 {
                pack_name: "WolfReview".to_owned(),
                files,
            },
            build_authority:
                ManagedDataAssetBuildAuthorityV1::ReviewedFixedLeafSinglePackageTriplet,
            publication_authority: ManagedDataAssetPublicationAuthorityV1::NotGranted,
            deployed: false,
            runtime_status: ManagedDataAssetRuntimeStatusV1::RuntimeUnqualified,
        };
        receipt.validate().expect("valid receipt fixture");
        receipt
    }

    fn basis_and_proof_projection_fixture() -> (
        VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1,
        ManagedVerifiedTripletPostPackProjectionV1,
    ) {
        let receipt = fixture();
        let basis = VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1 {
            basis: receipt.basis.clone(),
            stage: receipt.stage.clone(),
            reviewed: receipt.reviewed.clone(),
        };
        let proof = ManagedVerifiedTripletPostPackProjectionV1 {
            target_path: receipt.stage.target_path.clone(),
            generation: receipt.stage.generation.clone(),
            executable_length: receipt.basis.executable.byte_len,
            executable_sha256: *receipt.basis.executable.sha256.as_bytes(),
            replay_seal: receipt.post_pack.package.to_package_pair().unwrap(),
            post_pack: receipt.post_pack,
            output: receipt.output,
        };
        verify_proof_binding(&basis, &proof).unwrap();
        (basis, proof)
    }

    fn reject_proof_projection_mutation(
        mutate: impl FnOnce(&mut ManagedVerifiedTripletPostPackProjectionV1),
    ) {
        let (basis, mut proof) = basis_and_proof_projection_fixture();
        mutate(&mut proof);
        assert!(verify_proof_binding(&basis, &proof).is_err());
    }

    fn raw_json(receipt: &ManagedRevision3ReviewedDataAssetBuildReceiptV1) -> String {
        serde_json::to_string(receipt).unwrap()
    }

    fn reject_mutation(mutate: impl FnOnce(&mut ManagedRevision3ReviewedDataAssetBuildReceiptV1)) {
        let mut receipt = fixture();
        mutate(&mut receipt);
        assert!(
            ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(&receipt))
                .is_err()
        );
    }

    #[test]
    fn golden_canonical_roundtrip_is_stable() {
        let receipt = fixture();
        let json = receipt.to_canonical_json().unwrap();
        assert_eq!(
            ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&json).unwrap(),
            receipt
        );
        assert_eq!(
            sha256_hex(json.as_bytes()),
            "d4c4fa218ded9725d970dd990ffbbf0594acde3338a6e496f45c58f5ecff7b82"
        );
    }

    #[test]
    fn top_level_receipt_is_not_directly_deserializable() {
        trait AmbiguousIfDeserialize<Marker> {
            fn marker() {}
        }
        impl<T: ?Sized> AmbiguousIfDeserialize<()> for T {}
        impl<T> AmbiguousIfDeserialize<u8> for T where T: for<'de> serde::Deserialize<'de> {}

        let _ =
            <ManagedRevision3ReviewedDataAssetBuildReceiptV1 as AmbiguousIfDeserialize<_>>::marker
                as fn();
    }

    #[test]
    fn source_and_managed_basis_are_nonclone_and_nonserde_authority() {
        trait AmbiguousIfClone<Marker> {
            fn marker() {}
        }
        impl<T: ?Sized> AmbiguousIfClone<()> for T {}
        impl<T: ?Sized + Clone> AmbiguousIfClone<u8> for T {}

        trait AmbiguousIfSerialize<Marker> {
            fn marker() {}
        }
        impl<T: ?Sized> AmbiguousIfSerialize<()> for T {}
        impl<T: ?Sized + serde::Serialize> AmbiguousIfSerialize<u8> for T {}

        trait AmbiguousIfDeserialize<Marker> {
            fn marker() {}
        }
        impl<T: ?Sized> AmbiguousIfDeserialize<()> for T {}
        impl<T> AmbiguousIfDeserialize<u8> for T where T: for<'de> serde::Deserialize<'de> {}

        let _ = <VerifiedCurrentReviewedDataAssetStageSourceV1 as AmbiguousIfClone<_>>::marker
            as fn();
        let _ = <VerifiedCurrentReviewedDataAssetStageSourceV1 as AmbiguousIfSerialize<_>>::marker
            as fn();
        let _ = <VerifiedCurrentReviewedDataAssetStageSourceV1 as AmbiguousIfDeserialize<_>>::marker
            as fn();
        let _ =
            <VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1 as AmbiguousIfClone<_>>::marker
                as fn();
        let _ =
            <VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1 as AmbiguousIfSerialize<_>>::marker
                as fn();
        let _ = <VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1 as AmbiguousIfDeserialize<
            _,
        >>::marker as fn();
    }

    #[test]
    fn genuine_constructor_requires_basis_and_opaque_asset_proof() {
        let _: fn(
            VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1,
            &VerifiedManagedReviewedTripletPostPackV1,
        ) -> Result<
            ManagedRevision3ReviewedDataAssetBuildReceiptV1,
            ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1,
        > = ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_verified;
    }

    #[test]
    fn proof_binding_refuses_mismatched_authority_facts() {
        reject_proof_projection_mutation(|proof| proof.target_path = "/Game/Other".to_owned());
        reject_proof_projection_mutation(|proof| proof.generation.asset = "/Game/Other".to_owned());
        reject_proof_projection_mutation(|proof| proof.executable_length += 1);
        reject_proof_projection_mutation(|proof| proof.executable_sha256[0] ^= 0xff);
        reject_proof_projection_mutation(|proof| proof.replay_seal.uasset_sha256[0] ^= 0xff);
        reject_proof_projection_mutation(|proof| proof.post_pack.target_id.push_str(":other"));
        reject_proof_projection_mutation(|proof| proof.post_pack.requested.x_f64_bits ^= 1);
        reject_proof_projection_mutation(|proof| proof.post_pack.replacement_hex = "ab".repeat(32));
        reject_proof_projection_mutation(|proof| {
            proof.post_pack.reviewed_binding_sha256 = "ab".repeat(32)
        });
        reject_proof_projection_mutation(|proof| {
            proof.post_pack.package.uexp_sha256 = "ab".repeat(32)
        });
        reject_proof_projection_mutation(|proof| proof.post_pack.usmap_sha256 = "ab".repeat(32));
        reject_proof_projection_mutation(|proof| {
            let mut fresh = proof.post_pack.fresh_selector.open().unwrap();
            fresh.expected_hex = "ab".repeat(32);
            proof.post_pack.fresh_selector = SelectorProjectionV1::capture(&fresh).unwrap();
        });
        reject_proof_projection_mutation(|proof| proof.output.pack_name = "Other".to_owned());
    }

    #[test]
    fn public_triplet_seal_deserialization_revalidates_its_closed_invariants() {
        let valid =
            ManagedDataAssetTripletFileSealV1::try_new("WolfReview.pak", 1, digest(0x81)).unwrap();
        let json = serde_json::to_string(&valid).unwrap();
        assert_eq!(
            serde_json::from_str::<ManagedDataAssetTripletFileSealV1>(&json).unwrap(),
            valid
        );

        for (field, invalid_value) in [
            ("relative_name", json!("Wolf/Review.pak")),
            ("byte_len", json!(0)),
            ("byte_len", json!(MAX_CONTAINER_COMPONENT_BYTES + 1)),
        ] {
            let mut value = serde_json::to_value(&valid).unwrap();
            value[field] = invalid_value;
            assert!(
                serde_json::from_value::<ManagedDataAssetTripletFileSealV1>(value).is_err(),
                "invalid public triplet seal field {field:?} was accepted"
            );
        }

        let mut unknown = serde_json::to_value(&valid).unwrap();
        unknown["unknown"] = json!(true);
        assert!(serde_json::from_value::<ManagedDataAssetTripletFileSealV1>(unknown).is_err());
    }

    #[test]
    fn parser_rejects_unknown_noncanonical_duplicate_and_overlimit_input() {
        let receipt = fixture();
        let canonical = receipt.to_canonical_json().unwrap();
        let mut unknown: Value = serde_json::from_str(&canonical).unwrap();
        unknown["unknown"] = json!(true);
        assert!(ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(
            &serde_json::to_string(&unknown).unwrap()
        )
        .is_err());
        assert!(ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(
            &serde_json::to_string_pretty(&receipt).unwrap()
        )
        .is_err());
        let duplicate = canonical.replacen("{", "{\"format\":\"duplicate\",", 1);
        assert!(ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&duplicate).is_err());
        let overlimit =
            " ".repeat(MAX_MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_BYTES_V1 + 1);
        assert!(matches!(
            ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&overlimit),
            Err(ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::InputTooLarge { .. })
        ));
    }

    #[test]
    fn parser_revalidates_all_cross_bindings_and_sorted_unique_evidence() {
        reject_mutation(|receipt| receipt.basis.project_id = ProjectId::from_bytes([8; 16]));
        reject_mutation(|receipt| receipt.basis.project_revision += 1);
        reject_mutation(|receipt| receipt.basis.executable.sha256 = digest(0xef));
        reject_mutation(|receipt| receipt.basis.current_head.snapshot.byte_len = 0);
        reject_mutation(|receipt| {
            receipt.basis.current_head.snapshot.byte_len = crate::MAX_REVISION3_SNAPSHOT_BYTES + 1
        });
        reject_mutation(|receipt| {
            receipt.stage.basis_head.snapshot.byte_len = crate::MAX_REVISION3_SNAPSHOT_BYTES + 1
        });
        reject_mutation(|receipt| {
            receipt.basis.executable.byte_len = MAX_GAME_EXECUTABLE_BYTES + 1
        });
        reject_mutation(|receipt| {
            receipt.stage.project_target.executable.byte_len = MAX_GAME_EXECUTABLE_BYTES + 1
        });
        reject_mutation(|receipt| receipt.stage.manifest_asset.byte_len = 0);
        reject_mutation(|receipt| receipt.stage.project_id = ProjectId::from_bytes([8; 16]));
        reject_mutation(|receipt| receipt.stage.project_target.executable.sha256 = digest(0xee));
        reject_mutation(|receipt| receipt.stage.staged_project_revision += 1);
        reject_mutation(|receipt| receipt.stage.target_path = "/Game/Other".to_owned());
        reject_mutation(|receipt| receipt.stage.generation.asset = "/Game/Other".to_owned());
        reject_mutation(|receipt| receipt.stage.generation.usmap.sha256 = "ab".repeat(32));
        reject_mutation(|receipt| receipt.stage.replacement_hex = "ab".repeat(32));
        reject_mutation(|receipt| receipt.reviewed.selector_sha256 = "ab".repeat(32));
        reject_mutation(|receipt| receipt.reviewed.schema_revision += 1);
        reject_mutation(|receipt| receipt.reviewed.target_path = "/Game/Other".to_owned());
        reject_mutation(|receipt| receipt.reviewed.target_id.push_str(":other"));
        reject_mutation(|receipt| receipt.reviewed.requested.x_f64_bits = 0.0f64.to_bits());
        reject_mutation(|receipt| receipt.reviewed.replacement_hex = "ab".repeat(32));
        reject_mutation(|receipt| receipt.reviewed.binding_sha256 = "ab".repeat(32));
        reject_mutation(|receipt| receipt.post_pack.target_id.push_str(":other"));
        reject_mutation(|receipt| receipt.post_pack.requested.y_f64_bits = 13.0f64.to_bits());
        reject_mutation(|receipt| receipt.post_pack.replacement_hex = "ab".repeat(32));
        reject_mutation(|receipt| receipt.post_pack.reviewed_binding_sha256 = "ab".repeat(32));
        reject_mutation(|receipt| receipt.post_pack.package.uasset_sha256 = "ab".repeat(32));
        reject_mutation(|receipt| receipt.post_pack.usmap_sha256 = "ab".repeat(32));
        reject_mutation(|receipt| receipt.post_pack.fresh_selector.sha256 = "ab".repeat(32));
        reject_mutation(|receipt| {
            let mut fresh = receipt.post_pack.fresh_selector.open().unwrap();
            fresh.expected_hex = "ab".repeat(32);
            receipt.post_pack.fresh_selector = SelectorProjectionV1::capture(&fresh).unwrap();
        });
        reject_mutation(|receipt| {
            receipt
                .post_pack
                .source_seals
                .push(receipt.post_pack.source_seals[0].clone())
        });
        reject_mutation(|receipt| {
            receipt
                .post_pack
                .source_seals
                .push(ReadbackSourceSealProjectionV1 {
                    role: ReadbackSourceRoleProjectionV1::Fallback,
                    utoc_blake3: "fe".repeat(32),
                })
        });
        reject_mutation(|receipt| {
            receipt.post_pack.chunk_seals[1].source_utoc_blake3 = "ab".repeat(32)
        });
        reject_mutation(|receipt| {
            receipt.post_pack.chunk_seals.remove(0);
        });
        reject_mutation(|receipt| {
            receipt.post_pack.chunk_seals[1]
                .chunk_id
                .replace_range(..16, "abababababababab")
        });
        reject_mutation(|receipt| {
            let mut duplicate = receipt.post_pack.chunk_seals[1].clone();
            duplicate.blake3 = "fe".repeat(32);
            receipt.post_pack.chunk_seals.push(duplicate);
            receipt.post_pack.chunk_seals.sort();
        });
        reject_mutation(|receipt| receipt.output.pack_name = "Other".to_owned());
        reject_mutation(|receipt| receipt.output.files.swap(0, 1));
        reject_mutation(|receipt| receipt.output.files[0].byte_len = 0);
        reject_mutation(|receipt| {
            receipt.output.files[0].byte_len = MAX_CONTAINER_COMPONENT_BYTES + 1
        });
    }

    #[test]
    fn parser_rejects_primary_fallback_chunk_id_intersection() {
        let mut receipt = fixture();
        let fallback_source = ReadbackSourceSealProjectionV1 {
            role: ReadbackSourceRoleProjectionV1::Fallback,
            utoc_blake3: "fd".repeat(32),
        };
        let mut fallback_chunk = receipt
            .post_pack
            .chunk_seals
            .iter()
            .find(|chunk| {
                chunk.source_role == ReadbackSourceRoleProjectionV1::Primary
                    && chunk.chunk_type == "ExportBundleData"
            })
            .unwrap()
            .clone();
        fallback_chunk.source_role = ReadbackSourceRoleProjectionV1::Fallback;
        fallback_chunk.source_utoc_blake3 = fallback_source.utoc_blake3.clone();
        fallback_chunk.blake3 = "fc".repeat(32);
        fallback_chunk.toc_hash = "fb".repeat(20);
        receipt.post_pack.source_seals.push(fallback_source);
        receipt.post_pack.source_seals.sort();
        receipt.post_pack.chunk_seals.push(fallback_chunk);
        receipt.post_pack.chunk_seals.sort();

        let error = ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(&receipt))
            .unwrap_err();
        assert!(matches!(
            error,
            ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message)
                if message.contains("primary and fallback readback sources claim the same chunk id")
        ));
    }

    #[test]
    fn parser_rejects_primary_script_objects_and_duplicate_sidecar_roles() {
        let mut primary_script_objects = fixture();
        let mut script_objects = primary_script_objects.post_pack.chunk_seals[1].clone();
        script_objects.chunk_id = format!("{TARGET_CHUNK_PREFIX}00000003");
        script_objects.chunk_type = "ScriptObjects".to_owned();
        script_objects.blake3 = "a3".repeat(32);
        script_objects.toc_hash = "b3".repeat(20);
        primary_script_objects
            .post_pack
            .chunk_seals
            .push(script_objects);
        primary_script_objects.post_pack.chunk_seals.sort();
        let error = ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(
            &primary_script_objects,
        ))
        .unwrap_err();
        assert!(matches!(
            error,
            ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message)
                if message.contains("primary readback source cannot contain ScriptObjects")
        ));

        for chunk_type in ["BulkData", "OptionalBulkData", "MemoryMappedBulkData"] {
            let mut sidecar = fixture();
            let mut chunk = sidecar.post_pack.chunk_seals[1].clone();
            chunk.chunk_id = format!("{TARGET_CHUNK_PREFIX}00000003");
            chunk.chunk_type = chunk_type.to_owned();
            chunk.blake3 = "a4".repeat(32);
            chunk.toc_hash = "b4".repeat(20);
            sidecar.post_pack.chunk_seals.push(chunk);
            sidecar.post_pack.chunk_seals.sort();
            let error =
                ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(&sidecar))
                    .unwrap_err();
            assert!(matches!(
                error,
                ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message)
                    if message.contains("reviewed primary readback cannot contain sidecar chunks")
            ));
        }

        let mut duplicate_bulk = fixture();
        for (ordinal, byte) in [(3, 0xa4), (4, 0xa5)] {
            let mut chunk = duplicate_bulk.post_pack.chunk_seals[1].clone();
            chunk.chunk_id = format!("{TARGET_CHUNK_PREFIX}{ordinal:08x}");
            chunk.chunk_type = "BulkData".to_owned();
            chunk.blake3 = format!("{byte:02x}").repeat(32);
            chunk.toc_hash = format!("{:02x}", byte + 1).repeat(20);
            duplicate_bulk.post_pack.chunk_seals.push(chunk);
        }
        duplicate_bulk.post_pack.chunk_seals.sort();
        let error =
            ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(&duplicate_bulk))
                .unwrap_err();
        assert!(matches!(
            error,
            ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message)
                if message.contains("reviewed primary readback cannot contain sidecar chunks")
        ));
    }

    #[test]
    fn generation_target_selection_ignores_valid_dependency_chunks_but_stays_closed() {
        let mut dependencies = fixture();
        let winner = dependencies.stage.generation.main_utoc.clone();
        for (ordinal, chunk_type) in [
            (1, "ExportBundleData"),
            (2, "BulkData"),
            (3, "OptionalBulkData"),
            (4, "MemoryMappedBulkData"),
            (5, "ContainerHeader"),
        ] {
            let chunk_id = format!("{}{:08x}", "ab".repeat(8), ordinal);
            assert!(!gore_tex::container::chunk_id_matches_asset_path(
                &chunk_id, TARGET
            ));
            dependencies
                .stage
                .generation
                .target_chunks
                .push(GenerationChunkAnchor {
                    chunk_id,
                    chunk_type: chunk_type.to_owned(),
                    winner_utoc: winner.clone(),
                    length: 1,
                    blake3: format!("{:02x}", 0xc0 + ordinal).repeat(32),
                    toc_hash: format!("{:02x}", 0xd0 + ordinal).repeat(20),
                    toc_hash_bytes: 20,
                });
        }
        dependencies.validate().unwrap();

        let mut target_sidecar = fixture();
        let mut sidecar = target_sidecar.stage.generation.target_chunks[1].clone();
        sidecar.chunk_id = format!("{TARGET_CHUNK_PREFIX}00000003");
        sidecar.chunk_type = "BulkData".to_owned();
        target_sidecar.stage.generation.target_chunks.push(sidecar);
        let error =
            ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(&target_sidecar))
                .unwrap_err();
        assert!(matches!(
            error,
            ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message)
                if message.contains("reviewed generation cannot contain target-package sidecar")
        ));

        let mut duplicate_target_export = fixture();
        let mut extra_export = duplicate_target_export.stage.generation.target_chunks[1].clone();
        extra_export.chunk_id = format!("{TARGET_CHUNK_PREFIX}00000003");
        duplicate_target_export
            .stage
            .generation
            .target_chunks
            .push(extra_export);
        let error = ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(
            &duplicate_target_export,
        ))
        .unwrap_err();
        assert!(matches!(
            error,
            ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message)
                if message.contains("exactly one target-package export")
        ));
    }

    #[test]
    fn parser_rejects_every_generation_file_anchor_one_over_its_production_limit() {
        fn reject_anchor(label: &'static str, mutate: impl FnOnce(&mut AssetGenerationReceipt)) {
            let mut receipt = fixture();
            mutate(&mut receipt.stage.generation);
            let error =
                ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(&receipt))
                    .unwrap_err();
            assert!(
                matches!(
                    &error,
                    ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message)
                        if message.contains(label) && message.contains("exceeds its closed")
                ),
                "unexpected error for {label}: {error}"
            );
        }

        reject_anchor("generation USMAP", |generation| {
            generation.usmap.length = MAX_USMAP_BYTES + 1;
        });
        reject_anchor("generation main UTOC", |generation| {
            generation.main_utoc.length = MAX_CONTAINER_COMPONENT_BYTES + 1;
        });
        reject_anchor("generation global UTOC", |generation| {
            generation.global_utoc.length = MAX_CONTAINER_COMPONENT_BYTES + 1;
        });
        reject_anchor("generation global UCAS", |generation| {
            generation.global_ucas.length = MAX_CONTAINER_COMPONENT_BYTES + 1;
        });
        reject_anchor("generation source-container UTOC", |generation| {
            generation.container_set[0].length = MAX_CONTAINER_COMPONENT_BYTES + 1;
        });
        reject_anchor("generation chunk-winner UTOC", |generation| {
            generation.target_chunks[0].winner_utoc.length = MAX_CONTAINER_COMPONENT_BYTES + 1;
        });
    }

    #[test]
    fn parser_rejects_one_over_generation_and_readback_aggregate_limits() {
        let mut generation_total = fixture();
        generation_total.stage.generation.target_chunks[0].length =
            MAX_VERIFIED_IOSTORE_SNAPSHOT_CHUNK_BYTES;
        generation_total.stage.generation.target_chunks[1].length =
            MAX_VERIFIED_IOSTORE_SNAPSHOT_CHUNK_BYTES;
        let mut extra_generation_chunk = generation_total.stage.generation.target_chunks[1].clone();
        extra_generation_chunk.chunk_id = format!("{TARGET_CHUNK_PREFIX}00000003");
        extra_generation_chunk.chunk_type = "BulkData".to_owned();
        extra_generation_chunk.length = 1;
        generation_total
            .stage
            .generation
            .target_chunks
            .push(extra_generation_chunk);
        let error = ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(
            &generation_total,
        ))
        .unwrap_err();
        assert!(matches!(
            error,
            ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message)
                if message.contains("generation target chunks exceed their closed aggregate")
        ));

        let mut primary_total = fixture();
        primary_total.post_pack.chunk_seals[0].length = MAX_VERIFIED_IOSTORE_SNAPSHOT_CHUNK_BYTES;
        primary_total.post_pack.chunk_seals[1].length = MAX_VERIFIED_IOSTORE_SNAPSHOT_CHUNK_BYTES;
        let mut extra_primary_header = primary_total.post_pack.chunk_seals[0].clone();
        extra_primary_header.chunk_id = format!("{TARGET_CHUNK_PREFIX}00000003");
        extra_primary_header.length = 1;
        extra_primary_header.blake3 = "a6".repeat(32);
        extra_primary_header.toc_hash = "b6".repeat(20);
        primary_total
            .post_pack
            .chunk_seals
            .push(extra_primary_header);
        primary_total.post_pack.chunk_seals.sort();
        let error =
            ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(&primary_total))
                .unwrap_err();
        assert!(matches!(
            error,
            ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message)
                if message.contains("primary readback chunks exceed their closed aggregate")
        ));

        let mut readback_total = fixture();
        let fallback_source = ReadbackSourceSealProjectionV1 {
            role: ReadbackSourceRoleProjectionV1::Fallback,
            utoc_blake3: "fd".repeat(32),
        };
        readback_total
            .post_pack
            .source_seals
            .push(fallback_source.clone());
        let primary_bytes = readback_total
            .post_pack
            .chunk_seals
            .iter()
            .map(|chunk| chunk.length)
            .sum::<u64>();
        let mut remaining = MAX_READBACK_CHUNK_BYTES + 1 - primary_bytes;
        let mut ordinal = 0u64;
        while remaining > 0 {
            let length = remaining.min(MAX_VERIFIED_IOSTORE_SNAPSHOT_CHUNK_BYTES);
            readback_total
                .post_pack
                .chunk_seals
                .push(ReadbackChunkSealProjectionV1 {
                    source_role: ReadbackSourceRoleProjectionV1::Fallback,
                    source_utoc_blake3: fallback_source.utoc_blake3.clone(),
                    chunk_id: format!("{ordinal:024x}"),
                    chunk_type: "ScriptObjects".to_owned(),
                    length,
                    blake3: format!("{:02x}", 0xc0 + ordinal).repeat(32),
                    toc_hash: format!("{:02x}", 0xd0 + ordinal).repeat(20),
                });
            remaining -= length;
            ordinal += 1;
        }
        readback_total.post_pack.source_seals.sort();
        readback_total.post_pack.chunk_seals.sort();
        let error =
            ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&raw_json(&readback_total))
                .unwrap_err();
        assert!(matches!(
            error,
            ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1::Invalid(message)
                if message.contains("readback chunks exceed their closed aggregate")
        ));
    }

    #[test]
    fn multiple_fallback_sources_may_report_the_same_non_primary_chunk() {
        let mut receipt = fixture();
        let shared_chunk_id = "ab".repeat(12);
        for (source_byte, chunk_byte) in [(0xd1, 0xe1), (0xd2, 0xe2)] {
            let source_utoc_blake3 = format!("{source_byte:02x}").repeat(32);
            receipt
                .post_pack
                .source_seals
                .push(ReadbackSourceSealProjectionV1 {
                    role: ReadbackSourceRoleProjectionV1::Fallback,
                    utoc_blake3: source_utoc_blake3.clone(),
                });
            receipt
                .post_pack
                .chunk_seals
                .push(ReadbackChunkSealProjectionV1 {
                    source_role: ReadbackSourceRoleProjectionV1::Fallback,
                    source_utoc_blake3,
                    chunk_id: shared_chunk_id.clone(),
                    chunk_type: "ScriptObjects".to_owned(),
                    length: 1,
                    blake3: format!("{chunk_byte:02x}").repeat(32),
                    toc_hash: format!("{:02x}", chunk_byte + 1).repeat(20),
                });
        }
        receipt.post_pack.source_seals.sort();
        receipt.post_pack.chunk_seals.sort();

        receipt.validate().unwrap();
    }

    #[test]
    fn canonical_wire_is_path_free_and_has_no_legacy_path_keys() {
        let supplied_filesystem_paths = [
            r"C:\store\objects\deadbeef",
            r"D:\Games\GORE\Content\Paks",
            r"C:\output\WolfReview.utoc",
            r"C:\temp\extract",
            r"C:\source\G1R-Windows.utoc",
        ];
        let value = serde_json::to_value(fixture()).unwrap();
        let forbidden_keys = [
            "store_path",
            "game_root",
            "output_root",
            "temp_path",
            "patch_path",
            "extract_path",
            "source_utoc",
            "source_path",
        ];
        fn scan(value: &Value, paths: &[&str], keys: &[&str]) {
            match value {
                Value::Object(object) => {
                    for (key, child) in object {
                        assert!(!keys.contains(&key.as_str()), "forbidden key {key:?}");
                        assert!(
                            key == "target_path" || !key.ends_with("_path"),
                            "filesystem-shaped key {key:?}"
                        );
                        scan(child, paths, keys);
                    }
                }
                Value::Array(values) => {
                    for child in values {
                        scan(child, paths, keys);
                    }
                }
                Value::String(string) => {
                    for path in paths {
                        assert!(!string.contains(path), "leaked filesystem path {path:?}");
                    }
                    let bytes = string.as_bytes();
                    let drive_absolute = bytes.windows(3).any(|window| {
                        window[0].is_ascii_alphabetic()
                            && window[1] == b':'
                            && matches!(window[2], b'\\' | b'/')
                    });
                    let unc_absolute = string.contains(r"\\") || string.contains("//");
                    assert!(
                        !drive_absolute && !unc_absolute,
                        "absolute Windows/UNC path leaked: {string:?}"
                    );
                }
                _ => {}
            }
        }
        scan(&value, &supplied_filesystem_paths, &forbidden_keys);
        let canonical = fixture().to_canonical_json().unwrap();
        for forbidden in supplied_filesystem_paths {
            assert!(!canonical.contains(forbidden));
        }
    }

    #[test]
    fn parser_cannot_widen_deploy_publication_runtime_or_build_scope() {
        let canonical = fixture().to_canonical_json().unwrap();
        for (pointer, widened) in [
            ("/deployed", json!(true)),
            ("/publication_authority", json!("granted")),
            ("/runtime_status", json!("runtime_qualified")),
            ("/build_authority", json!("unbounded")),
        ] {
            let mut value: Value = serde_json::from_str(&canonical).unwrap();
            *value.pointer_mut(pointer).unwrap() = widened;
            let json = serde_json::to_string(&value).unwrap();
            assert!(ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&json).is_err());
        }
        let parsed =
            ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_json(&canonical).unwrap();
        assert!(!parsed.deployed());
        assert_eq!(
            parsed.publication_authority(),
            ManagedDataAssetPublicationAuthorityV1::NotGranted
        );
        assert_eq!(
            parsed.runtime_status(),
            ManagedDataAssetRuntimeStatusV1::RuntimeUnqualified
        );
    }
}
