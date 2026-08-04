//! Stable-slot-aware, exact-head-bound editing of one semantic revision-3 Quest outline.
//!
//! Generator-v4 objective titles are stored positionally while their technical identities live in
//! the retained transition plan. This transaction therefore accepts an ordered list of
//! `(slot, title)` pairs, proves the exact current plan and owned module, and lowers that list back
//! into the positional title fields without losing stable slot identity. It cannot add or remove
//! objectives, edit transition behavior, or grant build, publication, deployment, or runtime
//! authority.

use std::collections::BTreeSet;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    EntityKind, EntityPayload, OriginRef, QuestDraft, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_QUEST_GENERATOR_VERSION,
};
use crate::revision3_quest::regenerate_revision3_quest_module;
use crate::story_transaction_revision3_quest_transitions::{
    revision3_quest_transition_plan_basis_v1, revision3_quest_transition_plan_seal_v1,
};
use crate::strict_json::reject_duplicate_object_keys;
use crate::QuestCollisionCatalogInput;
use crate::{
    validate_draft_quest_objective_titles, ContentSeal, DraftQuestSkeletonError, EntityId,
    GameGenerationAnchor, ProjectId, ProjectRevision3, ProjectRevision3JsonError, WorkingHead,
    MAX_DRAFT_QUEST_OBJECTIVES,
};

pub const MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2: usize = 32 * 1024;
pub const MAX_REVISION3_QUEST_OUTLINE_EDIT_DISPLAY_NAME_BYTES_V2: usize = 256;

/// One existing stable objective slot, in requested presentation order, and its edited title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestOutlineObjectiveEditV2 {
    pub slot: u16,
    pub title: String,
}

/// Exact project/head/Quest/module/transition-plan-CAS-bound semantic Quest outline edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestOutlineEditRequestV2 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub quest_id: EntityId,
    pub expected_quest_revision: u64,
    pub expected_script_module_id: EntityId,
    pub expected_script_module_revision: u64,
    pub expected_transition_plan_seal: ContentSeal,
    pub display_name: String,
    pub quest_title: String,
    pub objectives: Vec<Revision3QuestOutlineObjectiveEditV2>,
}

