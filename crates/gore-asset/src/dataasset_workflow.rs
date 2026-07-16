//! Closed, bounded wire receipts for the receipt-bound cooked-DataAsset workflow.
//!
//! This module is deliberately narrower than a generic package authoring API. It owns the
//! extract-v2 and patch-fixed-v2 wire models plus the validation that turns untrusted receipt
//! bytes into an exact, filesystem-bound projection. Raw deserialized structs are facts only;
//! callers must obtain [`ValidatedExtractBinding`] through the validators in this module before
//! using any receipt as provenance.

use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use gore_tex::installed_package_index::{
    InstalledPackageSidecarRoleV1, InstalledPackageSourceEvidenceV1,
    VerifiedInstalledPackageExtractionV1, VerifiedInstalledUsmapV1,
};
use retoc::{FIoContainerId, FPackageId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::package::BoundRegularFile;
use crate::{
    FixedLeafPatch, FixedLeafPatchReceipt, FixedLeafSelector, FixedWireKind, LegacyPackageEnvelope,
    PackageCarrier, PackageComponent, PackageLimits, PackagePairSeal, PropertySpanWalker,
    ReviewedDataAssetStageEligibilityV1, ReviewedFootstepPresetReplacementV1, SchemaDb,
    UsmapLimits,
};

pub const MAX_USMAP_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_SELECTOR_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_RECEIPT_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_OPTIONAL_SIDECAR_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_COOKED_PACKAGE_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_CONTAINER_COMPONENT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_MOUNT_UTOC_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_MOUNT_UCAS_BYTES: u64 = 128 * 1024 * 1024 * 1024;
pub const MAX_GAME_EXECUTABLE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const EXTRACT_RECEIPT_NAME: &str = "gore-asset-extract.json";
pub const COPIED_USMAP_NAME: &str = "gore-generation.usmap";
pub const PATCH_RECEIPT_SUFFIX: &str = ".gore-asset-patch.json";
pub const EXTRACT_CONTENT_BINDING: &str = "each consumed decompressed chunk was verified against its winning container's TOC BLAKE3 hash and cached for all conversion reads";
pub const COMPOSITE_UCAS_ROLE: &str =
    "environment anchor only; consumed_chunks is the authoritative content binding";
pub const HELD_IDENTITY_VERIFICATION: &str = "identity_length_mtime_point_check";
pub const HELD_IDENTITY_LIMITATION: &str = "the large UCAS payload is not content-hashed; file identity, length, and modification stamp are held and point-rechecked before publication";
pub const MAX_GAME_ASSET_SEGMENTS: usize = 32;

pub fn asset_package_limits() -> PackageLimits {
    PackageLimits {
        max_uasset_bytes: 64 * 1024 * 1024,
        max_uexp_bytes: 256 * 1024 * 1024,
        max_total_bytes: 320 * 1024 * 1024,
    }
}

/// Content facts produced only by this module's bounded, no-follow file verifier.
#[derive(Debug, Clone)]
pub struct VerifiedFileSeal {
    path: PathBuf,
    length: u64,
    sha256: [u8; 32],
    blake3: [u8; 32],
}

impl VerifiedFileSeal {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }

    pub fn reverify(&self, limit: u64, code: &'static str) -> Result<()> {
        let current = validate_existing_path_no_reparse(&self.path, false, code)?;
        if current != self.path {
            bail!(
                "{code}: sealed source path changed: {}",
                self.path.display()
            );
        }
        verify_file_hash(&current, self.length, self.sha256, limit, code)
    }
}

/// Exact bytes and content facts produced only by a bounded, no-follow read.
#[derive(Debug)]
pub struct VerifiedInput {
    path: PathBuf,
    bytes: Vec<u8>,
    sha256: [u8; 32],
}

impl VerifiedInput {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn length(&self) -> u64 {
        u64::try_from(self.bytes.len()).expect("a loaded Vec length always fits u64")
    }

    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationFileAnchor {
    pub file_name: String,
    pub length: u64,
    pub sha256: String,
}

impl GenerationFileAnchor {
    pub fn matches_verified_input(&self, input: &VerifiedInput) -> bool {
        self.length == input.length() && self.sha256 == encode_hex(input.sha256())
    }

