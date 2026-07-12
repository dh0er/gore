//! Closed schema-revision-2 model for authoring format 2.
//!
//! Revision 2 deliberately owns every entity, payload, reference, and payload-supporting enum.
//! That keeps revision 1 frozen when later revision-2 work adds NPC or quest variants. Stable
//! value objects such as IDs, project metadata, generation anchors, and asset-store records remain
//! shared across schema revisions.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::marker::PhantomData;

use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::{
    ArchiveSeal, AssetRef, AssetStoreIndex, CatalogQualifiedParentQuest,
    CatalogQualifiedQuestGiver, ContentSeal, DraftQuestCollisionCatalog, DraftQuestSkeletonError,
    DraftQuestSkeletonInput, DraftQuestSkeletonV1, EntityId, FormatV2, GameGenerationAnchor,
    LocaleCode, LogicalNpcCloneDraft, LogicalNpcCloneDraftError, ProjectId, ProjectJsonError,
    ProjectMeta, Sha256Digest, DRAFT_QUEST_GENERATOR_ID, DRAFT_QUEST_GENERATOR_VERSION,
    LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION, MAX_PROJECT_JSON_BYTES,
};

/// Second closed schema revision carried inside authoring format 2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SchemaRevisionV2;

impl Serialize for SchemaRevisionV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(2)
    }
}

impl<'de> Deserialize<'de> for SchemaRevisionV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let revision = u32::deserialize(deserializer)?;
        if revision == 2 {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported authoring schema revision {revision}; expected 2"
            )))
        }
    }
}

/// Closed revision-2 entity kinds. This type is intentionally separate from revision 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    LocalizationEntry,
    DialogLine,
    VoiceSlot,
    VoiceTake,
    NpcDraft,
    QuestDraft,
    ScriptModule,
}

/// Revision-2 project-qualified authored reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedRef {
    pub project_id: ProjectId,
    pub id: EntityId,
    pub expected_kind: EntityKind,
}

