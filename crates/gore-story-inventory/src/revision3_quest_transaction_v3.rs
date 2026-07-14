//! Authority-sensitive, filesystem-free insertion of one revision-3 Quest Draft/module pair.
//!
//! The transaction consumes the linear v2 collision capsule. Its current-project input is only an
//! untrusted transport until its exact canonical bytes, raw seal, project identity, revision,
//! target, and published basis head all agree with the retained fresh capability. Parent and giver
//! values are never accepted from the request; only bounded catalog IDs are resolved by that
//! capability. The result remains an offline draft and grants no build, runtime, artifact, source
//! inspection, publication, store, or head-CAS authority.

use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Write};

use gore_authoring::{
    regenerate_revision3_quest_module_v2, AssetMeta, ContentSeal as AuthoringContentSeal,
    DraftQuestCollisionKind, DraftQuestSkeletonError, EntityId, ProjectId, ProjectRevision3,
    ProjectRevision3JsonError, QuestCollisionArtifactRef, Revision3Entity as Entity,
    Revision3EntityKind as EntityKind, Revision3EntityPayload as EntityPayload,
    Revision3OriginRef as OriginRef, Revision3QuestDraft as QuestDraft,
    Revision3QuestDraftInput as QuestDraftInput, Revision3QuestGenerationError,
    Revision3TypedRef as TypedRef, Sha256Digest as AuthoringSha256Digest, WorkingHead,
    MAX_PROJECT_JSON_BYTES, MAX_REVISION3_ASSETS, MAX_REVISION3_ENTITIES,
    MAX_REVISION3_QUEST_DRAFT_DISPLAY_NAME_BYTES, MAX_REVISION3_REFERENCED_ASSET_BYTES,
    QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2, REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION,
    REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest as _, Sha256};

use super::{
    ContentSeal, PreparedQuestCollisionArtifactV2, QuestCollisionCapabilityArtifactV2,
    Revision3QuestCollisionCapabilityArtifactVerificationErrorV2,
    Revision3QuestCollisionCapabilityErrorV2, VerifiedRevision3QuestCollisionCapabilityV2,
    BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2,
};

/// Maximum exact canonical request-v3 transport size.
pub const MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3: usize = 64 * 1024;

/// Bounded intent selected by the author. Catalog IDs are resolved only after capability binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestDraftIntentV3 {
    pub module_namespace: String,
    pub technical_id: String,
    pub text_helper: String,
    pub parent_catalog_id: String,
    pub giver_catalog_id: String,
    pub title: String,
    pub description: String,
    pub objective_title: String,
    /// Ordered objectives after objective 1. Empty is omitted to retain the exact canonical v3
    /// request spelling accepted by existing clients.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_objective_titles: Vec<String>,
}

/// Exact basis-CAS-bound request for one authority-sensitive revision-3 Quest transaction.
///
/// Target generation, artifact references, seals, resolved catalog values, source, and generated
/// names are deliberately absent. They are derived inside the transaction from the consumed fresh
/// capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestDraftInsertRequestV3 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub quest_id: EntityId,
    pub script_module_id: EntityId,
    pub display_name: String,
    pub intent: Revision3QuestDraftIntentV3,
}

