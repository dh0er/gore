//! Exact-basis-bound editing of one existing revision-3 NPC Draft profile.
//!
//! The transaction changes only the friendly entity display name and/or the complete three-parent
//! archetype provenance resolved from one fresh Story/NPC catalog context. Technical NPC identity
//! is immutable. The owned ScriptModule is mutated and revisioned only when the resolved parent
//! triple changes; a name-only edit preserves the complete module entity byte-for-byte.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision3::{
    EntityKind, EntityPayload, NpcDraft, NpcParentClassInput, OriginRef, ProjectRevision3,
    ScriptModule, ScriptModuleStatus, TypedRef,
};
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    ContentSeal, EntityId, GameGenerationAnchor, ProjectId, ProjectRevision3JsonError,
    Revision3NpcCatalogSelectionV1, Sha256Digest, WorkingHead, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION, MAX_REVISION3_NPC_CATALOG_ID_BYTES_V1,
    MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1,
};

/// Maximum exact canonical NPC profile-edit request size.
pub const MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1: usize = 32 * 1024;

/// Exact head/project/entity/catalog-CAS binding for one existing NPC profile edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3NpcProfileEditRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub expected_story_catalog_seal: ContentSeal,
    pub expected_npc_catalog_seal: ContentSeal,
    pub npc_id: EntityId,
    pub expected_npc_revision: u64,
    pub script_module_id: EntityId,
    pub expected_script_module_revision: u64,
    pub expected_parent_catalog_id: String,
    pub display_name: String,
    pub parent_catalog_id: String,
}

