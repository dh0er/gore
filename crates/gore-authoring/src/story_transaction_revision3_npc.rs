//! Atomic, filesystem-free insertion of one schema-revision-3 NPC Draft/module pair.
//!
//! The serializable request carries only author intent and exact project/head CAS values. Parent
//! class provenance and the base-game plus exact-current collision inventory arrive through
//! separate, consumed native-context values. Those values are deliberately not persisted as
//! authority: every later inspection or mutation must rebuild a fresh context.

use std::collections::BTreeSet;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::model_revision2::GeneratedStoryIdentity;
use crate::model_revision3::{
    Entity, EntityKind, EntityPayload, NpcDraft, NpcDraftInput, NpcParentClassInput, OriginRef,
    TypedRef,
};
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    ContentSeal, EntityId, GameGenerationAnchor, ProjectId, ProjectRevision3,
    ProjectRevision3JsonError, Sha256Digest, StoryRegenerationError, WorkingHead,
    LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION, MAX_REVISION3_ENTITIES,
    REVISION3_QUEST_GENERATOR_ID,
};

pub const MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1: usize = 32 * 1024;
pub const MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1: usize = 256;
pub const MAX_REVISION3_NPC_CATALOG_ID_BYTES_V1: usize = 256;
pub const REVISION3_NPC_EXACT_COLLISION_LAYER_V1: &str =
    "base-game-plus-exact-revision3-project.story-collisions.v2";

/// User-authored NPC intent. Every parent value is resolved from a fresh native catalog context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3NpcDraftIntentV1 {
    pub module_namespace: String,
    pub unique_name: String,
    pub parent_catalog_id: String,
}

/// Exact project/head-bound request for one revision-3 NPC Draft and owned ScriptModule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3NpcDraftInsertRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub npc_id: EntityId,
    pub script_module_id: EntityId,
    pub display_name: String,
    pub intent: Revision3NpcDraftIntentV1,
}

