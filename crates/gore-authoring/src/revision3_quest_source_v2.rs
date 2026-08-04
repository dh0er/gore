//! Exact-current-project collision source preparation for multi-Quest revision-3 authoring.
//!
//! Persisted Quest artifacts and their historical `basis_snapshot` fields are deliberately not
//! authority inputs here. The only source is one fully reconstituted current revision-3 project.
//! Every prior Quest/module pair is regenerated collision-independently, removed from an exact
//! native non-Quest basis, and then recomposed to prove that the split lost no current state.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Write};

use serde::Serialize;
use sha2::{Digest as _, Sha256};

use crate::model_revision3::{
    is_quest_collision_artifact_media_type, quest_collision_artifact_media_for_layer, Entity,
    EntityKind, EntityPayload, OriginRef, QuestDraft, QuestGiverInput, QuestParentInput,
    ScriptModuleStatus, TypedRef,
};
use crate::revision3_quest::regenerate_revision3_quest_module_with_identity;
use crate::story_collision::{
    collect_revision3_story_collision_identities_bounded, BoundedStoryCollisionCollectionError,
    StoryCollisionCollectionLimits,
};
use crate::QuestCollisionCatalogInput;
use crate::{
    ContentSeal, EntityId, GameGenerationAnchor, ProjectId, ProjectRevision3,
    ProjectStoryCollisionIdentities, Sha256Digest, WorkingHead, WorkingStoreError,
    MAX_PROJECT_JSON_BYTES, REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
};

pub const MAX_REVISION3_PRIOR_QUESTS_V2: usize = 14_285;
pub const MAX_REVISION3_COLLISION_IDENTITIES_V2: usize = 100_000;
pub const MAX_REVISION3_COLLISION_IDENTITY_BYTES_V2: usize = 16 * 1024 * 1024;
pub const MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2: usize = 512;

const PRIOR_QUEST_EVIDENCE_DOMAIN_V2: &[u8] =
    b"gore-authoring.revision3-current-quest-source-v2.prior-evidence\0";

/// One bounded, deterministically regenerated prior-Quest identity record.
///
/// Records remain internal evidence carried by [`PreparedRevision3QuestCollisionSourceV2`]. A
/// future artifact may retain only their count and semantic seal. This type intentionally has no
/// deserializer and cannot be used to manufacture a prepared source capsule.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3PriorQuestEvidenceV2 {
    quest_id: EntityId,
    module_id: EntityId,
    input_fingerprint: Sha256Digest,
    source_sha256: Sha256Digest,
    module_namespace: String,
    module_relative_path: String,
    symbols: Vec<String>,
    parent: QuestParentInput,
    giver: QuestGiverInput,
}

impl Revision3PriorQuestEvidenceV2 {
    pub const fn quest_id(&self) -> EntityId {
        self.quest_id
    }

    pub const fn module_id(&self) -> EntityId {
        self.module_id
    }

    pub const fn input_fingerprint(&self) -> Sha256Digest {
        self.input_fingerprint
    }

    pub const fn source_sha256(&self) -> Sha256Digest {
        self.source_sha256
    }

    pub fn module_namespace(&self) -> &str {
        &self.module_namespace
    }

    pub fn module_relative_path(&self) -> &str {
        &self.module_relative_path
    }

    pub fn symbols(&self) -> &[String] {
        &self.symbols
    }

    pub fn parent(&self) -> &QuestParentInput {
        &self.parent
    }

    pub fn giver(&self) -> &QuestGiverInput {
        &self.giver
    }
}

/// Exact native revision-3 Quest-free collision basis derived from one current revision-3 project.
///
/// Its fields are private and the type is not `Clone`: downstream consumers can inspect it or
/// consume it, but cannot construct or casually duplicate this source-bound evidence wrapper.
pub struct Revision3NonQuestCollisionBasisV2 {
    project: ProjectRevision3,
    canonical_project: ContentSeal,
    story_identities: ProjectStoryCollisionIdentities,
}

impl fmt::Debug for Revision3NonQuestCollisionBasisV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Revision3NonQuestCollisionBasisV2")
            .field("project_id", &self.project.project_id)
            .field("project_revision", &self.project.revision)
            .field("canonical_project", &self.canonical_project)
            .field("module_identities", &self.story_identities.modules().len())
            .field(
                "relative_path_identities",
                &self.story_identities.relative_paths().len(),
            )
            .field("symbol_identities", &self.story_identities.symbols().len())
            .finish()
    }
}

impl Revision3NonQuestCollisionBasisV2 {
    pub fn project(&self) -> &ProjectRevision3 {
        &self.project
    }

    pub fn canonical_project(&self) -> &ContentSeal {
        &self.canonical_project
    }

    pub fn canonical_project_seal(&self) -> &ContentSeal {
        &self.canonical_project
    }

    pub fn story_identities(&self) -> &ProjectStoryCollisionIdentities {
        &self.story_identities
    }
}

/// Opaque source capsule bound to the exact currently published revision-3 head.
///
/// It is constructible only by [`crate::WorkingProjectStore`] after current snapshot/entity and
/// non-Quest asset verification. It has no `Clone`, serialization, or public constructor, and
/// persisted Quest artifacts are never accepted as authority while preparing it.
pub struct PreparedRevision3QuestCollisionSourceV2 {
    current_head: WorkingHead,
    current_project: ContentSeal,
    nonquest_basis: Revision3NonQuestCollisionBasisV2,
    prior_quest_evidence: ContentSeal,
    prior_quests: BTreeMap<EntityId, Revision3PriorQuestEvidenceV2>,
}

/// Opaque, inspection-only collision evidence reconstructed from one immutable historical head.
///
/// This deliberately has a distinct type from [`PreparedRevision3QuestCollisionSourceV2`]. It
/// cannot be converted into exact-current authoring authority, and is accepted only by the
/// inventory crate's linear inspection verifier. The retained source remains fully sealed and
/// structurally identical to the source that originally produced a version-2 artifact.
pub struct PreparedRevision3QuestCollisionInspectionSourceV2 {
    source: PreparedRevision3QuestCollisionSourceV2,
}

impl fmt::Debug for PreparedRevision3QuestCollisionInspectionSourceV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedRevision3QuestCollisionInspectionSourceV2")
            .field("historical_head", self.source.current_head())
            .field("project_id", &self.source.project_id())
            .field("project_revision", &self.source.project_revision())
            .field("prior_quest_count", &self.source.prior_quest_count())
            .finish_non_exhaustive()
    }
}

impl PreparedRevision3QuestCollisionInspectionSourceV2 {
    pub(crate) fn new(source: PreparedRevision3QuestCollisionSourceV2) -> Self {
        Self { source }
    }

