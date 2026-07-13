//! Closed, bounded wire receipts for the receipt-bound cooked-DataAsset workflow.
//!
//! This module is deliberately narrower than a generic package authoring API. It owns the
//! extract-v2 and patch-fixed-v2 wire models plus the validation that turns untrusted receipt
//! bytes into an exact, filesystem-bound projection. Raw deserialized structs are facts only;
//! callers must obtain [`ValidatedExtractBinding`] through the validators in this module before
//! using any receipt as provenance.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use retoc::{FIoContainerId, FPackageId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    FixedLeafSelector, FixedWireKind, PackageCarrier, PackageComponent, PackageLimits,
    PackagePairSeal,
};

pub const MAX_USMAP_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_SELECTOR_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_RECEIPT_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_OPTIONAL_SIDECAR_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_COOKED_PACKAGE_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_CONTAINER_COMPONENT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_MOUNT_UTOC_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_MOUNT_UCAS_BYTES: u64 = 128 * 1024 * 1024 * 1024;
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
        if anchor.file_name.is_empty()
            || anchor.file_name.contains('/')
            || anchor.file_name.contains('\\')
            || anchor.length == 0
            || !valid_hex(&anchor.sha256, 32)
        {
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
    let mut has_export = false;
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
            "ExportBundleData" => has_export = true,
            "BulkData" | "OptionalBulkData" | "MemoryMappedBulkData" => {}
            _ => bail!("{code}: unsupported generation chunk type"),
        }
    }
    if !has_export || !has_header {
        bail!("{code}: generation must contain export and ContainerHeader chunks");
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

fn validate_game_asset_path(asset: &str, code: &'static str) -> Result<()> {
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
    let mut file = File::open(&canonical)
        .with_context(|| format!("{code}: opening '{}'", canonical.display()))?;
    let advertised = file.metadata().context(code)?.len();
    if advertised > limit {
        bail!(
            "{code}: '{}' is {advertised} bytes; limit is {limit}",
            canonical.display()
        );
    }
    let (length, sha256) = digest_reader(&mut file, limit, code)?;
    if length != advertised {
        bail!(
            "{code}: input changed length while being read: {}",
            canonical.display()
        );
    }
    verify_file_hash(&canonical, length, sha256, limit, code)?;
    Ok(VerifiedFileSeal {
        path: canonical,
        length,
        sha256,
    })
}

fn digest_reader(reader: &mut File, limit: u64, code: &'static str) -> Result<(u64, [u8; 32])> {
    let mut hasher = Sha256::new();
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
    }
    Ok((length, hasher.finalize().into()))
}

fn read_verified_bounded(path: &Path, limit: u64, code: &'static str) -> Result<VerifiedInput> {
    let link_metadata = fs::symlink_metadata(path)
        .with_context(|| format!("{code}: inspecting '{}'", path.display()))?;
    if metadata_is_reparse(&link_metadata) || !link_metadata.is_file() {
        bail!(
            "{code}: input is not a regular non-symlink file: {}",
            path.display()
        );
    }
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("{code}: canonicalizing '{}'", path.display()))?;
    let mut file = File::open(&canonical)
        .with_context(|| format!("{code}: opening '{}'", canonical.display()))?;
    let advertised = file
        .metadata()
        .with_context(|| format!("{code}: reading metadata for '{}'", canonical.display()))?
        .len();
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
    (&mut file)
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .with_context(|| format!("{code}: reading '{}'", canonical.display()))?;
    if u64::try_from(bytes.len())? != advertised {
        bail!(
            "{code}: input changed length while being read: {}",
            canonical.display()
        );
    }
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
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("{code}: re-inspecting '{}'", path.display()))?;
    if metadata_is_reparse(&metadata) || !metadata.is_file() {
        bail!(
            "{code}: input changed to a non-regular file: {}",
            path.display()
        );
    }
    let mut file =
        File::open(path).with_context(|| format!("{code}: reopening '{}'", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut actual_length = 0u64;
    loop {
        let read = file
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
    use crate::{FixedLeafRole, FIXED_LEAF_SELECTOR_FORMAT, FIXED_LEAF_SELECTOR_PROFILE};

    struct ExtractFixture {
        _temp: tempfile::TempDir,
        receipt: ExtractReceiptEnvelope,
        uasset: PathBuf,
        copied_usmap: PathBuf,
        sidecar: PathBuf,
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
}
