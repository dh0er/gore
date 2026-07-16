//! Legacy `gore asset pack` v2 lifecycle with staged, no-clobber publication.
//!
//! This module is deliberately exported as `legacy_offline_pack`: its v2 JSON
//! receipt contains local source/game/output paths and is not the future
//! path-free managed-stage evidence model. The CLI reads the receipt chain,
//! while this module independently rechecks the supplied [`VerifiedPatchReceipt`]
//! and [`VerifiedExtractReceipt`] cross-proofs before granting pack authority.
//! As with the existing authoring pipeline, source, staging, output, and game
//! directories are trusted single-writer boundaries; concurrent hostile
//! same-user path replacement is outside this lifecycle's authority contract.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::dataasset_workflow::{
    asset_package_limits, generation_mismatch_reason, probe_current_generation_receipt,
    read_verified_file_bounded, validate_patch_output_against_carrier, validate_patched_sidecars,
    validate_sidecar_generation_mapping, AssetGenerationReceipt, ReceiptComponent, ReceiptFileSeal,
    SidecarRole, SourceFileReceipt, VerifiedExtractReceipt, VerifiedFileSeal, VerifiedPatchReceipt,
    MAX_CONTAINER_COMPONENT_BYTES, MAX_COOKED_PACKAGE_BYTES, MAX_GAME_ASSET_SEGMENTS,
    MAX_MOUNT_UCAS_BYTES, MAX_MOUNT_UTOC_BYTES, MAX_OPTIONAL_SIDECAR_BYTES,
};
use crate::{PackageCarrier, PackageComponent, PackagePairSeal};

pub const OFFLINE_PACK_RECEIPT_NAME_V1: &str = "gore-asset-pack.json";

const MAX_STAGING_TREE_DEPTH: usize = MAX_GAME_ASSET_SEGMENTS + 32;
const MAX_STAGING_TREE_ENTRIES: usize = MAX_GAME_ASSET_SEGMENTS + 32;
const MAX_PAKS_SCAN_DEPTH: usize = 16;
const MAX_PAKS_SCAN_ENTRIES: usize = 4096;
static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A package pair and sidecars independently bound to one validated PatchReceipt
/// v2. Fields are intentionally private; arbitrary caller bytes cannot be
/// promoted into offline-pack authority.
#[derive(Debug)]
pub struct VerifiedOfflineDataAssetPackageV1 {
    asset: String,
    generation: AssetGenerationReceipt,
    source_uasset: PathBuf,
    source_uexp: PathBuf,
    source_seal: PackagePairSeal,
    uasset: Vec<u8>,
    uexp: Vec<u8>,
    sidecars: Vec<VerifiedOfflineSidecarV1>,
    patch_receipt: ReceiptFileSeal,
    extract_receipt: ReceiptFileSeal,
    input_package_seal: PackagePairSeal,
    patched_package_seal: PackagePairSeal,
}

impl VerifiedOfflineDataAssetPackageV1 {
    pub fn asset(&self) -> &str {
        &self.asset
    }

    pub fn generation(&self) -> &AssetGenerationReceipt {
        &self.generation
    }
}

#[derive(Debug)]
struct VerifiedOfflineSidecarV1 {
    role: SidecarRole,
    source: VerifiedFileSeal,
    bytes: Vec<u8>,
}

/// Prepared path and live-game guard for one offline pack. Construction is
/// read-only: it validates the fixed game tree, captures the mount inventory,
/// and reserves no output path.
#[derive(Debug)]
pub struct OfflineDataAssetPackRequestV1 {
    asset: String,
    mount: AssetMount,
    name: String,
    game_root: PathBuf,
    output: PathBuf,
    output_parent: HeldFileIdentity,
    mount_inventory: GameMountInventory,
    global_utoc: FileSeal,
    global_ucas: FileSeal,
}

impl OfflineDataAssetPackRequestV1 {
    /// Perform the same early argument/output/game preflight as `gore asset
    /// pack`. This intentionally runs before PatchReceipt reads so an existing
    /// output or active mounted mod wins error precedence without touching the
    /// receipt or package.
    pub fn prepare(game_root: &Path, asset: &str, name: &str, output: &Path) -> Result<Self> {
        let mount = validate_game_asset_path(asset, "ASSET_PACK_ASSET")?;
        validate_triplet_name(name)?;
        let game_root = resolve_game_root(game_root, "ASSET_PACK_GAME")?;
        let output = prepare_absent_output_directory(output, &game_root, "ASSET_PACK_OUTPUT")?;
        let output_parent = HeldFileIdentity::open_directory(
            output
                .parent()
                .context("ASSET_PACK_OUTPUT: output has no parent")?,
            "ASSET_PACK_OUTPUT",
        )?;
        let mount_inventory = capture_game_mount_inventory(&game_root, "ASSET_PACK_GAME")?;
        let (global_utoc, global_ucas) = seal_global_script_store(&game_root, "ASSET_PACK_GAME")?;
        Ok(Self {
            asset: asset.to_owned(),
            mount,
            name: name.to_owned(),
            game_root,
            output,
            output_parent,
            mount_inventory,
            global_utoc,
            global_ucas,
        })
    }

