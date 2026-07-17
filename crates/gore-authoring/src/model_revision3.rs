//! Closed schema-revision-3 model for content-addressed Quest collision evidence.
//!
//! Revision 3 reuses revision-2 value/entity payloads whose wire shape did not change. Its Quest
//! Draft is separate and retains only a bounded reference to an immutable collision artifact;
//! multi-megabyte collision arrays are not representable in this schema.
//!
//! This foundation is also available through [`crate::ProjectDocument`] for bounded parsing and
//! canonical serialization. Dispatch alone grants no Store, build, deployment, or runtime
//! authority.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Write};
use std::marker::PhantomData;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::strict_json::reject_duplicate_object_keys;
use crate::{
    AssetStoreIndex, ContentSeal, EntityId, FormatV2, GameGenerationAnchor, LocaleCode, ProjectId,
    ProjectMeta, Sha256Digest, MAX_PROJECT_JSON_BYTES,
};

pub use crate::model_revision2::{
    DialogLine, EntityKind, LocalizationEntry, NpcDraftInput, NpcParentClassInput, OggCodec,
    OggMetadata, OriginRef, QuestGiverInput, QuestParentInput, ScriptAuthoringStatus, ScriptModule,
    ScriptModuleStatus, ScriptRuntimeStatus, TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot,
    VoiceTake, VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
};

pub const QUEST_COLLISION_ARTIFACT_FORMAT: &str = "quest_collision_capability";
pub const QUEST_COLLISION_ARTIFACT_SCHEMA_REVISION: u32 = 1;
pub const QUEST_COLLISION_ARTIFACT_MEDIA_TYPE: &str =
    "application/vnd.gore.quest-collision-capability+json;version=1";
pub const QUEST_COLLISION_CATALOG_LAYER: &str = "base-game-plus-exact-project.story-collisions.v1";
/// Media type reserved for collision evidence rebuilt from the exact current revision-3
/// project, including already-authored Quest identities.
///
/// Version 1 constants and wire fields deliberately remain unchanged. The catalog layer is the
/// discriminator carried by [`QuestCollisionArtifactRef`]; closed-model validation requires this
/// media type and [`QUEST_COLLISION_CATALOG_LAYER_V2`] as an exact pair.
pub const QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2: &str =
    "application/vnd.gore.quest-collision-capability+json;version=2";
pub const QUEST_COLLISION_CATALOG_LAYER_V2: &str =
    "base-game-plus-exact-revision3-project.story-collisions.v2";
pub const MAX_QUEST_COLLISION_ARTIFACT_BYTES: u64 = 24 * 1024 * 1024;
pub const MAX_REVISION3_ENTITY_JSON_BYTES: usize = 1024 * 1024;
pub const MAX_REVISION3_ENTITIES: usize = 100_000;
pub const MAX_REVISION3_ASSETS: usize = 100_000;
pub const MAX_REVISION3_REFERENCED_ASSET_BYTES: u64 = 64 * 1024 * 1024 * 1024;
/// Maximum ordered project-local dialog-line bindings retained by one Quest draft.
pub const MAX_REVISION3_QUEST_TRANSCRIPT_BINDINGS_V1: usize = 256;
/// Maximum ordered project-local dialog-line greeting bindings retained by one NPC draft.
pub const MAX_REVISION3_NPC_GREETING_BINDINGS_V1: usize = 256;
/// Maximum history-free Store manifest projection for one revision-3 project.
pub const MAX_REVISION3_BASE_SNAPSHOT_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum final revision-3 Store snapshot, including its bounded retained-history envelope.
pub const MAX_REVISION3_SNAPSHOT_BYTES: u64 = 17 * 1024 * 1024;
pub const REVISION3_QUEST_GENERATOR_ID: &str = "gore-authoring.draft-quest-skeleton";
pub const REVISION3_QUEST_GENERATOR_VERSION: u32 = 2;
pub const REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION: u32 = 3;
pub const REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION: u32 = 4;
pub const MAX_QUEST_TRANSITION_PREDICATE_GROUPS_V1: usize = 8;
pub const MAX_QUEST_TRANSITION_PREDICATE_ATOMS_V1: usize = 8;
pub const MAX_QUEST_TRANSITION_EFFECTS_V1: usize = 8;
const MAX_CATALOG_LAYER_BYTES: usize = 128;

