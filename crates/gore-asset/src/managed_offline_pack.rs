//! Managed, reviewed DataAsset staging over the authority-neutral offline pack core.
//!
//! The public surface is deliberately two-phase. Preparation consumes the borrow-bound reviewed
//! package into a lifetime-free capsule and captures the output/game preflight. Staging then moves
//! the large pair buffers into sealed cooked copies, independently reopens the generated triplet,
//! and gives the caller one opaque, path-free proof from which to construct its bounded receipt.
//! Publication remains additive, no-clobber, and never deploys into the game.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::staged_pack_core::{
    MechanicalPublication, MechanicalPublishedPack, MechanicalReceipt, MechanicalStagedPack,
    PackBytes, PackInput,
};
use super::{
    reverify_file_seal, validate_output_component, FileSeal, HeldFileIdentity,
    OfflineDataAssetPackRequestV1,
};
use crate::dataasset_workflow::{
    probe_current_generation_receipt, AssetGenerationReceipt, VerifiedGameExecutableAnchor,
    VerifiedManagedOfflineDataAssetPackageV1, MAX_CONTAINER_COMPONENT_BYTES, MAX_RECEIPT_BYTES,
};
use crate::{
    verify_reviewed_footstep_preset_post_pack_v1, PackagePairSeal,
    ReviewedFootstepPresetReplacementV1, VerifiedReviewedFootstepPresetPostPackV1,
};

/// Path-free content seal for one member of a managed reviewed output triplet.
///
/// Fields and construction stay private. The enclosing opaque proof is the only producer, and its
/// array is always ordered as `.pak`, `.ucas`, `.utoc`.
#[derive(Debug, PartialEq, Eq)]
pub struct ManagedReviewedTripletFileSealV1 {
    relative_name: String,
    byte_len: u64,
    sha256: [u8; 32],
}

impl ManagedReviewedTripletFileSealV1 {
    pub fn relative_name(&self) -> &str {
        &self.relative_name
    }

    pub const fn byte_len(&self) -> u64 {
        self.byte_len
    }

    pub const fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

/// Exact path-free proof that one generated triplet contains the complete reviewed replacement.
///
/// This value has no `Clone`, serialization, public constructor, filesystem-path, or consuming
/// parts API. It can only be observed while constructing a receipt or through a successfully
/// published result.
#[derive(Debug)]
pub struct VerifiedManagedReviewedTripletPostPackV1 {
    pack_name: String,
    target_path: String,
    generation: AssetGenerationReceipt,
    executable_length: u64,
    executable_sha256: [u8; 32],
    replay_seal: PackagePairSeal,
    triplet_files: [ManagedReviewedTripletFileSealV1; 3],
    post_pack: VerifiedReviewedFootstepPresetPostPackV1,
}

impl VerifiedManagedReviewedTripletPostPackV1 {
    pub fn pack_name(&self) -> &str {
        &self.pack_name
    }

    pub fn target_path(&self) -> &str {
        &self.target_path
    }

    pub fn generation(&self) -> &AssetGenerationReceipt {
        &self.generation
    }

    pub const fn executable_length(&self) -> u64 {
        self.executable_length
    }

    pub const fn executable_sha256(&self) -> &[u8; 32] {
        &self.executable_sha256
    }

    pub fn replay_seal(&self) -> &PackagePairSeal {
        &self.replay_seal
    }

    pub fn triplet_files(&self) -> &[ManagedReviewedTripletFileSealV1; 3] {
        &self.triplet_files
    }

    pub fn post_pack(&self) -> &VerifiedReviewedFootstepPresetPostPackV1 {
        &self.post_pack
    }
}

struct ManagedLiveAuthorityV1 {
    target_path: String,
    generation: AssetGenerationReceipt,
    game_root: PathBuf,
    executable: VerifiedGameExecutableAnchor,
}

struct ProtectedRootAuthorityV1 {
    identity: HeldFileIdentity,
    output: PathBuf,
    output_parent: PathBuf,
}

impl ProtectedRootAuthorityV1 {
    fn capture(root: &Path) -> Result<HeldFileIdentity> {
        HeldFileIdentity::open_directory(root, "ASSET_MANAGED_PACK_PROTECTED_ROOT")
    }

