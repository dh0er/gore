//! Crate-private mechanics for building and publishing one offline additive triplet.
//!
//! This module deliberately knows nothing about Patch/Extract receipts, managed Stores,
//! reviewed schemas, deployment, or runtime authority. Callers supply already-authorized bytes,
//! their own receipt bytes, and the external source/live gates that must precede publication.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::{
    create_relative_directory_chain, create_staging_directory, digest_regular_file_bounded,
    ensure_path_absent, promote_directory_noclobber, remove_tree_bounded_no_follow,
    reverify_file_seal, sync_directory_before_publish, sync_existing_regular_file,
    sync_parents_after_publish, validate_flat_directory, validate_tree_no_reparse,
    write_new_synced, FileSeal, HeldFileIdentity, OfflineDataAssetPackRequestV1, StagingDirectory,
    MAX_STAGING_TREE_DEPTH, MAX_STAGING_TREE_ENTRIES,
};
use crate::dataasset_workflow::{
    asset_package_limits, SidecarRole, MAX_CONTAINER_COMPONENT_BYTES, MAX_COOKED_PACKAGE_BYTES,
    MAX_OPTIONAL_SIDECAR_BYTES, MAX_RECEIPT_BYTES,
};

#[derive(Debug)]
pub(crate) enum PackBytes<'a> {
    Borrowed(&'a [u8]),
    Owned(Vec<u8>),
}

impl PackBytes<'_> {
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Borrowed(bytes) => bytes,
            Self::Owned(bytes) => bytes,
        }
    }

    fn release_owned(&mut self) {
        if let Self::Owned(bytes) = std::mem::replace(self, Self::Borrowed(&[])) {
            drop(bytes);
        }
    }
}

#[derive(Debug)]
pub(crate) struct PackSidecarInput<'a> {
    pub(crate) role: SidecarRole,
    pub(crate) source_length: u64,
    pub(crate) source_sha256: [u8; 32],
    pub(crate) bytes: PackBytes<'a>,
}

/// Authority-neutral component views. Legacy callers can transfer owned vectors while managed
/// capsules lend already-verified bytes without cloning. Only [`PackBytes::Owned`] allocations are
/// released after the sealed staging copies exist. [`PackBytes::Borrowed`] keeps the external
/// owner borrowed for this entire staging call; this API does not grant that owner an early-release
/// point.
#[derive(Debug)]
pub(crate) struct PackInput<'a> {
    pub(crate) uasset: PackBytes<'a>,
    pub(crate) uexp: PackBytes<'a>,
    pub(crate) sidecars: Vec<PackSidecarInput<'a>>,
}

impl PackInput<'_> {
    fn release_owned(&mut self) {
        self.uasset.release_owned();
        self.uexp.release_owned();
        for sidecar in &mut self.sidecars {
            sidecar.bytes.release_owned();
        }
    }
}

#[derive(Debug)]
pub(crate) struct MechanicalReceipt<R> {
    name: String,
    bytes: Vec<u8>,
    max_bytes: u64,
    payload: R,
}

impl<R> MechanicalReceipt<R> {
    pub(crate) fn new(
        name: impl Into<String>,
        bytes: Vec<u8>,
        max_bytes: u64,
        payload: R,
    ) -> Result<Self> {
        let max_bytes = validate_receipt_bound(max_bytes)?;
        let receipt_length = u64::try_from(bytes.len())?;
        if receipt_length > max_bytes {
            bail!("ASSET_PACK_RECEIPT: receipt is {receipt_length} bytes; limit is {max_bytes}");
        }
        Ok(Self {
            name: name.into(),
            bytes,
            max_bytes,
            payload,
        })
    }
}

/// Read-only evidence made available while the caller constructs its own receipt. Paths here are
/// ephemeral process evidence; the core never chooses or serializes a provenance schema.
pub(crate) struct MechanicalStageEvidence<'a, G> {
    request: &'a OfflineDataAssetPackRequestV1,
    initial_live_evidence: &'a G,
    source_chunks: &'a [gore_tex::container::VerifiedChunkReceipt],
    source_utoc_seals: &'a [FileSeal],
    output_seals: &'a [FileSeal],
    strict: &'a gore_tex::container::StrictTripletVerification,
}

