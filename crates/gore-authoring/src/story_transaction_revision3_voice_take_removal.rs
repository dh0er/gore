//! Exact-basis-bound removal of one Voice take from one revision-3 Voice slot.
//!
//! The transaction is filesystem-free and slot-scoped. It removes exactly one candidate edge,
//! clears that slot's selection when necessary, and removes the `VoiceTake` entity only when no
//! other local slot still uses it. The complete `AssetStore` is always preserved: this operation
//! never deletes an Ogg blob, publishes a head, or grants build/deployment/runtime authority.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    DialogLine, EntityKind, EntityPayload, LocalizationEntry, ProjectRevision3, VoiceSlot,
};
use crate::revision3_content_index::{
    build_revision3_content_index_v1, Revision3ContentIndexErrorV1,
    Revision3ContentReferenceResolutionV1, Revision3ContentReferenceRoleV1,
};
use crate::story_transaction_revision3_voice_target::validate_revision3_voice_loc_id_basename_stem_v1;
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    EntityId, GameGenerationAnchor, LocaleCode, ProjectId, ProjectRevision3JsonError, WorkingHead,
};

/// Maximum exact canonical Voice-take removal request size.
pub const MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1: usize = 64 * 1024;

/// Exact head/project/line/slot/take CAS binding for one slot-scoped unlink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3VoiceTakeRemovalRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub line_id: EntityId,
    pub localization_id: EntityId,
    pub expected_loc_id: String,
    pub locale: LocaleCode,
    pub slot_id: EntityId,
    pub expected_slot_revision: u64,
    pub take_id: EntityId,
    pub expected_take_revision: u64,
    pub expected_selected_take_id: Option<EntityId>,
}

