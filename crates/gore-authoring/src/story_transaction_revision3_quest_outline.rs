//! Exact-head-bound, filesystem-free editing of one revision-3 Quest outline.
//!
//! This transaction changes only human-facing Quest outline text and the deterministic source
//! derived from it. Technical identity, provenance, graph shape, collision-artifact identity,
//! and every unrelated project value remain fixed. The existing owned module is first proven
//! against the same empty-collision regeneration used by the exact-current Quest source
//! preparation boundary. A successful edit still grants no artifact, build, compiler,
//! publication, deployment, or runtime authority.

use std::collections::BTreeSet;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision2::QuestCollisionCatalogInput;
use crate::model_revision3::{
    EntityKind, EntityPayload, REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
};
use crate::revision3_quest::regenerate_revision3_quest_module_v2;
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    validate_draft_quest_objective_titles, DraftQuestSkeletonError, EntityId, GameGenerationAnchor,
    ProjectId, ProjectRevision3, ProjectRevision3JsonError, WorkingHead,
    MAX_DRAFT_QUEST_OBJECTIVES,
};

pub const MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V1: usize = 32 * 1024;
pub const MAX_REVISION3_QUEST_OUTLINE_EDIT_DISPLAY_NAME_BYTES_V1: usize = 256;

/// Exact project/head/entity-CAS-bound human-facing Quest outline edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestOutlineEditRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub quest_id: EntityId,
    pub expected_quest_revision: u64,
    pub display_name: String,
    pub title: String,
    pub objective_titles: Vec<String>,
}

