//! Atomic, filesystem-free staging of one revision-3 dialog VoiceTake.
//!
//! The transaction binds an existing DialogLine and its LocalizationEntry to one canonical
//! locale, creates or updates that line's single VoiceSlot, and appends one imported Ogg-backed
//! VoiceTake. Existing sealed target resolution is preserved unchanged. The native Store boundary
//! supplies a verified [`ImportedOgg`] preview;
//! this module performs the complete semantic and capacity evaluation without filesystem access.
//! The adapter may install the exact accepted bytes only after this evaluation succeeds. No
//! archive member, build, runtime, deployment, or publication authority is created here.

use std::collections::BTreeSet;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry,
    OggCodec as Revision3OggCodec, OggMetadata as Revision3OggMetadata, OriginRef, TypedRef,
    VoiceSlot, VoiceTake, VoiceTakeStatus, VoiceTargetResolution,
};
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    AssetMeta, ContentSeal, EntityId, GameGenerationAnchor, ImportedOgg, LocaleCode, ProjectId,
    ProjectRevision3, ProjectRevision3JsonError, Sha256Digest, WorkingHead, MAX_REVISION3_ASSETS,
    MAX_REVISION3_ENTITIES, MAX_REVISION3_REFERENCED_ASSET_BYTES,
};

pub const REVISION3_VOICE_SLOT_GENERATOR_ID_V1: &str = "gore-authoring.voice-slot";
pub const REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1: u32 = 1;
pub const REVISION3_VOICE_TAKE_IMPORTER_ID_V1: &str = "gore-authoring.ogg-import";
pub const MAX_REVISION3_VOICE_REQUEST_JSON_BYTES_V1: usize = 64 * 1024;
pub const MAX_REVISION3_VOICE_TEXT_BYTES_V1: usize = 64 * 1024;
pub const MAX_REVISION3_VOICE_DISPLAY_NAME_BYTES_V1: usize = 256;
pub const MAX_REVISION3_VOICE_LOGICAL_NAME_BYTES_V1: usize = 1024;
pub const MAX_REVISION3_VOICE_SLOT_CANDIDATES_V1: usize = 1024;

/// Exact project/head-bound intent for one imported take on an existing dialog line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3VoiceTakeStageRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub line_id: EntityId,
    pub slot_id: EntityId,
    pub take_id: EntityId,
    pub locale: LocaleCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub take_display_name: String,
    pub logical_name: String,
    pub status: VoiceTakeStatus,
    #[serde(default)]
    pub select_take: bool,
}

