//! Authenticated, fixed-head-authoritative bounded history for managed revision-3 projects.
//!
//! The current immutable snapshot seals one complete newest-first retention vector. History never
//! enumerates CAS directories and never follows history embedded in a retained checkpoint. An
//! unpublished candidate therefore cannot become history merely by existing. Restoring a retained
//! checkpoint prepares a new `current + 1` checkpoint whose first retained member is the exact
//! current head; it never moves or publishes `gore-project.json`.

use super::*;

/// Maximum timeline size returned by history, including the current checkpoint.
pub const MAX_REVISION3_HISTORY_ENTRIES_V1: usize = 256;

/// Maximum prior checkpoints sealed directly by the current snapshot.
pub const MAX_REVISION3_HISTORY_PARENT_RECORDS_V1: usize = MAX_REVISION3_HISTORY_ENTRIES_V1 - 1;

/// Maximum aggregate manifest bytes directly retained by one current history authority.
pub const MAX_REVISION3_HISTORY_MANIFEST_BYTES_V1: u64 = 64 * 1024 * 1024;

/// Stable authority description used by native adapters.
pub const REVISION3_HISTORY_AUTHORITY_V1: &str = "authenticated_bounded_history";

const REVISION3_HISTORY_FIELD_PREFIX_BYTES_V1: usize = b",\"history\":".len();

/// Exact retained checkpoint embedded in the current immutable revision-3 snapshot manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3CheckpointParentV1 {
    pub head: WorkingHead,
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub target: GameGenerationAnchor,
}

/// Complete bounded retention vector authenticated by one current snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3CheckpointHistoryV1 {
    pub prior_checkpoints: Vec<Revision3CheckpointParentV1>,
    pub history_truncated: bool,
}

/// Friendly project-level identity of one fully sealed checkpoint in the current timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3HistoryEntryV1 {
    pub head: WorkingHead,
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub meta: ProjectMeta,
    pub target: GameGenerationAnchor,
}

/// Complete newest-first retained timeline rooted exclusively at one exact current fixed head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3HistoryV1 {
    pub basis_head: WorkingHead,
    pub current: Revision3HistoryEntryV1,
    pub parents: Vec<Revision3HistoryEntryV1>,
    pub history_truncated: bool,
}

/// Fully reopened prepare-only result for restoring one retained historical checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreparedRevision3HistoryRestoreV1 {
    pub basis_head: WorkingHead,
    pub restored_from: Revision3HistoryEntryV1,
    pub project: ProjectRevision3,
    pub checkpoint: Revision3CheckpointPreparation,
}

/// Closed failures for bounded history inspection and restore preparation.
#[derive(Debug, thiserror::Error)]
pub enum Revision3HistoryErrorV1 {
    #[error(transparent)]
    Store(#[from] WorkingStoreError),
    #[error("invalid revision-3 retained history: {0}")]
    InvalidLineage(String),
    #[error(
        "revision-3 history target is not retained by the exact current fixed head: {target:?}"
    )]
    TargetNotReachable { target: WorkingHead },
    #[error("revision-3 history restore cannot advance project revision {current}")]
    ProjectRevisionOverflow { current: u64 },
    #[error("revision-3 history restore candidate did not retain the exact current checkpoint")]
    CandidateReopenMismatch,
}

pub(super) enum Revision3CheckpointHistoryPlanV1 {
    Root,
    ExactNoOp {
        head: WorkingHead,
    },
    Successor {
        history: Revision3CheckpointHistoryV1,
    },
}