impl Revision3VoiceTakeRemovalRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3VoiceTakeRemovalRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1 {
            return Err(Revision3VoiceTakeRemovalRequestJsonErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3VoiceTakeRemovalRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3VoiceTakeRemovalRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3VoiceTakeRemovalRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3VoiceTakeRemovalRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3VoiceTakeRemovalRequestJsonErrorV1::InputTooLarge {
                actual,
                limit: MAX_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_JSON_BYTES_V1,
            });
        }
        serialized.map_err(Revision3VoiceTakeRemovalRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3VoiceTakeRemovalRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTakeRemovalRequestJsonErrorV1 {
    #[error(
        "revision-3 Voice take removal request exceeds the {limit}-byte limit: {actual} bytes"
    )]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Voice take removal request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Voice take removal request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Voice take removal request is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Voice take removal request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Stable semantic conflict. Rejection never exposes a partially changed project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3VoiceTakeRemovalConflictV1 {
    #[error("request basis head does not match the exact supplied head")]
    CurrentHeadMismatch,
    #[error("expected project {expected}, but exact basis is {actual}")]
    ProjectIdentityMismatch {
        expected: ProjectId,
        actual: ProjectId,
    },
    #[error("expected project revision {expected}, but exact basis is {actual}")]
    ProjectRevisionConflict { expected: u64, actual: u64 },
    #[error("request target does not match the exact project target")]
    ProjectTargetMismatch,
    #[error("project revision cannot be incremented")]
    ProjectRevisionOverflow,
    #[error(
        "DialogLine, LocalizationEntry, VoiceSlot, and VoiceTake IDs must be non-zero and distinct"
    )]
    InvalidEntityIdentity,
    #[error("expected localization ID is not one bounded portable Voice basename stem")]
    InvalidExpectedLocId,
    #[error("DialogLine entity {line} is missing or has the wrong kind")]
    InvalidDialogLine { line: EntityId },
    #[error("DialogLine {line} does not bind the requested exact local LocalizationEntry {localization}")]
    InvalidLocalizationReference {
        line: EntityId,
        localization: EntityId,
    },
    #[error("expected localization ID {expected}, but exact basis is {actual}")]
    LocalizationIdentityMismatch { expected: String, actual: String },
    #[error("line/locale is not linked to requested VoiceSlot {slot}")]
    VoiceSlotIdentityMismatch { slot: EntityId },
    #[error("VoiceSlot {slot} is missing, has the wrong kind, locale, or unique owner")]
    InvalidVoiceSlot { slot: EntityId },
    #[error("expected VoiceSlot revision {expected}, but exact basis is {actual}")]
    VoiceSlotRevisionConflict { expected: u64, actual: u64 },
    #[error("VoiceSlot {slot} revision cannot be incremented")]
    VoiceSlotRevisionOverflow { slot: EntityId },
    #[error("VoiceTake {take} is missing or has the wrong kind")]
    InvalidVoiceTake { take: EntityId },
    #[error("expected VoiceTake revision {expected}, but exact basis is {actual}")]
    VoiceTakeRevisionConflict { expected: u64, actual: u64 },
    #[error("VoiceTake {take} locale differs from the requested VoiceSlot locale")]
    VoiceTakeLocaleMismatch { take: EntityId },
    #[error("VoiceTake {take} is not exactly one candidate of VoiceSlot {slot}")]
    VoiceTakeNotExactCandidate { take: EntityId, slot: EntityId },
    #[error("expected selected VoiceTake {expected:?}, but exact basis has {actual:?}")]
    CurrentSelectionMismatch {
        expected: Option<EntityId>,
        actual: Option<EntityId>,
    },
    #[error("VoiceTake {take} has an unsafe local backlink from {source_entity} through {role:?}: {reason}")]
    InvalidLocalBacklink {
        take: EntityId,
        source_entity: EntityId,
        role: Revision3ContentReferenceRoleV1,
        reason: String,
    },
    #[error("Voice take removal preflight exceeds the {limit}-reference limit")]
    ReferenceLimit { limit: usize },
    #[error("Voice take removal candidate exceeds the {limit}-byte project limit: {actual} bytes")]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("Voice take removal candidate is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeRemovalRejectionV1 {
    pub conflict: Revision3VoiceTakeRemovalConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceTakeRemovalBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceTakeRemovalRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeRemovalOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub line_id: EntityId,
    pub localization_id: EntityId,
    pub slot_id: EntityId,
    pub slot_revision: u64,
    pub locale: LocaleCode,
    pub loc_id: String,
    pub take_id: EntityId,
    pub take_revision: u64,
    pub previous_selected_take_id: Option<EntityId>,
    pub selection_cleared: bool,
    pub take_entity_removed: bool,
    pub remaining_candidate_count: u64,
    pub build_status: Revision3VoiceTakeRemovalBuildStatusV1,
    pub runtime_status: Revision3VoiceTakeRemovalRuntimeStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3VoiceTakeRemovalEvaluationV1 {
    Applied(Box<Revision3VoiceTakeRemovalOutcomeV1>),
    Rejected(Revision3VoiceTakeRemovalRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTakeRemovalErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 Voice take removal request: {0}")]
    InvalidRequest(#[source] Revision3VoiceTakeRemovalRequestJsonErrorV1),
    #[error("could not build the exact revision-3 content index: {0}")]
    ContentIndex(#[source] Revision3ContentIndexErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 Voice take removal candidate reopen changed the project")]
    CanonicalReopenMismatch,
    #[error("Voice take removal changed a preserved project value")]
    CandidatePreservationMismatch,
}

/// Remove one exact take candidate from one exact Voice slot.
///
/// The take entity survives byte-for-byte when another local slot still uses it. When the edge
/// being removed is its final local use, only the entity is removed; the complete `AssetStore`
/// (including the referenced Ogg metadata/CAS identity) remains byte-for-byte unchanged.
pub fn apply_revision3_voice_take_removal_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3VoiceTakeRemovalEvaluationV1, Revision3VoiceTakeRemovalErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3VoiceTakeRemovalErrorV1::InvalidProject)?;
    let request = Revision3VoiceTakeRemovalRequestV1::from_json(canonical_request_json)
        .map_err(Revision3VoiceTakeRemovalErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3VoiceTakeRemovalEvaluationV1::Rejected(
                Revision3VoiceTakeRemovalRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3VoiceTakeRemovalConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3VoiceTakeRemovalConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3VoiceTakeRemovalConflictV1::ProjectRevisionOverflow);
    };
    if !distinct_nonzero_entity_ids([
        request.line_id,
        request.localization_id,
        request.slot_id,
        request.take_id,
    ]) {
        reject!(Revision3VoiceTakeRemovalConflictV1::InvalidEntityIdentity);
    }
    if validate_revision3_voice_loc_id_basename_stem_v1(&request.expected_loc_id).is_err() {
        reject!(Revision3VoiceTakeRemovalConflictV1::InvalidExpectedLocId);
    }

    let Some(line_entity) = project.entities.get(&request.line_id) else {
        reject!(Revision3VoiceTakeRemovalConflictV1::InvalidDialogLine {
            line: request.line_id,
        });
    };
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        reject!(Revision3VoiceTakeRemovalConflictV1::InvalidDialogLine {
            line: request.line_id,
        });
    };
    let Some(loc_id) = exact_localization(&project, line, request.localization_id) else {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::InvalidLocalizationReference {
                line: request.line_id,
                localization: request.localization_id,
            }
        );
    };
    if loc_id != request.expected_loc_id {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::LocalizationIdentityMismatch {
                expected: request.expected_loc_id,
                actual: loc_id,
            }
        );
    }
    let Some(slot_ref) = line.voice_slots.get(&request.locale) else {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::VoiceSlotIdentityMismatch {
                slot: request.slot_id,
            }
        );
    };
    if slot_ref.project_id != project.project_id
        || slot_ref.expected_kind != EntityKind::VoiceSlot
        || slot_ref.id != request.slot_id
    {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::VoiceSlotIdentityMismatch {
                slot: request.slot_id,
            }
        );
    }
    if !has_unique_slot_owner(&project, request.line_id, &request.locale, request.slot_id) {
        reject!(Revision3VoiceTakeRemovalConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    }

    let Some(slot_entity) = project.entities.get(&request.slot_id) else {
        reject!(Revision3VoiceTakeRemovalConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    };
    let EntityPayload::VoiceSlot(VoiceSlot {
        locale,
        candidates,
        selected,
        ..
    }) = &slot_entity.payload
    else {
        reject!(Revision3VoiceTakeRemovalConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    };
    if locale != &request.locale {
        reject!(Revision3VoiceTakeRemovalConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    }
    if slot_entity.revision != request.expected_slot_revision {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::VoiceSlotRevisionConflict {
                expected: request.expected_slot_revision,
                actual: slot_entity.revision,
            }
        );
    }
    let Some(next_slot_revision) = slot_entity.revision.checked_add(1) else {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::VoiceSlotRevisionOverflow {
                slot: request.slot_id,
            }
        );
    };
    let candidate_positions = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            (candidate.project_id == project.project_id
                && candidate.expected_kind == EntityKind::VoiceTake
                && candidate.id == request.take_id)
                .then_some(index)
        })
        .collect::<Vec<_>>();
    if candidate_positions.len() != 1 {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::VoiceTakeNotExactCandidate {
                take: request.take_id,
                slot: request.slot_id,
            }
        );
    }
    let candidate_position = candidate_positions[0];
    let current_selected_take_id = selected.as_ref().map(|reference| reference.id);
    if request.expected_selected_take_id != current_selected_take_id {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::CurrentSelectionMismatch {
                expected: request.expected_selected_take_id,
                actual: current_selected_take_id,
            }
        );
    }

    let Some(take_entity) = project.entities.get(&request.take_id) else {
        reject!(Revision3VoiceTakeRemovalConflictV1::InvalidVoiceTake {
            take: request.take_id,
        });
    };
    let EntityPayload::VoiceTake(take) = &take_entity.payload else {
        reject!(Revision3VoiceTakeRemovalConflictV1::InvalidVoiceTake {
            take: request.take_id,
        });
    };
    if take_entity.revision != request.expected_take_revision {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::VoiceTakeRevisionConflict {
                expected: request.expected_take_revision,
                actual: take_entity.revision,
            }
        );
    }
    if take.locale != request.locale {
        reject!(
            Revision3VoiceTakeRemovalConflictV1::VoiceTakeLocaleMismatch {
                take: request.take_id,
            }
        );
    }

    let index = match build_revision3_content_index_v1(&project) {
        Ok(index) => index,
        Err(Revision3ContentIndexErrorV1::TooManyReferences { limit }) => {
            reject!(Revision3VoiceTakeRemovalConflictV1::ReferenceLimit { limit });
        }
        Err(error) => return Err(Revision3VoiceTakeRemovalErrorV1::ContentIndex(error)),
    };
    let usage = match validate_take_backlinks(&project, &index, &request) {
        Ok(usage) => usage,
        Err(blocker) => {
            reject!(Revision3VoiceTakeRemovalConflictV1::InvalidLocalBacklink {
                take: request.take_id,
                source_entity: blocker.source_entity,
                role: blocker.role,
                reason: blocker.reason,
            });
        }
    };
    let take_entity_removed = usage.candidate_slots.len() == 1;
    let selection_cleared = current_selected_take_id == Some(request.take_id);
    let remaining_candidate_count = (candidates.len() - 1) as u64;

    let basis_project = project.clone();
    let Some(slot_entity) = project.entities.get_mut(&request.slot_id) else {
        unreachable!("VoiceSlot was resolved above")
    };
    let EntityPayload::VoiceSlot(slot) = &mut slot_entity.payload else {
        unreachable!("VoiceSlot kind was resolved above")
    };
    slot.candidates.remove(candidate_position);
    if selection_cleared {
        slot.selected = None;
    }
    slot_entity.revision = next_slot_revision;
    if take_entity_removed {
        project.entities.remove(&request.take_id);
    }
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3VoiceTakeRemovalConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(
                Revision3VoiceTakeRemovalConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3VoiceTakeRemovalErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3VoiceTakeRemovalErrorV1::CanonicalReopenMismatch);
    }
    if !preserves_exact_basis(
        &basis_project,
        &reopened,
        &request,
        candidate_position,
        selection_cleared,
        take_entity_removed,
    ) {
        return Err(Revision3VoiceTakeRemovalErrorV1::CandidatePreservationMismatch);
    }

    Ok(Revision3VoiceTakeRemovalEvaluationV1::Applied(Box::new(
        Revision3VoiceTakeRemovalOutcomeV1 {
            project: reopened,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            line_id: request.line_id,
            localization_id: request.localization_id,
            slot_id: request.slot_id,
            slot_revision: next_slot_revision,
            locale: request.locale,
            loc_id: request.expected_loc_id,
            take_id: request.take_id,
            take_revision: request.expected_take_revision,
            previous_selected_take_id: current_selected_take_id,
            selection_cleared,
            take_entity_removed,
            remaining_candidate_count,
            build_status: Revision3VoiceTakeRemovalBuildStatusV1::Blocked,
            runtime_status: Revision3VoiceTakeRemovalRuntimeStatusV1::RuntimeUnqualified,
        },
    )))
}

