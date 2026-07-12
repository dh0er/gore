//! Durable, deployment-independent authoring primitives for GORE Mod Studio.
//!
//! This first phase deliberately models only the concepts needed for a
//! line-centric voice workflow: localization entries, dialog lines, semantic
//! voice slots, and reusable voice takes. It does not imply runtime support for
//! quests, NPCs, or new cooked identities.

mod ids;
mod model;
mod npc;
mod quest;
mod validate;

pub use ids::{EntityId, FixedHexError, ProjectId, Sha256Digest};
pub use model::{
    ArchiveSeal, AssetMeta, AssetRef, AssetStoreIndex, ContentSeal, DialogLine, Entity, EntityKind,
    EntityPayload, FormatV2, GameGenerationAnchor, LocaleCode, LocaleCodeError, LocalizationEntry,
    OggCodec, OggMetadata, OriginRef, ProjectJsonError, ProjectMeta, ProjectV2, SchemaRevisionV1,
    TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake, VoiceTakeStatus, VoiceTarget,
    VoiceTargetResolution, MAX_PROJECT_JSON_BYTES,
};
pub use npc::{
    LogicalNpcCloneAuthoringStatus, LogicalNpcCloneCapabilityStatus, LogicalNpcCloneClassNames,
    LogicalNpcCloneDraft, LogicalNpcCloneDraftError, LogicalNpcCloneField,
    LogicalNpcCloneRuntimeStatus, LogicalNpcCloneSource, MAX_ANGELSCRIPT_IDENTIFIER_BYTES,
    MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES, MAX_ANGELSCRIPT_MODULE_SEGMENTS,
    MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES,
};
pub use quest::{
    CatalogQualifiedParentQuest, CatalogQualifiedQuestGiver, DraftQuestAuthoringStatus,
    DraftQuestCapabilityStatus, DraftQuestCatalogLayerAnchor, DraftQuestCollisionCatalog,
    DraftQuestCollisionKind, DraftQuestDiscoveryStatus, DraftQuestField, DraftQuestFixedShape,
    DraftQuestGeneratedSource, DraftQuestSkeletonError, DraftQuestSkeletonInput,
    DraftQuestSkeletonV1, DraftQuestTechnicalNames, DraftQuestTransitionStatus,
    DRAFT_QUEST_GENERATOR_VERSION, MAX_DRAFT_QUEST_CATALOG_LAYER_BYTES,
    MAX_DRAFT_QUEST_DESCRIPTION_BYTES, MAX_DRAFT_QUEST_OBJECTIVE_TITLE_BYTES,
    MAX_DRAFT_QUEST_TITLE_BYTES,
};
pub use validate::{Diagnostic, DiagnosticCode, DiagnosticSeverity, ValidationProfile};