    fn bind_output(identity: HeldFileIdentity, game_root: &Path, output: &Path) -> Result<Self> {
        let output_parent = output
            .parent()
            .context("ASSET_MANAGED_PACK_OUTPUT: output has no parent")?
            .to_path_buf();
        let authority = Self {
            identity,
            output: output.to_path_buf(),
            output_parent,
        };
        authority.reverify_disjoint(game_root)?;
        Ok(authority)
    }

    fn reverify_disjoint(&self, game_root: &Path) -> Result<()> {
        self.identity
            .reverify("ASSET_MANAGED_PACK_PROTECTED_ROOT")?;
        validate_protected_layout(
            &self.identity.path,
            game_root,
            &self.output,
            &self.output_parent,
        )
    }
}

impl ManagedLiveAuthorityV1 {
    fn reverify(&self) -> Result<()> {
        self.executable.reverify()?;
        let current = probe_current_generation_receipt(
            &self.game_root,
            &self.target_path,
            &self.generation,
            "ASSET_MANAGED_PACK_FINAL",
        )?;
        if current != self.generation {
            bail!("ASSET_MANAGED_PACK_FINAL: live target generation changed after verification");
        }
        self.executable.reverify()
    }
}

/// Lifetime-free prepared managed build. Construction writes nothing.
pub struct PreparedManagedReviewedDataAssetPackV1 {
    request: OfflineDataAssetPackRequestV1,
    target_path: String,
    generation: AssetGenerationReceipt,
    reviewed: ReviewedFootstepPresetReplacementV1,
    uasset: Vec<u8>,
    uexp: Vec<u8>,
    usmap: Vec<u8>,
    replay_seal: PackagePairSeal,
    live: ManagedLiveAuthorityV1,
    protected_root: ProtectedRootAuthorityV1,
}

impl fmt::Debug for PreparedManagedReviewedDataAssetPackV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedManagedReviewedDataAssetPackV1")
            .field("pack_name", &self.request.name())
            .field("target_path", &self.target_path)
            .field("generation", &self.generation)
            .field("uasset_bytes", &self.uasset.len())
            .field("uexp_bytes", &self.uexp.len())
            .field("usmap_bytes", &self.usmap.len())
            .finish()
    }
}

/// End the managed Store borrows and capture the complete offline-pack preflight.
///
/// The package's full live authority brackets request preparation. No source, sidecar, game, save,
/// or output file is written by this phase.
pub fn prepare_managed_reviewed_dataasset_pack_v1(
    package: VerifiedManagedOfflineDataAssetPackageV1<'_>,
    pack_name: &str,
    output: &Path,
    protected_root: &Path,
) -> Result<PreparedManagedReviewedDataAssetPackV1> {
    package.reverify_live_authority()?;
    let protected_identity = ProtectedRootAuthorityV1::capture(protected_root)?;
    let (
        target_path,
        generation,
        reviewed,
        uasset,
        uexp,
        usmap,
        replay_seal,
        game_root,
        executable,
    ) = package.into_owned_pack_parts().into_components();

    let live = ManagedLiveAuthorityV1 {
        target_path: target_path.clone(),
        generation: generation.clone(),
        game_root,
        executable,
    };
    protected_identity.reverify("ASSET_MANAGED_PACK_PROTECTED_ROOT")?;
    require_bidirectionally_disjoint(&protected_identity.path, &live.game_root, "live game root")?;
    let request =
        OfflineDataAssetPackRequestV1::prepare(&live.game_root, &target_path, pack_name, output)?;
    let protected_root = ProtectedRootAuthorityV1::bind_output(
        protected_identity,
        &live.game_root,
        request.output(),
    )?;
    live.reverify()?;

    Ok(PreparedManagedReviewedDataAssetPackV1 {
        request,
        target_path,
        generation,
        reviewed,
        uasset,
        uexp,
        usmap,
        replay_seal,
        live,
        protected_root,
    })
}