    pub fn historical_head(&self) -> &WorkingHead {
        self.source.current_head()
    }

    pub fn historical_project(&self) -> &ContentSeal {
        self.source.current_project()
    }

    pub fn project_id(&self) -> ProjectId {
        self.source.project_id()
    }

    pub fn project_revision(&self) -> u64 {
        self.source.project_revision()
    }

    pub fn target(&self) -> &GameGenerationAnchor {
        self.source.target()
    }

    pub fn nonquest_basis(&self) -> &Revision3NonQuestCollisionBasisV2 {
        self.source.nonquest_basis()
    }

    pub fn prior_quest_count(&self) -> usize {
        self.source.prior_quest_count()
    }

    pub fn prior_quest_count_u64(&self) -> u64 {
        self.source.prior_quest_count_u64()
    }

    pub fn prior_quest_evidence(&self) -> &ContentSeal {
        self.source.prior_quest_evidence()
    }

    pub fn prior_quests(&self) -> &BTreeMap<EntityId, Revision3PriorQuestEvidenceV2> {
        self.source.prior_quests()
    }
}

impl fmt::Debug for PreparedRevision3QuestCollisionSourceV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedRevision3QuestCollisionSourceV2")
            .field("current_head", &self.current_head)
            .field("current_project", &self.current_project)
            .field("nonquest_basis", &self.nonquest_basis)
            .field("prior_quest_count", &self.prior_quests.len())
            .field("prior_quest_evidence", &self.prior_quest_evidence)
            .finish()
    }
}

impl PreparedRevision3QuestCollisionSourceV2 {
    pub fn current_head(&self) -> &WorkingHead {
        &self.current_head
    }

    pub fn current_snapshot(&self) -> &ContentSeal {
        &self.current_head.snapshot
    }

    pub fn current_project(&self) -> &ContentSeal {
        &self.current_project
    }

    pub fn current_project_seal(&self) -> &ContentSeal {
        &self.current_project
    }

    pub fn project_id(&self) -> ProjectId {
        self.nonquest_basis.project.project_id
    }

    pub fn project_revision(&self) -> u64 {
        self.nonquest_basis.project.revision
    }

    pub fn target(&self) -> &GameGenerationAnchor {
        &self.nonquest_basis.project.target
    }

    pub fn nonquest_basis(&self) -> &Revision3NonQuestCollisionBasisV2 {
        &self.nonquest_basis
    }

    pub fn prior_quest_count(&self) -> usize {
        self.prior_quests.len()
    }

    pub fn prior_quest_count_u64(&self) -> u64 {
        self.prior_quests.len() as u64
    }

    pub fn prior_quest_evidence(&self) -> &ContentSeal {
        &self.prior_quest_evidence
    }

    pub fn prior_quest_evidence_seal(&self) -> &ContentSeal {
        &self.prior_quest_evidence
    }

    pub fn prior_quests(&self) -> &BTreeMap<EntityId, Revision3PriorQuestEvidenceV2> {
        &self.prior_quests
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestCollisionSourceErrorV2 {
    #[error(transparent)]
    Store(#[from] WorkingStoreError),
    #[error("current revision-3 snapshot differs from its exact published head")]
    CurrentSnapshotDrift,
    #[error("invalid exact current revision-3 project: {reason}")]
    InvalidCurrentProject { reason: String },
    #[error("current revision-3 project has {actual} prior Quests; maximum is {limit}")]
    TooManyPriorQuests { actual: usize, limit: usize },
    #[error(
        "revision-3 Quest module {module} is shared by prior Quests {first_owner} and {second_owner}"
    )]
    SharedQuestModule {
        module: EntityId,
        first_owner: EntityId,
        second_owner: EntityId,
    },
    #[error("revision-3 entity {entity} contains residual or orphan Quest generation state")]
    ResidualQuestState { entity: EntityId },
    #[error(
        "historical Quest artifact {artifact} is cross-referenced by non-Quest entity {entity}"
    )]
    HistoricalQuestArtifactCrossReference {
        artifact: Sha256Digest,
        entity: EntityId,
    },
    #[error("revision-3 Quest {quest} has a missing, foreign, or mistyped module owner binding")]
    QuestOwnerDrift { quest: EntityId },
    #[error("revision-3 Quest {quest} module {module} has drifted generated origin metadata")]
    ModuleOriginDrift { quest: EntityId, module: EntityId },
    #[error("revision-3 Quest {quest} or module {module} has a foreign generator contract")]
    ForeignGenerator { quest: EntityId, module: EntityId },
    #[error("revision-3 Quest {quest} module {module} has a mismatched persisted source hash")]
    SourceHashMismatch { quest: EntityId, module: EntityId },
    #[error("revision-3 Quest {quest} module {module} has a mismatched input fingerprint")]
    InputFingerprintMismatch { quest: EntityId, module: EntityId },
    #[error("revision-3 Quest {quest} module {module} differs from exact regeneration: {reason}")]
    PersistedModuleDrift {
        quest: EntityId,
        module: EntityId,
        reason: String,
    },
    #[error(
        "authored runtime identity {value:?} collides between entities {first_owner} and {second_owner}"
    )]
    DuplicateRuntimeId {
        value: String,
        first_owner: EntityId,
        second_owner: EntityId,
    },
    #[error(
        "prior/current Story {kind} identity {value:?} collides between {first_owner} and {second_owner}"
    )]
    PriorIdentityCollision {
        kind: &'static str,
        value: String,
        first_owner: EntityId,
        second_owner: EntityId,
    },
    #[error("invalid exact native non-Quest basis: {reason}")]
    NonQuestBasisInvalid { reason: String },
    #[error("revision-3 current-project source limit exceeded for {kind}: {actual} > {limit}")]
    Limit {
        kind: &'static str,
        actual: usize,
        limit: usize,
    },
}