impl Revision3QuestDraftInsertRequestV3 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3QuestDraftInsertRequestJsonErrorV3> {
        if json.len() > MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3 {
            return Err(Revision3QuestDraftInsertRequestJsonErrorV3::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3QuestDraftInsertRequestJsonErrorV3::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3QuestDraftInsertRequestJsonErrorV3::InvalidJson)?;
        let canonical = request.to_canonical_json()?;
        if canonical.as_bytes() != json.as_bytes() {
            return Err(Revision3QuestDraftInsertRequestJsonErrorV3::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3QuestDraftInsertRequestJsonErrorV3> {
        let mut writer = BoundedRequestWriter::new(MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3QuestDraftInsertRequestJsonErrorV3::InputTooLarge {
                actual,
                limit: MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3,
            });
        }
        serialized.map_err(Revision3QuestDraftInsertRequestJsonErrorV3::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3QuestDraftInsertRequestJsonErrorV3::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestDraftInsertRequestJsonErrorV3 {
    #[error("revision-3 Quest request-v3 exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Quest request-v3 JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Quest request-v3 JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Quest request-v3 JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Quest request-v3 serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestEntityRoleV3 {
    QuestDraft,
    ScriptModule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3StoryIdentityKindV3 {
    AuthoredRuntimeId,
    ModuleNamespace,
    ModuleRelativePath,
    GeneratedSymbol,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3QuestDraftBindingErrorV3 {
    #[error("request basis head does not match the exact capability head")]
    CurrentHeadMismatch,
    #[error("project identity does not match the exact capability/request binding")]
    ProjectIdentityMismatch,
    #[error("project revision does not match the exact capability/request binding")]
    ProjectRevisionMismatch,
    #[error("project target does not match the exact capability target")]
    ProjectTargetMismatch,
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestDraftConflictV3 {
    #[error("project revision cannot be incremented")]
    ProjectRevisionOverflow,
    #[error("{role:?} entity ID must not be zero")]
    ZeroEntityId { role: Revision3QuestEntityRoleV3 },
    #[error("Quest Draft and ScriptModule IDs must differ")]
    SharedEntityId,
    #[error("{role:?} entity ID {entity} already exists")]
    EntityIdCollision {
        role: Revision3QuestEntityRoleV3,
        entity: EntityId,
    },
    #[error("display name is empty, contains controls, or exceeds its byte limit")]
    InvalidDisplayName,
    #[error("revision-3 project cannot hold two additional entities")]
    EntityCapacityExceeded,
    #[error("revision-3 project cannot hold the structural collision artifact asset")]
    AssetCapacityExceeded,
    #[error("revision-3 project asset bytes would exceed the closed-model limit")]
    AssetBytesExceeded,
    #[error("structural collision artifact digest already has incompatible AssetStore metadata")]
    ArtifactAssetMetadataCollision,
    #[error("invalid catalog selection: {0}")]
    CatalogSelection(#[source] Revision3QuestCollisionCapabilityErrorV2),
    #[error("invalid Quest intent: {error}")]
    InvalidQuestIntent { error: DraftQuestSkeletonError },
    #[error("{kind:?} {value:?} collides with the exact base/current collision authority")]
    StoryIdentityCollision {
        kind: Revision3StoryIdentityKindV3,
        value: String,
    },
    #[error("authored runtime identity {value:?} already belongs to entity {existing_entity}")]
    RuntimeIdentityCollision {
        value: String,
        existing_entity: EntityId,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestDraftProjectTransportErrorV3 {
    #[error("current revision-3 project transport exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid exact canonical revision-3 project transport: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("current revision-3 project transport differs from the exact retained project seal")]
    CurrentProjectSealMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestDraftBuildStatusV3 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestDraftRuntimeStatusV3 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestArtifactAuthorityV3 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestSourceInspectionStatusV3 {
    FreshCapabilityRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestDraftPublicationStatusV3 {
    NotSupported,
}

/// Linear in-memory transaction result. It intentionally has no `Clone` implementation.
///
/// The structural artifact bytes are retained for a later store transaction, but are not
/// authority evidence. `basis_head` is the old exact head; no new head exists until a future CAS
/// persistence boundary publishes one. Its fields are externally opaque so a caller cannot
/// rewrite the capability-checked result before handing the whole value to that boundary.
///
/// ```compile_fail
/// fn rewrite(mut outcome: gore_story_inventory::Revision3QuestDraftInsertOutcomeV3) {
///     outcome.project.revision += 1;
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Revision3QuestDraftInsertOutcomeV3 {
    pub(crate) project: ProjectRevision3,
    pub(crate) canonical_project_json: String,
    pub(crate) collision_artifact: QuestCollisionCapabilityArtifactV2,
    pub(crate) basis_head: WorkingHead,
    pub(crate) quest_id: EntityId,
    pub(crate) script_module_id: EntityId,
    pub(crate) build_status: Revision3QuestDraftBuildStatusV3,
    pub(crate) runtime_status: Revision3QuestDraftRuntimeStatusV3,
    pub(crate) artifact_authority: Revision3QuestArtifactAuthorityV3,
    pub(crate) source_inspection: Revision3QuestSourceInspectionStatusV3,
    pub(crate) publication_status: Revision3QuestDraftPublicationStatusV3,
}

impl Revision3QuestDraftInsertOutcomeV3 {
    pub fn project(&self) -> &ProjectRevision3 {
        &self.project
    }

    pub fn canonical_project_json(&self) -> &str {
        &self.canonical_project_json
    }

    pub fn collision_artifact(&self) -> &QuestCollisionCapabilityArtifactV2 {
        &self.collision_artifact
    }

    pub fn basis_head(&self) -> &WorkingHead {
        &self.basis_head
    }

    pub const fn quest_id(&self) -> EntityId {
        self.quest_id
    }

    pub const fn script_module_id(&self) -> EntityId {
        self.script_module_id
    }

    pub const fn build_status(&self) -> Revision3QuestDraftBuildStatusV3 {
        self.build_status
    }

    pub const fn runtime_status(&self) -> Revision3QuestDraftRuntimeStatusV3 {
        self.runtime_status
    }

    pub const fn artifact_authority(&self) -> Revision3QuestArtifactAuthorityV3 {
        self.artifact_authority
    }

    pub const fn source_inspection(&self) -> Revision3QuestSourceInspectionStatusV3 {
        self.source_inspection
    }

    pub const fn publication_status(&self) -> Revision3QuestDraftPublicationStatusV3 {
        self.publication_status
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestDraftInsertErrorV3 {
    #[error("invalid prepared revision-3 collision capability: {0}")]
    Capability(#[source] Revision3QuestCollisionCapabilityArtifactVerificationErrorV2),
    #[error("invalid current-project transport: {0}")]
    ProjectTransport(#[source] Revision3QuestDraftProjectTransportErrorV3),
    #[error("invalid exact canonical revision-3 Quest request-v3: {0}")]
    Request(#[source] Revision3QuestDraftInsertRequestJsonErrorV3),
    #[error("revision-3 Quest transaction binding failed: {0}")]
    Binding(#[source] Revision3QuestDraftBindingErrorV3),
    #[error("revision-3 Quest transaction rejected: {0}")]
    Conflict(#[source] Revision3QuestDraftConflictV3),
    #[error("unexpected revision-3 Quest generation failure: {0}")]
    Generation(#[source] Revision3QuestGenerationError),
    #[error("candidate revision-3 project is not closed and persistable: {0}")]
    ClosedModel(#[source] ProjectRevision3JsonError),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    CanonicalReopen(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Atomically construct one offline revision-3 Quest Draft and deterministic ScriptModule.
///
/// This function performs no filesystem write and publishes no head. Both inputs are untrusted
/// transports; all changes occur on a locally owned project and are returned only after exact
/// canonical reopen equality succeeds.
pub fn apply_revision3_quest_draft_transaction_v3(
    prepared: PreparedQuestCollisionArtifactV2,
    current_project_transport_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3QuestDraftInsertOutcomeV3, Revision3QuestDraftInsertErrorV3> {
    if current_project_transport_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Revision3QuestDraftInsertErrorV3::ProjectTransport(
            Revision3QuestDraftProjectTransportErrorV3::InputTooLarge {
                actual: current_project_transport_json.len(),
                limit: MAX_PROJECT_JSON_BYTES,
            },
        ));
    }
    let (capability, collision_artifact) = prepared
        .into_transaction_authority()
        .map_err(Revision3QuestDraftInsertErrorV3::Capability)?;
    if raw_authoring_seal(current_project_transport_json.as_bytes())
        != *capability.current_project()
    {
        return Err(Revision3QuestDraftInsertErrorV3::ProjectTransport(
            Revision3QuestDraftProjectTransportErrorV3::CurrentProjectSealMismatch,
        ));
    }
    let mut project =
        ProjectRevision3::from_json(current_project_transport_json).map_err(|error| {
            Revision3QuestDraftInsertErrorV3::ProjectTransport(
                Revision3QuestDraftProjectTransportErrorV3::InvalidProject(error),
            )
        })?;

    let request = Revision3QuestDraftInsertRequestV3::from_json(canonical_request_json)
        .map_err(Revision3QuestDraftInsertErrorV3::Request)?;
    validate_exact_bindings(&capability, &project, &request)
        .map_err(Revision3QuestDraftInsertErrorV3::Binding)?;

    let basis_head = capability.current_head().clone();
    let parent = capability
        .resolve_parent(&request.intent.parent_catalog_id)
        .map_err(|error| {
            Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::CatalogSelection(error),
            )
        })?;
    let giver = capability
        .resolve_giver(&request.intent.giver_catalog_id)
        .map_err(|error| {
            Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::CatalogSelection(error),
            )
        })?;

    validate_request_shape(&project, &request)
        .map_err(Revision3QuestDraftInsertErrorV3::Conflict)?;
    let next_revision = project.revision.checked_add(1).ok_or({
        Revision3QuestDraftInsertErrorV3::Conflict(
            Revision3QuestDraftConflictV3::ProjectRevisionOverflow,
        )
    })?;
    let Revision3QuestDraftInsertRequestV3 {
        expected_head: _,
        expected_project_id: _,
        expected_revision: _,
        quest_id,
        script_module_id,
        display_name,
        intent,
    } = request;
    let Revision3QuestDraftIntentV3 {
        module_namespace,
        technical_id,
        text_helper,
        parent_catalog_id: _,
        giver_catalog_id: _,
        title,
        description,
        objective_title,
        additional_objective_titles,
    } = intent;

    let raw_artifact = authoring_seal(collision_artifact.artifact_seal());
    let semantic_artifact = authoring_seal(collision_artifact.source_seal());
    let artifact_meta = AssetMeta {
        byte_len: raw_artifact.byte_len,
        media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
    };
    let artifact_already_indexed = match project.asset_store.assets.get(&raw_artifact.sha256) {
        Some(existing) if existing == &artifact_meta => true,
        Some(_) => {
            return Err(Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::ArtifactAssetMetadataCollision,
            ));
        }
        None => false,
    };
    validate_asset_capacity(&project, artifact_already_indexed, raw_artifact.byte_len)
        .map_err(Revision3QuestDraftInsertErrorV3::Conflict)?;

    let artifact_reference = QuestCollisionArtifactRef {
        generation: project.target.clone(),
        catalog_layer: BASE_GAME_AND_EXACT_REVISION3_PROJECT_COLLISION_LAYER_V2.to_owned(),
        artifact: raw_artifact.clone(),
        source_seal: semantic_artifact,
        basis_snapshot: basis_head.snapshot.clone(),
    };
    let owner = TypedRef::new(project.project_id, quest_id, EntityKind::QuestDraft);
    let module_ref = TypedRef::new(
        project.project_id,
        script_module_id,
        EntityKind::ScriptModule,
    );
    let quest = QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
        generator_version: if additional_objective_titles.is_empty() {
            REVISION3_QUEST_GENERATOR_VERSION
        } else {
            REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION
        },
        input: QuestDraftInput {
            target: project.target.clone(),
            quest_id,
            module_namespace,
            technical_id,
            text_helper,
            parent_quest: parent,
            giver,
            title,
            description,
            objective_title,
            additional_objective_titles,
            collision_catalog: artifact_reference,
        },
        script_module: module_ref,
    };
    let runtime_id = quest.input.technical_id.clone();
    let collision_input = capability.into_quest_collision_input();
    let module = match regenerate_revision3_quest_module_v2(&quest, collision_input) {
        Ok(module) => module,
        Err(Revision3QuestGenerationError::InvalidQuestIntent(
            DraftQuestSkeletonError::GeneratedNameCollision { kind, name },
        )) => {
            return Err(Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::StoryIdentityCollision {
                    kind: story_identity_kind(kind),
                    value: name,
                },
            ));
        }
        Err(Revision3QuestGenerationError::InvalidQuestIntent(error)) => {
            return Err(Revision3QuestDraftInsertErrorV3::Conflict(
                Revision3QuestDraftConflictV3::InvalidQuestIntent { error },
            ));
        }
        Err(error) => return Err(Revision3QuestDraftInsertErrorV3::Generation(error)),
    };
    if let Some(existing_entity) = find_runtime_identity(&project, &runtime_id) {
        return Err(Revision3QuestDraftInsertErrorV3::Conflict(
            Revision3QuestDraftConflictV3::RuntimeIdentityCollision {
                value: runtime_id,
                existing_entity,
            },
        ));
    }

    if !artifact_already_indexed {
        let replaced = project
            .asset_store
            .assets
            .insert(raw_artifact.sha256, artifact_meta);
        debug_assert!(replaced.is_none());
    }
    let quest_generator_version = quest.generator_version;
    let quest_entity = Entity {
        id: quest_id,
        display_name,
        origin: OriginRef::New {
            authored_runtime_id: runtime_id,
        },
        revision: 0,
        payload: EntityPayload::QuestDraft(quest),
    };
    let module_entity = Entity {
        id: script_module_id,
        display_name: module.module_namespace.clone(),
        origin: OriginRef::Generated {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: quest_generator_version,
            owner,
        },
        revision: 0,
        payload: EntityPayload::ScriptModule(module),
    };
    let replaced_quest = project.entities.insert(quest_id, quest_entity);
    let replaced_module = project.entities.insert(script_module_id, module_entity);
    debug_assert!(replaced_quest.is_none());
    debug_assert!(replaced_module.is_none());
    project.revision = next_revision;

    let canonical_project_json = project
        .to_canonical_json()
        .map_err(Revision3QuestDraftInsertErrorV3::ClosedModel)?;
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3QuestDraftInsertErrorV3::CanonicalReopen)?;
    if reopened != project {
        return Err(Revision3QuestDraftInsertErrorV3::CanonicalReopenMismatch);
    }

    Ok(Revision3QuestDraftInsertOutcomeV3 {
        project,
        canonical_project_json,
        collision_artifact,
        basis_head,
        quest_id,
        script_module_id,
        build_status: Revision3QuestDraftBuildStatusV3::Blocked,
        runtime_status: Revision3QuestDraftRuntimeStatusV3::RuntimeUnqualified,
        artifact_authority: Revision3QuestArtifactAuthorityV3::NotGranted,
        source_inspection: Revision3QuestSourceInspectionStatusV3::FreshCapabilityRequired,
        publication_status: Revision3QuestDraftPublicationStatusV3::NotSupported,
    })
}

fn validate_exact_bindings(
    capability: &VerifiedRevision3QuestCollisionCapabilityV2,
    project: &ProjectRevision3,
    request: &Revision3QuestDraftInsertRequestV3,
) -> Result<(), Revision3QuestDraftBindingErrorV3> {
    if request.expected_head != *capability.current_head() {
        return Err(Revision3QuestDraftBindingErrorV3::CurrentHeadMismatch);
    }
    if project.project_id != capability.project_id()
        || request.expected_project_id != capability.project_id()
    {
        return Err(Revision3QuestDraftBindingErrorV3::ProjectIdentityMismatch);
    }
    if project.revision != capability.project_revision()
        || request.expected_revision != capability.project_revision()
    {
        return Err(Revision3QuestDraftBindingErrorV3::ProjectRevisionMismatch);
    }
    if &project.target != capability.project_target() {
        return Err(Revision3QuestDraftBindingErrorV3::ProjectTargetMismatch);
    }
    Ok(())
}

fn validate_request_shape(
    project: &ProjectRevision3,
    request: &Revision3QuestDraftInsertRequestV3,
) -> Result<(), Revision3QuestDraftConflictV3> {
    for (role, id) in [
        (Revision3QuestEntityRoleV3::QuestDraft, request.quest_id),
        (
            Revision3QuestEntityRoleV3::ScriptModule,
            request.script_module_id,
        ),
    ] {
        if is_zero_entity_id(id) {
            return Err(Revision3QuestDraftConflictV3::ZeroEntityId { role });
        }
        if project.entities.contains_key(&id) {
            return Err(Revision3QuestDraftConflictV3::EntityIdCollision { role, entity: id });
        }
    }
    if request.quest_id == request.script_module_id {
        return Err(Revision3QuestDraftConflictV3::SharedEntityId);
    }
    if !is_valid_revision3_quest_draft_display_name_v3(&request.display_name) {
        return Err(Revision3QuestDraftConflictV3::InvalidDisplayName);
    }
    validate_entity_capacity(project.entities.len())
}

pub(crate) fn is_valid_revision3_quest_draft_display_name_v3(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_REVISION3_QUEST_DRAFT_DISPLAY_NAME_BYTES
        && !value.chars().any(char::is_control)
}

fn validate_entity_capacity(existing_count: usize) -> Result<(), Revision3QuestDraftConflictV3> {
    if existing_count
        .checked_add(2)
        .is_none_or(|count| count > MAX_REVISION3_ENTITIES)
    {
        return Err(Revision3QuestDraftConflictV3::EntityCapacityExceeded);
    }
    Ok(())
}

fn validate_asset_capacity(
    project: &ProjectRevision3,
    artifact_already_indexed: bool,
    artifact_bytes: u64,
) -> Result<(), Revision3QuestDraftConflictV3> {
    let existing_bytes = project
        .asset_store
        .assets
        .values()
        .try_fold(0u64, |total, asset| total.checked_add(asset.byte_len))
        .ok_or(Revision3QuestDraftConflictV3::AssetBytesExceeded)?;
    validate_asset_capacity_values(
        project.asset_store.assets.len(),
        existing_bytes,
        artifact_already_indexed,
        artifact_bytes,
    )
}

fn validate_asset_capacity_values(
    existing_count: usize,
    existing_bytes: u64,
    artifact_already_indexed: bool,
    artifact_bytes: u64,
) -> Result<(), Revision3QuestDraftConflictV3> {
    if !artifact_already_indexed
        && existing_count
            .checked_add(1)
            .is_none_or(|count| count > MAX_REVISION3_ASSETS)
    {
        return Err(Revision3QuestDraftConflictV3::AssetCapacityExceeded);
    }
    let candidate_bytes = if artifact_already_indexed {
        existing_bytes
    } else {
        existing_bytes
            .checked_add(artifact_bytes)
            .ok_or(Revision3QuestDraftConflictV3::AssetBytesExceeded)?
    };
    if candidate_bytes > MAX_REVISION3_REFERENCED_ASSET_BYTES {
        return Err(Revision3QuestDraftConflictV3::AssetBytesExceeded);
    }
    Ok(())
}

fn find_runtime_identity(project: &ProjectRevision3, candidate: &str) -> Option<EntityId> {
    project.entities.iter().find_map(|(id, entity)| {
        if !matches!(
            &entity.payload,
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

fn story_identity_kind(kind: DraftQuestCollisionKind) -> Revision3StoryIdentityKindV3 {
    match kind {
        DraftQuestCollisionKind::Module => Revision3StoryIdentityKindV3::ModuleNamespace,
        DraftQuestCollisionKind::RelativePath => Revision3StoryIdentityKindV3::ModuleRelativePath,
        DraftQuestCollisionKind::Symbol => Revision3StoryIdentityKindV3::GeneratedSymbol,
    }
}

fn is_zero_entity_id(id: EntityId) -> bool {
    id.as_bytes().iter().all(|byte| *byte == 0)
}

fn authoring_seal(seal: &ContentSeal) -> AuthoringContentSeal {
    AuthoringContentSeal {
        byte_len: seal.byte_len,
        sha256: AuthoringSha256Digest::from_bytes(*seal.sha256.as_bytes()),
    }
}

fn raw_authoring_seal(bytes: &[u8]) -> AuthoringContentSeal {
    AuthoringContentSeal {
        byte_len: bytes.len() as u64,
        sha256: AuthoringSha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn reject_duplicate_object_keys(json: &str) -> Result<(), serde_json::Error> {
    serde_json::from_str::<DuplicateSafeIgnored>(json).map(|_| ())
}

struct DuplicateSafeIgnored;

impl<'de> Deserialize<'de> for DuplicateSafeIgnored {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateSafeIgnoredVisitor)
    }
}

struct DuplicateSafeIgnoredVisitor;

impl<'de> Visitor<'de> for DuplicateSafeIgnoredVisitor {
    type Value = DuplicateSafeIgnored;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any duplicate-key-free JSON value")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateSafeIgnored)
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while access.next_element::<DuplicateSafeIgnored>()?.is_some() {}
        Ok(DuplicateSafeIgnored)
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        while let Some(key) = access.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object key {key:?}"
                )));
            }
            access.next_value::<DuplicateSafeIgnored>()?;
        }
        Ok(DuplicateSafeIgnored)
    }
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
                "revision-3 Quest request-v3 JSON limit exceeded",
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
    use super::*;
    use gore_authoring::{
        ContentSeal as AuthoringContentSeal, Sha256Digest as AuthoringSha256Digest,
        WorkingStoreFormat,
    };

    trait AmbiguousIfClone<Marker> {
        fn marker() {}
    }

    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    impl<T: Clone> AmbiguousIfClone<u8> for T {}

    fn request() -> Revision3QuestDraftInsertRequestV3 {
        Revision3QuestDraftInsertRequestV3 {
            expected_head: WorkingHead {
                store_format: WorkingStoreFormat,
                snapshot: AuthoringContentSeal {
                    byte_len: 1,
                    sha256: AuthoringSha256Digest::from_bytes([1; 32]),
                },
            },
            expected_project_id: ProjectId::from_bytes([2; 16]),
            expected_revision: 7,
            quest_id: EntityId::from_bytes([3; 16]),
            script_module_id: EntityId::from_bytes([4; 16]),
            display_name: "Request v3".to_owned(),
            intent: Revision3QuestDraftIntentV3 {
                module_namespace: "GoreMods.Quests.RequestV3".to_owned(),
                technical_id: "GORE_REQUEST_V3".to_owned(),
                text_helper: "GoreRequestV3Text".to_owned(),
                parent_catalog_id: "g1r:quest-parent:swampcamp_scchapter2".to_owned(),
                giver_catalog_id: "g1r:npc:om_grd_asghan_263".to_owned(),
                title: "Request v3".to_owned(),
                description: "Strict request transport".to_owned(),
                objective_title: "Stay canonical".to_owned(),
                additional_objective_titles: Vec::new(),
            },
        }
    }

    #[test]
    fn request_v3_is_exact_canonical_bounded_and_duplicate_free() {
        let base_request = request();
        let canonical = base_request.to_canonical_json().unwrap();
        assert!(!canonical.contains("additional_objective_titles"));
        for forbidden in [
            "\"expected_target\":",
            "\"collision_catalog\":",
            "\"artifact_seal\":",
            "\"source_seal\":",
            "\"basis_snapshot\":",
            "\"parent_quest\":",
            "\"giver\":",
            "\"script_module\":",
            "\"generated_symbol\":",
        ] {
            assert!(
                !canonical.contains(forbidden),
                "request-v3 retained forbidden authority field {forbidden}"
            );
        }
        assert_eq!(
            Revision3QuestDraftInsertRequestV3::from_json(&canonical).unwrap(),
            base_request
        );
        assert!(matches!(
            Revision3QuestDraftInsertRequestV3::from_json(&(canonical.clone() + "\n")),
            Err(Revision3QuestDraftInsertRequestJsonErrorV3::NonCanonicalJson)
        ));
        let duplicate = canonical.replacen(
            "{\"expected_head\":",
            "{\"expected_revision\":7,\"expected_head\":",
            1,
        );
        assert!(matches!(
            Revision3QuestDraftInsertRequestV3::from_json(&duplicate),
            Err(Revision3QuestDraftInsertRequestJsonErrorV3::InvalidJson(_))
        ));
        let nested_duplicate = canonical.replacen(
            "{\"module_namespace\":",
            "{\"title\":\"duplicate\",\"module_namespace\":",
            1,
        );
        assert!(matches!(
            Revision3QuestDraftInsertRequestV3::from_json(&nested_duplicate),
            Err(Revision3QuestDraftInsertRequestJsonErrorV3::InvalidJson(_))
        ));
        assert!(matches!(
            Revision3QuestDraftInsertRequestV3::from_json(
                &"x".repeat(MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES_V3 + 1)
            ),
            Err(Revision3QuestDraftInsertRequestJsonErrorV3::InputTooLarge { .. })
        ));

        let mut multi = request();
        multi.intent.additional_objective_titles =
            vec!["Inspect the gate".to_owned(), "Report to Asghan".to_owned()];
        let multi_json = multi.to_canonical_json().unwrap();
        assert!(multi_json.contains(
            "\"additional_objective_titles\":[\"Inspect the gate\",\"Report to Asghan\"]"
        ));
        assert_eq!(
            Revision3QuestDraftInsertRequestV3::from_json(&multi_json).unwrap(),
            multi
        );
    }

    #[test]
    fn outcome_and_authority_inputs_are_not_cloneable() {
        let _ = <Revision3QuestDraftInsertOutcomeV3 as AmbiguousIfClone<_>>::marker as fn();
        let _ = <PreparedQuestCollisionArtifactV2 as AmbiguousIfClone<_>>::marker as fn();
    }

    #[test]
    fn entity_and_asset_capacity_preflights_accept_exact_limits_and_reject_plus_one() {
        assert!(validate_entity_capacity(MAX_REVISION3_ENTITIES - 2).is_ok());
        assert!(matches!(
            validate_entity_capacity(MAX_REVISION3_ENTITIES - 1),
            Err(Revision3QuestDraftConflictV3::EntityCapacityExceeded)
        ));

        assert!(validate_asset_capacity_values(
            MAX_REVISION3_ASSETS - 1,
            MAX_REVISION3_REFERENCED_ASSET_BYTES - 7,
            false,
            7,
        )
        .is_ok());
        assert!(matches!(
            validate_asset_capacity_values(MAX_REVISION3_ASSETS, 0, false, 1),
            Err(Revision3QuestDraftConflictV3::AssetCapacityExceeded)
        ));
        assert!(matches!(
            validate_asset_capacity_values(0, MAX_REVISION3_REFERENCED_ASSET_BYTES - 7, false, 8,),
            Err(Revision3QuestDraftConflictV3::AssetBytesExceeded)
        ));
        assert!(validate_asset_capacity_values(
            MAX_REVISION3_ASSETS,
            MAX_REVISION3_REFERENCED_ASSET_BYTES,
            true,
            u64::MAX,
        )
        .is_ok());
    }
}