struct ManagedReceiptPayloadV1<R> {
    proof: VerifiedManagedReviewedTripletPostPackV1,
    caller_receipt: R,
}

/// Completely built and receipt-bound managed triplet held in owned sibling staging.
pub struct StagedManagedReviewedDataAssetPackV1<R> {
    core: MechanicalStagedPack<ManagedReceiptPayloadV1<R>>,
    live: ManagedLiveAuthorityV1,
    protected_root: ProtectedRootAuthorityV1,
}

/// Build, strictly reopen, semantically verify, and receipt-bind one prepared reviewed package.
///
/// The callback runs exactly once, after the same three output seals have bracketed strict
/// primary readback and reviewed semantic verification. It receives no path and returns the exact
/// receipt bytes plus an arbitrary caller-owned parsed payload retained through publication.
pub fn stage_prepared_managed_reviewed_dataasset_pack_v1<R>(
    prepared: PreparedManagedReviewedDataAssetPackV1,
    receipt_name: &str,
    build_receipt: impl FnOnce(&VerifiedManagedReviewedTripletPostPackV1) -> Result<(Vec<u8>, R)>,
) -> Result<StagedManagedReviewedDataAssetPackV1<R>> {
    let PreparedManagedReviewedDataAssetPackV1 {
        request,
        target_path,
        generation,
        reviewed,
        uasset,
        uexp,
        usmap,
        replay_seal,
        live,
        protected_root,
    } = prepared;
    validate_managed_receipt_name(request.name(), receipt_name)?;
    let receipt_name = receipt_name.to_owned();

    let input = PackInput {
        uasset: PackBytes::Owned(uasset),
        uexp: PackBytes::Owned(uexp),
        sidecars: Vec::new(),
    };
    let core = super::staged_pack_core::stage_with_receipt(
        request,
        input,
        |_| {
            protected_root.reverify_disjoint(&live.game_root)?;
            live.reverify()?;
            Ok(())
        },
        |evidence| {
            protected_root.reverify_disjoint(&live.game_root)?;
            live.reverify()?;
            reverify_triplet(evidence.output_seals())?;

            let primary_utoc =
                exact_primary_utoc(evidence.request().name(), evidence.output_seals())?;
            let readback =
                gore_tex::container::reopen_primary_asset_with_game_fallback_to_memory_v1(
                    &primary_utoc.path,
                    &live.game_root,
                    &target_path,
                )
                .context("ASSET_MANAGED_PACK_READBACK: reopening exact generated primary asset")?;
            let post_pack = verify_reviewed_footstep_preset_post_pack_v1(
                &reviewed,
                &usmap,
                evidence.strict(),
                readback,
            )
            .context("ASSET_MANAGED_PACK_REVIEW: verifying reviewed post-pack replacement")?;
            if post_pack.package_seal() != &replay_seal {
                bail!(
                    "ASSET_MANAGED_PACK_REPLAY: post-pack package differs from the exact reviewed replay"
                );
            }

            live.reverify()?;
            protected_root.reverify_disjoint(&live.game_root)?;
            reverify_triplet(evidence.output_seals())?;
            let triplet_files =
                canonical_triplet_files(evidence.request().name(), evidence.output_seals())?;
            let proof = VerifiedManagedReviewedTripletPostPackV1 {
                pack_name: evidence.request().name().to_owned(),
                target_path,
                generation,
                executable_length: live.executable.length(),
                executable_sha256: *live.executable.sha256(),
                replay_seal,
                triplet_files,
                post_pack,
            };
            let (receipt_bytes, caller_receipt) = build_receipt(&proof)?;
            MechanicalReceipt::new(
                receipt_name,
                receipt_bytes,
                MAX_RECEIPT_BYTES,
                ManagedReceiptPayloadV1 {
                    proof,
                    caller_receipt,
                },
            )
        },
    )?;

    Ok(StagedManagedReviewedDataAssetPackV1 {
        core,
        live,
        protected_root,
    })
}