impl Revision3QuestOutlineEditRequestV2 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3QuestOutlineEditRequestJsonErrorV2> {
        if json.len() > MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2 {
            return Err(Revision3QuestOutlineEditRequestJsonErrorV2::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3QuestOutlineEditRequestJsonErrorV2::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3QuestOutlineEditRequestJsonErrorV2::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3QuestOutlineEditRequestJsonErrorV2::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3QuestOutlineEditRequestJsonErrorV2> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3QuestOutlineEditRequestJsonErrorV2::InputTooLarge {
                actual,
                limit: MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2,
            });
        }
        serialized.map_err(Revision3QuestOutlineEditRequestJsonErrorV2::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3QuestOutlineEditRequestJsonErrorV2::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestOutlineEditRequestJsonErrorV2 {
    #[error(
        "revision-3 Quest outline-v2 edit request exceeds the {limit}-byte limit: {actual} bytes"
    )]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Quest outline-v2 edit request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Quest outline-v2 edit request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error(
        "revision-3 Quest outline-v2 edit request JSON is not in its exact canonical spelling"
    )]
    NonCanonicalJson,
    #[error("revision-3 Quest outline-v2 edit request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Stable semantic conflict. A rejection never contains a partially edited project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3QuestOutlineEditConflictV2 {
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
    #[error(
        "Quest generator contract mismatch: expected {expected_id}@{expected_version}, got {actual_id}@{actual_version}"
    )]
    GeneratorContractMismatch {
        expected_id: &'static str,
        expected_version: u32,
        actual_id: String,
        actual_version: u32,
    },
    #[error("expected ScriptModule ID must not be zero")]
    ZeroExpectedScriptModuleId,
    #[error("expected owned ScriptModule {expected}, but the Quest references {actual}")]
    ScriptModuleIdentityConflict {
        expected: EntityId,
        actual: EntityId,
    },
    #[error("expected ScriptModule revision {expected}, but exact basis is {actual}")]
    ScriptModuleRevisionConflict { expected: u64, actual: u64 },
    #[error("Quest owned ScriptModule {module} revision cannot be incremented")]
    ScriptModuleRevisionOverflow { module: EntityId },
    #[error("expected transition-plan seal differs from the exact retained plan")]
    TransitionPlanSealConflict {
        expected: ContentSeal,
        actual: ContentSeal,
    },
    #[error("Quest display name is non-canonical, empty, contains controls, or exceeds its limit")]
    InvalidDisplayName,
    #[error("Quest outline contains {actual} objectives; expected 1 through {max}")]
    InvalidObjectiveCount { actual: usize, max: usize },
    #[error("Quest objective count cannot change: expected {expected}, got {actual}")]
    ObjectiveCountChange { expected: usize, actual: usize },
    #[error("Quest objective slot {slot} occurs more than once in the edit")]
    DuplicateObjectiveSlot { slot: u16 },
    #[error("Quest objective slot {slot} is not active in the exact retained plan")]
    ForeignObjectiveSlot { slot: u16 },
    #[error("active Quest objective slot {slot} is missing from the edit")]
    MissingObjectiveSlot { slot: u16 },
    #[error("Quest objective titles are invalid: {error}")]
    InvalidObjectiveTitles { error: DraftQuestSkeletonError },
    #[error("Quest outline-v2 edit does not change name, title, objective titles, or order")]
    NoChanges,
    #[error("Quest {quest} has an invalid owned ScriptModule closure: {reason}")]
    InvalidQuestClosure { quest: EntityId, reason: String },
    #[error("Quest {quest} owned ScriptModule differs from deterministic regeneration")]
    OwnedModuleDrift { quest: EntityId, module: EntityId },
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
pub struct Revision3QuestOutlineEditRejectionV2 {
    pub conflict: Revision3QuestOutlineEditConflictV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestOutlineEditBuildStatusV2 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestOutlineEditRuntimeStatusV2 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestOutlineEditPublicationStatusV2 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3QuestOutlineEditOutcomeV2 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub quest_id: EntityId,
    pub script_module_id: EntityId,
    pub quest_revision: u64,
    pub script_module_revision: u64,
    pub transition_plan_seal: ContentSeal,
    pub build_status: Revision3QuestOutlineEditBuildStatusV2,
    pub runtime_status: Revision3QuestOutlineEditRuntimeStatusV2,
    pub publication_status: Revision3QuestOutlineEditPublicationStatusV2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3QuestOutlineEditEvaluationV2 {
    Applied(Box<Revision3QuestOutlineEditOutcomeV2>),
    Rejected(Revision3QuestOutlineEditRejectionV2),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestOutlineEditErrorV2 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 Quest outline-v2 edit request: {0}")]
    InvalidRequest(#[source] Revision3QuestOutlineEditRequestJsonErrorV2),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 Quest outline-v2 candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Atomically edit one semantic Quest's stable-slot-aware outline and owned ScriptModule.
///
/// Both inputs must be exact canonical JSON. Existing source is first proven by deterministic
/// regeneration with the retained collision evidence represented by an empty collision input.
/// Objective entries are a full permutation of the exact active slots; their order becomes the
/// presentation order and their titles are lowered into the positional title fields. The retained
/// slot set, next ordinal, transitions, predicates, and effects are copied unchanged. No artifact
/// is opened and no filesystem, compiler, build, publication, deployment, save, or runtime action
/// is reachable from this pure transaction.
pub fn apply_revision3_quest_outline_edit_transaction_v2(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3QuestOutlineEditEvaluationV2, Revision3QuestOutlineEditErrorV2> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3QuestOutlineEditErrorV2::InvalidProject)?;
    let request = Revision3QuestOutlineEditRequestV2::from_json(canonical_request_json)
        .map_err(Revision3QuestOutlineEditErrorV2::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3QuestOutlineEditEvaluationV2::Rejected(
                Revision3QuestOutlineEditRejectionV2 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3QuestOutlineEditConflictV2::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3QuestOutlineEditConflictV2::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3QuestOutlineEditConflictV2::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3QuestOutlineEditConflictV2::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3QuestOutlineEditConflictV2::ProjectRevisionOverflow);
    };
    if is_zero_entity_id(request.quest_id) {
        reject!(Revision3QuestOutlineEditConflictV2::ZeroQuestId);
    }
    if !valid_display_name(&request.display_name) {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidDisplayName);
    }
    if request.objectives.is_empty() || request.objectives.len() > MAX_DRAFT_QUEST_OBJECTIVES {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidObjectiveCount {
            actual: request.objectives.len(),
            max: MAX_DRAFT_QUEST_OBJECTIVES,
        });
    }

