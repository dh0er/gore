//! Atomic, filesystem-free insertion of one managed revision-3 dialog line.
//!
//! The transaction creates a new [`DialogLine`] and either creates one new
//! [`LocalizationEntry`] or reuses one exact existing managed entry. It may also create one empty,
//! unresolved locale [`VoiceSlot`]. This is authoring metadata for the line-centric Voice workflow;
//! it creates no dialog topic, generated script, build, deployment, runtime, save, Store, or fixed-
//! head publication authority.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    DialogLine, Entity, EntityKind, EntityPayload, LocalizationEntry, OriginRef, TypedRef,
    VoiceSlot, VoiceTargetResolution,
};
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    validate_revision3_voice_loc_id_basename_stem_v1, EntityId, GameGenerationAnchor, LocaleCode,
    ProjectId, ProjectRevision3, ProjectRevision3JsonError, WorkingHead, MAX_REVISION3_ENTITIES,
    REVISION3_VOICE_SLOT_GENERATOR_ID_V1, REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
};

pub const MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1: usize = 1024 * 1024;
pub const MAX_REVISION3_DIALOG_DISPLAY_NAME_BYTES_V1: usize = 256;
pub const MAX_REVISION3_DIALOG_AUTHORED_IDENTITY_BYTES_V1: usize = 256;
pub const MAX_REVISION3_DIALOG_SPEAKER_HINT_BYTES_V1: usize = 256;
pub const MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1: usize = 64 * 1024;
pub const MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_TOTAL_BYTES_V1: usize = 512 * 1024;
pub const MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1: usize = 1000;

/// Create one new localization or bind the line to one exact existing managed localization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum Revision3DialogLocalizationIntentV1 {
    Create {
        localization_id: EntityId,
        display_name: String,
        loc_id: String,
        texts: BTreeMap<LocaleCode, String>,
    },
    ReuseExact {
        localization_id: EntityId,
        expected_localization_revision: u64,
        expected_loc_id: String,
    },
}

impl Revision3DialogLocalizationIntentV1 {
    pub const fn localization_id(&self) -> EntityId {
        match self {
            Self::Create {
                localization_id, ..
            }
            | Self::ReuseExact {
                localization_id, ..
            } => *localization_id,
        }
    }
}

/// Optional empty slot derived from the new line. A later Voice transaction may add takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3DialogEmptyVoiceSlotIntentV1 {
    pub slot_id: EntityId,
    pub locale: LocaleCode,
    pub display_name: String,
}

/// Exact project/head-bound intent for one new managed dialog line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3DialogLineInsertRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub line_id: EntityId,
    pub line_display_name: String,
    pub line_authored_identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_hint: Option<String>,
    pub localization: Revision3DialogLocalizationIntentV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_slot: Option<Revision3DialogEmptyVoiceSlotIntentV1>,
}

