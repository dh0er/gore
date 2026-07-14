//! Authority-sensitive editing of one existing revision-3 Quest's catalog context.
//!
//! The transaction consumes a fresh exact-current collision capsule even though it introduces no
//! new technical identity. That capsule authenticates the current project/head and carries the
//! only permitted parent/giver resolver. Its structural artifact is discarded: an edit never
//! imports, rewrites, or returns artifact authority. Only description, parent, giver, and the
//! deterministic owned module derived from those fields may change.

use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Write};

use gore_authoring::{
    regenerate_revision3_quest_module_v2, ContentSeal as AuthoringContentSeal,
    DraftQuestSkeletonError, EntityId, ProjectId, ProjectRevision3, ProjectRevision3JsonError,
    QuestCollisionCatalogInput, Revision3EntityKind as EntityKind,
    Revision3EntityPayload as EntityPayload, Revision3QuestDraft as QuestDraft,
    Revision3QuestGenerationError, Sha256Digest as AuthoringSha256Digest, WorkingHead,
    MAX_PROJECT_JSON_BYTES,
};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest as _, Sha256};

use super::{
    ContentSeal, PreparedQuestCollisionArtifactV2,
    Revision3QuestCollisionCapabilityArtifactVerificationErrorV2,
    Revision3QuestCollisionCapabilityErrorV2, VerifiedRevision3QuestCollisionCapabilityV2,
};

/// Maximum exact canonical context-edit request transport size.
pub const MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1: usize = 32 * 1024;

/// Exact head/project/catalog/entity-CAS-bound context edit for one existing Quest.
///
/// The field order is part of the canonical wire contract. Resolved parent/giver values, target,
/// collision data, module data, and all artifact seals other than the expected Story catalog seal
/// are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestContextEditRequestV1 {
    pub expected_head: WorkingHead,
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_story_catalog_seal: ContentSeal,
    pub quest_id: EntityId,
    pub expected_quest_revision: u64,
    pub description: String,
    pub parent_catalog_id: String,
    pub giver_catalog_id: String,
}