    pub fn same_content(&self, other: &Self) -> bool {
        self.length == other.length && self.sha256 == other.sha256
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationChunkAnchor {
    pub chunk_id: String,
    pub chunk_type: String,
    pub winner_utoc: GenerationFileAnchor,
    pub length: u64,
    pub blake3: String,
    pub toc_hash: String,
    pub toc_hash_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetGenerationReceipt {
    pub format: String,
    pub asset: String,
    pub usmap: GenerationFileAnchor,
    pub main_utoc: GenerationFileAnchor,
    pub global_utoc: GenerationFileAnchor,
    pub global_ucas: GenerationFileAnchor,
    pub container_set: Vec<GenerationFileAnchor>,
    pub target_chunks: Vec<GenerationChunkAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptComponent {
    pub relative_path: String,
    pub length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceFileReceipt {
    pub path: String,
    pub length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HeldIdentityReceipt {
    pub path: String,
    pub length: u64,
    pub modified_stamp: String,
    pub platform_identity: String,
    pub sha256: Option<String>,
    pub verification: String,
    pub content_hash_omitted: bool,
    pub limitation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtractCompositeStoreAnchor {
    pub utoc: SourceFileReceipt,
    pub ucas: HeldIdentityReceipt,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptVerifiedChunk {
    pub chunk_id: String,
    pub chunk_type: String,
    pub source_utoc: PathBuf,
    pub length: u64,
    pub blake3: String,
    pub toc_hash: String,
    pub toc_hash_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtractUsmapProof {
    pub source: SourceFileReceipt,
    pub copied_relative_path: String,
    pub copy: ReceiptComponent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GlobalScriptStoreProof {
    pub utoc: SourceFileReceipt,
    pub ucas: SourceFileReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtractReceiptSource {
    pub game_root: String,
    pub composite_store_anchor: ExtractCompositeStoreAnchor,
    pub consumed_chunks: Vec<ReceiptVerifiedChunk>,
    pub source_container_tocs: Vec<SourceFileReceipt>,
    pub content_binding: String,
    pub usmap: ExtractUsmapProof,
    pub global_script_store: GlobalScriptStoreProof,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtractReceiptOutput {
    pub root: String,
    pub receipt: String,
    pub components: Vec<ReceiptComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtractReceiptEnvelope {
    pub format: String,
    pub status: String,
    pub asset: String,
    pub generation: AssetGenerationReceipt,
    pub source: ExtractReceiptSource,
    pub package_seal: PackagePairSeal,
    pub output: ExtractReceiptOutput,
    pub deployed: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SidecarRole {
    #[serde(rename = "BulkData")]
    Bulk,
    #[serde(rename = "OptionalBulkData")]
    Optional,
    #[serde(rename = "MemoryMappedBulkData")]
    MemoryMapped,
}

impl SidecarRole {
    pub const ALL: [Self; 3] = [Self::Bulk, Self::Optional, Self::MemoryMapped];

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Bulk => "ubulk",
            Self::Optional => "uptnl",
            Self::MemoryMapped => "m.ubulk",
        }
    }

    pub fn chunk_type(self) -> &'static str {
        match self {
            Self::Bulk => "BulkData",
            Self::Optional => "OptionalBulkData",
            Self::MemoryMapped => "MemoryMappedBulkData",
        }
    }

    pub fn from_chunk_type(value: &str) -> Option<Self> {
        match value {
            "BulkData" => Some(Self::Bulk),
            "OptionalBulkData" => Some(Self::Optional),
            "MemoryMappedBulkData" => Some(Self::MemoryMapped),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Bulk => 0,
            Self::Optional => 1,
            Self::MemoryMapped => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SidecarReceipt {
    pub role: SidecarRole,
    pub file_name: String,
    pub length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct ValidatedExtractBinding {
    output_root: PathBuf,
    uasset: ReceiptComponent,
    uexp: ReceiptComponent,
    copied_usmap: ReceiptComponent,
    components: Vec<ReceiptComponent>,
    sidecars: Vec<SidecarReceipt>,
}

impl ValidatedExtractBinding {
    pub fn output_root(&self) -> &Path {
        &self.output_root
    }

    pub fn uasset(&self) -> &ReceiptComponent {
        &self.uasset
    }

    pub fn uexp(&self) -> &ReceiptComponent {
        &self.uexp
    }

    pub fn copied_usmap(&self) -> &ReceiptComponent {
        &self.copied_usmap
    }

    pub fn components(&self) -> &[ReceiptComponent] {
        &self.components
    }

    pub fn sidecars(&self) -> &[SidecarReceipt] {
        &self.sidecars
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PatchReceiptProvenance {
    pub extract_receipt: ReceiptFileSeal,
    pub generation: AssetGenerationReceipt,
    pub usmap: GenerationFileAnchor,
    pub extract_components: Vec<ReceiptComponent>,
    pub extracted_sidecars: Vec<SidecarReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PatchOperationProof {
    pub before: PackagePairSeal,
    pub after: PackagePairSeal,
    pub export_index: usize,
    pub component: PackageComponent,
    pub absolute_offset: usize,
    pub length: usize,
    pub kind: FixedWireKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComponentDigestProof {
    pub path: String,
    pub length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PatchReceiptOutput {
    pub uasset: ComponentDigestProof,
    pub uexp: ComponentDigestProof,
    pub sidecars: Vec<SidecarReceipt>,
    pub receipt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PatchReceiptEnvelope {
    pub format: String,
    pub status: String,
    pub asset: String,
    pub generation_bound: bool,
    pub provenance: PatchReceiptProvenance,
    pub input_package_seal: PackagePairSeal,
    pub output_package_seal: PackagePairSeal,
    pub output_sidecars: Vec<SidecarReceipt>,
    pub input_selector: FixedLeafSelector,
    pub output_requires_reinspect: bool,
    pub expected_hex: String,
    pub replacement_hex: String,
    pub patch: PatchOperationProof,
    pub output: PatchReceiptOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptFileSeal {
    pub path: String,
    pub length: u64,
    pub sha256: String,
}

#[derive(Debug)]
pub struct VerifiedExtractReceipt {
    input: VerifiedInput,
    receipt: ExtractReceiptEnvelope,
    binding: ValidatedExtractBinding,
}

impl VerifiedExtractReceipt {
    pub fn input(&self) -> &VerifiedInput {
        &self.input
    }

    pub fn receipt(&self) -> &ExtractReceiptEnvelope {
        &self.receipt
    }

    pub fn binding(&self) -> &ValidatedExtractBinding {
        &self.binding
    }
}

#[derive(Debug)]
pub struct VerifiedPatchReceipt {
    input: VerifiedInput,
    receipt: PatchReceiptEnvelope,
}

impl VerifiedPatchReceipt {
    pub fn input(&self) -> &VerifiedInput {
        &self.input
    }

    pub fn receipt(&self) -> &PatchReceiptEnvelope {
        &self.receipt
    }
}

/// One optional cooked-package sidecar whose bytes were bound to a PatchReceipt v2 and then
/// independently hashed from a no-follow file handle.
///
/// Fields remain private so receipt JSON or caller-provided digests cannot be promoted into a
/// managed-stage input without passing [`verify_fixed_leaf_stage_input`].
#[derive(Debug)]
pub struct VerifiedFixedLeafStageSidecar {
    role: SidecarRole,
    bytes: Vec<u8>,
    sha256: [u8; 32],
}

impl VerifiedFixedLeafStageSidecar {
    pub const fn role(&self) -> SidecarRole {
        self.role
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn length(&self) -> u64 {
        u64::try_from(self.bytes.len()).expect("a loaded Vec length always fits u64")
    }

    pub const fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

/// Opaque content identity of the live game executable selected by a validated ExtractReceipt.
///
/// The source path is intentionally not exposed. Construction hashes the executable through the
/// same bounded, reparse-refusing file path used by the workflow's other live generation anchors.
pub struct VerifiedGameExecutableAnchor {
    source: BoundRegularFile,
    length: u64,
    sha256: [u8; 32],
}

impl fmt::Debug for VerifiedGameExecutableAnchor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedGameExecutableAnchor")
            .field("length", &self.length())
            .field("sha256", &encode_hex(self.sha256()))
            .finish()
    }
}

impl VerifiedGameExecutableAnchor {
    pub const fn length(&self) -> u64 {
        self.length
    }

    pub const fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }

    pub fn reverify(&self) -> Result<()> {
        self.reverify_path_identity()?;
        verify_file_hash(
            self.source.path(),
            self.length,
            self.sha256,
            MAX_GAME_EXECUTABLE_BYTES,
            "ASSET_STAGE_EXECUTABLE",
        )?;
        self.reverify_path_identity()
    }

    /// Cheap write-boundary check against the originally held executable file identity and
    /// length. Full SHA-256 reverification brackets long staging phases separately.
    pub fn reverify_path_identity(&self) -> Result<()> {
        self.source
            .reverify_path_identity()
            .context("ASSET_STAGE_EXECUTABLE: executable path identity changed")?;
        let length = self
            .source
            .length()
            .context("ASSET_STAGE_EXECUTABLE: reading held executable length")?;
        if length != self.length {
            bail!("ASSET_STAGE_EXECUTABLE: executable length changed");
        }
        Ok(())
    }
}

/// Opaque, filesystem-bound input for one managed fixed-leaf DataAsset stage.
///
/// Construction consumes a verified PatchReceipt v2 and reopens its complete provenance chain,
/// original pair, copied USMAP, patched pair, optional sidecars, live game generation, and live
/// executable. Raw receipt bytes are deliberately discarded after verification. This value grants
/// no build, runtime, deployment, project-head publication, or future-reinspection authority.
pub struct VerifiedFixedLeafStageInput {
    target_path: String,
    generation: AssetGenerationReceipt,
    selector: FixedLeafSelector,
    replacement_hex: String,
    patched_uasset: Vec<u8>,
    patched_uexp: Vec<u8>,
    usmap: Vec<u8>,
    sidecars: Vec<VerifiedFixedLeafStageSidecar>,
    game_root: PathBuf,
    retained_source_roots: Vec<PathBuf>,
    executable: VerifiedGameExecutableAnchor,
}

impl fmt::Debug for VerifiedFixedLeafStageInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sidecar_roles: Vec<_> = self.sidecars.iter().map(|sidecar| sidecar.role).collect();
        formatter
            .debug_struct("VerifiedFixedLeafStageInput")
            .field("target_path", &self.target_path)
            .field("generation", &self.generation)
            .field("selector", &self.selector)
            .field("replacement_hex", &self.replacement_hex)
            .field("patched_uasset_bytes", &self.patched_uasset.len())
            .field("patched_uexp_bytes", &self.patched_uexp.len())
            .field("usmap_bytes", &self.usmap.len())
            .field("sidecar_roles", &sidecar_roles)
            .field("executable", &self.executable)
            .finish()
    }
}

impl VerifiedFixedLeafStageInput {
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

    pub fn patched_component_bytes(&self, component: PackageComponent) -> &[u8] {
        match component {
            PackageComponent::Uasset => &self.patched_uasset,
            PackageComponent::Uexp => &self.patched_uexp,
        }
    }

    pub fn usmap_bytes(&self) -> &[u8] {
        &self.usmap
    }

    pub fn sidecars(&self) -> &[VerifiedFixedLeafStageSidecar] {
        &self.sidecars
    }

    pub fn executable_anchor(&self) -> &VerifiedGameExecutableAnchor {
        &self.executable
    }

    /// Re-hash the opaque executable source without revealing its local path.
    pub fn reverify_executable_anchor(&self) -> Result<()> {
        self.executable.reverify()
    }

    /// Revalidate only held executable path identity/length for a tight Store write boundary.
    pub fn reverify_executable_path_identity(&self) -> Result<()> {
        self.executable.reverify_path_identity()
    }

    /// Re-probe the exact live IoStore generation selected by the validated ExtractReceipt.
    pub fn reverify_live_generation(&self) -> Result<()> {
        let current = probe_current_generation_receipt(
            &self.game_root,
            &self.target_path,
            &self.generation,
            "ASSET_STAGE_GENERATION",
        )?;
        if current != self.generation {
            bail!("ASSET_STAGE_GENERATION: live target generation changed after verification");
        }
        Ok(())
    }

    /// Require a managed Store root to be completely outside the live game tree and every
    /// retained extract/patch source root, in both containment directions and without revealing
    /// any retained path.
    pub fn require_store_root_disjoint(&self, store_root: &Path) -> Result<()> {
        let store_root =
            validate_existing_path_no_reparse(store_root, true, "ASSET_STAGE_STORE_ROOT")?;
        for source_root in std::iter::once(&self.game_root).chain(&self.retained_source_roots) {
            let source_root =
                validate_existing_path_no_reparse(source_root, true, "ASSET_STAGE_SOURCE_ROOT")?;
            if store_root.starts_with(&source_root) || source_root.starts_with(&store_root) {
                bail!(
                    "ASSET_STAGE_STORE_ROOT: managed Store and verified DataAsset sources must be disjoint"
                );
            }
        }
        Ok(())
    }
}

/// Untrusted, borrowed facts from one exact-current managed reviewed DataAsset stage.
///
/// This DTO deliberately owns no bytes and grants no authority. Its public fields make the FFI
/// boundary explicit: every duplicated project, stage, review, and live-install fact is checked by
/// [`verify_managed_offline_dataasset_package_v1`] before an opaque package is returned.
pub struct UnverifiedBorrowedManagedReviewedDataAssetSourceV1<'a> {
    pub target_path: &'a str,
    pub generation: &'a AssetGenerationReceipt,
    pub persisted_selector: &'a FixedLeafSelector,
    pub persisted_replacement_hex: &'a str,
    pub patched_uasset: &'a [u8],
    pub patched_uexp: &'a [u8],
    pub usmap: &'a [u8],
    pub sidecars: &'a [(SidecarRole, &'a [u8])],
    pub expected_executable_length: u64,
    pub expected_executable_sha256: [u8; 32],
    pub reviewed: &'a ReviewedFootstepPresetReplacementV1,
}

impl fmt::Debug for UnverifiedBorrowedManagedReviewedDataAssetSourceV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sidecar_roles: Vec<_> = self.sidecars.iter().map(|(role, _)| *role).collect();
        formatter
            .debug_struct("UnverifiedBorrowedManagedReviewedDataAssetSourceV1")
            .field("target_path", &self.target_path)
            .field("generation", &self.generation)
            .field("patched_uasset_bytes", &self.patched_uasset.len())
            .field("patched_uexp_bytes", &self.patched_uexp.len())
            .field("usmap_bytes", &self.usmap.len())
            .field("sidecar_roles", &sidecar_roles)
            .field(
                "expected_executable_length",
                &self.expected_executable_length,
            )
            .field(
                "expected_executable_sha256",
                &encode_hex(&self.expected_executable_sha256),
            )
            .field("reviewed", &self.reviewed)
            .finish()
    }
}

/// Exact reviewed package bytes independently replayed from the current installed generation.
///
/// Construction rechecks the managed Store facts against a fresh live conversion and retains
/// private install/executable guards for a later staging boundary. The value has no `Clone`,
/// serialization, filesystem-path, consuming-parts, build, deployment, or runtime API.
pub struct VerifiedManagedOfflineDataAssetPackageV1<'a> {
    target_path: &'a str,
    generation: &'a AssetGenerationReceipt,
    reviewed: &'a ReviewedFootstepPresetReplacementV1,
    uasset: Vec<u8>,
    uexp: Vec<u8>,
    usmap: Vec<u8>,
    replay_seal: PackagePairSeal,
    game_root: PathBuf,
    executable: VerifiedGameExecutableAnchor,
}

impl fmt::Debug for VerifiedManagedOfflineDataAssetPackageV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedManagedOfflineDataAssetPackageV1")
            .field("target_path", &self.target_path)
            .field("generation", &self.generation)
            .field("reviewed", &self.reviewed)
            .field("uasset_bytes", &self.uasset.len())
            .field("uexp_bytes", &self.uexp.len())
            .field("usmap_bytes", &self.usmap.len())
            .field("replay_seal", &self.replay_seal)
            .field("executable", &self.executable)
            .finish()
    }
}

impl<'a> VerifiedManagedOfflineDataAssetPackageV1<'a> {
    pub fn target_path(&self) -> &str {
        self.target_path
    }

    pub fn generation(&self) -> &AssetGenerationReceipt {
        self.generation
    }

    pub fn reviewed(&self) -> &ReviewedFootstepPresetReplacementV1 {
        self.reviewed
    }

    pub fn uasset_bytes(&self) -> &[u8] {
        &self.uasset
    }

    pub fn uexp_bytes(&self) -> &[u8] {
        &self.uexp
    }

    pub fn usmap_bytes(&self) -> &[u8] {
        &self.usmap
    }

    pub fn replay_seal(&self) -> &PackagePairSeal {
        &self.replay_seal
    }

    /// Recheck the retained executable and exact target generation without exposing either path.
    pub fn reverify_live_authority(&self) -> Result<()> {
        self.executable.reverify()?;
        let current = probe_current_generation_receipt(
            &self.game_root,
            self.target_path,
            self.generation,
            "ASSET_MANAGED_OFFLINE_FINAL",
        )?;
        if &current != self.generation {
            bail!("ASSET_MANAGED_OFFLINE_FINAL: live target generation changed after verification");
        }
        self.executable.reverify()
    }
}

/// Independently replay one exact-current managed reviewed stage against the installed game.
///
/// `game_root` is used only as private live-verification authority. No caller-selected output path
/// is accepted and this function never writes the game tree.
pub fn verify_managed_offline_dataasset_package_v1<'a>(
    game_root: &Path,
    source: UnverifiedBorrowedManagedReviewedDataAssetSourceV1<'a>,
) -> Result<VerifiedManagedOfflineDataAssetPackageV1<'a>> {
    verify_managed_offline_dataasset_package_v1_with_live_source(
        game_root,
        source,
        |game_root, target_path| {
            capture_live_converted_stage_source(
                game_root,
                target_path,
                "ASSET_MANAGED_OFFLINE_GENERATION",
            )
        },
        |game_root| {
            let path = gore_tex::paths::usmap(game_root)
                .context("ASSET_MANAGED_OFFLINE_USMAP: resolving exact live USMAP")?;
            read_verified_file_bounded(&path, MAX_USMAP_BYTES, "ASSET_MANAGED_OFFLINE_USMAP")
        },
        |game_root, target_path, expected| {
            probe_current_generation_receipt(
                game_root,
                target_path,
                expected,
                "ASSET_MANAGED_OFFLINE_FINAL",
            )
        },
    )
}

fn verify_managed_offline_dataasset_package_v1_with_live_source<'a, F, U, G>(
    game_root: &Path,
    source: UnverifiedBorrowedManagedReviewedDataAssetSourceV1<'a>,
    live_source: F,
    live_usmap_source: U,
    final_generation_probe: G,
) -> Result<VerifiedManagedOfflineDataAssetPackageV1<'a>>
where
    F: FnOnce(&Path, &str) -> Result<LiveConvertedStageSource>,
    U: FnOnce(&Path) -> Result<VerifiedInput>,
    G: FnOnce(&Path, &str, &AssetGenerationReceipt) -> Result<AssetGenerationReceipt>,
{
    validate_managed_offline_dataasset_source_v1(&source)?;

    let game_root = normalize_game_install_root(game_root, "ASSET_MANAGED_OFFLINE_ROOT")?;
    let executable = seal_game_executable(&game_root)?;
    if executable.length() != source.expected_executable_length
        || executable.sha256() != &source.expected_executable_sha256
    {
        bail!("ASSET_MANAGED_OFFLINE_EXECUTABLE: live executable differs from the managed project target");
    }

    let live = live_source(&game_root, source.target_path)
        .context("ASSET_MANAGED_OFFLINE_GENERATION: independently converting live target")?;
    if &live.generation != source.generation {
        bail!("ASSET_MANAGED_OFFLINE_GENERATION: live target generation differs from the managed stage");
    }
    if !live.sidecars.is_empty() {
        bail!("ASSET_MANAGED_OFFLINE_SIDECAR: reviewed package v1 does not support live sidecars");
    }

    let live_usmap = live_usmap_source(&game_root)
        .context("ASSET_MANAGED_OFFLINE_USMAP: independently reading live USMAP")?;
    if !live.generation.usmap.matches_verified_input(&live_usmap)
        || live_usmap.bytes() != source.usmap
    {
        bail!("ASSET_MANAGED_OFFLINE_USMAP: live USMAP differs from the managed stage");
    }
    let schemas = SchemaDb::from_usmap_bounded(live_usmap.bytes(), UsmapLimits::default())
        .context("ASSET_MANAGED_OFFLINE_USMAP: parsing exact live USMAP")?;

    let LiveConvertedStageSource {
        generation,
        uasset,
        uexp,
        sidecars: _,
    } = live;
    let mut replayed = PackageCarrier::from_bytes(uasset, uexp, asset_package_limits())
        .context("ASSET_MANAGED_OFFLINE_SOURCE_PAIR: loading freshly converted package")?;
    if PackagePairSeal::capture(&replayed) != source.persisted_selector.package_seal {
        bail!("ASSET_MANAGED_OFFLINE_SOURCE_PAIR: live vanilla pair differs from the reviewed selector snapshot");
    }
    apply_fixed_leaf_selector_patch(
        &mut replayed,
        &schemas,
        source.persisted_selector,
        source.reviewed.expected_bytes(),
        source.reviewed.replacement_bytes(),
    )
    .context("ASSET_MANAGED_OFFLINE_REPLAY: replaying reviewed selector on live vanilla bytes")?;
    if replayed.bytes(PackageComponent::Uasset) != source.patched_uasset
        || replayed.bytes(PackageComponent::Uexp) != source.patched_uexp
    {
        bail!("ASSET_MANAGED_OFFLINE_PATCHED_PAIR: managed pair contains bytes outside the exact reviewed replay");
    }
    let replay_seal = PackagePairSeal::capture(&replayed);

    verify_file_hash(
        live_usmap.path(),
        live_usmap.length(),
        *live_usmap.sha256(),
        MAX_USMAP_BYTES,
        "ASSET_MANAGED_OFFLINE_USMAP",
    )?;
    let current_generation = final_generation_probe(&game_root, source.target_path, &generation)?;
    if current_generation != generation {
        bail!("ASSET_MANAGED_OFFLINE_FINAL: live target generation changed during verification");
    }
    executable.reverify()?;

    let (uasset, uexp) = replayed.into_bytes();
    Ok(VerifiedManagedOfflineDataAssetPackageV1 {
        target_path: source.target_path,
        generation: source.generation,
        reviewed: source.reviewed,
        uasset,
        uexp,
        usmap: live_usmap.bytes,
        replay_seal,
        game_root,
        executable,
    })
}

fn validate_managed_offline_dataasset_source_v1(
    source: &UnverifiedBorrowedManagedReviewedDataAssetSourceV1<'_>,
) -> Result<()> {
    const CODE: &str = "ASSET_MANAGED_OFFLINE_INPUT";
    validate_game_asset_path(source.target_path, CODE)?;
    validate_generation_receipt(source.generation, CODE)?;
    if source.generation.asset != source.target_path {
        bail!("{CODE}: target path differs from the managed generation");
    }
    if source.reviewed.target().target_path() != source.target_path {
        bail!("{CODE}: target path differs from the reviewed intent");
    }
    if source.persisted_selector != source.reviewed.selector() {
        bail!("{CODE}: persisted selector differs from the reviewed intent");
    }
    if source.persisted_replacement_hex != encode_hex(source.reviewed.replacement_bytes()) {
        bail!("{CODE}: persisted replacement differs from the reviewed intent");
    }
    let expected = source
        .persisted_selector
        .expected_bytes()
        .context("ASSET_MANAGED_OFFLINE_INPUT: decoding persisted selector expectation")?;
    if expected.as_slice() != source.reviewed.expected_bytes() {
        bail!("{CODE}: persisted selector expectation differs from the reviewed intent");
    }
    if source.persisted_selector.usmap_sha256 != source.generation.usmap.sha256 {
        bail!("{CODE}: persisted selector USMAP differs from the managed generation");
    }
    match crate::evaluate_reviewed_dataasset_stage_v1(
        source.target_path,
        source.persisted_selector,
        source.persisted_replacement_hex,
    ) {
        ReviewedDataAssetStageEligibilityV1::Eligible(rederived)
            if rederived.as_ref() == source.reviewed => {}
        ReviewedDataAssetStageEligibilityV1::Eligible(_) => {
            bail!("{CODE}: supplied reviewed intent differs from exact stage re-derivation")
        }
        ReviewedDataAssetStageEligibilityV1::Blocked(reason) => {
            bail!("{CODE}: managed stage is not an exact reviewed v1 edit: {reason}")
        }
    }
    if !source.sidecars.is_empty() {
        bail!("ASSET_MANAGED_OFFLINE_SIDECAR: reviewed package v1 does not support persisted sidecars");
    }
    validate_sidecar_generation_mapping(&[], source.generation, "ASSET_MANAGED_OFFLINE_SIDECAR")?;
    if source.expected_executable_length == 0
        || source.expected_executable_length > MAX_GAME_EXECUTABLE_BYTES
    {
        bail!("{CODE}: expected executable length is outside the supported range");
    }

    let limits = asset_package_limits();
    let uasset_length = u64::try_from(source.patched_uasset.len())
        .context("ASSET_MANAGED_OFFLINE_INPUT: uasset length overflowed")?;
    let uexp_length = u64::try_from(source.patched_uexp.len())
        .context("ASSET_MANAGED_OFFLINE_INPUT: uexp length overflowed")?;
    let pair_length = uasset_length
        .checked_add(uexp_length)
        .context("ASSET_MANAGED_OFFLINE_INPUT: package pair length overflowed")?;
    if uasset_length > limits.max_uasset_bytes
        || uexp_length > limits.max_uexp_bytes
        || pair_length > limits.max_total_bytes
    {
        bail!("{CODE}: persisted package pair exceeds managed size limits");
    }
    let usmap_length = u64::try_from(source.usmap.len())
        .context("ASSET_MANAGED_OFFLINE_INPUT: USMAP length overflowed")?;
    if usmap_length > MAX_USMAP_BYTES
        || source.generation.usmap.length != usmap_length
        || source.generation.usmap.sha256 != encode_hex(&Sha256::digest(source.usmap))
    {
        bail!("{CODE}: persisted USMAP differs from the managed generation seal");
    }
    Ok(())
}

/// Consume one verified PatchReceipt v2 and bind every file needed by managed staging.
///
/// The output paths and historical source paths remain implementation details. Downstream
/// manifests should persist only the target package path, generation facts, and content seals.
pub fn verify_fixed_leaf_stage_input(
    patch: VerifiedPatchReceipt,
) -> Result<VerifiedFixedLeafStageInput> {
    verify_fixed_leaf_stage_input_with_live_source(
        patch,
        |game_root, asset, _expected| {
            capture_live_converted_stage_source(game_root, asset, "ASSET_STAGE_GENERATION")
        },
        |game_root, asset, expected| {
            probe_current_generation_receipt(game_root, asset, expected, "ASSET_STAGE_GENERATION")
        },
    )
}

/// Turn one extract-v2 capability plus an offset-free selector into the same opaque managed-stage
/// authority as a separately authored PatchReceipt v2.
///
/// The patch pair and receipt are materialized only inside a private, game-disjoint temporary
/// directory. They pass the complete existing PatchReceipt/live-generation verifier before this
/// function returns, then the temporary tree is deleted. Callers receive neither paths nor raw
/// offsets, and the returned authority remains build-, runtime-, deployment-, and publication-
/// neutral.
pub fn verify_fixed_leaf_stage_edit(
    extract: VerifiedExtractReceipt,
    selector: FixedLeafSelector,
    replacement_hex: &str,
) -> Result<VerifiedFixedLeafStageInput> {
    verify_fixed_leaf_stage_edit_with_live_source_in(
        extract,
        selector,
        replacement_hex,
        &std::env::temp_dir(),
        |game_root, asset, _expected| {
            capture_live_converted_stage_source(game_root, asset, "ASSET_STAGE_GENERATION")
        },
        |game_root, asset, expected| {
            probe_current_generation_receipt(game_root, asset, expected, "ASSET_STAGE_GENERATION")
        },
    )
}

/// Promote one still-live installed-package snapshot directly into the existing managed
/// fixed-leaf stage authority without accepting or materializing an extract/patch receipt.
///
/// The target path and original cooked bytes come only from the server-selected installed
/// extraction. This function independently reconstructs the same target from `game_root`,
/// compares the complete package pair, every role-bearing sidecar, and the exact single installed
/// USMAP, then applies the offset-free semantic edit in memory. The returned value deliberately
/// reuses [`VerifiedFixedLeafStageInput`] and its generation-v1 revalidation contract, so this
/// grants no deployment, build, runtime, or project-publication authority.
///
/// Both an install root and its direct `G1R` child are accepted. No game or caller-selected output
/// path is written; the live conversion uses only the workflow's private disjoint temporary tree.
pub fn verify_fixed_leaf_stage_edit_from_installed_snapshot_v1(
    game_root: &Path,
    extraction: VerifiedInstalledPackageExtractionV1<'_>,
    installed_usmap: &VerifiedInstalledUsmapV1,
    selector: FixedLeafSelector,
    replacement_hex: &str,
) -> Result<VerifiedFixedLeafStageInput> {
    extraction
        .revalidate_source()
        .context("ASSET_STAGE_INSTALLED_SOURCE: preflight source snapshot changed")?;
    installed_usmap
        .revalidate()
        .context("ASSET_STAGE_INSTALLED_USMAP: preflight installed USMAP changed")?;

    let mut installed_sidecars = BTreeMap::new();
    for sidecar in extraction.sidecars() {
        let role = match sidecar.role() {
            InstalledPackageSidecarRoleV1::Bulk => SidecarRole::Bulk,
            InstalledPackageSidecarRoleV1::Optional => SidecarRole::Optional,
            InstalledPackageSidecarRoleV1::MemoryMapped => SidecarRole::MemoryMapped,
        };
        if installed_sidecars.insert(role, sidecar.bytes()).is_some() {
            bail!("ASSET_STAGE_INSTALLED_SIDECAR: duplicate installed sidecar role");
        }
    }
    let installed_source =
        InstalledGenerationSourceProof::from_installed(extraction.source_evidence());

    let result = verify_fixed_leaf_stage_edit_from_installed_parts_with_live_source(
        game_root,
        extraction.target_path(),
        extraction.uasset_bytes(),
        extraction.uexp_bytes(),
        &installed_sidecars,
        &installed_source,
        installed_usmap.bytes(),
        installed_usmap.selected_file_name(),
        selector,
        replacement_hex,
        |root, asset| {
            capture_live_converted_stage_source(root, asset, "ASSET_STAGE_INSTALLED_GENERATION")
        },
        |root, asset, expected| {
            probe_current_generation_receipt(
                root,
                asset,
                expected,
                "ASSET_STAGE_INSTALLED_GENERATION",
            )
        },
    );

    // Drift wins over parsing or patch diagnostics. Both retained authorities are checked even
    // when the inner operation failed, and no stage value escapes unless each stayed exact.
    let source_revalidation = extraction.revalidate_source();
    let usmap_revalidation = installed_usmap.revalidate();
    if let Err(error) = source_revalidation {
        return Err(error)
            .context("ASSET_STAGE_INSTALLED_SOURCE: source snapshot changed during verification");
    }
    if let Err(error) = usmap_revalidation {
        return Err(error)
            .context("ASSET_STAGE_INSTALLED_USMAP: installed USMAP changed during verification");
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn verify_fixed_leaf_stage_edit_from_installed_parts_with_live_source<F, G>(
    game_root: &Path,
    target_path: &str,
    installed_uasset: &[u8],
    installed_uexp: &[u8],
    installed_sidecars: &BTreeMap<SidecarRole, &[u8]>,
    installed_source: &InstalledGenerationSourceProof,
    installed_usmap: &[u8],
    installed_usmap_file_name: &str,
    selector: FixedLeafSelector,
    replacement_hex: &str,
    live_source: F,
    final_generation_probe: G,
) -> Result<VerifiedFixedLeafStageInput>
where
    F: FnOnce(&Path, &str) -> Result<LiveConvertedStageSource>,
    G: FnOnce(&Path, &str, &AssetGenerationReceipt) -> Result<AssetGenerationReceipt>,
{
    let width = selector.kind.width();
    if !is_canonical_hex(replacement_hex, width) {
        bail!(
            "ASSET_STAGE_INSTALLED_REPLACEMENT: replacement must be exactly {width} lowercase wire bytes"
        );
    }
    let expected = selector
        .expected_bytes()
        .context("ASSET_STAGE_INSTALLED_SELECTOR: decoding sealed current value")?;
    let replacement =
        decode_canonical_hex(replacement_hex, width, "ASSET_STAGE_INSTALLED_REPLACEMENT")?;
    if replacement == expected {
        bail!("ASSET_STAGE_INSTALLED_REPLACEMENT: replacement would make no change");
    }

    let game_root = normalize_game_install_root(game_root, "ASSET_STAGE_INSTALLED_ROOT")?;
    let executable = seal_game_executable(&game_root)?;
    let live = live_source(&game_root, target_path)?;
    if live.generation.asset != target_path {
        bail!("ASSET_STAGE_INSTALLED_GENERATION: live conversion selected a different target");
    }
    if !installed_source.matches(&live.generation) {
        bail!(
            "ASSET_STAGE_INSTALLED_SOURCE: installed chunk winners or source UTOCs differ from the independent live generation"
        );
    }
    if live.uasset != installed_uasset || live.uexp != installed_uexp {
        bail!(
            "ASSET_STAGE_INSTALLED_GENERATION: installed package differs from an independent live conversion"
        );
    }
    if live.sidecars.len() != installed_sidecars.len()
        || live.sidecars.iter().any(|(role, bytes)| {
            installed_sidecars
                .get(role)
                .is_none_or(|installed| *installed != bytes.as_slice())
        })
    {
        bail!(
            "ASSET_STAGE_INSTALLED_GENERATION: installed sidecars differ from an independent live conversion"
        );
    }

    let live_usmap_path = gore_tex::paths::usmap(&game_root)
        .context("ASSET_STAGE_INSTALLED_USMAP: resolving exact live USMAP")?;
    let live_usmap = read_verified_file_bounded(
        &live_usmap_path,
        MAX_USMAP_BYTES,
        "ASSET_STAGE_INSTALLED_USMAP",
    )?;
    if live.generation.usmap.file_name != installed_usmap_file_name
        || !live.generation.usmap.matches_verified_input(&live_usmap)
        || live_usmap.bytes() != installed_usmap
    {
        bail!(
            "ASSET_STAGE_INSTALLED_USMAP: installed USMAP differs from the independently reconstructed generation"
        );
    }

    let schemas = SchemaDb::from_usmap_bounded(live_usmap.bytes(), UsmapLimits::default())
        .context("ASSET_STAGE_INSTALLED_USMAP: parsing exact installed USMAP")?;
    let LiveConvertedStageSource {
        generation,
        uasset,
        uexp,
        sidecars: live_sidecars,
    } = live;
    let mut patched = PackageCarrier::from_bytes(uasset, uexp, asset_package_limits())
        .context("ASSET_STAGE_INSTALLED_INPUT: loading exact installed package pair")?;
    apply_fixed_leaf_selector_patch(&mut patched, &schemas, &selector, &expected, &replacement)
        .context("ASSET_STAGE_INSTALLED_SEMANTICS: applying offset-free fixed-leaf edit")?;

    let mut cooked_bytes = u64::try_from(patched.len(PackageComponent::Uasset))?
        .checked_add(u64::try_from(patched.len(PackageComponent::Uexp))?)
        .context("ASSET_STAGE_INSTALLED_INPUT: cooked package size overflowed")?;
    let mut sidecars = Vec::with_capacity(live_sidecars.len());
    for (role, bytes) in live_sidecars {
        let length = u64::try_from(bytes.len())?;
        if length > MAX_OPTIONAL_SIDECAR_BYTES {
            bail!("ASSET_STAGE_INSTALLED_SIDECAR: sidecar exceeds the per-component size limit");
        }
        cooked_bytes = cooked_bytes
            .checked_add(length)
            .context("ASSET_STAGE_INSTALLED_INPUT: cooked package size overflowed")?;
        if cooked_bytes > MAX_COOKED_PACKAGE_BYTES {
            bail!("ASSET_STAGE_INSTALLED_INPUT: cooked package exceeds the aggregate size limit");
        }
        let sha256 = Sha256::digest(&bytes).into();
        sidecars.push(VerifiedFixedLeafStageSidecar {
            role,
            bytes,
            sha256,
        });
    }

    // Recheck the exact USMAP bytes, executable, and complete target generation after parsing and
    // patching. The retained installed guards are checked once more by the public wrapper.
    verify_file_hash(
        live_usmap.path(),
        live_usmap.length(),
        *live_usmap.sha256(),
        MAX_USMAP_BYTES,
        "ASSET_STAGE_INSTALLED_USMAP",
    )?;
    executable.reverify()?;
    let current_generation = final_generation_probe(&game_root, target_path, &generation)?;
    if current_generation != generation {
        bail!(
            "ASSET_STAGE_INSTALLED_GENERATION: live target generation changed during verification"
        );
    }
    let (patched_uasset, patched_uexp) = patched.into_bytes();

    Ok(VerifiedFixedLeafStageInput {
        target_path: target_path.to_owned(),
        generation,
        selector,
        replacement_hex: replacement_hex.to_owned(),
        patched_uasset,
        patched_uexp,
        usmap: live_usmap.bytes,
        sidecars,
        game_root,
        retained_source_roots: Vec::new(),
        executable,
    })
}

fn verify_fixed_leaf_stage_edit_with_live_source_in<F, G>(
    extract: VerifiedExtractReceipt,
    selector: FixedLeafSelector,
    replacement_hex: &str,
    private_parent: &Path,
    live_source: F,
    final_generation_probe: G,
) -> Result<VerifiedFixedLeafStageInput>
where
    F: FnOnce(&Path, &str, &AssetGenerationReceipt) -> Result<LiveConvertedStageSource>,
    G: FnOnce(&Path, &str, &AssetGenerationReceipt) -> Result<AssetGenerationReceipt>,
{
    let (private_patch, patch) =
        materialize_private_fixed_leaf_patch(extract, selector, replacement_hex, private_parent)?;
    let mut verified =
        verify_fixed_leaf_stage_input_with_live_source(patch, live_source, final_generation_probe)?;
    let private_root =
        validate_existing_path_no_reparse(private_patch.path(), true, "ASSET_STAGE_EDIT_TEMP")?;
    verified
        .retained_source_roots
        .retain(|root| root != &private_root);
    private_patch
        .close()
        .context("ASSET_STAGE_EDIT_TEMP: removing private patch chain")?;
    Ok(verified)
}

fn materialize_private_fixed_leaf_patch(
    extract: VerifiedExtractReceipt,
    selector: FixedLeafSelector,
    replacement_hex: &str,
    private_parent: &Path,
) -> Result<(tempfile::TempDir, VerifiedPatchReceipt)> {
    let receipt = extract.receipt();
    let binding = extract.binding();
    let game_root = Path::new(&receipt.source.game_root);
    let private = create_disjoint_private_conversion_dir_in(
        game_root,
        private_parent,
        "ASSET_STAGE_EDIT_TEMP",
    )?;

    let usmap_path = binding
        .output_root()
        .join(&binding.copied_usmap().relative_path);
    let usmap = read_verified_file_bounded(&usmap_path, MAX_USMAP_BYTES, "ASSET_STAGE_EDIT_USMAP")?;
    if !receipt.generation.usmap.matches_verified_input(&usmap) {
        bail!("ASSET_STAGE_EDIT_USMAP: copied USMAP differs from the extract generation");
    }
    let schemas = SchemaDb::from_usmap_bounded(usmap.bytes(), UsmapLimits::default())
        .context("ASSET_STAGE_EDIT_USMAP: parsing exact copied USMAP")?;

    let source_uasset = binding.output_root().join(&binding.uasset().relative_path);
    let mut carrier = PackageCarrier::load(&source_uasset, asset_package_limits())
        .context("ASSET_STAGE_EDIT_INPUT: loading extracted package pair")?;
    let input_package_seal = PackagePairSeal::capture(&carrier);
    if input_package_seal != receipt.package_seal {
        bail!("ASSET_STAGE_EDIT_INPUT: extracted package differs from its receipt");
    }
    validate_extract_receipt_components(&extract, &carrier, &usmap)
        .context("ASSET_STAGE_EDIT_INPUT: validating the complete extracted artifact set")?;

    let width = selector.kind.width();
    if !is_canonical_hex(replacement_hex, width) {
        bail!(
            "ASSET_STAGE_EDIT_REPLACEMENT: replacement must be exactly {width} lowercase wire bytes"
        );
    }
    let expected = selector
        .expected_bytes()
        .context("ASSET_STAGE_EDIT_SELECTOR: decoding sealed current value")?;
    let replacement = decode_canonical_hex(replacement_hex, width, "ASSET_STAGE_EDIT_REPLACEMENT")?;
    let patch =
        apply_fixed_leaf_selector_patch(&mut carrier, &schemas, &selector, &expected, &replacement)
            .context("ASSET_STAGE_EDIT_SEMANTICS: applying offset-free fixed-leaf edit")?;
    let output_package_seal = PackagePairSeal::capture(&carrier);

    let output_uasset = private.path().join("Patched.uasset");
    let written = carrier
        .write_new(&output_uasset)
        .context("ASSET_STAGE_EDIT_OUTPUT: writing private patched pair")?;

    let mut output_sidecars = Vec::with_capacity(binding.sidecars().len());
    for sidecar in binding.sidecars() {
        let source = binding.output_root().join(&sidecar.file_name);
        let input = read_verified_file_bounded(
            &source,
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_STAGE_EDIT_SIDECAR",
        )?;
        if input.length() != sidecar.length || encode_hex(input.sha256()) != sidecar.sha256 {
            bail!("ASSET_STAGE_EDIT_SIDECAR: extracted sidecar changed after validation");
        }
        let (file_name, output) =
            sidecar_path(&output_uasset, sidecar.role, "ASSET_STAGE_EDIT_SIDECAR")?;
        write_private_new(&output, input.bytes(), "ASSET_STAGE_EDIT_SIDECAR")?;
        output_sidecars.push(SidecarReceipt {
            role: sidecar.role,
            file_name,
            length: input.length(),
            sha256: encode_hex(input.sha256()),
        });
    }

    let receipt_path = private
        .path()
        .join(format!("Patched{PATCH_RECEIPT_SUFFIX}"));
    let patch_receipt = PatchReceiptEnvelope {
        format: "gore.asset.patch-fixed.v2".to_owned(),
        status: "patched".to_owned(),
        asset: receipt.asset.clone(),
        generation_bound: true,
        provenance: PatchReceiptProvenance {
            extract_receipt: ReceiptFileSeal {
                path: extract.input().path().display().to_string(),
                length: extract.input().length(),
                sha256: encode_hex(extract.input().sha256()),
            },
            generation: receipt.generation.clone(),
            usmap: GenerationFileAnchor {
                file_name: binding.copied_usmap().relative_path.clone(),
                length: binding.copied_usmap().length,
                sha256: binding.copied_usmap().sha256.clone(),
            },
            extract_components: binding.components().to_vec(),
            extracted_sidecars: binding.sidecars().to_vec(),
        },
        input_package_seal,
        output_package_seal,
        output_sidecars: output_sidecars.clone(),
        input_selector: selector,
        output_requires_reinspect: true,
        expected_hex: encode_hex(&expected),
        replacement_hex: encode_hex(&replacement),
        patch: PatchOperationProof {
            before: patch.before,
            after: patch.after,
            export_index: patch.export_index,
            component: patch.component,
            absolute_offset: patch.absolute_offset,
            length: patch.length,
            kind: patch.kind,
        },
        output: PatchReceiptOutput {
            uasset: ComponentDigestProof {
                path: canonical_leaf_path(&written.uasset.path, "ASSET_STAGE_EDIT_OUTPUT")?
                    .display()
                    .to_string(),
                length: written.uasset.length,
                sha256: encode_hex(&written.uasset.sha256),
            },
            uexp: ComponentDigestProof {
                path: canonical_leaf_path(&written.uexp.path, "ASSET_STAGE_EDIT_OUTPUT")?
                    .display()
                    .to_string(),
                length: written.uexp.length,
                sha256: encode_hex(&written.uexp.sha256),
            },
            sidecars: output_sidecars,
            receipt: receipt_path.display().to_string(),
        },
    };
    validate_patch_receipt_envelope(&patch_receipt, &receipt_path)?;
    let bytes = serde_json::to_vec(&patch_receipt)
        .context("ASSET_STAGE_EDIT_RECEIPT: serializing private PatchReceipt v2")?;
    if bytes.len() > MAX_RECEIPT_BYTES as usize {
        bail!("ASSET_STAGE_EDIT_RECEIPT: private PatchReceipt exceeds its resource limit");
    }
    write_private_new(&receipt_path, &bytes, "ASSET_STAGE_EDIT_RECEIPT")?;
    let verified = read_patch_receipt_v2(&receipt_path)?;
    Ok((private, verified))
}

fn write_private_new(path: &Path, bytes: &[u8], code: &'static str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("{code}: creating private output"))?;
    file.write_all(bytes)
        .with_context(|| format!("{code}: writing private output"))?;
    file.sync_all()
        .with_context(|| format!("{code}: syncing private output"))?;
    Ok(())
}

fn verify_fixed_leaf_stage_input_with_live_source<F, G>(
    patch: VerifiedPatchReceipt,
    live_source: F,
    final_generation_probe: G,
) -> Result<VerifiedFixedLeafStageInput>
where
    F: FnOnce(&Path, &str, &AssetGenerationReceipt) -> Result<LiveConvertedStageSource>,
    G: FnOnce(&Path, &str, &AssetGenerationReceipt) -> Result<AssetGenerationReceipt>,
{
    let extract = read_chained_extract_receipt(&patch)?;
    let patch_receipt = &patch.receipt;
    let extract_receipt = &extract.receipt;
    let extract_binding = &extract.binding;
    if extract_receipt.asset != patch_receipt.asset
        || extract_receipt.generation != patch_receipt.provenance.generation
        || extract_receipt.package_seal != patch_receipt.input_package_seal
        || extract_binding.components() != patch_receipt.provenance.extract_components
        || extract_binding.sidecars() != patch_receipt.provenance.extracted_sidecars
    {
        bail!(
            "ASSET_STAGE_INPUT: chained extract asset, generation, package, or component provenance mismatch"
        );
    }
    if patch_receipt.provenance.usmap.file_name != extract_binding.copied_usmap().relative_path
        || patch_receipt.provenance.usmap.length != extract_binding.copied_usmap().length
        || patch_receipt.provenance.usmap.sha256 != extract_binding.copied_usmap().sha256
    {
        bail!("ASSET_STAGE_INPUT: copied USMAP provenance mismatch");
    }
    if patch_receipt.output_sidecars.len() != extract_binding.sidecars().len()
        || patch_receipt
            .output_sidecars
            .iter()
            .zip(extract_binding.sidecars())
            .any(|(output, extracted)| {
                output.role != extracted.role
                    || output.length != extracted.length
                    || output.sha256 != extracted.sha256
            })
    {
        bail!("ASSET_STAGE_INPUT: patched sidecars differ from extracted provenance");
    }
    validate_sidecar_generation_mapping(
        &patch_receipt.output_sidecars,
        &patch_receipt.provenance.generation,
        "ASSET_STAGE_INPUT",
    )?;

    let usmap_path = extract_binding
        .output_root()
        .join(&extract_binding.copied_usmap().relative_path);
    let usmap = read_verified_file_bounded(&usmap_path, MAX_USMAP_BYTES, "ASSET_STAGE_USMAP")?;
    if !patch_receipt
        .provenance
        .usmap
        .matches_verified_input(&usmap)
    {
        bail!("ASSET_STAGE_INPUT: copied USMAP changed after receipt validation");
    }

    let original_uasset = extract_binding
        .output_root()
        .join(&extract_binding.uasset().relative_path);
    let mut reproduced = PackageCarrier::load(&original_uasset, asset_package_limits())
        .context("ASSET_STAGE_INPUT: loading original extracted package pair")?;
    let original_sidecars = validate_extract_receipt_components(&extract, &reproduced, &usmap)?;
    if original_sidecars.len() != extract_binding.sidecars().len() {
        bail!("ASSET_STAGE_INPUT: verified extracted sidecar set changed");
    }

    let game_root = validate_existing_path_no_reparse(
        Path::new(&extract_receipt.source.game_root),
        true,
        "ASSET_STAGE_GAME_ROOT",
    )?;
    // Seal the executable before touching the live containers so the final checks can prove that
    // neither side of the target binding changed across the complete live reconstruction.
    let executable = seal_game_executable(&game_root)?;
    let live = live_source(
        &game_root,
        &patch_receipt.asset,
        &patch_receipt.provenance.generation,
    )?;
    if live.generation != patch_receipt.provenance.generation {
        bail!(
            "ASSET_STAGE_GENERATION: converted live target generation differs from PatchReceipt v2"
        );
    }
    for component in [PackageComponent::Uasset, PackageComponent::Uexp] {
        let live_bytes = match component {
            PackageComponent::Uasset => &live.uasset,
            PackageComponent::Uexp => &live.uexp,
        };
        if reproduced.bytes(component) != live_bytes {
            bail!("ASSET_STAGE_GENERATION: extracted package differs from a fresh live conversion");
        }
    }
    let mut extracted_sidecars = BTreeMap::new();
    for receipt in extract_binding.sidecars() {
        let path = extract_binding.output_root().join(&receipt.file_name);
        let input = read_verified_file_bounded(
            &path,
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_STAGE_EXTRACTED_SIDECAR",
        )?;
        if input.length() != receipt.length || encode_hex(input.sha256()) != receipt.sha256 {
            bail!("ASSET_STAGE_GENERATION: extracted sidecar changed after validation");
        }
        if extracted_sidecars
            .insert(receipt.role, input.bytes)
            .is_some()
        {
            bail!("ASSET_STAGE_GENERATION: duplicate extracted sidecar role");
        }
    }
    if live.sidecars != extracted_sidecars {
        bail!("ASSET_STAGE_GENERATION: extracted sidecars differ from a fresh live conversion");
    }

    let schemas = SchemaDb::from_usmap_bounded(usmap.bytes(), UsmapLimits::default())
        .context("ASSET_STAGE_USMAP: parsing exact copied USMAP")?;
    let expected = patch_receipt
        .input_selector
        .expected_bytes()
        .context("ASSET_STAGE_SELECTOR: decoding expected bytes")?;
    let replacement = decode_canonical_hex(
        &patch_receipt.replacement_hex,
        patch_receipt.input_selector.kind.width(),
        "ASSET_STAGE_REPLACEMENT",
    )?;
    let reproduced_patch = apply_fixed_leaf_selector_patch(
        &mut reproduced,
        &schemas,
        &patch_receipt.input_selector,
        &expected,
        &replacement,
    )
    .context("ASSET_STAGE_SEMANTICS: reproducing fixed-leaf patch")?;
    if reproduced_patch.before != patch_receipt.patch.before
        || reproduced_patch.after != patch_receipt.patch.after
        || reproduced_patch.export_index != patch_receipt.patch.export_index
        || reproduced_patch.component != patch_receipt.patch.component
        || reproduced_patch.absolute_offset != patch_receipt.patch.absolute_offset
        || reproduced_patch.length != patch_receipt.patch.length
        || reproduced_patch.kind != patch_receipt.patch.kind
    {
        bail!("ASSET_STAGE_SEMANTICS: reproduced patch proof differs from PatchReceipt v2");
    }

    let patched_pair = PackageCarrier::load(
        Path::new(&patch_receipt.output.uasset.path),
        asset_package_limits(),
    )
    .context("ASSET_STAGE_INPUT: loading patched package pair")?;
    let source = patched_pair
        .source_paths()
        .context("ASSET_STAGE_INPUT: patched pair has no source paths")?;
    let extract_root = validate_existing_path_no_reparse(
        extract_binding.output_root(),
        true,
        "ASSET_STAGE_EXTRACT_ROOT",
    )?;
    let patched_root = source
        .uasset()
        .parent()
        .context("ASSET_STAGE_INPUT: patched pair has no parent")?;
    let patched_root =
        validate_existing_path_no_reparse(patched_root, true, "ASSET_STAGE_PATCH_ROOT")?;
    if PackagePairSeal::capture(&patched_pair) != patch_receipt.output_package_seal {
        bail!("ASSET_STAGE_INPUT: patched pair differs from PatchReceipt v2 output seal");
    }
    for component in [PackageComponent::Uasset, PackageComponent::Uexp] {
        if reproduced.bytes(component) != patched_pair.bytes(component) {
            bail!(
                "ASSET_STAGE_SEMANTICS: patched package contains bytes outside the reproduced fixed-leaf edit"
            );
        }
    }
    validate_patch_output_against_carrier(&patch, source.uasset(), source.uexp(), &patched_pair)?;
    let pair_bytes = u64::try_from(patched_pair.len(PackageComponent::Uasset))?
        .checked_add(u64::try_from(patched_pair.len(PackageComponent::Uexp))?)
        .context("ASSET_STAGE_INPUT: patched pair size overflowed")?;
    let sidecar_seals = validate_patched_sidecars(&patch, source.uasset(), pair_bytes)?;
    if sidecar_seals.len() != patch_receipt.output_sidecars.len() {
        bail!("ASSET_STAGE_INPUT: verified sidecar set changed");
    }
    let mut sidecars = Vec::with_capacity(sidecar_seals.len());
    for (receipt, seal) in patch_receipt.output_sidecars.iter().zip(sidecar_seals) {
        let input = read_verified_file_bounded(
            seal.path(),
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_STAGE_SIDECAR",
        )?;
        if input.length() != seal.length() || input.sha256() != seal.sha256() {
            bail!("ASSET_STAGE_INPUT: patched sidecar changed while being captured");
        }
        let sha256 = *input.sha256();
        sidecars.push(VerifiedFixedLeafStageSidecar {
            role: receipt.role,
            bytes: input.bytes,
            sha256,
        });
    }

    let patched_uasset = patched_pair.bytes(PackageComponent::Uasset).to_vec();
    let patched_uexp = patched_pair.bytes(PackageComponent::Uexp).to_vec();
    let usmap = usmap.bytes;

    // This is deliberately the final filesystem operation. The live conversion may take long
    // enough for either the executable or a container to change after its first seal; re-hash the
    // executable and then union-probe the complete live target generation immediately before the
    // opaque authority is returned.
    executable.reverify()?;
    let current_generation = final_generation_probe(
        &game_root,
        &patch_receipt.asset,
        &patch_receipt.provenance.generation,
    )?;
    if current_generation != patch_receipt.provenance.generation {
        bail!("ASSET_STAGE_GENERATION: live target generation changed during verification");
    }

    Ok(VerifiedFixedLeafStageInput {
        target_path: patch_receipt.asset.clone(),
        generation: patch_receipt.provenance.generation.clone(),
        selector: patch_receipt.input_selector.clone(),
        replacement_hex: patch_receipt.replacement_hex.clone(),
        patched_uasset,
        patched_uexp,
        usmap,
        sidecars,
        game_root,
        retained_source_roots: if patched_root == extract_root {
            vec![extract_root]
        } else {
            vec![extract_root, patched_root]
        },
        executable,
    })
}

/// Apply one offset-free selector edit through the same semantic path used by the CLI and managed
/// staging verification.
#[derive(Debug, Clone, Copy)]
pub struct FixedLeafPatchDiagnosticCodes {
    pub envelope: &'static str,
    pub export: &'static str,
    pub schema: &'static str,
    pub walk: &'static str,
    pub selector: &'static str,
    pub replacement: &'static str,
    pub drift: &'static str,
}

pub fn apply_fixed_leaf_selector_patch(
    carrier: &mut PackageCarrier,
    schemas: &SchemaDb,
    selector: &FixedLeafSelector,
    expected: &[u8],
    replacement: &[u8],
) -> Result<FixedLeafPatchReceipt> {
    apply_fixed_leaf_selector_patch_with_codes(
        carrier,
        schemas,
        selector,
        expected,
        replacement,
        FixedLeafPatchDiagnosticCodes {
            envelope: "ASSET_FIXED_LEAF_ENVELOPE",
            export: "ASSET_FIXED_LEAF_EXPORT",
            schema: "ASSET_FIXED_LEAF_SCHEMA",
            walk: "ASSET_FIXED_LEAF_WALK",
            selector: "ASSET_FIXED_LEAF_SELECTOR",
            replacement: "ASSET_FIXED_LEAF_REPLACEMENT",
            drift: "ASSET_FIXED_LEAF_DRIFT",
        },
    )
}

pub fn apply_fixed_leaf_selector_patch_with_codes(
    carrier: &mut PackageCarrier,
    schemas: &SchemaDb,
    selector: &FixedLeafSelector,
    expected: &[u8],
    replacement: &[u8],
    codes: FixedLeafPatchDiagnosticCodes,
) -> Result<FixedLeafPatchReceipt> {
    let patch = {
        let package = LegacyPackageEnvelope::parse_g1r_ue5_4(carrier).context(codes.envelope)?;
        let export = package
            .export(selector.export_index)
            .context(codes.export)?;
        let schema_id = export
            .boundary()
            .resolve_class_schema(schemas)
            .context(codes.schema)?;
        let block = PropertySpanWalker::g1r_ue5_4(schemas)
            .walk(export.bytes(), schema_id)
            .context(codes.walk)?;
        let leaf = selector
            .resolve(carrier, &export, schemas)
            .context(codes.selector)?;
        FixedLeafPatch::plan(
            carrier,
            &export,
            schemas,
            &block,
            &leaf,
            expected,
            replacement,
        )
        .context(codes.replacement)?
    };
    patch.apply(carrier, schemas).context(codes.drift)
}

struct LiveConvertedStageSource {
    generation: AssetGenerationReceipt,
    uasset: Vec<u8>,
    uexp: Vec<u8>,
    sidecars: BTreeMap<SidecarRole, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstalledGenerationSourceProof {
    container_set: Vec<GenerationFileAnchor>,
    target_chunks: Vec<GenerationChunkAnchor>,
}

impl InstalledGenerationSourceProof {
    fn from_installed(evidence: &InstalledPackageSourceEvidenceV1) -> Self {
        let mut container_set = evidence
            .metadata_utocs()
            .iter()
            .map(|utoc| GenerationFileAnchor {
                file_name: utoc.file_name().to_owned(),
                length: utoc.content_seal().byte_len,
                sha256: utoc.content_seal().sha256.clone(),
            })
            .collect::<Vec<_>>();
        container_set.sort_by(|left, right| {
            left.file_name
                .cmp(&right.file_name)
                .then(left.sha256.cmp(&right.sha256))
        });
        container_set.dedup();

        let mut target_chunks = evidence
            .consumed_chunks()
            .iter()
            .map(|chunk| GenerationChunkAnchor {
                chunk_id: chunk.chunk_id().to_owned(),
                chunk_type: chunk.chunk_type().to_owned(),
                winner_utoc: GenerationFileAnchor {
                    file_name: chunk.winner_utoc().file_name().to_owned(),
                    length: chunk.winner_utoc().content_seal().byte_len,
                    sha256: chunk.winner_utoc().content_seal().sha256.clone(),
                },
                length: chunk.length(),
                blake3: chunk.blake3().to_owned(),
                toc_hash: chunk.toc_hash().to_owned(),
                toc_hash_bytes: chunk.toc_hash_bytes(),
            })
            .collect::<Vec<_>>();
        target_chunks.sort_by(|left, right| {
            left.chunk_id
                .cmp(&right.chunk_id)
                .then(left.chunk_type.cmp(&right.chunk_type))
                .then(left.winner_utoc.file_name.cmp(&right.winner_utoc.file_name))
        });
        Self {
            container_set,
            target_chunks,
        }
    }

    fn matches(&self, generation: &AssetGenerationReceipt) -> bool {
        self.container_set == generation.container_set
            && self.target_chunks == generation.target_chunks
            && self
                .container_set
                .iter()
                .any(|utoc| utoc == &generation.main_utoc)
            && self
                .container_set
                .iter()
                .any(|utoc| utoc == &generation.global_utoc)
    }

    #[cfg(test)]
    fn from_generation(generation: &AssetGenerationReceipt) -> Self {
        Self {
            container_set: generation.container_set.clone(),
            target_chunks: generation.target_chunks.clone(),
        }
    }
}

fn normalize_game_install_root(game_root: &Path, code: &'static str) -> Result<PathBuf> {
    let root = validate_existing_path_no_reparse(game_root, true, code)?;
    if root.file_name().is_some_and(|name| name == "G1R") {
        let parent = root
            .parent()
            .context("ASSET_STAGE_INSTALLED_ROOT: direct G1R root has no install parent")?;
        let parent = validate_existing_path_no_reparse(parent, true, code)?;
        let canonical_g1r = validate_existing_path_no_reparse(&parent.join("G1R"), true, code)?;
        if canonical_g1r != root {
            bail!("{code}: direct G1R root does not belong to the normalized install parent");
        }
        Ok(parent)
    } else {
        let canonical_g1r = validate_existing_path_no_reparse(&root.join("G1R"), true, code)
            .with_context(|| format!("{code}: resolving the direct G1R directory"))?;
        if canonical_g1r.parent() != Some(root.as_path()) {
            bail!("{code}: G1R directory escaped the normalized install root");
        }
        Ok(root)
    }
}

fn create_disjoint_private_conversion_dir(
    game_root: &Path,
    code: &'static str,
) -> Result<tempfile::TempDir> {
    create_disjoint_private_conversion_dir_in(game_root, &std::env::temp_dir(), code)
}

fn create_disjoint_private_conversion_dir_in(
    game_root: &Path,
    temp_parent: &Path,
    code: &'static str,
) -> Result<tempfile::TempDir> {
    let game_root = validate_existing_path_no_reparse(game_root, true, code)?;
    let temp_parent = validate_existing_path_no_reparse(temp_parent, true, code)
        .with_context(|| format!("{code}: validating private conversion parent"))?;
    if temp_parent.starts_with(&game_root) || game_root.starts_with(&temp_parent) {
        bail!("{code}: private conversion parent and live game tree must be disjoint");
    }

    let temp = tempfile::Builder::new()
        .prefix("gore-stage-live-")
        .tempdir_in(&temp_parent)
        .with_context(|| format!("{code}: creating private conversion directory"))?;
    let created = validate_existing_path_no_reparse(temp.path(), true, code)
        .with_context(|| format!("{code}: validating private conversion directory"))?;
    if !created.starts_with(&temp_parent)
        || created.starts_with(&game_root)
        || game_root.starts_with(&created)
    {
        bail!("{code}: private conversion directory escaped its verified parent");
    }
    Ok(temp)
}

fn capture_live_converted_stage_source(
    game_root: &Path,
    asset: &str,
    code: &'static str,
) -> Result<LiveConvertedStageSource> {
    let game_root = validate_existing_path_no_reparse(game_root, true, code)?;
    let main_utoc = gore_tex::paths::main_container(&game_root)
        .with_context(|| format!("{code}: resolving main UTOC"))?;
    let live_usmap = gore_tex::paths::usmap(&game_root)
        .with_context(|| format!("{code}: resolving live USMAP"))?;
    let main_utoc_seal =
        digest_regular_file_bounded(&main_utoc, MAX_CONTAINER_COMPONENT_BYTES, code)?;
    let usmap_seal = digest_regular_file_bounded(&live_usmap, MAX_USMAP_BYTES, code)?;
    let global_utoc_seal = digest_regular_file_bounded(
        &game_root.join("G1R/Content/Paks/global.utoc"),
        MAX_CONTAINER_COMPONENT_BYTES,
        code,
    )?;
    let global_ucas_seal = digest_regular_file_bounded(
        &game_root.join("G1R/Content/Paks/global.ucas"),
        MAX_CONTAINER_COMPONENT_BYTES,
        code,
    )?;

    let temp = create_disjoint_private_conversion_dir(&game_root, code)?;
    let unpacked = gore_tex::container::unpack_asset_verified(
        &main_utoc_seal.path,
        &usmap_seal.path,
        asset,
        temp.path(),
    )
    .with_context(|| format!("{code}: converting the live target package"))?;
    let mut source_utoc_seals = Vec::with_capacity(unpacked.metadata_utocs.len());
    for source_utoc in &unpacked.metadata_utocs {
        source_utoc_seals.push(digest_regular_file_bounded(
            source_utoc,
            MAX_CONTAINER_COMPONENT_BYTES,
            code,
        )?);
    }
    let generation = build_generation_receipt_from_probe(
        asset,
        &usmap_seal,
        &main_utoc_seal,
        &global_utoc_seal,
        &global_ucas_seal,
        &unpacked.consumed_chunks,
        &source_utoc_seals,
        &unpacked.opened_utocs,
        code,
    )?;

    let pair = PackageCarrier::load(&unpacked.uasset, asset_package_limits())
        .with_context(|| format!("{code}: reopening freshly converted package"))?;
    let (uasset, uexp) = pair.into_bytes();
    let mut cooked_bytes = u64::try_from(uasset.len())?
        .checked_add(u64::try_from(uexp.len())?)
        .context("ASSET_STAGE_GENERATION: converted package size overflowed")?;
    let mut sidecars = BTreeMap::new();
    for role in SidecarRole::ALL {
        let (_, path) = sidecar_path(&unpacked.uasset, role, code)?;
        let Some(seal) =
            digest_optional_regular_file_bounded(&path, MAX_OPTIONAL_SIDECAR_BYTES, code)?
        else {
            continue;
        };
        let input = read_verified_file_bounded(&path, MAX_OPTIONAL_SIDECAR_BYTES, code)?;
        if input.length() != seal.length() || input.sha256() != seal.sha256() {
            bail!("{code}: converted sidecar changed while being captured");
        }
        cooked_bytes = cooked_bytes
            .checked_add(input.length())
            .context("ASSET_STAGE_GENERATION: converted cooked size overflowed")?;
        if cooked_bytes > MAX_COOKED_PACKAGE_BYTES {
            bail!("{code}: converted cooked package exceeds aggregate size limit");
        }
        if sidecars.insert(role, input.bytes).is_some() {
            bail!("{code}: duplicate converted sidecar role");
        }
    }

    for seal in [
        &usmap_seal,
        &main_utoc_seal,
        &global_utoc_seal,
        &global_ucas_seal,
    ]
    .into_iter()
    .chain(source_utoc_seals.iter())
    {
        let limit = if seal.path.extension().is_some_and(|value| value == "usmap") {
            MAX_USMAP_BYTES
        } else {
            MAX_CONTAINER_COMPONENT_BYTES
        };
        seal.reverify(limit, code)?;
    }

    drop(unpacked);
    temp.close()
        .context("ASSET_STAGE_GENERATION: removing private conversion directory")?;

    Ok(LiveConvertedStageSource {
        generation,
        uasset,
        uexp,
        sidecars,
    })
}

/// Reproduce the exact current asset generation from a live game tree.
///
/// `expected` contributes allowed dependency chunk IDs, but cannot narrow the live target package:
/// every currently present target export/bulk/optional/memory-mapped chunk is also re-read. Every
/// returned digest and winning-container fact is freshly derived from the live IoStore and bounded
/// regular files; callers must compare the complete returned receipt with their expected receipt.
pub fn probe_current_generation_receipt(
    game_root: &Path,
    asset: &str,
    expected: &AssetGenerationReceipt,
    code: &'static str,
) -> Result<AssetGenerationReceipt> {
    validate_generation_receipt(expected, code)?;
    if expected.asset != asset {
        bail!("{code}: expected generation targets a different asset");
    }
    let game_root = validate_existing_path_no_reparse(game_root, true, code)?;
    let main_utoc = gore_tex::paths::main_container(&game_root)
        .with_context(|| format!("{code}: resolving main UTOC"))?;
    let usmap =
        gore_tex::paths::usmap(&game_root).with_context(|| format!("{code}: resolving USMAP"))?;
    let main_utoc_seal =
        digest_regular_file_bounded(&main_utoc, MAX_CONTAINER_COMPONENT_BYTES, code)?;
    let usmap_seal = digest_regular_file_bounded(&usmap, MAX_USMAP_BYTES, code)?;
    let global_utoc_seal = digest_regular_file_bounded(
        &game_root.join("G1R/Content/Paks/global.utoc"),
        MAX_CONTAINER_COMPONENT_BYTES,
        code,
    )?;
    let global_ucas_seal = digest_regular_file_bounded(
        &game_root.join("G1R/Content/Paks/global.ucas"),
        MAX_CONTAINER_COMPONENT_BYTES,
        code,
    )?;
    let required_chunks: Vec<_> = expected
        .target_chunks
        .iter()
        .map(|chunk| chunk.chunk_id.clone())
        .collect();
    let probe = gore_tex::container::probe_asset_generation_for_chunks_verified(
        &main_utoc_seal.path,
        asset,
        &required_chunks,
    )
    .with_context(|| format!("{code}: probing live target generation"))?;
    let mut source_utoc_seals = Vec::with_capacity(probe.metadata_utocs.len());
    for source_utoc in &probe.metadata_utocs {
        source_utoc_seals.push(digest_regular_file_bounded(
            source_utoc,
            MAX_CONTAINER_COMPONENT_BYTES,
            code,
        )?);
    }
    let current = build_generation_receipt_from_probe(
        asset,
        &usmap_seal,
        &main_utoc_seal,
        &global_utoc_seal,
        &global_ucas_seal,
        &probe.consumed_chunks,
        &source_utoc_seals,
        &probe.opened_utocs,
        code,
    )?;

    for seal in [
        &usmap_seal,
        &main_utoc_seal,
        &global_utoc_seal,
        &global_ucas_seal,
    ]
    .into_iter()
    .chain(source_utoc_seals.iter())
    {
        let limit = if seal.path.extension().is_some_and(|value| value == "usmap") {
            MAX_USMAP_BYTES
        } else {
            MAX_CONTAINER_COMPONENT_BYTES
        };
        seal.reverify(limit, code)?;
    }
    Ok(current)
}

// The four distinguished game anchors and two probe collections are intentionally explicit: a
// positional aggregate would make it easier to swap global/main/USMAP authority accidentally.
#[allow(clippy::too_many_arguments)]
fn build_generation_receipt_from_probe(
    asset: &str,
    usmap: &VerifiedFileSeal,
    main_utoc: &VerifiedFileSeal,
    global_utoc: &VerifiedFileSeal,
    global_ucas: &VerifiedFileSeal,
    chunks: &[gore_tex::container::VerifiedChunkReceipt],
    source_utocs: &[VerifiedFileSeal],
    opened_utocs: &[gore_tex::container::VerifiedOpenedUtocReceipt],
    code: &'static str,
) -> Result<AssetGenerationReceipt> {
    let mut by_path = BTreeMap::new();
    let mut container_set = Vec::with_capacity(source_utocs.len());
    for seal in source_utocs {
        if by_path.insert(seal.path.clone(), seal).is_some() {
            bail!("{code}: duplicate verified source UTOC path");
        }
        container_set.push(generation_anchor_from_verified_file(seal)?);
    }
    let mut opened_by_path = BTreeMap::new();
    for opened in opened_utocs {
        let canonical = validate_existing_path_no_reparse(&opened.source_utoc, false, code)?;
        let sealed = by_path.get(&canonical).with_context(|| {
            format!(
                "{code}: parsed UTOC '{}' is absent from the verified source set",
                canonical.display()
            )
        })?;
        if opened.source_utoc_blake3 != encode_hex(&sealed.blake3) {
            bail!("{code}: parsed UTOC bytes differ from the verified source file");
        }
        if opened_by_path
            .insert(canonical, opened.source_utoc_blake3.as_str())
            .is_some()
        {
            bail!("{code}: duplicate parsed source UTOC identity");
        }
    }
    if opened_by_path.len() != by_path.len()
        || by_path
            .keys()
            .any(|source_path| !opened_by_path.contains_key(source_path))
    {
        bail!("{code}: parsed and verified source UTOC sets differ");
    }
    container_set.sort_by(|left, right| {
        left.file_name
            .cmp(&right.file_name)
            .then(left.sha256.cmp(&right.sha256))
    });
    container_set.dedup();

    let mut target_chunks = Vec::new();
    for chunk in chunks.iter().filter(|chunk| {
        matches!(
            chunk.chunk_type.as_str(),
            "ContainerHeader"
                | "ExportBundleData"
                | "BulkData"
                | "OptionalBulkData"
                | "MemoryMappedBulkData"
        )
    }) {
        let canonical = validate_existing_path_no_reparse(&chunk.source_utoc, false, code)?;
        let winner = by_path.get(&canonical).with_context(|| {
            format!(
                "{code}: winning TOC '{}' is absent from the verified source set",
                canonical.display()
            )
        })?;
        if chunk.source_utoc_blake3 != encode_hex(&winner.blake3) {
            bail!("{code}: winning chunk UTOC bytes differ from the verified source file");
        }
        target_chunks.push(GenerationChunkAnchor {
            chunk_id: chunk.chunk_id.clone(),
            chunk_type: chunk.chunk_type.clone(),
            winner_utoc: generation_anchor_from_verified_file(winner)?,
            length: chunk.length,
            blake3: chunk.blake3.clone(),
            toc_hash: chunk.toc_hash.clone(),
            toc_hash_bytes: chunk.toc_hash_bytes,
        });
    }
    target_chunks.sort_by(|left, right| {
        left.chunk_id
            .cmp(&right.chunk_id)
            .then(left.chunk_type.cmp(&right.chunk_type))
            .then(left.winner_utoc.file_name.cmp(&right.winner_utoc.file_name))
    });
    if !target_chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "ExportBundleData")
        || !target_chunks
            .iter()
            .any(|chunk| chunk.chunk_type == "ContainerHeader")
    {
        bail!("{code}: live probe did not seal required package chunks");
    }

    let receipt = AssetGenerationReceipt {
        format: "gore.asset.generation.v1".to_owned(),
        asset: asset.to_owned(),
        usmap: generation_anchor_from_verified_file(usmap)?,
        main_utoc: generation_anchor_from_verified_file(main_utoc)?,
        global_utoc: generation_anchor_from_verified_file(global_utoc)?,
        global_ucas: generation_anchor_from_verified_file(global_ucas)?,
        container_set,
        target_chunks,
    };
    validate_generation_receipt(&receipt, code)?;
    Ok(receipt)
}

fn generation_anchor_from_verified_file(seal: &VerifiedFileSeal) -> Result<GenerationFileAnchor> {
    Ok(GenerationFileAnchor {
        file_name: seal
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .context("generation anchor has a non-UTF-8 filename")?
            .to_owned(),
        length: seal.length,
        sha256: encode_hex(&seal.sha256),
    })
}

fn seal_game_executable(game_root: &Path) -> Result<VerifiedGameExecutableAnchor> {
    let path = game_root
        .join("G1R")
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    let code = "ASSET_STAGE_EXECUTABLE";
    let canonical = validate_existing_path_no_reparse(&path, false, code)?;
    let mut source = BoundRegularFile::open(&canonical).with_context(|| {
        format!(
            "{code}: opening '{}' without following links",
            canonical.display()
        )
    })?;
    let advertised = source.length().context(code)?;
    if advertised == 0 {
        bail!("ASSET_STAGE_EXECUTABLE: live game executable is empty");
    }
    if advertised > MAX_GAME_EXECUTABLE_BYTES {
        bail!("{code}: executable is {advertised} bytes; limit is {MAX_GAME_EXECUTABLE_BYTES}");
    }
    let (length, sha256, _blake3) =
        digest_reader(source.file_mut(), MAX_GAME_EXECUTABLE_BYTES, code)?;
    if length != advertised {
        bail!("{code}: executable changed length while being sealed");
    }
    source
        .reverify_path_identity()
        .context("ASSET_STAGE_EXECUTABLE: executable identity changed while hashing")?;
    verify_file_hash(
        source.path(),
        length,
        sha256,
        MAX_GAME_EXECUTABLE_BYTES,
        code,
    )?;
    Ok(VerifiedGameExecutableAnchor {
        source,
        length,
        sha256,
    })
}

/// Read arbitrary supporting input with the same bounded/no-follow semantics as receipt inputs.
/// The returned facts are opaque and therefore cannot be forged from caller-supplied hashes.
pub fn read_verified_file_bounded(
    path: &Path,
    limit: u64,
    code: &'static str,
) -> Result<VerifiedInput> {
    read_verified_bounded(path, limit, code)
}

/// Bounded-read, strictly deserialize, and filesystem-bind one extract-v2 receipt.
pub fn read_extract_receipt_v2(path: &Path) -> Result<VerifiedExtractReceipt> {
    let code = "ASSET_EXTRACT_RECEIPT";
    let input = read_verified_bounded(path, MAX_RECEIPT_BYTES, code)?;
    let receipt: ExtractReceiptEnvelope = serde_json::from_slice(&input.bytes)
        .context("ASSET_EXTRACT_RECEIPT: invalid receipt JSON")?;
    let binding = validate_extract_receipt_envelope(&receipt, code)?;
    let expected_path = canonical_leaf_path(&binding.output_root.join(EXTRACT_RECEIPT_NAME), code)?;
    if input.path != expected_path {
        bail!("{code}: receipt path disagrees with extract output proof");
    }
    Ok(VerifiedExtractReceipt {
        input,
        receipt,
        binding,
    })
}

/// Bounded-read, strictly deserialize, and filesystem-bind one patch-fixed-v2 receipt.
pub fn read_patch_receipt_v2(path: &Path) -> Result<VerifiedPatchReceipt> {
    let code = "ASSET_PATCH_RECEIPT";
    let input = read_verified_bounded(path, MAX_RECEIPT_BYTES, code)?;
    let receipt: PatchReceiptEnvelope = serde_json::from_slice(&input.bytes)
        .context("ASSET_PATCH_RECEIPT: invalid receipt JSON")?;
    validate_patch_receipt_envelope(&receipt, &input.path)?;
    Ok(VerifiedPatchReceipt { input, receipt })
}

pub fn read_chained_extract_receipt(
    patch: &VerifiedPatchReceipt,
) -> Result<VerifiedExtractReceipt> {
    let code = "ASSET_EXTRACT_RECEIPT";
    let expected = &patch.receipt.provenance.extract_receipt;
    if expected.path.is_empty()
        || expected.length > MAX_RECEIPT_BYTES
        || expected.sha256.len() != 64
        || !expected.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        || expected.sha256 != expected.sha256.to_ascii_lowercase()
    {
        bail!("{code}: invalid chained receipt seal");
    }
    let actual = read_verified_bounded(Path::new(&expected.path), MAX_RECEIPT_BYTES, code)?;
    if u64::try_from(actual.bytes.len())? != expected.length
        || encode_hex(&actual.sha256) != expected.sha256
    {
        bail!("{code}: chained receipt changed or does not match its patch receipt seal");
    }
    let receipt: ExtractReceiptEnvelope = serde_json::from_slice(&actual.bytes)
        .context("ASSET_EXTRACT_RECEIPT: invalid chained extract receipt JSON")?;
    let binding = validate_extract_receipt_envelope(&receipt, code)?;
    if actual.path != canonical_leaf_path(&binding.output_root.join(EXTRACT_RECEIPT_NAME), code)? {
        bail!("{code}: chained receipt path disagrees with extract output proof");
    }
    Ok(VerifiedExtractReceipt {
        input: actual,
        receipt,
        binding,
    })
}

pub fn validate_generation_receipt(
    generation: &AssetGenerationReceipt,
    code: &'static str,
) -> Result<()> {
    fn valid_hex(value: &str, bytes: usize) -> bool {
        value.len() == bytes.saturating_mul(2)
            && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            && value == value.to_ascii_lowercase()
    }

    fn validate_file(anchor: &GenerationFileAnchor, code: &'static str) -> Result<()> {
        validate_output_component(&anchor.file_name, code)?;
        if anchor.file_name.len() > 255 || anchor.length == 0 || !valid_hex(&anchor.sha256, 32) {
            bail!("{code}: malformed generation file anchor");
        }
        Ok(())
    }

    if generation.format != "gore.asset.generation.v1"
        || generation.asset.is_empty()
        || generation.container_set.is_empty()
        || generation.container_set.len() > 256
        || generation.target_chunks.is_empty()
        || generation.target_chunks.len() > 4096
    {
        bail!("{code}: malformed generation envelope");
    }
    validate_game_asset_path(&generation.asset, code)?;
    for anchor in [
        &generation.usmap,
        &generation.main_utoc,
        &generation.global_utoc,
        &generation.global_ucas,
    ] {
        validate_file(anchor, code)?;
    }
    for anchor in &generation.container_set {
        validate_file(anchor, code)?;
    }
    if !generation.container_set.contains(&generation.main_utoc)
        || !generation.container_set.contains(&generation.global_utoc)
    {
        bail!("{code}: main/global UTOC anchors are absent from container set");
    }
    let mut unique_containers = generation.container_set.clone();
    unique_containers.sort_by(|left, right| {
        left.file_name
            .cmp(&right.file_name)
            .then(left.sha256.cmp(&right.sha256))
    });
    unique_containers.dedup();
    if unique_containers.len() != generation.container_set.len() {
        bail!("{code}: duplicate generation container anchors");
    }

    let mut ids = std::collections::BTreeSet::new();
    let mut has_target_export = false;
    let mut has_header = false;
    for chunk in &generation.target_chunks {
        validate_file(&chunk.winner_utoc, code)?;
        let length_is_valid = match chunk.chunk_type.as_str() {
            "ContainerHeader" | "ExportBundleData" => chunk.length > 0,
            "BulkData" | "OptionalBulkData" | "MemoryMappedBulkData" => true,
            _ => false,
        };
        if !generation.container_set.contains(&chunk.winner_utoc)
            || !valid_hex(&chunk.chunk_id, 12)
            || !valid_hex(&chunk.blake3, 32)
            || !matches!(chunk.toc_hash_bytes, 20 | 32)
            || !valid_hex(&chunk.toc_hash, chunk.toc_hash_bytes)
            || !length_is_valid
            || chunk.length > 512 * 1024 * 1024
            || !ids.insert(chunk.chunk_id.clone())
        {
            bail!("{code}: malformed or duplicate generation chunk anchor");
        }
        match chunk.chunk_type.as_str() {
            "ContainerHeader" => has_header = true,
            "ExportBundleData" => {
                has_target_export |= chunk_id_matches_asset_path(&chunk.chunk_id, &generation.asset)
            }
            "BulkData" | "OptionalBulkData" | "MemoryMappedBulkData" => {}
            _ => bail!("{code}: unsupported generation chunk type"),
        }
    }
    if !has_target_export || !has_header {
        bail!("{code}: generation must contain the target export and ContainerHeader chunks");
    }
    Ok(())
}

pub fn validate_sidecar_receipts(
    sidecars: &[SidecarReceipt],
    stem: &str,
    code: &'static str,
) -> Result<()> {
    if sidecars.len() > SidecarRole::ALL.len() {
        bail!("{code}: more than three optional sidecars are refused");
    }
    let mut previous_role = None;
    for sidecar in sidecars {
        if previous_role.is_some_and(|previous| previous >= sidecar.role)
            || sidecar.file_name != format!("{stem}.{}", sidecar.role.suffix())
            || sidecar.length > MAX_OPTIONAL_SIDECAR_BYTES
            || !is_canonical_sha256(&sidecar.sha256)
        {
            bail!("{code}: malformed, duplicate, or noncanonical sidecar receipt");
        }
        previous_role = Some(sidecar.role);
    }
    Ok(())
}

pub fn validate_sidecar_generation_mapping(
    sidecars: &[SidecarReceipt],
    generation: &AssetGenerationReceipt,
    code: &'static str,
) -> Result<()> {
    let expected: Vec<_> = sidecars.iter().map(|sidecar| sidecar.role).collect();
    let mut actual: Vec<_> = generation
        .target_chunks
        .iter()
        .filter(|chunk| chunk_id_matches_asset_path(&chunk.chunk_id, &generation.asset))
        .filter_map(|chunk| SidecarRole::from_chunk_type(&chunk.chunk_type))
        .collect();
    actual.sort();
    if actual.windows(2).any(|pair| pair[0] == pair[1]) {
        bail!("{code}: original generation exposes duplicate target bulk chunk roles");
    }
    if actual != expected {
        let expected: Vec<_> = expected.iter().map(|role| role.chunk_type()).collect();
        let actual: Vec<_> = actual.iter().map(|role| role.chunk_type()).collect();
        bail!(
            "{code}: sidecar roles do not match original target bulk chunks; expected {expected:?}, generation has {actual:?}"
        );
    }
    Ok(())
}

pub fn generation_mismatch_reason(
    expected: &AssetGenerationReceipt,
    current: &AssetGenerationReceipt,
) -> &'static str {
    if expected.format != current.format || expected.asset != current.asset {
        "generation envelope"
    } else if expected.usmap != current.usmap {
        "USMAP anchor"
    } else if expected.main_utoc != current.main_utoc {
        "main UTOC anchor"
    } else if expected.global_utoc != current.global_utoc {
        "global UTOC anchor"
    } else if expected.global_ucas != current.global_ucas {
        "global UCAS anchor"
    } else if expected.container_set != current.container_set {
        "participating UTOC set"
    } else if expected.target_chunks != current.target_chunks {
        "target or ContainerHeader chunk winners"
    } else {
        "unknown field"
    }
}

pub fn validate_extract_receipt_envelope(
    receipt: &ExtractReceiptEnvelope,
    code: &'static str,
) -> Result<ValidatedExtractBinding> {
    if receipt.format != "gore.asset.extract.v2"
        || receipt.status != "extracted"
        || receipt.deployed
        || receipt.output.root.is_empty()
        || !Path::new(&receipt.output.root).is_absolute()
        || receipt.output.receipt != EXTRACT_RECEIPT_NAME
    {
        bail!("{code}: malformed or unsupported extract-v2 envelope");
    }
    validate_game_asset_path(&receipt.asset, code)?;
    if receipt.generation.asset != receipt.asset {
        bail!("{code}: generation targets a different asset");
    }
    validate_generation_receipt(&receipt.generation, code)?;

    let output_root = canonical_leaf_path(Path::new(&receipt.output.root), code)?;
    let components = &receipt.output.components;
    if !(3..=6).contains(&components.len()) {
        bail!("{code}: invalid output component count");
    }
    let mut names = std::collections::BTreeSet::new();
    for component in components {
        if component.relative_path.is_empty()
            || component.relative_path.contains('/')
            || component.relative_path.contains('\\')
            || !is_canonical_sha256(&component.sha256)
            || !names.insert(component.relative_path.clone())
        {
            bail!("{code}: malformed or duplicate output component");
        }
        validate_output_component(&component.relative_path, code)?;
    }

    let uasset = components
        .first()
        .context("ASSET_EXTRACT_RECEIPT: missing uasset component")?;
    let stem = uasset
        .relative_path
        .strip_suffix(".uasset")
        .filter(|value| !value.is_empty())
        .context("ASSET_EXTRACT_RECEIPT: first component must be lowercase .uasset")?;
    if windows_reserved_name(stem)
        || !stem
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        bail!("{code}: extracted package stem is noncanonical");
    }
    let uexp = components
        .get(1)
        .context("ASSET_EXTRACT_RECEIPT: missing uexp component")?;
    let copied_usmap = components
        .get(2)
        .context("ASSET_EXTRACT_RECEIPT: missing copied USMAP component")?;
    if uexp.relative_path != format!("{stem}.uexp")
        || copied_usmap.relative_path != COPIED_USMAP_NAME
        || uasset.length == 0
        || uasset.length > asset_package_limits().max_uasset_bytes
        || uexp.length == 0
        || uexp.length > asset_package_limits().max_uexp_bytes
        || copied_usmap.length == 0
        || copied_usmap.length > MAX_USMAP_BYTES
    {
        bail!("{code}: required extract component names or lengths are invalid");
    }
    let pair_total = uasset
        .length
        .checked_add(uexp.length)
        .context("ASSET_EXTRACT_RECEIPT: package size overflowed")?;
    if pair_total > asset_package_limits().max_total_bytes {
        bail!("{code}: extracted package pair exceeds aggregate limit");
    }
    if uasset.sha256 != encode_hex(&receipt.package_seal.uasset_sha256)
        || uexp.sha256 != encode_hex(&receipt.package_seal.uexp_sha256)
    {
        bail!("{code}: package component hashes disagree with package seal");
    }
    if copied_usmap.length != receipt.generation.usmap.length
        || copied_usmap.sha256 != receipt.generation.usmap.sha256
    {
        bail!("{code}: copied USMAP component disagrees with generation anchor");
    }

    let mut sidecars = Vec::new();
    let mut cooked_total = pair_total;
    for component in &components[3..] {
        let role = SidecarRole::ALL
            .into_iter()
            .find(|role| component.relative_path == format!("{stem}.{}", role.suffix()))
            .with_context(|| format!("{code}: unexpected extract output component"))?;
        cooked_total = cooked_total
            .checked_add(component.length)
            .context("ASSET_EXTRACT_RECEIPT: cooked size overflowed")?;
        sidecars.push(SidecarReceipt {
            role,
            file_name: component.relative_path.clone(),
            length: component.length,
            sha256: component.sha256.clone(),
        });
    }
    if cooked_total > MAX_COOKED_PACKAGE_BYTES {
        bail!("{code}: extracted cooked package exceeds aggregate limit");
    }
    validate_sidecar_receipts(&sidecars, stem, code)?;
    validate_sidecar_generation_mapping(&sidecars, &receipt.generation, code)?;
    validate_extract_source_proofs(receipt, copied_usmap, code)?;

    Ok(ValidatedExtractBinding {
        output_root,
        uasset: uasset.clone(),
        uexp: uexp.clone(),
        copied_usmap: copied_usmap.clone(),
        components: components.clone(),
        sidecars,
    })
}

fn validate_extract_source_proofs(
    receipt: &ExtractReceiptEnvelope,
    copied_usmap: &ReceiptComponent,
    code: &'static str,
) -> Result<()> {
    let source = &receipt.source;
    let game_root = validate_lexical_receipt_path(Path::new(&source.game_root), code)?;
    let paks = game_root.join("G1R").join("Content").join("Paks");
    let ue4ss = game_root
        .join("G1R")
        .join("Binaries")
        .join("Win64")
        .join("ue4ss");
    if source.game_root.is_empty()
        || source.content_binding != EXTRACT_CONTENT_BINDING
        || source.composite_store_anchor.role != COMPOSITE_UCAS_ROLE
        || source.usmap.copied_relative_path != COPIED_USMAP_NAME
        || source.usmap.copy != *copied_usmap
    {
        bail!("{code}: contradictory extract source envelope");
    }
    let held = &source.composite_store_anchor.ucas;
    let held_path = validate_lexical_receipt_path(Path::new(&held.path), code)?;
    if held.length == 0
        || held.length > MAX_MOUNT_UCAS_BYTES
        || held.modified_stamp.is_empty()
        || held.platform_identity.is_empty()
        || held.sha256.is_some()
        || held.verification != HELD_IDENTITY_VERIFICATION
        || !held.content_hash_omitted
        || held.limitation != HELD_IDENTITY_LIMITATION
    {
        bail!("{code}: malformed main UCAS identity proof");
    }

    let main_anchor = generation_anchor_from_source(
        &source.composite_store_anchor.utoc,
        MAX_MOUNT_UTOC_BYTES,
        code,
    )?;
    let usmap_anchor = generation_anchor_from_source(&source.usmap.source, MAX_USMAP_BYTES, code)?;
    let global_utoc_anchor = generation_anchor_from_source(
        &source.global_script_store.utoc,
        MAX_MOUNT_UTOC_BYTES,
        code,
    )?;
    let global_ucas_anchor = generation_anchor_from_source(
        &source.global_script_store.ucas,
        MAX_CONTAINER_COMPONENT_BYTES,
        code,
    )?;
    let main_utoc_path =
        validate_lexical_receipt_path(Path::new(&source.composite_store_anchor.utoc.path), code)?;
    let usmap_path = validate_lexical_receipt_path(Path::new(&source.usmap.source.path), code)?;
    let global_utoc_path =
        validate_lexical_receipt_path(Path::new(&source.global_script_store.utoc.path), code)?;
    let global_ucas_path =
        validate_lexical_receipt_path(Path::new(&source.global_script_store.ucas.path), code)?;
    if main_anchor != receipt.generation.main_utoc
        || usmap_anchor != receipt.generation.usmap
        || global_utoc_anchor != receipt.generation.global_utoc
        || global_ucas_anchor != receipt.generation.global_ucas
    {
        bail!("{code}: duplicated generation file anchors disagree");
    }
    require_receipt_path(
        &main_utoc_path,
        &paks.join("G1R-Windows.utoc"),
        "main UTOC proof",
        code,
    )?;
    require_receipt_path(
        &held_path,
        &paks.join("G1R-Windows.ucas"),
        "main UCAS proof",
        code,
    )?;
    require_receipt_path(
        &global_utoc_path,
        &paks.join("global.utoc"),
        "global UTOC proof",
        code,
    )?;
    require_receipt_path(
        &global_ucas_path,
        &paks.join("global.ucas"),
        "global UCAS proof",
        code,
    )?;
    require_direct_receipt_child(&usmap_path, &ue4ss, "usmap", "USMAP proof", code)?;
    let main_ucas_name = held_path
        .file_name()
        .and_then(|value| value.to_str())
        .context(format!("{code}: main UCAS filename is non-UTF-8"))?;
    if Path::new(&main_anchor.file_name).with_extension("ucas") != Path::new(main_ucas_name) {
        bail!("{code}: main UTOC and UCAS proofs are not a matching pair");
    }

    let mut toc_by_path = std::collections::BTreeMap::new();
    let mut container_set = Vec::with_capacity(source.source_container_tocs.len());
    for proof in &source.source_container_tocs {
        let anchor = generation_anchor_from_source(proof, MAX_MOUNT_UTOC_BYTES, code)?;
        let path = validate_lexical_receipt_path(Path::new(&proof.path), code)?;
        require_direct_receipt_child(&path, &paks, "utoc", "source container TOC proof", code)?;
        if toc_by_path.insert(path, anchor.clone()).is_some() {
            bail!("{code}: duplicate source container TOC proof");
        }
        container_set.push(anchor);
    }
    container_set.sort_by(|left, right| {
        left.file_name
            .cmp(&right.file_name)
            .then(left.sha256.cmp(&right.sha256))
    });
    if container_set != receipt.generation.container_set {
        bail!("{code}: source container TOCs disagree with generation container set");
    }
    if toc_by_path.get(&main_utoc_path) != Some(&main_anchor) {
        bail!("{code}: main UTOC proof is absent from source container TOCs");
    }
    if toc_by_path.get(&global_utoc_path) != Some(&global_utoc_anchor) {
        bail!("{code}: global UTOC proof is absent from source container TOCs");
    }

    let mut chunk_ids = std::collections::BTreeSet::new();
    let mut target_chunks = Vec::new();
    for chunk in &source.consumed_chunks {
        if !is_canonical_hex(&chunk.chunk_id, 12)
            || chunk.chunk_type.is_empty()
            || !chunk
                .chunk_type
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric())
            || chunk.length > 512 * 1024 * 1024
            || !is_canonical_hex(&chunk.blake3, 32)
            || !matches!(chunk.toc_hash_bytes, 20 | 32)
            || !is_canonical_hex(&chunk.toc_hash, chunk.toc_hash_bytes)
            || !chunk_ids.insert(chunk.chunk_id.clone())
        {
            bail!("{code}: malformed or duplicate consumed chunk proof");
        }
        let source_utoc = validate_lexical_receipt_path(&chunk.source_utoc, code)?;
        let winner = toc_by_path.get(&source_utoc).with_context(|| {
            format!("{code}: consumed chunk winner is absent from source TOC proofs")
        })?;
        if matches!(
            chunk.chunk_type.as_str(),
            "ContainerHeader"
                | "ExportBundleData"
                | "BulkData"
                | "OptionalBulkData"
                | "MemoryMappedBulkData"
        ) {
            target_chunks.push(GenerationChunkAnchor {
                chunk_id: chunk.chunk_id.clone(),
                chunk_type: chunk.chunk_type.clone(),
                winner_utoc: winner.clone(),
                length: chunk.length,
                blake3: chunk.blake3.clone(),
                toc_hash: chunk.toc_hash.clone(),
                toc_hash_bytes: chunk.toc_hash_bytes,
            });
        }
    }
    target_chunks.sort_by(|left, right| {
        left.chunk_id
            .cmp(&right.chunk_id)
            .then(left.chunk_type.cmp(&right.chunk_type))
            .then(left.winner_utoc.file_name.cmp(&right.winner_utoc.file_name))
    });
    if target_chunks != receipt.generation.target_chunks {
        bail!("{code}: consumed chunk proofs disagree with generation target chunks");
    }
    Ok(())
}

pub fn validate_patch_receipt_envelope(
    receipt: &PatchReceiptEnvelope,
    receipt_path: &Path,
) -> Result<()> {
    let code = "ASSET_PATCH_RECEIPT";
    if receipt.format != "gore.asset.patch-fixed.v2"
        || receipt.status != "patched"
        || !receipt.generation_bound
        || receipt.provenance.generation.format != "gore.asset.generation.v1"
        || receipt.asset != receipt.provenance.generation.asset
        || !receipt.output_requires_reinspect
    {
        bail!("{code}: unsupported, contradictory, or unbound patch-v2 envelope");
    }
    validate_game_asset_path(&receipt.asset, code)?;
    validate_generation_receipt(&receipt.provenance.generation, code)?;
    if !receipt
        .provenance
        .usmap
        .same_content(&receipt.provenance.generation.usmap)
        || receipt.input_selector.package_seal != receipt.input_package_seal
        || receipt.input_selector.usmap_sha256 != receipt.provenance.usmap.sha256
        || receipt.input_selector.expected_hex != receipt.expected_hex
        || receipt.patch.before != receipt.input_package_seal
        || receipt.patch.after != receipt.output_package_seal
        || receipt.patch.export_index != receipt.input_selector.export_index
        || receipt.patch.component != receipt.input_selector.component
        || receipt.patch.kind != receipt.input_selector.kind
        || receipt.patch.length != receipt.input_selector.kind.width()
    {
        bail!("{code}: duplicated selector, USMAP, package, or patch proofs disagree");
    }
    let width = receipt.input_selector.kind.width();
    if !is_canonical_hex(&receipt.expected_hex, width)
        || !is_canonical_hex(&receipt.replacement_hex, width)
    {
        bail!("{code}: expected/replacement wire proofs are noncanonical");
    }
    validate_component_digest_proof(
        &receipt.output.uasset,
        "uasset",
        asset_package_limits().max_uasset_bytes,
        code,
    )?;
    validate_component_digest_proof(
        &receipt.output.uexp,
        "uexp",
        asset_package_limits().max_uexp_bytes,
        code,
    )?;
    let canonical_receipt_path = canonical_leaf_path(receipt_path, code)?;
    let canonical_proof_receipt = canonical_leaf_path(Path::new(&receipt.output.receipt), code)?;
    if receipt.output.uasset.sha256 != encode_hex(&receipt.output_package_seal.uasset_sha256)
        || receipt.output.uexp.sha256 != encode_hex(&receipt.output_package_seal.uexp_sha256)
        || receipt
            .output
            .uasset
            .length
            .checked_add(receipt.output.uexp.length)
            .is_none_or(|total| total > asset_package_limits().max_total_bytes)
        || receipt.output.sidecars != receipt.output_sidecars
        || canonical_proof_receipt != canonical_receipt_path
    {
        bail!("{code}: duplicated output package, sidecar, or receipt proofs disagree");
    }
    let output_uasset = Path::new(&receipt.output.uasset.path);
    let output_uexp = Path::new(&receipt.output.uexp.path);
    if output_uasset.with_extension("uexp") != output_uexp {
        bail!("{code}: output uasset/uexp proof paths are not a pair");
    }
    let stem = output_uasset
        .file_stem()
        .and_then(|value| value.to_str())
        .context("ASSET_PATCH_RECEIPT: output package has no UTF-8 stem")?;
    let expected_receipt = output_uasset.with_file_name(format!("{stem}{PATCH_RECEIPT_SUFFIX}"));
    if canonical_proof_receipt != canonical_leaf_path(&expected_receipt, code)? {
        bail!("{code}: output receipt path does not match output package stem");
    }
    validate_sidecar_receipts(&receipt.output_sidecars, stem, code)?;
    validate_sidecar_generation_mapping(
        &receipt.output_sidecars,
        &receipt.provenance.generation,
        code,
    )?;
    let component_length = if receipt.patch.component == PackageComponent::Uasset {
        receipt.output.uasset.length
    } else {
        receipt.output.uexp.length
    };
    let patch_end = u64::try_from(receipt.patch.absolute_offset)?
        .checked_add(u64::try_from(receipt.patch.length)?)
        .context("ASSET_PATCH_RECEIPT: patch range overflowed")?;
    if patch_end > component_length {
        bail!("{code}: patch range exceeds sealed output component");
    }
    Ok(())
}

pub fn validate_extract_receipt_components(
    extract: &VerifiedExtractReceipt,
    carrier: &PackageCarrier,
    usmap: &VerifiedInput,
) -> Result<Vec<VerifiedFileSeal>> {
    let binding = &extract.binding;
    let source = carrier
        .source_paths()
        .context("ASSET_EXTRACT_RECEIPT: input package has no source paths")?;
    let source_uasset_name = source
        .uasset()
        .file_name()
        .and_then(|value| value.to_str())
        .context("ASSET_EXTRACT_RECEIPT: input uasset has a non-UTF-8 filename")?;
    let source_uexp_name = source
        .uexp()
        .file_name()
        .and_then(|value| value.to_str())
        .context("ASSET_EXTRACT_RECEIPT: input uexp has a non-UTF-8 filename")?;
    if source_uasset_name != binding.uasset.relative_path
        || source_uexp_name != binding.uexp.relative_path
        || source.uasset().parent() != Some(binding.output_root.as_path())
        || source.uexp().parent() != Some(binding.output_root.as_path())
    {
        bail!("ASSET_EXTRACT_RECEIPT: input package filenames differ from extract manifest");
    }
    let seal = PackagePairSeal::capture(carrier);
    if binding.uasset.length != u64::try_from(carrier.len(PackageComponent::Uasset))?
        || binding.uasset.sha256 != encode_hex(&seal.uasset_sha256)
        || binding.uexp.length != u64::try_from(carrier.len(PackageComponent::Uexp))?
        || binding.uexp.sha256 != encode_hex(&seal.uexp_sha256)
    {
        bail!("ASSET_EXTRACT_RECEIPT: package component list disagrees with sealed input pair");
    }
    if binding.copied_usmap.length != usmap.length()
        || binding.copied_usmap.sha256 != encode_hex(usmap.sha256())
    {
        bail!("ASSET_EXTRACT_RECEIPT: copied USMAP component disagrees with supplied USMAP");
    }

    let mut actual_sidecars = Vec::new();
    for role in SidecarRole::ALL {
        let (file_name, path) = sidecar_path(source.uasset(), role, "ASSET_EXTRACT_RECEIPT")?;
        let expected = binding.sidecars.iter().find(|sidecar| sidecar.role == role);
        let actual = digest_optional_regular_file_bounded(
            &path,
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_EXTRACT_RECEIPT",
        )?;
        match (expected, actual) {
            (None, None) => {}
            (Some(expected), Some(actual))
                if expected.file_name == file_name
                    && expected.length == actual.length
                    && expected.sha256 == encode_hex(&actual.sha256) =>
            {
                actual_sidecars.push(actual);
            }
            _ => bail!(
                "ASSET_GENERATION_MISMATCH: input optional sidecar set or content differs from extract receipt"
            ),
        }
    }
    Ok(actual_sidecars)
}

pub fn validate_patched_sidecars(
    patch: &VerifiedPatchReceipt,
    uasset: &Path,
    pair_bytes: u64,
) -> Result<Vec<VerifiedFileSeal>> {
    let expected_sidecars = &patch.receipt.output_sidecars;
    let stem = uasset
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .context("ASSET_PACK_SIDECAR: patched package requires a UTF-8 stem")?;
    validate_sidecar_receipts(expected_sidecars, stem, "ASSET_PATCH_RECEIPT")?;
    let mut actual_sidecars = Vec::new();
    let mut cooked_total = pair_bytes;
    for role in SidecarRole::ALL {
        let (_, path) = sidecar_path(uasset, role, "ASSET_PACK_SIDECAR")?;
        let expected = expected_sidecars
            .iter()
            .find(|sidecar| sidecar.role == role);
        let actual = digest_optional_regular_file_bounded(
            &path,
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_PACK_SIDECAR",
        )?;
        match (expected, actual) {
            (None, None) => {}
            (Some(expected), Some(actual))
                if expected.length == actual.length
                    && expected.sha256 == encode_hex(&actual.sha256) =>
            {
                cooked_total = cooked_total
                    .checked_add(actual.length)
                    .context("ASSET_PACK_SIDECAR: cooked size overflowed")?;
                actual_sidecars.push(actual);
            }
            (None, Some(_)) => bail!(
                "ASSET_PACK_SIDECAR: unexpected optional sidecar exists; patched sidecar set must exactly match PatchReceipt v2"
            ),
            (Some(_), None) => bail!(
                "ASSET_PACK_SIDECAR: required optional sidecar is missing; re-run patch-fixed"
            ),
            (Some(_), Some(_)) => bail!(
                "ASSET_PACK_SIDECAR: optional sidecar content differs from PatchReceipt v2"
            ),
        }
    }
    if cooked_total > MAX_COOKED_PACKAGE_BYTES {
        bail!(
            "ASSET_PACK_SIDECAR: cooked package is {cooked_total} bytes; aggregate limit is {MAX_COOKED_PACKAGE_BYTES}"
        );
    }
    Ok(actual_sidecars)
}

pub fn validate_patch_output_against_carrier(
    patch: &VerifiedPatchReceipt,
    source_uasset: &Path,
    source_uexp: &Path,
    carrier: &PackageCarrier,
) -> Result<()> {
    let receipt = &patch.receipt;
    let proof_uasset = canonical_leaf_path(
        Path::new(&receipt.output.uasset.path),
        "ASSET_PATCH_RECEIPT",
    )?;
    let proof_uexp =
        canonical_leaf_path(Path::new(&receipt.output.uexp.path), "ASSET_PATCH_RECEIPT")?;
    if source_uasset != proof_uasset
        || source_uexp != proof_uexp
        || receipt.output.uasset.length != u64::try_from(carrier.len(PackageComponent::Uasset))?
        || receipt.output.uexp.length != u64::try_from(carrier.len(PackageComponent::Uexp))?
    {
        bail!("ASSET_PATCH_RECEIPT: supplied patched pair differs from output path/length proofs");
    }
    let seal = PackagePairSeal::capture(carrier);
    if receipt.output.uasset.sha256 != encode_hex(&seal.uasset_sha256)
        || receipt.output.uexp.sha256 != encode_hex(&seal.uexp_sha256)
    {
        bail!("ASSET_PATCH_RECEIPT: supplied patched pair differs from output hash proofs");
    }
    let replacement = decode_canonical_hex(
        &receipt.replacement_hex,
        receipt.patch.length,
        "ASSET_PATCH_RECEIPT",
    )?;
    let patched_range = carrier
        .slice(
            receipt.patch.component,
            receipt.patch.absolute_offset,
            receipt.patch.length,
        )
        .context("ASSET_PATCH_RECEIPT: reading sealed output patch range")?;
    if patched_range != replacement {
        bail!("ASSET_PATCH_RECEIPT: replacement proof differs from sealed output patch range");
    }
    Ok(())
}

fn validate_component_digest_proof(
    proof: &ComponentDigestProof,
    expected_extension: &str,
    limit: u64,
    code: &'static str,
) -> Result<()> {
    let path = Path::new(&proof.path);
    if !path.is_absolute()
        || path.extension().and_then(|value| value.to_str()) != Some(expected_extension)
        || proof.length == 0
        || proof.length > limit
        || !is_canonical_sha256(&proof.sha256)
    {
        bail!("{code}: malformed patch output component proof");
    }
    Ok(())
}

fn generation_anchor_from_source(
    proof: &SourceFileReceipt,
    limit: u64,
    code: &'static str,
) -> Result<GenerationFileAnchor> {
    let path = Path::new(&proof.path);
    validate_lexical_receipt_path(path, code)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .with_context(|| format!("{code}: source proof has no UTF-8 filename"))?;
    if proof.length == 0 || proof.length > limit || !is_canonical_sha256(&proof.sha256) {
        bail!("{code}: malformed source file proof");
    }
    Ok(GenerationFileAnchor {
        file_name: file_name.to_owned(),
        length: proof.length,
        sha256: proof.sha256.clone(),
    })
}

pub fn validate_game_asset_path(asset: &str, code: &'static str) -> Result<()> {
    if asset.len() > 512 || !asset.starts_with("/Game/") {
        bail!("{code}: expected an extensionless package path beginning with '/Game/'");
    }
    if asset.contains('\\') || asset.ends_with('/') {
        bail!("{code}: backslashes and trailing separators are refused");
    }
    let segments: Vec<_> = asset["/Game/".len()..].split('/').collect();
    if segments.is_empty()
        || segments.len() > MAX_GAME_ASSET_SEGMENTS
        || segments.iter().any(|segment| {
            segment.is_empty()
                || windows_reserved_name(segment)
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
    {
        bail!(
            "{code}: /Game paths allow 1..={MAX_GAME_ASSET_SEGMENTS} non-device segments containing only ASCII letters, digits, or '_'"
        );
    }
    Ok(())
}

fn validate_lexical_receipt_path(path: &Path, code: &'static str) -> Result<PathBuf> {
    if !path.is_absolute() {
        bail!(
            "{code}: source proof path is not absolute: {}",
            path.display()
        );
    }

    // Historical paths are validated lexically: no disk access, normalization, or authority
    // is derived from the caller's current filesystem state.
    let mut rebuilt = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                let value = value.to_str().with_context(|| {
                    format!("{code}: source proof path contains a non-UTF-8 component")
                })?;
                if value.ends_with(['.', ' '])
                    || value.chars().any(|ch| {
                        ch.is_control()
                            || matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\')
                    })
                    || windows_reserved_name(value)
                {
                    bail!(
                        "{code}: source proof path contains an unsafe component: {}",
                        path.display()
                    );
                }
                rebuilt.push(component.as_os_str());
            }
            Component::Prefix(prefix) => {
                #[cfg(windows)]
                if !matches!(
                    prefix.kind(),
                    std::path::Prefix::Disk(_)
                        | std::path::Prefix::UNC(_, _)
                        | std::path::Prefix::VerbatimDisk(_)
                        | std::path::Prefix::VerbatimUNC(_, _)
                ) {
                    bail!(
                        "{code}: source proof path uses a non-filesystem Windows prefix: {}",
                        path.display()
                    );
                }
                let _ = prefix;
                rebuilt.push(component.as_os_str());
            }
            Component::RootDir => rebuilt.push(component.as_os_str()),
            Component::CurDir | Component::ParentDir => {
                bail!(
                    "{code}: source proof path contains a noncanonical component: {}",
                    path.display()
                );
            }
        }
    }
    if rebuilt.as_os_str() != path.as_os_str() {
        bail!(
            "{code}: source proof path is not in canonical lexical form: {}",
            path.display()
        );
    }
    Ok(rebuilt)
}

fn require_receipt_path(
    actual: &Path,
    expected: &Path,
    role: &str,
    code: &'static str,
) -> Result<()> {
    if actual.as_os_str() != expected.as_os_str() {
        bail!(
            "{code}: {role} is outside its canonical game-root location; expected '{}'",
            expected.display()
        );
    }
    Ok(())
}

fn require_direct_receipt_child(
    path: &Path,
    directory: &Path,
    extension: &str,
    role: &str,
    code: &'static str,
) -> Result<()> {
    let parent_matches = path
        .parent()
        .is_some_and(|parent| parent.as_os_str() == directory.as_os_str());
    if !parent_matches || path.extension().and_then(|value| value.to_str()) != Some(extension) {
        bail!(
            "{code}: {role} is not a canonical direct .{extension} child of '{}'",
            directory.display()
        );
    }
    Ok(())
}

fn windows_reserved_name(name: &str) -> bool {
    let trimmed = name.trim_end_matches(['.', ' ']);
    let base = trimmed.split('.').next().unwrap_or(trimmed);
    let upper = base.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && matches!(upper.as_bytes()[3], b'1'..=b'9'))
}

fn validate_output_component(name: &str, code: &'static str) -> Result<()> {
    if name.is_empty()
        || matches!(name, "." | "..")
        || name.ends_with(['.', ' '])
        || name.chars().any(|ch| {
            ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
        })
        || windows_reserved_name(name)
    {
        bail!("{code}: unsafe output directory name {name:?}");
    }
    Ok(())
}

fn absolute_without_parent_components(path: &Path, code: &'static str) -> Result<PathBuf> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        bail!("{code}: '..' path traversal is refused: {}", path.display());
    }
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir().context(code)?.join(path))
    }
}

fn canonical_leaf_path(path: &Path, code: &'static str) -> Result<PathBuf> {
    let absolute = absolute_without_parent_components(path, code)?;
    let file_name = absolute
        .file_name()
        .filter(|value| !value.is_empty())
        .with_context(|| format!("{code}: path requires a final component"))?;
    let parent = absolute
        .parent()
        .with_context(|| format!("{code}: path has no parent"))?;
    let canonical_parent = validate_existing_path_no_reparse(parent, true, code)?;
    Ok(canonical_parent.join(file_name))
}

fn validate_existing_path_no_reparse(
    path: &Path,
    expect_directory: bool,
    code: &'static str,
) -> Result<PathBuf> {
    let absolute = absolute_without_parent_components(path, code)?;
    for ancestor in absolute.ancestors() {
        let metadata = fs::symlink_metadata(ancestor)
            .with_context(|| format!("{code}: inspecting '{}'", ancestor.display()))?;
        if metadata_is_reparse(&metadata) {
            bail!(
                "{code}: symbolic-link or reparse traversal is refused: {}",
                ancestor.display()
            );
        }
    }
    let metadata = fs::symlink_metadata(&absolute)
        .with_context(|| format!("{code}: inspecting '{}'", absolute.display()))?;
    if expect_directory && !metadata.is_dir() {
        bail!("{code}: expected a directory: {}", absolute.display());
    }
    if !expect_directory && !metadata.is_file() {
        bail!("{code}: expected a regular file: {}", absolute.display());
    }
    fs::canonicalize(&absolute)
        .with_context(|| format!("{code}: canonicalizing '{}'", absolute.display()))
}

fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn sidecar_path(uasset: &Path, role: SidecarRole, code: &'static str) -> Result<(String, PathBuf)> {
    let stem = uasset
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .with_context(|| format!("{code}: non-empty UTF-8 package stem required"))?;
    let file_name = format!("{stem}.{}", role.suffix());
    Ok((file_name.clone(), uasset.with_file_name(file_name)))
}

fn digest_optional_regular_file_bounded(
    path: &Path,
    limit: u64,
    code: &'static str,
) -> Result<Option<VerifiedFileSeal>> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("{code}: inspecting '{}'", path.display()))
        }
        Ok(_) => digest_regular_file_bounded(path, limit, code).map(Some),
    }
}