impl Revision3DialogLineInsertRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3DialogLineInsertRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1 {
            return Err(Revision3DialogLineInsertRequestJsonErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3DialogLineInsertRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3DialogLineInsertRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3DialogLineInsertRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3DialogLineInsertRequestJsonErrorV1> {
        let mut writer = BoundedRequestWriter::new(MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3DialogLineInsertRequestJsonErrorV1::InputTooLarge {
                actual,
                limit: MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1,
            });
        }
        serialized.map_err(Revision3DialogLineInsertRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3DialogLineInsertRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3DialogLineInsertRequestJsonErrorV1 {
    #[error("revision-3 dialog-line request exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 dialog-line request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 dialog-line request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 dialog-line request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 dialog-line request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogEntityRoleV1 {
    DialogLine,
    LocalizationEntry,
    VoiceSlot,
}

/// A semantic rejection never exposes a partially changed project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3DialogLineInsertConflictV1 {
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
    ZeroEntityId { role: Revision3DialogEntityRoleV1 },
    #[error("dialog-line, localization, and VoiceSlot IDs must be pairwise distinct")]
    SharedEntityId,
    #[error("{role:?} entity ID {entity} already exists")]
    EntityIdCollision {
        role: Revision3DialogEntityRoleV1,
        entity: EntityId,
    },
    #[error("line display name is empty, non-canonical, contains controls, or is too long")]
    InvalidLineDisplayName,
    #[error("line authored identity is not one bounded canonical ASCII identity")]
    InvalidLineAuthoredIdentity,
    #[error("authored identity {value:?} collides case-insensitively")]
    AuthoredIdentityCollision {
        value: String,
        existing_entity: Option<EntityId>,
    },
    #[error("speaker hint is empty, non-canonical, contains controls, or is too long")]
    InvalidSpeakerHint,
    #[error(
        "localization display name is empty, non-canonical, contains controls, or is too long"
    )]
    InvalidLocalizationDisplayName,
    #[error("localization identity is not a portable Voice basename stem")]
    InvalidLocalizationId,
    #[error("localization identity {value:?} already belongs to {existing_entity}")]
    DuplicateLocalizationIdentity {
        value: String,
        existing_entity: EntityId,
    },
    #[error("localization texts are empty, invalid, or exceed their closed budget")]
    InvalidLocalizationTexts,
    #[error("LocalizationEntry {localization} is missing or has the wrong kind")]
    LocalizationMissingOrWrongKind { localization: EntityId },
    #[error(
        "LocalizationEntry {localization} revision differs: expected {expected}, actual {actual}"
    )]
    LocalizationRevisionConflict {
        localization: EntityId,
        expected: u64,
        actual: u64,
    },
    #[error(
        "LocalizationEntry {localization} identity differs: expected {expected:?}, actual {actual:?}"
    )]
    LocalizationIdentityConflict {
        localization: EntityId,
        expected: String,
        actual: String,
    },
    #[error("LocalizationEntry {localization} is already referenced by DialogLine {owner_line}")]
    LocalizationAlreadyReferenced {
        localization: EntityId,
        owner_line: EntityId,
    },
    #[error("VoiceSlot display name is empty, non-canonical, contains controls, or is too long")]
    InvalidVoiceSlotDisplayName,
    #[error("VoiceSlot locale {locale} has no non-empty exact localization text")]
    VoiceSlotLocaleHasNoText { locale: LocaleCode },
    #[error("revision-3 project cannot hold the required dialog entities")]
    EntityCapacityExceeded,
    #[error("candidate project exceeds the {limit}-byte limit: {actual} bytes")]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3DialogLineInsertRejectionV1 {
    pub conflict: Revision3DialogLineInsertConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogLocalizationActionV1 {
    Created,
    ReusedExact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogTopicAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3DialogLineInsertOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub line_id: EntityId,
    pub localization_id: EntityId,
    pub voice_slot_id: Option<EntityId>,
    pub localization_action: Revision3DialogLocalizationActionV1,
    pub build_status: Revision3DialogBuildStatusV1,
    pub runtime_status: Revision3DialogRuntimeStatusV1,
    pub topic_authority: Revision3DialogTopicAuthorityV1,
    pub publication_status: Revision3DialogPublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3DialogLineInsertEvaluationV1 {
    Applied(Box<Revision3DialogLineInsertOutcomeV1>),
    Rejected(Revision3DialogLineInsertRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3DialogLineInsertErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 dialog-line request: {0}")]
    InvalidRequest(#[source] Revision3DialogLineInsertRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 dialog-line candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Insert one new managed dialog line against an exact immutable basis.
///
/// This function performs no filesystem operation and cannot publish the fixed working head. The
/// returned candidate remains build-blocked, runtime-unqualified, and without dialog-topic
/// authority even when it contains a Voice-authorable line/localization/slot graph.
pub fn apply_revision3_dialog_line_insert_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3DialogLineInsertEvaluationV1, Revision3DialogLineInsertErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3DialogLineInsertErrorV1::InvalidProject)?;
    let request = Revision3DialogLineInsertRequestV1::from_json(canonical_request_json)
        .map_err(Revision3DialogLineInsertErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3DialogLineInsertEvaluationV1::Rejected(
                Revision3DialogLineInsertRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3DialogLineInsertConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3DialogLineInsertConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3DialogLineInsertConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3DialogLineInsertConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3DialogLineInsertConflictV1::ProjectRevisionOverflow);
    };

    if is_zero_entity_id(request.line_id) {
        reject!(Revision3DialogLineInsertConflictV1::ZeroEntityId {
            role: Revision3DialogEntityRoleV1::DialogLine,
        });
    }
    let localization_id = request.localization.localization_id();
    if is_zero_entity_id(localization_id) {
        reject!(Revision3DialogLineInsertConflictV1::ZeroEntityId {
            role: Revision3DialogEntityRoleV1::LocalizationEntry,
        });
    }
    if let Some(slot) = &request.voice_slot {
        if is_zero_entity_id(slot.slot_id) {
            reject!(Revision3DialogLineInsertConflictV1::ZeroEntityId {
                role: Revision3DialogEntityRoleV1::VoiceSlot,
            });
        }
    }
    let requested_ids = [
        Some(request.line_id),
        Some(localization_id),
        request.voice_slot.as_ref().map(|slot| slot.slot_id),
    ]
    .into_iter()
    .flatten()
    .collect::<BTreeSet<_>>();
    let expected_ids = 2 + usize::from(request.voice_slot.is_some());
    if requested_ids.len() != expected_ids {
        reject!(Revision3DialogLineInsertConflictV1::SharedEntityId);
    }

    if project.entities.contains_key(&request.line_id) {
        reject!(Revision3DialogLineInsertConflictV1::EntityIdCollision {
            role: Revision3DialogEntityRoleV1::DialogLine,
            entity: request.line_id,
        });
    }
    if let Some(slot) = &request.voice_slot {
        if project.entities.contains_key(&slot.slot_id) {
            reject!(Revision3DialogLineInsertConflictV1::EntityIdCollision {
                role: Revision3DialogEntityRoleV1::VoiceSlot,
                entity: slot.slot_id,
            });
        }
    }
    if !valid_display_name(&request.line_display_name) {
        reject!(Revision3DialogLineInsertConflictV1::InvalidLineDisplayName);
    }
    if !valid_authored_identity(&request.line_authored_identity) {
        reject!(Revision3DialogLineInsertConflictV1::InvalidLineAuthoredIdentity);
    }
    if let Some(existing_entity) = find_authored_identity(&project, &request.line_authored_identity)
    {
        reject!(
            Revision3DialogLineInsertConflictV1::AuthoredIdentityCollision {
                value: request.line_authored_identity.clone(),
                existing_entity: Some(existing_entity),
            }
        );
    }
    if request
        .speaker_hint
        .as_deref()
        .is_some_and(|speaker| !valid_speaker_hint(speaker))
    {
        reject!(Revision3DialogLineInsertConflictV1::InvalidSpeakerHint);
    }
    if let Some(slot) = &request.voice_slot {
        if !valid_display_name(&slot.display_name) {
            reject!(Revision3DialogLineInsertConflictV1::InvalidVoiceSlotDisplayName);
        }
    }

    let (localization_action, localization, localization_display_name, text_locales) =
        match &request.localization {
            Revision3DialogLocalizationIntentV1::Create {
                localization_id,
                display_name,
                loc_id,
                texts,
            } => {
                if project.entities.contains_key(localization_id) {
                    reject!(Revision3DialogLineInsertConflictV1::EntityIdCollision {
                        role: Revision3DialogEntityRoleV1::LocalizationEntry,
                        entity: *localization_id,
                    });
                }
                if !valid_display_name(display_name) {
                    reject!(Revision3DialogLineInsertConflictV1::InvalidLocalizationDisplayName);
                }
                if validate_revision3_voice_loc_id_basename_stem_v1(loc_id).is_err() {
                    reject!(Revision3DialogLineInsertConflictV1::InvalidLocalizationId);
                }
                if let Some(existing_entity) = find_localization_identity(&project, loc_id, None) {
                    reject!(
                        Revision3DialogLineInsertConflictV1::DuplicateLocalizationIdentity {
                            value: loc_id.clone(),
                            existing_entity,
                        }
                    );
                }
                if let Some(existing_entity) = find_authored_identity(&project, loc_id) {
                    reject!(
                        Revision3DialogLineInsertConflictV1::AuthoredIdentityCollision {
                            value: loc_id.clone(),
                            existing_entity: Some(existing_entity),
                        }
                    );
                }
                if request.line_authored_identity.eq_ignore_ascii_case(loc_id) {
                    reject!(
                        Revision3DialogLineInsertConflictV1::AuthoredIdentityCollision {
                            value: loc_id.clone(),
                            existing_entity: None,
                        }
                    );
                }
                if !valid_localization_texts(texts, true) {
                    reject!(Revision3DialogLineInsertConflictV1::InvalidLocalizationTexts);
                }
                (
                    Revision3DialogLocalizationActionV1::Created,
                    LocalizationEntry {
                        loc_id: loc_id.clone(),
                        texts: texts.clone(),
                    },
                    Some(display_name.clone()),
                    texts.keys().cloned().collect::<Vec<_>>(),
                )
            }
            Revision3DialogLocalizationIntentV1::ReuseExact {
                localization_id,
                expected_localization_revision,
                expected_loc_id,
            } => {
                let Some(entity) = project.entities.get(localization_id) else {
                    reject!(
                        Revision3DialogLineInsertConflictV1::LocalizationMissingOrWrongKind {
                            localization: *localization_id,
                        }
                    );
                };
                let EntityPayload::LocalizationEntry(localization) = &entity.payload else {
                    reject!(
                        Revision3DialogLineInsertConflictV1::LocalizationMissingOrWrongKind {
                            localization: *localization_id,
                        }
                    );
                };
                if entity.revision != *expected_localization_revision {
                    reject!(
                        Revision3DialogLineInsertConflictV1::LocalizationRevisionConflict {
                            localization: *localization_id,
                            expected: *expected_localization_revision,
                            actual: entity.revision,
                        }
                    );
                }
                if localization.loc_id != *expected_loc_id {
                    reject!(
                        Revision3DialogLineInsertConflictV1::LocalizationIdentityConflict {
                            localization: *localization_id,
                            expected: expected_loc_id.clone(),
                            actual: localization.loc_id.clone(),
                        }
                    );
                }
                if let Some(owner_line) =
                    find_dialog_line_localization_owner(&project, *localization_id)
                {
                    reject!(
                        Revision3DialogLineInsertConflictV1::LocalizationAlreadyReferenced {
                            localization: *localization_id,
                            owner_line,
                        }
                    );
                }
                if validate_revision3_voice_loc_id_basename_stem_v1(&localization.loc_id).is_err() {
                    reject!(Revision3DialogLineInsertConflictV1::InvalidLocalizationId);
                }
                if let Some(existing_entity) = find_localization_identity(
                    &project,
                    &localization.loc_id,
                    Some(*localization_id),
                ) {
                    reject!(
                        Revision3DialogLineInsertConflictV1::DuplicateLocalizationIdentity {
                            value: localization.loc_id.clone(),
                            existing_entity,
                        }
                    );
                }
                if !valid_localization_texts(&localization.texts, false) {
                    reject!(Revision3DialogLineInsertConflictV1::InvalidLocalizationTexts);
                }
                (
                    Revision3DialogLocalizationActionV1::ReusedExact,
                    localization.clone(),
                    None,
                    localization.texts.keys().cloned().collect::<Vec<_>>(),
                )
            }
        };

    if let Some(slot) = &request.voice_slot {
        if localization
            .texts
            .get(&slot.locale)
            .is_none_or(|text| text.trim().is_empty())
        {
            reject!(
                Revision3DialogLineInsertConflictV1::VoiceSlotLocaleHasNoText {
                    locale: slot.locale.clone(),
                }
            );
        }
    }

    let added_entities = usize::from(matches!(
        request.localization,
        Revision3DialogLocalizationIntentV1::Create { .. }
    )) + 1
        + usize::from(request.voice_slot.is_some());
    if project
        .entities
        .len()
        .checked_add(added_entities)
        .is_none_or(|count| count > MAX_REVISION3_ENTITIES)
    {
        reject!(Revision3DialogLineInsertConflictV1::EntityCapacityExceeded);
    }

    for locale in text_locales {
        project.authoring_locales.insert(locale);
    }

    if let Some(display_name) = localization_display_name {
        let loc_id = localization.loc_id.clone();
        let entity = Entity {
            id: localization_id,
            display_name,
            origin: OriginRef::New {
                authored_runtime_id: loc_id,
            },
            revision: 0,
            payload: EntityPayload::LocalizationEntry(localization),
        };
        debug_assert!(project.entities.insert(localization_id, entity).is_none());
    }

    let mut voice_slots = BTreeMap::new();
    let voice_slot_id = if let Some(slot) = &request.voice_slot {
        project.authoring_locales.insert(slot.locale.clone());
        voice_slots.insert(
            slot.locale.clone(),
            TypedRef::new(project.project_id, slot.slot_id, EntityKind::VoiceSlot),
        );
        let slot_entity = Entity {
            id: slot.slot_id,
            display_name: slot.display_name.clone(),
            origin: OriginRef::Generated {
                generator_id: REVISION3_VOICE_SLOT_GENERATOR_ID_V1.to_owned(),
                generator_version: REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
                owner: TypedRef::new(project.project_id, request.line_id, EntityKind::DialogLine),
            },
            revision: 0,
            payload: EntityPayload::VoiceSlot(VoiceSlot {
                locale: slot.locale.clone(),
                target_resolution: VoiceTargetResolution::Unresolved,
                candidates: Vec::new(),
                selected: None,
            }),
        };
        debug_assert!(project.entities.insert(slot.slot_id, slot_entity).is_none());
        Some(slot.slot_id)
    } else {
        None
    };

    let line_entity = Entity {
        id: request.line_id,
        display_name: request.line_display_name,
        origin: OriginRef::New {
            authored_runtime_id: request.line_authored_identity,
        },
        revision: 0,
        payload: EntityPayload::DialogLine(DialogLine {
            localization: TypedRef::new(
                project.project_id,
                localization_id,
                EntityKind::LocalizationEntry,
            ),
            speaker_hint: request.speaker_hint,
            voice_slots,
        }),
    };
    debug_assert!(project
        .entities
        .insert(request.line_id, line_entity)
        .is_none());
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3DialogLineInsertConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(
                Revision3DialogLineInsertConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3DialogLineInsertErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3DialogLineInsertErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3DialogLineInsertEvaluationV1::Applied(Box::new(
        Revision3DialogLineInsertOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            line_id: request.line_id,
            localization_id,
            voice_slot_id,
            localization_action,
            build_status: Revision3DialogBuildStatusV1::Blocked,
            runtime_status: Revision3DialogRuntimeStatusV1::RuntimeUnqualified,
            topic_authority: Revision3DialogTopicAuthorityV1::NotGranted,
            publication_status: Revision3DialogPublicationStatusV1::NotSupported,
        },
    )))
}

fn valid_display_name(value: &str) -> bool {
    value.trim() == value
        && !value.is_empty()
        && value.len() <= MAX_REVISION3_DIALOG_DISPLAY_NAME_BYTES_V1
        && !value.chars().any(char::is_control)
}

fn valid_authored_identity(value: &str) -> bool {
    value.trim() == value
        && !value.is_empty()
        && value.len() <= MAX_REVISION3_DIALOG_AUTHORED_IDENTITY_BYTES_V1
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'"' | b'\\'))
}

