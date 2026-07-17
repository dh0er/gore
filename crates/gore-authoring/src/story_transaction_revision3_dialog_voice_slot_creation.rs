//! Atomic, filesystem-free creation of one empty managed revision-3 dialog VoiceSlot.
//!
//! The transaction binds one existing [`DialogLine`] and its exact [`LocalizationEntry`] to one
//! previously absent locale [`VoiceSlot`]. It creates no take, Ogg asset, target evidence, build,
//! runtime, deployment, game/save/Store mutation, or fixed-head publication authority.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    Entity, EntityKind, EntityPayload, LocalizationEntry, OriginRef, TypedRef, VoiceSlot,
    VoiceTargetResolution,
};
use crate::revision3_content_index::{
    build_revision3_content_index_v1, Revision3ContentIndexErrorV1,
    Revision3ContentReferenceResolutionV1, Revision3ContentReferenceRoleV1,
};
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    validate_revision3_voice_loc_id_basename_stem_v1, EntityId, GameGenerationAnchor, LocaleCode,
    ProjectId, ProjectRevision3, ProjectRevision3JsonError, WorkingHead, MAX_REVISION3_ENTITIES,
    REVISION3_VOICE_SLOT_GENERATOR_ID_V1, REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
};

/// Maximum exact canonical dialog VoiceSlot-creation request size.
pub const MAX_REVISION3_DIALOG_VOICE_SLOT_CREATION_REQUEST_JSON_BYTES_V1: usize = 64 * 1024;

/// Exact head/project/line/localization binding for creating one empty managed VoiceSlot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3DialogVoiceSlotCreationRequestV1 {
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
}

impl Revision3DialogVoiceSlotCreationRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(
        json: &str,
    ) -> Result<Self, Revision3DialogVoiceSlotCreationRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_DIALOG_VOICE_SLOT_CREATION_REQUEST_JSON_BYTES_V1 {
            return Err(
                Revision3DialogVoiceSlotCreationRequestJsonErrorV1::InputTooLarge {
                    actual: json.len(),
                    limit: MAX_REVISION3_DIALOG_VOICE_SLOT_CREATION_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3DialogVoiceSlotCreationRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3DialogVoiceSlotCreationRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3DialogVoiceSlotCreationRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(
        &self,
    ) -> Result<String, Revision3DialogVoiceSlotCreationRequestJsonErrorV1> {
        let mut writer = BoundedRequestWriter::new(
            MAX_REVISION3_DIALOG_VOICE_SLOT_CREATION_REQUEST_JSON_BYTES_V1,
        );
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(
                Revision3DialogVoiceSlotCreationRequestJsonErrorV1::InputTooLarge {
                    actual,
                    limit: MAX_REVISION3_DIALOG_VOICE_SLOT_CREATION_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        serialized.map_err(Revision3DialogVoiceSlotCreationRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3DialogVoiceSlotCreationRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3DialogVoiceSlotCreationRequestJsonErrorV1 {
    #[error("revision-3 dialog VoiceSlot-creation request JSON is too large: {actual} bytes (limit {limit})")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 dialog VoiceSlot-creation request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 dialog VoiceSlot-creation request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error(
        "revision-3 dialog VoiceSlot-creation request JSON is not in its exact canonical spelling"
    )]
    NonCanonicalJson,
    #[error("revision-3 dialog VoiceSlot-creation request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Stable semantic conflict. Rejection never exposes a partially changed project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3DialogVoiceSlotCreationConflictV1 {
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
    #[error("VoiceSlot locale {locale} is absent from authoring locales")]
    VoiceSlotLocaleNotAuthorable { locale: LocaleCode },
    #[error("VoiceSlot locale {locale} has no non-empty exact localization text")]
    VoiceSlotLocaleHasNoText { locale: LocaleCode },
    #[error("DialogLine locale {locale} is already linked to VoiceSlot {slot}")]
    VoiceSlotLocaleAlreadyLinked { locale: LocaleCode, slot: EntityId },
    #[error("VoiceSlot entity ID {slot} already exists")]
    VoiceSlotIdCollision { slot: EntityId },
    #[error(
        "VoiceSlot {slot} has an unsafe pre-existing local backlink from {source_entity} through {role:?}: {reason}"
    )]
    InvalidLocalBacklink {
        slot: EntityId,
        source_entity: EntityId,
        role: Revision3ContentReferenceRoleV1,
        reason: String,
    },
    #[error("dialog VoiceSlot-creation preflight exceeds the {limit}-reference limit")]
    ReferenceLimit { limit: usize },
    #[error("revision-3 project cannot hold the new VoiceSlot")]
    EntityCapacityExceeded,
    #[error(
        "dialog VoiceSlot-creation candidate exceeds the {limit}-byte project limit: {actual} bytes"
    )]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("dialog VoiceSlot-creation candidate is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3DialogVoiceSlotCreationRejectionV1 {
    pub conflict: Revision3DialogVoiceSlotCreationConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotCreationBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotCreationRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotCreationTargetAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotCreationPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3DialogVoiceSlotCreationOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub line_id: EntityId,
    pub line_revision: u64,
    pub localization_id: EntityId,
    pub localization_revision: u64,
    pub slot_id: EntityId,
    pub slot_revision: u64,
    pub locale: LocaleCode,
    pub loc_id: String,
    pub build_status: Revision3DialogVoiceSlotCreationBuildStatusV1,
    pub runtime_status: Revision3DialogVoiceSlotCreationRuntimeStatusV1,
    pub target_authority: Revision3DialogVoiceSlotCreationTargetAuthorityV1,
    pub publication_status: Revision3DialogVoiceSlotCreationPublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3DialogVoiceSlotCreationEvaluationV1 {
    Applied(Box<Revision3DialogVoiceSlotCreationOutcomeV1>),
    Rejected(Revision3DialogVoiceSlotCreationRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3DialogVoiceSlotCreationErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 dialog VoiceSlot-creation request: {0}")]
    InvalidRequest(#[source] Revision3DialogVoiceSlotCreationRequestJsonErrorV1),
    #[error("could not build the exact revision-3 content index: {0}")]
    ContentIndex(#[source] Revision3ContentIndexErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 dialog VoiceSlot-creation candidate reopen changed the project")]
    CanonicalReopenMismatch,
    #[error("dialog VoiceSlot creation changed a preserved project value")]
    CandidatePreservationMismatch,
}

/// Create one exact empty managed VoiceSlot and its exact DialogLine/locale edge.
///
/// This pure transaction performs no filesystem operation and cannot publish a fixed working
/// head. The candidate remains build-blocked, runtime-unqualified, and without target authority.
pub fn apply_revision3_dialog_voice_slot_creation_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3DialogVoiceSlotCreationEvaluationV1, Revision3DialogVoiceSlotCreationErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3DialogVoiceSlotCreationErrorV1::InvalidProject)?;
    let request = Revision3DialogVoiceSlotCreationRequestV1::from_json(canonical_request_json)
        .map_err(Revision3DialogVoiceSlotCreationErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3DialogVoiceSlotCreationEvaluationV1::Rejected(
                Revision3DialogVoiceSlotCreationRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3DialogVoiceSlotCreationConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3DialogVoiceSlotCreationConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3DialogVoiceSlotCreationConflictV1::ProjectRevisionOverflow);
    };
    if !distinct_nonzero_entity_ids([request.line_id, request.localization_id, request.slot_id]) {
        reject!(Revision3DialogVoiceSlotCreationConflictV1::InvalidEntityIdentity);
    }
    if validate_revision3_voice_loc_id_basename_stem_v1(&request.expected_loc_id).is_err() {
        reject!(Revision3DialogVoiceSlotCreationConflictV1::InvalidExpectedLocId);
    }
    let Some(line_entity) = project.entities.get(&request.line_id) else {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::InvalidDialogLine {
                line: request.line_id,
            }
        );
    };
    let EntityPayload::DialogLine(line) = &line_entity.payload else {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::InvalidDialogLine {
                line: request.line_id,
            }
        );
    };
    if line_entity.revision != request.expected_line_revision {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::DialogLineRevisionConflict {
                expected: request.expected_line_revision,
                actual: line_entity.revision,
            }
        );
    }
    let Some(next_line_revision) = line_entity.revision.checked_add(1) else {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::DialogLineRevisionOverflow {
                line: request.line_id,
            }
        );
    };
    if line.localization.project_id != project.project_id
        || line.localization.expected_kind != EntityKind::LocalizationEntry
        || line.localization.id != request.localization_id
    {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::InvalidLocalizationReference {
                line: request.line_id,
                localization: request.localization_id,
            }
        );
    }
    let Some(localization_entity) = project.entities.get(&request.localization_id) else {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::InvalidLocalization {
                localization: request.localization_id,
            }
        );
    };
    let EntityPayload::LocalizationEntry(LocalizationEntry { loc_id, texts }) =
        &localization_entity.payload
    else {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::InvalidLocalization {
                localization: request.localization_id,
            }
        );
    };
    let localization_revision = localization_entity.revision;
    if loc_id != &request.expected_loc_id {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::LocalizationIdentityMismatch {
                expected: request.expected_loc_id.clone(),
                actual: loc_id.clone(),
            }
        );
    }
    if !project.authoring_locales.contains(&request.locale) {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::VoiceSlotLocaleNotAuthorable {
                locale: request.locale.clone(),
            }
        );
    }
    if texts
        .get(&request.locale)
        .is_none_or(|text| text.trim().is_empty())
    {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::VoiceSlotLocaleHasNoText {
                locale: request.locale.clone(),
            }
        );
    }
    if let Some(existing) = line.voice_slots.get(&request.locale) {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::VoiceSlotLocaleAlreadyLinked {
                locale: request.locale.clone(),
                slot: existing.id,
            }
        );
    }
    if project.entities.contains_key(&request.slot_id) {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::VoiceSlotIdCollision {
                slot: request.slot_id,
            }
        );
    }

    let index = match build_revision3_content_index_v1(&project) {
        Ok(index) => index,
        Err(Revision3ContentIndexErrorV1::TooManyReferences { limit }) => {
            reject!(Revision3DialogVoiceSlotCreationConflictV1::ReferenceLimit { limit });
        }
        Err(error) => return Err(Revision3DialogVoiceSlotCreationErrorV1::ContentIndex(error)),
    };
    if let Some(blocker) = first_local_backlink(&project, &index, request.slot_id) {
        reject!(
            Revision3DialogVoiceSlotCreationConflictV1::InvalidLocalBacklink {
                slot: request.slot_id,
                source_entity: blocker.source_entity,
                role: blocker.role,
                reason: blocker.reason,
            }
        );
    }
    if project
        .entities
        .len()
        .checked_add(1)
        .is_none_or(|count| count > MAX_REVISION3_ENTITIES)
    {
        reject!(Revision3DialogVoiceSlotCreationConflictV1::EntityCapacityExceeded);
    }

    let basis_project = project.clone();
    let slot_entity = expected_slot_entity(&project, &request);
    let Some(line_entity) = project.entities.get_mut(&request.line_id) else {
        unreachable!("DialogLine was resolved above")
    };
    let EntityPayload::DialogLine(line) = &mut line_entity.payload else {
        unreachable!("DialogLine kind was resolved above")
    };
    let replaced = line.voice_slots.insert(
        request.locale.clone(),
        TypedRef::new(project.project_id, request.slot_id, EntityKind::VoiceSlot),
    );
    debug_assert!(replaced.is_none());
    line_entity.revision = next_line_revision;
    debug_assert!(project
        .entities
        .insert(request.slot_id, slot_entity)
        .is_none());
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(
                Revision3DialogVoiceSlotCreationConflictV1::CandidateTooLarge { actual, limit }
            );
        }
        Err(error) => {
            reject!(
                Revision3DialogVoiceSlotCreationConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3DialogVoiceSlotCreationErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3DialogVoiceSlotCreationErrorV1::CanonicalReopenMismatch);
    }
    if !preserves_exact_basis(&basis_project, &reopened, &request, next_line_revision) {
        return Err(Revision3DialogVoiceSlotCreationErrorV1::CandidatePreservationMismatch);
    }

    Ok(Revision3DialogVoiceSlotCreationEvaluationV1::Applied(
        Box::new(Revision3DialogVoiceSlotCreationOutcomeV1 {
            project: reopened,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            line_id: request.line_id,
            line_revision: next_line_revision,
            localization_id: request.localization_id,
            localization_revision,
            slot_id: request.slot_id,
            slot_revision: 0,
            locale: request.locale,
            loc_id: request.expected_loc_id,
            build_status: Revision3DialogVoiceSlotCreationBuildStatusV1::Blocked,
            runtime_status: Revision3DialogVoiceSlotCreationRuntimeStatusV1::RuntimeUnqualified,
            target_authority: Revision3DialogVoiceSlotCreationTargetAuthorityV1::NotGranted,
            publication_status: Revision3DialogVoiceSlotCreationPublicationStatusV1::NotSupported,
        }),
    ))
}