fn digest_regular_file_bounded(
    path: &Path,
    limit: u64,
    code: &'static str,
) -> Result<VerifiedFileSeal> {
    let canonical = validate_existing_path_no_reparse(path, false, code)?;
    let mut file = BoundRegularFile::open(&canonical).with_context(|| {
        format!(
            "{code}: opening '{}' without following links",
            canonical.display()
        )
    })?;
    let canonical = file.path().to_path_buf();
    let advertised = file.length().context(code)?;
    if advertised > limit {
        bail!(
            "{code}: '{}' is {advertised} bytes; limit is {limit}",
            canonical.display()
        );
    }
    let (length, sha256, blake3) = digest_reader(file.file_mut(), limit, code)?;
    if length != advertised {
        bail!(
            "{code}: input changed length while being read: {}",
            canonical.display()
        );
    }
    file.reverify_path_identity()
        .with_context(|| format!("{code}: source identity changed while hashing"))?;
    verify_file_hash(&canonical, length, sha256, limit, code)?;
    Ok(VerifiedFileSeal {
        path: canonical,
        length,
        sha256,
        blake3,
    })
}

fn digest_reader<R: Read>(
    reader: &mut R,
    limit: u64,
    code: &'static str,
) -> Result<(u64, [u8; 32], [u8; 32])> {
    let mut hasher = Sha256::new();
    let mut blake3 = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut length = 0u64;
    loop {
        let read = reader.read(&mut buffer).context(code)?;
        if read == 0 {
            break;
        }
        length = length
            .checked_add(u64::try_from(read)?)
            .context("bounded file length overflowed")?;
        if length > limit {
            bail!("{code}: file exceeded {limit} bytes while being read");
        }
        hasher.update(&buffer[..read]);
        blake3.update(&buffer[..read]);
    }
    Ok((
        length,
        hasher.finalize().into(),
        *blake3.finalize().as_bytes(),
    ))
}