pub(crate) fn prepare_exact_revision3_quest_collision_source_v2(
    project: &ProjectRevision3,
    current_head: WorkingHead,
) -> Result<PreparedRevision3QuestCollisionSourceV2, Revision3QuestCollisionSourceErrorV2> {
    let prior_count = project
        .entities
        .values()
        .filter(|entity| matches!(entity.payload, EntityPayload::QuestDraft(_)))
        .count();
    if prior_count > MAX_REVISION3_PRIOR_QUESTS_V2 {
        return Err(Revision3QuestCollisionSourceErrorV2::TooManyPriorQuests {
            actual: prior_count,
            limit: MAX_REVISION3_PRIOR_QUESTS_V2,
        });
    }

    // Serialize first without invoking closed-model validation so targeted pair/regeneration
    // errors below are not collapsed into one generic model error. BTree-backed project fields
    // make these bytes deterministic; after targeted validation we require the public canonical
    // serializer to emit the exact same bytes.
    let canonical_current = bounded_json_bytes(
        project,
        MAX_PROJECT_JSON_BYTES,
        "canonical current project bytes",
    )?;
    let current_project = raw_seal(&canonical_current);

    let mut claimed_modules = BTreeMap::<EntityId, EntityId>::new();
    let mut removed_entities = BTreeMap::<EntityId, Entity>::new();
    let mut prior_quests = BTreeMap::new();

    for (quest_id, entity) in &project.entities {
        let EntityPayload::QuestDraft(quest) = &entity.payload else {
            continue;
        };
        let module_id = quest.script_module.id;
        if let Some(first_owner) = claimed_modules.insert(module_id, *quest_id) {
            return Err(Revision3QuestCollisionSourceErrorV2::SharedQuestModule {
                module: module_id,
                first_owner,
                second_owner: *quest_id,
            });
        }
        let module_entity = validate_quest_pair(project, *quest_id, entity, quest)?;
        let EntityPayload::ScriptModule(module) = &module_entity.payload else {
            unreachable!("validate_quest_pair returns only ScriptModule entities")
        };

        let empty_collision = QuestCollisionCatalogInput {
            generation: quest.input.collision_catalog.generation.clone(),
            source_seal: quest.input.collision_catalog.source_seal.clone(),
            catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
            modules: BTreeSet::new(),
            relative_paths: BTreeSet::new(),
            symbols: BTreeSet::new(),
        };
        let (regenerated, identity) =
            regenerate_revision3_quest_module_with_identity(quest, empty_collision).map_err(
                |error| Revision3QuestCollisionSourceErrorV2::PersistedModuleDrift {
                    quest: *quest_id,
                    module: module_id,
                    reason: error.to_string(),
                },
            )?;
        if module.source_sha256 != digest_bytes(module.source.as_bytes()) {
            return Err(Revision3QuestCollisionSourceErrorV2::SourceHashMismatch {
                quest: *quest_id,
                module: module_id,
            });
        }
        if module.input_fingerprint != regenerated.input_fingerprint {
            return Err(
                Revision3QuestCollisionSourceErrorV2::InputFingerprintMismatch {
                    quest: *quest_id,
                    module: module_id,
                },
            );
        }
        if module != &regenerated {
            return Err(Revision3QuestCollisionSourceErrorV2::PersistedModuleDrift {
                quest: *quest_id,
                module: module_id,
                reason:
                    "persisted module is not byte-for-value equal to empty-collision regeneration"
                        .to_owned(),
            });
        }

        let symbols = identity.symbols;
        let record = Revision3PriorQuestEvidenceV2 {
            quest_id: *quest_id,
            module_id,
            input_fingerprint: regenerated.input_fingerprint,
            source_sha256: regenerated.source_sha256,
            module_namespace: identity.module_namespace,
            module_relative_path: identity.module_relative_path,
            symbols,
            parent: quest.input.parent_quest.clone(),
            giver: quest.input.giver.clone(),
        };
        removed_entities.insert(*quest_id, entity.clone());
        removed_entities.insert(module_id, module_entity.clone());
        prior_quests.insert(*quest_id, record);
    }

    reject_residual_quest_state(project, &removed_entities)?;
    validate_runtime_identities(project)?;
    project.validate_closed_model().map_err(|error| {
        Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject {
            reason: error.to_string(),
        }
    })?;
    let public_canonical = project.to_canonical_json().map_err(|error| {
        Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject {
            reason: error.to_string(),
        }
    })?;
    if public_canonical.as_bytes() != canonical_current {
        return Err(
            Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject {
                reason: "current project canonical serialization drifted".to_owned(),
            },
        );
    }

    let mut nonquest_revision3 = project.clone();
    for id in removed_entities.keys() {
        nonquest_revision3.entities.remove(id);
    }
    let physically_referenced = physical_asset_references(&nonquest_revision3);
    let historical_artifacts = nonquest_revision3
        .asset_store
        .assets
        .iter()
        .filter_map(|(digest, meta)| {
            is_quest_collision_artifact_media_type(&meta.media_type).then_some(*digest)
        })
        .collect::<Vec<_>>();
    let mut removed_assets = BTreeMap::new();
    for digest in historical_artifacts {
        if let Some(entity) = physically_referenced.get(&digest) {
            return Err(
                Revision3QuestCollisionSourceErrorV2::HistoricalQuestArtifactCrossReference {
                    artifact: digest,
                    entity: *entity,
                },
            );
        }
        if let Some(meta) = nonquest_revision3.asset_store.assets.remove(&digest) {
            removed_assets.insert(digest, meta);
        }
    }

    let nonquest_project = nonquest_revision3;
    let mut recomposed = nonquest_project.clone();
    for (id, entity) in removed_entities {
        if recomposed.entities.insert(id, entity).is_some() {
            return Err(Revision3QuestCollisionSourceErrorV2::NonQuestBasisInvalid {
                reason: format!("recomposition collided at entity {id}"),
            });
        }
    }
    for (digest, meta) in removed_assets {
        if recomposed.asset_store.assets.insert(digest, meta).is_some() {
            return Err(Revision3QuestCollisionSourceErrorV2::NonQuestBasisInvalid {
                reason: format!("recomposition collided at asset {digest}"),
            });
        }
    }
    if recomposed != *project {
        return Err(Revision3QuestCollisionSourceErrorV2::NonQuestBasisInvalid {
            reason: "exact current project did not survive split/recomposition".to_owned(),
        });
    }

    let story_identities = collect_revision3_story_collision_identities_bounded(
        &nonquest_project,
        StoryCollisionCollectionLimits {
            max_count: MAX_REVISION3_COLLISION_IDENTITIES_V2,
            max_bytes: MAX_REVISION3_COLLISION_IDENTITY_BYTES_V2,
            max_value_bytes: MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2,
        },
    )
    .map_err(|error| match error {
        BoundedStoryCollisionCollectionError::ResourceLimit {
            kind,
            actual,
            limit,
        } => Revision3QuestCollisionSourceErrorV2::Limit {
            kind,
            actual,
            limit,
        },
        BoundedStoryCollisionCollectionError::Collection(other) => {
            Revision3QuestCollisionSourceErrorV2::NonQuestBasisInvalid {
                reason: other.to_string(),
            }
        }
    })?;
    validate_union_identities(&story_identities, &prior_quests)?;

    let canonical_nonquest = nonquest_project.to_canonical_json().map_err(|error| {
        Revision3QuestCollisionSourceErrorV2::NonQuestBasisInvalid {
            reason: error.to_string(),
        }
    })?;
    let canonical_project = raw_seal(canonical_nonquest.as_bytes());
    if story_identities.canonical_project() != &canonical_project {
        return Err(Revision3QuestCollisionSourceErrorV2::NonQuestBasisInvalid {
            reason: "non-Quest collision collector bound a different canonical project".to_owned(),
        });
    }
    let prior_quest_evidence = seal_prior_evidence(&prior_quests)?;

    Ok(PreparedRevision3QuestCollisionSourceV2 {
        current_head,
        current_project,
        nonquest_basis: Revision3NonQuestCollisionBasisV2 {
            project: nonquest_project,
            canonical_project,
            story_identities,
        },
        prior_quest_evidence,
        prior_quests,
    })
}

