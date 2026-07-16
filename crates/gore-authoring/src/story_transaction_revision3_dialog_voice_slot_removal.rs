//! Exact-basis-bound removal of one empty managed revision-3 dialog VoiceSlot.
//!
//! The transaction atomically removes the exact `DialogLine` locale edge and its uniquely owned
//! generated `VoiceSlot` entity. It never removes a take or asset, changes localization text,
//! opens a game/save installation, publishes a fixed head, or grants build/runtime/target
//! authority.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    EntityKind, EntityPayload, LocalizationEntry, OriginRef, ProjectRevision3, VoiceSlot,
    VoiceTargetResolution,
};
use crate::revision3_content_index::{
    build_revision3_content_index_v1, Revision3ContentIndexErrorV1,
    Revision3ContentReferenceResolutionV1, Revision3ContentReferenceRoleV1,
};
use crate::story_transaction_revision3_voice_target::validate_revision3_voice_loc_id_basename_stem_v1;
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    EntityId, GameGenerationAnchor, LocaleCode, ProjectId, ProjectRevision3JsonError, WorkingHead,
    REVISION3_VOICE_SLOT_GENERATOR_ID_V1, REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
};

/// Maximum exact canonical dialog VoiceSlot-removal request size.
pub const MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1: usize = 64 * 1024;

/// Exact head/project/line/slot CAS binding for removing one empty managed VoiceSlot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3DialogVoiceSlotRemovalRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub line_id: EntityId,
    pub expected_line_revision: u64,
    pub localization_id: EntityId,
    pub expected_loc_id: String,
    pub locale: LocaleCode,
    pub slot_id: EntityId,
    pub expected_slot_revision: u64,
}

impl Revision3DialogVoiceSlotRemovalRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(
        json: &str,
    ) -> Result<Self, Revision3DialogVoiceSlotRemovalRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1 {
            return Err(
                Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::InputTooLarge {
                    actual: json.len(),
                    limit: MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(
        &self,
    ) -> Result<String, Revision3DialogVoiceSlotRemovalRequestJsonErrorV1> {
        let mut writer = BoundedRequestWriter::new(
            MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1,
        );
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(
                Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::InputTooLarge {
                    actual,
                    limit: MAX_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        serialized.map_err(Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3DialogVoiceSlotRemovalRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3DialogVoiceSlotRemovalRequestJsonErrorV1 {
    #[error(
        "revision-3 dialog VoiceSlot-removal request exceeds the {limit}-byte limit: {actual} bytes"
    )]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 dialog VoiceSlot-removal request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 dialog VoiceSlot-removal request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 dialog VoiceSlot-removal request is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 dialog VoiceSlot-removal request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Stable semantic conflict. Rejection never exposes a partially changed project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3DialogVoiceSlotRemovalConflictV1 {
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
    #[error("DialogLine, LocalizationEntry, and VoiceSlot IDs must be non-zero and distinct")]
    InvalidEntityIdentity,
    #[error("expected localization ID is not one bounded portable Voice basename stem")]
    InvalidExpectedLocId,
    #[error("DialogLine entity {line} is missing or has the wrong kind")]
    InvalidDialogLine { line: EntityId },
    #[error("expected DialogLine revision {expected}, but exact basis is {actual}")]
    DialogLineRevisionConflict { expected: u64, actual: u64 },
    #[error("DialogLine {line} revision cannot be incremented")]
    DialogLineRevisionOverflow { line: EntityId },
    #[error(
        "DialogLine {line} does not bind the requested exact local LocalizationEntry {localization}"
    )]
    InvalidLocalizationReference {
        line: EntityId,
        localization: EntityId,
    },
    #[error("LocalizationEntry {localization} is missing or has the wrong kind")]
    InvalidLocalization { localization: EntityId },
    #[error("expected localization ID {expected}, but exact basis is {actual}")]
    LocalizationIdentityMismatch { expected: String, actual: String },
    #[error("line/locale is not linked to requested VoiceSlot {slot}")]
    VoiceSlotIdentityMismatch { slot: EntityId },
    #[error("VoiceSlot {slot} is missing, has the wrong kind, locale, or unique owner")]
    InvalidVoiceSlot { slot: EntityId },
    #[error("expected VoiceSlot revision {expected}, but exact basis is {actual}")]
    VoiceSlotRevisionConflict { expected: u64, actual: u64 },
    #[error("VoiceSlot {slot} is not the exact managed generated slot owned by DialogLine {line}")]
    VoiceSlotOriginMismatch { line: EntityId, slot: EntityId },
    #[error("VoiceSlot {slot} still has {candidate_count} take candidate(s)")]
    VoiceSlotHasCandidates {
        slot: EntityId,
        candidate_count: usize,
    },
    #[error("VoiceSlot {slot} still has a selected take")]
    VoiceSlotHasSelection { slot: EntityId },
    #[error(
        "VoiceSlot {slot} has an unsafe local backlink from {source_entity} through {role:?}: {reason}"
    )]
    InvalidLocalBacklink {
        slot: EntityId,
        source_entity: EntityId,
        role: Revision3ContentReferenceRoleV1,
        reason: String,
    },
    #[error("dialog VoiceSlot-removal preflight exceeds the {limit}-reference limit")]
    ReferenceLimit { limit: usize },
    #[error(
        "dialog VoiceSlot-removal candidate exceeds the {limit}-byte project limit: {actual} bytes"
    )]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("dialog VoiceSlot-removal candidate is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3DialogVoiceSlotRemovalRejectionV1 {
    pub conflict: Revision3DialogVoiceSlotRemovalConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotRemovalTargetResolutionV1 {
    Unresolved,
    Ambiguous,
    Resolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotRemovalBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotRemovalRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotRemovalTargetAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotRemovalPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3DialogVoiceSlotRemovalOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub line_id: EntityId,
    pub line_revision: u64,
    pub localization_id: EntityId,
    pub slot_id: EntityId,
    pub removed_slot_revision: u64,
    pub locale: LocaleCode,
    pub loc_id: String,
    pub removed_target_resolution: Revision3DialogVoiceSlotRemovalTargetResolutionV1,
    pub build_status: Revision3DialogVoiceSlotRemovalBuildStatusV1,
    pub runtime_status: Revision3DialogVoiceSlotRemovalRuntimeStatusV1,
    pub target_authority: Revision3DialogVoiceSlotRemovalTargetAuthorityV1,
    pub publication_status: Revision3DialogVoiceSlotRemovalPublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotRemovalEvaluationV1 {
    Applied(Box<Revision3DialogVoiceSlotRemovalOutcomeV1>),
    Rejected(Revision3DialogVoiceSlotRemovalRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3DialogVoiceSlotRemovalErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 dialog VoiceSlot-removal request: {0}")]
    InvalidRequest(#[source] Revision3DialogVoiceSlotRemovalRequestJsonErrorV1),
    #[error("could not build the exact revision-3 content index: {0}")]
    ContentIndex(#[source] Revision3ContentIndexErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 dialog VoiceSlot-removal candidate reopen changed the project")]
    CanonicalReopenMismatch,
    #[error("dialog VoiceSlot removal changed a preserved project value")]
    CandidatePreservationMismatch,
}

/// Remove one exact empty managed VoiceSlot and its exact DialogLine/locale edge.
///
/// This pure transaction performs no filesystem operation and cannot publish a fixed working
/// head. The candidate remains build-blocked, runtime-unqualified, and without target authority.
pub fn apply_revision3_dialog_voice_slot_removal_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3DialogVoiceSlotRemovalEvaluationV1, Revision3DialogVoiceSlotRemovalErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3DialogVoiceSlotRemovalErrorV1::InvalidProject)?;
    let request = Revision3DialogVoiceSlotRemovalRequestV1::from_json(canonical_request_json)
        .map_err(Revision3DialogVoiceSlotRemovalErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3DialogVoiceSlotRemovalEvaluationV1::Rejected(
                Revision3DialogVoiceSlotRemovalRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3DialogVoiceSlotRemovalConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3DialogVoiceSlotRemovalConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3DialogVoiceSlotRemovalConflictV1::ProjectRevisionOverflow);
    };
    if !distinct_nonzero_entity_ids([request.line_id, request.localization_id, request.slot_id]) {
        reject!(Revision3DialogVoiceSlotRemovalConflictV1::InvalidEntityIdentity);
    }
    if validate_revision3_voice_loc_id_basename_stem_v1(&request.expected_loc_id).is_err() {
        reject!(Revision3DialogVoiceSlotRemovalConflictV1::InvalidExpectedLocId);
    }

    let Some(line_entity) = project.entities.get(&request.line_id) else {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidDialogLine {
                line: request.line_id,
            }
        );
    };
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidDialogLine {
                line: request.line_id,
            }
        );
    };
    if line_entity.revision != request.expected_line_revision {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::DialogLineRevisionConflict {
                expected: request.expected_line_revision,
                actual: line_entity.revision,
            }
        );
    }
    let Some(next_line_revision) = line_entity.revision.checked_add(1) else {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::DialogLineRevisionOverflow {
                line: request.line_id,
            }
        );
    };
    if line.localization.project_id != project.project_id
        || line.localization.expected_kind != EntityKind::LocalizationEntry
        || line.localization.id != request.localization_id
    {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidLocalizationReference {
                line: request.line_id,
                localization: request.localization_id,
            }
        );
    }
    let Some(localization_entity) = project.entities.get(&request.localization_id) else {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidLocalization {
                localization: request.localization_id,
            }
        );
    };
    let EntityPayload::LocalizationEntry(LocalizationEntry { loc_id, .. }) =
        &localization_entity.payload
    else {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidLocalization {
                localization: request.localization_id,
            }
        );
    };
    if loc_id != &request.expected_loc_id {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::LocalizationIdentityMismatch {
                expected: request.expected_loc_id,
                actual: loc_id.clone(),
            }
        );
    }
    let Some(slot_ref) = line.voice_slots.get(&request.locale) else {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotIdentityMismatch {
                slot: request.slot_id,
            }
        );
    };
    if slot_ref.project_id != project.project_id
        || slot_ref.expected_kind != EntityKind::VoiceSlot
        || slot_ref.id != request.slot_id
    {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotIdentityMismatch {
                slot: request.slot_id,
            }
        );
    }
    if !has_unique_slot_owner(&project, request.line_id, &request.locale, request.slot_id) {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidVoiceSlot {
                slot: request.slot_id,
            }
        );
    }

    let Some(slot_entity) = project.entities.get(&request.slot_id) else {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidVoiceSlot {
                slot: request.slot_id,
            }
        );
    };
    let EntityPayload::VoiceSlot(VoiceSlot {
        locale,
        target_resolution,
        candidates,
        selected,
    }) = &slot_entity.payload
    else {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidVoiceSlot {
                slot: request.slot_id,
            }
        );
    };
    if locale != &request.locale {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidVoiceSlot {
                slot: request.slot_id,
            }
        );
    }
    if slot_entity.revision != request.expected_slot_revision {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotRevisionConflict {
                expected: request.expected_slot_revision,
                actual: slot_entity.revision,
            }
        );
    }
    if !matches!(
        &slot_entity.origin,
        OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } if generator_id == REVISION3_VOICE_SLOT_GENERATOR_ID_V1
            && *generator_version == REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1
            && owner.project_id == project.project_id
            && owner.id == request.line_id
            && owner.expected_kind == EntityKind::DialogLine
    ) {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotOriginMismatch {
                line: request.line_id,
                slot: request.slot_id,
            }
        );
    }
    if selected.is_some() {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotHasSelection {
                slot: request.slot_id,
            }
        );
    }
    if !candidates.is_empty() {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::VoiceSlotHasCandidates {
                slot: request.slot_id,
                candidate_count: candidates.len(),
            }
        );
    }
    let removed_target_resolution = target_resolution_state(target_resolution);

    let index = match build_revision3_content_index_v1(&project) {
        Ok(index) => index,
        Err(Revision3ContentIndexErrorV1::TooManyReferences { limit }) => {
            reject!(Revision3DialogVoiceSlotRemovalConflictV1::ReferenceLimit { limit });
        }
        Err(error) => return Err(Revision3DialogVoiceSlotRemovalErrorV1::ContentIndex(error)),
    };
    if let Err(blocker) = validate_slot_backlinks(&project, &index, &request) {
        reject!(
            Revision3DialogVoiceSlotRemovalConflictV1::InvalidLocalBacklink {
                slot: request.slot_id,
                source_entity: blocker.source_entity,
                role: blocker.role,
                reason: blocker.reason,
            }
        );
    }

    let basis_project = project.clone();
    let Some(line_entity) = project.entities.get_mut(&request.line_id) else {
        unreachable!("DialogLine was resolved above")
    };
    let EntityPayload::DialogLine(line) = &mut line_entity.payload else {
        unreachable!("DialogLine kind was resolved above")
    };
    let removed = line.voice_slots.remove(&request.locale);
    debug_assert_eq!(
        removed.as_ref().map(|reference| reference.id),
        Some(request.slot_id)
    );
    line_entity.revision = next_line_revision;
    let removed_slot = project.entities.remove(&request.slot_id);
    debug_assert!(removed_slot.is_some());
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3DialogVoiceSlotRemovalConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(
                Revision3DialogVoiceSlotRemovalConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3DialogVoiceSlotRemovalErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3DialogVoiceSlotRemovalErrorV1::CanonicalReopenMismatch);
    }
    if !preserves_exact_basis(&basis_project, &reopened, &request, next_line_revision) {
        return Err(Revision3DialogVoiceSlotRemovalErrorV1::CandidatePreservationMismatch);
    }

    Ok(Revision3DialogVoiceSlotRemovalEvaluationV1::Applied(
        Box::new(Revision3DialogVoiceSlotRemovalOutcomeV1 {
            project: reopened,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            line_id: request.line_id,
            line_revision: next_line_revision,
            localization_id: request.localization_id,
            slot_id: request.slot_id,
            removed_slot_revision: request.expected_slot_revision,
            locale: request.locale,
            loc_id: request.expected_loc_id,
            removed_target_resolution,
            build_status: Revision3DialogVoiceSlotRemovalBuildStatusV1::Blocked,
            runtime_status: Revision3DialogVoiceSlotRemovalRuntimeStatusV1::RuntimeUnqualified,
            target_authority: Revision3DialogVoiceSlotRemovalTargetAuthorityV1::NotGranted,
            publication_status: Revision3DialogVoiceSlotRemovalPublicationStatusV1::NotSupported,
        }),
    ))
}

