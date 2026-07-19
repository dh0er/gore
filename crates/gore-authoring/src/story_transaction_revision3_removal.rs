//! Exact-basis-bound removal of one revision-3 Story Draft and its uniquely owned module.
//!
//! This transaction is filesystem-free. It proves the complete local ownership closure through
//! the shared revision-3 content index, removes exactly two entities from a basis clone, and
//! preserves every other entity and the complete AssetStore. Persistence remains a separate
//! prepare-only working-store operation and never publishes the fixed head.

use std::collections::BTreeSet;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    EntityKind, EntityPayload, OriginRef, ProjectRevision3, ScriptModule, ScriptModuleStatus,
};
use crate::revision3_content_index::{
    build_revision3_content_index_v1, Revision3ContentIndexErrorV1,
    Revision3ContentReferenceResolutionV1, Revision3ContentReferenceRoleV1,
};
use crate::revision3_quest::regenerate_revision3_quest_module;
use crate::strict_json::reject_duplicate_object_keys;
use crate::QuestCollisionCatalogInput;
use crate::{EntityId, GameGenerationAnchor, ProjectId, ProjectRevision3JsonError, WorkingHead};

/// Maximum exact canonical Story Draft removal request size.
pub const MAX_REVISION3_STORY_DRAFT_REMOVAL_REQUEST_JSON_BYTES_V1: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3StoryDraftRemovalKindV1 {
    NpcDraft,
    QuestDraft,
}

impl Revision3StoryDraftRemovalKindV1 {
    pub const fn entity_kind(self) -> EntityKind {
        match self {
            Self::NpcDraft => EntityKind::NpcDraft,
            Self::QuestDraft => EntityKind::QuestDraft,
        }
    }
}

/// Exact head/project/entity-CAS binding for one two-entity Story Draft removal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3StoryDraftRemovalRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub draft_id: EntityId,
    pub draft_kind: Revision3StoryDraftRemovalKindV1,
    pub expected_draft_revision: u64,
    pub script_module_id: EntityId,
    pub expected_script_module_revision: u64,
}

