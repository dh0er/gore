//! Exact-head-bound, filesystem-free review-status editing for one revision-3 Voice take.
//!
//! This transaction changes only one uniquely owned `VoiceTake` status plus the project and take
//! revisions. Its line, localization, slot, candidate relationship, selection, Ogg/AssetStore
//! evidence, and every other project byte remain unchanged. It never reads media, a game
//! installation, or a save and grants no build, publication, deployment, or runtime authority.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    EntityKind, EntityPayload, LocalizationEntry, VoiceSlot, VoiceTakeStatus,
};
use crate::story_transaction_revision3_voice_target::validate_revision3_voice_loc_id_basename_stem_v1;
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    EntityId, GameGenerationAnchor, LocaleCode, ProjectId, ProjectRevision3,
    ProjectRevision3JsonError, WorkingHead,
};

pub const MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1: usize = 64 * 1024;

/// Exact project/head/graph-CAS-bound intent for changing one retained take's review status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3VoiceTakeStatusEditRequestV1 {
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
    pub expected_status: VoiceTakeStatus,
    pub desired_status: VoiceTakeStatus,
}

impl Revision3VoiceTakeStatusEditRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3VoiceTakeStatusEditRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1 {
            return Err(
                Revision3VoiceTakeStatusEditRequestJsonErrorV1::InputTooLarge {
                    actual: json.len(),
                    limit: MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(
        &self,
    ) -> Result<String, Revision3VoiceTakeStatusEditRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(
                Revision3VoiceTakeStatusEditRequestJsonErrorV1::InputTooLarge {
                    actual,
                    limit: MAX_REVISION3_VOICE_TAKE_STATUS_EDIT_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        serialized.map_err(Revision3VoiceTakeStatusEditRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3VoiceTakeStatusEditRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTakeStatusEditRequestJsonErrorV1 {
    #[error(
        "revision-3 Voice take status-edit request exceeds the {limit}-byte limit: {actual} bytes"
    )]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Voice take status-edit request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Voice take status-edit request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Voice take status-edit request is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Voice take status-edit request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Stable semantic conflict. Rejection never exposes a partially changed project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3VoiceTakeStatusEditConflictV1 {
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
    #[error("line, localization, VoiceSlot, and VoiceTake IDs must be non-zero and distinct")]
    InvalidEntityIdentity,
    #[error("expected localization ID is not one bounded portable Voice basename stem")]
    InvalidExpectedLocId,
    #[error("DialogLine entity {line} is missing or has the wrong kind")]
    InvalidDialogLine { line: EntityId },
    #[error(
        "DialogLine {line} does not reference the requested exact-project LocalizationEntry {localization}"
    )]
    LocalizationReferenceMismatch {
        line: EntityId,
        localization: EntityId,
    },
    #[error("LocalizationEntry {localization} is missing or has the wrong kind")]
    InvalidLocalization { localization: EntityId },
    #[error("expected localization identity {expected}, but exact basis is {actual}")]
    LocalizationIdentityMismatch { expected: String, actual: String },
    #[error("line/locale is not linked to requested VoiceSlot {slot}")]
    VoiceSlotIdentityMismatch { slot: EntityId },
    #[error("VoiceSlot {slot} is missing, has the wrong kind, locale, or unique owner")]
    InvalidVoiceSlot { slot: EntityId },
    #[error("expected VoiceSlot revision {expected}, but exact basis is {actual}")]
    VoiceSlotRevisionConflict { expected: u64, actual: u64 },
    #[error("VoiceTake {take} is not an exact candidate of requested VoiceSlot {slot}")]
    VoiceTakeNotCandidate { slot: EntityId, take: EntityId },
    #[error("VoiceTake {take} is retained by more than one VoiceSlot")]
    SharedVoiceTake { take: EntityId },
    #[error("VoiceTake {take} is missing or has the wrong kind")]
    InvalidVoiceTake { take: EntityId },
    #[error("VoiceTake {take} locale differs from the requested VoiceSlot locale")]
    VoiceTakeLocaleMismatch { take: EntityId },
    #[error("expected VoiceTake revision {expected}, but exact basis is {actual}")]
    VoiceTakeRevisionConflict { expected: u64, actual: u64 },
    #[error("VoiceTake {take} revision cannot be incremented")]
    VoiceTakeRevisionOverflow { take: EntityId },
    #[error("expected VoiceTake status {expected:?}, but exact basis is {actual:?}")]
    CurrentStatusMismatch {
        expected: VoiceTakeStatus,
        actual: VoiceTakeStatus,
    },
    #[error("Voice take status edit does not change the requested VoiceTake")]
    NoChanges,
    #[error("selected VoiceTake {take} cannot become non-Approved; clear its selection first")]
    SelectedTakeCannotBecomeUnapproved { take: EntityId },
    #[error(
        "Voice take status-edit candidate exceeds the {limit}-byte project limit: {actual} bytes"
    )]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeStatusEditRejectionV1 {
    pub conflict: Revision3VoiceTakeStatusEditConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceTakeStatusEditBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceTakeStatusEditRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeStatusEditOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub line_id: EntityId,
    pub localization_id: EntityId,
    pub slot_id: EntityId,
    pub slot_revision: u64,
    pub take_id: EntityId,
    pub take_revision: u64,
    pub locale: LocaleCode,
    pub loc_id: String,
    pub previous_status: VoiceTakeStatus,
    pub status: VoiceTakeStatus,
    pub build_status: Revision3VoiceTakeStatusEditBuildStatusV1,
    pub runtime_status: Revision3VoiceTakeStatusEditRuntimeStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3VoiceTakeStatusEditEvaluationV1 {
    Applied(Box<Revision3VoiceTakeStatusEditOutcomeV1>),
    Rejected(Revision3VoiceTakeStatusEditRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTakeStatusEditErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 Voice take status-edit request: {0}")]
    InvalidRequest(#[source] Revision3VoiceTakeStatusEditRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 Voice take status-edit candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Change the author-managed review status of one uniquely owned retained Voice take.
///
/// This pure semantic transaction performs no filesystem or media access. It cannot add, remove,
/// relink, select, or otherwise change a take, alter target evidence or AssetStore bytes, grant
/// build authority, publish a fixed head, deploy, or touch a game/save installation.
pub fn apply_revision3_voice_take_status_edit_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3VoiceTakeStatusEditEvaluationV1, Revision3VoiceTakeStatusEditErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3VoiceTakeStatusEditErrorV1::InvalidProject)?;
    let request = Revision3VoiceTakeStatusEditRequestV1::from_json(canonical_request_json)
        .map_err(Revision3VoiceTakeStatusEditErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3VoiceTakeStatusEditEvaluationV1::Rejected(
                Revision3VoiceTakeStatusEditRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3VoiceTakeStatusEditConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3VoiceTakeStatusEditConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3VoiceTakeStatusEditConflictV1::ProjectRevisionOverflow);
    };
    let identities = [
        request.line_id,
        request.localization_id,
        request.slot_id,
        request.take_id,
    ];
    if identities.iter().copied().any(is_zero_entity_id)
        || identities
            .iter()
            .enumerate()
            .any(|(index, id)| identities[index + 1..].contains(id))
    {
        reject!(Revision3VoiceTakeStatusEditConflictV1::InvalidEntityIdentity);
    }
    if validate_revision3_voice_loc_id_basename_stem_v1(&request.expected_loc_id).is_err() {
        reject!(Revision3VoiceTakeStatusEditConflictV1::InvalidExpectedLocId);
    }

    let Some(line_entity) = project.entities.get(&request.line_id) else {
        reject!(Revision3VoiceTakeStatusEditConflictV1::InvalidDialogLine {
            line: request.line_id,
        });
    };
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        reject!(Revision3VoiceTakeStatusEditConflictV1::InvalidDialogLine {
            line: request.line_id,
        });
    };
    if line.localization.project_id != project.project_id
        || line.localization.expected_kind != EntityKind::LocalizationEntry
        || line.localization.id != request.localization_id
    {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::LocalizationReferenceMismatch {
                line: request.line_id,
                localization: request.localization_id,
            }
        );
    }
    let Some(localization_entity) = project.entities.get(&request.localization_id) else {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::InvalidLocalization {
                localization: request.localization_id,
            }
        );
    };
    let EntityPayload::LocalizationEntry(LocalizationEntry { loc_id, .. }) =
        &localization_entity.payload
    else {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::InvalidLocalization {
                localization: request.localization_id,
            }
        );
    };
    if loc_id != &request.expected_loc_id {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::LocalizationIdentityMismatch {
                expected: request.expected_loc_id,
                actual: loc_id.clone(),
            }
        );
    }
    let Some(slot_ref) = line.voice_slots.get(&request.locale) else {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::VoiceSlotIdentityMismatch {
                slot: request.slot_id,
            }
        );
    };
    if slot_ref.project_id != project.project_id
        || slot_ref.expected_kind != EntityKind::VoiceSlot
        || slot_ref.id != request.slot_id
    {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::VoiceSlotIdentityMismatch {
                slot: request.slot_id,
            }
        );
    }
    if !has_unique_slot_owner(&project, request.line_id, &request.locale, request.slot_id) {
        reject!(Revision3VoiceTakeStatusEditConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    }

    let Some(slot_entity) = project.entities.get(&request.slot_id) else {
        reject!(Revision3VoiceTakeStatusEditConflictV1::InvalidVoiceSlot {
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
        reject!(Revision3VoiceTakeStatusEditConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    };
    if locale != &request.locale {
        reject!(Revision3VoiceTakeStatusEditConflictV1::InvalidVoiceSlot {
            slot: request.slot_id,
        });
    }
    if request.expected_slot_revision != slot_entity.revision {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::VoiceSlotRevisionConflict {
                expected: request.expected_slot_revision,
                actual: slot_entity.revision,
            }
        );
    }
    let candidate_count = candidates
        .iter()
        .filter(|candidate| {
            candidate.project_id == project.project_id
                && candidate.expected_kind == EntityKind::VoiceTake
                && candidate.id == request.take_id
        })
        .count();
    if candidate_count != 1 {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::VoiceTakeNotCandidate {
                slot: request.slot_id,
                take: request.take_id,
            }
        );
    }
    if !has_unique_take_owner(&project, request.slot_id, request.take_id) {
        reject!(Revision3VoiceTakeStatusEditConflictV1::SharedVoiceTake {
            take: request.take_id,
        });
    }
    let take_is_selected = selected.as_ref().is_some_and(|reference| {
        reference.project_id == project.project_id
            && reference.expected_kind == EntityKind::VoiceTake
            && reference.id == request.take_id
    });

    let Some(take_entity) = project.entities.get(&request.take_id) else {
        reject!(Revision3VoiceTakeStatusEditConflictV1::InvalidVoiceTake {
            take: request.take_id,
        });
    };
    let EntityPayload::VoiceTake(take) = &take_entity.payload else {
        reject!(Revision3VoiceTakeStatusEditConflictV1::InvalidVoiceTake {
            take: request.take_id,
        });
    };
    if take.locale != request.locale {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::VoiceTakeLocaleMismatch {
                take: request.take_id,
            }
        );
    }
    if request.expected_take_revision != take_entity.revision {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::VoiceTakeRevisionConflict {
                expected: request.expected_take_revision,
                actual: take_entity.revision,
            }
        );
    }
    let Some(next_take_revision) = take_entity.revision.checked_add(1) else {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::VoiceTakeRevisionOverflow {
                take: request.take_id,
            }
        );
    };
    if request.expected_status != take.status {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::CurrentStatusMismatch {
                expected: request.expected_status,
                actual: take.status,
            }
        );
    }
    if request.desired_status == take.status {
        reject!(Revision3VoiceTakeStatusEditConflictV1::NoChanges);
    }
    if take_is_selected && request.desired_status != VoiceTakeStatus::Approved {
        reject!(
            Revision3VoiceTakeStatusEditConflictV1::SelectedTakeCannotBecomeUnapproved {
                take: request.take_id,
            }
        );
    }

    let Some(take_entity) = project.entities.get_mut(&request.take_id) else {
        unreachable!("VoiceTake was resolved above")
    };
    let EntityPayload::VoiceTake(take) = &mut take_entity.payload else {
        unreachable!("VoiceTake kind was resolved above")
    };
    let previous_status = take.status;
    take.status = request.desired_status;
    take_entity.revision = next_take_revision;
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3VoiceTakeStatusEditConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(
                Revision3VoiceTakeStatusEditConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3VoiceTakeStatusEditErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3VoiceTakeStatusEditErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3VoiceTakeStatusEditEvaluationV1::Applied(Box::new(
        Revision3VoiceTakeStatusEditOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            line_id: request.line_id,
            localization_id: request.localization_id,
            slot_id: request.slot_id,
            slot_revision: request.expected_slot_revision,
            take_id: request.take_id,
            take_revision: next_take_revision,
            locale: request.locale,
            loc_id: request.expected_loc_id,
            previous_status,
            status: request.desired_status,
            build_status: Revision3VoiceTakeStatusEditBuildStatusV1::Blocked,
            runtime_status: Revision3VoiceTakeStatusEditRuntimeStatusV1::RuntimeUnqualified,
        },
    )))
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
            if reference.project_id != project.project_id
                || reference.expected_kind != EntityKind::VoiceSlot
                || reference.id != slot_id
            {
                continue;
            }
            if owner.replace((*line_id, locale)).is_some() {
                return false;
            }
        }
    }
    matches!(owner, Some((line, locale)) if line == expected_line && locale == expected_locale)
}

fn has_unique_take_owner(
    project: &ProjectRevision3,
    expected_slot: EntityId,
    take_id: EntityId,
) -> bool {
    let mut owner = None;
    for (slot_id, entity) in &project.entities {
        let EntityPayload::VoiceSlot(slot) = &entity.payload else {
            continue;
        };
        for candidate in &slot.candidates {
            if candidate.project_id != project.project_id
                || candidate.expected_kind != EntityKind::VoiceTake
                || candidate.id != take_id
            {
                continue;
            }
            if owner.replace(*slot_id).is_some() {
                return false;
            }
        }
    }
    owner == Some(expected_slot)
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
            bytes: Vec::with_capacity(limit.min(16 * 1024)),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedRequestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let actual = self.bytes.len().saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other(
                "revision-3 Voice take status-edit request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
