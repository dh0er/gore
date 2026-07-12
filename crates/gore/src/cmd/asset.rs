//! `gore asset` -- schema-backed, copy-on-write fixed-leaf DataAsset tooling.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use gore_asset::{
    describe_fixed_leaves, FixedLeafDescriptor, FixedLeafPatch, FixedLeafSelector,
    FixedLeafSelectorStep, FixedWireKind, LegacyPackageEnvelope, PackageCarrier, PackageComponent,
    PackageLimits, PackagePairSeal, PropertySpanWalker, SchemaDb, FIXED_LEAF_SELECTOR_FORMAT,
    FIXED_LEAF_SELECTOR_PROFILE,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

const MAX_USMAP_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SELECTOR_BYTES: u64 = 4 * 1024 * 1024;
const MAX_RECEIPT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_OPTIONAL_SIDECAR_BYTES: u64 = 256 * 1024 * 1024;
const MAX_COOKED_PACKAGE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_CONTAINER_COMPONENT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_MOUNT_UTOC_BYTES: u64 = 256 * 1024 * 1024;
const MAX_MOUNT_UCAS_BYTES: u64 = 128 * 1024 * 1024 * 1024;
const EXTRACT_RECEIPT_NAME: &str = "gore-asset-extract.json";
const COPIED_USMAP_NAME: &str = "gore-generation.usmap";
const PATCH_RECEIPT_SUFFIX: &str = ".gore-asset-patch.json";
const PACK_RECEIPT_NAME: &str = "gore-asset-pack.json";
const EXTRACT_CONTENT_BINDING: &str = "each consumed decompressed chunk was verified against its winning container's TOC BLAKE3 hash and cached for all conversion reads";
const COMPOSITE_UCAS_ROLE: &str =
    "environment anchor only; consumed_chunks is the authoritative content binding";
const HELD_IDENTITY_VERIFICATION: &str = "identity_length_mtime_point_check";
const HELD_IDENTITY_LIMITATION: &str = "the large UCAS payload is not content-hashed; file identity, length, and modification stamp are held and point-rechecked before publication";
const MAX_GAME_ASSET_SEGMENTS: usize = 32;
// `pack` adds `cooked/G1R/Content` above the accepted virtual path. Keep cleanup
// comfortably above that maximum while still refusing adversarial deep trees.
const MAX_STAGING_TREE_DEPTH: usize = MAX_GAME_ASSET_SEGMENTS + 32;
const MAX_STAGING_TREE_ENTRIES: usize = MAX_GAME_ASSET_SEGMENTS + 32;
const MAX_PAKS_SCAN_DEPTH: usize = 16;
const MAX_PAKS_SCAN_ENTRIES: usize = 4096;
static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn asset_package_limits() -> PackageLimits {
    PackageLimits {
        max_uasset_bytes: 64 * 1024 * 1024,
        max_uexp_bytes: 256 * 1024 * 1024,
        max_total_bytes: 320 * 1024 * 1024,
    }
}

#[derive(Debug, Subcommand)]
pub enum AssetAction {
    /// Extract one current IoStore package into a sealed legacy package directory.
    Extract(ExtractArgs),
    /// List structurally editable fixed-width leaves in a legacy split package.
    Inspect(InspectArgs),
    /// Apply one snapshot-bound raw wire edit to a new package pair.
    PatchFixed(PatchFixedArgs),
    /// Pack one legacy package as an additive, undeployed Zen triplet.
    Pack(PackArgs),
}

#[derive(Debug, Args)]
pub struct ExtractArgs {
    /// Gothic 1 Remake install root containing `G1R/`.
    #[arg(long, value_name = "GAME")]
    pub game: PathBuf,
    /// Exact cooked package path, beginning with `/Game/` and without an extension.
    #[arg(long, value_name = "/Game/...")]
    pub asset: String,
    /// New output directory; it must not exist and is never placed in the game tree.
    #[arg(short = 'o', long, value_name = "DIR")]
    pub out: PathBuf,
    /// Emit the same machine-readable receipt written into the output directory.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct InspectArgs {
    /// Input legacy `.uasset`; the sibling `.uexp` is required.
    #[arg(long, value_name = "INPUT.uasset")]
    pub uasset: PathBuf,
    /// Exact `.usmap` used to decode this package generation.
    #[arg(long, value_name = "MAPPINGS.usmap")]
    pub usmap: PathBuf,
    /// Inspect only this export; unsupported/missing selected exports are fatal.
    #[arg(long)]
    pub export_index: Option<usize>,
    /// Emit one machine-readable JSON document.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct PatchFixedArgs {
    /// Input legacy `.uasset`; it is never modified.
    #[arg(long, value_name = "INPUT.uasset")]
    pub uasset: PathBuf,
    /// Exact `.usmap` named by the selector.
    #[arg(long, value_name = "MAPPINGS.usmap")]
    pub usmap: PathBuf,
    /// Exact extract receipt that seals this package pair and USMAP generation.
    #[arg(long, value_name = "gore-asset-extract.json")]
    pub extract_receipt: PathBuf,
    /// JSON containing a selector, descriptor, or one inspect leaf object.
    #[arg(long, value_name = "SELECTOR.json")]
    pub selector: PathBuf,
    /// Exact current raw little-endian wire bytes; must agree with the selector.
    #[arg(long, value_name = "HEX")]
    pub expected_hex: String,
    /// Exact replacement wire bytes; no gameplay/domain validation is implied.
    #[arg(long, value_name = "HEX")]
    pub replacement_hex: String,
    /// New `.uasset` output; its sibling `.uexp` is created without clobbering.
    #[arg(short = 'o', long, value_name = "OUTPUT.uasset")]
    pub out: PathBuf,
    /// Emit one machine-readable JSON document.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct PackArgs {
    /// Gothic 1 Remake install root used only for the global script-object store.
    #[arg(long, value_name = "GAME")]
    pub game: PathBuf,
    /// Input legacy `.uasset`; its `.uexp` and optional same-stem sidecars are read.
    #[arg(long, value_name = "PATCHED.uasset")]
    pub uasset: PathBuf,
    /// Exact patch receipt produced beside `--uasset` by `patch-fixed`.
    #[arg(long, value_name = "*.gore-asset-patch.json")]
    pub patch_receipt: PathBuf,
    /// Exact target package path, beginning with `/Game/` and without an extension.
    #[arg(long, value_name = "/Game/...")]
    pub asset: String,
    /// Safe filename stem for `<NAME>.{utoc,ucas,pak}`.
    #[arg(long, value_name = "MOD")]
    pub name: String,
    /// New output directory; it must not exist and is never placed in the game tree.
    #[arg(short = 'o', long, value_name = "DIR")]
    pub out: PathBuf,
    /// Emit the same machine-readable receipt written into the output directory.
    #[arg(long)]
    pub json: bool,
}

pub fn run(action: AssetAction) -> Result<()> {
    match action {
        AssetAction::Extract(args) => extract(args),
        AssetAction::Inspect(args) => inspect(args),
        AssetAction::PatchFixed(args) => patch_fixed(args),
        AssetAction::Pack(args) => pack(args),
    }
}

#[derive(Debug, Clone)]
struct FileSeal {
    path: PathBuf,
    length: u64,
    sha256: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct GenerationFileAnchor {
    file_name: String,
    length: u64,
    sha256: String,
}

impl GenerationFileAnchor {
    fn from_seal(seal: &FileSeal) -> Result<Self> {
        Ok(Self {
            file_name: seal
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .context("generation anchor has a non-UTF-8 filename")?
                .to_owned(),
            length: seal.length,
            sha256: encode_hex(&seal.sha256),
        })
    }

    fn matches(&self, seal: &FileSeal) -> bool {
        self.length == seal.length && self.sha256 == encode_hex(&seal.sha256)
    }

