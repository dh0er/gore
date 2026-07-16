import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import '../core/providers.dart';
import 'managed_project_session.dart';
import 'project_controller.dart';
import 'revision3_content_index.dart';
import 'revision3_dataasset_authoring.dart';
import 'revision3_dialog_localization_authoring.dart';
import 'revision3_dialog_line_authoring.dart';
import 'revision3_dialog_voice_slot_removal_authoring.dart';
import 'revision3_npc_authoring.dart';
import 'revision3_npc_profile_edit_authoring.dart';
import 'revision3_quest_authoring.dart';
import 'revision3_quest_context_authoring.dart';
import 'revision3_quest_outline_authoring.dart';
import 'revision3_quest_transitions_authoring.dart';
import 'revision3_project_bootstrap.dart';
import 'revision3_project_history.dart';
import 'revision3_voice_authoring.dart';
import 'revision3_voice_take_removal_authoring.dart';
import 'revision3_voice_take_selection_authoring.dart';
import 'revision3_voice_take_status_authoring.dart';

enum CurrentProjectKind { none, legacyFormat1, managedRevision3 }

sealed class CurrentProjectState {
  const CurrentProjectState();

  CurrentProjectKind get kind;
}

final class NoCurrentProjectState extends CurrentProjectState {
  const NoCurrentProjectState();

  @override
  CurrentProjectKind get kind => CurrentProjectKind.none;
}

/// Snapshot of the compatibility `.goremod` session.
///
/// The legacy provider graph remains owned by [ProjectSessionController]; this
/// state deliberately does not reinterpret it as a managed authoring document.
final class LegacyCurrentProjectState extends CurrentProjectState {
  const LegacyCurrentProjectState({
    required this.path,
    required this.hasUnsavedChanges,
  });

  final String? path;
  final bool hasUnsavedChanges;

  @override
  CurrentProjectKind get kind => CurrentProjectKind.legacyFormat1;
}

/// Durable identity of the exact revision-3 checkpoint owned by the app.
///
/// No diagnostics, readiness, runtime, deployment, or publication authority is
/// inferred from this checkpoint-only state.
final class ManagedRevision3CurrentProjectState extends CurrentProjectState {
  const ManagedRevision3CurrentProjectState({
    required this.root,
    required this.projectId,
    required this.projectRevision,
    required this.head,
    required this.requiresReopen,
  });

  final Directory root;
  final String projectId;
  final int projectRevision;
  final AuthoringWorkingHead head;
  final bool requiresReopen;

  @override
  CurrentProjectKind get kind => CurrentProjectKind.managedRevision3;
}

class CurrentProjectCoordinatorException implements Exception {
  const CurrentProjectCoordinatorException(this.message);

  final String message;

  @override
  String toString() => 'CurrentProjectCoordinatorException: $message';
}

final class NoCurrentProjectException
    extends CurrentProjectCoordinatorException {
  const NoCurrentProjectException()
    : super('there is no current project to operate on');
}

final class CurrentProjectOperationUnsupportedException
    extends CurrentProjectCoordinatorException {
  const CurrentProjectOperationUnsupportedException(super.message);
}

final class CurrentProjectCoordinatorClosedException
    extends CurrentProjectCoordinatorException {
  const CurrentProjectCoordinatorClosedException()
    : super('the current-project coordinator is shutting down or disposed');
}

final class Revision3RecoveryStaleCheckpointException
    extends CurrentProjectCoordinatorException {
  const Revision3RecoveryStaleCheckpointException()
    : super('the managed project changed before recovery could start');
}

final class Revision3RecoveryFailedException
    extends CurrentProjectCoordinatorException {
  const Revision3RecoveryFailedException()
    : super(
        'managed project recovery did not complete; reopen the project before editing',
      );
}

final class Revision3RecoveryNotSupportedException
    extends CurrentProjectCoordinatorException {
  const Revision3RecoveryNotSupportedException()
    : super('the current project does not support in-place recovery');
}

final class Revision3RecoveryNotRequiredException
    extends CurrentProjectCoordinatorException {
  const Revision3RecoveryNotRequiredException()
    : super('the current managed project does not require recovery');
}

final class Revision3ProjectHistoryStaleCheckpointException
    extends CurrentProjectCoordinatorException {
  const Revision3ProjectHistoryStaleCheckpointException()
    : super('the managed project changed before the history operation started');
}

final class Revision3ProjectHistoryRequiresReopenException
    extends CurrentProjectCoordinatorException {
  const Revision3ProjectHistoryRequiresReopenException()
    : super(
        'the managed project must be recovered or reopened before history can change it',
      );
}

final class Revision3ProjectHistoryUnsupportedException
    extends CurrentProjectCoordinatorException {
  const Revision3ProjectHistoryUnsupportedException()
    : super(
        'this managed project session has no authenticated history capability',
      );
}

final class Revision3ProjectHistoryFailedException
    extends CurrentProjectCoordinatorException {
  const Revision3ProjectHistoryFailedException()
    : super(
        'the history result could not be bound to the current managed project',
      );
}

final class Revision3ProjectExportStaleCheckpointException
    extends CurrentProjectCoordinatorException {
  const Revision3ProjectExportStaleCheckpointException()
    : super('the managed project changed before export could start');
}

final class Revision3ProjectExportRequiresReopenException
    extends CurrentProjectCoordinatorException {
  const Revision3ProjectExportRequiresReopenException({
    this.publicationMayExist = false,
    this.code,
    this.cause,
  }) : super('the managed project must be recovered or reopened before export');

  final bool publicationMayExist;
  final String? code;
  final Object? cause;
}

final class Revision3ProjectExportUnsupportedException
    extends CurrentProjectCoordinatorException {
  const Revision3ProjectExportUnsupportedException()
    : super('this current project cannot export an exact managed project copy');
}

final class Revision3ProjectExportFailedException
    extends CurrentProjectCoordinatorException {
  const Revision3ProjectExportFailedException({
    this.cause,
    this.code,
    this.publicationMayExist = true,
    this.retryWithNewDestination = false,
  }) : assert(!retryWithNewDestination || !publicationMayExist),
       super('the exact managed project copy could not be exported safely');

  final Object? cause;
  final String? code;
  final bool publicationMayExist;
  final bool retryWithNewDestination;
}

const _revision3ProjectExportSafePrepublicationCodes = <String>{
  'AUTHORING_REVISION3_EXPORT_REQUEST_INVALID',
  'AUTHORING_REVISION3_EXPORT_INPUT_LIMIT',
  'AUTHORING_REVISION3_EXPORT_CLOSURE_LIMIT',
  'AUTHORING_REVISION3_EXPORT_OUTPUT_EXISTS',
  'AUTHORING_REVISION3_EXPORT_OUTPUT_INVALID',
  'AUTHORING_REVISION3_EXPORT_ARCHIVE_FAILED',
  'AUTHORING_REVISION3_EXPORT_VERIFY_FAILED',
  'AUTHORING_REVISION3_EXPORT_CLEANUP_FAILED',
  'AUTHORING_REVISION3_EXPORT_PUBLICATION_FAILED',
};

const _revision3ProjectExportDestinationRetryCodes = <String>{
  'AUTHORING_REVISION3_EXPORT_OUTPUT_EXISTS',
  'AUTHORING_REVISION3_EXPORT_OUTPUT_INVALID',
};

/// Exact managed checkpoint returned after removing one Story Draft and its
/// uniquely-owned generated ScriptModule. This is project-storage evidence
/// only and grants no build, runtime, artifact, deployment, or native
/// publication authority.
final class Revision3StoryDraftRemovalPublication {
  const Revision3StoryDraftRemovalPublication({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.removedDraftId,
    required this.removedDraftKind,
    required this.removedDraftRevision,
    required this.removedScriptModuleId,
    required this.removedScriptModuleRevision,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final String removedDraftId;
  final AuthoringStoryDraftKind removedDraftKind;
  final int removedDraftRevision;
  final String removedScriptModuleId;
  final int removedScriptModuleRevision;
}

final class Revision3StoryDraftRemovalStaleCheckpointException
    extends CurrentProjectCoordinatorException {
  const Revision3StoryDraftRemovalStaleCheckpointException()
    : super(
        'the managed project changed before Story Draft removal could start',
      );
}

final class Revision3StoryDraftRemovalRequiresReopenException
    extends CurrentProjectCoordinatorException {
  const Revision3StoryDraftRemovalRequiresReopenException({this.cause})
    : super(
        'the managed project must be recovered or reopened before removing a Story Draft',
      );

  final Object? cause;
}

final class Revision3StoryDraftRemovalUnsupportedException
    extends CurrentProjectCoordinatorException {
  const Revision3StoryDraftRemovalUnsupportedException()
    : super('this current project cannot remove Story Drafts safely');
}

/// A closed prepublication conflict that the author may correct and retry
/// after refreshing the exact current content index.
final class Revision3StoryDraftRemovalRejectedException
    extends CurrentProjectCoordinatorException {
  const Revision3StoryDraftRemovalRejectedException({
    required this.code,
    required this.cause,
  }) : super('the Story Draft removal request was rejected before publication');

  final String? code;
  final Object cause;
}

const _revision3StoryDraftRemovalCorrectableCodes = <String>{
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_INPUT_LIMIT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_TARGET_CONFLICT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_DRAFT_CONFLICT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_MODULE_CONFLICT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_OWNERSHIP_CONFLICT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_DRAFT_REFERENCED',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_MODULE_REFERENCED',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REFERENCE_LIMIT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REVISION_LIMIT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_LIMIT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_INVALID',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_LIMIT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_REJECTED',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_SIGNED_WIRE_LIMIT',
};

const _revision3VoiceTakeRemovalCorrectableCodes = <String>{
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_INPUT_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_TARGET_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_LINE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_LOCALIZATION_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_LOC_ID_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SLOT_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_TAKE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SELECTION_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_BACKLINK_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REFERENCE_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REVISION_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_PROJECT_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_INVALID',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REQUEST_REJECTED',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SIGNED_WIRE_LIMIT',
};

const _revision3DialogVoiceSlotRemovalCorrectableCodes = <String>{
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_INPUT_LIMIT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_TARGET_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_LINE_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_LOCALIZATION_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_LOC_ID_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_SLOT_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_NOT_EMPTY',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_BACKLINK_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REFERENCE_LIMIT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REVISION_LIMIT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_PROJECT_LIMIT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_INVALID',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_LIMIT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REQUEST_REJECTED',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_SIGNED_WIRE_LIMIT',
};

const _revision3NpcProfileEditCorrectableCodes = <String>{
  'AUTHORING_REVISION3_NPC_PROFILE_CATALOG_FAILED',
  'AUTHORING_REVISION3_NPC_PROFILE_CATALOG_LIMIT',
  'AUTHORING_REVISION3_NPC_PROFILE_CATALOG_CONFLICT',
  'AUTHORING_REVISION3_NPC_PROFILE_CATALOG_SELECTION_INVALID',
  'AUTHORING_REVISION3_NPC_PROFILE_INPUT_CHANGED',
  'AUTHORING_REVISION3_NPC_PROFILE_INPUT_LIMIT',
  'AUTHORING_REVISION3_NPC_PROFILE_INPUT_MISSING',
  'AUTHORING_REVISION3_NPC_PROFILE_INPUT_UNAVAILABLE',
  'AUTHORING_REVISION3_NPC_PROFILE_INPUT_UNSAFE',
  'AUTHORING_REVISION3_NPC_PROFILE_LIMIT',
  'AUTHORING_REVISION3_NPC_PROFILE_MODULE_CONFLICT',
  'AUTHORING_REVISION3_NPC_PROFILE_NO_CHANGES',
  'AUTHORING_REVISION3_NPC_PROFILE_NPC_CONFLICT',
  'AUTHORING_REVISION3_NPC_PROFILE_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_NPC_PROFILE_RECOVERY_REQUIRED',
  'AUTHORING_REVISION3_NPC_PROFILE_REQUEST_INVALID',
  'AUTHORING_REVISION3_NPC_PROFILE_REQUEST_LIMIT',
  'AUTHORING_REVISION3_NPC_PROFILE_UNSUPPORTED_GENERATION',
};

final class ManagedRevision3ProjectCreationException
    extends CurrentProjectCoordinatorException {
  const ManagedRevision3ProjectCreationException(super.message);
}

final class Revision3QuestSourceInspectionRequiresReopenException
    implements Exception {
  const Revision3QuestSourceInspectionRequiresReopenException();
}

final class Revision3QuestSourceInspectionStaleCheckpointException
    implements Exception {
  const Revision3QuestSourceInspectionStaleCheckpointException();
}

final class Revision3NpcSourceInspectionRequiresReopenException
    implements Exception {
  const Revision3NpcSourceInspectionRequiresReopenException();
}

final class Revision3NpcSourceInspectionStaleCheckpointException
    implements Exception {
  const Revision3NpcSourceInspectionStaleCheckpointException();
}

final class Revision3DialogLocalizationReadRequiresReopenException
    implements Exception {
  const Revision3DialogLocalizationReadRequiresReopenException();
}

final class Revision3DialogLocalizationReadStaleCheckpointException
    implements Exception {
  const Revision3DialogLocalizationReadStaleCheckpointException();
}

bool _revision3DialogLocalizationReadErrorIsStale(String code) => const {
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_IDENTITY_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_NOT_FOUND',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_REVISION_CONFLICT',
}.contains(code);

bool _revision3DialogLocalizationEditErrorIsStale(String code) => const {
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_IDENTITY_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_NOT_FOUND',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_ORIGIN_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TARGET_CONFLICT',
}.contains(code);

final class Revision3ManagedCompilerCheckRequiresReopenException
    implements Exception {
  const Revision3ManagedCompilerCheckRequiresReopenException();
}

final class Revision3ManagedCompilerCheckStaleCheckpointException
    implements Exception {
  const Revision3ManagedCompilerCheckStaleCheckpointException();
}

final class Revision3DataAssetPackageIndexRequiresReopenException
    implements Exception {
  const Revision3DataAssetPackageIndexRequiresReopenException();
}

final class Revision3DataAssetPackageIndexStaleCheckpointException
    implements Exception {
  const Revision3DataAssetPackageIndexStaleCheckpointException();
}

final class Revision3InstalledDataAssetInspectionRequiresReopenException
    implements Exception {
  const Revision3InstalledDataAssetInspectionRequiresReopenException();
}

final class Revision3InstalledDataAssetInspectionStaleCheckpointException
    implements Exception {
  const Revision3InstalledDataAssetInspectionStaleCheckpointException();
}

final class Revision3InstalledDataAssetEditSourceEvidenceStaleException
    implements Exception {
  const Revision3InstalledDataAssetEditSourceEvidenceStaleException();
}

enum Revision3InstalledDataAssetEditRejectionReason {
  targetAlreadyStaged,
  preparationFailed,
}

final class Revision3InstalledDataAssetEditRejectedException
    implements Exception {
  const Revision3InstalledDataAssetEditRejectedException(this.reason);

  final Revision3InstalledDataAssetEditRejectionReason reason;
}

/// Diagnostic evidence that one terminal lease close failed.
///
/// The coordinator deliberately retains no reference to the lease itself and
/// never retries it: production leases memoize their one permitted close
/// attempt. The error and stack trace remain available for reporting.
final class CurrentProjectCleanupFailure {
  const CurrentProjectCleanupFailure({
    required this.projectKind,
    required this.error,
    required this.stackTrace,
  });

  final CurrentProjectKind projectKind;
  final Object error;
  final StackTrace stackTrace;
}

/// Minimal ownership seam around the existing format-1 compatibility session.
abstract interface class LegacyCurrentProjectLease {
  String? get currentPath;
  bool get hasUnsavedChanges;

  Future<void> saveCurrent();
  Future<void> saveToPath(String path);
  Future<void> openFromPath(String path);
  Future<void> newProject();
  Future<void> close();
}

/// Minimal ownership seam around one fully-opened managed revision-3 session.
abstract interface class ManagedRevision3CurrentProjectLease {
  Directory get root;
  String get projectId;
  int get projectRevision;
  String get canonicalProjectJson;
  AuthoringWorkingHead get head;
  bool get requiresReopen;

  Future<Revision3ContentIndex> readContentIndex();
  Future<AuthoringRevision3QuestSourceInspectionResult> inspectQuestSourceV1({
    required String gameRoot,
    required String questId,
  });
  Future<AuthoringRevision3NpcSourceInspectionResult> inspectNpcSourceV1({
    required String npcId,
  });
  Future<ManagedRevision3CompilerCheckReceipt> checkCompilerV1({
    required AuthoringRevision3ManagedCompilerEntityKind entityKind,
    required String gameRoot,
    required String entityId,
    required int expectedEntityRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  });
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readDataAssetPackageIndexV1({required String gameRoot});
  Future<AuthoringRevision3InstalledDataAssetInspectionResult>
  inspectInstalledDataAssetV1({
    required String gameRoot,
    required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
  });
  Future<Revision3QuestDraftPublication> prepareAndPublishQuestDraftV3({
    required String gameRoot,
    required Revision3QuestDraftAuthoringInput input,
  });
  Future<Revision3QuestOutlineEditPublication>
  prepareAndPublishQuestOutlineEditV1({
    required Revision3QuestOutlineEditInput input,
  });
  Future<AuthoringRevision3QuestTransitionsSeed> readQuestTransitionsSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  });
  Future<Revision3QuestTransitionsEditPublication>
  prepareAndPublishQuestTransitionsEditV1({
    required Revision3QuestTransitionsEditTechnicalPlan plan,
  });
  Future<AuthoringRevision3QuestContextSeed> readQuestContextSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required String expectedParentRuntimeClass,
    required String expectedGiverRuntimeUniqueName,
  });
  Future<Revision3QuestContextEditPublication>
  prepareAndPublishQuestContextEditV1({
    required String gameRoot,
    required Revision3QuestContextEditTechnicalPlan plan,
  });
  Future<Revision3NpcDraftPublication> prepareAndPublishNpcDraftV1({
    required String gameRoot,
    required Revision3NpcDraftAuthoringInput input,
  });
  Future<Revision3DialogLineEntryPublication> prepareAndPublishDialogLineV1({
    required Revision3DialogLineEntryTechnicalPlan plan,
  });
  Future<Revision3VoiceTakePublication> prepareAndPublishVoiceTakeV1({
    required String gameRoot,
    required Revision3VoiceTakeTechnicalPlan plan,
  });
  Future<Revision3VoiceTakeSelectionPublication>
  prepareAndPublishVoiceTakeSelectionV1({
    required Revision3VoiceTakeSelectionTechnicalPlan plan,
  });
  Future<Revision3VoiceTargetPublication> prepareAndPublishVoiceTargetV1({
    required String gameRoot,
    required Revision3VoiceTargetTechnicalPlan plan,
  });
  Future<AuthoringRevision3VoiceBuildResult> buildVoiceV1({
    required String gameRoot,
    required String output,
  });
  Future<List<AuthoringRevision3DataAssetStage>> listDataAssetStagesV1();
  Future<Revision3DataAssetStagePublication> prepareAndPublishDataAssetStageV1({
    required String patchReceiptPath,
  });
  Future<Revision3DataAssetStagePublication> prepareAndPublishDataAssetEditV1({
    required DataAssetSemanticEditIntent intent,
  });
  Future<Revision3DataAssetStagePublication>
  prepareAndPublishInstalledDataAssetEditV1({
    required String gameRoot,
    required DataAssetInstalledSemanticEditIntent intent,
  });
  Future<Revision3DataAssetStagePublication>
  prepareAndPublishReviewedInstalledDataAssetEditV1({
    required String gameRoot,
    required ReviewedInstalledDataAssetEditIntent intent,
  });
  Future<Revision3DataAssetStageRemovalPublication>
  prepareAndPublishRemoveDataAssetStageV1({required String targetPath});
  Future<void> verifyCurrentHead();
  Future<void> close();
}

