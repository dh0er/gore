//! Pure, deterministic migrations between closed authoring schema revisions.

use std::fmt;

use serde::Serialize;

use crate::model as revision1;
use crate::model_revision2 as revision2;
use crate::{EntityId, FormatV2, LocaleCode, ProjectId, SchemaRevisionV1};

/// Complete result of a revision-1 to revision-2 migration.
///
/// This is an output-only value. It intentionally cannot be deserialized as a redundant proof
/// bundle whose project and report fields might contradict one another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Revision1ToRevision2Migration {
    pub project: revision2::ProjectRevision2,
    pub report: Revision1ToRevision2Report,
}

/// Closed, machine-readable account of the migration's two semantic transformations.
///
/// Reports are emitted by the migration but are not accepted back as verification evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Revision1ToRevision2Report {
    pub format: FormatV2,
    pub source_schema_revision: SchemaRevisionV1,
    pub target_schema_revision: revision2::SchemaRevisionV2,
    pub source_project_id: ProjectId,
    pub target_project_id: ProjectId,
    pub migrated_entities: u64,
    pub rewritten_internal_project_refs: u64,
    pub transformations: [Revision1ToRevision2Transformation; 2],
}

/// The fixed transformation vocabulary for revision-1 to revision-2 migration reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Revision1ToRevision2Transformation {
    ProjectIdentityReassigned {
        source_project_id: ProjectId,
        target_project_id: ProjectId,
    },
    InternalProjectReferencesRewritten {
        source_project_id: ProjectId,
        target_project_id: ProjectId,
        count: u64,
    },
}

/// Closed position vocabulary for every typed authored reference in schema revision 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "position", rename_all = "snake_case")]
pub enum Revision1TypedRefPosition {
    EntityOriginGeneratedOwner {
        entity_key: EntityId,
    },
    DialogLineLocalization {
        entity_key: EntityId,
    },
    DialogLineVoiceSlot {
        entity_key: EntityId,
        locale: LocaleCode,
    },
    VoiceSlotCandidate {
        entity_key: EntityId,
        index: u64,
    },
    VoiceSlotSelected {
        entity_key: EntityId,
    },
}

impl fmt::Display for Revision1TypedRefPosition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityOriginGeneratedOwner { entity_key } => {
                write!(formatter, "entities.{entity_key}.origin.owner")
            }
            Self::DialogLineLocalization { entity_key } => {
                write!(formatter, "entities.{entity_key}.payload.data.localization")
            }
            Self::DialogLineVoiceSlot { entity_key, locale } => write!(
                formatter,
                "entities.{entity_key}.payload.data.voice_slots.{locale}"
            ),
            Self::VoiceSlotCandidate { entity_key, index } => write!(
                formatter,
                "entities.{entity_key}.payload.data.candidates.{index}"
            ),
            Self::VoiceSlotSelected { entity_key } => {
                write!(formatter, "entities.{entity_key}.payload.data.selected")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Revision1ToRevision2Error {
    #[error("revision-2 migration requires a new project id distinct from {project_id}")]
    ProjectIdNotChanged { project_id: ProjectId },
    #[error("source entity map key {key} does not match embedded id {embedded_id}")]
    EntityKeyIdMismatch {
        key: EntityId,
        embedded_id: EntityId,
    },
    #[error(
        "target project id {target_project_id} is already used by {conflicting_refs} foreign source reference(s); first at {first_position}"
    )]
    ForeignReferenceWouldBecomeInternal {
        target_project_id: ProjectId,
        conflicting_refs: u64,
        first_position: Revision1TypedRefPosition,
    },
}