fn validate_quest_pair<'a>(
    project: &'a ProjectRevision3,
    quest_id: EntityId,
    quest_entity: &Entity,
    quest: &QuestDraft,
) -> Result<&'a Entity, Revision3QuestCollisionSourceErrorV2> {
    let module_id = quest.script_module.id;
    if quest.generator_id != REVISION3_QUEST_GENERATOR_ID
        || quest.generator_version != REVISION3_QUEST_GENERATOR_VERSION
    {
        return Err(Revision3QuestCollisionSourceErrorV2::ForeignGenerator {
            quest: quest_id,
            module: module_id,
        });
    }
    if quest.input.quest_id != quest_id
        || quest.script_module.project_id != project.project_id
        || quest.script_module.expected_kind != EntityKind::ScriptModule
    {
        return Err(Revision3QuestCollisionSourceErrorV2::QuestOwnerDrift { quest: quest_id });
    }
    if !matches!(
        &quest_entity.origin,
        OriginRef::New { authored_runtime_id }
            if authored_runtime_id == &quest.input.technical_id
    ) {
        return Err(Revision3QuestCollisionSourceErrorV2::QuestOwnerDrift { quest: quest_id });
    }
    let Some(module_entity) = project.entities.get(&module_id) else {
        return Err(Revision3QuestCollisionSourceErrorV2::QuestOwnerDrift { quest: quest_id });
    };
    let EntityPayload::ScriptModule(module) = &module_entity.payload else {
        return Err(Revision3QuestCollisionSourceErrorV2::QuestOwnerDrift { quest: quest_id });
    };
    let expected_owner = TypedRef::new(project.project_id, quest_id, EntityKind::QuestDraft);
    if module.owner != expected_owner {
        return Err(Revision3QuestCollisionSourceErrorV2::QuestOwnerDrift { quest: quest_id });
    }
    if module.generator_id != REVISION3_QUEST_GENERATOR_ID
        || module.generator_version != quest.generator_version
    {
        return Err(Revision3QuestCollisionSourceErrorV2::ForeignGenerator {
            quest: quest_id,
            module: module_id,
        });
    }
    if module.status != ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
        || !matches!(
            &module_entity.origin,
            OriginRef::Generated {
                generator_id,
                generator_version,
                owner,
            } if generator_id == REVISION3_QUEST_GENERATOR_ID
                && *generator_version == quest.generator_version
                && owner == &expected_owner
        )
    {
        return Err(Revision3QuestCollisionSourceErrorV2::ModuleOriginDrift {
            quest: quest_id,
            module: module_id,
        });
    }
    let reference = &quest.input.collision_catalog;
    let Some(expected_media) = quest_collision_artifact_media_for_layer(&reference.catalog_layer)
    else {
        return Err(
            Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject {
                reason: format!(
                    "revision-3 Quest {quest_id} has an unknown collision catalog layer"
                ),
            },
        );
    };
    let Some(meta) = project.asset_store.assets.get(&reference.artifact.sha256) else {
        return Err(
            Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject {
                reason: format!("revision-3 Quest {quest_id} collision artifact is not indexed"),
            },
        );
    };
    if meta.byte_len != reference.artifact.byte_len || meta.media_type != expected_media {
        return Err(
            Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject {
                reason: format!(
                    "revision-3 Quest {quest_id} collision layer/media/length pairing drifted"
                ),
            },
        );
    }
    Ok(module_entity)
}

fn reject_residual_quest_state(
    project: &ProjectRevision3,
    removed: &BTreeMap<EntityId, Entity>,
) -> Result<(), Revision3QuestCollisionSourceErrorV2> {
    for (id, entity) in &project.entities {
        if removed.contains_key(id) {
            continue;
        }
        let payload_marks_quest = matches!(
            &entity.payload,
            EntityPayload::ScriptModule(module)
                if module.owner.expected_kind == EntityKind::QuestDraft
                    || module.generator_id == REVISION3_QUEST_GENERATOR_ID
        );
        let origin_marks_quest = matches!(
            &entity.origin,
            OriginRef::Generated {
                generator_id,
                owner,
                ..
            } if generator_id == REVISION3_QUEST_GENERATOR_ID
                || owner.expected_kind == EntityKind::QuestDraft
        );
        if payload_marks_quest || origin_marks_quest {
            return Err(Revision3QuestCollisionSourceErrorV2::ResidualQuestState { entity: *id });
        }
    }
    Ok(())
}

fn validate_runtime_identities(
    project: &ProjectRevision3,
) -> Result<(), Revision3QuestCollisionSourceErrorV2> {
    let mut seen = BTreeMap::<String, EntityId>::new();
    let mut count = 0usize;
    let mut bytes = 0usize;
    for (id, entity) in &project.entities {
        if !matches!(
            entity.payload,
            EntityPayload::NpcDraft(_) | EntityPayload::QuestDraft(_)
        ) {
            continue;
        }
        let OriginRef::New {
            authored_runtime_id,
        } = &entity.origin
        else {
            return Err(
                Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject {
                    reason: format!("Story entity {id} does not retain an authored runtime id"),
                },
            );
        };
        // Runtime IDs are a separate uniqueness domain, not artifact collision entries. Apply an
        // independent finite map budget here without consuming the separately charged collision
        // identities (module + path + every generator-version-specific symbol).
        charge_runtime_identity(authored_runtime_id, &mut count, &mut bytes)?;
        let folded = authored_runtime_id.to_ascii_lowercase();
        if let Some(first_owner) = seen.insert(folded, *id) {
            return Err(Revision3QuestCollisionSourceErrorV2::DuplicateRuntimeId {
                value: authored_runtime_id.clone(),
                first_owner,
                second_owner: *id,
            });
        }
    }
    Ok(())
}