/// Optional exact-current capability for reading one managed LocalizationEntry.
/// Keeping it separate avoids widening unrelated current-project test leases.
abstract interface class ManagedRevision3DialogLocalizationReadLease {
  Future<AuthoringRevision3DialogLocalizationReadResult>
  readDialogLocalizationV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  });
}

/// Optional exact-current capability for full authored-text editing. Keeping
/// it narrow avoids granting unrelated managed lease fakes mutation authority.
abstract interface class ManagedRevision3DialogLocalizationEditLease {
  Future<AuthoringRevision3DialogLocalizationEditSeed>
  readDialogLocalizationEditSeedV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  });

  Future<Revision3DialogLocalizationEditPublication>
  prepareAndPublishDialogLocalizationEditV1({
    required Revision3DialogLocalizationEditTechnicalPlan plan,
  });
}

abstract interface class ManagedRevision3NpcProfileEditLease {
  bool get supportsNpcProfileEdit;
  void markRequiresReopenAfterNpcProfileEditUncertainty();

  Future<AuthoringRevision3NpcProfileEditSeed> readNpcProfileEditSeedV1({
    required String npcId,
    required int expectedNpcRevision,
    required String expectedScriptModuleId,
    required int expectedScriptModuleRevision,
    required String expectedUniqueName,
    required String expectedModuleNamespace,
    required String expectedParentCharacterDefinition,
    required String expectedParentAiAgentConfig,
    required String expectedParentSpawnDefinition,
  });

  Future<Revision3NpcProfileEditPublication> prepareAndPublishNpcProfileEditV1({
    required String gameRoot,
    required Revision3NpcProfileEditTechnicalPlan plan,
  });
}

/// Optional exact-current capability for changing one retained VoiceTake's
/// author-managed review status. Keeping it separate avoids widening unrelated
/// current-project lease fakes with mutation authority.
abstract interface class ManagedRevision3VoiceTakeStatusLease {
  /// Permanently remove mutation authority from this lease after a publication
  /// returned a receipt that cannot be bound to the requested checkpoint.
  void markRequiresReopenAfterPublicationUncertainty();

  Future<Revision3VoiceTakeStatusPublication>
  prepareAndPublishVoiceTakeStatusV1({
    required Revision3VoiceTakeStatusTechnicalPlan plan,
  });
}

/// Optional exact-current capability for atomically detaching one VoiceTake
/// candidate. Capability discovery stays explicit because this operation can
/// remove an otherwise-unreferenced project entity.
abstract interface class ManagedRevision3VoiceTakeRemovalLease {
  bool get supportsVoiceTakeRemoval;

  void markRequiresReopenAfterVoiceTakeRemovalUncertainty();

  Future<Revision3VoiceTakeRemovalPublication>
  prepareAndPublishVoiceTakeRemovalV1({
    required Revision3VoiceTakeRemovalTechnicalPlan plan,
  });
}

/// Optional exact-current authority for removing one empty, unselected dialog
/// VoiceSlot and its line locale binding.
abstract interface class ManagedRevision3DialogVoiceSlotRemovalLease {
  bool get supportsDialogVoiceSlotRemoval;

  void markRequiresReopenAfterDialogVoiceSlotRemovalUncertainty();

  Future<Revision3DialogVoiceSlotRemovalPublication>
  prepareAndPublishDialogVoiceSlotRemovalV1({
    required Revision3DialogVoiceSlotRemovalTechnicalPlan plan,
  });
}

/// Optional authority for reconciling one managed lease after an uncertain
/// publication. Recovery remains separate from normal editing so unrelated
/// leases and fakes do not accidentally claim repair authority.
abstract interface class ManagedRevision3RecoveryLease {
  Future<ManagedRevision3RecoveryCheckpoint> recoverAfterUncertainPublication();

  /// Fail closed when a returned recovery checkpoint cannot be bound to the
  /// lease's complete post-recovery state.
  void markRequiresReopenAfterRecoveryUncertainty();
}

/// Optional authenticated-history authority. It remains separate from normal
/// editing so unrelated leases and fakes cannot accidentally offer restore.
abstract interface class ManagedRevision3ProjectHistoryLease {
  bool get supportsProjectHistory;

  Future<Revision3ProjectHistorySnapshot> readProjectHistoryV1();

  Future<ManagedRevision3ProjectHistoryRestoreCheckpoint>
  prepareAndPublishProjectHistoryRestoreV1({
    required Revision3ProjectHistorySnapshot expectedHistory,
    required Revision3ProjectHistoryEntry target,
  });

  void markRequiresReopenAfterHistoryUncertainty();
}

/// Optional immutable-build capability. It is separate from checkpoint editing
/// so unrelated lease fakes do not accidentally claim artifact authority.
abstract interface class ManagedRevision3ReviewedDataAssetBuildLease {
  bool get supportsReviewedDataAssetBuild;

  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  buildReviewedDataAssetV1({
    required String gameRoot,
    required String targetPath,
    required String packName,
    required String output,
  });
}

/// Optional exact-snapshot export authority. Keeping it separate ensures that
/// test leases and alternate managed sessions do not accidentally claim
/// portable-copy support merely by implementing ordinary project editing.
abstract interface class ManagedRevision3ProjectExportLease {
  bool get supportsExactSnapshotExport;

  /// Permanently remove mutation/export authority after a post-call result
  /// cannot be bound to the requested checkpoint or publication terminal.
  void markRequiresReopenAfterPublicationUncertainty();

  Future<AuthoringRevision3ExactSnapshotExportResult> exportExactSnapshotV1({
    required String output,
  });
}

/// Optional exact-current Story Draft deletion authority. The explicit
/// capability bit and fail-closed latch keep unrelated alternate leases and
/// test doubles from accidentally claiming destructive authoring support.
abstract interface class ManagedRevision3StoryDraftRemovalLease {
  bool get supportsStoryDraftRemoval;

  void markRequiresReopenAfterStoryDraftRemovalUncertainty();

  Future<Revision3StoryDraftRemovalPublication>
  prepareAndPublishRemoveStoryDraftV1({
    required String draftId,
    required AuthoringStoryDraftKind draftKind,
    required int expectedDraftRevision,
    required String scriptModuleId,
    required int expectedScriptModuleRevision,
  });
}

typedef ManagedRevision3CurrentProjectOpener =
    Future<ManagedRevision3CurrentProjectLease> Function(Directory root);

typedef ManagedRevision3ProjectSessionCreator =
    Future<ManagedRevision3CurrentProjectLease> Function({
      required Directory root,
      required String projectJson,
    });

typedef ManagedRevision3StoryGenerationLoader =
    Future<AuthoringStoryCatalogGeneration> Function(String gameRoot);

/// Friendly inputs for one brand-new, empty managed revision-3 project.
///
/// The creator owns generation discovery, canonical project construction, and
/// first-head publication. The coordinator receives only the fully opened
/// candidate lease and therefore cannot accidentally adopt a partial project.
final class ManagedRevision3ProjectCreateRequest {
  ManagedRevision3ProjectCreateRequest({
    required this.root,
    required this.gameRoot,
    required this.name,
    required this.version,
    required this.author,
    required List<String> authoringLocales,
  }) : authoringLocales = List<String>.unmodifiable(authoringLocales);

  final Directory root;
  final String gameRoot;
  final String name;
  final String version;
  final String author;
  final List<String> authoringLocales;
}

typedef ManagedRevision3CurrentProjectCreator =
    Future<ManagedRevision3CurrentProjectLease> Function(
      ManagedRevision3ProjectCreateRequest request,
    );

typedef LegacyCurrentProjectLeaseFactory = LegacyCurrentProjectLease Function();

/// Compatibility adapter kept intentionally narrow so the existing provider
/// graph and archive session do not have to know about managed projects.
final class ProjectSessionLegacyCurrentProjectLease
    implements LegacyCurrentProjectLease {
  ProjectSessionLegacyCurrentProjectLease(this._session);

  final ProjectSessionController _session;
  Future<void>? _closeFuture;
  bool _closed = false;

  @override
  String? get currentPath {
    _requireOpen();
    return _session.currentPath;
  }

  @override
  bool get hasUnsavedChanges {
    _requireOpen();
    return _session.hasUnsavedChanges;
  }

  @override
  Future<void> saveCurrent() async {
    _requireOpen();
    await _session.saveToCurrentPath();
  }

  @override
  Future<void> saveToPath(String path) async {
    _requireOpen();
    await _session.saveToPath(path);
  }

  @override
  Future<void> openFromPath(String path) async {
    _requireOpen();
    await _session.openFromPath(path);
  }

  @override
  Future<void> newProject() async {
    _requireOpen();
    await _session.newProject();
  }

  @override
  Future<void> close() {
    final existing = _closeFuture;
    if (existing != null) return existing;
    _closed = true;
    return _closeFuture = Future<void>.sync(_session.newProject);
  }

  void _requireOpen() {
    if (_closed) {
      throw StateError('legacy current-project lease is already closed');
    }
  }
}