impl Revision3NpcProfileEditRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3NpcProfileEditRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1 {
            return Err(Revision3NpcProfileEditRequestJsonErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3NpcProfileEditRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3NpcProfileEditRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3NpcProfileEditRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3NpcProfileEditRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3NpcProfileEditRequestJsonErrorV1::InputTooLarge {
                actual,
                limit: MAX_REVISION3_NPC_PROFILE_EDIT_REQUEST_JSON_BYTES_V1,
            });
        }
        serialized.map_err(Revision3NpcProfileEditRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3NpcProfileEditRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3NpcProfileEditRequestJsonErrorV1 {
    #[error("revision-3 NPC profile-edit request exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 NPC profile-edit request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 NPC profile-edit request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 NPC profile-edit request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 NPC profile-edit request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Fresh native catalog resolutions consumed by the transaction.
///
/// This value is deliberately not serializable and grants no retained catalog authority. The FFI
/// adapter must construct both selections from one fresh, exact Story/NPC catalog pair.
#[derive(Debug, PartialEq, Eq)]
pub struct Revision3NpcProfileCatalogContextV1 {
    pub current_selection: Revision3NpcCatalogSelectionV1,
    pub desired_selection: Revision3NpcCatalogSelectionV1,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3NpcProfileEditConflictV1 {
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
    #[error("NPC entity ID must not be zero")]
    ZeroNpcId,
    #[error("ScriptModule entity ID must not be zero")]
    ZeroScriptModuleId,
    #[error("NPC and ScriptModule IDs must be distinct")]
    IdentityCollision,
    #[error("NPC entity {npc} is missing or has the wrong kind")]
    InvalidNpcEntity { npc: EntityId },
    #[error("expected NPC entity revision {expected}, but exact basis is {actual}")]
    NpcRevisionConflict { expected: u64, actual: u64 },
    #[error("NPC entity {npc} revision cannot be incremented")]
    NpcRevisionOverflow { npc: EntityId },
    #[error("display name is empty, contains controls, or exceeds its byte limit")]
    InvalidDisplayName,
    #[error("NPC {npc} does not bind the requested exact local ScriptModule {module}")]
    NpcModuleBindingMismatch { npc: EntityId, module: EntityId },
    #[error("ScriptModule entity {module} is missing or has the wrong kind")]
    InvalidScriptModuleEntity { module: EntityId },
    #[error("expected ScriptModule revision {expected}, but exact basis is {actual}")]
    ScriptModuleRevisionConflict { expected: u64, actual: u64 },
    #[error("NPC {npc} and ScriptModule {module} have an invalid closed ownership/generator state: {reason}")]
    InvalidNpcClosure {
        npc: EntityId,
        module: EntityId,
        reason: String,
    },
    #[error("NPC {npc} owned ScriptModule {module} differs from deterministic regeneration")]
    OwnedModuleDrift { npc: EntityId, module: EntityId },
    #[error("Story catalog seal differs from the exact reviewed catalog context")]
    StoryCatalogSealMismatch,
    #[error("NPC catalog seal differs from the exact reviewed catalog context")]
    NpcCatalogSealMismatch,
    #[error("catalog context generation differs from the exact project target")]
    CatalogGenerationMismatch,
    #[error("catalog context selections or parent provenance are invalid")]
    InvalidCatalogContext,
    #[error("current catalog selection differs from the requested catalog ID")]
    CurrentCatalogSelectionMismatch,
    #[error("desired catalog selection differs from the requested catalog ID")]
    DesiredCatalogSelectionMismatch,
    #[error("stored NPC parent provenance does not match the exact current catalog selection")]
    StoredArchetypeMismatch,
    #[error("NPC profile edit changes neither display name nor resolved parent provenance")]
    NoChanges,
    #[error("ScriptModule entity {module} revision cannot be incremented")]
    ScriptModuleRevisionOverflow { module: EntityId },
    #[error("NPC archetype edit unexpectedly changed a preserved technical module identity")]
    TechnicalIdentityChanged,
    #[error("NPC archetype edit produced an invalid deterministic module: {reason}")]
    InvalidDesiredArchetype { reason: String },
    #[error("NPC profile candidate exceeds the {limit}-byte project limit: {actual} bytes")]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("NPC profile candidate is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3NpcProfileEditRejectionV1 {
    pub conflict: Revision3NpcProfileEditConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcProfileEditBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcProfileEditRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcProfileEditCatalogAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcProfileEditCollisionAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcProfileEditPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3NpcProfileEditOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub npc_id: EntityId,
    pub npc_revision: u64,
    pub script_module_id: EntityId,
    pub script_module_revision: u64,
    pub name_changed: bool,
    pub archetype_changed: bool,
    pub module_regenerated: bool,
    pub build_status: Revision3NpcProfileEditBuildStatusV1,
    pub runtime_status: Revision3NpcProfileEditRuntimeStatusV1,
    pub catalog_authority: Revision3NpcProfileEditCatalogAuthorityV1,
    pub collision_authority: Revision3NpcProfileEditCollisionAuthorityV1,
    pub publication_status: Revision3NpcProfileEditPublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3NpcProfileEditEvaluationV1 {
    Applied(Box<Revision3NpcProfileEditOutcomeV1>),
    Rejected(Revision3NpcProfileEditRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3NpcProfileEditErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 NPC profile-edit request: {0}")]
    InvalidRequest(#[source] Revision3NpcProfileEditRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 NPC profile candidate reopen changed the project")]
    CanonicalReopenMismatch,
    #[error("NPC profile edit changed a preserved project value")]
    CandidatePreservationMismatch,
}

/// Atomically edit one existing NPC's friendly name and catalog-resolved parent chain.
///
/// No filesystem, compiler, game, deployment, spawn, runtime, save, or publication operation is
/// reachable here. The consumed catalog context is caller-verified input and is not returned as
/// reusable authority.
pub fn apply_revision3_npc_profile_edit_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
    catalog_context: Revision3NpcProfileCatalogContextV1,
) -> Result<Revision3NpcProfileEditEvaluationV1, Revision3NpcProfileEditErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3NpcProfileEditErrorV1::InvalidProject)?;
    let request = Revision3NpcProfileEditRequestV1::from_json(canonical_request_json)
        .map_err(Revision3NpcProfileEditErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3NpcProfileEditEvaluationV1::Rejected(
                Revision3NpcProfileEditRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3NpcProfileEditConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(Revision3NpcProfileEditConflictV1::ProjectIdentityMismatch {
            expected: request.expected_project_id,
            actual: project.project_id,
        });
    }
    if request.expected_revision != project.revision {
        reject!(Revision3NpcProfileEditConflictV1::ProjectRevisionConflict {
            expected: request.expected_revision,
            actual: project.revision,
        });
    }
    if request.expected_target != project.target {
        reject!(Revision3NpcProfileEditConflictV1::ProjectTargetMismatch);
    }
    let Some(next_project_revision) = project.revision.checked_add(1) else {
        reject!(Revision3NpcProfileEditConflictV1::ProjectRevisionOverflow);
    };
    if is_zero_entity_id(request.npc_id) {
        reject!(Revision3NpcProfileEditConflictV1::ZeroNpcId);
    }
    if is_zero_entity_id(request.script_module_id) {
        reject!(Revision3NpcProfileEditConflictV1::ZeroScriptModuleId);
    }
    if request.npc_id == request.script_module_id {
        reject!(Revision3NpcProfileEditConflictV1::IdentityCollision);
    }
    if !valid_display_name(&request.display_name) {
        reject!(Revision3NpcProfileEditConflictV1::InvalidDisplayName);
    }
    if !valid_catalog_id(&request.expected_parent_catalog_id)
        || !valid_catalog_id(&request.parent_catalog_id)
    {
        reject!(Revision3NpcProfileEditConflictV1::InvalidCatalogContext);
    }

    if let Err(conflict) = validate_catalog_context(&project, &request, &catalog_context) {
        reject!(conflict);
    }

    let (existing_npc, next_npc_revision, existing_module, module_entity) = {
        let Some(npc_entity) = project.entities.get(&request.npc_id) else {
            reject!(Revision3NpcProfileEditConflictV1::InvalidNpcEntity {
                npc: request.npc_id,
            });
        };
        let EntityPayload::NpcDraft(npc) = &npc_entity.payload else {
            reject!(Revision3NpcProfileEditConflictV1::InvalidNpcEntity {
                npc: request.npc_id,
            });
        };
        if npc_entity.revision != request.expected_npc_revision {
            reject!(Revision3NpcProfileEditConflictV1::NpcRevisionConflict {
                expected: request.expected_npc_revision,
                actual: npc_entity.revision,
            });
        }
        let Some(next_npc_revision) = npc_entity.revision.checked_add(1) else {
            reject!(Revision3NpcProfileEditConflictV1::NpcRevisionOverflow {
                npc: request.npc_id,
            });
        };
        if npc.script_module.project_id != project.project_id
            || npc.script_module.id != request.script_module_id
            || npc.script_module.expected_kind != EntityKind::ScriptModule
        {
            reject!(
                Revision3NpcProfileEditConflictV1::NpcModuleBindingMismatch {
                    npc: request.npc_id,
                    module: request.script_module_id,
                }
            );
        }
        let Some(module_entity) = project.entities.get(&request.script_module_id) else {
            reject!(
                Revision3NpcProfileEditConflictV1::InvalidScriptModuleEntity {
                    module: request.script_module_id,
                }
            );
        };
        let EntityPayload::ScriptModule(module) = &module_entity.payload else {
            reject!(
                Revision3NpcProfileEditConflictV1::InvalidScriptModuleEntity {
                    module: request.script_module_id,
                }
            );
        };
        if module_entity.revision != request.expected_script_module_revision {
            reject!(
                Revision3NpcProfileEditConflictV1::ScriptModuleRevisionConflict {
                    expected: request.expected_script_module_revision,
                    actual: module_entity.revision,
                }
            );
        }
        (
            npc.clone(),
            next_npc_revision,
            module.clone(),
            module_entity.clone(),
        )
    };

    if let Err(reason) = validate_exact_npc_module_closure(
        &project,
        request.npc_id,
        request.script_module_id,
        &existing_npc,
        &existing_module,
        &module_entity,
    ) {
        reject!(Revision3NpcProfileEditConflictV1::InvalidNpcClosure {
            npc: request.npc_id,
            module: request.script_module_id,
            reason,
        });
    }
    let owner = TypedRef::new(project.project_id, request.npc_id, EntityKind::NpcDraft);
    let regenerated_existing = match existing_npc.regenerate_script_module(owner.clone()) {
        Ok(module) => module,
        Err(error) => {
            reject!(Revision3NpcProfileEditConflictV1::InvalidNpcClosure {
                npc: request.npc_id,
                module: request.script_module_id,
                reason: error.to_string(),
            });
        }
    };
    if regenerated_existing != existing_module {
        reject!(Revision3NpcProfileEditConflictV1::OwnedModuleDrift {
            npc: request.npc_id,
            module: request.script_module_id,
        });
    }

    if !same_parent_triple(
        &existing_npc,
        &catalog_context
            .current_selection
            .parent_character_definition,
        &catalog_context.current_selection.parent_ai_agent_config,
        &catalog_context.current_selection.parent_spawn_definition,
    ) {
        reject!(Revision3NpcProfileEditConflictV1::StoredArchetypeMismatch);
    }

    let name_changed = project.entities[&request.npc_id].display_name != request.display_name;
    let archetype_changed = !same_parent_triple(
        &existing_npc,
        &catalog_context
            .desired_selection
            .parent_character_definition,
        &catalog_context.desired_selection.parent_ai_agent_config,
        &catalog_context.desired_selection.parent_spawn_definition,
    );
    if !name_changed && !archetype_changed {
        reject!(Revision3NpcProfileEditConflictV1::NoChanges);
    }

    let mut edited_npc = existing_npc.clone();
    let edited_module = if archetype_changed {
        edited_npc.input.parent_character_definition = catalog_context
            .desired_selection
            .parent_character_definition
            .clone();
        edited_npc.input.parent_ai_agent_config = catalog_context
            .desired_selection
            .parent_ai_agent_config
            .clone();
        edited_npc.input.parent_spawn_definition = catalog_context
            .desired_selection
            .parent_spawn_definition
            .clone();
        let regenerated = match edited_npc.regenerate_script_module(owner) {
            Ok(module) => module,
            Err(error) => {
                reject!(Revision3NpcProfileEditConflictV1::InvalidDesiredArchetype {
                    reason: error.to_string(),
                });
            }
        };
        if !same_technical_module_identity(&existing_module, &regenerated) {
            reject!(Revision3NpcProfileEditConflictV1::TechnicalIdentityChanged);
        }
        Some(regenerated)
    } else {
        None
    };

    let next_module_revision = if archetype_changed {
        let Some(revision) = module_entity.revision.checked_add(1) else {
            reject!(
                Revision3NpcProfileEditConflictV1::ScriptModuleRevisionOverflow {
                    module: request.script_module_id,
                }
            );
        };
        revision
    } else {
        module_entity.revision
    };

    let basis_project = project.clone();
    let npc_entity = project
        .entities
        .get_mut(&request.npc_id)
        .expect("NPC was resolved above");
    npc_entity.display_name = request.display_name.clone();
    npc_entity.revision = next_npc_revision;
    if archetype_changed {
        npc_entity.payload = EntityPayload::NpcDraft(edited_npc.clone());
    }
    if let Some(edited_module) = &edited_module {
        let module_entity = project
            .entities
            .get_mut(&request.script_module_id)
            .expect("ScriptModule was resolved above");
        module_entity.revision = next_module_revision;
        module_entity.payload = EntityPayload::ScriptModule(edited_module.clone());
    }
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            reject!(Revision3NpcProfileEditConflictV1::CandidateTooLarge { actual, limit });
        }
        Err(error) => {
            reject!(Revision3NpcProfileEditConflictV1::CandidateNotPersistable {
                reason: error.to_string(),
            });
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3NpcProfileEditErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3NpcProfileEditErrorV1::CanonicalReopenMismatch);
    }
    if !preserves_exact_basis(
        &basis_project,
        &reopened,
        &request,
        &edited_npc,
        edited_module.as_ref(),
        name_changed,
        archetype_changed,
    ) {
        return Err(Revision3NpcProfileEditErrorV1::CandidatePreservationMismatch);
    }

    Ok(Revision3NpcProfileEditEvaluationV1::Applied(Box::new(
        Revision3NpcProfileEditOutcomeV1 {
            project: reopened,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            npc_id: request.npc_id,
            npc_revision: next_npc_revision,
            script_module_id: request.script_module_id,
            script_module_revision: next_module_revision,
            name_changed,
            archetype_changed,
            module_regenerated: archetype_changed,
            build_status: Revision3NpcProfileEditBuildStatusV1::Blocked,
            runtime_status: Revision3NpcProfileEditRuntimeStatusV1::RuntimeUnqualified,
            catalog_authority: Revision3NpcProfileEditCatalogAuthorityV1::NotGranted,
            collision_authority: Revision3NpcProfileEditCollisionAuthorityV1::NotGranted,
            publication_status: Revision3NpcProfileEditPublicationStatusV1::NotSupported,
        },
    )))
}

fn validate_catalog_context(
    project: &ProjectRevision3,
    request: &Revision3NpcProfileEditRequestV1,
    context: &Revision3NpcProfileCatalogContextV1,
) -> Result<(), Revision3NpcProfileEditConflictV1> {
    if context.current_selection.catalog_id != request.expected_parent_catalog_id {
        return Err(Revision3NpcProfileEditConflictV1::CurrentCatalogSelectionMismatch);
    }
    if context.desired_selection.catalog_id != request.parent_catalog_id {
        return Err(Revision3NpcProfileEditConflictV1::DesiredCatalogSelectionMismatch);
    }
    if context.current_selection.catalog_id == context.desired_selection.catalog_id
        && context.current_selection != context.desired_selection
    {
        // One immutable catalog pair cannot resolve one canonical ID to two different records.
        return Err(Revision3NpcProfileEditConflictV1::InvalidCatalogContext);
    }
    for selection in [&context.current_selection, &context.desired_selection] {
        if selection.generation != project.target {
            return Err(Revision3NpcProfileEditConflictV1::CatalogGenerationMismatch);
        }
        if selection.story_catalog_seal != request.expected_story_catalog_seal {
            return Err(Revision3NpcProfileEditConflictV1::StoryCatalogSealMismatch);
        }
        if selection.npc_catalog_seal != request.expected_npc_catalog_seal {
            return Err(Revision3NpcProfileEditConflictV1::NpcCatalogSealMismatch);
        }
        if !valid_catalog_selection(selection) {
            return Err(Revision3NpcProfileEditConflictV1::InvalidCatalogContext);
        }
    }
    Ok(())
}

fn validate_exact_npc_module_closure(
    project: &ProjectRevision3,
    npc_id: EntityId,
    module_id: EntityId,
    npc: &NpcDraft,
    module: &ScriptModule,
    module_entity: &crate::Revision3Entity,
) -> Result<(), String> {
    if npc.generator_id != LOGICAL_NPC_CLONE_GENERATOR_ID
        || npc.generator_version != LOGICAL_NPC_CLONE_GENERATOR_VERSION
        || npc.input.target != project.target
    {
        return Err("NPC generator or target differs from the closed contract".to_owned());
    }
    if !matches!(
        &project.entities[&npc_id].origin,
        OriginRef::New { authored_runtime_id }
            if authored_runtime_id == &npc.input.unique_name
    ) {
        return Err("NPC origin differs from its stable authored runtime identity".to_owned());
    }
    if module.owner.project_id != project.project_id
        || module.owner.id != npc_id
        || module.owner.expected_kind != EntityKind::NpcDraft
        || module.generator_id != LOGICAL_NPC_CLONE_GENERATOR_ID
        || module.generator_version != LOGICAL_NPC_CLONE_GENERATOR_VERSION
        || module.status != ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
    {
        return Err("ScriptModule payload owner, generator, or status is not exact".to_owned());
    }
    if !matches!(
        &module_entity.origin,
        OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } if generator_id == LOGICAL_NPC_CLONE_GENERATOR_ID
            && *generator_version == LOGICAL_NPC_CLONE_GENERATOR_VERSION
            && owner == &module.owner
            && owner.id == npc_id
    ) {
        return Err("ScriptModule origin does not mirror its payload owner/generator".to_owned());
    }
    if module_entity.id != module_id {
        return Err("ScriptModule embedded identity differs from its entity key".to_owned());
    }
    Ok(())
}

fn preserves_exact_basis(
    basis: &ProjectRevision3,
    candidate: &ProjectRevision3,
    request: &Revision3NpcProfileEditRequestV1,
    expected_npc: &NpcDraft,
    expected_module: Option<&ScriptModule>,
    name_changed: bool,
    archetype_changed: bool,
) -> bool {
    if !name_changed && !archetype_changed {
        return false;
    }
    if candidate.revision != basis.revision.saturating_add(1)
        || candidate.project_id != basis.project_id
        || candidate.meta != basis.meta
        || candidate.target != basis.target
        || candidate.authoring_locales != basis.authoring_locales
        || candidate.asset_store != basis.asset_store
        || candidate.entities.len() != basis.entities.len()
    {
        return false;
    }
    for (id, entity) in &basis.entities {
        if *id != request.npc_id && *id != request.script_module_id {
            if candidate.entities.get(id) != Some(entity) {
                return false;
            }
        }
    }
    let Some(basis_npc) = basis.entities.get(&request.npc_id) else {
        return false;
    };
    let Some(candidate_npc) = candidate.entities.get(&request.npc_id) else {
        return false;
    };
    if candidate_npc.id != basis_npc.id
        || candidate_npc.origin != basis_npc.origin
        || candidate_npc.revision != basis_npc.revision.saturating_add(1)
        || candidate_npc.display_name != request.display_name
        || !matches!(&candidate_npc.payload, EntityPayload::NpcDraft(npc) if npc == expected_npc)
    {
        return false;
    }
    if !archetype_changed && candidate_npc.payload != basis_npc.payload {
        return false;
    }
    let Some(basis_module) = basis.entities.get(&request.script_module_id) else {
        return false;
    };
    let Some(candidate_module) = candidate.entities.get(&request.script_module_id) else {
        return false;
    };
    if !archetype_changed {
        return candidate_module == basis_module && expected_module.is_none();
    }
    let Some(expected_module) = expected_module else {
        return false;
    };
    candidate_module.id == basis_module.id
        && candidate_module.display_name == basis_module.display_name
        && candidate_module.origin == basis_module.origin
        && candidate_module.revision == basis_module.revision.saturating_add(1)
        && matches!(&candidate_module.payload, EntityPayload::ScriptModule(module) if module == expected_module)
}

fn same_parent_triple(
    npc: &NpcDraft,
    character: &NpcParentClassInput,
    ai: &NpcParentClassInput,
    spawn: &NpcParentClassInput,
) -> bool {
    npc.input.parent_character_definition == *character
        && npc.input.parent_ai_agent_config == *ai
        && npc.input.parent_spawn_definition == *spawn
}

fn same_technical_module_identity(before: &ScriptModule, after: &ScriptModule) -> bool {
    before.generator_id == after.generator_id
        && before.generator_version == after.generator_version
        && before.owner == after.owner
        && before.module_namespace == after.module_namespace
        && before.module_relative_path == after.module_relative_path
        && before.status == after.status
}

fn valid_catalog_selection(value: &Revision3NpcCatalogSelectionV1) -> bool {
    valid_catalog_id(&value.catalog_id)
        && valid_seal(&value.story_catalog_seal)
        && valid_seal(&value.npc_catalog_seal)
        && [
            &value.parent_character_definition,
            &value.parent_ai_agent_config,
            &value.parent_spawn_definition,
        ]
        .into_iter()
        .all(|parent| parent.generation == value.generation && valid_seal(&parent.source_seal))
}

fn valid_display_name(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.len() <= MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1
        && !value.chars().any(char::is_control)
}

fn valid_catalog_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_REVISION3_NPC_CATALOG_ID_BYTES_V1
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b':' | b'-' | b'_' | b'.')
        })
        && value.split(':').count() >= 3
        && value.split(':').all(|part| !part.is_empty())
}

fn valid_seal(value: &ContentSeal) -> bool {
    value.byte_len != 0 && !is_zero_digest(value.sha256)
}

fn is_zero_digest(value: Sha256Digest) -> bool {
    value.as_bytes().iter().all(|byte| *byte == 0)
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
                "revision-3 NPC profile-edit request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