fn read_verified_bounded(path: &Path, limit: u64, code: &'static str) -> Result<VerifiedInput> {
    let canonical = validate_existing_path_no_reparse(path, false, code)?;
    let mut file = BoundRegularFile::open(&canonical).with_context(|| {
        format!(
            "{code}: opening '{}' without following links",
            canonical.display()
        )
    })?;
    let canonical = file.path().to_path_buf();
    let advertised = file.length().with_context(|| {
        format!(
            "{code}: reading opened-file metadata for '{}'",
            canonical.display()
        )
    })?;
    if advertised > limit {
        bail!(
            "{code}: '{}' is {advertised} bytes; limit is {limit}",
            canonical.display()
        );
    }
    let allocation = usize::try_from(advertised)
        .with_context(|| format!("{code}: input length does not fit memory"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(allocation)
        .with_context(|| format!("{code}: reserving {allocation} bytes"))?;
    file.file_mut()
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .with_context(|| format!("{code}: reading '{}'", canonical.display()))?;
    if u64::try_from(bytes.len())? != advertised {
        bail!(
            "{code}: input changed length while being read: {}",
            canonical.display()
        );
    }
    file.reverify_path_identity()
        .with_context(|| format!("{code}: source identity changed while reading"))?;
    let sha256: [u8; 32] = Sha256::digest(&bytes).into();
    verify_file_hash(&canonical, advertised, sha256, limit, code)?;
    Ok(VerifiedInput {
        path: canonical,
        bytes,
        sha256,
    })
}

fn verify_file_hash(
    path: &Path,
    expected_length: u64,
    expected_sha256: [u8; 32],
    limit: u64,
    code: &'static str,
) -> Result<()> {
    let canonical = validate_existing_path_no_reparse(path, false, code)?;
    let mut file = BoundRegularFile::open(&canonical).with_context(|| {
        format!(
            "{code}: reopening '{}' without following links",
            canonical.display()
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut actual_length = 0u64;
    loop {
        let read = file
            .file_mut()
            .read(&mut buffer)
            .with_context(|| format!("{code}: reverifying '{}'", path.display()))?;
        if read == 0 {
            break;
        }
        actual_length = actual_length
            .checked_add(u64::try_from(read)?)
            .context("verified input length overflowed")?;
        if actual_length > limit {
            bail!("{code}: input grew beyond {limit} bytes while being reverified");
        }
        hasher.update(&buffer[..read]);
    }
    let actual_sha256: [u8; 32] = hasher.finalize().into();
    if actual_length != expected_length || actual_sha256 != expected_sha256 {
        bail!("{code}: input changed while being read: {}", path.display());
    }
    file.reverify_path_identity()
        .with_context(|| format!("{code}: source identity changed during reverify"))?;
    Ok(())
}

fn is_canonical_hex(value: &str, bytes: usize) -> bool {
    value.len() == bytes.saturating_mul(2)
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value == value.to_ascii_lowercase()
}

fn is_canonical_sha256(value: &str) -> bool {
    is_canonical_hex(value, 32)
}

fn decode_canonical_hex(value: &str, expected_bytes: usize, code: &'static str) -> Result<Vec<u8>> {
    if value.len() != expected_bytes.saturating_mul(2) {
        bail!(
            "{code}: expected {} hex characters, got {}",
            expected_bytes.saturating_mul(2),
            value.len()
        );
    }
    if !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{code}: value must be contiguous ASCII hex without prefixes or separators");
    }
    Ok(value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect())
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => unreachable!("hex input was validated before decoding"),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

fn chunk_id_matches_asset_path(chunk_id: &str, asset_path: &str) -> bool {
    let package_id = package_id_from_asset_path(asset_path);
    let Some(chunk_id) = parse_chunk_id(chunk_id) else {
        return false;
    };
    chunk_id[..8] == package_id.0.to_le_bytes()
}

fn package_id_from_asset_path(asset_path: &str) -> FPackageId {
    FPackageId(FIoContainerId::from_name(asset_path).0)
}

fn parse_chunk_id(value: &str) -> Option<[u8; 12]> {
    if value.len() != 24 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0u8; 12];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        describe_fixed_leaves, prepare_reviewed_footstep_preset_size_v1, FixedLeafRole,
        ReviewedFootstepPresetSizeV1, ReviewedFootstepPresetTargetV1, FIXED_LEAF_SELECTOR_FORMAT,
        FIXED_LEAF_SELECTOR_PROFILE,
    };
    use retoc::legacy_asset::{
        EPackageFlags, FLegacyPackageFileSummary, FLegacyPackageHeader, FObjectExport,
        FObjectImport,
    };
    use retoc::logging::Log;
    use retoc::version::EngineVersion;
    use retoc::zen::FPackageIndex;
    use std::io::Cursor;

    struct ExtractFixture {
        _temp: tempfile::TempDir,
        receipt: ExtractReceiptEnvelope,
        uasset: PathBuf,
        copied_usmap: PathBuf,
        sidecar: PathBuf,
    }

    struct ManagedStageFixture {
        _temp: tempfile::TempDir,
        game_root: PathBuf,
        output_root: PathBuf,
        original_uasset: Vec<u8>,
        original_uexp: Vec<u8>,
        patched_uasset: Vec<u8>,
        patched_uexp: Vec<u8>,
        usmap: Vec<u8>,
        sidecar: Vec<u8>,
        generation: AssetGenerationReceipt,
        patch_path: PathBuf,
    }

    struct ManagedOfflineFixture {
        _temp: tempfile::TempDir,
        game_root: PathBuf,
        executable_path: PathBuf,
        usmap_path: PathBuf,
        generation: AssetGenerationReceipt,
        selector: FixedLeafSelector,
        replacement_hex: String,
        reviewed: ReviewedFootstepPresetReplacementV1,
        original_uasset: Vec<u8>,
        original_uexp: Vec<u8>,
        patched_uasset: Vec<u8>,
        patched_uexp: Vec<u8>,
        usmap: Vec<u8>,
        executable_sha256: [u8; 32],
    }

    impl ManagedOfflineFixture {
        fn source(&self) -> UnverifiedBorrowedManagedReviewedDataAssetSourceV1<'_> {
            UnverifiedBorrowedManagedReviewedDataAssetSourceV1 {
                target_path: ReviewedFootstepPresetTargetV1::Wolf.target_path(),
                generation: &self.generation,
                persisted_selector: &self.selector,
                persisted_replacement_hex: &self.replacement_hex,
                patched_uasset: &self.patched_uasset,
                patched_uexp: &self.patched_uexp,
                usmap: &self.usmap,
                sidecars: &[],
                expected_executable_length: fs::metadata(&self.executable_path).unwrap().len(),
                expected_executable_sha256: self.executable_sha256,
                reviewed: &self.reviewed,
            }
        }

        fn live_source(&self) -> LiveConvertedStageSource {
            LiveConvertedStageSource {
                generation: self.generation.clone(),
                uasset: self.original_uasset.clone(),
                uexp: self.original_uexp.clone(),
                sidecars: BTreeMap::new(),
            }
        }

        fn live_usmap(&self) -> VerifiedInput {
            read_verified_file_bounded(
                &self.usmap_path,
                MAX_USMAP_BYTES,
                "TEST_MANAGED_OFFLINE_USMAP",
            )
            .unwrap()
        }
    }

    fn sha256(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }

    fn receipt_component(name: &str, bytes: &[u8]) -> ReceiptComponent {
        ReceiptComponent {
            relative_path: name.to_owned(),
            length: u64::try_from(bytes.len()).unwrap(),
            sha256: encode_hex(&sha256(bytes)),
        }
    }

    fn source_file(path: &Path, anchor: &GenerationFileAnchor) -> SourceFileReceipt {
        SourceFileReceipt {
            path: path.display().to_string(),
            length: anchor.length,
            sha256: anchor.sha256.clone(),
        }
    }

    fn chunk_id(asset: &str, ordinal: u32) -> String {
        let package = package_id_from_asset_path(asset);
        let mut raw = [0u8; 12];
        raw[..8].copy_from_slice(&package.0.to_le_bytes());
        raw[8..].copy_from_slice(&ordinal.to_be_bytes());
        encode_hex(&raw)
    }

    fn generation_chunk(
        asset: &str,
        ordinal: u32,
        chunk_type: &str,
        winner: &GenerationFileAnchor,
    ) -> GenerationChunkAnchor {
        GenerationChunkAnchor {
            chunk_id: chunk_id(asset, ordinal),
            chunk_type: chunk_type.to_owned(),
            winner_utoc: winner.clone(),
            length: 1,
            blake3: "a1".repeat(32),
            toc_hash: "b2".repeat(20),
            toc_hash_bytes: 20,
        }
    }

    fn managed_offline_fixture() -> ManagedOfflineFixture {
        let temp = tempfile::tempdir().unwrap();
        let game_root = fs::canonicalize(temp.path()).unwrap();
        let executable_path = game_root.join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe");
        fs::create_dir_all(executable_path.parent().unwrap()).unwrap();
        fs::write(&executable_path, b"managed offline fixture executable").unwrap();
        let executable_sha256 = sha256(&fs::read(&executable_path).unwrap());

        let usmap_path = game_root.join("G1R/Binaries/Win64/ue4ss/Mappings.usmap");
        fs::create_dir_all(usmap_path.parent().unwrap()).unwrap();
        crate::test_fixture::write_valid_usmap(&usmap_path).unwrap();
        let usmap = fs::read(&usmap_path).unwrap();
        let schemas = SchemaDb::from_usmap_bounded(&usmap, UsmapLimits::default()).unwrap();

        let target = ReviewedFootstepPresetTargetV1::Wolf;
        let mut package = FLegacyPackageHeader::default();
        package.summary.versioning_info.package_file_version =
            EngineVersion::UE5_4.package_file_version();
        package.summary.versioning_info.is_unversioned = true;
        package.summary.package_name = target.target_path().to_owned();
        package.summary.package_flags = EPackageFlags::Cooked as u32
            | EPackageFlags::FilterEditorOnly as u32
            | EPackageFlags::UsesUnversionedProperties as u32;
        let core_uobject = package.name_map.store("/Script/CoreUObject");
        let package_class = package.name_map.store("Package");
        let class_class = package.name_map.store("Class");
        let module_name = package.name_map.store("/Script/G1R");
        let class_name = package.name_map.store("FootstepTag");
        let module_index = package.imports.len();
        package.imports.push(FObjectImport {
            class_package: core_uobject,
            class_name: package_class,
            object_name: module_name,
            ..FObjectImport::default()
        });
        let class_index = package.imports.len();
        package.imports.push(FObjectImport {
            class_package: core_uobject,
            class_name: class_class,
            outer_index: FPackageIndex::create_import(module_index as u32),
            object_name: class_name,
            ..FObjectImport::default()
        });
        let object_name = package.name_map.store(target.object_name());
        let mut original_uexp = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x05];
        original_uexp.extend_from_slice(&[0x00, 0x09]);
        for value in [10.0_f64, 10.0, 0.0, 1.0] {
            original_uexp.extend_from_slice(&value.to_le_bytes());
        }
        original_uexp.extend_from_slice(&1_i32.to_le_bytes());
        original_uexp.extend_from_slice(&2_i32.to_le_bytes());
        original_uexp.extend_from_slice(&3_i32.to_le_bytes());
        original_uexp.extend_from_slice(&0_i32.to_le_bytes());
        original_uexp.extend_from_slice(&2_i32.to_le_bytes());
        original_uexp.extend_from_slice(&11_i32.to_le_bytes());
        original_uexp.extend_from_slice(&0_i32.to_le_bytes());
        original_uexp.extend_from_slice(&[0x80, 0x03, 0x01]);
        original_uexp.extend_from_slice(&22_i32.to_le_bytes());
        original_uexp.extend_from_slice(&1_i32.to_le_bytes());
        original_uexp.extend_from_slice(&[0x00, 0x03, 0x01]);
        assert_eq!(original_uexp.len(), 82);
        package.exports.push(FObjectExport {
            class_index: FPackageIndex::create_import(class_index as u32),
            object_name,
            serial_offset: 0,
            serial_size: original_uexp.len() as i64,
            ..FObjectExport::default()
        });
        let mut serialized_header = Cursor::new(Vec::new());
        package
            .serialize(&mut serialized_header, None, &Log::no_log())
            .unwrap();
        let original_uasset = serialized_header.into_inner();
        original_uexp.extend_from_slice(&[0_u8; 4]);
        original_uexp.extend_from_slice(&FLegacyPackageFileSummary::PACKAGE_FILE_TAG.to_le_bytes());

        let original = PackageCarrier::from_bytes(
            original_uasset.clone(),
            original_uexp.clone(),
            asset_package_limits(),
        )
        .unwrap();
        let selector = {
            let envelope = LegacyPackageEnvelope::parse_g1r_ue5_4(&original).unwrap();
            let export = envelope.export(0).unwrap();
            describe_fixed_leaves(&original, &export, &schemas)
                .unwrap()
                .into_iter()
                .find(|leaf| {
                    leaf.editable
                        && leaf.selector.kind == FixedWireKind::Vector4F64x4
                        && leaf.selector.object_name == target.object_name()
                })
                .unwrap()
                .selector
        };
        let reviewed = prepare_reviewed_footstep_preset_size_v1(
            target.target_path(),
            &selector,
            ReviewedFootstepPresetSizeV1::try_new(11.0, 12.0).unwrap(),
        )
        .unwrap();
        let replacement_hex = encode_hex(reviewed.replacement_bytes());
        let mut patched = original;
        apply_fixed_leaf_selector_patch(
            &mut patched,
            &schemas,
            &selector,
            reviewed.expected_bytes(),
            reviewed.replacement_bytes(),
        )
        .unwrap();
        let (patched_uasset, patched_uexp) = patched.into_bytes();

        let main_anchor = GenerationFileAnchor {
            file_name: "G1R-Windows.utoc".to_owned(),
            length: 64,
            sha256: "11".repeat(32),
        };
        let global_utoc_anchor = GenerationFileAnchor {
            file_name: "global.utoc".to_owned(),
            length: 32,
            sha256: "22".repeat(32),
        };
        let global_ucas_anchor = GenerationFileAnchor {
            file_name: "global.ucas".to_owned(),
            length: 96,
            sha256: "33".repeat(32),
        };
        let usmap_anchor = GenerationFileAnchor {
            file_name: "Mappings.usmap".to_owned(),
            length: usmap.len() as u64,
            sha256: encode_hex(&sha256(&usmap)),
        };
        let mut container_set = vec![main_anchor.clone(), global_utoc_anchor.clone()];
        container_set.sort_by(|left, right| {
            left.file_name
                .cmp(&right.file_name)
                .then(left.sha256.cmp(&right.sha256))
        });
        let mut target_chunks = vec![
            generation_chunk(target.target_path(), 1, "ContainerHeader", &main_anchor),
            generation_chunk(target.target_path(), 2, "ExportBundleData", &main_anchor),
        ];
        target_chunks.sort_by(|left, right| {
            left.chunk_id
                .cmp(&right.chunk_id)
                .then(left.chunk_type.cmp(&right.chunk_type))
                .then(left.winner_utoc.file_name.cmp(&right.winner_utoc.file_name))
        });
        let generation = AssetGenerationReceipt {
            format: "gore.asset.generation.v1".to_owned(),
            asset: target.target_path().to_owned(),
            usmap: usmap_anchor,
            main_utoc: main_anchor,
            global_utoc: global_utoc_anchor,
            global_ucas: global_ucas_anchor,
            container_set,
            target_chunks,
        };
        validate_generation_receipt(&generation, "TEST_MANAGED_OFFLINE_GENERATION").unwrap();

        ManagedOfflineFixture {
            _temp: temp,
            game_root,
            executable_path,
            usmap_path,
            generation,
            selector,
            replacement_hex,
            reviewed,
            original_uasset,
            original_uexp,
            patched_uasset,
            patched_uexp,
            usmap,
            executable_sha256,
        }
    }

    fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, current: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(current).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if entry.file_type().unwrap().is_dir() {
                    visit(root, &path, output);
                } else {
                    output.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }

        let mut output = BTreeMap::new();
        visit(root, root, &mut output);
        output
    }

    fn managed_offline_preflight_error(
        source: UnverifiedBorrowedManagedReviewedDataAssetSourceV1<'_>,
    ) -> String {
        verify_managed_offline_dataasset_package_v1_with_live_source(
            Path::new("managed offline invalid input must not touch disk"),
            source,
            |_root, _target| panic!("invalid source must fail before live conversion"),
            |_root| panic!("invalid source must fail before reading live USMAP"),
            |_root, _target, _expected| {
                panic!("invalid source must fail before final generation probe")
            },
        )
        .unwrap_err()
        .to_string()
    }

    fn verified_chunk(chunk: &GenerationChunkAnchor, source_utoc: &Path) -> ReceiptVerifiedChunk {
        ReceiptVerifiedChunk {
            chunk_id: chunk.chunk_id.clone(),
            chunk_type: chunk.chunk_type.clone(),
            source_utoc: source_utoc.to_path_buf(),
            length: chunk.length,
            blake3: chunk.blake3.clone(),
            toc_hash: chunk.toc_hash.clone(),
            toc_hash_bytes: chunk.toc_hash_bytes,
        }
    }

    fn extract_fixture() -> ExtractFixture {
        let temp = tempfile::tempdir().unwrap();
        let game_root = fs::canonicalize(temp.path()).unwrap();
        let output = game_root.join("output");
        fs::create_dir(&output).unwrap();

        let asset = "/Game/TestAsset";
        let uasset_bytes = [0x10, 0x11];
        let uexp_bytes = [0x20, 0x21, 0x22];
        let usmap_bytes = [0x30, 0x31, 0x32, 0x33];
        let sidecar_bytes = [0x40, 0x41, 0x42];
        let uasset = output.join("TestAsset.uasset");
        let uexp = output.join("TestAsset.uexp");
        let copied_usmap_path = output.join(COPIED_USMAP_NAME);
        let sidecar = output.join("TestAsset.ubulk");
        fs::write(&uasset, uasset_bytes).unwrap();
        fs::write(&uexp, uexp_bytes).unwrap();
        fs::write(&copied_usmap_path, usmap_bytes).unwrap();
        fs::write(&sidecar, sidecar_bytes).unwrap();

        let main_utoc = game_root.join("G1R/Content/Paks/G1R-Windows.utoc");
        let main_ucas = game_root.join("G1R/Content/Paks/G1R-Windows.ucas");
        let global_utoc = game_root.join("G1R/Content/Paks/global.utoc");
        let global_ucas = game_root.join("G1R/Content/Paks/global.ucas");
        let source_usmap = game_root.join("G1R/Binaries/Win64/ue4ss/Mappings.usmap");
        let main_anchor = GenerationFileAnchor {
            file_name: "G1R-Windows.utoc".to_owned(),
            length: 64,
            sha256: "11".repeat(32),
        };
        let global_utoc_anchor = GenerationFileAnchor {
            file_name: "global.utoc".to_owned(),
            length: 32,
            sha256: "22".repeat(32),
        };
        let global_ucas_anchor = GenerationFileAnchor {
            file_name: "global.ucas".to_owned(),
            length: 96,
            sha256: "33".repeat(32),
        };
        let usmap_anchor = GenerationFileAnchor {
            file_name: "Mappings.usmap".to_owned(),
            length: u64::try_from(usmap_bytes.len()).unwrap(),
            sha256: encode_hex(&sha256(&usmap_bytes)),
        };
        let mut container_set = vec![main_anchor.clone(), global_utoc_anchor.clone()];
        container_set.sort_by(|left, right| {
            left.file_name
                .cmp(&right.file_name)
                .then(left.sha256.cmp(&right.sha256))
        });
        let mut target_chunks = vec![
            generation_chunk(asset, 1, "ContainerHeader", &main_anchor),
            generation_chunk(asset, 2, "ExportBundleData", &main_anchor),
            generation_chunk(asset, 3, "BulkData", &main_anchor),
        ];
        target_chunks.sort_by(|left, right| {
            left.chunk_id
                .cmp(&right.chunk_id)
                .then(left.chunk_type.cmp(&right.chunk_type))
                .then(left.winner_utoc.file_name.cmp(&right.winner_utoc.file_name))
        });
        let generation = AssetGenerationReceipt {
            format: "gore.asset.generation.v1".to_owned(),
            asset: asset.to_owned(),
            usmap: usmap_anchor.clone(),
            main_utoc: main_anchor.clone(),
            global_utoc: global_utoc_anchor.clone(),
            global_ucas: global_ucas_anchor.clone(),
            container_set,
            target_chunks: target_chunks.clone(),
        };

        let uasset_component = receipt_component("TestAsset.uasset", &uasset_bytes);
        let uexp_component = receipt_component("TestAsset.uexp", &uexp_bytes);
        let copied_usmap_component = receipt_component(COPIED_USMAP_NAME, &usmap_bytes);
        let sidecar_component = receipt_component("TestAsset.ubulk", &sidecar_bytes);
        let receipt = ExtractReceiptEnvelope {
            format: "gore.asset.extract.v2".to_owned(),
            status: "extracted".to_owned(),
            asset: asset.to_owned(),
            generation,
            source: ExtractReceiptSource {
                game_root: game_root.display().to_string(),
                composite_store_anchor: ExtractCompositeStoreAnchor {
                    utoc: source_file(&main_utoc, &main_anchor),
                    ucas: HeldIdentityReceipt {
                        path: main_ucas.display().to_string(),
                        length: 128,
                        modified_stamp: "fixture-stamp".to_owned(),
                        platform_identity: "fixture-identity".to_owned(),
                        sha256: None,
                        verification: HELD_IDENTITY_VERIFICATION.to_owned(),
                        content_hash_omitted: true,
                        limitation: HELD_IDENTITY_LIMITATION.to_owned(),
                    },
                    role: COMPOSITE_UCAS_ROLE.to_owned(),
                },
                consumed_chunks: target_chunks
                    .iter()
                    .map(|chunk| verified_chunk(chunk, &main_utoc))
                    .collect(),
                source_container_tocs: vec![
                    source_file(&main_utoc, &main_anchor),
                    source_file(&global_utoc, &global_utoc_anchor),
                ],
                content_binding: EXTRACT_CONTENT_BINDING.to_owned(),
                usmap: ExtractUsmapProof {
                    source: source_file(&source_usmap, &usmap_anchor),
                    copied_relative_path: COPIED_USMAP_NAME.to_owned(),
                    copy: copied_usmap_component.clone(),
                },
                global_script_store: GlobalScriptStoreProof {
                    utoc: source_file(&global_utoc, &global_utoc_anchor),
                    ucas: source_file(&global_ucas, &global_ucas_anchor),
                },
            },
            package_seal: PackagePairSeal {
                uasset_sha256: sha256(&uasset_bytes),
                uexp_sha256: sha256(&uexp_bytes),
            },
            output: ExtractReceiptOutput {
                root: output.display().to_string(),
                receipt: EXTRACT_RECEIPT_NAME.to_owned(),
                components: vec![
                    uasset_component,
                    uexp_component,
                    copied_usmap_component,
                    sidecar_component,
                ],
            },
            deployed: false,
        };
        validate_extract_receipt_envelope(&receipt, "TEST_EXTRACT").unwrap();
        ExtractFixture {
            _temp: temp,
            receipt,
            uasset,
            copied_usmap: copied_usmap_path,
            sidecar,
        }
    }

    fn verified_extract(fixture: &ExtractFixture) -> VerifiedExtractReceipt {
        let receipt_path = Path::new(&fixture.receipt.output.root).join(EXTRACT_RECEIPT_NAME);
        fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&fixture.receipt).unwrap(),
        )
        .unwrap();
        read_extract_receipt_v2(&receipt_path).unwrap()
    }

    fn patch_receipt(fixture: &ExtractFixture) -> PatchReceiptEnvelope {
        let output_root = Path::new(&fixture.receipt.output.root);
        let output_uasset = output_root.join("Patched.uasset");
        let output_uexp = output_root.join("Patched.uexp");
        let receipt_path = output_root.join(format!("Patched{PATCH_RECEIPT_SUFFIX}"));
        let extract_bytes = serde_json::to_vec(&fixture.receipt).unwrap();
        let extracted_sidecar = SidecarReceipt {
            role: SidecarRole::Bulk,
            file_name: "TestAsset.ubulk".to_owned(),
            length: fixture.receipt.output.components[3].length,
            sha256: fixture.receipt.output.components[3].sha256.clone(),
        };
        let output_sidecar = SidecarReceipt {
            file_name: "Patched.ubulk".to_owned(),
            ..extracted_sidecar.clone()
        };
        let selector = FixedLeafSelector {
            format: FIXED_LEAF_SELECTOR_FORMAT,
            profile: FIXED_LEAF_SELECTOR_PROFILE.to_owned(),
            package_seal: fixture.receipt.package_seal.clone(),
            usmap_sha256: fixture.receipt.generation.usmap.sha256.clone(),
            export_index: 0,
            object_name: "TestAsset".to_owned(),
            class_path: "/Script/Test.TestAsset".to_owned(),
            component: PackageComponent::Uexp,
            export_sha256: "55".repeat(32),
            role: FixedLeafRole::PropertyValue,
            kind: FixedWireKind::Bool,
            path: Vec::new(),
            expected_hex: "00".to_owned(),
        };
        let uasset = &fixture.receipt.output.components[0];
        let uexp = &fixture.receipt.output.components[1];
        PatchReceiptEnvelope {
            format: "gore.asset.patch-fixed.v2".to_owned(),
            status: "patched".to_owned(),
            asset: fixture.receipt.asset.clone(),
            generation_bound: true,
            provenance: PatchReceiptProvenance {
                extract_receipt: ReceiptFileSeal {
                    path: output_root.join(EXTRACT_RECEIPT_NAME).display().to_string(),
                    length: u64::try_from(extract_bytes.len()).unwrap(),
                    sha256: encode_hex(&sha256(&extract_bytes)),
                },
                generation: fixture.receipt.generation.clone(),
                usmap: fixture.receipt.generation.usmap.clone(),
                extract_components: fixture.receipt.output.components.clone(),
                extracted_sidecars: vec![extracted_sidecar],
            },
            input_package_seal: fixture.receipt.package_seal.clone(),
            output_package_seal: fixture.receipt.package_seal.clone(),
            output_sidecars: vec![output_sidecar.clone()],
            input_selector: selector,
            output_requires_reinspect: true,
            expected_hex: "00".to_owned(),
            replacement_hex: "01".to_owned(),
            patch: PatchOperationProof {
                before: fixture.receipt.package_seal.clone(),
                after: fixture.receipt.package_seal.clone(),
                export_index: 0,
                component: PackageComponent::Uexp,
                absolute_offset: 0,
                length: 1,
                kind: FixedWireKind::Bool,
            },
            output: PatchReceiptOutput {
                uasset: ComponentDigestProof {
                    path: output_uasset.display().to_string(),
                    length: uasset.length,
                    sha256: uasset.sha256.clone(),
                },
                uexp: ComponentDigestProof {
                    path: output_uexp.display().to_string(),
                    length: uexp.length,
                    sha256: uexp.sha256.clone(),
                },
                sidecars: vec![output_sidecar],
                receipt: receipt_path.display().to_string(),
            },
        }
    }

    fn verified_stage_fixture() -> (ManagedStageFixture, VerifiedFixedLeafStageInput) {
        const EXPORT_BYTES: [u8; 3] = [0x00, 0x03, 0x01];

        let temp = tempfile::tempdir().unwrap();
        let game_root = fs::canonicalize(temp.path()).unwrap();
        let output_root = game_root.join("output");
        fs::create_dir(&output_root).unwrap();
        let executable = game_root.join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"fixture executable").unwrap();

        let mut package = FLegacyPackageHeader::default();
        package.summary.versioning_info.package_file_version =
            EngineVersion::UE5_4.package_file_version();
        package.summary.versioning_info.is_unversioned = true;
        package.summary.package_name = "/Game/TestAsset".to_owned();
        package.summary.package_flags = EPackageFlags::Cooked as u32
            | EPackageFlags::FilterEditorOnly as u32
            | EPackageFlags::UsesUnversionedProperties as u32;
        let core_uobject = package.name_map.store("/Script/CoreUObject");
        let package_class = package.name_map.store("Package");
        let class_class = package.name_map.store("Class");
        let module_name = package.name_map.store("/Script/Test");
        let class_name = package.name_map.store("Fixture");
        let module_index = package.imports.len();
        package.imports.push(FObjectImport {
            class_package: core_uobject,
            class_name: package_class,
            object_name: module_name,
            ..FObjectImport::default()
        });
        let class_index = package.imports.len();
        package.imports.push(FObjectImport {
            class_package: core_uobject,
            class_name: class_class,
            outer_index: FPackageIndex::create_import(module_index as u32),
            object_name: class_name,
            ..FObjectImport::default()
        });
        let object_name = package.name_map.store("TestAsset");
        package.exports.push(FObjectExport {
            class_index: FPackageIndex::create_import(class_index as u32),
            object_name,
            serial_offset: 0,
            serial_size: EXPORT_BYTES.len() as i64,
            ..FObjectExport::default()
        });
        let mut serialized_header = Cursor::new(Vec::new());
        package
            .serialize(&mut serialized_header, None, &Log::no_log())
            .unwrap();
        let original_uasset = serialized_header.into_inner();
        let mut original_uexp = EXPORT_BYTES.to_vec();
        original_uexp.extend_from_slice(&FLegacyPackageFileSummary::PACKAGE_FILE_TAG.to_le_bytes());

        let mapping = usmap::Usmap {
            enums: Vec::new(),
            structs: vec![usmap::Struct {
                name: "Fixture".to_owned(),
                super_struct: None,
                properties: vec![usmap::Property {
                    name: "Enabled".to_owned(),
                    array_dim: 1,
                    index: 0,
                    inner: usmap::PropertyInner::Bool,
                }],
            }],
            cext: None,
            ppth: Some(usmap::ExtPpth {
                version: 0,
                enums: Vec::new(),
                structs: vec!["/Script/Test".to_owned()],
            }),
            eatr: Some(usmap::ExtEatr {
                version: 0,
                enum_flags: Vec::new(),
                struct_flags: vec![usmap::StructFlags {
                    type_: usmap::FlagsType::Class,
                    value: 0,
                    prop_flags: Vec::new(),
                }],
            }),
            envp: None,
        };
        let mut usmap = Vec::new();
        mapping.write(&mut usmap).unwrap();

        let original_path = output_root.join("TestAsset.uasset");
        let sidecar = vec![0x40, 0x41, 0x42];
        fs::write(&original_path, &original_uasset).unwrap();
        fs::write(original_path.with_extension("uexp"), &original_uexp).unwrap();
        fs::write(output_root.join("TestAsset.ubulk"), &sidecar).unwrap();
        fs::write(output_root.join(COPIED_USMAP_NAME), &usmap).unwrap();
        let schemas = SchemaDb::from_usmap_bounded(&usmap, UsmapLimits::default()).unwrap();
        let original_carrier =
            PackageCarrier::load(&original_path, asset_package_limits()).unwrap();
        let selector = {
            let envelope = LegacyPackageEnvelope::parse_g1r_ue5_4(&original_carrier).unwrap();
            let export = envelope.export(0).unwrap();
            describe_fixed_leaves(&original_carrier, &export, &schemas)
                .unwrap()
                .into_iter()
                .find(|leaf| {
                    leaf.editable
                        && leaf.selector.kind == FixedWireKind::Bool
                        && leaf.selector.expected_hex == "01"
                })
                .unwrap()
                .selector
        };
        let mut patched_carrier = original_carrier.clone();
        let proof =
            apply_fixed_leaf_selector_patch(&mut patched_carrier, &schemas, &selector, &[1], &[0])
                .unwrap();
        let patched_uasset = patched_carrier.bytes(PackageComponent::Uasset).to_vec();
        let patched_uexp = patched_carrier.bytes(PackageComponent::Uexp).to_vec();
        let patched_path = output_root.join("Patched.uasset");
        fs::write(&patched_path, &patched_uasset).unwrap();
        fs::write(patched_path.with_extension("uexp"), &patched_uexp).unwrap();
        fs::write(output_root.join("Patched.ubulk"), &sidecar).unwrap();

        let asset = "/Game/TestAsset";
        let main_utoc = game_root.join("G1R/Content/Paks/G1R-Windows.utoc");
        let main_ucas = game_root.join("G1R/Content/Paks/G1R-Windows.ucas");
        let global_utoc = game_root.join("G1R/Content/Paks/global.utoc");
        let global_ucas = game_root.join("G1R/Content/Paks/global.ucas");
        let source_usmap = game_root.join("G1R/Binaries/Win64/ue4ss/Mappings.usmap");
        let main_anchor = GenerationFileAnchor {
            file_name: "G1R-Windows.utoc".to_owned(),
            length: 64,
            sha256: "11".repeat(32),
        };
        let global_utoc_anchor = GenerationFileAnchor {
            file_name: "global.utoc".to_owned(),
            length: 32,
            sha256: "22".repeat(32),
        };
        let global_ucas_anchor = GenerationFileAnchor {
            file_name: "global.ucas".to_owned(),
            length: 96,
            sha256: "33".repeat(32),
        };
        let usmap_anchor = GenerationFileAnchor {
            file_name: "Mappings.usmap".to_owned(),
            length: usmap.len() as u64,
            sha256: encode_hex(&sha256(&usmap)),
        };
        let mut container_set = vec![main_anchor.clone(), global_utoc_anchor.clone()];
        container_set.sort_by(|left, right| {
            left.file_name
                .cmp(&right.file_name)
                .then(left.sha256.cmp(&right.sha256))
        });
        let mut target_chunks = vec![
            generation_chunk(asset, 1, "ContainerHeader", &main_anchor),
            generation_chunk(asset, 2, "ExportBundleData", &main_anchor),
            generation_chunk(asset, 3, "BulkData", &main_anchor),
        ];
        target_chunks.sort_by(|left, right| {
            left.chunk_id
                .cmp(&right.chunk_id)
                .then(left.chunk_type.cmp(&right.chunk_type))
                .then(left.winner_utoc.file_name.cmp(&right.winner_utoc.file_name))
        });
        let generation = AssetGenerationReceipt {
            format: "gore.asset.generation.v1".to_owned(),
            asset: asset.to_owned(),
            usmap: usmap_anchor.clone(),
            main_utoc: main_anchor.clone(),
            global_utoc: global_utoc_anchor.clone(),
            global_ucas: global_ucas_anchor.clone(),
            container_set,
            target_chunks: target_chunks.clone(),
        };
        let components = vec![
            receipt_component("TestAsset.uasset", &original_uasset),
            receipt_component("TestAsset.uexp", &original_uexp),
            receipt_component(COPIED_USMAP_NAME, &usmap),
            receipt_component("TestAsset.ubulk", &sidecar),
        ];
        let extracted_sidecar = SidecarReceipt {
            role: SidecarRole::Bulk,
            file_name: "TestAsset.ubulk".to_owned(),
            length: sidecar.len() as u64,
            sha256: encode_hex(&sha256(&sidecar)),
        };
        let output_sidecar = SidecarReceipt {
            file_name: "Patched.ubulk".to_owned(),
            ..extracted_sidecar.clone()
        };
        let extract_receipt = ExtractReceiptEnvelope {
            format: "gore.asset.extract.v2".to_owned(),
            status: "extracted".to_owned(),
            asset: asset.to_owned(),
            generation: generation.clone(),
            source: ExtractReceiptSource {
                game_root: game_root.display().to_string(),
                composite_store_anchor: ExtractCompositeStoreAnchor {
                    utoc: source_file(&main_utoc, &main_anchor),
                    ucas: HeldIdentityReceipt {
                        path: main_ucas.display().to_string(),
                        length: 128,
                        modified_stamp: "fixture-stamp".to_owned(),
                        platform_identity: "fixture-identity".to_owned(),
                        sha256: None,
                        verification: HELD_IDENTITY_VERIFICATION.to_owned(),
                        content_hash_omitted: true,
                        limitation: HELD_IDENTITY_LIMITATION.to_owned(),
                    },
                    role: COMPOSITE_UCAS_ROLE.to_owned(),
                },
                consumed_chunks: target_chunks
                    .iter()
                    .map(|chunk| verified_chunk(chunk, &main_utoc))
                    .collect(),
                source_container_tocs: vec![
                    source_file(&main_utoc, &main_anchor),
                    source_file(&global_utoc, &global_utoc_anchor),
                ],
                content_binding: EXTRACT_CONTENT_BINDING.to_owned(),
                usmap: ExtractUsmapProof {
                    source: source_file(&source_usmap, &usmap_anchor),
                    copied_relative_path: COPIED_USMAP_NAME.to_owned(),
                    copy: components[2].clone(),
                },
                global_script_store: GlobalScriptStoreProof {
                    utoc: source_file(&global_utoc, &global_utoc_anchor),
                    ucas: source_file(&global_ucas, &global_ucas_anchor),
                },
            },
            package_seal: proof.before.clone(),
            output: ExtractReceiptOutput {
                root: output_root.display().to_string(),
                receipt: EXTRACT_RECEIPT_NAME.to_owned(),
                components: components.clone(),
            },
            deployed: false,
        };
        let extract_path = output_root.join(EXTRACT_RECEIPT_NAME);
        let extract_bytes = serde_json::to_vec(&extract_receipt).unwrap();
        fs::write(&extract_path, &extract_bytes).unwrap();

        let patch_path = output_root.join(format!("Patched{PATCH_RECEIPT_SUFFIX}"));
        let patch_receipt = PatchReceiptEnvelope {
            format: "gore.asset.patch-fixed.v2".to_owned(),
            status: "patched".to_owned(),
            asset: asset.to_owned(),
            generation_bound: true,
            provenance: PatchReceiptProvenance {
                extract_receipt: ReceiptFileSeal {
                    path: extract_path.display().to_string(),
                    length: extract_bytes.len() as u64,
                    sha256: encode_hex(&sha256(&extract_bytes)),
                },
                generation: generation.clone(),
                usmap: GenerationFileAnchor {
                    file_name: COPIED_USMAP_NAME.to_owned(),
                    ..usmap_anchor
                },
                extract_components: components,
                extracted_sidecars: vec![extracted_sidecar],
            },
            input_package_seal: proof.before.clone(),
            output_package_seal: proof.after.clone(),
            output_sidecars: vec![output_sidecar.clone()],
            input_selector: selector.clone(),
            output_requires_reinspect: true,
            expected_hex: selector.expected_hex.clone(),
            replacement_hex: "00".to_owned(),
            patch: PatchOperationProof {
                before: proof.before,
                after: proof.after,
                export_index: proof.export_index,
                component: proof.component,
                absolute_offset: proof.absolute_offset,
                length: proof.length,
                kind: proof.kind,
            },
            output: PatchReceiptOutput {
                uasset: ComponentDigestProof {
                    path: patched_path.display().to_string(),
                    length: patched_uasset.len() as u64,
                    sha256: encode_hex(&sha256(&patched_uasset)),
                },
                uexp: ComponentDigestProof {
                    path: patched_path.with_extension("uexp").display().to_string(),
                    length: patched_uexp.len() as u64,
                    sha256: encode_hex(&sha256(&patched_uexp)),
                },
                sidecars: vec![output_sidecar],
                receipt: patch_path.display().to_string(),
            },
        };
        fs::write(&patch_path, serde_json::to_vec(&patch_receipt).unwrap()).unwrap();
        let verified_patch = read_patch_receipt_v2(&patch_path).unwrap();
        let live_uasset = original_uasset.clone();
        let live_uexp = original_uexp.clone();
        let live_sidecar = sidecar.clone();
        let stage = verify_fixed_leaf_stage_input_with_live_source(
            verified_patch,
            move |_game_root, _asset, expected| {
                Ok(LiveConvertedStageSource {
                    generation: expected.clone(),
                    uasset: live_uasset,
                    uexp: live_uexp,
                    sidecars: BTreeMap::from([(SidecarRole::Bulk, live_sidecar)]),
                })
            },
            |_game_root, _asset, expected| Ok(expected.clone()),
        )
        .unwrap();
        (
            ManagedStageFixture {
                _temp: temp,
                game_root,
                output_root,
                original_uasset,
                original_uexp,
                patched_uasset,
                patched_uexp,
                usmap,
                sidecar,
                generation,
                patch_path,
            },
            stage,
        )
    }

    fn normalized_extract_wire(fixture: &ExtractFixture) -> ExtractReceiptEnvelope {
        let mut receipt = fixture.receipt.clone();
        let paks = "/fixture/game/G1R/Content/Paks";
        let main_utoc = format!("{paks}/G1R-Windows.utoc");
        receipt.source.game_root = "/fixture/game".to_owned();
        receipt.source.composite_store_anchor.utoc.path = main_utoc.clone();
        receipt.source.composite_store_anchor.ucas.path = format!("{paks}/G1R-Windows.ucas");
        for chunk in &mut receipt.source.consumed_chunks {
            chunk.source_utoc = PathBuf::from(&main_utoc);
        }
        for toc in &mut receipt.source.source_container_tocs {
            let file_name = Path::new(&toc.path).file_name().unwrap().to_string_lossy();
            toc.path = format!("{paks}/{file_name}");
        }
        receipt.source.usmap.source.path =
            "/fixture/game/G1R/Binaries/Win64/ue4ss/Mappings.usmap".to_owned();
        receipt.source.global_script_store.utoc.path = format!("{paks}/global.utoc");
        receipt.source.global_script_store.ucas.path = format!("{paks}/global.ucas");
        receipt.output.root = "/fixture/extract".to_owned();
        receipt
    }

    fn normalized_patch_wire(
        fixture: &ExtractFixture,
        extract: &ExtractReceiptEnvelope,
    ) -> PatchReceiptEnvelope {
        let mut receipt = patch_receipt(fixture);
        let extract_bytes = serde_json::to_vec(extract).unwrap();
        receipt.provenance.extract_receipt = ReceiptFileSeal {
            path: format!("/fixture/extract/{EXTRACT_RECEIPT_NAME}"),
            length: u64::try_from(extract_bytes.len()).unwrap(),
            sha256: encode_hex(&sha256(&extract_bytes)),
        };
        receipt.output.uasset.path = "/fixture/output/Patched.uasset".to_owned();
        receipt.output.uexp.path = "/fixture/output/Patched.uexp".to_owned();
        receipt.output.receipt = format!("/fixture/output/Patched{PATCH_RECEIPT_SUFFIX}");
        receipt
    }

    #[test]
    fn receipt_wire_is_golden_closed_and_round_trips() {
        let sidecar = SidecarReceipt {
            role: SidecarRole::Bulk,
            file_name: "TestAsset.ubulk".to_owned(),
            length: 3,
            sha256: "ab".repeat(32),
        };
        assert_eq!(
            serde_json::to_string(&sidecar).unwrap(),
            format!(
                "{{\"role\":\"BulkData\",\"file_name\":\"TestAsset.ubulk\",\"length\":3,\"sha256\":\"{}\"}}",
                "ab".repeat(32)
            )
        );

        let fixture = extract_fixture();
        let extract_json = serde_json::to_string(&fixture.receipt).unwrap();
        assert_eq!(
            serde_json::from_str::<ExtractReceiptEnvelope>(&extract_json).unwrap(),
            fixture.receipt
        );
        let mut unknown: serde_json::Value = serde_json::from_str(&extract_json).unwrap();
        unknown
            .as_object_mut()
            .unwrap()
            .insert("caller_authority".to_owned(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<ExtractReceiptEnvelope>(unknown).is_err());
        let duplicate = extract_json.replacen('{', "{\"format\":\"duplicate\",", 1);
        assert!(serde_json::from_str::<ExtractReceiptEnvelope>(&duplicate).is_err());

        let patch = patch_receipt(&fixture);
        let patch_json = serde_json::to_string(&patch).unwrap();
        assert_eq!(
            serde_json::from_str::<PatchReceiptEnvelope>(&patch_json).unwrap(),
            patch
        );
        let mut unknown: serde_json::Value = serde_json::from_str(&patch_json).unwrap();
        unknown
            .as_object_mut()
            .unwrap()
            .insert("caller_authority".to_owned(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<PatchReceiptEnvelope>(unknown).is_err());
        let duplicate = patch_json.replacen('{', "{\"format\":\"duplicate\",", 1);
        assert!(serde_json::from_str::<PatchReceiptEnvelope>(&duplicate).is_err());

        // These full compact-envelope bytes were frozen from the pre-extraction v2 wire, not
        // merely deserialized back into the same current type. Field reordering, rename/default
        // drift, or enum-wire drift changes the hashes even if a self-roundtrip would still pass.
        let golden_extract = normalized_extract_wire(&fixture);
        let extract_bytes = serde_json::to_vec(&golden_extract).unwrap();
        assert_eq!(extract_bytes.len(), 5_578);
        assert_eq!(
            encode_hex(&sha256(&extract_bytes)),
            "f454cfe2c2935ca204c232f40a8897acd8b21e21f3fc358317c97e7ae014f581"
        );
        let golden_patch = normalized_patch_wire(&fixture, &golden_extract);
        let patch_bytes = serde_json::to_vec(&golden_patch).unwrap();
        assert_eq!(patch_bytes.len(), 5_154);
        assert_eq!(
            encode_hex(&sha256(&patch_bytes)),
            "d190e481ddd3241ad099669ff5556f2b90a53493e90fcebe0040da1adb7db4d3"
        );
    }

    #[test]
    fn receipt_readers_are_bounded_and_bind_the_receipt_path() {
        let fixture = extract_fixture();
        let output_root = Path::new(&fixture.receipt.output.root);
        let extract_path = output_root.join(EXTRACT_RECEIPT_NAME);
        fs::write(
            &extract_path,
            serde_json::to_vec_pretty(&fixture.receipt).unwrap(),
        )
        .unwrap();
        let verified = read_extract_receipt_v2(&extract_path).unwrap();
        assert_eq!(verified.receipt(), &fixture.receipt);
        assert_eq!(verified.binding().output_root(), output_root);

        let patch = patch_receipt(&fixture);
        let patch_path = PathBuf::from(&patch.output.receipt);
        fs::write(&patch_path, serde_json::to_vec_pretty(&patch).unwrap()).unwrap();
        assert_eq!(
            read_patch_receipt_v2(&patch_path).unwrap().receipt(),
            &patch
        );

        let oversized = output_root.join("oversized.json");
        fs::write(
            &oversized,
            vec![b' '; usize::try_from(MAX_RECEIPT_BYTES + 1).unwrap()],
        )
        .unwrap();
        let error = read_extract_receipt_v2(&oversized).unwrap_err().to_string();
        assert!(error.contains("limit is 8388608"), "{error}");
    }

    #[test]
    fn pair_drift_is_rejected_after_extract_validation() {
        let fixture = extract_fixture();
        let extract = verified_extract(&fixture);
        let usmap =
            read_verified_file_bounded(&fixture.copied_usmap, MAX_USMAP_BYTES, "TEST_USMAP")
                .unwrap();
        let carrier = PackageCarrier::load(&fixture.uasset, asset_package_limits()).unwrap();
        assert_eq!(
            validate_extract_receipt_components(&extract, &carrier, &usmap)
                .unwrap()
                .len(),
            1
        );

        fs::write(&fixture.uasset, [0x99, 0x11]).unwrap();
        let drifted = PackageCarrier::load(&fixture.uasset, asset_package_limits()).unwrap();
        let error = validate_extract_receipt_components(&extract, &drifted, &usmap)
            .unwrap_err()
            .to_string();
        assert!(error.contains("sealed input pair"), "{error}");
    }

    #[test]
    fn usmap_drift_is_rejected_after_extract_validation() {
        let fixture = extract_fixture();
        let extract = verified_extract(&fixture);
        let carrier = PackageCarrier::load(&fixture.uasset, asset_package_limits()).unwrap();
        let drifted_path = Path::new(&fixture.receipt.output.root).join("drifted.usmap");
        fs::write(&drifted_path, [0x30, 0x31, 0x32, 0xff]).unwrap();
        let drifted =
            read_verified_file_bounded(&drifted_path, MAX_USMAP_BYTES, "TEST_USMAP").unwrap();
        let error = validate_extract_receipt_components(&extract, &carrier, &drifted)
            .unwrap_err()
            .to_string();
        assert!(error.contains("supplied USMAP"), "{error}");
    }

    #[test]
    fn sidecar_drift_is_rejected_after_extract_validation() {
        let fixture = extract_fixture();
        let extract = verified_extract(&fixture);
        let usmap =
            read_verified_file_bounded(&fixture.copied_usmap, MAX_USMAP_BYTES, "TEST_USMAP")
                .unwrap();
        let carrier = PackageCarrier::load(&fixture.uasset, asset_package_limits()).unwrap();
        let sealed_sidecars =
            validate_extract_receipt_components(&extract, &carrier, &usmap).unwrap();
        fs::write(&fixture.sidecar, [0x40, 0x41, 0xff]).unwrap();
        assert!(sealed_sidecars[0]
            .reverify(MAX_OPTIONAL_SIDECAR_BYTES, "TEST_SIDECAR")
            .is_err());
        let error = validate_extract_receipt_components(&extract, &carrier, &usmap)
            .unwrap_err()
            .to_string();
        assert!(error.contains("optional sidecar set or content"), "{error}");
    }

    #[test]
    fn generation_drift_is_rejected_by_extract_and_patch_receipts() {
        let fixture = extract_fixture();
        for unsafe_name in [".", "..", "C:foo", "NUL.utoc", "bad\u{7}.utoc", "bad."] {
            let mut malformed = fixture.receipt.generation.clone();
            malformed.usmap.file_name = unsafe_name.to_owned();
            assert!(
                validate_generation_receipt(&malformed, "TEST_GENERATION").is_err(),
                "unexpectedly accepted {unsafe_name:?}"
            );
        }
        let mut changed_generation = fixture.receipt.generation.clone();
        changed_generation.usmap.sha256 = "99".repeat(32);
        assert_eq!(
            generation_mismatch_reason(&fixture.receipt.generation, &changed_generation),
            "USMAP anchor"
        );
        let mut extract = fixture.receipt.clone();
        extract.generation = changed_generation;
        assert!(validate_extract_receipt_envelope(&extract, "TEST_EXTRACT").is_err());

        let patch_path =
            Path::new(&fixture.receipt.output.root).join(format!("Patched{PATCH_RECEIPT_SUFFIX}"));
        let patch = patch_receipt(&fixture);
        validate_patch_receipt_envelope(&patch, &patch_path).unwrap();

        let mut pair_drift = patch.clone();
        pair_drift.output.uexp.path = Path::new(&fixture.receipt.output.root)
            .join("Other.uexp")
            .display()
            .to_string();
        assert!(validate_patch_receipt_envelope(&pair_drift, &patch_path).is_err());

        let mut sidecar_drift = patch.clone();
        sidecar_drift.output_sidecars.clear();
        sidecar_drift.output.sidecars.clear();
        assert!(validate_patch_receipt_envelope(&sidecar_drift, &patch_path).is_err());

        let mut generation_drift = patch;
        generation_drift.provenance.generation.usmap.sha256 = "99".repeat(32);
        assert!(validate_patch_receipt_envelope(&generation_drift, &patch_path).is_err());
    }

    #[test]
    fn managed_stage_input_binds_the_complete_verified_chain() {
        let (fixture, stage) = verified_stage_fixture();
        assert_eq!(stage.target_path(), "/Game/TestAsset");
        assert_eq!(stage.generation().asset, stage.target_path());
        assert_eq!(
            stage.patched_component_bytes(PackageComponent::Uasset),
            fixture.patched_uasset
        );
        assert_eq!(
            stage.patched_component_bytes(PackageComponent::Uexp),
            fixture.patched_uexp
        );
        assert_eq!(stage.usmap_bytes(), fixture.usmap);
        assert_eq!(stage.sidecars().len(), 1);
        assert_eq!(stage.sidecars()[0].role(), SidecarRole::Bulk);
        assert_eq!(stage.sidecars()[0].bytes(), fixture.sidecar);
        assert_eq!(stage.sidecars()[0].length(), fixture.sidecar.len() as u64);
        assert_eq!(stage.sidecars()[0].sha256(), &sha256(&fixture.sidecar));
        assert_eq!(stage.generation(), &fixture.generation);
        assert!(stage.executable_anchor().length() > 0);

        let debug = format!("{stage:?}");
        assert!(!debug.contains(&fixture.game_root.display().to_string()));
        assert!(!debug.contains(&fixture.output_root.display().to_string()));
        assert!(!debug.contains(&fixture.patch_path.display().to_string()));
    }

    #[test]
    fn direct_semantic_stage_edit_uses_private_patch_chain_and_leaves_no_artifacts() {
        let (fixture, existing_stage) = verified_stage_fixture();
        let private_parent = tempfile::tempdir().unwrap();
        let extract =
            read_extract_receipt_v2(&fixture.output_root.join(EXTRACT_RECEIPT_NAME)).unwrap();
        let live_uasset = fixture.original_uasset.clone();
        let live_uexp = fixture.original_uexp.clone();
        let live_sidecar = fixture.sidecar.clone();
        let stage = verify_fixed_leaf_stage_edit_with_live_source_in(
            extract,
            existing_stage.selector().clone(),
            "00",
            private_parent.path(),
            move |_game_root, _asset, expected| {
                Ok(LiveConvertedStageSource {
                    generation: expected.clone(),
                    uasset: live_uasset,
                    uexp: live_uexp,
                    sidecars: BTreeMap::from([(SidecarRole::Bulk, live_sidecar)]),
                })
            },
            |_game_root, _asset, expected| Ok(expected.clone()),
        )
        .unwrap();

        assert_eq!(stage.target_path(), "/Game/TestAsset");
        assert_eq!(stage.replacement_hex(), "00");
        assert_eq!(
            stage.patched_component_bytes(PackageComponent::Uasset),
            fixture.patched_uasset
        );
        assert_eq!(
            stage.patched_component_bytes(PackageComponent::Uexp),
            fixture.patched_uexp
        );
        assert_eq!(stage.sidecars()[0].bytes(), fixture.sidecar);
        assert_eq!(fs::read_dir(private_parent.path()).unwrap().count(), 0);

        let outside_store = tempfile::tempdir().unwrap();
        stage
            .require_store_root_disjoint(outside_store.path())
            .unwrap();
        assert!(stage
            .require_store_root_disjoint(&fixture.output_root)
            .is_err());
    }

    #[test]
    fn direct_semantic_stage_edit_rejects_noncanonical_and_noop_values_before_live_use() {
        for replacement in ["01", "0A", "0000"] {
            let (fixture, existing_stage) = verified_stage_fixture();
            let private_parent = tempfile::tempdir().unwrap();
            let extract =
                read_extract_receipt_v2(&fixture.output_root.join(EXTRACT_RECEIPT_NAME)).unwrap();
            let error = verify_fixed_leaf_stage_edit_with_live_source_in(
                extract,
                existing_stage.selector().clone(),
                replacement,
                private_parent.path(),
                |_game_root, _asset, _expected| {
                    panic!("invalid semantic value must fail before live conversion")
                },
                |_game_root, _asset, _expected| {
                    panic!("invalid semantic value must fail before final generation probe")
                },
            )
            .unwrap_err()
            .to_string();
            assert!(
                error.contains("replacement")
                    || error.contains("NoChange")
                    || error.contains("no change")
                    || error.contains("ASSET_STAGE_EDIT_SEMANTICS"),
                "replacement {replacement:?}: {error}"
            );
            assert_eq!(fs::read_dir(private_parent.path()).unwrap().count(), 0);
        }
    }

    #[test]
    fn installed_snapshot_stage_edit_reuses_exact_live_bytes_and_normalizes_direct_g1r_root() {
        let (fixture, existing_stage) = verified_stage_fixture();
        let live_usmap = fixture
            .game_root
            .join("G1R/Binaries/Win64/ue4ss/Mappings.usmap");
        fs::create_dir_all(live_usmap.parent().unwrap()).unwrap();
        fs::write(&live_usmap, &fixture.usmap).unwrap();
        let target_path = existing_stage.target_path().to_owned();
        let selector = existing_stage.selector().clone();
        let installed_sidecars = BTreeMap::from([(SidecarRole::Bulk, fixture.sidecar.as_slice())]);
        let installed_source = InstalledGenerationSourceProof::from_generation(&fixture.generation);
        let normalized_root = fixture.game_root.clone();
        let closure_target_path = target_path.clone();
        let live_generation = fixture.generation.clone();
        let final_generation = fixture.generation.clone();
        let live_uasset = fixture.original_uasset.clone();
        let live_uexp = fixture.original_uexp.clone();
        let live_sidecar = fixture.sidecar.clone();

        let stage = verify_fixed_leaf_stage_edit_from_installed_parts_with_live_source(
            &fixture.game_root.join("G1R"),
            &target_path,
            &fixture.original_uasset,
            &fixture.original_uexp,
            &installed_sidecars,
            &installed_source,
            &fixture.usmap,
            "Mappings.usmap",
            selector,
            "00",
            move |root, asset| {
                assert_eq!(root, normalized_root);
                assert_eq!(asset, closure_target_path);
                Ok(LiveConvertedStageSource {
                    generation: live_generation,
                    uasset: live_uasset,
                    uexp: live_uexp,
                    sidecars: BTreeMap::from([(SidecarRole::Bulk, live_sidecar)]),
                })
            },
            move |_root, _asset, expected| {
                assert_eq!(expected, &final_generation);
                Ok(final_generation)
            },
        )
        .unwrap();

        assert_eq!(stage.target_path(), "/Game/TestAsset");
        assert_eq!(stage.generation(), &fixture.generation);
        assert_eq!(stage.replacement_hex(), "00");
        assert_eq!(
            stage.patched_component_bytes(PackageComponent::Uasset),
            fixture.patched_uasset
        );
        assert_eq!(
            stage.patched_component_bytes(PackageComponent::Uexp),
            fixture.patched_uexp
        );
        assert_eq!(stage.usmap_bytes(), fixture.usmap);
        assert_eq!(stage.sidecars()[0].role(), SidecarRole::Bulk);
        assert_eq!(stage.sidecars()[0].bytes(), fixture.sidecar);
        assert!(stage.retained_source_roots.is_empty());
    }

    #[test]
    fn installed_snapshot_stage_edit_rejects_pair_sidecar_winner_and_usmap_identity_mismatches() {
        enum Drift {
            Pair,
            Sidecar,
            Winner,
            Usmap,
            UsmapFileName,
        }

        for drift in [
            Drift::Pair,
            Drift::Sidecar,
            Drift::Winner,
            Drift::Usmap,
            Drift::UsmapFileName,
        ] {
            let (fixture, existing_stage) = verified_stage_fixture();
            let live_usmap = fixture
                .game_root
                .join("G1R/Binaries/Win64/ue4ss/Mappings.usmap");
            fs::create_dir_all(live_usmap.parent().unwrap()).unwrap();
            fs::write(&live_usmap, &fixture.usmap).unwrap();
            let target_path = existing_stage.target_path().to_owned();
            let selector = existing_stage.selector().clone();
            let mut installed_uasset = fixture.original_uasset.clone();
            let mut installed_sidecar = fixture.sidecar.clone();
            let mut installed_source =
                InstalledGenerationSourceProof::from_generation(&fixture.generation);
            let mut installed_usmap = fixture.usmap.clone();
            let mut installed_usmap_file_name = "Mappings.usmap";
            let expected_code = match drift {
                Drift::Pair => {
                    installed_uasset[0] ^= 0x01;
                    "ASSET_STAGE_INSTALLED_GENERATION"
                }
                Drift::Sidecar => {
                    installed_sidecar[0] ^= 0x01;
                    "ASSET_STAGE_INSTALLED_GENERATION"
                }
                Drift::Winner => {
                    installed_source
                        .target_chunks
                        .iter_mut()
                        .find(|chunk| chunk.chunk_type == "ExportBundleData")
                        .unwrap()
                        .winner_utoc
                        .file_name = "SameBytesOverride.utoc".to_owned();
                    "ASSET_STAGE_INSTALLED_SOURCE"
                }
                Drift::Usmap => {
                    installed_usmap[0] ^= 0x01;
                    "ASSET_STAGE_INSTALLED_USMAP"
                }
                Drift::UsmapFileName => {
                    installed_usmap_file_name = "SameBytesOther.usmap";
                    "ASSET_STAGE_INSTALLED_USMAP"
                }
            };
            let installed_sidecars =
                BTreeMap::from([(SidecarRole::Bulk, installed_sidecar.as_slice())]);
            let live_generation = fixture.generation.clone();
            let live_uasset = fixture.original_uasset.clone();
            let live_uexp = fixture.original_uexp.clone();
            let live_sidecar = fixture.sidecar.clone();

            let error = verify_fixed_leaf_stage_edit_from_installed_parts_with_live_source(
                &fixture.game_root,
                &target_path,
                &installed_uasset,
                &fixture.original_uexp,
                &installed_sidecars,
                &installed_source,
                &installed_usmap,
                installed_usmap_file_name,
                selector,
                "00",
                move |_root, _asset| {
                    Ok(LiveConvertedStageSource {
                        generation: live_generation,
                        uasset: live_uasset,
                        uexp: live_uexp,
                        sidecars: BTreeMap::from([(SidecarRole::Bulk, live_sidecar)]),
                    })
                },
                |_root, _asset, _expected| {
                    panic!("mismatched installed bytes must fail before final generation probe")
                },
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains(expected_code), "{error}");
        }
    }

    #[test]
    fn generation_builder_rejects_parsed_utoc_aba_and_incomplete_opened_set() {
        let temp = tempfile::tempdir().unwrap();
        let main_path = temp.path().join("G1R-Windows.utoc");
        let global_path = temp.path().join("global.utoc");
        let global_ucas_path = temp.path().join("global.ucas");
        let usmap_path = temp.path().join("Mappings.usmap");
        fs::write(&main_path, b"exact main toc bytes").unwrap();
        fs::write(&global_path, b"exact global toc bytes").unwrap();
        fs::write(&global_ucas_path, b"exact global ucas bytes").unwrap();
        fs::write(&usmap_path, b"exact mapping bytes").unwrap();
        let main = digest_regular_file_bounded(&main_path, 1024, "TEST_GENERATION").unwrap();
        let global = digest_regular_file_bounded(&global_path, 1024, "TEST_GENERATION").unwrap();
        let global_ucas =
            digest_regular_file_bounded(&global_ucas_path, 1024, "TEST_GENERATION").unwrap();
        let usmap = digest_regular_file_bounded(&usmap_path, 1024, "TEST_GENERATION").unwrap();
        let asset = "/Game/TestAsset";
        let chunks = vec![
            gore_tex::container::VerifiedChunkReceipt {
                chunk_id: chunk_id(asset, 1),
                chunk_type: "ContainerHeader".to_owned(),
                source_utoc: main.path.clone(),
                source_utoc_blake3: encode_hex(&main.blake3),
                length: 1,
                blake3: "a1".repeat(32),
                toc_hash: "b2".repeat(20),
                toc_hash_bytes: 20,
            },
            gore_tex::container::VerifiedChunkReceipt {
                chunk_id: chunk_id(asset, 2),
                chunk_type: "ExportBundleData".to_owned(),
                source_utoc: main.path.clone(),
                source_utoc_blake3: encode_hex(&main.blake3),
                length: 1,
                blake3: "a3".repeat(32),
                toc_hash: "b4".repeat(20),
                toc_hash_bytes: 20,
            },
        ];
        let opened = vec![
            gore_tex::container::VerifiedOpenedUtocReceipt {
                source_utoc: main.path.clone(),
                source_utoc_blake3: encode_hex(&main.blake3),
            },
            gore_tex::container::VerifiedOpenedUtocReceipt {
                source_utoc: global.path.clone(),
                source_utoc_blake3: encode_hex(&global.blake3),
            },
        ];

        build_generation_receipt_from_probe(
            asset,
            &usmap,
            &main,
            &global,
            &global_ucas,
            &chunks,
            &[main.clone(), global.clone()],
            &opened,
            "TEST_GENERATION",
        )
        .unwrap();

        let mut transient_open = opened.clone();
        transient_open[0].source_utoc_blake3 = "ff".repeat(32);
        let error = build_generation_receipt_from_probe(
            asset,
            &usmap,
            &main,
            &global,
            &global_ucas,
            &chunks,
            &[main.clone(), global.clone()],
            &transient_open,
            "TEST_GENERATION",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("parsed UTOC bytes differ"), "{error}");

        let mut transient_winner = chunks.clone();
        transient_winner[1].source_utoc_blake3 = "ee".repeat(32);
        let error = build_generation_receipt_from_probe(
            asset,
            &usmap,
            &main,
            &global,
            &global_ucas,
            &transient_winner,
            &[main.clone(), global.clone()],
            &opened,
            "TEST_GENERATION",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("winning chunk UTOC bytes differ"), "{error}");

        let error = build_generation_receipt_from_probe(
            asset,
            &usmap,
            &main,
            &global,
            &global_ucas,
            &chunks,
            &[main.clone(), global.clone()],
            &opened[..1],
            "TEST_GENERATION",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("source UTOC sets differ"), "{error}");
    }

    #[test]
    fn installed_snapshot_stage_edit_rejects_final_generation_drift() {
        let (fixture, existing_stage) = verified_stage_fixture();
        let live_usmap = fixture
            .game_root
            .join("G1R/Binaries/Win64/ue4ss/Mappings.usmap");
        fs::create_dir_all(live_usmap.parent().unwrap()).unwrap();
        fs::write(&live_usmap, &fixture.usmap).unwrap();
        let installed_sidecars = BTreeMap::from([(SidecarRole::Bulk, fixture.sidecar.as_slice())]);
        let installed_source = InstalledGenerationSourceProof::from_generation(&fixture.generation);
        let live_generation = fixture.generation.clone();
        let live_uasset = fixture.original_uasset.clone();
        let live_uexp = fixture.original_uexp.clone();
        let live_sidecar = fixture.sidecar.clone();

        let error = verify_fixed_leaf_stage_edit_from_installed_parts_with_live_source(
            &fixture.game_root,
            existing_stage.target_path(),
            &fixture.original_uasset,
            &fixture.original_uexp,
            &installed_sidecars,
            &installed_source,
            &fixture.usmap,
            "Mappings.usmap",
            existing_stage.selector().clone(),
            "00",
            move |_root, _asset| {
                Ok(LiveConvertedStageSource {
                    generation: live_generation,
                    uasset: live_uasset,
                    uexp: live_uexp,
                    sidecars: BTreeMap::from([(SidecarRole::Bulk, live_sidecar)]),
                })
            },
            |_root, _asset, expected| {
                let mut drifted = expected.clone();
                drifted.target_chunks[0].length += 1;
                Ok(drifted)
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("changed during verification"), "{error}");
    }

    #[test]
    fn installed_snapshot_stage_edit_rejects_invalid_values_before_live_access() {
        let root = Path::new("this root must never be opened");
        for replacement in ["01", "0A", "0000"] {
            let selector = FixedLeafSelector {
                format: FIXED_LEAF_SELECTOR_FORMAT,
                profile: FIXED_LEAF_SELECTOR_PROFILE.to_owned(),
                package_seal: PackagePairSeal {
                    uasset_sha256: [0; 32],
                    uexp_sha256: [0; 32],
                },
                usmap_sha256: "00".repeat(32),
                export_index: 0,
                object_name: "TestAsset".to_owned(),
                class_path: "/Script/Test.Fixture".to_owned(),
                component: PackageComponent::Uexp,
                export_sha256: "00".repeat(32),
                role: FixedLeafRole::PropertyValue,
                kind: FixedWireKind::Bool,
                path: Vec::new(),
                expected_hex: "01".to_owned(),
            };
            let error = verify_fixed_leaf_stage_edit_from_installed_parts_with_live_source(
                root,
                "/Game/TestAsset",
                &[],
                &[],
                &BTreeMap::new(),
                &InstalledGenerationSourceProof {
                    container_set: Vec::new(),
                    target_chunks: Vec::new(),
                },
                &[],
                "Mappings.usmap",
                selector,
                replacement,
                |_root, _asset| panic!("invalid edit must fail before live conversion"),
                |_root, _asset, _expected| {
                    panic!("invalid edit must fail before final generation probe")
                },
            )
            .unwrap_err()
            .to_string();
            assert!(
                error.contains("ASSET_STAGE_INSTALLED_REPLACEMENT"),
                "{error}"
            );
        }
    }

    #[test]
    fn managed_stage_input_owns_bytes_and_executable_drift_fails_closed() {
        let (fixture, stage) = verified_stage_fixture();
        fs::write(
            fixture.output_root.join("Patched.uexp"),
            vec![0xff; fixture.patched_uexp.len()],
        )
        .unwrap();
        assert_eq!(
            stage.patched_component_bytes(PackageComponent::Uexp),
            fixture.patched_uexp
        );

        let executable = fixture
            .game_root
            .join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe");
        let length = fs::metadata(&executable).unwrap().len() as usize;
        fs::write(&executable, vec![b'x'; length]).unwrap();
        stage.reverify_executable_path_identity().unwrap();
        assert!(stage.reverify_executable_anchor().is_err());
    }

    #[test]
    fn managed_stage_rejects_live_pair_and_final_generation_drift() {
        let (fixture, _stage) = verified_stage_fixture();
        let verified_patch = read_patch_receipt_v2(&fixture.patch_path).unwrap();
        let mut wrong_uasset = fixture.original_uasset.clone();
        wrong_uasset.push(0xff);
        let live_uexp = fixture.original_uexp.clone();
        let live_sidecar = fixture.sidecar.clone();
        let error = verify_fixed_leaf_stage_input_with_live_source(
            verified_patch,
            move |_root, _asset, expected| {
                Ok(LiveConvertedStageSource {
                    generation: expected.clone(),
                    uasset: wrong_uasset,
                    uexp: live_uexp,
                    sidecars: BTreeMap::from([(SidecarRole::Bulk, live_sidecar)]),
                })
            },
            |_root, _asset, expected| Ok(expected.clone()),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("fresh live conversion"), "{error}");

        let (fixture, _stage) = verified_stage_fixture();
        let verified_patch = read_patch_receipt_v2(&fixture.patch_path).unwrap();
        let live_uasset = fixture.original_uasset.clone();
        let live_uexp = fixture.original_uexp.clone();
        let live_sidecar = fixture.sidecar.clone();
        let error = verify_fixed_leaf_stage_input_with_live_source(
            verified_patch,
            move |_root, _asset, expected| {
                Ok(LiveConvertedStageSource {
                    generation: expected.clone(),
                    uasset: live_uasset,
                    uexp: live_uexp,
                    sidecars: BTreeMap::from([(SidecarRole::Bulk, live_sidecar)]),
                })
            },
            |_root, _asset, expected| {
                let mut drifted = expected.clone();
                drifted.target_chunks[0].length += 1;
                Ok(drifted)
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("changed during verification"), "{error}");
    }

    #[test]
    fn managed_stage_rejects_bytes_outside_the_semantic_edit() {
        let (fixture, _stage) = verified_stage_fixture();
        let mut patch: PatchReceiptEnvelope =
            serde_json::from_slice(&fs::read(&fixture.patch_path).unwrap()).unwrap();
        let uexp_path = PathBuf::from(&patch.output.uexp.path);
        let mut forged = fs::read(&uexp_path).unwrap();
        *forged.last_mut().unwrap() ^= 0x80;
        fs::write(&uexp_path, &forged).unwrap();
        let forged_pair = PackagePairSeal {
            uasset_sha256: patch.output_package_seal.uasset_sha256,
            uexp_sha256: sha256(&forged),
        };
        patch.output_package_seal = forged_pair.clone();
        patch.patch.after = forged_pair;
        patch.output.uexp.sha256 = encode_hex(&sha256(&forged));
        fs::write(&fixture.patch_path, serde_json::to_vec(&patch).unwrap()).unwrap();

        let verified_patch = read_patch_receipt_v2(&fixture.patch_path).unwrap();
        let live_uasset = fixture.original_uasset.clone();
        let live_uexp = fixture.original_uexp.clone();
        let live_sidecar = fixture.sidecar.clone();
        let error = verify_fixed_leaf_stage_input_with_live_source(
            verified_patch,
            move |_root, _asset, expected| {
                Ok(LiveConvertedStageSource {
                    generation: expected.clone(),
                    uasset: live_uasset,
                    uexp: live_uexp,
                    sidecars: BTreeMap::from([(SidecarRole::Bulk, live_sidecar)]),
                })
            },
            |_root, _asset, expected| Ok(expected.clone()),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("reproduced patch proof")
                || error.contains("outside the reproduced fixed-leaf edit"),
            "{error}"
        );
    }

    #[test]
    fn managed_offline_reviewed_stage_replays_exact_live_bytes_without_game_writes_or_pair_clones()
    {
        let fixture = managed_offline_fixture();
        let before = snapshot_tree(&fixture.game_root);
        let live = fixture.live_source();
        let live_uasset_ptr = live.uasset.as_ptr();
        let live_uexp_ptr = live.uexp.as_ptr();
        let live_usmap = fixture.live_usmap();
        let live_usmap_ptr = live_usmap.bytes.as_ptr();
        let verified = verify_managed_offline_dataasset_package_v1_with_live_source(
            &fixture.game_root,
            fixture.source(),
            move |_root, _target| Ok(live),
            move |_root| Ok(live_usmap),
            |_root, _target, expected| Ok(expected.clone()),
        )
        .unwrap();

        assert_eq!(
            verified.target_path(),
            ReviewedFootstepPresetTargetV1::Wolf.target_path()
        );
        assert_eq!(verified.generation(), &fixture.generation);
        assert_eq!(verified.reviewed(), &fixture.reviewed);
        assert_eq!(verified.uasset_bytes(), fixture.patched_uasset);
        assert_eq!(verified.uexp_bytes(), fixture.patched_uexp);
        assert_eq!(verified.usmap_bytes(), fixture.usmap);
        assert_eq!(
            verified.replay_seal(),
            &PackagePairSeal {
                uasset_sha256: sha256(&fixture.patched_uasset),
                uexp_sha256: sha256(&fixture.patched_uexp),
            }
        );
        assert_eq!(verified.uasset.as_ptr(), live_uasset_ptr);
        assert_eq!(verified.uexp.as_ptr(), live_uexp_ptr);
        assert_eq!(verified.usmap.as_ptr(), live_usmap_ptr);
        assert_eq!(snapshot_tree(&fixture.game_root), before);
        assert!(!format!("{verified:?}").contains(&fixture.game_root.display().to_string()));
    }

    #[test]
    fn managed_offline_reviewed_stage_rejects_wrong_target_selector_replacement_and_review() {
        let fixture = managed_offline_fixture();

        let mut source = fixture.source();
        source.target_path = ReviewedFootstepPresetTargetV1::Human.target_path();
        assert!(managed_offline_preflight_error(source).contains("ASSET_MANAGED_OFFLINE_INPUT"));

        let mut wrong_selector = fixture.selector.clone();
        wrong_selector.export_index += 1;
        let mut source = fixture.source();
        source.persisted_selector = &wrong_selector;
        assert!(managed_offline_preflight_error(source).contains("persisted selector"));

        let wrong_replacement = "00".repeat(32);
        let mut source = fixture.source();
        source.persisted_replacement_hex = &wrong_replacement;
        assert!(managed_offline_preflight_error(source).contains("persisted replacement"));

        let wrong_review = prepare_reviewed_footstep_preset_size_v1(
            ReviewedFootstepPresetTargetV1::Wolf.target_path(),
            &fixture.selector,
            ReviewedFootstepPresetSizeV1::try_new(13.0, 14.0).unwrap(),
        )
        .unwrap();
        let mut source = fixture.source();
        source.reviewed = &wrong_review;
        assert!(managed_offline_preflight_error(source).contains("reviewed intent"));
    }

    #[test]
    fn managed_offline_reviewed_stage_rejects_pair_usmap_sidecar_executable_and_generation_drift() {
        let fixture = managed_offline_fixture();
        let mut wrong_live = fixture.live_source();
        wrong_live.uexp[8] ^= 0x01;
        let error = verify_managed_offline_dataasset_package_v1_with_live_source(
            &fixture.game_root,
            fixture.source(),
            move |_root, _target| Ok(wrong_live),
            |_root| Ok(fixture.live_usmap()),
            |_root, _target, expected| Ok(expected.clone()),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("ASSET_MANAGED_OFFLINE_SOURCE_PAIR"),
            "{error}"
        );

        let fixture = managed_offline_fixture();
        let wrong_usmap_path = fixture.game_root.join("wrong.usmap");
        let mut wrong_usmap = fixture.usmap.clone();
        *wrong_usmap.last_mut().unwrap() ^= 0x01;
        fs::write(&wrong_usmap_path, wrong_usmap).unwrap();
        let error = verify_managed_offline_dataasset_package_v1_with_live_source(
            &fixture.game_root,
            fixture.source(),
            |_root, _target| Ok(fixture.live_source()),
            |_root| {
                read_verified_file_bounded(&wrong_usmap_path, MAX_USMAP_BYTES, "TEST_WRONG_USMAP")
            },
            |_root, _target, expected| Ok(expected.clone()),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("ASSET_MANAGED_OFFLINE_USMAP"), "{error}");

        let fixture = managed_offline_fixture();
        let mut wrong_live = fixture.live_source();
        wrong_live.sidecars.insert(SidecarRole::Bulk, vec![0x01]);
        let error = verify_managed_offline_dataasset_package_v1_with_live_source(
            &fixture.game_root,
            fixture.source(),
            move |_root, _target| Ok(wrong_live),
            |_root| Ok(fixture.live_usmap()),
            |_root, _target, expected| Ok(expected.clone()),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("ASSET_MANAGED_OFFLINE_SIDECAR"), "{error}");

        let fixture = managed_offline_fixture();
        let mut source = fixture.source();
        source.expected_executable_sha256 = [0xff; 32];
        let error = verify_managed_offline_dataasset_package_v1_with_live_source(
            &fixture.game_root,
            source,
            |_root, _target| panic!("executable mismatch must precede live conversion"),
            |_root| panic!("executable mismatch must precede live USMAP read"),
            |_root, _target, _expected| panic!("executable mismatch must precede final probe"),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("ASSET_MANAGED_OFFLINE_EXECUTABLE"),
            "{error}"
        );

        let fixture = managed_offline_fixture();
        let mut wrong_live = fixture.live_source();
        wrong_live.generation.target_chunks[0].length += 1;
        let error = verify_managed_offline_dataasset_package_v1_with_live_source(
            &fixture.game_root,
            fixture.source(),
            move |_root, _target| Ok(wrong_live),
            |_root| panic!("generation mismatch must precede live USMAP read"),
            |_root, _target, _expected| panic!("generation mismatch must precede final probe"),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("ASSET_MANAGED_OFFLINE_GENERATION"),
            "{error}"
        );
    }

    #[test]
    fn managed_offline_reviewed_stage_rejects_persisted_bytes_sidecars_and_final_drift() {
        let fixture = managed_offline_fixture();
        let mut wrong_patched_uexp = fixture.patched_uexp.clone();
        *wrong_patched_uexp.last_mut().unwrap() ^= 0x01;
        let mut source = fixture.source();
        source.patched_uexp = &wrong_patched_uexp;
        let error = verify_managed_offline_dataasset_package_v1_with_live_source(
            &fixture.game_root,
            source,
            |_root, _target| Ok(fixture.live_source()),
            |_root| Ok(fixture.live_usmap()),
            |_root, _target, expected| Ok(expected.clone()),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("ASSET_MANAGED_OFFLINE_PATCHED_PAIR"),
            "{error}"
        );

        let persisted_sidecar_bytes = [0x01];
        let persisted_sidecars = [(SidecarRole::Bulk, persisted_sidecar_bytes.as_slice())];
        let mut source = fixture.source();
        source.sidecars = &persisted_sidecars;
        assert!(managed_offline_preflight_error(source).contains("ASSET_MANAGED_OFFLINE_SIDECAR"));

        let fixture = managed_offline_fixture();
        let error = verify_managed_offline_dataasset_package_v1_with_live_source(
            &fixture.game_root,
            fixture.source(),
            |_root, _target| Ok(fixture.live_source()),
            |_root| Ok(fixture.live_usmap()),
            |_root, _target, expected| {
                let mut drifted = expected.clone();
                drifted.target_chunks[0].length += 1;
                Ok(drifted)
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("ASSET_MANAGED_OFFLINE_FINAL"), "{error}");

        let fixture = managed_offline_fixture();
        let executable_path = fixture.executable_path.clone();
        let executable_length = fs::metadata(&executable_path).unwrap().len() as usize;
        let error = verify_managed_offline_dataasset_package_v1_with_live_source(
            &fixture.game_root,
            fixture.source(),
            |_root, _target| Ok(fixture.live_source()),
            |_root| Ok(fixture.live_usmap()),
            move |_root, _target, expected| {
                fs::write(&executable_path, vec![b'x'; executable_length]).unwrap();
                Ok(expected.clone())
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("EXECUTABLE") || error.contains("hash differs"),
            "{error}"
        );
    }

    #[test]
    fn managed_offline_verified_package_is_not_cloneable_or_serializable() {
        trait AmbiguousIfClone<Marker> {
            fn marker() {}
        }
        impl<T: ?Sized> AmbiguousIfClone<()> for T {}
        impl<T: Clone> AmbiguousIfClone<u8> for T {}

        trait AmbiguousIfSerialize<Marker> {
            fn marker() {}
        }
        impl<T: ?Sized> AmbiguousIfSerialize<()> for T {}
        impl<T: serde::Serialize + ?Sized> AmbiguousIfSerialize<u8> for T {}

        let _ = <VerifiedManagedOfflineDataAssetPackageV1<'static> as AmbiguousIfClone<_>>::marker
            as fn();
        let _ =
            <VerifiedManagedOfflineDataAssetPackageV1<'static> as AmbiguousIfSerialize<_>>::marker
                as fn();
    }

    #[test]
    fn conversion_temp_parent_and_store_root_must_be_disjoint_from_game() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let sibling = root.path().join("private-temp");
        fs::create_dir(&game).unwrap();
        fs::create_dir(&sibling).unwrap();
        assert!(create_disjoint_private_conversion_dir_in(&game, root.path(), "TEST").is_err());
        assert!(create_disjoint_private_conversion_dir_in(&game, &game, "TEST").is_err());
        let owned = create_disjoint_private_conversion_dir_in(&game, &sibling, "TEST").unwrap();
        owned.close().unwrap();

        let (fixture, stage) = verified_stage_fixture();
        assert!(stage
            .require_store_root_disjoint(&fixture.game_root)
            .is_err());
        assert!(stage
            .require_store_root_disjoint(&fixture.output_root)
            .is_err());
        let outside = tempfile::tempdir().unwrap();
        stage.require_store_root_disjoint(outside.path()).unwrap();
    }

    #[test]
    fn empty_game_executable_is_not_an_anchor() {
        let root = tempfile::tempdir().unwrap();
        let executable = root
            .path()
            .join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(executable, []).unwrap();
        assert!(seal_game_executable(root.path()).is_err());
    }
}