/// Successfully published additive output. No deployment or runtime authority is implied.
pub struct PublishedManagedReviewedReceiptSealV1 {
    path: PathBuf,
    byte_len: u64,
    sha256: [u8; 32],
}

impl PublishedManagedReviewedReceiptSealV1 {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn byte_len(&self) -> u64 {
        self.byte_len
    }

    pub const fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

pub struct PublishedManagedReviewedDataAssetPackV1<R> {
    output: PathBuf,
    receipt_path: PathBuf,
    receipt_seal: PublishedManagedReviewedReceiptSealV1,
    proof: VerifiedManagedReviewedTripletPostPackV1,
    caller_receipt: R,
}

impl<R> PublishedManagedReviewedDataAssetPackV1<R> {
    pub fn output(&self) -> &Path {
        &self.output
    }

    pub fn receipt_path(&self) -> &Path {
        &self.receipt_path
    }

    pub fn receipt_seal(&self) -> &PublishedManagedReviewedReceiptSealV1 {
        &self.receipt_seal
    }

    pub fn proof(&self) -> &VerifiedManagedReviewedTripletPostPackV1 {
        &self.proof
    }

    pub fn caller_receipt(&self) -> &R {
        &self.caller_receipt
    }
}

pub struct ManagedReviewedDataAssetPackPublishedWithCleanupWarningV1<R> {
    published: PublishedManagedReviewedDataAssetPackV1<R>,
    detail: String,
}

impl<R> ManagedReviewedDataAssetPackPublishedWithCleanupWarningV1<R> {
    pub fn published(&self) -> &PublishedManagedReviewedDataAssetPackV1<R> {
        &self.published
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

pub struct ManagedReviewedDataAssetPackPublicationUncertainV1<R> {
    published: PublishedManagedReviewedDataAssetPackV1<R>,
    detail: String,
}

impl<R> ManagedReviewedDataAssetPackPublicationUncertainV1<R> {
    pub fn published(&self) -> &PublishedManagedReviewedDataAssetPackV1<R> {
        &self.published
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Typed result of atomic no-clobber publication.
pub enum ManagedReviewedDataAssetPackPublicationV1<R> {
    Published(PublishedManagedReviewedDataAssetPackV1<R>),
    PublishedWithCleanupWarning(ManagedReviewedDataAssetPackPublishedWithCleanupWarningV1<R>),
    PublicationUncertain(ManagedReviewedDataAssetPackPublicationUncertainV1<R>),
}

impl<R> StagedManagedReviewedDataAssetPackV1<R> {
    /// Recheck live authority, bracket the caller's exact-current managed Store gate, then let the
    /// core revalidate its publication candidate. A separate protected-root/live check remains the
    /// last non-core gate before the exact mount inventory check and atomic no-clobber rename.
    pub fn publish_with_final_source_gate(
        self,
        final_source_gate: impl FnOnce() -> Result<()>,
    ) -> Result<ManagedReviewedDataAssetPackPublicationV1<R>> {
        let Self {
            core,
            live,
            protected_root,
        } = self;
        protected_root.reverify_disjoint(&live.game_root)?;
        live.reverify()?;
        let publication = core.publish_with_hooks(
            || {
                protected_root.reverify_disjoint(&live.game_root)?;
                live.reverify()
            },
            |request| {
                if request.output() != protected_root.output {
                    bail!("ASSET_MANAGED_PACK_PROTECTED_ROOT: staged output authority changed");
                }
                run_bracketed_gate(
                    || {
                        protected_root.reverify_disjoint(&live.game_root)?;
                        live.reverify()
                    },
                    final_source_gate,
                    || {
                        protected_root.reverify_disjoint(&live.game_root)?;
                        live.reverify()
                    },
                )
            },
            |request| {
                if request.output() != protected_root.output {
                    bail!("ASSET_MANAGED_PACK_PROTECTED_ROOT: staged output authority changed");
                }
                protected_root.reverify_disjoint(&live.game_root)?;
                // Keep live last: the core proceeds directly to its exact mount inventory check.
                live.reverify()
            },
        )?;
        Ok(map_publication(publication))
    }
}

fn require_bidirectionally_disjoint(
    protected_root: &Path,
    other: &Path,
    other_label: &str,
) -> Result<()> {
    if protected_root.starts_with(other) || other.starts_with(protected_root) {
        bail!(
            "ASSET_MANAGED_PACK_PROTECTED_ROOT: managed Store root and {other_label} must be disjoint"
        );
    }
    Ok(())
}

fn validate_protected_layout(
    protected_root: &Path,
    game_root: &Path,
    output: &Path,
    output_parent: &Path,
) -> Result<()> {
    require_bidirectionally_disjoint(protected_root, game_root, "live game root")?;
    require_bidirectionally_disjoint(protected_root, output, "absent output directory")?;
    // Sibling Project/Build directories may share an ancestor. Only an output parent inside the
    // protected Store could put the staging lifecycle into that Store.
    if output_parent.starts_with(protected_root) {
        bail!(
            "ASSET_MANAGED_PACK_PROTECTED_ROOT: output parent must not be inside the managed Store root"
        );
    }
    Ok(())
}

fn validate_managed_receipt_name(pack_name: &str, receipt_name: &str) -> Result<()> {
    validate_output_component(receipt_name, "ASSET_MANAGED_PACK_RECEIPT")?;
    for extension in ["pak", "ucas", "utoc"] {
        if receipt_name.eq_ignore_ascii_case(&format!("{pack_name}.{extension}")) {
            bail!("ASSET_MANAGED_PACK_RECEIPT: receipt name collides with the output triplet");
        }
    }
    Ok(())
}

fn reverify_triplet(output_seals: &[FileSeal]) -> Result<()> {
    if output_seals.len() != 3 {
        bail!("ASSET_MANAGED_PACK_OUTPUT: expected exactly three sealed triplet files");
    }
    for seal in output_seals {
        reverify_file_seal(
            seal,
            MAX_CONTAINER_COMPONENT_BYTES,
            "ASSET_MANAGED_PACK_OUTPUT",
        )?;
    }
    Ok(())
}

fn exact_primary_utoc<'a>(pack_name: &str, output_seals: &'a [FileSeal]) -> Result<&'a FileSeal> {
    let expected = format!("{pack_name}.utoc");
    let mut matches = output_seals.iter().filter(|seal| {
        seal.path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == expected)
    });
    let primary = matches
        .next()
        .context("ASSET_MANAGED_PACK_OUTPUT: exact primary UTOC seal is missing")?;
    if matches.next().is_some() {
        bail!("ASSET_MANAGED_PACK_OUTPUT: exact primary UTOC seal is ambiguous");
    }
    Ok(primary)
}

fn canonical_triplet_files(
    pack_name: &str,
    output_seals: &[FileSeal],
) -> Result<[ManagedReviewedTripletFileSealV1; 3]> {
    if output_seals.len() != 3 {
        bail!("ASSET_MANAGED_PACK_OUTPUT: expected exactly three sealed triplet files");
    }
    let expected = [
        format!("{pack_name}.pak"),
        format!("{pack_name}.ucas"),
        format!("{pack_name}.utoc"),
    ];
    let mut ordered = [None, None, None];
    for seal in output_seals {
        let name = seal
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .context("ASSET_MANAGED_PACK_OUTPUT: sealed triplet file has no UTF-8 filename")?;
        let index = expected
            .iter()
            .position(|candidate| candidate == name)
            .context("ASSET_MANAGED_PACK_OUTPUT: sealed triplet filename is unexpected")?;
        if ordered[index].is_some() {
            bail!("ASSET_MANAGED_PACK_OUTPUT: sealed triplet filename is duplicated");
        }
        ordered[index] = Some(ManagedReviewedTripletFileSealV1 {
            relative_name: name.to_owned(),
            byte_len: seal.length,
            sha256: seal.sha256,
        });
    }
    let [pak, ucas, utoc] = ordered;
    Ok([
        pak.context("ASSET_MANAGED_PACK_OUTPUT: PAK seal is missing")?,
        ucas.context("ASSET_MANAGED_PACK_OUTPUT: UCAS seal is missing")?,
        utoc.context("ASSET_MANAGED_PACK_OUTPUT: UTOC seal is missing")?,
    ])
}

fn run_bracketed_gate(
    before: impl FnOnce() -> Result<()>,
    gate: impl FnOnce() -> Result<()>,
    after: impl FnOnce() -> Result<()>,
) -> Result<()> {
    before()?;
    gate()?;
    after()
}

fn map_published<R>(
    published: MechanicalPublishedPack<ManagedReceiptPayloadV1<R>>,
) -> PublishedManagedReviewedDataAssetPackV1<R> {
    let MechanicalPublishedPack {
        output,
        receipt_path,
        receipt_seal,
        receipt_payload,
        ..
    } = published;
    PublishedManagedReviewedDataAssetPackV1 {
        output,
        receipt_path,
        receipt_seal: PublishedManagedReviewedReceiptSealV1 {
            path: receipt_seal.path,
            byte_len: receipt_seal.length,
            sha256: receipt_seal.sha256,
        },
        proof: receipt_payload.proof,
        caller_receipt: receipt_payload.caller_receipt,
    }
}

fn map_publication<R>(
    publication: MechanicalPublication<ManagedReceiptPayloadV1<R>>,
) -> ManagedReviewedDataAssetPackPublicationV1<R> {
    match publication {
        MechanicalPublication::Published(published) => {
            ManagedReviewedDataAssetPackPublicationV1::Published(map_published(published))
        }
        MechanicalPublication::PublishedWithCleanupWarning { published, detail } => {
            ManagedReviewedDataAssetPackPublicationV1::PublishedWithCleanupWarning(
                ManagedReviewedDataAssetPackPublishedWithCleanupWarningV1 {
                    published: map_published(published),
                    detail,
                },
            )
        }
        MechanicalPublication::PublicationUncertain { published, detail } => {
            ManagedReviewedDataAssetPackPublicationV1::PublicationUncertain(
                ManagedReviewedDataAssetPackPublicationUncertainV1 {
                    published: map_published(published),
                    detail,
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use serde::{Deserialize, Serialize};

    use super::{
        canonical_triplet_files, exact_primary_utoc, run_bracketed_gate,
        validate_managed_receipt_name, validate_protected_layout, FileSeal,
        PreparedManagedReviewedDataAssetPackV1, VerifiedManagedReviewedTripletPostPackV1,
    };
    use crate::dataasset_workflow::VerifiedManagedOfflineDataAssetPackageV1;

    trait AmbiguousIfClone<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    impl<T: ?Sized + Clone> AmbiguousIfClone<u8> for T {}

    trait AmbiguousIfSerialize<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousIfSerialize<()> for T {}
    impl<T: ?Sized + Serialize> AmbiguousIfSerialize<u8> for T {}

    trait AmbiguousIfDeserialize<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousIfDeserialize<()> for T {}
    impl<T> AmbiguousIfDeserialize<u8> for T where T: for<'de> Deserialize<'de> {}

    #[test]
    fn managed_proof_is_not_clone_or_serde_authority() {
        let _ = <VerifiedManagedReviewedTripletPostPackV1 as AmbiguousIfClone<_>>::marker;
        let _ = <VerifiedManagedReviewedTripletPostPackV1 as AmbiguousIfSerialize<_>>::marker;
        let _ = <VerifiedManagedReviewedTripletPostPackV1 as AmbiguousIfDeserialize<_>>::marker;
    }

    #[allow(dead_code)]
    fn prepared_type_erases_the_managed_source_lifetime<'a>(
        package: VerifiedManagedOfflineDataAssetPackageV1<'a>,
        output: &Path,
        protected_root: &Path,
    ) -> Result<PreparedManagedReviewedDataAssetPackV1> {
        super::prepare_managed_reviewed_dataasset_pack_v1(
            package,
            "lifetime_probe",
            output,
            protected_root,
        )
    }

    fn seal(name: &str, length: u64, byte: u8) -> FileSeal {
        FileSeal {
            path: PathBuf::from("staged").join(name),
            length,
            sha256: [byte; 32],
        }
    }

    #[test]
    fn triplet_projection_is_path_free_and_canonical() {
        let seals = [
            seal("reviewed.utoc", 30, 3),
            seal("reviewed.pak", 10, 1),
            seal("reviewed.ucas", 20, 2),
        ];
        let files = canonical_triplet_files("reviewed", &seals).unwrap();
        assert_eq!(files[0].relative_name(), "reviewed.pak");
        assert_eq!(files[1].relative_name(), "reviewed.ucas");
        assert_eq!(files[2].relative_name(), "reviewed.utoc");
        assert_eq!(files[0].byte_len(), 10);
        assert_eq!(files[2].sha256(), &[3; 32]);
        assert!(files
            .iter()
            .all(|file| !file.relative_name().contains(['/', '\\'])));
        assert_eq!(exact_primary_utoc("reviewed", &seals).unwrap().length, 30);
    }

    #[test]
    fn triplet_projection_rejects_missing_duplicate_and_stray_files() {
        assert!(canonical_triplet_files(
            "reviewed",
            &[seal("reviewed.pak", 1, 1), seal("reviewed.ucas", 2, 2),],
        )
        .is_err());
        assert!(canonical_triplet_files(
            "reviewed",
            &[
                seal("reviewed.pak", 1, 1),
                seal("reviewed.pak", 2, 2),
                seal("reviewed.utoc", 3, 3),
            ],
        )
        .is_err());
        assert!(canonical_triplet_files(
            "reviewed",
            &[
                seal("reviewed.pak", 1, 1),
                seal("reviewed.ucas", 2, 2),
                seal("other.utoc", 3, 3),
            ],
        )
        .is_err());
    }

    #[test]
    fn receipt_name_cannot_replace_a_triplet_member() {
        validate_managed_receipt_name("reviewed", "build-receipt.json").unwrap();
        assert!(validate_managed_receipt_name("reviewed", "reviewed.pak").is_err());
        assert!(validate_managed_receipt_name("reviewed", "REVIEWED.UTOC").is_err());
        assert!(validate_managed_receipt_name("reviewed", "nested/receipt.json").is_err());
    }

    #[test]
    fn protected_layout_allows_siblings_but_rejects_containment() {
        let game = Path::new("C:/game");
        validate_protected_layout(
            Path::new("C:/mods/Project"),
            game,
            Path::new("C:/mods/Build/reviewed"),
            Path::new("C:/mods/Build"),
        )
        .unwrap();
        assert!(validate_protected_layout(
            Path::new("C:/mods/Project"),
            game,
            Path::new("C:/mods/Project/Build/reviewed"),
            Path::new("C:/mods/Project/Build"),
        )
        .is_err());
        assert!(validate_protected_layout(
            Path::new("C:/mods/Build/reviewed/Project"),
            game,
            Path::new("C:/mods/Build/reviewed"),
            Path::new("C:/mods/Build"),
        )
        .is_err());
    }

    #[test]
    fn final_source_gate_is_strictly_bracketed() {
        let events = RefCell::new(Vec::new());
        run_bracketed_gate(
            || {
                events.borrow_mut().push("live-before");
                Ok(())
            },
            || {
                events.borrow_mut().push("store-gate");
                Ok(())
            },
            || {
                events.borrow_mut().push("live-after");
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            events.into_inner(),
            ["live-before", "store-gate", "live-after"]
        );
    }
}