/// Shallow-validate the complete retention vector sealed in one manifest.
///
/// Ordinary current-project open deliberately does not open historical objects: an old missing
/// retained object must not make otherwise intact current authoring content unusable. Operations
/// claiming history authority separately reopen every directly retained member.
pub(super) fn validate_revision3_checkpoint_history_v1(
    manifest: &Revision3SnapshotManifest,
    limits: &WorkingStoreLimits,
) -> Result<(), WorkingStoreError> {
    let Some(history) = &manifest.history else {
        return Ok(());
    };
    if history.prior_checkpoints.is_empty() {
        return Err(WorkingStoreError::Invariant(
            "revision-3 retained history must not be empty when present".to_owned(),
        ));
    }
    if history.prior_checkpoints.len() > MAX_REVISION3_HISTORY_PARENT_RECORDS_V1 {
        return Err(WorkingStoreError::Invariant(format!(
            "revision-3 retained history exceeds {} prior checkpoints",
            MAX_REVISION3_HISTORY_PARENT_RECORDS_V1
        )));
    }
    let history_envelope_bytes = canonical_json(history)?
        .len()
        .checked_add(REVISION3_HISTORY_FIELD_PREFIX_BYTES_V1)
        .ok_or(WorkingStoreError::LimitExceeded {
            kind: "revision-3 history snapshot reserve bytes",
            actual: u64::MAX,
            limit: REVISION3_HISTORY_SNAPSHOT_RESERVE_BYTES_V1 as u64,
        })?;
    enforce_limit(
        "revision-3 history snapshot reserve bytes",
        history_envelope_bytes,
        REVISION3_HISTORY_SNAPSHOT_RESERVE_BYTES_V1,
    )?;
    let mut expected_revision = manifest.revision.checked_sub(1);
    let mut known = BTreeMap::<Sha256Digest, u64>::new();
    let mut prior_manifest_bytes = 0u64;
    for retained in &history.prior_checkpoints {
        validate_nonzero_seal(
            &retained.head.snapshot,
            revision3_total_snapshot_limit(limits),
            "revision-3 retained history snapshot",
        )?;
        prior_manifest_bytes = checked_bounded_sum(
            "aggregate revision-3 retained history manifest bytes",
            prior_manifest_bytes,
            retained.head.snapshot.byte_len,
            MAX_REVISION3_HISTORY_MANIFEST_BYTES_V1,
        )?;
        if retained.project_id != manifest.project_id || retained.target != manifest.target {
            return Err(WorkingStoreError::Invariant(
                "revision-3 retained history changed project identity or target".to_owned(),
            ));
        }
        if Some(retained.project_revision) != expected_revision {
            return Err(WorkingStoreError::Invariant(
                "revision-3 retained history revisions are not consecutive newest-first".to_owned(),
            ));
        }
        expected_revision = retained.project_revision.checked_sub(1);

        if let Some(existing) = known.insert(
            retained.head.snapshot.sha256,
            retained.head.snapshot.byte_len,
        ) {
            let reason = if existing == retained.head.snapshot.byte_len {
                "revision-3 retained history repeats a snapshot"
            } else {
                "revision-3 retained history gives one snapshot digest conflicting lengths"
            };
            return Err(WorkingStoreError::Invariant(reason.to_owned()));
        }
    }
    Ok(())
}

fn build_successor_history_v1(
    current: Revision3CheckpointParentV1,
    inherited: Option<&Revision3CheckpointHistoryV1>,
) -> Revision3CheckpointHistoryV1 {
    let mut prior_manifest_bytes = current.head.snapshot.byte_len;
    let mut prior_checkpoints = Vec::with_capacity(
        1 + inherited
            .map(|history| {
                history
                    .prior_checkpoints
                    .len()
                    .min(MAX_REVISION3_HISTORY_PARENT_RECORDS_V1.saturating_sub(1))
            })
            .unwrap_or(0),
    );
    prior_checkpoints.push(current);

    let mut inherited_kept = 0usize;
    if let Some(history) = inherited {
        for retained in &history.prior_checkpoints {
            if prior_checkpoints.len() == MAX_REVISION3_HISTORY_PARENT_RECORDS_V1 {
                break;
            }
            let Some(next_bytes) = prior_manifest_bytes
                .checked_add(retained.head.snapshot.byte_len)
                .filter(|total| *total <= MAX_REVISION3_HISTORY_MANIFEST_BYTES_V1)
            else {
                break;
            };
            prior_manifest_bytes = next_bytes;
            prior_checkpoints.push(retained.clone());
            inherited_kept += 1;
        }
    }
    let history_truncated = inherited
        .map(|history| {
            history.history_truncated || inherited_kept != history.prior_checkpoints.len()
        })
        .unwrap_or(false);

    Revision3CheckpointHistoryV1 {
        prior_checkpoints,
        history_truncated,
    }
}

