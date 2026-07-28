//! Bounded semantic projection of one closed revision-3 authoring project.
//!
//! This is the shared native read model for a future Mod Studio content library. It deliberately
//! omits generated source and artifact bytes, preserves exact typed-reference resolution, and
//! makes no build, publication, deployment, or runtime claim.

use std::collections::BTreeMap;
use std::io::{self, Write};

use serde::Serialize;

use crate::model_revision3::{
    quest_collision_artifact_media_for_layer, EntityKind, EntityPayload, ItemScalarTypeV1,
    ItemScalarValueV1, OggCodec, OriginRef, ProjectRevision3, ProjectRevision3ValidationError,
    ScriptModuleStatus, VoiceTakeStatus, VoiceTargetResolution,
};
use crate::{
    ContentSeal, EntityId, GameGenerationAnchor, LocaleCode, ProjectId, Sha256Digest,
    DATAASSET_FIXED_LEAF_COMPONENT_MEDIA_TYPE_V1,
    DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
};

/// Closed projection schema. A wire change requires a new version instead of widening this one.
pub const REVISION3_CONTENT_INDEX_SCHEMA_V1: u32 = 1;
/// Prevent one reference-heavy entity graph from becoming an unbounded UI/FFI response.
pub const MAX_REVISION3_CONTENT_REFERENCES_V1: usize = 1_000_000;
/// The projection excludes source/artifact bytes and must remain below this serialized cap.
pub const MAX_REVISION3_CONTENT_INDEX_JSON_BYTES_V1: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ContentIndexV1 {
    pub schema_revision: u32,
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub project_name: String,
    pub project_version: String,
    pub project_author: String,
    pub target: GameGenerationAnchor,
    pub authoring_locales: Vec<LocaleCode>,
    pub entity_counts: BTreeMap<EntityKind, u64>,
    pub entities: Vec<Revision3ContentEntityV1>,
    pub assets: Vec<Revision3ContentAssetV1>,
}