/// Migrates a revision-1 project without mutating it.
///
/// Entity IDs and all content are preserved. Every authored reference qualified by the source
/// project ID is rewritten to the explicitly supplied new project ID; foreign references remain
/// untouched. Iteration follows the source project's ordered maps, making project and report
/// bytes deterministic for identical inputs.
pub fn migrate_revision1_to_revision2(
    source: &revision1::ProjectV2,
    new_project_id: ProjectId,
) -> Result<Revision1ToRevision2Migration, Revision1ToRevision2Error> {
    if source.project_id == new_project_id {
        return Err(Revision1ToRevision2Error::ProjectIdNotChanged {
            project_id: source.project_id,
        });
    }
    preflight_source(source, new_project_id)?;

    let mut rewritten_internal_project_refs = 0_u64;
    let entities = source
        .entities
        .iter()
        .map(|(key, entity)| {
            (
                *key,
                migrate_entity(
                    entity,
                    source.project_id,
                    new_project_id,
                    &mut rewritten_internal_project_refs,
                ),
            )
        })
        .collect();
    let migrated_entities = u64::try_from(source.entities.len())
        .expect("bounded authoring entity count always fits in u64");

    let project = revision2::ProjectRevision2 {
        format: FormatV2,
        schema_revision: revision2::SchemaRevisionV2,
        project_id: new_project_id,
        revision: source.revision,
        meta: source.meta.clone(),
        target: source.target.clone(),
        authoring_locales: source.authoring_locales.clone(),
        entities,
        asset_store: source.asset_store.clone(),
    };
    let transformations = [
        Revision1ToRevision2Transformation::ProjectIdentityReassigned {
            source_project_id: source.project_id,
            target_project_id: new_project_id,
        },
        Revision1ToRevision2Transformation::InternalProjectReferencesRewritten {
            source_project_id: source.project_id,
            target_project_id: new_project_id,
            count: rewritten_internal_project_refs,
        },
    ];
    let report = Revision1ToRevision2Report {
        format: FormatV2,
        source_schema_revision: SchemaRevisionV1,
        target_schema_revision: revision2::SchemaRevisionV2,
        source_project_id: source.project_id,
        target_project_id: new_project_id,
        migrated_entities,
        rewritten_internal_project_refs,
        transformations,
    };

    Ok(Revision1ToRevision2Migration { project, report })
}

fn preflight_source(
    source: &revision1::ProjectV2,
    new_project_id: ProjectId,
) -> Result<(), Revision1ToRevision2Error> {
    for (key, entity) in &source.entities {
        if *key != entity.id {
            return Err(Revision1ToRevision2Error::EntityKeyIdMismatch {
                key: *key,
                embedded_id: entity.id,
            });
        }
    }

    let mut conflicting_refs = 0_u64;
    let mut first_position = None;
    walk_revision1_typed_refs(source, |position, reference| {
        if reference.project_id == new_project_id {
            conflicting_refs = conflicting_refs
                .checked_add(1)
                .expect("bounded authoring reference count always fits in u64");
            if first_position.is_none() {
                first_position = Some(position);
            }
        }
    });
    if let Some(first_position) = first_position {
        return Err(
            Revision1ToRevision2Error::ForeignReferenceWouldBecomeInternal {
                target_project_id: new_project_id,
                conflicting_refs,
                first_position,
            },
        );
    }
    Ok(())
}

fn walk_revision1_typed_refs(
    source: &revision1::ProjectV2,
    mut visit: impl FnMut(Revision1TypedRefPosition, &revision1::TypedRef),
) {
    for (entity_key, entity) in &source.entities {
        if let revision1::OriginRef::Generated { owner, .. } = &entity.origin {
            visit(
                Revision1TypedRefPosition::EntityOriginGeneratedOwner {
                    entity_key: *entity_key,
                },
                owner,
            );
        }

        match &entity.payload {
            revision1::EntityPayload::LocalizationEntry(_)
            | revision1::EntityPayload::VoiceTake(_) => {}
            revision1::EntityPayload::DialogLine(line) => {
                visit(
                    Revision1TypedRefPosition::DialogLineLocalization {
                        entity_key: *entity_key,
                    },
                    &line.localization,
                );
                for (locale, reference) in &line.voice_slots {
                    visit(
                        Revision1TypedRefPosition::DialogLineVoiceSlot {
                            entity_key: *entity_key,
                            locale: locale.clone(),
                        },
                        reference,
                    );
                }
            }
            revision1::EntityPayload::VoiceSlot(slot) => {
                for (index, reference) in slot.candidates.iter().enumerate() {
                    visit(
                        Revision1TypedRefPosition::VoiceSlotCandidate {
                            entity_key: *entity_key,
                            index: u64::try_from(index)
                                .expect("bounded candidate index always fits in u64"),
                        },
                        reference,
                    );
                }
                if let Some(selected) = &slot.selected {
                    visit(
                        Revision1TypedRefPosition::VoiceSlotSelected {
                            entity_key: *entity_key,
                        },
                        selected,
                    );
                }
            }
        }
    }
}