    pub fn asset(&self) -> &str {
        &self.asset
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn output(&self) -> &Path {
        &self.output
    }

    fn reverify_output_parent_and_disjoint(&self) -> Result<()> {
        self.output_parent.reverify("ASSET_PACK_OUTPUT")?;
        if self.output.starts_with(&self.game_root) || self.game_root.starts_with(&self.output) {
            bail!("ASSET_PACK_OUTPUT: output and live game tree must remain disjoint");
        }
        Ok(())
    }
}

/// A completely built and strictly reopened package held in an owned sibling
/// staging directory. Publication is a separate consuming operation.
#[derive(Debug)]
pub struct StagedOfflineDataAssetPackV1 {
    request: OfflineDataAssetPackRequestV1,
    proof: OfflinePackPublicationProofV1,
    staging: StagingDirectory,
    build_root: PathBuf,
    build_identity: HeldFileIdentity,
    main_ucas: HeldFileIdentity,
    pack_source_utocs: Vec<FileSeal>,
    output_seals: Vec<FileSeal>,
    receipt_seal: FileSeal,
    receipt: Value,
    printable_receipt: String,
}

#[derive(Debug)]
struct OfflinePackPublicationProofV1 {
    generation: AssetGenerationReceipt,
    source_uasset: PathBuf,
    source_seal: PackagePairSeal,
    sidecars: Vec<VerifiedFileSeal>,
}

/// Successfully published offline output. Publication never deploys it.
#[derive(Debug)]
pub struct PublishedOfflineDataAssetPackV1 {
    output: PathBuf,
    receipt_path: PathBuf,
    receipt: Value,
    printable_receipt: String,
    triplet_seals: Vec<PublishedOfflinePackFileSealV1>,
    receipt_seal: PublishedOfflinePackFileSealV1,
}

#[derive(Debug, Clone)]
pub struct PublishedOfflinePackFileSealV1 {
    path: PathBuf,
    length: u64,
    sha256: [u8; 32],
}

/// Publication distinguishes complete finalization, a harmless staging-cleanup
/// warning after durable promotion, and uncertain parent-directory durability.
/// Both post-rename cases retain full published evidence and stay in the
/// success channel so callers cannot mistake them for safe publication retries.
#[derive(Debug)]
pub enum OfflineDataAssetPackPublicationV1 {
    Published(PublishedOfflineDataAssetPackV1),
    PublishedWithCleanupWarning(OfflinePackPublishedWithCleanupWarningV1),
    PublicationUncertain(OfflinePackPublicationUncertainV1),
}

impl PublishedOfflineDataAssetPackV1 {
    pub fn output(&self) -> &Path {
        &self.output
    }

    pub fn receipt_path(&self) -> &Path {
        &self.receipt_path
    }

    pub fn receipt(&self) -> &Value {
        &self.receipt
    }

    pub fn printable_receipt(&self) -> &str {
        &self.printable_receipt
    }

    pub fn triplet_seals(&self) -> &[PublishedOfflinePackFileSealV1] {
        &self.triplet_seals
    }

    pub fn receipt_seal(&self) -> &PublishedOfflinePackFileSealV1 {
        &self.receipt_seal
    }
}

impl PublishedOfflinePackFileSealV1 {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

#[derive(Debug)]
pub enum OfflinePackPublicationUncertainReasonV1 {
    ParentDurabilitySyncFailed { detail: String },
}

/// A post-promotion parent-durability failure. The output and all expected
/// seals are retained for structured recovery instead of a blind retry.
#[derive(Debug)]
pub struct OfflinePackPublicationUncertainV1 {
    published: PublishedOfflineDataAssetPackV1,
    reason: OfflinePackPublicationUncertainReasonV1,
}

impl OfflinePackPublicationUncertainV1 {
    pub fn published(&self) -> &PublishedOfflineDataAssetPackV1 {
        &self.published
    }

