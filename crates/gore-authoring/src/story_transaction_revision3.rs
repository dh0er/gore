//! Atomic, filesystem-free insertion of one schema-revision-3 Quest Draft/module pair.
//!
//! This transaction accepts collision entries selected and verified by its caller, but the plain
//! [`QuestCollisionCatalogInput`] type is intentionally not authority evidence. A successful
//! transaction remains an offline, build-blocked draft; source inspection still requires S3 to
//! reopen the exact artifact/basis and bind a fresh collision capability.

use std::io::{self, Write};

use serde::{Deserialize, Serialize};

use crate::model_revision2::GeneratedStoryIdentity;
use crate::model_revision3::{
    quest_collision_artifact_media_for_layer, Entity, EntityKind, EntityPayload, OriginRef,
    QuestDraft, QuestDraftInput, TypedRef,
};
use crate::revision3_quest::regenerate_revision3_quest_module_v2_with_identity;
use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    collect_project_story_collision_identities, project_revision3_quest_free_basis_to_revision2,
    ContentSeal, DraftQuestCollisionKind, DraftQuestSkeletonError, EntityId, GameGenerationAnchor,
    ProjectId, ProjectRevision3, ProjectRevision3JsonError, QuestCollisionArtifactRef,
    QuestCollisionCatalogInput, Revision3QuestFreeBasisError, Revision3QuestGenerationError,
    Revision3QuestGiverInput, Revision3QuestParentInput, Sha256Digest,
    MAX_QUEST_COLLISION_ARTIFACT_BYTES, MAX_REVISION3_ENTITIES, MAX_REVISION3_SNAPSHOT_BYTES,
    REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
};

pub const MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES: usize = 64 * 1024;
pub const MAX_REVISION3_QUEST_DRAFT_DISPLAY_NAME_BYTES: usize = 256;

/// Bounded Quest intent. Target and Quest ID are inherited from the exact request/project pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestDraftIntentV2 {
    pub module_namespace: String,
    pub technical_id: String,
    pub text_helper: String,
    pub parent_quest: Revision3QuestParentInput,
    pub giver: Revision3QuestGiverInput,
    pub title: String,
    pub description: String,
    pub objective_title: String,
    pub collision_catalog: QuestCollisionArtifactRef,
}

/// Exact project-CAS-bound request to insert one revision-3 Quest and owned ScriptModule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3QuestDraftInsertRequestV2 {
    pub expected_project_id: ProjectId,
    pub expected_revision: u64,
    pub expected_target: GameGenerationAnchor,
    pub quest_id: EntityId,
    pub script_module_id: EntityId,
    pub display_name: String,
    pub intent: Revision3QuestDraftIntentV2,
}

