//! Durable, deployment-independent authoring primitives for GORE Mod Studio.
//!
//! This first phase deliberately models only the concepts needed for a
//! line-centric voice workflow: localization entries, dialog lines, semantic
//! voice slots, and reusable voice takes. It does not imply runtime support for
//! quests, NPCs, or new cooked identities.

mod dataasset_build_receipt;
mod dataasset_stage;
mod document;
mod ids;
mod migration;
mod migration_revision3;
mod model;
pub mod model_revision2;
pub mod model_revision3;
mod npc;
mod quest;
mod revision3_content_index;
mod revision3_quest;
mod revision3_quest_source_v2;
mod revision3_voice_build;
mod story_collision;
mod story_transaction;
mod story_transaction_revision3;
mod story_transaction_revision3_dialog;
mod story_transaction_revision3_dialog_localization_edit;
mod story_transaction_revision3_npc;
mod story_transaction_revision3_quest_outline;
mod story_transaction_revision3_quest_outline_v2;
mod story_transaction_revision3_quest_transitions;
mod story_transaction_revision3_voice;
mod story_transaction_revision3_voice_selection;
mod story_transaction_revision3_voice_target;
mod strict_json;
mod validate;
mod validate_revision2;
mod validate_revision3;
mod working_store;

pub use dataasset_build_receipt::{
    ManagedDataAssetBuildAuthorityV1, ManagedDataAssetPublicationAuthorityV1,
    ManagedDataAssetRuntimeStatusV1, ManagedDataAssetTripletFileSealV1,
    ManagedRevision3ReviewedDataAssetBuildReceiptErrorV1,
    ManagedRevision3ReviewedDataAssetBuildReceiptV1,
    MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_FORMAT_V1,
    MAX_MANAGED_REVISION3_REVIEWED_DATAASSET_BUILD_RECEIPT_BYTES_V1,
};
pub use dataasset_stage::{
    verify_reviewed_fixed_leaf_stage_v1, DataAssetStageArtifactAuthorityV1,
    DataAssetStageBuildStatusV1, DataAssetStageConflictV1, DataAssetStageManifestErrorV1,
    DataAssetStagePublicationStatusV1, DataAssetStageRuntimeStatusV1,
    PreparedRevision3DataAssetStageRemovalV1, PreparedRevision3DataAssetStageV1,
    Revision3DataAssetStageManifestV1, Revision3DataAssetStageViewV1,
    Revision3DataAssetStagingErrorV1, VerifiedCurrentReviewedDataAssetStageSourceV1,
    VerifiedReviewedFixedLeafStageV1, DATAASSET_FIXED_LEAF_COMPONENT_MEDIA_TYPE_V1,
    DATAASSET_FIXED_LEAF_STAGE_MANIFEST_MEDIA_TYPE_V1, MAX_DATAASSET_FIXED_LEAF_STAGES_V1,
    MAX_DATAASSET_FIXED_LEAF_STAGE_MANIFEST_BYTES_V1, MAX_DATAASSET_STAGE_HISTORICAL_BASES_V1,
    MAX_DATAASSET_STAGE_HISTORICAL_BASIS_BYTES_V1, MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_BYTES_V1,
    MAX_DATAASSET_STAGE_HISTORICAL_VERIFY_OBJECTS_V1, MAX_DATAASSET_STAGE_MANIFEST_BATCH_BYTES_V1,
};
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
    QuestTransitionConditionAtomV1, QuestTransitionConditionGroupV1, QuestTransitionEdgeV1,
    QuestTransitionEffectKindV1, QuestTransitionEffectV1, QuestTransitionNodeV1,
    QuestTransitionPlanV1, QuestTransitionPredicateV1, QuestTransitionStateTestV1,
    QuestTransitionV1, SchemaRevisionV3, ScriptModule as Revision3ScriptModule,
    TypedRef as Revision3TypedRef, MAX_QUEST_COLLISION_ARTIFACT_BYTES,
    MAX_QUEST_TRANSITION_EFFECTS_V1, MAX_QUEST_TRANSITION_PREDICATE_ATOMS_V1,
    MAX_QUEST_TRANSITION_PREDICATE_GROUPS_V1, MAX_REVISION3_ASSETS, MAX_REVISION3_ENTITIES,
    MAX_REVISION3_ENTITY_JSON_BYTES, MAX_REVISION3_REFERENCED_ASSET_BYTES,
    MAX_REVISION3_SNAPSHOT_BYTES, QUEST_COLLISION_ARTIFACT_FORMAT,
    QUEST_COLLISION_ARTIFACT_MEDIA_TYPE, QUEST_COLLISION_ARTIFACT_MEDIA_TYPE_V2,
    QUEST_COLLISION_ARTIFACT_SCHEMA_REVISION, QUEST_COLLISION_CATALOG_LAYER,
    QUEST_COLLISION_CATALOG_LAYER_V2, REVISION3_MULTI_OBJECTIVE_QUEST_GENERATOR_VERSION,
    REVISION3_QUEST_GENERATOR_ID, REVISION3_QUEST_GENERATOR_VERSION,
    REVISION3_SEMANTIC_QUEST_GENERATOR_VERSION,
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
    validate_draft_quest_objective_titles, validate_draft_quest_transition_plan_v1,
    CatalogQualifiedParentQuest, CatalogQualifiedQuestGiver,
    DraftQuestAdditionalObjectiveTechnicalNames, DraftQuestAuthoringStatus,
    DraftQuestCapabilityStatus, DraftQuestCatalogLayerAnchor, DraftQuestCollisionCatalog,
    DraftQuestCollisionKind, DraftQuestDiscoveryStatus, DraftQuestField, DraftQuestFixedShape,
    DraftQuestGeneratedSource, DraftQuestMultiObjectiveGeneratedSource,
    DraftQuestMultiObjectiveTechnicalNames, DraftQuestSemanticGeneratedSource,
    DraftQuestSemanticObjectiveTechnicalNames, DraftQuestSemanticTechnicalNames,
    DraftQuestSkeletonError, DraftQuestSkeletonInput, DraftQuestSkeletonInputV2,
    DraftQuestSkeletonInputV3, DraftQuestSkeletonV1, DraftQuestSkeletonV2, DraftQuestSkeletonV3,
    DraftQuestTechnicalNames, DraftQuestTransitionStatus, DRAFT_QUEST_GENERATOR_ID,
    DRAFT_QUEST_GENERATOR_VERSION, DRAFT_QUEST_MULTI_OBJECTIVE_GENERATOR_VERSION,
    DRAFT_QUEST_SEMANTIC_GENERATOR_VERSION, MAX_DRAFT_QUEST_CATALOG_LAYER_BYTES,
    MAX_DRAFT_QUEST_DESCRIPTION_BYTES, MAX_DRAFT_QUEST_OBJECTIVES,
    MAX_DRAFT_QUEST_OBJECTIVE_TITLES_BYTES, MAX_DRAFT_QUEST_OBJECTIVE_TITLE_BYTES,
    MAX_DRAFT_QUEST_TITLE_BYTES,
};
pub use revision3_content_index::{
    build_revision3_content_index_v1, Revision3ContentAssetClassV1,
    Revision3ContentAssetReferenceResolutionV1, Revision3ContentAssetReferenceRoleV1,
    Revision3ContentAssetReferenceV1, Revision3ContentAssetV1, Revision3ContentEntitySummaryV1,
    Revision3ContentEntityV1, Revision3ContentIndexErrorV1, Revision3ContentIndexJsonErrorV1,
    Revision3ContentIndexV1, Revision3ContentOriginV1, Revision3ContentReferenceResolutionV1,
    Revision3ContentReferenceRoleV1, Revision3ContentReferenceTargetV1,
    Revision3ContentReferenceV1, Revision3VoiceTargetResolutionV1,
    MAX_REVISION3_CONTENT_INDEX_JSON_BYTES_V1, MAX_REVISION3_CONTENT_REFERENCES_V1,
    REVISION3_CONTENT_INDEX_SCHEMA_V1,
};
pub use revision3_quest::{
    project_revision3_quest_free_basis_to_revision2, regenerate_revision3_quest_module_v2,
    revision3_quest_input_fingerprint_v2, Revision3QuestFreeBasisError,
    Revision3QuestGenerationError,
};
pub use revision3_quest_source_v2::{
    PreparedRevision3QuestCollisionInspectionSourceV2, PreparedRevision3QuestCollisionSourceV2,
    Revision3NonQuestCollisionBasisV2, Revision3PriorQuestEvidenceV2,
    Revision3QuestCollisionSourceErrorV2, MAX_REVISION3_COLLISION_IDENTITIES_V2,
    MAX_REVISION3_COLLISION_IDENTITY_BYTES_V2, MAX_REVISION3_COLLISION_IDENTITY_VALUE_BYTES_V2,
    MAX_REVISION3_PRIOR_QUESTS_V2,
};
pub use revision3_voice_build::{
    plan_revision3_voice_build_v1, Revision3VoiceBuildBlockReasonV1, Revision3VoiceBuildBlockedV1,
    Revision3VoiceBuildBlockerV1, Revision3VoiceBuildEditV1, Revision3VoiceBuildPlanErrorV1,
    Revision3VoiceBuildPlanEvaluationV1, Revision3VoiceBuildPlanV1,
    MAX_REVISION3_VOICE_BUILD_LINE_LABEL_BYTES_V1,
    MAX_REVISION3_VOICE_BUILD_SELECTED_PAYLOAD_BYTES_V1, MAX_REVISION3_VOICE_BUILD_SLOTS_V1,
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
pub use story_transaction_revision3_dialog::{
    apply_revision3_dialog_line_insert_transaction_v1, Revision3DialogBuildStatusV1,
    Revision3DialogEmptyVoiceSlotIntentV1, Revision3DialogEntityRoleV1,
    Revision3DialogLineInsertConflictV1, Revision3DialogLineInsertErrorV1,
    Revision3DialogLineInsertEvaluationV1, Revision3DialogLineInsertOutcomeV1,
    Revision3DialogLineInsertRejectionV1, Revision3DialogLineInsertRequestJsonErrorV1,
    Revision3DialogLineInsertRequestV1, Revision3DialogLocalizationActionV1,
    Revision3DialogLocalizationIntentV1, Revision3DialogPublicationStatusV1,
    Revision3DialogRuntimeStatusV1, Revision3DialogTopicAuthorityV1,
    MAX_REVISION3_DIALOG_AUTHORED_IDENTITY_BYTES_V1, MAX_REVISION3_DIALOG_DISPLAY_NAME_BYTES_V1,
    MAX_REVISION3_DIALOG_LINE_REQUEST_JSON_BYTES_V1, MAX_REVISION3_DIALOG_LOCALIZATION_TEXTS_V1,
    MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_BYTES_V1,
    MAX_REVISION3_DIALOG_LOCALIZATION_TEXT_TOTAL_BYTES_V1,
    MAX_REVISION3_DIALOG_SPEAKER_HINT_BYTES_V1,
};
pub use story_transaction_revision3_dialog_localization_edit::{
    apply_revision3_dialog_localization_edit_transaction_v1,
    Revision3DialogLocalizationEditBuildStatusV1, Revision3DialogLocalizationEditConflictV1,
    Revision3DialogLocalizationEditErrorV1, Revision3DialogLocalizationEditEvaluationV1,
    Revision3DialogLocalizationEditOutcomeV1, Revision3DialogLocalizationEditPublicationStatusV1,
    Revision3DialogLocalizationEditRejectionV1, Revision3DialogLocalizationEditRequestJsonErrorV1,
    Revision3DialogLocalizationEditRequestV1, Revision3DialogLocalizationEditRuntimeStatusV1,
    Revision3DialogLocalizationEditTopicAuthorityV1,
    MAX_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_JSON_BYTES_V1,
};
pub use story_transaction_revision3_npc::{
    apply_revision3_npc_draft_transaction_v1, Revision3NpcCatalogAuthorityV1,
    Revision3NpcCatalogSelectionV1, Revision3NpcCollisionAuthorityV1,
    Revision3NpcCollisionInventoryV1, Revision3NpcDraftBuildStatusV1,
    Revision3NpcDraftInsertConflictV1, Revision3NpcDraftInsertErrorV1,
    Revision3NpcDraftInsertEvaluationV1, Revision3NpcDraftInsertOutcomeV1,
    Revision3NpcDraftInsertRejectionV1, Revision3NpcDraftInsertRequestJsonErrorV1,
    Revision3NpcDraftInsertRequestV1, Revision3NpcDraftIntentV1,
    Revision3NpcDraftPublicationStatusV1, Revision3NpcDraftRuntimeStatusV1,
    Revision3NpcEntityRoleV1, Revision3NpcSourceInspectionStatusV1,
    Revision3NpcStoryIdentityKindV1, MAX_REVISION3_NPC_CATALOG_ID_BYTES_V1,
    MAX_REVISION3_NPC_DRAFT_DISPLAY_NAME_BYTES_V1, MAX_REVISION3_NPC_DRAFT_REQUEST_JSON_BYTES_V1,
    REVISION3_NPC_EXACT_COLLISION_LAYER_V1,
};
pub use story_transaction_revision3_quest_outline::{
    apply_revision3_quest_outline_edit_transaction_v1, Revision3QuestOutlineEditBuildStatusV1,
    Revision3QuestOutlineEditConflictV1, Revision3QuestOutlineEditErrorV1,
    Revision3QuestOutlineEditEvaluationV1, Revision3QuestOutlineEditOutcomeV1,
    Revision3QuestOutlineEditRejectionV1, Revision3QuestOutlineEditRequestJsonErrorV1,
    Revision3QuestOutlineEditRequestV1, Revision3QuestOutlineEditRuntimeStatusV1,
    MAX_REVISION3_QUEST_OUTLINE_EDIT_DISPLAY_NAME_BYTES_V1,
    MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V1,
};
pub use story_transaction_revision3_quest_outline_v2::{
    apply_revision3_quest_outline_edit_transaction_v2, Revision3QuestOutlineEditBuildStatusV2,
    Revision3QuestOutlineEditConflictV2, Revision3QuestOutlineEditErrorV2,
    Revision3QuestOutlineEditEvaluationV2, Revision3QuestOutlineEditOutcomeV2,
    Revision3QuestOutlineEditPublicationStatusV2, Revision3QuestOutlineEditRejectionV2,
    Revision3QuestOutlineEditRequestJsonErrorV2, Revision3QuestOutlineEditRequestV2,
    Revision3QuestOutlineEditRuntimeStatusV2, Revision3QuestOutlineObjectiveEditV2,
    MAX_REVISION3_QUEST_OUTLINE_EDIT_DISPLAY_NAME_BYTES_V2,
    MAX_REVISION3_QUEST_OUTLINE_EDIT_REQUEST_JSON_BYTES_V2,
};
pub use story_transaction_revision3_quest_transitions::{
    apply_revision3_quest_transition_plan_transaction_v1, revision3_quest_transition_plan_basis_v1,
    revision3_quest_transition_plan_seal_v1, Revision3QuestTransitionPlanBasisErrorV1,
    Revision3QuestTransitionPlanBasisV1, Revision3QuestTransitionPlanEditBuildStatusV1,
    Revision3QuestTransitionPlanEditConflictV1, Revision3QuestTransitionPlanEditErrorV1,
    Revision3QuestTransitionPlanEditEvaluationV1, Revision3QuestTransitionPlanEditOutcomeV1,
    Revision3QuestTransitionPlanEditPublicationStatusV1,
    Revision3QuestTransitionPlanEditRejectionV1,
    Revision3QuestTransitionPlanEditRequestJsonErrorV1, Revision3QuestTransitionPlanEditRequestV1,
    Revision3QuestTransitionPlanEditRuntimeStatusV1, Revision3QuestTransitionPlanSealErrorV1,
    MAX_REVISION3_QUEST_TRANSITION_PLAN_EDIT_REQUEST_JSON_BYTES_V1,
    MAX_REVISION3_QUEST_TRANSITION_PLAN_JSON_BYTES_V1,
};
pub use story_transaction_revision3_voice::{
    apply_revision3_voice_take_transaction_v1, preflight_revision3_voice_take_transaction_v1,
    Revision3VoiceBuildStatusV1, Revision3VoiceEntityRoleV1, Revision3VoicePublicationStatusV1,
    Revision3VoiceRuntimeStatusV1, Revision3VoiceTakePreflightEvaluationV1,
    Revision3VoiceTakeStageConflictV1, Revision3VoiceTakeStageErrorV1,
    Revision3VoiceTakeStageEvaluationV1, Revision3VoiceTakeStageOutcomeV1,
    Revision3VoiceTakeStageRejectionV1, Revision3VoiceTakeStageRequestJsonErrorV1,
    Revision3VoiceTakeStageRequestV1, Revision3VoiceTargetAuthorityV1,
    MAX_REVISION3_VOICE_DISPLAY_NAME_BYTES_V1, MAX_REVISION3_VOICE_LOGICAL_NAME_BYTES_V1,
    MAX_REVISION3_VOICE_REQUEST_JSON_BYTES_V1, MAX_REVISION3_VOICE_SLOT_CANDIDATES_V1,
    MAX_REVISION3_VOICE_TEXT_BYTES_V1, REVISION3_VOICE_SLOT_GENERATOR_ID_V1,
    REVISION3_VOICE_SLOT_GENERATOR_VERSION_V1, REVISION3_VOICE_TAKE_IMPORTER_ID_V1,
};
pub use story_transaction_revision3_voice_selection::{
    apply_revision3_voice_take_selection_transaction_v1, Revision3VoiceTakeSelectionBuildStatusV1,
    Revision3VoiceTakeSelectionConflictV1, Revision3VoiceTakeSelectionErrorV1,
    Revision3VoiceTakeSelectionEvaluationV1, Revision3VoiceTakeSelectionOutcomeV1,
    Revision3VoiceTakeSelectionRejectionV1, Revision3VoiceTakeSelectionRequestJsonErrorV1,
    Revision3VoiceTakeSelectionRequestV1, Revision3VoiceTakeSelectionRuntimeStatusV1,
    MAX_REVISION3_VOICE_TAKE_SELECTION_REQUEST_JSON_BYTES_V1,
};
pub use story_transaction_revision3_voice_target::{
    apply_revision3_voice_target_resolution_transaction_v1,
    validate_revision3_voice_loc_id_basename_stem_v1, Revision3VoiceLocIdBasenameStemErrorV1,
    Revision3VoiceTargetResolutionConflictV1, Revision3VoiceTargetResolutionErrorV1,
    Revision3VoiceTargetResolutionEvaluationV1, Revision3VoiceTargetResolutionOutcomeV1,
    Revision3VoiceTargetResolutionRejectionV1, Revision3VoiceTargetResolutionRequestJsonErrorV1,
    Revision3VoiceTargetResolutionRequestV1, Revision3VoiceTargetResolutionStateV1,
    MAX_REVISION3_VOICE_TARGET_ARCHIVE_BYTES_V1,
    MAX_REVISION3_VOICE_TARGET_ARCHIVE_CONTENT_BYTES_V1,
    MAX_REVISION3_VOICE_TARGET_LOC_ID_BYTES_V1, MAX_REVISION3_VOICE_TARGET_MATCHES_V1,
    MAX_REVISION3_VOICE_TARGET_MEMBER_BYTES_V1,
    MAX_REVISION3_VOICE_TARGET_MEMBER_UNCOMPRESSED_BYTES_V1,
    MAX_REVISION3_VOICE_TARGET_REQUEST_JSON_BYTES_V1,
};
pub use validate::{Diagnostic, DiagnosticCode, DiagnosticSeverity, ValidationProfile};
pub use working_store::{
    AssetVerification, CheckpointPreparation, ImportedOgg, ImportedQuestCollisionArtifactV1,
    ImportedQuestCollisionArtifactV2, OggImportError, OggImportFailureContext, OpenedCheckpoint,
    OpenedDocumentCheckpoint, OpenedRevision3Checkpoint, PreparedOggImport,
    Revision3CheckpointPreparation, Revision3SnapshotManifest, WorkingHead, WorkingProjectStore,
    WorkingStoreError, WorkingStoreFormat, WorkingStoreLimits,
};