impl Revision3NpcDraftInsertRequestV1 {
    pub fn from_json(json: &str) -> Result<Self, Revision3NpcDraftInsertRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1 {
            return Err(Revision3NpcDraftInsertRequestJsonErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3NpcDraftInsertRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3NpcDraftInsertRequestJsonErrorV1::InvalidJson)?;
        let canonical = request.to_canonical_json()?;
        if canonical.as_bytes() != json.as_bytes() {
            return Err(Revision3NpcDraftInsertRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3NpcDraftInsertRequestJsonErrorV1> {
        let mut writer = BoundedRequestWriter::new(MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3NpcDraftInsertRequestJsonErrorV1::InputTooLarge {
                actual,
                limit: MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1,
            });
        }
        serialized.map_err(Revision3NpcDraftInsertRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3NpcDraftInsertRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3NpcDraftInsertRequestJsonErrorV1 {
    #[error("revision-3 NPC request exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 NPC request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 NPC request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 NPC request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 NPC request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Resolved parent selection rebuilt by a native catalog boundary.
///
/// This is a consumed transaction input, not serializable authority. Public construction exists
/// for native adapters and tests; only a route that freshly rebuilds and revalidates the closed
/// Story and NPC archetype catalogs may claim native catalog provenance.
#[derive(Debug, PartialEq, Eq)]
pub struct Revision3NpcCatalogSelectionV1 {
    pub generation: GameGenerationAnchor,
    pub catalog_id: String,
    pub story_catalog_seal: ContentSeal,
    pub npc_catalog_seal: ContentSeal,
    pub parent_character_definition: NpcParentClassInput,
    pub parent_ai_agent_config: NpcParentClassInput,
    pub parent_spawn_definition: NpcParentClassInput,
}

/// Complete caller-verified base-game plus exact-current script collision layer and the runtime
/// identities exposed by the exact pinned Story catalog, consumed by the transaction.
#[derive(Debug, PartialEq, Eq)]
pub struct Revision3NpcCollisionInventoryV1 {
    pub basis_head: WorkingHead,
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub current_project: ContentSeal,
    pub generation: GameGenerationAnchor,
    pub story_catalog_seal: ContentSeal,
    pub source_seal: ContentSeal,
    pub catalog_layer: String,
    pub catalog_runtime_ids: BTreeSet<String>,
    pub modules: BTreeSet<String>,
    pub relative_paths: BTreeSet<String>,
    pub symbols: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcEntityRoleV1 {
    NpcDraft,
    ScriptModule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcStoryIdentityKindV1 {
    AuthoredRuntimeId,
    ModuleNamespace,
    ModuleRelativePath,
    GeneratedSymbol,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3NpcDraftInsertConflictV1 {
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
    ZeroEntityId { role: Revision3NpcEntityRoleV1 },
    #[error("NPC Draft and ScriptModule IDs must differ")]
    SharedEntityId,
    #[error("{role:?} entity ID {entity} already exists")]
    EntityIdCollision {
        role: Revision3NpcEntityRoleV1,
        entity: EntityId,
    },
    #[error("display name is empty, contains controls, or exceeds its byte limit")]
    InvalidDisplayName,
    #[error("revision-3 project cannot hold two additional entities")]
    EntityCapacityExceeded,
    #[error("NPC parent catalog ID is not a bounded canonical ID")]
    InvalidCatalogId,
    #[error("native NPC catalog selection differs from the requested catalog ID")]
    CatalogSelectionMismatch,
    #[error("native NPC catalog selection generation differs from the exact project target")]
    CatalogGenerationMismatch,
    #[error("native NPC catalog selection seals or parent provenance are invalid")]
    InvalidCatalogSelection,
    #[error("collision inventory generation differs from the exact project target")]
    CollisionGenerationMismatch,
    #[error("exact-current collision inventory is bound to a different project/head snapshot")]
    CollisionBasisMismatch,
    #[error("collision inventory is not bound to the selected Story catalog")]
    CollisionStoryCatalogMismatch,
    #[error("collision inventory layer is not the closed base-game plus exact-current layer")]
    CollisionLayerMismatch,
    #[error("collision inventory seal or entries are invalid or exceed their budget")]
    InvalidCollisionInventory,
    #[error("revision-3 basis has invalid existing Story state: {reason}")]
    InvalidBasisStoryState { reason: String },
    #[error("invalid NPC intent: {reason}")]
    InvalidNpcIntent { reason: String },
    #[error(
        "{kind:?} {value:?} collides with fresh native inputs or exact-project Story identity"
    )]
    StoryIdentityCollision {
        kind: Revision3NpcStoryIdentityKindV1,
        value: String,
        existing_entity: Option<EntityId>,
    },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3NpcDraftInsertRejectionV1 {
    pub conflict: Revision3NpcDraftInsertConflictV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcDraftBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcDraftRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcCatalogAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcCollisionAuthorityV1 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcSourceInspectionStatusV1 {
    FreshNativeContextRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3NpcDraftPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3NpcDraftInsertOutcomeV1 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub basis_head: WorkingHead,
    pub npc_id: EntityId,
    pub script_module_id: EntityId,
    pub build_status: Revision3NpcDraftBuildStatusV1,
    pub runtime_status: Revision3NpcDraftRuntimeStatusV1,
    pub catalog_authority: Revision3NpcCatalogAuthorityV1,
    pub collision_authority: Revision3NpcCollisionAuthorityV1,
    pub source_inspection: Revision3NpcSourceInspectionStatusV1,
    pub publication_status: Revision3NpcDraftPublicationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3NpcDraftInsertEvaluationV1 {
    Applied(Box<Revision3NpcDraftInsertOutcomeV1>),
    Rejected(Revision3NpcDraftInsertRejectionV1),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3NpcDraftInsertErrorV1 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 NPC request: {0}")]
    InvalidRequest(#[source] Revision3NpcDraftInsertRequestJsonErrorV1),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 NPC candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Atomically insert one offline NPC Draft and its deterministic ScriptModule.
///
/// No filesystem, compiler, game, deployment, spawn, runtime, save, or publication operation is
/// reachable here. Both context values are consumed, but are still only caller-verified inputs;
/// successful output explicitly grants no reusable catalog or collision authority.
pub fn apply_revision3_npc_draft_transaction_v1(
    exact_basis_head: &WorkingHead,
    canonical_project_json: &str,
    canonical_request_json: &str,
    selection: Revision3NpcCatalogSelectionV1,
    collision_inventory: Revision3NpcCollisionInventoryV1,
) -> Result<Revision3NpcDraftInsertEvaluationV1, Revision3NpcDraftInsertErrorV1> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3NpcDraftInsertErrorV1::InvalidProject)?;
    let request = Revision3NpcDraftInsertRequestV1::from_json(canonical_request_json)
        .map_err(Revision3NpcDraftInsertErrorV1::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3NpcDraftInsertEvaluationV1::Rejected(
                Revision3NpcDraftInsertRejectionV1 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if &request.expected_head != exact_basis_head {
        reject!(Revision3NpcDraftInsertConflictV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id {
        reject!(Revision3NpcDraftInsertConflictV1::ProjectIdentityMismatch {
            expected: request.expected_project_id,
            actual: project.project_id,
        });
    }
    if request.expected_revision != project.revision {
        reject!(Revision3NpcDraftInsertConflictV1::ProjectRevisionConflict {
            expected: request.expected_revision,
            actual: project.revision,
        });
    }
    if request.expected_target != project.target {
        reject!(Revision3NpcDraftInsertConflictV1::ProjectTargetMismatch);
    }
    let Some(next_revision) = project.revision.checked_add(1) else {
        reject!(Revision3NpcDraftInsertConflictV1::ProjectRevisionOverflow);
    };
    for (role, id) in [
        (Revision3NpcEntityRoleV1::NpcDraft, request.npc_id),
        (
            Revision3NpcEntityRoleV1::ScriptModule,
            request.script_module_id,
        ),
    ] {
        if is_zero_entity_id(id) {
            reject!(Revision3NpcDraftInsertConflictV1::ZeroEntityId { role });
        }
        if project.entities.contains_key(&id) {
            reject!(Revision3NpcDraftInsertConflictV1::EntityIdCollision { role, entity: id });
        }
    }
    if request.npc_id == request.script_module_id {
        reject!(Revision3NpcDraftInsertConflictV1::SharedEntityId);
    }
    if !valid_display_name(&request.display_name) {
        reject!(Revision3NpcDraftInsertConflictV1::InvalidDisplayName);
    }
    if project
        .entities
        .len()
        .checked_add(2)
        .is_none_or(|count| count > MAX_REVISION3_ENTITIES)
    {
        reject!(Revision3NpcDraftInsertConflictV1::EntityCapacityExceeded);
    }
    if !valid_catalog_id(&request.intent.parent_catalog_id) {
        reject!(Revision3NpcDraftInsertConflictV1::InvalidCatalogId);
    }
    if selection.catalog_id != request.intent.parent_catalog_id {
        reject!(Revision3NpcDraftInsertConflictV1::CatalogSelectionMismatch);
    }
    if selection.generation != project.target {
        reject!(Revision3NpcDraftInsertConflictV1::CatalogGenerationMismatch);
    }
    if !valid_selection(&selection) {
        reject!(Revision3NpcDraftInsertConflictV1::InvalidCatalogSelection);
    }
    if collision_inventory.generation != project.target {
        reject!(Revision3NpcDraftInsertConflictV1::CollisionGenerationMismatch);
    }
    if collision_inventory.basis_head != *exact_basis_head
        || collision_inventory.project_id != project.project_id
        || collision_inventory.project_revision != project.revision
        || collision_inventory.current_project != seal_bytes(canonical_project_json.as_bytes())
    {
        reject!(Revision3NpcDraftInsertConflictV1::CollisionBasisMismatch);
    }
    if collision_inventory.story_catalog_seal != selection.story_catalog_seal {
        reject!(Revision3NpcDraftInsertConflictV1::CollisionStoryCatalogMismatch);
    }
    if collision_inventory.catalog_layer != REVISION3_NPC_EXACT_COLLISION_LAYER_V1 {
        reject!(Revision3NpcDraftInsertConflictV1::CollisionLayerMismatch);
    }
    if !valid_collision_inventory(&collision_inventory) {
        reject!(Revision3NpcDraftInsertConflictV1::InvalidCollisionInventory);
    }

    if let Some(reason) = residual_quest_state(&project) {
        reject!(Revision3NpcDraftInsertConflictV1::InvalidBasisStoryState { reason });
    }

    let owner = TypedRef::new(project.project_id, request.npc_id, EntityKind::NpcDraft);
    let module_ref = TypedRef::new(
        project.project_id,
        request.script_module_id,
        EntityKind::ScriptModule,
    );
    let npc = NpcDraft {
        generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
        generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        input: NpcDraftInput {
            target: project.target.clone(),
            module_namespace: request.intent.module_namespace,
            unique_name: request.intent.unique_name,
            parent_character_definition: selection.parent_character_definition,
            parent_ai_agent_config: selection.parent_ai_agent_config,
            parent_spawn_definition: selection.parent_spawn_definition,
        },
        script_module: module_ref,
    };
    let runtime_id = npc.input.unique_name.clone();
    let (module, generated_identity) =
        match npc.regenerate_script_module_with_identity(owner.clone()) {
            Ok(generated) => generated,
            Err(error) => {
                reject!(Revision3NpcDraftInsertConflictV1::InvalidNpcIntent {
                    reason: regeneration_reason(error),
                });
            }
        };

    if let Some(existing_entity) = find_runtime_identity(&project, &runtime_id) {
        reject!(Revision3NpcDraftInsertConflictV1::StoryIdentityCollision {
            kind: Revision3NpcStoryIdentityKindV1::AuthoredRuntimeId,
            value: runtime_id,
            existing_entity: Some(existing_entity),
        });
    }
    if collision_inventory
        .catalog_runtime_ids
        .contains(&runtime_id.to_ascii_lowercase())
    {
        reject!(Revision3NpcDraftInsertConflictV1::StoryIdentityCollision {
            kind: Revision3NpcStoryIdentityKindV1::AuthoredRuntimeId,
            value: runtime_id,
            existing_entity: None,
        });
    }
    if let Some(conflict) = find_exact_collision(&generated_identity, &collision_inventory) {
        reject!(conflict);
    }

    let npc_entity = Entity {
        id: request.npc_id,
        display_name: request.display_name,
        origin: OriginRef::New {
            authored_runtime_id: runtime_id,
        },
        revision: 0,
        payload: EntityPayload::NpcDraft(npc),
    };
    let module_entity = Entity {
        id: request.script_module_id,
        display_name: module.module_namespace.clone(),
        origin: OriginRef::Generated {
            generator_id: LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
            generator_version: LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            owner,
        },
        revision: 0,
        payload: EntityPayload::ScriptModule(module),
    };
    let replaced_npc = project.entities.insert(request.npc_id, npc_entity);
    let replaced_module = project
        .entities
        .insert(request.script_module_id, module_entity);
    debug_assert!(replaced_npc.is_none());
    debug_assert!(replaced_module.is_none());
    project.revision = next_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(error) => {
            reject!(Revision3NpcDraftInsertConflictV1::CandidateNotPersistable {
                reason: error.to_string(),
            });
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3NpcDraftInsertErrorV1::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3NpcDraftInsertErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3NpcDraftInsertEvaluationV1::Applied(Box::new(
        Revision3NpcDraftInsertOutcomeV1 {
            project,
            canonical_project_json,
            basis_head: exact_basis_head.clone(),
            npc_id: request.npc_id,
            script_module_id: request.script_module_id,
            build_status: Revision3NpcDraftBuildStatusV1::Blocked,
            runtime_status: Revision3NpcDraftRuntimeStatusV1::RuntimeUnqualified,
            catalog_authority: Revision3NpcCatalogAuthorityV1::NotGranted,
            collision_authority: Revision3NpcCollisionAuthorityV1::NotGranted,
            source_inspection: Revision3NpcSourceInspectionStatusV1::FreshNativeContextRequired,
            publication_status: Revision3NpcDraftPublicationStatusV1::NotSupported,
        },
    )))
}

fn valid_display_name(value: &str) -> bool {
    !value.trim().is_empty()
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

fn valid_selection(value: &Revision3NpcCatalogSelectionV1) -> bool {
    valid_seal(&value.story_catalog_seal)
        && valid_seal(&value.npc_catalog_seal)
        && [
            &value.parent_character_definition,
            &value.parent_ai_agent_config,
            &value.parent_spawn_definition,
        ]
        .into_iter()
        .all(|parent| parent.generation == value.generation && valid_seal(&parent.source_seal))
}

fn valid_collision_inventory(value: &Revision3NpcCollisionInventoryV1) -> bool {
    if !valid_seal(&value.current_project)
        || !valid_seal(&value.source_seal)
        || !valid_seal(&value.story_catalog_seal)
    {
        return false;
    }
    let mut count = 0usize;
    let mut bytes = 0usize;
    for entry in value
        .catalog_runtime_ids
        .iter()
        .chain(value.modules.iter())
        .chain(value.relative_paths.iter())
        .chain(value.symbols.iter())
    {
        if entry.is_empty()
            || entry.len() > crate::quest::MAX_COLLISION_ENTRY_BYTES
            || !entry.is_ascii()
            || entry
                .bytes()
                .any(|byte| byte.is_ascii_uppercase() || byte.is_ascii_control())
        {
            return false;
        }
        count = count.saturating_add(1);
        bytes = bytes.saturating_add(entry.len());
        if count > crate::quest::MAX_COLLISION_ENTRIES
            || bytes > crate::quest::MAX_COLLISION_TOTAL_BYTES
        {
            return false;
        }
    }
    true
}

fn valid_seal(value: &ContentSeal) -> bool {
    value.byte_len != 0 && !is_zero_digest(value.sha256)
}

fn is_zero_digest(value: Sha256Digest) -> bool {
    value.as_bytes().iter().all(|byte| *byte == 0)
}

fn is_zero_entity_id(value: EntityId) -> bool {
    value.as_bytes().iter().all(|byte| *byte == 0)
}

fn regeneration_reason(error: StoryRegenerationError) -> String {
    error.to_string()
}

fn find_runtime_identity(project: &ProjectRevision3, candidate: &str) -> Option<EntityId> {
    project.entities.iter().find_map(|(id, entity)| {
        if !matches!(
            entity.payload,
            EntityPayload::NpcDraft(_) | EntityPayload::QuestDraft(_)
        ) {
            return None;
        }
        match &entity.origin {
            OriginRef::New {
                authored_runtime_id,
            } if authored_runtime_id.eq_ignore_ascii_case(candidate) => Some(*id),
            _ => None,
        }
    })
}

fn find_exact_collision(
    generated: &GeneratedStoryIdentity,
    inventory: &Revision3NpcCollisionInventoryV1,
) -> Option<Revision3NpcDraftInsertConflictV1> {
    find_collision_in_sets(
        generated,
        &inventory.modules,
        &inventory.relative_paths,
        &inventory.symbols,
        None,
    )
}

fn residual_quest_state(project: &ProjectRevision3) -> Option<String> {
    for (module_id, entity) in &project.entities {
        let EntityPayload::ScriptModule(module) = &entity.payload else {
            continue;
        };
        let quest_marker = module.owner.expected_kind == EntityKind::QuestDraft
            || module.generator_id == REVISION3_QUEST_GENERATOR_ID
            || matches!(
                &entity.origin,
                OriginRef::Generated { generator_id, owner, .. }
                    if generator_id == REVISION3_QUEST_GENERATOR_ID
                        || owner.expected_kind == EntityKind::QuestDraft
            );
        if !quest_marker {
            continue;
        }
        let Some(owner) = project.entities.get(&module.owner.id) else {
            return Some(format!(
                "Quest-generated ScriptModule {module_id} has no owner"
            ));
        };
        let EntityPayload::QuestDraft(quest) = &owner.payload else {
            return Some(format!(
                "Quest-generated ScriptModule {module_id} owner has the wrong kind"
            ));
        };
        if module.owner.project_id != project.project_id
            || module.owner.expected_kind != EntityKind::QuestDraft
            || quest.script_module
                != TypedRef::new(project.project_id, *module_id, EntityKind::ScriptModule)
        {
            return Some(format!(
                "Quest-generated ScriptModule {module_id} has no exact reverse ownership"
            ));
        }
    }
    None
}

fn seal_bytes(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn find_collision_in_sets(
    generated: &GeneratedStoryIdentity,
    modules: &BTreeSet<String>,
    relative_paths: &BTreeSet<String>,
    symbols: &BTreeSet<String>,
    existing_entity: Option<EntityId>,
) -> Option<Revision3NpcDraftInsertConflictV1> {
    for (kind, value, values) in [
        (
            Revision3NpcStoryIdentityKindV1::ModuleNamespace,
            generated.module_namespace.as_str(),
            modules,
        ),
        (
            Revision3NpcStoryIdentityKindV1::ModuleRelativePath,
            generated.module_relative_path.as_str(),
            relative_paths,
        ),
    ] {
        if values.contains(&value.to_ascii_lowercase()) {
            return Some(Revision3NpcDraftInsertConflictV1::StoryIdentityCollision {
                kind,
                value: value.to_owned(),
                existing_entity,
            });
        }
    }
    for symbol in &generated.symbols {
        if symbols.contains(&symbol.to_ascii_lowercase()) {
            return Some(Revision3NpcDraftInsertConflictV1::StoryIdentityCollision {
                kind: Revision3NpcStoryIdentityKindV1::GeneratedSymbol,
                value: symbol.clone(),
                existing_entity,
            });
        }
    }
    None
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
                "revision-3 NPC request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