impl Revision3QuestDraftInsertRequestV2 {
    /// Parse only bounded, duplicate-free JSON in its exact canonical spelling.
    pub fn from_json(json: &str) -> Result<Self, Revision3QuestDraftInsertRequestJsonErrorV2> {
        if json.len() > MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES {
            return Err(Revision3QuestDraftInsertRequestJsonErrorV2::InputTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES,
            });
        }
        reject_duplicate_object_keys(json)
            .map_err(Revision3QuestDraftInsertRequestJsonErrorV2::InvalidJson)?;
        let request: Self = serde_json::from_str(json)
            .map_err(Revision3QuestDraftInsertRequestJsonErrorV2::InvalidJson)?;
        let canonical = request.to_canonical_json()?;
        if canonical.as_bytes() != json.as_bytes() {
            return Err(Revision3QuestDraftInsertRequestJsonErrorV2::NonCanonicalJson);
        }
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3QuestDraftInsertRequestJsonErrorV2> {
        let mut writer = BoundedRequestWriter::new(MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES);
        let result = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3QuestDraftInsertRequestJsonErrorV2::InputTooLarge {
                actual,
                limit: MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES,
            });
        }
        result.map_err(Revision3QuestDraftInsertRequestJsonErrorV2::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3QuestDraftInsertRequestJsonErrorV2::NonUtf8Serialization)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestDraftInsertRequestJsonErrorV2 {
    #[error("revision-3 Quest request exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid revision-3 Quest request JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize revision-3 Quest request JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 Quest request JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("revision-3 Quest request serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestEntityRoleV2 {
    QuestDraft,
    ScriptModule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3StoryIdentityKindV2 {
    AuthoredRuntimeId,
    ModuleNamespace,
    ModuleRelativePath,
    GeneratedSymbol,
}

/// Stable semantic conflict. A rejection never contains a partially changed project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3QuestDraftInsertConflictV2 {
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
    ZeroEntityId { role: Revision3QuestEntityRoleV2 },
    #[error("Quest Draft and ScriptModule IDs must differ")]
    SharedEntityId,
    #[error("{role:?} entity ID {entity} already exists")]
    EntityIdCollision {
        role: Revision3QuestEntityRoleV2,
        entity: EntityId,
    },
    #[error("display name is empty, contains controls, or exceeds its byte limit")]
    InvalidDisplayName,
    #[error("revision-3 project cannot hold two additional entities")]
    EntityCapacityExceeded,
    #[error("Quest-free basis contains Quest Draft {entity}")]
    RecursiveQuestBasis { entity: EntityId },
    #[error("Quest-free basis contains residual Quest state {entity}")]
    ResidualQuestBasis { entity: EntityId },
    #[error("Quest-free basis has invalid existing Story state: {reason}")]
    InvalidBasisStoryState { reason: String },
    #[error("collision artifact generation does not match the project target")]
    ArtifactGenerationMismatch,
    #[error("collision artifact catalog layer is not the closed revision-3 layer")]
    ArtifactCatalogLayerMismatch,
    #[error("collision artifact raw/semantic seals are invalid")]
    InvalidArtifactSeals,
    #[error("collision artifact basis snapshot seal is invalid")]
    InvalidBasisSnapshot,
    #[error("collision artifact {artifact} is absent from the exact AssetStore")]
    MissingArtifactAsset { artifact: Sha256Digest },
    #[error("collision artifact {artifact} AssetStore metadata does not match its raw seal")]
    ArtifactAssetMetadataMismatch { artifact: Sha256Digest },
    #[error("caller-verified collision input generation differs from its ArtifactRef")]
    CollisionInputGenerationMismatch,
    #[error("caller-verified collision input source seal differs from its ArtifactRef")]
    CollisionInputSourceSealMismatch,
    #[error("caller-verified collision input layer differs from its ArtifactRef")]
    CollisionInputCatalogLayerMismatch,
    #[error("invalid Quest intent: {error}")]
    InvalidQuestIntent { error: DraftQuestSkeletonError },
    #[error("{kind:?} {value:?} collides with existing Story identity")]
    StoryIdentityCollision {
        kind: Revision3StoryIdentityKindV2,
        value: String,
        existing_entity: Option<EntityId>,
    },
    #[error("candidate project is not persistable: {reason}")]
    CandidateNotPersistable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3QuestDraftInsertRejectionV2 {
    pub conflict: Revision3QuestDraftInsertConflictV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestDraftBuildStatusV2 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestDraftRuntimeStatusV2 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestArtifactAuthorityV2 {
    NotGranted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision3QuestSourceInspectionStatusV2 {
    FreshCapabilityRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision3QuestDraftInsertOutcomeV2 {
    pub project: ProjectRevision3,
    pub canonical_project_json: String,
    pub quest_id: EntityId,
    pub script_module_id: EntityId,
    pub build_status: Revision3QuestDraftBuildStatusV2,
    pub runtime_status: Revision3QuestDraftRuntimeStatusV2,
    pub artifact_authority: Revision3QuestArtifactAuthorityV2,
    pub source_inspection: Revision3QuestSourceInspectionStatusV2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision3QuestDraftInsertEvaluationV2 {
    Applied(Box<Revision3QuestDraftInsertOutcomeV2>),
    Rejected(Revision3QuestDraftInsertRejectionV2),
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestDraftInsertErrorV2 {
    #[error("invalid exact canonical revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3JsonError),
    #[error("invalid exact canonical revision-3 Quest request: {0}")]
    InvalidRequest(#[source] Revision3QuestDraftInsertRequestJsonErrorV2),
    #[error("unexpected revision-3 Quest generation failure: {0}")]
    Generation(#[source] Revision3QuestGenerationError),
    #[error("could not reopen generated canonical revision-3 project: {0}")]
    ReopenCandidate(#[source] ProjectRevision3JsonError),
    #[error("canonical revision-3 candidate reopen changed the project")]
    CanonicalReopenMismatch,
}

/// Atomically insert one offline revision-3 Quest Draft and its deterministic ScriptModule.
///
/// Both JSON inputs must be their exact canonical bytes. `collision_input` is consumed and must
/// already have been verified by the caller; matching its retained provenance does not elevate it
/// into artifact authority. The returned project therefore remains build-blocked and requires an
/// S3 source inspection with a fresh capability before its source may be trusted for inspection.
pub fn apply_revision3_quest_draft_transaction_v2(
    canonical_project_json: &str,
    canonical_request_json: &str,
    collision_input: QuestCollisionCatalogInput,
) -> Result<Revision3QuestDraftInsertEvaluationV2, Revision3QuestDraftInsertErrorV2> {
    let mut project = ProjectRevision3::from_json(canonical_project_json)
        .map_err(Revision3QuestDraftInsertErrorV2::InvalidProject)?;
    let request = Revision3QuestDraftInsertRequestV2::from_json(canonical_request_json)
        .map_err(Revision3QuestDraftInsertErrorV2::InvalidRequest)?;

    macro_rules! reject {
        ($conflict:expr) => {
            return Ok(Revision3QuestDraftInsertEvaluationV2::Rejected(
                Revision3QuestDraftInsertRejectionV2 {
                    conflict: $conflict,
                },
            ))
        };
    }

    if request.expected_project_id != project.project_id {
        reject!(
            Revision3QuestDraftInsertConflictV2::ProjectIdentityMismatch {
                expected: request.expected_project_id,
                actual: project.project_id,
            }
        );
    }
    if request.expected_revision != project.revision {
        reject!(
            Revision3QuestDraftInsertConflictV2::ProjectRevisionConflict {
                expected: request.expected_revision,
                actual: project.revision,
            }
        );
    }
    if request.expected_target != project.target {
        reject!(Revision3QuestDraftInsertConflictV2::ProjectTargetMismatch);
    }
    let Some(next_revision) = project.revision.checked_add(1) else {
        reject!(Revision3QuestDraftInsertConflictV2::ProjectRevisionOverflow);
    };
    for (role, id) in [
        (Revision3QuestEntityRoleV2::QuestDraft, request.quest_id),
        (
            Revision3QuestEntityRoleV2::ScriptModule,
            request.script_module_id,
        ),
    ] {
        if is_zero_entity_id(id) {
            reject!(Revision3QuestDraftInsertConflictV2::ZeroEntityId { role });
        }
        if project.entities.contains_key(&id) {
            reject!(Revision3QuestDraftInsertConflictV2::EntityIdCollision { role, entity: id });
        }
    }
    if request.quest_id == request.script_module_id {
        reject!(Revision3QuestDraftInsertConflictV2::SharedEntityId);
    }
    if request.display_name.trim().is_empty()
        || request.display_name.len() > MAX_REVISION3_QUEST_DRAFT_DISPLAY_NAME_BYTES
        || request.display_name.chars().any(char::is_control)
    {
        reject!(Revision3QuestDraftInsertConflictV2::InvalidDisplayName);
    }
    if project
        .entities
        .len()
        .checked_add(2)
        .is_none_or(|count| count > MAX_REVISION3_ENTITIES)
    {
        reject!(Revision3QuestDraftInsertConflictV2::EntityCapacityExceeded);
    }

    let revision2_basis = match project_revision3_quest_free_basis_to_revision2(&project) {
        Ok(basis) => basis,
        Err(Revision3QuestFreeBasisError::InvalidProject { reason }) => {
            reject!(Revision3QuestDraftInsertConflictV2::InvalidBasisStoryState { reason });
        }
        Err(Revision3QuestFreeBasisError::RecursiveQuest { entity }) => {
            reject!(Revision3QuestDraftInsertConflictV2::RecursiveQuestBasis { entity });
        }
        Err(Revision3QuestFreeBasisError::ResidualQuestState { entity }) => {
            reject!(Revision3QuestDraftInsertConflictV2::ResidualQuestBasis { entity });
        }
    };
    let existing_story = match collect_project_story_collision_identities(&revision2_basis) {
        Ok(identities) => identities,
        Err(error) => {
            reject!(
                Revision3QuestDraftInsertConflictV2::InvalidBasisStoryState {
                    reason: error.to_string(),
                }
            );
        }
    };

    let reference = &request.intent.collision_catalog;
    if reference.generation != project.target {
        reject!(Revision3QuestDraftInsertConflictV2::ArtifactGenerationMismatch);
    }
    let Some(expected_media_type) =
        quest_collision_artifact_media_for_layer(&reference.catalog_layer)
    else {
        reject!(Revision3QuestDraftInsertConflictV2::ArtifactCatalogLayerMismatch);
    };
    if !valid_artifact_seals(&reference.artifact, &reference.source_seal) {
        reject!(Revision3QuestDraftInsertConflictV2::InvalidArtifactSeals);
    }
    if !valid_basis_snapshot(&reference.basis_snapshot) {
        reject!(Revision3QuestDraftInsertConflictV2::InvalidBasisSnapshot);
    }
    let Some(asset_meta) = project.asset_store.assets.get(&reference.artifact.sha256) else {
        reject!(Revision3QuestDraftInsertConflictV2::MissingArtifactAsset {
            artifact: reference.artifact.sha256,
        });
    };
    if asset_meta.byte_len != reference.artifact.byte_len
        || asset_meta.media_type != expected_media_type
    {
        reject!(
            Revision3QuestDraftInsertConflictV2::ArtifactAssetMetadataMismatch {
                artifact: reference.artifact.sha256,
            }
        );
    }
    if collision_input.generation != reference.generation {
        reject!(Revision3QuestDraftInsertConflictV2::CollisionInputGenerationMismatch);
    }
    if collision_input.source_seal != reference.source_seal {
        reject!(Revision3QuestDraftInsertConflictV2::CollisionInputSourceSealMismatch);
    }
    if collision_input.catalog_layer != reference.catalog_layer {
        reject!(Revision3QuestDraftInsertConflictV2::CollisionInputCatalogLayerMismatch);
    }

    let owner = TypedRef::new(project.project_id, request.quest_id, EntityKind::QuestDraft);
    let module_ref = TypedRef::new(
        project.project_id,
        request.script_module_id,
        EntityKind::ScriptModule,
    );
    let quest = QuestDraft {
        generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
        generator_version: REVISION3_QUEST_GENERATOR_VERSION,
        input: QuestDraftInput {
            target: project.target.clone(),
            quest_id: request.quest_id,
            module_namespace: request.intent.module_namespace,
            technical_id: request.intent.technical_id,
            text_helper: request.intent.text_helper,
            parent_quest: request.intent.parent_quest,
            giver: request.intent.giver,
            title: request.intent.title,
            description: request.intent.description,
            objective_title: request.intent.objective_title,
            additional_objective_titles: Vec::new(),
            transition_plan: None,
            collision_catalog: request.intent.collision_catalog,
        },
        script_module: module_ref,
        transcript: Vec::new(),
    };
    let runtime_id = quest.input.technical_id.clone();
    let (module, generated_identity) =
        match regenerate_revision3_quest_module_v2_with_identity(&quest, collision_input) {
            Ok(generated) => generated,
            Err(Revision3QuestGenerationError::InvalidQuestIntent(error)) => {
                if let Some(conflict) = generator_collision_conflict(&error) {
                    reject!(conflict);
                }
                reject!(Revision3QuestDraftInsertConflictV2::InvalidQuestIntent { error });
            }
            Err(error) => return Err(Revision3QuestDraftInsertErrorV2::Generation(error)),
        };

    if let Some(existing_entity) = find_runtime_identity(&project, &runtime_id) {
        reject!(
            Revision3QuestDraftInsertConflictV2::StoryIdentityCollision {
                kind: Revision3StoryIdentityKindV2::AuthoredRuntimeId,
                value: runtime_id,
                existing_entity: Some(existing_entity),
            }
        );
    }
    if let Some(conflict) = find_generated_identity_collision(&generated_identity, &existing_story)
    {
        reject!(conflict);
    }

    let quest_entity = Entity {
        id: request.quest_id,
        display_name: request.display_name,
        origin: OriginRef::New {
            authored_runtime_id: runtime_id,
        },
        revision: 0,
        payload: EntityPayload::QuestDraft(quest),
    };
    let module_entity = Entity {
        id: request.script_module_id,
        display_name: module.module_namespace.clone(),
        origin: OriginRef::Generated {
            generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            owner,
        },
        revision: 0,
        payload: EntityPayload::ScriptModule(module),
    };
    let replaced_quest = project.entities.insert(request.quest_id, quest_entity);
    let replaced_module = project
        .entities
        .insert(request.script_module_id, module_entity);
    debug_assert!(replaced_quest.is_none());
    debug_assert!(replaced_module.is_none());
    project.revision = next_revision;

    let canonical_project_json = match project.to_canonical_json() {
        Ok(json) => json,
        Err(error) => {
            reject!(
                Revision3QuestDraftInsertConflictV2::CandidateNotPersistable {
                    reason: error.to_string(),
                }
            );
        }
    };
    let reopened = ProjectRevision3::from_json(&canonical_project_json)
        .map_err(Revision3QuestDraftInsertErrorV2::ReopenCandidate)?;
    if reopened != project {
        return Err(Revision3QuestDraftInsertErrorV2::CanonicalReopenMismatch);
    }

    Ok(Revision3QuestDraftInsertEvaluationV2::Applied(Box::new(
        Revision3QuestDraftInsertOutcomeV2 {
            project,
            canonical_project_json,
            quest_id: request.quest_id,
            script_module_id: request.script_module_id,
            build_status: Revision3QuestDraftBuildStatusV2::Blocked,
            runtime_status: Revision3QuestDraftRuntimeStatusV2::RuntimeUnqualified,
            artifact_authority: Revision3QuestArtifactAuthorityV2::NotGranted,
            source_inspection: Revision3QuestSourceInspectionStatusV2::FreshCapabilityRequired,
        },
    )))
}

fn is_zero_entity_id(id: EntityId) -> bool {
    id.as_bytes().iter().all(|byte| *byte == 0)
}

fn is_zero_digest(digest: Sha256Digest) -> bool {
    digest.as_bytes().iter().all(|byte| *byte == 0)
}

fn valid_artifact_seals(raw: &ContentSeal, semantic: &ContentSeal) -> bool {
    raw.byte_len != 0
        && raw.byte_len <= MAX_QUEST_COLLISION_ARTIFACT_BYTES
        && semantic.byte_len == raw.byte_len
        && !is_zero_digest(raw.sha256)
        && !is_zero_digest(semantic.sha256)
}

fn valid_basis_snapshot(snapshot: &ContentSeal) -> bool {
    snapshot.byte_len != 0
        && snapshot.byte_len <= MAX_REVISION3_SNAPSHOT_BYTES
        && !is_zero_digest(snapshot.sha256)
}

fn find_runtime_identity(project: &ProjectRevision3, candidate: &str) -> Option<EntityId> {
    project.entities.iter().find_map(|(id, entity)| {
        if !matches!(entity.payload, EntityPayload::NpcDraft(_)) {
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

fn find_generated_identity_collision(
    generated: &GeneratedStoryIdentity,
    existing: &crate::ProjectStoryCollisionIdentities,
) -> Option<Revision3QuestDraftInsertConflictV2> {
    let checks = [
        (
            Revision3StoryIdentityKindV2::ModuleNamespace,
            generated.module_namespace.as_str(),
            existing.modules(),
        ),
        (
            Revision3StoryIdentityKindV2::ModuleRelativePath,
            generated.module_relative_path.as_str(),
            existing.relative_paths(),
        ),
    ];
    for (kind, value, values) in checks {
        if let Some(existing_entity) = values.get(&value.to_ascii_lowercase()) {
            return Some(
                Revision3QuestDraftInsertConflictV2::StoryIdentityCollision {
                    kind,
                    value: value.to_owned(),
                    existing_entity: Some(*existing_entity),
                },
            );
        }
    }
    for symbol in &generated.symbols {
        if let Some(existing_entity) = existing.symbols().get(&symbol.to_ascii_lowercase()) {
            return Some(
                Revision3QuestDraftInsertConflictV2::StoryIdentityCollision {
                    kind: Revision3StoryIdentityKindV2::GeneratedSymbol,
                    value: symbol.clone(),
                    existing_entity: Some(*existing_entity),
                },
            );
        }
    }
    None
}

fn generator_collision_conflict(
    error: &DraftQuestSkeletonError,
) -> Option<Revision3QuestDraftInsertConflictV2> {
    let DraftQuestSkeletonError::GeneratedNameCollision { kind, name } = error else {
        return None;
    };
    let kind = match kind {
        DraftQuestCollisionKind::Module => Revision3StoryIdentityKindV2::ModuleNamespace,
        DraftQuestCollisionKind::RelativePath => Revision3StoryIdentityKindV2::ModuleRelativePath,
        DraftQuestCollisionKind::Symbol => Revision3StoryIdentityKindV2::GeneratedSymbol,
    };
    Some(
        Revision3QuestDraftInsertConflictV2::StoryIdentityCollision {
            kind,
            value: name.clone(),
            existing_entity: None,
        },
    )
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
                "revision-3 Quest request JSON limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
