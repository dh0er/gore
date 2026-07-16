//! Exact-basis orchestration for one managed reviewed revision-3 DataAsset build.
//!
//! This boundary accepts project identity and output intent only. Selector/replacement bytes,
//! receipt paths, overwrite policy, deployment, and runtime authority remain closed inside the
//! verified Store and `gore-asset` workflows. The basis must be exact-current at the final Store
//! gate. A later independent head publication does not invalidate the already sealed,
//! receipt-bound artifact; Studio callers serialize this operation and audit the head again through
//! their managed-session basis-snapshot lane.

use std::fmt::Display;
use std::fs;
use std::io;
use std::path::Path;

use gore_asset::dataasset_workflow::{
    verify_managed_offline_dataasset_package_v1, UnverifiedBorrowedManagedReviewedDataAssetSourceV1,
};
use gore_asset::{
    prepare_managed_reviewed_dataasset_pack_v1, stage_prepared_managed_reviewed_dataasset_pack_v1,
    ManagedReviewedDataAssetPackPublicationUncertainV1, ManagedReviewedDataAssetPackPublicationV1,
    ManagedReviewedDataAssetPackPublishedWithCleanupWarningV1, PackageComponent,
    PublishedManagedReviewedDataAssetPackV1, ReviewedFootstepPresetReplacementV1,
};

use crate::{
    AssetVerification, GameGenerationAnchor, ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1,
    ManagedRevision3ReviewedDataAssetBuildReceiptV1, ProjectId, ProjectRevision3,
    Revision3DataAssetStageViewV1, Revision3DataAssetStagingErrorV1,
    VerifiedCurrentReviewedDataAssetStageSourceV1,
    VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1, WorkingHead, WorkingProjectStore,
    WorkingStoreError,
};

/// Fixed receipt leaf used by every managed authoring build.
pub const REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FILE_V1: &str =
    "gore-authoring-dataasset-build.json";

const MAX_SANITIZED_BUILD_ERROR_DETAIL_BYTES: usize = 1024;

pub type PublishedRevision3ReviewedDataAssetBuildV1 =
    PublishedManagedReviewedDataAssetPackV1<ManagedRevision3ReviewedDataAssetBuildReceiptV1>;
pub type Revision3ReviewedDataAssetBuildCleanupWarningV1 =
    ManagedReviewedDataAssetPackPublishedWithCleanupWarningV1<
        ManagedRevision3ReviewedDataAssetBuildReceiptV1,
    >;
pub type Revision3ReviewedDataAssetBuildPublicationUncertainV1 =
    ManagedReviewedDataAssetPackPublicationUncertainV1<
        ManagedRevision3ReviewedDataAssetBuildReceiptV1,
    >;

/// Typed additive publication result. `PublicationUncertain` is a successful, non-retry outcome.
pub enum Revision3ReviewedDataAssetBuildPublicationV1 {
    Published(PublishedRevision3ReviewedDataAssetBuildV1),
    PublishedWithCleanupWarning(Revision3ReviewedDataAssetBuildCleanupWarningV1),
    PublicationUncertain(Revision3ReviewedDataAssetBuildPublicationUncertainV1),
}

impl Revision3ReviewedDataAssetBuildPublicationV1 {
    pub fn published(&self) -> &PublishedRevision3ReviewedDataAssetBuildV1 {
        match self {
            Self::Published(published) => published,
            Self::PublishedWithCleanupWarning(warning) => warning.published(),
            Self::PublicationUncertain(uncertain) => uncertain.published(),
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Published(_) => None,
            Self::PublishedWithCleanupWarning(warning) => Some(warning.detail()),
            Self::PublicationUncertain(uncertain) => Some(uncertain.detail()),
        }
    }

    pub const fn publication_is_uncertain(&self) -> bool {
        matches!(self, Self::PublicationUncertain(_))
    }
}