    pub fn reason(&self) -> &OfflinePackPublicationUncertainReasonV1 {
        &self.reason
    }
}

impl OfflinePackPublicationUncertainReasonV1 {
    pub fn detail(&self) -> &str {
        match self {
            Self::ParentDurabilitySyncFailed { detail } => detail,
        }
    }
}

#[derive(Debug)]
pub struct OfflinePackPublishedWithCleanupWarningV1 {
    published: PublishedOfflineDataAssetPackV1,
    detail: String,
}

impl OfflinePackPublishedWithCleanupWarningV1 {
    pub fn published(&self) -> &PublishedOfflineDataAssetPackV1 {
        &self.published
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Bind an already-validated PatchReceipt v2 to the exact cooked package pair
/// and optional sidecars currently present on disk.
pub fn verify_patch_receipted_offline_dataasset_package_v1(
    expected_asset: &str,
    patched_uasset: &Path,
    patch: &VerifiedPatchReceipt,
    extract: &VerifiedExtractReceipt,
) -> Result<VerifiedOfflineDataAssetPackageV1> {
    let receipt = patch.receipt();
    if receipt.asset != expected_asset || receipt.provenance.generation.asset != expected_asset {
        bail!("ASSET_GENERATION_MISMATCH: patch receipt targets a different asset");
    }

    // A PatchReceipt alone is insufficient publication authority. Rebind its
    // exact chained ExtractReceipt seal and every cross-receipt proof here so
    // non-CLI callers cannot bypass the adapter's provenance checks.
    let extract_input = extract.input();
    let chained_seal = &receipt.provenance.extract_receipt;
    // The normal chained reader already opened `chained_seal.path`, but this
    // opaque type can also come from the standalone extract-receipt reader.
    // Requiring canonical no-follow identity here protects direct API callers;
    // comparing against the raw spelling instead would incorrectly reject
    // equivalent Windows paths such as a drive path and its `\\?\` form.
    verify_chained_extract_identity(
        extract_input.path(),
        extract_input.length(),
        extract_input.sha256(),
        chained_seal,
    )?;
    let extract_receipt = extract.receipt();
    let extract_binding = extract.binding();
    if extract_receipt.asset != receipt.asset
        || extract_receipt.generation != receipt.provenance.generation
        || extract_receipt.package_seal != receipt.input_package_seal
        || extract_binding.components() != receipt.provenance.extract_components
        || extract_binding.sidecars() != receipt.provenance.extracted_sidecars
    {
        bail!(
            "ASSET_PATCH_RECEIPT: chained extract asset, generation, package, or component provenance mismatch"
        );
    }
    if receipt.provenance.usmap.file_name != extract_binding.copied_usmap().relative_path
        || receipt.provenance.usmap.length != extract_binding.copied_usmap().length
        || receipt.provenance.usmap.sha256 != extract_binding.copied_usmap().sha256
    {
        bail!("ASSET_PATCH_RECEIPT: copied USMAP provenance mismatch");
    }
    if receipt.output_sidecars.len() != extract_binding.sidecars().len()
        || receipt
            .output_sidecars
            .iter()
            .zip(extract_binding.sidecars())
            .any(|(output, extracted)| {
                output.role != extracted.role
                    || output.length != extracted.length
                    || output.sha256 != extracted.sha256
            })
    {
        bail!("ASSET_PATCH_RECEIPT: patched sidecars differ from extracted provenance");
    }
    validate_sidecar_generation_mapping(
        &receipt.output_sidecars,
        &receipt.provenance.generation,
        "ASSET_PATCH_RECEIPT",
    )?;

    let input_uasset =
        validate_existing_path_no_reparse(patched_uasset, false, "ASSET_PACK_INPUT")?;
    input_uasset
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .context("ASSET_PACK_INPUT: input requires a non-empty UTF-8 file stem")?;
    let limits = asset_package_limits();
    let carrier = PackageCarrier::load(&input_uasset, limits).context("ASSET_PACK_INPUT")?;
    let source_paths = carrier
        .source_paths()
        .context("ASSET_PACK_INPUT: loaded package has no source paths")?;
    let source_uasset = source_paths.uasset().to_path_buf();
    let source_uexp = source_paths.uexp().to_path_buf();
    let source_seal = PackagePairSeal::capture(&carrier);
    if source_seal != receipt.output_package_seal {
        bail!("ASSET_GENERATION_MISMATCH: input package pair differs from patch receipt");
    }
    validate_patch_output_against_carrier(patch, &source_uasset, &source_uexp, &carrier)?;
    let pair_bytes = u64::try_from(carrier.len(PackageComponent::Uasset))?
        .checked_add(u64::try_from(carrier.len(PackageComponent::Uexp))?)
        .context("ASSET_PACK_INPUT: cooked size overflowed")?;
    let sidecar_seals = validate_patched_sidecars(patch, &source_uasset, pair_bytes)?;

    let mut sidecars = Vec::with_capacity(sidecar_seals.len());
    for expected in &receipt.output_sidecars {
        let (_, expected_path) = sidecar_path(&source_uasset, expected.role, "ASSET_PACK_SIDECAR")?;
        let source = sidecar_seals
            .iter()
            .find(|seal| seal.path() == expected_path)
            .with_context(|| {
                format!(
                    "ASSET_PACK_SIDECAR: no verified source for {:?}",
                    expected.role
                )
            })?
            .clone();
        let input = read_verified_file_bounded(
            source.path(),
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_PACK_SIDECAR",
        )?;
        if input.length() != source.length() || input.sha256() != source.sha256() {
            bail!("ASSET_PACK_SIDECAR: sidecar changed while being captured");
        }
        sidecars.push(VerifiedOfflineSidecarV1 {
            role: expected.role,
            source,
            bytes: input.bytes().to_vec(),
        });
    }

    let (uasset, uexp) = carrier.into_bytes();
    let patch_receipt = ReceiptFileSeal {
        path: patch.input().path().display().to_string(),
        length: patch.input().length(),
        sha256: encode_hex(patch.input().sha256()),
    };
    Ok(VerifiedOfflineDataAssetPackageV1 {
        asset: expected_asset.to_owned(),
        generation: receipt.provenance.generation.clone(),
        source_uasset,
        source_uexp,
        source_seal,
        uasset,
        uexp,
        sidecars,
        patch_receipt,
        extract_receipt: receipt.provenance.extract_receipt.clone(),
        input_package_seal: receipt.input_package_seal.clone(),
        patched_package_seal: receipt.output_package_seal.clone(),
    })
}

fn verify_chained_extract_identity(
    input_path: &Path,
    input_length: u64,
    input_sha256: &[u8; 32],
    chained_seal: &ReceiptFileSeal,
) -> Result<()> {
    let chained_path = validate_existing_path_no_reparse(
        Path::new(&chained_seal.path),
        false,
        "ASSET_PATCH_RECEIPT",
    )?;
    if input_path != chained_path
        || input_length != chained_seal.length
        || encode_hex(input_sha256) != chained_seal.sha256
    {
        bail!("ASSET_PATCH_RECEIPT: supplied extract receipt differs from chained receipt seal");
    }
    Ok(())
}

/// Build, structurally reopen, seal, and receipt one offline package under an
/// owned sibling staging directory. This function never publishes or deploys.
pub fn stage_offline_dataasset_pack_v1(
    mut package: VerifiedOfflineDataAssetPackageV1,
    request: OfflineDataAssetPackRequestV1,
) -> Result<StagedOfflineDataAssetPackV1> {
    if package.asset != request.asset || package.generation.asset != request.asset {
        bail!("ASSET_GENERATION_MISMATCH: verified package and request target different assets");
    }
    request.reverify_output_parent_and_disjoint()?;

    let main_utoc =
        gore_tex::paths::main_container(&request.game_root).context("ASSET_PACK_CONTAINER")?;
    let main_ucas = HeldFileIdentity::open(
        &main_utoc.with_extension("ucas"),
        64 * 1024 * 1024 * 1024,
        "ASSET_PACK_CONTAINER",
    )?;
    let current_generation = probe_current_generation_receipt(
        &request.game_root,
        &request.asset,
        &package.generation,
        "ASSET_PACK_GENERATION",
    )
    .context("ASSET_PACK_GENERATION")?;
    if current_generation != package.generation {
        let reason = generation_mismatch_reason(&package.generation, &current_generation);
        bail!(
            "ASSET_GENERATION_MISMATCH: installed target/USMAP/UTOC/global generation changed since extract ({reason}); re-extract and reapply the patch"
        );
    }

    // The generation probe may be long. Re-pin the output parent immediately
    // before creating owned staging so a swapped junction cannot redirect it.
    request.reverify_output_parent_and_disjoint()?;
    ensure_path_absent(&request.output, "ASSET_PACK_OUTPUT")?;

    let parent = request.output_parent.path.as_path();
    let staging = create_staging_directory(parent, "pack", "ASSET_PACK_OUTPUT")?;
    staging.verify_owned("ASSET_PACK_OUTPUT")?;
    let cooked_root = staging.path.join("cooked");
    let build_root = staging.path.join("triplet");
    fs::create_dir(&cooked_root).context("ASSET_PACK_STAGE")?;
    fs::create_dir(&build_root).context("ASSET_PACK_STAGE")?;
    let staged_uasset = cooked_root.join(&request.mount.cooked_uasset);
    let staged_relative_parent = request
        .mount
        .cooked_uasset
        .parent()
        .context("ASSET_PACK_ASSET: cooked asset has no parent")?;
    create_relative_directory_chain(&cooked_root, staged_relative_parent, "ASSET_PACK_STAGE")?;
    write_new_synced(&staged_uasset, &package.uasset, "ASSET_PACK_STAGE")?;
    write_new_synced(
        &staged_uasset.with_extension("uexp"),
        &package.uexp,
        "ASSET_PACK_STAGE",
    )?;

    let limits = asset_package_limits();
    let mut staged_input_seals = vec![
        digest_regular_file_bounded(&staged_uasset, limits.max_uasset_bytes, "ASSET_PACK_STAGE")?,
        digest_regular_file_bounded(
            &staged_uasset.with_extension("uexp"),
            limits.max_uexp_bytes,
            "ASSET_PACK_STAGE",
        )?,
    ];
    let mut input_components = vec![
        packed_input_receipt(
            &package.source_uasset,
            &request.mount.cooked_uasset,
            u64::try_from(package.uasset.len())?,
            &package.source_seal.uasset_sha256,
        ),
        packed_input_receipt(
            &package.source_uexp,
            &request.mount.cooked_uasset.with_extension("uexp"),
            u64::try_from(package.uexp.len())?,
            &package.source_seal.uexp_sha256,
        ),
    ];
    let mut cooked_total = u64::try_from(package.uasset.len())?
        .checked_add(u64::try_from(package.uexp.len())?)
        .context("ASSET_PACK_INPUT: cooked size overflowed")?;
    let mut expected_sidecars = [false; 3];
    for sidecar in &package.sidecars {
        let (target_relative_name, target) =
            sidecar_path(&staged_uasset, sidecar.role, "ASSET_PACK_SIDECAR")?;
        write_new_synced(&target, &sidecar.bytes, "ASSET_PACK_SIDECAR")?;
        let target_seal =
            digest_regular_file_bounded(&target, MAX_OPTIONAL_SIDECAR_BYTES, "ASSET_PACK_STAGE")?;
        if target_seal.length != sidecar.source.length()
            || &target_seal.sha256 != sidecar.source.sha256()
        {
            bail!("ASSET_PACK_SIDECAR: staged sidecar differs from verified source");
        }
        cooked_total = cooked_total
            .checked_add(target_seal.length)
            .context("ASSET_PACK_SIDECAR: cooked size overflowed")?;
        if cooked_total > MAX_COOKED_PACKAGE_BYTES {
            bail!(
                "ASSET_PACK_SIDECAR: cooked package is {cooked_total} bytes; aggregate limit is {MAX_COOKED_PACKAGE_BYTES}"
            );
        }
        input_components.push(packed_input_receipt(
            sidecar.source.path(),
            &request
                .mount
                .cooked_uasset
                .with_file_name(target_relative_name),
            sidecar.source.length(),
            sidecar.source.sha256(),
        ));
        expected_sidecars[sidecar.role.index()] = true;
        staged_input_seals.push(target_seal);
    }

    // Staging now owns sealed copies. Release the potentially large package
    // buffers before conversion; the staged value retains only source seals
    // needed at the publication boundary.
    drop(std::mem::take(&mut package.uasset));
    drop(std::mem::take(&mut package.uexp));
    for sidecar in &mut package.sidecars {
        drop(std::mem::take(&mut sidecar.bytes));
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
            MAX_OPTIONAL_SIDECAR_BYTES.max(limits.max_uexp_bytes),
            "ASSET_PACK_STAGE",
        )?;
    }

    let repacked = gore_tex::container::repack_to_zen_verified(
        &cooked_root,
        &request.name,
        &build_root,
        &request.game_root,
        false,
    )
    .context("ASSET_PACK_CONVERT")?;
    let triplet = repacked.triplet;
    let mut pack_source_paths = std::collections::BTreeSet::new();
    pack_source_paths.extend(repacked.metadata_utocs.iter().cloned());
    for chunk in &repacked.source_chunks {
        pack_source_paths.insert(chunk.source_utoc.clone());
    }
    let mut pack_source_utocs = Vec::with_capacity(pack_source_paths.len());
    for source_utoc in pack_source_paths {
        pack_source_utocs.push(digest_regular_file_bounded(
            &source_utoc,
            MAX_CONTAINER_COMPONENT_BYTES,
            "ASSET_PACK_GAME",
        )?);
    }
    let pack_source_receipts: Vec<_> = pack_source_utocs.iter().map(source_file_receipt).collect();

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
            MAX_OPTIONAL_SIDECAR_BYTES.max(limits.max_uexp_bytes),
            "ASSET_PACK_STAGE",
        )?;
    }
    let reopened = gore_tex::container::verify_single_package_triplet(
        &triplet[0],
        &triplet[2],
        &request.asset,
        gore_tex::container::ExpectedSidecars {
            bulk: expected_sidecars[0],
            optional_bulk: expected_sidecars[1],
            memory_mapped_bulk: expected_sidecars[2],
        },
    )
    .context("ASSET_PACK_REOPEN")?;