fn valid_speaker_hint(value: &str) -> bool {
    value.trim() == value
        && !value.is_empty()
        && value.len() <= MAX_REVISION3_DIALOG_SPEAKER_HINT_BYTES_V1
        && !value.chars().any(char::is_control)
}

fn valid_localization_texts(
    texts: &BTreeMap<LocaleCode, String>,
    require_every_nonempty: bool,
) -> bool {
    if texts.is_empty() || texts.len() > MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1 {
        return false;
    }
    let mut total = 0usize;
    let mut has_nonempty = false;
    for text in texts.values() {
        total = match total.checked_add(text.len()) {
            Some(total) => total,
            None => return false,
        };
        let nonempty = !text.trim().is_empty();
        has_nonempty |= nonempty;
        if (require_every_nonempty && !nonempty)
            || text.len() > MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1
            || text.contains('\0')
            || total > MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_TOTAL_BYTES_V1
        {
            return false;
        }
    }
    has_nonempty
}

fn find_localization_identity(
    project: &ProjectRevision3,
    candidate: &str,
    except: Option<EntityId>,
) -> Option<EntityId> {
    project.entities.iter().find_map(|(id, entity)| {
        if Some(*id) == except {
            return None;
        }
        let EntityPayload::LocalizationEntry(localization) = &entity.payload else {
            return None;
        };
        localization
            .loc_id
            .eq_ignore_ascii_case(candidate)
            .then_some(*id)
    })
}

fn find_authored_identity(project: &ProjectRevision3, candidate: &str) -> Option<EntityId> {
    project
        .entities
        .iter()
        .find_map(|(id, entity)| match &entity.origin {
            OriginRef::New {
                authored_runtime_id,
            } if authored_runtime_id.eq_ignore_ascii_case(candidate) => Some(*id),
            _ => None,
        })
}

fn find_dialog_line_localization_owner(
    project: &ProjectRevision3,
    localization_id: EntityId,
) -> Option<EntityId> {
    project.entities.iter().find_map(|(line_id, entity)| {
        let EntityPayload::DialogLine(line) = &entity.payload else {
            return None;
        };
        (line.localization.project_id == project.project_id
            && line.localization.expected_kind == EntityKind::LocalizationEntry
            && line.localization.id == localization_id)
            .then_some(*line_id)
    })
}

fn is_zero_entity_id(value: EntityId) -> bool {
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
                "revision-3 dialog-line request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