final class _ManagedRevision3SessionLease
    implements
        ManagedRevision3CurrentProjectLease,
        ManagedRevision3DialogLocalizationReadLease,
        ManagedRevision3DialogLocalizationEditLease,
        ManagedRevision3NpcProfileEditLease,
        ManagedRevision3VoiceTakeStatusLease,
        ManagedRevision3VoiceTakeRemovalLease,
        ManagedRevision3DialogVoiceSlotRemovalLease,
        ManagedRevision3RecoveryLease,
        ManagedRevision3ProjectHistoryLease,
        ManagedRevision3ReviewedDataAssetBuildLease,
        ManagedRevision3ProjectExportLease,
        ManagedRevision3StoryDraftRemovalLease {
  const _ManagedRevision3SessionLease(this._session);

  final ManagedRevision3AuthoringProjectSession _session;

  @override
  AuthoringWorkingHead get head => _session.head;

  @override
  String get projectId => _session.projectId;

  @override
  int get projectRevision => _session.projectRevision;

  @override
  String get canonicalProjectJson => _session.projectJson;

  @override
  bool get requiresReopen => _session.requiresReopen;

  @override
  void markRequiresReopenAfterPublicationUncertainty() =>
      _session.markRequiresReopenAfterPublicationUncertainty();

  @override
  void markRequiresReopenAfterRecoveryUncertainty() =>
      _session.markRequiresReopenAfterPublicationUncertainty();

  @override
  Future<ManagedRevision3RecoveryCheckpoint>
  recoverAfterUncertainPublication() =>
      _session.recoverAfterUncertainPublication();

  @override
  bool get supportsProjectHistory => _session.supportsProjectHistory;

  @override
  Future<Revision3ProjectHistorySnapshot> readProjectHistoryV1() =>
      _session.readProjectHistoryV1();

  @override
  Future<ManagedRevision3ProjectHistoryRestoreCheckpoint>
  prepareAndPublishProjectHistoryRestoreV1({
    required Revision3ProjectHistorySnapshot expectedHistory,
    required Revision3ProjectHistoryEntry target,
  }) => _session.prepareAndPublishProjectHistoryRestoreV1(
    expectedHistory: expectedHistory,
    target: target,
  );

  @override
  void markRequiresReopenAfterHistoryUncertainty() =>
      _session.markRequiresReopenAfterPublicationUncertainty();

  @override
  bool get supportsReviewedDataAssetBuild =>
      _session.supportsReviewedDataAssetBuild;

  @override
  bool get supportsExactSnapshotExport => _session.supportsExactSnapshotExport;

  @override
  bool get supportsStoryDraftRemoval => _session.supportsStoryDraftRemoval;

  @override
  bool get supportsVoiceTakeRemoval => _session.supportsVoiceTakeRemoval;

  @override
  bool get supportsDialogVoiceSlotRemoval =>
      _session.supportsDialogVoiceSlotRemoval;

  @override
  bool get supportsNpcProfileEdit => _session.supportsNpcProfileEdit;

  @override
  void markRequiresReopenAfterStoryDraftRemovalUncertainty() =>
      _session.markRequiresReopenAfterPublicationUncertainty();

  @override
  void markRequiresReopenAfterVoiceTakeRemovalUncertainty() =>
      _session.markRequiresReopenAfterPublicationUncertainty();

  @override
  void markRequiresReopenAfterDialogVoiceSlotRemovalUncertainty() =>
      _session.markRequiresReopenAfterPublicationUncertainty();

  @override
  void markRequiresReopenAfterNpcProfileEditUncertainty() =>
      _session.markRequiresReopenAfterPublicationUncertainty();

  @override
  Future<AuthoringRevision3ExactSnapshotExportResult> exportExactSnapshotV1({
    required String output,
  }) => _session.exportExactSnapshotV1(output: output);

  @override
  Directory get root => _session.root;

  @override
  Future<void> verifyCurrentHead() => _session.verifyCurrentHead();

  @override
  Future<Revision3ContentIndex> readContentIndex() =>
      _session.readContentIndex();

  @override
  Future<AuthoringRevision3QuestSourceInspectionResult> inspectQuestSourceV1({
    required String gameRoot,
    required String questId,
  }) => _session.inspectQuestSourceV1(gameRoot: gameRoot, questId: questId);

  @override
  Future<AuthoringRevision3NpcSourceInspectionResult> inspectNpcSourceV1({
    required String npcId,
  }) => _session.inspectNpcSourceV1(npcId: npcId);

  @override
  Future<AuthoringRevision3NpcProfileEditSeed> readNpcProfileEditSeedV1({
    required String npcId,
    required int expectedNpcRevision,
    required String expectedScriptModuleId,
    required int expectedScriptModuleRevision,
    required String expectedUniqueName,
    required String expectedModuleNamespace,
    required String expectedParentCharacterDefinition,
    required String expectedParentAiAgentConfig,
    required String expectedParentSpawnDefinition,
  }) => _session.readNpcProfileEditSeedV1(
    npcId: npcId,
    expectedNpcRevision: expectedNpcRevision,
    expectedScriptModuleId: expectedScriptModuleId,
    expectedScriptModuleRevision: expectedScriptModuleRevision,
    expectedUniqueName: expectedUniqueName,
    expectedModuleNamespace: expectedModuleNamespace,
    expectedParentCharacterDefinition: expectedParentCharacterDefinition,
    expectedParentAiAgentConfig: expectedParentAiAgentConfig,
    expectedParentSpawnDefinition: expectedParentSpawnDefinition,
  );

  @override
  Future<AuthoringRevision3DialogLocalizationReadResult>
  readDialogLocalizationV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) => _session.readDialogLocalizationV1(
    localizationId: localizationId,
    expectedLocalizationRevision: expectedLocalizationRevision,
    expectedLocId: expectedLocId,
  );

  @override
  Future<AuthoringRevision3DialogLocalizationEditSeed>
  readDialogLocalizationEditSeedV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) => _session.readDialogLocalizationEditSeedV1(
    localizationId: localizationId,
    expectedLocalizationRevision: expectedLocalizationRevision,
    expectedLocId: expectedLocId,
  );

  @override
  Future<ManagedRevision3CompilerCheckReceipt> checkCompilerV1({
    required AuthoringRevision3ManagedCompilerEntityKind entityKind,
    required String gameRoot,
    required String entityId,
    required int expectedEntityRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) => _session.checkCompilerV1(
    entityKind: entityKind,
    gameRoot: gameRoot,
    entityId: entityId,
    expectedEntityRevision: expectedEntityRevision,
    expectedModuleId: expectedModuleId,
    expectedModuleRevision: expectedModuleRevision,
  );

  @override
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readDataAssetPackageIndexV1({required String gameRoot}) =>
      _session.readDataAssetPackageIndexV1(gameRoot: gameRoot);

  @override
  Future<AuthoringRevision3InstalledDataAssetInspectionResult>
  inspectInstalledDataAssetV1({
    required String gameRoot,
    required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
  }) => _session.inspectInstalledDataAssetV1(
    gameRoot: gameRoot,
    expectedSnapshot: expectedSnapshot,
    candidate: candidate,
  );

  @override
  Future<Revision3QuestDraftPublication> prepareAndPublishQuestDraftV3({
    required String gameRoot,
    required Revision3QuestDraftAuthoringInput input,
  }) async {
    final plan = Revision3QuestDraftTechnicalPlan.forCheckpoint(
      projectId: _session.projectId,
      projectRevision: _session.projectRevision,
      input: input,
    );
    final checkpoint = await _session.prepareAndPublishQuestDraftV3(
      gameRoot: gameRoot,
      questId: plan.questId,
      scriptModuleId: plan.scriptModuleId,
      displayName: plan.displayName,
      intent: plan.intent,
    );
    return Revision3QuestDraftPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      questId: checkpoint.questId,
      scriptModuleId: checkpoint.scriptModuleId,
    );
  }

  @override
  Future<Revision3QuestOutlineEditPublication>
  prepareAndPublishQuestOutlineEditV1({
    required Revision3QuestOutlineEditInput input,
  }) async {
    final slots = input.objectiveSlots;
    final planSeal = input.expectedTransitionPlanSeal;
    final checkpoint = slots == null || planSeal == null
        ? await _session.prepareAndPublishQuestOutlineEditV1(
            questId: input.questId,
            expectedQuestRevision: input.expectedQuestRevision,
            expectedModuleId: input.moduleId,
            expectedModuleRevision: input.expectedModuleRevision,
            displayName: input.displayName,
            title: input.title,
            objectiveTitles: input.objectiveTitles,
          )
        : await _session.prepareAndPublishQuestOutlineEditV2(
            questId: input.questId,
            expectedQuestRevision: input.expectedQuestRevision,
            expectedModuleId: input.moduleId,
            expectedModuleRevision: input.expectedModuleRevision,
            expectedTransitionPlanSeal: planSeal,
            displayName: input.displayName,
            title: input.title,
            objectiveSlots: slots,
            objectiveTitles: input.objectiveTitles,
          );
    return Revision3QuestOutlineEditPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      questId: checkpoint.questId,
      moduleId: checkpoint.moduleId,
      questRevision: checkpoint.questRevision,
      moduleRevision: checkpoint.moduleRevision,
    );
  }

  @override
  Future<AuthoringRevision3QuestTransitionsSeed> readQuestTransitionsSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) => _session.readQuestTransitionsSeedV1(
    questId: questId,
    expectedQuestRevision: expectedQuestRevision,
    expectedModuleId: expectedModuleId,
    expectedModuleRevision: expectedModuleRevision,
  );

  @override
  Future<Revision3QuestTransitionsEditPublication>
  prepareAndPublishQuestTransitionsEditV1({
    required Revision3QuestTransitionsEditTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishQuestTransitionsEditV1(
      questId: plan.questId,
      expectedQuestRevision: plan.expectedQuestRevision,
      expectedModuleId: plan.moduleId,
      expectedModuleRevision: plan.expectedModuleRevision,
      expectedTransitionPlanSeal: plan.expectedTransitionPlanSeal,
      transitionPlan: plan.transitionPlan,
    );
    return Revision3QuestTransitionsEditPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      questId: checkpoint.questId,
      moduleId: checkpoint.moduleId,
      questRevision: checkpoint.questRevision,
      moduleRevision: checkpoint.moduleRevision,
      transitionPlanSeal: checkpoint.transitionPlanSeal,
    );
  }

  @override
  Future<AuthoringRevision3QuestContextSeed> readQuestContextSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required String expectedParentRuntimeClass,
    required String expectedGiverRuntimeUniqueName,
  }) => _session.readQuestContextSeedV1(
    questId: questId,
    expectedQuestRevision: expectedQuestRevision,
    expectedModuleId: expectedModuleId,
    expectedModuleRevision: expectedModuleRevision,
    expectedParentRuntimeClass: expectedParentRuntimeClass,
    expectedGiverRuntimeUniqueName: expectedGiverRuntimeUniqueName,
  );

  @override
  Future<Revision3QuestContextEditPublication>
  prepareAndPublishQuestContextEditV1({
    required String gameRoot,
    required Revision3QuestContextEditTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishQuestContextEditV1(
      gameRoot: gameRoot,
      questId: plan.questId,
      expectedQuestRevision: plan.expectedQuestRevision,
      expectedModuleId: plan.moduleId,
      expectedModuleRevision: plan.expectedModuleRevision,
      expectedStoryCatalogSeal: plan.expectedStoryCatalogSeal,
      description: plan.description,
      parentCatalogId: plan.parentCatalogId,
      giverCatalogId: plan.giverCatalogId,
      expectedParentRuntimeClass: plan.expectedParentRuntimeClass,
      expectedParentCatalogLayer: plan.expectedParentCatalogLayer,
      expectedParentAuthoringSelector: plan.expectedParentAuthoringSelector,
      expectedParentSourceSeal: plan.expectedParentSourceSeal,
      expectedGiverRuntimeUniqueName: plan.expectedGiverRuntimeUniqueName,
      expectedGiverCatalogLayer: plan.expectedGiverCatalogLayer,
      expectedGiverAuthoringSelector: plan.expectedGiverAuthoringSelector,
      expectedGiverSourceSeal: plan.expectedGiverSourceSeal,
    );
    return Revision3QuestContextEditPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      questId: checkpoint.questId,
      moduleId: checkpoint.moduleId,
      questRevision: checkpoint.questRevision,
      moduleRevision: checkpoint.moduleRevision,
    );
  }

  @override
  Future<Revision3NpcDraftPublication> prepareAndPublishNpcDraftV1({
    required String gameRoot,
    required Revision3NpcDraftAuthoringInput input,
  }) async {
    final plan = Revision3NpcDraftTechnicalPlan.forCheckpoint(
      projectId: _session.projectId,
      projectRevision: _session.projectRevision,
      input: input,
    );
    final checkpoint = await _session.prepareAndPublishNpcDraftV1(
      gameRoot: gameRoot,
      npcId: plan.npcId,
      scriptModuleId: plan.scriptModuleId,
      displayName: plan.displayName,
      intent: plan.intent,
    );
    return Revision3NpcDraftPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      npcId: checkpoint.npcId,
      scriptModuleId: checkpoint.scriptModuleId,
    );
  }

  @override
  Future<Revision3NpcProfileEditPublication> prepareAndPublishNpcProfileEditV1({
    required String gameRoot,
    required Revision3NpcProfileEditTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishNpcProfileEditV1(
      gameRoot: gameRoot,
      seed: plan.seed,
      expectedStoryCatalogSeal: plan.expectedStoryCatalogSeal,
      expectedNpcCatalogSeal: plan.expectedNpcCatalogSeal,
      expectedParentCatalogId: plan.expectedParentCatalogId,
      expectedCurrentParentTriple: plan.expectedCurrentParentTriple,
      displayName: plan.displayName,
      parentCatalogId: plan.parentCatalogId,
      expectedParentTriple: plan.expectedParentTriple,
      expectedArchetypeChanged: plan.archetypeChanged,
      expectedModuleRegenerated: plan.moduleRegenerated,
    );
    return Revision3NpcProfileEditPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      npcId: checkpoint.npcId,
      npcRevision: checkpoint.npcRevision,
      scriptModuleId: checkpoint.scriptModuleId,
      scriptModuleRevision: checkpoint.scriptModuleRevision,
      displayName: checkpoint.displayName,
      previousParentCatalogId: checkpoint.previousParentCatalogId,
      parentCatalogId: checkpoint.parentCatalogId,
      nameChanged: checkpoint.nameChanged,
      archetypeChanged: checkpoint.archetypeChanged,
      moduleRegenerated: checkpoint.moduleRegenerated,
    );
  }

  @override
  Future<Revision3DialogLineEntryPublication> prepareAndPublishDialogLineV1({
    required Revision3DialogLineEntryTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishDialogLineV1(plan: plan);
    return Revision3DialogLineEntryPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      lineId: checkpoint.lineId,
      localizationId: checkpoint.localizationId,
      localizationAction: checkpoint.localizationAction,
      voiceSlotId: checkpoint.voiceSlotId,
      locale: plan.locale,
    );
  }

  @override
  Future<Revision3DialogLocalizationEditPublication>
  prepareAndPublishDialogLocalizationEditV1({
    required Revision3DialogLocalizationEditTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishDialogLocalizationEditV1(
      plan: plan,
    );
    return Revision3DialogLocalizationEditPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      localizationId: checkpoint.localizationId,
      localizationRevision: checkpoint.localizationRevision,
      addedLocales: checkpoint.addedLocales,
      removedLocales: checkpoint.removedLocales,
    );
  }

  @override
  Future<Revision3VoiceTakePublication> prepareAndPublishVoiceTakeV1({
    required String gameRoot,
    required Revision3VoiceTakeTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishVoiceTakeV1(
      gameRoot: gameRoot,
      source: plan.sourcePath,
      lineId: plan.lineId,
      slotId: plan.slotId,
      takeId: plan.takeId,
      locale: plan.locale,
      text: plan.text,
      takeDisplayName: plan.takeDisplayName,
      logicalName: plan.logicalName,
      status: plan.status,
      selectTake: plan.selectTake,
    );
    return Revision3VoiceTakePublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      lineId: checkpoint.lineId,
      slotId: checkpoint.slotId,
      takeId: checkpoint.takeId,
      slotCreated: checkpoint.slotCreated,
      selected: checkpoint.selected,
    );
  }

  @override
  Future<Revision3VoiceTakeSelectionPublication>
  prepareAndPublishVoiceTakeSelectionV1({
    required Revision3VoiceTakeSelectionTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishVoiceTakeSelectionV1(
      lineId: plan.lineId,
      slotId: plan.slotId,
      expectedSlotRevision: plan.expectedSlotRevision,
      locale: plan.locale,
      expectedLocId: plan.locId,
      expectedSelectedTakeId: plan.expectedSelectedTakeId,
      selectedTakeId: plan.selectedTakeId,
    );
    return Revision3VoiceTakeSelectionPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      lineId: checkpoint.lineId,
      slotId: checkpoint.slotId,
      slotRevision: checkpoint.slotRevision,
      locale: checkpoint.locale,
      locId: checkpoint.locId,
      previousSelectedTakeId: checkpoint.previousSelectedTakeId,
      selectedTakeId: checkpoint.selectedTakeId,
    );
  }

  @override
  Future<Revision3VoiceTakeRemovalPublication>
  prepareAndPublishVoiceTakeRemovalV1({
    required Revision3VoiceTakeRemovalTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishVoiceTakeRemovalV1(
      lineId: plan.lineId,
      localizationId: plan.localizationId,
      slotId: plan.slotId,
      expectedSlotRevision: plan.expectedSlotRevision,
      locale: plan.locale,
      expectedLocId: plan.locId,
      takeId: plan.takeId,
      expectedTakeRevision: plan.expectedTakeRevision,
      expectedSelectedTakeId: plan.expectedSelectedTakeId,
    );
    return Revision3VoiceTakeRemovalPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      lineId: checkpoint.lineId,
      localizationId: checkpoint.localizationId,
      slotId: checkpoint.slotId,
      slotRevision: checkpoint.slotRevision,
      locale: checkpoint.locale,
      locId: checkpoint.locId,
      takeId: checkpoint.takeId,
      takeRevision: checkpoint.takeRevision,
      previousSelectedTakeId: checkpoint.previousSelectedTakeId,
      selectionCleared: checkpoint.selectionCleared,
      takeEntityRemoved: checkpoint.takeEntityRemoved,
      remainingCandidateCount: checkpoint.remainingCandidateCount,
    );
  }

  @override
  Future<Revision3DialogVoiceSlotRemovalPublication>
  prepareAndPublishDialogVoiceSlotRemovalV1({
    required Revision3DialogVoiceSlotRemovalTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishDialogVoiceSlotRemovalV1(
      lineId: plan.lineId,
      expectedLineRevision: plan.expectedLineRevision,
      localizationId: plan.localizationId,
      slotId: plan.slotId,
      expectedSlotRevision: plan.expectedSlotRevision,
      locale: plan.locale,
      expectedLocId: plan.locId,
    );
    return Revision3DialogVoiceSlotRemovalPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      lineId: checkpoint.lineId,
      lineRevision: checkpoint.lineRevision,
      localizationId: checkpoint.localizationId,
      slotId: checkpoint.slotId,
      removedSlotRevision: checkpoint.removedSlotRevision,
      locale: checkpoint.locale,
      locId: checkpoint.locId,
      removedTargetResolution: checkpoint.removedTargetResolution,
    );
  }

  @override
  Future<Revision3VoiceTakeStatusPublication>
  prepareAndPublishVoiceTakeStatusV1({
    required Revision3VoiceTakeStatusTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishVoiceTakeStatusV1(
      lineId: plan.lineId,
      localizationId: plan.localizationId,
      expectedLocId: plan.locId,
      locale: plan.locale,
      slotId: plan.slotId,
      expectedSlotRevision: plan.expectedSlotRevision,
      takeId: plan.takeId,
      expectedTakeRevision: plan.expectedTakeRevision,
      expectedStatus: plan.expectedStatus,
      desiredStatus: plan.desiredStatus,
    );
    return Revision3VoiceTakeStatusPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      lineId: checkpoint.lineId,
      localizationId: checkpoint.localizationId,
      slotId: checkpoint.slotId,
      slotRevision: checkpoint.slotRevision,
      locale: checkpoint.locale,
      locId: checkpoint.locId,
      takeId: checkpoint.takeId,
      takeRevision: checkpoint.takeRevision,
      previousStatus: checkpoint.previousStatus,
      status: checkpoint.status,
    );
  }

  @override
  Future<Revision3VoiceTargetPublication> prepareAndPublishVoiceTargetV1({
    required String gameRoot,
    required Revision3VoiceTargetTechnicalPlan plan,
  }) async {
    final checkpoint = await _session.prepareAndPublishVoiceTargetV1(
      gameRoot: gameRoot,
      lineId: plan.lineId,
      slotId: plan.slotId,
      locale: plan.locale,
      expectedLocId: plan.locId,
    );
    return Revision3VoiceTargetPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      lineId: checkpoint.lineId,
      slotId: checkpoint.slotId,
      locale: checkpoint.locale,
      locId: checkpoint.locId,
      resolution: checkpoint.resolution,
      matchCount: checkpoint.targets.length,
    );
  }

  @override
  Future<AuthoringRevision3VoiceBuildResult> buildVoiceV1({
    required String gameRoot,
    required String output,
  }) => _session.buildVoiceV1(gameRoot: gameRoot, output: output);

  @override
  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  buildReviewedDataAssetV1({
    required String gameRoot,
    required String targetPath,
    required String packName,
    required String output,
  }) => _session.buildReviewedDataAssetV1(
    gameRoot: gameRoot,
    targetPath: targetPath,
    packName: packName,
    output: output,
  );

  @override
  Future<List<AuthoringRevision3DataAssetStage>> listDataAssetStagesV1() =>
      _session.listDataAssetStagesV1();

  @override
  Future<Revision3DataAssetStagePublication> prepareAndPublishDataAssetStageV1({
    required String patchReceiptPath,
  }) async {
    final checkpoint = await _session.prepareAndPublishDataAssetStageV1(
      patchReceiptPath: patchReceiptPath,
    );
    return Revision3DataAssetStagePublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      stage: checkpoint.stage,
      deduplicatedBlobs: checkpoint.deduplicatedBlobs,
    );
  }

  @override
  Future<Revision3DataAssetStagePublication> prepareAndPublishDataAssetEditV1({
    required DataAssetSemanticEditIntent intent,
  }) async {
    final checkpoint = await _session.prepareAndPublishDataAssetEditV1(
      intent: intent,
    );
    return Revision3DataAssetStagePublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      stage: checkpoint.stage,
      deduplicatedBlobs: checkpoint.deduplicatedBlobs,
    );
  }

  @override
  Future<Revision3DataAssetStagePublication>
  prepareAndPublishInstalledDataAssetEditV1({
    required String gameRoot,
    required DataAssetInstalledSemanticEditIntent intent,
  }) async {
    final checkpoint = await _session.prepareAndPublishInstalledDataAssetEditV1(
      gameRoot: gameRoot,
      intent: intent,
    );
    return Revision3DataAssetStagePublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      stage: checkpoint.stage,
      deduplicatedBlobs: checkpoint.deduplicatedBlobs,
    );
  }

  @override
  Future<Revision3DataAssetStagePublication>
  prepareAndPublishReviewedInstalledDataAssetEditV1({
    required String gameRoot,
    required ReviewedInstalledDataAssetEditIntent intent,
  }) async {
    final checkpoint = await _session
        .prepareAndPublishReviewedInstalledDataAssetEditV1(
          gameRoot: gameRoot,
          intent: intent,
        );
    return Revision3DataAssetStagePublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      stage: checkpoint.stage,
      deduplicatedBlobs: checkpoint.deduplicatedBlobs,
    );
  }

  @override
  Future<Revision3DataAssetStageRemovalPublication>
  prepareAndPublishRemoveDataAssetStageV1({required String targetPath}) async {
    final checkpoint = await _session.prepareAndPublishRemoveDataAssetStageV1(
      targetPath: targetPath,
    );
    return Revision3DataAssetStageRemovalPublication(
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      removed: checkpoint.removed,
    );
  }

  @override
  Future<Revision3StoryDraftRemovalPublication>
  prepareAndPublishRemoveStoryDraftV1({
    required String draftId,
    required AuthoringStoryDraftKind draftKind,
    required int expectedDraftRevision,
    required String scriptModuleId,
    required int expectedScriptModuleRevision,
  }) async {
    final checkpoint = await _session.prepareAndPublishRemoveStoryDraftV1(
      draftId: draftId,
      draftKind: draftKind,
      expectedDraftRevision: expectedDraftRevision,
      scriptModuleId: scriptModuleId,
      expectedScriptModuleRevision: expectedScriptModuleRevision,
    );
    return Revision3StoryDraftRemovalPublication(
      head: checkpoint.head,
      projectId: checkpoint.projectId,
      projectRevision: checkpoint.projectRevision,
      removedDraftId: checkpoint.removedDraftId,
      removedDraftKind: checkpoint.removedDraftKind,
      removedDraftRevision: checkpoint.removedDraftRevision,
      removedScriptModuleId: checkpoint.removedScriptModuleId,
      removedScriptModuleRevision: checkpoint.removedScriptModuleRevision,
    );
  }

  @override
  Future<void> close() => _session.close();
}