fn migrate_entity(
    source: &revision1::Entity,
    source_project_id: ProjectId,
    target_project_id: ProjectId,
    rewritten_refs: &mut u64,
) -> revision2::Entity {
    revision2::Entity {
        id: source.id,
        display_name: source.display_name.clone(),
        origin: migrate_origin(
            &source.origin,
            source_project_id,
            target_project_id,
            rewritten_refs,
        ),
        revision: source.revision,
        payload: migrate_payload(
            &source.payload,
            source_project_id,
            target_project_id,
            rewritten_refs,
        ),
    }
}

fn migrate_origin(
    source: &revision1::OriginRef,
    source_project_id: ProjectId,
    target_project_id: ProjectId,
    rewritten_refs: &mut u64,
) -> revision2::OriginRef {
    match source {
        revision1::OriginRef::New {
            authored_runtime_id,
        } => revision2::OriginRef::New {
            authored_runtime_id: authored_runtime_id.clone(),
        },
        revision1::OriginRef::Vanilla {
            generation,
            catalog_layer,
            canonical_selector,
            source_seal,
        } => revision2::OriginRef::Vanilla {
            generation: generation.clone(),
            catalog_layer: catalog_layer.clone(),
            canonical_selector: canonical_selector.clone(),
            source_seal: source_seal.clone(),
        },
        revision1::OriginRef::Imported {
            importer,
            source_seal,
            external_identity,
        } => revision2::OriginRef::Imported {
            importer: importer.clone(),
            source_seal: source_seal.clone(),
            external_identity: external_identity.clone(),
        },
        revision1::OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } => revision2::OriginRef::Generated {
            generator_id: generator_id.clone(),
            generator_version: *generator_version,
            owner: migrate_typed_ref(owner, source_project_id, target_project_id, rewritten_refs),
        },
    }
}

fn migrate_payload(
    source: &revision1::EntityPayload,
    source_project_id: ProjectId,
    target_project_id: ProjectId,
    rewritten_refs: &mut u64,
) -> revision2::EntityPayload {
    match source {
        revision1::EntityPayload::LocalizationEntry(localization) => {
            revision2::EntityPayload::LocalizationEntry(revision2::LocalizationEntry {
                loc_id: localization.loc_id.clone(),
                texts: localization.texts.clone(),
            })
        }
        revision1::EntityPayload::DialogLine(line) => {
            revision2::EntityPayload::DialogLine(revision2::DialogLine {
                localization: migrate_typed_ref(
                    &line.localization,
                    source_project_id,
                    target_project_id,
                    rewritten_refs,
                ),
                speaker_hint: line.speaker_hint.clone(),
                voice_slots: line
                    .voice_slots
                    .iter()
                    .map(|(locale, reference)| {
                        (
                            locale.clone(),
                            migrate_typed_ref(
                                reference,
                                source_project_id,
                                target_project_id,
                                rewritten_refs,
                            ),
                        )
                    })
                    .collect(),
            })
        }
        revision1::EntityPayload::VoiceSlot(slot) => {
            revision2::EntityPayload::VoiceSlot(revision2::VoiceSlot {
                locale: slot.locale.clone(),
                target_resolution: migrate_target_resolution(&slot.target_resolution),
                candidates: slot
                    .candidates
                    .iter()
                    .map(|reference| {
                        migrate_typed_ref(
                            reference,
                            source_project_id,
                            target_project_id,
                            rewritten_refs,
                        )
                    })
                    .collect(),
                selected: slot.selected.as_ref().map(|reference| {
                    migrate_typed_ref(
                        reference,
                        source_project_id,
                        target_project_id,
                        rewritten_refs,
                    )
                }),
            })
        }
        revision1::EntityPayload::VoiceTake(take) => {
            revision2::EntityPayload::VoiceTake(revision2::VoiceTake {
                locale: take.locale.clone(),
                asset: take.asset.clone(),
                ogg: migrate_ogg_metadata(&take.ogg),
                status: migrate_take_status(take.status),
            })
        }
    }
}