fn is_false(value: &bool) -> bool {
    !*value
}

pub(crate) fn revision3_voice_target_key_v1(target: &VoiceTarget) -> (String, String) {
    (
        target.archive.replace('\\', "/").to_lowercase(),
        target.member.replace('\\', "/").to_lowercase(),
    )
}

pub(crate) fn quest_collision_artifact_media_for_layer(layer: &str) -> Option<&'static str> {
    match layer {
        QUEST_COLLISION_CATALOG_LAYER => Some(QUEST_COLLISION_ARTIFACT_MEDIA_TYPE),
        QUEST_COLLISION_CATALOG_LAYER_V2 => Some(QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2),
        _ => None,
    }
}

pub(crate) fn is_quest_collision_artifact_media_type(media_type: &str) -> bool {
    media_type == QUEST_COLLISION_ARTIFACT_MEDIA_TYPE
        || media_type == QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SchemaRevisionV3;

impl Serialize for SchemaRevisionV3 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(3)
    }
}

impl<'de> Deserialize<'de> for SchemaRevisionV3 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let revision = u32::deserialize(deserializer)?;
        if revision == 3 {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported authoring schema revision {revision}; expected 3"
            )))
        }
    }
}

/// Exact immutable-object reference retained by one revision-3 Quest Draft.
///
/// `artifact` is the ordinary raw SHA-256 content address. `source_seal` is the independent
/// domain-separated semantic seal. `basis_snapshot` identifies the exact immutable project
/// snapshot from which the project collision layer was derived. This object contains no
/// collision entries and makes no runtime, build, deployment, or publication claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuestCollisionArtifactRef {
    pub generation: GameGenerationAnchor,
    pub catalog_layer: String,
    pub artifact: ContentSeal,
    pub source_seal: ContentSeal,
    pub basis_snapshot: ContentSeal,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestCollisionArtifactRefWire {
    generation: GameGenerationAnchor,
    catalog_layer: BoundedString<MAX_CATALOG_LAYER_BYTES>,
    artifact: ContentSeal,
    source_seal: ContentSeal,
    basis_snapshot: ContentSeal,
}

impl<'de> Deserialize<'de> for QuestCollisionArtifactRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = QuestCollisionArtifactRefWire::deserialize(deserializer)?;
        Ok(Self {
            generation: wire.generation,
            catalog_layer: wire.catalog_layer.0,
            artifact: wire.artifact,
            source_seal: wire.source_seal,
            basis_snapshot: wire.basis_snapshot,
        })
    }
}

/// Stable identity of one node in a semantic Quest transition plan.
///
/// Objective slot ordinals are technical identities. They do not change when the author changes
/// presentation order and are never reused after deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum QuestTransitionNodeV1 {
    Root,
    Objective { slot: u16 },
}

/// One of the four fixed lifecycle edges supported by the bounded semantic Quest model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestTransitionEdgeV1 {
    Availability,
    Start,
    Success,
    Failure,
}

/// Typed state observation used by a transition predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestTransitionStateTestV1 {
    Available,
    Running,
    Started,
    Succeeded,
    Failed,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestTransitionConditionAtomV1 {
    pub node: QuestTransitionNodeV1,
    pub test: QuestTransitionStateTestV1,
    pub negated: bool,
}

/// One conjunction in the bounded disjunctive-normal-form predicate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestTransitionConditionGroupV1 {
    pub all_of: Vec<QuestTransitionConditionAtomV1>,
}

/// Bounded DNF predicate: at least one `any_of` group must have every `all_of` atom true.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestTransitionPredicateV1 {
    pub any_of: Vec<QuestTransitionConditionGroupV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestTransitionEffectKindV1 {
    Start,
    Succeed,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestTransitionEffectV1 {
    pub target: QuestTransitionNodeV1,
    pub effect: QuestTransitionEffectKindV1,
}

/// Driver and side effects for one fixed lifecycle edge of one plan node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestTransitionV1 {
    pub node: QuestTransitionNodeV1,
    pub edge: QuestTransitionEdgeV1,
    pub external_allowed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicate: Option<QuestTransitionPredicateV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<QuestTransitionEffectV1>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub succeeds_parent: bool,
}

