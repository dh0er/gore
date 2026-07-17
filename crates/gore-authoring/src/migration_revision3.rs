//! Explicit schema-revision-2 to revision-3 migration without artifact fabrication.

use serde::Serialize;

use crate::model_revision2 as revision2;
use crate::model_revision3 as revision3;
use crate::{EntityId, FormatV2, ProjectId, DRAFT_QUEST_GENERATOR_ID};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Revision2ToRevision3Migration {
    pub project: revision3::ProjectRevision3,
    pub report: Revision2ToRevision3Report,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Revision2ToRevision3Report {
    pub format: FormatV2,
    pub source_schema_revision: revision2::SchemaRevisionV2,
    pub target_schema_revision: revision3::SchemaRevisionV3,
    pub project_id: ProjectId,
    pub migrated_entities: u64,
    pub collision_artifacts_created: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum Revision2ToRevision3Error {
    #[error(
        "revision-2 project contains {quest_count} Quest Draft(s), first at {first_quest}; explicit collision-artifact repinning is required"
    )]
    QuestRepinRequired {
        quest_count: u64,
        first_quest: EntityId,
    },
    #[error(
        "revision-2 Script Module {module} owned by {owner} contains residual Quest generation state; explicit collision-artifact repinning is required"
    )]
    QuestModuleRepinRequired { module: EntityId, owner: EntityId },
    #[error("revision-2 entity map key {key} does not match embedded id {embedded}")]
    EntityKeyMismatch { key: EntityId, embedded: EntityId },
    #[error("migrated revision-3 model is invalid: {0}")]
    InvalidTarget(#[source] revision3::ProjectRevision3ValidationError),
}

/// Copy one Quest-free revision-2 project into the closed revision-3 model.
///
/// Project/entity identities and every unchanged payload byte value are preserved. This migration
/// never discards a collision catalog, invents a content seal, writes an asset, or implicitly
/// repins a Quest. The presence of even one revision-2 Quest or residual Quest-owned/generated
/// Script Module rejects the complete operation.
pub fn migrate_revision2_to_revision3(
    source: &revision2::ProjectRevision2,
) -> Result<Revision2ToRevision3Migration, Revision2ToRevision3Error> {
    let mut quest_count = 0u64;
    let mut first_quest = None;
    let mut first_quest_module = None;
    for (key, entity) in &source.entities {
        if key != &entity.id {
            return Err(Revision2ToRevision3Error::EntityKeyMismatch {
                key: *key,
                embedded: entity.id,
            });
        }
        if matches!(entity.payload, revision2::EntityPayload::QuestDraft(_)) {
            quest_count = quest_count.saturating_add(1);
            first_quest.get_or_insert(*key);
        }
        if let revision2::EntityPayload::ScriptModule(module) = &entity.payload {
            let payload_marks_quest = module.owner.expected_kind
                == revision2::EntityKind::QuestDraft
                || module.generator_id == DRAFT_QUEST_GENERATOR_ID;
            let origin_marks_quest = matches!(
                &entity.origin,
                revision2::OriginRef::Generated {
                    generator_id,
                    owner,
                    ..
                } if generator_id == DRAFT_QUEST_GENERATOR_ID
                    || owner.expected_kind == revision2::EntityKind::QuestDraft
            );
            if payload_marks_quest || origin_marks_quest {
                first_quest_module.get_or_insert((*key, module.owner.id));
            }
        }
    }
    if let Some(first_quest) = first_quest {
        return Err(Revision2ToRevision3Error::QuestRepinRequired {
            quest_count,
            first_quest,
        });
    }
    if let Some((module, owner)) = first_quest_module {
        return Err(Revision2ToRevision3Error::QuestModuleRepinRequired { module, owner });
    }

    let entities = source
        .entities
        .iter()
        .map(|(id, entity)| (*id, migrate_entity(entity)))
        .collect();
    let project = revision3::ProjectRevision3 {
        format: FormatV2,
        schema_revision: revision3::SchemaRevisionV3,
        project_id: source.project_id,
        revision: source.revision,
        meta: source.meta.clone(),
        target: source.target.clone(),
        authoring_locales: source.authoring_locales.clone(),
        entities,
        asset_store: source.asset_store.clone(),
    };
    project
        .validate_closed_model()
        .map_err(Revision2ToRevision3Error::InvalidTarget)?;

    Ok(Revision2ToRevision3Migration {
        report: Revision2ToRevision3Report {
            format: FormatV2,
            source_schema_revision: revision2::SchemaRevisionV2,
            target_schema_revision: revision3::SchemaRevisionV3,
            project_id: source.project_id,
            migrated_entities: u64::try_from(source.entities.len())
                .expect("bounded revision-2 entity count fits in u64"),
            collision_artifacts_created: 0,
        },
        project,
    })
}

fn migrate_entity(source: &revision2::Entity) -> revision3::Entity {
    let payload = match &source.payload {
        revision2::EntityPayload::LocalizationEntry(value) => {
            revision3::EntityPayload::LocalizationEntry(value.clone())
        }
        revision2::EntityPayload::DialogLine(value) => {
            revision3::EntityPayload::DialogLine(value.clone())
        }
        revision2::EntityPayload::VoiceSlot(value) => {
            revision3::EntityPayload::VoiceSlot(value.clone())
        }
        revision2::EntityPayload::VoiceTake(value) => {
            revision3::EntityPayload::VoiceTake(value.clone())
        }
        revision2::EntityPayload::NpcDraft(value) => {
            revision3::EntityPayload::NpcDraft(revision3::NpcDraft {
                generator_id: value.generator_id.clone(),
                generator_version: value.generator_version,
                input: value.input.clone(),
                script_module: value.script_module.clone(),
                greetings: Vec::new(),
            })
        }
        revision2::EntityPayload::ScriptModule(value) => {
            revision3::EntityPayload::ScriptModule(value.clone())
        }
        revision2::EntityPayload::QuestDraft(_) => {
            unreachable!("Quest presence is rejected before migration")
        }
    };
    revision3::Entity {
        id: source.id,
        display_name: source.display_name.clone(),
        origin: source.origin.clone(),
        revision: source.revision,
        payload,
    }
}