fn exact_localization(
    project: &ProjectRevision3,
    line: &DialogLine,
    localization_id: EntityId,
) -> Option<String> {
    if line.localization.project_id != project.project_id
        || line.localization.expected_kind != EntityKind::LocalizationEntry
        || line.localization.id != localization_id
    {
        return None;
    }
    let entity = project.entities.get(&localization_id)?;
    let EntityPayload::LocalizationEntry(LocalizationEntry { loc_id, .. }) = &entity.payload else {
        return None;
    };
    Some(loc_id.clone())
}

fn has_unique_slot_owner(
    project: &ProjectRevision3,
    expected_line: EntityId,
    expected_locale: &LocaleCode,
    slot_id: EntityId,
) -> bool {
    let mut owner = None;
    for (line_id, entity) in &project.entities {
        let EntityPayload::DialogLine(line) = &entity.payload else {
            continue;
        };
        for (locale, reference) in &line.voice_slots {
            if reference.project_id == project.project_id
                && reference.expected_kind == EntityKind::VoiceSlot
                && reference.id == slot_id
            {
                if owner.replace((*line_id, locale)).is_some() {
                    return false;
                }
            }
        }
    }
    matches!(owner, Some((line, locale)) if line == expected_line && locale == expected_locale)
}

#[derive(Debug)]
struct TakeUsage {
    candidate_slots: BTreeSet<EntityId>,
}