    let mut triplet_receipts = Vec::with_capacity(3);
    let mut output_seals = Vec::with_capacity(3);
    for path in &triplet {
        sync_existing_regular_file(path, "ASSET_PACK_OUTPUT")?;
        let seal =
            digest_regular_file_bounded(path, MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_OUTPUT")?;
        let relative = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("ASSET_PACK_OUTPUT: non-UTF-8 triplet filename")?;
        triplet_receipts.push(component_receipt(relative, seal.length, &seal.sha256));
        output_seals.push(seal);
    }
    validate_flat_triplet_directory(&build_root, &request.name)?;

    let receipt = json!({
        "format": "gore.asset.pack.v2",
        "status": "packed",
        "asset": request.asset,
        "name": request.name,
        "generation_bound": true,
        "provenance": {
            "patch_receipt": package.patch_receipt,
            "extract_receipt": package.extract_receipt,
            "generation": current_generation,
            "input_package_seal": package.input_package_seal,
            "patched_package_seal": package.patched_package_seal,
        },
        "source": {
            "game_root": request.game_root.display().to_string(),
            "consumed_chunks": repacked.source_chunks,
            "source_container_tocs": pack_source_receipts,
            "content_binding": "script-object and container-header chunks were verified against the winning containers' TOC BLAKE3 hashes before conversion",
            "global_script_store": {
                "utoc": source_file_receipt(&request.global_utoc),
                "ucas": source_file_receipt(&request.global_ucas),
            },
        },
        "input": {
            "package_seal": package.source_seal,
            "components": input_components,
        },
        "output": {
            "root": request.output.display().to_string(),
            "receipt": OFFLINE_PACK_RECEIPT_NAME_V1,
            "triplet": triplet_receipts,
            "reopened_packages": [reopened.package.clone()],
            "strict_reopen": reopened,
            "compressed": false,
        },
        "deployed": false,
    });
    let printable_receipt = serde_json::to_string_pretty(&receipt)?;
    let receipt_path = build_root.join(OFFLINE_PACK_RECEIPT_NAME_V1);
    write_new_synced(
        &receipt_path,
        printable_receipt.as_bytes(),
        "ASSET_PACK_RECEIPT",
    )?;
    let receipt_seal = digest_regular_file_bounded(
        &receipt_path,
        crate::dataasset_workflow::MAX_RECEIPT_BYTES,
        "ASSET_PACK_RECEIPT",
    )?;
    validate_flat_build_directory(&build_root, &request.name)?;

    // Cooked input is private staging-only data. Remove it before the final
    // publication gate so promotion leaves only an empty owned staging parent.
    staging.verify_owned("ASSET_PACK_OUTPUT")?;
    validate_tree_no_reparse(
        &cooked_root,
        MAX_STAGING_TREE_DEPTH,
        MAX_STAGING_TREE_ENTRIES,
        "ASSET_PACK_STAGE",
    )?;
    remove_tree_bounded_no_follow(
        &cooked_root,
        MAX_STAGING_TREE_DEPTH,
        MAX_STAGING_TREE_ENTRIES,
        "ASSET_PACK_STAGE",
    )?;
    staging.verify_owned("ASSET_PACK_OUTPUT")?;
    let build_identity = HeldFileIdentity::open_directory(&build_root, "ASSET_PACK_OUTPUT")?;
    let proof = OfflinePackPublicationProofV1 {
        generation: package.generation,
        source_uasset: package.source_uasset,
        source_seal: package.source_seal,
        sidecars: package
            .sidecars
            .into_iter()
            .map(|sidecar| sidecar.source)
            .collect(),
    };

    Ok(StagedOfflineDataAssetPackV1 {
        request,
        proof,
        staging,
        build_root,
        build_identity,
        main_ucas,
        pack_source_utocs,
        output_seals,
        receipt_seal,
        receipt,
        printable_receipt,
    })
}

impl StagedOfflineDataAssetPackV1 {
    /// Reverify every source and staged output, run the final generation probe
    /// followed by the exact mount-inventory check, then atomically promote the
    /// absent output. No deploy/game/save write occurs.
    pub fn publish_with_bound_receipt_new(mut self) -> Result<OfflineDataAssetPackPublicationV1> {
        self.request
            .global_utoc
            .reverify(MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_GAME")?;
        self.request
            .global_ucas
            .reverify(MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_GAME")?;
        for seal in &self.pack_source_utocs {
            reverify_file_seal(seal, MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_GAME")?;
        }
        self.main_ucas.reverify("ASSET_PACK_GENERATION")?;

        let reloaded = PackageCarrier::load(&self.proof.source_uasset, asset_package_limits())
            .context("ASSET_PACK_INPUT: reverifying source package")?;
        if PackagePairSeal::capture(&reloaded) != self.proof.source_seal {
            bail!("ASSET_PACK_INPUT: source package changed during pack");
        }
        drop(reloaded);
        for sidecar in &self.proof.sidecars {
            sidecar.reverify(MAX_OPTIONAL_SIDECAR_BYTES, "ASSET_PACK_SIDECAR")?;
        }
        for seal in &self.output_seals {
            reverify_file_seal(seal, MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_OUTPUT")?;
        }
        reverify_file_seal(
            &self.receipt_seal,
            crate::dataasset_workflow::MAX_RECEIPT_BYTES,
            "ASSET_PACK_RECEIPT",
        )?;
        self.staging.verify_owned("ASSET_PACK_OUTPUT")?;
        validate_tree_no_reparse(
            &self.staging.path,
            MAX_STAGING_TREE_DEPTH,
            MAX_STAGING_TREE_ENTRIES,
            "ASSET_PACK_OUTPUT",
        )?;
        validate_flat_build_directory(&self.build_root, &self.request.name)?;
        let published_triplet_seals = self
            .output_seals
            .iter()
            .map(|seal| retarget_published_seal(&self.request.output, seal))
            .collect::<Result<Vec<_>>>()?;
        let published_receipt_seal =
            retarget_published_seal(&self.request.output, &self.receipt_seal)?;
        self.request.reverify_output_parent_and_disjoint()?;
        ensure_path_absent(&self.request.output, "ASSET_PACK_OUTPUT")?;
        sync_directory_before_publish(&self.build_root).with_context(|| {
            format!(
                "ASSET_PACK_OUTPUT: syncing staged directory '{}' before publication",
                self.build_root.display()
            )
        })?;
        self.request.reverify_output_parent_and_disjoint()?;
        self.build_identity.reverify("ASSET_PACK_OUTPUT")?;

        // Ordering is a security invariant and is kept in one injected helper:
        // full generation probe+comparison, exact mount inventory, then the
        // atomic no-clobber rename with no intervening fallible operation.
        run_final_gate_and_promote(
            || {
                probe_current_generation_receipt(
                    &self.request.game_root,
                    &self.request.asset,
                    &self.proof.generation,
                    "ASSET_PACK_GENERATION",
                )
            },
            |current| {
                if current != &self.proof.generation {
                    let reason = generation_mismatch_reason(&self.proof.generation, current);
                    bail!(
                        "ASSET_GENERATION_MISMATCH: target generation changed before pack publication ({reason})"
                    );
                }
                Ok(())
            },
            || {
                self.request
                    .mount_inventory
                    .reverify_exact("ASSET_PACK_GAME")
            },
            || {
                promote_directory_noclobber(&self.build_root, &self.request.output).with_context(
                    || {
                        format!(
                            "ASSET_PACK_OUTPUT: publishing staged directory '{}' as '{}'",
                            self.build_root.display(),
                            self.request.output.display()
                        )
                    },
                )
            },
        )?;

        // From here onward the output is published and is never deleted by an
        // error path. Parent durability and cleanup have distinct outcomes.
        let finalization = classify_post_publication(
            sync_parents_after_publish(&self.build_root, &self.request.output)
                .context("syncing publication parents"),
            || self.staging.remove_empty_after_publication(),
        );

        let output = self.request.output.clone();
        let published = PublishedOfflineDataAssetPackV1 {
            receipt_path: output.join(OFFLINE_PACK_RECEIPT_NAME_V1),
            output,
            receipt: self.receipt,
            printable_receipt: self.printable_receipt,
            triplet_seals: published_triplet_seals,
            receipt_seal: published_receipt_seal,
        };
        Ok(finish_publication_outcome(published, finalization))
    }
}

fn finish_publication_outcome(
    published: PublishedOfflineDataAssetPackV1,
    finalization: PostPublicationFinalizationV1,
) -> OfflineDataAssetPackPublicationV1 {
    match finalization {
        PostPublicationFinalizationV1::Complete => {
            OfflineDataAssetPackPublicationV1::Published(published)
        }
        PostPublicationFinalizationV1::CleanupWarning { detail } => {
            OfflineDataAssetPackPublicationV1::PublishedWithCleanupWarning(
                OfflinePackPublishedWithCleanupWarningV1 { published, detail },
            )
        }
        PostPublicationFinalizationV1::PublicationUncertain { reason } => {
            OfflineDataAssetPackPublicationV1::PublicationUncertain(
                OfflinePackPublicationUncertainV1 { published, reason },
            )
        }
    }
}

fn run_final_gate_and_promote<T>(
    probe_generation: impl FnOnce() -> Result<T>,
    compare_generation: impl FnOnce(&T) -> Result<()>,
    verify_mount_inventory: impl FnOnce() -> Result<()>,
    promote_noclobber: impl FnOnce() -> Result<()>,
) -> Result<T> {
    let current = probe_generation()?;
    compare_generation(&current)?;
    verify_mount_inventory()?;
    promote_noclobber()?;
    Ok(current)
}

#[derive(Debug)]
enum PostPublicationFinalizationV1 {
    Complete,
    CleanupWarning {
        detail: String,
    },
    PublicationUncertain {
        reason: OfflinePackPublicationUncertainReasonV1,
    },
}

fn classify_post_publication(
    parent_sync: Result<()>,
    cleanup_staging: impl FnOnce() -> Result<()>,
) -> PostPublicationFinalizationV1 {
    if let Err(error) = parent_sync {
        return PostPublicationFinalizationV1::PublicationUncertain {
            reason: OfflinePackPublicationUncertainReasonV1::ParentDurabilitySyncFailed {
                detail: format!("syncing publication parents: {error:#}"),
            },
        };
    }
    if let Err(error) = cleanup_staging() {
        return PostPublicationFinalizationV1::CleanupWarning {
            detail: format!("removing empty owned staging directory: {error:#}"),
        };
    }
    PostPublicationFinalizationV1::Complete
}

fn retarget_published_seal(
    output: &Path,
    seal: &FileSeal,
) -> Result<PublishedOfflinePackFileSealV1> {
    let file_name = seal
        .path
        .file_name()
        .context("ASSET_PACK_OUTPUT: sealed staged file has no filename")?;
    Ok(PublishedOfflinePackFileSealV1 {
        path: output.join(file_name),
        length: seal.length,
        sha256: seal.sha256,
    })
}

#[derive(Debug, Clone)]
struct FileSeal {
    path: PathBuf,
    length: u64,
    sha256: [u8; 32],
}

impl FileSeal {
    fn reverify(&self, limit: u64, code: &'static str) -> Result<()> {
        reverify_file_seal(self, limit, code)
    }
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
    cooked_uasset: PathBuf,
}

#[derive(Debug)]
struct StagingDirectory {
    path: PathBuf,
    identity: HeldFileIdentity,
    armed: bool,
}

impl StagingDirectory {
    fn verify_owned(&self, code: &'static str) -> Result<()> {
        self.identity.reverify(code)
    }

    fn remove_empty_after_publication(&mut self) -> Result<()> {
        self.verify_owned("ASSET_PACK_OUTPUT")?;
        fs::remove_dir(&self.path).with_context(|| {
            format!(
                "ASSET_PACK_OUTPUT: removing empty staging root '{}'",
                self.path.display()
            )
        })?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if self.armed
            && self.verify_owned("ASSET_STAGE_CLEANUP").is_ok()
            && remove_tree_bounded_no_follow(
                &self.path,
                MAX_STAGING_TREE_DEPTH,
                4096,
                "ASSET_STAGE_CLEANUP",
            )
            .is_ok()
        {
            self.armed = false;
        }
    }
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
    let mut cooked_uasset = PathBuf::from("G1R");
    cooked_uasset.push("Content");
    for segment in segments {
        cooked_uasset.push(segment);
    }
    cooked_uasset.set_extension("uasset");
    Ok(AssetMount { cooked_uasset })
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
    if output.starts_with(&canonical_game) || canonical_game.starts_with(&output) {
        bail!(
            "{code}: output and live game tree must be disjoint: {}",
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

/// Remove a bounded tree without any recursive filesystem primitive. Every
/// entry is classified with no-follow metadata, files are removed directly,
/// and directories are removed deepest-first. A concurrent replacement can at
/// worst make the operation fail or remove the replacement entry itself; this
/// function never follows it into another tree.
fn remove_tree_bounded_no_follow(
    root: &Path,
    max_depth: usize,
    max_entries: usize,
    code: &'static str,
) -> Result<()> {
    let root = validate_existing_path_no_reparse(root, true, code)?;
    let mut pending = vec![(root.clone(), 0usize)];
    let mut files = Vec::new();
    let mut directories = vec![(root, 0usize)];
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
                let child_depth = depth.checked_add(1).context("staging depth overflowed")?;
                directories.push((path.clone(), child_depth));
                pending.push((path, child_depth));
            } else if metadata.is_file() {
                files.push(path);
            } else {
                bail!(
                    "{code}: non-regular staging entry is refused: {}",
                    path.display()
                );
            }
        }
    }

    for path in files {
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("{code}: reverifying '{}' before removal", path.display()))?;
        if metadata_is_reparse(&metadata) || !metadata.is_file() {
            bail!(
                "{code}: staging file changed type before removal: {}",
                path.display()
            );
        }
        fs::remove_file(&path)
            .with_context(|| format!("{code}: removing staging file '{}'", path.display()))?;
    }

    directories.sort_by_key(|right| std::cmp::Reverse(right.1));
    for (path, _) in directories {
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("{code}: reverifying '{}' before removal", path.display()))?;
        if metadata_is_reparse(&metadata) || !metadata.is_dir() {
            bail!(
                "{code}: staging directory changed type before removal: {}",
                path.display()
            );
        }
        fs::remove_dir(&path)
            .with_context(|| format!("{code}: removing staging directory '{}'", path.display()))?;
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

fn scan_game_iostore_directory(
    game_root: &Path,
    code: &'static str,
) -> Result<Vec<(String, DirectMountKind, PathBuf)>> {
    let paks = validate_existing_path_no_reparse(&game_root.join("G1R/Content/Paks"), true, code)?;
    let mut pending = vec![(paks, 0usize)];
    let mut entries = 0usize;
    let mut direct_mounts = Vec::new();
    while let Some((directory, depth)) = pending.pop() {
        let checked = validate_existing_path_no_reparse(&directory, true, code)?;
        if checked != directory {
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

fn sidecar_path(uasset: &Path, role: SidecarRole, code: &'static str) -> Result<(String, PathBuf)> {
    let stem = uasset
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .with_context(|| format!("{code}: non-empty UTF-8 package stem required"))?;
    let file_name = format!("{stem}.{}", role.suffix());
    Ok((file_name.clone(), uasset.with_file_name(file_name)))
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

        // SAFETY: `information` is the exact writable Win32 structure and
        // `file` owns a live handle for the duration of the call.
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

fn verify_file_hash(
    path: &Path,
    expected_length: u64,
    expected_sha256: [u8; 32],
    limit: u64,
    code: &'static str,
) -> Result<()> {
    let mut file = File::open(path)
        .with_context(|| format!("{code}: reopening '{}' for verification", path.display()))?;
    let (length, sha256) = digest_reader(&mut file, limit, code)?;
    if length != expected_length || sha256 != expected_sha256 {
        bail!(
            "{code}: file content changed during operation: {}",
            path.display()
        );
    }
    Ok(())
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

fn validate_flat_triplet_directory(directory: &Path, name: &str) -> Result<()> {
    validate_flat_directory(
        directory,
        &[
            format!("{name}.utoc"),
            format!("{name}.ucas"),
            format!("{name}.pak"),
        ],
        "ASSET_PACK_OUTPUT",
    )
}

fn validate_flat_build_directory(directory: &Path, name: &str) -> Result<()> {
    validate_flat_directory(
        directory,
        &[
            format!("{name}.utoc"),
            format!("{name}.ucas"),
            format!("{name}.pak"),
            OFFLINE_PACK_RECEIPT_NAME_V1.to_owned(),
        ],
        "ASSET_PACK_OUTPUT",
    )
}

fn validate_flat_directory(directory: &Path, allowed: &[String], code: &'static str) -> Result<()> {
    let mut found = Vec::new();
    for entry in fs::read_dir(directory).context(code)? {
        let entry = entry.context(code)?;
        let metadata = fs::symlink_metadata(entry.path()).context(code)?;
        let filename = entry.file_name();
        let filename = filename
            .to_str()
            .context("ASSET_PACK_OUTPUT: non-UTF-8 output filename")?;
        if metadata_is_reparse(&metadata)
            || !metadata.is_file()
            || !allowed.iter().any(|candidate| candidate == filename)
        {
            bail!("{code}: unexpected output entry {filename:?}");
        }
        found.push(filename.to_owned());
    }
    found.sort();
    let mut expected = allowed.to_vec();
    expected.sort();
    if found != expected {
        bail!("{code}: incomplete output set; expected {expected:?}, got {found:?}");
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
) -> Value {
    json!({
        "source_path": source_path.display().to_string(),
        "packed_relative_path": packed_relative_path.to_string_lossy().replace('\\', "/"),
        "length": length,
        "sha256": encode_hex(sha256),
    })
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing into String cannot fail");
    }
    encoded
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
fn promote_directory_noclobber(_staged: &Path, _output: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "atomic no-clobber directory promotion is unsupported on this platform",
    ))
}

#[cfg(unix)]
fn sync_directory_before_publish(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory_before_publish(_path: &Path) -> std::io::Result<()> {
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    fn fixture_published(output: PathBuf) -> PublishedOfflineDataAssetPackV1 {
        let triplet_seals = ["fixture.utoc", "fixture.ucas", "fixture.pak"]
            .into_iter()
            .map(|name| PublishedOfflinePackFileSealV1 {
                path: output.join(name),
                length: 1,
                sha256: [7; 32],
            })
            .collect();
        PublishedOfflineDataAssetPackV1 {
            receipt_path: output.join(OFFLINE_PACK_RECEIPT_NAME_V1),
            receipt: json!({"format": "gore.asset.pack.v2"}),
            printable_receipt: "{}".to_owned(),
            receipt_seal: PublishedOfflinePackFileSealV1 {
                path: output.join(OFFLINE_PACK_RECEIPT_NAME_V1),
                length: 2,
                sha256: [8; 32],
            },
            triplet_seals,
            output,
        }
    }

    #[test]
    fn pack_name_validation_matches_cli_contract() {
        validate_triplet_name("zzz_GoreWolfProof_P").unwrap();
        for invalid in ["", "../escape", "has.dot", "CON", "com1", "LPT9"] {
            assert!(
                validate_triplet_name(invalid).is_err(),
                "accepted {invalid:?}"
            );
        }
    }

    #[test]
    fn prepared_request_rejects_existing_output_before_mount_reads() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("Game");
        fs::create_dir_all(game.join("G1R")).unwrap();
        let output = temp.path().join("Existing");
        fs::create_dir(&output).unwrap();
        fs::write(output.join("sentinel"), b"keep").unwrap();
        let error = OfflineDataAssetPackRequestV1::prepare(
            &game,
            "/Game/Test/DA_Fixture",
            "zzz_Test_P",
            &output,
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("ASSET_PACK_OUTPUT"));
        assert_eq!(fs::read(output.join("sentinel")).unwrap(), b"keep");
    }

    #[test]
    fn chained_extract_identity_is_canonical_and_path_bound() {
        let temp = tempfile::tempdir().unwrap();
        let receipt = temp.path().join("gore-asset-extract.json");
        let bytes = b"sealed extract receipt";
        fs::write(&receipt, bytes).unwrap();
        let canonical = validate_existing_path_no_reparse(&receipt, false, "TEST").unwrap();
        let sha256: [u8; 32] = Sha256::digest(bytes).into();

        let equivalent_spelling = receipt
            .parent()
            .unwrap()
            .join(".")
            .join(receipt.file_name().unwrap());
        let equivalent = ReceiptFileSeal {
            path: equivalent_spelling.display().to_string(),
            length: bytes.len() as u64,
            sha256: encode_hex(&sha256),
        };
        verify_chained_extract_identity(&canonical, bytes.len() as u64, &sha256, &equivalent)
            .unwrap();

        let foreign = temp.path().join("foreign-extract.json");
        fs::write(&foreign, bytes).unwrap();
        let foreign_seal = ReceiptFileSeal {
            path: foreign.display().to_string(),
            length: bytes.len() as u64,
            sha256: encode_hex(&sha256),
        };
        let error =
            verify_chained_extract_identity(&canonical, bytes.len() as u64, &sha256, &foreign_seal)
                .unwrap_err();
        assert!(format!("{error:#}").contains("supplied extract receipt differs"));

        let missing_seal = ReceiptFileSeal {
            path: temp
                .path()
                .join("missing-extract.json")
                .display()
                .to_string(),
            length: bytes.len() as u64,
            sha256: encode_hex(&sha256),
        };
        let error =
            verify_chained_extract_identity(&canonical, bytes.len() as u64, &sha256, &missing_seal)
                .unwrap_err();
        assert!(format!("{error:#}").contains("ASSET_PATCH_RECEIPT"));
    }

    #[test]
    fn atomic_directory_promotion_never_clobbers_racer() {
        let temp = tempfile::tempdir().unwrap();
        let staged = temp.path().join("staged");
        let output = temp.path().join("output");
        fs::create_dir(&staged).unwrap();
        fs::write(staged.join("ours"), b"ours").unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(output.join("racer"), b"racer").unwrap();
        assert!(promote_directory_noclobber(&staged, &output).is_err());
        assert_eq!(fs::read(output.join("racer")).unwrap(), b"racer");
        assert_eq!(fs::read(staged.join("ours")).unwrap(), b"ours");
    }

    #[test]
    fn atomic_directory_promotion_refuses_empty_racer_directory() {
        let temp = tempfile::tempdir().unwrap();
        let staged = temp.path().join("staged");
        let output = temp.path().join("output");
        fs::create_dir(&staged).unwrap();
        fs::write(staged.join("ours"), b"ours").unwrap();
        fs::create_dir(&output).unwrap();

        assert!(promote_directory_noclobber(&staged, &output).is_err());
        assert!(output.is_dir());
        assert_eq!(fs::read(staged.join("ours")).unwrap(), b"ours");
    }

    #[test]
    fn final_gate_order_is_generation_compare_mount_then_promote() {
        let order = RefCell::new(Vec::new());
        let current = run_final_gate_and_promote(
            || {
                order.borrow_mut().push("generation");
                Ok(7u8)
            },
            |value| {
                order.borrow_mut().push("compare");
                assert_eq!(*value, 7);
                Ok(())
            },
            || {
                order.borrow_mut().push("mount");
                Ok(())
            },
            || {
                order.borrow_mut().push("promote");
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(current, 7);
        assert_eq!(
            order.into_inner(),
            ["generation", "compare", "mount", "promote"]
        );
    }

    #[test]
    fn final_gate_failure_cannot_reach_promotion() {
        let promoted = Cell::new(false);
        let error = run_final_gate_and_promote(
            || Ok(()),
            |_| Ok(()),
            || bail!("mount inventory changed"),
            || {
                promoted.set(true);
                Ok(())
            },
        )
        .unwrap_err();

        assert!(format!("{error:#}").contains("mount inventory changed"));
        assert!(!promoted.get());
    }

    #[test]
    fn post_publication_failure_is_typed_and_retains_output() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("published");
        fs::create_dir(&output).unwrap();
        fs::write(output.join("sentinel"), b"published").unwrap();

        let finalization =
            classify_post_publication(Err(anyhow::anyhow!("forced parent sync failure")), || {
                panic!("cleanup must not run after parent-sync failure")
            });
        let outcome = finish_publication_outcome(fixture_published(output.clone()), finalization);
        let OfflineDataAssetPackPublicationV1::PublicationUncertain(uncertain) = outcome else {
            panic!("parent-sync failure must be typed as publication uncertain");
        };

        assert_eq!(uncertain.published().output(), output);
        assert_eq!(uncertain.published().triplet_seals().len(), 3);
        assert_eq!(
            uncertain.published().receipt_seal().path(),
            output.join(OFFLINE_PACK_RECEIPT_NAME_V1)
        );
        assert!(uncertain
            .reason()
            .detail()
            .contains("forced parent sync failure"));
        assert_eq!(fs::read(output.join("sentinel")).unwrap(), b"published");
    }

    #[test]
    fn cleanup_failure_is_a_published_warning_not_durability_uncertain() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("published");
        fs::create_dir(&output).unwrap();
        let finalization =
            classify_post_publication(Ok(()), || bail!("forced staging cleanup failure"));

        let outcome = finish_publication_outcome(fixture_published(output.clone()), finalization);
        let OfflineDataAssetPackPublicationV1::PublishedWithCleanupWarning(warning) = outcome
        else {
            panic!("cleanup-only failure must remain a published outcome");
        };
        assert_eq!(warning.published().output(), output);
        assert!(warning.detail().contains("forced staging cleanup failure"));
    }

    #[test]
    fn owned_staging_is_cleaned_after_prepublication_failure() {
        let temp = tempfile::tempdir().unwrap();
        let staging = create_staging_directory(temp.path(), "pack", "TEST").unwrap();
        let staging_path = staging.path.clone();
        fs::create_dir(staging.path.join("nested")).unwrap();
        fs::write(staging.path.join("nested/file"), b"private").unwrap();

        drop(staging);

        assert!(!staging_path.exists());
    }

    #[test]
    fn written_receipt_bytes_remain_bound_to_their_seal() {
        let temp = tempfile::tempdir().unwrap();
        let receipt = temp.path().join(OFFLINE_PACK_RECEIPT_NAME_V1);
        write_new_synced(&receipt, br#"{"format":"gore.asset.pack.v2"}"#, "TEST").unwrap();
        let seal = digest_regular_file_bounded(
            &receipt,
            crate::dataasset_workflow::MAX_RECEIPT_BYTES,
            "TEST",
        )
        .unwrap();

        fs::write(&receipt, br#"{"format":"gore.asset.pack.v1"}"#).unwrap();

        assert!(
            reverify_file_seal(&seal, crate::dataasset_workflow::MAX_RECEIPT_BYTES, "TEST")
                .is_err()
        );
    }
}