impl Revision3QuestContextEditRequestV1 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3QuestContextEditRequestJsonErrorV1> {
        if json.len() > MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1 {
            return Err(Revision3QuestContextEditRequestJsonErrorV1::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3QuestContextEditRequestJsonErrorV1::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3QuestContextEditRequestJsonErrorV1::InvalidJson)?;
        if request.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3QuestContextEditRequestJsonErrorV1::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3QuestContextEditRequestJsonErrorV1> {
        let mut writer =
            BoundedRequestWriter::new(MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1);
        let serialized = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3QuestContextEditRequestJsonErrorV1::InputTooLarge {
                actual,
                limit: MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1,
            });
        }
        serialized.map_err(Revision3QuestContextEditRequestJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3QuestContextEditRequestJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestContextEditRequestJsonErrorV1 {
    #[error(
        "revision-3 Quest context-edit request exceeds the {limit}-byte limit: {actual} bytes"
    )]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Quest context-edit request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Quest context-edit request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Quest context-edit request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Quest context-edit request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3QuestContextEditBindingErrorV1 {
    #[error("request basis head does not match the exact capability head")]
    CurrentHeadMismatch,
    #[error("project identity does not match the exact capability/request binding")]
    ProjectIdentityMismatch,
    #[error("project revision does not match the exact capability/request binding")]
    ProjectRevisionMismatch,
    #[error("project target does not match the exact capability target")]
    ProjectTargetMismatch,
    #[error("request Story catalog seal does not match the exact capability catalog")]
    StoryCatalogSealMismatch,
}

/// Stable semantic rejection. A conflict never returns a partially edited project.
#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestContextEditConflictV1 {
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
    #[error("invalid catalog selection: {0}")]
    CatalogSelection(#[source] Revision3QuestCollisionCapabilityErrorV2),
    #[error("Quest context edit does not change description, parent, or giver")]
    NoChanges,
    #[error("Quest context is invalid: {error}")]
    InvalidQuestContext { error: DraftQuestSkeletonError },
    #[error("Quest context edit unexpectedly changed a preserved technical module identity")]
    TechnicalIdentityChanged,
    #[error("Quest context candidate exceeds the {limit}-byte project limit: {actual} bytes")]
    CandidateTooLarge { actual: usize, limit: usize },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestContextEditProjectTransportErrorV1 {
    #[error("current revision-3 project transport exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid exact canonical revision-3 project transport: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("current revision-3 project transport differs from the exact retained project seal")]
    CurrentProjectSealMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestContextEditBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestContextEditRuntimeStatusV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestContextEditPublicationStatusV1 {
    NotSupported,
}

/// Linear, externally opaque offline edit result. It intentionally has no `Clone` implementation.
///
/// No capability, collision artifact, build result, runtime qualification, persistence token, or
/// publication authority is retained.
#[derive(Debug, PartialEq, Eq)]
pub struct Revision3QuestContextEditOutcomeV1 {
    pub(crate) project: ProjectRevision3,
    pub(crate) canonical_project_json: String,
    pub(crate) basis_head: WorkingHead,
    pub(crate) quest_id: EntityId,
    pub(crate) script_module_id: EntityId,
    pub(crate) quest_revision: u64,
    pub(crate) script_module_revision: u64,
    pub(crate) build_status: Revision3QuestContextEditBuildStatusV1,
    pub(crate) runtime_status: Revision3QuestContextEditRuntimeStatusV1,
    pub(crate) publication_status: Revision3QuestContextEditPublicationStatusV1,
}

impl Revision3QuestContextEditOutcomeV1 {
    pub fn project(&self) -> &ProjectRevision3 {
        &self.project
    }

    pub fn canonical_project_json(&self) -> &str {
        &self.canonical_project_json
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

    pub const fn module_id(&self) -> EntityId {
        self.script_module_id
    }

    pub const fn quest_revision(&self) -> u64 {
        self.quest_revision
    }

    pub const fn script_module_revision(&self) -> u64 {
        self.script_module_revision
    }

    pub const fn module_revision(&self) -> u64 {
        self.script_module_revision
    }

    pub const fn build_status(&self) -> Revision3QuestContextEditBuildStatusV1 {
        self.build_status
    }

    pub const fn runtime_status(&self) -> Revision3QuestContextEditRuntimeStatusV1 {
        self.runtime_status
    }

    pub const fn publication_status(&self) -> Revision3QuestContextEditPublicationStatusV1 {
        self.publication_status
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestContextEditErrorV1 {
    #[error("invalid prepared revision-3 collision capability: {0}")]
    Capability(#[source] Revision3QuestCollisionCapabilityArtifactVerificationErrorV2),
    #[error("invalid current-project transport: {0}")]
    ProjectTransport(#[source] Revision3QuestContextEditProjectTransportErrorV1),
    #[error("invalid exact canonical revision-3 Quest context-edit request: {0}")]
    Request(#[source] Revision3QuestContextEditRequestJsonErrorV1),
    #[error("revision-3 Quest context-edit transaction binding failed: {0}")]
    Binding(#[source] Revision3QuestContextEditBindingErrorV1),
    #[error("revision-3 Quest context-edit transaction rejected: {0}")]
    Conflict(#[source] Revision3QuestContextEditConflictV1),
    #[error("unexpected revision-3 Quest generation failure: {0}")]
    Generation(#[source] Revision3QuestGenerationError),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    CanonicalReopen(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 Quest context candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Atomically edit one existing Quest's description and catalog-resolved parent/giver.
///
/// Both JSON inputs are untrusted transports. The consumed capability authenticates the exact
/// current project and is the only resolver for catalog IDs. The candidate is returned only after
/// full canonical reopen equality; this function performs no filesystem write or head publish.
pub fn apply_revision3_quest_context_edit_transaction_v1(
    prepared: PreparedQuestCollisionArtifactV2,
    current_project_transport_json: &str,
    canonical_request_json: &str,
) -> Result<Revision3QuestContextEditOutcomeV1, Revision3QuestContextEditErrorV1> {
    if current_project_transport_json.len() > MAX_PROJECT_JSON_BYTES {
        return Err(Revision3QuestContextEditErrorV1::ProjectTransport(
            Revision3QuestContextEditProjectTransportErrorV1::InputTooLarge {
                actual: current_project_transport_json.len(),
                limit: MAX_PROJECT_JSON_BYTES,
            },
        ));
    }

    // `_` immediately destroys the structural artifact. Context editing neither imports nor
    // exposes it and cannot turn this transaction into artifact authority.
    let (capability, _) = prepared
        .into_transaction_authority()
        .map_err(Revision3QuestContextEditErrorV1::Capability)?;
    if raw_authoring_seal(current_project_transport_json.as_bytes())
        != *capability.current_project()
    {
        return Err(Revision3QuestContextEditErrorV1::ProjectTransport(
            Revision3QuestContextEditProjectTransportErrorV1::CurrentProjectSealMismatch,
        ));
    }

    let mut project =
        ProjectRevision3::from_json(current_project_transport_json).map_err(|error| {
            Revision3QuestContextEditErrorV1::ProjectTransport(
                Revision3QuestContextEditProjectTransportErrorV1::InvalidProject(error),
            )
        })?;
    let request = Revision3QuestContextEditRequestV1::from_json(canonical_request_json)
        .map_err(Revision3QuestContextEditErrorV1::Request)?;
    validate_exact_bindings(&capability, &project, &request)
        .map_err(Revision3QuestContextEditErrorV1::Binding)?;

    let basis_head = capability.current_head().clone();
    let parent = capability
        .resolve_parent(&request.parent_catalog_id)
        .map_err(|error| {
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::CatalogSelection(error),
            )
        })?;
    let giver = capability
        .resolve_giver(&request.giver_catalog_id)
        .map_err(|error| {
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::CatalogSelection(error),
            )
        })?;

    let next_project_revision =
        project
            .revision
            .checked_add(1)
            .ok_or(Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::ProjectRevisionOverflow,
            ))?;
    if is_zero_entity_id(request.quest_id) {
        return Err(Revision3QuestContextEditErrorV1::Conflict(
            Revision3QuestContextEditConflictV1::ZeroQuestId,
        ));
    }

    let (quest, quest_revision, script_module_id, existing_module, script_module_revision) = {
        let Some(quest_entity) = project.entities.get(&request.quest_id) else {
            return Err(Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::InvalidQuestEntity {
                    quest: request.quest_id,
                },
            ));
        };
        let EntityPayload::QuestDraft(quest) = &quest_entity.payload else {
            return Err(Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::InvalidQuestEntity {
                    quest: request.quest_id,
                },
            ));
        };
        if request.expected_quest_revision != quest_entity.revision {
            return Err(Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::QuestRevisionConflict {
                    expected: request.expected_quest_revision,
                    actual: quest_entity.revision,
                },
            ));
        }
        let next_quest_revision = quest_entity.revision.checked_add(1).ok_or(
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::QuestRevisionOverflow {
                    quest: request.quest_id,
                },
            ),
        )?;
        validate_quest_identity(&project, request.quest_id, quest)?;

        let script_module_id = quest.script_module.id;
        let Some(module_entity) = project.entities.get(&script_module_id) else {
            return Err(invalid_closure(
                request.quest_id,
                "owned ScriptModule is missing",
            ));
        };
        let EntityPayload::ScriptModule(module) = &module_entity.payload else {
            return Err(invalid_closure(
                request.quest_id,
                "owned entity is not a ScriptModule",
            ));
        };
        if module_entity.id != script_module_id
            || module.owner.project_id != project.project_id
            || module.owner.id != request.quest_id
            || module.owner.expected_kind != EntityKind::QuestDraft
        {
            return Err(invalid_closure(
                request.quest_id,
                "ScriptModule owner is not the exact Quest",
            ));
        }
        let next_module_revision = module_entity.revision.checked_add(1).ok_or(
            Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::ScriptModuleRevisionOverflow {
                    module: script_module_id,
                },
            ),
        )?;
        (
            quest.clone(),
            next_quest_revision,
            script_module_id,
            module.clone(),
            next_module_revision,
        )
    };

    let collision_input = empty_collision_input(&quest);
    let regenerated_existing =
        regenerate_revision3_quest_module_v2(&quest, collision_input.clone()).map_err(|error| {
            invalid_closure_owned(
                request.quest_id,
                format!("deterministic regeneration failed: {error}"),
            )
        })?;
    if existing_module != regenerated_existing {
        return Err(Revision3QuestContextEditErrorV1::Conflict(
            Revision3QuestContextEditConflictV1::OwnedModuleDrift {
                quest: request.quest_id,
                module: script_module_id,
            },
        ));
    }
    if request.description == quest.input.description
        && parent == quest.input.parent_quest
        && giver == quest.input.giver
    {
        return Err(Revision3QuestContextEditErrorV1::Conflict(
            Revision3QuestContextEditConflictV1::NoChanges,
        ));
    }

    let mut edited_quest = quest;
    edited_quest.input.description = request.description;
    edited_quest.input.parent_quest = parent;
    edited_quest.input.giver = giver;
    let edited_module = match regenerate_revision3_quest_module_v2(&edited_quest, collision_input) {
        Ok(module) => module,
        Err(Revision3QuestGenerationError::InvalidQuestIntent(error)) => {
            return Err(Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::InvalidQuestContext { error },
            ));
        }
        Err(error) => return Err(Revision3QuestContextEditErrorV1::Generation(error)),
    };
    if !same_technical_module_identity(&existing_module, &edited_module) {
        return Err(Revision3QuestContextEditErrorV1::Conflict(
            Revision3QuestContextEditConflictV1::TechnicalIdentityChanged,
        ));
    }

    let Some(quest_entity) = project.entities.get_mut(&request.quest_id) else {
        unreachable!("Quest was resolved above")
    };
    quest_entity.revision = quest_revision;
    quest_entity.payload = EntityPayload::QuestDraft(edited_quest);
    let Some(module_entity) = project.entities.get_mut(&script_module_id) else {
        unreachable!("owned ScriptModule was resolved above")
    };
    module_entity.revision = script_module_revision;
    module_entity.payload = EntityPayload::ScriptModule(edited_module);
    project.revision = next_project_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(ProjectRevision3JsonError::InputTooLarge { actual, limit }) => {
            return Err(Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::CandidateTooLarge { actual, limit },
            ));
        }
        Err(error) => {
            return Err(Revision3QuestContextEditErrorV1::Conflict(
                Revision3QuestContextEditConflictV1::CandidateNotPersistable {
                    reason: error.to_string(),
                },
            ));
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3QuestContextEditErrorV1::CanonicalReopen)?;
    if reopened != project {
        return Err(Revision3QuestContextEditErrorV1::CanonicalReopenMismatch);
    }

    Ok(Revision3QuestContextEditOutcomeV1 {
        project,
        canonical_project_json,
        basis_head,
        quest_id: request.quest_id,
        script_module_id,
        quest_revision,
        script_module_revision,
        build_status: Revision3QuestContextEditBuildStatusV1::Blocked,
        runtime_status: Revision3QuestContextEditRuntimeStatusV1::RuntimeUnqualified,
        publication_status: Revision3QuestContextEditPublicationStatusV1::NotSupported,
    })
}

fn validate_exact_bindings(
    capability: &VerifiedRevision3QuestCollisionCapabilityV2,
    project: &ProjectRevision3,
    request: &Revision3QuestContextEditRequestV1,
) -> Result<(), Revision3QuestContextEditBindingErrorV1> {
    if &request.expected_head != capability.current_head() {
        return Err(Revision3QuestContextEditBindingErrorV1::CurrentHeadMismatch);
    }
    if request.expected_project_id != project.project_id
        || request.expected_project_id != capability.project_id()
    {
        return Err(Revision3QuestContextEditBindingErrorV1::ProjectIdentityMismatch);
    }
    if request.expected_revision != project.revision
        || request.expected_revision != capability.project_revision()
    {
        return Err(Revision3QuestContextEditBindingErrorV1::ProjectRevisionMismatch);
    }
    if &project.target != capability.project_target() {
        return Err(Revision3QuestContextEditBindingErrorV1::ProjectTargetMismatch);
    }
    if &request.expected_story_catalog_seal != capability.story_catalog_seal() {
        return Err(Revision3QuestContextEditBindingErrorV1::StoryCatalogSealMismatch);
    }
    Ok(())
}

fn validate_quest_identity(
    project: &ProjectRevision3,
    quest_id: EntityId,
    quest: &QuestDraft,
) -> Result<(), Revision3QuestContextEditErrorV1> {
    if quest.input.quest_id != quest_id || quest.input.target != project.target {
        return Err(invalid_closure_owned(
            quest_id,
            "Quest input identity or target differs from the exact project",
        ));
    }
    if quest.script_module.project_id != project.project_id
        || quest.script_module.expected_kind != EntityKind::ScriptModule
        || is_zero_entity_id(quest.script_module.id)
    {
        return Err(invalid_closure_owned(
            quest_id,
            "ScriptModule reference is not exact-project, non-zero, and kind-bound",
        ));
    }
    if quest.input.collision_catalog.generation != project.target {
        return Err(invalid_closure_owned(
            quest_id,
            "collision ArtifactRef generation differs from the exact project target",
        ));
    }
    Ok(())
}

fn invalid_closure(quest: EntityId, reason: &str) -> Revision3QuestContextEditErrorV1 {
    invalid_closure_owned(quest, reason.to_owned())
}

fn invalid_closure_owned(
    quest: EntityId,
    reason: impl Into<String>,
) -> Revision3QuestContextEditErrorV1 {
    Revision3QuestContextEditErrorV1::Conflict(
        Revision3QuestContextEditConflictV1::InvalidQuestClosure {
            quest,
            reason: reason.into(),
        },
    )
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

fn same_technical_module_identity(
    before: &gore_authoring::Revision3ScriptModule,
    after: &gore_authoring::Revision3ScriptModule,
) -> bool {
    before.generator_id == after.generator_id
        && before.generator_version == after.generator_version
        && before.owner == after.owner
        && before.module_namespace == after.module_namespace
        && before.module_relative_path == after.module_relative_path
        && before.status == after.status
}

fn is_zero_entity_id(id: EntityId) -> bool {
    id.as_bytes().iter().all(|byte| *byte == 0)
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
                "revision-3 Quest context-edit request JSON limit exceeded",
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
    use gore_story_catalog::Sha256Digest;

    trait AmbiguousIfClone<Marker> {
        fn marker() {}
    }

    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    impl<T: Clone> AmbiguousIfClone<u8> for T {}

    fn request() -> Revision3QuestContextEditRequestV1 {
        Revision3QuestContextEditRequestV1 {
            expected_head: WorkingHead {
                store_format: WorkingStoreFormat,
                snapshot: AuthoringContentSeal {
                    byte_len: 1,
                    sha256: AuthoringSha256Digest::from_bytes([1; 32]),
                },
            },
            expected_project_id: ProjectId::from_bytes([2; 16]),
            expected_revision: 7,
            expected_story_catalog_seal: ContentSeal {
                byte_len: 3,
                sha256: Sha256Digest::from_bytes([3; 32]),
            },
            quest_id: EntityId::from_bytes([4; 16]),
            expected_quest_revision: 5,
            description: "Edit only catalog-bound context.".to_owned(),
            parent_catalog_id: "g1r:quest-parent:swampcamp_scchapter2".to_owned(),
            giver_catalog_id: "g1r:npc:om_grd_asghan_263".to_owned(),
        }
    }

    #[test]
    fn request_is_exact_canonical_bounded_duplicate_free_and_ordered() {
        let request = request();
        let canonical = request.to_canonical_json().unwrap();
        assert_eq!(
            Revision3QuestContextEditRequestV1::from_json(&canonical).unwrap(),
            request
        );
        let ordered_fields = [
            "\"expected_head\":",
            "\"expected_project_id\":",
            "\"expected_revision\":",
            "\"expected_story_catalog_seal\":",
            "\"quest_id\":",
            "\"expected_quest_revision\":",
            "\"description\":",
            "\"parent_catalog_id\":",
            "\"giver_catalog_id\":",
        ];
        let mut prior = 0;
        for field in ordered_fields {
            let position = canonical.find(field).unwrap();
            assert!(position >= prior, "field {field} is out of canonical order");
            prior = position;
        }
        for forbidden in [
            "\"expected_target\":",
            "\"parent_quest\":",
            "\"giver\":",
            "\"script_module\":",
            "\"collision_catalog\":",
            "\"artifact_seal\":",
        ] {
            assert!(!canonical.contains(forbidden));
        }
        assert!(matches!(
            Revision3QuestContextEditRequestV1::from_json(&(canonical.clone() + "\n")),
            Err(Revision3QuestContextEditRequestJsonErrorV1::NonCanonicalJson)
        ));
        let duplicate = canonical.replacen(
            "{\"expected_head\":",
            "{\"expected_revision\":7,\"expected_head\":",
            1,
        );
        assert!(matches!(
            Revision3QuestContextEditRequestV1::from_json(&duplicate),
            Err(Revision3QuestContextEditRequestJsonErrorV1::InvalidJson(_))
        ));
        assert!(matches!(
            Revision3QuestContextEditRequestV1::from_json(
                &"x".repeat(MAX_REVISION3_QUEST_CONTEXT_EDIT_REQUEST_JSON_BYTES_V1 + 1)
            ),
            Err(Revision3QuestContextEditRequestJsonErrorV1::InputTooLarge { .. })
        ));
    }

    #[test]
    fn outcome_and_authority_input_are_not_cloneable() {
        let _ = <Revision3QuestContextEditOutcomeV1 as AmbiguousIfClone<_>>::marker as fn();
        let _ = <PreparedQuestCollisionArtifactV2 as AmbiguousIfClone<_>>::marker as fn();
    }
}