impl Revision3VoiceTakeStageRequestV1 {
    pub fn from_json(json: &str) -> Result<Self, Revision3VoiceTakeStageRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_VOICE_REQUEST_JSON_BYTES_V1 {
            return Err(Revision3VoiceTakeStageRequestJsonErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_VOICE_REQUEST_JSON_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3VoiceTakeStageRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3VoiceTakeStageRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3VoiceTakeStageRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3VoiceTakeStageRequestJsonErrorV1> {
        let mut writer = BoundedRequestWriter::new(MAX_REVISION3_VOICE_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3VoiceTakeStageRequestJsonErrorV1::InputTooLarge {
                actual,
                limit: MAX_REVISION3_VOICE_REQUEST_JSON_BYTES_V1,
            });
        }
        serialized.map_err(Revision3VoiceTakeStageRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3VoiceTakeStageRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTakeStageRequestJsonErrorV1 {
    #[error("revision-3 Voice request exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Voice request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Voice request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Voice request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Voice request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceEntityRoleV1 {
    DialogLine,
    LocalizationEntry,
    VoiceSlot,
    VoiceTake,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3VoiceTakeStageConflictV1 {
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
    #[error("{role:?} entity ID must not be zero")]
    ZeroEntityId { role: Revision3VoiceEntityRoleV1 },
    #[error("VoiceSlot and VoiceTake IDs must differ")]
    SharedEntityId,
    #[error("DialogLine {line} is missing or has the wrong entity kind")]
    InvalidDialogLine { line: EntityId },
    #[error("DialogLine {line} has an invalid LocalizationEntry reference")]
    InvalidLocalizationReference { line: EntityId },
    #[error("LocalizationEntry {localization} cannot increment its entity revision")]
    LocalizationRevisionOverflow { localization: EntityId },
    #[error("DialogLine {line} cannot increment its entity revision")]
    DialogLineRevisionOverflow { line: EntityId },
    #[error("VoiceSlot {slot} cannot increment its entity revision")]
    VoiceSlotRevisionOverflow { slot: EntityId },
    #[error("localized text is empty, contains NUL, or exceeds its byte limit")]
    InvalidLocalizedText,
    #[error("take display name is empty, contains controls, or exceeds its byte limit")]
    InvalidTakeDisplayName,
    #[error("Ogg logical name is not one bounded control-free .ogg name")]
    InvalidLogicalName,
    #[error("VoiceTake entity ID {take} already exists")]
    VoiceTakeIdCollision { take: EntityId },
    #[error("VoiceSlot entity ID {slot} already exists but is not linked by this line/locale")]
    VoiceSlotIdCollision { slot: EntityId },
    #[error("line/locale is linked to VoiceSlot {actual}, not requested slot {expected}")]
    VoiceSlotIdentityMismatch {
        expected: EntityId,
        actual: EntityId,
    },
    #[error("VoiceSlot {slot} has an invalid or shared graph: {reason}")]
    InvalidVoiceSlot { slot: EntityId, reason: String },
    #[error("an unapproved VoiceTake cannot become the selected take")]
    UnapprovedTakeSelection,
    #[error("revision-3 project cannot hold the required Voice entities")]
    EntityCapacityExceeded,
    #[error("the native imported Ogg receipt is invalid or differs from request intent")]
    InvalidImportedOgg,
    #[error("the Ogg digest already has incompatible AssetStore metadata")]
    AssetMetadataConflict,
    #[error("revision-3 project cannot retain the imported Ogg asset")]
    AssetCapacityExceeded,
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeStageRejectionV1 {
    pub conflict: Revision3VoiceTakeStageConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoiceTargetAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3VoicePublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3VoiceTakeStageOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub line_id: EntityId,
    pub localization_id: EntityId,
    pub slot_id: EntityId,
    pub take_id: EntityId,
    pub locale: LocaleCode,
    pub status: VoiceTakeStatus,
    pub slot_created: bool,
    pub selected: bool,
    pub imported_ogg: ImportedOgg,
    pub build_status: Revision3VoiceBuildStatusV1,
    pub runtime_status: Revision3VoiceRuntimeStatusV1,
    pub target_authority: Revision3VoiceTargetAuthorityV1,
    pub publication_status: Revision3VoicePublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3VoiceTakeStageEvaluationV1 {
    Applied(Box<Revision3VoiceTakeStageOutcomeV1>),
    Rejected(Revision3VoiceTakeStageRejectionV1),
}

/// Filesystem-free result of validating every Voice intent and project-graph
/// condition that does not depend on the imported Ogg receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3VoiceTakePreflightEvaluationV1 {
    Ready,
    Rejected(Revision3VoiceTakeStageRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3VoiceTakeStageErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 Voice request: {0}")]
    InvalidRequest(#[source] Revision3VoiceTakeStageRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 Voice candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

struct Revision3VoiceTakePreparedV1 {
    project: ProjectRevision3,
    request: Revision3VoiceTakeStageRequestV1,
    localization_id: EntityId,
    slot_created: bool,
    next_project_revision: u64,
}

enum Revision3VoiceTakePreparedEvaluationV1 {
    Ready(Box<Revision3VoiceTakePreparedV1>),
    Rejected(Revision3VoiceTakeStageRejectionV1),
}

/// Validate the exact request and existing line/localization/slot graph before
/// any source file is opened or immutable CAS object can be installed.
///
/// [`apply_revision3_voice_take_transaction_v1`] repeats this same preflight
/// after import and additionally verifies the concrete Ogg receipt and asset
/// capacity, so a source race cannot bypass the semantic boundary.
pub fn preflight_revision3_voice_take_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3VoiceTakePreflightEvaluationV1, Revision3VoiceTakeStageErrorV1> {
    Ok(
        match prepare_revision3_voice_take_v1(
            exact_basis_head,
            canonical_project_json,
            canonical_request_json,
        )? {
            Revision3VoiceTakePreparedEvaluationV1::Ready(_) => {
                Revision3VoiceTakePreflightEvaluationV1::Ready
            }
            Revision3VoiceTakePreparedEvaluationV1::Rejected(rejection) => {
                Revision3VoiceTakePreflightEvaluationV1::Rejected(rejection)
            }
        },
    )
}

fn prepare_revision3_voice_take_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3VoiceTakePreparedEvaluationV1, Revision3VoiceTakeStageErrorV1> {
    let project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3VoiceTakeStageErrorV1::InvalidProject)?;
    let request = Revision3VoiceTakeStageRequestV1::from_json(canonical_request_json)
        .map_err(Revision3VoiceTakeStageErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3VoiceTakePreparedEvaluationV1::Rejected(
                Revision3VoiceTakeStageRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3VoiceTakeStageConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(Revision3VoiceTakeStageConflictV1::ProjectIdentityMismatch {
            expected: request.expected_project_id,
            actual: project.project_id,
        });
    }
    if request.expected_revision != project.revision {
        reject!(Revision3VoiceTakeStageConflictV1::ProjectRevisionConflict {
            expected: request.expected_revision,
            actual: project.revision,
        });
    }
    if request.expected_target != project.target {
        reject!(Revision3VoiceTakeStageConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3VoiceTakeStageConflictV1::ProjectRevisionOverflow);
    };
    for (role, id) in [
        (Revision3VoiceEntityRoleV1::DialogLine, request.line_id),
        (Revision3VoiceEntityRoleV1::VoiceSlot, request.slot_id),
        (Revision3VoiceEntityRoleV1::VoiceTake, request.take_id),
    ] {
        if is_zero_entity_id(id) {
            reject!(Revision3VoiceTakeStageConflictV1::ZeroEntityId { role });
        }
    }
    if request.slot_id == request.take_id {
        reject!(Revision3VoiceTakeStageConflictV1::SharedEntityId);
    }
    if request
        .text
        .as_deref()
        .is_some_and(|text| !valid_localized_text(text))
    {
        reject!(Revision3VoiceTakeStageConflictV1::InvalidLocalizedText);
    }
    if !valid_display_name(&request.take_display_name) {
        reject!(Revision3VoiceTakeStageConflictV1::InvalidTakeDisplayName);
    }
    if !valid_logical_name(&request.logical_name) {
        reject!(Revision3VoiceTakeStageConflictV1::InvalidLogicalName);
    }
    if request.select_take && request.status != VoiceTakeStatus::Approved {
        reject!(Revision3VoiceTakeStageConflictV1::UnapprovedTakeSelection);
    }
    if project.entities.contains_key(&request.take_id) {
        reject!(Revision3VoiceTakeStageConflictV1::VoiceTakeIdCollision {
            take: request.take_id,
        });
    }

    let Some(line_entity) = project.entities.get(&request.line_id) else {
        reject!(Revision3VoiceTakeStageConflictV1::InvalidDialogLine {
            line: request.line_id,
        });
    };
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        reject!(Revision3VoiceTakeStageConflictV1::InvalidDialogLine {
            line: request.line_id,
        });
    };
    let localization_id = match exact_localization(&project, request.line_id, line) {
        Some(value) => value,
        None => reject!(
            Revision3VoiceTakeStageConflictV1::InvalidLocalizationReference {
                line: request.line_id,
            }
        ),
    };

    let existing_slot_ref = line.voice_slots.get(&request.locale).cloned();
    let slot_created = existing_slot_ref.is_none();
    if let Some(reference) = &existing_slot_ref {
        if reference.id != request.slot_id {
            reject!(
                Revision3VoiceTakeStageConflictV1::VoiceSlotIdentityMismatch {
                    expected: request.slot_id,
                    actual: reference.id,
                }
            );
        }
        if let Err(reason) =
            validate_existing_slot_graph(&project, request.line_id, &request.locale, reference)
        {
            reject!(Revision3VoiceTakeStageConflictV1::InvalidVoiceSlot {
                slot: request.slot_id,
                reason,
            });
        }
        if project.entities[&request.slot_id]
            .revision
            .checked_add(1)
            .is_none()
        {
            reject!(
                Revision3VoiceTakeStageConflictV1::VoiceSlotRevisionOverflow {
                    slot: request.slot_id,
                }
            );
        }
    } else {
        if project.entities.contains_key(&request.slot_id) {
            reject!(Revision3VoiceTakeStageConflictV1::VoiceSlotIdCollision {
                slot: request.slot_id,
            });
        }
        if line_entity.revision.checked_add(1).is_none() {
            reject!(
                Revision3VoiceTakeStageConflictV1::DialogLineRevisionOverflow {
                    line: request.line_id,
                }
            );
        }
    }

    let localization_entity = &project.entities[&localization_id];
    let EntityPayload::LocalizationEntry(localization) = &localization_entity.payload else {
        unreachable!("localization kind was resolved above")
    };
    if request
        .text
        .as_ref()
        .is_some_and(|text| localization.texts.get(&request.locale) != Some(text))
        && localization_entity.revision.checked_add(1).is_none()
    {
        reject!(
            Revision3VoiceTakeStageConflictV1::LocalizationRevisionOverflow {
                localization: localization_id,
            }
        );
    }

    let additional_entities = if slot_created { 2 } else { 1 };
    if project
        .entities
        .len()
        .checked_add(additional_entities)
        .is_none_or(|count| count > MAX_REVISION3_ENTITIES)
    {
        reject!(Revision3VoiceTakeStageConflictV1::EntityCapacityExceeded);
    }

    Ok(Revision3VoiceTakePreparedEvaluationV1::Ready(Box::new(
        Revision3VoiceTakePreparedV1 {
            project,
            request,
            localization_id,
            slot_created,
            next_project_revision,
        },
    )))
}

/// Stage one immutable Ogg-backed VoiceTake against an existing revision-3 DialogLine.
///
/// This function performs no filesystem operation. The Ogg receipt may be a source-preparation
/// preview whose deduplication bit is not final; its exact identity and derived metadata are
/// checked and the complete candidate capacity is evaluated here. The native Store adapter must
/// subsequently install those exact accepted bytes and replace the preview receipt with the
/// actual installation receipt. A newly created slot remains unresolved; an existing valid target
/// resolution is preserved. Build/runtime qualification and fixed-head publication are separate
/// caller-owned operations.
pub fn apply_revision3_voice_take_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
    imported_ogg: ImportedOgg,
) -> Result<Revision3VoiceTakeStageEvaluationV1, Revision3VoiceTakeStageErrorV1> {
    let Revision3VoiceTakePreparedV1 {
        mut project,
        request,
        localization_id,
        slot_created,
        next_project_revision,
    } = match prepare_revision3_voice_take_v1(
        exact_basis_head,
        canonical_project_json,
        canonical_request_json,
    )? {
        Revision3VoiceTakePreparedEvaluationV1::Ready(prepared) => *prepared,
        Revision3VoiceTakePreparedEvaluationV1::Rejected(rejection) => {
            return Ok(Revision3VoiceTakeStageEvaluationV1::Rejected(rejection));
        }
    };

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3VoiceTakeStageEvaluationV1::Rejected(
                Revision3VoiceTakeStageRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if !valid_imported_ogg(&imported_ogg, &request.logical_name) {
        reject!(Revision3VoiceTakeStageConflictV1::InvalidImportedOgg);
    }
    if let Err(conflict) = retain_imported_asset(&mut project, &imported_ogg) {
        reject!(conflict);
    }

    let take_ref = TypedRef::new(project.project_id, request.take_id, EntityKind::VoiceTake);
    let source_seal = ContentSeal {
        byte_len: imported_ogg.asset.byte_len,
        sha256: imported_ogg.asset.sha256,
    };
    let take = VoiceTake {
        locale: request.locale.clone(),
        asset: imported_ogg.asset.clone(),
        ogg: revision3_ogg_metadata(&imported_ogg.ogg),
        status: request.status,
    };
    let take_entity = Entity {
        id: request.take_id,
        display_name: request.take_display_name,
        origin: OriginRef::Imported {
            importer: REVISION3_VOICE_TAKE_IMPORTER_ID_V1.to_owned(),
            source_seal,
            external_identity: None,
        },
        revision: 0,
        payload: EntityPayload::VoiceTake(take),
    };

    if slot_created {
        let line_ref = TypedRef::new(project.project_id, request.line_id, EntityKind::DialogLine);
        let slot = VoiceSlot {
            locale: request.locale.clone(),
            target_resolution: VoiceTargetResolution::Unresolved,
            candidates: vec![take_ref.clone()],
            selected: request.select_take.then_some(take_ref.clone()),
        };
        let slot_entity = Entity {
            id: request.slot_id,
            display_name: format!("Voice {}", request.locale),
            origin: OriginRef::Generated {
                generator_id: REVISION3_VOICE_SLOT_GENERATOR_ID_V1.to_owned(),
                generator_version: REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
                owner: line_ref,
            },
            revision: 0,
            payload: EntityPayload::VoiceSlot(slot),
        };
        let Some(line_entity) = project.entities.get_mut(&request.line_id) else {
            unreachable!("line was resolved above")
        };
        let EntityPayload::DialogLine(line) = &mut line_entity.payload else {
            unreachable!("line kind was resolved above")
        };
        let Some(next) = line_entity.revision.checked_add(1) else {
            reject!(
                Revision3VoiceTakeStageConflictV1::DialogLineRevisionOverflow {
                    line: request.line_id,
                }
            );
        };
        line.voice_slots.insert(
            request.locale.clone(),
            TypedRef::new(project.project_id, request.slot_id, EntityKind::VoiceSlot),
        );
        line_entity.revision = next;
        debug_assert!(project
            .entities
            .insert(request.slot_id, slot_entity)
            .is_none());
    } else {
        let Some(slot_entity) = project.entities.get_mut(&request.slot_id) else {
            unreachable!("existing slot was resolved above")
        };
        let EntityPayload::VoiceSlot(slot) = &mut slot_entity.payload else {
            unreachable!("existing slot kind was resolved above")
        };
        let Some(next) = slot_entity.revision.checked_add(1) else {
            reject!(
                Revision3VoiceTakeStageConflictV1::VoiceSlotRevisionOverflow {
                    slot: request.slot_id,
                }
            );
        };
        slot.candidates.push(take_ref.clone());
        if request.select_take {
            slot.selected = Some(take_ref.clone());
        }
        slot_entity.revision = next;
    }

    let Some(localization_entity) = project.entities.get_mut(&localization_id) else {
        unreachable!("localization was resolved above")
    };
    let EntityPayload::LocalizationEntry(localization) = &mut localization_entity.payload else {
        unreachable!("localization kind was resolved above")
    };
    if let Some(text) = &request.text {
        if localization.texts.get(&request.locale) != Some(text) {
            let Some(next) = localization_entity.revision.checked_add(1) else {
                reject!(
                    Revision3VoiceTakeStageConflictV1::LocalizationRevisionOverflow {
                        localization: localization_id,
                    }
                );
            };
            localization
                .texts
                .insert(request.locale.clone(), text.clone());
            localization_entity.revision = next;
        }
    }
    project.authoring_locales.insert(request.locale.clone());
    debug_assert!(project
        .entities
        .insert(request.take_id, take_entity)
        .is_none());
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(error) => {
            reject!(Revision3VoiceTakeStageConflictV1::CandidateNotPersistable {
                reason: error.to_string(),
            });
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3VoiceTakeStageErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3VoiceTakeStageErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3VoiceTakeStageEvaluationV1::Applied(Box::new(
        Revision3VoiceTakeStageOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            line_id: request.line_id,
            localization_id,
            slot_id: request.slot_id,
            take_id: request.take_id,
            locale: request.locale,
            status: request.status,
            slot_created,
            selected: request.select_take,
            imported_ogg,
            build_status: Revision3VoiceBuildStatusV1::Blocked,
            runtime_status: Revision3VoiceRuntimeStatusV1::RuntimeUnqualified,
            target_authority: Revision3VoiceTargetAuthorityV1::NotGranted,
            publication_status: Revision3VoicePublicationStatusV1::NotSupported,
        },
    )))
}

fn exact_localization(
    project: &ProjectRevision3,
    line_id: EntityId,
    line: &DialogLine,
) -> Option<EntityId> {
    let reference = &line.localization;
    if reference.project_id != project.project_id
        || reference.expected_kind != EntityKind::LocalizationEntry
    {
        return None;
    }
    let entity = project.entities.get(&reference.id)?;
    let EntityPayload::LocalizationEntry(LocalizationEntry { loc_id, .. }) = &entity.payload else {
        return None;
    };
    if crate::validate_revision3_voice_loc_id_basename_stem_v1(loc_id).is_err() {
        return None;
    }
    if line_id == reference.id {
        return None;
    }
    Some(reference.id)
}

fn validate_existing_slot_graph(
    project: &ProjectRevision3,
    line_id: EntityId,
    locale: &LocaleCode,
    reference: &TypedRef,
) -> Result<(), String> {
    if reference.project_id != project.project_id
        || reference.expected_kind != EntityKind::VoiceSlot
    {
        return Err("line slot reference is not exact-project VoiceSlot".to_owned());
    }
    let Some(entity) = project.entities.get(&reference.id) else {
        return Err("referenced VoiceSlot is missing".to_owned());
    };
    let EntityPayload::VoiceSlot(slot) = &entity.payload else {
        return Err("referenced entity is not a VoiceSlot".to_owned());
    };
    if &slot.locale != locale {
        return Err("VoiceSlot locale differs from line locale".to_owned());
    }
    if slot.candidates.len() >= MAX_REVISION3_VOICE_SLOT_CANDIDATES_V1 {
        return Err("VoiceSlot candidate limit is exhausted".to_owned());
    }

    let mut candidates = BTreeSet::new();
    for candidate in &slot.candidates {
        if candidate.project_id != project.project_id
            || candidate.expected_kind != EntityKind::VoiceTake
            || !candidates.insert(candidate.id)
        {
            return Err("VoiceSlot candidate references are invalid or duplicated".to_owned());
        }
        let Some(candidate_entity) = project.entities.get(&candidate.id) else {
            return Err("VoiceSlot candidate is missing".to_owned());
        };
        let EntityPayload::VoiceTake(take) = &candidate_entity.payload else {
            return Err("VoiceSlot candidate has the wrong entity kind".to_owned());
        };
        if &take.locale != locale {
            return Err("VoiceSlot candidate locale differs from slot locale".to_owned());
        }
    }
    if let Some(selected) = &slot.selected {
        if selected.project_id != project.project_id
            || selected.expected_kind != EntityKind::VoiceTake
            || !candidates.contains(&selected.id)
        {
            return Err("selected VoiceTake is not an exact slot candidate".to_owned());
        }
        let Some(selected_entity) = project.entities.get(&selected.id) else {
            return Err("selected VoiceTake is missing".to_owned());
        };
        let EntityPayload::VoiceTake(_) = &selected_entity.payload else {
            return Err("selected VoiceTake has the wrong entity kind".to_owned());
        };
    }

    for (owner_id, owner) in &project.entities {
        let EntityPayload::DialogLine(other_line) = &owner.payload else {
            continue;
        };
        for (other_locale, other_ref) in &other_line.voice_slots {
            if other_ref.project_id == project.project_id
                && other_ref.id == reference.id
                && (*owner_id != line_id || other_locale != locale)
            {
                return Err("VoiceSlot is shared by another line or locale".to_owned());
            }
        }
    }
    Ok(())
}

fn retain_imported_asset(
    project: &mut ProjectRevision3,
    imported: &ImportedOgg,
) -> Result<(), Revision3VoiceTakeStageConflictV1> {
    match project.asset_store.assets.get(&imported.asset.sha256) {
        Some(meta)
            if meta.byte_len == imported.asset.byte_len && meta.media_type == "audio/ogg" =>
        {
            return Ok(())
        }
        Some(_) => return Err(Revision3VoiceTakeStageConflictV1::AssetMetadataConflict),
        None => {}
    }
    if project.asset_store.assets.len() >= MAX_REVISION3_ASSETS {
        return Err(Revision3VoiceTakeStageConflictV1::AssetCapacityExceeded);
    }
    let current_bytes = project
        .asset_store
        .assets
        .values()
        .try_fold(0u64, |total, meta| total.checked_add(meta.byte_len))
        .ok_or(Revision3VoiceTakeStageConflictV1::AssetCapacityExceeded)?;
    if current_bytes
        .checked_add(imported.asset.byte_len)
        .is_none_or(|total| total > MAX_REVISION3_REFERENCED_ASSET_BYTES)
    {
        return Err(Revision3VoiceTakeStageConflictV1::AssetCapacityExceeded);
    }
    project.asset_store.assets.insert(
        imported.asset.sha256,
        AssetMeta {
            byte_len: imported.asset.byte_len,
            media_type: "audio/ogg".to_owned(),
        },
    );
    Ok(())
}

fn valid_imported_ogg(value: &ImportedOgg, logical_name: &str) -> bool {
    value.asset.byte_len != 0
        && !is_zero_digest(value.asset.sha256)
        && value.asset.logical_name == logical_name
        && valid_logical_name(&value.asset.logical_name)
        && value.ogg.channels != 0
        && value.ogg.sample_rate != 0
        && value.ogg.pages != 0
        && value.ogg.logical_streams != 0
}

fn revision3_ogg_metadata(value: &crate::OggMetadata) -> Revision3OggMetadata {
    Revision3OggMetadata {
        codec: match value.codec {
            crate::OggCodec::Vorbis => Revision3OggCodec::Vorbis,
            crate::OggCodec::Opus => Revision3OggCodec::Opus,
        },
        channels: value.channels,
        sample_rate: value.sample_rate,
        pages: value.pages,
        logical_streams: value.logical_streams,
    }
}

fn valid_localized_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_REVISION3_VOICE_TEXT_BYTES_V1
        && !value.contains('\0')
}

fn valid_display_name(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_REVISION3_VOICE_DISPLAY_NAME_BYTES_V1
        && !value.chars().any(char::is_control)
}

fn valid_logical_name(value: &str) -> bool {
    let folded = value.to_ascii_lowercase();
    if value.trim() != value {
        return false;
    }
    if value.len() <= 4
        || value.len() > MAX_REVISION3_VOICE_LOGICAL_NAME_BYTES_V1
        || !folded.ends_with(".ogg")
        || value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        })
    {
        return false;
    }
    let stem = &value[..value.len() - 4];
    if stem.is_empty() || stem == "." || stem == ".." {
        return false;
    }
    let device_stem = stem
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !matches!(device_stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(device_stem.len() == 4
            && (device_stem.starts_with("COM") || device_stem.starts_with("LPT"))
            && matches!(device_stem.as_bytes()[3], b'1'..=b'9'))
}

fn is_zero_entity_id(value: EntityId) -> bool {
    value.as_bytes().iter().all(|byte| *byte == 0)
}

fn is_zero_digest(value: Sha256Digest) -> bool {
    value.as_bytes().iter().all(|byte| *byte == 0)
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
                "revision-3 Voice request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