    fn same_content(&self, other: &Self) -> bool {
        self.length == other.length && self.sha256 == other.sha256
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct GenerationChunkAnchor {
    chunk_id: String,
    chunk_type: String,
    winner_utoc: GenerationFileAnchor,
    length: u64,
    blake3: String,
    toc_hash: String,
    toc_hash_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct AssetGenerationReceipt {
    format: String,
    asset: String,
    usmap: GenerationFileAnchor,
    main_utoc: GenerationFileAnchor,
    global_utoc: GenerationFileAnchor,
    global_ucas: GenerationFileAnchor,
    container_set: Vec<GenerationFileAnchor>,
    target_chunks: Vec<GenerationChunkAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ReceiptComponent {
    relative_path: String,
    length: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SourceFileReceipt {
    path: String,
    length: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct HeldIdentityReceipt {
    path: String,
    length: u64,
    modified_stamp: String,
    platform_identity: String,
    sha256: Option<String>,
    verification: String,
    content_hash_omitted: bool,
    limitation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExtractCompositeStoreAnchor {
    utoc: SourceFileReceipt,
    ucas: HeldIdentityReceipt,
    role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ReceiptVerifiedChunk {
    chunk_id: String,
    chunk_type: String,
    source_utoc: PathBuf,
    length: u64,
    blake3: String,
    toc_hash: String,
    toc_hash_bytes: usize,
}

impl From<&gore_tex::container::VerifiedChunkReceipt> for ReceiptVerifiedChunk {
    fn from(value: &gore_tex::container::VerifiedChunkReceipt) -> Self {
        Self {
            chunk_id: value.chunk_id.clone(),
            chunk_type: value.chunk_type.clone(),
            source_utoc: value.source_utoc.clone(),
            length: value.length,
            blake3: value.blake3.clone(),
            toc_hash: value.toc_hash.clone(),
            toc_hash_bytes: value.toc_hash_bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExtractUsmapProof {
    source: SourceFileReceipt,
    copied_relative_path: String,
    copy: ReceiptComponent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct GlobalScriptStoreProof {
    utoc: SourceFileReceipt,
    ucas: SourceFileReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExtractReceiptSource {
    game_root: String,
    composite_store_anchor: ExtractCompositeStoreAnchor,
    consumed_chunks: Vec<ReceiptVerifiedChunk>,
    source_container_tocs: Vec<SourceFileReceipt>,
    content_binding: String,
    usmap: ExtractUsmapProof,
    global_script_store: GlobalScriptStoreProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractReceiptOutput {
    root: String,
    receipt: String,
    components: Vec<ReceiptComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractReceiptEnvelope {
    format: String,
    status: String,
    asset: String,
    generation: AssetGenerationReceipt,
    source: ExtractReceiptSource,
    package_seal: PackagePairSeal,
    output: ExtractReceiptOutput,
    deployed: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
enum SidecarRole {
    #[serde(rename = "BulkData")]
    Bulk,
    #[serde(rename = "OptionalBulkData")]
    Optional,
    #[serde(rename = "MemoryMappedBulkData")]
    MemoryMapped,
}

impl SidecarRole {
    const ALL: [Self; 3] = [Self::Bulk, Self::Optional, Self::MemoryMapped];

    fn suffix(self) -> &'static str {
        match self {
            Self::Bulk => "ubulk",
            Self::Optional => "uptnl",
            Self::MemoryMapped => "m.ubulk",
        }
    }

    fn chunk_type(self) -> &'static str {
        match self {
            Self::Bulk => "BulkData",
            Self::Optional => "OptionalBulkData",
            Self::MemoryMapped => "MemoryMappedBulkData",
        }
    }

    fn from_chunk_type(value: &str) -> Option<Self> {
        match value {
            "BulkData" => Some(Self::Bulk),
            "OptionalBulkData" => Some(Self::Optional),
            "MemoryMappedBulkData" => Some(Self::MemoryMapped),
            _ => None,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Bulk => 0,
            Self::Optional => 1,
            Self::MemoryMapped => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SidecarReceipt {
    role: SidecarRole,
    file_name: String,
    length: u64,
    sha256: String,
}

#[derive(Debug, Clone)]
struct ValidatedExtractBinding {
    output_root: PathBuf,
    uasset: ReceiptComponent,
    uexp: ReceiptComponent,
    copied_usmap: ReceiptComponent,
    components: Vec<ReceiptComponent>,
    sidecars: Vec<SidecarReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchReceiptProvenance {
    extract_receipt: ReceiptFileSeal,
    generation: AssetGenerationReceipt,
    usmap: GenerationFileAnchor,
    extract_components: Vec<ReceiptComponent>,
    extracted_sidecars: Vec<SidecarReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PatchOperationProof {
    before: PackagePairSeal,
    after: PackagePairSeal,
    export_index: usize,
    component: PackageComponent,
    absolute_offset: usize,
    length: usize,
    kind: FixedWireKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ComponentDigestProof {
    path: String,
    length: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PatchReceiptOutput {
    uasset: ComponentDigestProof,
    uexp: ComponentDigestProof,
    sidecars: Vec<SidecarReceipt>,
    receipt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchReceiptEnvelope {
    format: String,
    status: String,
    asset: String,
    generation_bound: bool,
    provenance: PatchReceiptProvenance,
    input_package_seal: PackagePairSeal,
    output_package_seal: PackagePairSeal,
    output_sidecars: Vec<SidecarReceipt>,
    input_selector: FixedLeafSelector,
    output_requires_reinspect: bool,
    expected_hex: String,
    replacement_hex: String,
    patch: PatchOperationProof,
    output: PatchReceiptOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptFileSeal {
    path: String,
    length: u64,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentitySnapshot {
    length: u64,
    modified_stamp: String,
    platform_identity: String,
}

#[derive(Debug)]
struct HeldFileIdentity {
    path: PathBuf,
    file: File,
    snapshot: FileIdentitySnapshot,
    expect_directory: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DirectMountKind {
    Utoc,
    Ucas,
    Pak,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DirectMountAnchor {
    Hashed {
        file_name: String,
        kind: DirectMountKind,
        length: u64,
        sha256: [u8; 32],
    },
    Identity {
        file_name: String,
        kind: DirectMountKind,
        snapshot: FileIdentitySnapshot,
    },
}

#[derive(Debug)]
struct GameMountInventory {
    game_root: PathBuf,
    anchors: Vec<DirectMountAnchor>,
    hashed_files: Vec<(FileSeal, u64)>,
    held_ucas: Vec<HeldFileIdentity>,
}

#[derive(Debug)]
struct AssetMount {
    leaf: String,
    cooked_uasset: PathBuf,
}

#[derive(Debug)]
struct StagingDirectory {
    path: PathBuf,
    identity: HeldFileIdentity,
    armed: bool,
}

#[derive(Debug)]
struct OwnedPublishedPath {
    path: PathBuf,
    armed: bool,
}

impl OwnedPublishedPath {
    fn armed(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    fn cleanup(&mut self) {
        if !self.armed {
            return;
        }
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.armed = false;
            }
            Ok(metadata)
                if metadata.is_file()
                    && !metadata_is_reparse(&metadata)
                    && fs::remove_file(&self.path).is_ok() =>
            {
                self.armed = false;
            }
            _ => {}
        }
    }
}

impl Drop for OwnedPublishedPath {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[derive(Debug)]
struct PublishedCopy {
    source_seal: FileSeal,
    target_seal: FileSeal,
    ownership: OwnedPublishedPath,
}

#[derive(Debug)]
struct PatchOutputGuard {
    files: Vec<OwnedPublishedPath>,
}

impl PatchOutputGuard {
    fn new() -> Self {
        Self { files: Vec::new() }
    }

    fn own(&mut self, path: PathBuf) {
        self.files.push(OwnedPublishedPath::armed(path));
    }

    fn adopt(&mut self, ownership: OwnedPublishedPath) {
        self.files.push(ownership);
    }

    fn disarm(&mut self) {
        for file in &mut self.files {
            file.disarm();
        }
    }
}

impl Drop for PatchOutputGuard {
    fn drop(&mut self) {
        for file in self.files.iter_mut().rev() {
            file.cleanup();
        }
    }
}

impl StagingDirectory {
    fn verify_owned(&self, code: &'static str) -> Result<()> {
        self.identity.reverify(code)
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    fn cleanup(&mut self, code: &'static str) -> Result<()> {
        self.verify_owned(code)?;
        validate_tree_no_reparse(&self.path, MAX_STAGING_TREE_DEPTH, 4096, code)?;
        fs::remove_dir_all(&self.path)
            .with_context(|| format!("{code}: removing owned staging directory"))?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if self.armed
            && self.verify_owned("ASSET_STAGE_CLEANUP").is_ok()
            && validate_tree_no_reparse(
                &self.path,
                MAX_STAGING_TREE_DEPTH,
                4096,
                "ASSET_STAGE_CLEANUP",
            )
            .is_ok()
        {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn extract(args: ExtractArgs) -> Result<()> {
    let mount = validate_game_asset_path(&args.asset, "ASSET_EXTRACT_ASSET")?;
    let game = resolve_game_root(&args.game, "ASSET_EXTRACT_GAME")?;
    let out = prepare_absent_output_directory(&args.out, &game, "ASSET_EXTRACT_OUTPUT")?;
    let mount_inventory = capture_game_mount_inventory(&game, "ASSET_EXTRACT_GAME")?;

    let utoc = gore_tex::paths::main_container(&game).context("ASSET_EXTRACT_CONTAINER")?;
    let usmap = gore_tex::paths::usmap(&game).context("ASSET_EXTRACT_USMAP")?;
    let (global_utoc_seal, global_ucas_seal) =
        seal_global_script_store(&game, "ASSET_EXTRACT_GAME")?;
    let utoc_seal = digest_regular_file_bounded(
        &utoc,
        MAX_CONTAINER_COMPONENT_BYTES,
        "ASSET_EXTRACT_CONTAINER",
    )?;
    let main_ucas_identity = HeldFileIdentity::open(
        &utoc.with_extension("ucas"),
        64 * 1024 * 1024 * 1024,
        "ASSET_EXTRACT_CONTAINER",
    )?;
    let usmap_seal = digest_regular_file_bounded(&usmap, MAX_USMAP_BYTES, "ASSET_EXTRACT_USMAP")?;

    let parent = out
        .parent()
        .context("ASSET_EXTRACT_OUTPUT: output has no parent")?;
    let mut staging = create_staging_directory(parent, "extract", "ASSET_EXTRACT_OUTPUT")?;
    staging.verify_owned("ASSET_EXTRACT_OUTPUT")?;
    let copied_usmap = staging.path.join(COPIED_USMAP_NAME);
    let copied_usmap_file = copy_optional_verified_file(
        &usmap_seal.path,
        &copied_usmap,
        MAX_USMAP_BYTES,
        "ASSET_EXTRACT_USMAP",
    )?
    .context("ASSET_EXTRACT_USMAP: sealed USMAP disappeared while copying")?;
    let PublishedCopy {
        target_seal: copied_usmap_seal,
        mut ownership,
        ..
    } = copied_usmap_file;
    ownership.disarm();
    if copied_usmap_seal.length != usmap_seal.length
        || copied_usmap_seal.sha256 != usmap_seal.sha256
    {
        bail!("ASSET_EXTRACT_USMAP: copied USMAP does not match sealed source");
    }
    let unpacked = gore_tex::container::unpack_asset_verified(
        &utoc_seal.path,
        &usmap_seal.path,
        &args.asset,
        &staging.path,
    )
    .context("ASSET_EXTRACT_CONVERT")?;
    staging.verify_owned("ASSET_EXTRACT_OUTPUT")?;
    validate_tree_no_reparse(&staging.path, 4, 8, "ASSET_EXTRACT_OUTPUT")?;
    let mut consumed_utoc_paths = std::collections::BTreeSet::new();
    for chunk in &unpacked.consumed_chunks {
        consumed_utoc_paths.insert(chunk.source_utoc.clone());
    }
    consumed_utoc_paths.extend(unpacked.metadata_utocs.iter().cloned());
    let mut consumed_utoc_seals = Vec::with_capacity(consumed_utoc_paths.len());
    for source_utoc in consumed_utoc_paths {
        consumed_utoc_seals.push(digest_regular_file_bounded(
            &source_utoc,
            MAX_CONTAINER_COMPONENT_BYTES,
            "ASSET_EXTRACT_CONTAINER",
        )?);
    }
    let consumed_utoc_receipts: Vec<_> = consumed_utoc_seals
        .iter()
        .map(source_file_receipt)
        .collect();
    let extracted_uasset = unpacked.uasset;

    let expected_uasset = staging.path.join(format!("{}.uasset", mount.leaf));
    if extracted_uasset != expected_uasset {
        bail!(
            "ASSET_EXTRACT_CONVERT: converter returned unexpected path '{}'; expected '{}'",
            extracted_uasset.display(),
            expected_uasset.display()
        );
    }
    let package_limits = asset_package_limits();
    let carrier =
        PackageCarrier::load(&expected_uasset, package_limits).context("ASSET_EXTRACT_PAIR")?;
    sync_existing_regular_file(&expected_uasset, "ASSET_EXTRACT_PAIR")?;
    sync_existing_regular_file(
        &expected_uasset.with_extension("uexp"),
        "ASSET_EXTRACT_PAIR",
    )?;
    let package_seal = PackagePairSeal::capture(&carrier);
    let uasset_output_seal = digest_regular_file_bounded(
        &expected_uasset,
        package_limits.max_uasset_bytes,
        "ASSET_EXTRACT_PAIR",
    )?;
    let uexp_output_seal = digest_regular_file_bounded(
        &expected_uasset.with_extension("uexp"),
        package_limits.max_uexp_bytes,
        "ASSET_EXTRACT_PAIR",
    )?;
    if uasset_output_seal.sha256 != package_seal.uasset_sha256
        || uexp_output_seal.sha256 != package_seal.uexp_sha256
    {
        bail!("ASSET_EXTRACT_PAIR: output pair changed after carrier verification");
    }
    let mut output_component_seals = vec![
        uasset_output_seal.clone(),
        uexp_output_seal.clone(),
        copied_usmap_seal.clone(),
    ];
    let mut cooked_total = uasset_output_seal
        .length
        .checked_add(uexp_output_seal.length)
        .context("ASSET_EXTRACT_PAIR: cooked size overflowed")?;
    let mut components = vec![
        component_receipt(
            &format!("{}.uasset", mount.leaf),
            uasset_output_seal.length,
            &uasset_output_seal.sha256,
        ),
        component_receipt(
            &format!("{}.uexp", mount.leaf),
            uexp_output_seal.length,
            &uexp_output_seal.sha256,
        ),
        component_receipt(
            COPIED_USMAP_NAME,
            copied_usmap_seal.length,
            &copied_usmap_seal.sha256,
        ),
    ];
    let mut extracted_sidecars = Vec::new();
    for role in SidecarRole::ALL {
        let (relative_name, path) = sidecar_path(&expected_uasset, role, "ASSET_EXTRACT_SIDECAR")?;
        if path.exists() {
            sync_existing_regular_file(&path, "ASSET_EXTRACT_SIDECAR")?;
        }
        if let Some(seal) = digest_optional_regular_file_bounded(
            &path,
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_EXTRACT_SIDECAR",
        )? {
            cooked_total = cooked_total
                .checked_add(seal.length)
                .context("ASSET_EXTRACT_SIDECAR: cooked size overflowed")?;
            if cooked_total > MAX_COOKED_PACKAGE_BYTES {
                bail!(
                    "ASSET_EXTRACT_SIDECAR: cooked package is {cooked_total} bytes; aggregate limit is {MAX_COOKED_PACKAGE_BYTES}"
                );
            }
            components.push(component_receipt(&relative_name, seal.length, &seal.sha256));
            extracted_sidecars.push(SidecarReceipt {
                role,
                file_name: relative_name,
                length: seal.length,
                sha256: encode_hex(&seal.sha256),
            });
            output_component_seals.push(seal);
        }
    }
    validate_flat_extraction_directory(&staging.path, &components)?;

    let generation = build_generation_receipt(
        &args.asset,
        &usmap_seal,
        &utoc_seal,
        &global_utoc_seal,
        &global_ucas_seal,
        &unpacked.consumed_chunks,
        &consumed_utoc_seals,
    )?;
    validate_sidecar_generation_mapping(&extracted_sidecars, &generation, "ASSET_EXTRACT_SIDECAR")?;

    let receipt = ExtractReceiptEnvelope {
        format: "gore.asset.extract.v2".to_owned(),
        status: "extracted".to_owned(),
        asset: args.asset.clone(),
        generation: generation.clone(),
        source: ExtractReceiptSource {
            game_root: game.display().to_string(),
            composite_store_anchor: ExtractCompositeStoreAnchor {
                utoc: source_file_receipt(&utoc_seal),
                ucas: main_ucas_identity.receipt(),
                role: COMPOSITE_UCAS_ROLE.to_owned(),
            },
            consumed_chunks: unpacked
                .consumed_chunks
                .iter()
                .map(ReceiptVerifiedChunk::from)
                .collect(),
            source_container_tocs: consumed_utoc_receipts,
            content_binding: EXTRACT_CONTENT_BINDING.to_owned(),
            usmap: ExtractUsmapProof {
                source: source_file_receipt(&usmap_seal),
                copied_relative_path: COPIED_USMAP_NAME.to_owned(),
                copy: component_receipt(
                    COPIED_USMAP_NAME,
                    copied_usmap_seal.length,
                    &copied_usmap_seal.sha256,
                ),
            },
            global_script_store: GlobalScriptStoreProof {
                utoc: source_file_receipt(&global_utoc_seal),
                ucas: source_file_receipt(&global_ucas_seal),
            },
        },
        package_seal,
        output: ExtractReceiptOutput {
            root: out.display().to_string(),
            receipt: EXTRACT_RECEIPT_NAME.to_owned(),
            components,
        },
        deployed: false,
    };
    validate_extract_receipt_envelope(&receipt, "ASSET_EXTRACT_RECEIPT")?;
    let printable = serde_json::to_string_pretty(&receipt)?;
    write_new_synced(
        &staging.path.join(EXTRACT_RECEIPT_NAME),
        printable.as_bytes(),
        "ASSET_EXTRACT_RECEIPT",
    )?;
    reverify_file_seal(
        &utoc_seal,
        MAX_CONTAINER_COMPONENT_BYTES,
        "ASSET_EXTRACT_CONTAINER",
    )?;
    main_ucas_identity.reverify("ASSET_EXTRACT_CONTAINER")?;
    reverify_file_seal(&usmap_seal, MAX_USMAP_BYTES, "ASSET_EXTRACT_USMAP")?;
    reverify_file_seal(
        &global_utoc_seal,
        MAX_CONTAINER_COMPONENT_BYTES,
        "ASSET_EXTRACT_GAME",
    )?;
    reverify_file_seal(
        &global_ucas_seal,
        MAX_CONTAINER_COMPONENT_BYTES,
        "ASSET_EXTRACT_GAME",
    )?;
    for seal in &consumed_utoc_seals {
        reverify_file_seal(
            seal,
            MAX_CONTAINER_COMPONENT_BYTES,
            "ASSET_EXTRACT_CONTAINER",
        )?;
    }
    for seal in &output_component_seals {
        let limit = if seal
            .path
            .extension()
            .is_some_and(|extension| extension == "uasset")
        {
            package_limits.max_uasset_bytes
        } else if seal
            .path
            .extension()
            .is_some_and(|extension| extension == "uexp")
        {
            package_limits.max_uexp_bytes
        } else if seal
            .path
            .extension()
            .is_some_and(|extension| extension == "usmap")
        {
            MAX_USMAP_BYTES
        } else {
            MAX_OPTIONAL_SIDECAR_BYTES
        };
        reverify_file_seal(seal, limit, "ASSET_EXTRACT_OUTPUT")?;
    }
    staging.verify_owned("ASSET_EXTRACT_OUTPUT")?;
    validate_tree_no_reparse(&staging.path, 4, 8, "ASSET_EXTRACT_OUTPUT")?;
    run_final_publish_gate(
        &mount_inventory,
        "ASSET_EXTRACT_GAME",
        || {
            probe_current_generation_receipt(
                &game,
                &args.asset,
                &generation,
                "ASSET_EXTRACT_GENERATION",
            )
        },
        |current| {
            if current != &generation {
                let reason = generation_mismatch_reason(&generation, current);
                bail!(
                    "ASSET_GENERATION_MISMATCH: target generation changed before extract publication ({reason})"
                );
            }
            Ok(())
        },
    )?;
    publish_staged_directory(&staging.path, &out, "ASSET_EXTRACT_OUTPUT")?;
    staging.disarm();

    if args.json {
        println!("{printable}");
    } else {
        println!(
            "EXTRACTED\tasset={}\toutput={}\treceipt={}",
            serde_json::to_string(&receipt.asset)?,
            serde_json::to_string(&out.display().to_string())?,
            serde_json::to_string(&out.join(EXTRACT_RECEIPT_NAME).display().to_string())?,
        );
    }
    Ok(())
}

fn pack(args: PackArgs) -> Result<()> {
    let mount = validate_game_asset_path(&args.asset, "ASSET_PACK_ASSET")?;
    validate_triplet_name(&args.name)?;
    let game = resolve_game_root(&args.game, "ASSET_PACK_GAME")?;
    let out = prepare_absent_output_directory(&args.out, &game, "ASSET_PACK_OUTPUT")?;
    let mount_inventory = capture_game_mount_inventory(&game, "ASSET_PACK_GAME")?;
    let (global_utoc_seal, global_ucas_seal) = seal_global_script_store(&game, "ASSET_PACK_GAME")?;
    let patch_receipt_input = read_verified_bounded(
        &args.patch_receipt,
        MAX_RECEIPT_BYTES,
        "ASSET_PATCH_RECEIPT",
    )?;
    let patch_receipt: PatchReceiptEnvelope = serde_json::from_slice(&patch_receipt_input.bytes)
        .context("ASSET_PATCH_RECEIPT: invalid receipt JSON")?;
    validate_patch_receipt_envelope(&patch_receipt, &patch_receipt_input.path)?;
    if patch_receipt.asset != args.asset || patch_receipt.provenance.generation.asset != args.asset
    {
        bail!("ASSET_GENERATION_MISMATCH: patch receipt targets a different asset");
    }
    let (extract_receipt, extract_binding) =
        read_chained_extract_receipt(&patch_receipt.provenance.extract_receipt)?;
    if extract_receipt.asset != patch_receipt.asset
        || extract_receipt.generation != patch_receipt.provenance.generation
        || extract_receipt.package_seal != patch_receipt.input_package_seal
        || extract_binding.components != patch_receipt.provenance.extract_components
        || extract_binding.sidecars != patch_receipt.provenance.extracted_sidecars
    {
        bail!(
            "ASSET_PATCH_RECEIPT: chained extract asset, generation, package, or component provenance mismatch"
        );
    }
    if patch_receipt.provenance.usmap.file_name != extract_binding.copied_usmap.relative_path
        || patch_receipt.provenance.usmap.length != extract_binding.copied_usmap.length
        || patch_receipt.provenance.usmap.sha256 != extract_binding.copied_usmap.sha256
    {
        bail!("ASSET_PATCH_RECEIPT: copied USMAP provenance mismatch");
    }
    if patch_receipt.output_sidecars.len() != extract_binding.sidecars.len()
        || patch_receipt
            .output_sidecars
            .iter()
            .zip(&extract_binding.sidecars)
            .any(|(output, extracted)| {
                output.role != extracted.role
                    || output.length != extracted.length
                    || output.sha256 != extracted.sha256
            })
    {
        bail!("ASSET_PATCH_RECEIPT: patched sidecars differ from extracted provenance");
    }
    validate_sidecar_generation_mapping(
        &patch_receipt.output_sidecars,
        &patch_receipt.provenance.generation,
        "ASSET_PATCH_RECEIPT",
    )?;

    let input_uasset = validate_existing_path_no_reparse(&args.uasset, false, "ASSET_PACK_INPUT")?;
    input_uasset
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .context("ASSET_PACK_INPUT: input requires a non-empty UTF-8 file stem")?;
    let package_limits = asset_package_limits();
    let carrier =
        PackageCarrier::load(&input_uasset, package_limits).context("ASSET_PACK_INPUT")?;
    let source_paths = carrier
        .source_paths()
        .context("ASSET_PACK_INPUT: loaded package has no source paths")?;
    let source_uasset = source_paths.uasset().to_path_buf();
    let source_uexp = source_paths.uexp().to_path_buf();
    let source_seal = PackagePairSeal::capture(&carrier);
    if source_seal != patch_receipt.output_package_seal {
        bail!("ASSET_GENERATION_MISMATCH: input package pair differs from patch receipt");
    }
    validate_patch_output_against_carrier(&patch_receipt, &source_uasset, &source_uexp, &carrier)?;
    let pair_bytes = u64::try_from(carrier.len(PackageComponent::Uasset))?
        .checked_add(u64::try_from(carrier.len(PackageComponent::Uexp))?)
        .context("ASSET_PACK_INPUT: cooked size overflowed")?;
    let validated_sidecar_seals =
        validate_patched_sidecars(&source_uasset, &patch_receipt.output_sidecars, pair_bytes)?;

    // Resolve and hash the live target generation before any output staging is
    // created. This catches hotfixes and newly winning sibling containers even
    // when the edited legacy pair itself still parses.
    let main_utoc = gore_tex::paths::main_container(&game).context("ASSET_PACK_CONTAINER")?;
    let current_usmap = gore_tex::paths::usmap(&game).context("ASSET_PACK_USMAP")?;
    let main_utoc_seal = digest_regular_file_bounded(
        &main_utoc,
        MAX_CONTAINER_COMPONENT_BYTES,
        "ASSET_PACK_CONTAINER",
    )?;
    let main_ucas_identity = HeldFileIdentity::open(
        &main_utoc.with_extension("ucas"),
        64 * 1024 * 1024 * 1024,
        "ASSET_PACK_CONTAINER",
    )?;
    let current_usmap_seal =
        digest_regular_file_bounded(&current_usmap, MAX_USMAP_BYTES, "ASSET_PACK_USMAP")?;
    let required_generation_chunks: Vec<_> = patch_receipt
        .provenance
        .generation
        .target_chunks
        .iter()
        .map(|chunk| chunk.chunk_id.clone())
        .collect();
    let generation_probe = gore_tex::container::probe_asset_generation_for_chunks_verified(
        &main_utoc_seal.path,
        &args.asset,
        &required_generation_chunks,
    )
    .context("ASSET_PACK_GENERATION")?;
    let mut generation_utoc_seals = Vec::with_capacity(generation_probe.metadata_utocs.len());
    for source_utoc in &generation_probe.metadata_utocs {
        generation_utoc_seals.push(digest_regular_file_bounded(
            source_utoc,
            MAX_CONTAINER_COMPONENT_BYTES,
            "ASSET_PACK_GENERATION",
        )?);
    }
    let current_generation = build_generation_receipt(
        &args.asset,
        &current_usmap_seal,
        &main_utoc_seal,
        &global_utoc_seal,
        &global_ucas_seal,
        &generation_probe.consumed_chunks,
        &generation_utoc_seals,
    )?;
    if current_generation != patch_receipt.provenance.generation {
        let reason =
            generation_mismatch_reason(&patch_receipt.provenance.generation, &current_generation);
        bail!(
            "ASSET_GENERATION_MISMATCH: installed target/USMAP/UTOC/global generation changed since extract ({reason}); re-extract and reapply the patch"
        );
    }

    let parent = out
        .parent()
        .context("ASSET_PACK_OUTPUT: output has no parent")?;
    let mut staging = create_staging_directory(parent, "pack", "ASSET_PACK_OUTPUT")?;
    staging.verify_owned("ASSET_PACK_OUTPUT")?;
    let cooked_root = staging.path.join("cooked");
    let build_root = staging.path.join("triplet");
    fs::create_dir(&cooked_root).context("ASSET_PACK_STAGE")?;
    fs::create_dir(&build_root).context("ASSET_PACK_STAGE")?;
    let staged_uasset = cooked_root.join(&mount.cooked_uasset);
    let staged_relative_parent = mount
        .cooked_uasset
        .parent()
        .context("ASSET_PACK_ASSET: cooked asset has no parent")?;
    create_relative_directory_chain(&cooked_root, staged_relative_parent, "ASSET_PACK_STAGE")?;
    write_new_synced(
        &staged_uasset,
        carrier.bytes(PackageComponent::Uasset),
        "ASSET_PACK_STAGE",
    )?;
    write_new_synced(
        &staged_uasset.with_extension("uexp"),
        carrier.bytes(PackageComponent::Uexp),
        "ASSET_PACK_STAGE",
    )?;
    let mut staged_input_seals = vec![
        digest_regular_file_bounded(
            &staged_uasset,
            package_limits.max_uasset_bytes,
            "ASSET_PACK_STAGE",
        )?,
        digest_regular_file_bounded(
            &staged_uasset.with_extension("uexp"),
            package_limits.max_uexp_bytes,
            "ASSET_PACK_STAGE",
        )?,
    ];

    let mut input_components = vec![
        packed_input_receipt(
            &source_uasset,
            &mount.cooked_uasset,
            u64::try_from(carrier.len(PackageComponent::Uasset))?,
            &source_seal.uasset_sha256,
        ),
        packed_input_receipt(
            &source_uexp,
            &mount.cooked_uasset.with_extension("uexp"),
            u64::try_from(carrier.len(PackageComponent::Uexp))?,
            &source_seal.uexp_sha256,
        ),
    ];
    let mut cooked_total = pair_bytes;
    let sidecar_source_seals = validated_sidecar_seals;
    let mut expected_sidecars = [false; 3];
    for (expected, validated) in patch_receipt
        .output_sidecars
        .iter()
        .zip(&sidecar_source_seals)
    {
        let (_, source) = sidecar_path(&source_uasset, expected.role, "ASSET_PACK_SIDECAR")?;
        let (target_relative_name, target) =
            sidecar_path(&staged_uasset, expected.role, "ASSET_PACK_SIDECAR")?;
        let copied = copy_optional_verified_file(
            &source,
            &target,
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_PACK_SIDECAR",
        )?
        .context("ASSET_PACK_SIDECAR: validated sidecar disappeared while staging")?;
        let PublishedCopy {
            source_seal: seal,
            mut ownership,
            ..
        } = copied;
        ownership.disarm();
        if seal.length != validated.length || seal.sha256 != validated.sha256 {
            bail!("ASSET_PACK_SIDECAR: sidecar changed after pre-staging validation");
        }
        cooked_total = cooked_total
            .checked_add(seal.length)
            .context("ASSET_PACK_SIDECAR: cooked size overflowed")?;
        if cooked_total > MAX_COOKED_PACKAGE_BYTES {
            bail!(
                "ASSET_PACK_SIDECAR: cooked package is {cooked_total} bytes; aggregate limit is {MAX_COOKED_PACKAGE_BYTES}"
            );
        }
        let target_relative = mount.cooked_uasset.with_file_name(target_relative_name);
        input_components.push(packed_input_receipt(
            &seal.path,
            &target_relative,
            seal.length,
            &seal.sha256,
        ));
        expected_sidecars[expected.role.index()] = true;
        staged_input_seals.push(digest_regular_file_bounded(
            &target,
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_PACK_STAGE",
        )?);
    }

    staging.verify_owned("ASSET_PACK_OUTPUT")?;
    validate_tree_no_reparse(
        &staging.path,
        MAX_STAGING_TREE_DEPTH,
        MAX_STAGING_TREE_ENTRIES,
        "ASSET_PACK_STAGE",
    )?;
    for seal in &staged_input_seals {
        reverify_file_seal(
            seal,
            MAX_OPTIONAL_SIDECAR_BYTES.max(package_limits.max_uexp_bytes),
            "ASSET_PACK_STAGE",
        )?;
    }
    drop(carrier);

    let repacked = gore_tex::container::repack_to_zen_verified(
        &cooked_root,
        &args.name,
        &build_root,
        &game,
        false,
    )
    .context("ASSET_PACK_CONVERT")?;
    let triplet = repacked.triplet;
    let mut pack_source_utoc_paths = std::collections::BTreeSet::new();
    pack_source_utoc_paths.extend(repacked.metadata_utocs.iter().cloned());
    for chunk in &repacked.source_chunks {
        pack_source_utoc_paths.insert(chunk.source_utoc.clone());
    }
    let mut pack_source_utoc_seals = Vec::with_capacity(pack_source_utoc_paths.len());
    for source_utoc in pack_source_utoc_paths {
        pack_source_utoc_seals.push(digest_regular_file_bounded(
            &source_utoc,
            MAX_CONTAINER_COMPONENT_BYTES,
            "ASSET_PACK_GAME",
        )?);
    }
    let pack_source_utoc_receipts: Vec<_> = pack_source_utoc_seals
        .iter()
        .map(source_file_receipt)
        .collect();
    staging.verify_owned("ASSET_PACK_OUTPUT")?;
    validate_tree_no_reparse(
        &staging.path,
        MAX_STAGING_TREE_DEPTH,
        MAX_STAGING_TREE_ENTRIES,
        "ASSET_PACK_OUTPUT",
    )?;
    for seal in &staged_input_seals {
        reverify_file_seal(
            seal,
            MAX_OPTIONAL_SIDECAR_BYTES.max(package_limits.max_uexp_bytes),
            "ASSET_PACK_STAGE",
        )?;
    }
    let reopened = gore_tex::container::verify_single_package_triplet(
        &triplet[0],
        &triplet[2],
        &args.asset,
        gore_tex::container::ExpectedSidecars {
            bulk: expected_sidecars[0],
            optional_bulk: expected_sidecars[1],
            memory_mapped_bulk: expected_sidecars[2],
        },
    )
    .context("ASSET_PACK_REOPEN")?;

    let mut triplet_receipts = Vec::with_capacity(3);
    let mut triplet_seals = Vec::with_capacity(3);
    for path in &triplet {
        sync_existing_regular_file(path, "ASSET_PACK_OUTPUT")?;
        let seal =
            digest_regular_file_bounded(path, MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_OUTPUT")?;
        let relative = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("ASSET_PACK_OUTPUT: non-UTF-8 triplet filename")?;
        triplet_receipts.push(component_receipt(relative, seal.length, &seal.sha256));
        triplet_seals.push(seal);
    }
    validate_flat_triplet_directory(&build_root, &args.name)?;

    let receipt = json!({
        "format": "gore.asset.pack.v2",
        "status": "packed",
        "asset": args.asset,
        "name": args.name,
        "generation_bound": true,
        "provenance": {
            "patch_receipt": ReceiptFileSeal {
                path: patch_receipt_input.path.display().to_string(),
                length: u64::try_from(patch_receipt_input.bytes.len())?,
                sha256: encode_hex(&patch_receipt_input.sha256),
            },
            "extract_receipt": patch_receipt.provenance.extract_receipt,
            "generation": current_generation.clone(),
            "input_package_seal": patch_receipt.input_package_seal,
            "patched_package_seal": patch_receipt.output_package_seal,
        },
        "source": {
            "game_root": game.display().to_string(),
            "consumed_chunks": repacked.source_chunks,
            "source_container_tocs": pack_source_utoc_receipts,
            "content_binding": "script-object and container-header chunks were verified against the winning containers' TOC BLAKE3 hashes before conversion",
            "global_script_store": {
                "utoc": source_file_receipt(&global_utoc_seal),
                "ucas": source_file_receipt(&global_ucas_seal),
            },
        },
        "input": {
            "package_seal": source_seal,
            "components": input_components,
        },
        "output": {
            "root": out.display().to_string(),
            "receipt": PACK_RECEIPT_NAME,
            "triplet": triplet_receipts,
            "reopened_packages": [reopened.package.clone()],
            "strict_reopen": reopened,
            "compressed": false,
        },
        "deployed": false,
    });
    let printable = serde_json::to_string_pretty(&receipt)?;
    write_new_synced(
        &build_root.join(PACK_RECEIPT_NAME),
        printable.as_bytes(),
        "ASSET_PACK_RECEIPT",
    )?;
    reverify_file_seal(
        &global_utoc_seal,
        MAX_CONTAINER_COMPONENT_BYTES,
        "ASSET_PACK_GAME",
    )?;
    reverify_file_seal(
        &global_ucas_seal,
        MAX_CONTAINER_COMPONENT_BYTES,
        "ASSET_PACK_GAME",
    )?;
    for seal in &pack_source_utoc_seals {
        reverify_file_seal(seal, MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_GAME")?;
    }
    reverify_file_seal(
        &main_utoc_seal,
        MAX_CONTAINER_COMPONENT_BYTES,
        "ASSET_PACK_GENERATION",
    )?;
    main_ucas_identity.reverify("ASSET_PACK_GENERATION")?;
    reverify_file_seal(
        &current_usmap_seal,
        MAX_USMAP_BYTES,
        "ASSET_PACK_GENERATION",
    )?;
    for seal in &generation_utoc_seals {
        reverify_file_seal(seal, MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_GENERATION")?;
    }
    let reloaded = PackageCarrier::load(&source_uasset, package_limits)
        .context("ASSET_PACK_INPUT: reverifying source package")?;
    if PackagePairSeal::capture(&reloaded) != source_seal {
        bail!("ASSET_PACK_INPUT: source package changed during pack");
    }
    for seal in &sidecar_source_seals {
        reverify_file_seal(seal, MAX_OPTIONAL_SIDECAR_BYTES, "ASSET_PACK_SIDECAR")?;
    }
    for seal in &triplet_seals {
        reverify_file_seal(seal, MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_OUTPUT")?;
    }
    staging.verify_owned("ASSET_PACK_OUTPUT")?;
    validate_tree_no_reparse(
        &staging.path,
        MAX_STAGING_TREE_DEPTH,
        MAX_STAGING_TREE_ENTRIES,
        "ASSET_PACK_OUTPUT",
    )?;
    run_final_publish_gate(
        &mount_inventory,
        "ASSET_PACK_GAME",
        || {
            probe_current_generation_receipt(
                &game,
                &args.asset,
                &patch_receipt.provenance.generation,
                "ASSET_PACK_GENERATION",
            )
        },
        |current| {
            if current != &patch_receipt.provenance.generation {
                let reason =
                    generation_mismatch_reason(&patch_receipt.provenance.generation, current);
                bail!(
                    "ASSET_GENERATION_MISMATCH: target generation changed before pack publication ({reason})"
                );
            }
            Ok(())
        },
    )?;
    publish_staged_directory(&build_root, &out, "ASSET_PACK_OUTPUT")?;
    staging.cleanup("ASSET_PACK_OUTPUT")?;

    if args.json {
        println!("{printable}");
    } else {
        println!(
            "PACKED\tasset={}\tname={}\toutput={}\treceipt={}\tdeployed=false",
            serde_json::to_string(&receipt["asset"])?,
            serde_json::to_string(&receipt["name"])?,
            serde_json::to_string(&out.display().to_string())?,
            serde_json::to_string(&out.join(PACK_RECEIPT_NAME).display().to_string())?,
        );
    }
    Ok(())
}

fn validate_game_asset_path(asset: &str, code: &'static str) -> Result<AssetMount> {
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
    let leaf = segments
        .last()
        .expect("a non-empty segment list was checked above")
        .to_string();
    let mut cooked_uasset = PathBuf::from("G1R");
    cooked_uasset.push("Content");
    for segment in segments {
        cooked_uasset.push(segment);
    }
    cooked_uasset.set_extension("uasset");
    Ok(AssetMount {
        leaf,
        cooked_uasset,
    })
}

fn validate_triplet_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 96
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        || windows_reserved_name(name)
    {
        bail!(
            "ASSET_PACK_NAME: --name must be 1..=96 ASCII letters, digits, '_' or '-', and not a reserved device name"
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

fn resolve_game_root(path: &Path, code: &'static str) -> Result<PathBuf> {
    let root = validate_existing_path_no_reparse(path, true, code)?;
    validate_existing_path_no_reparse(&root.join("G1R"), true, code).with_context(|| {
        format!("{code}: --game must be the install root containing a plain G1R directory")
    })?;
    Ok(root)
}

fn scan_game_iostore_directory(
    game_root: &Path,
    code: &'static str,
) -> Result<Vec<(String, DirectMountKind, PathBuf)>> {
    let paks = validate_existing_path_no_reparse(&game_root.join("G1R/Content/Paks"), true, code)?;
    let mut pending = vec![(paks.clone(), 0usize)];
    let mut entries = 0usize;
    let mut direct_mounts = Vec::new();
    while let Some((directory, depth)) = pending.pop() {
        let checked_directory = validate_existing_path_no_reparse(&directory, true, code)?;
        if checked_directory != directory {
            bail!(
                "{code}: Paks directory identity changed during bounded scan: {}",
                directory.display()
            );
        }
        for entry in fs::read_dir(&directory).with_context(|| {
            format!(
                "{code}: reading IoStore directory '{}'",
                directory.display()
            )
        })? {
            let entry = entry.with_context(|| format!("{code}: reading IoStore entry"))?;
            entries = entries
                .checked_add(1)
                .context("IoStore directory entry count overflowed")?;
            if entries > MAX_PAKS_SCAN_ENTRIES {
                bail!(
                    "{code}: Paks tree exceeds bounded scan limit of {MAX_PAKS_SCAN_ENTRIES} entries; undeploy every mod and clean the Paks tree before retrying"
                );
            }
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .with_context(|| format!("{code}: inspecting '{}'", path.display()))?;
            if metadata_is_reparse(&metadata) {
                bail!(
                    "{code}: symbolic-link or reparse entry in the Paks tree is refused: {}",
                    path.display()
                );
            }
            if metadata.is_dir() {
                let next_depth = depth
                    .checked_add(1)
                    .context("IoStore directory depth overflowed")?;
                if next_depth > MAX_PAKS_SCAN_DEPTH {
                    bail!(
                        "{code}: Paks tree exceeds bounded scan depth {MAX_PAKS_SCAN_DEPTH}; undeploy every mod and clean the Paks tree before retrying"
                    );
                }
                pending.push((path, next_depth));
                continue;
            }
            if !metadata.is_file() {
                bail!(
                    "{code}: non-regular entry in the Paks tree is refused: {}",
                    path.display()
                );
            }

            let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            let canonical = [
                ("utoc", DirectMountKind::Utoc),
                ("ucas", DirectMountKind::Ucas),
                ("pak", DirectMountKind::Pak),
            ]
            .into_iter()
            .find(|(candidate, _)| extension.eq_ignore_ascii_case(candidate));
            let Some((canonical_extension, kind)) = canonical else {
                continue;
            };
            if depth == 0 {
                if extension != canonical_extension {
                    bail!(
                        "{code}: noncanonical IoStore extension casing is refused at '{}'; rename .{extension} to .{canonical_extension}",
                        path.display()
                    );
                }
                let file_name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .context(format!("{code}: direct mount filename is non-UTF-8"))?
                    .to_owned();
                direct_mounts.push((file_name, kind, path));
            } else {
                bail!(
                    "{code}: active game-mountable container file found below Paks: '{}'; undeploy every mod and remove or relocate all .utoc/.ucas/.pak files from Paks subdirectories before retrying against a clean game tree",
                    path.display()
                );
            }
        }
    }
    direct_mounts.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(direct_mounts)
}

fn capture_game_mount_inventory(
    game_root: &Path,
    code: &'static str,
) -> Result<GameMountInventory> {
    let direct_mounts = scan_game_iostore_directory(game_root, code)?;
    let mut anchors = Vec::with_capacity(direct_mounts.len());
    let mut hashed_files = Vec::new();
    let mut held_ucas = Vec::new();
    for (file_name, kind, path) in direct_mounts {
        match kind {
            DirectMountKind::Utoc | DirectMountKind::Pak => {
                let limit = if kind == DirectMountKind::Utoc {
                    MAX_MOUNT_UTOC_BYTES
                } else {
                    MAX_CONTAINER_COMPONENT_BYTES
                };
                let seal = digest_regular_file_bounded(&path, limit, code)?;
                anchors.push(DirectMountAnchor::Hashed {
                    file_name,
                    kind,
                    length: seal.length,
                    sha256: seal.sha256,
                });
                hashed_files.push((seal, limit));
            }
            DirectMountKind::Ucas => {
                let held = HeldFileIdentity::open(&path, MAX_MOUNT_UCAS_BYTES, code)?;
                anchors.push(DirectMountAnchor::Identity {
                    file_name,
                    kind,
                    snapshot: held.snapshot.clone(),
                });
                held_ucas.push(held);
            }
        }
    }
    Ok(GameMountInventory {
        game_root: game_root.to_path_buf(),
        anchors,
        hashed_files,
        held_ucas,
    })
}

impl GameMountInventory {
    fn reverify_exact(&self, code: &'static str) -> Result<()> {
        for (seal, limit) in &self.hashed_files {
            reverify_file_seal(seal, *limit, code)?;
        }
        for held in &self.held_ucas {
            held.reverify(code)?;
        }
        let current = capture_game_mount_inventory(&self.game_root, code)?;
        if current.anchors != self.anchors {
            bail!(
                "{code}: direct-root Paks mount inventory changed during the operation; retry against a clean, single-writer game tree"
            );
        }
        Ok(())
    }
}

fn seal_global_script_store(game_root: &Path, code: &'static str) -> Result<(FileSeal, FileSeal)> {
    let paks = game_root.join("G1R/Content/Paks");
    let utoc = digest_regular_file_bounded(
        &paks.join("global.utoc"),
        MAX_CONTAINER_COMPONENT_BYTES,
        code,
    )?;
    let ucas = digest_regular_file_bounded(
        &paks.join("global.ucas"),
        MAX_CONTAINER_COMPONENT_BYTES,
        code,
    )?;
    Ok((utoc, ucas))
}

fn prepare_absent_output_directory(
    requested: &Path,
    game_root: &Path,
    code: &'static str,
) -> Result<PathBuf> {
    let absolute = absolute_without_parent_components(requested, code)?;
    let file_name = absolute
        .file_name()
        .and_then(|name| name.to_str())
        .context(format!(
            "{code}: output requires a UTF-8 final directory name"
        ))?;
    validate_output_component(file_name, code)?;
    let parent = absolute
        .parent()
        .context(format!("{code}: output directory has no parent"))?;
    let canonical_parent = validate_existing_path_no_reparse(parent, true, code)?;
    let output = canonical_parent.join(file_name);
    ensure_path_absent(&output, code)?;
    let canonical_game = fs::canonicalize(game_root)
        .with_context(|| format!("{code}: canonicalizing game root '{}'", game_root.display()))?;
    if output.starts_with(&canonical_game) {
        bail!(
            "{code}: output inside the live game tree is refused: {}",
            output.display()
        );
    }
    Ok(output)
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

fn validate_tree_no_reparse(
    root: &Path,
    max_depth: usize,
    max_entries: usize,
    code: &'static str,
) -> Result<()> {
    validate_existing_path_no_reparse(root, true, code)?;
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut entries = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        if depth > max_depth {
            bail!("{code}: staging tree exceeds depth limit {max_depth}");
        }
        for entry in fs::read_dir(&directory)
            .with_context(|| format!("{code}: reading '{}'", directory.display()))?
        {
            let entry = entry.with_context(|| format!("{code}: reading staging entry"))?;
            entries = entries
                .checked_add(1)
                .context("staging entry count overflowed")?;
            if entries > max_entries {
                bail!("{code}: staging tree exceeds entry limit {max_entries}");
            }
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .with_context(|| format!("{code}: inspecting '{}'", path.display()))?;
            if metadata_is_reparse(&metadata) {
                bail!(
                    "{code}: symbolic-link or reparse entry is refused: {}",
                    path.display()
                );
            }
            if metadata.is_dir() {
                pending.push((path, depth + 1));
            } else if !metadata.is_file() {
                bail!(
                    "{code}: non-regular staging entry is refused: {}",
                    path.display()
                );
            }
        }
    }
    Ok(())
}

fn ensure_path_absent(path: &Path, code: &'static str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("{code}: inspecting '{}'", path.display()))
        }
        Ok(_) => bail!(
            "{code}: output already exists; no-clobber policy refused {}",
            path.display()
        ),
    }
}

fn create_staging_directory(
    parent: &Path,
    purpose: &str,
    code: &'static str,
) -> Result<StagingDirectory> {
    let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let prefix = format!(".gore-asset-{purpose}-{}-{sequence}-", std::process::id());
    let temporary = tempfile::Builder::new()
        .prefix(&prefix)
        .tempdir_in(parent)
        .with_context(|| format!("{code}: reserving random staging directory"))?;
    let identity = HeldFileIdentity::open_directory(temporary.path(), code)?;
    let path = temporary.keep();
    Ok(StagingDirectory {
        path,
        identity,
        armed: true,
    })
}

fn create_relative_directory_chain(
    root: &Path,
    relative: &Path,
    code: &'static str,
) -> Result<PathBuf> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            bail!("{code}: non-normal staged path component is refused");
        };
        current.push(component);
        fs::create_dir(&current).with_context(|| {
            format!(
                "{code}: exclusively creating staged directory '{}'",
                current.display()
            )
        })?;
        let metadata = fs::symlink_metadata(&current)
            .with_context(|| format!("{code}: inspecting '{}'", current.display()))?;
        if metadata_is_reparse(&metadata) || !metadata.is_dir() {
            bail!(
                "{code}: staged path is not a plain directory: {}",
                current.display()
            );
        }
    }
    Ok(current)
}

fn publish_staged_directory(staged: &Path, output: &Path, code: &'static str) -> Result<()> {
    ensure_path_absent(output, code)?;
    sync_directory_before_publish(staged).with_context(|| {
        format!(
            "{code}: syncing staged directory '{}' before publication",
            staged.display()
        )
    })?;
    promote_directory_noclobber(staged, output).with_context(|| {
        format!(
            "{code}: publishing staged directory '{}' as '{}'",
            staged.display(),
            output.display()
        )
    })?;
    sync_parents_after_publish(staged, output).with_context(|| {
        format!(
            "{code}: syncing output parent after publishing '{}'",
            output.display()
        )
    })
}

#[cfg(windows)]
fn promote_directory_noclobber(staged: &Path, output: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

    let source: Vec<u16> = staged
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let destination: Vec<u16> = output
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // Deliberately omit MOVEFILE_REPLACE_EXISTING. A destination created after
    // the friendly preflight check wins the race and is never replaced.
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn promote_directory_noclobber(staged: &Path, output: &Path) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt as _;

    let source = CString::new(staged.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let destination = CString::new(output.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let renamed = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if renamed == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn promote_directory_noclobber(staged: &Path, output: &Path) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt as _;

    let source = CString::new(staged.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let destination = CString::new(output.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let renamed =
        unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    if renamed == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn promote_directory_noclobber(staged: &Path, output: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(output) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::rename(staged, output),
        Ok(_) => Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists)),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn sync_directory_before_publish(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory_before_publish(_path: &Path) -> std::io::Result<()> {
    // Windows publication itself uses MOVEFILE_WRITE_THROUGH. Every flat child
    // file is flushed explicitly before this point.
    Ok(())
}

#[cfg(unix)]
fn sync_parents_after_publish(staged: &Path, output: &Path) -> std::io::Result<()> {
    let source_parent = staged
        .parent()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let output_parent = output
        .parent()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    File::open(source_parent)?.sync_all()?;
    if source_parent != output_parent {
        File::open(output_parent)?.sync_all()?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn sync_parents_after_publish(_staged: &Path, _output: &Path) -> std::io::Result<()> {
    // MOVEFILE_WRITE_THROUGH is the Windows directory-entry durability barrier.
    Ok(())
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
) -> Result<Option<FileSeal>> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("{code}: inspecting '{}'", path.display()))
        }
        Ok(_) => digest_regular_file_bounded(path, limit, code).map(Some),
    }
}

impl HeldFileIdentity {
    fn open(path: &Path, limit: u64, code: &'static str) -> Result<Self> {
        let path = validate_existing_path_no_reparse(path, false, code)?;
        let file =
            File::open(&path).with_context(|| format!("{code}: opening '{}'", path.display()))?;
        let snapshot = file_identity_snapshot(&file, &file.metadata().context(code)?, code)?;
        if snapshot.length > limit {
            bail!(
                "{code}: '{}' is {} bytes; identity-only limit is {limit}",
                path.display(),
                snapshot.length
            );
        }
        Ok(Self {
            path,
            file,
            snapshot,
            expect_directory: false,
        })
    }

    fn open_directory(path: &Path, code: &'static str) -> Result<Self> {
        let path = validate_existing_path_no_reparse(path, true, code)?;
        let file = open_directory_no_follow(&path, code)?;
        let metadata = file.metadata().context(code)?;
        if metadata_is_reparse(&metadata) || !metadata.is_dir() {
            bail!("{code}: staging root handle is not a plain directory");
        }
        let snapshot = file_identity_snapshot(&file, &metadata, code)?;
        Ok(Self {
            path,
            file,
            snapshot,
            expect_directory: true,
        })
    }

    fn receipt(&self) -> HeldIdentityReceipt {
        HeldIdentityReceipt {
            path: self.path.display().to_string(),
            length: self.snapshot.length,
            modified_stamp: self.snapshot.modified_stamp.clone(),
            platform_identity: self.snapshot.platform_identity.clone(),
            sha256: None,
            verification: HELD_IDENTITY_VERIFICATION.to_owned(),
            content_hash_omitted: true,
            limitation: HELD_IDENTITY_LIMITATION.to_owned(),
        }
    }

    fn reverify(&self, code: &'static str) -> Result<()> {
        let handle_snapshot =
            file_identity_snapshot(&self.file, &self.file.metadata().context(code)?, code)?;
        if !self.snapshot_matches(&handle_snapshot) {
            bail!(
                "{code}: held source file changed during conversion: {}",
                self.path.display()
            );
        }
        let current_path =
            validate_existing_path_no_reparse(&self.path, self.expect_directory, code)?;
        if current_path != self.path {
            bail!(
                "{code}: source file path identity changed during conversion: {}",
                self.path.display()
            );
        }
        let reopened = if self.expect_directory {
            open_directory_no_follow(&current_path, code)?
        } else {
            File::open(&current_path)
                .with_context(|| format!("{code}: reopening '{}'", current_path.display()))?
        };
        let path_snapshot =
            file_identity_snapshot(&reopened, &reopened.metadata().context(code)?, code)?;
        if !self.snapshot_matches(&path_snapshot) {
            bail!(
                "{code}: source file was replaced during conversion: {}",
                self.path.display()
            );
        }
        Ok(())
    }

    fn snapshot_matches(&self, actual: &FileIdentitySnapshot) -> bool {
        self.snapshot.platform_identity == actual.platform_identity
            && (self.expect_directory
                || (self.snapshot.length == actual.length
                    && self.snapshot.modified_stamp == actual.modified_stamp))
    }
}

fn file_identity_snapshot(
    file: &File,
    metadata: &fs::Metadata,
    code: &'static str,
) -> Result<FileIdentitySnapshot> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        use std::os::windows::io::AsRawHandle as _;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        };

        // SAFETY: `information` is a writable instance of the exact Win32
        // structure and `file` owns a live handle for the duration of the call.
        let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        let succeeded =
            unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) };
        if succeeded == 0 {
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("{code}: reading stable Windows file identity"));
        }
        let index =
            (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow);
        Ok(FileIdentitySnapshot {
            length: metadata.file_size(),
            modified_stamp: metadata.last_write_time().to_string(),
            platform_identity: format!(
                "windows-volume-{:08x}-file-{index:016x}",
                information.dwVolumeSerialNumber
            ),
        })
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        let _ = file;
        Ok(FileIdentitySnapshot {
            length: metadata.len(),
            modified_stamp: format!("{}.{:09}", metadata.mtime(), metadata.mtime_nsec()),
            platform_identity: format!("unix-dev-{:x}-ino-{:x}", metadata.dev(), metadata.ino()),
        })
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = file;
        let _ = metadata;
        bail!("{code}: robust file identity is unsupported on this platform")
    }
}

#[cfg(windows)]
fn open_directory_no_follow(path: &Path, code: &'static str) -> Result<File> {
    use std::os::windows::ffi::OsStrExt as _;
    use std::os::windows::io::FromRawHandle as _;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("{code}: opening directory without following reparse data"));
    }
    // SAFETY: CreateFileW returned a new owned handle and ownership transfers to File.
    Ok(unsafe { File::from_raw_handle(handle) })
}

#[cfg(unix)]
fn open_directory_no_follow(path: &Path, code: &'static str) -> Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path)
        .with_context(|| format!("{code}: opening directory without following symlinks"))
}

#[cfg(not(any(windows, unix)))]
fn open_directory_no_follow(path: &Path, code: &'static str) -> Result<File> {
    File::open(path).with_context(|| format!("{code}: opening directory"))
}

fn digest_regular_file_bounded(path: &Path, limit: u64, code: &'static str) -> Result<FileSeal> {
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
    Ok(FileSeal {
        path: canonical,
        length,
        sha256,
    })
}

fn reverify_file_seal(seal: &FileSeal, limit: u64, code: &'static str) -> Result<()> {
    let current = validate_existing_path_no_reparse(&seal.path, false, code)?;
    if current != seal.path {
        bail!(
            "{code}: sealed source path changed: {}",
            seal.path.display()
        );
    }
    verify_file_hash(&current, seal.length, seal.sha256, limit, code)
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

fn copy_optional_verified_file(
    source: &Path,
    target: &Path,
    limit: u64,
    code: &'static str,
) -> Result<Option<PublishedCopy>> {
    copy_optional_verified_file_with_post_publish(source, target, limit, code, || Ok(()))
}

fn copy_optional_verified_file_with_post_publish(
    source: &Path,
    target: &Path,
    limit: u64,
    code: &'static str,
    post_publish: impl FnOnce() -> Result<()>,
) -> Result<Option<PublishedCopy>> {
    let source_seal = match digest_optional_regular_file_bounded(source, limit, code)? {
        Some(seal) => seal,
        None => return Ok(None),
    };
    ensure_path_absent(target, code)?;
    let target_parent = target
        .parent()
        .with_context(|| format!("{code}: sidecar target has no parent"))?;
    validate_existing_path_no_reparse(target_parent, true, code)?;
    let mut reader = File::open(&source_seal.path)
        .with_context(|| format!("{code}: reopening '{}'", source_seal.path.display()))?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".gore-sidecar-")
        .tempfile_in(target_parent)
        .with_context(|| format!("{code}: creating temporary sidecar copy"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut length = 0u64;
    loop {
        let read = reader.read(&mut buffer).context(code)?;
        if read == 0 {
            break;
        }
        length = length.checked_add(u64::try_from(read)?).context(code)?;
        if length > limit {
            bail!("{code}: sidecar grew beyond {limit} bytes while being copied");
        }
        temporary
            .as_file_mut()
            .write_all(&buffer[..read])
            .context(code)?;
        hasher.update(&buffer[..read]);
    }
    temporary.as_file_mut().sync_all().context(code)?;
    let copied_sha256: [u8; 32] = hasher.finalize().into();
    if length != source_seal.length || copied_sha256 != source_seal.sha256 {
        bail!(
            "{code}: source changed while being copied: {}",
            source.display()
        );
    }
    verify_file_hash(
        &source_seal.path,
        source_seal.length,
        source_seal.sha256,
        limit,
        code,
    )?;
    let published = temporary
        .persist_noclobber(target)
        .map_err(|error| error.error)
        .with_context(|| format!("{code}: publishing sidecar copy '{}'", target.display()))?;
    // Arm ownership before the first fallible operation after publication.
    let ownership = OwnedPublishedPath::armed(target.to_path_buf());
    post_publish()?;
    published.sync_all().context(code)?;
    let target_seal = digest_regular_file_bounded(target, limit, code)?;
    if target_seal.length != source_seal.length || target_seal.sha256 != source_seal.sha256 {
        bail!(
            "{code}: staged copy verification failed: {}",
            target.display()
        );
    }
    Ok(Some(PublishedCopy {
        source_seal,
        target_seal,
        ownership,
    }))
}

fn write_owned_new_synced(
    path: &Path,
    bytes: &[u8],
    code: &'static str,
) -> Result<OwnedPublishedPath> {
    write_owned_new_synced_with_post_publish(path, bytes, code, || Ok(()))
}

fn write_owned_new_synced_with_post_publish(
    path: &Path,
    bytes: &[u8],
    code: &'static str,
    post_publish: impl FnOnce() -> Result<()>,
) -> Result<OwnedPublishedPath> {
    ensure_path_absent(path, code)?;
    let parent = path
        .parent()
        .with_context(|| format!("{code}: output file has no parent"))?;
    validate_existing_path_no_reparse(parent, true, code)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".gore-receipt-")
        .tempfile_in(parent)
        .with_context(|| format!("{code}: creating temporary receipt"))?;
    temporary
        .as_file_mut()
        .write_all(bytes)
        .with_context(|| format!("{code}: writing temporary receipt"))?;
    temporary
        .as_file_mut()
        .sync_all()
        .with_context(|| format!("{code}: syncing temporary receipt"))?;
    let published = temporary
        .persist_noclobber(path)
        .map_err(|error| error.error)
        .with_context(|| format!("{code}: publishing receipt '{}'", path.display()))?;
    // Arm ownership before sync, injected hooks, or any later validation.
    let ownership = OwnedPublishedPath::armed(path.to_path_buf());
    post_publish()?;
    published
        .sync_all()
        .with_context(|| format!("{code}: syncing published receipt"))?;
    Ok(ownership)
}

fn write_new_synced(path: &Path, bytes: &[u8], code: &'static str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("{code}: creating '{}'", path.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("{code}: writing '{}'", path.display()))?;
    file.sync_all()
        .with_context(|| format!("{code}: syncing '{}'", path.display()))?;
    Ok(())
}

fn sync_existing_regular_file(path: &Path, code: &'static str) -> Result<()> {
    let path = validate_existing_path_no_reparse(path, false, code)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("{code}: opening '{}' for durability sync", path.display()))?;
    file.sync_all()
        .with_context(|| format!("{code}: syncing '{}'", path.display()))
}

fn validate_flat_extraction_directory(
    directory: &Path,
    components: &[ReceiptComponent],
) -> Result<()> {
    let mut allowed: Vec<_> = components
        .iter()
        .map(|component| component.relative_path.clone())
        .collect();
    allowed.sort();
    let mut found = Vec::new();
    for entry in fs::read_dir(directory).context("ASSET_EXTRACT_PAIR")? {
        let entry = entry.context("ASSET_EXTRACT_PAIR")?;
        let metadata = fs::symlink_metadata(entry.path()).context("ASSET_EXTRACT_PAIR")?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .context("ASSET_EXTRACT_PAIR: non-UTF-8 output filename")?;
        if metadata_is_reparse(&metadata)
            || !metadata.is_file()
            || !allowed.iter().any(|allowed| allowed == name)
        {
            bail!(
                "ASSET_EXTRACT_PAIR: converter produced unexpected entry {:?}",
                name
            );
        }
        found.push(name.to_owned());
    }
    found.sort();
    if found != allowed {
        bail!(
            "ASSET_EXTRACT_PAIR: converter output set is incomplete: expected {:?}, got {:?}",
            allowed,
            found
        );
    }
    Ok(())
}

fn validate_flat_triplet_directory(directory: &Path, name: &str) -> Result<()> {
    let allowed = [
        format!("{name}.utoc"),
        format!("{name}.ucas"),
        format!("{name}.pak"),
    ];
    let mut found = 0usize;
    for entry in fs::read_dir(directory).context("ASSET_PACK_OUTPUT")? {
        let entry = entry.context("ASSET_PACK_OUTPUT")?;
        let metadata = fs::symlink_metadata(entry.path()).context("ASSET_PACK_OUTPUT")?;
        let filename = entry.file_name();
        let filename = filename
            .to_str()
            .context("ASSET_PACK_OUTPUT: non-UTF-8 triplet filename")?;
        if metadata_is_reparse(&metadata)
            || !metadata.is_file()
            || !allowed.iter().any(|x| x == filename)
        {
            bail!("ASSET_PACK_OUTPUT: unexpected triplet entry {filename:?}");
        }
        found += 1;
    }
    if found != allowed.len() {
        bail!(
            "ASSET_PACK_OUTPUT: incomplete triplet; found {found} of {} files",
            allowed.len()
        );
    }
    Ok(())
}

fn source_file_receipt(seal: &FileSeal) -> SourceFileReceipt {
    SourceFileReceipt {
        path: seal.path.display().to_string(),
        length: seal.length,
        sha256: encode_hex(&seal.sha256),
    }
}

fn read_chained_extract_receipt(
    expected: &ReceiptFileSeal,
) -> Result<(ExtractReceiptEnvelope, ValidatedExtractBinding)> {
    let code = "ASSET_EXTRACT_RECEIPT";
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
    Ok((receipt, binding))
}

fn build_generation_receipt(
    asset: &str,
    usmap: &FileSeal,
    main_utoc: &FileSeal,
    global_utoc: &FileSeal,
    global_ucas: &FileSeal,
    chunks: &[gore_tex::container::VerifiedChunkReceipt],
    source_utocs: &[FileSeal],
) -> Result<AssetGenerationReceipt> {
    let mut by_path = std::collections::BTreeMap::new();
    let mut container_set = Vec::with_capacity(source_utocs.len());
    for seal in source_utocs {
        by_path.insert(seal.path.clone(), seal);
        container_set.push(GenerationFileAnchor::from_seal(seal)?);
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
        let canonical = fs::canonicalize(&chunk.source_utoc).with_context(|| {
            format!(
                "ASSET_GENERATION: canonicalizing winning TOC '{}'",
                chunk.source_utoc.display()
            )
        })?;
        let winner = by_path.get(&canonical).with_context(|| {
            format!(
                "ASSET_GENERATION: winning TOC '{}' is missing from sealed container set",
                canonical.display()
            )
        })?;
        target_chunks.push(GenerationChunkAnchor {
            chunk_id: chunk.chunk_id.clone(),
            chunk_type: chunk.chunk_type.clone(),
            winner_utoc: GenerationFileAnchor::from_seal(winner)?,
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
    {
        bail!("ASSET_GENERATION: no target ExportBundleData chunk was sealed");
    }
    if !target_chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "ContainerHeader")
    {
        bail!("ASSET_GENERATION: no ContainerHeader chunk was sealed");
    }

    Ok(AssetGenerationReceipt {
        format: "gore.asset.generation.v1".to_owned(),
        asset: asset.to_owned(),
        usmap: GenerationFileAnchor::from_seal(usmap)?,
        main_utoc: GenerationFileAnchor::from_seal(main_utoc)?,
        global_utoc: GenerationFileAnchor::from_seal(global_utoc)?,
        global_ucas: GenerationFileAnchor::from_seal(global_ucas)?,
        container_set,
        target_chunks,
    })
}

fn generation_mismatch_reason(
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

fn probe_current_generation_receipt(
    game: &Path,
    asset: &str,
    expected: &AssetGenerationReceipt,
    code: &'static str,
) -> Result<AssetGenerationReceipt> {
    let main_utoc =
        gore_tex::paths::main_container(game).with_context(|| format!("{code}: main UTOC"))?;
    let usmap = gore_tex::paths::usmap(game).with_context(|| format!("{code}: USMAP"))?;
    let main_utoc_seal =
        digest_regular_file_bounded(&main_utoc, MAX_CONTAINER_COMPONENT_BYTES, code)?;
    let usmap_seal = digest_regular_file_bounded(&usmap, MAX_USMAP_BYTES, code)?;
    let (global_utoc_seal, global_ucas_seal) = seal_global_script_store(game, code)?;
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
    .with_context(|| format!("{code}: final target generation probe"))?;
    let mut source_utoc_seals = Vec::with_capacity(probe.metadata_utocs.len());
    for source_utoc in &probe.metadata_utocs {
        source_utoc_seals.push(digest_regular_file_bounded(
            source_utoc,
            MAX_CONTAINER_COMPONENT_BYTES,
            code,
        )?);
    }
    build_generation_receipt(
        asset,
        &usmap_seal,
        &main_utoc_seal,
        &global_utoc_seal,
        &global_ucas_seal,
        &probe.consumed_chunks,
        &source_utoc_seals,
    )
}

fn run_final_publish_gate<T>(
    mount_inventory: &GameMountInventory,
    code: &'static str,
    probe: impl FnOnce() -> Result<T>,
    validate_probe: impl FnOnce(&T) -> Result<()>,
) -> Result<T> {
    let result = probe()?;
    validate_probe(&result)?;
    // This inventory check intentionally follows the generation probe. A new
    // direct-root mount that appears after the probe cannot silently become a
    // game winner before publication.
    mount_inventory.reverify_exact(code)?;
    Ok(result)
}

fn validate_generation_receipt(
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

fn is_canonical_hex(value: &str, bytes: usize) -> bool {
    value.len() == bytes.saturating_mul(2)
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value == value.to_ascii_lowercase()
}

fn is_canonical_sha256(value: &str) -> bool {
    is_canonical_hex(value, 32)
}

fn validate_sidecar_receipts(
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

fn validate_sidecar_generation_mapping(
    sidecars: &[SidecarReceipt],
    generation: &AssetGenerationReceipt,
    code: &'static str,
) -> Result<()> {
    let expected: Vec<_> = sidecars.iter().map(|sidecar| sidecar.role).collect();
    let mut actual: Vec<_> = generation
        .target_chunks
        .iter()
        .filter(|chunk| {
            gore_tex::container::chunk_id_matches_asset_path(&chunk.chunk_id, &generation.asset)
        })
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

fn validate_patch_receipt_envelope(
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

fn validate_lexical_receipt_path(path: &Path, code: &'static str) -> Result<PathBuf> {
    if !path.is_absolute() {
        bail!(
            "{code}: source proof path is not absolute: {}",
            path.display()
        );
    }

    // Do not touch the historical source path on disk here. Rebuild it from its
    // lexical components instead, then require byte-for-byte native spelling.
    // This rejects `.`/`..`, repeated or alternate separators, and unsafe
    // Windows component aliases while preserving canonical verbatim/UNC
    // prefixes emitted by `fs::canonicalize`. Exact comparisons below also
    // reject casing differences between duplicated receipt fields.
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

fn validate_extract_receipt_envelope(
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

fn component_receipt(relative_path: &str, length: u64, sha256: &[u8; 32]) -> ReceiptComponent {
    ReceiptComponent {
        relative_path: relative_path.to_owned(),
        length,
        sha256: encode_hex(sha256),
    }
}

fn packed_input_receipt(
    source_path: &Path,
    packed_relative_path: &Path,
    length: u64,
    sha256: &[u8; 32],
) -> serde_json::Value {
    json!({
        "source_path": source_path.display().to_string(),
        "packed_relative_path": packed_relative_path.to_string_lossy().replace('\\', "/"),
        "length": length,
        "sha256": encode_hex(sha256),
    })
}

#[derive(Debug)]
struct VerifiedInput {
    path: PathBuf,
    bytes: Vec<u8>,
    sha256: [u8; 32],
}

#[derive(Debug, Serialize)]
struct ExportReport {
    index: usize,
    object_name: String,
    class_path: String,
    component: PackageComponent,
    offset: usize,
    length: usize,
    status: &'static str,
    error: Option<String>,
    schema: Option<String>,
    property_bytes: Option<usize>,
    native_suffix_bytes: Option<usize>,
    leaves: Vec<LeafReport>,
}

#[derive(Debug, Serialize)]
struct LeafReport {
    index: usize,
    semantic_path: String,
    editable: bool,
    selector: FixedLeafSelector,
}

fn inspect(args: InspectArgs) -> Result<()> {
    let usmap = read_verified_bounded(&args.usmap, MAX_USMAP_BYTES, "ASSET_USMAP")?;
    let schemas = SchemaDb::from_usmap(&usmap.bytes).context("ASSET_USMAP")?;
    let carrier =
        PackageCarrier::load(&args.uasset, asset_package_limits()).context("ASSET_INPUT")?;
    let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).context("ASSET_ENVELOPE")?;

    let indices = match args.export_index {
        Some(index) => {
            if index >= package.exports().len() {
                bail!(
                    "ASSET_EXPORT: export {index} does not exist (exports={})",
                    package.exports().len()
                );
            }
            vec![index]
        }
        None => (0..package.exports().len()).collect(),
    };

    let mut reports = Vec::with_capacity(indices.len());
    for index in indices {
        let boundary = package
            .exports()
            .get(index)
            .expect("selected export index was bounded above");
        let mut report = ExportReport {
            index,
            object_name: boundary.object_name().to_owned(),
            class_path: boundary.class_path().to_owned(),
            component: boundary.component(),
            offset: boundary.offset(),
            length: boundary.length(),
            status: "unsupported",
            error: None,
            schema: None,
            property_bytes: None,
            native_suffix_bytes: None,
            leaves: Vec::new(),
        };

        let walked = (|| -> Result<_> {
            let export = package.export(index).context("ASSET_EXPORT")?;
            let schema_id = export
                .boundary()
                .resolve_class_schema(&schemas)
                .context("ASSET_SCHEMA")?;
            let block = PropertySpanWalker::g1r_ue5_4(&schemas)
                .walk(export.bytes(), schema_id)
                .context("ASSET_WALK")?;
            let descriptors =
                describe_fixed_leaves(&carrier, &export, &schemas).context("ASSET_SELECTOR")?;
            let property_bytes = block.consumed();
            let native_suffix_bytes = export
                .bytes()
                .len()
                .checked_sub(property_bytes)
                .context("ASSET_WALK: property range exceeds export")?;
            Ok((
                block.schema_name().to_owned(),
                property_bytes,
                native_suffix_bytes,
                descriptors,
            ))
        })();

        match walked {
            Ok((schema, property_bytes, native_suffix_bytes, descriptors)) => {
                report.status = "walked";
                report.schema = Some(schema);
                report.property_bytes = Some(property_bytes);
                report.native_suffix_bytes = Some(native_suffix_bytes);
                report.leaves = descriptors
                    .into_iter()
                    .enumerate()
                    .map(|(leaf_index, descriptor)| leaf_report(leaf_index, descriptor))
                    .collect();
            }
            Err(error) if args.export_index.is_none() => {
                report.error = Some(format!("{error:#}"));
            }
            Err(error) => return Err(error),
        }
        reports.push(report);
    }

    let walked_exports = reports
        .iter()
        .filter(|report| report.status == "walked")
        .count();
    let editable_leaves = reports
        .iter()
        .flat_map(|report| &report.leaves)
        .filter(|leaf| leaf.editable)
        .count();
    let status = if walked_exports == 0 {
        "unsupported"
    } else if walked_exports == reports.len() {
        "walked"
    } else {
        "partial"
    };

    let source = carrier
        .source_paths()
        .expect("a loaded package always retains canonical source paths");
    let seal = PackagePairSeal::capture(&carrier);
    let document = json!({
        "format": 1,
        "status": status,
        "summary": {
            "exports": reports.len(),
            "walked_exports": walked_exports,
            "editable_leaves": editable_leaves,
        },
        "selector_format": {
            "format": FIXED_LEAF_SELECTOR_FORMAT,
            "profile": FIXED_LEAF_SELECTOR_PROFILE,
        },
        "binding": {
            "package_seal": seal,
            "usmap_sha256": encode_hex(&usmap.sha256),
        },
        "input": {
            "uasset": source.uasset().display().to_string(),
            "uexp": source.uexp().display().to_string(),
            "uasset_length": carrier.len(PackageComponent::Uasset),
            "uexp_length": carrier.len(PackageComponent::Uexp),
        },
        "usmap": {
            "path": usmap.path.display().to_string(),
            "length": usmap.bytes.len(),
            "sha256": encode_hex(&usmap.sha256),
        },
        "exports": reports,
    });

    if args.json {
        println!("{}", serde_json::to_string_pretty(&document)?);
    } else {
        print_inspect_text(&document)?;
    }
    Ok(())
}

fn leaf_report(index: usize, descriptor: FixedLeafDescriptor) -> LeafReport {
    LeafReport {
        index,
        semantic_path: semantic_path(&descriptor.selector.path),
        editable: descriptor.editable,
        selector: descriptor.selector,
    }
}

fn print_inspect_text(document: &serde_json::Value) -> Result<()> {
    let binding = &document["binding"];
    println!(
        "SUMMARY\tstatus={}\texports={}\twalked_exports={}\teditable_leaves={}",
        document["status"].as_str().unwrap_or("unknown"),
        document["summary"]["exports"],
        document["summary"]["walked_exports"],
        document["summary"]["editable_leaves"],
    );
    println!(
        "BINDING\tprofile={}\tformat={}\tuasset_sha256={}\tuexp_sha256={}\tusmap_sha256={}",
        FIXED_LEAF_SELECTOR_PROFILE,
        FIXED_LEAF_SELECTOR_FORMAT,
        binding["package_seal"]["uasset_sha256"]
            .as_str()
            .context("serializing uasset seal")?,
        binding["package_seal"]["uexp_sha256"]
            .as_str()
            .context("serializing uexp seal")?,
        binding["usmap_sha256"]
            .as_str()
            .context("serializing USMAP seal")?,
    );
    for export in document["exports"]
        .as_array()
        .context("serializing exports")?
    {
        println!(
            "EXPORT\tindex={}\tobject={}\tclass={}\tcomponent={}\toffset={}\tlength={}\tstatus={}\terror={}",
            export["index"],
            serde_json::to_string(&export["object_name"] )?,
            serde_json::to_string(&export["class_path"] )?,
            export["component"].as_str().unwrap_or("unknown"),
            export["offset"],
            export["length"],
            export["status"].as_str().unwrap_or("unknown"),
            serde_json::to_string(&export["error"] )?,
        );
        if let Some(leaves) = export["leaves"].as_array() {
            for leaf in leaves {
                println!(
                    "LEAF\texport={}\tindex={}\tpath={}\tkind={}\texpected_hex={}\teditable={}\tselector={}",
                    export["index"],
                    leaf["index"],
                    serde_json::to_string(&leaf["semantic_path"] )?,
                    leaf["selector"]["kind"].as_str().unwrap_or("unknown"),
                    leaf["selector"]["expected_hex"].as_str().unwrap_or(""),
                    leaf["editable"],
                    serde_json::to_string(&leaf["selector"] )?,
                );
            }
        }
    }
    Ok(())
}

fn patch_fixed(args: PatchFixedArgs) -> Result<()> {
    let extract_receipt_input = read_verified_bounded(
        &args.extract_receipt,
        MAX_RECEIPT_BYTES,
        "ASSET_EXTRACT_RECEIPT",
    )?;
    let extract_receipt: ExtractReceiptEnvelope =
        serde_json::from_slice(&extract_receipt_input.bytes)
            .context("ASSET_EXTRACT_RECEIPT: invalid receipt JSON")?;
    let extract_binding =
        validate_extract_receipt_envelope(&extract_receipt, "ASSET_EXTRACT_RECEIPT")?;
    if extract_receipt_input.path
        != canonical_leaf_path(
            &extract_binding.output_root.join(EXTRACT_RECEIPT_NAME),
            "ASSET_EXTRACT_RECEIPT",
        )?
    {
        bail!("ASSET_EXTRACT_RECEIPT: receipt path disagrees with extract output proof");
    }

    let selector_input =
        read_verified_bounded(&args.selector, MAX_SELECTOR_BYTES, "ASSET_SELECTOR")?;
    let selector = parse_selector_document(&selector_input.bytes)?;
    let expected = decode_cli_hex(&args.expected_hex, selector.kind.width(), "ASSET_EXPECTED")?;
    let replacement = decode_cli_hex(
        &args.replacement_hex,
        selector.kind.width(),
        "ASSET_REPLACEMENT",
    )?;
    let expected_hex = encode_hex(&expected);
    if expected_hex != selector.expected_hex {
        bail!(
            "ASSET_EXPECTED: explicit expected bytes {expected_hex} do not match selector bytes {}",
            selector.expected_hex
        );
    }

    let usmap = read_verified_bounded(&args.usmap, MAX_USMAP_BYTES, "ASSET_USMAP")?;
    let usmap_seal = FileSeal {
        path: usmap.path.clone(),
        length: u64::try_from(usmap.bytes.len())?,
        sha256: usmap.sha256,
    };
    if !extract_receipt.generation.usmap.matches(&usmap_seal) {
        bail!("ASSET_GENERATION_MISMATCH: supplied USMAP differs from extract generation");
    }
    let schemas = SchemaDb::from_usmap(&usmap.bytes).context("ASSET_USMAP")?;
    let mut carrier =
        PackageCarrier::load(&args.uasset, asset_package_limits()).context("ASSET_INPUT")?;
    let input_package_seal = PackagePairSeal::capture(&carrier);
    if input_package_seal != extract_receipt.package_seal {
        bail!("ASSET_GENERATION_MISMATCH: input package pair differs from extract receipt");
    }
    let input_sidecar_seals =
        validate_extract_receipt_components(&extract_binding, &carrier, &usmap_seal)?;

    let patch_receipt_path = patch_receipt_path(&args.out)?;
    ensure_path_absent(&patch_receipt_path, "ASSET_PATCH_RECEIPT")?;
    ensure_path_absent(&args.out, "ASSET_OUTPUT")?;
    ensure_path_absent(&args.out.with_extension("uexp"), "ASSET_OUTPUT")?;
    let mut output_sidecar_paths = Vec::new();
    for role in SidecarRole::ALL {
        let (_, path) = sidecar_path(&args.out, role, "ASSET_OUTPUT")?;
        ensure_path_absent(&path, "ASSET_OUTPUT")?;
        output_sidecar_paths.push((role, path));
    }

    let patch = {
        let package = LegacyPackageEnvelope::parse_g1r_ue5_4(&carrier).context("ASSET_ENVELOPE")?;
        let export = package
            .export(selector.export_index)
            .context("ASSET_EXPORT")?;
        let schema_id = export
            .boundary()
            .resolve_class_schema(&schemas)
            .context("ASSET_SCHEMA")?;
        let block = PropertySpanWalker::g1r_ue5_4(&schemas)
            .walk(export.bytes(), schema_id)
            .context("ASSET_WALK")?;
        let leaf = selector
            .resolve(&carrier, &export, &schemas)
            .context("ASSET_SELECTOR")?;
        FixedLeafPatch::plan(
            &carrier,
            &export,
            &schemas,
            &block,
            &leaf,
            &expected,
            &replacement,
        )
        .context("ASSET_REPLACEMENT")?
    };

    let patch_receipt = patch.apply(&mut carrier, &schemas).context("ASSET_DRIFT")?;
    let output_package_seal = PackagePairSeal::capture(&carrier);
    let mut output_guard = PatchOutputGuard::new();
    let mut output_sidecars = Vec::new();
    let mut output_sidecar_seals = Vec::new();
    for ((role, target), source_seal) in output_sidecar_paths
        .iter()
        .filter(|(role, _)| {
            extract_binding
                .sidecars
                .iter()
                .any(|item| item.role == *role)
        })
        .zip(&input_sidecar_seals)
    {
        let (_, source) = sidecar_path(&args.uasset, *role, "ASSET_OUTPUT")?;
        let copied = copy_optional_verified_file(
            &source,
            target,
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_OUTPUT",
        )?
        .context("ASSET_OUTPUT: expected extracted sidecar disappeared while copying")?;
        let PublishedCopy {
            source_seal: copied_source,
            target_seal,
            ownership,
        } = copied;
        output_guard.adopt(ownership);
        if copied_source.length != source_seal.length || copied_source.sha256 != source_seal.sha256
        {
            bail!("ASSET_OUTPUT: extracted sidecar changed after provenance validation");
        }
        let file_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .context("ASSET_OUTPUT: non-UTF-8 sidecar filename")?
            .to_owned();
        output_sidecars.push(SidecarReceipt {
            role: *role,
            file_name,
            length: target_seal.length,
            sha256: encode_hex(&target_seal.sha256),
        });
        output_sidecar_seals.push(target_seal);
    }
    validate_sidecar_generation_mapping(
        &output_sidecars,
        &extract_receipt.generation,
        "ASSET_OUTPUT",
    )?;
    for seal in &output_sidecar_seals {
        reverify_file_seal(seal, MAX_OPTIONAL_SIDECAR_BYTES, "ASSET_OUTPUT")?;
    }
    // Publish sidecars first. The pair writer publishes `.uexp` and then the
    // `.uasset` commit marker, so a process interruption can leave only orphan
    // payloads, never a visible `.uasset` that claims a complete sidecar set.
    let write_receipt = carrier.write_new(&args.out).context("ASSET_OUTPUT")?;
    // Cleanup walks in reverse, so register the commit marker last.
    output_guard.own(write_receipt.uexp.path.clone());
    output_guard.own(write_receipt.uasset.path.clone());
    let copied_usmap_anchor = GenerationFileAnchor {
        file_name: extract_binding.copied_usmap.relative_path.clone(),
        length: extract_binding.copied_usmap.length,
        sha256: extract_binding.copied_usmap.sha256.clone(),
    };

    let replacement_hex = encode_hex(&replacement);
    let result = PatchReceiptEnvelope {
        format: "gore.asset.patch-fixed.v2".to_owned(),
        status: "patched".to_owned(),
        asset: extract_receipt.asset,
        generation_bound: true,
        provenance: PatchReceiptProvenance {
            extract_receipt: ReceiptFileSeal {
                path: extract_receipt_input.path.display().to_string(),
                length: u64::try_from(extract_receipt_input.bytes.len())?,
                sha256: encode_hex(&extract_receipt_input.sha256),
            },
            generation: extract_receipt.generation,
            usmap: copied_usmap_anchor,
            extract_components: extract_binding.components,
            extracted_sidecars: extract_binding.sidecars,
        },
        input_package_seal,
        output_package_seal,
        output_sidecars: output_sidecars.clone(),
        input_selector: selector,
        output_requires_reinspect: true,
        expected_hex: expected_hex.clone(),
        replacement_hex: replacement_hex.clone(),
        patch: PatchOperationProof {
            before: patch_receipt.before.clone(),
            after: patch_receipt.after.clone(),
            export_index: patch_receipt.export_index,
            component: patch_receipt.component,
            absolute_offset: patch_receipt.absolute_offset,
            length: patch_receipt.length,
            kind: patch_receipt.kind,
        },
        output: PatchReceiptOutput {
            uasset: component_digest_proof(&write_receipt.uasset),
            uexp: component_digest_proof(&write_receipt.uexp),
            sidecars: output_sidecars,
            receipt: patch_receipt_path.display().to_string(),
        },
    };
    validate_patch_receipt_envelope(&result, &patch_receipt_path)?;
    let printable = serde_json::to_string_pretty(&result)?;
    let receipt_ownership = write_owned_new_synced(
        &patch_receipt_path,
        printable.as_bytes(),
        "ASSET_PATCH_RECEIPT",
    )?;
    output_guard.adopt(receipt_ownership);
    output_guard.disarm();
    if args.json {
        println!("{printable}");
    } else {
        println!(
            "PATCHED\texport={}\tkind={}\texpected_hex={}\treplacement_hex={}\tcomponent={}\toffset={}\tlength={}",
            patch_receipt.export_index,
            kind_name(patch_receipt.kind),
            expected_hex,
            replacement_hex,
            patch_receipt.component,
            patch_receipt.absolute_offset,
            patch_receipt.length,
        );
        for component in [&write_receipt.uexp, &write_receipt.uasset] {
            println!(
                "WROTE\tpath={}\tlength={}\tsha256={}",
                serde_json::to_string(&component.path.display().to_string())?,
                component.length,
                encode_hex(&component.sha256),
            );
        }
        println!(
            "RECEIPT\tpath={}",
            serde_json::to_string(&patch_receipt_path.display().to_string())?
        );
        println!("NOTICE\toutput_requires_reinspect=true");
    }
    Ok(())
}

fn component_digest_proof(digest: &gore_asset::ComponentDigest) -> ComponentDigestProof {
    ComponentDigestProof {
        path: digest.path.display().to_string(),
        length: digest.length,
        sha256: encode_hex(&digest.sha256),
    }
}

fn patch_receipt_path(output_uasset: &Path) -> Result<PathBuf> {
    let absolute = absolute_without_parent_components(output_uasset, "ASSET_PATCH_RECEIPT")?;
    if absolute.extension().and_then(|value| value.to_str()) != Some("uasset") {
        bail!("ASSET_PATCH_RECEIPT: --out must end in lowercase .uasset");
    }
    let parent = absolute
        .parent()
        .context("ASSET_PATCH_RECEIPT: output has no parent")?;
    let parent = validate_existing_path_no_reparse(parent, true, "ASSET_PATCH_RECEIPT")?;
    let stem = absolute
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .context("ASSET_PATCH_RECEIPT: output has no UTF-8 file stem")?;
    let name = format!("{stem}{PATCH_RECEIPT_SUFFIX}");
    validate_output_component(&name, "ASSET_PATCH_RECEIPT")?;
    Ok(parent.join(name))
}

fn validate_extract_receipt_components(
    binding: &ValidatedExtractBinding,
    carrier: &PackageCarrier,
    usmap: &FileSeal,
) -> Result<Vec<FileSeal>> {
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
    if binding.copied_usmap.length != usmap.length
        || binding.copied_usmap.sha256 != encode_hex(&usmap.sha256)
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

fn validate_patched_sidecars(
    uasset: &Path,
    expected_sidecars: &[SidecarReceipt],
    pair_bytes: u64,
) -> Result<Vec<FileSeal>> {
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

fn validate_patch_output_against_carrier(
    receipt: &PatchReceiptEnvelope,
    source_uasset: &Path,
    source_uexp: &Path,
    carrier: &PackageCarrier,
) -> Result<()> {
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
    let replacement = decode_cli_hex(
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

fn parse_selector_document(bytes: &[u8]) -> Result<FixedLeafSelector> {
    match serde_json::from_slice::<FixedLeafSelector>(bytes) {
        Ok(selector) => Ok(selector),
        Err(selector_error) => {
            match serde_json::from_slice::<FixedLeafDescriptor>(bytes) {
                Ok(descriptor) => Ok(descriptor.selector),
                Err(descriptor_error) => {
                    match serde_json::from_slice::<InspectLeafDocument>(bytes) {
                    Ok(leaf) => Ok(leaf.selector),
                    Err(leaf_error) => bail!(
                        "ASSET_SELECTOR: expected FixedLeafSelector, FixedLeafDescriptor, or inspect leaf JSON; selector error: {selector_error}; descriptor error: {descriptor_error}; inspect leaf error: {leaf_error}"
                    ),
                }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct InspectLeafDocument {
    selector: FixedLeafSelector,
}

fn decode_cli_hex(value: &str, expected_bytes: usize, code: &'static str) -> Result<Vec<u8>> {
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

fn semantic_path(path: &[FixedLeafSelectorStep]) -> String {
    let mut result = String::new();
    for step in path {
        match step {
            FixedLeafSelectorStep::Property {
                property_name,
                array_index,
                array_dimension,
                ..
            } => {
                result.push('/');
                result.push_str(property_name);
                if *array_dimension > 1 {
                    result.push('[');
                    result.push_str(&array_index.to_string());
                    result.push(']');
                }
            }
            FixedLeafSelectorStep::Struct { name, .. } => {
                result.push_str("/struct:");
                result.push_str(name);
            }
            FixedLeafSelectorStep::Map { .. } => result.push_str("/map"),
            FixedLeafSelectorStep::MapEntryValue { key } => {
                result.push_str("/value:key=");
                result.push_str(&key.sha256[..key.sha256.len().min(12)]);
            }
            FixedLeafSelectorStep::MapEntryKey { key } => {
                result.push_str("/key=");
                result.push_str(&key.sha256[..key.sha256.len().min(12)]);
            }
            FixedLeafSelectorStep::RemovedMapKey { key } => {
                result.push_str("/removed-key=");
                result.push_str(&key.sha256[..key.sha256.len().min(12)]);
            }
        }
    }
    if result.is_empty() {
        "/".to_owned()
    } else {
        result
    }
}

fn kind_name(kind: gore_asset::FixedWireKind) -> &'static str {
    match kind {
        gore_asset::FixedWireKind::Byte => "byte",
        gore_asset::FixedWireKind::Bool => "bool",
        gore_asset::FixedWireKind::Int32 => "int32",
        gore_asset::FixedWireKind::Float32 => "float32",
        gore_asset::FixedWireKind::PackageIndex => "package_index",
        gore_asset::FixedWireKind::FName => "fname",
        gore_asset::FixedWireKind::Float64 => "float64",
        gore_asset::FixedWireKind::UInt64 => "uint64",
        gore_asset::FixedWireKind::UInt32 => "uint32",
        gore_asset::FixedWireKind::UInt16 => "uint16",
        gore_asset::FixedWireKind::Int64 => "int64",
        gore_asset::FixedWireKind::Int16 => "int16",
        gore_asset::FixedWireKind::Int8 => "int8",
        gore_asset::FixedWireKind::LinearColorF32x4 => "linear_color_f32x4",
        gore_asset::FixedWireKind::Vector4F64x4 => "vector4_f64x4",
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_hex_is_full_width_and_canonicalized() {
        assert_eq!(decode_cli_hex("00AaFf", 3, "HEX").unwrap(), [0, 0xaa, 0xff]);
        assert!(decode_cli_hex("0x00", 2, "HEX").is_err());
        assert!(decode_cli_hex("0 00", 2, "HEX").is_err());
        assert!(decode_cli_hex("000", 2, "HEX").is_err());
    }

    #[test]
    fn bounded_reader_rejects_symlinks_or_oversize_and_detects_stable_input() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("selector.json");
        fs::write(&path, b"{}").unwrap();
        let input = read_verified_bounded(&path, 2, "TEST").unwrap();
        assert_eq!(input.bytes, b"{}");
        let expected_sha256: [u8; 32] = Sha256::digest(b"{}").into();
        assert_eq!(input.sha256, expected_sha256);
        assert!(read_verified_bounded(&path, 1, "TEST").is_err());
    }

    #[test]
    fn virtual_asset_and_triplet_names_are_strict_and_deterministic() {
        let mount =
            validate_game_asset_path("/Game/Blueprints/Test/DA_Fixture_01", "TEST_ASSET").unwrap();
        assert_eq!(mount.leaf, "DA_Fixture_01");
        assert_eq!(
            mount.cooked_uasset,
            PathBuf::from("G1R/Content/Blueprints/Test/DA_Fixture_01.uasset")
        );
        for invalid in [
            "Game/Test/DA_X",
            "/Engine/Test/DA_X",
            "/Game/Test/../DA_X",
            "/Game/Test/DA-X",
            "/Game/Test/DA_X.uasset",
            "/Game/Test/",
            "/Game/CON/DA_X",
            "/Game/com1/DA_X",
        ] {
            assert!(validate_game_asset_path(invalid, "TEST_ASSET").is_err());
        }
        assert!(validate_triplet_name("zzz_MyMod_P").is_ok());
        for invalid in ["", "../escape", "a/b", "has.dot", "CON", "LPT9"] {
            assert!(validate_triplet_name(invalid).is_err());
        }
        assert!(windows_reserved_name("con.txt "));
        assert!(windows_reserved_name("LpT1.anything"));
        let too_deep = format!(
            "/Game/{}/Leaf",
            std::iter::repeat_n("Segment", MAX_GAME_ASSET_SEGMENTS)
                .collect::<Vec<_>>()
                .join("/")
        );
        assert!(validate_game_asset_path(&too_deep, "TEST_ASSET").is_err());
        assert!(MAX_STAGING_TREE_DEPTH > MAX_GAME_ASSET_SEGMENTS + 3);
        assert!(MAX_STAGING_TREE_ENTRIES > MAX_GAME_ASSET_SEGMENTS + 8);
    }

    #[test]
    fn inspect_and_patch_limits_are_the_public_asset_limits() {
        let limits = asset_package_limits();
        assert_eq!(limits.max_uasset_bytes, 64 * 1024 * 1024);
        assert_eq!(limits.max_uexp_bytes, 256 * 1024 * 1024);
        assert_eq!(limits.max_total_bytes, 320 * 1024 * 1024);
    }

    #[test]
    fn output_directory_is_absent_outside_game_and_staging_cleans_up() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("Game");
        let outside = temp.path().join("OfflineOutput");
        fs::create_dir_all(game.join("G1R/Content")).unwrap();
        let game = resolve_game_root(&game, "TEST_GAME").unwrap();

        let output = prepare_absent_output_directory(&outside, &game, "TEST_OUTPUT").unwrap();
        assert!(!output.exists());
        assert!(prepare_absent_output_directory(
            &game.join("G1R/Content/Unsafe"),
            &game,
            "TEST_OUTPUT"
        )
        .is_err());

        let parent = output.parent().unwrap();
        let staging_path = {
            let staging = create_staging_directory(parent, "unit", "TEST_OUTPUT").unwrap();
            let path = staging.path.clone();
            assert!(path.is_dir());
            path
        };
        assert!(!staging_path.exists());
    }

    #[test]
    fn exclusive_directory_promotion_never_replaces_a_racing_empty_directory() {
        let temp = tempfile::tempdir().unwrap();
        let staging = temp.path().join("staging");
        let destination = temp.path().join("destination");
        fs::create_dir(&staging).unwrap();
        fs::write(staging.join("owned.txt"), b"ours").unwrap();

        // This represents a destination created after the earlier friendly
        // ensure-absent preflight but before the atomic promotion syscall.
        fs::create_dir(&destination).unwrap();
        assert!(promote_directory_noclobber(&staging, &destination).is_err());
        assert!(staging.join("owned.txt").is_file());
        assert!(destination.is_dir());
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
    }

    #[test]
    fn exclusive_directory_publication_moves_complete_staging_once() {
        let temp = tempfile::tempdir().unwrap();
        let staging = temp.path().join("staging");
        let destination = temp.path().join("destination");
        fs::create_dir(&staging).unwrap();
        write_new_synced(&staging.join("owned.txt"), b"complete", "TEST_PUBLISH").unwrap();
        publish_staged_directory(&staging, &destination, "TEST_PUBLISH").unwrap();
        assert!(!staging.exists());
        assert_eq!(
            fs::read(destination.join("owned.txt")).unwrap(),
            b"complete"
        );
    }

    #[test]
    fn optional_sidecar_copy_is_content_sealed_and_no_clobber() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("Source.ubulk");
        let target = temp.path().join("Target.ubulk");
        fs::write(&source, b"sidecar bytes").unwrap();
        let published = copy_optional_verified_file(&source, &target, 1024, "TEST_SIDECAR")
            .unwrap()
            .unwrap();
        assert_eq!(published.source_seal.length, 13);
        assert_eq!(fs::read(&target).unwrap(), b"sidecar bytes");
        let mut ownership = published.ownership;
        ownership.disarm();
        assert!(copy_optional_verified_file(&source, &target, 1024, "TEST_SIDECAR").is_err());
        assert_eq!(fs::read(&target).unwrap(), b"sidecar bytes");
        assert!(copy_optional_verified_file(
            &temp.path().join("missing.ubulk"),
            &temp.path().join("unused.ubulk"),
            1024,
            "TEST_SIDECAR"
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn post_publish_sidecar_failure_cleans_owned_output_and_retry_succeeds() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("Source.ubulk");
        let target = temp.path().join("Target.ubulk");
        fs::write(&source, b"retryable sidecar").unwrap();

        let error = copy_optional_verified_file_with_post_publish(
            &source,
            &target,
            1024,
            "TEST_SIDECAR",
            || bail!("injected post-publish failure"),
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("injected post-publish failure"));
        assert!(!target.exists());
        assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".gore-sidecar-")
        }));

        let mut published = copy_optional_verified_file(&source, &target, 1024, "TEST_SIDECAR")
            .unwrap()
            .unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"retryable sidecar");
        published.ownership.disarm();
    }

    #[test]
    fn post_publish_receipt_failure_cleans_owned_output_and_retry_succeeds() {
        let temp = tempfile::tempdir().unwrap();
        let receipt = temp.path().join("asset.gore-asset-patch.json");

        let error = write_owned_new_synced_with_post_publish(
            &receipt,
            b"retryable receipt",
            "TEST_RECEIPT",
            || bail!("injected receipt failure"),
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("injected receipt failure"));
        assert!(!receipt.exists());
        assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".gore-receipt-")
        }));

        let mut ownership =
            write_owned_new_synced(&receipt, b"retryable receipt", "TEST_RECEIPT").unwrap();
        assert_eq!(fs::read(&receipt).unwrap(), b"retryable receipt");
        ownership.disarm();
    }

    #[test]
    fn final_publish_gate_rejects_mount_triplet_added_after_probe() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("Game");
        let paks = game.join("G1R/Content/Paks");
        fs::create_dir_all(&paks).unwrap();
        let inventory = capture_game_mount_inventory(&game, "TEST_GATE").unwrap();

        let error = run_final_publish_gate(
            &inventory,
            "TEST_GATE",
            || {
                let probe_result = 42u32;
                fs::write(paks.join("zzz_Racing_P.pak"), b"pak").unwrap();
                fs::write(paks.join("zzz_Racing_P.utoc"), b"utoc").unwrap();
                fs::write(paks.join("zzz_Racing_P.ucas"), b"ucas").unwrap();
                Ok(probe_result)
            },
            |result| {
                assert_eq!(*result, 42);
                Ok(())
            },
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("mount inventory changed"));
    }

    #[test]
    fn held_identity_and_content_seal_detect_late_source_change() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("large.ucas");
        fs::write(&path, b"generation one").unwrap();
        let held = HeldFileIdentity::open(&path, 1024, "TEST_IDENTITY").unwrap();
        let sealed = digest_regular_file_bounded(&path, 1024, "TEST_SEAL").unwrap();
        held.reverify("TEST_IDENTITY").unwrap();
        reverify_file_seal(&sealed, 1024, "TEST_SEAL").unwrap();
        let receipt = held.receipt();
        assert!(receipt.content_hash_omitted);
        assert!(receipt.sha256.is_none());

        fs::write(&path, b"generation two is longer").unwrap();
        assert!(held.reverify("TEST_IDENTITY").is_err());
        assert!(reverify_file_seal(&sealed, 1024, "TEST_SEAL").is_err());
    }

    #[test]
    fn output_parent_symlink_or_reparse_is_refused_when_supported() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("Game");
        let real_parent = temp.path().join("RealParent");
        let link_parent = temp.path().join("LinkedParent");
        fs::create_dir_all(game.join("G1R")).unwrap();
        fs::create_dir(&real_parent).unwrap();
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&real_parent, &link_parent);
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&real_parent, &link_parent);
        #[cfg(not(any(windows, unix)))]
        let linked: std::io::Result<()> = Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "symlinks unsupported on this target",
        ));
        if linked.is_err() {
            return;
        }
        let game = resolve_game_root(&game, "TEST_GAME").unwrap();
        let error =
            prepare_absent_output_directory(&link_parent.join("Output"), &game, "TEST_OUTPUT")
                .unwrap_err();
        assert!(format!("{error:#}").contains("reparse"));
        assert!(!real_parent.join("Output").exists());
    }
}
