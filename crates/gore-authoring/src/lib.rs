//! Durable, deployment-independent authoring primitives for GORE Mod Studio.
//!
//! This first phase deliberately models only the concepts needed for a
//! line-centric voice workflow: localization entries, dialog lines, semantic
//! voice slots, and reusable voice takes. It does not imply runtime support for
//! quests, NPCs, or new cooked identities.

mod document;
mod ids;
mod migration;
mod model;
pub mod model_revision2;
mod npc;
mod quest;
mod story_transaction;
mod strict_json;
mod validate;
mod validate_revision2;
mod working_store;

pub use document::{ProjectDocument, ProjectDocumentError};
pub use ids::{EntityId, FixedHexError, ProjectId, Sha256Digest};
pub use migration::{
    migrate_revision1_to_revision2, Revision1ToRevision2Error, Revision1ToRevision2Migration,
    Revision1ToRevision2Report, Revision1ToRevision2Transformation, Revision1TypedRefPosition,
};
pub use model::{
    ArchiveSeal, AssetMeta, AssetRef, AssetStoreIndex, ContentSeal, DialogLine, Entity, EntityKind,
    EntityPayload, FormatV2, GameGenerationAnchor, LocaleCode, LocaleCodeError, LocalizationEntry,
    OggCodec, OggMetadata, OriginRef, ProjectJsonError, ProjectMeta, ProjectV2, SchemaRevisionV1,
    TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot, VoiceTake, VoiceTakeStatus, VoiceTarget,
    VoiceTargetResolution, MAX_PROJECT_JSON_BYTES,
};
pub use model_revision2::{
    DialogLine as Revision2DialogLine, Entity as Revision2Entity,
    EntityKind as Revision2EntityKind, EntityPayload as Revision2EntityPayload,
    LocalizationEntry as Revision2LocalizationEntry, NpcDraft as Revision2NpcDraft,
    NpcDraftInput as Revision2NpcDraftInput, NpcParentClassInput as Revision2NpcParentClassInput,
    OggCodec as Revision2OggCodec, OggMetadata as Revision2OggMetadata,
    OriginRef as Revision2OriginRef, ProjectRevision2, QuestCollisionCatalogInput,
    QuestDraft as Revision2QuestDraft, QuestDraftInput as Revision2QuestDraftInput,
    QuestGiverInput as Revision2QuestGiverInput, QuestParentInput as Revision2QuestParentInput,
    SchemaRevisionV2, ScriptAuthoringStatus, ScriptModule as Revision2ScriptModule,
    ScriptModuleStatus, ScriptRuntimeStatus, StoryRegenerationError, TypedRef as Revision2TypedRef,
    VoiceMemberProof as Revision2VoiceMemberProof, VoiceOperation as Revision2VoiceOperation,
    VoiceSlot as Revision2VoiceSlot, VoiceTake as Revision2VoiceTake,
    VoiceTakeStatus as Revision2VoiceTakeStatus, VoiceTarget as Revision2VoiceTarget,
    VoiceTargetResolution as Revision2VoiceTargetResolution,
};
pub use npc::{
    LogicalNpcCloneAuthoringStatus, LogicalNpcCloneCapabilityStatus, LogicalNpcCloneClassNames,
    LogicalNpcCloneDraft, LogicalNpcCloneDraftError, LogicalNpcCloneField,
    LogicalNpcCloneRuntimeStatus, LogicalNpcCloneSource, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION, MAX_ANGELSCRIPT_IDENTIFIER_BYTES,
    MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES, MAX_ANGELSCRIPT_MODULE_SEGMENTS,
    MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES,
};
pub use quest::{
    CatalogQualifiedParentQuest, CatalogQualifiedQuestGiver, DraftQuestAuthoringStatus,
    DraftQuestCapabilityStatus, DraftQuestCatalogLayerAnchor, DraftQuestCollisionCatalog,
    DraftQuestCollisionKind, DraftQuestDiscoveryStatus, DraftQuestField, DraftQuestFixedShape,
    DraftQuestGeneratedSource, DraftQuestSkeletonError, DraftQuestSkeletonInput,
    DraftQuestSkeletonV1, DraftQuestTechnicalNames, DraftQuestTransitionStatus,
    DRAFT_QUEST_GENERATOR_ID, DRAFT_QUEST_GENERATOR_VERSION, MAX_DRAFT_QUEST_CATALOG_LAYER_BYTES,
    MAX_DRAFT_QUEST_DESCRIPTION_BYTES, MAX_DRAFT_QUEST_OBJECTIVE_TITLE_BYTES,
    MAX_DRAFT_QUEST_TITLE_BYTES,
};
pub use story_transaction::{
    story_draft_insert_request_binding_sha256, NpcDraftCreateInput, QuestDraftCreateInput,
    StoryDraftCreate, StoryDraftInsertError, StoryDraftInsertEvaluation, StoryDraftInsertJsonError,
    StoryDraftInsertOutcome, StoryDraftInsertRejection, StoryDraftInsertRequest,
    MAX_STORY_DRAFT_DISPLAY_NAME_BYTES, MAX_STORY_DRAFT_INSERT_JSON_BYTES,
};
pub use validate::{Diagnostic, DiagnosticCode, DiagnosticSeverity, ValidationProfile};
pub use working_store::{
    AssetVerification, CheckpointPreparation, ImportedOgg, OpenedCheckpoint,
    OpenedDocumentCheckpoint, WorkingHead, WorkingProjectStore, WorkingStoreError,
    WorkingStoreFormat, WorkingStoreLimits,
};