/// Produces a fresh one-shot ownership token for each legacy adoption.
///
/// The underlying compatibility session remains provider-scoped, but a closed
/// lease can never be adopted again or accidentally skip a later `newProject`.
final legacyCurrentProjectLeaseFactoryProvider =
    Provider<LegacyCurrentProjectLeaseFactory>((ref) {
      final session = ref.read(projectSessionProvider);
      return () => ProjectSessionLegacyCurrentProjectLease(session);
    });

final managedRevision3StoryGenerationLoaderProvider =
    Provider<ManagedRevision3StoryGenerationLoader>((ref) {
      final ffi = ModFfi(ref.read(coreServiceProvider));
      return (gameRoot) async =>
          (await ffi.authoringStoryCatalogV1BuildForGameRoot(
            gameRoot: gameRoot,
          )).generation;
    });

final managedRevision3ProjectSessionCreatorProvider =
    Provider<ManagedRevision3ProjectSessionCreator>((ref) {
      final store = ModFfiManagedRevision3AuthoringStore(
        ModFfi(ref.read(coreServiceProvider)),
      );
      return ({required root, required projectJson}) async =>
          _ManagedRevision3SessionLease(
            await ManagedRevision3AuthoringProjectSession.create(
              root: root,
              store: store,
              projectJson: projectJson,
            ),
          );
    });

final managedRevision3CurrentProjectOpenerProvider =
    Provider<ManagedRevision3CurrentProjectOpener>((ref) {
      final store = ModFfiManagedRevision3AuthoringStore(
        ModFfi(ref.read(coreServiceProvider)),
      );
      return (root) async => _ManagedRevision3SessionLease(
        await ManagedRevision3AuthoringProjectSession.open(
          root: root,
          store: store,
        ),
      );
    });

typedef ManagedRevision3ProjectIdFactory = String Function();

final managedRevision3ProjectIdFactoryProvider =
    Provider<ManagedRevision3ProjectIdFactory>((ref) {
      final random = Random.secure();
      return () {
        final buffer = StringBuffer();
        do {
          buffer.clear();
          for (var index = 0; index < 16; index++) {
            buffer.write(random.nextInt(256).toRadixString(16).padLeft(2, '0'));
          }
        } while (buffer.toString() == '00000000000000000000000000000000');
        return buffer.toString();
      };
    });

/// Production creator for one empty, generation-bound managed R3 project.
///
/// The selected root must already exist and be completely empty. Native code
/// first authenticates the installed Story generation; the Dart bootstrap then
/// copies only its exact executable seal into canonical project JSON. The
/// managed session owns immutable-object preparation, absent-head CAS
/// publication, repair, and full published reopen.
final managedRevision3CurrentProjectCreatorProvider =
    Provider<ManagedRevision3CurrentProjectCreator>((ref) {
      final loadGeneration = ref.read(
        managedRevision3StoryGenerationLoaderProvider,
      );
      final createSession = ref.read(
        managedRevision3ProjectSessionCreatorProvider,
      );
      final recoverSession = ref.read(
        managedRevision3CurrentProjectOpenerProvider,
      );
      final newProjectId = ref.read(managedRevision3ProjectIdFactoryProvider);
      return (request) async {
        final root = await _requireNewManagedRevision3Root(request.root);
        await _requireManagedRevision3RootOutsideGame(root, request.gameRoot);
        final generation = await loadGeneration(request.gameRoot);
        // Generation discovery may hash hundreds of MiB. Recheck the selected
        // destination after that read-only work, before creating any control or
        // Store object in it.
        await _requireNewManagedRevision3Root(root);
        final bootstrap = Revision3ProjectBootstrap.create(
          generation: generation,
          projectId: newProjectId(),
          name: request.name,
          version: request.version,
          author: request.author,
          authoringLocales: request.authoringLocales,
        );
        ManagedRevision3CurrentProjectLease? session;
        try {
          session = await createSession(
            root: root,
            projectJson: bootstrap.canonicalProjectJson,
          );
          if (!_managedRevision3LeaseMatchesBootstrap(
            session,
            root,
            bootstrap,
          )) {
            throw const ManagedRevision3ProjectCreationException(
              'the fully reopened managed project differs from its bootstrap identity',
            );
          }
          return session;
        } catch (error, stackTrace) {
          if (session != null) {
            try {
              await session.close();
            } catch (cleanupError) {
              Error.throwWithStackTrace(
                ManagedRevision3ProjectCreationException(
                  'managed project creation failed and candidate cleanup also failed: '
                  '$cleanupError',
                ),
                stackTrace,
              );
            }
            Error.throwWithStackTrace(error, stackTrace);
          }

          ManagedRevision3CurrentProjectLease? recovered;
          Object? recoveryError;
          try {
            recovered = await recoverSession(root);
            if (_managedRevision3LeaseMatchesBootstrap(
              recovered,
              root,
              bootstrap,
            )) {
              return recovered;
            }
            recoveryError = const ManagedRevision3ProjectCreationException(
              'the recoverable managed project differs from its bootstrap identity',
            );
          } catch (candidateRecoveryError) {
            recoveryError = candidateRecoveryError;
          }
          if (recovered != null) {
            try {
              await recovered.close();
            } catch (cleanupError) {
              recoveryError = ManagedRevision3ProjectCreationException(
                'managed project recovery and cleanup both failed: $cleanupError',
              );
            }
          }
          Error.throwWithStackTrace(
            ManagedRevision3ProjectCreationException(
              'managed project creation did not finish. The selected folder '
              'may contain a recoverable project; use Open managed project '
              'before retrying, or choose another empty folder. Initial '
              'failure: $error. Recovery: $recoveryError',
            ),
            stackTrace,
          );
        }
      };
    });

final currentProjectCoordinatorProvider =
    StateNotifierProvider<CurrentProjectCoordinator, CurrentProjectState>((
      ref,
    ) {
      final createLegacy = ref.read(legacyCurrentProjectLeaseFactoryProvider);
      return CurrentProjectCoordinator(
        initialLegacy: createLegacy(),
        createLegacy: createLegacy,
        createManagedRevision3: ref.read(
          managedRevision3CurrentProjectCreatorProvider,
        ),
        openManagedRevision3: ref.read(
          managedRevision3CurrentProjectOpenerProvider,
        ),
      );
    });

bool _managedRevision3LeaseMatchesBootstrap(
  ManagedRevision3CurrentProjectLease lease,
  Directory root,
  Revision3ProjectBootstrap bootstrap,
) =>
    lease.projectId == bootstrap.identity.projectId &&
    lease.projectRevision == Revision3ProjectBootstrap.initialRevision &&
    lease.canonicalProjectJson == bootstrap.canonicalProjectJson &&
    !lease.requiresReopen &&
    _sameManagedRevision3Path(lease.root.path, root.path);

Future<Directory> _requireNewManagedRevision3Root(Directory requested) async {
  final absolute = p.normalize(p.absolute(requested.path));
  final type = await FileSystemEntity.type(absolute, followLinks: false);
  if (type != FileSystemEntityType.directory) {
    throw ManagedRevision3ProjectCreationException(
      'managed project destination must be an existing real directory: $absolute',
    );
  }
  final resolved = p.normalize(
    await Directory(absolute).resolveSymbolicLinks(),
  );
  final resolvedType = await FileSystemEntity.type(
    resolved,
    followLinks: false,
  );
  if (resolvedType != FileSystemEntityType.directory) {
    throw ManagedRevision3ProjectCreationException(
      'managed project destination must resolve to a real directory: $absolute',
    );
  }
  final root = Directory(resolved);
  if (!await root.list(followLinks: false).isEmpty) {
    throw ManagedRevision3ProjectCreationException(
      'managed project destination must be completely empty: $resolved',
    );
  }
  return root;
}

Future<void> _requireManagedRevision3RootOutsideGame(
  Directory root,
  String requestedGameRoot,
) async {
  final game = p.normalize(
    await Directory(
      p.normalize(p.absolute(requestedGameRoot)),
    ).resolveSymbolicLinks(),
  );
  final project = p.normalize(root.path);
  if (_sameManagedRevision3Path(project, game) ||
      _managedRevision3PathWithin(game, project) ||
      _managedRevision3PathWithin(project, game)) {
    throw const ManagedRevision3ProjectCreationException(
      'managed project destination and game installation must be disjoint',
    );
  }
}

bool _managedRevision3PathWithin(String parent, String child) => p.isWithin(
  _managedRevision3CasePath(parent),
  _managedRevision3CasePath(child),
);

bool _sameManagedRevision3Path(String left, String right) =>
    _managedRevision3CasePath(left) == _managedRevision3CasePath(right);

String _managedRevision3CasePath(String value) {
  final normalized = p.normalize(value);
  return Platform.isWindows ? normalized.toLowerCase() : normalized;
}

final RegExp _managedRevision3RecoveryProjectIdPattern = RegExp(
  r'^[0-9a-f]{32}$',
);
const _managedRevision3RecoveryZeroProjectId =
    '00000000000000000000000000000000';

String? _managedRevision3CanonicalTargetForCheckpoint({
  required String projectJson,
  required String projectId,
  required int projectRevision,
  required AuthoringWorkingHead head,
}) {
  try {
    final bytes = utf8.encode(projectJson);
    if (bytes.length != head.snapshotByteLength ||
        crypto.sha256.convert(bytes).toString() != head.snapshotSha256) {
      return null;
    }
    final decoded = jsonDecode(projectJson);
    if (decoded is! Map) return null;
    final project = decoded.cast<String, Object?>();
    final decodedProjectId = project['project_id'];
    final decodedRevision = project['revision'];
    final target = project['target'];
    if (jsonEncode(project) != projectJson ||
        project['format'] != 2 ||
        project['schema_revision'] != 3 ||
        decodedProjectId is! String ||
        !_managedRevision3RecoveryProjectIdPattern.hasMatch(decodedProjectId) ||
        decodedProjectId == _managedRevision3RecoveryZeroProjectId ||
        decodedProjectId != projectId ||
        decodedRevision is! int ||
        decodedRevision < 0 ||
        decodedRevision > 0x7fffffffffffffff ||
        decodedRevision != projectRevision ||
        target is! Map) {
      return null;
    }
    return jsonEncode(target);
  } catch (_) {
    return null;
  }
}