impl Revision3StoryDraftRemovalRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3StoryDraftRemovalRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_STORY_DRAFT_REMOVAL_REQUEST_JSON_BYTES_V1 {
            return Err(
                Revision3StoryDraftRemovalRequestJsonErrorV1::InputTooLarge {
                    actual: json.len(),
                    limit: MAX_REVISION3_STORY_DRAFT_REMOVAL_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3StoryDraftRemovalRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3StoryDraftRemovalRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3StoryDraftRemovalRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(
        &self,
    ) -> Result<String, Revision3StoryDraftRemovalRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_STORY_DRAFT_REMOVAL_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(
                Revision3StoryDraftRemovalRequestJsonErrorV1::InputTooLarge {
                    actual,
                    limit: MAX_REVISION3_STORY_DRAFT_REMOVAL_REQUEST_JSON_BYTES_V1,
                },
            );
        }
        serialized.map_err(Revision3StoryDraftRemovalRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3StoryDraftRemovalRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3StoryDraftRemovalRequestJsonErrorV1 {
    #[error(
        "revision-3 Story Draft removal request exceeds the {limit}-byte limit: {actual} bytes"
    )]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Story Draft removal request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Story Draft removal request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Story Draft removal request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Story Draft removal request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3StoryDraftRemovalConflictV1 {
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
    #[error("Story Draft entity ID must not be zero")]
    ZeroDraftId,
    #[error("ScriptModule entity ID must not be zero")]
    ZeroScriptModuleId,
    #[error("Story Draft and ScriptModule IDs must be distinct")]
    IdentityCollision,
    #[error("Story Draft entity {draft} is missing")]
    MissingDraftEntity { draft: EntityId },
    #[error("Story Draft entity {draft} has kind {actual:?}; expected {expected:?}")]
    DraftKindMismatch {
        draft: EntityId,
        expected: EntityKind,
        actual: EntityKind,
    },
    #[error("expected Story Draft revision {expected}, but exact basis is {actual}")]
    DraftRevisionConflict { expected: u64, actual: u64 },
    #[error("Story Draft {draft} does not bind the requested exact local ScriptModule {module}")]
    DraftModuleBindingMismatch { draft: EntityId, module: EntityId },
    #[error("ScriptModule entity {module} is missing or has the wrong kind")]
    InvalidScriptModuleEntity { module: EntityId },
    #[error("expected ScriptModule revision {expected}, but exact basis is {actual}")]
    ScriptModuleRevisionConflict { expected: u64, actual: u64 },
    #[error("Story Draft {draft} and ScriptModule {module} are inconsistent: {reason}")]
    PayloadOriginGeneratorMismatch {
        draft: EntityId,
        module: EntityId,
        reason: String,
    },
    #[error(
        "Story Draft {draft} and ScriptModule {module} do not have the exact three-edge ownership closure: {reason}"
    )]
    OwnershipConflict {
        draft: EntityId,
        module: EntityId,
        reason: String,
    },
    #[error("Story Draft {draft} is additionally referenced by {source_entity} through {role:?}")]
    DraftReferenced {
        draft: EntityId,
        source_entity: EntityId,
        role: Revision3ContentReferenceRoleV1,
    },
    #[error(
        "ScriptModule {module} is additionally referenced by {source_entity} through {role:?}"
    )]
    ModuleReferenced {
        module: EntityId,
        source_entity: EntityId,
        role: Revision3ContentReferenceRoleV1,
    },
    #[error("Story Draft removal preflight exceeds the {limit}-reference limit")]
    ReferenceLimit { limit: usize },
    #[error(
        "Story Draft removal candidate exceeds the {limit}-byte project limit: {actual} bytes"
    )]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("Story Draft removal candidate is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3StoryDraftRemovalRejectionV1 {
    pub conflict: Revision3StoryDraftRemovalConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3StoryDraftRemovalBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3StoryDraftRemovalRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3StoryDraftRemovalArtifactAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3StoryDraftRemovalPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3StoryDraftRemovalOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub draft_id: EntityId,
    pub draft_kind: Revision3StoryDraftRemovalKindV1,
    pub draft_revision: u64,
    pub script_module_id: EntityId,
    pub script_module_revision: u64,
    pub build_status: Revision3StoryDraftRemovalBuildStatusV1,
    pub runtime_status: Revision3StoryDraftRemovalRuntimeStatusV1,
    pub artifact_authority: Revision3StoryDraftRemovalArtifactAuthorityV1,
    pub publication_status: Revision3StoryDraftRemovalPublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3StoryDraftRemovalEvaluationV1 {
    Applied(Box<Revision3StoryDraftRemovalOutcomeV1>),
    Rejected(Revision3StoryDraftRemovalRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3StoryDraftRemovalErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 Story Draft removal request: {0}")]
    InvalidRequest(#[source] Revision3StoryDraftRemovalRequestJsonErrorV1),
    #[error("could not build the exact revision-3 content index: {0}")]
    ContentIndex(#[source] Revision3ContentIndexErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 Story Draft removal candidate reopen changed the project")]
    CanonicalReopenMismatch,
    #[error("Story Draft removal changed a preserved project value")]
    CandidatePreservationMismatch,
}

/// Remove exactly one Story Draft and its one isolated, deterministic ScriptModule.
pub fn apply_revision3_story_draft_removal_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3StoryDraftRemovalEvaluationV1, Revision3StoryDraftRemovalErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3StoryDraftRemovalErrorV1::InvalidProject)?;
    let request = Revision3StoryDraftRemovalRequestV1::from_json(canonical_request_json)
        .map_err(Revision3StoryDraftRemovalErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3StoryDraftRemovalEvaluationV1::Rejected(
                Revision3StoryDraftRemovalRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3StoryDraftRemovalConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(
            Revision3StoryDraftRemovalConflictV1::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3StoryDraftRemovalConflictV1::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3StoryDraftRemovalConflictV1::ProjectTargetMismatch);
    }
    let Some(next_revision) = project.revision.checked_add(1) else {
        reject!(Revision3StoryDraftRemovalConflictV1::ProjectRevisionOverflow);
    };
    if is_zero_entity_id(request.draft_id) {
        reject!(Revision3StoryDraftRemovalConflictV1::ZeroDraftId);
    }
    if is_zero_entity_id(request.script_module_id) {
        reject!(Revision3StoryDraftRemovalConflictV1::ZeroScriptModuleId);
    }
    if request.draft_id == request.script_module_id {
        reject!(Revision3StoryDraftRemovalConflictV1::IdentityCollision);
    }

    let Some(draft_entity) = project.entities.get(&request.draft_id) else {
        reject!(Revision3StoryDraftRemovalConflictV1::MissingDraftEntity {
            draft: request.draft_id,
        });
    };
    let expected_kind = request.draft_kind.entity_kind();
    if draft_entity.kind() != expected_kind {
        reject!(Revision3StoryDraftRemovalConflictV1::DraftKindMismatch {
            draft: request.draft_id,
            expected: expected_kind,
            actual: draft_entity.kind(),
        });
    }
    if draft_entity.revision != request.expected_draft_revision {
        reject!(
            Revision3StoryDraftRemovalConflictV1::DraftRevisionConflict {
                expected: request.expected_draft_revision,
                actual: draft_entity.revision,
            }
        );
    }

    let (draft_module, draft_generator_id, draft_generator_version, regenerated_module) =
        match (&request.draft_kind, &draft_entity.payload) {
            (Revision3StoryDraftRemovalKindV1::NpcDraft, EntityPayload::NpcDraft(draft)) => {
                let owner = crate::Revision3TypedRef::new(
                    project.project_id,
                    request.draft_id,
                    EntityKind::NpcDraft,
                );
                let regenerated = match draft.regenerate_script_module(owner) {
                    Ok(module) => module,
                    Err(error) => reject!(
                        Revision3StoryDraftRemovalConflictV1::PayloadOriginGeneratorMismatch {
                            draft: request.draft_id,
                            module: request.script_module_id,
                            reason: error.to_string(),
                        }
                    ),
                };
                (
                    draft.script_module.clone(),
                    draft.generator_id.clone(),
                    draft.generator_version,
                    regenerated,
                )
            }
            (Revision3StoryDraftRemovalKindV1::QuestDraft, EntityPayload::QuestDraft(draft)) => {
                let regenerated =
                    match regenerate_revision3_quest_module(draft, empty_collision_input(draft)) {
                        Ok(module) => module,
                        Err(error) => reject!(
                            Revision3StoryDraftRemovalConflictV1::PayloadOriginGeneratorMismatch {
                                draft: request.draft_id,
                                module: request.script_module_id,
                                reason: error.to_string(),
                            }
                        ),
                    };
                (
                    draft.script_module.clone(),
                    draft.generator_id.clone(),
                    draft.generator_version,
                    regenerated,
                )
            }
            _ => reject!(Revision3StoryDraftRemovalConflictV1::DraftKindMismatch {
                draft: request.draft_id,
                expected: expected_kind,
                actual: draft_entity.kind(),
            }),
        };

    if draft_module.project_id != project.project_id
        || draft_module.id != request.script_module_id
        || draft_module.expected_kind != EntityKind::ScriptModule
    {
        reject!(
            Revision3StoryDraftRemovalConflictV1::DraftModuleBindingMismatch {
                draft: request.draft_id,
                module: request.script_module_id,
            }
        );
    }
    let Some(module_entity) = project.entities.get(&request.script_module_id) else {
        reject!(
            Revision3StoryDraftRemovalConflictV1::InvalidScriptModuleEntity {
                module: request.script_module_id,
            }
        );
    };
    let EntityPayload::ScriptModule(module) = &module_entity.payload else {
        reject!(
            Revision3StoryDraftRemovalConflictV1::InvalidScriptModuleEntity {
                module: request.script_module_id,
            }
        );
    };
    if module_entity.revision != request.expected_script_module_revision {
        reject!(
            Revision3StoryDraftRemovalConflictV1::ScriptModuleRevisionConflict {
                expected: request.expected_script_module_revision,
                actual: module_entity.revision,
            }
        );
    }
    if let Err(reason) = validate_exact_module_binding(
        &project,
        &request,
        module,
        module_entity,
        &draft_generator_id,
        draft_generator_version,
        &regenerated_module,
    ) {
        reject!(
            Revision3StoryDraftRemovalConflictV1::PayloadOriginGeneratorMismatch {
                draft: request.draft_id,
                module: request.script_module_id,
                reason,
            }
        );
    }

    let index = match build_revision3_content_index_v1(&project) {
        Ok(index) => index,
        Err(Revision3ContentIndexErrorV1::TooManyReferences { limit }) => {
            reject!(Revision3StoryDraftRemovalConflictV1::ReferenceLimit { limit });
        }
        Err(error) => return Err(Revision3StoryDraftRemovalErrorV1::ContentIndex(error)),
    };
    if let Err(blocker) = validate_isolated_removal_closure(&project, &request, &index) {
        let conflict = match blocker {
            ClosureBlocker::Ownership(reason) => {
                Revision3StoryDraftRemovalConflictV1::OwnershipConflict {
                    draft: request.draft_id,
                    module: request.script_module_id,
                    reason,
                }
            }
            ClosureBlocker::DraftReferenced {
                source_entity,
                role,
            } => Revision3StoryDraftRemovalConflictV1::DraftReferenced {
                draft: request.draft_id,
                source_entity,
                role,
            },
            ClosureBlocker::ModuleReferenced {
                source_entity,
                role,
            } => Revision3StoryDraftRemovalConflictV1::ModuleReferenced {
                module: request.script_module_id,
                source_entity,
                role,
            },
        };
        reject!(conflict);
    }

    let basis_project = project.clone();
    project.entities.remove(&request.draft_id);
    project.entities.remove(&request.script_module_id);
    project.revision = next_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3StoryDraftRemovalConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(
                Revision3StoryDraftRemovalConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3StoryDraftRemovalErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3StoryDraftRemovalErrorV1::CanonicalReopenMismatch);
    }
    if !preserves_exact_basis(&basis_project, &reopened, &request) {
        return Err(Revision3StoryDraftRemovalErrorV1::CandidatePreservationMismatch);
    }
    build_revision3_content_index_v1(&reopened)
        .map_err(Revision3StoryDraftRemovalErrorV1::ContentIndex)?;

    Ok(Revision3StoryDraftRemovalEvaluationV1::Applied(Box::new(
        Revision3StoryDraftRemovalOutcomeV1 {
            project: reopened,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            draft_id: request.draft_id,
            draft_kind: request.draft_kind,
            draft_revision: request.expected_draft_revision,
            script_module_id: request.script_module_id,
            script_module_revision: request.expected_script_module_revision,
            build_status: Revision3StoryDraftRemovalBuildStatusV1::Blocked,
            runtime_status: Revision3StoryDraftRemovalRuntimeStatusV1::RuntimeUnqualified,
            artifact_authority: Revision3StoryDraftRemovalArtifactAuthorityV1::NotGranted,
            publication_status: Revision3StoryDraftRemovalPublicationStatusV1::NotSupported,
        },
    )))
}

fn validate_exact_module_binding(
    project: &ProjectRevision3,
    request: &Revision3StoryDraftRemovalRequestV1,
    module: &ScriptModule,
    module_entity: &crate::Revision3Entity,
    draft_generator_id: &str,
    draft_generator_version: u32,
    regenerated_module: &ScriptModule,
) -> Result<(), String> {
    if module.owner.project_id != project.project_id
        || module.owner.id != request.draft_id
        || module.owner.expected_kind != request.draft_kind.entity_kind()
    {
        return Err("ScriptModule payload owner is not the exact requested Draft".to_owned());
    }
    if module.generator_id != draft_generator_id
        || module.generator_version != draft_generator_version
        || module.status != ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
    {
        return Err("ScriptModule payload generator/status differs from its Draft".to_owned());
    }
    if !matches!(
        &module_entity.origin,
        OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } if generator_id == draft_generator_id
            && *generator_version == draft_generator_version
            && owner == &module.owner
    ) {
        return Err("ScriptModule origin does not mirror payload generator and owner".to_owned());
    }
    if module != regenerated_module {
        return Err("ScriptModule payload differs from deterministic regeneration".to_owned());
    }
    Ok(())
}

fn validate_isolated_removal_closure(
    project: &ProjectRevision3,
    request: &Revision3StoryDraftRemovalRequestV1,
    index: &crate::Revision3ContentIndexV1,
) -> Result<(), ClosureBlocker> {
    let mut draft_module_edges = 0u8;
    let mut origin_owner_edges = 0u8;
    let mut script_owner_edges = 0u8;

    for source in &index.entities {
        for reference in &source.references {
            if reference.target.project_id != project.project_id {
                // A foreign-project target with the same 128-bit entity ID is not a local edge.
                continue;
            }
            let source_is_removed =
                source.id == request.draft_id || source.id == request.script_module_id;
            let target_is_removed = reference.target.entity_id == request.draft_id
                || reference.target.entity_id == request.script_module_id;
            if !source_is_removed && !target_is_removed {
                continue;
            }
            match (
                source.id,
                reference.role,
                reference.target.entity_id,
                reference.target.expected_kind,
            ) {
                (
                    source,
                    Revision3ContentReferenceRoleV1::DraftScriptModule,
                    target,
                    EntityKind::ScriptModule,
                ) if source == request.draft_id && target == request.script_module_id => {
                    require_exact_ownership_edge(reference)?;
                    draft_module_edges = draft_module_edges.saturating_add(1);
                }
                (source, Revision3ContentReferenceRoleV1::OriginOwner, target, kind)
                    if source == request.script_module_id
                        && target == request.draft_id
                        && kind == request.draft_kind.entity_kind() =>
                {
                    require_exact_ownership_edge(reference)?;
                    origin_owner_edges = origin_owner_edges.saturating_add(1);
                }
                (source, Revision3ContentReferenceRoleV1::ScriptOwner, target, kind)
                    if source == request.script_module_id
                        && target == request.draft_id
                        && kind == request.draft_kind.entity_kind() =>
                {
                    require_exact_ownership_edge(reference)?;
                    script_owner_edges = script_owner_edges.saturating_add(1);
                }
                (
                    source,
                    Revision3ContentReferenceRoleV1::QuestTranscriptLine,
                    _,
                    EntityKind::DialogLine,
                ) if source == request.draft_id
                    && request.draft_kind == Revision3StoryDraftRemovalKindV1::QuestDraft =>
                {
                    // Transcript bindings are authoring-only outgoing edges. Removing their Quest
                    // drops the edges but deliberately leaves shared DialogLine/localization/voice
                    // entities untouched.
                }
                (
                    source,
                    Revision3ContentReferenceRoleV1::NpcGreetingLine,
                    _,
                    EntityKind::DialogLine,
                ) if source == request.draft_id
                    && request.draft_kind == Revision3StoryDraftRemovalKindV1::NpcDraft =>
                {
                    // Greeting bindings are authoring-only outgoing edges. Removing their NPC
                    // drops the edges but deliberately leaves shared DialogLine/localization/voice
                    // entities untouched.
                }
                _ if reference.target.entity_id == request.draft_id => {
                    return Err(ClosureBlocker::DraftReferenced {
                        source_entity: source.id,
                        role: reference.role,
                    });
                }
                _ if reference.target.entity_id == request.script_module_id => {
                    return Err(ClosureBlocker::ModuleReferenced {
                        source_entity: source.id,
                        role: reference.role,
                    });
                }
                _ => {
                    return Err(ClosureBlocker::Ownership(
                        "a removed entity has an additional local reference".to_owned(),
                    ));
                }
            }
        }
    }

    if (draft_module_edges, origin_owner_edges, script_owner_edges) != (1, 1, 1) {
        return Err(ClosureBlocker::Ownership(
            "the three exact ownership edges do not each occur exactly once".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ClosureBlocker {
    Ownership(String),
    DraftReferenced {
        source_entity: EntityId,
        role: Revision3ContentReferenceRoleV1,
    },
    ModuleReferenced {
        source_entity: EntityId,
        role: Revision3ContentReferenceRoleV1,
    },
}

fn require_exact_ownership_edge(
    reference: &crate::Revision3ContentReferenceV1,
) -> Result<(), ClosureBlocker> {
    if reference.qualifier.is_some()
        || reference.resolution != Revision3ContentReferenceResolutionV1::Resolved
    {
        return Err(ClosureBlocker::Ownership(
            "an ownership edge is qualified or unresolved".to_owned(),
        ));
    }
    Ok(())
}

fn preserves_exact_basis(
    basis: &ProjectRevision3,
    candidate: &ProjectRevision3,
    request: &Revision3StoryDraftRemovalRequestV1,
) -> bool {
    if candidate.revision != basis.revision.saturating_add(1)
        || candidate.project_id != basis.project_id
        || candidate.meta != basis.meta
        || candidate.target != basis.target
        || candidate.authoring_locales != basis.authoring_locales
        || candidate.asset_store != basis.asset_store
        || candidate.entities.len().saturating_add(2) != basis.entities.len()
        || candidate.entities.contains_key(&request.draft_id)
        || candidate.entities.contains_key(&request.script_module_id)
    {
        return false;
    }
    basis.entities.iter().all(|(id, entity)| {
        if *id == request.draft_id || *id == request.script_module_id {
            true
        } else {
            candidate.entities.get(id) == Some(entity)
        }
    })
}

fn empty_collision_input(quest: &crate::Revision3QuestDraft) -> QuestCollisionCatalogInput {
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
                "revision-3 Story Draft removal request JSON limit exceeded",
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
    use std::collections::{BTreeMap, BTreeSet};

    use sha2::{Digest as _, Sha256};

    use super::*;
    use crate::model_revision3::{
        DialogLine, Entity, LocalizationEntry, NpcDraft, NpcDraftInput, NpcGreetingBindingV1,
        NpcParentClassInput, QuestCollisionArtifactRef, QuestDraft, QuestDraftInput,
        QuestGiverInput, QuestParentInput, QuestTranscriptBindingV1, SchemaRevisionV3, TypedRef,
    };
    use crate::{
        AssetMeta, AssetStoreIndex, ContentSeal, FormatV2, ProjectMeta, Sha256Digest,
        WorkingStoreFormat, LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2, QUEST_COLLISION_CATALOG_LAYER_V2,
        REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
    };

    const DRAFT_REVISION: u64 = 3;
    const MODULE_REVISION: u64 = 5;

    #[derive(Clone)]
    struct Fixture {
        project: ProjectRevision3,
        head: WorkingHead,
        draft_id: EntityId,
        module_id: EntityId,
        kind: Revision3StoryDraftRemovalKindV1,
        preserved_entity_id: EntityId,
    }

    fn digest(value: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([value; 32])
    }

    fn seal(value: u8, byte_len: u64) -> ContentSeal {
        ContentSeal {
            byte_len,
            sha256: digest(value),
        }
    }

    fn project_id(value: u8) -> ProjectId {
        ProjectId::from_bytes([value; 16])
    }

    fn entity_id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
    }

    fn target(value: u8) -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: seal(value, 123),
        }
    }

    fn head(value: u8) -> WorkingHead {
        WorkingHead {
            store_format: WorkingStoreFormat,
            snapshot: seal(value, 456),
        }
    }

    fn new_origin(value: &str) -> OriginRef {
        OriginRef::New {
            authored_runtime_id: value.to_owned(),
        }
    }

    fn base_project() -> (ProjectRevision3, EntityId) {
        let preserved_entity_id = entity_id(0x40);
        let mut project = ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: project_id(1),
            revision: 7,
            meta: ProjectMeta {
                name: "Removal fixture".to_owned(),
                version: "1.0.0".to_owned(),
                author: "GORE".to_owned(),
            },
            target: target(0xa0),
            authoring_locales: BTreeSet::from(["en".parse().unwrap()]),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        };
        project.asset_store.assets.insert(
            digest(0xe0),
            AssetMeta {
                byte_len: 17,
                media_type: "application/octet-stream".to_owned(),
            },
        );
        project.entities.insert(
            preserved_entity_id,
            Entity {
                id: preserved_entity_id,
                display_name: "Preserved localization".to_owned(),
                origin: new_origin("LOC_PRESERVED"),
                revision: 11,
                payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "LOC_PRESERVED".to_owned(),
                    texts: BTreeMap::from([("en".parse().unwrap(), "Preserved".to_owned())]),
                }),
            },
        );
        (project, preserved_entity_id)
    }

    fn npc_parent(project: &ProjectRevision3, runtime_class: &str) -> NpcParentClassInput {
        NpcParentClassInput {
            generation: project.target.clone(),
            source_seal: seal(0xb0, 4),
            catalog_layer: "fixture.npcs.v1".to_owned(),
            canonical_selector: runtime_class.to_owned(),
            runtime_class: runtime_class.to_owned(),
        }
    }

    fn npc_fixture() -> Fixture {
        let (mut project, preserved_entity_id) = base_project();
        let draft_id = entity_id(0x10);
        let module_id = entity_id(0x11);
        let owner =
            crate::Revision3TypedRef::new(project.project_id, draft_id, EntityKind::NpcDraft);
        let draft = NpcDraft {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: NpcDraftInput {
                target: project.target.clone(),
                module_namespace: "PROJECT.NPCS.REMOVABLE".to_owned(),
                unique_name: "GORE_REMOVABLE_NPC".to_owned(),
                parent_character_definition: npc_parent(&project, "UCharacterDefinition_Asghan"),
                parent_ai_agent_config: npc_parent(&project, "UAIAgentConfig_Asghan"),
                parent_spawn_definition: npc_parent(&project, "USpawnAIAgentDefinition_Asghan"),
            },
            script_module: crate::Revision3TypedRef::new(
                project.project_id,
                module_id,
                EntityKind::ScriptModule,
            ),
            greetings: Vec::new(),
        };
        let module = draft.regenerate_script_module(owner.clone()).unwrap();
        project.entities.insert(
            draft_id,
            Entity {
                id: draft_id,
                display_name: "Removable NPC".to_owned(),
                origin: new_origin("GORE_REMOVABLE_NPC"),
                revision: DRAFT_REVISION,
                payload: EntityPayload::NpcDraft(draft),
            },
        );
        project.entities.insert(
            module_id,
            Entity {
                id: module_id,
                display_name: "Removable NPC module".to_owned(),
                origin: OriginRef::Generated {
                    generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                    generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                    owner,
                },
                revision: MODULE_REVISION,
                payload: EntityPayload::ScriptModule(module),
            },
        );
        Fixture {
            project,
            head: head(0xc0),
            draft_id,
            module_id,
            kind: Revision3StoryDraftRemovalKindV1::NpcDraft,
            preserved_entity_id,
        }
    }

    fn quest_fixture() -> Fixture {
        let (mut project, preserved_entity_id) = base_project();
        let draft_id = entity_id(0x20);
        let module_id = entity_id(0x21);
        let artifact = seal(0x71, 42);
        project.asset_store.assets.insert(
            artifact.sha256,
            AssetMeta {
                byte_len: artifact.byte_len,
                media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
            },
        );
        let owner =
            crate::Revision3TypedRef::new(project.project_id, draft_id, EntityKind::QuestDraft);
        let draft = QuestDraft {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            input: QuestDraftInput {
                target: project.target.clone(),
                quest_id: draft_id,
                module_namespace: "GoreMods.Quests.Removable".to_owned(),
                technical_id: "GORE_QUEST_REMOVABLE".to_owned(),
                text_helper: "GoreQuestRemovableText".to_owned(),
                parent_quest: QuestParentInput {
                    generation: project.target.clone(),
                    source_seal: seal(0x72, 4),
                    catalog_layer: "base-game.g1r.quests".to_owned(),
                    canonical_selector: "CatalogQuest_Parent".to_owned(),
                    runtime_class: "UQuest_Parent".to_owned(),
                },
                giver: QuestGiverInput {
                    generation: project.target.clone(),
                    source_seal: seal(0x73, 4),
                    catalog_layer: "base-game.g1r.characters".to_owned(),
                    canonical_selector: "CatalogCharacter_Asghan".to_owned(),
                    runtime_unique_name: "OM_GRD_Asghan_263".to_owned(),
                },
                title: "Remove me".to_owned(),
                description: "A removable quest".to_owned(),
                objective_title: "Leave no trace".to_owned(),
                additional_objective_titles: Vec::new(),
                transition_plan: Box::new(
                    crate::QuestTransitionPlanV1::default_for_objectives(1).unwrap(),
                ),
                collision_catalog: QuestCollisionArtifactRef {
                    generation: project.target.clone(),
                    catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
                    artifact,
                    source_seal: seal(0x74, 42),
                    basis_snapshot: seal(0x75, 10),
                },
            },
            script_module: crate::Revision3TypedRef::new(
                project.project_id,
                module_id,
                EntityKind::ScriptModule,
            ),
            transcript: Vec::new(),
        };
        let module =
            regenerate_revision3_quest_module(&draft, empty_collision_input(&draft)).unwrap();
        project.entities.insert(
            draft_id,
            Entity {
                id: draft_id,
                display_name: "Removable Quest".to_owned(),
                origin: new_origin("GORE_QUEST_REMOVABLE"),
                revision: DRAFT_REVISION,
                payload: EntityPayload::QuestDraft(draft),
            },
        );
        project.entities.insert(
            module_id,
            Entity {
                id: module_id,
                display_name: "Removable Quest module".to_owned(),
                origin: OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner,
                },
                revision: MODULE_REVISION,
                payload: EntityPayload::ScriptModule(module),
            },
        );
        Fixture {
            project,
            head: head(0xc1),
            draft_id,
            module_id,
            kind: Revision3StoryDraftRemovalKindV1::QuestDraft,
            preserved_entity_id,
        }
    }

    fn request(fixture: &Fixture) -> Revision3StoryDraftRemovalRequestV1 {
        Revision3StoryDraftRemovalRequestV1 {
            expected_head: fixture.head.clone(),
            expected_project_id: fixture.project.project_id,
            expected_revision: fixture.project.revision,
            expected_target: fixture.project.target.clone(),
            draft_id: fixture.draft_id,
            draft_kind: fixture.kind,
            expected_draft_revision: DRAFT_REVISION,
            script_module_id: fixture.module_id,
            expected_script_module_revision: MODULE_REVISION,
        }
    }

    fn evaluate(
        fixture: &Fixture,
        project: &ProjectRevision3,
        request: &Revision3StoryDraftRemovalRequestV1,
    ) -> Result<Revision3StoryDraftRemovalEvaluationV1, Revision3StoryDraftRemovalErrorV1> {
        apply_revision3_story_draft_removal_transaction_v1(
            &fixture.head,
            &project.to_canonical_json().unwrap(),
            &request.to_canonical_json().unwrap(),
        )
    }

    fn conflict_tag(conflict: &Revision3StoryDraftRemovalConflictV1) -> &'static str {
        match conflict {
            Revision3StoryDraftRemovalConflictV1::CurrentHeadMismatch => "head",
            Revision3StoryDraftRemovalConflictV1::ProjectIdentityMismatch { .. } => "project_id",
            Revision3StoryDraftRemovalConflictV1::ProjectRevisionConflict { .. } => {
                "project_revision"
            }
            Revision3StoryDraftRemovalConflictV1::ProjectTargetMismatch => "target",
            Revision3StoryDraftRemovalConflictV1::DraftKindMismatch { .. } => "draft_kind",
            Revision3StoryDraftRemovalConflictV1::DraftRevisionConflict { .. } => "draft_revision",
            Revision3StoryDraftRemovalConflictV1::DraftModuleBindingMismatch { .. } => {
                "module_binding"
            }
            Revision3StoryDraftRemovalConflictV1::ScriptModuleRevisionConflict { .. } => {
                "module_revision"
            }
            other => panic!("unexpected conflict in binding table: {other:?}"),
        }
    }

    #[test]
    fn npc_and_quest_success_remove_exact_pair_and_preserve_everything_else() {
        for fixture in [npc_fixture(), quest_fixture()] {
            let basis = fixture.project.clone();
            let request = request(&fixture);
            let Revision3StoryDraftRemovalEvaluationV1::Applied(outcome) =
                evaluate(&fixture, &basis, &request).unwrap()
            else {
                panic!("valid isolated closure was rejected");
            };

            assert_eq!(outcome.basis_head, fixture.head);
            assert_eq!(outcome.draft_id, fixture.draft_id);
            assert_eq!(outcome.draft_kind, fixture.kind);
            assert_eq!(outcome.draft_revision, DRAFT_REVISION);
            assert_eq!(outcome.script_module_id, fixture.module_id);
            assert_eq!(outcome.script_module_revision, MODULE_REVISION);
            assert_eq!(outcome.project.revision, basis.revision + 1);
            assert!(!outcome.project.entities.contains_key(&fixture.draft_id));
            assert!(!outcome.project.entities.contains_key(&fixture.module_id));
            assert_eq!(outcome.project.asset_store, basis.asset_store);
            assert_eq!(
                outcome.project.entities.get(&fixture.preserved_entity_id),
                basis.entities.get(&fixture.preserved_entity_id)
            );
            assert_eq!(
                ProjectRevision3::from_json(&outcome.canonical_project_json).unwrap(),
                outcome.project
            );
            assert_eq!(
                outcome.build_status,
                Revision3StoryDraftRemovalBuildStatusV1::Blocked
            );
            assert_eq!(
                outcome.runtime_status,
                Revision3StoryDraftRemovalRuntimeStatusV1::RuntimeUnqualified
            );
            assert_eq!(
                outcome.artifact_authority,
                Revision3StoryDraftRemovalArtifactAuthorityV1::NotGranted
            );
            assert_eq!(
                outcome.publication_status,
                Revision3StoryDraftRemovalPublicationStatusV1::NotSupported
            );
        }
    }

    #[test]
    fn quest_removal_drops_outgoing_transcript_edges_without_deleting_shared_content() {
        let mut fixture = quest_fixture();
        let localization_id = entity_id(0x81);
        let line_id = entity_id(0x82);
        fixture.project.entities.insert(
            localization_id,
            Entity {
                id: localization_id,
                display_name: "Shared Quest text".to_owned(),
                origin: new_origin("GORE_SHARED_QUEST_TEXT"),
                revision: 4,
                payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "GORE_SHARED_QUEST_TEXT_LOC".to_owned(),
                    texts: BTreeMap::new(),
                }),
            },
        );
        fixture.project.entities.insert(
            line_id,
            Entity {
                id: line_id,
                display_name: "Shared Quest line".to_owned(),
                origin: new_origin("GORE_SHARED_QUEST_LINE"),
                revision: 6,
                payload: EntityPayload::DialogLine(DialogLine {
                    localization: TypedRef::new(
                        fixture.project.project_id,
                        localization_id,
                        EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: Some("Asghan".to_owned()),
                    voice_slots: BTreeMap::new(),
                }),
            },
        );
        let EntityPayload::QuestDraft(quest) = &mut fixture
            .project
            .entities
            .get_mut(&fixture.draft_id)
            .unwrap()
            .payload
        else {
            panic!("fixture Quest kind")
        };
        quest.transcript.push(QuestTranscriptBindingV1 {
            line: TypedRef::new(fixture.project.project_id, line_id, EntityKind::DialogLine),
            objective_slot: None,
        });
        fixture.project.validate_closed_model().unwrap();
        let basis = fixture.project.clone();

        let Revision3StoryDraftRemovalEvaluationV1::Applied(outcome) =
            evaluate(&fixture, &basis, &request(&fixture)).unwrap()
        else {
            panic!("Quest transcript outgoing edge blocked pair removal")
        };

        assert!(!outcome.project.entities.contains_key(&fixture.draft_id));
        assert!(!outcome.project.entities.contains_key(&fixture.module_id));
        assert_eq!(
            outcome.project.entities.get(&line_id),
            basis.entities.get(&line_id)
        );
        assert_eq!(
            outcome.project.entities.get(&localization_id),
            basis.entities.get(&localization_id)
        );
    }

    #[test]
    fn npc_removal_drops_outgoing_greeting_edges_without_deleting_shared_content() {
        let mut fixture = npc_fixture();
        let localization_id = entity_id(0x83);
        let line_id = entity_id(0x84);
        fixture.project.entities.insert(
            localization_id,
            Entity {
                id: localization_id,
                display_name: "Shared NPC greeting text".to_owned(),
                origin: new_origin("GORE_SHARED_NPC_GREETING_TEXT"),
                revision: 4,
                payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "GORE_SHARED_NPC_GREETING_TEXT_LOC".to_owned(),
                    texts: BTreeMap::new(),
                }),
            },
        );
        fixture.project.entities.insert(
            line_id,
            Entity {
                id: line_id,
                display_name: "Shared NPC greeting line".to_owned(),
                origin: new_origin("GORE_SHARED_NPC_GREETING_LINE"),
                revision: 6,
                payload: EntityPayload::DialogLine(DialogLine {
                    localization: TypedRef::new(
                        fixture.project.project_id,
                        localization_id,
                        EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: Some("Asghan".to_owned()),
                    voice_slots: BTreeMap::new(),
                }),
            },
        );
        let EntityPayload::NpcDraft(npc) = &mut fixture
            .project
            .entities
            .get_mut(&fixture.draft_id)
            .unwrap()
            .payload
        else {
            panic!("fixture NPC kind")
        };
        npc.greetings.push(NpcGreetingBindingV1 {
            line: TypedRef::new(fixture.project.project_id, line_id, EntityKind::DialogLine),
        });
        fixture.project.validate_closed_model().unwrap();
        let basis = fixture.project.clone();

        let Revision3StoryDraftRemovalEvaluationV1::Applied(outcome) =
            evaluate(&fixture, &basis, &request(&fixture)).unwrap()
        else {
            panic!("NPC greeting outgoing edge blocked pair removal")
        };

        assert!(!outcome.project.entities.contains_key(&fixture.draft_id));
        assert!(!outcome.project.entities.contains_key(&fixture.module_id));
        assert_eq!(
            outcome.project.entities.get(&line_id),
            basis.entities.get(&line_id)
        );
        assert_eq!(
            outcome.project.entities.get(&localization_id),
            basis.entities.get(&localization_id)
        );
    }

    #[test]
    fn exact_binding_conflicts_are_table_driven() {
        let fixture = npc_fixture();
        let cases: Vec<(
            &'static str,
            Box<dyn Fn(&mut Revision3StoryDraftRemovalRequestV1)>,
        )> = vec![
            ("head", Box::new(|value| value.expected_head = head(0xff))),
            (
                "project_id",
                Box::new(|value| value.expected_project_id = project_id(0xff)),
            ),
            (
                "project_revision",
                Box::new(|value| value.expected_revision += 1),
            ),
            (
                "target",
                Box::new(|value| value.expected_target = target(0xff)),
            ),
            (
                "draft_kind",
                Box::new(|value| value.draft_kind = Revision3StoryDraftRemovalKindV1::QuestDraft),
            ),
            (
                "draft_revision",
                Box::new(|value| value.expected_draft_revision += 1),
            ),
            (
                "module_binding",
                Box::new(|value| value.script_module_id = entity_id(0xee)),
            ),
            (
                "module_revision",
                Box::new(|value| value.expected_script_module_revision += 1),
            ),
        ];

        for (expected_tag, mutate) in cases {
            let mut changed = request(&fixture);
            mutate(&mut changed);
            let Revision3StoryDraftRemovalEvaluationV1::Rejected(rejection) =
                evaluate(&fixture, &fixture.project, &changed).unwrap()
            else {
                panic!("changed {expected_tag} binding was accepted");
            };
            assert_eq!(conflict_tag(&rejection.conflict), expected_tag);
        }
    }

    #[test]
    fn local_draft_and_module_backlinks_block_with_source_and_role() {
        let fixture = npc_fixture();
        for (target_id, expected_draft) in [(fixture.draft_id, true), (fixture.module_id, false)] {
            let mut project = fixture.project.clone();
            let source_id = fixture.preserved_entity_id;
            project.entities.get_mut(&source_id).unwrap().origin = OriginRef::Generated {
                generator_id: "fixture.extra-reference".to_owned(),
                generator_version: 1,
                owner: crate::Revision3TypedRef::new(
                    project.project_id,
                    target_id,
                    if expected_draft {
                        EntityKind::NpcDraft
                    } else {
                        EntityKind::ScriptModule
                    },
                ),
            };
            let Revision3StoryDraftRemovalEvaluationV1::Rejected(rejection) =
                evaluate(&fixture, &project, &request(&fixture)).unwrap()
            else {
                panic!("additional local backlink was accepted");
            };
            match (expected_draft, rejection.conflict) {
                (
                    true,
                    Revision3StoryDraftRemovalConflictV1::DraftReferenced {
                        source_entity,
                        role,
                        ..
                    },
                ) => {
                    assert_eq!(source_entity, source_id);
                    assert_eq!(role, Revision3ContentReferenceRoleV1::OriginOwner);
                }
                (
                    false,
                    Revision3StoryDraftRemovalConflictV1::ModuleReferenced {
                        source_entity,
                        role,
                        ..
                    },
                ) => {
                    assert_eq!(source_entity, source_id);
                    assert_eq!(role, Revision3ContentReferenceRoleV1::OriginOwner);
                }
                (_, other) => panic!("wrong backlink conflict: {other:?}"),
            }
        }
    }

    #[test]
    fn second_owned_generated_object_blocks_but_foreign_same_id_does_not() {
        let fixture = quest_fixture();
        let mut second_owner = fixture.project.clone();
        let extra_id = entity_id(0x30);
        let owner = crate::Revision3TypedRef::new(
            second_owner.project_id,
            fixture.draft_id,
            EntityKind::QuestDraft,
        );
        let source = "class UExtraOwnedModule {}";
        second_owner.entities.insert(
            extra_id,
            Entity {
                id: extra_id,
                display_name: "Extra owned module".to_owned(),
                origin: OriginRef::Generated {
                    generator_id: "fixture.extra-generator".to_owned(),
                    generator_version: 1,
                    owner: owner.clone(),
                },
                revision: 0,
                payload: EntityPayload::ScriptModule(ScriptModule {
                    generator_id: "fixture.extra-generator".to_owned(),
                    generator_version: 1,
                    owner,
                    module_namespace: "PROJECT.QUESTS.EXTRA".to_owned(),
                    module_relative_path: "Project/Quests/Extra.as".to_owned(),
                    source: source.to_owned(),
                    source_sha256: Sha256Digest::from_bytes(
                        Sha256::digest(source.as_bytes()).into(),
                    ),
                    input_fingerprint: digest(0x31),
                    status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
                }),
            },
        );
        assert!(matches!(
            second_owner.to_canonical_json().unwrap_err(),
            crate::ProjectRevision3JsonError::InvalidModel(
                crate::ProjectRevision3ValidationError::InvalidScriptModuleOwnerClosure {
                    module,
                }
            ) if module == extra_id
        ));

        let mut foreign = fixture.project.clone();
        foreign
            .entities
            .get_mut(&fixture.preserved_entity_id)
            .unwrap()
            .origin = OriginRef::Generated {
            generator_id: "fixture.foreign-reference".to_owned(),
            generator_version: 1,
            owner: crate::Revision3TypedRef::new(
                project_id(0xfe),
                fixture.draft_id,
                EntityKind::QuestDraft,
            ),
        };
        assert!(matches!(
            evaluate(&fixture, &foreign, &request(&fixture)).unwrap(),
            Revision3StoryDraftRemovalEvaluationV1::Applied(_)
        ));
    }

    #[test]
    fn kind_mismatch_reference_and_deterministic_module_drift_fail_closed() {
        let fixture = quest_fixture();
        let mut kind_mismatch = fixture.project.clone();
        let source_id = fixture.preserved_entity_id;
        kind_mismatch.entities.get_mut(&source_id).unwrap().origin = OriginRef::Generated {
            generator_id: "fixture.kind-mismatch".to_owned(),
            generator_version: 1,
            owner: crate::Revision3TypedRef::new(
                kind_mismatch.project_id,
                fixture.draft_id,
                EntityKind::ScriptModule,
            ),
        };
        let Revision3StoryDraftRemovalEvaluationV1::Rejected(rejection) =
            evaluate(&fixture, &kind_mismatch, &request(&fixture)).unwrap()
        else {
            panic!("kind-mismatched local reference was accepted");
        };
        assert!(matches!(
            rejection.conflict,
            Revision3StoryDraftRemovalConflictV1::DraftReferenced {
                source_entity,
                role: Revision3ContentReferenceRoleV1::OriginOwner,
                ..
            } if source_entity == source_id
        ));

        let mut drifted = fixture.project.clone();
        let EntityPayload::ScriptModule(module) = &mut drifted
            .entities
            .get_mut(&fixture.module_id)
            .unwrap()
            .payload
        else {
            unreachable!()
        };
        module.input_fingerprint = digest(0xfd);
        let Revision3StoryDraftRemovalEvaluationV1::Rejected(rejection) =
            evaluate(&fixture, &drifted, &request(&fixture)).unwrap()
        else {
            panic!("deterministically drifted module was accepted");
        };
        assert!(matches!(
            rejection.conflict,
            Revision3StoryDraftRemovalConflictV1::PayloadOriginGeneratorMismatch { .. }
        ));
    }

    #[test]
    fn request_is_exact_canonical_duplicate_free_and_bounded() {
        let fixture = npc_fixture();
        let request = request(&fixture);
        let canonical = request.to_canonical_json().unwrap();
        assert_eq!(
            Revision3StoryDraftRemovalRequestV1::from_json(&canonical).unwrap(),
            request
        );
        assert!(matches!(
            Revision3StoryDraftRemovalRequestV1::from_json(&(canonical.clone() + "\n")),
            Err(Revision3StoryDraftRemovalRequestJsonErrorV1::NonCanonicalJson)
        ));
        let duplicate = canonical.replacen(
            &format!("\"expected_revision\":{},", fixture.project.revision),
            &format!(
                "\"expected_revision\":{},\"expected_revision\":{},",
                fixture.project.revision, fixture.project.revision
            ),
            1,
        );
        assert!(matches!(
            Revision3StoryDraftRemovalRequestV1::from_json(&duplicate),
            Err(Revision3StoryDraftRemovalRequestJsonErrorV1::InvalidJson(_))
        ));
        assert!(matches!(
            Revision3StoryDraftRemovalRequestV1::from_json(
                &"x".repeat(MAX_REVISION3_STORY_DRAFT_REMOVAL_REQUEST_JSON_BYTES_V1 + 1)
            ),
            Err(Revision3StoryDraftRemovalRequestJsonErrorV1::InputTooLarge { .. })
        ));
    }

    #[test]
    fn malformed_primary_backlink_is_rejected_as_invalid_project() {
        let fixture = quest_fixture();
        let mut invalid = fixture.project.clone();
        let EntityPayload::ScriptModule(module) = &mut invalid
            .entities
            .get_mut(&fixture.module_id)
            .unwrap()
            .payload
        else {
            unreachable!()
        };
        module.owner.id = entity_id(0xee);
        let invalid_json = serde_json::to_string(&invalid).unwrap();
        let error = apply_revision3_story_draft_removal_transaction_v1(
            &fixture.head,
            &invalid_json,
            &request(&fixture).to_canonical_json().unwrap(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            Revision3StoryDraftRemovalErrorV1::InvalidProject(_)
        ));
    }
}