#[derive(Debug)]
struct BacklinkBlocker {
    source_entity: EntityId,
    role: Revision3ContentReferenceRoleV1,
    reason: String,
}

fn validate_take_backlinks(
    project: &ProjectRevision3,
    index: &crate::Revision3ContentIndexV1,
    request: &Revision3VoiceTakeRemovalRequestV1,
) -> Result<TakeUsage, BacklinkBlocker> {
    let mut counts = BTreeMap::<EntityId, (u8, u8)>::new();

    for source in &index.entities {
        for reference in &source.references {
            if reference.target.entity_id != request.take_id
                || reference.target.project_id != project.project_id
            {
                // Foreign-project references with the same 128-bit ID are not local backlinks.
                continue;
            }
            let invalid = |reason: &str| BacklinkBlocker {
                source_entity: source.id,
                role: reference.role,
                reason: reason.to_owned(),
            };
            if reference.target.expected_kind != EntityKind::VoiceTake
                || reference.resolution != Revision3ContentReferenceResolutionV1::Resolved
                || reference.qualifier.is_some()
            {
                return Err(invalid(
                    "reference is kind-mismatched, unresolved, or unexpectedly qualified",
                ));
            }
            if !matches!(
                source.summary,
                crate::Revision3ContentEntitySummaryV1::VoiceSlot { .. }
            ) {
                return Err(invalid(
                    "Voice backlink does not originate from a VoiceSlot",
                ));
            }
            let entry = counts.entry(source.id).or_default();
            match reference.role {
                Revision3ContentReferenceRoleV1::VoiceCandidate => {
                    entry.0 = entry.0.saturating_add(1)
                }
                Revision3ContentReferenceRoleV1::VoiceSelected => {
                    entry.1 = entry.1.saturating_add(1)
                }
                _ => {
                    return Err(invalid(
                        "reference role is not VoiceCandidate or VoiceSelected",
                    ))
                }
            }
        }
    }

    for (&source_entity, &(candidate_count, selected_count)) in &counts {
        if candidate_count != 1 || selected_count > 1 {
            return Err(BacklinkBlocker {
                source_entity,
                role: if selected_count > 1 {
                    Revision3ContentReferenceRoleV1::VoiceSelected
                } else {
                    Revision3ContentReferenceRoleV1::VoiceCandidate
                },
                reason: "candidate/selection usage is inconsistent".to_owned(),
            });
        }
        let Some(source) = project.entities.get(&source_entity) else {
            return Err(BacklinkBlocker {
                source_entity,
                role: Revision3ContentReferenceRoleV1::VoiceCandidate,
                reason: "source VoiceSlot is missing".to_owned(),
            });
        };
        let EntityPayload::VoiceSlot(slot) = &source.payload else {
            return Err(BacklinkBlocker {
                source_entity,
                role: Revision3ContentReferenceRoleV1::VoiceCandidate,
                reason: "source entity is not a VoiceSlot".to_owned(),
            });
        };
        let payload_selected =
            slot.selected.as_ref().map(|reference| reference.id) == Some(request.take_id);
        if (selected_count == 1) != payload_selected {
            return Err(BacklinkBlocker {
                source_entity,
                role: Revision3ContentReferenceRoleV1::VoiceSelected,
                reason: "content index and VoiceSlot selection disagree".to_owned(),
            });
        }
    }

    let Some(&(requested_candidates, requested_selected)) = counts.get(&request.slot_id) else {
        return Err(BacklinkBlocker {
            source_entity: request.slot_id,
            role: Revision3ContentReferenceRoleV1::VoiceCandidate,
            reason: "requested VoiceSlot has no indexed candidate backlink".to_owned(),
        });
    };
    let expected_selected = u8::from(request.expected_selected_take_id == Some(request.take_id));
    if requested_candidates != 1 || requested_selected != expected_selected {
        return Err(BacklinkBlocker {
            source_entity: request.slot_id,
            role: Revision3ContentReferenceRoleV1::VoiceSelected,
            reason: "requested candidate/selection edges do not match the bound slot".to_owned(),
        });
    }

    Ok(TakeUsage {
        candidate_slots: counts.keys().copied().collect(),
    })
}