impl Revision3QuestOutlineEditRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3QuestOutlineEditRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V1 {
            return Err(Revision3QuestOutlineEditRequestJsonErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3QuestOutlineEditRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3QuestOutlineEditRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3QuestOutlineEditRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3QuestOutlineEditRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3QuestOutlineEditRequestJsonErrorV1::InputTooLarge {
                actual,
                limit: MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V1,
            });
        }
        serialized.map_err(Revision3QuestOutlineEditRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3QuestOutlineEditRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestOutlineEditRequestJsonErrorV1 {
    #[error(
        "revision-3 Quest outline edit request exceeds the {limit}-byte limit: {actual} bytes"
    )]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Quest outline edit request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Quest outline edit request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Quest outline edit request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Quest outline edit request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Stable semantic conflict. A rejection never contains a partially edited project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3QuestOutlineEditConflictV1 {
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
    #[error("Quest entity ID must not be zero")]
    ZeroQuestId,
    #[error("Quest entity {quest} is missing or has the wrong kind")]
    InvalidQuestEntity { quest: EntityId },
    #[error("expected Quest entity revision {expected}, but exact basis is {actual}")]
    QuestRevisionConflict { expected: u64, actual: u64 },
    #[error("Quest entity {quest} revision cannot be incremented")]
    QuestRevisionOverflow { quest: EntityId },
    #[error("Quest display name is non-canonical, empty, contains controls, or exceeds its limit")]
    InvalidDisplayName,
    #[error("Quest outline contains {actual} objectives; expected 1 through {max}")]
    InvalidObjectiveCount { actual: usize, max: usize },
    #[error("Quest objective count cannot change: expected {expected}, got {actual}")]
    ObjectiveCountChange { expected: usize, actual: usize },
    #[error("semantic Quest outlines require the stable-slot-aware outline editor v2")]
    SemanticQuestRequiresOutlineV2,
    #[error("Quest outline edit does not change display name, title, or objective titles")]
    NoChanges,
    #[error("Quest objective titles are invalid: {error}")]
    InvalidObjectiveTitles { error: DraftQuestSkeletonError },
    #[error("Quest {quest} has an invalid owned ScriptModule closure: {reason}")]
    InvalidQuestClosure { quest: EntityId, reason: String },
    #[error("Quest {quest} owned ScriptModule differs from deterministic regeneration")]
    OwnedModuleDrift { quest: EntityId, module: EntityId },
    #[error("Quest owned ScriptModule {module} revision cannot be incremented")]
    ScriptModuleRevisionOverflow { module: EntityId },
    #[error("Quest outline text is invalid: {error}")]
    InvalidOutlineText { error: DraftQuestSkeletonError },
    #[error("Quest outline edit unexpectedly changed a preserved technical module identity")]
    TechnicalIdentityChanged,
    #[error("Quest outline candidate exceeds the {limit}-byte project limit: {actual} bytes")]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3QuestOutlineEditRejectionV1 {
    pub conflict: Revision3QuestOutlineEditConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestOutlineEditBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestOutlineEditRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3QuestOutlineEditOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub quest_id: EntityId,
    pub script_module_id: EntityId,
    pub quest_revision: u64,
    pub script_module_revision: u64,
    pub build_status: Revision3QuestOutlineEditBuildStatusV1,
    pub runtime_status: Revision3QuestOutlineEditRuntimeStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3QuestOutlineEditEvaluationV1 {
    Applied(Box<Revision3QuestOutlineEditOutcomeV1>),
    Rejected(Revision3QuestOutlineEditRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestOutlineEditErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 Quest outline edit request: {0}")]
    InvalidRequest(#[source] Revision3QuestOutlineEditRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 Quest outline candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Atomically edit one Quest's human-facing outline and deterministic owned ScriptModule.
///
/// Both JSON inputs must be their exact canonical bytes. The collision input used for old-module
/// proof and candidate regeneration is derived only from the existing Quest's retained sealed
/// ArtifactRef, matching exact-current Quest source preparation. It is intentionally empty: this
/// edit cannot add or change any generated technical identity. No artifact is opened and no
/// build, compiler, filesystem, publication, deployment, or runtime authority is granted.
pub fn apply_revision3_quest_outline_edit_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3QuestOutlineEditEvaluationV1, Revision3QuestOutlineEditErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3QuestOutlineEditErrorV1::InvalidProject)?;
    let request = Revision3QuestOutlineEditRequestV1::from_json(canonical_request_json)
        .map_err(Revision3QuestOutlineEditErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3QuestOutlineEditEvaluationV1::Rejected(
                Revision3QuestOutlineEditRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3QuestOutlineEditConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3QuestOutlineEditConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3QuestOutlineEditConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3QuestOutlineEditConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3QuestOutlineEditConflictV1::ProjectRevisionOverflow);
    };
    if is_zero_entity_id(request.quest_id) {
        reject!(Revision3QuestOutlineEditConflictV1::ZeroQuestId);
    }
    if !valid_display_name(&request.display_name) {
        reject!(Revision3QuestOutlineEditConflictV1::InvalidDisplayName);
    }
    if request.objective_titles.is_empty()
        || request.objective_titles.len() > MAX_DRAFT_QUEST_OBJECTIVES
    {
        reject!(Revision3QuestOutlineEditConflictV1::InvalidObjectiveCount {
            actual: request.objective_titles.len(),
            max: MAX_DRAFT_QUEST_OBJECTIVES,
        });
    }
    if let Err(error) = validate_draft_quest_objective_titles(
        &request.objective_titles[0],
        &request.objective_titles[1..],
    ) {
        reject!(Revision3QuestOutlineEditConflictV1::InvalidObjectiveTitles { error });
    }

    let Some(quest_entity) = project.entities.get(&request.quest_id) else {
        reject!(Revision3QuestOutlineEditConflictV1::InvalidQuestEntity {
            quest: request.quest_id,
        });
    };
    let EntityPayload::QuestDraft(quest) = &quest_entity.payload else {
        reject!(Revision3QuestOutlineEditConflictV1::InvalidQuestEntity {
            quest: request.quest_id,
        });
    };
    if quest.generator_version == REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION {
        reject!(Revision3QuestOutlineEditConflictV1::SemanticQuestRequiresOutlineV2);
    }
    if request.expected_quest_revision != quest_entity.revision {
        reject!(Revision3QuestOutlineEditConflictV1::QuestRevisionConflict {
            expected: request.expected_quest_revision,
            actual: quest_entity.revision,
        });
    }
    let Some(next_quest_revision) = quest_entity.revision.checked_add(1) else {
        reject!(Revision3QuestOutlineEditConflictV1::QuestRevisionOverflow {
            quest: request.quest_id,
        });
    };

    let existing_objective_count =
        1usize.saturating_add(quest.input.additional_objective_titles.len());
    if request.objective_titles.len() != existing_objective_count {
        reject!(Revision3QuestOutlineEditConflictV1::ObjectiveCountChange {
            expected: existing_objective_count,
            actual: request.objective_titles.len(),
        });
    }
    let script_module_id = quest.script_module.id;
    if quest.script_module.project_id != project.project_id
        || quest.script_module.expected_kind != EntityKind::ScriptModule
        || is_zero_entity_id(script_module_id)
    {
        reject!(Revision3QuestOutlineEditConflictV1::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "ScriptModule reference is not exact-project, non-zero, and kind-bound"
                .to_owned(),
        });
    }
    let Some(module_entity) = project.entities.get(&script_module_id) else {
        reject!(Revision3QuestOutlineEditConflictV1::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "owned ScriptModule is missing".to_owned(),
        });
    };
    let EntityPayload::ScriptModule(existing_module) = &module_entity.payload else {
        reject!(Revision3QuestOutlineEditConflictV1::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "owned entity is not a ScriptModule".to_owned(),
        });
    };
    if existing_module.owner.project_id != project.project_id
        || existing_module.owner.id != request.quest_id
        || existing_module.owner.expected_kind != EntityKind::QuestDraft
    {
        reject!(Revision3QuestOutlineEditConflictV1::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "ScriptModule owner is not the exact Quest".to_owned(),
        });
    }
    let Some(next_script_module_revision) = module_entity.revision.checked_add(1) else {
        reject!(
            Revision3QuestOutlineEditConflictV1::ScriptModuleRevisionOverflow {
                module: script_module_id,
            }
        );
    };

    let collision_input = empty_collision_input(quest);
    let regenerated_existing =
        match regenerate_revision3_quest_module_v2(quest, collision_input.clone()) {
            Ok(module) => module,
            Err(error) => {
                reject!(Revision3QuestOutlineEditConflictV1::InvalidQuestClosure {
                    quest: request.quest_id,
                    reason: error.to_string(),
                });
            }
        };
    if existing_module != &regenerated_existing {
        reject!(Revision3QuestOutlineEditConflictV1::OwnedModuleDrift {
            quest: request.quest_id,
            module: script_module_id,
        });
    }
    if request.display_name == quest_entity.display_name
        && request.title == quest.input.title
        && request.objective_titles[0] == quest.input.objective_title
        && request.objective_titles[1..] == quest.input.additional_objective_titles
    {
        reject!(Revision3QuestOutlineEditConflictV1::NoChanges);
    }

    let mut edited_quest = quest.clone();
    edited_quest.input.title = request.title;
    edited_quest.input.objective_title = request.objective_titles[0].clone();
    edited_quest.input.additional_objective_titles = request.objective_titles[1..].to_vec();
    let edited_module = match regenerate_revision3_quest_module_v2(&edited_quest, collision_input) {
        Ok(module) => module,
        Err(crate::Revision3QuestGenerationError::InvalidQuestIntent(error)) => {
            reject!(Revision3QuestOutlineEditConflictV1::InvalidOutlineText { error });
        }
        Err(error) => {
            reject!(Revision3QuestOutlineEditConflictV1::InvalidQuestClosure {
                quest: request.quest_id,
                reason: error.to_string(),
            });
        }
    };
    if edited_module.module_namespace != existing_module.module_namespace
        || edited_module.module_relative_path != existing_module.module_relative_path
        || edited_module.owner != existing_module.owner
        || edited_module.generator_id != existing_module.generator_id
        || edited_module.generator_version != existing_module.generator_version
        || edited_module.status != existing_module.status
    {
        reject!(Revision3QuestOutlineEditConflictV1::TechnicalIdentityChanged);
    }

    let Some(quest_entity) = project.entities.get_mut(&request.quest_id) else {
        unreachable!("Quest was resolved above")
    };
    quest_entity.display_name = request.display_name;
    quest_entity.revision = next_quest_revision;
    quest_entity.payload = EntityPayload::QuestDraft(edited_quest);
    let Some(module_entity) = project.entities.get_mut(&script_module_id) else {
        unreachable!("owned ScriptModule was resolved above")
    };
    module_entity.revision = next_script_module_revision;
    module_entity.payload = EntityPayload::ScriptModule(edited_module);
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3QuestOutlineEditConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(
                Revision3QuestOutlineEditConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3QuestOutlineEditErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3QuestOutlineEditErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3QuestOutlineEditEvaluationV1::Applied(Box::new(
        Revision3QuestOutlineEditOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            quest_id: request.quest_id,
            script_module_id,
            quest_revision: next_quest_revision,
            script_module_revision: next_script_module_revision,
            build_status: Revision3QuestOutlineEditBuildStatusV1::Blocked,
            runtime_status: Revision3QuestOutlineEditRuntimeStatusV1::RuntimeUnqualified,
        },
    )))
}

fn empty_collision_input(quest: &crate::model_revision3::QuestDraft) -> QuestCollisionCatalogInput {
    QuestCollisionCatalogInput {
        generation: quest.input.collision_catalog.generation.clone(),
        source_seal: quest.input.collision_catalog.source_seal.clone(),
        catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
        modules: BTreeSet::new(),
        relative_paths: BTreeSet::new(),
        symbols: BTreeSet::new(),
    }
}

fn valid_display_name(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.len() <= MAX_REVISION3_QUEST_OUTLINE_EDIT_DISPLAY_NAME_BYTES_V1
        && !value.chars().any(char::is_control)
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
                "revision-3 Quest outline edit request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