fn migrate_typed_ref(
    source: &revision1::TypedRef,
    source_project_id: ProjectId,
    target_project_id: ProjectId,
    rewritten_refs: &mut u64,
) -> revision2::TypedRef {
    let project_id = if source.project_id == source_project_id {
        *rewritten_refs = rewritten_refs
            .checked_add(1)
            .expect("bounded authoring reference count always fits in u64");
        target_project_id
    } else {
        source.project_id
    };
    revision2::TypedRef::new(project_id, source.id, migrate_kind(source.expected_kind))
}

const fn migrate_kind(source: revision1::EntityKind) -> revision2::EntityKind {
    match source {
        revision1::EntityKind::LocalizationEntry => revision2::EntityKind::LocalizationEntry,
        revision1::EntityKind::DialogLine => revision2::EntityKind::DialogLine,
        revision1::EntityKind::VoiceSlot => revision2::EntityKind::VoiceSlot,
        revision1::EntityKind::VoiceTake => revision2::EntityKind::VoiceTake,
    }
}

fn migrate_target_resolution(
    source: &revision1::VoiceTargetResolution,
) -> revision2::VoiceTargetResolution {
    match source {
        revision1::VoiceTargetResolution::Unresolved => {
            revision2::VoiceTargetResolution::Unresolved
        }
        revision1::VoiceTargetResolution::Ambiguous { candidates } => {
            revision2::VoiceTargetResolution::Ambiguous {
                candidates: candidates.iter().map(migrate_voice_target).collect(),
            }
        }
        revision1::VoiceTargetResolution::Resolved { target } => {
            revision2::VoiceTargetResolution::Resolved {
                target: migrate_voice_target(target),
            }
        }
    }
}

fn migrate_voice_target(source: &revision1::VoiceTarget) -> revision2::VoiceTarget {
    revision2::VoiceTarget {
        archive: source.archive.clone(),
        member: source.member.clone(),
        operation: match source.operation {
            revision1::VoiceOperation::Add => revision2::VoiceOperation::Add,
            revision1::VoiceOperation::Replace => revision2::VoiceOperation::Replace,
        },
        archive_seal: source.archive_seal.clone(),
        member_proof: match source.member_proof {
            revision1::VoiceMemberProof::Present {
                uncompressed_size,
                crc32,
            } => revision2::VoiceMemberProof::Present {
                uncompressed_size,
                crc32,
            },
            revision1::VoiceMemberProof::Absent => revision2::VoiceMemberProof::Absent,
        },
    }
}

fn migrate_ogg_metadata(source: &revision1::OggMetadata) -> revision2::OggMetadata {
    revision2::OggMetadata {
        codec: match source.codec {
            revision1::OggCodec::Vorbis => revision2::OggCodec::Vorbis,
            revision1::OggCodec::Opus => revision2::OggCodec::Opus,
        },
        channels: source.channels,
        sample_rate: source.sample_rate,
        pages: source.pages,
        logical_streams: source.logical_streams,
    }
}

const fn migrate_take_status(source: revision1::VoiceTakeStatus) -> revision2::VoiceTakeStatus {
    match source {
        revision1::VoiceTakeStatus::Draft => revision2::VoiceTakeStatus::Draft,
        revision1::VoiceTakeStatus::Recorded => revision2::VoiceTakeStatus::Recorded,
        revision1::VoiceTakeStatus::Reviewed => revision2::VoiceTakeStatus::Reviewed,
        revision1::VoiceTakeStatus::Approved => revision2::VoiceTakeStatus::Approved,
    }
}