fn preserves_exact_basis(
    basis: &ProjectRevision3,
    candidate: &ProjectRevision3,
    request: &Revision3VoiceTakeRemovalRequestV1,
    candidate_position: usize,
    selection_cleared: bool,
    take_entity_removed: bool,
) -> bool {
    if candidate.revision != basis.revision.saturating_add(1)
        || candidate.project_id != basis.project_id
        || candidate.meta != basis.meta
        || candidate.target != basis.target
        || candidate.authoring_locales != basis.authoring_locales
        || candidate.asset_store != basis.asset_store
        || candidate.entities.len()
            != basis
                .entities
                .len()
                .saturating_sub(usize::from(take_entity_removed))
    {
        return false;
    }

    for (id, entity) in &basis.entities {
        if *id == request.take_id {
            if take_entity_removed {
                if candidate.entities.contains_key(id) {
                    return false;
                }
            } else if candidate.entities.get(id) != Some(entity) {
                return false;
            }
            continue;
        }
        if *id == request.slot_id {
            let mut expected = entity.clone();
            let Some(next_revision) = expected.revision.checked_add(1) else {
                return false;
            };
            let EntityPayload::VoiceSlot(slot) = &mut expected.payload else {
                return false;
            };
            if candidate_position >= slot.candidates.len() {
                return false;
            }
            slot.candidates.remove(candidate_position);
            if selection_cleared {
                slot.selected = None;
            }
            expected.revision = next_revision;
            if candidate.entities.get(id) != Some(&expected) {
                return false;
            }
            continue;
        }
        if candidate.entities.get(id) != Some(entity) {
            return false;
        }
    }
    true
}

fn distinct_nonzero_entity_ids(ids: [EntityId; 4]) -> bool {
    ids.iter()
        .all(|id| id.as_bytes().iter().any(|byte| *byte != 0))
        && ids.into_iter().collect::<BTreeSet<_>>().len() == 4
}

struct BoundedRequestWriter {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedRequestWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedRequestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let new_len = self.bytes.len().saturating_add(bytes.len());
        if new_len > self.limit {
            self.first_exceeded_size.get_or_insert(new_len);
            return Err(io::Error::other(
                "revision-3 Voice take removal request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_identity_requires_four_distinct_nonzero_ids() {
        let one = EntityId::from_bytes([1; 16]);
        let two = EntityId::from_bytes([2; 16]);
        let three = EntityId::from_bytes([3; 16]);
        let four = EntityId::from_bytes([4; 16]);
        let zero = EntityId::from_bytes([0; 16]);
        assert!(distinct_nonzero_entity_ids([one, two, three, four]));
        assert!(!distinct_nonzero_entity_ids([one, two, three, one]));
        assert!(!distinct_nonzero_entity_ids([one, two, three, zero]));
    }
}
