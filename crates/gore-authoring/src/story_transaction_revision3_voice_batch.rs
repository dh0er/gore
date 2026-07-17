//! Atomic, filesystem-free staging of a bounded set of revision-3 Voice takes.
//!
//! Every item is an existing single-take V1 request bound to the same exact
//! project/head basis. The batch policy deliberately permits only Recorded,
//! unselected takes with no localization edit. Items must target distinct
//! line/locale pairs. The existing single-take transaction remains the semantic
//! authority for every graph mutation; this adapter applies it to a private
//! candidate and collapses the project revision to one batch advance before the
//! complete candidate is reopened. No filesystem, Store, publication, build,
//! deployment, game, or save operation occurs here.

use std::collections::BTreeSet;

use crate::model_revision3::VoiceTakeStatus;
use crate::{
    apply_revision3_voice_take_transaction_v1, preflight_revision3_voice_take_transaction_v1,
    ImportedOgg, ProjectRevision3, ProjectRevision3JsonError, Revision3VoiceBuildStatusV1,
    Revision3VoicePublicationStatusV1, Revision3VoiceRuntimeStatusV1,
    Revision3VoiceTakePreflightEvaluationV1, Revision3VoiceTakeStageConflictV1,
    Revision3VoiceTakeStageErrorV1, Revision3VoiceTakeStageEvaluationV1,
    Revision3VoiceTakeStageRequestJsonErrorV1, Revision3VoiceTakeStageRequestV1,
    Revision3VoiceTargetAuthorityV1, WorkingHead,
};