/// Closed, bounded semantic transition plan carried only by generator version 4.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestTransitionPlanV1 {
    /// Active stable objective identities, in ascending ordinal order.
    pub objective_slots: Vec<u16>,
    /// Full permutation of `objective_slots` defining presentation order.
    pub objective_order: Vec<u16>,
    /// First never-used objective ordinal; strictly greater than every active slot.
    pub next_slot_ordinal: u16,
    pub transitions: Vec<QuestTransitionV1>,
}

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
    /// Ordered objectives after the frozen first objective. Omitted when empty so all existing
    /// generator-v2 project bytes remain canonical and byte-identical.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_objective_titles: Vec<String>,
    /// Optional only for exact generator-v2/v3 compatibility. Generator v4 requires a plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_plan: Option<Box<QuestTransitionPlanV1>>,
    pub collision_catalog: QuestCollisionArtifactRef,
}

/// One ordered, authoring-only dialog-line placement in a Quest transcript.
///
/// `objective_slot` is a stable semantic-objective ordinal for generator-v4 Quests. It is absent
/// for the Quest root/unassigned transcript and for every legacy generator-v2/v3 Quest. This
/// relationship is project metadata only: it grants no topic, selection-effect, build, or runtime
/// authority and deliberately remains outside [`QuestDraftInput`] so Quest source and its input
/// fingerprint are unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestTranscriptBindingV1 {
    pub line: TypedRef,
    pub objective_slot: Option<u16>,
}

/// One ordered, authoring-only dialog-line greeting attached to an NPC draft.
///
/// The relationship is project metadata only. It grants no dialog topic, speaker, selection,
/// build, publication, or runtime authority and deliberately remains outside [`NpcDraftInput`]
/// so deterministic NPC source and its input fingerprint stay byte-identical.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcGreetingBindingV1 {
    pub line: TypedRef,
}

/// Revision-3 NPC authoring intent plus ordered authoring-only greeting metadata.
///
/// The first four fields retain the exact revision-2 wire shape. Empty `greetings` are omitted so
/// every pre-greeting canonical revision-3 project remains byte-identical.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcDraft {
    pub generator_id: String,
    pub generator_version: u32,
    pub input: NpcDraftInput,
    pub script_module: TypedRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub greetings: Vec<NpcGreetingBindingV1>,
}

impl NpcDraft {
    fn revision2_generation_core(&self) -> crate::model_revision2::NpcDraft {
        crate::model_revision2::NpcDraft {
            generator_id: self.generator_id.clone(),
            generator_version: self.generator_version,
            input: self.input.clone(),
            script_module: self.script_module.clone(),
        }
    }

    /// Rebuild the exact owned module from durable generation intent only.
    ///
    /// Greeting metadata is intentionally excluded from generation.
    pub fn regenerate_script_module(
        &self,
        owner: TypedRef,
    ) -> Result<ScriptModule, crate::model_revision2::StoryRegenerationError> {
        self.revision2_generation_core()
            .regenerate_script_module(owner)
    }