impl<'a, G> MechanicalStageEvidence<'a, G> {
    pub(crate) fn request(&self) -> &'a OfflineDataAssetPackRequestV1 {
        self.request
    }

    pub(crate) fn initial_live_evidence(&self) -> &'a G {
        self.initial_live_evidence
    }

    pub(crate) fn source_chunks(&self) -> &'a [gore_tex::container::VerifiedChunkReceipt] {
        self.source_chunks
    }

    pub(super) fn source_utoc_seals(&self) -> &'a [FileSeal] {
        self.source_utoc_seals
    }

    pub(super) fn output_seals(&self) -> &'a [FileSeal] {
        self.output_seals
    }

    pub(crate) fn strict(&self) -> &'a gore_tex::container::StrictTripletVerification {
        self.strict
    }
}

#[derive(Debug)]
pub(crate) struct MechanicalStagedPack<R> {
    request: OfflineDataAssetPackRequestV1,
    staging: StagingDirectory,
    build_root: PathBuf,
    build_identity: HeldFileIdentity,
    main_ucas: HeldFileIdentity,
    pack_source_utocs: Vec<FileSeal>,
    output_seals: Vec<FileSeal>,
    receipt_name: String,
    receipt_max_bytes: u64,
    receipt_seal: FileSeal,
    receipt_payload: R,
}

#[derive(Debug)]
pub(crate) struct MechanicalPublishedPack<R> {
    pub(super) output: PathBuf,
    pub(super) receipt_path: PathBuf,
    pub(super) triplet_seals: Vec<FileSeal>,
    pub(super) receipt_seal: FileSeal,
    pub(super) receipt_payload: R,
}

#[derive(Debug)]
pub(crate) enum MechanicalPublication<R> {
    Published(MechanicalPublishedPack<R>),
    PublishedWithCleanupWarning {
        published: MechanicalPublishedPack<R>,
        detail: String,
    },
    PublicationUncertain {
        published: MechanicalPublishedPack<R>,
        detail: String,
    },
}