fn physical_asset_references(project: &ProjectRevision3) -> BTreeMap<Sha256Digest, EntityId> {
    project
        .entities
        .iter()
        .filter_map(|(id, entity)| match &entity.payload {
            EntityPayload::VoiceTake(take) => Some((take.asset.sha256, *id)),
            _ => None,
        })
        .collect()
}

fn validate_union_identities(
    nonquest: &ProjectStoryCollisionIdentities,
    prior: &BTreeMap<EntityId, Revision3PriorQuestEvidenceV2>,
) -> Result<(), Revision3QuestCollisionSourceErrorV2> {
    let mut prior_modules = BTreeMap::new();
    let mut prior_paths = BTreeMap::new();
    let mut prior_symbols = BTreeMap::new();
    let mut count = 0usize;
    let mut bytes = 0usize;

    for (kind, values) in [
        ("module", nonquest.modules()),
        ("relative path", nonquest.relative_paths()),
        ("symbol", nonquest.symbols()),
    ] {
        for value in values.keys() {
            charge_identity(kind, value, &mut count, &mut bytes)?;
        }
    }
    for (owner, record) in prior {
        insert_prior_identity(
            "module",
            nonquest.modules(),
            &mut prior_modules,
            &record.module_namespace,
            *owner,
            &mut count,
            &mut bytes,
        )?;
        insert_prior_identity(
            "relative path",
            nonquest.relative_paths(),
            &mut prior_paths,
            &record.module_relative_path,
            *owner,
            &mut count,
            &mut bytes,
        )?;
        for symbol in &record.symbols {
            insert_prior_identity(
                "symbol",
                nonquest.symbols(),
                &mut prior_symbols,
                symbol,
                *owner,
                &mut count,
                &mut bytes,
            )?;
        }
    }
    Ok(())
}

fn insert_prior_identity(
    kind: &'static str,
    nonquest_values: &BTreeMap<String, EntityId>,
    prior_values: &mut BTreeMap<String, EntityId>,
    value: &str,
    owner: EntityId,
    count: &mut usize,
    bytes: &mut usize,
) -> Result<(), Revision3QuestCollisionSourceErrorV2> {
    charge_identity(kind, value, count, bytes)?;
    let folded = value.to_ascii_lowercase();
    if let Some(first_owner) = nonquest_values
        .get(&folded)
        .or_else(|| prior_values.get(&folded))
    {
        return Err(
            Revision3QuestCollisionSourceErrorV2::PriorIdentityCollision {
                kind,
                value: value.to_owned(),
                first_owner: *first_owner,
                second_owner: owner,
            },
        );
    }
    prior_values.insert(folded, owner);
    Ok(())
}

fn charge_identity(
    kind: &'static str,
    value: &str,
    count: &mut usize,
    bytes: &mut usize,
) -> Result<(), Revision3QuestCollisionSourceErrorV2> {
    enforce_identity_value(kind, value)?;
    *count = count.saturating_add(1);
    if *count > MAX_REVISION3_COLLISION_IDENTITIES_V2 {
        return Err(Revision3QuestCollisionSourceErrorV2::Limit {
            kind: "collision identity count",
            actual: *count,
            limit: MAX_REVISION3_COLLISION_IDENTITIES_V2,
        });
    }
    *bytes = bytes.saturating_add(value.len());
    if *bytes > MAX_REVISION3_COLLISION_IDENTITY_BYTES_V2 {
        return Err(Revision3QuestCollisionSourceErrorV2::Limit {
            kind: "collision identity bytes",
            actual: *bytes,
            limit: MAX_REVISION3_COLLISION_IDENTITY_BYTES_V2,
        });
    }
    Ok(())
}

fn charge_runtime_identity(
    value: &str,
    count: &mut usize,
    bytes: &mut usize,
) -> Result<(), Revision3QuestCollisionSourceErrorV2> {
    enforce_identity_value("authored runtime id", value)?;
    *count = count.saturating_add(1);
    if *count > MAX_REVISION3_COLLISION_IDENTITIES_V2 {
        return Err(Revision3QuestCollisionSourceErrorV2::Limit {
            kind: "runtime identity count",
            actual: *count,
            limit: MAX_REVISION3_COLLISION_IDENTITIES_V2,
        });
    }
    *bytes = bytes.saturating_add(value.len());
    if *bytes > MAX_REVISION3_COLLISION_IDENTITY_BYTES_V2 {
        return Err(Revision3QuestCollisionSourceErrorV2::Limit {
            kind: "runtime identity bytes",
            actual: *bytes,
            limit: MAX_REVISION3_COLLISION_IDENTITY_BYTES_V2,
        });
    }
    Ok(())
}

fn enforce_identity_value(
    _kind: &'static str,
    value: &str,
) -> Result<(), Revision3QuestCollisionSourceErrorV2> {
    if value.len() > MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2 {
        return Err(Revision3QuestCollisionSourceErrorV2::Limit {
            kind: "single collision identity bytes",
            actual: value.len(),
            limit: MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2,
        });
    }
    Ok(())
}

fn seal_prior_evidence(
    prior: &BTreeMap<EntityId, Revision3PriorQuestEvidenceV2>,
) -> Result<ContentSeal, Revision3QuestCollisionSourceErrorV2> {
    let bytes = bounded_json_bytes(prior, MAX_PROJECT_JSON_BYTES, "prior Quest evidence bytes")?;
    let mut hasher = Sha256::new();
    hasher.update(PRIOR_QUEST_EVIDENCE_DOMAIN_V2);
    hasher.update((prior.len() as u64).to_be_bytes());
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    Ok(ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(hasher.finalize().into()),
    })
}

fn raw_seal(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: digest_bytes(bytes),
    }
}