impl
    From<ManagedReviewedDataAssetPackPublicationV1<ManagedRevision3ReviewedDataAssetBuildReceiptV1>>
    for Revision3ReviewedDataAssetBuildPublicationV1
{
    fn from(
        value: ManagedReviewedDataAssetPackPublicationV1<
            ManagedRevision3ReviewedDataAssetBuildReceiptV1,
        >,
    ) -> Self {
        match value {
            ManagedReviewedDataAssetPackPublicationV1::Published(published) => {
                Self::Published(published)
            }
            ManagedReviewedDataAssetPackPublicationV1::PublishedWithCleanupWarning(warning) => {
                Self::PublishedWithCleanupWarning(warning)
            }
            ManagedReviewedDataAssetPackPublicationV1::PublicationUncertain(uncertain) => {
                Self::PublicationUncertain(uncertain)
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3ReviewedDataAssetBuildErrorV1 {
    #[error(transparent)]
    Store(#[from] WorkingStoreError),
    #[error(transparent)]
    StageSource(#[from] Revision3DataAssetStagingErrorV1),
    #[error(transparent)]
    Receipt(#[from] ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1),
    #[error(
        "expected revision-3 project {expected_project_id}@{expected_revision} differs from the fully reopened project {actual_project_id}@{actual_revision}"
    )]
    ExpectedProjectMismatch {
        expected_project_id: ProjectId,
        expected_revision: u64,
        actual_project_id: ProjectId,
        actual_revision: u64,
    },
    #[error("the exact-current reviewed DataAsset source changed during the build")]
    CurrentSourceChanged,
    #[error("managed DataAsset output already exists; additive publication will not overwrite it")]
    OutputAlreadyExists,
    #[error("could not inspect the managed DataAsset output: {detail}")]
    OutputInspection { detail: String },
    #[error("managed DataAsset live verification failed: {detail}")]
    LiveVerification { detail: String },
    #[error("managed DataAsset pack preparation failed: {detail}")]
    PackPreparation { detail: String },
    #[error("managed DataAsset pack staging failed: {detail}")]
    PackStaging { detail: String },
    #[error("managed DataAsset publication failed before a typed result existed: {detail}")]
    Publication { detail: String },
}

#[derive(Debug, Clone, PartialEq)]
struct ExactCurrentReviewedDataAssetSourceIdentityV1 {
    current_head: WorkingHead,
    project_id: ProjectId,
    project_revision: u64,
    project_target: GameGenerationAnchor,
    stage: Revision3DataAssetStageViewV1,
    reviewed: ReviewedFootstepPresetReplacementV1,
}

impl ExactCurrentReviewedDataAssetSourceIdentityV1 {
    fn capture(source: &VerifiedCurrentReviewedDataAssetStageSourceV1) -> Self {
        Self {
            current_head: source.current_head().clone(),
            project_id: source.project_id(),
            project_revision: source.project_revision(),
            project_target: source.project_target().clone(),
            stage: source.stage().clone(),
            reviewed: source.reviewed().clone(),
        }
    }
}

impl WorkingProjectStore {
    /// Build and atomically publish one selected reviewed revision-3 DataAsset from an exact basis.
    ///
    /// The Store and installed game are read-only. The only successful write is a new, absent
    /// output directory containing the generated triplet and fixed canonical authoring receipt.
    /// The supplied basis is proven current again inside the final publication gate; the receipt
    /// remains truthful if another cooperative operation publishes a newer head afterwards.
    #[allow(clippy::too_many_arguments)]
    pub fn build_revision3_reviewed_dataasset_v1(
        &self,
        expected_head: &WorkingHead,
        expected_project: &ProjectRevision3,
        game_root: &Path,
        target_path: &str,
        pack_name: &str,
        output: &Path,
    ) -> Result<Revision3ReviewedDataAssetBuildPublicationV1, Revision3ReviewedDataAssetBuildErrorV1>
    {
        let (source, source_identity) = open_exact_current_reviewed_source(
            self,
            expected_head,
            expected_project,
            target_path,
            None,
        )?;
        require_absent_output(output)?;

        let manifest = source.stage().manifest();
        let sidecars = source.sidecars().collect::<Vec<_>>();
        let borrowed = UnverifiedBorrowedManagedReviewedDataAssetSourceV1 {
            target_path: manifest.target_path(),
            generation: manifest.generation(),
            persisted_selector: manifest.selector(),
            persisted_replacement_hex: manifest.replacement_hex(),
            patched_uasset: source.patched_component_bytes(PackageComponent::Uasset),
            patched_uexp: source.patched_component_bytes(PackageComponent::Uexp),
            usmap: source.usmap_bytes(),
            sidecars: &sidecars,
            expected_executable_length: source.project_target().executable.byte_len,
            expected_executable_sha256: *source.project_target().executable.sha256.as_bytes(),
            reviewed: source.reviewed(),
        };
        let package =
            verify_managed_offline_dataasset_package_v1(game_root, borrowed).map_err(|error| {
                Revision3ReviewedDataAssetBuildErrorV1::LiveVerification {
                    detail: sanitized_detail(error),
                }
            })?;
        let prepared =
            prepare_managed_reviewed_dataasset_pack_v1(package, pack_name, output, self.root())
                .map_err(
                    |error| Revision3ReviewedDataAssetBuildErrorV1::PackPreparation {
                        detail: sanitized_detail(error),
                    },
                )?;
        drop(sidecars);

        // `prepared` is lifetime-free, so the Store source can now be consumed before Retoc runs.
        let basis =
            VerifiedManagedRevision3ReviewedDataAssetBuildBasisV1::from_current_source(source)?;
        let staged = stage_prepared_managed_reviewed_dataasset_pack_v1(
            prepared,
            REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FILE_V1,
            move |proof| {
                let receipt =
                    ManagedRevision3ReviewedDataAssetBuildReceiptV1::from_verified(basis, proof)
                        .map_err(|error| io::Error::other(sanitized_detail(error)))?;
                let bytes = receipt
                    .to_canonical_json()
                    .map_err(|error| io::Error::other(sanitized_detail(error)))?
                    .into_bytes();
                Ok((bytes, receipt))
            },
        )
        .map_err(
            |error| Revision3ReviewedDataAssetBuildErrorV1::PackStaging {
                detail: sanitized_detail(error),
            },
        )?;

        let publication = staged
            .publish_with_final_source_gate(|| {
                let (reopened, _) = open_exact_current_reviewed_source(
                    self,
                    expected_head,
                    expected_project,
                    target_path,
                    Some(&source_identity),
                )
                .map_err(|error| io::Error::other(sanitized_detail(error)))?;
                drop(reopened);
                Ok(())
            })
            .map_err(
                |error| Revision3ReviewedDataAssetBuildErrorV1::Publication {
                    detail: sanitized_detail(error),
                },
            )?;
        Ok(publication.into())
    }
}

fn require_exact_current_project(
    store: &WorkingProjectStore,
    expected_head: &WorkingHead,
    expected_project: &ProjectRevision3,
) -> Result<(), Revision3ReviewedDataAssetBuildErrorV1> {
    store.require_exact_head_for_dataasset(Some(expected_head))?;
    let opened = store.open_current_revision3(AssetVerification::Full)?;
    if &opened.head != expected_head {
        return Err(WorkingStoreError::HeadConflict {
            expected: Some(expected_head.clone()),
            actual: Some(opened.head),
        }
        .into());
    }
    if &opened.project != expected_project {
        return Err(
            Revision3ReviewedDataAssetBuildErrorV1::ExpectedProjectMismatch {
                expected_project_id: expected_project.project_id,
                expected_revision: expected_project.revision,
                actual_project_id: opened.project.project_id,
                actual_revision: opened.project.revision,
            },
        );
    }
    store.require_exact_head_for_dataasset(Some(expected_head))?;
    Ok(())
}

fn open_exact_current_reviewed_source(
    store: &WorkingProjectStore,
    expected_head: &WorkingHead,
    expected_project: &ProjectRevision3,
    target_path: &str,
    expected_identity: Option<&ExactCurrentReviewedDataAssetSourceIdentityV1>,
) -> Result<
    (
        VerifiedCurrentReviewedDataAssetStageSourceV1,
        ExactCurrentReviewedDataAssetSourceIdentityV1,
    ),
    Revision3ReviewedDataAssetBuildErrorV1,
> {
    require_exact_current_project(store, expected_head, expected_project)?;
    let source =
        store.open_current_reviewed_dataasset_stage_source_v1(expected_head, target_path)?;
    let identity = ExactCurrentReviewedDataAssetSourceIdentityV1::capture(&source);
    if expected_identity.is_some_and(|expected| expected != &identity) {
        return Err(Revision3ReviewedDataAssetBuildErrorV1::CurrentSourceChanged);
    }
    require_exact_current_project(store, expected_head, expected_project)?;
    Ok((source, identity))
}

fn require_absent_output(output: &Path) -> Result<(), Revision3ReviewedDataAssetBuildErrorV1> {
    // This read-only check gives an existing output deterministic precedence after the exact
    // reviewed Store source is proven but before any live-game work. The managed facade repeats
    // the authoritative absent/no-clobber checks across staging and atomic publication.
    match fs::symlink_metadata(output) {
        Ok(_) => Err(Revision3ReviewedDataAssetBuildErrorV1::OutputAlreadyExists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Revision3ReviewedDataAssetBuildErrorV1::OutputInspection {
            detail: sanitized_detail(error),
        }),
    }
}

fn sanitized_detail(error: impl Display) -> String {
    let mut detail = error
        .to_string()
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    if detail.len() > MAX_SANITIZED_BUILD_ERROR_DETAIL_BYTES {
        let mut end = MAX_SANITIZED_BUILD_ERROR_DETAIL_BYTES;
        while !detail.is_char_boundary(end) {
            end -= 1;
        }
        detail.truncate(end);
    }
    detail
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::dataasset_stage::tests::{
        publish_reviewed_stage, reviewed_wolf_fixture, reviewed_wolf_fixture_without_sidecars,
        TestRoot,
    };
    use crate::WorkingStoreLimits;

    fn snapshot_tree(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
        fn visit(root: &Path, current: &Path, output: &mut Vec<(PathBuf, Option<Vec<u8>>)>) {
            let mut entries = fs::read_dir(current)
                .unwrap()
                .map(|entry| entry.unwrap())
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                let metadata = fs::symlink_metadata(&path).unwrap();
                if metadata.is_dir() {
                    output.push((relative, None));
                    visit(root, &path, output);
                } else {
                    output.push((relative, Some(fs::read(&path).unwrap())));
                }
            }
        }

        let mut output = Vec::new();
        visit(root, root, &mut output);
        output
    }

    #[test]
    fn exact_source_gate_rejects_stale_project_and_identity_without_writes() {
        let store_root = TestRoot::new("build-exact-gate-store");
        let game_root = TestRoot::new("build-exact-gate-game");
        let store =
            WorkingProjectStore::at(store_root.path(), WorkingStoreLimits::default()).unwrap();
        let fixture = reviewed_wolf_fixture();
        let staged = publish_reviewed_stage(&store_root, &store, &fixture);
        let expected_head = &staged.checkpoint().head;
        let expected_project = staged.project();
        let before_store = snapshot_tree(store_root.path());
        let before_game = snapshot_tree(game_root.path());

        assert!(matches!(
            open_exact_current_reviewed_source(
                &store,
                staged.basis_head(),
                expected_project,
                fixture.target_path(),
                None,
            ),
            Err(Revision3ReviewedDataAssetBuildErrorV1::Store(
                WorkingStoreError::HeadConflict { .. }
            ))
        ));

        let mut wrong_project = expected_project.clone();
        wrong_project.revision += 1;
        assert!(matches!(
            open_exact_current_reviewed_source(
                &store,
                expected_head,
                &wrong_project,
                fixture.target_path(),
                None,
            ),
            Err(Revision3ReviewedDataAssetBuildErrorV1::ExpectedProjectMismatch { .. })
        ));

        let (source, identity) = open_exact_current_reviewed_source(
            &store,
            expected_head,
            expected_project,
            fixture.target_path(),
            None,
        )
        .unwrap();
        drop(source);
        let (reopened, reopened_identity) = open_exact_current_reviewed_source(
            &store,
            expected_head,
            expected_project,
            fixture.target_path(),
            Some(&identity),
        )
        .unwrap();
        drop(reopened);
        assert_eq!(reopened_identity, identity);

        let mut changed_identity = identity;
        changed_identity.project_revision += 1;
        assert!(matches!(
            open_exact_current_reviewed_source(
                &store,
                expected_head,
                expected_project,
                fixture.target_path(),
                Some(&changed_identity),
            ),
            Err(Revision3ReviewedDataAssetBuildErrorV1::CurrentSourceChanged)
        ));

        assert_eq!(snapshot_tree(store_root.path()), before_store);
        assert_eq!(snapshot_tree(game_root.path()), before_game);
    }

    #[test]
    fn existing_output_stops_before_live_work_and_nothing_is_modified() {
        let store_root = TestRoot::new("build-no-clobber-store");
        let game_root = TestRoot::new("build-no-clobber-game");
        let output_root = TestRoot::new("build-no-clobber-output");
        let store =
            WorkingProjectStore::at(store_root.path(), WorkingStoreLimits::default()).unwrap();
        let fixture = reviewed_wolf_fixture();
        let staged = publish_reviewed_stage(&store_root, &store, &fixture);
        let output = output_root.path().join("WolfReview");
        fs::create_dir(&output).unwrap();
        fs::write(output.join("keep.txt"), b"do not replace").unwrap();
        fs::write(game_root.path().join("keep.txt"), b"do not touch game").unwrap();

        let before_store = snapshot_tree(store_root.path());
        let before_game = snapshot_tree(game_root.path());
        let before_output = snapshot_tree(output_root.path());
        let result = store.build_revision3_reviewed_dataasset_v1(
            &staged.checkpoint().head,
            staged.project(),
            game_root.path(),
            fixture.target_path(),
            "WolfReview",
            &output,
        );

        assert!(matches!(
            result,
            Err(Revision3ReviewedDataAssetBuildErrorV1::OutputAlreadyExists)
        ));
        assert_eq!(snapshot_tree(store_root.path()), before_store);
        assert_eq!(snapshot_tree(game_root.path()), before_game);
        assert_eq!(snapshot_tree(output_root.path()), before_output);
    }

    #[test]
    fn absent_output_reaches_live_verification_without_touching_any_root() {
        let store_root = TestRoot::new("build-live-failure-store");
        let game_root = TestRoot::new("build-live-failure-game");
        let output_root = TestRoot::new("build-live-failure-output");
        let store =
            WorkingProjectStore::at(store_root.path(), WorkingStoreLimits::default()).unwrap();
        let fixture = reviewed_wolf_fixture_without_sidecars();
        let staged = publish_reviewed_stage(&store_root, &store, &fixture);
        let output = output_root.path().join("WolfReview");
        fs::write(game_root.path().join("keep.txt"), b"do not touch game").unwrap();

        let before_store = snapshot_tree(store_root.path());
        let before_game = snapshot_tree(game_root.path());
        let before_output = snapshot_tree(output_root.path());
        let result = store.build_revision3_reviewed_dataasset_v1(
            &staged.checkpoint().head,
            staged.project(),
            game_root.path(),
            fixture.target_path(),
            "WolfReview",
            &output,
        );

        assert!(matches!(
            result,
            Err(Revision3ReviewedDataAssetBuildErrorV1::LiveVerification { .. })
        ));
        assert!(!output.exists());
        assert_eq!(snapshot_tree(store_root.path()), before_store);
        assert_eq!(snapshot_tree(game_root.path()), before_game);
        assert_eq!(snapshot_tree(output_root.path()), before_output);
    }

    #[test]
    fn fixed_receipt_leaf_and_sanitized_detail_are_closed_and_bounded() {
        assert_eq!(
            REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FILE_V1,
            "gore-authoring-dataasset-build.json"
        );
        assert!(!REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FILE_V1.contains(['/', '\\']));

        // 1,024 falls inside a three-byte UTF-8 code point, exercising boundary-safe truncation.
        let detail = sanitized_detail(format!("{}\nsecret", "\u{20ac}".repeat(400)));
        assert!(detail.len() <= MAX_SANITIZED_BUILD_ERROR_DETAIL_BYTES);
        assert!(!detail.chars().any(char::is_control));
        assert!(detail.is_char_boundary(detail.len()));
    }
}
