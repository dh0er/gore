//! Exact-head-bound, filesystem-free editing of one revision-3 Quest transition plan.
//!
//! Every Quest retains one generator-v4 semantic plan. No successful result grants artifact,
//! build, compiler, publication, deployment, save, or runtime authority.

use std::collections::BTreeSet;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::model_revision3::{
    EntityKind, EntityPayload, OriginRef, QuestDraft, QuestTransitionPlanV1,
    REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
};
use crate::quest::validate_draft_quest_transition_plan_v1;
use crate::revision3_quest::regenerate_revision3_quest_module;
use crate::strict_json::reject_duplicate_object_keys;
use crate::QuestCollisionCatalogInput;
use crate::{
    ContentSeal, EntityId, GameGenerationAnchor, ProjectId, ProjectRevision3,
    ProjectRevision3JsonError, Sha256Digest, WorkingHead,
};

const REVISION3_QUEST_TRANSITION_PLAN_SEAL_DOMAIN_V1: &[u8] =
    b"gore-authoring.revision3-quest-transition-plan-v1\0";

pub const MAX_REVISION3_QUEST_TRANSITION_PLAN_JSON_BYTES_V1: usize = 384 * 1024;
pub const MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1: usize = 512 * 1024;

/// The exact transition-plan basis of one revision-3 Quest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3QuestTransitionPlanBasisV1 {
    pub plan: QuestTransitionPlanV1,
    pub seal: ContentSeal,
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestTransitionPlanBasisErrorV1 {
    #[error(
        "unsupported Quest generator contract: expected {expected_id}@4, got {actual_id}@{actual_version}"
    )]
    UnsupportedGeneratorContract {
        expected_id: &'static str,
        actual_id: String,
        actual_version: u32,
    },
    #[error("invalid Quest transition-plan basis: {reason}")]
    InvalidPlan { reason: String },
    #[error("could not seal Quest transition-plan basis: {0}")]
    Seal(#[from] Revision3QuestTransitionPlanSealErrorV1),
}

