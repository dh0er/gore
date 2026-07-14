//! Exact-head-bound, filesystem-free selection of one existing revision-3 Voice take.
//!
//! This transaction changes only the selected-take reference of one existing `VoiceSlot`.
//! Candidate order, target evidence, every `VoiceTake`, and the complete `AssetStore` remain
//! unchanged. It never reads media, the game installation, or a save, and grants no build,
//! publication, deployment, or runtime authority.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    DialogLine, EntityKind, EntityPayload, LocalizationEntry, VoiceSlot, VoiceTakeStatus,
};
use crate::story_transaction_revision3_voice_target::validate_revision3_voice_loc_id_basename_stem_v1;
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    EntityId, GameGenerationAnchor, LocaleCode, ProjectId, ProjectRevision3,
    ProjectRevision3JsonError, WorkingHead,
};

pub const MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1: usize = 64 * 1024;

/// Exact project/head/slot-CAS-bound selection of one already-retained Voice take, or `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3VoiceTakeSelectionRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub line_id: EntityId,
    pub slot_id: EntityId,
    pub expected_slot_revision: u64,
    pub locale: LocaleCode,
    pub expected_loc_id: String,
    pub expected_selected_take_id: Option<EntityId>,
    pub selected_take_id: Option<EntityId>,
}