fn target_resolution_state(
    resolution: &VoiceTargetResolution,
) -> Revision3DialogVoiceSlotRemovalTargetResolutionV1 {
    match resolution {
        VoiceTargetResolution::Unresolved => {
            Revision3DialogVoiceSlotRemovalTargetResolutionV1::Unresolved
        }
        VoiceTargetResolution::Ambiguous { .. } => {
            Revision3DialogVoiceSlotRemovalTargetResolutionV1::Ambiguous
        }
        VoiceTargetResolution::Resolved { .. } => {
            Revision3DialogVoiceSlotRemovalTargetResolutionV1::Resolved
        }
    }
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
struct BacklinkBlocker {
    source_entity: EntityId,
    role: Revision3ContentReferenceRoleV1,
    reason: String,
}

fn validate_slot_backlinks(
    project: &ProjectRevision3,
    index: &crate::Revision3ContentIndexV1,
    request: &Revision3DialogVoiceSlotRemovalRequestV1,
) -> Result<(), BacklinkBlocker> {
    let mut expected_edges = 0usize;
    for source in &index.entities {
        for reference in &source.references {
            if reference.target.entity_id != request.slot_id
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
            let expected = source.id == request.line_id
                && reference.role == Revision3ContentReferenceRoleV1::DialogVoiceSlot
                && reference.qualifier.as_deref() == Some(request.locale.as_str())
                && reference.target.expected_kind == EntityKind::VoiceSlot
                && reference.resolution == Revision3ContentReferenceResolutionV1::Resolved;
            if !expected {
                return Err(invalid(
                    "reference is not the one exact owning DialogLine/locale edge",
                ));
            }
            expected_edges = expected_edges.saturating_add(1);
        }
    }
    if expected_edges != 1 {
        return Err(BacklinkBlocker {
            source_entity: request.line_id,
            role: Revision3ContentReferenceRoleV1::DialogVoiceSlot,
            reason: "content index does not contain exactly one owning edge".to_owned(),
        });
    }
    Ok(())
}

fn preserves_exact_basis(
    basis: &ProjectRevision3,
    candidate: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotRemovalRequestV1,
    next_line_revision: u64,
) -> bool {
    if candidate.format != basis.format
        || candidate.schema_revision != basis.schema_revision
        || candidate.revision != basis.revision.saturating_add(1)
        || candidate.project_id != basis.project_id
        || candidate.meta != basis.meta
        || candidate.target != basis.target
        || candidate.authoring_locales != basis.authoring_locales
        || candidate.asset_store != basis.asset_store
        || candidate.entities.len() != basis.entities.len().saturating_sub(1)
        || candidate.entities.contains_key(&request.slot_id)
    {
        return false;
    }
    for (id, entity) in &basis.entities {
        if *id == request.slot_id {
            continue;
        }
        if *id == request.line_id {
            let mut expected = entity.clone();
            let EntityPayload::DialogLine(line) = &mut expected.payload else {
                return false;
            };
            let Some(removed) = line.voice_slots.remove(&request.locale) else {
                return false;
            };
            if removed.id != request.slot_id {
                return false;
            }
            expected.revision = next_line_revision;
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

fn distinct_nonzero_entity_ids(ids: [EntityId; 3]) -> bool {
    ids.iter()
        .all(|id| id.as_bytes().iter().any(|byte| *byte != 0))
        && ids[0] != ids[1]
        && ids[0] != ids[2]
        && ids[1] != ids[2]
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
                "revision-3 dialog VoiceSlot-removal request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
