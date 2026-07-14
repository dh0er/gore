//! Exact-project source and readiness inspection for one schema-revision-3 NPC Draft.
//!
//! This module is intentionally pure and read-only. It accepts exact canonical project JSON,
//! verifies the selected NPC/module closure, regenerates the module from the persisted parent
//! triple, and returns a bounded canonical plan. It has no Store, filesystem, compiler, game,
//! spawn, deployment, publication, or runtime entry point. Persisted parent provenance is not
//! widened into a fresh game-catalog claim.

use std::fmt;
use std::io::{self, Write};

use gore_authoring::{
    ContentSeal, EntityId, GameGenerationAnchor, ProjectDocument, ProjectDocumentError, ProjectId,
    Revision2NpcParentClassInput, Revision3Entity, Revision3EntityKind, Revision3EntityPayload,
    Revision3NpcDraft, Revision3NpcDraftInput, Revision3OriginRef, Revision3ScriptModule,
    Revision3TypedRef, ScriptModuleStatus, Sha256Digest, StoryRegenerationError,
    LOGICAL_NPC_CLONE_GENERATOR_ID, LOGICAL_NPC_CLONE_GENERATOR_VERSION,
    MAX_ANGELSCRIPT_IDENTIFIER_BYTES, MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES,
    MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES, MAX_PROJECT_JSON_BYTES, MAX_REVISION3_ENTITY_JSON_BYTES,
    MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1,
};
use serde::de::{self, IgnoredAny, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

use crate::{
    BoundedString, MAX_STORY_BUILD_DIAGNOSTIC_MESSAGE_BYTES, MAX_STORY_BUILD_GENERATOR_ID_BYTES,
    MAX_STORY_BUILD_PROPERTY_PATH_BYTES,
};

const PLAN_FORMAT: &str = "revision3_npc_source_inspection_plan";
const PLAN_SCHEMA_REVISION: u32 = 1;
// These are the closed bounds enforced by revision-2/3 NPC provenance validation. They are not
// exported by gore-authoring, so this inspection mirrors them rather than widening the envelope.
const MAX_NPC_PARENT_CATALOG_LAYER_BYTES: usize = 128;
const MAX_NPC_PARENT_SELECTOR_BYTES: usize = MAX_ANGELSCRIPT_IDENTIFIER_BYTES;
const EXPECTED_DIAGNOSTIC_COUNT: usize = 4;
const MAX_PLAN_FORMAT_BYTES: usize = 64;
const PROJECT_ID_HEX_BYTES: usize = 32;
const ENTITY_ID_HEX_BYTES: usize = 32;
const SHA256_HEX_BYTES: usize = 64;
const MAX_CLOSED_TOKEN_BYTES: usize = 64;

/// The input uses the same bounded envelope as every authoring project document.
pub const MAX_REVISION3_NPC_INSPECTION_PROJECT_JSON_BYTES: usize = MAX_PROJECT_JSON_BYTES;
/// A one-NPC plan repeats one bounded input and one bounded generated source.
pub const MAX_REVISION3_NPC_INSPECTION_PLAN_JSON_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PlanFormat;

impl Serialize for PlanFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(PLAN_FORMAT)
    }
}

impl<'de> Deserialize<'de> for PlanFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = BoundedString::<MAX_PLAN_FORMAT_BYTES>::deserialize(deserializer)?.into_inner();
        if value == PLAN_FORMAT {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported revision-3 NPC inspection format {value:?}"
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PlanSchemaRevision;

impl Serialize for PlanSchemaRevision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(PLAN_SCHEMA_REVISION)
    }
}

impl<'de> Deserialize<'de> for PlanSchemaRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == PLAN_SCHEMA_REVISION {
            Ok(Self)
        } else {
            Err(de::Error::custom(format!(
                "unsupported revision-3 NPC inspection schema revision {value}"
            )))
        }
    }
}