    let Some(quest_entity) = project.entities.get(&request.quest_id) else {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestEntity {
            quest: request.quest_id,
        });
    };
    let EntityPayload::QuestDraft(quest) = &quest_entity.payload else {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestEntity {
            quest: request.quest_id,
        });
    };
    if request.expected_quest_revision != quest_entity.revision {
        reject!(Revision3QuestOutlineEditConflictV2::QuestRevisionConflict {
            expected: request.expected_quest_revision,
            actual: quest_entity.revision,
        });
    }
    let Some(next_quest_revision) = quest_entity.revision.checked_add(1) else {
        reject!(Revision3QuestOutlineEditConflictV2::QuestRevisionOverflow {
            quest: request.quest_id,
        });
    };
    if quest.generator_id != REVISION3_QUEST_GENERATOR_ID
        || quest.generator_version != REVISION3_QUEST_GENERATOR_VERSION
    {
        reject!(
            Revision3QuestOutlineEditConflictV2::GeneratorContractMismatch {
                expected_id: REVISION3_QUEST_GENERATOR_ID,
                expected_version: REVISION3_QUEST_GENERATOR_VERSION,
                actual_id: quest.generator_id.clone(),
                actual_version: quest.generator_version,
            }
        );
    }
    let quest = quest.clone();

    if is_zero_entity_id(request.expected_script_module_id) {
        reject!(Revision3QuestOutlineEditConflictV2::ZeroExpectedScriptModuleId);
    }
    let script_module_id = quest.script_module.id;
    if request.expected_script_module_id != script_module_id {
        reject!(
            Revision3QuestOutlineEditConflictV2::ScriptModuleIdentityConflict {
                expected: request.expected_script_module_id,
                actual: script_module_id,
            }
        );
    }
    if quest.input.quest_id != request.quest_id
        || quest.input.target != project.target
        || quest.script_module.project_id != project.project_id
        || quest.script_module.expected_kind != EntityKind::ScriptModule
        || is_zero_entity_id(script_module_id)
    {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "Quest identity/target or ScriptModule reference is not exact-project, non-zero, and kind-bound"
                .to_owned(),
        });
    }
    let Some(module_entity) = project.entities.get(&script_module_id) else {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "owned ScriptModule is missing".to_owned(),
        });
    };
    if request.expected_script_module_revision != module_entity.revision {
        reject!(
            Revision3QuestOutlineEditConflictV2::ScriptModuleRevisionConflict {
                expected: request.expected_script_module_revision,
                actual: module_entity.revision,
            }
        );
    }
    let Some(next_script_module_revision) = module_entity.revision.checked_add(1) else {
        reject!(
            Revision3QuestOutlineEditConflictV2::ScriptModuleRevisionOverflow {
                module: script_module_id,
            }
        );
    };
    let EntityPayload::ScriptModule(existing_module) = &module_entity.payload else {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "owned entity is not a ScriptModule".to_owned(),
        });
    };
    if existing_module.owner.project_id != project.project_id
        || existing_module.owner.id != request.quest_id
        || existing_module.owner.expected_kind != EntityKind::QuestDraft
    {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "ScriptModule owner is not the exact Quest".to_owned(),
        });
    }
    let OriginRef::Generated {
        generator_id: origin_generator_id,
        generator_version: origin_generator_version,
        owner: origin_owner,
    } = &module_entity.origin
    else {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "ScriptModule origin is not generated".to_owned(),
        });
    };
    if origin_generator_id != &quest.generator_id
        || *origin_generator_version != quest.generator_version
        || origin_owner != &existing_module.owner
        || existing_module.generator_id != quest.generator_id
        || existing_module.generator_version != quest.generator_version
    {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "Quest, module, and generated origin contracts differ".to_owned(),
        });
    }
    let existing_module = existing_module.clone();
    let existing_module_origin = module_entity.origin.clone();
    let existing_module_display_name = module_entity.display_name.clone();

    let collision_input = empty_collision_input(&quest);
    let regenerated_existing =
        match regenerate_revision3_quest_module(&quest, collision_input.clone()) {
            Ok(module) => module,
            Err(error) => {
                reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
                    quest: request.quest_id,
                    reason: error.to_string(),
                });
            }
        };
    if existing_module != regenerated_existing {
        reject!(Revision3QuestOutlineEditConflictV2::OwnedModuleDrift {
            quest: request.quest_id,
            module: script_module_id,
        });
    }

    let basis = match revision3_quest_transition_plan_basis_v1(&quest) {
        Ok(basis) => basis,
        Err(error) => {
            reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
                quest: request.quest_id,
                reason: error.to_string(),
            });
        }
    };
    if request.expected_transition_plan_seal != basis.seal {
        reject!(
            Revision3QuestOutlineEditConflictV2::TransitionPlanSealConflict {
                expected: request.expected_transition_plan_seal,
                actual: basis.seal,
            }
        );
    }

    let active_slots = basis
        .plan
        .objective_slots
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut requested_slots = BTreeSet::new();
    for objective in &request.objectives {
        if !requested_slots.insert(objective.slot) {
            reject!(
                Revision3QuestOutlineEditConflictV2::DuplicateObjectiveSlot {
                    slot: objective.slot,
                }
            );
        }
        if !active_slots.contains(&objective.slot) {
            reject!(Revision3QuestOutlineEditConflictV2::ForeignObjectiveSlot {
                slot: objective.slot,
            });
        }
    }
    if let Some(slot) = active_slots.difference(&requested_slots).next().copied() {
        reject!(Revision3QuestOutlineEditConflictV2::MissingObjectiveSlot { slot });
    }
    if request.objectives.len() != basis.plan.objective_slots.len() {
        reject!(Revision3QuestOutlineEditConflictV2::ObjectiveCountChange {
            expected: basis.plan.objective_slots.len(),
            actual: request.objectives.len(),
        });
    }
    let requested_titles = request
        .objectives
        .iter()
        .map(|objective| objective.title.clone())
        .collect::<Vec<_>>();
    if let Err(error) =
        validate_draft_quest_objective_titles(&requested_titles[0], &requested_titles[1..])
    {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidObjectiveTitles { error });
    }

    let current_titles = std::iter::once(quest.input.objective_title.as_str())
        .chain(
            quest
                .input
                .additional_objective_titles
                .iter()
                .map(String::as_str),
        )
        .collect::<Vec<_>>();
    if current_titles.len() != basis.plan.objective_order.len() {
        reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
            quest: request.quest_id,
            reason: "objective title count differs from retained objective order".to_owned(),
        });
    }
    let current_objectives = basis
        .plan
        .objective_order
        .iter()
        .copied()
        .zip(current_titles)
        .map(|(slot, title)| Revision3QuestOutlineObjectiveEditV2 {
            slot,
            title: title.to_owned(),
        })
        .collect::<Vec<_>>();
    if request.display_name == project.entities[&request.quest_id].display_name
        && request.quest_title == quest.input.title
        && request.objectives == current_objectives
    {
        reject!(Revision3QuestOutlineEditConflictV2::NoChanges);
    }

    let mut edited_plan = basis.plan.clone();
    edited_plan.objective_order = request
        .objectives
        .iter()
        .map(|objective| objective.slot)
        .collect();
    let mut edited_quest = quest.clone();
    edited_quest.input.title = request.quest_title.clone();
    edited_quest.input.objective_title = requested_titles[0].clone();
    edited_quest.input.additional_objective_titles = requested_titles[1..].to_vec();
    edited_quest.input.transition_plan = Box::new(edited_plan.clone());
    let edited_module = match regenerate_revision3_quest_module(&edited_quest, collision_input) {
        Ok(module) => module,
        Err(crate::Revision3QuestGenerationError::InvalidQuestIntent(error)) => {
            reject!(Revision3QuestOutlineEditConflictV2::InvalidOutlineText { error });
        }
        Err(error) => {
            reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
                quest: request.quest_id,
                reason: error.to_string(),
            });
        }
    };
    if edited_plan.objective_slots != basis.plan.objective_slots
        || edited_plan.next_slot_ordinal != basis.plan.next_slot_ordinal
        || edited_plan.transitions != basis.plan.transitions
        || edited_module.module_namespace != existing_module.module_namespace
        || edited_module.module_relative_path != existing_module.module_relative_path
        || edited_module.owner != existing_module.owner
        || edited_module.generator_id != existing_module.generator_id
        || edited_module.generator_version != existing_module.generator_version
        || edited_module.status != existing_module.status
    {
        reject!(Revision3QuestOutlineEditConflictV2::TechnicalIdentityChanged);
    }
    let transition_plan_seal = match revision3_quest_transition_plan_seal_v1(&edited_plan) {
        Ok(seal) => seal,
        Err(error) => {
            reject!(Revision3QuestOutlineEditConflictV2::InvalidQuestClosure {
                quest: request.quest_id,
                reason: error.to_string(),
            });
        }
    };

    let Some(quest_entity) = project.entities.get_mut(&request.quest_id) else {
        unreachable!("Quest was resolved above")
    };
    quest_entity.display_name = request.display_name;
    quest_entity.revision = next_quest_revision;
    quest_entity.payload = EntityPayload::QuestDraft(edited_quest);
    let Some(module_entity) = project.entities.get_mut(&script_module_id) else {
        unreachable!("owned ScriptModule was resolved above")
    };
    module_entity.display_name = existing_module_display_name;
    module_entity.origin = existing_module_origin;
    module_entity.revision = next_script_module_revision;
    module_entity.payload = EntityPayload::ScriptModule(edited_module);
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3QuestOutlineEditConflictV2::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(
                Revision3QuestOutlineEditConflictV2::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3QuestOutlineEditErrorV2::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3QuestOutlineEditErrorV2::CanonicalReopenMismatch);
    }

    Ok(Revision3QuestOutlineEditEvaluationV2::Applied(Box::new(
        Revision3QuestOutlineEditOutcomeV2 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            quest_id: request.quest_id,
            script_module_id,
            quest_revision: next_quest_revision,
            script_module_revision: next_script_module_revision,
            transition_plan_seal,
            build_status: Revision3QuestOutlineEditBuildStatusV2::Blocked,
            runtime_status: Revision3QuestOutlineEditRuntimeStatusV2::RuntimeUnqualified,
            publication_status: Revision3QuestOutlineEditPublicationStatusV2::NotSupported,
        },
    )))
}

fn empty_collision_input(quest: &QuestDraft) -> QuestCollisionCatalogInput {
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
        && value.len() <= MAX_REVISION3_QUEST_OUTLINE_EDIT_DISPLAY_NAME_BYTES_V2
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
                "revision-3 Quest outline-v2 request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