impl TypedRef {
    pub const fn new(project_id: ProjectId, id: EntityId, expected_kind: EntityKind) -> Self {
        Self {
            project_id,
            id,
            expected_kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalizationEntry {
    pub loc_id: String,
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    pub texts: BTreeMap<LocaleCode, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogLine {
    pub localization: TypedRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_hint: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    pub voice_slots: BTreeMap<LocaleCode, TypedRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceOperation {
    Add,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum VoiceMemberProof {
    Present { uncompressed_size: u64, crc32: u32 },
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceTarget {
    pub archive: String,
    pub member: String,
    pub operation: VoiceOperation,
    pub archive_seal: ArchiveSeal,
    pub member_proof: VoiceMemberProof,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum VoiceTargetResolution {
    Unresolved,
    Ambiguous { candidates: Vec<VoiceTarget> },
    Resolved { target: VoiceTarget },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OggCodec {
    Vorbis,
    Opus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OggMetadata {
    pub codec: OggCodec,
    pub channels: u8,
    pub sample_rate: u32,
    pub pages: u32,
    pub logical_streams: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceTakeStatus {
    Draft,
    Recorded,
    Reviewed,
    Approved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceTake {
    pub locale: LocaleCode,
    pub asset: AssetRef,
    pub ogg: OggMetadata,
    pub status: VoiceTakeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceSlot {
    pub locale: LocaleCode,
    pub target_resolution: VoiceTargetResolution,
    #[serde(default)]
    pub candidates: Vec<TypedRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<TypedRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum OriginRef {
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        external_identity: Option<String>,
    },
    Generated {
        generator_id: String,
        generator_version: u32,
        owner: TypedRef,
    },
}

/// One sealed catalog identity for a parent class used by a revision-2 NPC draft.
///
/// The revision-1 NPC source generator accepts only the runtime class string. Revision 2 retains
/// the complete catalog provenance as well and binds it into its own input fingerprint so a seal
/// or selector cannot drift silently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcParentClassInput {
    pub generation: GameGenerationAnchor,
    pub source_seal: ContentSeal,
    pub catalog_layer: String,
    pub canonical_selector: String,
    pub runtime_class: String,
}

/// Complete, persistable intent and catalog provenance for one logical NPC clone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcDraftInput {
    pub target: GameGenerationAnchor,
    pub module_namespace: String,
    pub unique_name: String,
    pub parent_character_definition: NpcParentClassInput,
    pub parent_ai_agent_config: NpcParentClassInput,
    pub parent_spawn_definition: NpcParentClassInput,
}

/// Persisted NPC authoring intent. The script is a separate, typed, owned entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcDraft {
    pub generator_id: String,
    pub generator_version: u32,
    pub input: NpcDraftInput,
    pub script_module: TypedRef,
}

/// Exact sealed quest-giver input retained by a revision-2 quest draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestGiverInput {
    pub generation: GameGenerationAnchor,
    pub source_seal: ContentSeal,
    pub catalog_layer: String,
    pub canonical_selector: String,
    pub runtime_unique_name: String,
}

/// Exact sealed parent-quest input retained by a revision-2 quest draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestParentInput {
    pub generation: GameGenerationAnchor,
    pub source_seal: ContentSeal,
    pub catalog_layer: String,
    pub canonical_selector: String,
    pub runtime_class: String,
}

/// Complete collision inventory used by the quest generator, not merely its catalog seal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuestCollisionCatalogInput {
    pub generation: GameGenerationAnchor,
    pub source_seal: ContentSeal,
    pub catalog_layer: String,
    #[serde(default)]
    pub modules: BTreeSet<String>,
    #[serde(default)]
    pub relative_paths: BTreeSet<String>,
    #[serde(default)]
    pub symbols: BTreeSet<String>,
}

impl<'de> Deserialize<'de> for QuestCollisionCatalogInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "generation",
            "source_seal",
            "catalog_layer",
            "modules",
            "relative_paths",
            "symbols",
        ];

        struct CollisionCatalogVisitor;

        impl<'de> Visitor<'de> for CollisionCatalogVisitor {
            type Value = QuestCollisionCatalogInput;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bounded quest collision catalog object")
            }

            fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut generation = None;
                let mut source_seal = None;
                let mut catalog_layer = None;
                let mut modules = None;
                let mut relative_paths = None;
                let mut symbols = None;
                let mut remaining_count = crate::quest::MAX_COLLISION_ENTRIES;
                let mut remaining_bytes = crate::quest::MAX_COLLISION_TOTAL_BYTES;

                while let Some(field) = access.next_key::<String>()? {
                    match field.as_str() {
                        "generation" => {
                            set_once(&mut generation, access.next_value()?, "generation")?
                        }
                        "source_seal" => {
                            set_once(&mut source_seal, access.next_value()?, "source_seal")?
                        }
                        "catalog_layer" => {
                            set_once(&mut catalog_layer, access.next_value()?, "catalog_layer")?
                        }
                        "modules" => {
                            if modules.is_some() {
                                return Err(de::Error::duplicate_field("modules"));
                            }
                            let bounded = access.next_value_seed(CollisionSetSeed {
                                remaining_count,
                                remaining_bytes,
                            })?;
                            remaining_count -= bounded.count;
                            remaining_bytes -= bounded.bytes;
                            modules = Some(bounded.values);
                        }
                        "relative_paths" => {
                            if relative_paths.is_some() {
                                return Err(de::Error::duplicate_field("relative_paths"));
                            }
                            let bounded = access.next_value_seed(CollisionSetSeed {
                                remaining_count,
                                remaining_bytes,
                            })?;
                            remaining_count -= bounded.count;
                            remaining_bytes -= bounded.bytes;
                            relative_paths = Some(bounded.values);
                        }
                        "symbols" => {
                            if symbols.is_some() {
                                return Err(de::Error::duplicate_field("symbols"));
                            }
                            let bounded = access.next_value_seed(CollisionSetSeed {
                                remaining_count,
                                remaining_bytes,
                            })?;
                            remaining_count -= bounded.count;
                            remaining_bytes -= bounded.bytes;
                            symbols = Some(bounded.values);
                        }
                        _ => return Err(de::Error::unknown_field(&field, FIELDS)),
                    }
                }

                Ok(QuestCollisionCatalogInput {
                    generation: generation.ok_or_else(|| de::Error::missing_field("generation"))?,
                    source_seal: source_seal
                        .ok_or_else(|| de::Error::missing_field("source_seal"))?,
                    catalog_layer: catalog_layer
                        .ok_or_else(|| de::Error::missing_field("catalog_layer"))?,
                    modules: modules.unwrap_or_default(),
                    relative_paths: relative_paths.unwrap_or_default(),
                    symbols: symbols.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "QuestCollisionCatalogInput",
            FIELDS,
            CollisionCatalogVisitor,
        )
    }
}

fn set_once<E, T>(slot: &mut Option<T>, value: T, field: &'static str) -> Result<(), E>
where
    E: de::Error,
{
    if slot.replace(value).is_some() {
        Err(E::duplicate_field(field))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
struct BoundedCollisionSet {
    values: BTreeSet<String>,
    count: usize,
    bytes: usize,
}

struct CollisionSetSeed {
    remaining_count: usize,
    remaining_bytes: usize,
}

struct BoundedCollisionString(String);

impl<'de> Deserialize<'de> for BoundedCollisionString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BoundedStringVisitor;

        impl Visitor<'_> for BoundedStringVisitor {
            type Value = BoundedCollisionString;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "a collision string of at most {} bytes",
                    crate::quest::MAX_COLLISION_ENTRY_BYTES
                )
            }

            fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(value)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                validate_collision_string_length(value)?;
                Ok(BoundedCollisionString(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                validate_collision_string_length(&value)?;
                Ok(BoundedCollisionString(value))
            }
        }

        deserializer.deserialize_string(BoundedStringVisitor)
    }
}

fn validate_collision_string_length<E>(value: &str) -> Result<(), E>
where
    E: de::Error,
{
    if value.len() > crate::quest::MAX_COLLISION_ENTRY_BYTES {
        Err(E::custom(format!(
            "collision entry is {} bytes; maximum is {}",
            value.len(),
            crate::quest::MAX_COLLISION_ENTRY_BYTES
        )))
    } else {
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for CollisionSetSeed {
    type Value = BoundedCollisionSet;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CollisionSetVisitor {
            remaining_count: usize,
            remaining_bytes: usize,
        }

        impl<'de> Visitor<'de> for CollisionSetVisitor {
            type Value = BoundedCollisionSet;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bounded array of unique collision strings")
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                if sequence
                    .size_hint()
                    .is_some_and(|hint| hint > self.remaining_count)
                {
                    return Err(de::Error::custom(format!(
                        "collision catalog exceeds the remaining aggregate entry budget {}; maximum total is {}",
                        self.remaining_count,
                        crate::quest::MAX_COLLISION_ENTRIES,
                    )));
                }
                let mut values = BTreeSet::new();
                let mut count = 0usize;
                let mut bytes = 0usize;
                while let Some(BoundedCollisionString(value)) =
                    sequence.next_element::<BoundedCollisionString>()?
                {
                    count = count
                        .checked_add(1)
                        .ok_or_else(|| de::Error::custom("collision entry count overflow"))?;
                    if count > self.remaining_count {
                        return Err(de::Error::custom(format!(
                            "collision catalog exceeds the remaining aggregate entry budget {}; maximum total is {}",
                            self.remaining_count,
                            crate::quest::MAX_COLLISION_ENTRIES,
                        )));
                    }
                    bytes = bytes.checked_add(value.len()).ok_or_else(|| {
                        de::Error::custom("collision catalog byte count overflow")
                    })?;
                    if bytes > self.remaining_bytes {
                        return Err(de::Error::custom(format!(
                            "collision catalog exceeds the {}-byte limit",
                            crate::quest::MAX_COLLISION_TOTAL_BYTES
                        )));
                    }
                    if !values.insert(value) {
                        return Err(de::Error::custom("duplicate collision set value"));
                    }
                }
                Ok(BoundedCollisionSet {
                    values,
                    count,
                    bytes,
                })
            }
        }

        deserializer.deserialize_seq(CollisionSetVisitor {
            remaining_count: self.remaining_count,
            remaining_bytes: self.remaining_bytes,
        })
    }
}

/// Complete, persistable intent and provenance accepted by the discovery-only quest generator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestDraftInput {
    pub target: GameGenerationAnchor,
    pub quest_id: EntityId,
    pub module_namespace: String,
    pub technical_id: String,
    pub text_helper: String,
    pub parent_quest: QuestParentInput,
    pub giver: QuestGiverInput,
    pub title: String,
    pub description: String,
    pub objective_title: String,
    pub collision_catalog: QuestCollisionCatalogInput,
}

/// Persisted quest authoring intent. The generated script remains a separate owned entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestDraft {
    pub generator_id: String,
    pub generator_version: u32,
    pub input: QuestDraftInput,
    pub script_module: TypedRef,
}

/// Closed authoring status for generated revision-2 modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptAuthoringStatus {
    OfflineDraft,
}

/// Closed runtime status. There is deliberately no user-settable qualified variant or evidence
/// boolean; qualification requires a later schema and verified evidence model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptRuntimeStatus {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptModuleStatus {
    pub authoring: ScriptAuthoringStatus,
    pub runtime: ScriptRuntimeStatus,
}