impl WorkingProjectStore {
    /// Build the complete bounded retention vector for a new checkpoint before immutable writes.
    pub(super) fn prepare_revision3_checkpoint_history_v1(
        &self,
        expected_head: Option<&WorkingHead>,
        candidate: &ProjectRevision3,
    ) -> Result<Revision3CheckpointHistoryPlanV1, WorkingStoreError> {
        let Some(expected_head) = expected_head else {
            return Ok(Revision3CheckpointHistoryPlanV1::Root);
        };
        let current = self.inspect_revision3_dataasset_basis(&expected_head.snapshot)?;
        if &current.head != expected_head {
            return Err(WorkingStoreError::Invariant(
                "revision-3 history basis head did not reproduce exactly".to_owned(),
            ));
        }
        if current.project_id != candidate.project_id || current.target != candidate.target {
            return Err(WorkingStoreError::Invariant(
                "revision-3 checkpoint changed project identity or target".to_owned(),
            ));
        }
        if current.revision == candidate.revision {
            let opened =
                self.open_revision3_snapshot(&expected_head.snapshot, AssetVerification::Full)?;
            if opened.head != *expected_head || opened.project != *candidate {
                return Err(WorkingStoreError::Invariant(
                    "same-revision revision-3 checkpoint is not an exact current-project no-op"
                        .to_owned(),
                ));
            }
            return Ok(Revision3CheckpointHistoryPlanV1::ExactNoOp {
                head: expected_head.clone(),
            });
        }
        if current.revision.checked_add(1) != Some(candidate.revision) {
            return Err(WorkingStoreError::Invariant(
                "revision-3 checkpoint revision is not exactly current + 1".to_owned(),
            ));
        }

        let current_record = Revision3CheckpointParentV1 {
            head: expected_head.clone(),
            project_id: current.project_id,
            project_revision: current.revision,
            target: current.target.clone(),
        };
        let history = build_successor_history_v1(current_record, current.manifest.history.as_ref());

        Ok(Revision3CheckpointHistoryPlanV1::Successor { history })
    }

    /// List the complete retained timeline rooted only at the exact current fixed head.
    ///
    /// Every retained manifest is canonical-parsed and fully hash checked, while entity and asset
    /// payloads are intentionally not opened for this metadata-only operation.
    pub fn list_current_revision3_history_v1(
        &self,
        expected_head: &WorkingHead,
    ) -> Result<Revision3HistoryV1, Revision3HistoryErrorV1> {
        self.ensure_root_safe()?;
        self.check_expected_head(Some(expected_head))?;

        let current_preflight = self.inspect_revision3_dataasset_basis(&expected_head.snapshot)?;
        if &current_preflight.head != expected_head {
            return Err(Revision3HistoryErrorV1::InvalidLineage(
                "current snapshot did not reproduce the expected head".to_owned(),
            ));
        }
        let current = history_entry(&current_preflight);
        let (retained, history_truncated) = current_preflight
            .manifest
            .history
            .as_ref()
            .map(|history| {
                (
                    history.prior_checkpoints.as_slice(),
                    history.history_truncated,
                )
            })
            .unwrap_or((&[], false));
        let mut parents = Vec::with_capacity(retained.len());
        let mut known = BTreeMap::<Sha256Digest, u64>::from([(
            expected_head.snapshot.sha256,
            expected_head.snapshot.byte_len,
        )]);
        let mut manifest_bytes = 0u64;

        for retained_record in retained {
            if let Some(existing) = known.insert(
                retained_record.head.snapshot.sha256,
                retained_record.head.snapshot.byte_len,
            ) {
                let reason = if existing == retained_record.head.snapshot.byte_len {
                    "retained history repeats the current or a prior snapshot"
                } else {
                    "retained history gives one snapshot digest conflicting lengths"
                };
                return Err(Revision3HistoryErrorV1::InvalidLineage(reason.to_owned()));
            }
            manifest_bytes = checked_bounded_sum(
                "aggregate revision-3 retained history manifest bytes",
                manifest_bytes,
                retained_record.head.snapshot.byte_len,
                MAX_REVISION3_HISTORY_MANIFEST_BYTES_V1,
            )?;
            let retained_preflight =
                self.inspect_revision3_dataasset_basis(&retained_record.head.snapshot)?;
            if retained_preflight.head != retained_record.head
                || retained_preflight.project_id != retained_record.project_id
                || retained_preflight.revision != retained_record.project_revision
                || retained_preflight.target != retained_record.target
                || retained_preflight.project_id != current.project_id
                || retained_preflight.target != current.target
            {
                return Err(Revision3HistoryErrorV1::InvalidLineage(
                    "retained history record disagrees with its sealed snapshot".to_owned(),
                ));
            }
            parents.push(history_entry(&retained_preflight));
        }

        self.check_expected_head(Some(expected_head))?;
        Ok(Revision3HistoryV1 {
            basis_head: expected_head.clone(),
            current,
            parents,
            history_truncated,
        })
    }