pub const MAX_REVISION3_VOICE_BATCH_ITEMS_V1: usize = 256;
/// Upper bound for `canonical project bytes * batch items` before any
/// per-item project parsing or private mutation. The pure adapter deliberately
/// reuses the single-take authority, so this product bounds that reuse's JSON
/// work amplification independently of the ordinary project/item ceilings.
pub const MAX_REVISION3_VOICE_BATCH_PROJECT_WORK_BYTES_V1: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3VoiceTakeBatchConflictV1 {
    EmptyBatch,
    TooManyItems {
        actual: usize,
        max: usize,
    },
    ReceiptCountMismatch {
        requests: usize,
        receipts: usize,
    },
    ProjectWorkLimitExceeded {
        project_bytes: usize,
        items: usize,
        max_bytes: usize,
    },
    UnsupportedItemPolicy {
        item_index: usize,
    },
    MixedLocale {
        item_index: usize,
    },
    DuplicateLine {
        item_index: usize,
    },
    DuplicateSlot {
        item_index: usize,
    },
    Item {
        item_index: usize,
        conflict: Revision3VoiceTakeStageConflictV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeBatchRejectionV1 {
    pub conflict: Revision3VoiceTakeBatchConflictV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeBatchItemOutcomeV1 {
    pub line_id: crate::EntityId,
    pub localization_id: crate::EntityId,
    pub slot_id: crate::EntityId,
    pub take_id: crate::EntityId,
    pub locale: crate::LocaleCode,
    pub status: VoiceTakeStatus,
    pub slot_created: bool,
    pub selected: bool,
    pub imported_ogg: ImportedOgg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeBatchOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub items: Vec<Revision3VoiceTakeBatchItemOutcomeV1>,
    pub build_status: Revision3VoiceBuildStatusV1,
    pub runtime_status: Revision3VoiceRuntimeStatusV1,
    pub target_authority: Revision3VoiceTargetAuthorityV1,
    pub publication_status: Revision3VoicePublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3VoiceTakeBatchEvaluationV1 {
    Applied(Box<Revision3VoiceTakeBatchOutcomeV1>),
    Rejected(Revision3VoiceTakeBatchRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTakeBatchErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid canonical Voice request at batch item {item_index}: {source}")]
    InvalidRequest {
        item_index: usize,
        #[source]
        source: Revision3VoiceTakeStageRequestJsonErrorV1,
    },
    #[error("Voice batch item {item_index} failed: {source}")]
    Item {
        item_index: usize,
        #[source]
        source: Revision3VoiceTakeStageErrorV1,
    },
    #[error("could not serialize an internally rebound Voice batch item: {0}")]
    SerializeReboundRequest(#[source] Revision3VoiceTakeStageRequestJsonErrorV1),
    #[error("could not serialize the final Voice batch candidate: {0}")]
    SerializeCandidate(#[source] ProjectRevision3JsonError),
    #[error("could not reopen the final Voice batch candidate: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical Voice batch candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Apply a complete bounded Voice batch to one private candidate.
///
/// `canonical_request_jsons` and `imported_oggs` are positional peers. Every
/// request is first preflighted against the untouched exact basis, which proves
/// its original head/project/revision/target binding before any internal
/// rebinding is used to reuse the single-take transaction on the evolving
/// private candidate. A rejection returns no candidate at all.
pub fn apply_revision3_voice_take_batch_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_jsons: &[String],
    imported_oggs: Vec<ImportedOgg>,
) -> Result<Revision3VoiceTakeBatchEvaluationV1, Revision3VoiceTakeBatchErrorV1> {
    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3VoiceTakeBatchEvaluationV1::Rejected(
                Revision3VoiceTakeBatchRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if canonical_request_jsons.is_empty() {
        reject!(Revision3VoiceTakeBatchConflictV1::EmptyBatch);
    }
    if canonical_request_jsons.len() > MAX_REVISION3_VOICE_BATCH_ITEMS_V1 {
        reject!(Revision3VoiceTakeBatchConflictV1::TooManyItems {
            actual: canonical_request_jsons.len(),
            max: MAX_REVISION3_VOICE_BATCH_ITEMS_V1,
        });
    }
    if canonical_request_jsons.len() != imported_oggs.len() {
        reject!(Revision3VoiceTakeBatchConflictV1::ReceiptCountMismatch {
            requests: canonical_request_jsons.len(),
            receipts: imported_oggs.len(),
        });
    }

    let basis_project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3VoiceTakeBatchErrorV1::InvalidProject)?;
    if canonical_project_json
        .len()
        .checked_mul(canonical_request_jsons.len())
        .is_none_or(|work_bytes| work_bytes > MAX_REVISION3_VOICE_BATCH_PROJECT_WORK_BYTES_V1)
    {
        reject!(
            Revision3VoiceTakeBatchConflictV1::ProjectWorkLimitExceeded {
                project_bytes: canonical_project_json.len(),
                items: canonical_request_jsons.len(),
                max_bytes: MAX_REVISION3_VOICE_BATCH_PROJECT_WORK_BYTES_V1,
            }
        );
    }
    let basis_revision = basis_project.revision;
    let Some(batch_revision) = basis_revision.checked_add(1) else {
        reject!(Revision3VoiceTakeBatchConflictV1::Item {
            item_index: 0,
            conflict: Revision3VoiceTakeStageConflictV1::ProjectRevisionOverflow,
        });
    };

    let mut requests = Vec::with_capacity(canonical_request_jsons.len());
    let mut batch_locale = None;
    let mut target_lines = BTreeSet::new();
    let mut target_slots = BTreeSet::new();
    for (item_index, canonical_request_json) in canonical_request_jsons.iter().enumerate() {
        let request = Revision3VoiceTakeStageRequestV1::from_json(canonical_request_json).map_err(
            |source| Revision3VoiceTakeBatchErrorV1::InvalidRequest { item_index, source },
        )?;
        if request.status != VoiceTakeStatus::Recorded
            || request.select_take
            || request.text.is_some()
        {
            reject!(Revision3VoiceTakeBatchConflictV1::UnsupportedItemPolicy { item_index });
        }
        if let Some(locale) = &batch_locale {
            if locale != &request.locale {
                reject!(Revision3VoiceTakeBatchConflictV1::MixedLocale { item_index });
            }
        } else {
            batch_locale = Some(request.locale.clone());
        }
        // A V1 folder batch has one locale and may touch each DialogLine at
        // most once. This is what makes collapsing the private sequential
        // applications to one externally visible revision increment sound:
        // no line or slot entity has been incremented twice.
        if !target_lines.insert(request.line_id) {
            reject!(Revision3VoiceTakeBatchConflictV1::DuplicateLine { item_index });
        }
        if !target_slots.insert(request.slot_id) {
            reject!(Revision3VoiceTakeBatchConflictV1::DuplicateSlot { item_index });
        }
        match preflight_revision3_voice_take_transaction_v1(
            exact_basis_head,
            canonical_project_json,
            canonical_request_json,
        )
        .map_err(|source| Revision3VoiceTakeBatchErrorV1::Item { item_index, source })?
        {
            Revision3VoiceTakePreflightEvaluationV1::Ready => {}
            Revision3VoiceTakePreflightEvaluationV1::Rejected(rejection) => {
                reject!(Revision3VoiceTakeBatchConflictV1::Item {
                    item_index,
                    conflict: rejection.conflict,
                });
            }
        }
        requests.push(request);
    }

    let mut project = basis_project;
    let mut item_outcomes = Vec::with_capacity(requests.len());
    for (item_index, (mut request, imported_ogg)) in
        requests.into_iter().zip(imported_oggs).enumerate()
    {
        // Each private single-take application represents the same one-step
        // batch transaction. Keep its synthetic project basis fixed instead
        // of accumulating per-item global revisions; otherwise a valid batch
        // at `u64::MAX - 1` could overflow only because of internal reuse.
        project.revision = basis_revision;
        request.expected_revision = basis_revision;
        let rebound_request_json = request
            .to_canonical_json()
            .map_err(Revision3VoiceTakeBatchErrorV1::SerializeReboundRequest)?;
        let current_project_json = project
            .to_canonical_json()
            .map_err(Revision3VoiceTakeBatchErrorV1::SerializeCandidate)?;
        let outcome = match apply_revision3_voice_take_transaction_v1(
            exact_basis_head,
            &current_project_json,
            &rebound_request_json,
            imported_ogg,
        )
        .map_err(|source| Revision3VoiceTakeBatchErrorV1::Item { item_index, source })?
        {
            Revision3VoiceTakeStageEvaluationV1::Applied(outcome) => *outcome,
            Revision3VoiceTakeStageEvaluationV1::Rejected(rejection) => {
                reject!(Revision3VoiceTakeBatchConflictV1::Item {
                    item_index,
                    conflict: rejection.conflict,
                });
            }
        };
        project = outcome.project;
        item_outcomes.push(Revision3VoiceTakeBatchItemOutcomeV1 {
            line_id: outcome.line_id,
            localization_id: outcome.localization_id,
            slot_id: outcome.slot_id,
            take_id: outcome.take_id,
            locale: outcome.locale,
            status: outcome.status,
            slot_created: outcome.slot_created,
            selected: outcome.selected,
            imported_ogg: outcome.imported_ogg,
        });
    }

    // One accepted batch is one semantic project transaction and therefore
    // advances the externally persisted project revision exactly once.
    project.revision = batch_revision;
    let canonical_project_json = project
        .to_canonical_json()
        .map_err(Revision3VoiceTakeBatchErrorV1::SerializeCandidate)?;
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3VoiceTakeBatchErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3VoiceTakeBatchErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3VoiceTakeBatchEvaluationV1::Applied(Box::new(
        Revision3VoiceTakeBatchOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            items: item_outcomes,
            build_status: Revision3VoiceBuildStatusV1::Blocked,
            runtime_status: Revision3VoiceRuntimeStatusV1::RuntimeUnqualified,
            target_authority: Revision3VoiceTargetAuthorityV1::NotGranted,
            publication_status: Revision3VoicePublicationStatusV1::NotSupported,
        },
    )))
}