/// Closed scope marker. No compile, build, spawn, deploy, or publication operation exists here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcInspectionScopeV1 {
    SourceReadinessInspectionOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcInspectionSourceStatusV1 {
    PersistedAndRegeneratedExact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcInspectionCompilerStatusV1 {
    NotRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcInspectionBuildStatusV1 {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcInspectionRuntimeQualificationV1 {
    RuntimeUnqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcInspectionSpawnStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcInspectionPublicationStatusV1 {
    NotSupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NpcInspectionDiagnosticCodeV1 {
    NpcCompilerNotRun,
    NpcProductionLoweringUnavailable,
    NpcRuntimeResidenceUnqualified,
    NpcSpawnUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcInspectionDiagnosticSeverityV1 {
    Error,
    Warning,
}

/// Stable, entity-addressable readiness evidence for a normal UI Problems surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3NpcInspectionDiagnosticV1 {
    pub(crate) code: NpcInspectionDiagnosticCodeV1,
    pub(crate) severity: NpcInspectionDiagnosticSeverityV1,
    pub(crate) entity: Revision3TypedRef,
    pub(crate) property_path: String,
    pub(crate) message: String,
    pub(crate) blocks_build: bool,
}

impl Revision3NpcInspectionDiagnosticV1 {
    pub const fn code(&self) -> NpcInspectionDiagnosticCodeV1 {
        self.code
    }

    pub const fn severity(&self) -> NpcInspectionDiagnosticSeverityV1 {
        self.severity
    }

    pub const fn entity(&self) -> &Revision3TypedRef {
        &self.entity
    }

    pub fn property_path(&self) -> &str {
        &self.property_path
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub const fn blocks_build(&self) -> bool {
        self.blocks_build
    }
}

/// Exact project bytes and generation against which the selected closure was inspected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3NpcInspectionProvenanceV1 {
    pub(crate) project_id: ProjectId,
    pub(crate) project_revision: u64,
    pub(crate) target: GameGenerationAnchor,
    pub(crate) canonical_project: ContentSeal,
}

impl Revision3NpcInspectionProvenanceV1 {
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub const fn project_revision(&self) -> u64 {
        self.project_revision
    }

    pub const fn target(&self) -> &GameGenerationAnchor {
        &self.target
    }

    pub const fn canonical_project(&self) -> &ContentSeal {
        &self.canonical_project
    }
}

/// Persisted NPC entity facts, including the complete sealed parent-class provenance triple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3NpcInspectionEntityV1 {
    pub(crate) reference: Revision3TypedRef,
    pub(crate) entity_revision: u64,
    pub(crate) display_name: String,
    pub(crate) origin: Revision3OriginRef,
    pub(crate) generator_id: String,
    pub(crate) generator_version: u32,
    pub(crate) input: Revision3NpcDraftInput,
    pub(crate) input_seal: ContentSeal,
    pub(crate) script_module: Revision3TypedRef,
}

impl Revision3NpcInspectionEntityV1 {
    pub const fn reference(&self) -> &Revision3TypedRef {
        &self.reference
    }

    pub const fn entity_revision(&self) -> u64 {
        self.entity_revision
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub const fn origin(&self) -> &Revision3OriginRef {
        &self.origin
    }

    pub fn generator_id(&self) -> &str {
        &self.generator_id
    }

    pub const fn generator_version(&self) -> u32 {
        self.generator_version
    }

    pub const fn input(&self) -> &Revision3NpcDraftInput {
        &self.input
    }

    pub const fn input_seal(&self) -> &ContentSeal {
        &self.input_seal
    }

    pub const fn script_module(&self) -> &Revision3TypedRef {
        &self.script_module
    }
}

/// One persisted ScriptModule that was independently regenerated from [`Revision3NpcInspectionEntityV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3NpcInspectionModuleV1 {
    pub(crate) reference: Revision3TypedRef,
    pub(crate) entity_revision: u64,
    pub(crate) display_name: String,
    pub(crate) origin: Revision3OriginRef,
    pub(crate) persisted_source: ContentSeal,
    pub(crate) generated: Revision3ScriptModule,
}

impl Revision3NpcInspectionModuleV1 {
    pub const fn reference(&self) -> &Revision3TypedRef {
        &self.reference
    }

    pub const fn entity_revision(&self) -> u64 {
        self.entity_revision
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub const fn origin(&self) -> &Revision3OriginRef {
        &self.origin
    }

    pub const fn persisted_source(&self) -> &ContentSeal {
        &self.persisted_source
    }

    pub const fn generated(&self) -> &Revision3ScriptModule {
        &self.generated
    }
}

/// Deterministic, canonical source/readiness inspection for exactly one revision-3 NPC Draft.
///
/// The source status proves equality only against the exact persisted parent triple. The closed
/// status enums make it impossible for this plan to claim compilation, production buildability,
/// runtime residence, spawning, or publication support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Revision3NpcSourceInspectionPlanV1 {
    #[serde(rename = "format")]
    pub(crate) format_marker: PlanFormat,
    pub(crate) schema_revision: PlanSchemaRevision,
    pub(crate) scope: NpcInspectionScopeV1,
    pub(crate) source_status: NpcInspectionSourceStatusV1,
    pub(crate) compiler_status: NpcInspectionCompilerStatusV1,
    pub(crate) build_status: NpcInspectionBuildStatusV1,
    pub(crate) runtime_qualification: NpcInspectionRuntimeQualificationV1,
    pub(crate) spawn_status: NpcInspectionSpawnStatusV1,
    pub(crate) publication_status: NpcInspectionPublicationStatusV1,
    pub(crate) provenance: Revision3NpcInspectionProvenanceV1,
    pub(crate) npc: Revision3NpcInspectionEntityV1,
    pub(crate) module: Revision3NpcInspectionModuleV1,
    pub(crate) diagnostics: Vec<Revision3NpcInspectionDiagnosticV1>,
}

macro_rules! bounded_hex_wire {
    ($wire:ident, $value:ty, $limit:expr) => {
        #[derive(Debug)]
        struct $wire($value);

        impl<'de> Deserialize<'de> for $wire {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                BoundedString::<$limit>::deserialize(deserializer)?
                    .into_inner()
                    .parse::<$value>()
                    .map(Self)
                    .map_err(de::Error::custom)
            }
        }
    };
}

bounded_hex_wire!(ProjectIdWire, ProjectId, PROJECT_ID_HEX_BYTES);
bounded_hex_wire!(EntityIdWire, EntityId, ENTITY_ID_HEX_BYTES);
bounded_hex_wire!(Sha256DigestWire, Sha256Digest, SHA256_HEX_BYTES);

macro_rules! bounded_literal_wire {
    ($wire:ident, $literal:literal) => {
        struct $wire;

        impl<'de> Deserialize<'de> for $wire {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = BoundedString::<MAX_CLOSED_TOKEN_BYTES>::deserialize(deserializer)?
                    .into_inner();
                if value == $literal {
                    Ok(Self)
                } else {
                    Err(de::Error::custom(format!(
                        "unsupported closed token {value:?}; expected {:?}",
                        $literal
                    )))
                }
            }
        }
    };
}

bounded_literal_wire!(
    SourceReadinessInspectionOnlyWire,
    "source_readiness_inspection_only"
);
bounded_literal_wire!(
    PersistedAndRegeneratedExactWire,
    "persisted_and_regenerated_exact"
);
bounded_literal_wire!(NotRunWire, "not_run");
bounded_literal_wire!(BlockedWire, "blocked");
bounded_literal_wire!(RuntimeUnqualifiedWire, "runtime_unqualified");
bounded_literal_wire!(NotSupportedWire, "not_supported");
bounded_literal_wire!(NewOriginTypeWire, "new");
bounded_literal_wire!(GeneratedOriginTypeWire, "generated");
bounded_literal_wire!(OfflineDraftWire, "offline_draft");

struct EntityKindWire(Revision3EntityKind);

impl<'de> Deserialize<'de> for EntityKindWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value =
            BoundedString::<MAX_CLOSED_TOKEN_BYTES>::deserialize(deserializer)?.into_inner();
        let kind = match value.as_str() {
            "localization_entry" => Revision3EntityKind::LocalizationEntry,
            "dialog_line" => Revision3EntityKind::DialogLine,
            "voice_slot" => Revision3EntityKind::VoiceSlot,
            "voice_take" => Revision3EntityKind::VoiceTake,
            "npc_draft" => Revision3EntityKind::NpcDraft,
            "quest_draft" => Revision3EntityKind::QuestDraft,
            "script_module" => Revision3EntityKind::ScriptModule,
            _ => {
                return Err(de::Error::custom(format!(
                    "unsupported revision-3 entity kind {value:?}"
                )))
            }
        };
        Ok(Self(kind))
    }
}

struct DiagnosticCodeWire(NpcInspectionDiagnosticCodeV1);

impl<'de> Deserialize<'de> for DiagnosticCodeWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value =
            BoundedString::<MAX_CLOSED_TOKEN_BYTES>::deserialize(deserializer)?.into_inner();
        let code = match value.as_str() {
            "NPC_COMPILER_NOT_RUN" => NpcInspectionDiagnosticCodeV1::NpcCompilerNotRun,
            "NPC_PRODUCTION_LOWERING_UNAVAILABLE" => {
                NpcInspectionDiagnosticCodeV1::NpcProductionLoweringUnavailable
            }
            "NPC_RUNTIME_RESIDENCE_UNQUALIFIED" => {
                NpcInspectionDiagnosticCodeV1::NpcRuntimeResidenceUnqualified
            }
            "NPC_SPAWN_UNAVAILABLE" => NpcInspectionDiagnosticCodeV1::NpcSpawnUnavailable,
            _ => {
                return Err(de::Error::custom(format!(
                    "unsupported revision-3 NPC diagnostic code {value:?}"
                )))
            }
        };
        Ok(Self(code))
    }
}

struct DiagnosticSeverityWire(NpcInspectionDiagnosticSeverityV1);

impl<'de> Deserialize<'de> for DiagnosticSeverityWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value =
            BoundedString::<MAX_CLOSED_TOKEN_BYTES>::deserialize(deserializer)?.into_inner();
        let severity = match value.as_str() {
            "error" => NpcInspectionDiagnosticSeverityV1::Error,
            "warning" => NpcInspectionDiagnosticSeverityV1::Warning,
            _ => {
                return Err(de::Error::custom(format!(
                    "unsupported revision-3 NPC diagnostic severity {value:?}"
                )))
            }
        };
        Ok(Self(severity))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentSealWire {
    byte_len: u64,
    sha256: Sha256DigestWire,
}