/// Return the retained plan and CAS seal for one Quest Draft.
pub fn revision3_quest_transition_plan_basis_v1(
    quest: &QuestDraft,
) -> Result<Revision3QuestTransitionPlanBasisV1, Revision3QuestTransitionPlanBasisErrorV1> {
    if quest.generator_id != REVISION3_QUEST_GENERATOR_ID
        || quest.generator_version != REVISION3_QUEST_GENERATOR_VERSION
    {
        return Err(
            Revision3QuestTransitionPlanBasisErrorV1::UnsupportedGeneratorContract {
                expected_id: REVISION3_QUEST_GENERATOR_ID,
                actual_id: quest.generator_id.clone(),
                actual_version: quest.generator_version,
            },
        );
    }

    let objective_count = 1usize.saturating_add(quest.input.additional_objective_titles.len());
    let plan = (*quest.input.transition_plan).clone();
    validate_draft_quest_transition_plan_v1(&plan, objective_count).map_err(|error| {
        Revision3QuestTransitionPlanBasisErrorV1::InvalidPlan {
            reason: error.to_string(),
        }
    })?;
    let seal = revision3_quest_transition_plan_seal_v1(&plan)?;
    Ok(Revision3QuestTransitionPlanBasisV1 { plan, seal })
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestTransitionPlanSealErrorV1 {
    #[error("Quest transition plan exceeds the {limit}-byte canonical JSON limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("could not serialize Quest transition plan: {0}")]
    Serialize(#[source] serde_json::Error),
}

/// Compute the bounded, domain-separated canonical seal used by transition-plan CAS.
pub fn revision3_quest_transition_plan_seal_v1(
    plan: &QuestTransitionPlanV1,
) -> Result<ContentSeal, Revision3QuestTransitionPlanSealErrorV1> {
    let mut writer = BoundedJsonWriter::new(MAX_REVISION3_QUEST_TRANSITION_PLAN_JSON_BYTES_V1);
    let serialized = serde_json::to_writer(&mut writer, plan);
    if let Some(actual) = writer.first_exceeded_size {
        return Err(Revision3QuestTransitionPlanSealErrorV1::InputTooLarge {
            actual,
            limit: MAX_REVISION3_QUEST_TRANSITION_PLAN_JSON_BYTES_V1,
        });
    }
    serialized.map_err(Revision3QuestTransitionPlanSealErrorV1::Serialize)?;
    let mut hasher = Sha256::new();
    hasher.update(REVISION3_QUEST_TRANSITION_PLAN_SEAL_DOMAIN_V1);
    hasher.update((writer.bytes.len() as u64).to_be_bytes());
    hasher.update(&writer.bytes);
    Ok(ContentSeal {
        byte_len: writer.bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(hasher.finalize().into()),
    })
}

/// Exact project/head/entity/plan-CAS-bound transition-plan edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestTransitionPlanEditRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub quest_id: EntityId,
    pub expected_quest_revision: u64,
    pub expected_transition_plan_seal: ContentSeal,
    pub transition_plan: QuestTransitionPlanV1,
}

impl Revision3QuestTransitionPlanEditRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(
        json: &str,
    ) -> Result<Self, Revision3QuestTransitionPlanEditRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1 {
            return Err(
                Revision3QuestTransitionPlanEditRequestJsonErrorV1::InputTooLarge {
                    actual: json.len(),
                    limit: MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3QuestTransitionPlanEditRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3QuestTransitionPlanEditRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3QuestTransitionPlanEditRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(
        &self,
    ) -> Result<String, Revision3QuestTransitionPlanEditRequestJsonErrorV1> {
        let mut writer =
            BoundedJsonWriter::new(MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(
                Revision3QuestTransitionPlanEditRequestJsonErrorV1::InputTooLarge {
                    actual,
                    limit: MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        serialized.map_err(Revision3QuestTransitionPlanEditRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3QuestTransitionPlanEditRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestTransitionPlanEditRequestJsonErrorV1 {
    #[error(
        "revision-3 Quest transition-plan edit request exceeds the {limit}-byte limit: {actual} bytes"
    )]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Quest transition-plan edit request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Quest transition-plan edit request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error(
        "revision-3 Quest transition-plan edit request JSON is not in its exact canonical spelling"
    )]
    NonCanonicalJson,
    #[error("revision-3 Quest transition-plan edit request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Stable semantic conflict. A rejection never contains a partially edited project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3QuestTransitionPlanEditConflictV1 {
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
    #[error("Quest {quest} has an invalid owned ScriptModule closure: {reason}")]
    InvalidQuestClosure { quest: EntityId, reason: String },
    #[error("Quest {quest} owned ScriptModule differs from deterministic regeneration")]
    OwnedModuleDrift { quest: EntityId, module: EntityId },
    #[error("Quest owned ScriptModule {module} revision cannot be incremented")]
    ScriptModuleRevisionOverflow { module: EntityId },
    #[error("expected transition-plan seal differs from the exact effective plan")]
    TransitionPlanSealConflict {
        expected: ContentSeal,
        actual: ContentSeal,
    },
    #[error("generator-v4 Quest transition-plan edit does not change the retained plan")]
    NoChanges,
    #[error("transition-plan edit cannot replace active stable objective slots")]
    ObjectiveSlotsChanged,
    #[error("transition-plan next unused slot ordinal regressed from {current} to {requested}")]
    NextSlotOrdinalRegression { current: u16, requested: u16 },
    #[error("invalid Quest transition plan: {reason}")]
    InvalidTransitionPlan { reason: String },
    #[error("transition-plan edit unexpectedly changed a preserved technical module identity")]
    TechnicalIdentityChanged,
    #[error("transition-plan candidate exceeds the {limit}-byte project limit: {actual} bytes")]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3QuestTransitionPlanEditRejectionV1 {
    pub conflict: Revision3QuestTransitionPlanEditConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestTransitionPlanEditBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestTransitionPlanEditRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestTransitionPlanEditPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3QuestTransitionPlanEditOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub quest_id: EntityId,
    pub script_module_id: EntityId,
    pub quest_revision: u64,
    pub script_module_revision: u64,
    pub transition_plan_seal: ContentSeal,
    pub build_status: Revision3QuestTransitionPlanEditBuildStatusV1,
    pub runtime_status: Revision3QuestTransitionPlanEditRuntimeStatusV1,
    pub publication_status: Revision3QuestTransitionPlanEditPublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3QuestTransitionPlanEditEvaluationV1 {
    Applied(Box<Revision3QuestTransitionPlanEditOutcomeV1>),
    Rejected(Revision3QuestTransitionPlanEditRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestTransitionPlanEditErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 Quest transition-plan edit request: {0}")]
    InvalidRequest(#[source] Revision3QuestTransitionPlanEditRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 Quest transition-plan candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Atomically edit one Quest's semantic transition plan and deterministic owned ScriptModule.
///
/// Both JSON inputs must be their exact canonical bytes. Existing source is first proven through
/// deterministic regeneration with the retained collision evidence represented by an empty
/// collision input. The same empty input regenerates the candidate, so this operation cannot add
/// technical identities. No artifact is opened and no external operation is reachable here.
pub fn apply_revision3_quest_transition_plan_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3QuestTransitionPlanEditEvaluationV1, Revision3QuestTransitionPlanEditErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3QuestTransitionPlanEditErrorV1::InvalidProject)?;
    let request = Revision3QuestTransitionPlanEditRequestV1::from_json(canonical_request_json)
        .map_err(Revision3QuestTransitionPlanEditErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3QuestTransitionPlanEditEvaluationV1::Rejected(
                Revision3QuestTransitionPlanEditRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3QuestTransitionPlanEditConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3QuestTransitionPlanEditConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3QuestTransitionPlanEditConflictV1::ProjectRevisionOverflow);
    };
    if is_zero_entity_id(request.quest_id) {
        reject!(Revision3QuestTransitionPlanEditConflictV1::ZeroQuestId);
    }

    let Some(quest_entity) = project.entities.get(&request.quest_id) else {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::InvalidQuestEntity {
                quest: request.quest_id,
            }
        );
    };
    let EntityPayload::QuestDraft(quest) = &quest_entity.payload else {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::InvalidQuestEntity {
                quest: request.quest_id,
            }
        );
    };
    if request.expected_quest_revision != quest_entity.revision {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::QuestRevisionConflict {
                expected: request.expected_quest_revision,
                actual: quest_entity.revision,
            }
        );
    }
    let Some(next_quest_revision) = quest_entity.revision.checked_add(1) else {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::QuestRevisionOverflow {
                quest: request.quest_id,
            }
        );
    };
    let quest = quest.clone();
    let script_module_id = quest.script_module.id;
    if quest.script_module.project_id != project.project_id
        || quest.script_module.expected_kind != EntityKind::ScriptModule
        || is_zero_entity_id(script_module_id)
    {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::InvalidQuestClosure {
                quest: request.quest_id,
                reason: "ScriptModule reference is not exact-project, non-zero, and kind-bound"
                    .to_owned(),
            }
        );
    }

    let Some(module_entity) = project.entities.get(&script_module_id) else {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::InvalidQuestClosure {
                quest: request.quest_id,
                reason: "owned ScriptModule is missing".to_owned(),
            }
        );
    };
    let EntityPayload::ScriptModule(existing_module) = &module_entity.payload else {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::InvalidQuestClosure {
                quest: request.quest_id,
                reason: "owned entity is not a ScriptModule".to_owned(),
            }
        );
    };
    if existing_module.owner.project_id != project.project_id
        || existing_module.owner.id != request.quest_id
        || existing_module.owner.expected_kind != EntityKind::QuestDraft
    {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::InvalidQuestClosure {
                quest: request.quest_id,
                reason: "ScriptModule owner is not the exact Quest".to_owned(),
            }
        );
    }
    let Some(next_script_module_revision) = module_entity.revision.checked_add(1) else {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::ScriptModuleRevisionOverflow {
                module: script_module_id,
            }
        );
    };
    let existing_module = existing_module.clone();
    let existing_module_origin = module_entity.origin.clone();
    let OriginRef::Generated {
        generator_id: origin_generator_id,
        generator_version: origin_generator_version,
        owner: origin_owner,
    } = &existing_module_origin
    else {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::InvalidQuestClosure {
                quest: request.quest_id,
                reason: "ScriptModule origin is not generated".to_owned(),
            }
        );
    };
    if origin_generator_id != &quest.generator_id
        || *origin_generator_version != quest.generator_version
        || origin_owner != &existing_module.owner
        || existing_module.generator_id != quest.generator_id
        || existing_module.generator_version != quest.generator_version
    {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::InvalidQuestClosure {
                quest: request.quest_id,
                reason: "Quest, module, and generated origin contracts differ".to_owned(),
            }
        );
    }

    let collision_input = empty_collision_input(&quest);
    let regenerated_existing =
        match regenerate_revision3_quest_module(&quest, collision_input.clone()) {
            Ok(module) => module,
            Err(error) => {
                reject!(
                    Revision3QuestTransitionPlanEditConflictV1::InvalidQuestClosure {
                        quest: request.quest_id,
                        reason: error.to_string(),
                    }
                );
            }
        };
    if existing_module != regenerated_existing {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::OwnedModuleDrift {
                quest: request.quest_id,
                module: script_module_id,
            }
        );
    }

    let basis = match revision3_quest_transition_plan_basis_v1(&quest) {
        Ok(basis) => basis,
        Err(error) => {
            reject!(
                Revision3QuestTransitionPlanEditConflictV1::InvalidQuestClosure {
                    quest: request.quest_id,
                    reason: error.to_string(),
                }
            );
        }
    };
    if request.expected_transition_plan_seal != basis.seal {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::TransitionPlanSealConflict {
                expected: request.expected_transition_plan_seal,
                actual: basis.seal,
            }
        );
    }
    if request.transition_plan == basis.plan {
        reject!(Revision3QuestTransitionPlanEditConflictV1::NoChanges);
    }
    if request.transition_plan.objective_slots != basis.plan.objective_slots {
        reject!(Revision3QuestTransitionPlanEditConflictV1::ObjectiveSlotsChanged);
    }
    if request.transition_plan.next_slot_ordinal < basis.plan.next_slot_ordinal {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::NextSlotOrdinalRegression {
                current: basis.plan.next_slot_ordinal,
                requested: request.transition_plan.next_slot_ordinal,
            }
        );
    }

    let objective_count = 1usize.saturating_add(quest.input.additional_objective_titles.len());
    if let Err(error) =
        validate_draft_quest_transition_plan_v1(&request.transition_plan, objective_count)
    {
        reject!(
            Revision3QuestTransitionPlanEditConflictV1::InvalidTransitionPlan {
                reason: error.to_string(),
            }
        );
    }

    let mut edited_quest = quest.clone();
    edited_quest.input.transition_plan = Box::new(request.transition_plan.clone());
    let edited_module = match regenerate_revision3_quest_module(&edited_quest, collision_input) {
        Ok(module) => module,
        Err(error) => {
            reject!(
                Revision3QuestTransitionPlanEditConflictV1::InvalidTransitionPlan {
                    reason: error.to_string(),
                }
            );
        }
    };
    if edited_module.module_namespace != existing_module.module_namespace
        || edited_module.module_relative_path != existing_module.module_relative_path
        || edited_module.owner != existing_module.owner
        || edited_module.generator_id != existing_module.generator_id
        || edited_module.generator_version != REVISION3_QUEST_GENERATOR_VERSION
        || edited_module.status != existing_module.status
    {
        reject!(Revision3QuestTransitionPlanEditConflictV1::TechnicalIdentityChanged);
    }
    let transition_plan_seal =
        match revision3_quest_transition_plan_seal_v1(&request.transition_plan) {
            Ok(seal) => seal,
            Err(error) => {
                reject!(
                    Revision3QuestTransitionPlanEditConflictV1::InvalidTransitionPlan {
                        reason: error.to_string(),
                    }
                );
            }
        };

    let Some(quest_entity) = project.entities.get_mut(&request.quest_id) else {
        unreachable!("Quest was resolved above")
    };
    quest_entity.revision = next_quest_revision;
    quest_entity.payload = EntityPayload::QuestDraft(edited_quest);
    let Some(module_entity) = project.entities.get_mut(&script_module_id) else {
        unreachable!("owned ScriptModule was resolved above")
    };
    module_entity.revision = next_script_module_revision;
    module_entity.origin = OriginRef::Generated {
        generator_id: origin_generator_id.clone(),
        generator_version: REVISION3_QUEST_GENERATOR_VERSION,
        owner: origin_owner.clone(),
    };
    module_entity.payload = EntityPayload::ScriptModule(edited_module);
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(
                Revision3QuestTransitionPlanEditConflictV1::CandidateTooLarge { actual, limit }
            );
        }
        Err(error) => {
            reject!(
                Revision3QuestTransitionPlanEditConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3QuestTransitionPlanEditErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3QuestTransitionPlanEditErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3QuestTransitionPlanEditEvaluationV1::Applied(
        Box::new(Revision3QuestTransitionPlanEditOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            quest_id: request.quest_id,
            script_module_id,
            quest_revision: next_quest_revision,
            script_module_revision: next_script_module_revision,
            transition_plan_seal,
            build_status: Revision3QuestTransitionPlanEditBuildStatusV1::Blocked,
            runtime_status: Revision3QuestTransitionPlanEditRuntimeStatusV1::RuntimeUnqualified,
            publication_status: Revision3QuestTransitionPlanEditPublicationStatusV1::NotSupported,
        }),
    ))
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

fn is_zero_entity_id(id: EntityId) -> bool {
    id.as_bytes().iter().all(|byte| *byte == 0)
}

struct BoundedJsonWriter {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedJsonWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedJsonWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let new_len = self.bytes.len().saturating_add(bytes.len());
        if new_len > self.limit {
            self.first_exceeded_size.get_or_insert(new_len);
            return Err(io::Error::other(
                "revision-3 Quest transition-plan JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