impl Revision3ContentIndexV1 {
    /// Serialize the deterministic projection without ever allocating beyond its response cap.
    pub fn to_canonical_json(&self) -> Result<String, Revision3ContentIndexJsonErrorV1> {
        let mut writer = BoundedWriter::new(MAX_REVISION3_CONTENT_INDEX_JSON_BYTES_V1);
        let result = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3ContentIndexJsonErrorV1::TooLarge {
                actual,
                limit: MAX_REVISION3_CONTENT_INDEX_JSON_BYTES_V1,
            });
        }
        result.map_err(Revision3ContentIndexJsonErrorV1::Serialize)?;
        String::from_utf8(writer.bytes)
            .map_err(|_| Revision3ContentIndexJsonErrorV1::NonUtf8Serialization)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ContentEntityV1 {
    pub id: EntityId,
    pub kind: EntityKind,
    pub display_name: String,
    pub revision: u64,
    pub origin: Revision3ContentOriginV1,
    pub summary: Revision3ContentEntitySummaryV1,
    pub references: Vec<Revision3ContentReferenceV1>,
    pub asset_references: Vec<Revision3ContentAssetReferenceV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Revision3ContentOriginV1 {
    New {
        authored_runtime_id: String,
    },
    Vanilla {
        generation: GameGenerationAnchor,
        catalog_layer: String,
        canonical_selector: String,
        source_seal: ContentSeal,
    },
    Imported {
        importer: String,
        source_seal: ContentSeal,
        external_identity: Option<String>,
    },
    Generated {
        generator_id: String,
        generator_version: u32,
        owner: Revision3ContentReferenceTargetV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Revision3ContentEntitySummaryV1 {
    LocalizationEntry {
        loc_id: String,
        locales: Vec<LocaleCode>,
    },
    DialogLine {
        speaker_hint: Option<String>,
        voice_slot_locales: Vec<LocaleCode>,
    },
    VoiceSlot {
        locale: LocaleCode,
        target_resolution: Revision3VoiceTargetResolutionV1,
        candidate_count: u64,
        has_selected_take: bool,
    },
    VoiceTake {
        locale: LocaleCode,
        status: VoiceTakeStatus,
        codec: OggCodec,
        channels: u8,
        sample_rate: u32,
    },
    NpcDraft {
        unique_name: String,
        module_namespace: String,
        parent_character_definition: String,
        parent_ai_agent_config: String,
        parent_spawn_definition: String,
        greeting_count: u64,
    },
    QuestDraft {
        technical_id: String,
        title: String,
        objective_title: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        additional_objective_titles: Vec<String>,
        objective_slots: Vec<u16>,
        transcript_count: u64,
        module_namespace: String,
        parent_runtime_class: String,
        giver_runtime_unique_name: String,
    },
    ScriptModule {
        generator_id: String,
        generator_version: u32,
        module_namespace: String,
        module_relative_path: String,
        status: ScriptModuleStatus,
    },
    ItemPatch {
        vanilla_class: String,
        field_count: u64,
        field_types: BTreeMap<String, ItemScalarTypeV1>,
        fields: BTreeMap<String, ItemScalarValueV1>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3VoiceTargetResolutionV1 {
    Unresolved,
    Ambiguous,
    Resolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ContentReferenceRoleV1 {
    OriginOwner,
    DialogLocalization,
    DialogVoiceSlot,
    VoiceCandidate,
    VoiceSelected,
    QuestTranscriptLine,
    DraftScriptModule,
    NpcGreetingLine,
    ScriptOwner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ContentReferenceTargetV1 {
    pub project_id: ProjectId,
    pub entity_id: EntityId,
    pub expected_kind: EntityKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ContentReferenceV1 {
    pub role: Revision3ContentReferenceRoleV1,
    pub qualifier: Option<String>,
    pub target: Revision3ContentReferenceTargetV1,
    pub resolution: Revision3ContentReferenceResolutionV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ContentReferenceResolutionV1 {
    Resolved,
    ForeignProject,
    MissingEntity,
    KindMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ContentAssetReferenceRoleV1 {
    VoiceAudio,
    QuestCollisionArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ContentAssetReferenceV1 {
    pub role: Revision3ContentAssetReferenceRoleV1,
    pub sha256: Sha256Digest,
    pub byte_len: u64,
    pub logical_name: Option<String>,
    pub expected_media_type: String,
    pub resolution: Revision3ContentAssetReferenceResolutionV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ContentAssetReferenceResolutionV1 {
    Resolved,
    MissingAsset,
    ByteLengthMismatch,
    MediaTypeMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Revision3ContentAssetClassV1 {
    VoiceAudio,
    QuestCollisionArtifact,
    DataAssetStageManifest,
    DataAssetStageComponent,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3ContentAssetV1 {
    pub sha256: Sha256Digest,
    pub byte_len: u64,
    pub media_type: String,
    pub class: Revision3ContentAssetClassV1,
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3ContentIndexErrorV1 {
    #[error("invalid revision-3 project: {0}")]
    InvalidProject(#[source] ProjectRevision3ValidationError),
    #[error("revision-3 content graph has more than {limit} typed references")]
    TooManyReferences { limit: usize },
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3ContentIndexJsonErrorV1 {
    #[error("revision-3 content index exceeds the {limit}-byte limit: {actual} bytes")]
    TooLarge { actual: usize, limit: usize },
    #[error("could not serialize revision-3 content index: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("revision-3 content index serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

/// Build one deterministic, source-free semantic content projection.
pub fn build_revision3_content_index_v1(
    project: &ProjectRevision3,
) -> Result<Revision3ContentIndexV1, Revision3ContentIndexErrorV1> {
    project
        .validate_closed_model()
        .map_err(Revision3ContentIndexErrorV1::InvalidProject)?;

    let mut entity_counts = BTreeMap::new();
    let mut entities = Vec::with_capacity(project.entities.len());
    let mut reference_count = 0usize;

    for entity in project.entities.values() {
        *entity_counts.entry(entity.kind()).or_insert(0u64) += 1;
        let mut references = Vec::new();
        let mut asset_references = Vec::new();

        if let OriginRef::Generated { owner, .. } = &entity.origin {
            push_reference(
                project,
                &mut references,
                &mut reference_count,
                Revision3ContentReferenceRoleV1::OriginOwner,
                None,
                owner,
            )?;
        }

        let summary = match &entity.payload {
            EntityPayload::LocalizationEntry(value) => {
                Revision3ContentEntitySummaryV1::LocalizationEntry {
                    loc_id: value.loc_id.clone(),
                    locales: value.texts.keys().cloned().collect(),
                }
            }
            EntityPayload::DialogLine(value) => {
                push_reference(
                    project,
                    &mut references,
                    &mut reference_count,
                    Revision3ContentReferenceRoleV1::DialogLocalization,
                    None,
                    &value.localization,
                )?;
                for (locale, target) in &value.voice_slots {
                    push_reference(
                        project,
                        &mut references,
                        &mut reference_count,
                        Revision3ContentReferenceRoleV1::DialogVoiceSlot,
                        Some(locale.to_string()),
                        target,
                    )?;
                }
                Revision3ContentEntitySummaryV1::DialogLine {
                    speaker_hint: value.speaker_hint.clone(),
                    voice_slot_locales: value.voice_slots.keys().cloned().collect(),
                }
            }
            EntityPayload::VoiceSlot(value) => {
                for target in &value.candidates {
                    push_reference(
                        project,
                        &mut references,
                        &mut reference_count,
                        Revision3ContentReferenceRoleV1::VoiceCandidate,
                        None,
                        target,
                    )?;
                }
                if let Some(target) = &value.selected {
                    push_reference(
                        project,
                        &mut references,
                        &mut reference_count,
                        Revision3ContentReferenceRoleV1::VoiceSelected,
                        None,
                        target,
                    )?;
                }
                Revision3ContentEntitySummaryV1::VoiceSlot {
                    locale: value.locale.clone(),
                    target_resolution: match value.target_resolution {
                        VoiceTargetResolution::Unresolved => {
                            Revision3VoiceTargetResolutionV1::Unresolved
                        }
                        VoiceTargetResolution::Ambiguous { .. } => {
                            Revision3VoiceTargetResolutionV1::Ambiguous
                        }
                        VoiceTargetResolution::Resolved { .. } => {
                            Revision3VoiceTargetResolutionV1::Resolved
                        }
                    },
                    candidate_count: value.candidates.len() as u64,
                    has_selected_take: value.selected.is_some(),
                }
            }
            EntityPayload::VoiceTake(value) => {
                asset_references.push(asset_reference(
                    project,
                    Revision3ContentAssetReferenceRoleV1::VoiceAudio,
                    value.asset.sha256,
                    value.asset.byte_len,
                    Some(value.asset.logical_name.clone()),
                    "audio/ogg",
                ));
                Revision3ContentEntitySummaryV1::VoiceTake {
                    locale: value.locale.clone(),
                    status: value.status,
                    codec: value.ogg.codec,
                    channels: value.ogg.channels,
                    sample_rate: value.ogg.sample_rate,
                }
            }
            EntityPayload::NpcDraft(value) => {
                push_reference(
                    project,
                    &mut references,
                    &mut reference_count,
                    Revision3ContentReferenceRoleV1::DraftScriptModule,
                    None,
                    &value.script_module,
                )?;
                for binding in &value.greetings {
                    push_reference(
                        project,
                        &mut references,
                        &mut reference_count,
                        Revision3ContentReferenceRoleV1::NpcGreetingLine,
                        None,
                        &binding.line,
                    )?;
                }
                Revision3ContentEntitySummaryV1::NpcDraft {
                    unique_name: value.input.unique_name.clone(),
                    module_namespace: value.input.module_namespace.clone(),
                    greeting_count: value.greetings.len() as u64,
                    parent_character_definition: value
                        .input
                        .parent_character_definition
                        .runtime_class
                        .clone(),
                    parent_ai_agent_config: value
                        .input
                        .parent_ai_agent_config
                        .runtime_class
                        .clone(),
                    parent_spawn_definition: value
                        .input
                        .parent_spawn_definition
                        .runtime_class
                        .clone(),
                }
            }
            EntityPayload::QuestDraft(value) => {
                push_reference(
                    project,
                    &mut references,
                    &mut reference_count,
                    Revision3ContentReferenceRoleV1::DraftScriptModule,
                    None,
                    &value.script_module,
                )?;
                for binding in &value.transcript {
                    push_reference(
                        project,
                        &mut references,
                        &mut reference_count,
                        Revision3ContentReferenceRoleV1::QuestTranscriptLine,
                        Some(binding.objective_slot.to_string()),
                        &binding.line,
                    )?;
                }
                let expected_media_type = quest_collision_artifact_media_for_layer(
                    &value.input.collision_catalog.catalog_layer,
                )
                .unwrap_or(QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2);
                asset_references.push(asset_reference(
                    project,
                    Revision3ContentAssetReferenceRoleV1::QuestCollisionArtifact,
                    value.input.collision_catalog.artifact.sha256,
                    value.input.collision_catalog.artifact.byte_len,
                    None,
                    expected_media_type,
                ));
                Revision3ContentEntitySummaryV1::QuestDraft {
                    technical_id: value.input.technical_id.clone(),
                    title: value.input.title.clone(),
                    objective_title: value.input.objective_title.clone(),
                    additional_objective_titles: value.input.additional_objective_titles.clone(),
                    objective_slots: value.input.transition_plan.objective_order.clone(),
                    transcript_count: value.transcript.len() as u64,
                    module_namespace: value.input.module_namespace.clone(),
                    parent_runtime_class: value.input.parent_quest.runtime_class.clone(),
                    giver_runtime_unique_name: value.input.giver.runtime_unique_name.clone(),
                }
            }
            EntityPayload::ScriptModule(value) => {
                push_reference(
                    project,
                    &mut references,
                    &mut reference_count,
                    Revision3ContentReferenceRoleV1::ScriptOwner,
                    None,
                    &value.owner,
                )?;
                Revision3ContentEntitySummaryV1::ScriptModule {
                    generator_id: value.generator_id.clone(),
                    generator_version: value.generator_version,
                    module_namespace: value.module_namespace.clone(),
                    module_relative_path: value.module_relative_path.clone(),
                    status: value.status,
                }
            }
            EntityPayload::ItemPatch(value) => Revision3ContentEntitySummaryV1::ItemPatch {
                vanilla_class: value.vanilla_class.clone(),
                field_count: value.fields.len() as u64,
                field_types: value
                    .fields
                    .iter()
                    .map(|(name, value)| (name.clone(), value.scalar_type()))
                    .collect(),
                fields: value.fields.clone(),
            },
        };

        entities.push(Revision3ContentEntityV1 {
            id: entity.id,
            kind: entity.kind(),
            display_name: entity.display_name.clone(),
            revision: entity.revision,
            origin: origin_view(&entity.origin),
            summary,
            references,
            asset_references,
        });
    }

    let assets = project
        .asset_store
        .assets
        .iter()
        .map(|(sha256, meta)| Revision3ContentAssetV1 {
            sha256: *sha256,
            byte_len: meta.byte_len,
            media_type: meta.media_type.clone(),
            class: classify_asset(&meta.media_type),
        })
        .collect();

    Ok(Revision3ContentIndexV1 {
        schema_revision: REVISION3_CONTENT_INDEX_SCHEMA_V1,
        project_id: project.project_id,
        project_revision: project.revision,
        project_name: project.meta.name.clone(),
        project_version: project.meta.version.clone(),
        project_author: project.meta.author.clone(),
        target: project.target.clone(),
        authoring_locales: project.authoring_locales.iter().cloned().collect(),
        entity_counts,
        entities,
        assets,
    })
}

fn origin_view(origin: &OriginRef) -> Revision3ContentOriginV1 {
    match origin {
        OriginRef::New {
            authored_runtime_id,
        } => Revision3ContentOriginV1::New {
            authored_runtime_id: authored_runtime_id.clone(),
        },
        OriginRef::Vanilla {
            generation,
            catalog_layer,
            canonical_selector,
            source_seal,
        } => Revision3ContentOriginV1::Vanilla {
            generation: generation.clone(),
            catalog_layer: catalog_layer.clone(),
            canonical_selector: canonical_selector.clone(),
            source_seal: source_seal.clone(),
        },
        OriginRef::Imported {
            importer,
            source_seal,
            external_identity,
        } => Revision3ContentOriginV1::Imported {
            importer: importer.clone(),
            source_seal: source_seal.clone(),
            external_identity: external_identity.clone(),
        },
        OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } => Revision3ContentOriginV1::Generated {
            generator_id: generator_id.clone(),
            generator_version: *generator_version,
            owner: reference_target(owner),
        },
    }
}

fn push_reference(
    project: &ProjectRevision3,
    output: &mut Vec<Revision3ContentReferenceV1>,
    count: &mut usize,
    role: Revision3ContentReferenceRoleV1,
    qualifier: Option<String>,
    target: &crate::model_revision3::TypedRef,
) -> Result<(), Revision3ContentIndexErrorV1> {
    *count = count
        .checked_add(1)
        .ok_or(Revision3ContentIndexErrorV1::TooManyReferences {
            limit: MAX_REVISION3_CONTENT_REFERENCES_V1,
        })?;
    if *count > MAX_REVISION3_CONTENT_REFERENCES_V1 {
        return Err(Revision3ContentIndexErrorV1::TooManyReferences {
            limit: MAX_REVISION3_CONTENT_REFERENCES_V1,
        });
    }
    let resolution = if target.project_id != project.project_id {
        Revision3ContentReferenceResolutionV1::ForeignProject
    } else {
        match project.entities.get(&target.id) {
            None => Revision3ContentReferenceResolutionV1::MissingEntity,
            Some(entity) if entity.kind() != target.expected_kind => {
                Revision3ContentReferenceResolutionV1::KindMismatch
            }
            Some(_) => Revision3ContentReferenceResolutionV1::Resolved,
        }
    };
    output.push(Revision3ContentReferenceV1 {
        role,
        qualifier,
        target: reference_target(target),
        resolution,
    });
    Ok(())
}

fn reference_target(
    target: &crate::model_revision3::TypedRef,
) -> Revision3ContentReferenceTargetV1 {
    Revision3ContentReferenceTargetV1 {
        project_id: target.project_id,
        entity_id: target.id,
        expected_kind: target.expected_kind,
    }
}

fn asset_reference(
    project: &ProjectRevision3,
    role: Revision3ContentAssetReferenceRoleV1,
    sha256: Sha256Digest,
    byte_len: u64,
    logical_name: Option<String>,
    expected_media_type: &str,
) -> Revision3ContentAssetReferenceV1 {
    let resolution = match project.asset_store.assets.get(&sha256) {
        None => Revision3ContentAssetReferenceResolutionV1::MissingAsset,
        Some(meta) if meta.byte_len != byte_len => {
            Revision3ContentAssetReferenceResolutionV1::ByteLengthMismatch
        }
        Some(meta) if meta.media_type != expected_media_type => {
            Revision3ContentAssetReferenceResolutionV1::MediaTypeMismatch
        }
        Some(_) => Revision3ContentAssetReferenceResolutionV1::Resolved,
    };
    Revision3ContentAssetReferenceV1 {
        role,
        sha256,
        byte_len,
        logical_name,
        expected_media_type: expected_media_type.to_owned(),
        resolution,
    }
}

fn classify_asset(media_type: &str) -> Revision3ContentAssetClassV1 {
    match media_type {
        "audio/ogg" => Revision3ContentAssetClassV1::VoiceAudio,
        QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2 => {
            Revision3ContentAssetClassV1::QuestCollisionArtifact
        }
        DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1 => {
            Revision3ContentAssetClassV1::DataAssetStageManifest
        }
        DATAASSET_FIXED_LEAF_COMPONENT_MEDIA_TYPE_V1 => {
            Revision3ContentAssetClassV1::DataAssetStageComponent
        }
        _ => Revision3ContentAssetClassV1::Other,
    }
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(64 * 1024)),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let actual = self.bytes.len().saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other("revision-3 content index limit exceeded"));
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

    use sha2::{Digest, Sha256};

    use super::*;
    use crate::model_revision3::{
        DialogLine, Entity, NpcDraft, NpcDraftInput, NpcParentClassInput, OggMetadata,
        QuestCollisionArtifactRef, QuestDraft, QuestDraftInput, QuestGiverInput, QuestParentInput,
        ScriptModule, TypedRef, VoiceSlot, VoiceTake,
    };
    use crate::{
        AssetMeta, AssetRef, AssetStoreIndex, FormatV2, ProjectMeta, SchemaRevisionV3,
        QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_QUEST_GENERATOR_ID,
        REVISION3_QUEST_GENERATOR_VERSION,
    };

    fn project_id(value: u8) -> ProjectId {
        ProjectId::from_bytes([value; 16])
    }

    fn entity_id(value: u8) -> EntityId {
        EntityId::from_bytes([value; 16])
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

    fn target() -> GameGenerationAnchor {
        GameGenerationAnchor {
            executable: seal(0xa0, 123),
        }
    }

    fn locale(value: &str) -> LocaleCode {
        value.parse().unwrap()
    }

    fn new_origin(value: &str) -> OriginRef {
        OriginRef::New {
            authored_runtime_id: value.to_owned(),
        }
    }

    fn parent(runtime_class: &str) -> NpcParentClassInput {
        NpcParentClassInput {
            generation: target(),
            source_seal: seal(0xb0, 4),
            catalog_layer: "fixture.npcs.v1".to_owned(),
            canonical_selector: runtime_class.to_owned(),
            runtime_class: runtime_class.to_owned(),
        }
    }

    fn base_project() -> ProjectRevision3 {
        ProjectRevision3 {
            format: FormatV2,
            schema_revision: SchemaRevisionV3,
            project_id: project_id(1),
            revision: 7,
            meta: ProjectMeta {
                name: "Semantic fixture".to_owned(),
                version: "0.1.0".to_owned(),
                author: "GORE".to_owned(),
            },
            target: target(),
            authoring_locales: BTreeSet::from([locale("de"), locale("en")]),
            entities: BTreeMap::new(),
            asset_store: AssetStoreIndex::default(),
        }
    }

    #[test]
    fn projection_is_semantic_deterministic_and_never_copies_generated_source() {
        let mut project = base_project();
        let localization_id = entity_id(1);
        let dialog_id = entity_id(2);
        let slot_id = entity_id(3);
        let take_id = entity_id(4);
        let npc_id = entity_id(5);
        let npc_module_id = entity_id(6);
        let audio = digest(0x31);

        project.asset_store.assets.insert(
            audio,
            AssetMeta {
                byte_len: 10,
                media_type: "audio/ogg".to_owned(),
            },
        );
        project.asset_store.assets.insert(
            digest(0x32),
            AssetMeta {
                byte_len: 12,
                media_type: DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1.to_owned(),
            },
        );

        project.entities.insert(
            localization_id,
            Entity {
                id: localization_id,
                display_name: "Greeting".to_owned(),
                origin: new_origin("LOC_GREETING"),
                revision: 1,
                payload: EntityPayload::LocalizationEntry(
                    crate::model_revision3::LocalizationEntry {
                        loc_id: "LOC_GREETING".to_owned(),
                        texts: BTreeMap::from([
                            (locale("de"), "Hallo".to_owned()),
                            (locale("en"), "Hello".to_owned()),
                        ]),
                    },
                ),
            },
        );
        project.entities.insert(
            dialog_id,
            Entity {
                id: dialog_id,
                display_name: "Greeting line".to_owned(),
                origin: new_origin("DIA_GREETING"),
                revision: 0,
                payload: EntityPayload::DialogLine(DialogLine {
                    localization: TypedRef::new(
                        project.project_id,
                        localization_id,
                        EntityKind::LocalizationEntry,
                    ),
                    speaker_hint: Some("Asghan".to_owned()),
                    voice_slots: BTreeMap::from([(
                        locale("de"),
                        TypedRef::new(project.project_id, slot_id, EntityKind::VoiceSlot),
                    )]),
                }),
            },
        );
        project.entities.insert(
            slot_id,
            Entity {
                id: slot_id,
                display_name: "Greeting DE".to_owned(),
                origin: new_origin("VOICE_GREETING_DE"),
                revision: 0,
                payload: EntityPayload::VoiceSlot(VoiceSlot {
                    locale: locale("de"),
                    target_resolution: VoiceTargetResolution::Unresolved,
                    candidates: vec![TypedRef::new(
                        project.project_id,
                        take_id,
                        EntityKind::VoiceTake,
                    )],
                    selected: Some(TypedRef::new(
                        project.project_id,
                        take_id,
                        EntityKind::VoiceTake,
                    )),
                }),
            },
        );
        project.entities.insert(
            take_id,
            Entity {
                id: take_id,
                display_name: "Take 1".to_owned(),
                origin: new_origin("TAKE_GREETING_DE_1"),
                revision: 2,
                payload: EntityPayload::VoiceTake(VoiceTake {
                    locale: locale("de"),
                    asset: AssetRef {
                        sha256: audio,
                        byte_len: 10,
                        logical_name: "greeting_de.ogg".to_owned(),
                    },
                    ogg: OggMetadata {
                        codec: OggCodec::Vorbis,
                        channels: 1,
                        sample_rate: 48_000,
                        pages: 2,
                        logical_streams: 1,
                    },
                    status: VoiceTakeStatus::Approved,
                }),
            },
        );
        let npc_owner = TypedRef::new(project.project_id, npc_id, EntityKind::NpcDraft);
        let npc = NpcDraft {
            generator_id: crate::LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
            generator_version: crate::LOGICAL_NPC_CLONE_GENERATOR_VERSION,
            input: NpcDraftInput {
                target: target(),
                module_namespace: "PROJECT.NPCS.GATEGUARD".to_owned(),
                unique_name: "GORE_GATE_GUARD".to_owned(),
                parent_character_definition: parent("UCharacterDefinition_Asghan"),
                parent_ai_agent_config: parent("UAIAgentConfig_Asghan"),
                parent_spawn_definition: parent("USpawnAIAgentDefinition_Asghan"),
            },
            script_module: TypedRef::new(
                project.project_id,
                npc_module_id,
                EntityKind::ScriptModule,
            ),
            greetings: Vec::new(),
        };
        let npc_module = npc.regenerate_script_module(npc_owner.clone()).unwrap();
        project.entities.insert(
            npc_id,
            Entity {
                id: npc_id,
                display_name: "Gate Guard".to_owned(),
                origin: new_origin("GORE_GATE_GUARD"),
                revision: 0,
                payload: EntityPayload::NpcDraft(npc),
            },
        );
        project.entities.insert(
            npc_module_id,
            Entity {
                id: npc_module_id,
                display_name: "Gate Guard source".to_owned(),
                origin: OriginRef::Generated {
                    generator_id: crate::LOGICAL_NPC_CLONE_GENERATOR_ID.to_owned(),
                    generator_version: crate::LOGICAL_NPC_CLONE_GENERATOR_VERSION,
                    owner: npc_owner.clone(),
                },
                revision: 0,
                payload: EntityPayload::ScriptModule(npc_module),
            },
        );

        let index = build_revision3_content_index_v1(&project).unwrap();
        assert_eq!(index.entities.len(), 6);
        assert_eq!(index.entity_counts[&EntityKind::NpcDraft], 1);
        assert_eq!(
            index.assets[1].class,
            Revision3ContentAssetClassV1::DataAssetStageManifest
        );

        let slot = index
            .entities
            .iter()
            .find(|entry| entry.id == slot_id)
            .unwrap();
        assert_eq!(
            slot.references[0].resolution,
            Revision3ContentReferenceResolutionV1::Resolved
        );
        assert_eq!(
            slot.references[1].resolution,
            Revision3ContentReferenceResolutionV1::Resolved
        );

        let json = index.to_canonical_json().unwrap();
        assert_eq!(json, index.to_canonical_json().unwrap());
        assert!(!json.contains("class UCharacterDefinition_Human_GORE_GATE_GUARD"));
        assert!(json.contains("GORE_GATE_GUARD"));
        assert!(json.contains("voice_selected"));
    }

    #[test]
    fn quest_projection_retains_semantics_and_only_artifact_metadata() {
        let mut project = base_project();
        let quest_id = entity_id(20);
        let module_id = entity_id(21);
        let artifact = seal(0x71, 42);
        let source = "class UQuest_Gore_Test {}";
        let source_sha256 = Sha256Digest::from_bytes(Sha256::digest(source.as_bytes()).into());
        let owner = TypedRef::new(project.project_id, quest_id, EntityKind::QuestDraft);

        project.asset_store.assets.insert(
            artifact.sha256,
            AssetMeta {
                byte_len: artifact.byte_len,
                media_type: QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2.to_owned(),
            },
        );
        project.entities.insert(
            quest_id,
            Entity {
                id: quest_id,
                display_name: "A test quest".to_owned(),
                origin: new_origin("GORE_QUEST_TEST"),
                revision: 0,
                payload: EntityPayload::QuestDraft(QuestDraft {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    input: QuestDraftInput {
                        target: target(),
                        quest_id,
                        module_namespace: "PROJECT.QUESTS.TEST".to_owned(),
                        technical_id: "GORE_QUEST_TEST".to_owned(),
                        text_helper: "GORE_QUEST_TEST_TEXT".to_owned(),
                        parent_quest: QuestParentInput {
                            generation: target(),
                            source_seal: seal(0x72, 4),
                            catalog_layer: "fixture.quests.v1".to_owned(),
                            canonical_selector: "UQuest".to_owned(),
                            runtime_class: "UQuest".to_owned(),
                        },
                        giver: QuestGiverInput {
                            generation: target(),
                            source_seal: seal(0x73, 4),
                            catalog_layer: "fixture.npcs.v1".to_owned(),
                            canonical_selector: "Asghan".to_owned(),
                            runtime_unique_name: "ASGHAN".to_owned(),
                        },
                        title: "The test".to_owned(),
                        description: "Do the thing".to_owned(),
                        objective_title: "Thing".to_owned(),
                        additional_objective_titles: vec![
                            "Inspect the thing".to_owned(),
                            "Report the thing".to_owned(),
                        ],
                        transition_plan: Box::new(
                            crate::QuestTransitionPlanV1::default_for_objectives(3).unwrap(),
                        ),
                        collision_catalog: QuestCollisionArtifactRef {
                            generation: target(),
                            catalog_layer: QUEST_COLLISION_CATALOG_LAYER_V2.to_owned(),
                            artifact: artifact.clone(),
                            source_seal: seal(0x74, artifact.byte_len),
                            basis_snapshot: seal(0x75, 10),
                        },
                    },
                    script_module: TypedRef::new(
                        project.project_id,
                        module_id,
                        EntityKind::ScriptModule,
                    ),
                    transcript: Vec::new(),
                }),
            },
        );
        project.entities.insert(
            module_id,
            Entity {
                id: module_id,
                display_name: "A test quest source".to_owned(),
                origin: OriginRef::Generated {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner: owner.clone(),
                },
                revision: 0,
                payload: EntityPayload::ScriptModule(ScriptModule {
                    generator_id: REVISION3_QUEST_GENERATOR_ID.to_owned(),
                    generator_version: REVISION3_QUEST_GENERATOR_VERSION,
                    owner,
                    module_namespace: "PROJECT.QUESTS.TEST".to_owned(),
                    module_relative_path: "Project/Quests/Test.as".to_owned(),
                    source: source.to_owned(),
                    source_sha256,
                    input_fingerprint: digest(0x76),
                    status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
                }),
            },
        );

        let index = build_revision3_content_index_v1(&project).unwrap();
        let quest = index
            .entities
            .iter()
            .find(|entry| entry.id == quest_id)
            .unwrap();
        assert!(matches!(
            &quest.summary,
            Revision3ContentEntitySummaryV1::QuestDraft {
                technical_id,
                title,
                additional_objective_titles,
                giver_runtime_unique_name,
                ..
            } if technical_id == "GORE_QUEST_TEST"
                && title == "The test"
                && additional_objective_titles == &["Inspect the thing", "Report the thing"]
                && giver_runtime_unique_name == "ASGHAN"
        ));
        assert_eq!(
            quest.asset_references[0].resolution,
            Revision3ContentAssetReferenceResolutionV1::Resolved
        );
        let json = index.to_canonical_json().unwrap();
        assert!(!json.contains(source));
        assert!(!json.contains("Do the thing"));
        assert!(json.contains("\"objective_slots\":[1,2,3]"));
        assert!(json.contains(QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2));
    }

    #[test]
    fn quest_summary_always_spells_mandatory_objective_slots() {
        let summary = Revision3ContentEntitySummaryV1::QuestDraft {
            technical_id: "GORE_EMPTY_SLOTS".to_owned(),
            title: "Invalid fixture".to_owned(),
            objective_title: "Missing objective slot".to_owned(),
            additional_objective_titles: Vec::new(),
            objective_slots: Vec::new(),
            transcript_count: 0,
            module_namespace: "PROJECT.QUESTS.EMPTY_SLOTS".to_owned(),
            parent_runtime_class: "UQuest".to_owned(),
            giver_runtime_unique_name: "ASGHAN".to_owned(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"objective_slots\":[]"));
    }

    #[test]
    fn invalid_closed_project_is_rejected_before_projection() {
        let mut project = base_project();
        project.target.executable.byte_len = 0;
        assert!(matches!(
            build_revision3_content_index_v1(&project),
            Err(Revision3ContentIndexErrorV1::InvalidProject(
                ProjectRevision3ValidationError::InvalidTarget
            ))
        ));
    }
}