impl From<ContentSealWire> for ContentSeal {
    fn from(wire: ContentSealWire) -> Self {
        Self {
            byte_len: wire.byte_len,
            sha256: wire.sha256.0,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GameGenerationAnchorWire {
    executable: ContentSealWire,
}

impl From<GameGenerationAnchorWire> for GameGenerationAnchor {
    fn from(wire: GameGenerationAnchorWire) -> Self {
        Self {
            executable: wire.executable.into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TypedRefWire {
    project_id: ProjectIdWire,
    id: EntityIdWire,
    expected_kind: EntityKindWire,
}

impl From<TypedRefWire> for Revision3TypedRef {
    fn from(wire: TypedRefWire) -> Self {
        Self::new(wire.project_id.0, wire.id.0, wire.expected_kind.0)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcOriginWire {
    #[serde(rename = "type")]
    type_marker: NewOriginTypeWire,
    authored_runtime_id: BoundedString<MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES>,
}

impl From<NpcOriginWire> for Revision3OriginRef {
    fn from(wire: NpcOriginWire) -> Self {
        let NpcOriginWire {
            type_marker,
            authored_runtime_id,
        } = wire;
        let NewOriginTypeWire = type_marker;
        Self::New {
            authored_runtime_id: authored_runtime_id.into_inner(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleOriginWire {
    #[serde(rename = "type")]
    type_marker: GeneratedOriginTypeWire,
    generator_id: BoundedString<MAX_STORY_BUILD_GENERATOR_ID_BYTES>,
    generator_version: u32,
    owner: TypedRefWire,
}

impl From<ModuleOriginWire> for Revision3OriginRef {
    fn from(wire: ModuleOriginWire) -> Self {
        let ModuleOriginWire {
            type_marker,
            generator_id,
            generator_version,
            owner,
        } = wire;
        let GeneratedOriginTypeWire = type_marker;
        Self::Generated {
            generator_id: generator_id.into_inner(),
            generator_version,
            owner: owner.into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcParentClassInputWire {
    generation: GameGenerationAnchorWire,
    source_seal: ContentSealWire,
    catalog_layer: BoundedString<MAX_NPC_PARENT_CATALOG_LAYER_BYTES>,
    canonical_selector: BoundedString<MAX_NPC_PARENT_SELECTOR_BYTES>,
    runtime_class: BoundedString<MAX_ANGELSCRIPT_IDENTIFIER_BYTES>,
}

impl From<NpcParentClassInputWire> for Revision2NpcParentClassInput {
    fn from(wire: NpcParentClassInputWire) -> Self {
        Self {
            generation: wire.generation.into(),
            source_seal: wire.source_seal.into(),
            catalog_layer: wire.catalog_layer.into_inner(),
            canonical_selector: wire.canonical_selector.into_inner(),
            runtime_class: wire.runtime_class.into_inner(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NpcDraftInputWire {
    target: GameGenerationAnchorWire,
    module_namespace: BoundedString<MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES>,
    unique_name: BoundedString<MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES>,
    parent_character_definition: NpcParentClassInputWire,
    parent_ai_agent_config: NpcParentClassInputWire,
    parent_spawn_definition: NpcParentClassInputWire,
}

impl From<NpcDraftInputWire> for Revision3NpcDraftInput {
    fn from(wire: NpcDraftInputWire) -> Self {
        Self {
            target: wire.target.into(),
            module_namespace: wire.module_namespace.into_inner(),
            unique_name: wire.unique_name.into_inner(),
            parent_character_definition: wire.parent_character_definition.into(),
            parent_ai_agent_config: wire.parent_ai_agent_config.into(),
            parent_spawn_definition: wire.parent_spawn_definition.into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionProvenanceWire {
    project_id: ProjectIdWire,
    project_revision: u64,
    target: GameGenerationAnchorWire,
    canonical_project: ContentSealWire,
}

impl From<InspectionProvenanceWire> for Revision3NpcInspectionProvenanceV1 {
    fn from(wire: InspectionProvenanceWire) -> Self {
        Self {
            project_id: wire.project_id.0,
            project_revision: wire.project_revision,
            target: wire.target.into(),
            canonical_project: wire.canonical_project.into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionNpcWire {
    reference: TypedRefWire,
    entity_revision: u64,
    display_name: BoundedString<MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1>,
    origin: NpcOriginWire,
    generator_id: BoundedString<MAX_STORY_BUILD_GENERATOR_ID_BYTES>,
    generator_version: u32,
    input: NpcDraftInputWire,
    input_seal: ContentSealWire,
    script_module: TypedRefWire,
}

impl From<InspectionNpcWire> for Revision3NpcInspectionEntityV1 {
    fn from(wire: InspectionNpcWire) -> Self {
        Self {
            reference: wire.reference.into(),
            entity_revision: wire.entity_revision,
            display_name: wire.display_name.into_inner(),
            origin: wire.origin.into(),
            generator_id: wire.generator_id.into_inner(),
            generator_version: wire.generator_version,
            input: wire.input.into(),
            input_seal: wire.input_seal.into(),
            script_module: wire.script_module.into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptModuleStatusWire {
    authoring: OfflineDraftWire,
    runtime: RuntimeUnqualifiedWire,
}

impl From<ScriptModuleStatusWire> for ScriptModuleStatus {
    fn from(wire: ScriptModuleStatusWire) -> Self {
        let ScriptModuleStatusWire { authoring, runtime } = wire;
        let OfflineDraftWire = authoring;
        let RuntimeUnqualifiedWire = runtime;
        ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptModuleWire {
    generator_id: BoundedString<MAX_STORY_BUILD_GENERATOR_ID_BYTES>,
    generator_version: u32,
    owner: TypedRefWire,
    module_namespace: BoundedString<MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES>,
    module_relative_path: BoundedString<{ MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES + 3 }>,
    source: BoundedString<MAX_REVISION3_ENTITY_JSON_BYTES>,
    source_sha256: Sha256DigestWire,
    input_fingerprint: Sha256DigestWire,
    status: ScriptModuleStatusWire,
}

impl From<ScriptModuleWire> for Revision3ScriptModule {
    fn from(wire: ScriptModuleWire) -> Self {
        Self {
            generator_id: wire.generator_id.into_inner(),
            generator_version: wire.generator_version,
            owner: wire.owner.into(),
            module_namespace: wire.module_namespace.into_inner(),
            module_relative_path: wire.module_relative_path.into_inner(),
            source: wire.source.into_inner(),
            source_sha256: wire.source_sha256.0,
            input_fingerprint: wire.input_fingerprint.0,
            status: wire.status.into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionModuleWire {
    reference: TypedRefWire,
    entity_revision: u64,
    display_name: BoundedString<MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1>,
    origin: ModuleOriginWire,
    persisted_source: ContentSealWire,
    generated: ScriptModuleWire,
}

impl From<InspectionModuleWire> for Revision3NpcInspectionModuleV1 {
    fn from(wire: InspectionModuleWire) -> Self {
        Self {
            reference: wire.reference.into(),
            entity_revision: wire.entity_revision,
            display_name: wire.display_name.into_inner(),
            origin: wire.origin.into(),
            persisted_source: wire.persisted_source.into(),
            generated: wire.generated.into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticWire {
    code: DiagnosticCodeWire,
    severity: DiagnosticSeverityWire,
    entity: TypedRefWire,
    property_path: BoundedString<MAX_STORY_BUILD_PROPERTY_PATH_BYTES>,
    message: BoundedString<MAX_STORY_BUILD_DIAGNOSTIC_MESSAGE_BYTES>,
    blocks_build: bool,
}

impl From<DiagnosticWire> for Revision3NpcInspectionDiagnosticV1 {
    fn from(wire: DiagnosticWire) -> Self {
        Self {
            code: wire.code.0,
            severity: wire.severity.0,
            entity: wire.entity.into(),
            property_path: wire.property_path.into_inner(),
            message: wire.message.into_inner(),
            blocks_build: wire.blocks_build,
        }
    }
}

struct ExactDiagnosticsWire([DiagnosticWire; EXPECTED_DIAGNOSTIC_COUNT]);

impl ExactDiagnosticsWire {
    fn into_output(self) -> Vec<Revision3NpcInspectionDiagnosticV1> {
        Vec::from(self.0.map(Revision3NpcInspectionDiagnosticV1::from))
    }
}

const EXPECTED_DIAGNOSTIC_CODES: [NpcInspectionDiagnosticCodeV1; EXPECTED_DIAGNOSTIC_COUNT] = [
    NpcInspectionDiagnosticCodeV1::NpcCompilerNotRun,
    NpcInspectionDiagnosticCodeV1::NpcProductionLoweringUnavailable,
    NpcInspectionDiagnosticCodeV1::NpcRuntimeResidenceUnqualified,
    NpcInspectionDiagnosticCodeV1::NpcSpawnUnavailable,
];

const EXPECTED_DIAGNOSTIC_SEVERITIES: [NpcInspectionDiagnosticSeverityV1;
    EXPECTED_DIAGNOSTIC_COUNT] = [
    NpcInspectionDiagnosticSeverityV1::Warning,
    NpcInspectionDiagnosticSeverityV1::Error,
    NpcInspectionDiagnosticSeverityV1::Error,
    NpcInspectionDiagnosticSeverityV1::Error,
];

impl<'de> Deserialize<'de> for ExactDiagnosticsWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ExactDiagnosticsVisitor;

        impl<'de> Visitor<'de> for ExactDiagnosticsVisitor {
            type Value = ExactDiagnosticsWire;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("the exact four ordered revision-3 NPC readiness diagnostics")
            }

            fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                if access
                    .size_hint()
                    .is_some_and(|hint| hint > EXPECTED_DIAGNOSTIC_COUNT)
                {
                    return Err(de::Error::invalid_length(
                        access.size_hint().unwrap_or(EXPECTED_DIAGNOSTIC_COUNT + 1),
                        &self,
                    ));
                }

                fn next<'de, A>(
                    access: &mut A,
                    index: usize,
                    visitor: &dyn de::Expected,
                ) -> Result<DiagnosticWire, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    access
                        .next_element::<DiagnosticWire>()?
                        .ok_or_else(|| de::Error::invalid_length(index, visitor))
                }

                fn verify<E>(diagnostic: &DiagnosticWire, index: usize) -> Result<(), E>
                where
                    E: de::Error,
                {
                    if diagnostic.code.0 != EXPECTED_DIAGNOSTIC_CODES[index]
                        || diagnostic.severity.0 != EXPECTED_DIAGNOSTIC_SEVERITIES[index]
                        || !diagnostic.blocks_build
                    {
                        return Err(E::custom(format!(
                            "diagnostic {index} does not match the closed code/severity/blocker sequence"
                        )));
                    }
                    Ok(())
                }

                let first = next(&mut access, 0, &self)?;
                verify::<A::Error>(&first, 0)?;
                let second = next(&mut access, 1, &self)?;
                verify::<A::Error>(&second, 1)?;
                let third = next(&mut access, 2, &self)?;
                verify::<A::Error>(&third, 2)?;
                let fourth = next(&mut access, 3, &self)?;
                verify::<A::Error>(&fourth, 3)?;
                if access.next_element::<IgnoredAny>()?.is_some() {
                    return Err(de::Error::invalid_length(
                        EXPECTED_DIAGNOSTIC_COUNT + 1,
                        &self,
                    ));
                }
                Ok(ExactDiagnosticsWire([first, second, third, fourth]))
            }
        }

        deserializer.deserialize_seq(ExactDiagnosticsVisitor)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanWire {
    #[serde(rename = "format")]
    format_marker: PlanFormat,
    schema_revision: PlanSchemaRevision,
    scope: SourceReadinessInspectionOnlyWire,
    source_status: PersistedAndRegeneratedExactWire,
    compiler_status: NotRunWire,
    build_status: BlockedWire,
    runtime_qualification: RuntimeUnqualifiedWire,
    spawn_status: NotSupportedWire,
    publication_status: NotSupportedWire,
    provenance: InspectionProvenanceWire,
    npc: InspectionNpcWire,
    module: InspectionModuleWire,
    diagnostics: ExactDiagnosticsWire,
}

impl From<PlanWire> for Revision3NpcSourceInspectionPlanV1 {
    fn from(wire: PlanWire) -> Self {
        Self {
            format_marker: wire.format_marker,
            schema_revision: wire.schema_revision,
            scope: {
                let SourceReadinessInspectionOnlyWire = wire.scope;
                NpcInspectionScopeV1::SourceReadinessInspectionOnly
            },
            source_status: {
                let PersistedAndRegeneratedExactWire = wire.source_status;
                NpcInspectionSourceStatusV1::PersistedAndRegeneratedExact
            },
            compiler_status: {
                let NotRunWire = wire.compiler_status;
                NpcInspectionCompilerStatusV1::NotRun
            },
            build_status: {
                let BlockedWire = wire.build_status;
                NpcInspectionBuildStatusV1::Blocked
            },
            runtime_qualification: {
                let RuntimeUnqualifiedWire = wire.runtime_qualification;
                NpcInspectionRuntimeQualificationV1::RuntimeUnqualified
            },
            spawn_status: {
                let NotSupportedWire = wire.spawn_status;
                NpcInspectionSpawnStatusV1::NotSupported
            },
            publication_status: {
                let NotSupportedWire = wire.publication_status;
                NpcInspectionPublicationStatusV1::NotSupported
            },
            provenance: wire.provenance.into(),
            npc: wire.npc.into(),
            module: wire.module.into(),
            diagnostics: wire.diagnostics.into_output(),
        }
    }
}

impl Revision3NpcSourceInspectionPlanV1 {
    pub const fn format(&self) -> &'static str {
        PLAN_FORMAT
    }

    pub const fn schema_revision(&self) -> u32 {
        PLAN_SCHEMA_REVISION
    }

    pub const fn scope(&self) -> NpcInspectionScopeV1 {
        self.scope
    }

    pub const fn source_status(&self) -> NpcInspectionSourceStatusV1 {
        self.source_status
    }

    pub const fn compiler_status(&self) -> NpcInspectionCompilerStatusV1 {
        self.compiler_status
    }

    pub const fn build_status(&self) -> NpcInspectionBuildStatusV1 {
        self.build_status
    }

    pub const fn runtime_qualification(&self) -> NpcInspectionRuntimeQualificationV1 {
        self.runtime_qualification
    }

    pub const fn spawn_status(&self) -> NpcInspectionSpawnStatusV1 {
        self.spawn_status
    }

    pub const fn publication_status(&self) -> NpcInspectionPublicationStatusV1 {
        self.publication_status
    }

    pub const fn provenance(&self) -> &Revision3NpcInspectionProvenanceV1 {
        &self.provenance
    }

    pub const fn npc(&self) -> &Revision3NpcInspectionEntityV1 {
        &self.npc
    }

    pub const fn module(&self) -> &Revision3NpcInspectionModuleV1 {
        &self.module
    }

    pub fn diagnostics(&self) -> &[Revision3NpcInspectionDiagnosticV1] {
        &self.diagnostics
    }

    pub fn to_canonical_json(&self) -> Result<String, Revision3NpcInspectionErrorV1> {
        self.validate_closed_invariants()?;
        let mut writer = BoundedJsonWriter::new(MAX_REVISION3_NPC_INSPECTION_PLAN_JSON_BYTES);
        let result = serde_json::to_writer(&mut writer, self);
        if let Some(actual) = writer.first_exceeded_size {
            return Err(Revision3NpcInspectionErrorV1::PlanJsonTooLarge {
                actual,
                limit: MAX_REVISION3_NPC_INSPECTION_PLAN_JSON_BYTES,
            });
        }
        result.map_err(Revision3NpcInspectionErrorV1::SerializePlan)?;
        String::from_utf8(writer.bytes).map_err(|_| {
            Revision3NpcInspectionErrorV1::PlanInvariant(
                "canonical plan serialization produced non-UTF-8 bytes".to_owned(),
            )
        })
    }

    pub fn from_json(json: &str) -> Result<Self, Revision3NpcInspectionErrorV1> {
        if json.len() > MAX_REVISION3_NPC_INSPECTION_PLAN_JSON_BYTES {
            return Err(Revision3NpcInspectionErrorV1::PlanJsonTooLarge {
                actual: json.len(),
                limit: MAX_REVISION3_NPC_INSPECTION_PLAN_JSON_BYTES,
            });
        }
        let plan = serde_json::from_str::<PlanWire>(json)
            .map(Self::from)
            .map_err(Revision3NpcInspectionErrorV1::InvalidPlanJson)?;
        plan.validate_closed_invariants()?;
        if plan.to_canonical_json()?.as_bytes() != json.as_bytes() {
            return Err(Revision3NpcInspectionErrorV1::NonCanonicalPlanJson);
        }
        Ok(plan)
    }

    pub fn content_seal(&self) -> Result<ContentSeal, Revision3NpcInspectionErrorV1> {
        self.to_canonical_json()
            .map(|json| seal_bytes(json.as_bytes()))
    }

    /// Rebuild the complete plan from exact project JSON and require byte-semantic equivalence.
    pub fn verify_against_project(
        &self,
        canonical_project_json: &str,
    ) -> Result<(), Revision3NpcInspectionErrorV1> {
        let expected = build_revision3_npc_source_inspection_plan_v1(
            canonical_project_json,
            self.npc.reference.id,
        )?;
        if &expected != self {
            return Err(Revision3NpcInspectionErrorV1::PlanProjectBindingMismatch);
        }
        Ok(())
    }

    fn validate_closed_invariants(&self) -> Result<(), Revision3NpcInspectionErrorV1> {
        self.validate_field_envelopes()?;
        if self.scope != NpcInspectionScopeV1::SourceReadinessInspectionOnly
            || self.source_status != NpcInspectionSourceStatusV1::PersistedAndRegeneratedExact
            || self.compiler_status != NpcInspectionCompilerStatusV1::NotRun
            || self.build_status != NpcInspectionBuildStatusV1::Blocked
            || self.runtime_qualification != NpcInspectionRuntimeQualificationV1::RuntimeUnqualified
            || self.spawn_status != NpcInspectionSpawnStatusV1::NotSupported
            || self.publication_status != NpcInspectionPublicationStatusV1::NotSupported
        {
            return invariant("inspection plan contains an authority claim");
        }
        if self.provenance.target.executable.byte_len == 0
            || self.provenance.canonical_project.byte_len == 0
            || self.provenance.canonical_project.byte_len
                > u64::try_from(MAX_REVISION3_NPC_INSPECTION_PROJECT_JSON_BYTES).unwrap_or(u64::MAX)
            || self.npc.input_seal.byte_len == 0
            || self.npc.input_seal.byte_len
                > u64::try_from(MAX_REVISION3_ENTITY_JSON_BYTES).unwrap_or(u64::MAX)
            || self.module.persisted_source.byte_len == 0
        {
            return invariant("inspection provenance contains an empty or oversized seal");
        }

        let expected_npc = Revision3TypedRef::new(
            self.provenance.project_id,
            self.npc.reference.id,
            Revision3EntityKind::NpcDraft,
        );
        if self.npc.reference != expected_npc
            || self.module.reference != self.npc.script_module
            || self.module.reference.project_id != self.provenance.project_id
            || self.module.reference.expected_kind != Revision3EntityKind::ScriptModule
            || self.module.reference.id == self.npc.reference.id
        {
            return invariant("NPC/module typed references are not one exact local closure");
        }
        if self.npc.generator_id != LOGICAL_NPC_CLONE_GENERATOR_ID
            || self.npc.generator_version != LOGICAL_NPC_CLONE_GENERATOR_VERSION
            || self.npc.input.target != self.provenance.target
            || self.npc.input.parent_character_definition.generation != self.provenance.target
            || self.npc.input.parent_ai_agent_config.generation != self.provenance.target
            || self.npc.input.parent_spawn_definition.generation != self.provenance.target
        {
            return invariant("NPC generator or persisted parent generation drifted");
        }
        if !matches!(
            &self.npc.origin,
            Revision3OriginRef::New { authored_runtime_id }
                if authored_runtime_id == &self.npc.input.unique_name
        ) {
            return invariant("NPC origin does not match its persisted unique name");
        }
        if !matches!(
            &self.module.origin,
            Revision3OriginRef::Generated {
                generator_id,
                generator_version,
                owner,
            } if generator_id == LOGICAL_NPC_CLONE_GENERATOR_ID
                && *generator_version == LOGICAL_NPC_CLONE_GENERATOR_VERSION
                && owner == &self.npc.reference
        ) {
            return invariant("ScriptModule origin does not match the selected NPC");
        }
        if self.module.generated.owner != self.npc.reference
            || self.module.generated.generator_id != LOGICAL_NPC_CLONE_GENERATOR_ID
            || self.module.generated.generator_version != LOGICAL_NPC_CLONE_GENERATOR_VERSION
            || self.module.generated.status != ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
            || self.module.generated.module_namespace != self.npc.input.module_namespace
        {
            return invariant("regenerated ScriptModule contract drifted");
        }
        if self.module.generated.source.len() > MAX_REVISION3_ENTITY_JSON_BYTES
            || self.module.generated.module_namespace.len() > MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES
            || self.module.generated.module_relative_path.len()
                > MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES + 3
        {
            return invariant("regenerated source or module identity exceeds its closed envelope");
        }
        let expected_path = format!(
            "{}.as",
            self.module.generated.module_namespace.replace('.', "/")
        );
        if self.module.generated.module_relative_path != expected_path {
            return invariant("module namespace/path identity drifted");
        }

        if seal_npc_input(&self.npc.input)? != self.npc.input_seal {
            return invariant("persisted NPC input seal drifted");
        }
        let actual_source = seal_bytes(self.module.generated.source.as_bytes());
        if actual_source != self.module.persisted_source
            || actual_source.sha256 != self.module.generated.source_sha256
        {
            return invariant("persisted and regenerated source seals differ");
        }

        let reconstructed = Revision3NpcDraft {
            generator_id: self.npc.generator_id.clone(),
            generator_version: self.npc.generator_version,
            input: self.npc.input.clone(),
            script_module: self.npc.script_module.clone(),
        };
        let expected = reconstructed
            .regenerate_script_module(self.npc.reference.clone())
            .map_err(|error| {
                Revision3NpcInspectionErrorV1::PlanInvariant(format!(
                    "persisted parent triple cannot regenerate the NPC module: {error}"
                ))
            })?;
        if expected != self.module.generated {
            return invariant(
                "generated source or input fingerprint differs from exact regeneration",
            );
        }
        self.validate_entity_envelopes(&reconstructed)?;
        if self.diagnostics != expected_diagnostics(&self.npc.reference, &self.module.reference) {
            return invariant("readiness diagnostics are incomplete or contain a widened claim");
        }
        Ok(())
    }

    fn validate_field_envelopes(&self) -> Result<(), Revision3NpcInspectionErrorV1> {
        validate_bounded_string(
            "npc.display_name",
            &self.npc.display_name,
            MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1,
        )?;
        validate_bounded_string(
            "module.display_name",
            &self.module.display_name,
            MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1,
        )?;
        validate_bounded_string(
            "npc.generator_id",
            &self.npc.generator_id,
            MAX_STORY_BUILD_GENERATOR_ID_BYTES,
        )?;
        validate_bounded_string(
            "npc.input.module_namespace",
            &self.npc.input.module_namespace,
            MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES,
        )?;
        validate_bounded_string(
            "npc.input.unique_name",
            &self.npc.input.unique_name,
            MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES,
        )?;
        for (field, parent) in [
            (
                "npc.input.parent_character_definition",
                &self.npc.input.parent_character_definition,
            ),
            (
                "npc.input.parent_ai_agent_config",
                &self.npc.input.parent_ai_agent_config,
            ),
            (
                "npc.input.parent_spawn_definition",
                &self.npc.input.parent_spawn_definition,
            ),
        ] {
            validate_bounded_string(
                parent_field(field, "catalog_layer"),
                &parent.catalog_layer,
                MAX_NPC_PARENT_CATALOG_LAYER_BYTES,
            )?;
            validate_bounded_string(
                parent_field(field, "canonical_selector"),
                &parent.canonical_selector,
                MAX_NPC_PARENT_SELECTOR_BYTES,
            )?;
            validate_bounded_string(
                parent_field(field, "runtime_class"),
                &parent.runtime_class,
                MAX_ANGELSCRIPT_IDENTIFIER_BYTES,
            )?;
        }
        let Revision3OriginRef::New {
            authored_runtime_id,
        } = &self.npc.origin
        else {
            return invariant("NPC origin is not a bounded authored runtime identity");
        };
        validate_bounded_string(
            "npc.origin.authored_runtime_id",
            authored_runtime_id,
            MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES,
        )?;
        let Revision3OriginRef::Generated { generator_id, .. } = &self.module.origin else {
            return invariant("ScriptModule origin is not bounded generated provenance");
        };
        validate_bounded_string(
            "module.origin.generator_id",
            generator_id,
            MAX_STORY_BUILD_GENERATOR_ID_BYTES,
        )?;
        validate_bounded_string(
            "module.generated.generator_id",
            &self.module.generated.generator_id,
            MAX_STORY_BUILD_GENERATOR_ID_BYTES,
        )?;
        validate_bounded_string(
            "module.generated.module_namespace",
            &self.module.generated.module_namespace,
            MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES,
        )?;
        validate_bounded_string(
            "module.generated.module_relative_path",
            &self.module.generated.module_relative_path,
            MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES + 3,
        )?;
        validate_bounded_string(
            "module.generated.source",
            &self.module.generated.source,
            MAX_REVISION3_ENTITY_JSON_BYTES,
        )?;
        if self.diagnostics.len() != EXPECTED_DIAGNOSTIC_COUNT {
            return invariant("readiness diagnostic count is not the closed V1 count");
        }
        for diagnostic in &self.diagnostics {
            validate_bounded_string(
                "diagnostic.property_path",
                &diagnostic.property_path,
                MAX_STORY_BUILD_PROPERTY_PATH_BYTES,
            )?;
            validate_bounded_string(
                "diagnostic.message",
                &diagnostic.message,
                MAX_STORY_BUILD_DIAGNOSTIC_MESSAGE_BYTES,
            )?;
        }
        Ok(())
    }

    fn validate_entity_envelopes(
        &self,
        reconstructed: &Revision3NpcDraft,
    ) -> Result<(), Revision3NpcInspectionErrorV1> {
        let npc_entity = Revision3Entity {
            id: self.npc.reference.id,
            display_name: self.npc.display_name.clone(),
            origin: self.npc.origin.clone(),
            revision: self.npc.entity_revision,
            payload: Revision3EntityPayload::NpcDraft(reconstructed.clone()),
        };
        validate_serialized_envelope("NPC entity JSON", &npc_entity)?;
        let module_entity = Revision3Entity {
            id: self.module.reference.id,
            display_name: self.module.display_name.clone(),
            origin: self.module.origin.clone(),
            revision: self.module.entity_revision,
            payload: Revision3EntityPayload::ScriptModule(self.module.generated.clone()),
        };
        validate_serialized_envelope("ScriptModule entity JSON", &module_entity)
    }
}

/// Build one pure read-only plan from exact canonical schema-revision-3 project JSON.
pub fn build_revision3_npc_source_inspection_plan_v1(
    canonical_project_json: &str,
    npc_id: EntityId,
) -> Result<Revision3NpcSourceInspectionPlanV1, Revision3NpcInspectionErrorV1> {
    if canonical_project_json.len() > MAX_REVISION3_NPC_INSPECTION_PROJECT_JSON_BYTES {
        return Err(Revision3NpcInspectionErrorV1::ProjectJsonTooLarge {
            actual: canonical_project_json.len(),
            limit: MAX_REVISION3_NPC_INSPECTION_PROJECT_JSON_BYTES,
        });
    }
    let document = ProjectDocument::from_json(canonical_project_json)
        .map_err(Revision3NpcInspectionErrorV1::InvalidProjectDocument)?;
    let canonical = document
        .to_canonical_json()
        .map_err(Revision3NpcInspectionErrorV1::SerializeProject)?;
    if canonical.as_bytes() != canonical_project_json.as_bytes() {
        return Err(Revision3NpcInspectionErrorV1::NonCanonicalProjectJson);
    }
    let ProjectDocument::Revision3(project) = document else {
        return Err(Revision3NpcInspectionErrorV1::Revision3Required);
    };

    let npc_entity = project
        .entities
        .get(&npc_id)
        .ok_or(Revision3NpcInspectionErrorV1::MissingNpc(npc_id))?;
    let Revision3EntityPayload::NpcDraft(npc) = &npc_entity.payload else {
        return Err(Revision3NpcInspectionErrorV1::NotAnNpc(npc_id));
    };
    let expected = verify_project_closure(&project, npc_id, npc_entity, npc)?;
    let module_entity = project.entities.get(&npc.script_module.id).ok_or(
        Revision3NpcInspectionErrorV1::MissingScriptModule {
            npc: npc_id,
            module: npc.script_module.id,
        },
    )?;

    let npc_ref = Revision3TypedRef::new(project.project_id, npc_id, Revision3EntityKind::NpcDraft);
    let module_ref = npc.script_module.clone();
    let plan = Revision3NpcSourceInspectionPlanV1 {
        format_marker: PlanFormat,
        schema_revision: PlanSchemaRevision,
        scope: NpcInspectionScopeV1::SourceReadinessInspectionOnly,
        source_status: NpcInspectionSourceStatusV1::PersistedAndRegeneratedExact,
        compiler_status: NpcInspectionCompilerStatusV1::NotRun,
        build_status: NpcInspectionBuildStatusV1::Blocked,
        runtime_qualification: NpcInspectionRuntimeQualificationV1::RuntimeUnqualified,
        spawn_status: NpcInspectionSpawnStatusV1::NotSupported,
        publication_status: NpcInspectionPublicationStatusV1::NotSupported,
        provenance: Revision3NpcInspectionProvenanceV1 {
            project_id: project.project_id,
            project_revision: project.revision,
            target: project.target.clone(),
            canonical_project: seal_bytes(canonical_project_json.as_bytes()),
        },
        npc: Revision3NpcInspectionEntityV1 {
            reference: npc_ref.clone(),
            entity_revision: npc_entity.revision,
            display_name: npc_entity.display_name.clone(),
            origin: npc_entity.origin.clone(),
            generator_id: npc.generator_id.clone(),
            generator_version: npc.generator_version,
            input: npc.input.clone(),
            input_seal: seal_npc_input(&npc.input)?,
            script_module: module_ref.clone(),
        },
        module: Revision3NpcInspectionModuleV1 {
            reference: module_ref.clone(),
            entity_revision: module_entity.revision,
            display_name: module_entity.display_name.clone(),
            origin: module_entity.origin.clone(),
            persisted_source: seal_bytes(expected.source.as_bytes()),
            generated: expected,
        },
        diagnostics: expected_diagnostics(&npc_ref, &module_ref),
    };
    plan.validate_closed_invariants()?;
    let canonical_plan = plan.to_canonical_json()?;
    if Revision3NpcSourceInspectionPlanV1::from_json(&canonical_plan)? != plan {
        return invariant("canonical NPC inspection plan did not reopen exactly");
    }
    Ok(plan)
}

fn verify_project_closure(
    project: &gore_authoring::ProjectRevision3,
    npc_id: EntityId,
    npc_entity: &Revision3Entity,
    npc: &Revision3NpcDraft,
) -> Result<Revision3ScriptModule, Revision3NpcInspectionErrorV1> {
    if npc_entity.id != npc_id
        || npc.generator_id != LOGICAL_NPC_CLONE_GENERATOR_ID
        || npc.generator_version != LOGICAL_NPC_CLONE_GENERATOR_VERSION
    {
        return Err(Revision3NpcInspectionErrorV1::ForeignGenerator { npc: npc_id });
    }
    if npc.input.target != project.target
        || npc.input.parent_character_definition.generation != project.target
        || npc.input.parent_ai_agent_config.generation != project.target
        || npc.input.parent_spawn_definition.generation != project.target
    {
        return Err(Revision3NpcInspectionErrorV1::ForeignGeneration { npc: npc_id });
    }
    if !matches!(
        &npc_entity.origin,
        Revision3OriginRef::New { authored_runtime_id }
            if authored_runtime_id == &npc.input.unique_name
    ) {
        return Err(Revision3NpcInspectionErrorV1::OwnerMismatch {
            npc: npc_id,
            module: npc.script_module.id,
        });
    }
    if npc.script_module.project_id != project.project_id
        || npc.script_module.expected_kind != Revision3EntityKind::ScriptModule
        || npc.script_module.id == npc_id
    {
        return Err(Revision3NpcInspectionErrorV1::OwnerMismatch {
            npc: npc_id,
            module: npc.script_module.id,
        });
    }
    let module_entity = project.entities.get(&npc.script_module.id).ok_or(
        Revision3NpcInspectionErrorV1::MissingScriptModule {
            npc: npc_id,
            module: npc.script_module.id,
        },
    )?;
    let Revision3EntityPayload::ScriptModule(persisted) = &module_entity.payload else {
        return Err(Revision3NpcInspectionErrorV1::MissingScriptModule {
            npc: npc_id,
            module: npc.script_module.id,
        });
    };
    let owner = Revision3TypedRef::new(project.project_id, npc_id, Revision3EntityKind::NpcDraft);
    if persisted.owner != owner
        || persisted.generator_id != LOGICAL_NPC_CLONE_GENERATOR_ID
        || persisted.generator_version != LOGICAL_NPC_CLONE_GENERATOR_VERSION
        || persisted.status != ScriptModuleStatus::OFFLINE_DRAFT_RUNTIME_UNQUALIFIED
        || !matches!(
            &module_entity.origin,
            Revision3OriginRef::Generated {
                generator_id,
                generator_version,
                owner: origin_owner,
            } if generator_id == LOGICAL_NPC_CLONE_GENERATOR_ID
                && *generator_version == LOGICAL_NPC_CLONE_GENERATOR_VERSION
                && origin_owner == &owner
        )
    {
        return Err(Revision3NpcInspectionErrorV1::OwnerMismatch {
            npc: npc_id,
            module: npc.script_module.id,
        });
    }
    let source_sha256 =
        Sha256Digest::from_bytes(Sha256::digest(persisted.source.as_bytes()).into());
    if persisted.source_sha256 != source_sha256 {
        return Err(Revision3NpcInspectionErrorV1::PersistedSourceSealMismatch {
            npc: npc_id,
            module: npc.script_module.id,
        });
    }
    let expected = npc
        .regenerate_script_module(owner)
        .map_err(Revision3NpcInspectionErrorV1::RegenerateNpc)?;
    if persisted.input_fingerprint != expected.input_fingerprint {
        return Err(Revision3NpcInspectionErrorV1::InputFingerprintMismatch {
            npc: npc_id,
            module: npc.script_module.id,
        });
    }
    if persisted != &expected {
        return Err(Revision3NpcInspectionErrorV1::PersistedModuleDrift {
            npc: npc_id,
            module: npc.script_module.id,
        });
    }
    Ok(expected)
}

fn expected_diagnostics(
    npc: &Revision3TypedRef,
    module: &Revision3TypedRef,
) -> Vec<Revision3NpcInspectionDiagnosticV1> {
    vec![
        Revision3NpcInspectionDiagnosticV1 {
            code: NpcInspectionDiagnosticCodeV1::NpcCompilerNotRun,
            severity: NpcInspectionDiagnosticSeverityV1::Warning,
            entity: module.clone(),
            property_path: "payload.data.source".to_owned(),
            message: "The exact generated NPC source was not submitted to a compiler by this read-only inspection.".to_owned(),
            blocks_build: true,
        },
        Revision3NpcInspectionDiagnosticV1 {
            code: NpcInspectionDiagnosticCodeV1::NpcProductionLoweringUnavailable,
            severity: NpcInspectionDiagnosticSeverityV1::Error,
            entity: npc.clone(),
            property_path: "payload.data.script_module".to_owned(),
            message: "Production lowering for revision-3 NPC drafts is unavailable.".to_owned(),
            blocks_build: true,
        },
        Revision3NpcInspectionDiagnosticV1 {
            code: NpcInspectionDiagnosticCodeV1::NpcRuntimeResidenceUnqualified,
            severity: NpcInspectionDiagnosticSeverityV1::Error,
            entity: npc.clone(),
            property_path: "payload.data.script_module".to_owned(),
            message: "NPC class residence, effective behavior, distinct state, and persistence are runtime-unqualified.".to_owned(),
            blocks_build: true,
        },
        Revision3NpcInspectionDiagnosticV1 {
            code: NpcInspectionDiagnosticCodeV1::NpcSpawnUnavailable,
            severity: NpcInspectionDiagnosticSeverityV1::Error,
            entity: npc.clone(),
            property_path: "payload.data.input".to_owned(),
            message: "No qualified spawn or world-placement mechanism is available for this NPC draft.".to_owned(),
            blocks_build: true,
        },
    ]
}

fn validate_bounded_string(
    field: &'static str,
    value: &str,
    limit: usize,
) -> Result<(), Revision3NpcInspectionErrorV1> {
    if value.len() > limit {
        return Err(Revision3NpcInspectionErrorV1::PlanFieldTooLarge {
            field,
            actual: value.len(),
            limit,
        });
    }
    Ok(())
}

fn parent_field(parent: &'static str, property: &'static str) -> &'static str {
    match (parent, property) {
        ("npc.input.parent_character_definition", "catalog_layer") => {
            "npc.input.parent_character_definition.catalog_layer"
        }
        ("npc.input.parent_character_definition", "canonical_selector") => {
            "npc.input.parent_character_definition.canonical_selector"
        }
        ("npc.input.parent_character_definition", "runtime_class") => {
            "npc.input.parent_character_definition.runtime_class"
        }
        ("npc.input.parent_ai_agent_config", "catalog_layer") => {
            "npc.input.parent_ai_agent_config.catalog_layer"
        }
        ("npc.input.parent_ai_agent_config", "canonical_selector") => {
            "npc.input.parent_ai_agent_config.canonical_selector"
        }
        ("npc.input.parent_ai_agent_config", "runtime_class") => {
            "npc.input.parent_ai_agent_config.runtime_class"
        }
        ("npc.input.parent_spawn_definition", "catalog_layer") => {
            "npc.input.parent_spawn_definition.catalog_layer"
        }
        ("npc.input.parent_spawn_definition", "canonical_selector") => {
            "npc.input.parent_spawn_definition.canonical_selector"
        }
        ("npc.input.parent_spawn_definition", "runtime_class") => {
            "npc.input.parent_spawn_definition.runtime_class"
        }
        _ => "npc.input.parent.unknown",
    }
}

fn seal_npc_input(
    input: &Revision3NpcDraftInput,
) -> Result<ContentSeal, Revision3NpcInspectionErrorV1> {
    let mut writer = BoundedHashWriter::new(MAX_REVISION3_ENTITY_JSON_BYTES);
    let result = serde_json::to_writer(&mut writer, input);
    if let Some(actual) = writer.first_exceeded_size {
        return Err(Revision3NpcInspectionErrorV1::PlanFieldTooLarge {
            field: "npc.input JSON",
            actual,
            limit: MAX_REVISION3_ENTITY_JSON_BYTES,
        });
    }
    result.map_err(Revision3NpcInspectionErrorV1::SerializeNpcInput)?;
    Ok(writer.finish())
}

fn validate_serialized_envelope<T: Serialize>(
    envelope: &'static str,
    value: &T,
) -> Result<(), Revision3NpcInspectionErrorV1> {
    let mut writer = BoundedHashWriter::new(MAX_REVISION3_ENTITY_JSON_BYTES);
    let result = serde_json::to_writer(&mut writer, value);
    if let Some(actual) = writer.first_exceeded_size {
        return Err(Revision3NpcInspectionErrorV1::PlanFieldTooLarge {
            field: envelope,
            actual,
            limit: MAX_REVISION3_ENTITY_JSON_BYTES,
        });
    }
    result.map_err(
        |source| Revision3NpcInspectionErrorV1::SerializeEntityEnvelope { envelope, source },
    )
}

struct BoundedJsonWriter {
    bytes: Vec<u8>,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedJsonWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(64 * 1_024)),
            limit,
            first_exceeded_size: None,
        }
    }
}

impl Write for BoundedJsonWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let attempted = self.bytes.len().saturating_add(buffer.len());
        if attempted > self.limit {
            self.first_exceeded_size = Some(attempted);
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "revision-3 NPC inspection plan exceeded its JSON limit",
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BoundedHashWriter {
    hasher: Sha256,
    byte_len: usize,
    limit: usize,
    first_exceeded_size: Option<usize>,
}

impl BoundedHashWriter {
    fn new(limit: usize) -> Self {
        Self {
            hasher: Sha256::new(),
            byte_len: 0,
            limit,
            first_exceeded_size: None,
        }
    }

    fn finish(self) -> ContentSeal {
        ContentSeal {
            byte_len: self.byte_len as u64,
            sha256: Sha256Digest::from_bytes(self.hasher.finalize().into()),
        }
    }
}

impl Write for BoundedHashWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let attempted = self.byte_len.saturating_add(buffer.len());
        if attempted > self.limit {
            self.first_exceeded_size = Some(attempted);
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "revision-3 NPC inspection envelope exceeded its JSON limit",
            ));
        }
        self.hasher.update(buffer);
        self.byte_len = attempted;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn seal_bytes(bytes: &[u8]) -> ContentSeal {
    ContentSeal {
        byte_len: bytes.len() as u64,
        sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
    }
}

fn invariant<T>(message: impl Into<String>) -> Result<T, Revision3NpcInspectionErrorV1> {
    Err(Revision3NpcInspectionErrorV1::PlanInvariant(message.into()))
}

#[derive(Debug, thiserror::Error)]
pub enum Revision3NpcInspectionErrorV1 {
    #[error("revision-3 NPC project JSON exceeds the {limit}-byte limit: {actual} bytes")]
    ProjectJsonTooLarge { actual: usize, limit: usize },
    #[error("invalid authoring project document: {0}")]
    InvalidProjectDocument(#[source] ProjectDocumentError),
    #[error("could not canonicalize authoring project: {0}")]
    SerializeProject(#[source] serde_json::Error),
    #[error("revision-3 NPC inspection requires exact canonical project JSON")]
    NonCanonicalProjectJson,
    #[error("revision-3 NPC inspection requires authoring schema revision 3")]
    Revision3Required,
    #[error("revision-3 NPC {0} is absent")]
    MissingNpc(EntityId),
    #[error("revision-3 entity {0} is not an NPC Draft")]
    NotAnNpc(EntityId),
    #[error("revision-3 NPC {npc} has a foreign generator contract")]
    ForeignGenerator { npc: EntityId },
    #[error("revision-3 NPC {npc} has foreign generation provenance")]
    ForeignGeneration { npc: EntityId },
    #[error("revision-3 NPC {npc} ScriptModule {module} is missing or mistyped")]
    MissingScriptModule { npc: EntityId, module: EntityId },
    #[error("revision-3 NPC {npc} / ScriptModule {module} owner or origin mismatch")]
    OwnerMismatch { npc: EntityId, module: EntityId },
    #[error("revision-3 NPC {npc} ScriptModule {module} source seal mismatch")]
    PersistedSourceSealMismatch { npc: EntityId, module: EntityId },
    #[error("revision-3 NPC {npc} ScriptModule {module} input fingerprint mismatch")]
    InputFingerprintMismatch { npc: EntityId, module: EntityId },
    #[error("revision-3 NPC {npc} ScriptModule {module} differs from exact regeneration")]
    PersistedModuleDrift { npc: EntityId, module: EntityId },
    #[error("could not regenerate the persisted revision-3 NPC parent triple: {0}")]
    RegenerateNpc(#[source] StoryRegenerationError),
    #[error("could not serialize revision-3 NPC input: {0}")]
    SerializeNpcInput(#[source] serde_json::Error),
    #[error("revision-3 NPC inspection field {field} exceeds {limit} bytes: {actual}")]
    PlanFieldTooLarge {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("could not serialize bounded {envelope}: {source}")]
    SerializeEntityEnvelope {
        envelope: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("could not serialize revision-3 NPC inspection plan: {0}")]
    SerializePlan(#[source] serde_json::Error),
    #[error("invalid revision-3 NPC inspection plan JSON: {0}")]
    InvalidPlanJson(#[source] serde_json::Error),
    #[error("revision-3 NPC inspection plan JSON is not canonical")]
    NonCanonicalPlanJson,
    #[error("revision-3 NPC inspection plan exceeds {limit} bytes: {actual}")]
    PlanJsonTooLarge { actual: usize, limit: usize },
    #[error("revision-3 NPC inspection invariant failed: {0}")]
    PlanInvariant(String),
    #[error("revision-3 NPC inspection plan does not match the exact source project")]
    PlanProjectBindingMismatch,
}

#[cfg(test)]
mod bounded_writer_tests {
    use std::io::Write as _;

    use super::{BoundedHashWriter, BoundedJsonWriter};

    #[test]
    fn bounded_writers_never_retain_or_hash_bytes_past_their_limit() {
        let mut json = BoundedJsonWriter::new(4);
        json.write_all(b"1234").unwrap();
        assert!(json.write_all(b"5").is_err());
        assert_eq!(json.bytes, b"1234");
        assert_eq!(json.first_exceeded_size, Some(5));

        let mut hash = BoundedHashWriter::new(4);
        hash.write_all(b"1234").unwrap();
        assert!(hash.write_all(b"5").is_err());
        assert_eq!(hash.byte_len, 4);
        assert_eq!(hash.first_exceeded_size, Some(5));
    }
}