impl Revision3VoiceTakeSelectionRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3VoiceTakeSelectionRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1 {
            return Err(
                Revision3VoiceTakeSelectionRequestJsonErrorV1::InputTooLarge {
                    actual: json.len(),
                    limit: MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3VoiceTakeSelectionRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3VoiceTakeSelectionRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3VoiceTakeSelectionRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(
        &self,
    ) -> Result<String, Revision3VoiceTakeSelectionRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(
                Revision3VoiceTakeSelectionRequestJsonErrorV1::InputTooLarge {
                    actual,
                    limit: MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        serialized.map_err(Revision3VoiceTakeSelectionRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3VoiceTakeSelectionRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTakeSelectionRequestJsonErrorV1 {
    #[error(
        "revision-3 Voice take selection request exceeds the {limit}-byte limit: {actual} bytes"
    )]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Voice take selection request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Voice take selection request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Voice take selection request is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Voice take selection request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Stable semantic conflict. Rejection never exposes a partially changed project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3VoiceTakeSelectionConflictV1 {
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
    #[error("DialogLine and VoiceSlot IDs must be non-zero and different")]
    InvalidEntityIdentity,
    #[error("VoiceTake entity ID {take} must be non-zero and distinct from line and slot IDs")]
    InvalidTakeIdentity { take: EntityId },
    #[error("expected localization ID is not one bounded portable Voice basename stem")]
    InvalidExpectedLocId,
    #[error("DialogLine entity {line} is missing or has the wrong kind")]
    InvalidDialogLine { line: EntityId },
    #[error("DialogLine {line} does not own one exact local LocalizationEntry")]
    InvalidLocalizationReference { line: EntityId },
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
    #[error("expected selected VoiceTake {expected:?}, but exact basis has {actual:?}")]
    CurrentSelectionMismatch {
        expected: Option<EntityId>,
        actual: Option<EntityId>,
    },
    #[error("VoiceTake {take} is not an exact candidate of the requested VoiceSlot")]
    SelectedTakeNotCandidate { take: EntityId },
    #[error("selected VoiceTake {take} is missing or has the wrong kind")]
    InvalidSelectedTake { take: EntityId },
    #[error("selected VoiceTake {take} locale differs from the requested VoiceSlot locale")]
    SelectedTakeLocaleMismatch { take: EntityId },
    #[error("selected VoiceTake {take} is not Approved")]
    SelectedTakeNotApproved { take: EntityId },
    #[error("Voice take selection does not change the requested VoiceSlot")]
    NoChanges,
    #[error(
        "Voice take selection candidate exceeds the {limit}-byte project limit: {actual} bytes"
    )]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeSelectionRejectionV1 {
    pub conflict: Revision3VoiceTakeSelectionConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceTakeSelectionBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceTakeSelectionRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeSelectionOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub line_id: EntityId,
    pub localization_id: EntityId,
    pub slot_id: EntityId,
    pub slot_revision: u64,
    pub locale: LocaleCode,
    pub loc_id: String,
    pub previous_selected_take_id: Option<EntityId>,
    pub selected_take_id: Option<EntityId>,
    pub build_status: Revision3VoiceTakeSelectionBuildStatusV1,
    pub runtime_status: Revision3VoiceTakeSelectionRuntimeStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3VoiceTakeSelectionEvaluationV1 {
    Applied(Box<Revision3VoiceTakeSelectionOutcomeV1>),
    Rejected(Revision3VoiceTakeSelectionRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTakeSelectionErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 Voice take selection request: {0}")]
    InvalidRequest(#[source] Revision3VoiceTakeSelectionRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 Voice take selection candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Select one already-retained Approved take for one exact Voice slot, or clear its selection.
///
/// This pure semantic transaction performs no filesystem or media access. It cannot add, remove,
/// relink, or change a take, alter target evidence, grant build authority, publish a fixed head,
/// deploy, or touch a game/save installation.
pub fn apply_revision3_voice_take_selection_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3VoiceTakeSelectionEvaluationV1, Revision3VoiceTakeSelectionErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3VoiceTakeSelectionErrorV1::InvalidProject)?;
    let request = Revision3VoiceTakeSelectionRequestV1::from_json(canonical_request_json)
        .map_err(Revision3VoiceTakeSelectionErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3VoiceTakeSelectionEvaluationV1::Rejected(
                Revision3VoiceTakeSelectionRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3VoiceTakeSelectionConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3VoiceTakeSelectionConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3VoiceTakeSelectionConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3VoiceTakeSelectionConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3VoiceTakeSelectionConflictV1::ProjectRevisionOverflow);
    };
    if is_zero_entity_id(request.line_id)
        || is_zero_entity_id(request.slot_id)
        || request.line_id == request.slot_id
    {
        reject!(Revision3VoiceTakeSelectionConflictV1::InvalidEntityIdentity);
    }
    for take in [request.expected_selected_take_id, request.selected_take_id]
        .into_iter()
        .flatten()
    {
        if is_zero_entity_id(take) || take == request.line_id || take == request.slot_id {
            reject!(Revision3VoiceTakeSelectionConflictV1::InvalidTakeIdentity { take });
        }
    }
    if validate_revision3_voice_loc_id_basename_stem_v1(&request.expected_loc_id).is_err() {
        reject!(Revision3VoiceTakeSelectionConflictV1::InvalidExpectedLocId);
    }

    let Some(line_entity) = project.entities.get(&request.line_id) else {
        reject!(Revision3VoiceTakeSelectionConflictV1::InvalidDialogLine {
            line: request.line_id,
        });
    };
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        reject!(Revision3VoiceTakeSelectionConflictV1::InvalidDialogLine {
            line: request.line_id,
        });
    };
    let Some((localization_id, loc_id)) = exact_localization(&project, line) else {
        reject!(
            Revision3VoiceTakeSelectionConflictV1::InvalidLocalizationReference {
                line: request.line_id,
            }
        );
    };
    if loc_id != request.expected_loc_id {
        reject!(
            Revision3VoiceTakeSelectionConflictV1::LocalizationIdentityMismatch {
                expected: request.expected_loc_id,
                actual: loc_id,
            }
        );
    }
    let Some(slot_ref) = line.voice_slots.get(&request.locale) else {
        reject!(
            Revision3VoiceTakeSelectionConflictV1::VoiceSlotIdentityMismatch {
                slot: request.slot_id,
            }
        );
    };
    if slot_ref.project_id != project.project_id
        || slot_ref.expected_kind != EntityKind::VoiceSlot
        || slot_ref.id != request.slot_id
    {
        reject!(
            Revision3VoiceTakeSelectionConflictV1::VoiceSlotIdentityMismatch {
                slot: request.slot_id,
            }
        );
    }
    if !has_unique_slot_owner(&project, request.line_id, &request.locale, request.slot_id) {
        reject!(Revision3VoiceTakeSelectionConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    }

    let Some(slot_entity) = project.entities.get(&request.slot_id) else {
        reject!(Revision3VoiceTakeSelectionConflictV1::InvalidVoiceSlot {
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
        reject!(Revision3VoiceTakeSelectionConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    };
    if locale != &request.locale {
        reject!(Revision3VoiceTakeSelectionConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    }
    if request.expected_slot_revision != slot_entity.revision {
        reject!(
            Revision3VoiceTakeSelectionConflictV1::VoiceSlotRevisionConflict {
                expected: request.expected_slot_revision,
                actual: slot_entity.revision,
            }
        );
    }
    let Some(next_slot_revision) = slot_entity.revision.checked_add(1) else {
        reject!(
            Revision3VoiceTakeSelectionConflictV1::VoiceSlotRevisionOverflow {
                slot: request.slot_id,
            }
        );
    };
    let current_selected_take_id = selected.as_ref().map(|reference| reference.id);
    if request.expected_selected_take_id != current_selected_take_id {
        reject!(
            Revision3VoiceTakeSelectionConflictV1::CurrentSelectionMismatch {
                expected: request.expected_selected_take_id,
                actual: current_selected_take_id,
            }
        );
    }
    if request.selected_take_id == current_selected_take_id {
        reject!(Revision3VoiceTakeSelectionConflictV1::NoChanges);
    }

    let selected_reference = if let Some(selected_take_id) = request.selected_take_id {
        let Some(candidate) = candidates.iter().find(|candidate| {
            candidate.project_id == project.project_id
                && candidate.expected_kind == EntityKind::VoiceTake
                && candidate.id == selected_take_id
        }) else {
            reject!(
                Revision3VoiceTakeSelectionConflictV1::SelectedTakeNotCandidate {
                    take: selected_take_id,
                }
            );
        };
        let Some(take_entity) = project.entities.get(&selected_take_id) else {
            reject!(Revision3VoiceTakeSelectionConflictV1::InvalidSelectedTake {
                take: selected_take_id,
            });
        };
        let EntityPayload::VoiceTake(take) = &take_entity.payload else {
            reject!(Revision3VoiceTakeSelectionConflictV1::InvalidSelectedTake {
                take: selected_take_id,
            });
        };
        if take.locale != request.locale {
            reject!(
                Revision3VoiceTakeSelectionConflictV1::SelectedTakeLocaleMismatch {
                    take: selected_take_id,
                }
            );
        }
        if take.status != VoiceTakeStatus::Approved {
            reject!(
                Revision3VoiceTakeSelectionConflictV1::SelectedTakeNotApproved {
                    take: selected_take_id,
                }
            );
        }
        Some(candidate.clone())
    } else {
        None
    };

    let Some(slot_entity) = project.entities.get_mut(&request.slot_id) else {
        unreachable!("VoiceSlot was resolved above")
    };
    let EntityPayload::VoiceSlot(slot) = &mut slot_entity.payload else {
        unreachable!("VoiceSlot kind was resolved above")
    };
    slot.selected = selected_reference;
    slot_entity.revision = next_slot_revision;
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3VoiceTakeSelectionConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(
                Revision3VoiceTakeSelectionConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3VoiceTakeSelectionErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3VoiceTakeSelectionErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3VoiceTakeSelectionEvaluationV1::Applied(Box::new(
        Revision3VoiceTakeSelectionOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            line_id: request.line_id,
            localization_id,
            slot_id: request.slot_id,
            slot_revision: next_slot_revision,
            locale: request.locale,
            loc_id: request.expected_loc_id,
            previous_selected_take_id: current_selected_take_id,
            selected_take_id: request.selected_take_id,
            build_status: Revision3VoiceTakeSelectionBuildStatusV1::Blocked,
            runtime_status: Revision3VoiceTakeSelectionRuntimeStatusV1::RuntimeUnqualified,
        },
    )))
}

fn exact_localization(project: &ProjectRevision3, line: &DialogLine) -> Option<(EntityId, String)> {
    if line.localization.project_id != project.project_id
        || line.localization.expected_kind != EntityKind::LocalizationEntry
    {
        return None;
    }
    let entity = project.entities.get(&line.localization.id)?;
    let EntityPayload::LocalizationEntry(LocalizationEntry { loc_id, .. }) = &entity.payload else {
        return None;
    };
    Some((line.localization.id, loc_id.clone()))
}

fn has_unique_slot_owner(
    project: &ProjectRevision3,
    expected_line: EntityId,
    expected_locale: &LocaleCode,
    slot_id: EntityId,
) -> bool {
    // Count every line/locale edge. `find_map` per line would miss a second alias from another
    // locale of the same line and could incorrectly treat that shared slot as uniquely owned.
    let owners = project
        .entities
        .iter()
        .filter_map(|(line_id, entity)| {
            let EntityPayload::DialogLine(line) = &entity.payload else {
                return None;
            };
            Some((*line_id, line))
        })
        .flat_map(|(line_id, line)| {
            line.voice_slots
                .iter()
                .filter_map(move |(locale, reference)| {
                    (reference.project_id == project.project_id
                        && reference.expected_kind == EntityKind::VoiceSlot
                        && reference.id == slot_id)
                        .then_some((line_id, locale))
                })
        });
    exactly_one_expected_owner(owners, expected_line, expected_locale)
}

fn exactly_one_expected_owner<'a>(
    owners: impl Iterator<Item = (EntityId, &'a LocaleCode)>,
    expected_line: EntityId,
    expected_locale: &LocaleCode,
) -> bool {
    let mut owner = None;
    for candidate in owners {
        if owner.replace(candidate).is_some() {
            return false;
        }
    }
    matches!(owner, Some((line, locale)) if line == expected_line && locale == expected_locale)
}

fn is_zero_entity_id(id: EntityId) -> bool {
    id.as_bytes().iter().all(|byte| *byte == 0)
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
                "revision-3 Voice take selection request JSON limit exceeded",
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
    fn same_line_different_locale_alias_is_not_unique_ownership() {
        let line = EntityId::from_bytes([1; 16]);
        let de: LocaleCode = "de".parse().unwrap();
        let en: LocaleCode = "en".parse().unwrap();
        assert!(!exactly_one_expected_owner(
            [(line, &de), (line, &en)].into_iter(),
            line,
            &de,
        ));
        assert!(exactly_one_expected_owner(
            [(line, &de)].into_iter(),
            line,
            &de,
        ));
    }
}