/// Stage borrowed component views into an owned sibling tree, build and strictly reopen the
/// triplet, then bind caller-produced bounded receipt bytes. The initial live gate runs at the
/// legacy-compatible point: after the main UCAS identity is held and before staging is created.
pub(crate) fn stage_with_receipt<G, R>(
    request: OfflineDataAssetPackRequestV1,
    mut input: PackInput<'_>,
    initial_live_gate: impl FnOnce(&OfflineDataAssetPackRequestV1) -> Result<G>,
    build_receipt: impl FnOnce(&MechanicalStageEvidence<'_, G>) -> Result<MechanicalReceipt<R>>,
) -> Result<MechanicalStagedPack<R>> {
    request.reverify_output_parent_and_disjoint()?;

    let main_utoc =
        gore_tex::paths::main_container(&request.game_root).context("ASSET_PACK_CONTAINER")?;
    let main_ucas = HeldFileIdentity::open(
        &main_utoc.with_extension("ucas"),
        64 * 1024 * 1024 * 1024,
        "ASSET_PACK_CONTAINER",
    )?;
    let initial_live_evidence = initial_live_gate(&request)?;

    // The live gate may be long. Re-pin the output parent immediately before creating staging.
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
    write_new_synced(&staged_uasset, input.uasset.as_slice(), "ASSET_PACK_STAGE")?;
    write_new_synced(
        &staged_uasset.with_extension("uexp"),
        input.uexp.as_slice(),
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
    let mut cooked_total = u64::try_from(input.uasset.as_slice().len())?
        .checked_add(u64::try_from(input.uexp.as_slice().len())?)
        .context("ASSET_PACK_INPUT: cooked size overflowed")?;
    let mut expected_sidecars = [false; 3];
    for sidecar in &input.sidecars {
        let (target_relative_name, target) =
            super::sidecar_path(&staged_uasset, sidecar.role, "ASSET_PACK_SIDECAR")?;
        let _ = target_relative_name;
        let target_seal = write_borrowed_component_new(
            &target,
            sidecar.bytes.as_slice(),
            MAX_OPTIONAL_SIDECAR_BYTES,
            "ASSET_PACK_SIDECAR",
            "ASSET_PACK_STAGE",
        )?;
        if target_seal.length != sidecar.source_length
            || target_seal.sha256 != sidecar.source_sha256
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
        expected_sidecars[sidecar.role.index()] = true;
        staged_input_seals.push(target_seal);
    }

    // Staging now owns sealed copies. Release only transferred `Owned` allocations before Retoc;
    // a `Borrowed` input remains tied to its external owner until this staging call returns.
    input.release_owned();
    drop(input);

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
    let mut pack_source_paths = BTreeSet::new();
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
    let strict = gore_tex::container::verify_single_package_triplet(
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

    let mut output_seals = Vec::with_capacity(3);
    for path in &triplet {
        sync_existing_regular_file(path, "ASSET_PACK_OUTPUT")?;
        output_seals.push(digest_regular_file_bounded(
            path,
            MAX_CONTAINER_COMPONENT_BYTES,
            "ASSET_PACK_OUTPUT",
        )?);
    }
    validate_flat_triplet_directory(&build_root, &request.name)?;

    let evidence = MechanicalStageEvidence {
        request: &request,
        initial_live_evidence: &initial_live_evidence,
        source_chunks: &repacked.source_chunks,
        source_utoc_seals: &pack_source_utocs,
        output_seals: &output_seals,
        strict: &strict,
    };
    let receipt = build_receipt(&evidence)?;
    let receipt_seal = write_bounded_receipt_new(&build_root, &receipt)?;
    let receipt_max_bytes = validate_receipt_bound(receipt.max_bytes)?;
    validate_flat_build_directory(&build_root, &request.name, &receipt.name)?;

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

    Ok(MechanicalStagedPack {
        request,
        staging,
        build_root,
        build_identity,
        main_ucas,
        pack_source_utocs,
        output_seals,
        receipt_name: receipt.name,
        receipt_max_bytes,
        receipt_seal,
        receipt_payload: receipt.payload,
    })
}

fn write_borrowed_component_new(
    path: &Path,
    bytes: &[u8],
    limit: u64,
    write_code: &'static str,
    seal_code: &'static str,
) -> Result<FileSeal> {
    write_new_synced(path, bytes, write_code)?;
    digest_regular_file_bounded(path, limit, seal_code)
}

fn write_bounded_receipt_new<R>(
    build_root: &Path,
    receipt: &MechanicalReceipt<R>,
) -> Result<FileSeal> {
    let max_bytes = validate_receipt_bound(receipt.max_bytes)?;
    let receipt_length = u64::try_from(receipt.bytes.len())?;
    if receipt_length > max_bytes {
        bail!(
            "ASSET_PACK_RECEIPT: receipt is {receipt_length} bytes; limit is {}",
            max_bytes
        );
    }
    super::validate_output_component(&receipt.name, "ASSET_PACK_RECEIPT")?;
    let receipt_path = build_root.join(&receipt.name);
    write_new_synced(&receipt_path, &receipt.bytes, "ASSET_PACK_RECEIPT")?;
    digest_regular_file_bounded(&receipt_path, max_bytes, "ASSET_PACK_RECEIPT")
}

fn validate_receipt_bound(max_bytes: u64) -> Result<u64> {
    if max_bytes == 0 || max_bytes > MAX_RECEIPT_BYTES {
        bail!("ASSET_PACK_RECEIPT: receipt limit must be between 1 and {MAX_RECEIPT_BYTES} bytes");
    }
    Ok(max_bytes)
}

impl<R> MechanicalStagedPack<R> {
    /// Reverify core-held sources and staged output, invoke the caller's external source proof,
    /// then close the publication races around a potentially long final source gate. The final
    /// live gate is deliberately separate and remains the last non-core check before the exact
    /// mount inventory check and atomic rename.
    pub(crate) fn publish_with_hooks(
        mut self,
        verify_external_sources: impl FnOnce() -> Result<()>,
        final_source_gate: impl FnOnce(&OfflineDataAssetPackRequestV1) -> Result<()>,
        final_live_gate: impl FnOnce(&OfflineDataAssetPackRequestV1) -> Result<()>,
    ) -> Result<MechanicalPublication<R>> {
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
        verify_external_sources()?;

        for seal in &self.output_seals {
            reverify_file_seal(seal, MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_OUTPUT")?;
        }
        let receipt_max_bytes = validate_receipt_bound(self.receipt_max_bytes)?;
        reverify_file_seal(&self.receipt_seal, receipt_max_bytes, "ASSET_PACK_RECEIPT")?;
        self.staging.verify_owned("ASSET_PACK_OUTPUT")?;
        validate_tree_no_reparse(
            &self.staging.path,
            MAX_STAGING_TREE_DEPTH,
            MAX_STAGING_TREE_ENTRIES,
            "ASSET_PACK_OUTPUT",
        )?;
        validate_flat_build_directory(&self.build_root, &self.request.name, &self.receipt_name)?;
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

        run_publication_tail(
            || final_source_gate(&self.request),
            || {
                // The caller's final source gate may perform arbitrarily slow Store/CAS work.
                // Re-pin every publication-side authority after it returns rather than trusting
                // any check that preceded the gate.
                self.request.reverify_output_parent_and_disjoint()?;
                ensure_path_absent(&self.request.output, "ASSET_PACK_OUTPUT")?;
                self.staging.verify_owned("ASSET_PACK_OUTPUT")?;
                self.build_identity.reverify("ASSET_PACK_OUTPUT")?;
                validate_tree_no_reparse(
                    &self.staging.path,
                    MAX_STAGING_TREE_DEPTH,
                    MAX_STAGING_TREE_ENTRIES,
                    "ASSET_PACK_OUTPUT",
                )?;
                validate_flat_build_directory(
                    &self.build_root,
                    &self.request.name,
                    &self.receipt_name,
                )?;
                for seal in &self.output_seals {
                    reverify_file_seal(seal, MAX_CONTAINER_COMPONENT_BYTES, "ASSET_PACK_OUTPUT")?;
                }
                reverify_file_seal(&self.receipt_seal, receipt_max_bytes, "ASSET_PACK_RECEIPT")?;
                Ok(())
            },
            || final_live_gate(&self.request),
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

        let finalization = classify_post_publication(
            sync_parents_after_publish(&self.build_root, &self.request.output)
                .context("syncing publication parents"),
            || self.staging.remove_empty_after_publication(),
        );
        let output = self.request.output.clone();
        let published = MechanicalPublishedPack {
            receipt_path: output.join(&self.receipt_name),
            output,
            triplet_seals: published_triplet_seals,
            receipt_seal: published_receipt_seal,
            receipt_payload: self.receipt_payload,
        };
        Ok(match finalization {
            PostPublicationFinalization::Complete => MechanicalPublication::Published(published),
            PostPublicationFinalization::CleanupWarning { detail } => {
                MechanicalPublication::PublishedWithCleanupWarning { published, detail }
            }
            PostPublicationFinalization::PublicationUncertain { detail } => {
                MechanicalPublication::PublicationUncertain { published, detail }
            }
        })
    }
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

fn validate_flat_build_directory(directory: &Path, name: &str, receipt_name: &str) -> Result<()> {
    validate_flat_directory(
        directory,
        &[
            format!("{name}.utoc"),
            format!("{name}.ucas"),
            format!("{name}.pak"),
            receipt_name.to_owned(),
        ],
        "ASSET_PACK_OUTPUT",
    )
}

fn retarget_published_seal(output: &Path, staged: &FileSeal) -> Result<FileSeal> {
    let name = staged
        .path
        .file_name()
        .context("ASSET_PACK_OUTPUT: sealed staged file has no filename")?;
    Ok(FileSeal {
        path: output.join(name),
        length: staged.length,
        sha256: staged.sha256,
    })
}

fn run_publication_tail(
    final_source_gate: impl FnOnce() -> Result<()>,
    reverify_after_source_gate: impl FnOnce() -> Result<()>,
    final_live_gate: impl FnOnce() -> Result<()>,
    verify_mount_inventory: impl FnOnce() -> Result<()>,
    promote_noclobber: impl FnOnce() -> Result<()>,
) -> Result<()> {
    final_source_gate()?;
    reverify_after_source_gate()?;
    final_live_gate()?;
    verify_mount_inventory()?;
    promote_noclobber()?;
    Ok(())
}

#[derive(Debug)]
enum PostPublicationFinalization {
    Complete,
    CleanupWarning { detail: String },
    PublicationUncertain { detail: String },
}

fn classify_post_publication(
    durability: Result<()>,
    cleanup: impl FnOnce() -> Result<()>,
) -> PostPublicationFinalization {
    if let Err(error) = durability {
        return PostPublicationFinalization::PublicationUncertain {
            detail: format!("syncing publication parents: {error:#}"),
        };
    }
    match cleanup() {
        Ok(()) => PostPublicationFinalization::Complete,
        Err(error) => PostPublicationFinalization::CleanupWarning {
            detail: format!("removing empty owned staging directory: {error:#}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;

    use anyhow::{bail, Result};

    use super::super::{
        create_staging_directory, promote_directory_noclobber, reverify_file_seal, FileSeal,
    };
    use super::{
        classify_post_publication, retarget_published_seal, run_publication_tail,
        write_borrowed_component_new, write_bounded_receipt_new, MechanicalReceipt,
        PostPublicationFinalization,
    };
    use crate::dataasset_workflow::MAX_RECEIPT_BYTES;

    #[test]
    fn borrowed_component_becomes_an_independent_owned_sealed_file() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("component.bin");
        let mut source = vec![0x11, 0x22, 0x33, 0x44];
        let seal =
            write_borrowed_component_new(&target, &source, 16, "CORE_TEST_WRITE", "CORE_TEST_SEAL")
                .unwrap();
        source.fill(0xff);
        assert_eq!(fs::read(&target).unwrap(), [0x11, 0x22, 0x33, 0x44]);
        reverify_file_seal(&seal, 16, "CORE_TEST_SEAL").unwrap();
    }

    #[test]
    fn bounded_receipt_is_sealed_and_tamper_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let receipt =
            MechanicalReceipt::new("receipt.json", br#"{"ok":true}"#.to_vec(), 64, ()).unwrap();
        let seal = write_bounded_receipt_new(temp.path(), &receipt).unwrap();
        reverify_file_seal(&seal, 64, "CORE_TEST_RECEIPT").unwrap();
        fs::write(temp.path().join("receipt.json"), br#"{"ok":false}"#).unwrap();
        assert!(reverify_file_seal(&seal, 64, "CORE_TEST_RECEIPT").is_err());

        assert!(MechanicalReceipt::new("too-large.json", vec![0; 65], 64, ()).is_err());
        assert!(!temp.path().join("too-large.json").exists());

        let mut invalid_before_write =
            MechanicalReceipt::new("invalid-limit.json", Vec::new(), 64, ()).unwrap();
        invalid_before_write.max_bytes = MAX_RECEIPT_BYTES + 1;
        assert!(write_bounded_receipt_new(temp.path(), &invalid_before_write).is_err());
        assert!(!temp.path().join("invalid-limit.json").exists());
    }

    #[test]
    fn receipt_limit_is_always_bounded_by_the_absolute_cap() {
        assert!(MechanicalReceipt::new("zero.json", Vec::new(), 0, ()).is_err());
        assert!(
            MechanicalReceipt::new("over-cap.json", Vec::new(), MAX_RECEIPT_BYTES + 1, ()).is_err()
        );
        MechanicalReceipt::new("at-cap.json", Vec::new(), MAX_RECEIPT_BYTES, ()).unwrap();
    }

    #[test]
    fn malformed_staged_seal_retains_the_legacy_error_wording() {
        let error = retarget_published_seal(
            PathBuf::from("output").as_path(),
            &FileSeal {
                path: PathBuf::new(),
                length: 0,
                sha256: [0; 32],
            },
        )
        .unwrap_err();
        assert!(
            format!("{error:#}").contains("ASSET_PACK_OUTPUT: sealed staged file has no filename")
        );
    }

    #[test]
    fn atomic_promotion_never_clobbers_a_racing_output() {
        let temp = tempfile::tempdir().unwrap();
        let staged = temp.path().join("staged");
        let output = temp.path().join("output");
        fs::create_dir(&staged).unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(staged.join("new"), b"new").unwrap();
        fs::write(output.join("winner"), b"winner").unwrap();
        assert!(promote_directory_noclobber(&staged, &output).is_err());
        assert_eq!(fs::read(output.join("winner")).unwrap(), b"winner");
        assert!(staged.join("new").is_file());
    }

    #[test]
    fn owned_staging_cleans_itself_before_publication() {
        let temp = tempfile::tempdir().unwrap();
        let staging =
            create_staging_directory(temp.path(), "core-test", "CORE_TEST_STAGE").unwrap();
        let root = staging.path.clone();
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/input.bin"), b"owned").unwrap();
        drop(staging);
        assert!(!root.exists());
    }

    #[test]
    fn publication_tail_order_closes_source_races_before_final_live() {
        let events = RefCell::new(Vec::new());
        run_publication_tail(
            || {
                events.borrow_mut().push("source");
                Ok(())
            },
            || {
                events.borrow_mut().push("mechanical");
                Ok(())
            },
            || {
                events.borrow_mut().push("live");
                Ok(())
            },
            || {
                events.borrow_mut().push("mount");
                Ok(())
            },
            || {
                events.borrow_mut().push("promote");
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            *events.borrow(),
            ["source", "mechanical", "live", "mount", "promote"]
        );
    }

    #[test]
    fn failed_gate_never_reaches_promotion() {
        let promoted = RefCell::new(false);
        let result = run_publication_tail(
            || Ok(()),
            || Ok(()),
            || -> Result<()> { bail!("drift") },
            || Ok(()),
            || {
                *promoted.borrow_mut() = true;
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(!*promoted.borrow());
    }

    #[test]
    fn output_created_during_source_gate_fails_before_final_live_or_promotion() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("output");
        let events = RefCell::new(Vec::new());
        let result = run_publication_tail(
            || {
                fs::create_dir(&output).unwrap();
                fs::write(output.join("racer"), b"winner").unwrap();
                Ok(())
            },
            || {
                super::super::ensure_path_absent(&output, "CORE_TEST_OUTPUT")?;
                events.borrow_mut().push("mechanical");
                Ok(())
            },
            || {
                events.borrow_mut().push("live");
                Ok(())
            },
            || {
                events.borrow_mut().push("mount");
                Ok(())
            },
            || {
                events.borrow_mut().push("promote");
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(events.borrow().is_empty());
        assert_eq!(fs::read(output.join("racer")).unwrap(), b"winner");
    }

    #[test]
    fn terminal_postpublication_outcomes_remain_distinct() {
        assert!(matches!(
            classify_post_publication(Ok(()), || Ok(())),
            PostPublicationFinalization::Complete
        ));
        assert!(matches!(
            classify_post_publication(Ok(()), || bail!("cleanup")),
            PostPublicationFinalization::CleanupWarning { .. }
        ));
        assert!(matches!(
            classify_post_publication(Err(anyhow::anyhow!("sync")), || Ok(())),
            PostPublicationFinalization::PublicationUncertain { .. }
        ));
    }
}
