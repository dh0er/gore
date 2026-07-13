//! Durable, deployment-independent authoring primitives for GORE Mod Studio.
//!
//! This first phase deliberately models only the concepts needed for a
//! line-centric voice workflow: localization entries, dialog lines, semantic
//! voice slots, and reusable voice takes. It does not imply runtime support for
//! quests, NPCs, or new cooked identities.

mod document;
mod ids;
mod migration;
mod migration_revision3;
mod model;
pub mod model_revision2;
pub mod model_revision3;
mod npc;
mod quest;
mod revision3_quest;
mod revision3_quest_source_v2;
mod story_collision;
mod story_transaction;
mod story_transaction_revision3;
mod strict_json;
mod validate;
mod validate_revision2;
mod validate_revision3;
mod working_store;

pub use document::{ProjectDocument, ProjectDocumentError};
pub use ids::{EntityId, FixedHexError, ProjectId, Sha256Digest};
pub use migration::{
    migrate_revision1_to_revision2, Revision1ToRevision2Error, Revision1ToRevision2Migration,
    Revision1ToRevision2Report, Revision1ToRevision2Transformation, Revision1TypedRefPosition,
};
pub use migration_revision3::{
    migrate_revision2_to_revision3, Revision2ToRevision3Error, Revision2ToRevision3Migration,
    Revision2ToRevision3Report,
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
pub use model_revision3::{
    Entity as Revision3Entity, EntityKind as Revision3EntityKind,
    EntityPayload as Revision3EntityPayload, NpcDraft as Revision3NpcDraft,
    NpcDraftInput as Revision3NpcDraftInput, OriginRef as Revision3OriginRef, ProjectRevision3,
    ProjectRevision3JsonError, ProjectRevision3ValidationError, QuestCollisionArtifactRef,
    QuestDraft as Revision3QuestDraft, QuestDraftInput as Revision3QuestDraftInput,
    QuestGiverInput as Revision3QuestGiverInput, QuestParentInput as Revision3QuestParentInput,
    SchemaRevisionV3, ScriptModule as Revision3ScriptModule, TypedRef as Revision3TypedRef,
    MAX_QUEST_COLLISION_ARTIFACT_BYTES, MAX_REVISION3_ASSETS, MAX_REVISION3_ENTITIES,
    MAX_REVISION3_ENTITY_JSON_BYTES, MAX_REVISION3_REFERENCED_ASSET_BYTES,
    MAX_REVISION3_SNAPSHOT_BYTES, QUEST_COLLISION_ARTIFACT_FORMAT,
    QUEST_COLLISION_ARTIFACT_MEDIA_TYPE, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
    QUEST_COLLISION_ARTIFACT_SCHEMA_REVISION, QUEST_COLLISION_CATALOG_LAYER,
    QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_QUEST_GENERATOR_ID,
    REVISION3_QUEST_GENERATOR_VERSION,
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
pub use revision3_quest::{
    project_revision3_quest_free_basis_to_revision2, regenerate_revision3_quest_module_v2,
    revision3_quest_input_fingerprint_v2, Revision3QuestFreeBasisError,
    Revision3QuestGenerationError,
};
pub use revision3_quest_source_v2::{
    PreparedRevision3QuestCollisionSourceV2, Revision3NonQuestCollisionBasisV2,
    Revision3PriorQuestEvidenceV2, Revision3QuestCollisionSourceErrorV2,
    MAX_REVISION3_COLLISION_IDENTITIES_V2, MAX_REVISION3_COLLISION_IDENTITY_BYTES_V2,
    MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2, MAX_REVISION3_PRIOR_QUESTS_V2,
};
pub use story_collision::{
    collect_project_story_collision_identities, ProjectStoryCollisionIdentities,
    StoryCollisionCollectionError,
};
pub use story_transaction::{
    story_draft_insert_request_binding_sha256, NpcDraftCreateInput, QuestDraftCreateInput,
    StoryDraftCreate, StoryDraftInsertError, StoryDraftInsertEvaluation, StoryDraftInsertJsonError,
    StoryDraftInsertOutcome, StoryDraftInsertRejection, StoryDraftInsertRequest,
    MAX_STORY_DRAFT_DISPLAY_NAME_BYTES, MAX_STORY_DRAFT_INSERT_JSON_BYTES,
};
pub use story_transaction_revision3::{
    apply_revision3_quest_draft_transaction_v2, Revision3QuestArtifactAuthorityV2,
    Revision3QuestDraftBuildStatusV2, Revision3QuestDraftInsertConflictV2,
    Revision3QuestDraftInsertErrorV2, Revision3QuestDraftInsertEvaluationV2,
    Revision3QuestDraftInsertOutcomeV2, Revision3QuestDraftInsertRejectionV2,
    Revision3QuestDraftInsertRequestJsonErrorV2, Revision3QuestDraftInsertRequestV2,
    Revision3QuestDraftIntentV2, Revision3QuestDraftRuntimeStatusV2, Revision3QuestEntityRoleV2,
    Revision3QuestSourceInspectionStatusV2, Revision3StoryIdentityKindV2,
    MAX_REVISION3_QUEST_DRAFT_DISPLAY_NAME_BYTES, MAX_REVISION3_QUEST_DRAFT_REQUEST_JSON_BYTES,
};
pub use validate::{Diagnostic, DiagnosticCode, DiagnosticSeverity, ValidationProfile};
pub use working_store::{
    AssetVerification, CheckpointPreparation, ImportedOgg, ImportedQuestCollisionArtifactV1,
    OpenedCheckpoint, OpenedDocumentCheckpoint, OpenedRevision3Checkpoint,
    Revision3CheckpointPreparation, Revision3SnapshotManifest, WorkingHead, WorkingProjectStore,
    WorkingStoreError, WorkingStoreFormat, WorkingStoreLimits,
};