fn digest_bytes(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

fn bounded_json_bytes<T: Serialize + ?Sized>(
    value: &T,
    limit: usize,
    kind: &'static str,
) -> Result<Vec<u8>, Revision3QuestCollisionSourceErrorV2> {
    let mut writer = BoundedBytesWriter::new(limit);
    let result = serde_json::to_writer(&mut writer, value);
    if let Some(actual) = writer.first_exceeded_size {
        return Err(Revision3QuestCollisionSourceErrorV2::Limit {
            kind,
            actual,
            limit,
        });
    }
    result.map_err(
        |error| Revision3QuestCollisionSourceErrorV2::InvalidCurrentProject {
            reason: format!("could not serialize {kind}: {error}"),
        },
    )?;
    Ok(writer.bytes)
}

struct BoundedBytesWriter {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedBytesWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(64 * 1024)),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedBytesWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let actual = self.bytes.len().saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other(
                "bounded revision-3 source serialization exceeded",
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
    use crate::model_revision3::{
        DialogLine, Entity as Revision3Entity, ItemPatchV1, ItemScalarValueV1, LocalizationEntry,
        NpcDraft, NpcDraftInput, NpcGreetingBindingV1, NpcParentClassInput,
        QuestCollisionArtifactRef, QuestDraftInput, QuestTranscriptBindingV1,
    };
    use crate::{
        AssetMeta, AssetStoreIndex, FormatV2, ProjectMeta, QuestTransitionPlanV1, SchemaRevisionV3,
        QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2, QUEST_COLLISION_CATALOG_LAYER_V2,
    };

    trait AmbiguousIfClone<Marker> {
        fn marker() {}
    }

    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    impl<T: Clone> AmbiguousIfClone<u8> for T {}

    type ProjectMutation = Box<dyn FnOnce(&mut ProjectRevision3)>;

    fn id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
    }

    fn seal(value: u8, byte_len: u64) -> ContentSeal {
        ContentSeal {
            byte_len,
            sha256: Sha256Digest::from_bytes([value; 32]),
        }
    }

    fn exact_project() -> (ProjectRevision3, WorkingHead) {
        let target = GameGenerationAnchor {
            executable: seal(1, 170_000_000),
        };
        let project_id = ProjectId::from_bytes([3; 16]);
        let quest_id = id(10);
        let module_id = id(11);
        let artifact = seal(7, 2_048);
        let quest = QuestDraft {
            generator_id: REVISION3_QUEST_GENERATOR_ID.into(),
            generator_version: REVISION3_QUEST_GENERATOR_VERSION,
            input: QuestDraftInput {
                target: target.clone(),
                quest_id,
                module_namespace: "GoreMods.Quests.ExactPrior".into(),
                technical_id: "GORE_EXACT_PRIOR".into(),
                text_helper: "GoreExactPriorText".into(),
                parent_quest: QuestParentInput {
                    generation: target.clone(),
                    source_seal: seal(2, 2_000),
                    catalog_layer: "base-game.g1r.quests".into(),
                    canonical_selector: "CatalogQuest_Parent".into(),
                    runtime_class: "UQuest_Parent".into(),
                },
                giver: QuestGiverInput {
                    generation: target.clone(),
                    source_seal: seal(3, 3_000),
                    catalog_layer: "base-game.g1r.characters".into(),
                    canonical_selector: "CatalogCharacter_Asghan".into(),
                    runtime_unique_name: "OM_GRD_Asghan_263".into(),
                },
                title: "Exact prior".into(),
                description: "Regenerated only from the exact current project".into(),
                objective_title: "Reject every persisted drift".into(),
                additional_objective_titles: Vec::new(),
                transition_plan: Box::new(
                    QuestTransitionPlanV1::default_for_objectives(1).unwrap(),
                ),
                collision_catalog: QuestCollisionArtifactRef {
                    generation: target.clone(),
                    catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.into(),
                    artifact: artifact.clone(),
                    source_seal: seal(8, artifact.byte_len),
                    basis_snapshot: seal(9, 4_096),
                },
            },
            script_module: TypedRef::new(project_id, module_id, EntityKind::ScriptModule),
            transcript: Vec::new(),
        };
        let collision = QuestCollisionCatalogInput {
            generation: quest.input.collision_catalog.generation.clone(),
            source_seal: quest.input.collision_catalog.source_seal.clone(),
            catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
            modules: BTreeSet::new(),
            relative_paths: BTreeSet::new(),
            symbols: BTreeSet::new(),
        };
        let module = crate::regenerate_revision3_quest_module(&quest, collision).unwrap();
        let owner = TypedRef::new(project_id, quest_id, EntityKind::QuestDraft);
        let entities = BTreeMap::from([
            (
                quest_id,
                Revision3Entity {
                    id: quest_id,
                    display_name: "Exact prior Quest".into(),
                    origin: OriginRef::New {
                        authored_runtime_id: quest.input.technical_id.clone(),
                    },
                    revision: 0,
                    payload: EntityPayload::QuestDraft(quest),
                },
            ),
            (
                module_id,
                Revision3Entity {
                    id: module_id,
                    display_name: "Exact prior module".into(),
                    origin: OriginRef::Generated {
                        generator_id: REVISION3_QUEST_GENERATOR_ID.into(),
                        generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                        owner,
                    },
                    revision: 0,
                    payload: EntityPayload::ScriptModule(module),
                },
            ),
        ]);
        let project = ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id,
            revision: 8,
            meta: ProjectMeta {
                name: "exact current source".into(),
                version: "0.1.0".into(),
                author: "tests".into(),
            },
            target,
            authoring_locales: BTreeSet::new(),
            entities,
            asset_store: AssetStoreIndex {
                assets: BTreeMap::from([(
                    artifact.sha256,
                    AssetMeta {
                        byte_len: artifact.byte_len,
                        media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.into(),
                    },
                )]),
            },
        };
        let head = WorkingHead {
            store_format: crate::WorkingStoreFormat,
            snapshot: seal(6, 5_000),
        };
        (project, head)
    }

    #[test]
    fn source_capsules_and_evidence_records_are_not_cloneable_authority_tokens() {
        let _ = <PreparedRevision3QuestCollisionSourceV2 as AmbiguousIfClone<_>>::marker as fn();
        let _ = <PreparedRevision3QuestCollisionInspectionSourceV2 as AmbiguousIfClone<_>>::marker
            as fn();
        let _ = <Revision3NonQuestCollisionBasisV2 as AmbiguousIfClone<_>>::marker as fn();
        let _ = <Revision3PriorQuestEvidenceV2 as AmbiguousIfClone<_>>::marker as fn();
        let _prepare: fn(
            &crate::WorkingProjectStore,
            &WorkingHead,
        ) -> Result<
            PreparedRevision3QuestCollisionSourceV2,
            Revision3QuestCollisionSourceErrorV2,
        > = crate::WorkingProjectStore::prepare_current_revision3_quest_collision_source_v2;
        let _inspect: fn(
            &crate::WorkingProjectStore,
            &WorkingHead,
        ) -> Result<
            PreparedRevision3QuestCollisionInspectionSourceV2,
            Revision3QuestCollisionSourceErrorV2,
        > = crate::WorkingProjectStore::prepare_revision3_quest_collision_inspection_source_v2;
    }

    #[test]
    fn exact_regeneration_rejects_owner_origin_generator_source_fingerprint_and_shape_drift() {
        let (project, head) = exact_project();
        let prepared = prepare_exact_revision3_quest_collision_source_v2(&project, head.clone())
            .expect("valid exact current project");
        assert_eq!(prepared.prior_quest_count(), 1);
        assert_eq!(prepared.prior_quests()[&id(10)].module_id(), id(11));
        assert!(prepared.nonquest_basis().project().entities.is_empty());
        assert!(prepared
            .nonquest_basis()
            .project()
            .asset_store
            .assets
            .is_empty());

        let mut mutations: Vec<ProjectMutation> = vec![
            Box::new(|candidate| {
                let EntityPayload::ScriptModule(module) =
                    &mut candidate.entities.get_mut(&id(11)).unwrap().payload
                else {
                    unreachable!()
                };
                module.owner.id = id(12);
            }),
            Box::new(|candidate| {
                candidate.entities.get_mut(&id(11)).unwrap().origin = OriginRef::Generated {
                    generator_id: "foreign.generator".into(),
                    generator_version: 99,
                    owner: TypedRef::new(candidate.project_id, id(10), EntityKind::QuestDraft),
                };
            }),
            Box::new(|candidate| {
                let EntityPayload::QuestDraft(quest) =
                    &mut candidate.entities.get_mut(&id(10)).unwrap().payload
                else {
                    unreachable!()
                };
                quest.generator_version += 1;
            }),
            Box::new(|candidate| {
                let EntityPayload::ScriptModule(module) =
                    &mut candidate.entities.get_mut(&id(11)).unwrap().payload
                else {
                    unreachable!()
                };
                module.source.push_str("// drift\n");
            }),
            Box::new(|candidate| {
                let EntityPayload::ScriptModule(module) =
                    &mut candidate.entities.get_mut(&id(11)).unwrap().payload
                else {
                    unreachable!()
                };
                module.input_fingerprint = Sha256Digest::from_bytes([0x91; 32]);
            }),
            Box::new(|candidate| {
                let EntityPayload::ScriptModule(module) =
                    &mut candidate.entities.get_mut(&id(11)).unwrap().payload
                else {
                    unreachable!()
                };
                module.module_namespace = "GoreMods.Quests.Drifted".into();
            }),
            Box::new(|candidate| {
                let EntityPayload::ScriptModule(module) =
                    &mut candidate.entities.get_mut(&id(11)).unwrap().payload
                else {
                    unreachable!()
                };
                module.module_relative_path = "GoreMods/Quests/Drifted.as".into();
            }),
            Box::new(|candidate| {
                let EntityPayload::QuestDraft(quest) =
                    &mut candidate.entities.get_mut(&id(10)).unwrap().payload
                else {
                    unreachable!()
                };
                quest.input.parent_quest.runtime_class = "UQuest_Drifted".into();
            }),
            Box::new(|candidate| {
                let EntityPayload::QuestDraft(quest) =
                    &mut candidate.entities.get_mut(&id(10)).unwrap().payload
                else {
                    unreachable!()
                };
                quest.input.giver.runtime_unique_name = "OM_DRIFTED".into();
            }),
            Box::new(|candidate| {
                let EntityPayload::QuestDraft(quest) =
                    &mut candidate.entities.get_mut(&id(10)).unwrap().payload
                else {
                    unreachable!()
                };
                quest.input.collision_catalog.source_seal.sha256 =
                    Sha256Digest::from_bytes([0x92; 32]);
            }),
        ];
        for mutate in mutations.drain(..) {
            let mut candidate = project.clone();
            mutate(&mut candidate);
            assert!(
                prepare_exact_revision3_quest_collision_source_v2(&candidate, head.clone())
                    .is_err()
            );
        }
    }

    #[test]
    fn npc_greetings_survive_exact_quest_collision_source_split_and_recomposition() {
        let (mut project, head) = exact_project();
        let npc_id = id(20);
        let npc_module_id = id(21);
        let localization_id = id(22);
        let line_id = id(23);
        let parent = |value: u8, runtime_class: &str| NpcParentClassInput {
            generation: project.target.clone(),
            source_seal: seal(value, 4_096),
            catalog_layer: "base-game.g1r.npcs".to_owned(),
            canonical_selector: format!("Catalog_{runtime_class}"),
            runtime_class: runtime_class.to_owned(),
        };
        let owner = TypedRef::new(project.project_id, npc_id, EntityKind::NpcDraft);
        let npc = NpcDraft {
            generator_id: crate::LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
            generator_version: crate::LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: NpcDraftInput {
                target: project.target.clone(),
                module_namespace: "GoreMods.Npcs.GreetingSource".to_owned(),
                unique_name: "GORE_GREETING_SOURCE_NPC".to_owned(),
                parent_character_definition: parent(0x31, "UCharacterDefinition_Asghan"),
                parent_ai_agent_config: parent(0x32, "UAIAgentConfig_Asghan"),
                parent_spawn_definition: parent(0x33, "USpawnAIAgentDefinition_Asghan"),
            },
            script_module: TypedRef::new(
                project.project_id,
                npc_module_id,
                EntityKind::ScriptModule,
            ),
            greetings: vec![NpcGreetingBindingV1 {
                line: TypedRef::new(project.project_id, line_id, EntityKind::DialogLine),
            }],
        };
        let npc_module = npc.regenerate_script_module(owner.clone()).unwrap();
        project.entities.insert(
            localization_id,
            Revision3Entity {
                id: localization_id,
                display_name: "NPC greeting localization".to_owned(),
                origin: OriginRef::New {
                    authored_runtime_id: "GORE_GREETING_SOURCE_LOC".to_owned(),
                },
                revision: 2,
                payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "GORE_GREETING_SOURCE".to_owned(),
                    texts: BTreeMap::new(),
                }),
            },
        );
        project.entities.insert(
            line_id,
            Revision3Entity {
                id: line_id,
                display_name: "NPC greeting line".to_owned(),
                origin: OriginRef::New {
                    authored_runtime_id: "GORE_GREETING_SOURCE_LINE".to_owned(),
                },
                revision: 3,
                payload: EntityPayload::DialogLine(DialogLine {
                    localization: TypedRef::new(
                        project.project_id,
                        localization_id,
                        EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: Some("Asghan".to_owned()),
                    voice_slots: BTreeMap::new(),
                }),
            },
        );
        project.entities.insert(
            npc_id,
            Revision3Entity {
                id: npc_id,
                display_name: "Greeting source NPC".to_owned(),
                origin: OriginRef::New {
                    authored_runtime_id: "GORE_GREETING_SOURCE_NPC".to_owned(),
                },
                revision: 4,
                payload: EntityPayload::NpcDraft(npc),
            },
        );
        project.entities.insert(
            npc_module_id,
            Revision3Entity {
                id: npc_module_id,
                display_name: "Greeting source NPC module".to_owned(),
                origin: OriginRef::Generated {
                    generator_id: crate::LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                    generator_version: crate::LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                    owner,
                },
                revision: 5,
                payload: EntityPayload::ScriptModule(npc_module),
            },
        );
        project.validate_closed_model().unwrap();

        let prepared = prepare_exact_revision3_quest_collision_source_v2(&project, head)
            .expect("authoring-only NPC greetings do not break exact Quest collision source");
        assert_eq!(prepared.prior_quest_count(), 1);
        assert_eq!(prepared.nonquest_basis().project().entities.len(), 4);
        let native_npc = &prepared.nonquest_basis().project().entities[&npc_id];
        assert!(matches!(
            native_npc.payload,
            crate::Revision3EntityPayload::NpcDraft(_)
        ));
    }

    #[test]
    fn native_item_patches_survive_exact_quest_collision_source_split_and_recomposition() {
        let (mut project, head) = exact_project();
        let item_id = id(30);
        project.entities.insert(
            item_id,
            Revision3Entity {
                id: item_id,
                display_name: "Apple patch".to_owned(),
                origin: OriginRef::Vanilla {
                    generation: project.target.clone(),
                    catalog_layer: "base-game.items.g1r.v1".to_owned(),
                    canonical_selector: "ItFo_Apple".to_owned(),
                    source_seal: seal(0x52, 1_024),
                },
                revision: 3,
                payload: EntityPayload::ItemPatch(ItemPatchV1 {
                    vanilla_class: "ItFo_Apple".to_owned(),
                    fields: BTreeMap::from([(
                        "m_Value".to_owned(),
                        ItemScalarValueV1::Integer(17),
                    )]),
                }),
            },
        );
        project.validate_closed_model().unwrap();

        let prepared = prepare_exact_revision3_quest_collision_source_v2(&project, head)
            .expect("native ItemPatch is retained in the native Story collision basis");
        assert_eq!(prepared.prior_quest_count(), 1);
        assert_eq!(
            prepared.nonquest_basis().project().entities[&item_id],
            project.entities[&item_id]
        );
    }

    #[test]
    fn semantic_v4_quest_remains_an_exact_collision_source() {
        let (mut project, head) = exact_project();
        let project_id = project.project_id;
        let localization_id = id(12);
        let line_id = id(13);
        let quest = {
            let EntityPayload::QuestDraft(quest) =
                &mut project.entities.get_mut(&id(10)).unwrap().payload
            else {
                unreachable!()
            };
            quest.generator_version = REVISION3_QUEST_GENERATOR_VERSION;
            quest.input.transition_plan = Box::new(
                QuestTransitionPlanV1::default_for_objectives(1).expect("one-objective seed"),
            );
            quest.transcript = vec![QuestTranscriptBindingV1 {
                line: TypedRef::new(project_id, line_id, EntityKind::DialogLine),
                objective_slot: 1,
            }];
            quest.clone()
        };
        project.entities.insert(
            localization_id,
            Revision3Entity {
                id: localization_id,
                display_name: "Source-inspection transcript text".into(),
                origin: OriginRef::New {
                    authored_runtime_id: "GORE_SOURCE_TRANSCRIPT_LOC_ENTITY".into(),
                },
                revision: 2,
                payload: EntityPayload::LocalizationEntry(LocalizationEntry {
                    loc_id: "GORE_SOURCE_TRANSCRIPT_TEXT".into(),
                    texts: BTreeMap::new(),
                }),
            },
        );
        project.entities.insert(
            line_id,
            Revision3Entity {
                id: line_id,
                display_name: "Source-inspection transcript line".into(),
                origin: OriginRef::New {
                    authored_runtime_id: "GORE_SOURCE_TRANSCRIPT_LINE".into(),
                },
                revision: 4,
                payload: EntityPayload::DialogLine(DialogLine {
                    localization: TypedRef::new(
                        project_id,
                        localization_id,
                        EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: Some("Asghan".into()),
                    voice_slots: BTreeMap::new(),
                }),
            },
        );
        let collision = QuestCollisionCatalogInput {
            generation: quest.input.collision_catalog.generation.clone(),
            source_seal: quest.input.collision_catalog.source_seal.clone(),
            catalog_layer: quest.input.collision_catalog.catalog_layer.clone(),
            modules: BTreeSet::new(),
            relative_paths: BTreeSet::new(),
            symbols: BTreeSet::new(),
        };
        let module = crate::regenerate_revision3_quest_module(&quest, collision).unwrap();
        let module_entity = project.entities.get_mut(&id(11)).unwrap();
        let OriginRef::Generated {
            generator_version, ..
        } = &mut module_entity.origin
        else {
            unreachable!()
        };
        *generator_version = REVISION3_QUEST_GENERATOR_VERSION;
        module_entity.payload = EntityPayload::ScriptModule(module);

        let prepared = prepare_exact_revision3_quest_collision_source_v2(&project, head)
            .expect("semantic Quest remains a reconstructable exact source");
        assert_eq!(prepared.prior_quest_count(), 1);
        assert_eq!(prepared.prior_quests()[&id(10)].module_id(), id(11));
        assert_eq!(
            project.entities[&id(10)].payload,
            EntityPayload::QuestDraft(quest)
        );
    }

    #[test]
    fn orphan_and_shared_quest_modules_are_not_reinterpreted() {
        let (project, head) = exact_project();
        let mut orphan = project.clone();
        let mut orphan_entity = orphan.entities[&id(11)].clone();
        orphan_entity.id = id(12);
        orphan.entities.insert(id(12), orphan_entity);
        assert!(matches!(
            prepare_exact_revision3_quest_collision_source_v2(&orphan, head.clone()),
            Err(Revision3QuestCollisionSourceErrorV2::ResidualQuestState { entity })
                if entity == id(12)
        ));

        let mut shared = project;
        let EntityPayload::QuestDraft(mut second) = shared.entities[&id(10)].payload.clone() else {
            unreachable!()
        };
        second.input.quest_id = id(20);
        shared.entities.insert(
            id(20),
            Revision3Entity {
                id: id(20),
                display_name: "Shared module Quest".into(),
                origin: OriginRef::New {
                    authored_runtime_id: second.input.technical_id.clone(),
                },
                revision: 0,
                payload: EntityPayload::QuestDraft(second),
            },
        );
        assert!(matches!(
            prepare_exact_revision3_quest_collision_source_v2(&shared, head),
            Err(Revision3QuestCollisionSourceErrorV2::SharedQuestModule { module, .. })
                if module == id(11)
        ));
    }
}