    /// Prepare a new current+1 checkpoint containing the exact project content of one directly
    /// retained member. The fixed head is checked throughout but never replaced by this operation.
    pub fn prepare_revision3_history_restore_v1(
        &self,
        expected_head: &WorkingHead,
        target_head: &WorkingHead,
    ) -> Result<PreparedRevision3HistoryRestoreV1, Revision3HistoryErrorV1> {
        let history = self.list_current_revision3_history_v1(expected_head)?;
        let restored_from = history
            .parents
            .iter()
            .find(|entry| &entry.head == target_head)
            .cloned()
            .ok_or_else(|| Revision3HistoryErrorV1::TargetNotReachable {
                target: target_head.clone(),
            })?;

        self.check_expected_head(Some(expected_head))?;
        let current = self.open_current_revision3(AssetVerification::Full)?;
        if &current.head != expected_head
            || current.project.project_id != history.current.project_id
            || current.project.revision != history.current.project_revision
            || current.project.target != history.current.target
        {
            return Err(Revision3HistoryErrorV1::InvalidLineage(
                "fully reopened current checkpoint disagrees with history metadata".to_owned(),
            ));
        }
        let historical =
            self.open_revision3_snapshot(&restored_from.head.snapshot, AssetVerification::Full)?;
        if historical.head != restored_from.head
            || historical.project.project_id != restored_from.project_id
            || historical.project.revision != restored_from.project_revision
            || historical.project.meta != restored_from.meta
            || historical.project.target != restored_from.target
        {
            return Err(Revision3HistoryErrorV1::InvalidLineage(
                "fully reopened restore target disagrees with history metadata".to_owned(),
            ));
        }

        let mut candidate = historical.project;
        candidate.revision = current.project.revision.checked_add(1).ok_or(
            Revision3HistoryErrorV1::ProjectRevisionOverflow {
                current: current.project.revision,
            },
        )?;
        let checkpoint = self.prepare_revision3_checkpoint(Some(expected_head), &candidate)?;
        let prepared = self.inspect_revision3_dataasset_basis(&checkpoint.head.snapshot)?;
        let retained = prepared.manifest.history.as_ref();
        let exact_current = retained.and_then(|history| history.prior_checkpoints.first());
        if prepared.head != checkpoint.head
            || prepared.project_id != candidate.project_id
            || prepared.revision != candidate.revision
            || exact_current.map(|entry| &entry.head) != Some(expected_head)
            || exact_current.map(|entry| entry.project_revision) != Some(current.project.revision)
        {
            return Err(Revision3HistoryErrorV1::CandidateReopenMismatch);
        }
        self.check_expected_head(Some(expected_head))?;

        Ok(PreparedRevision3HistoryRestoreV1 {
            basis_head: expected_head.clone(),
            restored_from,
            project: candidate,
            checkpoint,
        })
    }
}