/// Single app-wide owner for compatibility and managed project lifetimes.
///
/// Candidate opens complete before adoption, so a failed open cannot disturb
/// the current project. Open/adopt/save/verify/close operations share one lane;
/// after adoption the previous lease is closed before the next transition can
/// run. At most one lease is authoritative. Because leases have terminal,
/// memoized close semantics, cleanup failures are retained only as diagnostics
/// and are never misleadingly retried.
final class CurrentProjectCoordinator
    extends StateNotifier<CurrentProjectState> {
  factory CurrentProjectCoordinator({
    LegacyCurrentProjectLease? initialLegacy,
    LegacyCurrentProjectLeaseFactory? createLegacy,
    ManagedRevision3CurrentProjectCreator? createManagedRevision3,
    required ManagedRevision3CurrentProjectOpener openManagedRevision3,
  }) {
    final initial = initialLegacy == null
        ? null
        : _OwnedLegacyCurrentProject(initialLegacy);
    return CurrentProjectCoordinator._(
      current: initial,
      initialState: initial == null
          ? const NoCurrentProjectState()
          : _stateOf(initial),
      createLegacy: createLegacy,
      createManagedRevision3: createManagedRevision3,
      openManagedRevision3: openManagedRevision3,
    );
  }

  CurrentProjectCoordinator._({
    required this._current,
    required CurrentProjectState initialState,
    required this._createLegacy,
    required this._createManagedRevision3,
    required this._openManagedRevision3,
  }) : super(initialState);

  final LegacyCurrentProjectLeaseFactory? _createLegacy;
  final ManagedRevision3CurrentProjectCreator? _createManagedRevision3;
  final ManagedRevision3CurrentProjectOpener _openManagedRevision3;
  _OwnedCurrentProject? _current;
  final List<CurrentProjectCleanupFailure> _terminalCleanupFailures =
      <CurrentProjectCleanupFailure>[];
  Future<void> _tail = Future<void>.value();
  Future<void>? _shutdownFuture;
  bool _shutdownRequested = false;
  bool _notifierDisposed = false;

  /// Terminal close failures retained for diagnostics. No failed lease is
  /// retained or closed again because both production lease types memoize the
  /// first close attempt.
  List<CurrentProjectCleanupFailure> get terminalCleanupFailures =>
      List<CurrentProjectCleanupFailure>.unmodifiable(_terminalCleanupFailures);

  bool get isShutdownRequested => _shutdownRequested;

  /// Fully open and verify [root], then atomically make it the current project.
  /// A failed candidate open or candidate snapshot leaves the current lease and
  /// public state unchanged.
  Future<ManagedRevision3CurrentProjectState> openManagedRevision3(
    Directory root,
  ) => _enqueue(() async {
    ManagedRevision3CurrentProjectLease? candidateLease;
    var adopted = false;
    try {
      candidateLease = await _openManagedRevision3(root);
      final candidate = _OwnedManagedRevision3CurrentProject(candidateLease);
      final candidateState = _stateOf(candidate);
      await _adopt(candidate, candidateState);
      adopted = true;
      return candidateState as ManagedRevision3CurrentProjectState;
    } catch (error, stackTrace) {
      if (candidateLease != null && !adopted) {
        await _closeUnadopted(
          _OwnedManagedRevision3CurrentProject(candidateLease),
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    }
  });

  /// Create, fully reopen, and only then adopt one empty managed R3 project.
  ///
  /// Creation shares the same invocation-ordered ownership lane as Open. A
  /// failed creator or invalid candidate snapshot cannot displace the current
  /// project; an unadopted candidate is terminally closed exactly once.
  Future<ManagedRevision3CurrentProjectState> createManagedRevision3(
    ManagedRevision3ProjectCreateRequest request,
  ) => _enqueue(() async {
    final create = _createManagedRevision3;
    if (create == null) {
      throw const CurrentProjectOperationUnsupportedException(
        'creating managed revision-3 projects is unavailable',
      );
    }
    ManagedRevision3CurrentProjectLease? candidateLease;
    var adopted = false;
    try {
      candidateLease = await create(request);
      final candidate = _OwnedManagedRevision3CurrentProject(candidateLease);
      final candidateState = _stateOf(candidate);
      await _adopt(candidate, candidateState);
      adopted = true;
      return candidateState as ManagedRevision3CurrentProjectState;
    } catch (error, stackTrace) {
      if (candidateLease != null && !adopted) {
        await _closeUnadopted(
          _OwnedManagedRevision3CurrentProject(candidateLease),
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    }
  });

  /// Adopt an independently-opened compatibility lease.
  ///
  /// Ownership transfers when this invocation reaches the serialized lane.
  Future<LegacyCurrentProjectState> adoptLegacy(
    LegacyCurrentProjectLease lease,
  ) => _enqueue(() async {
    final current = _current;
    if (current is _OwnedLegacyCurrentProject &&
        identical(current.lease, lease)) {
      final refreshed = _stateOf(current) as LegacyCurrentProjectState;
      _publish(refreshed);
      return refreshed;
    }
    final candidate = _OwnedLegacyCurrentProject(lease);
    try {
      final candidateState = _stateOf(candidate) as LegacyCurrentProjectState;
      await _adopt(candidate, candidateState);
      return candidateState;
    } catch (error, stackTrace) {
      if (!identical(_current, candidate)) await _closeUnadopted(candidate);
      Error.throwWithStackTrace(error, stackTrace);
    }
  });

  /// Start a clean compatibility project inside the same cross-format lane.
  ///
  /// When a managed project is current, the fresh compatibility lease remains
  /// a candidate until its provider graph has been reset successfully. A
  /// failure therefore leaves the managed lease authoritative.
  Future<LegacyCurrentProjectState> newLegacyProject() =>
      _operateOnLegacy((lease) => lease.newProject());

  /// Fully load a compatibility archive before adopting it as current.
  ///
  /// Candidate cleanup is terminal and best-effort, matching managed-open
  /// semantics. A failed load never displaces the current managed lease.
  Future<LegacyCurrentProjectState> openLegacyFromPath(String path) =>
      _operateOnLegacy((lease) => lease.openFromPath(path));

  /// Save the current compatibility project to [path] and refresh its exact
  /// path/dirty snapshot. Managed projects intentionally have no Save As path
  /// until a native clone/fork transaction exists.
  Future<LegacyCurrentProjectState> saveLegacyToPath(String path) =>
      _enqueue(() async {
        final current = _current;
        if (current == null) throw const NoCurrentProjectException();
        if (current is! _OwnedLegacyCurrentProject) {
          throw const CurrentProjectOperationUnsupportedException(
            'Save As is unavailable for managed revision-3 projects',
          );
        }
        await current.lease.saveToPath(path);
        return _refreshCurrentIfUnchanged(current) as LegacyCurrentProjectState;
      });

  /// Ctrl+S-sized durability action for the active backend.
  ///
  /// Compatibility projects write their captured provider snapshot. Managed
  /// revision-3 projects already publish every semantic transaction, so this
  /// performs only an exact-head, full-asset reopen verification.
  Future<CurrentProjectState> saveCurrent() => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    late CurrentProjectState refreshed;
    try {
      switch (current) {
        case _OwnedLegacyCurrentProject(:final lease):
          await lease.saveCurrent();
        case _OwnedManagedRevision3CurrentProject(:final lease):
          if (lease.requiresReopen) {
            throw const CurrentProjectOperationUnsupportedException(
              'managed revision-3 verification is blocked until the project is reopened',
            );
          }
          await lease.verifyCurrentHead();
      }
    } finally {
      refreshed = _refreshCurrentIfUnchanged(current);
    }
    return refreshed;
  });

  /// Read-only exact-head verification for a managed revision-3 current
  /// project. Legacy archives have no equivalent full-reopen contract.
  Future<ManagedRevision3CurrentProjectState>
  verifyCurrent() => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'exact current-head verification is available only for managed revision-3 projects',
      );
    }
    if (current.lease.requiresReopen) {
      throw const CurrentProjectOperationUnsupportedException(
        'managed revision-3 verification is blocked until the project is reopened',
      );
    }
    late ManagedRevision3CurrentProjectState refreshed;
    try {
      await current.lease.verifyCurrentHead();
    } finally {
      refreshed =
          _refreshCurrentIfUnchanged(current)
              as ManagedRevision3CurrentProjectState;
    }
    return refreshed;
  });

  /// Reconcile one exact, visibly poisoned managed checkpoint with any durable
  /// publication that may have completed before its caller observed a result.
  /// Normal editing remains blocked until the complete recovery result can be
  /// bound to the same lease, project JSON, revision, and working head.
  Future<ManagedRevision3RecoveryCheckpoint> recoverCurrentRevision3({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
  }) => _enqueue(() async {
    final current = _current;
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const Revision3RecoveryNotSupportedException();
    }
    final lease = current.lease;
    if (!lease.requiresReopen) {
      throw const Revision3RecoveryNotRequiredException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3RecoveryStaleCheckpointException();
    }
    if (lease is! ManagedRevision3RecoveryLease) {
      throw const Revision3RecoveryNotSupportedException();
    }
    final recovery = lease as ManagedRevision3RecoveryLease;
    final previousProjectJson = lease.canonicalProjectJson;
    try {
      final checkpoint = await recovery.recoverAfterUncertainPublication();
      final previousCanonicalTarget =
          _managedRevision3CanonicalTargetForCheckpoint(
            projectJson: previousProjectJson,
            projectId: expectedProjectId,
            projectRevision: expectedProjectRevision,
            head: expectedHead,
          );
      final recoveredCanonicalTarget =
          _managedRevision3CanonicalTargetForCheckpoint(
            projectJson: checkpoint.canonicalProjectJson,
            projectId: expectedProjectId,
            projectRevision: checkpoint.recoveredProjectRevision,
            head: checkpoint.recoveredHead,
          );
      final unchanged =
          checkpoint.recoveredProjectRevision == expectedProjectRevision &&
          checkpoint.recoveredHead.canonicalJson ==
              expectedHead.canonicalJson &&
          checkpoint.canonicalProjectJson == previousProjectJson;
      final advanced =
          checkpoint.recoveredProjectRevision == expectedProjectRevision + 1 &&
          checkpoint.recoveredHead.canonicalJson != expectedHead.canonicalJson;
      if (lease.requiresReopen ||
          lease.root.path != expectedRoot ||
          lease.projectId != expectedProjectId ||
          lease.projectRevision != checkpoint.recoveredProjectRevision ||
          lease.head.canonicalJson != checkpoint.recoveredHead.canonicalJson ||
          lease.canonicalProjectJson != checkpoint.canonicalProjectJson ||
          checkpoint.projectId != expectedProjectId ||
          checkpoint.previousProjectRevision != expectedProjectRevision ||
          checkpoint.previousHead.canonicalJson != expectedHead.canonicalJson ||
          (!unchanged && !advanced) ||
          previousCanonicalTarget == null ||
          recoveredCanonicalTarget == null ||
          recoveredCanonicalTarget != previousCanonicalTarget) {
        throw const Revision3RecoveryFailedException();
      }
      _refreshCurrentIfUnchanged(current);
      return checkpoint;
    } catch (error, stackTrace) {
      if (!lease.requiresReopen) {
        recovery.markRequiresReopenAfterRecoveryUncertainty();
      }
      _refreshCurrentIfUnchanged(current);
      if (error is Revision3RecoveryFailedException) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      Error.throwWithStackTrace(
        const Revision3RecoveryFailedException(),
        stackTrace,
      );
    }
  });

  /// Read a bounded authenticated history for the exact visibly current R3
  /// checkpoint. No Store directory scan or project mutation is authorized.
  Future<Revision3ProjectHistorySnapshot> readCurrentRevision3ProjectHistory({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
  }) => _enqueue(() async {
    final current = _current;
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const Revision3ProjectHistoryUnsupportedException();
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3ProjectHistoryRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3ProjectHistoryStaleCheckpointException();
    }
    if (lease is! ManagedRevision3ProjectHistoryLease ||
        !(lease as ManagedRevision3ProjectHistoryLease)
            .supportsProjectHistory) {
      throw const Revision3ProjectHistoryUnsupportedException();
    }
    final historyLease = lease as ManagedRevision3ProjectHistoryLease;
    try {
      final history = await historyLease.readProjectHistoryV1();
      if (lease.requiresReopen ||
          lease.root.path != expectedRoot ||
          lease.projectId != expectedProjectId ||
          lease.projectRevision != expectedProjectRevision ||
          lease.head.canonicalJson != expectedHead.canonicalJson ||
          history.basisHead.canonicalJson != expectedHead.canonicalJson ||
          history.projectId != expectedProjectId ||
          history.currentRevision != expectedProjectRevision) {
        historyLease.markRequiresReopenAfterHistoryUncertainty();
        throw const Revision3ProjectHistoryFailedException();
      }
      return history;
    } catch (error, stackTrace) {
      if (lease.requiresReopen &&
          error is! Revision3ProjectHistoryFailedException) {
        Error.throwWithStackTrace(
          const Revision3ProjectHistoryRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Restore one selected authenticated ancestor as a new current+1 project
  /// generation. The old current generation remains in the lineage.
  Future<Revision3ProjectHistoryRestorePublication>
  restoreCurrentRevision3ProjectHistory({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required Revision3ProjectHistorySnapshot expectedHistory,
    required Revision3ProjectHistoryEntry target,
  }) => _enqueue(() async {
    final current = _current;
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const Revision3ProjectHistoryUnsupportedException();
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3ProjectHistoryRequiresReopenException();
    }
    final targetIsListed = expectedHistory.entries.any(
      (entry) =>
          !entry.isCurrent &&
          entry.projectRevision == target.projectRevision &&
          entry.head.canonicalJson == target.head.canonicalJson,
    );
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson ||
        expectedHistory.basisHead.canonicalJson != expectedHead.canonicalJson ||
        expectedHistory.projectId != expectedProjectId ||
        expectedHistory.currentRevision != expectedProjectRevision ||
        target.projectId != expectedProjectId ||
        target.isCurrent ||
        target.projectRevision >= expectedProjectRevision ||
        !targetIsListed) {
      throw const Revision3ProjectHistoryStaleCheckpointException();
    }
    if (lease is! ManagedRevision3ProjectHistoryLease ||
        !(lease as ManagedRevision3ProjectHistoryLease)
            .supportsProjectHistory) {
      throw const Revision3ProjectHistoryUnsupportedException();
    }
    final historyLease = lease as ManagedRevision3ProjectHistoryLease;
    final previousProjectJson = lease.canonicalProjectJson;
    try {
      final checkpoint = await historyLease
          .prepareAndPublishProjectHistoryRestoreV1(
            expectedHistory: expectedHistory,
            target: target,
          );
      final previousTarget = _managedRevision3CanonicalTargetForCheckpoint(
        projectJson: previousProjectJson,
        projectId: expectedProjectId,
        projectRevision: expectedProjectRevision,
        head: expectedHead,
      );
      final restoredTarget = _managedRevision3CanonicalTargetForCheckpoint(
        projectJson: checkpoint.projectJson,
        projectId: expectedProjectId,
        projectRevision: checkpoint.projectRevision,
        head: checkpoint.head,
      );
      if (lease.requiresReopen ||
          lease.root.path != expectedRoot ||
          lease.projectId != expectedProjectId ||
          lease.projectRevision != expectedProjectRevision + 1 ||
          lease.head.canonicalJson != checkpoint.head.canonicalJson ||
          lease.canonicalProjectJson != checkpoint.projectJson ||
          checkpoint.previousHead.canonicalJson != expectedHead.canonicalJson ||
          checkpoint.projectId != expectedProjectId ||
          checkpoint.previousProjectRevision != expectedProjectRevision ||
          checkpoint.projectRevision != expectedProjectRevision + 1 ||
          checkpoint.restoredFromHead.canonicalJson !=
              target.head.canonicalJson ||
          checkpoint.restoredFromRevision != target.projectRevision ||
          checkpoint.head.canonicalJson == expectedHead.canonicalJson ||
          previousTarget == null ||
          restoredTarget == null ||
          restoredTarget != previousTarget) {
        historyLease.markRequiresReopenAfterHistoryUncertainty();
        throw const Revision3ProjectHistoryFailedException();
      }
      return Revision3ProjectHistoryRestorePublication(
        previousHead: checkpoint.previousHead,
        head: checkpoint.head,
        projectId: checkpoint.projectId,
        previousProjectRevision: checkpoint.previousProjectRevision,
        projectRevision: checkpoint.projectRevision,
        restoredFromHead: checkpoint.restoredFromHead,
        restoredFromRevision: checkpoint.restoredFromRevision,
      );
    } catch (error, stackTrace) {
      if (lease.requiresReopen &&
          error is! Revision3ProjectHistoryFailedException) {
        Error.throwWithStackTrace(
          const Revision3ProjectHistoryRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Read the semantic index of the exact managed revision-3 current project.
  ///
  /// This shares the app-wide project lane with save/open/close transitions. A legacy project has
  /// no equivalent semantic projection, and a poisoned managed lease must first be reopened.
  Future<Revision3ContentIndex>
  readCurrentRevision3ContentIndex() => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'the revision-3 content index is available only for managed revision-3 projects',
      );
    }
    if (current.lease.requiresReopen) {
      throw const Revision3ContentRequiresReopenException();
    }
    try {
      return await current.lease.readContentIndex();
    } catch (error, stackTrace) {
      if (current.lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3ContentRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Read bounded previews for one exact LocalizationEntry from the visible
  /// managed revision-3 checkpoint. No game, project mutation, or publication
  /// input crosses this read-only capability boundary.
  Future<AuthoringRevision3DialogLocalizationReadResult>
  readCurrentRevision3DialogLocalization({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'dialog localization reads are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DialogLocalizationReadRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DialogLocalizationReadStaleCheckpointException();
    }
    if (lease is! ManagedRevision3DialogLocalizationReadLease) {
      throw const CurrentProjectOperationUnsupportedException(
        'this managed revision-3 lease cannot read dialog localization previews',
      );
    }
    final reader = lease as ManagedRevision3DialogLocalizationReadLease;
    try {
      final result = await reader.readDialogLocalizationV1(
        localizationId: localizationId,
        expectedLocalizationRevision: expectedLocalizationRevision,
        expectedLocId: expectedLocId,
      );
      if (result.head.canonicalJson != expectedHead.canonicalJson ||
          result.projectId != expectedProjectId ||
          result.projectRevision != expectedProjectRevision ||
          result.localizationId != localizationId ||
          result.localizationRevision != expectedLocalizationRevision ||
          result.locId != expectedLocId) {
        throw const Revision3DialogLocalizationReadStaleCheckpointException();
      }
      return result;
    } on ModFfiException catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DialogLocalizationReadRequiresReopenException(),
          stackTrace,
        );
      }
      if (_revision3DialogLocalizationReadErrorIsStale(error.code)) {
        Error.throwWithStackTrace(
          const Revision3DialogLocalizationReadStaleCheckpointException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DialogLocalizationReadRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Read a complete exact-current authored text plus the bounded DialogLine
  /// and VoiceSlot facts required by the Localization & Voice workbench.
  Future<AuthoringRevision3DialogLocalizationEditSeed>
  readCurrentRevision3DialogLocalizationEditSeed({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'localization editing is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DialogLocalizationEditRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DialogLocalizationEditStaleCheckpointException();
    }
    if (lease is! ManagedRevision3DialogLocalizationEditLease) {
      throw const CurrentProjectOperationUnsupportedException(
        'this managed revision-3 lease cannot edit dialog localization',
      );
    }
    final editor = lease as ManagedRevision3DialogLocalizationEditLease;
    try {
      final result = await editor.readDialogLocalizationEditSeedV1(
        localizationId: localizationId,
        expectedLocalizationRevision: expectedLocalizationRevision,
        expectedLocId: expectedLocId,
      );
      if (result.head.canonicalJson != expectedHead.canonicalJson ||
          result.projectId != expectedProjectId ||
          result.projectRevision != expectedProjectRevision ||
          result.localizationId != localizationId ||
          result.localizationRevision != expectedLocalizationRevision ||
          result.locId != expectedLocId) {
        throw const Revision3DialogLocalizationEditStaleCheckpointException();
      }
      return result;
    } on ManagedRevision3DialogLocalizationEditStaleException catch (
      _,
      stackTrace
    ) {
      Error.throwWithStackTrace(
        const Revision3DialogLocalizationEditStaleCheckpointException(),
        stackTrace,
      );
    } on ModFfiException catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DialogLocalizationEditRequiresReopenException(),
          stackTrace,
        );
      }
      if (_revision3DialogLocalizationEditErrorIsStale(error.code)) {
        Error.throwWithStackTrace(
          const Revision3DialogLocalizationEditStaleCheckpointException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DialogLocalizationEditRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Publish one exact authored LocalizationEntry text-map edit through the
  /// app-wide project lane. The managed lease rebinds current_project_json and
  /// performs native prepare, full reopen, head CAS, and published reopen.
  Future<Revision3DialogLocalizationEditPublication>
  prepareAndPublishCurrentRevision3DialogLocalizationEdit({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required Revision3DialogLocalizationEditTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'localization editing is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DialogLocalizationEditRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson ||
        plan.expectedHead.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DialogLocalizationEditStaleCheckpointException();
    }
    if (lease is! ManagedRevision3DialogLocalizationEditLease) {
      throw const CurrentProjectOperationUnsupportedException(
        'this managed revision-3 lease cannot edit dialog localization',
      );
    }
    final editor = lease as ManagedRevision3DialogLocalizationEditLease;
    try {
      final publication = await editor
          .prepareAndPublishDialogLocalizationEditV1(plan: plan);
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.localizationId != plan.localizationId ||
          publication.localizationRevision !=
              plan.expectedLocalizationRevision + 1) {
        throw const CurrentProjectCoordinatorException(
          'published localization edit disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } on ManagedRevision3DialogLocalizationEditStaleException catch (
      _,
      stackTrace
    ) {
      Error.throwWithStackTrace(
        const Revision3DialogLocalizationEditStaleCheckpointException(),
        stackTrace,
      );
    } on ModFfiException catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DialogLocalizationEditRequiresReopenException(),
          stackTrace,
        );
      }
      if (_revision3DialogLocalizationEditErrorIsStale(error.code)) {
        Error.throwWithStackTrace(
          const Revision3DialogLocalizationEditStaleCheckpointException(),
          stackTrace,
        );
      }
      if (error.code ==
          'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_VOICE_CONFLICT') {
        Error.throwWithStackTrace(
          const Revision3DialogLocalizationEditLockedVoiceTextException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DialogLocalizationEditRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Verify and expose the generated source for one Quest in the exact visible
  /// managed revision-3 checkpoint. This is a read-only inspection: it neither
  /// prepares nor publishes project state and grants no build/runtime authority.
  Future<AuthoringRevision3QuestSourceInspectionResult>
  inspectCurrentRevision3QuestSource({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required String questId,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Quest source inspection is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3QuestSourceInspectionRequiresReopenException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'a configured game installation is required for Quest source inspection',
      );
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3QuestSourceInspectionStaleCheckpointException();
    }
    try {
      final inspection = await lease.inspectQuestSourceV1(
        gameRoot: gameRoot,
        questId: questId,
      );
      if (inspection.head.canonicalJson != expectedHead.canonicalJson ||
          inspection.projectId != expectedProjectId ||
          inspection.projectRevision != expectedProjectRevision ||
          inspection.questId != questId) {
        throw const Revision3QuestSourceInspectionStaleCheckpointException();
      }
      return inspection;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3QuestSourceInspectionRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Inspect persisted NPC source/readiness evidence in the exact visible
  /// managed revision-3 checkpoint. This project-only read requires no game
  /// installation and grants no compile/build/runtime/spawn authority.
  Future<AuthoringRevision3NpcSourceInspectionResult>
  inspectCurrentRevision3NpcSource({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String npcId,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'NPC source inspection is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3NpcSourceInspectionRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3NpcSourceInspectionStaleCheckpointException();
    }
    try {
      final inspection = await lease.inspectNpcSourceV1(npcId: npcId);
      if (inspection.head.canonicalJson != expectedHead.canonicalJson ||
          inspection.projectId != expectedProjectId ||
          inspection.projectRevision != expectedProjectRevision ||
          inspection.npcId != npcId) {
        throw const Revision3NpcSourceInspectionStaleCheckpointException();
      }
      return inspection;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3NpcSourceInspectionRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Check one exact-current Quest/NPC ScriptModule with the game compiler.
  ///
  /// The selected revisions and module identity are compared with the
  /// coordinator's canonical project snapshot before the lease callback can
  /// run. Native receives only the selected entity ID and derives the source
  /// itself. Returned evidence is compiler-only: it grants no build, runtime,
  /// deployment, publication, or reusable-artifact authority.
  Future<ManagedRevision3CompilerCheckReceipt>
  checkCurrentRevision3ManagedCompiler({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required AuthoringRevision3ManagedCompilerEntityKind entityKind,
    required String entityId,
    required int expectedEntityRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required String gameRoot,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'managed compiler checks are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3ManagedCompilerCheckRequiresReopenException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'managed compiler checks require a game installation',
      );
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson ||
        !revision3ManagedCompilerSelectionMatches(
          currentProjectJson: lease.canonicalProjectJson,
          entityKind: entityKind,
          entityId: entityId,
          expectedEntityRevision: expectedEntityRevision,
          expectedModuleId: expectedModuleId,
          expectedModuleRevision: expectedModuleRevision,
        )) {
      throw const Revision3ManagedCompilerCheckStaleCheckpointException();
    }
    try {
      final receipt = await lease.checkCompilerV1(
        entityKind: entityKind,
        gameRoot: gameRoot,
        entityId: entityId,
        expectedEntityRevision: expectedEntityRevision,
        expectedModuleId: expectedModuleId,
        expectedModuleRevision: expectedModuleRevision,
      );
      final result = receipt.result;
      if (result.head.canonicalJson != expectedHead.canonicalJson ||
          result.project.id != expectedProjectId ||
          result.project.revision != expectedProjectRevision ||
          result.entity.kind != entityKind ||
          result.entity.id != entityId ||
          result.entity.revision != expectedEntityRevision ||
          result.module.id != expectedModuleId ||
          result.module.revision != expectedModuleRevision ||
          receipt.storeStillExactCurrent != !lease.requiresReopen) {
        throw const CurrentProjectCoordinatorException(
          'managed compiler receipt disagrees with the selected exact checkpoint',
        );
      }
      return receipt;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3ManagedCompilerCheckRequiresReopenException(),
          stackTrace,
        );
      }
      if (error is ManagedRevision3CompilerSelectionStaleException) {
        Error.throwWithStackTrace(
          const Revision3ManagedCompilerCheckStaleCheckpointException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Read path-only installed DataAsset package candidates for the exact
  /// visible managed revision-3 checkpoint and its project-pinned executable
  /// generation. The result is metadata-only and grants no extraction, edit,
  /// build, runtime, deployment, or publication authority.
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readCurrentRevision3DataAssetPackageIndex({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'DataAsset package browsing is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DataAssetPackageIndexRequiresReopenException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'a configured game installation is required for DataAsset package browsing',
      );
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DataAssetPackageIndexStaleCheckpointException();
    }
    try {
      final result = await lease.readDataAssetPackageIndexV1(
        gameRoot: gameRoot,
      );
      if (result.head.canonicalJson != expectedHead.canonicalJson ||
          result.projectId != expectedProjectId ||
          result.projectRevision != expectedProjectRevision) {
        throw const Revision3DataAssetPackageIndexStaleCheckpointException();
      }
      return result;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DataAssetPackageIndexRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Inspect one installed DataAsset selected from an exact package-index
  /// snapshot previously returned for the same visible checkpoint. The
  /// original candidate ordinal is retained across UI filtering; no caller
  /// path is accepted as extraction authority.
  Future<AuthoringRevision3InstalledDataAssetInspectionResult>
  inspectCurrentRevision3InstalledDataAsset({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'installed DataAsset inspection is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3InstalledDataAssetInspectionRequiresReopenException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'a configured game installation is required for installed DataAsset inspection',
      );
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson ||
        expectedSnapshot.head.canonicalJson != expectedHead.canonicalJson ||
        expectedSnapshot.projectId != expectedProjectId ||
        expectedSnapshot.projectRevision != expectedProjectRevision ||
        candidate.ordinal < 0 ||
        candidate.ordinal >= expectedSnapshot.index.candidates.length ||
        !identical(
          candidate,
          expectedSnapshot.index.candidates[candidate.ordinal],
        )) {
      throw const Revision3InstalledDataAssetInspectionStaleCheckpointException();
    }
    try {
      final result = await lease.inspectInstalledDataAssetV1(
        gameRoot: gameRoot,
        expectedSnapshot: expectedSnapshot,
        candidate: candidate,
      );
      if (result.head.canonicalJson != expectedHead.canonicalJson ||
          result.projectId != expectedProjectId ||
          result.projectRevision != expectedProjectRevision ||
          result.candidateOrdinal != candidate.ordinal ||
          result.targetPath != candidate.targetPath ||
          result.packageIdHex != candidate.packageIdHex ||
          result.packageIndexSeal.byteLength !=
              expectedSnapshot.packageIndexSeal.byteLength ||
          result.packageIndexSeal.sha256 !=
              expectedSnapshot.packageIndexSeal.sha256 ||
          result.sourceSnapshotSeal.byteLength !=
              expectedSnapshot.sourceSnapshotSeal.byteLength ||
          result.sourceSnapshotSeal.sha256 !=
              expectedSnapshot.sourceSnapshotSeal.sha256) {
        throw const Revision3InstalledDataAssetInspectionStaleCheckpointException();
      }
      return result;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3InstalledDataAssetInspectionRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Publish one offline-only Quest Draft into an exact current managed R3
  /// checkpoint.
  ///
  /// The expected identity is captured by the visible wizard before its fresh
  /// catalog scan. A project switch or intervening revision therefore fails
  /// closed instead of applying the form to another checkpoint.
  Future<Revision3QuestDraftPublication> createCurrentRevision3QuestDraft({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required Revision3QuestDraftAuthoringInput input,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Quest Draft creation is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3QuestDraftRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3QuestDraftStaleCheckpointException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'a configured game installation is required for Quest Draft creation',
      );
    }

    try {
      final publication = await lease.prepareAndPublishQuestDraftV3(
        gameRoot: gameRoot,
        input: input,
      );
      if (publication.projectId != lease.projectId ||
          publication.projectId != expectedProjectId ||
          publication.projectRevision != lease.projectRevision ||
          publication.projectRevision != expectedProjectRevision + 1) {
        throw const CurrentProjectCoordinatorException(
          'published Quest Draft disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3QuestDraftRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Edit one selected exact-current Quest outline without a game root. The
  /// selected Quest/module revisions and visible project checkpoint are all
  /// checked before entering the managed publication lane.
  Future<Revision3QuestOutlineEditPublication>
  editCurrentRevision3QuestOutline({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required Revision3QuestOutlineEditInput input,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Quest outline editing is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3QuestOutlineRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3QuestOutlineStaleCheckpointException();
    }
    try {
      final publication = await lease.prepareAndPublishQuestOutlineEditV1(
        input: input,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.questId != input.questId ||
          publication.moduleId != input.moduleId ||
          publication.questRevision != input.expectedQuestRevision + 1 ||
          publication.moduleRevision != input.expectedModuleRevision + 1) {
        throw const CurrentProjectCoordinatorException(
          'published Quest outline disagrees with the selected managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3QuestOutlineRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Read one effective transition-plan seed from the selected exact-current
  /// Quest. Canonical project JSON remains private to the managed lease.
  Future<AuthoringRevision3QuestTransitionsSeed>
  readCurrentRevision3QuestTransitionsSeed({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Quest transitions are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3QuestTransitionsRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3QuestTransitionsStaleCheckpointException();
    }
    try {
      final seed = await lease.readQuestTransitionsSeedV1(
        questId: questId,
        expectedQuestRevision: expectedQuestRevision,
        expectedModuleId: expectedModuleId,
        expectedModuleRevision: expectedModuleRevision,
      );
      if (seed.projectId != expectedProjectId ||
          seed.projectRevision != expectedProjectRevision ||
          seed.questId != questId ||
          seed.questRevision != expectedQuestRevision ||
          seed.moduleId != expectedModuleId ||
          seed.moduleRevision != expectedModuleRevision) {
        throw const Revision3QuestTransitionsStaleCheckpointException();
      }
      return seed;
    } on FormatException catch (_, stackTrace) {
      Error.throwWithStackTrace(
        const Revision3QuestTransitionsStaleCheckpointException(),
        stackTrace,
      );
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3QuestTransitionsRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Publish one separately reviewed semantic Quest transition plan without a
  /// game root. The visible checkpoint, Quest/module revisions and basis plan
  /// seal are checked before the managed prepare-and-CAS lane is entered.
  Future<Revision3QuestTransitionsEditPublication>
  editCurrentRevision3QuestTransitions({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required Revision3QuestTransitionsEditTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Quest transitions are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3QuestTransitionsRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3QuestTransitionsStaleCheckpointException();
    }
    try {
      final publication = await lease.prepareAndPublishQuestTransitionsEditV1(
        plan: plan,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.questId != plan.questId ||
          publication.moduleId != plan.moduleId ||
          publication.questRevision != plan.expectedQuestRevision + 1 ||
          publication.moduleRevision != plan.expectedModuleRevision + 1 ||
          publication.transitionPlanSeal.byteLength !=
              plan.transitionPlan.contentSeal.byteLength ||
          publication.transitionPlanSeal.sha256 !=
              plan.transitionPlan.contentSeal.sha256) {
        throw const CurrentProjectCoordinatorException(
          'published Quest transitions disagree with the selected managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3QuestTransitionsRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Read the private description and catalog join keys for one selected
  /// exact-current Quest. Canonical project JSON remains inside the lease.
  Future<AuthoringRevision3QuestContextSeed>
  readCurrentRevision3QuestContextSeed({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required String expectedParentRuntimeClass,
    required String expectedGiverRuntimeUniqueName,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Quest context editing is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3QuestContextRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3QuestContextStaleCheckpointException();
    }
    try {
      final seed = await lease.readQuestContextSeedV1(
        questId: questId,
        expectedQuestRevision: expectedQuestRevision,
        expectedModuleId: expectedModuleId,
        expectedModuleRevision: expectedModuleRevision,
        expectedParentRuntimeClass: expectedParentRuntimeClass,
        expectedGiverRuntimeUniqueName: expectedGiverRuntimeUniqueName,
      );
      if (seed.projectId != expectedProjectId ||
          seed.projectRevision != expectedProjectRevision ||
          seed.questId != questId ||
          seed.questRevision != expectedQuestRevision ||
          seed.moduleId != expectedModuleId ||
          seed.moduleRevision != expectedModuleRevision) {
        throw const Revision3QuestContextStaleCheckpointException();
      }
      return seed;
    } on FormatException catch (_, stackTrace) {
      Error.throwWithStackTrace(
        const Revision3QuestContextStaleCheckpointException(),
        stackTrace,
      );
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3QuestContextRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Publish one separately reviewed Quest description/family/giver edit.
  Future<Revision3QuestContextEditPublication>
  editCurrentRevision3QuestContext({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required Revision3QuestContextEditTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Quest context editing is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3QuestContextRequiresReopenException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'a configured game installation is required for Quest context editing',
      );
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3QuestContextStaleCheckpointException();
    }
    try {
      final publication = await lease.prepareAndPublishQuestContextEditV1(
        gameRoot: gameRoot,
        plan: plan,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.questId != plan.questId ||
          publication.moduleId != plan.moduleId ||
          publication.questRevision != plan.expectedQuestRevision + 1 ||
          publication.moduleRevision != plan.expectedModuleRevision + 1) {
        throw const CurrentProjectCoordinatorException(
          'published Quest context disagrees with the selected managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3QuestContextRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Publish one offline logical NPC clone shell into one exact visible
  /// managed revision-3 checkpoint. This never builds, deploys, spawns, or
  /// writes into the game installation.
  Future<Revision3NpcDraftPublication> createCurrentRevision3NpcDraft({
    required String expectedRoot,
    required AuthoringWorkingHead expectedHead,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String gameRoot,
    required Revision3NpcDraftAuthoringInput input,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'NPC Draft creation is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3NpcDraftRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3NpcDraftStaleCheckpointException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'a configured game installation is required for NPC Draft creation',
      );
    }

    try {
      final publication = await lease.prepareAndPublishNpcDraftV1(
        gameRoot: gameRoot,
        input: input,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision) {
        throw const CurrentProjectCoordinatorException(
          'published NPC Draft disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3NpcDraftRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Derive the private exact-basis seed for one visible existing NPC. The
  /// canonical project stays inside the managed lease and this read grants no
  /// catalog, build, runtime, deployment, or publication authority.
  Future<AuthoringRevision3NpcProfileEditSeed>
  readCurrentRevision3NpcProfileEditSeed({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String npcId,
    required int expectedNpcRevision,
    required String expectedScriptModuleId,
    required int expectedScriptModuleRevision,
    required String expectedUniqueName,
    required String expectedModuleNamespace,
    required String expectedParentCharacterDefinition,
    required String expectedParentAiAgentConfig,
    required String expectedParentSpawnDefinition,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'NPC profile editing is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3NpcProfileEditRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3NpcProfileEditStaleCheckpointException();
    }
    if (lease is! ManagedRevision3NpcProfileEditLease) {
      throw const Revision3NpcProfileEditUnavailableException();
    }
    final editor = lease as ManagedRevision3NpcProfileEditLease;
    if (!editor.supportsNpcProfileEdit) {
      throw const Revision3NpcProfileEditUnavailableException();
    }
    try {
      final seed = await editor.readNpcProfileEditSeedV1(
        npcId: npcId,
        expectedNpcRevision: expectedNpcRevision,
        expectedScriptModuleId: expectedScriptModuleId,
        expectedScriptModuleRevision: expectedScriptModuleRevision,
        expectedUniqueName: expectedUniqueName,
        expectedModuleNamespace: expectedModuleNamespace,
        expectedParentCharacterDefinition: expectedParentCharacterDefinition,
        expectedParentAiAgentConfig: expectedParentAiAgentConfig,
        expectedParentSpawnDefinition: expectedParentSpawnDefinition,
      );
      if (seed.head.canonicalJson != expectedHead.canonicalJson ||
          seed.projectId != expectedProjectId ||
          seed.projectRevision != expectedProjectRevision ||
          seed.npcId != npcId ||
          seed.npcRevision != expectedNpcRevision ||
          seed.scriptModuleId != expectedScriptModuleId ||
          seed.scriptModuleRevision != expectedScriptModuleRevision ||
          seed.uniqueName != expectedUniqueName ||
          seed.moduleNamespace != expectedModuleNamespace ||
          seed.parentCharacterDefinition.runtimeClass !=
              expectedParentCharacterDefinition ||
          seed.parentAiAgentConfig.runtimeClass !=
              expectedParentAiAgentConfig ||
          seed.parentSpawnDefinition.runtimeClass !=
              expectedParentSpawnDefinition) {
        throw const Revision3NpcProfileEditStaleCheckpointException();
      }
      return seed;
    } on FormatException catch (_, stackTrace) {
      Error.throwWithStackTrace(
        const Revision3NpcProfileEditStaleCheckpointException(),
        stackTrace,
      );
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3NpcProfileEditRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Publish one separately reviewed existing-NPC name/archetype edit against
  /// the exact visible managed checkpoint. Any ambiguous post-call result
  /// permanently removes mutation authority until recovery or reopen.
  Future<Revision3NpcProfileEditPublication> editCurrentRevision3NpcProfile({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required Revision3NpcProfileEditTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'NPC profile editing is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3NpcProfileEditRequiresReopenException();
    }
    if (gameRoot.isEmpty) {
      throw const Revision3NpcProfileEditUnavailableException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson ||
        plan.projectId != expectedProjectId ||
        plan.projectRevision != expectedProjectRevision ||
        plan.seed.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3NpcProfileEditStaleCheckpointException();
    }
    if (lease is! ManagedRevision3NpcProfileEditLease) {
      throw const Revision3NpcProfileEditUnavailableException();
    }
    final editor = lease as ManagedRevision3NpcProfileEditLease;
    if (!editor.supportsNpcProfileEdit) {
      throw const Revision3NpcProfileEditUnavailableException();
    }
    try {
      final publication = await editor.prepareAndPublishNpcProfileEditV1(
        gameRoot: gameRoot,
        plan: plan,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.npcId != plan.npcId ||
          publication.npcRevision != plan.expectedNpcRevision + 1 ||
          publication.scriptModuleId != plan.scriptModuleId ||
          publication.scriptModuleRevision !=
              plan.expectedScriptModuleRevision +
                  (plan.moduleRegenerated ? 1 : 0) ||
          publication.displayName != plan.displayName ||
          publication.previousParentCatalogId != plan.expectedParentCatalogId ||
          publication.parentCatalogId != plan.parentCatalogId ||
          publication.nameChanged != plan.nameChanged ||
          publication.archetypeChanged != plan.archetypeChanged ||
          publication.moduleRegenerated != plan.moduleRegenerated) {
        editor.markRequiresReopenAfterNpcProfileEditUncertainty();
        throw const Revision3NpcProfileEditRequiresReopenException();
      }
      return publication;
    } catch (error, stackTrace) {
      if (error is Revision3NpcProfileEditRequiresReopenException) {
        if (!lease.requiresReopen) {
          editor.markRequiresReopenAfterNpcProfileEditUncertainty();
        }
        Error.throwWithStackTrace(error, stackTrace);
      }
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3NpcProfileEditRequiresReopenException(),
          stackTrace,
        );
      }
      if ((error is ModFfiException &&
              _revision3NpcProfileEditCorrectableCodes.contains(error.code)) ||
          error is FormatException ||
          error is Revision3NpcProfileEditStaleCheckpointException) {
        Error.throwWithStackTrace(
          const Revision3NpcProfileEditStaleCheckpointException(),
          stackTrace,
        );
      }
      editor.markRequiresReopenAfterNpcProfileEditUncertainty();
      Error.throwWithStackTrace(
        const Revision3NpcProfileEditRequiresReopenException(),
        stackTrace,
      );
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Create one project-local DialogLine prerequisite without touching a game
  /// installation or save. The plan comes from a fresh exact ContentIndex and
  /// is rebound to the managed head inside the serialized session lane.
  Future<Revision3DialogLineEntryPublication> createCurrentRevision3DialogLine({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required Revision3DialogLineEntryTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'dialog-line authoring is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DialogLineEntryRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DialogLineEntryStaleCheckpointException();
    }

    try {
      final publication = await lease.prepareAndPublishDialogLineV1(plan: plan);
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.lineId != plan.lineId ||
          publication.localizationId != plan.localization.localizationId ||
          publication.voiceSlotId != plan.voiceSlot?.slotId ||
          publication.locale != plan.locale) {
        throw const CurrentProjectCoordinatorException(
          'published dialog line disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DialogLineEntryRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Import one Ogg-backed Voice take into one exact current managed R3
  /// checkpoint. The plan comes from a freshly projected ContentIndex and
  /// contains no build, deployment, game-write, save-write, or runtime claim.
  /// The configured game root is only a forbidden Store-root safety boundary,
  /// not a catalog/content input.
  Future<Revision3VoiceTakePublication> addCurrentRevision3VoiceTake({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required Revision3VoiceTakeTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Voice takes are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3VoiceTakeRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3VoiceTakeStaleCheckpointException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'a configured game installation is required for Voice take authoring',
      );
    }

    try {
      final publication = await lease.prepareAndPublishVoiceTakeV1(
        gameRoot: gameRoot,
        plan: plan,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.lineId != plan.lineId ||
          publication.slotId != plan.slotId ||
          publication.takeId != plan.takeId ||
          publication.slotCreated != plan.expectsSlotCreated ||
          publication.selected != plan.selectTake) {
        throw const CurrentProjectCoordinatorException(
          'published Voice take disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3VoiceTakeRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Select one existing Approved Voice take, or clear an exact current slot,
  /// without requiring or reading a game installation.
  Future<Revision3VoiceTakeSelectionPublication>
  selectCurrentRevision3VoiceTake({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required Revision3VoiceTakeSelectionTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Voice take selection is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3VoiceTakeSelectionRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3VoiceTakeSelectionStaleCheckpointException();
    }
    try {
      final publication = await lease.prepareAndPublishVoiceTakeSelectionV1(
        plan: plan,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.lineId != plan.lineId ||
          publication.slotId != plan.slotId ||
          publication.slotRevision != plan.expectedSlotRevision + 1 ||
          publication.locale != plan.locale ||
          publication.locId != plan.locId ||
          publication.previousSelectedTakeId != plan.expectedSelectedTakeId ||
          publication.selectedTakeId != plan.selectedTakeId) {
        throw const CurrentProjectCoordinatorException(
          'published Voice take selection disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3VoiceTakeSelectionRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Detach one exact listed Voice take from one exact line/language slot.
  /// The operation is project-only, preserves immutable audio CAS metadata,
  /// and never gains game, build, runtime, deployment, or save authority.
  Future<Revision3VoiceTakeRemovalPublication> removeCurrentRevision3VoiceTake({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required Revision3VoiceTakeRemovalTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Voice take removal is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3VoiceTakeRemovalRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3VoiceTakeRemovalStaleCheckpointException();
    }
    if (lease is! ManagedRevision3VoiceTakeRemovalLease) {
      throw const CurrentProjectOperationUnsupportedException(
        'the managed project lease does not support Voice take removal',
      );
    }
    final removalLease = lease as ManagedRevision3VoiceTakeRemovalLease;
    if (!removalLease.supportsVoiceTakeRemoval) {
      throw const CurrentProjectOperationUnsupportedException(
        'the managed project Store does not support Voice take removal',
      );
    }
    try {
      final publication = await removalLease
          .prepareAndPublishVoiceTakeRemovalV1(plan: plan);
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.lineId != plan.lineId ||
          publication.localizationId != plan.localizationId ||
          publication.slotId != plan.slotId ||
          publication.slotRevision != plan.expectedSlotRevision + 1 ||
          publication.locale != plan.locale ||
          publication.locId != plan.locId ||
          publication.takeId != plan.takeId ||
          publication.takeRevision != plan.expectedTakeRevision ||
          publication.previousSelectedTakeId != plan.expectedSelectedTakeId ||
          publication.selectionCleared != plan.expectsSelectionCleared ||
          publication.takeEntityRemoved != plan.expectedTakeEntityRemoved ||
          publication.remainingCandidateCount !=
              plan.expectedRemainingCandidateCount) {
        removalLease.markRequiresReopenAfterVoiceTakeRemovalUncertainty();
        throw const Revision3VoiceTakeRemovalRequiresReopenException();
      }
      return publication;
    } catch (error, stackTrace) {
      if (error is Revision3VoiceTakeRemovalRequiresReopenException) {
        if (!lease.requiresReopen) {
          removalLease.markRequiresReopenAfterVoiceTakeRemovalUncertainty();
        }
        Error.throwWithStackTrace(error, stackTrace);
      }
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3VoiceTakeRemovalRequiresReopenException(),
          stackTrace,
        );
      }
      if (error is ModFfiException &&
          _revision3VoiceTakeRemovalCorrectableCodes.contains(error.code)) {
        Error.throwWithStackTrace(
          const Revision3VoiceTakeRemovalStaleCheckpointException(),
          stackTrace,
        );
      }
      if (error is Revision3VoiceTakeRemovalStaleCheckpointException) {
        Error.throwWithStackTrace(
          const Revision3VoiceTakeRemovalStaleCheckpointException(),
          stackTrace,
        );
      }
      removalLease.markRequiresReopenAfterVoiceTakeRemovalUncertainty();
      Error.throwWithStackTrace(
        const Revision3VoiceTakeRemovalRequiresReopenException(),
        stackTrace,
      );
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Remove one exact empty and unselected dialog Voice slot. Correctable
  /// prepublication conflicts become stale checkpoints; uncertain outcomes
  /// permanently remove mutation authority until recovery or reopen.
  Future<Revision3DialogVoiceSlotRemovalPublication>
  removeCurrentRevision3DialogVoiceSlot({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required Revision3DialogVoiceSlotRemovalTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'dialog Voice slot removal is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DialogVoiceSlotRemovalRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DialogVoiceSlotRemovalStaleCheckpointException();
    }
    if (lease is! ManagedRevision3DialogVoiceSlotRemovalLease) {
      throw const CurrentProjectOperationUnsupportedException(
        'the managed project lease does not support dialog Voice slot removal',
      );
    }
    final removalLease = lease as ManagedRevision3DialogVoiceSlotRemovalLease;
    if (!removalLease.supportsDialogVoiceSlotRemoval) {
      throw const CurrentProjectOperationUnsupportedException(
        'the managed project Store does not support dialog Voice slot removal',
      );
    }
    try {
      final publication = await removalLease
          .prepareAndPublishDialogVoiceSlotRemovalV1(plan: plan);
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.lineId != plan.lineId ||
          publication.lineRevision != plan.expectedLineRevision + 1 ||
          publication.localizationId != plan.localizationId ||
          publication.slotId != plan.slotId ||
          publication.removedSlotRevision != plan.expectedSlotRevision ||
          publication.locale != plan.locale ||
          publication.locId != plan.locId ||
          publication.removedTargetResolution != plan.targetResolution) {
        removalLease.markRequiresReopenAfterDialogVoiceSlotRemovalUncertainty();
        throw const Revision3DialogVoiceSlotRemovalRequiresReopenException();
      }
      return publication;
    } catch (error, stackTrace) {
      if (error is Revision3DialogVoiceSlotRemovalRequiresReopenException) {
        if (!lease.requiresReopen) {
          removalLease
              .markRequiresReopenAfterDialogVoiceSlotRemovalUncertainty();
        }
        Error.throwWithStackTrace(error, stackTrace);
      }
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DialogVoiceSlotRemovalRequiresReopenException(),
          stackTrace,
        );
      }
      if (error is ModFfiException &&
          _revision3DialogVoiceSlotRemovalCorrectableCodes.contains(
            error.code,
          )) {
        Error.throwWithStackTrace(
          const Revision3DialogVoiceSlotRemovalStaleCheckpointException(),
          stackTrace,
        );
      }
      if (error is Revision3DialogVoiceSlotRemovalStaleCheckpointException) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      removalLease.markRequiresReopenAfterDialogVoiceSlotRemovalUncertainty();
      Error.throwWithStackTrace(
        const Revision3DialogVoiceSlotRemovalRequiresReopenException(),
        stackTrace,
      );
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Change one retained take's author-managed review status against an exact
  /// visible Voice checkpoint. This grants no game, media, build, deployment,
  /// or runtime authority.
  Future<Revision3VoiceTakeStatusPublication>
  editCurrentRevision3VoiceTakeStatus({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required Revision3VoiceTakeStatusTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Voice take status editing is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3VoiceTakeStatusRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3VoiceTakeStatusStaleCheckpointException();
    }
    if (lease is! ManagedRevision3VoiceTakeStatusLease) {
      throw const CurrentProjectOperationUnsupportedException(
        'the managed project lease does not support Voice take status editing',
      );
    }
    final editor = lease as ManagedRevision3VoiceTakeStatusLease;
    try {
      final publication = await editor.prepareAndPublishVoiceTakeStatusV1(
        plan: plan,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.lineId != plan.lineId ||
          publication.localizationId != plan.localizationId ||
          publication.slotId != plan.slotId ||
          publication.slotRevision != plan.expectedSlotRevision ||
          publication.locale != plan.locale ||
          publication.locId != plan.locId ||
          publication.takeId != plan.takeId ||
          publication.takeRevision != plan.expectedTakeRevision + 1 ||
          publication.previousStatus != plan.expectedStatus ||
          publication.status != plan.desiredStatus) {
        editor.markRequiresReopenAfterPublicationUncertainty();
        throw const Revision3VoiceTakeStatusRequiresReopenException();
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3VoiceTakeStatusRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Resolve one installed-archive target for an exact current Voice slot and
  /// publish only the sealed evidence checkpoint.
  Future<Revision3VoiceTargetPublication> resolveCurrentRevision3VoiceTarget({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required Revision3VoiceTargetTechnicalPlan plan,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Voice target resolution is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3VoiceTargetRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3VoiceTargetStaleCheckpointException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'a configured game installation is required for Voice target resolution',
      );
    }
    try {
      final publication = await lease.prepareAndPublishVoiceTargetV1(
        gameRoot: gameRoot,
        plan: plan,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.lineId != plan.lineId ||
          publication.slotId != plan.slotId ||
          publication.locale != plan.locale ||
          publication.locId != plan.locId) {
        throw const CurrentProjectCoordinatorException(
          'published Voice target disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3VoiceTargetRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Build the exact current Voice graph into a new offline bundle. The
  /// managed checkpoint remains unchanged whether the result is built or
  /// structurally blocked.
  Future<AuthoringRevision3VoiceBuildResult> buildCurrentRevision3Voice({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required String output,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Voice bundle build is available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3VoiceBuildRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3VoiceBuildStaleCheckpointException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'a configured game installation is required to keep Voice build output outside the game',
      );
    }
    try {
      final result = await lease.buildVoiceV1(
        gameRoot: gameRoot,
        output: output,
      );
      if (result.projectId != expectedProjectId ||
          result.projectId != lease.projectId ||
          result.projectRevision != expectedProjectRevision ||
          result.projectRevision != lease.projectRevision ||
          result.basisHead.canonicalJson != expectedHead.canonicalJson ||
          lease.head.canonicalJson != expectedHead.canonicalJson) {
        throw const CurrentProjectCoordinatorException(
          'Voice build disagrees with the current managed checkpoint',
        );
      }
      return result;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3VoiceBuildRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Export one immutable portable copy from the exact visible managed
  /// checkpoint. The current project remains open and unchanged; publication
  /// uncertainty is a sealed terminal result and must not be retried.
  Future<AuthoringRevision3ExactSnapshotExportResult>
  exportCurrentRevision3ExactSnapshot({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String output,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const Revision3ProjectExportUnsupportedException();
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3ProjectExportRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3ProjectExportStaleCheckpointException();
    }
    if (lease is! ManagedRevision3ProjectExportLease) {
      throw const Revision3ProjectExportUnsupportedException();
    }
    final exportLease = lease as ManagedRevision3ProjectExportLease;
    if (!exportLease.supportsExactSnapshotExport) {
      throw const Revision3ProjectExportUnsupportedException();
    }
    try {
      final result = await exportLease.exportExactSnapshotV1(output: output);
      if (result.basisHead.canonicalJson != expectedHead.canonicalJson ||
          result.projectId != expectedProjectId ||
          result.projectId != lease.projectId ||
          result.projectRevision != expectedProjectRevision ||
          result.projectRevision != lease.projectRevision ||
          result.output != output ||
          lease.head.canonicalJson != expectedHead.canonicalJson) {
        exportLease.markRequiresReopenAfterPublicationUncertainty();
        throw const Revision3ProjectExportRequiresReopenException(
          publicationMayExist: true,
        );
      }
      return result;
    } catch (error, stackTrace) {
      if (error is Revision3ProjectExportRequiresReopenException) {
        if (!lease.requiresReopen) {
          exportLease.markRequiresReopenAfterPublicationUncertainty();
        }
        Error.throwWithStackTrace(error, stackTrace);
      }
      if (error is Revision3ProjectExportFailedException) {
        if (!error.publicationMayExist) {
          Error.throwWithStackTrace(error, stackTrace);
        }
        if (!lease.requiresReopen) {
          exportLease.markRequiresReopenAfterPublicationUncertainty();
        }
        Error.throwWithStackTrace(
          Revision3ProjectExportRequiresReopenException(
            publicationMayExist: true,
            code: error.code,
            cause: error,
          ),
          stackTrace,
        );
      }
      if (error is ManagedProjectHeadConflictException) {
        Error.throwWithStackTrace(
          Revision3ProjectExportRequiresReopenException(
            publicationMayExist: false,
            cause: error,
          ),
          stackTrace,
        );
      }
      if (error is ManagedRevision3ExactSnapshotExportPrepublicationException) {
        Error.throwWithStackTrace(
          Revision3ProjectExportRequiresReopenException(
            publicationMayExist: false,
            code: error.code,
            cause: error,
          ),
          stackTrace,
        );
      }
      if (error is ModFfiException &&
          _revision3ProjectExportSafePrepublicationCodes.contains(error.code)) {
        Error.throwWithStackTrace(
          Revision3ProjectExportFailedException(
            cause: error,
            code: error.code,
            publicationMayExist: false,
            retryWithNewDestination:
                _revision3ProjectExportDestinationRetryCodes.contains(
                  error.code,
                ),
          ),
          stackTrace,
        );
      }
      if (error is ArgumentError) {
        Error.throwWithStackTrace(
          Revision3ProjectExportFailedException(
            cause: error,
            publicationMayExist: false,
          ),
          stackTrace,
        );
      }
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          Revision3ProjectExportRequiresReopenException(
            publicationMayExist: true,
            cause: error,
          ),
          stackTrace,
        );
      }
      exportLease.markRequiresReopenAfterPublicationUncertainty();
      Error.throwWithStackTrace(
        Revision3ProjectExportRequiresReopenException(
          publicationMayExist: true,
          cause: error,
        ),
        stackTrace,
      );
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Build one reviewed DataAsset stage from the exact visible managed
  /// checkpoint. The Store is not mutated; every terminal publication outcome,
  /// including uncertainty after rename, remains a successful sealed result.
  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  buildCurrentRevision3ReviewedDataAsset({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required String targetPath,
    required String packName,
    required String output,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'reviewed DataAsset builds are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DataAssetRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DataAssetStaleCheckpointException();
    }
    if (gameRoot.isEmpty) {
      throw const CurrentProjectOperationUnsupportedException(
        'a configured game installation is required for reviewed DataAsset builds',
      );
    }
    if (lease is! ManagedRevision3ReviewedDataAssetBuildLease) {
      throw const CurrentProjectOperationUnsupportedException(
        'this managed project session cannot build reviewed DataAssets',
      );
    }
    final buildLease = lease as ManagedRevision3ReviewedDataAssetBuildLease;
    if (!buildLease.supportsReviewedDataAssetBuild) {
      throw const CurrentProjectOperationUnsupportedException(
        'this managed project session cannot build reviewed DataAssets',
      );
    }
    try {
      final result = await buildLease.buildReviewedDataAssetV1(
        gameRoot: gameRoot,
        targetPath: targetPath,
        packName: packName,
        output: output,
      );
      if (result.basisHead.canonicalJson != expectedHead.canonicalJson ||
          result.projectId != expectedProjectId ||
          result.projectId != lease.projectId ||
          result.projectRevision != expectedProjectRevision ||
          result.projectRevision != lease.projectRevision ||
          result.targetPath != targetPath ||
          result.packName != packName ||
          result.output != output ||
          lease.head.canonicalJson != expectedHead.canonicalJson) {
        throw const CurrentProjectCoordinatorException(
          'reviewed DataAsset build disagrees with the current managed checkpoint or intent',
        );
      }
      return result;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DataAssetRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Read the receipt-verified DataAsset stage registry for one exact visible
  /// managed revision-3 checkpoint.
  Future<List<AuthoringRevision3DataAssetStage>>
  listCurrentRevision3DataAssetStages({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'DataAsset edits are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DataAssetRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DataAssetStaleCheckpointException();
    }
    try {
      final stages = await lease.listDataAssetStagesV1();
      if (stages.any(
        (stage) =>
            stage.projectId != expectedProjectId ||
            stage.stagedProjectRevision > expectedProjectRevision,
      )) {
        throw const CurrentProjectCoordinatorException(
          'DataAsset stage list disagrees with the current managed checkpoint',
        );
      }
      return List<AuthoringRevision3DataAssetStage>.unmodifiable(stages);
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DataAssetRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Import one independently receipt-verified fixed-leaf edit into the exact
  /// current project checkpoint. No build, deployment, or game write occurs.
  Future<Revision3DataAssetStagePublication> addCurrentRevision3DataAssetStage({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String patchReceiptPath,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'DataAsset edits are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DataAssetRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DataAssetStaleCheckpointException();
    }
    if (patchReceiptPath.trim().isEmpty) {
      throw ArgumentError.value(
        patchReceiptPath,
        'patchReceiptPath',
        'must identify a verified DataAsset edit receipt',
      );
    }
    try {
      final publication = await lease.prepareAndPublishDataAssetStageV1(
        patchReceiptPath: patchReceiptPath,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.stage.projectId != expectedProjectId ||
          publication.stage.stagedProjectRevision !=
              publication.projectRevision) {
        throw const CurrentProjectCoordinatorException(
          'published DataAsset edit disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DataAssetRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Create one typed value edit from an inspected fixed-leaf selector and
  /// its exact ExtractReceipt-v2. The managed lease performs native semantic
  /// encoding and proof verification before the same exact-head publication
  /// lane used by receipt imports. No game or save file is written.
  Future<Revision3DataAssetStagePublication> addCurrentRevision3DataAssetEdit({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required DataAssetSemanticEditIntent intent,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'DataAsset edits are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DataAssetRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DataAssetStaleCheckpointException();
    }
    try {
      final publication = await lease.prepareAndPublishDataAssetEditV1(
        intent: intent,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.stage.projectId != expectedProjectId ||
          publication.stage.targetPath != intent.expectedTargetPath ||
          publication.stage.stagedProjectRevision !=
              publication.projectRevision) {
        throw const CurrentProjectCoordinatorException(
          'published DataAsset value edit disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DataAssetRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Promote one exact installed-package inspection into a typed managed
  /// value edit. Native code revalidates the package index, USMAP inventory,
  /// independent live reconstruction, and fixed head before publication.
  Future<Revision3DataAssetStagePublication>
  addCurrentRevision3InstalledDataAssetEdit({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required DataAssetInstalledSemanticEditIntent intent,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Installed DataAsset edits are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DataAssetRequiresReopenException();
    }
    final snapshot = intent.snapshot;
    final inspection = intent.inspection;
    if (gameRoot.isEmpty ||
        lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson ||
        snapshot.head.canonicalJson != expectedHead.canonicalJson ||
        snapshot.projectId != expectedProjectId ||
        snapshot.projectRevision != expectedProjectRevision ||
        inspection.head.canonicalJson != expectedHead.canonicalJson ||
        inspection.projectId != expectedProjectId ||
        inspection.projectRevision != expectedProjectRevision) {
      throw const Revision3DataAssetStaleCheckpointException();
    }
    try {
      final publication = await lease.prepareAndPublishInstalledDataAssetEditV1(
        gameRoot: gameRoot,
        intent: intent,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.stage.projectId != expectedProjectId ||
          publication.stage.targetPath != intent.expectedTargetPath ||
          publication.stage.stagedProjectRevision !=
              publication.projectRevision) {
        throw const CurrentProjectCoordinatorException(
          'published installed DataAsset edit disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DataAssetRequiresReopenException(),
          stackTrace,
        );
      }
      if (error is ModFfiException) {
        if (_revision3InstalledDataAssetEditSourceEvidenceIsStale(error.code)) {
          Error.throwWithStackTrace(
            const Revision3InstalledDataAssetEditSourceEvidenceStaleException(),
            stackTrace,
          );
        }
        Error.throwWithStackTrace(
          Revision3InstalledDataAssetEditRejectedException(
            error.code == 'AUTHORING_REVISION3_DATAASSET_TARGET_EXISTS'
                ? Revision3InstalledDataAssetEditRejectionReason
                      .targetAlreadyStaged
                : Revision3InstalledDataAssetEditRejectionReason
                      .preparationFailed,
          ),
          stackTrace,
        );
      }
      if (error is ArgumentError || error is FormatException) {
        Error.throwWithStackTrace(
          const Revision3InstalledDataAssetEditRejectedException(
            Revision3InstalledDataAssetEditRejectionReason.preparationFailed,
          ),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Publish one closed reviewed installed DataAsset edit. The UI supplies
  /// only semantic schema values; native code rediscovers selector and bytes.
  Future<Revision3DataAssetStagePublication>
  addCurrentRevision3ReviewedInstalledDataAssetEdit({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String gameRoot,
    required ReviewedInstalledDataAssetEditIntent intent,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'Reviewed installed DataAsset edits are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DataAssetRequiresReopenException();
    }
    final snapshot = intent.snapshot;
    final inspection = intent.inspection;
    if (gameRoot.isEmpty ||
        lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson ||
        snapshot.head.canonicalJson != expectedHead.canonicalJson ||
        snapshot.projectId != expectedProjectId ||
        snapshot.projectRevision != expectedProjectRevision ||
        inspection.head.canonicalJson != expectedHead.canonicalJson ||
        inspection.projectId != expectedProjectId ||
        inspection.projectRevision != expectedProjectRevision) {
      throw const Revision3DataAssetStaleCheckpointException();
    }
    try {
      final publication = await lease
          .prepareAndPublishReviewedInstalledDataAssetEditV1(
            gameRoot: gameRoot,
            intent: intent,
          );
      if (publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.stage.projectId != expectedProjectId ||
          publication.stage.targetPath != intent.expectedTargetPath ||
          publication.stage.stagedProjectRevision !=
              publication.projectRevision) {
        throw const CurrentProjectCoordinatorException(
          'published reviewed installed DataAsset edit disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DataAssetRequiresReopenException(),
          stackTrace,
        );
      }
      if (error is ModFfiException) {
        if (_revision3InstalledDataAssetEditSourceEvidenceIsStale(error.code)) {
          Error.throwWithStackTrace(
            const Revision3InstalledDataAssetEditSourceEvidenceStaleException(),
            stackTrace,
          );
        }
        Error.throwWithStackTrace(
          Revision3InstalledDataAssetEditRejectedException(
            error.code == 'AUTHORING_REVISION3_DATAASSET_TARGET_EXISTS'
                ? Revision3InstalledDataAssetEditRejectionReason
                      .targetAlreadyStaged
                : Revision3InstalledDataAssetEditRejectionReason
                      .preparationFailed,
          ),
          stackTrace,
        );
      }
      if (error is ArgumentError || error is FormatException) {
        Error.throwWithStackTrace(
          const Revision3InstalledDataAssetEditRejectedException(
            Revision3InstalledDataAssetEditRejectionReason.preparationFailed,
          ),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  bool _revision3InstalledDataAssetEditSourceEvidenceIsStale(String code) =>
      code.startsWith('AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_') ||
      const {
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_CANDIDATE_INVALID',
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INSPECTION_BINDING_MISMATCH',
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INSPECTION_FAILED',
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INVALID',
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_PACKAGE_INDEX_MISMATCH',
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_SELECTOR_MISMATCH',
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_SOURCE_SNAPSHOT_MISMATCH',
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_USMAP_CONTENT_MISMATCH',
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_USMAP_INVENTORY_MISMATCH',
        'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_CANDIDATE_INVALID',
        'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_MATCH_INVALID',
        'AUTHORING_REVISION3_DATAASSET_EXECUTABLE_MISMATCH',
        'AUTHORING_REVISION3_DATAASSET_INPUT_INVALID',
        'AUTHORING_REVISION3_DATAASSET_INPUT_MISSING',
        'AUTHORING_REVISION3_DATAASSET_INPUT_UNSAFE',
      }.contains(code);

  /// Remove one exact NPC/Quest Draft and only its uniquely-owned generated
  /// ScriptModule through the managed full-reopen and fixed-head CAS lane.
  Future<Revision3StoryDraftRemovalPublication>
  removeCurrentRevision3StoryDraft({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String draftId,
    required AuthoringStoryDraftKind draftKind,
    required int expectedDraftRevision,
    required String scriptModuleId,
    required int expectedScriptModuleRevision,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const Revision3StoryDraftRemovalUnsupportedException();
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3StoryDraftRemovalRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3StoryDraftRemovalStaleCheckpointException();
    }
    if (lease is! ManagedRevision3StoryDraftRemovalLease) {
      throw const Revision3StoryDraftRemovalUnsupportedException();
    }
    final removalLease = lease as ManagedRevision3StoryDraftRemovalLease;
    if (!removalLease.supportsStoryDraftRemoval) {
      throw const Revision3StoryDraftRemovalUnsupportedException();
    }
    if (!_managedRevision3RecoveryProjectIdPattern.hasMatch(draftId) ||
        draftId == _managedRevision3RecoveryZeroProjectId ||
        !_managedRevision3RecoveryProjectIdPattern.hasMatch(scriptModuleId) ||
        scriptModuleId == _managedRevision3RecoveryZeroProjectId ||
        draftId == scriptModuleId ||
        expectedProjectRevision < 0 ||
        expectedProjectRevision >= 0x7fffffffffffffff ||
        expectedDraftRevision < 0 ||
        expectedDraftRevision > 0x7fffffffffffffff ||
        expectedScriptModuleRevision < 0 ||
        expectedScriptModuleRevision > 0x7fffffffffffffff) {
      final cause = ArgumentError(
        'Story Draft removal requires distinct nonzero IDs and signed-safe revisions',
      );
      throw Revision3StoryDraftRemovalRejectedException(
        code: null,
        cause: cause,
      );
    }
    try {
      final publication = await removalLease
          .prepareAndPublishRemoveStoryDraftV1(
            draftId: draftId,
            draftKind: draftKind,
            expectedDraftRevision: expectedDraftRevision,
            scriptModuleId: scriptModuleId,
            expectedScriptModuleRevision: expectedScriptModuleRevision,
          );
      if (publication.head.canonicalJson == expectedHead.canonicalJson ||
          publication.head.canonicalJson != lease.head.canonicalJson ||
          publication.projectId != expectedProjectId ||
          publication.projectId != lease.projectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.projectRevision != lease.projectRevision ||
          publication.removedDraftId != draftId ||
          publication.removedDraftKind != draftKind ||
          publication.removedDraftRevision != expectedDraftRevision ||
          publication.removedScriptModuleId != scriptModuleId ||
          publication.removedScriptModuleRevision !=
              expectedScriptModuleRevision) {
        removalLease.markRequiresReopenAfterStoryDraftRemovalUncertainty();
        throw const Revision3StoryDraftRemovalRequiresReopenException();
      }
      return publication;
    } catch (error, stackTrace) {
      if (error is Revision3StoryDraftRemovalRequiresReopenException) {
        if (!lease.requiresReopen) {
          removalLease.markRequiresReopenAfterStoryDraftRemovalUncertainty();
        }
        Error.throwWithStackTrace(error, stackTrace);
      }
      if (error is ModFfiException &&
          !lease.requiresReopen &&
          _revision3StoryDraftRemovalCorrectableCodes.contains(error.code)) {
        Error.throwWithStackTrace(
          Revision3StoryDraftRemovalRejectedException(
            code: error.code,
            cause: error,
          ),
          stackTrace,
        );
      }
      if (error is ArgumentError && !lease.requiresReopen) {
        Error.throwWithStackTrace(
          Revision3StoryDraftRemovalRejectedException(code: null, cause: error),
          stackTrace,
        );
      }
      if (!lease.requiresReopen) {
        removalLease.markRequiresReopenAfterStoryDraftRemovalUncertainty();
      }
      Error.throwWithStackTrace(
        Revision3StoryDraftRemovalRequiresReopenException(cause: error),
        stackTrace,
      );
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Remove one exact listed DataAsset stage from the project registry. This
  /// neither deletes source artifacts nor writes to the game installation.
  Future<Revision3DataAssetStageRemovalPublication>
  removeCurrentRevision3DataAssetStage({
    required String expectedRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
  }) => _enqueue(() async {
    final current = _current;
    if (current == null) throw const NoCurrentProjectException();
    if (current is! _OwnedManagedRevision3CurrentProject) {
      throw const CurrentProjectOperationUnsupportedException(
        'DataAsset edits are available only for managed revision-3 projects',
      );
    }
    final lease = current.lease;
    if (lease.requiresReopen) {
      throw const Revision3DataAssetRequiresReopenException();
    }
    if (lease.root.path != expectedRoot ||
        lease.projectId != expectedProjectId ||
        lease.projectRevision != expectedProjectRevision ||
        lease.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3DataAssetStaleCheckpointException();
    }
    if (targetPath.isEmpty) {
      throw ArgumentError.value(
        targetPath,
        'targetPath',
        'must identify a listed DataAsset edit',
      );
    }
    try {
      final publication = await lease.prepareAndPublishRemoveDataAssetStageV1(
        targetPath: targetPath,
      );
      if (publication.projectId != expectedProjectId ||
          publication.projectRevision != expectedProjectRevision + 1 ||
          publication.removed.projectId != expectedProjectId ||
          publication.removed.targetPath.toLowerCase() !=
              targetPath.toLowerCase()) {
        throw const CurrentProjectCoordinatorException(
          'removed DataAsset edit disagrees with the current managed checkpoint',
        );
      }
      return publication;
    } catch (error, stackTrace) {
      if (lease.requiresReopen) {
        Error.throwWithStackTrace(
          const Revision3DataAssetRequiresReopenException(),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      _refreshCurrentIfUnchanged(current);
    }
  });

  /// Detach and close the current lease in the operation lane.
  Future<void> closeCurrent() => _enqueue(() async {
    final current = _current;
    if (current == null) return;
    _current = null;
    _publish(const NoCurrentProjectState());
    try {
      await _closeOwned(current);
    } catch (error, stackTrace) {
      _recordCleanupFailure(current, error, stackTrace);
      Error.throwWithStackTrace(error, stackTrace);
    }
  });

  /// Stop accepting work, drain accepted transitions, and close every lease.
  /// Idempotent and safe to await in tests or an orderly application shutdown.
  Future<void> shutdown() {
    final existing = _shutdownFuture;
    if (existing != null) return existing;
    _shutdownRequested = true;
    final result = _tail.then((_) async {
      final closing = _current;
      _current = null;
      _publish(const NoCurrentProjectState());
      if (closing != null) {
        try {
          await _closeOwned(closing);
        } catch (error, stackTrace) {
          _recordCleanupFailure(closing, error, stackTrace);
          Error.throwWithStackTrace(error, stackTrace);
        }
      }
    });
    _tail = result.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    _shutdownFuture = result;
    return result;
  }

  Future<void> _adopt(
    _OwnedCurrentProject candidate,
    CurrentProjectState candidateState,
  ) async {
    final previous = _current;
    _current = candidate;
    _publish(candidateState);
    if (previous != null) await _retire(previous);
  }

  Future<void> _retire(_OwnedCurrentProject owned) async {
    try {
      await _closeOwned(owned);
    } catch (error, stackTrace) {
      _recordCleanupFailure(owned, error, stackTrace);
    }
  }

  Future<void> _closeUnadopted(_OwnedCurrentProject owned) async {
    try {
      await _closeOwned(owned);
    } catch (error, stackTrace) {
      _recordCleanupFailure(owned, error, stackTrace);
    }
  }

  void _recordCleanupFailure(
    _OwnedCurrentProject owned,
    Object error,
    StackTrace stackTrace,
  ) {
    _terminalCleanupFailures.add(
      CurrentProjectCleanupFailure(
        projectKind: switch (owned) {
          _OwnedLegacyCurrentProject() => CurrentProjectKind.legacyFormat1,
          _OwnedManagedRevision3CurrentProject() =>
            CurrentProjectKind.managedRevision3,
        },
        error: error,
        stackTrace: stackTrace,
      ),
    );
  }

  CurrentProjectState _refreshCurrentIfUnchanged(
    _OwnedCurrentProject expected,
  ) {
    if (!identical(_current, expected)) {
      throw const CurrentProjectCoordinatorException(
        'current project changed inside the serialized operation lane',
      );
    }
    final refreshed = _stateOf(expected);
    _publish(refreshed);
    return refreshed;
  }

  void _publish(CurrentProjectState next) {
    if (!_notifierDisposed) state = next;
  }

  Future<T> _enqueue<T>(Future<T> Function() operation) {
    if (_shutdownRequested) {
      return Future<T>.error(const CurrentProjectCoordinatorClosedException());
    }
    final result = _tail.then((_) => operation());
    _tail = result.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    return result;
  }

  Future<LegacyCurrentProjectState> _operateOnLegacy(
    Future<void> Function(LegacyCurrentProjectLease lease) operation,
  ) => _enqueue(() async {
    final current = _current;
    if (current case _OwnedLegacyCurrentProject(:final lease)) {
      await operation(lease);
      return _refreshCurrentIfUnchanged(current) as LegacyCurrentProjectState;
    }

    final createLegacy = _createLegacy;
    if (createLegacy == null) {
      throw const CurrentProjectOperationUnsupportedException(
        'this coordinator cannot create a compatibility project lease',
      );
    }

    LegacyCurrentProjectLease? candidateLease;
    var adopted = false;
    try {
      candidateLease = createLegacy();
      await operation(candidateLease);
      final candidate = _OwnedLegacyCurrentProject(candidateLease);
      final candidateState = _stateOf(candidate) as LegacyCurrentProjectState;
      await _adopt(candidate, candidateState);
      adopted = true;
      return candidateState;
    } catch (error, stackTrace) {
      if (candidateLease != null && !adopted) {
        await _closeUnadopted(_OwnedLegacyCurrentProject(candidateLease));
      }
      Error.throwWithStackTrace(error, stackTrace);
    }
  });

  @override
  void dispose() {
    _notifierDisposed = true;
    unawaited(
      shutdown().then<void>((_) {}, onError: (Object _, StackTrace _) {}),
    );
    super.dispose();
  }
}

sealed class _OwnedCurrentProject {
  const _OwnedCurrentProject();
}

final class _OwnedLegacyCurrentProject extends _OwnedCurrentProject {
  const _OwnedLegacyCurrentProject(this.lease);

  final LegacyCurrentProjectLease lease;
}

final class _OwnedManagedRevision3CurrentProject extends _OwnedCurrentProject {
  const _OwnedManagedRevision3CurrentProject(this.lease);

  final ManagedRevision3CurrentProjectLease lease;
}

CurrentProjectState _stateOf(_OwnedCurrentProject owned) => switch (owned) {
  _OwnedLegacyCurrentProject(:final lease) => LegacyCurrentProjectState(
    path: lease.currentPath,
    hasUnsavedChanges: lease.hasUnsavedChanges,
  ),
  _OwnedManagedRevision3CurrentProject(:final lease) =>
    ManagedRevision3CurrentProjectState(
      root: lease.root,
      projectId: lease.projectId,
      projectRevision: lease.projectRevision,
      head: lease.head,
      requiresReopen: lease.requiresReopen,
    ),
};

Future<void> _closeOwned(_OwnedCurrentProject owned) => switch (owned) {
  _OwnedLegacyCurrentProject(:final lease) => lease.close(),
  _OwnedManagedRevision3CurrentProject(:final lease) => lease.close(),
};