fn expected_slot_entity(
    project: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotCreationRequestV1,
) -> Entity {
    Entity {
        id: request.slot_id,
        display_name: format!("Voice {}", request.locale),
        origin: OriginRef::Generated {
            generator_id: REVISION3_VOICE_SLOT_GENERATOR_ID_V1.to_owned(),
            generator_version: REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1,
            owner: TypedRef::new(project.project_id, request.line_id, EntityKind::DialogLine),
        },
        revision: 0,
        payload: EntityPayload::VoiceSlot(VoiceSlot {
            locale: request.locale.clone(),
            target_resolution: VoiceTargetResolution::Unresolved,
            candidates: Vec::new(),
            selected: None,
        }),
    }
}

#[derive(Debug)]
struct BacklinkBlocker {
    source_entity: EntityId,
    role: Revision3ContentReferenceRoleV1,
    reason: String,
}

fn first_local_backlink(
    project: &ProjectRevision3,
    index: &crate::Revision3ContentIndexV1,
    slot_id: EntityId,
) -> Option<BacklinkBlocker> {
    for source in &index.entities {
        for reference in &source.references {
            if reference.target.entity_id != slot_id
                || reference.target.project_id != project.project_id
            {
                // Foreign-project references with the same 128-bit ID are not local backlinks.
                continue;
            }
            let resolution = match reference.resolution {
                Revision3ContentReferenceResolutionV1::Resolved => "resolved",
                Revision3ContentReferenceResolutionV1::ForeignProject => "foreign_project",
                Revision3ContentReferenceResolutionV1::MissingEntity => "missing_entity",
                Revision3ContentReferenceResolutionV1::KindMismatch => "kind_mismatch",
            };
            return Some(BacklinkBlocker {
                source_entity: source.id,
                role: reference.role,
                reason: format!("requested new ID already has a local {resolution} reference"),
            });
        }
    }
    None
}

fn preserves_exact_basis(
    basis: &ProjectRevision3,
    candidate: &ProjectRevision3,
    request: &Revision3DialogVoiceSlotCreationRequestV1,
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
        || candidate.entities.len() != basis.entities.len().saturating_add(1)
        || basis.entities.contains_key(&request.slot_id)
        || candidate.entities.get(&request.slot_id) != Some(&expected_slot_entity(basis, request))
    {
        return false;
    }
    for (id, entity) in &basis.entities {
        if *id == request.line_id {
            let mut expected = entity.clone();
            let EntityPayload::DialogLine(line) = &mut expected.payload else {
                return false;
            };
            if line
                .voice_slots
                .insert(
                    request.locale.clone(),
                    TypedRef::new(basis.project_id, request.slot_id, EntityKind::VoiceSlot),
                )
                .is_some()
            {
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
                "revision-3 dialog VoiceSlot-creation request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
