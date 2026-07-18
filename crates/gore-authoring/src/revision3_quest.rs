//! Shared, filesystem-free schema-revision-3 Quest primitives.
//!
//! These helpers deterministically lower already-selected Quest intent into offline source and
//! project a Quest-free revision-3 basis into the older collision-source model. Neither helper
//! authenticates a collision artifact, qualifies runtime behavior, or grants build/publication
//! authority.

use sha2::{Digest as _, Sha256};

use crate::model_revision3::{
    EntityKind, EntityPayload, OriginRef, ProjectRevision3, QuestDraft, QuestDraftInput,
    ScriptModule, ScriptModuleStatus, TypedRef, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_QUEST_GENERATOR_VERSION,
};
use crate::revision3_story_generation::GeneratedStoryIdentity;
use crate::{
    CatalogQualifiedParentQuest, CatalogQualifiedQuestGiver, DraftQuestCollisionCatalog,
    DraftQuestSkeleton, DraftQuestSkeletonError, DraftQuestSkeletonInput, EntityId,
    QuestCollisionCatalogInput, Sha256Digest,
};

const REVISION3_QUEST_INPUT_FINGERPRINT_DOMAIN: &[u8] =
    b"gore-authoring.revision3-quest.input-fingerprint\0";

#[derive(Debug, thiserror::Error)]
pub enum Revision3QuestGenerationError {
    #[error(
        "revision-3 Quest generator contract mismatch: expected {expected_id}@{expected_version}, got {actual_id}@{actual_version}"
    )]
    GeneratorContract {
        expected_id: &'static str,
        expected_version: u32,
        actual_id: String,
        actual_version: u32,
    },
    #[error("revision-3 Quest ScriptModule reference is zero, foreign, or mistyped")]
    InvalidScriptModuleReference,
    #[error("revision-3 Quest collision generation differs from its ArtifactRef")]
    CollisionGenerationMismatch,
    #[error("revision-3 Quest collision source seal differs from its ArtifactRef")]
    CollisionSourceSealMismatch,
    #[error("revision-3 Quest collision catalog layer differs from its ArtifactRef")]
    CollisionCatalogLayerMismatch,
    #[error("invalid revision-3 Quest generator intent: {0}")]
    InvalidQuestIntent(#[source] DraftQuestSkeletonError),
    #[error("could not serialize revision-3 Quest input fingerprint: {0}")]
    SerializeQuestInput(#[source] serde_json::Error),
}

/// Stable fingerprint of every bounded revision-3 Quest input field, including the raw,
/// semantic, and basis seals retained by its artifact reference.
///
/// This is deterministic source-generation metadata only. A matching digest is not artifact
/// authenticity or runtime evidence.
pub fn revision3_quest_input_fingerprint(
    input: &QuestDraftInput,
) -> Result<Sha256Digest, Revision3QuestGenerationError> {
    let canonical =
        serde_json::to_vec(input).map_err(Revision3QuestGenerationError::SerializeQuestInput)?;
    let mut hasher = Sha256::new();
    hasher.update(REVISION3_QUEST_INPUT_FINGERPRINT_DOMAIN);
    hasher.update((canonical.len() as u64).to_be_bytes());
    hasher.update(&canonical);
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}

/// Regenerate the one exact offline ScriptModule described by a revision-3 Quest Draft.
///
/// `collision` must come from a separately verified caller. This function validates its closed
/// generator shape and collision entries but deliberately cannot authenticate its provenance.
pub fn regenerate_revision3_quest_module(
    quest: &QuestDraft,
    collision: QuestCollisionCatalogInput,
) -> Result<ScriptModule, Revision3QuestGenerationError> {
    regenerate_revision3_quest_module_with_identity(quest, collision).map(|(module, _)| module)
}

pub(crate) fn regenerate_revision3_quest_module_with_identity(
    quest: &QuestDraft,
    collision: QuestCollisionCatalogInput,
) -> Result<(ScriptModule, GeneratedStoryIdentity), Revision3QuestGenerationError> {
    if quest.generator_id != REVISION3_QUEST_GENERATOR_ID
        || quest.generator_version != REVISION3_QUEST_GENERATOR_VERSION
    {
        return Err(Revision3QuestGenerationError::GeneratorContract {
            expected_id: REVISION3_QUEST_GENERATOR_ID,
            expected_version: REVISION3_QUEST_GENERATOR_VERSION,
            actual_id: quest.generator_id.clone(),
            actual_version: quest.generator_version,
        });
    }
    if quest
        .script_module
        .project_id
        .as_bytes()
        .iter()
        .all(|byte| *byte == 0)
        || quest
            .script_module
            .id
            .as_bytes()
            .iter()
            .all(|byte| *byte == 0)
        || quest.script_module.expected_kind != EntityKind::ScriptModule
    {
        return Err(Revision3QuestGenerationError::InvalidScriptModuleReference);
    }
    let reference = &quest.input.collision_catalog;
    if collision.generation != reference.generation {
        return Err(Revision3QuestGenerationError::CollisionGenerationMismatch);
    }
    if collision.source_seal != reference.source_seal {
        return Err(Revision3QuestGenerationError::CollisionSourceSealMismatch);
    }
    if collision.catalog_layer != reference.catalog_layer {
        return Err(Revision3QuestGenerationError::CollisionCatalogLayerMismatch);
    }

    let parent = CatalogQualifiedParentQuest::new(
        quest.input.parent_quest.generation.clone(),
        quest.input.parent_quest.source_seal.clone(),
        quest.input.parent_quest.catalog_layer.clone(),
        quest.input.parent_quest.canonical_selector.clone(),
        quest.input.parent_quest.runtime_class.clone(),
    )
    .map_err(Revision3QuestGenerationError::InvalidQuestIntent)?;
    let giver = CatalogQualifiedQuestGiver::new(
        quest.input.giver.generation.clone(),
        quest.input.giver.source_seal.clone(),
        quest.input.giver.catalog_layer.clone(),
        quest.input.giver.canonical_selector.clone(),
        quest.input.giver.runtime_unique_name.clone(),
    )
    .map_err(Revision3QuestGenerationError::InvalidQuestIntent)?;
    let collision_catalog = DraftQuestCollisionCatalog::new(
        collision.generation,
        collision.source_seal,
        collision.catalog_layer,
        collision.modules.into_iter().collect(),
        collision.relative_paths.into_iter().collect(),
        collision.symbols.into_iter().collect(),
    )
    .map_err(Revision3QuestGenerationError::InvalidQuestIntent)?;
    let generated = DraftQuestSkeleton::new(DraftQuestSkeletonInput {
        target: quest.input.target.clone(),
        quest_id: quest.input.quest_id,
        module_namespace: quest.input.module_namespace.clone(),
        technical_id: quest.input.technical_id.clone(),
        text_helper: quest.input.text_helper.clone(),
        parent_quest: parent,
        giver,
        title: quest.input.title.clone(),
        description: quest.input.description.clone(),
        objective_title: quest.input.objective_title.clone(),
        additional_objective_titles: quest.input.additional_objective_titles.clone(),
        transition_plan: (*quest.input.transition_plan).clone(),
        collision_catalog,
    })
    .map_err(Revision3QuestGenerationError::InvalidQuestIntent)?
    .generate();
    let names = generated.technical_names;
    let slot_one = names
        .objectives
        .iter()
        .find(|objective| objective.slot == 1)
        .expect("validated semantic plan retains slot 1");
    let mut symbols = vec![
        names.base.root_class.clone(),
        slot_one.objective_class.clone(),
        names.base.text_helper.clone(),
        names.base.root_getter.clone(),
        slot_one.objective_getter.clone(),
    ];
    for objective in names
        .objectives
        .into_iter()
        .filter(|objective| objective.slot != 1)
    {
        symbols.push(objective.objective_class);
        symbols.push(objective.objective_getter);
    }
    let (module_namespace, module_relative_path, source, source_sha256) = (
        names.base.module_namespace,
        names.base.module_relative_path,
        generated.source,
        generated.source_sha256,
    );
    let identity = GeneratedStoryIdentity {
        module_namespace: module_namespace.clone(),
        module_relative_path: module_relative_path.clone(),
        symbols,
    };
    let module = ScriptModule {
        generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
        generator_version: quest.generator_version,
        owner: TypedRef::new(
            quest.script_module.project_id,
            quest.input.quest_id,
            EntityKind::QuestDraft,
        ),
        module_namespace,
        module_relative_path,
        source,
        source_sha256,
        input_fingerprint: revision3_quest_input_fingerprint(&quest.input)?,
        status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
    };
    Ok((module, identity))
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision3QuestFreeBasisError {
    #[error("revision-3 Quest-free basis is not a closed valid project: {reason}")]
    InvalidProject { reason: String },
    #[error("revision-3 Quest-free basis contains Quest Draft {entity}")]
    RecursiveQuest { entity: EntityId },
    #[error("revision-3 Quest-free basis contains residual Quest-generated state {entity}")]
    ResidualQuestState { entity: EntityId },
}

/// Validate one native revision-3 project as a closed, Quest-free Story collision basis.
///
/// Quest Drafts and residual Quest-owned generator state are rejected. Callers that need
/// immutable-basis proof must separately reopen and verify the pinned revision-3 snapshot.
pub fn validate_revision3_quest_free_basis(
    source: &ProjectRevision3,
) -> Result<(), Revision3QuestFreeBasisError> {
    // Classify recursive/residual Quest state before general validation so callers receive the
    // stable boundary conflict even when that forbidden state is itself incomplete.
    for (id, entity) in &source.entities {
        reject_quest_basis_state(*id, entity)?;
    }
    source.validate_closed_model().map_err(|error| {
        Revision3QuestFreeBasisError::InvalidProject {
            reason: error.to_string(),
        }
    })?;
    Ok(())
}

fn reject_quest_basis_state(
    id: EntityId,
    entity: &crate::model_revision3::Entity,
) -> Result<(), Revision3QuestFreeBasisError> {
    if matches!(entity.payload, EntityPayload::QuestDraft(_)) {
        return Err(Revision3QuestFreeBasisError::RecursiveQuest { entity: id });
    }
    if let EntityPayload::ScriptModule(module) = &entity.payload {
        if module.owner.expected_kind == EntityKind::QuestDraft
            || module.generator_id == REVISION3_QUEST_GENERATOR_ID
        {
            return Err(Revision3QuestFreeBasisError::ResidualQuestState { entity: id });
        }
    }
    if matches!(
        &entity.origin,
        OriginRef::Generated {
            generator_id,
            owner,
            ..
        } if generator_id == REVISION3_QUEST_GENERATOR_ID
            || owner.expected_kind == EntityKind::QuestDraft
    ) {
        return Err(Revision3QuestFreeBasisError::ResidualQuestState { entity: id });
    }
    Ok(())
}