    pub(crate) fn regenerate_script_module_with_identity(
        &self,
        owner: TypedRef,
    ) -> Result<
        (ScriptModule, crate::model_revision2::GeneratedStoryIdentity),
        crate::model_revision2::StoryRegenerationError,
    > {
        self.revision2_generation_core()
            .regenerate_script_module_with_identity(owner)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestDraft {
    pub generator_id: String,
    pub generator_version: u32,
    pub input: QuestDraftInput,
    pub script_module: TypedRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transcript: Vec<QuestTranscriptBindingV1>,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectRevision3 {
    pub format: FormatV2,
    pub schema_revision: SchemaRevisionV3,
    pub project_id: ProjectId,
    #[serde(default)]
    pub revision: u64,
    pub meta: ProjectMeta,
    pub target: GameGenerationAnchor,
    #[serde(default)]
    pub authoring_locales: BTreeSet<LocaleCode>,
    #[serde(default)]
    pub entities: BTreeMap<EntityId, Entity>,
    pub asset_store: AssetStoreIndex,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectRevision3Wire {
    format: FormatV2,
    schema_revision: SchemaRevisionV3,
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

impl<'de> Deserialize<'de> for ProjectRevision3 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProjectRevision3Wire::deserialize(deserializer)?;
        let project = Self {
            format: wire.format,
            schema_revision: wire.schema_revision,
            project_id: wire.project_id,
            revision: wire.revision,
            meta: wire.meta,
            target: wire.target,
            authoring_locales: wire.authoring_locales,
            entities: wire.entities,
            asset_store: wire.asset_store,
        };
        project.validate_closed_model().map_err(de::Error::custom)?;
        Ok(project)
    }
}

impl ProjectRevision3 {
    /// Parse only exact canonical, duplicate-free revision-3 JSON under the unchanged project cap.
    pub fn from_json(json: &str) -> Result<Self, ProjectRevision3JsonError> {
        if json.len() > MAX_PROJECT_JSON_BYTES {
            return Err(ProjectRevision3JsonError::InputTooLarge {
                actual: json.len(),
                limit: MAX_PROJECT_JSON_BYTES,
            });
        }
        reject_duplicate_object_keys(json).map_err(ProjectRevision3JsonError::InvalidJson)?;
        let project: Self =
            serde_json::from_str(json).map_err(ProjectRevision3JsonError::InvalidJson)?;
        let canonical = project.to_canonical_json()?;
        if canonical.as_bytes() != json.as_bytes() {
            return Err(ProjectRevision3JsonError::NonCanonicalJson);
        }
        Ok(project)
    }

    pub fn to_canonical_json(&self) -> Result<String, ProjectRevision3JsonError> {
        self.validate_closed_model()
            .map_err(ProjectRevision3JsonError::InvalidModel)?;
        let mut writer = BoundedProjectWriter::new(MAX_PROJECT_JSON_BYTES);
        let result = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(ProjectRevision3JsonError::InputTooLarge {
                actual,
                limit: MAX_PROJECT_JSON_BYTES,
            });
        }
        result.map_err(ProjectRevision3JsonError::Serialize)?;
        String::from_utf8(writer.bytes).map_err(|_| {
            ProjectRevision3JsonError::InvalidModel(
                ProjectRevision3ValidationError::NonUtf8Serialization,
            )
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectRevision3JsonError {
    #[error("authoring revision-3 project JSON exceeds the {limit}-byte limit: {actual} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("invalid authoring revision-3 project JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("could not serialize canonical authoring revision-3 project JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("authoring revision-3 project JSON is not in its exact canonical spelling")]
    NonCanonicalJson,
    #[error("invalid authoring revision-3 project model: {0}")]
    InvalidModel(#[source] ProjectRevision3ValidationError),
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectRevision3ValidationError {
    #[error("revision-3 project id must not be all zeroes")]
    ZeroProjectId,
    #[error("revision-3 target executable seal must have a non-zero byte length")]
    InvalidTarget,
    #[error("revision-3 project has {actual} entities; maximum is {max}")]
    TooManyEntities { actual: usize, max: usize },
    #[error("revision-3 project has {actual} assets; maximum is {max}")]
    TooManyAssets { actual: usize, max: usize },
    #[error("revision-3 asset byte total exceeds {max}: {actual}")]
    AssetBytesTooLarge { actual: u64, max: u64 },
    #[error("revision-3 asset {asset} has invalid metadata")]
    InvalidAssetMetadata { asset: Sha256Digest },
    #[error("revision-3 entity map key {key} does not match embedded id {embedded}")]
    EntityKeyMismatch { key: EntityId, embedded: EntityId },
    #[error("could not serialize revision-3 entity {entity}: {source}")]
    SerializeEntity {
        entity: EntityId,
        #[source]
        source: serde_json::Error,
    },
    #[error("revision-3 entity {entity} is {actual} bytes; maximum is {max}")]
    EntityTooLarge {
        entity: EntityId,
        actual: usize,
        max: usize,
    },
    #[error("revision-3 Quest {quest} has invalid artifact reference: {reason}")]
    InvalidQuestArtifactRef { quest: EntityId, reason: String },
    #[error("revision-3 Quest {quest} has an invalid authoring transcript: {reason}")]
    InvalidQuestTranscript { quest: EntityId, reason: String },
    #[error("revision-3 Quest {quest} collision artifact {artifact} is absent from asset_store")]
    MissingQuestArtifact {
        quest: EntityId,
        artifact: Sha256Digest,
    },
    #[error(
        "revision-3 Quest {quest} collision artifact metadata mismatch for {artifact}: {reason}"
    )]
    QuestArtifactMetadataMismatch {
        quest: EntityId,
        artifact: Sha256Digest,
        reason: String,
    },
    #[error("revision-3 Quest {quest} has an invalid local ScriptModule reference")]
    InvalidQuestScriptReference { quest: EntityId },
    #[error("revision-3 Quest {quest} ScriptModule target is missing or has the wrong kind")]
    MissingQuestScriptModule { quest: EntityId },
    #[error("revision-3 NPC {npc} has invalid closed generator state: {reason}")]
    InvalidNpcDraft { npc: EntityId, reason: String },
    #[error("revision-3 NPC {npc} has invalid authoring greetings: {reason}")]
    InvalidNpcGreetings { npc: EntityId, reason: String },
    #[error("revision-3 NPC {npc} has an invalid local ScriptModule reference")]
    InvalidNpcScriptReference { npc: EntityId },
    #[error("revision-3 NPC {npc} ScriptModule target is missing or has the wrong kind")]
    MissingNpcScriptModule { npc: EntityId },
    #[error("revision-3 NPC-generated ScriptModule {module} has no exact owning NPC closure")]
    OrphanNpcScriptModule { module: EntityId },
    #[error("revision-3 Voice graph at entity {entity} is invalid: {reason}")]
    InvalidVoiceGraph { entity: EntityId, reason: String },
    #[error("revision-3 VoiceTake {take} is invalid: {reason}")]
    InvalidVoiceTake { take: EntityId, reason: String },
    #[error("revision-3 VoiceSlot {slot} target evidence is invalid: {reason}")]
    InvalidVoiceTarget { slot: EntityId, reason: String },
    #[error(
        "revision-3 VoiceSlot {slot} duplicates the resolved archive/member target of slot {existing_slot}"
    )]
    DuplicateVoiceTarget {
        slot: EntityId,
        existing_slot: EntityId,
    },
    #[error("revision-3 canonical serializer emitted non-UTF-8 bytes")]
    NonUtf8Serialization,
}

struct BoundedProjectWriter {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedProjectWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(64 * 1024)),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedProjectWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let actual = self.bytes.len().saturating_add(bytes.len());
        if actual > self.limit {
            self.first_exceeded_size.get_or_insert(actual);
            return Err(io::Error::other("revision-3 project JSON limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct BoundedString<const MAX: usize>(String);

impl<'de, const MAX: usize> Deserialize<'de> for BoundedString<MAX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringVisitor<const MAX: usize>;

        impl<const MAX: usize> Visitor<'_> for StringVisitor<MAX> {
            type Value = BoundedString<MAX>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a UTF-8 string of at most {MAX} bytes")
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
                if value.len() > MAX {
                    return Err(E::invalid_length(value.len(), &self));
                }
                Ok(BoundedString(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.len() > MAX {
                    return Err(E::invalid_length(value.len(), &self));
                }
                Ok(BoundedString(value))
            }
        }

        deserializer.deserialize_string(StringVisitor::<MAX>)
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
                if values.insert(key, value).is_some() {
                    return Err(de::Error::custom("duplicate map key"));
                }
            }
            Ok(values)
        }
    }

    deserializer.deserialize_map(UniqueMapVisitor(PhantomData))
}