impl ScriptModuleStatus {
    pub const OFFLINE_DRAFT_RUNTIME_UNQUALIFIED: Self = Self {
        authoring: ScriptAuthoringStatus::OfflineDraft,
        runtime: ScriptRuntimeStatus::RuntimeUnqualified,
    };
}

/// Deterministically generated source owned by exactly one NPC or quest draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptModule {
    pub generator_id: String,
    pub generator_version: u32,
    pub owner: TypedRef,
    pub module_namespace: String,
    pub module_relative_path: String,
    pub source: String,
    pub source_sha256: Sha256Digest,
    pub input_fingerprint: Sha256Digest,
    pub status: ScriptModuleStatus,
}

/// Generated names that must remain unique across every story module in one project.
///
/// Kept crate-private because this is validation/transaction evidence, not another durable wire
/// shape. The durable draft input remains the single source from which it is regenerated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedStoryIdentity {
    pub module_namespace: String,
    pub module_relative_path: String,
    pub symbols: Vec<String>,
}

/// Fail-closed errors returned while regenerating a persisted revision-2 story draft.
#[derive(Debug, thiserror::Error)]
pub enum StoryRegenerationError {
    #[error(
        "generator contract mismatch: expected {expected_id}@{expected_version}, got {actual_id}@{actual_version}"
    )]
    GeneratorContract {
        expected_id: &'static str,
        expected_version: u32,
        actual_id: String,
        actual_version: u32,
    },
    #[error("script owner reference must declare kind {expected:?}, got {actual:?}")]
    OwnerKind {
        expected: EntityKind,
        actual: EntityKind,
    },
    #[error("invalid NPC provenance: {0}")]
    InvalidNpcProvenance(String),
    #[error("invalid NPC generator intent: {0}")]
    InvalidNpcIntent(#[source] LogicalNpcCloneDraftError),
    #[error("could not fingerprint NPC generator input: {0}")]
    NpcFingerprint(#[source] serde_json::Error),
    #[error("invalid quest generator intent: {0}")]
    InvalidQuestIntent(#[source] DraftQuestSkeletonError),
}

impl NpcDraft {
    /// Rebuild the exact owned module from durable intent without filesystem or runtime actions.
    pub fn regenerate_script_module(
        &self,
        owner: TypedRef,
    ) -> Result<ScriptModule, StoryRegenerationError> {
        self.regenerate_script_module_with_identity(owner)
            .map(|(module, _)| module)
    }

    pub(crate) fn regenerate_script_module_with_identity(
        &self,
        owner: TypedRef,
    ) -> Result<(ScriptModule, GeneratedStoryIdentity), StoryRegenerationError> {
        validate_generator_contract(
            &self.generator_id,
            self.generator_version,
            LOGICAL_NPC_CLONE_GENERATOR_ID,
            LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        )?;
        if owner.expected_kind != EntityKind::NpcDraft {
            return Err(StoryRegenerationError::OwnerKind {
                expected: EntityKind::NpcDraft,
                actual: owner.expected_kind,
            });
        }
        validate_npc_input_provenance(&self.input)?;
        let draft = LogicalNpcCloneDraft::new(
            self.input.module_namespace.clone(),
            self.input.unique_name.clone(),
            self.input.parent_character_definition.runtime_class.clone(),
            self.input.parent_ai_agent_config.runtime_class.clone(),
            self.input.parent_spawn_definition.runtime_class.clone(),
        )
        .map_err(StoryRegenerationError::InvalidNpcIntent)?;
        let generated = draft.generate();
        let input_fingerprint = fingerprint_npc_revision2_input(&self.input)
            .map_err(StoryRegenerationError::NpcFingerprint)?;
        let identity = GeneratedStoryIdentity {
            module_namespace: generated.module_namespace.clone(),
            module_relative_path: generated.module_relative_path.clone(),
            symbols: vec![
                generated.classes.character_definition.clone(),
                generated.classes.ai_agent_config.clone(),
                generated.classes.spawn_definition.clone(),
            ],
        };
        let module = ScriptModule {
            generator_id: generated.generator_id.to_owned(),
            generator_version: generated.generator_version,
            owner,
            module_namespace: generated.module_namespace,
            module_relative_path: generated.module_relative_path,
            source: generated.source,
            source_sha256: generated.source_sha256,
            input_fingerprint,
            status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
        };
        Ok((module, identity))
    }
}

impl QuestDraft {
    /// Rebuild the exact owned module from durable intent without filesystem or runtime actions.
    pub fn regenerate_script_module(
        &self,
        owner: TypedRef,
    ) -> Result<ScriptModule, StoryRegenerationError> {
        self.regenerate_script_module_with_identity(owner)
            .map(|(module, _)| module)
    }

    pub(crate) fn regenerate_script_module_with_identity(
        &self,
        owner: TypedRef,
    ) -> Result<(ScriptModule, GeneratedStoryIdentity), StoryRegenerationError> {
        validate_generator_contract(
            &self.generator_id,
            self.generator_version,
            DRAFT_QUEST_GENERATOR_ID,
            DRAFT_QUEST_GENERATOR_VERSION,
        )?;
        if owner.expected_kind != EntityKind::QuestDraft {
            return Err(StoryRegenerationError::OwnerKind {
                expected: EntityKind::QuestDraft,
                actual: owner.expected_kind,
            });
        }
        validate_canonical_collision_inventory(&self.input.collision_catalog)?;
        let parent = &self.input.parent_quest;
        let giver = &self.input.giver;
        let collisions = &self.input.collision_catalog;
        let parent_quest = CatalogQualifiedParentQuest::new(
            parent.generation.clone(),
            parent.source_seal.clone(),
            parent.catalog_layer.clone(),
            parent.canonical_selector.clone(),
            parent.runtime_class.clone(),
        )
        .map_err(StoryRegenerationError::InvalidQuestIntent)?;
        let giver = CatalogQualifiedQuestGiver::new(
            giver.generation.clone(),
            giver.source_seal.clone(),
            giver.catalog_layer.clone(),
            giver.canonical_selector.clone(),
            giver.runtime_unique_name.clone(),
        )
        .map_err(StoryRegenerationError::InvalidQuestIntent)?;
        let collision_catalog = DraftQuestCollisionCatalog::new(
            collisions.generation.clone(),
            collisions.source_seal.clone(),
            collisions.catalog_layer.clone(),
            collisions.modules.iter().cloned().collect(),
            collisions.relative_paths.iter().cloned().collect(),
            collisions.symbols.iter().cloned().collect(),
        )
        .map_err(StoryRegenerationError::InvalidQuestIntent)?;
        let generated = DraftQuestSkeletonV1::new(DraftQuestSkeletonInput {
            target: self.input.target.clone(),
            quest_id: self.input.quest_id,
            module_namespace: self.input.module_namespace.clone(),
            technical_id: self.input.technical_id.clone(),
            text_helper: self.input.text_helper.clone(),
            parent_quest,
            giver,
            title: self.input.title.clone(),
            description: self.input.description.clone(),
            objective_title: self.input.objective_title.clone(),
            collision_catalog,
        })
        .map_err(StoryRegenerationError::InvalidQuestIntent)?
        .generate();
        let identity = GeneratedStoryIdentity {
            module_namespace: generated.technical_names.module_namespace.clone(),
            module_relative_path: generated.technical_names.module_relative_path.clone(),
            symbols: vec![
                generated.technical_names.root_class.clone(),
                generated.technical_names.objective_class.clone(),
                generated.technical_names.text_helper.clone(),
                generated.technical_names.root_getter.clone(),
                generated.technical_names.objective_getter.clone(),
            ],
        };
        let module = ScriptModule {
            generator_id: generated.generator_id.to_owned(),
            generator_version: generated.generator_version,
            owner,
            module_namespace: generated.technical_names.module_namespace,
            module_relative_path: generated.technical_names.module_relative_path,
            source: generated.source,
            source_sha256: generated.source_sha256,
            input_fingerprint: generated.input_fingerprint,
            status: ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED,
        };
        Ok((module, identity))
    }
}

fn validate_generator_contract(
    actual_id: &str,
    actual_version: u32,
    expected_id: &'static str,
    expected_version: u32,
) -> Result<(), StoryRegenerationError> {
    if actual_id == expected_id && actual_version == expected_version {
        Ok(())
    } else {
        Err(StoryRegenerationError::GeneratorContract {
            expected_id,
            expected_version,
            actual_id: actual_id.to_owned(),
            actual_version,
        })
    }
}

fn validate_npc_input_provenance(input: &NpcDraftInput) -> Result<(), StoryRegenerationError> {
    if input.target.executable.byte_len == 0 {
        return Err(StoryRegenerationError::InvalidNpcProvenance(
            "target executable seal has zero byte length".to_owned(),
        ));
    }
    for (label, parent) in [
        (
            "parent_character_definition",
            &input.parent_character_definition,
        ),
        ("parent_ai_agent_config", &input.parent_ai_agent_config),
        ("parent_spawn_definition", &input.parent_spawn_definition),
    ] {
        if parent.generation != input.target {
            return Err(StoryRegenerationError::InvalidNpcProvenance(format!(
                "{label} generation does not match target"
            )));
        }
        if parent.source_seal.byte_len == 0 {
            return Err(StoryRegenerationError::InvalidNpcProvenance(format!(
                "{label} source seal has zero byte length"
            )));
        }
        if !canonical_catalog_layer(&parent.catalog_layer) {
            return Err(StoryRegenerationError::InvalidNpcProvenance(format!(
                "{label} catalog layer is not a canonical lowercase identifier"
            )));
        }
        if !canonical_selector(&parent.canonical_selector) {
            return Err(StoryRegenerationError::InvalidNpcProvenance(format!(
                "{label} selector is not a canonical technical identifier"
            )));
        }
    }
    Ok(())
}

fn canonical_catalog_layer(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    let mut previous_separator = true;
    for byte in value.bytes() {
        let separator = matches!(byte, b'.' | b'-' | b'_');
        if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || separator)
            || (separator && previous_separator)
        {
            return false;
        }
        previous_separator = separator;
    }
    !previous_separator
}

fn canonical_selector(value: &str) -> bool {
    if value.is_empty() || value.len() > 96 {
        return false;
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !value.starts_with("__")
}

fn validate_canonical_collision_inventory(
    catalog: &QuestCollisionCatalogInput,
) -> Result<(), StoryRegenerationError> {
    for (kind, values) in [
        (crate::DraftQuestCollisionKind::Module, &catalog.modules),
        (
            crate::DraftQuestCollisionKind::RelativePath,
            &catalog.relative_paths,
        ),
        (crate::DraftQuestCollisionKind::Symbol, &catalog.symbols),
    ] {
        if let Some(value) = values
            .iter()
            .find(|value| value.to_ascii_lowercase() != value.as_str())
        {
            return Err(StoryRegenerationError::InvalidQuestIntent(
                DraftQuestSkeletonError::UnsafeCollisionEntry {
                    kind,
                    value: value.clone(),
                },
            ));
        }
    }
    Ok(())
}

fn fingerprint_npc_revision2_input(
    input: &NpcDraftInput,
) -> Result<Sha256Digest, serde_json::Error> {
    let canonical = serde_json::to_vec(input)?;
    let mut hasher = Sha256::new();
    hasher.update(b"gore-authoring.revision2.npc-draft.input-fingerprint\0");
    hasher.update((LOGICAL_NPC_CLONE_GENERATOR_ID.len() as u64).to_be_bytes());
    hasher.update(LOGICAL_NPC_CLONE_GENERATOR_ID.as_bytes());
    hasher.update(u64::from(LOGICAL_NPC_CLONE_GENERATOR_VERSION).to_be_bytes());
    hasher.update((canonical.len() as u64).to_be_bytes());
    hasher.update(canonical);
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}

/// Closed revision-2 payload set. Later variants can be added here without changing revision 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EntityPayload {
    LocalizationEntry(LocalizationEntry),
    DialogLine(DialogLine),
    VoiceSlot(VoiceSlot),
    VoiceTake(VoiceTake),
    NpcDraft(NpcDraft),
    QuestDraft(QuestDraft),
    ScriptModule(ScriptModule),
}

impl EntityPayload {
    pub const fn kind(&self) -> EntityKind {
        match self {
            Self::LocalizationEntry(_) => EntityKind::LocalizationEntry,
            Self::DialogLine(_) => EntityKind::DialogLine,
            Self::VoiceSlot(_) => EntityKind::VoiceSlot,
            Self::VoiceTake(_) => EntityKind::VoiceTake,
            Self::NpcDraft(_) => EntityKind::NpcDraft,
            Self::QuestDraft(_) => EntityKind::QuestDraft,
            Self::ScriptModule(_) => EntityKind::ScriptModule,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub id: EntityId,
    pub display_name: String,
    pub origin: OriginRef,
    #[serde(default)]
    pub revision: u64,
    pub payload: EntityPayload,
}

impl Entity {
    pub const fn kind(&self) -> EntityKind {
        self.payload.kind()
    }
}

/// Canonical authoring snapshot for format 2, schema revision 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectRevision2 {
    pub format: FormatV2,
    pub schema_revision: SchemaRevisionV2,
    pub project_id: ProjectId,
    #[serde(default)]
    pub revision: u64,
    pub meta: ProjectMeta,
    pub target: GameGenerationAnchor,
    #[serde(default, deserialize_with = "deserialize_unique_locales")]
    pub authoring_locales: BTreeSet<LocaleCode>,
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    pub entities: BTreeMap<EntityId, Entity>,
    pub asset_store: AssetStoreIndex,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectRevision2Wire {
    format: FormatV2,
    schema_revision: SchemaRevisionV2,
    project_id: ProjectId,
    #[serde(default)]
    revision: u64,
    meta: ProjectMeta,
    target: GameGenerationAnchor,
    #[serde(default, deserialize_with = "deserialize_unique_locales")]
    authoring_locales: BTreeSet<LocaleCode>,
    #[serde(default, deserialize_with = "deserialize_unique_btree_map")]
    entities: BTreeMap<EntityId, Entity>,
    asset_store: AssetStoreIndex,
}

impl<'de> Deserialize<'de> for ProjectRevision2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProjectRevision2Wire::deserialize(deserializer)?;
        for (key, entity) in &wire.entities {
            if key != &entity.id {
                return Err(de::Error::custom(format!(
                    "entity map key {key} does not match embedded id {}",
                    entity.id
                )));
            }
        }
        Ok(Self {
            format: wire.format,
            schema_revision: wire.schema_revision,
            project_id: wire.project_id,
            revision: wire.revision,
            meta: wire.meta,
            target: wire.target,
            authoring_locales: wire.authoring_locales,
            entities: wire.entities,
            asset_store: wire.asset_store,
        })
    }
}

impl ProjectRevision2 {
    pub fn from_json(json: &str) -> Result<Self, ProjectJsonError> {
        if json.len() > MAX_PROJECT_JSON_BYTES {
            return Err(ProjectJsonError::InputTooLarge {
                actual: json.len(),
                limit: MAX_PROJECT_JSON_BYTES,
            });
        }
        serde_json::from_str(json).map_err(ProjectJsonError::InvalidJson)
    }

    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

fn deserialize_unique_locales<'de, D>(deserializer: D) -> Result<BTreeSet<LocaleCode>, D::Error>
where
    D: Deserializer<'de>,
{
    let locales = Vec::<LocaleCode>::deserialize(deserializer)?;
    let mut unique = BTreeSet::new();
    for locale in locales {
        if !unique.insert(locale.clone()) {
            return Err(de::Error::custom(format!(
                "duplicate authoring locale {locale}"
            )));
        }
    }
    Ok(unique)
}

fn deserialize_unique_btree_map<'de, D, K, V>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Ord + fmt::Display,
    V: Deserialize<'de>,
{
    struct UniqueMapVisitor<K, V>(PhantomData<(K, V)>);

    impl<'de, K, V> Visitor<'de> for UniqueMapVisitor<K, V>
    where
        K: Deserialize<'de> + Ord + fmt::Display,
        V: Deserialize<'de>,
    {
        type Value = BTreeMap<K, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object with unique keys")
        }

        fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = access.next_entry()? {
                match values.entry(key) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(value);
                    }
                    std::collections::btree_map::Entry::Occupied(entry) => {
                        return Err(de::Error::custom(format!(
                            "duplicate map key {}",
                            entry.key()
                        )));
                    }
                }
            }
            Ok(values)
        }
    }

    deserializer.deserialize_map(UniqueMapVisitor(PhantomData))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_marker_accepts_only_revision_two() {
        assert!(serde_json::from_str::<SchemaRevisionV2>("2").is_ok());
        assert!(serde_json::from_str::<SchemaRevisionV2>("1").is_err());
        assert!(serde_json::from_str::<SchemaRevisionV2>("3").is_err());
    }

    #[test]
    fn sha_digest_remains_a_valid_unique_map_key() {
        let digest = "ab".repeat(32).parse::<crate::Sha256Digest>().unwrap();
        let json = format!("{{\"{digest}\":{{\"byte_len\":1,\"media_type\":\"audio/ogg\"}}}}");
        let parsed = serde_json::from_str::<BTreeMap<crate::Sha256Digest, crate::AssetMeta>>(&json);
        assert!(parsed.is_ok());
    }

    #[test]
    fn collision_set_seed_stops_at_the_remaining_aggregate_budget() {
        let mut deserializer = serde_json::Deserializer::from_str("[\"first\",\"second\"]");
        let error = CollisionSetSeed {
            remaining_count: 1,
            remaining_bytes: 512,
        }
        .deserialize(&mut deserializer)
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("remaining aggregate entry budget 1"));

        let oversized = format!("[\"{}\"]", "x".repeat(513));
        let mut deserializer = serde_json::Deserializer::from_str(&oversized);
        let error = CollisionSetSeed {
            remaining_count: 1,
            remaining_bytes: 1_024,
        }
        .deserialize(&mut deserializer)
        .unwrap_err();
        assert!(error.to_string().contains("maximum is 512"));
    }
}