fn history_entry(preflight: &Revision3DataAssetBasisPreflight) -> Revision3HistoryEntryV1 {
    Revision3HistoryEntryV1 {
        head: preflight.head.clone(),
        project_id: preflight.project_id,
        project_revision: preflight.revision,
        meta: preflight.manifest.meta.clone(),
        target: preflight.target.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn retained(revision: u64, tag: u8, byte_len: u64) -> Revision3CheckpointParentV1 {
        Revision3CheckpointParentV1 {
            head: WorkingHead {
                store_format: WorkingStoreFormat,
                snapshot: ContentSeal {
                    byte_len,
                    sha256: Sha256Digest::from_bytes([tag; 32]),
                },
            },
            project_id: ProjectId::from_bytes([1; 16]),
            project_revision: revision,
            target: GameGenerationAnchor {
                executable: ContentSeal {
                    byte_len: 1,
                    sha256: Sha256Digest::from_bytes([2; 32]),
                },
            },
        }
    }

    #[test]
    fn successor_retention_accepts_exact_byte_budget_and_truncates_first_over() {
        let chunk = MAX_REVISION3_HISTORY_MANIFEST_BYTES_V1 / 4;
        let exact_inherited = Revision3CheckpointHistoryV1 {
            prior_checkpoints: vec![
                retained(3, 3, chunk),
                retained(2, 2, chunk),
                retained(1, 1, chunk),
            ],
            history_truncated: false,
        };
        let exact = build_successor_history_v1(retained(4, 4, chunk), Some(&exact_inherited));
        assert_eq!(exact.prior_checkpoints.len(), 4);
        assert!(!exact.history_truncated);

        let over_inherited = Revision3CheckpointHistoryV1 {
            prior_checkpoints: vec![
                retained(4, 4, chunk),
                retained(3, 3, chunk),
                retained(2, 2, chunk),
                retained(1, 1, chunk),
            ],
            history_truncated: false,
        };
        let over = build_successor_history_v1(retained(5, 5, chunk), Some(&over_inherited));
        assert_eq!(over.prior_checkpoints.len(), 4);
        assert_eq!(over.prior_checkpoints.last().unwrap().project_revision, 2);
        assert!(over.history_truncated);

        let sticky_inherited = Revision3CheckpointHistoryV1 {
            prior_checkpoints: vec![retained(5, 5, 1)],
            history_truncated: true,
        };
        let sticky = build_successor_history_v1(retained(6, 6, 1), Some(&sticky_inherited));
        assert_eq!(sticky.prior_checkpoints.len(), 2);
        assert!(sticky.history_truncated);
    }

    #[test]
    fn maximum_history_envelope_fits_the_reserved_snapshot_megabyte() {
        assert_eq!(
            MAX_REVISION3_BASE_SNAPSHOT_BYTES as usize
                + REVISION3_HISTORY_SNAPSHOT_RESERVE_BYTES_V1,
            MAX_REVISION3_SNAPSHOT_BYTES as usize
        );
        let history = Revision3CheckpointHistoryV1 {
            prior_checkpoints: (0..MAX_REVISION3_HISTORY_PARENT_RECORDS_V1)
                .map(|index| {
                    let mut record = retained(u64::MAX - index as u64, index as u8, u64::MAX);
                    record.target.executable.byte_len = u64::MAX;
                    record
                })
                .collect(),
            history_truncated: false,
        };
        let envelope_bytes =
            canonical_json(&history).unwrap().len() + REVISION3_HISTORY_FIELD_PREFIX_BYTES_V1;
        assert!(envelope_bytes < REVISION3_HISTORY_SNAPSHOT_RESERVE_BYTES_V1);
    }
}
